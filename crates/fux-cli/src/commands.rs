//! The commands besides `render`: generating, checking and listing.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use fux_core::corpus::{fux_cantus, fux_examples};
use fux_core::generate::{
    CantusRequest, CounterpointRequest, GenerateError, Generated, MIN_MEASURES, NewVoice, SungText,
    choir_with, generate_cantus, generate_counterpoint,
};
use fux_core::rules::{
    Checker, Level, ParamKind, ParamValue, Profile, Report, RuleInfo, Violation, all_rules,
};
use fux_core::text::plan::{Plan, plan};
use fux_core::text::{Eleison, Text};
use fux_core::{Lang, Meter, ModeSpec, Note, Pitch, Role, Score, Species, Voice, VoiceType};
use fux_export::{Color, Highlight, Sheet};

use crate::free::{Repeat, text_line};
use crate::output::{self, OutputArgs};
use crate::text::{restates, text_of};

pub fn print_files(out: &mut impl Write, files: &[PathBuf], lang: Lang) -> Result<()> {
    for file in files {
        writeln!(
            out,
            "{} {}",
            lang.pick("geschrieben:", "written:"),
            file.display()
        )?;
    }
    Ok(())
}

/// Options of `fux cf`.
pub struct CantusOptions {
    pub mode: ModeSpec,
    pub length: usize,
    pub voice: VoiceType,
    pub seed: Option<u64>,
    pub count: usize,
    pub beam: usize,
    pub noise: f64,
    pub details: bool,
}

/// `fux cf`: generates, prints and writes cantus firmi.
pub fn cantus(
    out: &mut impl Write,
    options: &CantusOptions,
    profile: &Profile,
    output: &OutputArgs,
    lang: Lang,
) -> Result<()> {
    let seed = options.seed.unwrap_or_else(time_seed);
    let request = CantusRequest {
        voice_type: options.voice,
        seed,
        count: options.count,
        beam: options.beam,
        noise: options.noise,
        ..CantusRequest::new(options.mode, options.length)
    };
    let results = generate_cantus(&request, profile).map_err(|e| anyhow!("{}", e.message(lang)))?;
    writeln!(
        out,
        "{} – {} {}, seed {seed}",
        options.mode.name(lang),
        options.length,
        lang.pick("Töne", "notes")
    )?;
    let checker = Checker::new(profile);
    let mut scores = Vec::new();
    for (i, mut generated) in results.into_iter().enumerate() {
        localize_names(&mut generated.score, lang);
        let notes: Vec<String> = generated.score.voices[0]
            .pitches()
            .iter()
            .map(ToString::to_string)
            .collect();
        writeln!(
            out,
            "{:>2}. {}   ({} {})",
            i + 1,
            notes.join(" "),
            lang.pick("Kosten", "cost"),
            generated.cost
        )?;
        if options.details {
            print_violations(
                out,
                &generated.score,
                &checker,
                &generated.report,
                lang,
                "      ",
            )?;
        }
        let mut score = generated.score;
        score.title = Some(format!("Cantus firmus {}", i + 1));
        scores.push(score);
    }
    let sheets: Vec<Sheet> = scores
        .iter()
        .map(|score| Sheet {
            seed: Some(seed),
            ..Sheet::new(score)
        })
        .collect();
    let title = format!("Cantus firmi – {}", options.mode.name(lang));
    let files = output::write(Some(&title), &sheets, output, "out/cantus", lang)?;
    print_files(out, &files, lang)
}

/// Options of `fux cp`.
#[derive(Clone, Debug)]
pub struct CounterpointOptions {
    pub cf: Option<String>,
    pub mode: Option<ModeSpec>,
    pub cf_voice: Option<VoiceType>,
    pub cadences: crate::input::Cadences,
    pub length: usize,
    pub species: u8,
    /// Number of voices with the cantus.
    pub voices: usize,
    /// Voice types from top to bottom, the cantus' among them.
    pub parts: Option<String>,
    pub below: bool,
    pub voice: Option<VoiceType>,
    pub seed: Option<u64>,
    pub count: usize,
    pub beam: usize,
    pub noise: f64,
    pub details: bool,
    /// What the free voices sing in free counterpoint (species 0): a section of the Ordinary
    /// (`kyrie`), some of its sense units (`gloria:1-5`), an own Latin text, or `none`, the
    /// default. The cantus keeps its own syllables, if it has any, and never sings the text.
    pub text: String,
    /// How freely the free voices restate their text.
    pub repeat: Repeat,
    /// The printed syllables keep the acute of the liturgical books.
    pub accents: bool,
    pub eleison: Eleison,
}

impl Default for CounterpointOptions {
    /// As `fux cp` without options: a counterpoint in the first species above a generated
    /// Dorian cantus of eleven notes, without a text.
    fn default() -> Self {
        CounterpointOptions {
            cf: None,
            mode: None,
            cf_voice: None,
            cadences: crate::input::Cadences::default(),
            length: 11,
            species: 1,
            voices: 2,
            parts: None,
            below: false,
            voice: None,
            seed: None,
            count: 1,
            beam: 200,
            noise: 2.0,
            details: false,
            text: "none".to_string(),
            repeat: Repeat::Auto,
            accents: true,
            eleison: Eleison::Four,
        }
    }
}

/// What `fux cp` hands to the search: the request with its seed, the cantus' score, the species
/// and the profile to search with; where the free voices sing a text, how they sing it and the
/// warnings about it.
#[derive(Clone, Debug)]
pub struct Prepared {
    pub request: CounterpointRequest,
    pub seed: u64,
    pub base: Score,
    pub species: Species,
    /// The profile for the search and the report: the one given, with a text with TXT-REPEAT as
    /// `--repeat` sets it.
    pub profile: Profile,
    /// How freely the free voices restate their text, which a check of a result takes over.
    pub repeat: Repeat,
    /// The printed syllables keep the acute of the liturgical books.
    pub accents: bool,
    /// Warnings about the text in the user's language, such as uncertain accents.
    pub warnings: Vec<String>,
}

impl Prepared {
    /// The search for the prepared request under the prepared profile. Where the free voices
    /// sing a text, each result has its syllables written out under them; without a text the
    /// results are those of the request and the profile given.
    pub fn generate(&self) -> Result<Vec<Generated>, GenerateError> {
        let mut results = generate_counterpoint(&self.request, &self.profile)?;
        if self.request.text.is_some() {
            for result in &mut results {
                result.score.fill_syllables(self.accents);
            }
        }
        Ok(results)
    }
}

/// The plan of `text` sung by free voices around a cantus of `measures` measures, which fixes
/// the length. The free voices hold no cadence formulas there: every minim beat but the second
/// of the last measure may begin a syllable, as the search counts it, which is the plan of a
/// single voice without internal cadences; the cadences stay those of the cantus. A text that
/// needs more fails with [`GenerateError::TextTooLong`].
pub fn plan_around_cantus(
    text: &Text,
    mode: ModeSpec,
    measures: usize,
    restatements: bool,
) -> Result<Plan, GenerateError> {
    plan(
        text,
        mode,
        1,
        Some(measures),
        Some(Vec::new()),
        restatements,
    )
}

/// The fewest measures of a cantus around which free voices can sing `text`.
pub fn least_cantus(text: &Text, mode: ModeSpec) -> usize {
    match plan_around_cantus(text, mode, MIN_MEASURES, true) {
        Err(GenerateError::TextTooLong { measures }) => measures,
        _ => MIN_MEASURES,
    }
}

/// The text the free voices sing around the cantus of `base`, at the pace planned for the
/// cantus' length under `profile`. A text too long for the cantus is an error that says what
/// may help.
fn sung_around(base: &Score, text: Text, profile: &Profile, lang: Lang) -> Result<SungText> {
    let measures = base.measure_count();
    let planned = plan_around_cantus(&text, base.mode, measures, restates(profile, &text))
        .map_err(|error| {
            let message = error.message(lang);
            match (error, lang) {
                (GenerateError::TextTooLong { measures: least }, Lang::De) => anyhow!(
                    "{message} Der Cantus hat {measures} Takte. Abhilfe: ein Cantus von mindestens {least} Tönen oder ein kürzerer Text (etwa weniger Sinnabschnitte wie gloria:1-2)."
                ),
                (GenerateError::TextTooLong { measures: least }, Lang::En) => anyhow!(
                    "{message} The cantus has {measures} measures. Remedies: a cantus of at least {least} notes or a shorter text (such as fewer sense units like gloria:1-2)."
                ),
                _ => anyhow!("{message}"),
            }
        })?;
    Ok(SungText {
        text: Arc::new(text),
        rate: planned.rate,
    })
}

/// Builds the search request of `fux cp`: the given cantus (or a generated one) and the voices
/// to set around it; in free counterpoint with the text they sing.
pub fn prepare_counterpoint(
    options: &CounterpointOptions,
    profile: &Profile,
    lang: Lang,
) -> Result<Prepared> {
    let seed = options.seed.unwrap_or_else(time_seed);
    let species = match options.species {
        0 => Species::Free,
        n => Species::from_number(n)
            .ok_or_else(|| anyhow!("{} {n}", lang.pick("Unbekannte Gattung", "Unknown species")))?,
    };
    // The text before the cantus, which may take a search: an error in it shows at once.
    let sung = text_of(&options.text, options.eleison, lang)?;
    if sung.is_some() && species != Species::Free {
        bail!(
            "{}",
            lang.pick(
                "Einen Text singen nur freie Gegenstimmen (Gattung 0).",
                "Only free counterpoint (species 0) sings a text."
            )
        );
    }
    let base = match &options.cf {
        Some(cf) => {
            crate::input::cantus_score(cf, options.mode, options.cf_voice, &options.cadences, lang)?
        }
        None => {
            let mode = options
                .mode
                .unwrap_or_else(|| "dorian".parse().expect("valid mode"));
            let request = CantusRequest {
                voice_type: options.cf_voice.unwrap_or(VoiceType::Alto),
                seed,
                ..CantusRequest::new(mode, options.length)
            };
            let mut generated =
                generate_cantus(&request, profile).map_err(|e| anyhow!("{}", e.message(lang)))?;
            generated.remove(0).score
        }
    };
    let cantus_voice = base.voices[0].voice_type;
    let new_voices = new_voices(options, cantus_voice, species, lang)?;
    let (profile, text, warnings) = match sung {
        None => (profile.clone(), None, Vec::new()),
        Some((text, warnings)) => {
            let profile = options.repeat.apply(profile)?;
            let text = sung_around(&base, text, &profile, lang)?;
            (profile, Some(text), warnings)
        }
    };
    let request = CounterpointRequest {
        seed,
        count: options.count,
        beam: options.beam,
        noise: options.noise,
        text,
        ..CounterpointRequest::new(base.clone(), new_voices)
    };
    Ok(Prepared {
        request,
        seed,
        base,
        species,
        profile,
        repeat: options.repeat,
        accents: options.accents,
        warnings,
    })
}

/// The message of a failed search of `fux cp`; where the free voices sing a text, with what may
/// help.
pub fn counterpoint_error(error: &GenerateError, texted: bool, lang: Lang) -> String {
    let message = error.message(lang);
    match (error, texted) {
        (GenerateError::NoCounterpoint { .. }, true) => format!(
            "{message} {}",
            lang.pick(
                "Abhilfe: --repeat frei, ein kürzerer Text, ein längerer Cantus oder andere Binnenschlüsse (--cadences).",
                "Remedies: --repeat free, a shorter text, a longer cantus or other internal cadences (--cadences).",
            )
        ),
        _ => message,
    }
}

/// Lines describing a counterpoint search for the windows (GUI, web page): mode, species, voices
/// and seed, the title, the cantus as set (with its leading tones) and the internal cadences.
pub fn counterpoint_summary(prepared: &Prepared, results: &[Generated], lang: Lang) -> Vec<String> {
    let base = &prepared.base;
    let mut summary = vec![format!(
        "{} – {}, {} {}, seed {}",
        base.mode.name(lang),
        prepared.species.name(lang),
        prepared.request.voices.len() + 1,
        lang.pick("Stimmen", "voices"),
        prepared.seed
    )];
    if let Some(title) = &base.title {
        summary.push(title.clone());
    }
    if let Some((_, cantus)) = results.first().and_then(|r| r.score.cantus_firmus()) {
        summary.push(format!(
            "C. f. ({}): {}",
            cantus.voice_type.name(lang),
            voice_text(cantus, base.meter)
        ));
    }
    if let Some(sung) = &prepared.request.text {
        summary.push(text_line(&sung.text, prepared.repeat, lang));
    }
    if !base.cadences.is_empty() {
        let list: Vec<String> = base
            .cadences
            .iter()
            .map(|c| {
                let target = Pitch::new(c.target, base.mode.scale_alter(c.target), 4);
                format!("{} {}", c.measure + 1, target.class_name(lang))
            })
            .collect();
        summary.push(format!(
            "{}: {}",
            lang.pick(
                "Binnenschlüsse (Takt, Ziel)",
                "Internal cadences (measure, target)"
            ),
            list.join(", ")
        ));
    }
    summary
}

/// One line on the violations of a checked score: prohibitions and cost.
pub fn report_summary(report: &Report, lang: Lang) -> String {
    let forbidden = report.forbidden();
    let preferences = report.violations.len() - forbidden;
    match lang {
        Lang::De => format!(
            "{forbidden} Verbot(e), Kosten {} aus {preferences} Präferenz(en).",
            report.cost()
        ),
        Lang::En => format!(
            "{forbidden} prohibition(s), cost {} from {preferences} preference(s).",
            report.cost()
        ),
    }
}

/// `fux cp`: sets counterpoint to a given or generated cantus, prints and writes the solutions.
/// Free voices that sing a text show it under their notes.
pub fn counterpoint(
    out: &mut impl Write,
    options: &CounterpointOptions,
    profile: &Profile,
    output: &OutputArgs,
    lang: Lang,
) -> Result<()> {
    let prepared = prepare_counterpoint(options, profile, lang)?;
    for warning in &prepared.warnings {
        writeln!(out, "{}: {warning}", lang.pick("Warnung", "Warning"))?;
    }
    let texted = prepared.request.text.is_some();
    let results = prepared
        .generate()
        .map_err(|e| anyhow!("{}", counterpoint_error(&e, texted, lang)))?;
    let Prepared {
        request,
        seed,
        base,
        species,
        profile,
        repeat,
        ..
    } = prepared;
    let cantus_voice = base.voices[0].voice_type;
    // The cantus as set, with the leading tones it may have got before cadences.
    let cantus: Vec<String> = results[0]
        .score
        .cantus_firmus()
        .map_or(&base.voices[0], |(_, v)| v)
        .pitches()
        .iter()
        .map(ToString::to_string)
        .collect();
    let count = request.voices.len() + 1;
    if count == 2 {
        writeln!(
            out,
            "{} – {}, seed {seed}",
            base.mode.name(lang),
            species.name(lang)
        )?;
    } else {
        writeln!(
            out,
            "{} – {}, {count} {}, seed {seed}",
            base.mode.name(lang),
            species.name(lang),
            lang.pick("Stimmen", "voices")
        )?;
    }
    if let Some(title) = &base.title {
        writeln!(out, "{title}")?;
    }
    writeln!(
        out,
        "C. f. ({}): {}",
        cantus_voice.name(lang),
        cantus.join(" ")
    )?;
    if let Some(sung) = &request.text {
        writeln!(out, "{}", text_line(&sung.text, repeat, lang))?;
    }
    if !base.cadences.is_empty() {
        let list: Vec<String> = base
            .cadences
            .iter()
            .map(|c| {
                let target = Pitch::new(c.target, base.mode.scale_alter(c.target), 4);
                format!(
                    "{} {} {} {}",
                    lang.pick("Takt", "measure"),
                    c.measure + 1,
                    lang.pick("auf", "on"),
                    target.class_name(lang)
                )
            })
            .collect();
        writeln!(
            out,
            "{}: {}",
            lang.pick("Binnenschlüsse", "Internal cadences"),
            list.join(", ")
        )?;
    }
    let checker = Checker::new(&profile);
    let mut scores = Vec::new();
    for (i, mut generated) in results.into_iter().enumerate() {
        localize_names(&mut generated.score, lang);
        let score = &generated.score;
        // The new voices with what they sing: the text of the setting, if they sing one.
        let new: Vec<(&Voice, Option<String>)> = (0..score.voices.len())
            .filter(|&v| score.voices[v].role != Role::CantusFirmus)
            .map(|v| (&score.voices[v], score.voice_lyrics(v)))
            .collect();
        if let [(cp, lyrics)] = new.as_slice() {
            writeln!(
                out,
                "{:>2}. {} ({}): {}   ({} {})",
                i + 1,
                lang.pick("Gegenstimme", "Counterpoint"),
                cp.voice_type.name(lang),
                voice_text(cp, score.meter),
                lang.pick("Kosten", "cost"),
                generated.cost
            )?;
            if let Some(lyrics) = lyrics {
                writeln!(out, "      Text: {lyrics}")?;
            }
        } else {
            writeln!(
                out,
                "{:>2}. {} {}",
                i + 1,
                lang.pick("Kosten", "cost"),
                generated.cost
            )?;
            for (voice, lyrics) in new {
                let label = match voice.role {
                    Role::Counterpoint(s) if s != Species::First => {
                        format!("{} ({})", voice.name, s.name(lang))
                    }
                    _ => voice.name.clone(),
                };
                writeln!(out, "      {label}: {}", voice_text(voice, score.meter))?;
                if let Some(lyrics) = lyrics {
                    writeln!(out, "        Text: {lyrics}")?;
                }
            }
        }
        if options.details {
            print_violations(
                out,
                &generated.score,
                &checker,
                &generated.report,
                lang,
                "      ",
            )?;
        }
        let mut score = generated.score;
        score.title = Some(format!(
            "{} {}",
            lang.pick("Kontrapunkt", "Counterpoint"),
            i + 1
        ));
        scores.push(score);
    }
    let sheets: Vec<Sheet> = scores
        .iter()
        .map(|score| Sheet {
            seed: Some(seed),
            ..Sheet::new(score)
        })
        .collect();
    let mut title = format!("{} – {}", base.mode.name(lang), species.name(lang));
    if let Some(sung) = &request.text {
        title = format!("{title}, {}", sung.text.name(lang));
    }
    let files = output::write(Some(&title), &sheets, output, "out/kontrapunkt", lang)?;
    print_files(out, &files, lang)
}

/// Gives voices that carry the default German name of their type the name in `lang`; the
/// generators name new voices in German. Voices of the same type are numbered from the top.
pub fn localize_names(score: &mut Score, lang: Lang) {
    let types: Vec<VoiceType> = score.voices.iter().map(|v| v.voice_type).collect();
    let mut seen: BTreeMap<VoiceType, usize> = BTreeMap::new();
    for voice in &mut score.voices {
        if voice.name == voice.voice_type.name(Lang::De) {
            voice.name = voice.voice_type.name(lang).to_string();
            if types.iter().filter(|t| **t == voice.voice_type).count() > 1 {
                let number = seen.entry(voice.voice_type).or_default();
                *number += 1;
                voice.name = format!("{} {number}", voice.name);
            }
        }
    }
}

/// The voices to generate: in two parts the counterpoint of `--voice` or the neighbour of the
/// cantus; with more parts all voices of `--parts` or a usual choir with the cantus' type, the
/// cantus taking one of them, the voice of `--voice` (or the highest) in `species`, the others in
/// whole notes as in Fux's examples.
fn new_voices(
    options: &CounterpointOptions,
    cantus: VoiceType,
    species: Species,
    lang: Lang,
) -> Result<Vec<NewVoice>> {
    let parts: Vec<VoiceType> = match &options.parts {
        Some(text) => {
            let mut parts = text
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|t| !t.is_empty())
                .map(|t| {
                    t.parse::<VoiceType>().map_err(|_| {
                        anyhow!(
                            "{}: {t}",
                            lang.pick("Unbekannte Stimmlage", "Unknown voice type")
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            parts.sort();
            parts
        }
        None if options.voices > 2 => choir_with(options.voices, cantus, options.voice),
        None => {
            let voice_type = options
                .voice
                .or_else(|| cantus.neighbor(!options.below))
                .ok_or_else(|| {
                    anyhow!(
                        "{}",
                        lang.pick(
                            "Keine Stimmlage für die Gegenstimme frei; bitte --voice angeben",
                            "No voice type free for the counterpoint; please give --voice"
                        )
                    )
                })?;
            return Ok(vec![NewVoice {
                voice_type,
                species,
            }]);
        }
    };
    let mut others = parts.clone();
    let Some(at) = others.iter().position(|v| *v == cantus) else {
        return Err(anyhow!(
            "{} {}",
            lang.pick(
                "Die Besetzung enthält keine Stimme für den Cantus:",
                "The parts have no voice for the cantus:"
            ),
            cantus.name(lang)
        ));
    };
    others.remove(at);
    if others.is_empty() || others.len() > 5 {
        return Err(anyhow!(
            "{}",
            lang.pick(
                "Die Besetzung braucht zwei bis sechs Stimmen.",
                "The parts need two to six voices."
            )
        ));
    }
    let florid = options.voice.unwrap_or(others[0]);
    if !others.contains(&florid) {
        return Err(anyhow!(
            "{} {}",
            lang.pick(
                "Die Gattungsstimme fehlt in der Besetzung:",
                "The voice for the species is not among the parts:"
            ),
            florid.name(lang)
        ));
    }
    let mut assigned = false;
    Ok(others
        .into_iter()
        .map(|voice_type| {
            // Free counterpoint has no voices in whole notes besides the cantus.
            let species = if species == Species::Free || !assigned && voice_type == florid {
                assigned = true;
                species
            } else {
                Species::First
            };
            NewVoice {
                voice_type,
                species,
            }
        })
        .collect())
}

/// The notes of a voice by measure: `r` for a rest, `~` after a tied note, `|` between measures.
pub fn voice_text(voice: &Voice, meter: Meter) -> String {
    let per_measure = meter.ticks_per_measure();
    let mut text = String::new();
    for (start, note) in voice.timed() {
        if start > 0 {
            text.push_str(if start % per_measure == 0 { " | " } else { " " });
        }
        match note.pitch {
            Some(pitch) => text.push_str(&pitch.to_string()),
            None => text.push('r'),
        }
        if note.tie {
            text.push('~');
        }
    }
    text
}

/// A seed from the clock, for runs without `--seed`.
pub fn time_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64 % 1_000_000)
        .unwrap_or(1)
}

/// Where a violation happens: at the note it marks; in a voice that sings the score's text, with
/// the syllable sung there (`Takt 3, Viertel 2 („ri-“)`).
pub fn violation_position(score: &Score, violation: &Violation, lang: Lang) -> String {
    violation_position_after(score, violation, 0, lang)
}

/// Where a violation happens as [`violation_position`] tells it, in a score that follows `before`
/// measures of a larger whole, such as a part of a mass in the file of its movement (phase 14):
/// its measures are counted on from there.
pub fn violation_position_after(
    score: &Score,
    violation: &Violation,
    before: usize,
    lang: Lang,
) -> String {
    let tick = score
        .note_start(violation.voice, violation.note)
        .unwrap_or(violation.tick);
    let position = position(score, tick, before, lang);
    match sung_syllable(score, violation.voice, violation.note) {
        Some(syllable) => match lang {
            Lang::De => format!("{position} („{syllable}“)"),
            Lang::En => format!("{position} (\"{syllable}\")"),
        },
        None => position,
    }
}

/// The syllable that a voice singing the score's text sings at a note, as printed: the note's
/// own, or the one it continues; at a rest, the one the silence ends. `None` in other voices, and
/// where an attack after a rest has no syllable.
fn sung_syllable(score: &Score, voice: usize, note: usize) -> Option<&str> {
    let sings = score
        .text
        .as_ref()
        .is_some_and(|text| text.voices.get(voice).copied().unwrap_or(false));
    if !sings {
        return None;
    }
    fn printed(note: &Note) -> Option<&str> {
        note.syllable
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }
    let (marked, before) = score.voices.get(voice)?.notes.get(..=note)?.split_last()?;
    if let Some(syllable) = printed(marked) {
        return Some(syllable);
    }
    let mut before = before.iter().rev().peekable();
    if marked.is_rest() {
        while before.next_if(|n| n.is_rest()).is_some() {}
    }
    for n in before {
        if n.is_rest() {
            return None;
        }
        if let Some(syllable) = printed(n) {
            return Some(syllable);
        }
    }
    None
}

/// A position in the score: measure, and quarter within the measure if not on the downbeat; the
/// measures counted on after `before` of a larger whole.
fn position(score: &Score, tick: u32, before: usize, lang: Lang) -> String {
    let per_measure = score.meter.ticks_per_measure();
    let measure = (tick / per_measure) as usize + 1 + before;
    let within = tick % per_measure;
    match (within, lang) {
        (0, Lang::De) => format!("Takt {measure}"),
        (0, Lang::En) => format!("Measure {measure}"),
        (w, Lang::De) => format!("Takt {measure}, Viertel {}", w / 2 + 1),
        (w, Lang::En) => format!("Measure {measure}, beat {}", w / 2 + 1),
    }
}

pub fn print_violations(
    out: &mut impl Write,
    score: &Score,
    checker: &Checker,
    report: &Report,
    lang: Lang,
    indent: &str,
) -> Result<()> {
    print_violations_after(out, score, checker, report, 0, lang, indent)
}

/// Prints the violations as [`print_violations`] does, for a score that follows `before`
/// measures of a larger whole ([`violation_position_after`]).
pub fn print_violations_after(
    out: &mut impl Write,
    score: &Score,
    checker: &Checker,
    report: &Report,
    before: usize,
    lang: Lang,
    indent: &str,
) -> Result<()> {
    for v in &report.violations {
        writeln!(
            out,
            "{indent}{}, {}: {} [{}, {}]",
            violation_position_after(score, v, before, lang),
            score.voices[v.voice].name,
            checker.message(v, lang),
            v.rule,
            v.level.label(lang)
        )?;
    }
    Ok(())
}

/// `fux check`: prints the violations; returns the number of broken prohibitions.
pub fn check(
    out: &mut impl Write,
    score: &Score,
    profile: &Profile,
    mark: bool,
    output: &OutputArgs,
    lang: Lang,
) -> Result<usize> {
    let checker = Checker::new(profile);
    let report = checker.check(score);
    print_violations(out, score, &checker, &report, lang, "")?;
    let forbidden = report.forbidden();
    writeln!(out, "{}", report_line(&report, lang))?;
    if mark {
        let sheet = marked_sheet(score, &report, &checker, lang);
        let files = output::write(None, &[sheet], output, "out/pruefung", lang)?;
        print_files(out, &files, lang)?;
    }
    Ok(forbidden)
}

/// The last line of a check: „0 Verbot(e), Kosten 45 aus 7 Präferenz(en).“
pub fn report_line(report: &Report, lang: Lang) -> String {
    let forbidden = report.forbidden();
    let preferences = report.violations.len() - forbidden;
    match lang {
        Lang::De => format!(
            "{forbidden} Verbot(e), Kosten {} aus {preferences} Präferenz(en).",
            report.cost()
        ),
        Lang::En => format!(
            "{forbidden} prohibition(s), cost {} from {preferences} preference(s).",
            report.cost()
        ),
    }
}

/// The score with its violations marked: prohibitions red, costs orange, each with the rule's id
/// above the note and a remark below the score.
pub fn marked_sheet<'a>(
    score: &'a Score,
    report: &Report,
    checker: &Checker,
    lang: Lang,
) -> Sheet<'a> {
    marked_sheet_after(score, report, checker, 0, lang)
}

/// The score with its violations marked as [`marked_sheet`] marks them, for a score that follows
/// `before` measures of a larger whole: its measures, and those its remarks name, are counted on
/// from there.
pub fn marked_sheet_after<'a>(
    score: &'a Score,
    report: &Report,
    checker: &Checker,
    before: usize,
    lang: Lang,
) -> Sheet<'a> {
    let mut sheet = Sheet {
        first_bar: before + 1,
        ..Sheet::new(score)
    };
    for v in &report.violations {
        sheet.highlights.push(Highlight {
            voice: v.voice,
            note: v.note,
            color: if v.is_forbidden() {
                Color::Red
            } else {
                Color::Orange
            },
            label: Some(v.rule.to_string()),
        });
        sheet.remarks.push(format!(
            "{}, {}: {} [{}, {}]",
            violation_position_after(score, v, before, lang),
            score.voices[v.voice].name,
            checker.message(v, lang),
            v.rule,
            v.level.label(lang)
        ));
    }
    sheet
}

/// Parameters of a rule as `name=value` in the rule's order, or just the level for rules with a
/// single level.
fn settings_summary(profile: &Profile, info: &RuleInfo, lang: Lang) -> String {
    let settings = profile.settings(info.id);
    let label = |name: &str| {
        settings
            .values()
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, value)| match value {
                ParamValue::Level(level) => level.label(lang),
                other => other.to_string(),
            })
            .unwrap_or_default()
    };
    match info.params {
        [single] if single.kind == ParamKind::Level => label(single.name),
        params => params
            .iter()
            .map(|p| format!("{}={}", p.name, label(p.name)))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// `fux rules`: one line per rule.
pub fn rule_list(out: &mut impl Write, profile: &Profile, lang: Lang) -> Result<()> {
    writeln!(out, "{} {}", lang.pick("Profil:", "Profile:"), profile.name)?;
    for rule in all_rules() {
        let info = rule.info();
        writeln!(out, "{:<22} {}", info.id, info.title(lang))?;
        writeln!(out, "{:<22} {}", "", settings_summary(profile, info, lang))?;
    }
    Ok(())
}

/// `fux rules --id ID`: one rule with its parameters.
pub fn rule_details(out: &mut impl Write, profile: &Profile, id: &str, lang: Lang) -> Result<()> {
    let rule = all_rules()
        .into_iter()
        .find(|r| r.info().id.eq_ignore_ascii_case(id))
        .ok_or_else(|| anyhow!("{} {id}", lang.pick("Unbekannte Regel", "Unknown rule")))?;
    let info = rule.info();
    writeln!(out, "{}: {}", info.id, info.title(lang))?;
    writeln!(out, "{} {}", lang.pick("Quelle:", "Source:"), info.source)?;
    let settings = profile.settings(info.id);
    for param in info.params {
        let value = settings
            .values()
            .iter()
            .find(|(n, _)| *n == param.name)
            .map(|(_, v)| match v {
                ParamValue::Level(level) => level.label(lang),
                other => other.to_string(),
            })
            .unwrap_or_default();
        let kind = match param.kind {
            ParamKind::Level => lang.pick("Stufe", "level"),
            ParamKind::Int => lang.pick("Zahl", "number"),
            ParamKind::Bool => lang.pick("ja/nein", "yes/no"),
        };
        writeln!(
            out,
            "  {} = {value}   ({kind}) {}",
            param.name,
            lang.pick(param.de, param.en)
        )?;
    }
    Ok(())
}

/// `fux rules --corpus`: how often each rule fires on Fux's own examples.
/// How often each rule fires in Fux's examples: (all violations, prohibitions).
pub fn corpus_counts(profile: &Profile) -> BTreeMap<&'static str, (usize, usize)> {
    let checker = Checker::new(profile);
    let mut counts: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    for example in fux_examples() {
        for v in checker.check(&example.score()).violations {
            let entry = counts.entry(v.rule).or_default();
            entry.0 += 1;
            if v.level == Level::Forbidden {
                entry.1 += 1;
            }
        }
    }
    counts
}

pub fn rule_corpus(out: &mut impl Write, profile: &Profile, lang: Lang) -> Result<()> {
    let counts = corpus_counts(profile);
    let examples = fux_examples();
    writeln!(
        out,
        "{} ({} {})",
        lang.pick(
            "Verstöße in Fux' Beispielen",
            "Violations in Fux's examples"
        ),
        examples.len(),
        lang.pick("Beispiele", "examples")
    )?;
    writeln!(
        out,
        "{:<22} {:>9} {:>9}",
        "",
        lang.pick("gesamt", "total"),
        lang.pick("Verbote", "forbidden")
    )?;
    for rule in all_rules() {
        let id = rule.info().id;
        let (all, hard) = counts.get(id).copied().unwrap_or_default();
        writeln!(out, "{id:<22} {all:>9} {hard:>9}")?;
    }
    Ok(())
}

pub fn print_cantus(out: &mut impl Write, lang: Lang) -> Result<()> {
    for c in fux_cantus() {
        let notes: Vec<String> = c.notes.iter().map(ToString::to_string).collect();
        writeln!(
            out,
            "fux:{:<16} {:<12} {}",
            c.id,
            c.mode.name(lang),
            notes.join(" ")
        )?;
    }
    Ok(())
}

pub fn print_examples(out: &mut impl Write, lang: Lang) -> Result<()> {
    writeln!(
        out,
        "{:<6} {:<12} {:<9} {:<9} {:<18} {}",
        "Fig.",
        lang.pick("Modus", "Mode"),
        lang.pick("Stimmen", "Voices"),
        lang.pick("Gattung", "Species"),
        "C. f.",
        lang.pick("Seite 1725", "Page 1725")
    )?;
    for example in fux_examples() {
        let score = example.score();
        let species: Vec<String> = example
            .species
            .iter()
            .map(|s| s.number().to_string())
            .collect();
        let cf = score
            .voices
            .iter()
            .find(|v| v.role == Role::CantusFirmus)
            .map(|v| v.voice_type.name(lang))
            .unwrap_or("-");
        writeln!(
            out,
            "{:<6} {:<12} {:<9} {:<9} {:<18} {}",
            example.figure,
            example.mode.name(lang),
            example.parts.len(),
            species.join("+"),
            cf,
            example.page
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fux_core::rules::Detail;
    use fux_core::text::{TextOptions, resolve};
    use fux_core::{Duration, ScoreText, Step};

    use super::*;

    /// `fux cp` around Fux's Dorian cantus with seed 1 and these options.
    fn around_dorian(options: CounterpointOptions, lang: Lang) -> Result<Prepared> {
        let options = CounterpointOptions {
            cf: Some("fux:dorian".into()),
            seed: Some(1),
            ..options
        };
        prepare_counterpoint(&options, &Profile::fux(), lang)
    }

    /// The overrides a prepared profile makes to Fux's profile, as TOML lines.
    fn changes(prepared: &Prepared) -> Vec<String> {
        prepared
            .profile
            .to_toml(Some(&Profile::fux()))
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with("name"))
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn free_voices_around_a_cantus_plan_their_text() {
        let free = |text: &str| CounterpointOptions {
            species: 0,
            voices: 3,
            text: text.into(),
            ..CounterpointOptions::default()
        };
        // Without a text the request is the one before: no text, no fixed notes, the profile
        // given.
        let plain = around_dorian(free("none"), Lang::De).unwrap();
        assert!(plain.request.text.is_none() && plain.request.fixed.is_empty());
        assert!(changes(&plain).is_empty() && plain.warnings.is_empty());
        // Kyrie I around the eleven measures of the cantus, planned twice: 14 syllable starts
        // over the nine measures before the last two.
        let kyrie = around_dorian(free("kyrie"), Lang::De).unwrap();
        let sung = kyrie.request.text.as_ref().unwrap();
        assert_eq!((sung.text.id.as_str(), sung.rate), ("kyrie", 78));
        assert_eq!(kyrie.request.voices.len(), 2);
        assert!(kyrie.request.fixed.is_empty() && kyrie.base.cadences.is_empty());
        // The cantus sings nothing.
        assert!(kyrie.base.text.is_none());
        assert!(kyrie.base.voices[0].notes.iter().all(|n| n.lyric.is_none()));
        assert!(changes(&kyrie).is_empty());
        assert_eq!(
            counterpoint_summary(&kyrie, &[], Lang::De),
            [
                "dorisch – freier Satz, 3 Stimmen, seed 1",
                "Text: Kyrie I (7 Silben, 1 Sinnabschnitt), Wiederholung auto"
            ]
        );
        // Without restatements, sung once, under a profile that forbids them.
        let once = around_dorian(
            CounterpointOptions {
                repeat: Repeat::None,
                eleison: Eleison::Three,
                ..free("kyrie")
            },
            Lang::En,
        )
        .unwrap();
        let sung = once.request.text.as_ref().unwrap();
        assert_eq!((sung.text.syllables.len(), sung.rate), (6, 33));
        assert_eq!(
            changes(&once),
            ["[TXT-REPEAT]", "short = \"V\"", "long = \"V\""]
        );
        assert_eq!(
            counterpoint_summary(&once, &[], Lang::En)[1],
            "Text: Kyrie I (6 syllables, 1 sense unit), repetition none"
        );
        // An own text brings its warnings.
        let own = around_dorian(free("Ad te levavi animam meam."), Lang::De).unwrap();
        assert_eq!(
            own.warnings,
            ["Akzent unsicher: animam → á-ni-mam; mit Akut angeben"]
        );
    }

    #[test]
    fn texts_that_free_voices_cannot_sing_around_a_cantus() {
        let message = |options: CounterpointOptions, lang| {
            around_dorian(options, lang).unwrap_err().to_string()
        };
        // Only free counterpoint sings.
        let species = CounterpointOptions {
            species: 2,
            text: "kyrie".into(),
            ..CounterpointOptions::default()
        };
        assert_eq!(
            message(species.clone(), Lang::De),
            "Einen Text singen nur freie Gegenstimmen (Gattung 0)."
        );
        assert_eq!(
            message(species, Lang::En),
            "Only free counterpoint (species 0) sings a text."
        );
        // The 36 syllables of the first five sentences of the Gloria need 19 measures, a minim
        // beat for each besides the second half of the last measure.
        let gloria = CounterpointOptions {
            species: 0,
            text: "gloria:1-5".into(),
            ..CounterpointOptions::default()
        };
        assert_eq!(
            message(gloria.clone(), Lang::En),
            "The text needs at least 19 measures. The cantus has 11 measures. Remedies: a cantus \
             of at least 19 notes or a shorter text (such as fewer sense units like gloria:1-2)."
        );
        let text = |spec| text_of(spec, Eleison::Four, Lang::De).unwrap().unwrap().0;
        let dorian = ModeSpec::new(fux_core::Mode::Dorian);
        assert_eq!(least_cantus(&text("gloria:1-5"), dorian), 19);
        assert!(plan_around_cantus(&text("gloria:1-5"), dorian, 19, true).is_ok());
        assert_eq!(least_cantus(&text("kyrie"), dorian), MIN_MEASURES);
        // An unknown text, before a cantus is generated.
        let unknown = CounterpointOptions {
            species: 0,
            text: "kyrei".into(),
            ..CounterpointOptions::default()
        };
        let error = prepare_counterpoint(&unknown, &Profile::fux(), Lang::De).unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("Unbekannter Abschnitt „kyrei“"),
            "{error}"
        );
        // A search that gets stuck on the text says what may help.
        let stuck = GenerateError::NoCounterpoint {
            measure: 9,
            rules: vec!["TXT-COMPLETE"],
        };
        assert!(counterpoint_error(&stuck, true, Lang::De).ends_with(
            "(vor allem TXT-COMPLETE). Abhilfe: --repeat frei, ein kürzerer Text, ein \
                 längerer Cantus oder andere Binnenschlüsse (--cadences)."
        ));
        assert_eq!(
            counterpoint_error(&stuck, false, Lang::En),
            stuck.message(Lang::En)
        );
    }

    #[test]
    fn positions_name_the_syllable_sung() {
        let note = |step, syllable: Option<&str>| Note {
            syllable: syllable.map(str::to_string),
            ..Note::new(Pitch::new(step, 0, 4), Duration::HALF)
        };
        // Ký- with a melisma, a rest, an entry without a syllable, then ri-.
        let notes = vec![
            note(Step::D, Some("Ký-")),
            note(Step::E, None),
            Note::rest(Duration::HALF),
            note(Step::F, None),
            note(Step::G, Some("ri-")),
            note(Step::F, None),
            Note::rest(Duration::HALF),
            Note::rest(Duration::HALF),
        ];
        let voice = Voice::new(VoiceType::Soprano, Role::Counterpoint(Species::Free), notes);
        let mut score = Score::new("dorian".parse().unwrap(), vec![voice]);
        let at = |score: &Score, note: usize, lang| {
            let violation = Violation {
                rule: "TXT-ONSET",
                level: Level::P1,
                voice: 0,
                note,
                other: None,
                tick: 0,
                detail: Detail::None,
            };
            violation_position(score, &violation, lang)
        };
        // Without a text the positions stay as they were.
        assert_eq!(at(&score, 0, Lang::De), "Takt 1");
        let (kyrie, _) = resolve("kyrie", &TextOptions::default()).unwrap();
        score.text = Some(ScoreText {
            text: Arc::new(kyrie),
            voices: vec![true],
        });
        assert_eq!(at(&score, 0, Lang::De), "Takt 1 („Ký-“)");
        assert_eq!(at(&score, 1, Lang::En), "Measure 1, beat 3 (\"Ký-\")");
        // A rest names the syllable it ends; an attack after it without a syllable none.
        assert_eq!(at(&score, 2, Lang::De), "Takt 2 („Ký-“)");
        assert_eq!(at(&score, 3, Lang::De), "Takt 2, Viertel 3");
        assert_eq!(at(&score, 5, Lang::De), "Takt 3, Viertel 3 („ri-“)");
        assert_eq!(at(&score, 7, Lang::De), "Takt 4, Viertel 3 („ri-“)");
        // A voice that does not sing the text.
        score.text.as_mut().unwrap().voices = vec![false];
        assert_eq!(at(&score, 0, Lang::De), "Takt 1");
    }
}
