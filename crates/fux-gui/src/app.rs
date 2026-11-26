//! The window: the tasks on the left, the results with their notes in the middle, the violations
//! on the right, playback and export at the bottom.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::Duration;

use eframe::egui::{self, Color32, RichText};
use fux_cli::book::Workbook;
use fux_cli::commands::{
    marked_sheet_after, report_line, time_seed, violation_position_after, voice_text,
};
use fux_cli::free::{PreparedFree, Repeat, prepare_free, regular_degrees};
use fux_cli::imitation::{Terms, chant_mode as chant_mode_of, form_message};
use fux_cli::input::ScoreInput;
use fux_cli::mass::{about, default_jobs, native_seconds};
use fux_cli::output::{Clefs, Format, OutputArgs, write, write_book};
use fux_core::corpus::{fux_cantus, fux_examples};
use fux_core::generate::{MIN_MEASURES, part_endings};
use fux_core::imitation::chant::Kyriale;
use fux_core::mass::{ChantMass, Lengths, MassPlan, Movement, PartRef, Preset};
use fux_core::rules::{Checker, Profile, Report};
use fux_core::text::latin::strip_acute;
use fux_core::text::{Eleison, SylFlags, Text, TextOptions, ordinary};
use fux_core::{Ambitus, Lang, Mode, ModeSpec, Note, Pitch, Score, VoiceType};
use fux_export::{
    MusicXmlOptions, Sheet, document, lilypond_version, to_midi, to_musicxml,
    to_musicxml_with_fields,
};
use serde::{Deserialize, Serialize};

use crate::audio::{Audio, find_soundfont};
use crate::forms::{
    AnswerKind, BookForm, CadenceChoice, CantusForm, CheckForm, CheckVoice, CounterpointForm,
    FreeForm, FreePreview, ImitationForm, LaterVoices, MassForm, NO_TEXT, OWN_TEXT, SectionChoice,
    SourceKind, SubjectKind, Switch, TextedScore,
};
use crate::jobs::{self, CheckJob, Entry, Found, Job};
use crate::mass::{MassRun, PartState, failure};
use crate::preview::{Preview, Request};
use crate::rules::RulesWindow;

/// The task shown on the left.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tab {
    Counterpoint,
    Free,
    /// A whole mass (phase 14).
    Mass,
    Cantus,
    Check,
    Examples,
    Book,
}

/// What is kept between runs.
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Stored {
    lang: String,
    tab: Tab,
    cp: CounterpointForm,
    free: FreeForm,
    mass: MassForm,
    cf: CantusForm,
    check: CheckForm,
    book: BookForm,
    /// The profile as TOML, as far as it differs from Fux's.
    profile: String,
    output: String,
    formats: Vec<String>,
    original_clefs: bool,
    figures: bool,
    tempo: u32,
    program: u8,
    soundfont: String,
    zoom: f32,
    mark: bool,
    /// Brackets over the entries of an imitation in the notes and the exports.
    entry_marks: bool,
}

impl Default for Stored {
    fn default() -> Self {
        Stored {
            lang: "de".into(),
            tab: Tab::Counterpoint,
            cp: CounterpointForm::default(),
            free: FreeForm::default(),
            mass: MassForm::default(),
            cf: CantusForm::default(),
            check: CheckForm::default(),
            book: BookForm::default(),
            profile: String::new(),
            output: "out/satz".into(),
            formats: vec!["ly".into(), "pdf".into(), "midi".into()],
            original_clefs: false,
            figures: false,
            tempo: 72,
            program: 53,
            soundfont: find_soundfont()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            zoom: 1.0,
            mark: false,
            entry_marks: true,
        }
    }
}

/// How the middle shows a result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum View {
    Notes,
    Text,
}

/// The whole application state.
pub struct FuxApp {
    pub lang: Lang,
    pub tab: Tab,
    pub cp: CounterpointForm,
    pub free: FreeForm,
    pub mass: MassForm,
    pub cf: CantusForm,
    pub check: CheckForm,
    pub book: BookForm,
    pub profile: Profile,
    rules: RulesWindow,
    /// Where to export, without extension, and in which formats.
    pub output: String,
    pub formats: Vec<Format>,
    all_results: bool,
    original_clefs: bool,
    figures: bool,
    tempo: u32,
    /// General MIDI program, 1–128.
    program: u8,
    soundfont: String,
    zoom: f32,
    /// Marks the violations in the notes of every result, not only of checked scores.
    mark: bool,
    /// Brackets over the entries of an imitation in the notes and the exports.
    entry_marks: bool,
    view: View,
    only_forbidden: bool,
    example_filter: String,
    job: Option<Job>,
    /// The results of the last task, the one shown, and lines describing them.
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub summary: Vec<String>,
    /// A message for the status line, and whether it is an error.
    pub message: Option<(bool, String)>,
    preview: Preview,
    audio: Audio,
    /// The LilyPond version found, `None` without LilyPond.
    lilypond: Option<String>,
    /// Counts the sets of results, so that the preview notices new ones.
    generation: u64,
    /// The workbook the results belong to, and whether it has the exercises.
    workbook: Option<(Workbook, bool)>,
    /// The seed that „Plan zeigen“ drew for the free form, which the next „Setzen“ takes, so that
    /// it sets the plan shown.
    planned_seed: Option<u64>,
    /// The mass set last, or being set (phase 14).
    mass_run: Option<MassRun>,
    /// The parts the tab „Messe“ sets at once (K6): as many as `fux mass` without `--jobs`
    /// ([`default_jobs`]); the form has no field for it.
    pub mass_jobs: usize,
    /// The generation of the results while they are the movements of `mass_run`, and the
    /// language they are in.
    mass_shown: Option<(u64, Lang)>,
    /// The seed that „Plan zeigen“ drew for the mass, which the next „Messe setzen“ takes.
    mass_planned_seed: Option<u64>,
    /// What the form of the mass plans, for its table of sections; made anew when the form
    /// changes.
    mass_sketch: Option<MassSketch>,
}

/// What the form of a mass plans before any search, for the form: the rows of its table of
/// sections, from the plan without the form's changes, and the plan with them, as a line on its
/// size and time, or what is wrong with the form.
struct MassSketch {
    /// The form, the seed and the language it was made for.
    key: String,
    rows: Vec<SectionRow>,
    planned: Result<String, String>,
}

/// A section in the table of the mass form.
struct SectionRow {
    id: &'static str,
    title: &'static str,
    /// Its voices by default, from the top: „S A T“.
    voices: String,
    /// The degrees its last part may end on besides the final, by letter and as its movement's
    /// mode spells them; none where it ends on the final (the last section of a movement, two
    /// voices).
    endings: Vec<(String, String)>,
}

impl MassSketch {
    /// The sketch of `form` with the seed `seed` in `lang`; `profile` only for the preparation,
    /// `jobs` for the estimate, the parts the window sets at once. The rows come from the plan
    /// without the changes of sections and parts, and without the head motif, which changes no
    /// section, so that they stay while those are wrong.
    fn new(form: &MassForm, seed: u64, profile: &Profile, jobs: usize, lang: Lang) -> MassSketch {
        let plain = MassForm {
            sections: Vec::new(),
            imitation: String::new(),
            endings: String::new(),
            motif: false,
            ..form.clone()
        };
        let rows = jobs::prepare_mass(&plain, seed, profile, lang)
            .map(|(_, plan)| section_rows(&plan, lang))
            .unwrap_or_default();
        let planned = jobs::prepare_mass(form, seed, profile, lang).map(|(_, plan)| {
            let estimate = plan.estimate();
            let time = about(native_seconds(&plan, jobs), lang);
            match lang {
                Lang::De => format!(
                    "{} Teile, {} Silben; geschätzt {} Takte, {time}",
                    plan.part_count(),
                    plan.syllables(),
                    estimate.measures,
                ),
                Lang::En => format!(
                    "{} parts, {} syllables; estimated {} measures, {time}",
                    plan.part_count(),
                    plan.syllables(),
                    estimate.measures,
                ),
            }
        });
        MassSketch {
            key: sketch_key(form, seed, lang),
            rows,
            planned,
        }
    }
}

/// What a sketch of the mass form depends on.
fn sketch_key(form: &MassForm, seed: u64, lang: Lang) -> String {
    format!("{form:?}|{seed}|{lang}")
}

/// The rows of the table of sections for `plan`: every section but an Osanna ut supra, with its
/// voices and the degrees it may end on.
fn section_rows(plan: &MassPlan, lang: Lang) -> Vec<SectionRow> {
    let mut rows = Vec::new();
    for movement in &plan.movements {
        for (si, section) in movement.sections.iter().enumerate() {
            if section.ut_supra.is_some() {
                continue;
            }
            let mode = movement.mode;
            let voices: Vec<&str> = section
                .voices
                .iter()
                .map(|&v| movement.voices[v].abbreviation())
                .collect();
            // The last section ends the movement on its final, unless an Osanna ut supra
            // follows; so does a section in two voices (MASS-ENDING).
            let ends = si + 1 == movement.sections.len();
            let endings = if ends || section.voices.len() < 3 {
                Vec::new()
            } else {
                part_endings(mode)
                    .into_iter()
                    .map(|step| {
                        let name = Pitch::new(step, mode.scale_alter(step), 4).class_name(lang);
                        (step.letter().to_string(), name)
                    })
                    .collect()
            };
            rows.push(SectionRow {
                id: section.id,
                title: section.title,
                voices: voices.join(" "),
                endings,
            });
        }
    }
    rows
}

impl FuxApp {
    /// The application as eframe starts it, with the settings of the last run.
    pub fn new(cc: &eframe::CreationContext<'_>) -> FuxApp {
        let stored: Stored = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();
        FuxApp::from_stored(stored, lilypond_version())
    }

    /// The application with default settings, for tests: no stored state, no LilyPond.
    pub fn headless() -> FuxApp {
        FuxApp::from_stored(Stored::default(), None)
    }

    fn from_stored(stored: Stored, lilypond: Option<String>) -> FuxApp {
        let profile = Profile::fux()
            .merged_toml(&stored.profile)
            .unwrap_or_else(|_| Profile::fux());
        let formats = stored
            .formats
            .iter()
            .filter_map(|f| format_from_name(f))
            .collect();
        FuxApp {
            lang: stored.lang.parse().unwrap_or_default(),
            tab: stored.tab,
            cp: stored.cp,
            free: stored.free,
            mass: stored.mass,
            cf: stored.cf,
            check: stored.check,
            book: stored.book,
            profile,
            rules: RulesWindow::default(),
            output: stored.output,
            formats,
            all_results: false,
            original_clefs: stored.original_clefs,
            figures: stored.figures,
            tempo: stored.tempo,
            program: stored.program,
            soundfont: stored.soundfont,
            zoom: stored.zoom.clamp(0.3, 3.0),
            mark: stored.mark,
            entry_marks: stored.entry_marks,
            view: View::Notes,
            only_forbidden: false,
            example_filter: String::new(),
            job: None,
            entries: Vec::new(),
            selected: 0,
            summary: Vec::new(),
            message: None,
            preview: Preview::default(),
            audio: Audio::default(),
            lilypond,
            generation: 0,
            workbook: None,
            planned_seed: None,
            mass_run: None,
            mass_jobs: default_jobs(),
            mass_shown: None,
            mass_planned_seed: None,
            mass_sketch: None,
        }
    }

    fn stored(&self) -> Stored {
        Stored {
            lang: self.lang.to_string(),
            tab: self.tab,
            cp: self.cp.clone(),
            free: self.free.clone(),
            mass: self.mass.clone(),
            cf: self.cf.clone(),
            check: self.check.clone(),
            book: self.book.clone(),
            profile: self.profile.to_toml(Some(&Profile::fux())),
            output: self.output.clone(),
            formats: self
                .formats
                .iter()
                .map(|f| format_name(*f).into())
                .collect(),
            original_clefs: self.original_clefs,
            figures: self.figures,
            tempo: self.tempo,
            program: self.program,
            soundfont: self.soundfont.clone(),
            zoom: self.zoom,
            mark: self.mark,
            entry_marks: self.entry_marks,
        }
    }

    /// With the installed LilyPond, so that the notes are rendered (for tests).
    pub fn with_lilypond(mut self) -> FuxApp {
        self.lilypond = lilypond_version();
        self
    }

    /// Whether the notes of the selected result are rendered (or failed to render).
    pub fn preview_ready(&self) -> bool {
        self.preview.has_pages() || self.preview.error.is_some()
    }

    /// Whether a task is running.
    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    /// Waits for the running task, for tests.
    pub fn finish(&mut self) {
        if let Some(job) = self.job.take() {
            self.take(job.join());
        }
    }

    /// The mass set last, or being set.
    pub fn mass_run(&self) -> Option<&MassRun> {
        self.mass_run.as_ref()
    }

    /// Whether parts of a mass are being set.
    pub fn mass_busy(&self) -> bool {
        self.mass_run.as_ref().is_some_and(MassRun::is_running)
    }

    /// The mass whose movements the results are now.
    fn shown_mass(&self) -> Option<&MassRun> {
        self.mass_run.as_ref().filter(|_| self.mass_is_shown())
    }

    /// Whether the results are the movements of the mass.
    fn mass_is_shown(&self) -> bool {
        self.mass_shown
            .is_some_and(|(generation, _)| generation == self.generation)
    }

    /// Draws the window.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.poll(ui.ctx());
        egui::Panel::top("top").show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("bottom").show(ui, |ui| self.bottom_bar(ui));
        egui::Panel::left("task")
            .resizable(true)
            .default_size(340.0)
            .min_size(300.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| self.task_panel(ui));
            });
        egui::Panel::right("report")
            .resizable(true)
            .default_size(380.0)
            .min_size(320.0)
            .show(ui, |ui| self.report_panel(ui));
        egui::CentralPanel::default().show(ui, |ui| self.result_panel(ui));
        let ctx = ui.ctx().clone();
        if self.rules.show(&ctx, &mut self.profile, self.lang) {
            self.message = Some((
                false,
                self.lang
                    .pick(
                        "Regeln geändert; sie gelten ab dem nächsten Lauf.",
                        "Rules changed; they apply from the next run.",
                    )
                    .into(),
            ));
        }
    }

    /// Takes finished work over: the task, the preview, the SoundFont.
    fn poll(&mut self, ctx: &egui::Context) {
        if self.job.as_ref().is_some_and(Job::is_finished) {
            let job = self.job.take().expect("checked");
            self.take(job.join());
        }
        self.poll_mass();
        self.preview.poll(ctx);
        self.audio.poll();
        if let Some(request) = self.preview_request() {
            self.preview.request(request);
        }
        if self.job.is_some()
            || self.mass_busy()
            || self.preview.is_rendering()
            || self.audio.is_loading()
            || self.audio.is_playing()
        {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    /// Takes over what the job of the mass did: the results follow the parts set, also in
    /// another language and under the title of the form, which the mass takes as the other
    /// items of its notation; when the job ends the status line says how it ended.
    fn poll_mass(&mut self) {
        let Some(run) = &mut self.mass_run else {
            return;
        };
        let running = run.is_running();
        let changed = run.poll();
        let titled = run.set_title(&self.mass.title);
        let ended = running && !run.is_running();
        let other_lang = self.mass_shown.is_some_and(|(_, lang)| lang != self.lang);
        if self.mass_is_shown() && (changed || titled || other_lang) {
            self.refresh_mass();
        }
        if ended {
            self.message = self.mass_run.as_ref().map(|run| mass_ended(run, self.lang));
        }
    }

    /// Shows the movements of the mass as the results, the first selected.
    pub fn show_mass(&mut self) {
        let Some(run) = &self.mass_run else {
            return;
        };
        let found = Found {
            entries: run.entries(self.lang),
            summary: mass_lines(run, self.lang),
            ..Found::default()
        };
        self.show_found(found);
        self.mass_shown = Some((self.generation, self.lang));
    }

    /// The movements of the mass shown as results anew, after parts changed, the one selected
    /// staying selected.
    fn refresh_mass(&mut self) {
        let Some(run) = &self.mass_run else {
            return;
        };
        self.entries = run.entries(self.lang);
        self.summary = mass_lines(run, self.lang);
        if self.selected >= self.entries.len() {
            self.selected = 0;
        }
        self.mass_shown = Some((self.generation, self.lang));
    }

    /// The movements of the mass as the results: anew where they are shown, else shown.
    fn present_mass(&mut self) {
        if self.mass_is_shown() {
            self.refresh_mass();
        } else {
            self.show_mass();
        }
    }

    /// The seed of the mass for this run: a new one if the form asks for it, except where the
    /// plan shown drew it: a mass set after „Plan zeigen“ takes the plan's seed, so that it sets
    /// the plan shown. With `plan`, the seed drawn for the plan.
    fn mass_seed(&mut self, plan: bool) -> u64 {
        let kept = if plan {
            None
        } else {
            self.mass_planned_seed.take()
        };
        let seed = match kept {
            Some(seed) if self.mass.random_seed => seed,
            _ if self.mass.random_seed => time_seed(),
            _ => self.mass.seed,
        };
        if plan && self.mass.random_seed {
            self.mass_planned_seed = Some(seed);
        }
        self.mass.seed = seed;
        seed
    }

    /// Shows the plan of the mass of the form without a search, at once: its movements, sections
    /// and parts, how each begins and the degree it ends on, the estimate.
    pub fn show_mass_plan(&mut self) {
        let seed = self.mass_seed(true);
        let found =
            jobs::mass_plan_lines(&self.mass, seed, &self.profile, self.mass_jobs, self.lang);
        self.take(found);
    }

    /// Plans the mass of the form and sets all its parts in the background, its movements shown
    /// as the results as they come. The plan is made first, so that an error in the form shows
    /// at once.
    pub fn start_mass(&mut self) {
        if self.mass_busy() {
            return;
        }
        let seed = self.mass_seed(false);
        match jobs::prepare_mass(&self.mass, seed, &self.profile, self.lang) {
            Ok((prepared, plan)) => {
                let parts = plan.parts().map(|(at, _)| at).collect();
                let mut run = MassRun::new(prepared, plan);
                run.jobs = self.mass_jobs;
                run.start(parts);
                self.mass_run = Some(run);
                self.message = None;
                self.show_mass();
            }
            Err(error) => self.message = Some((true, error)),
        }
    }

    /// Stops the parts being set; the parts set so far stay.
    pub fn cancel_mass(&mut self) {
        if let Some(run) = &self.mass_run {
            run.cancel();
        }
    }

    /// Sets the parts the mass lacks whose search was cancelled or has not run, with the seeds of
    /// its plan and the request it was set with, as `fux mass --resume` with the same options
    /// sets them ([`MassRun::resumable`]). A part whose search failed would fail again so; it is
    /// left to „Teil neu setzen“.
    pub fn set_missing_parts(&mut self) {
        let Some(run) = &mut self.mass_run else {
            return;
        };
        let resumable = run.resumable();
        if resumable.is_empty() || run.is_running() {
            return;
        }
        run.start(resumable);
        self.message = None;
        self.present_mass();
    }

    /// Chooses the part `at` of the mass in the list of parts, for „Teil neu setzen“.
    pub fn choose_part(&mut self, at: PartRef) {
        if let Some(run) = &mut self.mass_run {
            run.chosen = Some(at);
        }
    }

    /// Sets the part chosen in the list of parts again with a new seed, as `fux mass --only
    /// PART --seed N` does.
    pub fn set_part_again(&mut self) {
        let Some(run) = &mut self.mass_run else {
            return;
        };
        let Some(at) = run.chosen.filter(|_| !run.is_running()) else {
            return;
        };
        run.set_again(at, time_seed());
        self.message = None;
        self.present_mass();
    }

    fn take(&mut self, result: Result<Found, String>) {
        match result {
            Ok(found) => {
                let count = found.entries.len();
                let note = found.note.clone();
                self.show_found(found);
                self.message = Some((
                    false,
                    note.unwrap_or_else(|| match self.lang {
                        Lang::De => format!("{count} Ergebnis(se)."),
                        Lang::En => format!("{count} result(s)."),
                    }),
                ));
            }
            Err(error) => self.message = Some((true, error)),
        }
    }

    fn show_found(&mut self, found: Found) {
        self.audio.stop();
        self.entries = found.entries;
        self.summary = found.summary;
        // A workbook is exported whole.
        if found.book.is_some() {
            self.all_results = true;
        }
        self.workbook = found.book;
        self.selected = 0;
        self.generation += 1;
        self.preview.clear();
    }

    /// The LilyPond source of the selected result, if the notes are to be shown.
    fn preview_request(&self) -> Option<Request> {
        let entry = self.entries.get(self.selected)?;
        if self.lilypond.is_none() || self.view != View::Notes {
            return None;
        }
        if let Some(run) = self.shown_mass() {
            return self.movement_request(run);
        }
        let mut hasher = DefaultHasher::new();
        (
            self.generation,
            self.selected,
            self.mark || entry.marked,
            self.original_clefs,
            self.figures,
            self.entry_marks,
            self.lang.to_string(),
        )
            .hash(&mut hasher);
        Some(Request {
            key: hasher.finish(),
            source: document(
                None,
                &[self.sheet(entry)],
                &self.output_args().lilypond_options(self.lang),
            ),
        })
    }

    /// The pages of the selected movement of the mass shown, as the book of the mass prints them,
    /// with the notation and the title of the form; none while no section of it is set. The
    /// source is made only where these pages are not shown or rendered already: the movement
    /// changes as the parts it prints are set, set anew or found missing
    /// ([`MassRun::movement_key`]).
    fn movement_request(&self, run: &MassRun) -> Option<Request> {
        let movement = self.selected;
        if !run.has_notes(movement) {
            return None;
        }
        let notation = self.mass.notation();
        let mut hasher = DefaultHasher::new();
        (
            "mass",
            run.plan().seed,
            movement,
            run.movement_key(movement, self.lang),
            run.title(),
            self.original_clefs,
            self.figures,
            self.entry_marks,
            notation.mensurstriche,
            notation.intonation,
            self.lang.to_string(),
        )
            .hash(&mut hasher);
        let key = hasher.finish();
        if self.preview.knows(key) {
            return None;
        }
        Some(Request {
            key,
            source: run.movement_book(
                movement,
                &self.output_args().lilypond_options(self.lang),
                &notation,
                self.lang,
            ),
        })
    }

    /// The sheet of a result, marked if asked for.
    fn sheet<'a>(&self, entry: &'a Entry) -> Sheet<'a> {
        entry_sheet(entry, self.mark, &Checker::new(&self.profile), self.lang)
    }

    fn output_args(&self) -> OutputArgs {
        OutputArgs {
            output: Some(PathBuf::from(&self.output)),
            format: self.formats.clone(),
            clefs: if self.original_clefs {
                Clefs::Original
            } else {
                Clefs::Modern
            },
            figures: self.figures,
            tempo: self.tempo,
            program: self.program,
            marks: self.entry_marks,
        }
    }

    // ---- top bar ----

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        ui.horizontal(|ui| {
            for (tab, label) in [
                (Tab::Counterpoint, lang.pick("Kontrapunkt", "Counterpoint")),
                (Tab::Free, lang.pick("Freier Satz", "Free counterpoint")),
                (Tab::Mass, lang.pick("Messe", "Mass")),
                (Tab::Cantus, "Cantus firmus"),
                (Tab::Check, lang.pick("Prüfen", "Check")),
                (Tab::Examples, lang.pick("Fux' Beispiele", "Fux's examples")),
                (Tab::Book, lang.pick("Übungsheft", "Workbook")),
            ] {
                ui.selectable_value(&mut self.tab, tab, label);
            }
            ui.separator();
            if ui.button(lang.pick("Regeln …", "Rules …")).clicked() {
                self.rules.open = !self.rules.open;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.selectable_value(&mut self.lang, Lang::En, "English");
                ui.selectable_value(&mut self.lang, Lang::De, "Deutsch");
            });
        });
    }

    // ---- tasks ----

    fn task_panel(&mut self, ui: &mut egui::Ui) {
        match self.tab {
            Tab::Counterpoint => self.counterpoint_task(ui),
            Tab::Free => self.free_task(ui),
            Tab::Mass => self.mass_task(ui),
            Tab::Cantus => self.cantus_task(ui),
            Tab::Check => self.check_task(ui),
            Tab::Examples => self.examples_task(ui),
            Tab::Book => self.book_task(ui),
        }
    }

    fn book_task(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let book = &mut self.book;
        ui.heading(lang.pick("Übungsheft", "Workbook"));
        ui.label(lang.pick(
            "Ein Satz in jedem Modus, jeder Gattung und Stimmenzahl, zweistimmig mit der \
             Gegenstimme oben und unten; exportiert als ein Dokument.",
            "A setting in every mode, species and number of voices, in two voices with the \
             counterpoint above and below; exported as one document.",
        ));
        ui.horizontal(|ui| {
            ui.label(lang.pick("Modi:", "Modes:"));
            ui.radio_value(
                &mut book.all_modes,
                false,
                lang.pick("die sechs von Fux", "Fux's six"),
            );
            ui.radio_value(
                &mut book.all_modes,
                true,
                lang.pick("alle zwölf", "all twelve"),
            );
        });
        ui.horizontal(|ui| {
            ui.label(lang.pick("Gattungen:", "Species:"));
            for (i, on) in book.species.iter_mut().enumerate() {
                ui.checkbox(on, (i + 1).to_string());
            }
        });
        ui.horizontal(|ui| {
            ui.label(lang.pick("Stimmen:", "Voices:"));
            for (i, on) in book.voices.iter_mut().enumerate() {
                ui.checkbox(on, (i + 2).to_string());
            }
        });
        ui.checkbox(
            &mut book.generate,
            lang.pick(
                "Cantus erzeugen statt Fux' Cantus",
                "Generated cantus instead of Fux's",
            ),
        );
        if book.generate || book.all_modes {
            ui.horizontal(|ui| {
                let label =
                    ui.label(lang.pick("Länge erzeugter Cantus:", "Length of generated cantus:"));
                ui.add(egui::DragValue::new(&mut book.length).range(6..=20))
                    .labelled_by(label.id);
            });
        }
        ui.checkbox(
            &mut book.exercises,
            lang.pick(
                "Aufgaben vor den Lösungen (Cantus mit leeren Systemen)",
                "Exercises before the solutions (cantus with empty staves)",
            ),
        );
        seed_choice(ui, lang, &mut book.random_seed, &mut book.seed);
        ui.add_space(8.0);
        let ready = book.species.contains(&true) && book.voices.contains(&true);
        if ui
            .add_enabled(
                self.job.is_none() && ready,
                egui::Button::new(RichText::new(lang.pick("Erstellen", "Create")).strong()),
            )
            .clicked()
        {
            self.start_book();
        }
    }

    /// Starts `fux book` with the form.
    pub fn start_book(&mut self) {
        let seed = if self.book.random_seed {
            time_seed()
        } else {
            self.book.seed
        };
        self.book.seed = seed;
        let options = self.book.options(seed);
        let profile = self.profile.clone();
        let lang = self.lang;
        self.job = Some(Job::spawn(
            lang.pick("Erstelle Übungsheft …", "Creating the workbook …")
                .into(),
            move || jobs::book(&options, &profile, lang),
        ));
    }

    fn counterpoint_task(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let cp = &mut self.cp;
        ui.heading("Cantus firmus");
        for kind in SourceKind::ALL {
            ui.radio_value(&mut cp.source, kind, kind.label(lang));
        }
        match cp.source {
            SourceKind::Fux => {
                let cantus = fux_cantus();
                ui.horizontal(|ui| {
                    let label = ui.label("Cantus:");
                    egui::ComboBox::from_id_salt("fux-cantus")
                        .selected_text(format!("fux:{}", cp.fux_cantus))
                        .show_ui(ui, |ui| {
                            for c in &cantus {
                                ui.selectable_value(
                                    &mut cp.fux_cantus,
                                    c.id.clone(),
                                    format!("{} – {}", c.id, c.mode.name(lang)),
                                );
                            }
                        })
                        .response
                        .labelled_by(label.id);
                });
            }
            SourceKind::Notes => {
                let label = ui.label(lang.pick(
                    "Notennamen (D4 F4 E4) oder LilyPond (d'1 f' e'):",
                    "Note names (D4 F4 E4) or LilyPond (d'1 f' e'):",
                ));
                ui.add(
                    egui::TextEdit::multiline(&mut cp.notes)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(label.id);
            }
            SourceKind::Gabc => {
                ui.horizontal(|ui| {
                    let label = ui.label(lang.pick("Datei:", "File:"));
                    ui.add(egui::TextEdit::singleline(&mut cp.gabc).desired_width(200.0))
                        .labelled_by(label.id);
                    if ui
                        .button("…")
                        .on_hover_text(lang.pick("Datei wählen", "Choose a file"))
                        .clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .add_filter("GABC, MusicXML", &["gabc", "musicxml", "xml"])
                            .pick_file()
                    {
                        cp.gabc = path.display().to_string();
                    }
                });
            }
            SourceKind::Generated => {
                mode_choice(ui, "generated-mode", lang, &mut cp.generated_mode);
                ui.horizontal(|ui| {
                    let label = ui.label(lang.pick("Länge:", "Length:"));
                    ui.add(egui::DragValue::new(&mut cp.length).range(6..=20))
                        .labelled_by(label.id);
                });
            }
        }
        if cp.source != SourceKind::Generated {
            optional_mode_choice(ui, "cp-mode", lang, &mut cp.mode);
        }
        optional_voice_choice(
            ui,
            "cp-cf-voice",
            lang.pick("Stimmlage des Cantus:", "Voice of the cantus:"),
            lang,
            &mut cp.cf_voice,
        );
        cadence_choice(
            ui,
            "cp-cadences",
            lang,
            &mut cp.cadences,
            &mut cp.cadence_measures,
        );
        ui.separator();
        ui.heading(lang.pick("Satz", "Setting"));
        ui.horizontal(|ui| {
            let label = ui.label(lang.pick("Stimmen:", "Voices:"));
            ui.add(egui::DragValue::new(&mut cp.voices).range(2..=6))
                .labelled_by(label.id);
            let label = ui.label(lang.pick("Gattung:", "Species:"));
            ui.add(egui::DragValue::new(&mut cp.species).range(0..=5))
                .on_hover_text(lang.pick(
                    "0: freier Satz, alle Gegenstimmen frei im Stil Palestrinas",
                    "0: free counterpoint, every voice free in Palestrina's style",
                ))
                .labelled_by(label.id);
        });
        if cp.voices > 2 {
            ui.horizontal(|ui| {
                let label = ui.label(lang.pick("Besetzung:", "Parts:"));
                ui.add(
                    egui::TextEdit::singleline(&mut cp.parts)
                        .hint_text(lang.pick("z. B. S A T B", "e.g. S A T B"))
                        .desired_width(120.0),
                )
                .labelled_by(label.id);
            });
        }
        optional_voice_choice(
            ui,
            "cp-voice",
            if cp.voices > 2 {
                lang.pick("Gattungsstimme:", "Voice in the species:")
            } else {
                lang.pick("Gegenstimme:", "Counterpoint:")
            },
            lang,
            &mut cp.voice,
        );
        if cp.voices == 2 {
            ui.checkbox(
                &mut cp.below,
                lang.pick("unter dem Cantus", "below the cantus"),
            );
        }
        // Free voices may sing a text, which the cantus never sings.
        if cp.species == 0 {
            ui.separator();
            ui.heading("Text");
            ui.label(lang.pick(
                "Die freien Stimmen singen den Text; der Cantus behält seinen eigenen (Choral) \
                 oder bleibt ohne.",
                "The free voices sing the text; the cantus keeps its own (chant) or has none.",
            ));
            let preview = cp.preview(lang);
            let sung = preview.as_ref().map(|p| Sung {
                text: p.text.as_deref(),
                warnings: &p.warnings,
                length: match lang {
                    Lang::De => format!(
                        "Länge: die des Cantus; dieser Text braucht mindestens {} Takte.",
                        p.least
                    ),
                    Lang::En => format!(
                        "Length: the cantus'; this text needs at least {} measures.",
                        p.least
                    ),
                },
            });
            let singing = Singing {
                id: "cp",
                who: ("die freien Stimmen singen", "the free voices sing"),
                text: &mut cp.text,
                own_text: &mut cp.own_text,
                repeat: &mut cp.repeat,
                accents: &mut cp.accents,
                eleison3: &mut cp.eleison3,
            };
            text_choice(ui, lang, singing, sung);
        }
        ui.separator();
        ui.heading(lang.pick("Suche", "Search"));
        seed_choice(ui, lang, &mut cp.random_seed, &mut cp.seed);
        egui::Grid::new("cp-search").num_columns(2).show(ui, |ui| {
            let label = ui.label(lang.pick("Lösungen:", "Solutions:"));
            ui.add(egui::DragValue::new(&mut cp.count).range(1..=20))
                .labelled_by(label.id);
            ui.end_row();
            let label = ui.label(lang.pick("Breite:", "Beam:"));
            ui.add(egui::DragValue::new(&mut cp.beam).range(20..=2000))
                .labelled_by(label.id);
            ui.end_row();
            let label = ui.label(lang.pick("Zufall:", "Noise:"));
            ui.add(
                egui::DragValue::new(&mut cp.noise)
                    .range(0.0..=10.0)
                    .speed(0.1),
            )
            .labelled_by(label.id);
            ui.end_row();
        });
        ui.add_space(8.0);
        if ui
            .add_enabled(
                self.job.is_none(),
                egui::Button::new(RichText::new(lang.pick("Setzen", "Compose")).strong()),
            )
            .clicked()
        {
            self.start_counterpoint();
        }
    }

    /// Starts `fux cp` with the form.
    pub fn start_counterpoint(&mut self) {
        let seed = if self.cp.random_seed {
            time_seed()
        } else {
            self.cp.seed
        };
        self.cp.seed = seed;
        match self.cp.options(seed) {
            Ok(options) => {
                let profile = self.profile.clone();
                let lang = self.lang;
                self.job = Some(Job::spawn(
                    self.lang.pick("Setze Kontrapunkt …", "Composing …").into(),
                    move || jobs::counterpoint(&options, &profile, lang),
                ));
            }
            Err(error) => self.message = Some((true, error)),
        }
    }

    fn free_task(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        // What the form sets, before the search: the text, its warnings and the lengths.
        let preview = self.free.preview(&self.profile, lang);
        let free = &mut self.free;
        ui.heading(lang.pick("Freier Satz", "Free counterpoint"));
        ui.label(lang.pick(
            "Alle Stimmen frei im Stil Palestrinas, ohne Cantus firmus; alle singen den \
             gewählten Text. Die Schlüsse stehen als Klauseln fest: Diskantklausel mit Vorhalt \
             in der Oberstimme, Tenor- und Bassklausel darunter; alles andere findet die Suche.",
            "Every voice free in Palestrina's style, without a cantus firmus; every voice sings \
             the chosen text. The cadences are fixed as clausulae: the cantizans with its \
             suspension in the top voice, tenorizans and basizans below; the search finds the \
             rest.",
        ));
        // An imitation's plan sets the length, which only the planner knows.
        let imitating = free.imitates(lang);
        // The section of the Ordinary the voices sing, for the chant of the Kyriale.
        let section = preview
            .as_ref()
            .ok()
            .and_then(|p| p.text.as_ref())
            .map(|text| text.id.clone());
        // A chant's subjects in the chant's mode: the form's mode does not count then.
        let chant_mode = imitating && free.imitation.chant_subject() && free.imitation.chant_mode;
        let own_mode = match free.imitation.subject {
            SubjectKind::Kyriale if chant_mode => section
                .as_deref()
                .and_then(Kyriale::for_section)
                .and_then(|kyriale| chant_mode_of(&kyriale.chant())),
            _ => None,
        };
        ui.add_enabled_ui(!chant_mode, |ui| {
            mode_choice(ui, "free-mode", lang, &mut free.mode);
        });
        if chant_mode {
            ui.label(match (own_mode, lang) {
                (Some(mode), Lang::De) => format!("Im Modus des Chorals: {}", mode.name(lang)),
                (Some(mode), Lang::En) => format!("In the chant's mode: {}", mode.name(lang)),
                (None, _) => lang
                    .pick("Im Modus des Chorals", "In the chant's mode")
                    .to_string(),
            });
        }
        let sung = preview.as_ref().map(|p| Sung {
            text: p.text.as_deref(),
            warnings: &p.warnings,
            length: match (imitating, lang) {
                (true, _) => lang
                    .pick(
                        "Länge: nach dem Plan der Imitation („Plan zeigen“)",
                        "Length: by the plan of the imitation (“Show the plan”)",
                    )
                    .to_string(),
                (false, Lang::De) => format!("Automatische Länge: {} Takte", p.automatic),
                (false, Lang::En) => format!("Automatic length: {} measures", p.automatic),
            },
        });
        let singing = Singing {
            id: "free",
            who: ("alle Stimmen singen", "every voice sings"),
            text: &mut free.text,
            own_text: &mut free.own_text,
            repeat: &mut free.repeat,
            accents: &mut free.accents,
            eleison3: &mut free.eleison3,
        };
        text_choice(ui, lang, singing, sung);
        let automatic = preview.as_ref().map_or(free.length, |p| p.automatic);
        egui::Grid::new("free-grid").num_columns(2).show(ui, |ui| {
            let label = ui.label(lang.pick("Stimmen:", "Voices:"));
            ui.add(egui::DragValue::new(&mut free.voices).range(2..=6))
                .labelled_by(label.id);
            ui.end_row();
            let label = ui.label(lang.pick("Besetzung:", "Parts:"));
            ui.add(
                egui::TextEdit::singleline(&mut free.parts)
                    .hint_text(lang.pick("z. B. S A T B", "e.g. S A T B"))
                    .desired_width(120.0),
            )
            .labelled_by(label.id);
            ui.end_row();
            let label = ui.label(lang.pick("Takte:", "Measures:"));
            ui.horizontal(|ui| {
                if free.length_auto && imitating {
                    // The plan of the imitation sets it.
                    ui.weak(lang.pick("aus dem Plan", "from the plan"))
                        .labelled_by(label.id);
                } else if free.length_auto {
                    // The length the text plans, shown but not edited.
                    let mut planned = automatic;
                    ui.add_enabled(false, egui::DragValue::new(&mut planned))
                        .labelled_by(label.id);
                } else {
                    ui.add(egui::DragValue::new(&mut free.length).range(MIN_MEASURES..=200))
                        .labelled_by(label.id);
                }
                let toggled = ui
                    .checkbox(&mut free.length_auto, lang.pick("automatisch", "automatic"))
                    .on_hover_text(lang.pick(
                        "So viele Takte, wie der Text braucht; ohne Text 12",
                        "As many measures as the text needs; 12 without a text",
                    ))
                    .changed();
                // Set by hand from the length planned.
                if toggled && !free.length_auto {
                    free.length = automatic;
                }
            });
            ui.end_row();
            let label = ui.label(lang.pick("Binnenschlüsse:", "Internal cadences:"));
            ui.add(
                egui::TextEdit::singleline(&mut free.cadences)
                    .hint_text(lang.pick("auto, none, 7, 12:F", "auto, none, 7, 12:F"))
                    .desired_width(120.0),
            )
            .on_hover_text(lang.pick(
                "auto: mit Text an Enden von Sinnabschnitten, sonst etwa alle sechs Takte; none: \
                 keine; sonst die Takte, in denen das Ziel erreicht wird, mit oder ohne Zielton",
                "auto: with a text at ends of sense units, otherwise about every six measures; \
                 none: none; otherwise the measures the target arrives in, with or without the \
                 target",
            ))
            .labelled_by(label.id);
            ui.end_row();
        });
        if let Ok(FreePreview {
            least: Some(least), ..
        }) = &preview
        {
            ui.colored_label(
                Color32::from_rgb(220, 50, 50),
                match lang {
                    Lang::De => format!("Der Text braucht mindestens {least} Takte."),
                    Lang::En => format!("The text needs at least {least} measures."),
                },
            );
        }
        if !chant_mode || own_mode.is_some() {
            ui.label(format!(
                "{}: {}",
                lang.pick("Kadenzstufen des Modus", "Cadence degrees of the mode"),
                regular_degrees(own_mode.unwrap_or(free.mode), lang)
            ));
        }
        ui.separator();
        let voices = free.voice_count(lang);
        let show_plan = imitation_group(
            ui,
            lang,
            &mut free.imitation,
            imitating,
            section.as_deref(),
            voices,
            self.job.is_none(),
        );
        ui.separator();
        ui.heading(lang.pick("Suche", "Search"));
        seed_choice(ui, lang, &mut free.random_seed, &mut free.seed);
        egui::Grid::new("free-search")
            .num_columns(2)
            .show(ui, |ui| {
                let label = ui.label(lang.pick("Lösungen:", "Solutions:"));
                ui.add(egui::DragValue::new(&mut free.count).range(1..=20))
                    .labelled_by(label.id);
                ui.end_row();
                let label = ui.label(lang.pick("Breite:", "Beam:"));
                ui.add(egui::DragValue::new(&mut free.beam).range(20..=2000))
                    .labelled_by(label.id);
                ui.end_row();
                let label = ui.label(lang.pick("Zufall:", "Noise:"));
                ui.add(
                    egui::DragValue::new(&mut free.noise)
                        .range(0.0..=10.0)
                        .speed(0.1),
                )
                .labelled_by(label.id);
                ui.end_row();
            });
        ui.add_space(8.0);
        if ui
            .add_enabled(
                self.job.is_none(),
                egui::Button::new(RichText::new(lang.pick("Setzen", "Compose")).strong()),
            )
            .clicked()
        {
            self.start_free();
        }
        if show_plan {
            self.start_plan();
        }
    }

    /// The prepared request of the free form with a seed for this run: a new one if the form
    /// asks for it, except where the plan shown (`plan`) drew it: a setting after „Plan zeigen“
    /// takes the plan's seed, so that it sets the plan shown.
    fn prepared_free(&mut self, plan: bool) -> Result<PreparedFree, String> {
        let kept = if plan { None } else { self.planned_seed.take() };
        let seed = match kept {
            Some(seed) if self.free.random_seed => seed,
            _ if self.free.random_seed => time_seed(),
            _ => self.free.seed,
        };
        if plan && self.free.random_seed {
            self.planned_seed = Some(seed);
        }
        self.free.seed = seed;
        let lang = self.lang;
        self.free.options(seed, lang).and_then(|options| {
            prepare_free(&options, &self.profile, lang)
                .map_err(|e| form_message(&e, lang, Terms::Window))
        })
    }

    /// Starts `fux free` with the form. The text is read first, so that an error in it shows at
    /// once, like the other errors of the form.
    pub fn start_free(&mut self) {
        let lang = self.lang;
        match self.prepared_free(false) {
            Ok(prepared) => {
                self.job = Some(Job::spawn(
                    lang.pick("Setze freien Satz …", "Composing …").into(),
                    move || jobs::free(&prepared, lang),
                ));
            }
            Err(error) => self.message = Some((true, error)),
        }
    }

    /// Plans the imitation of the free form without a search and shows the plan: its points,
    /// their heads and entries.
    pub fn start_plan(&mut self) {
        let lang = self.lang;
        match self.prepared_free(true) {
            Ok(prepared) => {
                self.job = Some(Job::spawn(
                    lang.pick("Plane die Imitation …", "Planning the imitation …")
                        .into(),
                    move || jobs::plan(&prepared, lang),
                ));
            }
            Err(error) => self.message = Some((true, error)),
        }
    }

    /// The form of a mass (phase 14), its buttons, and while a mass is set or after, where it
    /// stands: a bar, a line on the parts being searched and the list of its parts.
    fn mass_task(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        // What the form plans, made anew when the form changes.
        let key = sketch_key(&self.mass, self.mass.seed, lang);
        if self
            .mass_sketch
            .as_ref()
            .is_none_or(|sketch| sketch.key != key)
        {
            self.mass_sketch = Some(MassSketch::new(
                &self.mass,
                self.mass.seed,
                &self.profile,
                self.mass_jobs,
                lang,
            ));
        }
        let busy = self.mass_busy();
        let form = &mut self.mass;
        let sketch = self.mass_sketch.as_ref().expect("made above");
        ui.heading(lang.pick("Messe nach Palestrina", "A mass after Palestrina"));
        ui.label(lang.pick(
            "Kyrie, Gloria, Credo, Sanctus, Benedictus und Agnus Dei in den Formen von \
             Palestrinas Messen, mit dem Text des Ordinariums. Ein Plan teilt die Messe vor der \
             Suche in Teile; jeder Teil ist ein freier Satz mit seinem Text, der imitierend oder \
             gemeinsam beginnt, und wird für sich gesucht.",
            "Kyrie, Gloria, Credo, Sanctus, Benedictus and Agnus Dei in the forms of \
             Palestrina's masses, with the text of the Ordinary. A plan divides the mass into \
             parts before the search; each part is a free setting with its text that begins in \
             imitation or together, searched on its own.",
        ));
        ui.horizontal(|ui| {
            ui.label(lang.pick("Form:", "Form:"));
            ui.radio_value(&mut form.preset, Preset::Brevis, "brevis")
                .on_hover_text(lang.pick(
                    "Nach Palestrinas Missa brevis (1570): das Gloria mit Halteklang bei Qui \
                     tollis, das Credo ungeteilt mit Halteklängen bei Et incarnatus und \
                     Crucifixus, das Sanctus durchkomponiert, das Benedictus ohne Bass mit neuem \
                     Osanna",
                    "After Palestrina's Missa brevis (1570): the Gloria with a held chord at Qui \
                     tollis, the Credo undivided with held chords at Et incarnatus and \
                     Crucifixus, the Sanctus through-composed, the Benedictus without the bass \
                     and with an Osanna of its own",
                ));
            ui.radio_value(&mut form.preset, Preset::Normal, "normal")
                .on_hover_text(lang.pick(
                    "Nach den Mittelwerten seiner vierstimmigen Messen: Gloria und Credo in \
                     Abschnitten, Sanctus, Pleni und Osanna für sich, nach dem Benedictus \
                     „Osanna ut supra“",
                    "After the medians of his four-voice masses: Gloria and Credo in sections, \
                     Sanctus, Pleni and Osanna apart, after the Benedictus “Osanna ut supra”",
                ));
        });
        let chant_before = form.chant;
        ui.horizontal(|ui| {
            let label = ui.label(lang.pick("Choral:", "Chant:"));
            let name = |chant: Option<ChantMass>| match chant {
                None => lang.pick("keiner", "none"),
                Some(ChantMass::IX) => "IX (Cum jubilo)",
                Some(ChantMass::IV) => "IV (Cunctipotens)",
            };
            egui::ComboBox::from_id_salt("mass-chant")
                .selected_text(name(form.chant))
                .show_ui(ui, |ui| {
                    for chant in [None, Some(ChantMass::IX), Some(ChantMass::IV)] {
                        ui.selectable_value(&mut form.chant, chant, name(chant));
                    }
                })
                .response
                .on_hover_text(lang.pick(
                    "Die Soggetti der imitierenden Teile paraphrasieren den Choral des Kyriale: \
                     Messe IX (Cum jubilo) mit Credo I, oder Kyrie IV (Cunctipotens genitor \
                     Deus) und die übrigen Sätze wie IX",
                    "The subjects of the imitative parts paraphrase the chant of the Kyriale: \
                     Mass IX (Cum jubilo) with Credo I, or Kyrie IV (Cunctipotens genitor Deus) \
                     and the other movements as IX",
                ))
                .labelled_by(label.id);
        });
        // A chant chosen gives each movement its mode, as `fux mass --chant` without `--mode`.
        if chant_before.is_none() && form.chant.is_some() {
            form.chant_mode = true;
        }
        ui.horizontal(|ui| {
            let label = ui.label(lang.pick("Modus:", "Mode:"));
            let by_chant = lang.pick("nach dem Choral", "by the chant");
            let shown = if form.modes_of_chant() {
                by_chant.to_string()
            } else {
                form.mode.name(lang)
            };
            egui::ComboBox::from_id_salt("mass-mode")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    if form.chant.is_some()
                        && ui.selectable_label(form.chant_mode, by_chant).clicked()
                    {
                        form.chant_mode = true;
                    }
                    for mode in all_modes() {
                        let chosen = !form.modes_of_chant() && form.mode == mode;
                        if ui.selectable_label(chosen, mode.name(lang)).clicked() {
                            form.mode = mode;
                            form.chant_mode = false;
                        }
                    }
                })
                .response
                .on_hover_text(lang.pick(
                    "Ein Modus für die ganze Messe; „nach dem Choral“: jeder Satz im Modus seines \
                     Chorals",
                    "One mode for the whole mass; “by the chant”: each movement in the mode of \
                     its chant",
                ))
                .labelled_by(label.id);
        });
        ui.checkbox(&mut form.motif, lang.pick("Kopfmotiv", "Head motif"))
            .on_hover_text(lang.pick(
                "Das Soggetto des ersten Punktes von Kyrie I eröffnet Gloria, Credo, Sanctus und \
                 Agnus Dei I, seine Töne neu deklamiert; nicht zusammen mit einem Choral",
                "The subject of the first point of Kyrie I opens Gloria, Credo, Sanctus and \
                 Agnus Dei I, its notes declaimed anew; not together with a chant",
            ));
        egui::Grid::new("mass-choir").num_columns(2).show(ui, |ui| {
            let label = ui.label(lang.pick("Stimmen:", "Voices:"));
            ui.add(egui::DragValue::new(&mut form.voices).range(2..=6))
                .labelled_by(label.id);
            ui.end_row();
            let label = ui.label(lang.pick("Besetzung:", "Parts:"));
            ui.add(
                egui::TextEdit::singleline(&mut form.parts)
                    .hint_text(lang.pick("z. B. S A T B", "e.g. S A T B"))
                    .desired_width(120.0),
            )
            .labelled_by(label.id);
            ui.end_row();
        });
        ui.label(lang.pick("Sätze:", "Movements:"));
        ui.horizontal_wrapped(|ui| {
            for (movement, on) in Movement::ALL.into_iter().zip(form.movements.iter_mut()) {
                ui.checkbox(on, movement.title());
            }
        });
        egui::CollapsingHeader::new(lang.pick("Abschnitte", "Sections"))
            .id_salt("mass-sections")
            .show(ui, |ui| section_table(ui, lang, form, &sketch.rows));
        egui::Grid::new("mass-parts-grid")
            .num_columns(2)
            .show(ui, |ui| {
                let label = ui.label(lang.pick("Silben je Teil:", "Syllables per part:"));
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        !form.whole,
                        egui::DragValue::new(&mut form.chunk)
                            .range(fux_core::mass::tables::CHUNK_RANGE),
                    )
                    .on_hover_text(lang.pick(
                        "Gloria und Credo werden an Satzenden in die wenigsten Teile von \
                         höchstens so vielen Silben geschnitten, an Halteklängen immer",
                        "Gloria and Credo are cut at the ends of sentences into the fewest parts \
                         of at most so many syllables, at held chords always",
                    ))
                    .labelled_by(label.id);
                    ui.checkbox(&mut form.whole, lang.pick("ungeteilt", "uncut"))
                        .on_hover_text(lang.pick(
                            "Ein Teil von Halteklang zu Halteklang; lange Teile brauchen ein \
                             Vielfaches der Zeit",
                            "A part from held chord to held chord; long parts take many times \
                             as long",
                        ));
                });
                ui.end_row();
                let label = ui.label(lang.pick("Längen:", "Lengths:"));
                let name = |lengths: Lengths| match lengths {
                    Lengths::Palestrina => "palestrina",
                    Lengths::Natural => lang.pick("natürlich", "natural"),
                };
                egui::ComboBox::from_id_salt("mass-lengths")
                    .selected_text(name(form.lengths))
                    .show_ui(ui, |ui| {
                        for lengths in [Lengths::Palestrina, Lengths::Natural] {
                            ui.selectable_value(&mut form.lengths, lengths, name(lengths));
                        }
                    })
                    .response
                    .on_hover_text(lang.pick(
                        "palestrina: Benedictus und Agnus Dei I mit zwei Imitationspunkten mehr, \
                         näher an den Längen seiner Messen; natürlich: so viele Punkte, wie die \
                         Imitation selbst plant",
                        "palestrina: Benedictus and Agnus Dei I with two points of imitation \
                         more, nearer the lengths of his masses; natural: as many points as the \
                         imitation plans by itself",
                    ))
                    .labelled_by(label.id);
                ui.end_row();
            });
        singing_choice(
            ui,
            "mass",
            lang,
            &mut form.repeat,
            &mut form.accents,
            &mut form.eleison3,
        );
        let original_clefs = &mut self.original_clefs;
        egui::CollapsingHeader::new(lang.pick("Notenbild", "Notation"))
            .id_salt("mass-notation")
            .show(ui, |ui| {
                ui.checkbox(&mut form.mensurstriche, "Mensurstriche")
                    .on_hover_text(lang.pick(
                        "Taktstriche nur zwischen den Systemen, über sie gebundene Noten als ein \
                         Wert, wie in Ausgaben der Musik des 16. Jahrhunderts",
                        "Bar lines between the staves only, notes tied over them as one value, \
                         as in editions of 16th-century music",
                    ));
                ui.checkbox(
                    &mut form.intonation,
                    lang.pick(
                        "Intonation vor Gloria und Credo",
                        "Intonation before Gloria and Credo",
                    ),
                )
                .on_hover_text("Intonatio: Glória in excélsis Deo. · Credo in unum Deum,");
                ui.checkbox(
                    &mut form.hide_tacet,
                    lang.pick(
                        "pausierende Stimmen verbergen (MusicXML)",
                        "hide voices at rest (MusicXML)",
                    ),
                )
                .on_hover_text(lang.pick(
                    "Eine Stimme, die einen Abschnitt lang schweigt, in MusicXML dort verbergen; \
                     das Buch zeigt ohnehin nur die Stimmen, die singen",
                    "Hide a voice that rests through a section there in MusicXML; the book \
                     shows only the voices that sing anyway",
                ));
                ui.checkbox(
                    original_clefs,
                    lang.pick("Originalschlüssel", "Original clefs"),
                )
                .on_hover_text(lang.pick(
                    "C- und F-Schlüssel wie bei Fux, im Buch und in MusicXML",
                    "C and F clefs as in Fux, in the book and in MusicXML",
                ));
                ui.horizontal(|ui| {
                    let label = ui.label(lang.pick("Titel:", "Title:"));
                    ui.add(
                        egui::TextEdit::singleline(&mut form.title)
                            .hint_text(form.preset.title())
                            .desired_width(180.0),
                    )
                    .on_hover_text(lang.pick(
                        "Der Titel über dem Buch und den Sätzen, auch einer schon gesetzten Messe; \
                         leer: der Titel der Form",
                        "The title over the book and the movements, also of a mass set already; \
                         empty: the title of the form",
                    ))
                    .labelled_by(label.id);
                });
            });
        ui.separator();
        ui.heading(lang.pick("Suche", "Search"));
        seed_choice(ui, lang, &mut form.random_seed, &mut form.seed);
        egui::Grid::new("mass-search")
            .num_columns(2)
            .show(ui, |ui| {
                let label = ui.label(lang.pick("Breite:", "Beam:"));
                ui.add(egui::DragValue::new(&mut form.beam).range(20..=2000))
                    .labelled_by(label.id);
                ui.end_row();
                let label = ui.label(lang.pick("Zufall:", "Noise:"));
                ui.add(
                    egui::DragValue::new(&mut form.noise)
                        .range(0.0..=10.0)
                        .speed(0.1),
                )
                .labelled_by(label.id);
                ui.end_row();
            });
        match &sketch.planned {
            Ok(line) => ui.label(line.as_str()),
            Err(error) => ui.colored_label(Color32::from_rgb(220, 50, 50), error.as_str()),
        };
        ui.add_space(8.0);
        let (mut plan, mut set, mut cancel) = (false, false, false);
        ui.horizontal(|ui| {
            plan = ui
                .add_enabled(
                    !busy,
                    egui::Button::new(lang.pick("Plan zeigen", "Show the plan")),
                )
                .on_hover_text(lang.pick(
                    "Den Plan der Messe ohne Suche: Sätze, Abschnitte und Teile, wie jeder \
                     beginnt und auf welcher Stufe er schließt, die geschätzte Zeit",
                    "The plan of the mass without a search: movements, sections and parts, how \
                     each begins and the degree it ends on, the estimated time",
                ))
                .clicked();
            set = ui
                .add_enabled(
                    !busy,
                    egui::Button::new(
                        RichText::new(lang.pick("Messe setzen", "Set the mass")).strong(),
                    ),
                )
                .clicked();
            cancel = ui
                .add_enabled(busy, egui::Button::new(lang.pick("Abbrechen", "Cancel")))
                .on_hover_text(lang.pick(
                    "Die Suche anhalten; die gesetzten Teile bleiben",
                    "Stop the search; the parts set stay",
                ))
                .clicked();
        });
        let actions = self
            .mass_run
            .as_ref()
            .map(|run| mass_progress(ui, lang, run, !self.mass_is_shown()))
            .unwrap_or_default();
        if plan {
            self.show_mass_plan();
        }
        if set {
            self.start_mass();
        }
        if cancel {
            self.cancel_mass();
        }
        if let Some(at) = actions.chosen {
            self.choose_part(at);
        }
        if actions.again {
            self.set_part_again();
        }
        if actions.missing {
            self.set_missing_parts();
        }
        if actions.show {
            self.show_mass();
        }
    }

    fn cantus_task(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let cf = &mut self.cf;
        ui.heading(lang.pick("Cantus firmus erzeugen", "Generate cantus firmi"));
        mode_choice(ui, "cf-mode", lang, &mut cf.mode);
        egui::Grid::new("cf-grid").num_columns(2).show(ui, |ui| {
            let label = ui.label(lang.pick("Länge:", "Length:"));
            ui.add(egui::DragValue::new(&mut cf.length).range(6..=20))
                .labelled_by(label.id);
            ui.end_row();
            let label = ui.label(lang.pick("Anzahl:", "Count:"));
            ui.add(egui::DragValue::new(&mut cf.count).range(1..=20))
                .labelled_by(label.id);
            ui.end_row();
        });
        voice_choice(
            ui,
            "cf-voice",
            lang.pick("Stimmlage:", "Voice:"),
            lang,
            &mut cf.voice_type,
        );
        seed_choice(ui, lang, &mut cf.random_seed, &mut cf.seed);
        ui.add_space(8.0);
        if ui
            .add_enabled(
                self.job.is_none(),
                egui::Button::new(RichText::new(lang.pick("Erzeugen", "Generate")).strong()),
            )
            .clicked()
        {
            self.start_cantus();
        }
        ui.separator();
        let selected = self
            .entries
            .get(self.selected)
            .filter(|e| e.score.voices.len() == 1)
            .map(|e| e.score.clone());
        if ui
            .add_enabled(
                selected.is_some(),
                egui::Button::new(lang.pick(
                    "Als Cantus für den Kontrapunkt",
                    "Use as cantus for counterpoint",
                )),
            )
            .clicked()
            && let Some(score) = selected
        {
            self.use_as_cantus(&score);
        }
    }

    /// Starts `fux cf` with the form.
    pub fn start_cantus(&mut self) {
        let seed = if self.cf.random_seed {
            time_seed()
        } else {
            self.cf.seed
        };
        self.cf.seed = seed;
        let form = self.cf.clone();
        let profile = self.profile.clone();
        let lang = self.lang;
        self.job = Some(Job::spawn(
            lang.pick("Erzeuge Cantus …", "Generating …").into(),
            move || jobs::cantus(&form, seed, &profile, lang),
        ));
    }

    /// Takes a generated cantus as the cantus of the counterpoint task.
    fn use_as_cantus(&mut self, score: &Score) {
        let notes: Vec<String> = score.voices[0]
            .pitches()
            .iter()
            .map(ToString::to_string)
            .collect();
        self.cp.source = SourceKind::Notes;
        self.cp.notes = notes.join(" ");
        self.cp.mode = Some(score.mode);
        self.cp.cf_voice = Some(score.voices[0].voice_type);
        self.tab = Tab::Counterpoint;
    }

    fn check_task(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let check = &mut self.check;
        ui.heading(lang.pick("Satz prüfen", "Check a score"));
        if let Some(texted) = &check.texted {
            // A result with its text: checked as it is, the notes cannot be edited.
            let mut edit = false;
            ui.group(|ui| {
                ui.label(match lang {
                    Lang::De => format!(
                        "Übernommener Satz mit Text: {} ({} Stimmen)",
                        texted.title, texted.voices
                    ),
                    Lang::En => format!(
                        "Score taken over with its text: {} ({} voices)",
                        texted.title, texted.voices
                    ),
                });
                ui.weak(lang.pick(
                    "Geprüft wird der Satz mit seinem Liedtext, wie er als MusicXML gespeichert \
                     wird.",
                    "The score is checked with its lyrics, as it is saved as MusicXML.",
                ));
                edit = ui
                    .button(lang.pick("Noten bearbeiten (ohne Text)", "Edit the notes (no text)"))
                    .on_hover_text(lang.pick(
                        "Die Noten zum Bearbeiten übernehmen; der Liedtext geht dabei verloren",
                        "Take the notes for editing; the lyrics are lost",
                    ))
                    .clicked();
            });
            if edit {
                check.texted = None;
                check.text.clear();
            }
        } else {
            let label = ui.label(lang.pick(
                "Cantus: Noten, fux:NAME, GABC- oder MusicXML-Datei",
                "Cantus: notes, fux:NAME, a GABC or MusicXML file",
            ));
            ui.add(
                egui::TextEdit::multiline(&mut check.cantus)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            )
            .labelled_by(label.id);
            optional_voice_choice(
                ui,
                "check-cf-voice",
                lang.pick("Stimmlage des Cantus:", "Voice of the cantus:"),
                lang,
                &mut check.cf_voice,
            );
        }
        optional_mode_choice(ui, "check-mode", lang, &mut check.mode);
        cadence_choice(
            ui,
            "check-cadences",
            lang,
            &mut check.cadences,
            &mut check.cadence_measures,
        );
        ui.horizontal(|ui| {
            let label = ui.label("Text:");
            ui.add(
                egui::TextEdit::singleline(&mut check.text)
                    .hint_text(
                        lang.pick("aus der Datei oder erkennen", "from the file or recognise"),
                    )
                    .desired_width(200.0),
            )
            .on_hover_text(lang.pick(
                "Text der Stimmen mit Liedtext (MusicXML): Abschnitt des Ordinariums (kyrie, \
                 gloria:1-5 …), eigener lateinischer Text oder none (ohne Textregeln); leer: der \
                 Text, den eine Datei von fux-kontrapunkt nennt, sonst den Abschnitt oder Bereich \
                 erkennen",
                "Text of the voices with lyrics (MusicXML): a section of the Ordinary (kyrie, \
                 gloria:1-5 …), an own Latin text or none (no text rules); empty: the text a file \
                 of fux-kontrapunkt names, otherwise recognise the section or range",
            ))
            .labelled_by(label.id);
        });
        ui.horizontal(|ui| {
            let label = ui.label(lang.pick("Wiederholung:", "Repetition:"));
            egui::ComboBox::from_id_salt("check-repeat")
                .selected_text(check.repeat.name(lang))
                .show_ui(ui, |ui| {
                    for repeat in [Repeat::Auto, Repeat::None, Repeat::Free] {
                        ui.selectable_value(&mut check.repeat, repeat, repeat.name(lang));
                    }
                })
                .response
                .on_hover_text(lang.pick(
                    "Textwiederholung, wie beim Erzeugen des Satzes eingestellt",
                    "Restating the text, as set when the score was made",
                ))
                .labelled_by(label.id);
        });
        if check.texted.is_none() {
            ui.separator();
            ui.label(lang.pick(
                "Weitere Stimmen (LilyPond oder Notennamen):",
                "Further voices (LilyPond or note names):",
            ));
            let mut remove = None;
            for (i, voice) in check.voices.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    voice_choice(ui, ("check-voice", i), "", lang, &mut voice.voice_type);
                    if ui
                        .button("-")
                        .on_hover_text(lang.pick("Stimme entfernen", "Remove voice"))
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
                ui.add(
                    egui::TextEdit::multiline(&mut voice.notes)
                        .desired_rows(2)
                        .desired_width(f32::INFINITY),
                )
                .on_hover_text(voice.voice_type.name(lang));
            }
            if let Some(i) = remove {
                check.voices.remove(i);
            }
            if ui.button(lang.pick("+ Stimme", "+ Voice")).clicked() {
                check.voices.push(CheckVoice {
                    voice_type: VoiceType::Soprano,
                    notes: String::new(),
                });
            }
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.job.is_none(),
                    egui::Button::new(RichText::new(lang.pick("Prüfen", "Check")).strong()),
                )
                .clicked()
            {
                self.start_check();
            }
            if ui
                .add_enabled(
                    self.job.is_none(),
                    egui::Button::new(lang.pick("Datei …", "File …")),
                )
                .on_hover_text(lang.pick(
                    "Einen Satz aus einer Jobdatei (TOML), aus MusicXML oder aus Humdrum (**kern, freier Satz) prüfen",
                    "Check a score from a job file (TOML), from MusicXML or from Humdrum (**kern, free counterpoint)",
                ))
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        lang.pick(
                            "Jobdatei, MusicXML oder Humdrum",
                            "Job file, MusicXML or Humdrum",
                        ),
                        &["toml", "musicxml", "xml", "krn"],
                    )
                    .pick_file()
            {
                self.start_job_file(path);
            }
            // A movement of a mass only where every part it prints is set, those of the Osanna
            // it repeats for an Osanna ut supra too: else it has no file.
            let takeable = self.selected < self.entries.len()
                && self
                    .shown_mass()
                    .is_none_or(|run| run.complete(self.selected));
            if ui
                .add_enabled(
                    takeable,
                    egui::Button::new(lang.pick("Ergebnis übernehmen", "Take the result")),
                )
                .on_hover_text(lang.pick(
                    "Den gezeigten Satz zum Bearbeiten übernehmen; einen Satz mit Text samt \
                     Liedtext zum Prüfen",
                    "Take the score shown for editing; a score with a text with its lyrics, for \
                     checking",
                ))
                .clicked()
            {
                match self.taken_result() {
                    Some(form) => self.check = form,
                    None => {
                        self.message = Some((
                            true,
                            lang.pick(
                                "Dieser Satz lässt sich nicht übernehmen: Ihm fehlt ein Teil.",
                                "This movement cannot be taken: it lacks a part.",
                            )
                            .to_string(),
                        ));
                    }
                }
            }
        });
    }

    /// The selected result as the check form ([`check_form_of`]); a movement of a mass with the
    /// file `fux mass` writes of it, so that the check takes it apart into its parts and judges
    /// each as its search did; a part of a mass checked from its file or from that of its
    /// movement with the fields of its file and its cadences with their degrees, so that the
    /// check judges it again as it did (D14). `None` for a movement of a mass that lacks a part.
    fn taken_result(&self) -> Option<CheckForm> {
        let entry = self.entries.get(self.selected)?;
        let mut form = check_form_of(&entry.score, entry.repeat);
        let musicxml = match self.shown_mass() {
            Some(run) => Some(run.movement_musicxml(
                self.selected,
                &self.output_args().musicxml_options(self.lang),
                &self.mass.notation(),
                self.lang,
            )?),
            None if !entry.fields.is_empty() => {
                form.cadence_measures = entry
                    .score
                    .cadences
                    .iter()
                    .map(|c| format!("{}:{}", c.measure + 1, c.target.letter()))
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(to_musicxml_with_fields(
                    &entry.score,
                    &MusicXmlOptions::default(),
                    &entry.fields,
                ))
            }
            None => None,
        };
        if let (Some(texted), Some(musicxml)) = (&mut form.texted, musicxml) {
            texted.musicxml = musicxml;
        }
        Some(form)
    }

    /// Starts `fux check` with the form.
    pub fn start_check(&mut self) {
        match self.check.job() {
            Ok(job) => self.spawn_check(job),
            Err(error) => self.message = Some((true, error)),
        }
    }

    /// Checks the score of a job file, of a MusicXML file or of a Humdrum file; lyrics with the
    /// text of the form.
    pub fn start_job_file(&mut self, path: PathBuf) {
        let extension = |name: &str| {
            path.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(name))
        };
        let musicxml = extension("musicxml") || extension("xml");
        let input = if extension("krn") {
            ScoreInput {
                kern: Some(path),
                ..ScoreInput::default()
            }
        } else if musicxml {
            ScoreInput {
                musicxml: Some(path),
                ..ScoreInput::default()
            }
        } else {
            ScoreInput {
                job: Some(path),
                ..ScoreInput::default()
            }
        };
        self.spawn_check(CheckJob {
            input,
            musicxml: None,
            text: self.check.text_spec(),
            repeat: self.check.repeat,
        });
    }

    fn spawn_check(&mut self, job: CheckJob) {
        let profile = self.profile.clone();
        let lang = self.lang;
        self.job = Some(Job::spawn(
            lang.pick("Prüfe …", "Checking …").into(),
            move || jobs::check(&job, &profile, lang),
        ));
    }

    fn examples_task(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        ui.heading(lang.pick("Fux' Beispiele", "Fux's examples"));
        ui.horizontal(|ui| {
            let label = ui.label(lang.pick("Suche:", "Search:"));
            ui.add(
                egui::TextEdit::singleline(&mut self.example_filter)
                    .hint_text(lang.pick("Figur, Modus, Gattung", "figure, mode, species"))
                    .desired_width(180.0),
            )
            .labelled_by(label.id);
        });
        let filter = self.example_filter.to_lowercase();
        let mut open = None;
        for example in fux_examples() {
            let species: Vec<String> = example
                .species
                .iter()
                .map(|s| s.number().to_string())
                .collect();
            let voices = example.score().voices.len();
            let label = format!(
                "Fig. {} – {}, {} {}, {}. {}",
                example.figure,
                example.mode.name(lang),
                voices,
                lang.pick("Stimmen", "voices"),
                species.join("+"),
                lang.pick("Gattung", "species")
            );
            if !filter.is_empty() && !label.to_lowercase().contains(&filter) {
                continue;
            }
            if ui.selectable_label(false, label.as_str()).clicked() {
                open = Some((example.score(), label));
            }
        }
        if let Some((mut score, label)) = open {
            score.title = Some(label.clone());
            fux_cli::commands::localize_names(&mut score, lang);
            let found = jobs::checked(score, &self.profile, lang, &label);
            self.show_found(found);
        }
    }

    // ---- results ----

    fn result_panel(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        for line in &self.summary {
            ui.label(line.as_str());
        }
        if !self.entries.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for (i, entry) in self.entries.iter().enumerate() {
                    if ui
                        .selectable_label(i == self.selected, entry.label.as_str())
                        .clicked()
                        && i != self.selected
                    {
                        self.selected = i;
                        self.audio.stop();
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.view, View::Notes, lang.pick("Noten", "Notes"));
                ui.selectable_value(&mut self.view, View::Text, "Text");
                ui.separator();
                let label = ui.label("Zoom:");
                ui.add(egui::Slider::new(&mut self.zoom, 0.3..=3.0).show_value(false))
                    .labelled_by(label.id);
                ui.checkbox(
                    &mut self.mark,
                    lang.pick("Verstöße markieren", "Mark violations"),
                );
                ui.checkbox(
                    &mut self.original_clefs,
                    lang.pick("Fux' Schlüssel", "Fux's clefs"),
                );
                ui.checkbox(
                    &mut self.figures,
                    lang.pick("Intervallziffern", "Interval figures"),
                );
                ui.checkbox(
                    &mut self.entry_marks,
                    lang.pick("Einsätze markieren", "Mark the entries"),
                )
                .on_hover_text(lang.pick(
                    "Klammern mit der Nummer des Punktes über den Einsätzen einer Imitation",
                    "Brackets with the number of the point over the entries of an imitation",
                ));
            });
            ui.separator();
        }
        egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
            let Some(entry) = self.entries.get(self.selected) else {
                ui.label(lang.pick(
                    "Links eine Aufgabe wählen und starten.",
                    "Choose a task on the left and start it.",
                ));
                return;
            };
            if self.view == View::Text || self.lilypond.is_none() {
                if self.lilypond.is_none() {
                    ui.colored_label(
                        Color32::from_rgb(200, 120, 0),
                        lang.pick(
                            "LilyPond nicht gefunden: Noten nur als Text (https://lilypond.org).",
                            "LilyPond not found: notes as text only (https://lilypond.org).",
                        ),
                    );
                }
                for (v, voice) in entry.score.voices.iter().enumerate() {
                    ui.label(RichText::new(voice.name.as_str()).strong());
                    ui.label(RichText::new(voice_text(voice, entry.score.meter)).monospace());
                    // What the voice sings, syllable by syllable.
                    if let Some(lyrics) = entry.score.voice_lyrics(v) {
                        ui.label(format!("Text: {lyrics}"));
                    }
                }
                return;
            }
            // A movement of a mass of which no section is set yet has no pages.
            if let Some(run) = self.shown_mass()
                && !run.has_notes(self.selected)
            {
                ui.label(lang.pick(
                    "Von diesem Satz ist noch kein Abschnitt gesetzt.",
                    "No section of this movement is set yet.",
                ));
                return;
            }
            if let Some(error) = &self.preview.error {
                ui.colored_label(Color32::RED, error.as_str());
            } else if !self.preview.has_pages() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(lang.pick("LilyPond setzt die Noten …", "LilyPond engraves …"));
                });
            }
            self.preview.show(ui, self.zoom);
        });
    }

    fn report_panel(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        if self.mass_is_shown() && self.selected < self.entries.len() {
            self.movement_report(ui);
            return;
        }
        let Some(entry) = self.entries.get(self.selected) else {
            ui.heading(lang.pick("Verstöße", "Violations"));
            return;
        };
        let report = &entry.report;
        let forbidden = report.forbidden();
        ui.heading(lang.pick("Verstöße", "Violations"));
        ui.strong(match lang {
            Lang::De => format!("{forbidden} Verbot(e), Kosten {}", report.cost()),
            Lang::En => format!("{forbidden} prohibition(s), cost {}", report.cost()),
        });
        ui.checkbox(
            &mut self.only_forbidden,
            lang.pick("nur Verbote", "prohibitions only"),
        );
        let checker = Checker::new(&self.profile);
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for v in report
                    .violations
                    .iter()
                    .filter(|v| !self.only_forbidden || v.is_forbidden())
                {
                    let color = if v.is_forbidden() {
                        Color32::from_rgb(220, 50, 50)
                    } else {
                        Color32::from_rgb(220, 140, 0)
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(v.level.label(lang)).color(color).strong());
                        ui.label(format!(
                            "{}, {}",
                            violation_position_after(&entry.score, v, entry.before, lang),
                            entry.score.voices[v.voice].name
                        ));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.weak(v.rule);
                        });
                    });
                    ui.indent(("violation", v.voice, v.note, v.rule, v.tick), |ui| {
                        ui.label(checker.message(v, lang));
                    });
                    ui.add_space(2.0);
                }
            });
    }

    /// The violations of the selected movement of the mass, part by part (D14): the prohibitions
    /// and cost of its parts together, the cadence plan of the movement, and for each part its
    /// heading with its measures in the movement, its prohibitions and cost, its warnings (a
    /// fallback, a first point without the head motif, a chant from another final) and its
    /// violations, their measures counted through the movement where every part before it is set;
    /// for a part not set why it is missing.
    fn movement_report(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let movement = self.selected;
        let only_forbidden = &mut self.only_forbidden;
        let Some(run) = self.mass_run.as_ref() else {
            return;
        };
        let reports = run.part_reports(movement, lang);
        let set: Vec<&Report> = reports
            .iter()
            .filter_map(|report| report.result.map(|result| &result.generated.report))
            .collect();
        let forbidden: usize = set.iter().map(|report| report.forbidden()).sum();
        let cost: u32 = set.iter().map(|report| report.cost()).sum();
        ui.heading(lang.pick("Verstöße", "Violations"));
        ui.strong(match lang {
            Lang::De => format!("{forbidden} Verbot(e), Kosten {cost}"),
            Lang::En => format!("{forbidden} prohibition(s), cost {cost}"),
        });
        ui.checkbox(
            only_forbidden,
            lang.pick("nur Verbote", "prohibitions only"),
        );
        if let Some(plan) = run.cadence_plan(movement, lang) {
            ui.weak(plan);
        }
        let only = *only_forbidden;
        let checker = Checker::new(&run.prepared.profile);
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for report in &reports {
                    ui.add_space(4.0);
                    ui.strong(report.heading.as_str());
                    let Some(result) = report.result else {
                        let why = run.why_missing(report.at, lang).unwrap_or_default();
                        ui.colored_label(Color32::from_rgb(220, 140, 0), why);
                        continue;
                    };
                    let (score, found) = (&result.generated.score, &result.generated.report);
                    ui.weak(report_line(found, lang));
                    for warning in run.warnings(report.at, lang) {
                        let warning = format!("{}: {warning}", lang.pick("Warnung", "Warning"));
                        ui.colored_label(Color32::from_rgb(220, 140, 0), warning);
                    }
                    let before = report.before.unwrap_or(0);
                    for v in found
                        .violations
                        .iter()
                        .filter(|v| !only || v.is_forbidden())
                    {
                        let color = if v.is_forbidden() {
                            Color32::from_rgb(220, 50, 50)
                        } else {
                            Color32::from_rgb(220, 140, 0)
                        };
                        let voice = report
                            .voices
                            .get(v.voice)
                            .cloned()
                            .unwrap_or_else(|| score.voices[v.voice].name.clone());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(v.level.label(lang)).color(color).strong());
                            ui.label(format!(
                                "{}, {voice}",
                                violation_position_after(score, v, before, lang)
                            ));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.weak(v.rule);
                                },
                            );
                        });
                        ui.indent(
                            ("mass-violation", report.at, v.voice, v.note, v.rule, v.tick),
                            |ui| {
                                ui.label(checker.message(v, lang));
                            },
                        );
                        ui.add_space(2.0);
                    }
                }
            });
    }

    // ---- bottom bar: playback, export, status ----

    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        ui.horizontal(|ui| {
            let has_entry = self.selected < self.entries.len();
            if self.audio.is_playing() {
                if ui.button(lang.pick("Stopp", "Stop")).clicked() {
                    self.audio.stop();
                }
            } else if ui
                .add_enabled(has_entry, egui::Button::new(lang.pick("Abspielen", "Play")))
                .clicked()
            {
                self.play();
            }
            let label = ui.label("Tempo:");
            ui.add(
                egui::DragValue::new(&mut self.tempo)
                    .range(20..=200)
                    .suffix(" /min"),
            )
            .labelled_by(label.id);
            let label = ui.label(lang.pick("Klang:", "Sound:"));
            egui::ComboBox::from_id_salt("program")
                .selected_text(program_name(self.program, lang))
                .show_ui(ui, |ui| {
                    for program in [53, 20, 1, 41, 72] {
                        ui.selectable_value(
                            &mut self.program,
                            program,
                            program_name(program, lang),
                        );
                    }
                })
                .response
                .labelled_by(label.id);
            if self.audio.is_loading() {
                ui.spinner();
                ui.label("SoundFont …");
            } else if let Some(error) = self.audio.error() {
                ui.colored_label(Color32::RED, error.to_string());
            }
            ui.separator();
            let label = ui.label(lang.pick("Ausgabe:", "Output:"));
            ui.add(egui::TextEdit::singleline(&mut self.output).desired_width(160.0))
                .labelled_by(label.id);
            if ui
                .button("…")
                .on_hover_text(lang.pick("Ziel wählen", "Choose a target"))
                .clicked()
                && let Some(path) = rfd::FileDialog::new().save_file()
            {
                self.output = path.with_extension("").display().to_string();
            }
            for format in ALL_FORMATS {
                let mut on = self.formats.contains(&format);
                if ui.checkbox(&mut on, format_name(format)).changed() {
                    if on {
                        self.formats.push(format);
                    } else {
                        self.formats.retain(|f| *f != format);
                    }
                }
            }
            ui.checkbox(&mut self.all_results, lang.pick("alle", "all"))
                .on_hover_text(
                    lang.pick("alle Ergebnisse in eine Datei", "all results in one file"),
                );
            if ui
                .add_enabled(
                    has_entry,
                    egui::Button::new(lang.pick("Exportieren", "Export")),
                )
                .clicked()
            {
                self.export();
            }
        });
        ui.horizontal(|ui| {
            let mass = self.mass_run.as_ref().and_then(|run| run.status(lang));
            if let Some(job) = &self.job {
                ui.spinner();
                ui.label(format!(
                    "{} {:.0} s",
                    job.label,
                    job.started.elapsed().as_secs_f32()
                ));
            } else if let Some(status) = mass {
                ui.spinner();
                if let Some((fraction, done, total)) =
                    self.mass_run.as_ref().and_then(MassRun::progress)
                {
                    ui.add(
                        egui::ProgressBar::new(fraction)
                            .desired_width(140.0)
                            .text(format!("{done}/{total} {}", lang.pick("Teile", "parts"))),
                    );
                }
                ui.label(status);
            } else if let Some((error, text)) = &self.message {
                let color = if *error {
                    Color32::from_rgb(220, 50, 50)
                } else {
                    ui.visuals().text_color()
                };
                ui.colored_label(color, text.as_str());
            }
        });
    }

    /// Plays the selected result, loading the SoundFont first if needed; a movement of a mass
    /// as `fux mass` writes its MIDI file, or the sections of it that are set.
    fn play(&mut self) {
        let Some(entry) = self.entries.get(self.selected) else {
            return;
        };
        let options = self.output_args().midi_options();
        let midi = match self.shown_mass() {
            Some(run) => run.movement_midi(self.selected, &options, self.lang),
            None => Some(to_midi(&entry.score, &options)),
        };
        let Some(midi) = midi else {
            self.message = Some((
                true,
                self.lang
                    .pick(
                        "Von diesem Satz ist noch kein Abschnitt gesetzt.",
                        "No section of this movement is set yet.",
                    )
                    .into(),
            ));
            return;
        };
        if !self.audio.is_ready() {
            if self.soundfont.is_empty() {
                self.message = Some((
                    true,
                    self.lang
                        .pick(
                            "Keine SoundFont gefunden; FUX_SOUNDFONT setzen.",
                            "No SoundFont found; set FUX_SOUNDFONT.",
                        )
                        .into(),
                ));
                return;
            }
            self.audio.load(std::path::Path::new(&self.soundfont));
            self.message = Some((
                false,
                self.lang
                    .pick(
                        "SoundFont wird geladen; gleich noch einmal abspielen.",
                        "Loading the SoundFont; press play again in a moment.",
                    )
                    .into(),
            ));
            return;
        }
        if let Err(error) = self.audio.play(&midi) {
            self.message = Some((true, error));
        }
    }

    /// Writes the selected result (or all) in the chosen formats; a mass whole: its book, the
    /// files of its movements and of its parts.
    fn export(&mut self) {
        let lang = self.lang;
        if self.formats.is_empty() {
            self.message = Some((
                true,
                lang.pick("Kein Format gewählt", "No format chosen")
                    .to_string(),
            ));
            return;
        }
        if let Some(run) = self.shown_mass() {
            let written = run.export(&self.output_args(), &self.mass.notation(), lang);
            self.message = Some(written_message(written, lang));
            return;
        }
        if self.all_results
            && let Some((workbook, exercises)) = &self.workbook
        {
            let parts = workbook.parts(*exercises, lang);
            let written = write_book(
                Workbook::title(lang),
                Some(&workbook.subtitle(lang)),
                &parts,
                &self.output_args(),
                "out/uebungsheft",
                lang,
            );
            self.message = Some(written_message(written, lang));
            return;
        }
        let checker = Checker::new(&self.profile);
        let chosen: Vec<&Entry> = if self.all_results {
            self.entries.iter().collect()
        } else {
            self.entries.get(self.selected).into_iter().collect()
        };
        let sheets: Vec<Sheet> = chosen
            .iter()
            .map(|e| entry_sheet(e, self.mark, &checker, lang))
            .collect();
        let title = self.summary.first().cloned();
        let written = write(
            title.as_deref(),
            &sheets,
            &self.output_args(),
            "out/satz",
            lang,
        );
        self.message = Some(written_message(written, lang));
    }
}

/// The sheet of a result, its violations marked where it is marked or `mark` asks for it, its
/// measures counted on from those before it (a part of a mass in the file of its movement), with
/// the seed it was set with.
fn entry_sheet<'a>(entry: &'a Entry, mark: bool, checker: &Checker, lang: Lang) -> Sheet<'a> {
    let sheet = if mark || entry.marked {
        marked_sheet_after(&entry.score, &entry.report, checker, entry.before, lang)
    } else {
        Sheet {
            first_bar: entry.before + 1,
            ..Sheet::new(&entry.score)
        }
    };
    Sheet {
        seed: entry.seed,
        ..sheet
    }
}

/// The status line after an export: the files, or the error.
fn written_message(written: anyhow::Result<Vec<PathBuf>>, lang: Lang) -> (bool, String) {
    match written {
        Ok(files) => {
            let names: Vec<String> = files.iter().map(|f| f.display().to_string()).collect();
            (
                false,
                format!(
                    "{}: {}",
                    lang.pick("Geschrieben", "Written"),
                    names.join(", ")
                ),
            )
        }
        Err(error) => (true, format!("{error:#}")),
    }
}

impl eframe::App for FuxApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.stored());
    }
}

// ---- choices ----

/// All modes: six finals, authentic or plagal, with or without the flat.
fn all_modes() -> Vec<ModeSpec> {
    let mut modes = Vec::new();
    for transposed in [false, true] {
        for mode in Mode::ALL {
            for ambitus in [Ambitus::Authentic, Ambitus::Plagal] {
                modes.push(
                    ModeSpec::new(mode)
                        .with_ambitus(ambitus)
                        .with_transposition(transposed),
                );
            }
        }
    }
    modes
}

fn mode_choice(ui: &mut egui::Ui, id: &str, lang: Lang, mode: &mut ModeSpec) {
    ui.horizontal(|ui| {
        let label = ui.label(lang.pick("Modus:", "Mode:"));
        egui::ComboBox::from_id_salt(id)
            .selected_text(mode.name(lang))
            .show_ui(ui, |ui| {
                for m in all_modes() {
                    ui.selectable_value(mode, m, m.name(lang));
                }
            })
            .response
            .labelled_by(label.id);
    });
}

fn optional_mode_choice(ui: &mut egui::Ui, id: &str, lang: Lang, mode: &mut Option<ModeSpec>) {
    let automatic = lang.pick("aus dem Cantus", "from the cantus");
    ui.horizontal(|ui| {
        let label = ui.label(lang.pick("Modus:", "Mode:"));
        egui::ComboBox::from_id_salt(id)
            .selected_text(mode.map_or(automatic.to_string(), |m| m.name(lang)))
            .show_ui(ui, |ui| {
                ui.selectable_value(mode, None, automatic);
                for m in all_modes() {
                    ui.selectable_value(mode, Some(m), m.name(lang));
                }
            })
            .response
            .labelled_by(label.id);
    });
}

fn voice_choice(
    ui: &mut egui::Ui,
    id: impl Hash + std::fmt::Debug,
    label: &str,
    lang: Lang,
    voice: &mut VoiceType,
) {
    ui.horizontal(|ui| {
        let label = ui.label(if label.is_empty() {
            lang.pick("Stimme:", "Voice:")
        } else {
            label
        });
        egui::ComboBox::from_id_salt(id)
            .selected_text(voice.name(lang))
            .show_ui(ui, |ui| {
                for v in VoiceType::ALL {
                    ui.selectable_value(voice, v, v.name(lang));
                }
            })
            .response
            .labelled_by(label.id);
    });
}

fn optional_voice_choice(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    lang: Lang,
    voice: &mut Option<VoiceType>,
) {
    let automatic = lang.pick("automatisch", "automatic");
    ui.horizontal(|ui| {
        let label = ui.label(label);
        egui::ComboBox::from_id_salt(id)
            .selected_text(voice.map_or(automatic, |v| v.name(lang)))
            .show_ui(ui, |ui| {
                ui.selectable_value(voice, None, automatic);
                for v in VoiceType::ALL {
                    ui.selectable_value(voice, Some(v), v.name(lang));
                }
            })
            .response
            .labelled_by(label.id);
    });
}

fn cadence_choice(
    ui: &mut egui::Ui,
    id: &str,
    lang: Lang,
    choice: &mut CadenceChoice,
    measures: &mut String,
) {
    ui.horizontal(|ui| {
        let label = ui.label(lang.pick("Binnenschlüsse:", "Internal cadences:"));
        egui::ComboBox::from_id_salt(id)
            .selected_text(choice.label(lang))
            .show_ui(ui, |ui| {
                for c in CadenceChoice::ALL {
                    ui.selectable_value(choice, c, c.label(lang));
                }
            })
            .response
            .labelled_by(label.id);
    });
    if *choice == CadenceChoice::Measures {
        ui.horizontal(|ui| {
            let label = ui.label(lang.pick("Takte:", "Measures:"));
            ui.add(
                egui::TextEdit::singleline(measures)
                    .hint_text("8, 15")
                    .desired_width(120.0),
            )
            .labelled_by(label.id);
        });
    }
}

/// The imitation of the free form (phase 13): whether the voices imitate each other, where the
/// subjects come from, and the choices of the points and their entries, each `auto` unless set;
/// a button that plans it without a search. Returns whether the button was clicked. `imitating`
/// says whether the form imitates as it is set, `section` the section of the Ordinary its voices
/// sing, for the chant of the Kyriale.
fn imitation_group(
    ui: &mut egui::Ui,
    lang: Lang,
    form: &mut ImitationForm,
    imitating: bool,
    section: Option<&str>,
    voices: usize,
    idle: bool,
) -> bool {
    ui.heading("Imitation");
    ui.horizontal(|ui| {
        let label = ui.label("Imitation:");
        egui::ComboBox::from_id_salt("free-imitation")
            .selected_text(form.mode.label(lang))
            .show_ui(ui, |ui| {
                for choice in Switch::ALL {
                    ui.selectable_value(&mut form.mode, choice, choice.label(lang));
                }
            })
            .response
            .on_hover_text(lang.pick(
                "auto: an mit Text, drei bis fünf Stimmen, automatischen Binnenschlüssen und automatischer Länge",
                "auto: on with a text, three to five voices, automatic internal cadences and an automatic length",
            ))
            .labelled_by(label.id);
        ui.weak(if imitating {
            lang.pick("(imitiert)", "(imitates)")
        } else {
            lang.pick("(imitiert nicht)", "(does not imitate)")
        });
    });
    if form.mode == Switch::Off {
        return false;
    }
    ui.horizontal(|ui| {
        let label = ui.label("Soggetto:");
        egui::ComboBox::from_id_salt("free-subject")
            .selected_text(form.subject.label(lang))
            .show_ui(ui, |ui| {
                for kind in SubjectKind::ALL {
                    ui.selectable_value(&mut form.subject, kind, kind.label(lang));
                }
            })
            .response
            .labelled_by(label.id);
    });
    match form.subject {
        SubjectKind::Text => {}
        SubjectKind::Kyriale => {
            let chant = section.and_then(Kyriale::for_section).map(Kyriale::chant);
            match chant {
                Some(chant) => {
                    let mode = fux_cli::imitation::chant_mode(&chant)
                        .map(|m| format!(" ({})", m.name(lang)))
                        .unwrap_or_default();
                    ui.label(format!("{}{mode}", chant.name.as_deref().unwrap_or("?")));
                }
                None => {
                    ui.colored_label(
                        Color32::from_rgb(220, 50, 50),
                        lang.pick(
                            "Für diesen Text hat das Kyriale keinen Choral.",
                            "The Kyriale has no chant for this text.",
                        ),
                    );
                }
            }
        }
        SubjectKind::File => {
            ui.horizontal(|ui| {
                let label = ui.label(lang.pick("Datei:", "File:"));
                ui.add(egui::TextEdit::singleline(&mut form.chant_file).desired_width(200.0))
                    .labelled_by(label.id);
                if ui
                    .button("…")
                    .on_hover_text(lang.pick("Datei wählen", "Choose a file"))
                    .clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("GABC", &["gabc"])
                        .pick_file()
                {
                    form.chant_file = path.display().to_string();
                }
            });
        }
        SubjectKind::Notes => {
            let label = ui.label(lang.pick(
                "Köpfe in LilyPond, | zwischen den Punkten:",
                "Heads in LilyPond, | between the points:",
            ));
            ui.add(
                egui::TextEdit::multiline(&mut form.notes)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .hint_text("d'1 d'2 a'2 g'4 f'4 e'2 d'1 | a'1 a'2 g'2 f'2 e'2 d'1"),
            )
            .labelled_by(label.id);
        }
    }
    if form.chant_subject() {
        ui.horizontal(|ui| {
            let label = ui.label(lang.pick("Phrasen:", "Phrases:"));
            ui.add(
                egui::TextEdit::singleline(&mut form.phrases)
                    .hint_text(lang.pick("nach den Worten", "by the words"))
                    .desired_width(120.0),
            )
            .on_hover_text(lang.pick(
                "Die Phrasen des Chorals für die Punkte, ab 1, z. B. 2, 3; leer: wo seine Worte stehen",
                "The chant's phrases for the points, from 1, such as 2, 3; empty: where its words are",
            ))
            .labelled_by(label.id);
        });
        ui.checkbox(
            &mut form.chant_mode,
            lang.pick("im Modus des Chorals", "in the chant's mode"),
        )
        .on_hover_text(lang.pick(
            "Sonst im gewählten Modus: die Stufen des Chorals von dessen Finalis aus",
            "Otherwise in the mode chosen: the chant's degrees from its final",
        ));
    }
    egui::CollapsingHeader::new(lang.pick("Punkte und Einsätze", "Points and entries"))
        .id_salt("free-entries")
        .show(ui, |ui| {
            egui::Grid::new("free-imitation-grid")
                .num_columns(2)
                .show(ui, |ui| {
                    let auto = |n: usize| {
                        if n == 0 {
                            "auto".to_string()
                        } else {
                            n.to_string()
                        }
                    };
                    let label = ui.label(lang.pick("Punkte:", "Points:"));
                    egui::ComboBox::from_id_salt("free-points")
                        .selected_text(auto(form.points))
                        .show_ui(ui, |ui| {
                            for n in 0..=6 {
                                ui.selectable_value(&mut form.points, n, auto(n));
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    ui.end_row();
                    let label = ui.label(lang.pick(
                        "Stimmen späterer Punkte:",
                        "Voices of later points:",
                    ));
                    egui::ComboBox::from_id_salt("free-later-voices")
                        .selected_text(form.later_voices.label(lang))
                        .show_ui(ui, |ui| {
                            // Two voices up to all the choir has.
                            let choices = [LaterVoices::Auto, LaterVoices::All]
                                .into_iter()
                                .chain((2..=voices.max(2)).map(LaterVoices::Count));
                            for choice in choices {
                                ui.selectable_value(
                                    &mut form.later_voices,
                                    choice,
                                    choice.label(lang),
                                );
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    ui.end_row();
                    for (label, field, hint, help) in [
                        (
                            lang.pick("Reihenfolge:", "Order:"),
                            &mut form.order,
                            "A S T B; T B",
                            lang.pick(
                                "Die Stimmen jedes Punktes in der Folge ihrer Einsätze, der Führer zuerst; ; trennt die Punkte; T1, T2 bei gleichen Stimmlagen",
                                "The voices of each point in the order of their entries, the leader first; ; separates the points; T1, T2 for voices of one type",
                            ),
                        ),
                        (
                            lang.pick("Intervalle:", "Intervals:"),
                            &mut form.intervals,
                            "d 0 d; 0",
                            lang.pick(
                                "Die späteren Einsätze zum Führer: 0 gleiche Stufe, d andere Hauptstufe (Quinte, Quarte), +n/-n Stufen",
                                "The later entries against the leader: 0 the same degree, d the other main degree (fifth, fourth), +n/-n steps",
                            ),
                        ),
                        (
                            lang.pick("Abstände:", "Delays:"),
                            &mut form.delays,
                            "1 2 1.5; 1",
                            lang.pick(
                                "Von Einsatz zu Einsatz in Takten, Halbe erlaubt",
                                "From entry to entry in measures, halves allowed",
                            ),
                        ),
                    ] {
                        let label = ui.label(label);
                        ui.add(
                            egui::TextEdit::singleline(field)
                                .hint_text(hint)
                                .desired_width(140.0),
                        )
                        .on_hover_text(help)
                        .labelled_by(label.id);
                        ui.end_row();
                    }
                    for (label, id, field, help) in [
                        (
                            lang.pick("Paare:", "Pairs:"),
                            "free-pairs",
                            &mut form.pairs,
                            lang.pick(
                                "Ab vier Stimmen: der 3. und 4. Einsatz wiederholen den 1. und 2. eine Oktave tiefer oder höher",
                                "From four voices: the 3rd and 4th entries repeat the 1st and 2nd an octave lower or higher",
                            ),
                        ),
                        (
                            lang.pick("Punkt 1 wiederholen:", "Restate point 1:"),
                            "free-restate",
                            &mut form.restate,
                            lang.pick(
                                "Den zweiten Durchgang eines kurzen Textes als Wiederholung von Punkt 1 in engerer Engführung",
                                "The second statement of a short text as point 1 restated in closer stretto",
                            ),
                        ),
                    ] {
                        let label = ui.label(label);
                        egui::ComboBox::from_id_salt(id)
                            .selected_text(field.label(lang))
                            .show_ui(ui, |ui| {
                                for choice in Switch::ALL {
                                    ui.selectable_value(field, choice, choice.label(lang));
                                }
                            })
                            .response
                            .on_hover_text(help)
                            .labelled_by(label.id);
                        ui.end_row();
                    }
                    let label = ui.label(lang.pick("Antwort:", "Answer:"));
                    egui::ComboBox::from_id_salt("free-answer")
                        .selected_text(form.answer.label())
                        .show_ui(ui, |ui| {
                            for choice in AnswerKind::ALL {
                                ui.selectable_value(&mut form.answer, choice, choice.label());
                            }
                        })
                        .response
                        .on_hover_text(lang.pick(
                            "real, bisweilen modal (Quarte und Quinte getauscht)",
                            "real, now and then modal (fourth and fifth exchanged)",
                        ))
                        .labelled_by(label.id);
                    ui.end_row();
                    let label = ui.label(lang.pick("Kopf:", "Head:"));
                    let notes = |n: usize| {
                        if n == 0 {
                            "auto".to_string()
                        } else {
                            match lang {
                                Lang::De => format!("{n} Töne"),
                                Lang::En => format!("{n} notes"),
                            }
                        }
                    };
                    egui::ComboBox::from_id_salt("free-head")
                        .selected_text(notes(form.head))
                        .show_ui(ui, |ui| {
                            for n in [0, 4, 5, 6, 7, 8, 9, 10] {
                                ui.selectable_value(&mut form.head, n, notes(n));
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    ui.end_row();
                });
        });
    if let Err(error) = form.options(lang) {
        ui.colored_label(Color32::from_rgb(220, 50, 50), error);
    }
    ui.add_enabled(
        idle && imitating,
        egui::Button::new(lang.pick("Plan zeigen", "Show the plan")),
    )
    .on_hover_text(lang.pick(
        "Die Punkte, ihre Köpfe und Einsätze planen, ohne Suche",
        "Plan the points, their heads and entries, without a search",
    ))
    .clicked()
}

/// The fields of a form that choose a text and how its voices sing it.
struct Singing<'a> {
    /// What tells the widgets of the forms apart: `free`, `cp`.
    id: &'static str,
    /// Who sings the text, for the help of the choice, in German and in English: „alle Stimmen
    /// singen“, "every voice sings".
    who: (&'static str, &'static str),
    text: &'a mut String,
    own_text: &'a mut String,
    repeat: &'a mut Repeat,
    accents: &'a mut bool,
    eleison3: &'a mut bool,
}

/// What a form knows of its text before the search: the text, if one is chosen, its warnings,
/// and a line on the length of the setting.
struct Sung<'a> {
    text: Option<&'a Text>,
    warnings: &'a [String],
    length: String,
}

/// The text of a setting: a section of the Ordinary, of which some sense units may be chosen, an
/// own Latin text with a preview, or none; and how it is sung: repetition, accents and the
/// division of eleison.
fn text_choice(
    ui: &mut egui::Ui,
    lang: Lang,
    form: Singing<'_>,
    preview: Result<Sung<'_>, &String>,
) {
    let Singing {
        id,
        who,
        text: chosen,
        own_text,
        repeat,
        accents,
        eleison3,
    } = form;
    let eleison = if *eleison3 {
        Eleison::Three
    } else {
        Eleison::Four
    };
    let sections = ordinary::sections(&TextOptions { eleison });
    // The section a text names, with or without a range of its sense units.
    let section_of = |spec: &str| {
        let id = spec.split(':').next().unwrap_or_default().trim();
        sections.iter().find(|s| s.id.eq_ignore_ascii_case(id))
    };
    let section = section_of(chosen);
    let own = lang.pick("Eigener Text …", "Own text …");
    let none = lang.pick("ohne Text", "no text");
    let selected = match (chosen.as_str(), &preview, section) {
        (OWN_TEXT, _, _) => own.to_string(),
        (NO_TEXT, _, _) => none.to_string(),
        (
            _,
            Ok(Sung {
                text: Some(text), ..
            }),
            _,
        ) => section_label(text, lang),
        (_, _, Some(section)) => section_label(section, lang),
        (spec, _, _) => spec.to_string(),
    };
    ui.horizontal(|ui| {
        let label = ui.label("Text:");
        egui::ComboBox::from_id_salt(format!("{id}-text"))
            .selected_text(selected)
            .width(240.0)
            .height(520.0)
            .show_ui(ui, |ui| {
                // First what is no section, so that it shows without scrolling.
                ui.selectable_value(chosen, OWN_TEXT.to_string(), own);
                ui.selectable_value(chosen, NO_TEXT.to_string(), none);
                ui.separator();
                for s in sections {
                    let picked = section.is_some_and(|c| c.id == s.id);
                    if ui
                        .selectable_label(picked, section_label(s, lang))
                        .clicked()
                    {
                        *chosen = s.id.clone();
                    }
                }
            })
            .response
            .on_hover_text(match lang {
                Lang::De => format!(
                    "Was {}: ein Abschnitt des Ordinariums nach dem Graduale Romanum, ein eigener \
                     lateinischer Text oder kein Text",
                    who.0
                ),
                Lang::En => format!(
                    "What {}: a section of the Ordinary after the Graduale Romanum, an own Latin \
                     text, or no text",
                    who.1
                ),
            })
            .labelled_by(label.id);
    });
    // Some sense units of a section, such as gloria:1-5.
    if let Some(section) = section_of(chosen)
        && section.units.len() > 1
    {
        let units = section.units.len();
        let (mut from, mut to) = unit_range(chosen, units);
        ui.horizontal(|ui| {
            let label = ui.label(lang.pick("Sinnabschnitte:", "Sense units:"));
            ui.add(egui::DragValue::new(&mut from).range(1..=to))
                .labelled_by(label.id);
            let label = ui.label(lang.pick("bis", "to"));
            ui.add(egui::DragValue::new(&mut to).range(from..=units))
                .labelled_by(label.id);
        });
        *chosen = if (from, to) == (1, units) {
            section.id.clone()
        } else if from == to {
            format!("{}:{from}", section.id)
        } else {
            format!("{}:{from}-{to}", section.id)
        };
    }
    if *chosen == OWN_TEXT {
        let label = ui.label(lang.pick("Eigener lateinischer Text:", "Own Latin text:"));
        ui.add(
            egui::TextEdit::multiline(own_text)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .hint_text("Ad te le-vá-vi á-ni-mam me-am."),
        )
        .on_hover_text(lang.pick(
            "Bindestriche zwischen den Silben, wo die Silbentrennung sie nicht findet; die \
             Betonung mit Akut (á é í ó ú); Satzzeichen beenden Sinnabschnitte; | markiert eine \
             Zäsur, / einen Schlussteil, der für sich wiederholt werden darf",
            "Hyphens between the syllables where the syllabifier does not find them; the stress \
             with an acute (á é í ó ú); punctuation ends sense units; | marks a break, / a tail \
             that may be restated on its own",
        ))
        .labelled_by(label.id);
    }
    if *chosen == NO_TEXT {
        return;
    }
    match preview {
        Ok(Sung {
            text: Some(text),
            warnings,
            length,
        }) => {
            let shown = *accents;
            let show = |ui: &mut egui::Ui| {
                text_preview(ui, text, warnings, &length, shown, lang);
            };
            if *chosen == OWN_TEXT {
                show(ui);
            } else {
                egui::CollapsingHeader::new(
                    lang.pick("Silben und Betonungen", "Syllables and stresses"),
                )
                .id_salt(format!("{id}-text-preview"))
                .show(ui, show);
            }
        }
        Ok(_) => {}
        Err(error) => {
            ui.colored_label(Color32::from_rgb(220, 50, 50), error.as_str());
        }
    }
    singing_choice(ui, id, lang, repeat, accents, eleison3);
}

/// How the voices sing their text: how freely they restate it, whether the syllables keep the
/// accents of the books, and the division of eleison. `id` tells the widgets of the forms apart.
fn singing_choice(
    ui: &mut egui::Ui,
    id: &str,
    lang: Lang,
    repeat: &mut Repeat,
    accents: &mut bool,
    eleison3: &mut bool,
) {
    ui.horizontal(|ui| {
        let label = ui.label(lang.pick("Wiederholung:", "Repetition:"));
        egui::ComboBox::from_id_salt(format!("{id}-repeat"))
            .selected_text(repeat.name(lang))
            .show_ui(ui, |ui| {
                for choice in [Repeat::Auto, Repeat::None, Repeat::Free] {
                    ui.selectable_value(repeat, choice, choice.name(lang));
                }
            })
            .response
            .on_hover_text(lang.pick(
                "Textwiederholung: auto (Kyrie, Sanctus, Agnus frei; Gloria, Credo selten), \
                 keine, frei",
                "Restating the text: auto (free in Kyrie, Sanctus, Agnus; rare in Gloria, \
                 Credo), none, free",
            ))
            .labelled_by(label.id);
    });
    ui.horizontal(|ui| {
        ui.checkbox(accents, lang.pick("Akzente zeigen", "Show accents"))
            .on_hover_text(lang.pick(
                "Die Akzente der liturgischen Bücher unter den Noten (Ký-ri-e)",
                "The accents of the liturgical books under the notes (Ký-ri-e)",
            ));
        ui.checkbox(eleison3, "e-léi-son").on_hover_text(lang.pick(
            "eleison mit 3 Silben (Palestrina nach Jeppesen) statt 4 (Bücher)",
            "eleison in 3 syllables (Palestrina after Jeppesen) instead of 4 (books)",
        ));
    });
}

/// The table of the sections of the mass form: for each its voices, whether its parts begin in
/// imitation and the degree it ends on, each as in Palestrina's masses unless set (`fux mass
/// --scoring`, `--imitation`, `--endings` with the section's id); then changes of single parts
/// as `--imitation` and `--endings` take them. A section that ends on the final offers no
/// ending.
fn section_table(ui: &mut egui::Ui, lang: Lang, form: &mut MassForm, rows: &[SectionRow]) {
    ui.weak(lang.pick(
        "Leer und „auto“: wie in Palestrinas Messen; Stimmen von oben, etwa S A T.",
        "Empty and “auto”: as in Palestrina's masses; voices from the top, such as S A T.",
    ));
    egui::Grid::new("mass-section-grid")
        .num_columns(4)
        .striped(true)
        .show(ui, |ui| {
            ui.strong(lang.pick("Abschnitt", "Section"));
            ui.strong(lang.pick("Stimmen", "Voices"));
            ui.strong("Imitation");
            ui.strong(lang.pick("Schluss", "Ending"));
            ui.end_row();
            for row in rows {
                let before = form
                    .section(row.id)
                    .cloned()
                    .unwrap_or_else(|| SectionChoice {
                        id: row.id.to_string(),
                        ..SectionChoice::default()
                    });
                let mut choice = before.clone();
                let label = ui.label(row.title);
                ui.add(
                    egui::TextEdit::singleline(&mut choice.voices)
                        .hint_text(row.voices.as_str())
                        .desired_width(64.0),
                )
                .labelled_by(label.id);
                egui::ComboBox::from_id_salt(("mass-imitation", row.id))
                    .selected_text(choice.imitation.label(lang))
                    .width(56.0)
                    .show_ui(ui, |ui| {
                        for switch in Switch::ALL {
                            ui.selectable_value(&mut choice.imitation, switch, switch.label(lang));
                        }
                    })
                    .response
                    .labelled_by(label.id);
                if row.endings.is_empty() {
                    choice.ending.clear();
                    ui.weak(lang.pick("Finalis", "final"));
                } else {
                    let shown = match row.endings.iter().find(|(l, _)| *l == choice.ending) {
                        Some((_, name)) => name.clone(),
                        None if choice.ending.is_empty() => "auto".to_string(),
                        None => choice.ending.clone(),
                    };
                    egui::ComboBox::from_id_salt(("mass-ending", row.id))
                        .selected_text(shown)
                        .width(56.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut choice.ending, String::new(), "auto");
                            for (letter, name) in &row.endings {
                                ui.selectable_value(&mut choice.ending, letter.clone(), name);
                            }
                        })
                        .response
                        .labelled_by(label.id);
                }
                ui.end_row();
                if choice != before {
                    *form.section_mut(row.id) = choice;
                }
            }
        });
    form.sections.retain(|choice| !choice.is_empty());
    egui::Grid::new("mass-single-parts")
        .num_columns(2)
        .show(ui, |ui| {
            let label = ui.label(lang.pick("Imitation (Teile):", "Imitation (parts):"));
            ui.add(
                egui::TextEdit::singleline(&mut form.imitation)
                    .hint_text("credo/3=aus; incarnatus=an")
                    .desired_width(160.0),
            )
            .on_hover_text(lang.pick(
                "Wie fux mass --imitation: Stelle=auto, an oder aus, durch ; getrennt; eine \
                 Stelle ist ein Teil wie credo/3 (der dritte Teil des Credo, wie der Plan sie \
                 zählt), ein Abschnitt oder ein Halteklang wie incarnatus",
                "As fux mass --imitation: place=auto, an (on) or aus (off), separated by ;; a \
                 place is a part such as credo/3 (the third part of the Credo, as the plan \
                 counts them), a section or a held chord such as incarnatus",
            ))
            .labelled_by(label.id);
            ui.end_row();
            let label = ui.label(lang.pick("Schlüsse (Teile):", "Endings (parts):"));
            ui.add(
                egui::TextEdit::singleline(&mut form.endings)
                    .hint_text("gloria/3=D")
                    .desired_width(160.0),
            )
            .on_hover_text(lang.pick(
                "Wie fux mass --endings: Stelle=Stufe, durch ; getrennt; die Finalis, die Quinte \
                 oder die Quarte, wo der Modus dort eine Klausel hat",
                "As fux mass --endings: place=degree, separated by ;; the final, the fifth or \
                 the fourth where the mode has a clausula there",
            ))
            .labelled_by(label.id);
            ui.end_row();
        });
}

/// What the user asked of the mass in the list of its parts.
#[derive(Default)]
struct MassActions {
    /// A part chosen in the list.
    chosen: Option<PartRef>,
    /// „Teil neu setzen“.
    again: bool,
    /// „Fehlende Teile setzen“.
    missing: bool,
    /// „Ergebnis zeigen“.
    show: bool,
}

/// Where the mass stands: while parts are set, a bar and the line on the parts being searched
/// with the time the mass still takes; the list of its parts with where each stands, a part set
/// with warnings (such as a first point without the head motif) in amber with them on hover, in
/// which a part is chosen; the buttons that set the chosen part again and the missing parts, and,
/// where other results are shown (`hidden`), the button that shows the mass again.
fn mass_progress(ui: &mut egui::Ui, lang: Lang, run: &MassRun, hidden: bool) -> MassActions {
    let red = Color32::from_rgb(220, 50, 50);
    let amber = Color32::from_rgb(220, 140, 0);
    let mut actions = MassActions::default();
    if let Some((fraction, done, total)) = run.progress() {
        ui.add(
            egui::ProgressBar::new(fraction)
                .text(format!("{done}/{total} {}", lang.pick("Teile", "parts"))),
        );
        if let Some(status) = run.status(lang) {
            ui.label(status);
        }
    }
    if let Some(error) = &run.error {
        ui.colored_label(red, error.as_str());
    }
    ui.separator();
    let parts = run.mass.results.len();
    let title = match lang {
        Lang::De => format!("Teile ({} von {parts} gesetzt)", run.set_count()),
        Lang::En => format!("Parts ({} of {parts} set)", run.set_count()),
    };
    egui::CollapsingHeader::new(title)
        .id_salt("mass-parts")
        .default_open(true)
        .show(ui, |ui| {
            for ((at, name, how), state) in run.part_lines(lang).into_iter().zip(&run.states) {
                if ui.selectable_label(run.chosen == Some(at), name).clicked() {
                    actions.chosen = Some(at);
                }
                ui.indent(("mass-part", at), |ui| match state {
                    PartState::Failed(error) => {
                        ui.colored_label(red, how)
                            .on_hover_text(failure(error, lang));
                    }
                    PartState::Cancelled if run.mass.result(at).is_none() => {
                        ui.colored_label(amber, how);
                    }
                    PartState::Running { .. } => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(how);
                        });
                    }
                    _ => {
                        let warnings = run.warnings(at, lang);
                        if warnings.is_empty() {
                            ui.weak(how);
                        } else {
                            ui.colored_label(amber, how)
                                .on_hover_text(warnings.join("\n"));
                        }
                    }
                });
            }
        });
    let idle = !run.is_running();
    ui.horizontal_wrapped(|ui| {
        actions.again = ui
            .add_enabled(
                idle && run.chosen.is_some(),
                egui::Button::new(lang.pick("Teil neu setzen", "Set the part again")),
            )
            .on_hover_text(lang.pick(
                "Den gewählten Teil mit einem neuen Seed setzen; findet keiner seiner Versuche \
                 einen Satz, bleibt er, wie er war. Mit dem Kyrie I einer Messe mit Kopfmotiv \
                 auch die Teile, die mit dem Kopfmotiv beginnen, wie fux mass --only kyrie/1",
                "Set the chosen part with a new seed; where none of its attempts finds a \
                 setting, it stays as it was. With Kyrie I of a mass with a head motif also the \
                 parts that open with the head motif, as fux mass --only kyrie/1 does",
            ))
            .clicked();
        actions.missing = ui
            .add_enabled(
                idle && !run.resumable().is_empty(),
                egui::Button::new(lang.pick("Fehlende Teile setzen", "Set the missing parts")),
            )
            .on_hover_text(lang.pick(
                "Die Teile, deren Suche abgebrochen wurde oder noch nicht lief, mit den Seeds des \
                 Plans und den Angaben setzen, mit denen die Messe gesetzt wurde, wie fux mass \
                 --resume mit denselben Angaben; Änderungen im Formular gelten ab dem nächsten \
                 „Messe setzen“. Einen gescheiterten Teil nicht: Mit seinem Seed scheiterte er \
                 wieder.",
                "Set the parts whose search was cancelled or has not run, with the seeds of the \
                 plan and the options the mass was set with, as fux mass --resume with the same \
                 options does; changes in the form apply from the next “Set the mass” on. Not a \
                 part that failed: with its seed it would fail again.",
            ))
            .clicked();
        if hidden {
            actions.show = ui
                .button(lang.pick("Ergebnis zeigen", "Show the result"))
                .on_hover_text(lang.pick(
                    "Die Sätze dieser Messe wieder als Ergebnisse zeigen",
                    "Show the movements of this mass as the results again",
                ))
                .clicked();
        }
    });
    actions
}

/// The lines over the results of a mass: the first two of its plan (title, mode, choir, seed
/// and counts; the estimate) and, once a part is set or no job sets parts, the totals, the
/// warnings and what is missing.
fn mass_lines(run: &MassRun, lang: Lang) -> Vec<String> {
    let mut lines: Vec<String> = run
        .summary(lang)
        .lines()
        .take(2)
        .map(str::to_string)
        .collect();
    if run.set_count() > 0 || !run.is_running() {
        lines.extend(run.totals(lang));
    }
    lines
}

/// The status line when the job of a mass has ended, and whether it tells of an error: the mass
/// set, or incomplete with how to go on — „Fehlende Teile setzen“ for parts cancelled or not
/// set yet, „Teil neu setzen“ for those whose search failed, named, which the same seed would
/// fail again; a part that could not be set again stays as it was.
fn mass_ended(run: &MassRun, lang: Lang) -> (bool, String) {
    if let Some(error) = &run.error {
        return (true, error.clone());
    }
    if let Some((at, error)) = &run.kept {
        let address = run.plan().address(*at);
        let why = failure(error, lang);
        return (
            true,
            match lang {
                Lang::De => format!("Teil {address} bleibt, wie er war. {why}"),
                Lang::En => format!("Part {address} stays as it was. {why}"),
            },
        );
    }
    let (set, planned) = (run.set_count(), run.mass.results.len());
    if set == planned {
        return (
            false,
            match lang {
                Lang::De => format!("Messe gesetzt: {set} von {planned} Teilen."),
                Lang::En => format!("Mass set: {set} of {planned} parts."),
            },
        );
    }
    let failed: Vec<String> = run
        .failed()
        .into_iter()
        .map(|at| run.plan().address(at))
        .collect();
    let failed = failed.join(", ");
    let resumable = !run.resumable().is_empty();
    let text = match (lang, failed.is_empty(), resumable) {
        (Lang::De, true, _) => format!(
            "Messe unvollständig: {set} von {planned} Teilen gesetzt; „Fehlende Teile setzen“ \
             setzt die übrigen."
        ),
        (Lang::En, true, _) => format!(
            "Mass incomplete: {set} of {planned} parts set; “Set the missing parts” sets the \
             others."
        ),
        (Lang::De, false, true) => format!(
            "Messe unvollständig: {set} von {planned} Teilen gesetzt; gescheitert: {failed}. \
             „Fehlende Teile setzen“ setzt die übrigen, „Teil neu setzen“ einen gescheiterten mit \
             einem anderen Seed."
        ),
        (Lang::En, false, true) => format!(
            "Mass incomplete: {set} of {planned} parts set; failed: {failed}. “Set the missing \
             parts” sets the others, “Set the part again” one that failed with another seed."
        ),
        (Lang::De, false, false) => format!(
            "Messe unvollständig: {set} von {planned} Teilen gesetzt; gescheitert: {failed}. \
             „Teil neu setzen“ versucht einen mit einem anderen Seed."
        ),
        (Lang::En, false, false) => format!(
            "Mass incomplete: {set} of {planned} parts set; failed: {failed}. “Set the part \
             again” tries one with another seed."
        ),
    };
    (!failed.is_empty(), text)
}

/// A text with its number of syllables, as the choice of texts lists it: „Kyrie I (7 Silben)“.
fn section_label(text: &Text, lang: Lang) -> String {
    let n = text.syllables.len();
    let noun = match (lang, n) {
        (Lang::De, 1) => "Silbe",
        (Lang::De, _) => "Silben",
        (Lang::En, 1) => "syllable",
        (Lang::En, _) => "syllables",
    };
    format!("{} ({n} {noun})", text.name(lang))
}

/// The sense units of a section with `units` of them that a text such as `gloria:1-5` or
/// `gloria:3` selects; all of them without a valid range.
fn unit_range(spec: &str, units: usize) -> (usize, usize) {
    let parsed = spec.split_once(':').and_then(|(_, range)| {
        let range = range.trim();
        match range.split_once('-') {
            Some((a, b)) => Some((a.trim().parse().ok()?, b.trim().parse().ok()?)),
            None => range.parse().ok().map(|a| (a, a)),
        }
    });
    match parsed {
        Some((a, b)) if 1 <= a && a <= b && b <= units => (a, b),
        _ => (1, units),
    }
}

/// A text as the voices sing it, for the form: one line per sense unit with the stressed
/// syllables strong, its size, the warnings about it and a line on the length of the setting.
fn text_preview(
    ui: &mut egui::Ui,
    text: &Text,
    warnings: &[String],
    length: &str,
    accents: bool,
    lang: Lang,
) {
    for line in unit_lines(text, accents) {
        let pieces = line.into_iter().map(|(piece, stressed)| {
            let piece = RichText::new(piece);
            if stressed { piece.strong() } else { piece }
        });
        ui.label(egui::WidgetText::concat(ui, pieces));
    }
    ui.weak(fux_cli::text::text_label(text, lang));
    for warning in warnings {
        ui.colored_label(
            Color32::from_rgb(220, 140, 0),
            format!("{}: {warning}", lang.pick("Warnung", "Warning")),
        );
    }
    ui.label(length);
}

/// The sense units of a text as lines of pieces, each with whether it is a stressed syllable:
/// the syllables joined by hyphens, with the acutes if `accents` is set and with the punctuation,
/// and between the words the markers `/` (a tail begins) and `|` (a break), as `fux text` shows
/// them.
fn unit_lines(text: &Text, accents: bool) -> Vec<Vec<(String, bool)>> {
    fn push(line: &mut Vec<(String, bool)>, piece: &str, stressed: bool) {
        match line.last_mut() {
            Some((last, s)) if *s == stressed => last.push_str(piece),
            _ => line.push((piece.to_string(), stressed)),
        }
    }
    text.units
        .iter()
        .map(|unit| {
            let mut line = Vec::new();
            let word_of = |syl: u16| usize::from(text.syllables[usize::from(syl)].word);
            let (first_word, last_word) = (word_of(unit.first), word_of(unit.last));
            for w in first_word..=last_word {
                let word = &text.words[w];
                let first = usize::from(word.first);
                if w > first_word {
                    let broken = text.syllables[first - 1]
                        .flags
                        .contains(SylFlags::BREAK_AFTER);
                    let tail = text.syllables[first].flags.contains(SylFlags::TAIL_START);
                    let gap = match (broken, tail) {
                        (true, true) => " / ",
                        (true, false) => " | ",
                        (false, _) => " ",
                    };
                    push(&mut line, gap, false);
                }
                for i in word.range() {
                    if i > first {
                        push(&mut line, "-", false);
                    }
                    let syllable = &text.syllables[i];
                    let shown = if accents {
                        syllable.text.clone()
                    } else {
                        strip_acute(&syllable.text)
                    };
                    push(&mut line, &shown, syllable.flags.contains(SylFlags::STRESS));
                }
                if let Some(punct) = word.punct {
                    push(&mut line, &punct.to_string(), false);
                }
            }
            line
        })
        .collect()
}

fn seed_choice(ui: &mut egui::Ui, lang: Lang, random: &mut bool, seed: &mut u64) {
    ui.horizontal(|ui| {
        ui.checkbox(random, lang.pick("neuer Seed", "new seed"));
        let label = ui.label("Seed:");
        ui.add_enabled(!*random, egui::DragValue::new(seed))
            .labelled_by(label.id);
    });
}

/// General MIDI sounds offered for playback.
fn program_name(program: u8, lang: Lang) -> String {
    match program {
        53 => lang.pick("Chor", "Choir").into(),
        20 => lang.pick("Orgel", "Organ").into(),
        1 => "Piano".into(),
        41 => lang.pick("Violine", "Violin").into(),
        72 => lang.pick("Klarinette", "Clarinet").into(),
        n => format!("GM {n}"),
    }
}

/// The formats offered for export.
const ALL_FORMATS: [Format; 6] = [
    Format::Ly,
    Format::Pdf,
    Format::Svg,
    Format::Png,
    Format::Midi,
    Format::Musicxml,
];

fn format_name(format: Format) -> &'static str {
    match format {
        Format::Ly => "ly",
        Format::Pdf => "pdf",
        Format::Svg => "svg",
        Format::Png => "png",
        Format::Midi => "midi",
        Format::Musicxml => "musicxml",
    }
}

fn format_from_name(name: &str) -> Option<Format> {
    ALL_FORMATS.into_iter().find(|f| format_name(*f) == name)
}

/// A score as the check form: the cantus and the other voices in LilyPond. A score that sings a
/// text goes along as MusicXML, which names the text it sings, so that its lyrics are checked;
/// the notes in LilyPond are there to edit it without them. The field for the text stays empty,
/// for files checked later. `repeat` is how freely its voices could restate the text.
fn check_form_of(score: &Score, repeat: Repeat) -> CheckForm {
    let cantus = score.cantus_firmus().map(|(_, v)| v);
    let texted = score.text.as_ref().map(|_| TextedScore {
        title: score.title.clone().unwrap_or_default(),
        voices: score.voices.len(),
        musicxml: to_musicxml(score, &MusicXmlOptions::default()),
    });
    CheckForm {
        text: String::new(),
        repeat,
        texted,
        cantus: cantus.map_or(String::new(), |v| lilypond_notes(&v.notes, score)),
        cf_voice: cantus.map(|v| v.voice_type),
        mode: Some(score.mode),
        cadences: if score.cadences.is_empty() {
            CadenceChoice::None
        } else {
            CadenceChoice::Measures
        },
        cadence_measures: score
            .cadences
            .iter()
            .map(|c| (c.measure + 1).to_string())
            .collect::<Vec<_>>()
            .join(", "),
        voices: score
            .voices
            .iter()
            .filter(|v| !v.is_cantus_firmus())
            .map(|v| CheckVoice {
                voice_type: v.voice_type,
                notes: lilypond_notes(&v.notes, score),
            })
            .collect(),
    }
}

/// Notes in LilyPond with bar lines, e.g. `r2 a'2~ | a'2 d''2`.
fn lilypond_notes(notes: &[Note], score: &Score) -> String {
    let per_measure = score.meter.ticks_per_measure();
    let mut text = String::new();
    let mut tick = 0;
    for note in notes {
        if tick > 0 {
            text.push_str(if tick % per_measure == 0 { " | " } else { " " });
        }
        text.push_str(&note.pitch.map_or("r".to_string(), |p| p.to_lilypond()));
        let ticks = note.duration.ticks();
        text.push_str(&match ticks {
            12 => "1.".to_string(),
            6 => "2.".to_string(),
            3 => "4.".to_string(),
            t if t > 0 && 8 % t == 0 => (8 / t).to_string(),
            t => format!("8*{t}"),
        });
        if note.tie {
            text.push('~');
        }
        tick += ticks;
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_goes_back_into_the_check_form() {
        let example = fux_examples()
            .into_iter()
            .find(|e| e.figure == "11")
            .unwrap();
        let score = example.score();
        let form = check_form_of(&score, Repeat::Auto);
        let again = form.input().unwrap().score(Lang::De).unwrap();
        assert_eq!(again.voices.len(), score.voices.len());
        for (a, b) in again.voices.iter().zip(&score.voices) {
            assert_eq!(a.pitches(), b.pitches());
        }
    }

    #[test]
    fn a_texted_result_goes_back_with_its_text() {
        // Christe in two voices, as the window sets it, its text restated freely.
        let form = FreeForm {
            text: "christe".into(),
            voices: 2,
            count: 1,
            repeat: Repeat::Free,
            ..FreeForm::default()
        };
        let options = form.options(4, Lang::De).unwrap();
        let prepared = prepare_free(&options, &Profile::fux(), Lang::De).unwrap();
        let found = jobs::free(&prepared, Lang::De).unwrap();
        let generated = &found.entries[0];
        assert!(generated.score.text.is_some());
        assert_eq!(generated.repeat, Repeat::Free);
        let form = check_form_of(&generated.score, generated.repeat);
        // The MusicXML names the text; the field stays free for files checked later.
        assert_eq!((form.text.as_str(), form.repeat), ("", Repeat::Free));
        let texted = form.texted.as_ref().unwrap();
        assert_eq!((texted.title.as_str(), texted.voices), ("Freier Satz 1", 2));
        assert!(texted.musicxml.contains("<lyric"));
        // Checked again with its lyrics: the same report, the text rules included.
        let checked = jobs::check(&form.job().unwrap(), &Profile::fux(), Lang::De).unwrap();
        let entry = &checked.entries[0];
        assert!(entry.score.text.is_some());
        assert_eq!(entry.report, generated.report);
        assert!(
            checked.summary[1].starts_with("Text: Christe; von "),
            "{:?}",
            checked.summary
        );
        assert!(checked.summary[1].ends_with(" weichen 0 ab"));
        // A file checked afterwards is checked with its own text, not the one taken over.
        let kyrie = FreeForm {
            voices: 2,
            count: 1,
            ..FreeForm::default()
        };
        let prepared = prepare_free(
            &kyrie.options(1, Lang::De).unwrap(),
            &Profile::fux(),
            Lang::De,
        )
        .unwrap();
        let kyrie = &jobs::free(&prepared, Lang::De).unwrap().entries[0].score;
        let path = std::env::temp_dir().join("fux-gui-test-handover-kyrie.musicxml");
        std::fs::write(&path, to_musicxml(kyrie, &MusicXmlOptions::default())).unwrap();
        let job = CheckJob {
            input: ScoreInput {
                musicxml: Some(path.clone()),
                ..ScoreInput::default()
            },
            musicxml: None,
            text: form.text_spec(),
            repeat: Repeat::Auto,
        };
        let checked = jobs::check(&job, &Profile::fux(), Lang::De).unwrap();
        std::fs::remove_file(&path).ok();
        assert!(
            checked.summary[1].starts_with("Text: Kyrie I; von "),
            "{:?}",
            checked.summary
        );
        // Without the MusicXML the notes are checked, without their text.
        let notes = CheckForm {
            texted: None,
            text: String::new(),
            ..form
        };
        let checked = jobs::check(&notes.job().unwrap(), &Profile::fux(), Lang::De).unwrap();
        assert!(checked.entries[0].score.text.is_none());
        assert!(
            checked.entries[0]
                .report
                .violations
                .iter()
                .all(|v| !v.rule.starts_with("TXT-"))
        );
    }

    #[test]
    fn own_texts_are_named_in_full() {
        let options = TextOptions::default();
        let (own, _) =
            fux_core::text::resolve("Ad te le-vá-vi | á-ni-mam / me-am.", &options).unwrap();
        let spec = own.spec();
        assert_eq!(spec, "Ad te le-vá-vi | á-ni-mam / me-am.");
        let (again, _) = fux_core::text::resolve(&spec, &options).unwrap();
        assert_eq!(again, own);
        let (range, _) = fux_core::text::resolve("Gloria: 1-5", &options).unwrap();
        assert_eq!(range.spec(), "gloria:1-5");
    }

    #[test]
    fn texts_are_shown_with_their_stresses() {
        let options = TextOptions::default();
        let line = |id: &str, accents: bool| {
            let text = ordinary::section(id, &options).unwrap();
            let lines = unit_lines(text, accents);
            assert_eq!(lines.len(), 1);
            lines.into_iter().next().unwrap()
        };
        let piece = |text: &str, stressed: bool| (text.to_string(), stressed);
        assert_eq!(
            line("kyrie", true),
            [
                piece("Ký", true),
                piece("-ri-e / e-", false),
                piece("lé", true),
                piece("-i-son.", false)
            ]
        );
        assert_eq!(line("kyrie", false)[0], piece("Ky", true));
        let benedictus = line("benedictus", true);
        let shown: String = benedictus.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(shown, "Be-ne-dí-ctus | qui ve-nit / in nó-mi-ne Dó-mi-ni.");
        let stressed: Vec<&str> = benedictus
            .iter()
            .filter(|(_, stressed)| *stressed)
            .map(|(text, _)| text.as_str())
            .collect();
        assert_eq!(stressed, ["dí", "ve", "nó", "Dó"]);
        // The Agnus has three sense units, one per line.
        let agnus = ordinary::section("agnus", &options).unwrap();
        assert_eq!(unit_lines(agnus, true).len(), 3);
        assert_eq!(
            section_label(agnus, Lang::De),
            "Agnus Dei I und II (18 Silben)"
        );
        assert_eq!(unit_range("gloria:2-5", 13), (2, 5));
        assert_eq!(unit_range("gloria:3", 13), (3, 3));
        assert_eq!(unit_range("gloria", 13), (1, 13));
        assert_eq!(unit_range("gloria:5-2", 13), (1, 13));
        assert_eq!(unit_range("gloria:1-20", 13), (1, 13));
    }

    #[test]
    fn settings_survive_a_restart() {
        let mut app = FuxApp::headless();
        app.lang = Lang::En;
        app.profile.apply_override("MEL-REPEAT.g1=P3").unwrap();
        app.cp.species = 4;
        app.free.text = "gloria:1-5".into();
        app.free.repeat = Repeat::Free;
        app.free.imitation.subject = SubjectKind::Notes;
        app.free.imitation.notes = "d'1 d'2 a'2 g'4 f'4 e'2 d'1".into();
        app.free.imitation.later_voices = LaterVoices::All;
        app.entry_marks = false;
        app.check.text = "kyrie".into();
        let again = FuxApp::from_stored(ron_roundtrip(&app.stored()), None);
        assert_eq!(again.free.imitation, app.free.imitation);
        assert!(!again.entry_marks);
        assert_eq!(again.lang, Lang::En);
        assert_eq!(again.cp.species, 4);
        assert_eq!(
            again.profile.settings("MEL-REPEAT").level("g1"),
            fux_core::rules::Level::P3
        );
        assert_eq!(
            (again.free.text.as_str(), again.free.repeat),
            ("gloria:1-5", Repeat::Free)
        );
        assert_eq!(again.check.text, "kyrie");
    }

    /// The tab of the mass and its form are kept between runs; the mass set is not.
    #[test]
    fn the_mass_form_survives_a_restart() {
        let mut app = FuxApp::headless();
        app.tab = Tab::Mass;
        app.mass.preset = Preset::Normal;
        app.mass.chant = Some(ChantMass::IX);
        app.mass.movements = [true, false, true, false, false, false];
        app.mass.section_mut("christe").ending = "A".into();
        app.mass.imitation = "credo/3=aus".into();
        app.mass.mensurstriche = true;
        let again = FuxApp::from_stored(ron_roundtrip(&app.stored()), None);
        assert_eq!(again.tab, Tab::Mass);
        assert_eq!(again.mass, app.mass);
        assert!(again.mass_run().is_none());
    }

    /// The table of sections offers the endings a section may have: the degrees of its
    /// movement's mode with a clausula, none for the last section of a movement, which ends on
    /// the final, or for one in two voices; an Osanna ut supra has no row.
    #[test]
    fn sections_offer_their_endings() {
        let form = MassForm {
            preset: Preset::Normal,
            voices: 3,
            movements: [true, false, false, true, true, false],
            ..MassForm::default()
        };
        let (_, plan) = jobs::prepare_mass(&form, 1, &Profile::fux(), Lang::De).unwrap();
        let rows = section_rows(&plan, Lang::De);
        let ids: Vec<&str> = rows.iter().map(|row| row.id).collect();
        assert_eq!(
            ids,
            [
                "kyrie",
                "christe",
                "kyrie2",
                "sanctus",
                "pleni",
                "osanna",
                "benedictus"
            ]
        );
        let endings = |id: &str| -> Vec<String> {
            let row = rows.iter().find(|row| row.id == id).unwrap();
            row.endings.iter().map(|(_, name)| name.clone()).collect()
        };
        assert_eq!(endings("christe"), ["D", "A", "G"]);
        assert!(endings("kyrie2").is_empty());
        // The Benedictus is followed by the Osanna ut supra: it need not end on the final.
        assert_eq!(endings("benedictus"), ["D", "A", "G"]);
        let christe = rows.iter().find(|row| row.id == "christe").unwrap();
        assert_eq!(
            (christe.title, christe.voices.as_str()),
            ("Christe", "A T B")
        );
        let two = MassForm { voices: 2, ..form };
        let (_, plan) = jobs::prepare_mass(&two, 1, &Profile::fux(), Lang::De).unwrap();
        assert!(
            section_rows(&plan, Lang::De)
                .iter()
                .all(|row| row.endings.is_empty())
        );
    }

    /// The status line at the end of a mass that lacks parts names those whose search failed, for
    /// „Teil neu setzen“, and points to „Fehlende Teile setzen“ only for parts cancelled or not
    /// searched yet: a part that failed would fail again with its seed, so the button sets none
    /// where only such parts are missing.
    #[test]
    fn the_end_of_a_mass_names_the_parts_that_failed() {
        let form = MassForm {
            movements: [true, false, false, false, false, false],
            voices: 3,
            ..MassForm::default()
        };
        let (prepared, plan) = jobs::prepare_mass(&form, 1, &Profile::fux(), Lang::De).unwrap();
        let mut run = MassRun::new(prepared, plan);
        let failed = || PartState::Failed(fux_core::generate::GenerateError::Invalid("no".into()));
        run.states = vec![failed(), PartState::Cancelled, PartState::Cancelled];
        assert_eq!(
            mass_ended(&run, Lang::De),
            (
                true,
                "Messe unvollständig: 0 von 3 Teilen gesetzt; gescheitert: kyrie/1. „Fehlende \
                 Teile setzen“ setzt die übrigen, „Teil neu setzen“ einen gescheiterten mit einem \
                 anderen Seed."
                    .to_string()
            )
        );
        assert_eq!(
            mass_ended(&run, Lang::En).1,
            "Mass incomplete: 0 of 3 parts set; failed: kyrie/1. “Set the missing parts” sets the \
             others, “Set the part again” one that failed with another seed."
        );
        run.states = vec![failed(), failed(), failed()];
        assert_eq!(
            mass_ended(&run, Lang::De),
            (
                true,
                "Messe unvollständig: 0 von 3 Teilen gesetzt; gescheitert: kyrie/1, kyrie/2, \
                 kyrie/3. „Teil neu setzen“ versucht einen mit einem anderen Seed."
                    .to_string()
            )
        );
        assert_eq!(
            mass_ended(&run, Lang::En).1,
            "Mass incomplete: 0 of 3 parts set; failed: kyrie/1, kyrie/2, kyrie/3. “Set the part \
             again” tries one with another seed."
        );
        let mut app = FuxApp::headless();
        app.mass_run = Some(run);
        app.set_missing_parts();
        assert!(!app.mass_busy());
    }

    /// Through the storage eframe uses.
    fn ron_roundtrip(stored: &Stored) -> Stored {
        let mut storage = crate::Memory::default();
        eframe::set_value(&mut storage, eframe::APP_KEY, stored);
        eframe::get_value(&storage, eframe::APP_KEY).unwrap()
    }
}
