//! The workbook (`fux book`): a setting in every mode, species and number of voices, in one
//! document, optionally with the exercises before the solutions.

use std::io::Write;

use anyhow::{Result, anyhow, bail};
use fux_core::corpus::fux_cantus;
use fux_core::generate::{CantusRequest, generate_cantus, generate_counterpoint};
use fux_core::rules::{Checker, Profile, Report};
use fux_core::{Ambitus, Lang, Mode, ModeSpec, Score, VoiceType};
use fux_export::{BookPart, Sheet};

use crate::commands::{
    CounterpointOptions, localize_names, prepare_counterpoint, print_files, time_seed,
};
use crate::input::Cadences;
use crate::output::{self, OutputArgs};

/// Options of `fux book`.
#[derive(Clone, Debug)]
pub struct BookOptions {
    /// Mode names, or `alle` for all twelve; none: the modes of Fux's cantus.
    pub modes: Vec<String>,
    pub species: Vec<u8>,
    /// Numbers of voices with the cantus; two voices give a setting above and one below.
    pub voices: Vec<u8>,
    /// A generated cantus in every mode instead of Fux's.
    pub generate: bool,
    /// Length of a generated cantus.
    pub length: usize,
    /// The exercises first: the cantus with empty staves for the other voices.
    pub exercises: bool,
    pub seed: Option<u64>,
    pub beam: usize,
    pub noise: f64,
}

impl Default for BookOptions {
    fn default() -> Self {
        BookOptions {
            modes: Vec::new(),
            species: vec![1, 2, 3, 4, 5],
            voices: vec![2],
            generate: false,
            length: 11,
            exercises: false,
            seed: None,
            beam: 200,
            noise: 2.0,
        }
    }
}

/// One setting of the workbook.
#[derive(Clone, Debug)]
pub struct BookEntry {
    /// Index into [`Workbook::modes`].
    pub mode: usize,
    /// What is set, e.g. "1. Gattung, Gegenstimme oben".
    pub task: String,
    /// The setting, or the cantus alone if the search found none.
    pub score: Score,
    pub cost: u32,
    /// The violations of the setting.
    pub report: Report,
    /// Why the search found no setting.
    pub failure: Option<String>,
}

/// The settings of a workbook, mode by mode.
#[derive(Clone, Debug)]
pub struct Workbook {
    pub modes: Vec<ModeSpec>,
    pub entries: Vec<BookEntry>,
    pub seed: u64,
    /// Name of the rule profile.
    pub profile: String,
}

impl Workbook {
    pub fn title(lang: Lang) -> &'static str {
        lang.pick("Übungsheft", "Workbook")
    }

    pub fn subtitle(&self, lang: Lang) -> String {
        format!(
            "{} · {} {} · Seed {}",
            lang.pick(
                "Kontrapunkt im Palestrinastil",
                "Counterpoint in the Palestrina style"
            ),
            lang.pick("Profil", "profile"),
            self.profile,
            self.seed
        )
    }

    /// The parts of the document: with `exercises` first those of every mode, then the
    /// solutions; each mode begins on a new page.
    pub fn parts(&self, exercises: bool, lang: Lang) -> Vec<BookPart<'_>> {
        let mut parts = Vec::new();
        if exercises {
            for (m, mode) in self.modes.iter().enumerate() {
                parts.push(BookPart {
                    heading: format!(
                        "{} – {}",
                        lang.pick("Aufgaben", "Exercises"),
                        capitalized(&mode.name(lang))
                    ),
                    intro: Vec::new(),
                    sheets: self.sheets(m, true, lang),
                });
            }
        }
        for (m, mode) in self.modes.iter().enumerate() {
            let name = capitalized(&mode.name(lang));
            parts.push(BookPart {
                heading: if exercises {
                    format!("{} – {name}", lang.pick("Lösungen", "Solutions"))
                } else {
                    name
                },
                intro: Vec::new(),
                sheets: self.sheets(m, false, lang),
            });
        }
        parts
    }

    fn sheets(&self, mode: usize, exercise: bool, lang: Lang) -> Vec<Sheet<'_>> {
        self.entries
            .iter()
            .filter(|e| e.mode == mode)
            .map(|e| {
                let mut sheet = Sheet::new(&e.score);
                sheet.subtitle = Some(e.task.clone());
                sheet.exercise = exercise;
                // Every setting of the workbook is set with its seed.
                sheet.seed = Some(self.seed);
                if let (Some(failure), false) = (&e.failure, exercise) {
                    sheet.remarks.push(format!(
                        "{}: {failure}",
                        lang.pick("Keine Lösung gefunden", "No solution found")
                    ));
                }
                sheet
            })
            .collect()
    }

    /// How many settings the search could not find.
    pub fn failures(&self) -> usize {
        self.entries.iter().filter(|e| e.failure.is_some()).count()
    }
}

/// "dorisch" → "Dorisch".
fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// The modes of the workbook: the named ones, all twelve for `alle`, or else those of Fux's
/// cantus.
pub fn book_modes(names: &[String]) -> Result<Vec<ModeSpec>> {
    if names
        .iter()
        .any(|n| matches!(n.trim().to_lowercase().as_str(), "alle" | "all"))
    {
        return Ok(Mode::ALL
            .into_iter()
            .flat_map(|m| {
                let authentic = ModeSpec::new(m);
                [authentic, authentic.with_ambitus(Ambitus::Plagal)]
            })
            .collect());
    }
    if names.is_empty() {
        let mut modes = Vec::new();
        for cantus in fux_cantus() {
            if !modes.contains(&cantus.mode) {
                modes.push(cantus.mode);
            }
        }
        return Ok(modes);
    }
    names
        .iter()
        .map(|n| n.trim().parse::<ModeSpec>().map_err(|e| anyhow!("{e}")))
        .collect()
}

/// What a setting asks for, e.g. "2. Gattung, Gegenstimme unten" or "3. Gattung, 4 Stimmen".
fn task_name(species: u8, voices: u8, below: Option<bool>, lang: Lang) -> String {
    let species = format!("{species}. {}", lang.pick("Gattung", "species"));
    match below {
        Some(below) => format!(
            "{species}, {}",
            match (below, lang) {
                (false, Lang::De) => "Gegenstimme oben",
                (true, Lang::De) => "Gegenstimme unten",
                (false, Lang::En) => "counterpoint above",
                (true, Lang::En) => "counterpoint below",
            }
        ),
        None => format!("{species}, {voices} {}", lang.pick("Stimmen", "voices")),
    }
}

/// The cantus of a mode: Fux's as `fux:ID`, or else a generated one as note names.
fn book_cantus(
    mode: ModeSpec,
    options: &BookOptions,
    seed: u64,
    profile: &Profile,
    lang: Lang,
) -> Result<String> {
    if !options.generate
        && let Some(cantus) = fux_cantus().into_iter().find(|c| c.mode == mode)
    {
        return Ok(format!("fux:{}", cantus.id));
    }
    let request = CantusRequest {
        voice_type: VoiceType::Alto,
        seed,
        ..CantusRequest::new(mode, options.length)
    };
    let mut generated = generate_cantus(&request, profile)
        .map_err(|e| anyhow!("{}: {}", mode.name(lang), e.message(lang)))?;
    let cantus = generated.remove(0).score;
    let notes: Vec<String> = cantus.voices[0]
        .pitches()
        .iter()
        .map(ToString::to_string)
        .collect();
    Ok(notes.join(" "))
}

/// Sets every mode in every species and number of voices; `progress` hears of each setting as
/// it is done. A setting the search cannot find keeps the cantus alone and says why.
pub fn workbook(
    options: &BookOptions,
    profile: &Profile,
    lang: Lang,
    mut progress: impl FnMut(&ModeSpec, &BookEntry),
) -> Result<Workbook> {
    let modes = book_modes(&options.modes)?;
    if options.species.is_empty() || options.voices.is_empty() {
        bail!(
            "{}",
            lang.pick(
                "Keine Gattung oder Stimmenzahl gewählt",
                "No species or number of voices chosen"
            )
        );
    }
    let seed = options.seed.unwrap_or_else(time_seed);
    let mut entries = Vec::new();
    for (m, &mode) in modes.iter().enumerate() {
        let cf = book_cantus(mode, options, seed, profile, lang)?;
        for &species in &options.species {
            for &voices in &options.voices {
                let places: &[Option<bool>] = if voices == 2 {
                    &[Some(false), Some(true)]
                } else {
                    &[None]
                };
                for &below in places {
                    let request = CounterpointOptions {
                        cf: Some(cf.clone()),
                        mode: Some(mode),
                        cf_voice: None,
                        cadences: Cadences::default(),
                        length: options.length,
                        species,
                        voices: usize::from(voices),
                        parts: None,
                        below: below == Some(true),
                        voice: None,
                        seed: Some(seed),
                        count: 1,
                        beam: options.beam,
                        noise: options.noise,
                        details: false,
                        // The species of the workbook sing no text.
                        ..CounterpointOptions::default()
                    };
                    let prepared = prepare_counterpoint(&request, profile, lang)?;
                    let task = task_name(species, voices, below, lang);
                    let mut entry = match generate_counterpoint(&prepared.request, profile) {
                        Ok(mut results) => {
                            let generated = results.remove(0);
                            BookEntry {
                                mode: m,
                                task,
                                score: generated.score,
                                cost: generated.cost,
                                report: generated.report,
                                failure: None,
                            }
                        }
                        Err(error) => BookEntry {
                            mode: m,
                            task,
                            report: Checker::new(profile).check(&prepared.base),
                            score: prepared.base,
                            cost: 0,
                            failure: Some(error.message(lang)),
                        },
                    };
                    localize_names(&mut entry.score, lang);
                    entry.score.title = None;
                    progress(&mode, &entry);
                    entries.push(entry);
                }
            }
        }
    }
    Ok(Workbook {
        modes,
        entries,
        seed,
        profile: profile.name.clone(),
    })
}

/// `fux book`: sets the workbook, prints a line per setting and writes the document.
pub fn book(
    out: &mut impl Write,
    options: &BookOptions,
    profile: &Profile,
    output: &OutputArgs,
    lang: Lang,
) -> Result<()> {
    let seed = options.seed.unwrap_or_else(time_seed);
    let options = BookOptions {
        seed: Some(seed),
        ..options.clone()
    };
    writeln!(
        out,
        "{}, {} {}, seed {seed}",
        Workbook::title(lang),
        lang.pick("Profil", "profile"),
        profile.name
    )?;
    let mut printed = Ok(());
    let workbook = workbook(&options, profile, lang, |mode, entry| {
        let result = match &entry.failure {
            None => format!("{} {}", lang.pick("Kosten", "cost"), entry.cost),
            Some(failure) => format!("{}: {failure}", lang.pick("keine Lösung", "no solution")),
        };
        if printed.is_ok() {
            printed = writeln!(out, "{} – {}: {result}", mode.name(lang), entry.task);
        }
    })?;
    printed?;
    let parts = workbook.parts(options.exercises, lang);
    let files = output::write_book(
        Workbook::title(lang),
        Some(&workbook.subtitle(lang)),
        &parts,
        output,
        "out/uebungsheft",
        lang,
    )?;
    print_files(out, &files, lang)?;
    let failures = workbook.failures();
    if failures > 0 {
        writeln!(
            out,
            "{failures} {}",
            lang.pick("Sätze ohne Lösung", "settings without a solution")
        )?;
    }
    Ok(())
}
