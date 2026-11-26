//! Work that runs in a background thread: setting counterpoint, generating cantus firmi,
//! checking a score, setting a mass. Each produces entries to show: a score with its violations;
//! a mass tells what it does as it goes ([`MassJob`]).

use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Instant;

use fux_cli::book::{BookOptions, Workbook, workbook};
use fux_cli::commands::{
    CounterpointOptions, counterpoint_summary, localize_names, prepare_counterpoint, report_line,
    report_summary,
};
use fux_cli::free::{PreparedFree, Repeat, free_summary, result_warnings};
use fux_cli::imitation::{Given, ImitationSource, attach_ending, plan_summary};
use fux_cli::input::{ReadScore, ScoreInput, add_cadences, musicxml_text_fields};
use fux_cli::mass::{
    FIELD_LYRICS, MassControl, MovementCheck, check_movement, check_profile, lyrics_field,
    mass_plan, mass_summary, restore_part, run_mass,
};
use fux_cli::text::{align_lyrics, unrecognised};
use fux_core::generate::{
    CantusRequest, FreeRequest, GenerateError, Generated, generate_cantus, generate_free,
};
use fux_core::imitation::OptionError;
use fux_core::mass::{
    FIELD_MASS, FIELD_PART, Mass, MassEvent, MassPlan, MassRequest, PartField, PartRef,
    movement_parts,
};
use fux_core::rules::{Checker, Profile, Report};
use fux_core::{Lang, Score};

use crate::forms::{CantusForm, MassForm};

/// A result to show: a score, its violations, and whether the violations are marked in it.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Short name in the list of results, e.g. "1. Kosten 170".
    pub label: String,
    pub score: Score,
    pub report: Report,
    /// Marks violations in the notes (for checked scores).
    pub marked: bool,
    /// How freely the voices of a free setting could restate their text, which a check of the
    /// result takes over.
    pub repeat: Repeat,
    /// The measures of a larger whole before the score, from which its measures count on: those
    /// of a part of a mass before it in the file of its movement; 0 for a score of its own.
    pub before: usize,
    /// For a part of a mass checked from its file or from that of its movement, the fields its
    /// MusicXML has besides those of a single setting, as `fux mass` writes the file of the part
    /// (`part_file_fields`): so that taken over into the check again, it is judged as its
    /// search judged it (D14). Empty for any other score.
    pub fields: Vec<(String, String)>,
    /// The seed the score was set with, where it is known (a counterpoint, a free setting, a
    /// cantus, a setting of a workbook): its MusicXML file names it.
    pub seed: Option<u64>,
}

/// What a task found: its entries and lines describing them.
#[derive(Clone, Debug, Default)]
pub struct Found {
    pub entries: Vec<Entry>,
    pub summary: Vec<String>,
    /// A workbook, whose entries are its settings, and whether it has the exercises.
    pub book: Option<(Workbook, bool)>,
    /// What the status line says instead of the number of results, such as for a plan.
    pub note: Option<String>,
}

/// A task running in the background.
pub struct Job {
    pub label: String,
    pub started: Instant,
    handle: JoinHandle<Result<Found, String>>,
}

impl Job {
    pub fn spawn(
        label: String,
        work: impl FnOnce() -> Result<Found, String> + Send + 'static,
    ) -> Job {
        Job {
            label,
            started: Instant::now(),
            handle: std::thread::spawn(work),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    /// The result; a panic in the task becomes an error.
    pub fn join(self) -> Result<Found, String> {
        self.handle
            .join()
            .unwrap_or_else(|_| Err("internal error".to_string()))
    }
}

/// `fux cp`: the solutions, best first; where the free voices sing a text, the warnings about it
/// follow the summary.
pub fn counterpoint(
    options: &CounterpointOptions,
    profile: &Profile,
    lang: Lang,
) -> Result<Found, String> {
    let prepared = prepare_counterpoint(options, profile, lang).map_err(|e| e.to_string())?;
    let texted = prepared.request.text.is_some();
    let results = prepared
        .generate()
        .map_err(|e| counterpoint_error(&e, texted, lang))?;
    let mut summary = counterpoint_summary(&prepared, &results, lang);
    summary.extend(
        prepared
            .warnings
            .iter()
            .map(|w| format!("{}: {w}", lang.pick("Warnung", "Warning"))),
    );
    Ok(Found {
        entries: numbered(
            results,
            lang.pick("Kontrapunkt", "Counterpoint"),
            prepared.repeat,
            prepared.seed,
            lang,
        ),
        summary,
        ..Found::default()
    })
}

/// The message of a failed counterpoint search; where the free voices sing a text and the
/// search got stuck on it, with what the form can change.
fn counterpoint_error(error: &GenerateError, texted: bool, lang: Lang) -> String {
    match (error, texted) {
        (GenerateError::NoCounterpoint { .. }, true) => format!(
            "{} {}",
            error.message(lang),
            lang.pick(
                "Abhilfe: Wiederholung „frei“, ein kürzerer Text, ein längerer Cantus oder andere Binnenschlüsse.",
                "Remedies: repetition “free”, a shorter text, a longer cantus or other internal cadences.",
            )
        ),
        _ => error.message(lang),
    }
}

/// `fux free`: the settings, best first, of a request prepared from the form; the warnings about
/// its text and its imitation, and those its results carry, follow the summary.
pub fn free(prepared: &PreparedFree, lang: Lang) -> Result<Found, String> {
    let request = &prepared.request;
    let results =
        generate_free(request, &prepared.profile).map_err(|e| free_error(&e, request, lang))?;
    let mut summary = free_summary(prepared, &results, lang);
    summary.extend(
        prepared
            .warnings
            .iter()
            .cloned()
            .chain(result_warnings(&results, lang))
            .map(|w| format!("{}: {w}", lang.pick("Warnung", "Warning"))),
    );
    Ok(Found {
        entries: numbered(
            results,
            lang.pick("Freier Satz", "Free counterpoint"),
            prepared.repeat,
            prepared.seed,
            lang,
        ),
        summary,
        ..Found::default()
    })
}

/// The plan of the imitation of a request prepared from the form, without a search: its points,
/// their heads and entries, as lines of the summary, without results.
pub fn plan(prepared: &PreparedFree, lang: Lang) -> Result<Found, String> {
    let lines = plan_summary(prepared, lang).ok_or_else(|| {
        lang.pick(
            "Dieser Satz imitiert nicht (auto: mit Text, drei bis fünf Stimmen, automatischen Binnenschlüssen und automatischer Länge); Imitation „an“ schaltet sie ein.",
            "This setting does not imitate (auto: with a text, three to five voices, automatic internal cadences and an automatic length); imitation “on” turns it on.",
        )
        .to_string()
    })?;
    let mut summary = lines.map_err(|e| free_error(&e, &prepared.request, lang))?;
    summary.extend(
        prepared
            .warnings
            .iter()
            .map(|w| format!("{}: {w}", lang.pick("Warnung", "Warning"))),
    );
    Ok(Found {
        summary,
        note: Some(
            lang.pick(
                "Plan der Imitation, ohne Suche.",
                "Plan of the imitation, without a search.",
            )
            .to_string(),
        ),
        ..Found::default()
    })
}

/// The message of a failed free search of `request`; where a text is too long, the search got
/// stuck on it or the imitation found no setting, with what the form can change.
fn free_error(error: &GenerateError, request: &FreeRequest, lang: Lang) -> String {
    let texted = request.text.is_some();
    if request.imitation.is_some()
        && let Some(hint) = imitation_remedies(error, request, lang)
    {
        return format!("{} {hint}", error.message(lang));
    }
    let hint = match (error, texted) {
        (GenerateError::TextTooLong { .. }, _) => lang.pick(
            "Abhilfe: mehr Takte oder „automatisch“, ein kürzerer Text (etwa weniger Sinnabschnitte) oder weniger Binnenschlüsse.",
            "Remedies: more measures or “automatic”, a shorter text (such as fewer sense units) or fewer internal cadences.",
        ),
        (GenerateError::NoCounterpoint { .. }, true) => lang.pick(
            "Abhilfe: mehr Takte, Wiederholung „frei“, ein kürzerer Text oder andere Binnenschlüsse.",
            "Remedies: more measures, repetition “free”, a shorter text or other internal cadences.",
        ),
        _ => return error.message(lang),
    };
    format!("{} {hint}", error.message(lang))
}

/// What the form can change where an imitation fails, in the words of the form.
fn imitation_remedies(error: &GenerateError, request: &FreeRequest, lang: Lang) -> Option<String> {
    let undo = Given::of(request).undo(match lang {
        Lang::De => [
            "Kopf „auto“",
            "Reihenfolge leer",
            "Intervalle leer",
            "Takte „automatisch“",
        ],
        Lang::En => [
            "head “auto”",
            "order empty",
            "intervals empty",
            "measures “automatic”",
        ],
    });
    Some(match (error, lang) {
        (GenerateError::NoImitation { .. }, Lang::De) => format!(
            "Abhilfe: ein anderer Seed, andere Abstände, weniger Stimmen späterer Punkte{undo} oder Imitation „aus“."
        ),
        (GenerateError::NoImitation { .. }, Lang::En) => format!(
            "Remedies: another seed, other delays, fewer voices of later points{undo} or imitation “off”."
        ),
        (GenerateError::ImitationTooLong { measures }, Lang::De) => {
            format!("Abhilfe: {measures} Takte oder „automatisch“, oder Imitation „aus“.")
        }
        (GenerateError::ImitationTooLong { measures }, Lang::En) => {
            format!("Remedies: {measures} measures or “automatic”, or imitation “off”.")
        }
        (GenerateError::SubjectInvalid { .. }, _) => lang
            .pick(
                "Abhilfe: andere Noten für das Soggetto, oder Soggetto „aus dem Text“.",
                "Remedies: other notes for the subject, or the subject “from the text”.",
            )
            .to_string(),
        (GenerateError::NoChantPhrase { phrases, .. }, Lang::De) => {
            format!("Abhilfe: Phrasen von 1 bis {phrases}, oder keine.")
        }
        (GenerateError::NoChantPhrase { phrases, .. }, Lang::En) => {
            format!("Remedies: phrases from 1 to {phrases}, or none.")
        }
        (GenerateError::PointCount { most }, Lang::De) => {
            format!("Abhilfe: Punkte „auto“ oder höchstens {most}.")
        }
        (GenerateError::PointCount { most }, Lang::En) => {
            format!("Remedies: points “auto” or {most} at most.")
        }
        (GenerateError::ImitationOption(option), _) => option_remedy(option, lang),
        _ => return None,
    })
}

/// What the form can change where the imitation has an option the request cannot have, in the
/// words of the form.
fn option_remedy(option: &OptionError, lang: Lang) -> String {
    match (option, lang) {
        (OptionError::NoPoint { points, .. }, Lang::De) => format!(
            "Abhilfe: Reihenfolge, Intervalle und Abstände für höchstens {points} Punkte, durch ; getrennt."
        ),
        (OptionError::NoPoint { points, .. }, Lang::En) => format!(
            "Remedies: order, intervals and delays for {points} points at most, separated by ;."
        ),
        (
            OptionError::NoVoice { .. }
            | OptionError::VoiceTwice { .. }
            | OptionError::FewVoices { .. },
            _,
        ) => lang
            .pick(
                "Abhilfe: eine andere Reihenfolge, oder keine.",
                "Remedies: another order, or none.",
            )
            .into(),
        (OptionError::TooMany { .. }, _) => lang
            .pick(
                "Abhilfe: weniger Intervalle oder Abstände für diesen Punkt, oder mehr Stimmen für ihn (Stimmen späterer Punkte, Reihenfolge).",
                "Remedies: fewer intervals or delays for this point, or more voices for it (voices of later points, order).",
            )
            .into(),
        (OptionError::Delay { .. }, _) => lang
            .pick(
                "Abhilfe: Abstände in Takten ab einer Halben, Halbe erlaubt.",
                "Remedies: delays in measures from a minim on, halves allowed.",
            )
            .into(),
        (OptionError::LaterVoices { most, .. }, Lang::De) => format!(
            "Abhilfe: Stimmen späterer Punkte „auto“, „alle“ oder 2 bis {most}."
        ),
        (OptionError::LaterVoices { most, .. }, Lang::En) => {
            format!("Remedies: voices of later points “auto”, “all” or 2 to {most}.")
        }
        (OptionError::Head { most, .. }, Lang::De) => {
            format!("Abhilfe: Kopf „auto“ oder 4 bis {most} Töne.")
        }
        (OptionError::Head { most, .. }, Lang::En) => {
            format!("Remedies: head “auto” or 4 to {most} notes.")
        }
        (OptionError::Subjects { points, .. }, Lang::De) => format!(
            "Abhilfe: höchstens {points} Köpfe oder Phrasen, oder Punkt 1 wiederholen „aus“."
        ),
        (OptionError::Subjects { points, .. }, Lang::En) => format!(
            "Remedies: {points} heads or phrases at most, or restate point 1 “off”."
        ),
        (OptionError::Restate, _) => lang
            .pick(
                "Abhilfe: Punkt 1 wiederholen „auto“ oder „aus“.",
                "Remedies: restate point 1 “auto” or “off”.",
            )
            .into(),
        (OptionError::Relation { .. }, _) => lang
            .pick(
                "Abhilfe: andere Intervalle, oder keine.",
                "Remedies: other intervals, or none.",
            )
            .into(),
    }
}

/// The solutions as entries, titled and numbered, made with the repetition `repeat` and the
/// seed `seed`.
fn numbered(
    results: Vec<Generated>,
    title: &str,
    repeat: Repeat,
    seed: u64,
    lang: Lang,
) -> Vec<Entry> {
    results
        .into_iter()
        .enumerate()
        .map(|(i, mut generated)| {
            localize_names(&mut generated.score, lang);
            generated.score.title = Some(format!("{title} {}", i + 1));
            Entry {
                label: format!(
                    "{}. {} {}",
                    i + 1,
                    lang.pick("Kosten", "cost"),
                    generated.cost
                ),
                score: generated.score,
                report: generated.report,
                marked: false,
                repeat,
                before: 0,
                fields: Vec::new(),
                seed: Some(seed),
            }
        })
        .collect()
}

/// `fux book`: a setting in every mode and species, each an entry.
pub fn book(options: &BookOptions, profile: &Profile, lang: Lang) -> Result<Found, String> {
    let book = workbook(options, profile, lang, |_, _| {}).map_err(|e| format!("{e:#}"))?;
    let entries = book
        .entries
        .iter()
        .map(|entry| {
            let mode = book.modes[entry.mode].name(lang);
            let result = match &entry.failure {
                None => format!("{} {}", lang.pick("Kosten", "cost"), entry.cost),
                Some(_) => lang.pick("keine Lösung", "no solution").to_string(),
            };
            Entry {
                label: format!("{mode} – {} ({result})", entry.task),
                score: entry.score.clone(),
                report: entry.report.clone(),
                marked: false,
                repeat: Repeat::Auto,
                before: 0,
                fields: Vec::new(),
                seed: Some(book.seed),
            }
        })
        .collect();
    let mut summary = vec![
        format!("{} – {}", Workbook::title(lang), book.subtitle(lang)),
        format!(
            "{} {}, {} {}",
            book.modes.len(),
            lang.pick("Modi", "modes"),
            book.entries.len(),
            lang.pick("Sätze", "settings")
        ),
    ];
    if book.failures() > 0 {
        summary.push(format!(
            "{} {}",
            book.failures(),
            lang.pick("Sätze ohne Lösung", "settings without a solution")
        ));
    }
    Ok(Found {
        entries,
        summary,
        book: Some((book, options.exercises)),
        note: None,
    })
}

/// `fux cf`: cantus firmi, best first.
pub fn cantus(
    form: &CantusForm,
    seed: u64,
    profile: &Profile,
    lang: Lang,
) -> Result<Found, String> {
    let request = CantusRequest {
        voice_type: form.voice_type,
        seed,
        count: form.count,
        ..CantusRequest::new(form.mode, form.length)
    };
    let generated = generate_cantus(&request, profile).map_err(|e| e.message(lang))?;
    let summary = vec![format!(
        "{} – {} {}, seed {seed}",
        form.mode.name(lang),
        form.length,
        lang.pick("Töne", "notes")
    )];
    let entries = generated
        .into_iter()
        .enumerate()
        .map(|(i, mut g)| {
            localize_names(&mut g.score, lang);
            g.score.title = Some(format!("Cantus firmus {}", i + 1));
            Entry {
                label: format!("{}. {} {}", i + 1, lang.pick("Kosten", "cost"), g.cost),
                score: g.score,
                report: g.report,
                marked: false,
                repeat: Repeat::Auto,
                before: 0,
                fields: Vec::new(),
                seed: Some(seed),
            }
        })
        .collect();
    Ok(Found {
        entries,
        summary,
        ..Found::default()
    })
}

/// What `fux check` checks in the window.
#[derive(Debug)]
pub struct CheckJob {
    /// The score of the form or of a file.
    pub input: ScoreInput,
    /// A result with a text as MusicXML, checked instead of the notes of `input`, with its mode
    /// and its cadences.
    pub musicxml: Option<String>,
    /// The text of the lyrics as `fux check --text` takes it; `None` for the text a MusicXML
    /// file of fux-kontrapunkt names, or else the section recognised.
    pub text: Option<String>,
    /// How freely the voices could restate the text when the setting was made
    /// (`fux check --repeat`).
    pub repeat: Repeat,
}

/// `fux check`: the score with its violations marked. Lyrics are aligned with their text first,
/// so that the text rules apply, and the plan of an imitation that the file keeps judges its
/// entries; the lines saying with which follow the summary. A plan that does not fit the score
/// is left out, with a warning.
///
/// The files of a mass (phase 14) are checked as `fux check` checks them: the file of a
/// movement part by part, each as its search judged it, and another file in two sections or
/// more section by section ([`fux_cli::mass::check_movement`], [`movement_check`]); the file of
/// a part under the overlay of the mass, its text going on where it went on, its syllables
/// singing the positions they sang, its last point ending on its ending (D14).
pub fn check(job: &CheckJob, profile: &Profile, lang: Lang) -> Result<Found, String> {
    let error = |e: anyhow::Error| format!("{e:#}");
    let (read, fields) = match &job.musicxml {
        Some(musicxml) => {
            let (mut read, fields) =
                musicxml_text_fields(musicxml, job.input.mode, lang).map_err(error)?;
            add_cadences(&mut read.score, None, &job.input.cadences);
            (read, fields)
        }
        None => job.input.read_with_fields(lang).map_err(error)?,
    };
    let source = ImitationSource::File;
    let text = job.text.as_deref();
    if let Some(check) =
        check_movement(&read, &fields, text, profile, &[], job.repeat, source, lang)
            .map_err(error)?
    {
        return Ok(movement_check(check, &fields, job.repeat, lang));
    }
    let (part, profile) = check_profile(&fields, profile, &[], job.repeat, lang).map_err(error)?;
    let ReadScore {
        mut score,
        text: stored,
        imitation,
    } = read;
    localize_names(&mut score, lang);
    let text = job.text.clone().or(stored);
    let mut lines = align_lyrics(&mut score, text.as_deref(), lang).map_err(error)?;
    if let Some(part) = &part {
        restore_part(&mut score, part, &fields);
    }
    let ending = part
        .as_ref()
        .map_or_else(|| score.mode.final_step(), |part| part.ending);
    match attach_ending(&mut score, imitation, source, "", ending, lang) {
        Ok(planned) => lines.extend(planned),
        Err(unfit) => lines.push(format!(
            "{}: {unfit:#}; {}",
            lang.pick("Warnung", "Warning"),
            lang.pick("geprüft ohne ihn", "checked without it")
        )),
    }
    let kept = part
        .as_ref()
        .map(|part| part_file_fields(&fields, part, &score));
    let mut found = checked(
        score,
        &profile,
        lang,
        lang.pick("Geprüfter Satz", "Checked score"),
    );
    if let (Some(kept), Some(entry)) = (kept, found.entries.first_mut()) {
        entry.fields = kept;
    }
    found
        .summary
        .extend(lines.into_iter().map(|line| in_window_words(line, lang)));
    Ok(found)
}

/// The fields the file of a part of a mass has besides those of a single setting, as `fux mass`
/// writes it ([`fux_cli::mass::write_part`]), for `score`, the part `part` as a check judged it
/// from a file with `fields`: the mass (`fux-mass`), the part (`fux-part`) with its measures from
/// 1, and the positions its syllables sing (`fux-lyrics`). With them a check of the score judges
/// it as the check of the file did.
fn part_file_fields(
    fields: &[(String, String)],
    part: &PartField,
    score: &Score,
) -> Vec<(String, String)> {
    let mut kept: Vec<(String, String)> = fields
        .iter()
        .filter(|(name, _)| name == FIELD_MASS)
        .cloned()
        .collect();
    let part = PartField {
        bars: 1..=score.measure_count(),
        ..part.clone()
    };
    kept.push((FIELD_PART.to_string(), part.to_string()));
    kept.push((FIELD_LYRICS.to_string(), lyrics_field(score)));
    kept
}

/// The check of a file in parts or sections as results ([`fux_cli::mass::check_movement`]):
/// each piece an entry with its violations marked, its measures counted on from those of the
/// file before it, a part of a mass movement with the fields of its own file, read from the
/// fields `fields` of the movement's ([`part_file_fields`]); the summary the totals, as `fux
/// check` ends its report, and a line per piece with its heading, its prohibitions and its cost.
fn movement_check(
    check: MovementCheck,
    fields: &[(String, String)],
    repeat: Repeat,
    lang: Lang,
) -> Found {
    let mut all = Report::default();
    let mut summary = Vec::new();
    let count = check.pieces.len();
    // The parts of the movement, which its check has read already.
    let parts = if check.parts {
        movement_parts(fields).unwrap_or_default()
    } else {
        Vec::new()
    };
    let entries = check
        .pieces
        .into_iter()
        .enumerate()
        .map(|(n, piece)| {
            summary.push(format!(
                "{} – {}",
                piece.heading,
                report_line(&piece.report, lang)
            ));
            all.violations
                .extend(piece.report.violations.iter().cloned());
            let kept = parts
                .get(n)
                .map(|part| part_file_fields(fields, part, &piece.score))
                .unwrap_or_default();
            let mut score = piece.score;
            localize_names(&mut score, lang);
            score.title = Some(piece.heading.clone());
            Entry {
                label: piece.heading,
                score,
                report: piece.report,
                marked: true,
                repeat,
                before: piece.before,
                fields: kept,
                seed: None,
            }
        })
        .collect();
    let pieces = match (check.parts, lang, count) {
        (true, Lang::De, 1) => "1 Teil".to_string(),
        (true, Lang::De, n) => format!("{n} Teilen"),
        (true, Lang::En, 1) => "1 part".to_string(),
        (true, Lang::En, n) => format!("{n} parts"),
        (false, Lang::De, 1) => "1 Abschnitt".to_string(),
        (false, Lang::De, n) => format!("{n} Abschnitten"),
        (false, Lang::En, 1) => "1 section".to_string(),
        (false, Lang::En, n) => format!("{n} sections"),
    };
    let totals = report_line(&all, lang);
    let totals = totals.trim_end_matches('.');
    summary.insert(
        0,
        match lang {
            Lang::De => format!("Gesamt: {totals} in {pieces}."),
            Lang::En => format!("Total: {totals} in {pieces}."),
        },
    );
    Found {
        entries,
        summary,
        ..Found::default()
    }
}

/// A line of [`align_lyrics`] in the words of the window, which has a field where `fux check`
/// has the option `--text`.
fn in_window_words(line: String, lang: Lang) -> String {
    if line == unrecognised(lang) {
        lang.pick(
            "Text nicht erkannt; im Feld „Text:“ angeben",
            "Text not recognised; give it in the field “Text:”",
        )
        .to_string()
    } else {
        line
    }
}

/// A mass prepared from the form before its search: the request of `fux mass`, and the profile
/// its parts are set and checked with, the profile of the window with the overlay of the mass.
#[derive(Clone, Debug)]
pub struct PreparedMass {
    pub request: MassRequest,
    pub profile: Profile,
}

/// The mass of `form` with the seed `seed`, prepared on `profile` and planned without a search,
/// in milliseconds: what the form asks that the plan cannot have is an error in the user's
/// language ([`fux_cli::mass::prepare_mass`], [`fux_cli::mass::mass_plan`]).
pub fn prepare_mass(
    form: &MassForm,
    seed: u64,
    profile: &Profile,
    lang: Lang,
) -> Result<(PreparedMass, MassPlan), String> {
    let (request, profile) = form.prepare(seed, profile, lang)?;
    let plan = mass_plan(&request, lang).map_err(|e| format!("{e:#}"))?;
    Ok((PreparedMass { request, profile }, plan))
}

/// The plan of the mass of `form` with the seed `seed` as `fux mass --plan` prints it, without
/// a search (docs/design/phase-14/spec.md § 3.10), its estimate with `jobs` parts set at once,
/// as the window sets them.
pub fn mass_plan_lines(
    form: &MassForm,
    seed: u64,
    profile: &Profile,
    jobs: usize,
    lang: Lang,
) -> Result<Found, String> {
    let (_, plan) = prepare_mass(form, seed, profile, lang)?;
    Ok(Found {
        summary: mass_summary(&plan, jobs, lang)
            .lines()
            .map(str::to_string)
            .collect(),
        note: Some(
            lang.pick(
                "Plan der Messe, ohne Suche.",
                "Plan of the mass, without a search.",
            )
            .to_string(),
        ),
        ..Found::default()
    })
}

/// `fux mass` in the window (docs/design/phase-14/spec.md § 7): sets the parts `parts` of `m`, a
/// mass of `prepared`, `jobs` of them at once, as the program sets them
/// ([`fux_cli::mass::run_mass`]), and sends what happens to `events`: each part as its search
/// begins ([`MassEvent::Started`]) and as it is set or fails, and [`MassEvent::Cancelled`] where
/// `control` cancels the searches or the window no longer listens. The measure each search has
/// reached `control` tells. Returns the mass with the parts set, and those it had.
pub fn mass(
    prepared: &PreparedMass,
    m: Mass,
    parts: &[PartRef],
    jobs: usize,
    events: &Sender<MassEvent>,
    control: &MassControl,
) -> Mass {
    let on = &mut |event| match events.send(event) {
        Ok(()) => ControlFlow::Continue(()),
        Err(_) => ControlFlow::Break(()),
    };
    run_mass(
        &prepared.request,
        m,
        parts,
        &prepared.profile,
        Some(control),
        jobs,
        on,
    )
}

/// A mass being set in a thread of its own ([`mass`]): what it tells, and its control.
pub struct MassJob {
    handle: Option<JoinHandle<Mass>>,
    pub events: Receiver<MassEvent>,
    pub control: Arc<MassControl>,
}

impl MassJob {
    /// Sets `parts` of `m`, a mass of `prepared`, `jobs` of them at once, in a thread of its own.
    pub fn spawn(prepared: PreparedMass, m: Mass, parts: Vec<PartRef>, jobs: usize) -> MassJob {
        let (sender, events) = mpsc::channel();
        let control = Arc::new(MassControl::new());
        let searched = Arc::clone(&control);
        let handle =
            std::thread::spawn(move || mass(&prepared, m, &parts, jobs, &sender, &searched));
        MassJob {
            handle: Some(handle),
            events,
            control,
        }
    }

    /// Asks the searches to stop: the parts searched end within a second, the parts set stay.
    pub fn cancel(&self) {
        self.control.cancel();
    }

    pub fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// The mass, once the thread has ended; a panic in it becomes an error.
    pub fn join(mut self) -> Result<Mass, String> {
        let handle = self.handle.take().expect("a job is joined once");
        handle.join().map_err(|_| "internal error".to_string())
    }
}

impl Drop for MassJob {
    /// A job the window lets go of before it ends stops its searches.
    fn drop(&mut self) {
        if self.handle.is_some() {
            self.control.cancel();
        }
    }
}

/// A score with all its violations, marked.
pub fn checked(score: Score, profile: &Profile, lang: Lang, label: &str) -> Found {
    let report = Checker::new(profile).check(&score);
    let summary = vec![report_summary(&report, lang)];
    Found {
        entries: vec![Entry {
            label: label.to_string(),
            score,
            report,
            marked: true,
            repeat: Repeat::Auto,
            before: 0,
            fields: Vec::new(),
            seed: None,
        }],
        summary,
        ..Found::default()
    }
}

#[cfg(test)]
mod tests {
    use fux_cli::free::{FreeOptions, prepare_free};
    use fux_cli::imitation::ImitationOptions;
    use fux_core::imitation::Toggle;

    use super::*;

    /// An option of the imitation that the planner refuses fails the plan and the search at once,
    /// with what the form can change, in the words of the form.
    #[test]
    fn refused_options_say_what_the_form_can_change() {
        let options = FreeOptions {
            text: "gloria:1-5".into(),
            seed: Some(1),
            imitation: ImitationOptions {
                restate: Toggle::On,
                ..ImitationOptions::default()
            },
            ..FreeOptions::default()
        };
        let prepared = prepare_free(&options, &Profile::fux(), Lang::De).unwrap();
        let expected = "Der Text wird nur einmal gesungen; es gibt keinen Punkt zu wiederholen. \
                        Abhilfe: Punkt 1 wiederholen „auto“ oder „aus“.";
        assert_eq!(plan(&prepared, Lang::De).unwrap_err(), expected);
        assert_eq!(free(&prepared, Lang::De).unwrap_err(), expected);
    }
}
