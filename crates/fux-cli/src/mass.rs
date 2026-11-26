//! `fux mass` (phase 14): a mass after the forms of Palestrina's masses, planned before any
//! search and set part by part ([`fux_core::mass`]). This module holds what the program, the
//! window and the web version share: the options ([`MassOptions`]), the request and the profile
//! they make ([`prepare_mass`]), the plan as text ([`mass_summary`]), the setting of the parts
//! ([`run_mass`]) and the lines that report them ([`PartLine`], [`Eta`], [`cadence_plan`]), the
//! files of the finished parts ([`part_path`], [`write_part`]), the files of the mass
//! ([`write_mass`]), the exit code ([`exit_code`]) and the check of the file of a movement
//! ([`check_movement`]).
//!
//! **Persistence** (D16). Every part is written to `NAME.teile/` as soon as it is set, with what
//! the plan said of it and of its mass and the version of the generator that set it (`fux-part`,
//! `fux-mass`). `--resume` takes up the parts there that fit the plan, their seed and the profile
//! and that this version set ([`mass::GENERATOR`]), and sets the others; `--only` sets the
//! parts it names again and takes the others from there. Both go on with the mass the files
//! record, its form, mode, choir, seed, chant and head motif. With `--only` a seed is the new seed
//! of the parts it names; `--resume` with a seed whose options plan another mass than the one
//! recorded is refused, since it would set that mass over the parts kept, and so is `--resume`
//! with options that plan a part otherwise than its file keeps it in a movement whose parts are
//! all there (other `--imitation`, `--eleison`, `--repeat`, …), or where another version of the
//! generator set a part there, since it would set that part again over its file. The head motif
//! is the mass's whichever movements a run sets (MASS-MOTIF): it comes from Kyrie I, so the
//! movements of such a mass include the Kyrie, whose parts come from their files. Where Kyrie I
//! is set again, so is every part that takes its head motif, and a part taken from its file must
//! have taken the motif that Kyrie I gives. `--rewrite` writes the files of a finished mass anew
//! from its parts as they are, with the plan their files keep, whatever the options would plan
//! now ([`fux_core::mass::mass_of_parts`]).
//! Ctrl-C ends the program: the parts set are on disk already.
//!
//! **Files** (docs/design/phase-14/spec.md §§ 6 and 7): `NAME.txt` with all that `fux mass`
//! prints; per movement `NAME-3-credo.musicxml` and `NAME-3-credo.mid`, its sections one after
//! the other, the MusicXML file with the fields of its parts, from which `fux check` repeats the
//! report of every part ([`check_movement`]); the book `NAME.ly` and `NAME.pdf`, a book part per
//! movement and a score per section ([`fux_export::mass_book`]). LilyPond runs last, so that its
//! failure loses no other file. A movement that lacks a part has no file of its own: the book
//! shows where the part is missing, and why, and how to set it ([`write_mass`]).

use std::io::{IsTerminal, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail};
use fux_core::generate::{GenerateError, Rng, SearchControl, Warning};
use fux_core::mass::{
    self, FIELD_MASS, FIELD_PART, FieldError, Join, KeptError, KeptPart, Mass, MassEvent,
    MassField, MassOptionError, MassPlan, MassRequest, Motif, Movement, MovementPlan, Opening,
    PartField, PartPlan, PartRef, PartResult, SectionPlan, fields_of_part, generate_part,
    mass_field, mass_of_parts, movement_parts, movement_score, parse_chant, parse_chunk,
    parse_endings, parse_imitation, parse_movements, parse_scoring, part_chant, part_fields,
    part_from_parts, part_profile, part_request, plan_mass, profile_hash, split_movement, tables,
    voice_names,
};
use fux_core::rules::{Checker, Profile, Report};
use fux_core::text::plan::CadenceChain;
use fux_core::text::{Text, TextOptions};
use fux_core::{
    Cadence, Lang, Lyric, ModeSpec, Note, Pitch, Score, SectionKind, SectionMark, Step, Voice,
    VoiceType,
};
use fux_export::{
    FIELD_TEXT, LilypondOptions, MassBookOptions, MovementHeading, MovementOptions,
    MusicXmlOptions, from_musicxml, mass_book, to_movement_midi, to_movement_musicxml,
    to_musicxml_with_fields,
};

use crate::commands::{
    localize_names, marked_sheet_after, print_files, print_violations_after, report_line, time_seed,
};
use crate::free::{PreparedFree, Repeat, SingingArgs, free_summary, parse_parts};
use crate::imitation::{ImitationSource, attach, attach_after};
use crate::input::{ReadScore, musicxml_text_fields};
use crate::output::{self, Format, OutputArgs, unwritable, with_extension};
use crate::text::align_lyrics;

/// The options of `fux mass` (docs/design/phase-14/spec.md § 1.4), as the window and the web
/// version give them too. What names parts of the plan stays text until the plan is made, so
/// that a mistake in it is told in the user's language ([`prepare_mass`], [`only_parts`]).
#[derive(clap::Args, Clone, Debug, PartialEq)]
pub struct MassOptions {
    /// Form der Messe: brevis (nach Palestrinas Missa brevis und den kurzen vierstimmigen Messen)
    /// oder normal (nach den Mittelwerten seiner vierstimmigen Messen)
    #[arg(long, value_name = "FORM", default_value = "brevis")]
    pub preset: String,
    /// Sätze, kommagetrennt: kyrie, gloria, credo, sanctus, benedictus, agnus
    #[arg(
        long,
        value_name = "SÄTZE",
        default_value = "kyrie,gloria,credo,sanctus,benedictus,agnus"
    )]
    pub movements: String,
    /// Anzahl der Stimmen (2–6)
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u8).range(2..=6))]
    pub voices: u8,
    /// Besetzung von oben nach unten, z. B. "S A T B" (sonst S T, A T B, S A T B, S A T T B bzw.
    /// S S A T T B)
    #[arg(long, value_name = "STIMMEN")]
    pub parts: Option<String>,
    /// Modus der Messe, z. B. dorisch, phrygisch, ionisch-transponiert (sonst dorisch)
    #[arg(long)]
    pub mode: Option<ModeSpec>,
    /// Besetzung einzelner Abschnitte, durch ; getrennt, z. B. "benedictus=S A T; agnus3=S A T T
    /// B" (sonst wie in Palestrinas Messen: das Benedictus ohne Bass, Agnus Dei II mit einer
    /// Stimme mehr)
    #[arg(long, value_name = "ABSCHNITT=STIMMEN")]
    pub scoring: Option<String>,
    /// Neubeginn der Teile in Imitation: auto (Standard, nach Palestrinas Messen), an oder aus,
    /// allein für alle Teile oder für Stellen (Satz/Teil wie credo/3, Abschnitt wie christe,
    /// Halteklang wie incarnatus), durch ; getrennt, z. B. "auto; incarnatus=aus; credo/3=an"
    #[arg(long, value_name = "STELLE=WERT")]
    pub imitation: Option<String>,
    /// Schlussstufen einzelner Teile (sonst gezogen; ein Abschnitt nennt seinen letzten Teil),
    /// durch ; getrennt, z. B. "christe=A; gloria/3=D"
    #[arg(long, value_name = "STELLE=STUFE")]
    pub endings: Option<String>,
    /// Größte Zahl der Silben eines Teils von Gloria und Credo (20–200), oder aus: ein Teil von
    /// Halteklang zu Halteklang
    #[arg(long, value_name = "SILBEN|AUS", default_value_t = tables::CHUNK.to_string())]
    pub chunk: String,
    /// Länge der kurzen Abschnitte: palestrina (Benedictus und Agnus Dei I mit zwei
    /// Imitationspunkten mehr) oder natürlich
    #[arg(long, value_name = "LÄNGEN", default_value = "palestrina")]
    pub lengths: String,
    /// Kopfmotiv: an (Gloria, Credo, Sanctus und Agnus Dei I beginnen mit dem ersten Soggetto des
    /// Kyrie I, seine Töne neu auf ihren Text deklamiert) oder aus
    #[arg(
        long,
        value_name = "AN|AUS",
        default_value = "aus",
        action = clap::ArgAction::Set,
        value_parser = motif
    )]
    pub motif: bool,
    /// Choral des Kyriale, den die Soggetti paraphrasieren: keiner, ix (Messe IX Cum jubilo mit
    /// Credo I) oder iv (Kyrie IV Cunctipotens genitor Deus, die übrigen Sätze wie ix); ohne
    /// --mode steht jeder Satz im Modus seines Chorals
    #[arg(long, value_name = "KEINER|IX|IV", default_value = "keiner")]
    pub chant: String,
    #[command(flatten)]
    pub singing: SingingArgs,
    /// Startwert des Zufalls (sonst zufällig; er wird ausgegeben); mit --only der eigene Seed
    /// der genannten Teile
    #[arg(long)]
    pub seed: Option<u64>,
    /// Breite der Suche
    #[arg(long, default_value_t = 200)]
    pub beam: usize,
    /// Zufallsanteil bei der Auswahl (0 = immer die günstigste Lösung)
    #[arg(long, default_value_t = 2.0)]
    pub noise: f64,
    /// Titel der Messe (sonst Missa brevis bzw. Missa)
    #[arg(long, value_name = "TITEL")]
    pub title: Option<String>,
    /// Nur den Plan zeigen, ohne Suche: Sätze und Teile mit Neubeginn und Schlussstufe, die
    /// geschätzte Zeit
    #[arg(long)]
    pub plan: bool,
    /// Nur diese Teile neu setzen (Satz/Teil wie credo/3, Abschnitt oder Halteklang),
    /// kommagetrennt; die übrigen kommen aus NAME.teile/; mit --seed bekommen die genannten
    /// Teile einen eigenen Seed
    #[arg(long, value_name = "TEILE", value_delimiter = ',')]
    pub only: Vec<String>,
    /// Teile wiederverwenden, die NAME.teile/ schon hat (gleicher Plan, Seed, Regelprofil und
    /// Generatorversion); ohne --seed kommen Form, Modus, Besetzung, Seed, Choral und Kopfmotiv
    /// von dort, mit --seed müssen sie dazu passen; die übrigen Angaben müssen die beim Setzen
    /// sein
    #[arg(long)]
    pub resume: bool,
    /// Die Dateien der Messe neu schreiben aus den Teilen in NAME.teile/, wie sie sind: ohne zu
    /// setzen und ohne Plan und Regelprofil zu vergleichen, mit den Angaben des Notenbilds; Form,
    /// Plan und Seeds kommen aus den Dateien, --movements wählt Sätze aus
    #[arg(long)]
    pub rewrite: bool,
    /// Je Teil die Zusammenfassung wie bei fux free (Text, Punkte, Binnenschlüsse, Schluss); mit
    /// --plan je Teil eine Zeile mit Stelle, Seed und Schätzung
    #[arg(long)]
    pub details: bool,
    /// Teile, die zugleich gesetzt werden (sonst die Hälfte der Threads des Rechners, höchstens 4);
    /// 1 setzt sie nacheinander, die Noten sind dieselben
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u8).range(1..))]
    pub jobs: Option<u8>,
    #[command(flatten)]
    pub notation: MassNotation,
}

/// How the files of a mass print it (docs/design/phase-14/spec.md § 1.4), besides the clefs and
/// the brackets over the entries, which `--clefs` and `--marks` set as for every command.
#[derive(clap::Args, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MassNotation {
    /// Mensurstriche: Taktstriche nur zwischen den Systemen und über sie gebundene Noten, wo es
    /// geht, als ein Wert (im Buch; in MusicXML Taktstriche zwischen den Systemen)
    #[arg(long)]
    pub mensurstriche: bool,
    /// Intonation des Priesters vor Gloria und Credo („Intonatio: Glória in excélsis Deo.“): text
    /// oder aus
    #[arg(
        long,
        value_name = "TEXT|AUS",
        default_value = "text",
        action = clap::ArgAction::Set,
        value_parser = intonation
    )]
    pub intonation: bool,
    /// Stimmen, die einen Abschnitt lang schweigen, in MusicXML: zeigen (mit Ganztaktpausen) oder
    /// verbergen (für Verovio; das Buch zeigt ohnehin nur die Stimmen, die singen)
    #[arg(
        long = "tacet",
        value_name = "ZEIGEN|VERBERGEN",
        default_value = "zeigen",
        action = clap::ArgAction::Set,
        value_parser = tacet
    )]
    pub hide_tacet: bool,
}

impl Default for MassNotation {
    /// As the options leave it: the bar lines through the staves, the intonation printed, the
    /// voices that rest through a section shown.
    fn default() -> Self {
        MassNotation {
            mensurstriche: false,
            intonation: true,
            hide_tacet: false,
        }
    }
}

/// `text` or `aus` (`off`), as `--intonation` takes it: whether the intonation is printed.
pub fn intonation(text: &str) -> Result<bool, String> {
    match text.trim().to_lowercase().as_str() {
        "text" => Ok(true),
        "aus" | "off" => Ok(false),
        _ => Err(format!("{text:?}: text or aus (off)")),
    }
}

/// `zeigen` or `verbergen` (`show`, `hide`), as `--tacet` takes it: whether the voices that rest
/// through a section are hidden there.
pub fn tacet(text: &str) -> Result<bool, String> {
    match text.trim().to_lowercase().as_str() {
        "zeigen" | "show" => Ok(false),
        "verbergen" | "hide" => Ok(true),
        _ => Err(format!("{text:?}: zeigen (show) or verbergen (hide)")),
    }
}

/// `an` or `aus` (`on`, `off`), as `--motif` takes it: whether Gloria, Credo, Sanctus and Agnus
/// Dei I open with the head motif of Kyrie I (MASS-MOTIF).
pub fn motif(text: &str) -> Result<bool, String> {
    match text.trim().to_lowercase().as_str() {
        "an" | "on" => Ok(true),
        "aus" | "off" => Ok(false),
        _ => Err(format!("{text:?}: an (on) or aus (off)")),
    }
}

impl Default for MassOptions {
    /// As `fux mass` without options: a Missa brevis in the Dorian mode for S A T B, all six
    /// movements, parts of at most 45 syllables, Palestrina's lengths.
    fn default() -> Self {
        MassOptions {
            preset: mass::Preset::default().id().to_string(),
            movements: Movement::ALL.map(Movement::id).join(","),
            voices: 4,
            parts: None,
            mode: None,
            scoring: None,
            imitation: None,
            endings: None,
            chunk: tables::CHUNK.to_string(),
            lengths: mass::Lengths::default().id().to_string(),
            motif: false,
            chant: "keiner".to_string(),
            singing: SingingArgs::default(),
            seed: None,
            beam: 200,
            noise: 2.0,
            title: None,
            plan: false,
            only: Vec::new(),
            resume: false,
            rewrite: false,
            details: false,
            jobs: None,
            notation: MassNotation::default(),
        }
    }
}

/// The most parts that `fux mass` and the window set at once unless `--jobs` says otherwise
/// (K6): a search alone gains little from more than four threads (2.72 times one thread with 4,
/// 3.32 with 16), so four parts at once fill a machine of eight cores with sixteen threads. A
/// four-voice search alone takes about 0.2 GB, and example 11 took at most 0.62 GB with four
/// parts at once (0.51 GB one at a time).
pub const JOBS_MAX: usize = 4;

/// The parts that `fux mass` and the window set at once unless `--jobs` says otherwise: half
/// the threads the machine offers (`std::thread::available_parallelism`, on a machine with eight
/// cores 8 or 16), at least one and at most [`JOBS_MAX`]; one where the machine does not say.
pub fn default_jobs() -> usize {
    std::thread::available_parallelism().map_or(1, |threads| (threads.get() / 2).clamp(1, JOBS_MAX))
}

/// An option of a mass that the plan cannot have, as an error in the user's language: the
/// sentence of [`MassOptionError::message`] completed with its full stop and capital, as
/// `GenerateError::message` completes the errors of the options of an imitation.
fn option_error(error: &MassOptionError, lang: Lang) -> anyhow::Error {
    sentence(error.message(lang))
}

/// A message without its full stop, as the mass layer words them, as an error: with its capital
/// and its full stop.
fn sentence(message: String) -> anyhow::Error {
    let mut chars = message.chars();
    let capitalized: String = match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => message,
    };
    anyhow!("{capitalized}.")
}

/// Builds the request of `fux mass` and the profile its parts are set and checked with: the
/// choir, the movements and the changes to the plan read from `options` (each mistake in the
/// user's language), the seed from the clock unless given; the profile `base` (the one chosen,
/// Fux's by default) with the overlay of the mass ([`fux_core::mass::profile`]), then the single
/// overrides `rules` (`--rule`), then those of `--repeat` ([`mass_profile`]). Whether the changes
/// name parts the plan has, [`mass_plan`] says.
pub fn prepare_mass(
    options: &MassOptions,
    base: &Profile,
    rules: &[String],
    lang: Lang,
) -> Result<(MassRequest, Profile)> {
    let failed = |error: MassOptionError| option_error(&error, lang);
    let choir = match &options.parts {
        Some(text) => parse_parts(text, lang)?,
        None => mass::choir(usize::from(options.voices)),
    };
    let preset = options.preset.parse().map_err(failed)?;
    let movements = parse_movements(&options.movements).map_err(failed)?;
    let mut overrides = Vec::new();
    if let Some(scoring) = &options.scoring {
        overrides.extend(parse_scoring(scoring).map_err(failed)?);
    }
    if let Some(imitation) = &options.imitation {
        overrides.extend(parse_imitation(imitation).map_err(failed)?);
    }
    if let Some(endings) = &options.endings {
        overrides.extend(parse_endings(endings).map_err(failed)?);
    }
    let request = MassRequest {
        preset,
        mode: options.mode,
        choir,
        movements,
        overrides,
        chant: parse_chant(&options.chant).map_err(failed)?,
        motif: options.motif,
        lengths: options.lengths.parse().map_err(failed)?,
        chunk: parse_chunk(&options.chunk).map_err(failed)?,
        text: TextOptions {
            eleison: options.singing.eleison,
        },
        repeat: options.singing.repeat.into(),
        accents: options.singing.accents,
        seed: options.seed.unwrap_or_else(time_seed),
        beam: options.beam,
        noise: options.noise,
        title: options.title.clone(),
    };
    let profile = mass_profile(base, rules, options.singing.repeat)?;
    Ok((request, profile))
}

/// The profile of a mass: `base` with the overlay of the mass ([`fux_core::mass::profile`], D7:
/// IMI-HELD and IMI-SILENCE `thin` judge the parts without a plan of imitation at the levels of
/// the parts with one), then the single overrides `rules` (`--rule`), which may change the
/// overlay by its parameters (`IMI-HELD.unplanned_struck=P4`) or switch it off with its rule
/// (`IMI-HELD=aus`), then the overrides of `repeat`, in this order as for `fux free`. The search
/// of a part applies `--repeat` again ([`fux_core::mass::part_profile`]), which changes nothing.
pub fn mass_profile(base: &Profile, rules: &[String], repeat: Repeat) -> Result<Profile> {
    let mut profile = mass::profile(base);
    for spec in rules {
        profile
            .apply_override(spec)
            .with_context(|| format!("--rule {spec}"))?;
    }
    repeat.apply(&profile)
}

/// The plan of `request`, without a search, in milliseconds: an option that names what the plan
/// does not have, or asks what it cannot give, fails here in the user's language
/// ([`GenerateError::MassOption`]). The one place in the frontends that calls `plan_mass`.
pub fn mass_plan(request: &MassRequest, lang: Lang) -> Result<MassPlan> {
    plan_mass(request).map_err(|error| anyhow!("{}", error.message(lang)))
}

/// The parts that `--only` names, each once, in the order of the mass: a part by its number in
/// its movement (`credo/3`), every part of a section (`christe`) or the part that begins at a
/// Break (`incarnatus`). An address that names nothing fails in the user's language.
pub fn only_parts(plan: &MassPlan, only: &[String], lang: Lang) -> Result<Vec<PartRef>> {
    let mut parts = Vec::new();
    for at in only.iter().map(|at| at.trim()).filter(|at| !at.is_empty()) {
        let found = plan
            .find(at)
            .map_err(|error| option_error(&error, lang).context("--only"))?;
        parts.extend(found);
    }
    parts.sort();
    parts.dedup();
    Ok(parts)
}

/// A degree as the scale of `mode` spells it: H and B in German, B and Bb in English.
fn degree_name(mode: ModeSpec, step: Step, lang: Lang) -> String {
    Pitch::new(step, mode.scale_alter(step), 4).class_name(lang)
}

/// Voices from the top by their abbreviations: „S A T B“.
fn abbreviated(voices: &[VoiceType]) -> String {
    voices
        .iter()
        .map(|voice| voice.abbreviation())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The voices of a section, from the top.
fn section_voices(movement: &MovementPlan, section: &SectionPlan) -> Vec<VoiceType> {
    section.voices.iter().map(|&v| movement.voices[v]).collect()
}

/// A number with its noun: „1 Teil“, „26 Teile“.
fn counted(n: usize, de: (&str, &str), en: (&str, &str), lang: Lang) -> String {
    let (one, many) = match lang {
        Lang::De => de,
        Lang::En => en,
    };
    format!("{n} {}", if n == 1 { one } else { many })
}

/// A number with one decimal, as the user's language writes it: „8,8“, "8.8".
pub fn decimal(value: f64, lang: Lang) -> String {
    let text = format!("{value:.1}");
    match lang {
        Lang::De => text.replace('.', ","),
        Lang::En => text,
    }
}

/// A time as a clock shows it, to the second: „3:40“, „1:02:05“.
pub fn clock(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    let (hours, minutes, seconds) = (total / 3600, total / 60 % 60, total % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// An estimated time, as roughly as it is known: in seconds below a minute, else in minutes:
/// „etwa 40 s“, „etwa 6 min“.
pub fn about(seconds: f64, lang: Lang) -> String {
    let word = lang.pick("etwa", "about");
    if seconds < 59.5 {
        format!("{word} {} s", seconds.round().max(1.0))
    } else {
        format!("{word} {} min", (seconds / 60.0).round())
    }
}

/// How the lines of a mass name a part: a section of one part by its title („Kyrie I“,
/// „Sanctus“), a part that begins at a Break by its label („Et incarnatus est“), every other
/// part of a long section by its text („credo:5-7“).
pub fn part_label(plan: &MassPlan, at: PartRef) -> String {
    label(
        &plan.movements[at.movement].sections[at.section],
        plan.part(at),
    )
}

fn label(section: &SectionPlan, part: &PartPlan) -> String {
    match part.join {
        Join::Break(label) => label.to_string(),
        _ if section.parts.len() == 1 => section.title.to_string(),
        _ => part.text.clone(),
    }
}

/// The plan of a mass as `fux mass --plan` prints it (docs/design/phase-14/spec.md § 3.10),
/// without a search:
/// - a line with the title (the form after a title of one's own), the mode, the choir and the
///   seed, the number of movements, sections, parts and syllables, and the principal degree;
/// - a line with the estimate: measures, time natively with `jobs` parts set at once
///   ([`native_seconds`]) and in the browser with four workers;
/// - per movement a line with its parts, each with how it begins and the degree it ends on: ‖
///   between sections, · between the parts of a section; a part that begins at a Break carries
///   its label, a section in other voices than the choir names them.
///
/// The lines are joined by line breaks, without one at the end.
pub fn mass_summary(plan: &MassPlan, jobs: usize, lang: Lang) -> String {
    let total = plan.estimate();
    let browser = f64::from(plan.pool_seconds(4, tables::ESTIMATE_WASM));
    let (native, browser) = (
        about(native_seconds(plan, jobs), lang),
        about(browser, lang),
    );
    let estimate = match lang {
        Lang::De => format!(
            "geschätzt {} Takte, {native} (Browser, 4 Worker: {browser})",
            total.measures
        ),
        Lang::En => format!(
            "estimated {} measures, {native} (browser, 4 workers: {browser})",
            total.measures
        ),
    };
    plan_lines(plan, estimate, lang).join("\n")
}

/// The lines of the plan of a mass as [`mass_summary`] prints them, with `second` as the second
/// line: the estimate of a mass to set, or where the parts of a mass written anew come from
/// (`--rewrite`).
fn plan_lines(plan: &MassPlan, second: String, lang: Lang) -> Vec<String> {
    let (mode, several) = mode_name(plan, lang);
    let title = if plan.title == plan.preset.title() {
        plan.title.clone()
    } else {
        format!(
            "{} ({} {})",
            plan.title,
            lang.pick("Form", "form"),
            plan.preset.id()
        )
    };
    let principal = plan
        .movements
        .first()
        .map(|movement| degree_name(movement.mode, plan.principal, lang))
        .unwrap_or_default();
    let mut choir = plan.choir.clone();
    choir.sort();
    let mut lines = vec![format!(
        "{title} – {mode}, {}, {} {}: {}, {}, {}, {}, {} {principal}",
        abbreviated(&choir),
        lang.pick("Seed", "seed"),
        plan.seed,
        counted(
            plan.movements.len(),
            ("Satz", "Sätze"),
            ("movement", "movements"),
            lang
        ),
        counted(
            plan.section_count(),
            ("Abschnitt", "Abschnitte"),
            ("section", "sections"),
            lang
        ),
        counted(
            plan.part_count(),
            ("Teil", "Teile"),
            ("part", "parts"),
            lang
        ),
        counted(
            plan.syllables(),
            ("Silbe", "Silben"),
            ("syllable", "syllables"),
            lang
        ),
        lang.pick("Hauptstufe", "principal degree"),
    )];
    lines.push(second);
    for movement in &plan.movements {
        lines.push(movement_line(movement, &choir, several, lang));
    }
    if plan.chant.is_some() {
        lines.push(chant_line(plan, lang));
    }
    lines
}

/// The seconds the search of the parts of `plan` takes natively with `jobs` of them set at once
/// (K6), as [`run_mass`] sets them, in the order of the pool of the web version
/// ([`start_order`], [`MassPlan::pool_order`]): the pool's estimate ([`MassPlan::pool_seconds`]),
/// each part taking [`tables::estimate_jobs`] times as long as alone beside the others, but no
/// more than the sum of the parts' estimates ([`MassPlan::estimate`]), which one job takes. The
/// factor is fitted on the parts of a Missa brevis in four voices; small parts, which use fewer
/// threads alone, gain more from being set at once (a mass in two voices took 0.65 of its time
/// with four jobs, where the factor alone would make it longer).
pub fn native_seconds(plan: &MassPlan, jobs: usize) -> f64 {
    let sum = f64::from(plan.estimate().seconds);
    match jobs.clamp(1, plan.part_count().max(1)) {
        1 => sum,
        jobs => f64::from(plan.pool_seconds(jobs, tables::estimate_jobs(jobs))).min(sum),
    }
}

/// The chants of the Kyriale that the movements of a mass on chant paraphrase (MASS-CHANT), as
/// the plan of the mass names them: „Choral: Kyrie IX, Gloria IX, Credo I, Sanctus IX, Agnus Dei
/// IX“, a chant that two movements share (the Sanctus' with the Benedictus) once.
fn chant_line(plan: &MassPlan, lang: Lang) -> String {
    let mut names: Vec<String> = Vec::new();
    for (m, movement) in plan.movements.iter().enumerate() {
        let first = (0..movement.sections.len()).find_map(|section| {
            let at = PartRef {
                movement: m,
                section,
                part: 0,
            };
            (!movement.sections[section].parts.is_empty()).then_some(at)
        });
        let name = first
            .and_then(|at| part_chant(plan, at))
            .and_then(|chant| chant.name.clone());
        if let Some(name) = name
            && !names.contains(&name)
        {
            names.push(name);
        }
    }
    format!("{}: {}", lang.pick("Choral", "Chant"), names.join(", "))
}

/// The mode of a mass as its lines name it: the one mode of its movements, or „Modus je Satz“
/// where they have several (with a chant, phase 14c); and whether they have.
fn mode_name(plan: &MassPlan, lang: Lang) -> (String, bool) {
    let mut modes: Vec<ModeSpec> = Vec::new();
    for movement in &plan.movements {
        if !modes.contains(&movement.mode) {
            modes.push(movement.mode);
        }
    }
    match modes.as_slice() {
        [one] => (one.name(lang), false),
        _ => (
            lang.pick("Modus je Satz", "mode per movement").to_string(),
            true,
        ),
    }
}

/// The line of a movement in [`mass_summary`]: „Kyrie: Kyrie I (Imitation) → D ‖ Christe
/// (Imitation) → A ‖ …“, „Credo: credo:1-4 (Imitation) → A · credo:5-7 (gemeinsam, Halbe Atem)
/// → D · … · Et incarnatus est (gemeinsam, Takt Pause) → A · …“. A section in other voices than
/// `choir` names them at its first part; „Osanna ut supra“ repeats the Osanna of the Sanctus and
/// has no parts. With `mode`, the mode of the movement follows its title.
fn movement_line(movement: &MovementPlan, choir: &[VoiceType], mode: bool, lang: Lang) -> String {
    let sections: Vec<String> = movement
        .sections
        .iter()
        .map(|section| {
            if section.ut_supra.is_some() {
                return section.title.to_string();
            }
            let voices = section_voices(movement, section);
            let parts: Vec<String> = section
                .parts
                .iter()
                .enumerate()
                .map(|(p, part)| {
                    let mut how = Vec::new();
                    if p == 0 && voices != choir {
                        how.push(abbreviated(&voices));
                    }
                    how.push(part.opening.name(lang).to_string());
                    if part.motif {
                        how.push(lang.pick("Kopfmotiv", "head motif").into());
                    }
                    match part.join {
                        Join::Seam => how.push(lang.pick("Halbe Atem", "minim breath").into()),
                        Join::Break(_) => {
                            how.push(lang.pick("Takt Pause", "measure rest").into());
                        }
                        Join::Start | Join::Section => {}
                    }
                    format!(
                        "{} ({}) → {}",
                        label(section, part),
                        how.join(", "),
                        degree_name(movement.mode, part.ending, lang)
                    )
                })
                .collect();
            parts.join(" · ")
        })
        .collect();
    let title = movement.movement.title();
    let title = if mode {
        format!("{title} ({})", movement.mode.name(lang))
    } else {
        title.to_string()
    };
    format!("{title}: {}", sections.join(" ‖ "))
}

/// One line per part of a plan, as `fux mass --plan --details` prints them, for the addresses of
/// `--only`, `--imitation` and `--endings`: „credo/2: credo:5-7 (gemeinsam), S A T B, Schluss D,
/// Seed 4711; geschätzt 31 Takte, etwa 9 s“.
pub fn plan_details(plan: &MassPlan, lang: Lang) -> Vec<String> {
    plan.parts()
        .map(|(at, part)| {
            let line = PartLine::of(plan, at, lang);
            format!(
                "{}: {}; {} {} {}, {}",
                plan.address(at),
                line.description(lang),
                lang.pick("geschätzt", "estimated"),
                part.estimate.measures,
                lang.pick("Takte", "measures"),
                about(f64::from(part.estimate.seconds), lang)
            )
        })
        .collect()
}

/// A part as the lines of a mass name it while the mass is set (§ 7), in plain values, so that
/// the program, the window and the web version word it alike.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartLine {
    /// Its number in the order of the mass, from 1, and the number of parts.
    pub number: usize,
    pub total: usize,
    /// The title of its movement: „Credo“.
    pub movement: String,
    /// The part as [`part_label`] names it.
    pub label: String,
    pub opening: Opening,
    /// Its voices from the top, abbreviated: „S A T B“.
    pub voices: String,
    /// The degree it ends on, as its mode spells it: „D“.
    pub ending: String,
    /// The seed of the part, as `--plan --details` shows it: the plan's, or the one `--only …
    /// --seed` gives it. Its attempts search with seeds made from it (MASS-SEED); the file of the
    /// part records the one that set it.
    pub seed: u64,
}

impl PartLine {
    /// The part `at` of `plan`, with the seed the plan gives it.
    pub fn of(plan: &MassPlan, at: PartRef, lang: Lang) -> PartLine {
        let movement = &plan.movements[at.movement];
        let section = &movement.sections[at.section];
        let part = plan.part(at);
        PartLine {
            number: plan
                .parts()
                .position(|(other, _)| other == at)
                .map_or(0, |i| i + 1),
            total: plan.part_count(),
            movement: movement.movement.title().to_string(),
            label: label(section, part),
            opening: part.opening,
            voices: abbreviated(&section_voices(movement, section)),
            ending: degree_name(movement.mode, part.ending, lang),
            seed: part.seed,
        }
    }

    /// The part as `result` sets it, a part of `plan`: with the voices it sings, which after a
    /// fallback of the scoring are the choir of the mass instead of its section's (Agnus Dei II).
    pub fn set(plan: &MassPlan, result: &PartResult, lang: Lang) -> PartLine {
        let movement = &plan.movements[result.at.movement];
        let voices: Vec<VoiceType> = result.voices.iter().map(|&v| movement.voices[v]).collect();
        PartLine {
            voices: abbreviated(&voices),
            ..PartLine::of(plan, result.at, lang)
        }
    }

    /// The part with how it begins, its voices, its ending and its seed: „credo:5-7
    /// (gemeinsam), S A T B, Schluss D, Seed 4711“.
    pub fn description(&self, lang: Lang) -> String {
        format!(
            "{} ({}), {}, {} {}, {} {}",
            self.label,
            self.opening.name(lang),
            self.voices,
            lang.pick("Schluss", "ending"),
            self.ending,
            lang.pick("Seed", "seed"),
            self.seed
        )
    }

    /// The part counted in the mass, with its movement: „[ 7/26] Credo · credo:5-7 (gemeinsam),
    /// S A T B, Schluss D, Seed 4711“. The number takes as many places as the total.
    pub fn heading(&self, lang: Lang) -> String {
        let width = self.total.to_string().len();
        format!(
            "[{:>width$}/{}] {} · {}",
            self.number,
            self.total,
            self.movement,
            self.description(lang)
        )
    }

    /// The line on stderr when the part is set: „[ 7/26] Credo · credo:5-7 (gemeinsam), S A T
    /// B, Schluss D, Seed 4711: 31 Takte, Kosten 71, 8,8 s – noch etwa 3:40“, with the seconds
    /// the mass still takes ([`Eta`]); without them for the last part.
    pub fn finished(
        &self,
        measures: usize,
        cost: u32,
        seconds: f64,
        remaining: Option<f64>,
        lang: Lang,
    ) -> String {
        let mut line = format!(
            "{}: {measures} {}, {} {cost}, {} s",
            self.heading(lang),
            lang.pick("Takte", "measures"),
            lang.pick("Kosten", "cost"),
            decimal(seconds, lang)
        );
        if let Some(remaining) = remaining {
            line.push_str(&match lang {
                Lang::De => format!(" – noch etwa {}", clock(remaining)),
                Lang::En => format!(" – about {} left", clock(remaining)),
            });
        }
        line
    }

    /// The line of a part taken from its file (`--resume`, `--only`) instead of searched: „[
    /// 7/26] Credo · credo:5-7 (gemeinsam), S A T B, Schluss D, Seed 4711: 31 Takte, Kosten 71,
    /// aus out/messe.teile“.
    pub fn taken(&self, measures: usize, cost: u32, from: &str, lang: Lang) -> String {
        format!(
            "{}: {measures} {}, {} {cost}, {} {from}",
            self.heading(lang),
            lang.pick("Takte", "measures"),
            lang.pick("Kosten", "cost"),
            lang.pick("aus", "from")
        )
    }

    /// The line of a part that the mass lacks, with why: „[ 7/26] Credo · … Seed 4711: fehlt:
    /// Teil credo/2 („…“) ließ sich in 3 Versuchen nicht setzen. …“.
    pub fn missing(&self, reason: &str, lang: Lang) -> String {
        format!(
            "{}: {} {reason}",
            self.heading(lang),
            lang.pick("fehlt:", "missing:")
        )
    }

    /// The line on stderr when an attempt (from 0) at the part fails, with why: „[ 7/26] Credo ·
    /// … Seed 4711: Versuch 1 gescheitert: Keine Lösung: …“.
    pub fn failed(&self, attempt: u8, reason: &str, lang: Lang) -> String {
        let attempt = u32::from(attempt) + 1;
        match lang {
            Lang::De => format!(
                "{}: Versuch {attempt} gescheitert: {reason}",
                self.heading(lang)
            ),
            Lang::En => format!("{}: attempt {attempt} failed: {reason}", self.heading(lang)),
        }
    }

    /// The status while the part is searched (§ 4.5), from the measure the search has reached
    /// and the measures the plan expects: „Teil 7/26 · Credo · Takt 23/≈40 · noch etwa 3:40“.
    pub fn running(
        &self,
        measure: u32,
        measures: usize,
        remaining: Option<f64>,
        lang: Lang,
    ) -> String {
        let mut line = format!(
            "{} {}/{} · {} · {} {measure}/≈{measures}",
            lang.pick("Teil", "Part"),
            self.number,
            self.total,
            self.movement,
            lang.pick("Takt", "measure")
        );
        if let Some(remaining) = remaining {
            line.push_str(&match lang {
                Lang::De => format!(" · noch etwa {}", clock(remaining)),
                Lang::En => format!(" · about {} left", clock(remaining)),
            });
        }
        line
    }
}

/// The time a mass still takes (§ 4.5): the plan's estimates of the parts not yet set, scaled by
/// how long the parts searched so far took against their own estimates, so that on a machine
/// faster or slower than the one the estimates were fitted on the time soon reads right. Parts
/// taken from files (`--resume`) are not searched and not recorded. Where several parts are set
/// at once, the estimates are those of a part beside the others ([`tables::estimate_jobs`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Eta {
    /// The estimates of the parts searched so far, in seconds.
    estimated: f64,
    /// The seconds their searches took.
    measured: f64,
}

impl Eta {
    /// A part searched for `seconds` against its `estimate`, set or failed.
    pub fn record(&mut self, estimate: f64, seconds: f64) {
        self.estimated += estimate.max(0.0);
        self.measured += seconds.max(0.0);
    }

    /// The seconds measured per second estimated so far; 1 before the first part.
    pub fn ratio(&self) -> f64 {
        if self.estimated > 0.0 {
            self.measured / self.estimated
        } else {
            1.0
        }
    }

    /// The seconds the mass still takes with `jobs` parts set at once, as [`run_mass`] sets them
    /// and `schedule()` of the pool of the web version counts them: the parts `running`, each with
    /// the seconds it has run, go on first; then each job, once free, takes the first of the parts
    /// `waiting` that may begin, in the order they begin, the order of the pool ([`start_order`]).
    /// A part that waits for the head motif may begin once Kyrie I is no longer running or
    /// waiting, and a job with only such parts left waits for it. Every estimate is scaled by
    /// [`Eta::ratio`]; a running part beyond its scaled estimate is taken to finish at once. With
    /// one job: the sum of what is left.
    pub fn remaining(&self, jobs: usize, running: &[(EtaPart, f64)], waiting: &[EtaPart]) -> f64 {
        let ratio = self.ratio();
        let mut free: Vec<f64> = running
            .iter()
            .map(|&(part, elapsed)| (part.seconds * ratio - elapsed).max(0.0))
            .collect();
        // When the head motif is known: once Kyrie I ends where it runs or waits, else at once.
        let mut motif = if waiting.iter().any(|part| part.source) {
            f64::INFINITY
        } else {
            0.0
        };
        if let Some(k) = running.iter().position(|(part, _)| part.source) {
            motif = free[k];
        }
        free.resize(free.len().max(jobs).max(1), 0.0);
        let mut waiting = waiting.to_vec();
        while !waiting.is_empty() {
            // The job free first, the first of equal ones.
            let (job, &earliest) = free
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.total_cmp(b.1))
                .expect("one job at least");
            let known = motif;
            let ready = |part: &EtaPart, time: f64| !part.waits || known <= time;
            let time = if waiting.iter().any(|part| ready(part, earliest)) {
                earliest
            } else {
                earliest.max(known)
            };
            let Some(i) = waiting.iter().position(|part| ready(part, time)) else {
                break;
            };
            let part = waiting.remove(i);
            free[job] = time + part.seconds.max(0.0) * ratio;
            if part.source {
                motif = free[job];
            }
        }
        free.into_iter().fold(0.0, f64::max)
    }
}

/// A part of a mass for [`Eta::remaining`], as `schedule()` of the pool of the web version takes
/// a job: the seconds the plan expects of it beside the others, and how it stands to the head
/// motif (MASS-MOTIF).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EtaPart {
    /// The seconds the plan expects of it beside the others ([`tables::estimate_jobs`]).
    pub seconds: f64,
    /// It is Kyrie I, which gives the head motif ([`MassPlan::motif_source`]).
    pub source: bool,
    /// It opens with the head motif, so it begins only once Kyrie I is set or has failed.
    pub waits: bool,
}

impl EtaPart {
    /// The part `at` of `plan`, expected to take `seconds`.
    pub fn of(plan: &MassPlan, at: PartRef, seconds: f64) -> EtaPart {
        EtaPart {
            seconds,
            source: plan.motif_source() == Some(at),
            waits: plan.part(at).motif,
        }
    }
}

/// The cadences of a set part for the cadence plan of its movement, in measures of the movement
/// (from 1), each with its target as the mode spells it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartCadences {
    /// The internal cadences: the measure their target arrives in, and the target.
    pub internal: Vec<(usize, String)>,
    /// The final cadence: the last measure of the part and the degree it ends on.
    pub ending: (usize, String),
}

/// The cadence plan of a movement (§ 7), one line: its parts in order, each with the join by
/// which it follows the part before (its [`PartPlan::join`]) and its cadences, `None` for a part
/// not set („…“). The cadences follow one another with `·`, also across a seam, where the ending
/// before it is marked „(Naht)“; a Break comes as `|`, a new section as `‖`: „Credo – Kadenzplan:
/// T. 7 A · T. 12 D (Naht) · T. 18 G · T. 25 A | T. 31 A ‖ …“.
pub fn cadence_plan(movement: &str, parts: &[(Join, Option<PartCadences>)], lang: Lang) -> String {
    let at = lang.pick("T.", "m.");
    let mut line = format!("{movement} – {}: ", lang.pick("Kadenzplan", "cadence plan"));
    for (i, (join, cadences)) in parts.iter().enumerate() {
        if i > 0 {
            line.push_str(match join {
                Join::Start | Join::Section => " ‖ ",
                Join::Break(_) => " | ",
                Join::Seam => " · ",
            });
        }
        let Some(cadences) = cadences else {
            line.push('…');
            continue;
        };
        let items: Vec<String> = cadences
            .internal
            .iter()
            .chain([&cadences.ending])
            .map(|(measure, target)| format!("{at} {measure} {target}"))
            .collect();
        line.push_str(&items.join(" · "));
        if parts
            .get(i + 1)
            .is_some_and(|(next, _)| *next == Join::Seam)
        {
            line.push_str(&format!(" ({})", Join::Seam.name(lang)));
        }
    }
    line
}

/// The folder of the finished parts of a mass written to `base` (D16): `NAME.teile` beside
/// `NAME.ly`.
pub fn parts_dir(base: &Path) -> PathBuf {
    with_extension(base, "teile")
}

/// The file of a finished part in [`parts_dir`]: `M-PP-id.musicxml`, with M the number of its
/// movement in the mass (Kyrie 1 … Agnus Dei 6, whichever movements are set, so that a part
/// keeps its file), PP its number in the movement as the addresses count it (`credo/2` gives
/// 02), and id its text with every character but letters, digits, `-` and `+` as `_`:
/// `3-02-credo_5-7.musicxml`.
pub fn part_file_name(movement: Movement, number: usize, text: &str) -> String {
    let id: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '+' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{}-{number:02}-{id}.musicxml", movement as usize + 1)
}

/// The file of the part `at` of `plan`, for a mass written to `base`
/// ([`parts_dir`], [`part_file_name`]).
pub fn part_path(base: &Path, plan: &MassPlan, at: PartRef) -> PathBuf {
    let movement = &plan.movements[at.movement];
    let number = movement.sections[..at.section]
        .iter()
        .map(|section| section.parts.len())
        .sum::<usize>()
        + at.part
        + 1;
    let name = part_file_name(movement.movement, number, &plan.part(at).text);
    parts_dir(base).join(name)
}

/// The exit code of `fux mass` for a complete mass, and for `--plan`.
pub const EXIT_COMPLETE: i32 = 0;

/// The exit code of `fux mass` for a mass with parts missing, which failed every attempt or
/// whose search was cancelled: the parts set are written, and `--resume` sets the others. (An
/// error before any part is set gives 1, a check that finds a prohibition 2.)
pub const EXIT_INCOMPLETE: i32 = 3;

/// The exit code of a mass of which `set` of the `planned` parts are set (D16): 0 only for a
/// complete mass.
pub const fn exit_code(set: usize, planned: usize) -> i32 {
    if set >= planned {
        EXIT_COMPLETE
    } else {
        EXIT_INCOMPLETE
    }
}

/// Where `fux mass` writes the mass unless `--output` says otherwise: `NAME` of its files.
pub const DEFAULT_BASE: &str = "out/messe";

/// The formats of a mass unless `--format` says otherwise: all four (§ 1.4).
pub const DEFAULT_FORMATS: [Format; 4] = [Format::Ly, Format::Pdf, Format::Midi, Format::Musicxml];

/// The seed with which attempt `attempt` (from 0) searches a part whose seed is `part`
/// (MASS-SEED, § 3.10): `splitmix64(part + attempt)`, the first number of the generator from it.
fn attempt_seed(part: u64, attempt: u8) -> u64 {
    Rng::new(part.wrapping_add(u64::from(attempt))).next_u64()
}

/// The mass whose parts the folder `dir` keeps ([`parts_dir`]), as the newest of its files that
/// names a mass says (`fux-mass`), so that the files of an older mass left there (with other
/// parts, after another `--chunk`) do not count; `None` without such a file.
pub fn recorded_mass(dir: &Path) -> Option<MassField> {
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let written = entry.metadata().ok()?.modified().ok()?;
            path.extension()
                .is_some_and(|e| e == "musicxml")
                .then_some((written, path))
        })
        .collect();
    files.sort();
    files.iter().rev().find_map(|(_, path)| {
        let text = std::fs::read_to_string(path).ok()?;
        from_musicxml(&text).ok()?.field(FIELD_MASS)?.parse().ok()
    })
}

/// `request` going on with the mass `record` (`--resume`, `--only`): its form, mode, choir, seed,
/// chant and head motif are the record's, so that the plan and its draws are those of the mass
/// whose parts are kept, and the parts take their subjects as they did; everything else is the
/// request's.
pub fn recorded_request(request: MassRequest, record: &MassField) -> MassRequest {
    MassRequest {
        preset: record.preset,
        // A mode per movement (`kyriale-ix`) is no mode of the mass: each movement takes the mode
        // of its chant.
        mode: record.mode.parse().ok(),
        choir: record.choir.clone(),
        seed: record.seed,
        chant: record.chant,
        motif: record.motif,
        ..request
    }
}

/// The error of `--resume` with a seed whose options plan another mass (`planned`) than the one
/// whose parts the folder `dir` keeps (`record`): the search would take none of them, since
/// their files name the mass recorded (`fux-mass`), and would put the parts of the other mass in
/// their place, one by one as they are set. So nothing is set, and the error says how to go on:
/// without `--seed` with the mass recorded, a single part with a seed of its own through
/// `--only`, a new mass without `--resume` or in another folder. Besides the seeds it names the
/// head motif and the chant where the two masses differ in them ([`material`]).
fn another_mass(dir: &Path, record: &MassField, planned: &MassField, lang: Lang) -> anyhow::Error {
    let (dir, recorded, seed) = (dir.display(), record.seed, planned.seed);
    let (theirs, ours) = material(record, planned, lang);
    anyhow!(
        "{}",
        match lang {
            Lang::De => format!(
                "In {dir} liegen Teile der Messe mit Seed {recorded}{theirs}, die --resume mit \
                 diesen Angaben (Seed {seed}{ours}) durch Teile einer anderen Messe ersetzen \
                 würde. Ohne --seed setzt --resume die Messe dort fort; einem einzelnen Teil gibt \
                 --only TEIL --seed N einen anderen Seed; eine neue Messe setzt fux mass ohne \
                 --resume oder mit einem anderen -o."
            ),
            Lang::En => format!(
                "{dir} holds parts of the mass with seed {recorded}{theirs}, which --resume with \
                 these options (seed {seed}{ours}) would replace with parts of another mass. \
                 Without --seed, --resume goes on with the mass there; --only PART --seed N gives \
                 a single part another seed; fux mass without --resume, or with another -o, sets \
                 a new mass."
            ),
        }
    )
}

/// The head motif and the chant of the mass recorded (`record`) and of the mass the options plan
/// (`planned`) where the two differ in them (MASS-MOTIF, MASS-CHANT), as [`another_mass`] adds
/// them to the seed of each: „Seed 4 und Kopfmotiv“ and „(Seed 4, ohne Kopfmotiv)“, „Seed 4 auf
/// Choral IX“ and „(Seed 4, ohne Choral)“; nothing where they agree, as for every mass without
/// either.
fn material(record: &MassField, planned: &MassField, lang: Lang) -> (String, String) {
    let (mut theirs, mut ours) = (String::new(), String::new());
    if record.motif != planned.motif {
        theirs.push_str(match record.motif {
            true => lang.pick(" und Kopfmotiv", " and the head motif"),
            false => lang.pick(" ohne Kopfmotiv", " without the head motif"),
        });
        ours.push_str(match planned.motif {
            true => lang.pick(", mit Kopfmotiv", ", with the head motif"),
            false => lang.pick(", ohne Kopfmotiv", ", without the head motif"),
        });
    }
    if record.chant != planned.chant {
        let chant = |chant: Option<mass::ChantMass>| match chant {
            Some(chant) => format!(
                "{} {}",
                lang.pick("auf Choral", "on chant"),
                chant.id().to_uppercase()
            ),
            None => lang.pick("ohne Choral", "without chant").to_string(),
        };
        theirs.push_str(&format!(" {}", chant(record.chant)));
        ours.push_str(&format!(", {}", chant(planned.chant)));
    }
    (theirs, ours)
}

/// The error of `--resume` and `--only` with `--movements` without the Kyrie where the parts
/// that the folder `dir` keeps are of a mass with a head motif (`motif=1` of `fux-mass`): the
/// motif comes from Kyrie I (MASS-MOTIF), which the plan must have, as it must for `--motif an`.
/// The Kyrie costs nothing where its files are there: its parts come from them.
fn kyrie_of_the_motif(dir: &Path, lang: Lang) -> anyhow::Error {
    let dir = dir.display();
    anyhow!(
        "{}",
        match lang {
            Lang::De => format!(
                "Die Messe in {dir} hat ein Kopfmotiv, und das kommt aus dem Kyrie I; dazu gehört \
                 das Kyrie in --movements. Seine Teile nehmen --resume und --only aus ihren \
                 Dateien."
            ),
            Lang::En => format!(
                "The mass in {dir} has a head motif, which comes from Kyrie I; choose the Kyrie in \
                 --movements as well. --resume and --only take its parts from their files."
            ),
        }
    )
}

/// Whether the part a file keeps (`field`) can stand for the part at `at` of `plan` under the
/// profile numbered `hash`: as [`PartField::fits`] says, but the seed of its search may be the
/// plan's for its attempt or one of its own, so that a part set again with a seed of its own
/// (`--only … --seed`) stays with its mass. That the file belongs to the mass of the plan, its
/// seed included, the caller checks (`fux-mass`).
fn fits(field: &PartField, plan: &MassPlan, at: PartRef, hash: u64) -> bool {
    let planned = PartField {
        seed: attempt_seed(plan.part(at).seed, field.attempt),
        ..field.clone()
    };
    planned.fits(plan, at, hash)
}

/// What the file of a part says of the part at its place in a plan ([`taken_part`]).
enum PartFile {
    /// It keeps the part, as the mass takes it up.
    Fits(Box<PartResult>),
    /// It keeps a part of the mass of the plan (`fux-mass`), but not the part these options plan
    /// there; what differs.
    Differs(Vec<Difference>),
    /// It keeps no part of the mass of the plan: it is missing, cannot be read or names another
    /// mass, or its part fits but took another head motif than Kyrie I gives.
    Missing,
}

/// What differs between the part the file of a part of a mass keeps and the part the options
/// plan at its place ([`PartFile::Differs`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Difference {
    /// The voices it sings (`--scoring`).
    Voices,
    /// Its imitation: whether it begins in imitation or together, and its points (`--imitation`,
    /// and `--lengths`, which adds points).
    Opening,
    /// Its ending, or the ending of the part before it, from which its cadences go (`--endings`).
    Ending,
    /// The profile it was set with (`--profile`, `--rule`, `--repeat`).
    Profile,
    /// The version of the generator that set it ([`mass::GENERATOR`]): another program, which
    /// may set other notes from the same plan and seed, so that no option makes it fit.
    Generator,
    /// The syllables of its text: eleison in three or in four (`--eleison`).
    Syllables,
    /// Anything else of its plan, such as how it joins the part before it (`--chunk`).
    Plan,
    /// The part as the file keeps it does not pass the check under the profile.
    Check,
}

impl Difference {
    fn name(self, lang: Lang) -> &'static str {
        match self {
            Difference::Voices => lang.pick("Stimmen", "voices"),
            Difference::Opening => lang.pick("Imitation", "imitation"),
            Difference::Ending => lang.pick("Schlussstufe", "ending"),
            Difference::Profile => lang.pick("Regelprofil", "rule profile"),
            Difference::Generator => lang.pick("Generatorversion", "generator version"),
            Difference::Syllables => lang.pick("Silben", "syllables"),
            Difference::Plan => lang.pick("Plan", "plan"),
            Difference::Check => lang.pick("Prüfung", "check"),
        }
    }
}

/// What differs between the part a file keeps (`field`), which does not fit the part at `at` of
/// `plan` under the profile numbered `hash` ([`fits`]), and that part, as far as the file says it:
/// its voices, its opening, its ending, the profile and the version of the generator that set it;
/// nothing where the difference lies elsewhere in the plan.
fn differences(field: &PartField, plan: &MassPlan, at: PartRef, hash: u64) -> Vec<Difference> {
    let part = plan.part(at);
    let section = &plan.movements[at.movement].sections[at.section];
    // The first three attempts set the part as planned; a fallback sings other voices or points.
    let planned = field.attempt < 3;
    let mut found = Vec::new();
    if planned && field.voices != section.voices {
        found.push(Difference::Voices);
    }
    if field.chord.is_some() != (part.opening == Opening::Tutti)
        || (planned && field.points != part.points)
    {
        found.push(Difference::Opening);
    }
    if field.ending != part.ending || field.before != plan.ending_before(at) {
        found.push(Difference::Ending);
    }
    if field.profile != hash {
        found.push(Difference::Profile);
    }
    if field.generator != mass::GENERATOR {
        found.push(Difference::Generator);
    }
    found
}

/// The part at `at` of `plan` from its file at `path`, where the file keeps it
/// ([`PartFile::Fits`]): the file names the mass of the plan (`fux-mass`), and its part fits the
/// plan's ([`fits`]) under `profile`, the profile the parts are set with ([`part_profile`]); its
/// lyrics are aligned with their text and sing the positions the file keeps ([`restore_part`]),
/// its plan of imitation is attached, as `fux check` does, and the part is checked again under
/// `profile` ([`part_from_parts`]), breaking no prohibition. A file of the mass of the plan that
/// keeps the part otherwise says what differs ([`PartFile::Differs`]), also where it opens with
/// the head motif and the options plan the part without it or the other way round
/// (`--imitation`); any other file, or none, keeps nothing of the mass ([`PartFile::Missing`]),
/// and neither does the file of a part that fits but took another head motif than `motif`,
/// where that is known: the motif of the mass as Kyrie I gives it (`Some(None)` where it gives
/// none, [`PartField::motif_fits`]). Such a part was set before Kyrie I was set anew, and no
/// option makes it fit; it is set again, whatever the options.
fn taken_part(
    path: &Path,
    plan: &MassPlan,
    at: PartRef,
    profile: &Profile,
    motif: Option<Option<&Motif>>,
    lang: Lang,
) -> PartFile {
    let Some((read, fields)) = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| musicxml_text_fields(&text, None, lang).ok())
    else {
        return PartFile::Missing;
    };
    let field = |name: &str| {
        fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, value)| value.as_str())
    };
    let mass = field(FIELD_MASS).and_then(|value| value.parse::<MassField>().ok());
    if mass.is_none_or(|mass| mass != MassField::of(plan)) {
        return PartFile::Missing;
    }
    let Some(part) = field(FIELD_PART).and_then(|value| value.parse::<PartField>().ok()) else {
        return PartFile::Differs(vec![Difference::Plan]);
    };
    let hash = profile_hash(profile);
    let fitting = fits(&part, plan, at, hash);
    // A part that fits opens as planned, with the head motif where the plan gives it one; that
    // it took another motif than Kyrie I gives, only a Kyrie I set anew explains. A part that
    // opens otherwise than planned differs in its imitation, whatever motif it took.
    if fitting && motif.is_some_and(|motif| !part.motif_fits(plan, at, motif)) {
        return PartFile::Missing;
    }
    let mut found = if fitting {
        Vec::new()
    } else {
        differences(&part, plan, at, hash)
    };
    // The part as `fux check` restores it, also where it does not fit, for the division of its
    // syllables.
    let mut score = read.score;
    let restored = align_lyrics(&mut score, read.text.as_deref(), lang).is_ok() && {
        restore_part(&mut score, &part, &fields);
        attach(&mut score, read.imitation, ImitationSource::File, "", lang).is_ok()
    };
    match restored.then(|| part_from_parts(plan, at, score, &part, profile)) {
        Some(Ok(result)) if fitting && result.generated.report.forbidden() == 0 => {
            return PartFile::Fits(Box::new(result));
        }
        Some(Err(FieldError::Division { .. })) => found.push(Difference::Syllables),
        _ if fitting => found.push(Difference::Check),
        _ => {}
    }
    if found.is_empty() {
        found.push(Difference::Plan);
    }
    PartFile::Differs(found)
}

/// The parts that `--resume` would set again although the files in `NAME.teile/` keep them: in
/// each movement that its files keep whole, each part in a file of the mass of `plan` (`files`,
/// in the order of the plan's parts) and none named by `--only` (`only`), those whose files keep
/// them otherwise than the options plan them ([`PartFile::Differs`]), with what differs. In a
/// movement that lacks a part (as one does whose part is set again with Kyrie I for its head
/// motif), or one whose part `--only` sets again, a part whose file does not fit is set again as
/// before: the parts after one set anew begin where it ends (as after the remedy for a part that
/// failed, `--resume --endings …`).
fn stale_parts<'a>(
    plan: &MassPlan,
    files: &'a [PartFile],
    only: &[PartRef],
) -> Vec<(PartRef, &'a [Difference])> {
    let places: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
    let mut stale = Vec::new();
    for movement in 0..plan.movements.len() {
        let parts: Vec<(PartRef, &PartFile)> = places
            .iter()
            .copied()
            .zip(files)
            .filter(|(at, _)| at.movement == movement)
            .collect();
        let whole = parts
            .iter()
            .all(|(at, file)| !only.contains(at) && !matches!(file, PartFile::Missing));
        if whole {
            stale.extend(parts.iter().filter_map(|(at, file)| match file {
                PartFile::Differs(differences) => Some((*at, differences.as_slice())),
                _ => None,
            }));
        }
    }
    stale
}

/// The error of `--resume` where it would replace parts of the mass whose files the folder `dir`
/// keeps, `stale` ([`stale_parts`]): the options plan them otherwise than they were set, or
/// another version of the generator set them, so it would set them again and write the new parts
/// over their files. So nothing is set, and the error names the parts, what differs and how to
/// go on: with the options the mass was set with, `--resume` takes them up, except the parts of
/// another version of the generator, which no option makes fit; `--only` sets them again with
/// these; `--rewrite` writes the mass anew from the parts as they are ([`rewrite`]); a new mass is
/// set without `--resume` or under another `-o`.
fn replaced_parts(
    dir: &Path,
    plan: &MassPlan,
    stale: &[(PartRef, &[Difference])],
    lang: Lang,
) -> anyhow::Error {
    let parts: Vec<String> = stale
        .iter()
        .map(|(at, differences)| {
            let names: Vec<&str> = differences.iter().map(|d| d.name(lang)).collect();
            format!("{} ({})", plan.address(*at), names.join(", "))
        })
        .collect();
    let named: Vec<String> = stale.iter().map(|(at, _)| plan.address(*at)).collect();
    let (dir, parts, named) = (dir.display(), parts.join(", "), named.join(","));
    let options = "--scoring, --imitation, --endings, --chunk, --lengths, --eleison, --profile, \
                   --rule, --repeat";
    let other = stale
        .iter()
        .filter(|(_, differences)| differences.contains(&Difference::Generator))
        .count();
    // How --resume takes them up: with the options of the setting, but none where another
    // version of the generator set them.
    let remedy = match (lang, other) {
        (Lang::De, 0) => format!(
            "Mit den Angaben, mit denen die Messe gesetzt wurde ({options}), nimmt --resume sie \
             aus ihren Dateien; "
        ),
        (Lang::En, 0) => format!(
            "With the options the mass was set with ({options}), --resume takes them from their \
             files; "
        ),
        (Lang::De, n) if n < stale.len() => format!(
            "Mit den Angaben, mit denen die Messe gesetzt wurde ({options}), nimmt --resume sie \
             aus ihren Dateien, außer denen einer anderen Generatorversion, die mit keinen \
             Angaben passen: Sie stammen von einem Programm, das aus demselben Plan andere Noten \
             setzen kann; "
        ),
        (Lang::En, n) if n < stale.len() => format!(
            "With the options the mass was set with ({options}), --resume takes them from their \
             files, except those of another generator version, which fit with no options: they \
             come from a program that may set other notes from the same plan; "
        ),
        (Lang::De, _) => "Sie stammen von einer anderen Version des Generators, die aus demselben \
                          Plan andere Noten setzen kann, und passen darum mit keinen Angaben. "
            .to_string(),
        (Lang::En, _) => "They come from another version of the generator, which may set other \
                          notes from the same plan, and so fit with no options. "
            .to_string(),
    };
    anyhow!(
        "{}",
        match lang {
            Lang::De => format!(
                "In {dir} liegen Teile dieser Messe, deren Dateien nicht zu diesen Angaben passen: \
                 {parts}. --resume würde sie neu setzen und ihre Dateien ersetzen. {remedy}--only \
                 {named} setzt sie mit diesen Angaben neu; --rewrite schreibt die Messe aus den \
                 Teilen neu, wie sie sind, ohne zu setzen; eine neue Messe setzt fux mass ohne \
                 --resume oder mit einem anderen -o."
            ),
            Lang::En => format!(
                "{dir} holds parts of this mass whose files do not fit these options: {parts}. \
                 --resume would set them again and replace their files. {remedy}--only {named} \
                 sets them again with these options; --rewrite writes the mass anew from the \
                 parts as they are, without setting any; fux mass without --resume, or with \
                 another -o, sets a new mass."
            ),
        }
    )
}

/// Gives the part at `at` of `plan` the seed `seed` instead of the plan's (`--only … --seed`):
/// the seed of its search and of its chain of internal cadences.
fn reseed(plan: &mut MassPlan, at: PartRef, seed: u64) {
    let part = &mut plan.movements[at.movement].sections[at.section].parts[at.part];
    part.seed = seed;
    if let CadenceChain::Weighted { seed: chain, .. } = &mut part.chain {
        *chain = seed;
    }
}

/// Writes the file of `result`, a part of the mass of `plan`, for a mass written to `base`
/// ([`part_path`], D16): its score as MusicXML with its voices and its head in `lang` and its
/// movement and label as title, without a seed under it (the seed of the part is in its field),
/// and besides the fields of a single setting those of the mass, of the part and of its lyrics
/// (`fux-mass`, `fux-part`, [`FIELD_LYRICS`]), from which `--resume` takes the part up again and
/// `fux check` repeats its report. Returns the file.
pub fn write_part(
    plan: &MassPlan,
    result: &PartResult,
    base: &Path,
    options: &MusicXmlOptions,
    lang: Lang,
) -> Result<PathBuf> {
    let at = result.at;
    let dir = parts_dir(base);
    std::fs::create_dir_all(&dir).with_context(|| unwritable(&dir, lang))?;
    let mut score = result.generated.score.clone();
    localize_names(&mut score, lang);
    score.title = Some(format!(
        "{} – {}",
        plan.movements[at.movement].movement.title(),
        part_label(plan, at)
    ));
    let part = PartField::of(plan, at, result, 1);
    let fields = [
        mass_field(plan),
        (FIELD_PART.to_string(), part.to_string()),
        (FIELD_LYRICS.to_string(), lyrics_field(&score)),
    ];
    let path = part_path(base, plan, at);
    let options = MusicXmlOptions {
        lang,
        seed: None,
        ..*options
    };
    // Written whole or not at all, also where Ctrl-C ends the program while it writes.
    let unfinished = path.with_extension("tmp");
    std::fs::write(
        &unfinished,
        to_musicxml_with_fields(&score, &options, &fields),
    )
    .with_context(|| unwritable(&unfinished, lang))?;
    std::fs::rename(&unfinished, &path).with_context(|| unwritable(&path, lang))?;
    Ok(path)
}

/// What stops a mass being set and tells how far its parts have come (§ 4.5): its cancel stops
/// the search of every part being set and keeps the others from beginning, and each part being
/// set has a control of its own ([`SearchControl`]), which tells the measure its search has
/// reached, so that the measures of parts set at once do not mix ([`run_mass`]).
#[derive(Debug, Default)]
pub struct MassControl {
    searches: Mutex<Searches>,
}

/// What a [`MassControl`] knows: whether the mass is cancelled, and the parts being set, each
/// with the control of its search.
#[derive(Debug, Default)]
struct Searches {
    cancelled: bool,
    running: Vec<(PartRef, Arc<SearchControl>)>,
}

impl MassControl {
    pub fn new() -> MassControl {
        MassControl::default()
    }

    fn lock(&self) -> MutexGuard<'_, Searches> {
        self.searches.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Asks the mass to stop: the search of every part being set returns
    /// [`GenerateError::Cancelled`] within a step, and no other part begins.
    pub fn cancel(&self) {
        let mut searches = self.lock();
        searches.cancelled = true;
        for (_, control) in &searches.running {
            control.cancel();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.lock().cancelled
    }

    /// The measure (from 1) the search of the part `at` has reached, 0 before it tells one;
    /// `None` where the part is not being set.
    pub fn reached(&self, at: PartRef) -> Option<u32> {
        let searches = self.lock();
        let (_, control) = searches.running.iter().find(|(place, _)| *place == at)?;
        Some(control.reached())
    }

    /// The part `at` begins: the control of its search, cancelled at once where the mass is.
    fn begin(&self, at: PartRef) -> Arc<SearchControl> {
        let control = Arc::new(SearchControl::new());
        let mut searches = self.lock();
        if searches.cancelled {
            control.cancel();
        }
        searches.running.push((at, Arc::clone(&control)));
        control
    }

    /// The part `at` is set, failed or cancelled.
    fn end(&self, at: PartRef) {
        self.lock().running.retain(|(place, _)| *place != at);
    }
}

/// The order in which [`run_mass`] begins the parts `parts` of `plan` with `jobs` of them at
/// once (K6): one at a time in their order, the order of the mass; several at once in the order
/// of the pool of the web version on as many workers ([`MassPlan::pool_order_of`], K12 (a)),
/// which [`MassPlan::pool_seconds`] estimates: Kyrie I first where it gives the head motif and
/// is among them, then the `jobs − 1` longest by the plan's estimate, then the others movement
/// by movement, each movement's longest first, the last [`tables::POOL_TAIL`] per job longest
/// first. Wherever it stands, a part that opens with the head motif begins only once Kyrie I of
/// the run is set or has failed, and then takes the motif the mass has from Kyrie I.
pub fn start_order(plan: &MassPlan, parts: &[PartRef], jobs: usize) -> Vec<PartRef> {
    if jobs > 1 {
        plan.pool_order_of(parts, jobs)
    } else {
        parts.to_vec()
    }
}

/// What the search of a part came to, in the thread that searched it: its result, or the panic
/// of its search.
type Outcome = std::thread::Result<Result<PartResult, GenerateError>>;

/// Sets the parts `parts` of the mass `m`, a mass of `request`, `jobs` of them at once (K6): one
/// at a time in their order and in the calling thread where `jobs` is 1, else in the order of
/// the pool of the web version ([`start_order`]), each in a thread of its own, the searches
/// sharing rayon's pool, each job once free taking the first part of that order that may begin;
/// each part as [`generate_part`] sets it under `profile` (the profile of the mass with the
/// user's overrides, [`prepare_mass`]). A part that takes the head motif (MASS-MOTIF) begins once
/// Kyrie I is set or has failed, where Kyrie I is among `parts`, and takes the motif of Kyrie I
/// as the mass has it by then (set before it or taken from its file; after a failed Kyrie I the
/// one the mass had before, in `fux mass` none, so that the part takes its subject from its words
/// with a warning). Each part keeps its seed, so the parts are the same with any number of jobs
/// (D13). Each part set goes into the mass, which keeps the parts it has already, such as those
/// taken from files.
///
/// Tells `on` in the calling thread of each part as it goes: [`MassEvent::Started`] with its first
/// attempt, then [`MassEvent::Finished`], or where no attempt set it [`MassEvent::Failed`] with the
/// error of its last attempt, which names the part and what may help; such a part stays unset,
/// and the others go on. With several jobs the events of different parts come as their searches
/// end, not in the order of the mass. The control (`control`, else one of the run's own) and `on`
/// breaking off end the mass: the searches of the parts being set stop (a part whose search ends
/// before it learns of it is set all the same), no other part begins, and `on` is told
/// [`MassEvent::Cancelled`] last; once `on` has broken off, it is told nothing else. The parts set
/// so far stay. A panic of a search stops the other searches and goes on once they have ended.
pub fn run_mass(
    request: &MassRequest,
    mut m: Mass,
    parts: &[PartRef],
    profile: &Profile,
    control: Option<&MassControl>,
    jobs: usize,
    on: &mut dyn FnMut(MassEvent) -> ControlFlow<()>,
) -> Mass {
    let plan = Arc::clone(&m.plan);
    let source = plan.motif_source();
    let own = MassControl::new();
    let control = control.unwrap_or(&own);
    let jobs = jobs.clamp(1, parts.len().max(1));
    let mut waiting = start_order(&plan, parts, jobs);
    // Kyrie I while it is still to be set in this run: the parts that take its motif wait.
    let mut kyrie_i = source.filter(|kyrie| parts.contains(kyrie));
    let (sender, ended) = std::sync::mpsc::channel::<(PartRef, Outcome)>();
    let mut running = 0;
    // A cancel or `on` breaking off; `on` is still told, unless it was `on` that broke off.
    let (mut stopped, mut listening) = (false, true);
    std::thread::scope(|scope| {
        loop {
            while !stopped && running < jobs {
                if control.is_cancelled() {
                    stopped = true;
                    break;
                }
                let ready = |at: &PartRef| kyrie_i.is_none() || !plan.part(*at).motif;
                let Some(next) = waiting.iter().position(ready) else {
                    break;
                };
                let at = waiting.remove(next);
                let seed = attempt_seed(plan.part(at).seed, 0);
                if on(MassEvent::Started {
                    at,
                    attempt: 0,
                    seed,
                })
                .is_break()
                {
                    (stopped, listening) = (true, false);
                    break;
                }
                // The head motif as Kyrie I gives it, set before this part or taken from its file.
                let motif = source
                    .and_then(|kyrie| m.result(kyrie))
                    .and_then(|kyrie| kyrie.motif.clone());
                let search = Some(control.begin(at));
                let (sender, plan) = (sender.clone(), &*plan);
                let task = move || {
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        generate_part(request, plan, at, profile, motif.as_ref(), search)
                    }));
                    let _ = sender.send((at, outcome));
                };
                if jobs == 1 {
                    task();
                } else {
                    scope.spawn(task);
                }
                running += 1;
            }
            if stopped {
                control.cancel();
            }
            if running == 0 {
                break;
            }
            let (at, outcome) = ended.recv().expect("every part begun tells its end");
            running -= 1;
            if kyrie_i == Some(at) {
                kyrie_i = None;
            }
            let outcome = outcome.unwrap_or_else(|panic| {
                // The other searches stop, the scope waits for them, and the panic goes on.
                control.cancel();
                std::panic::resume_unwind(panic)
            });
            let flow = match outcome {
                Ok(result) => {
                    m.insert(result.clone());
                    listening.then(|| on(MassEvent::Finished(Box::new(result))))
                }
                Err(GenerateError::Cancelled) => {
                    stopped = true;
                    None
                }
                Err(error) => {
                    // After every attempt, or at once where another attempt changes nothing.
                    let attempt = match &error {
                        GenerateError::MassPart { attempts, .. } => attempts.saturating_sub(1),
                        _ => 0,
                    };
                    listening.then(|| on(MassEvent::Failed { at, attempt, error }))
                }
            };
            // Its search counts as running until `on` has heard of its end.
            control.end(at);
            if flow.is_some_and(|flow| flow.is_break()) {
                (stopped, listening) = (true, false);
            }
        }
    });
    if stopped {
        let _ = on(MassEvent::Cancelled);
    }
    m
}

/// The summary of a set part of the mass of `request` as `fux free --details` gives a setting's
/// (`--details`, § 7): mode, voices, measures, its ending and its seed, the text, the internal
/// cadences and the points of its imitation, the last ending on the part's ending.
pub fn part_summary(
    request: &MassRequest,
    plan: &MassPlan,
    result: &PartResult,
    profile: &Profile,
    lang: Lang,
) -> Vec<String> {
    let at = result.at;
    let mut search = part_request(request, plan, at, result.motif.as_ref());
    // The voices it sings: those of its section, or the choir of the mass after a fallback.
    search.voices = result
        .generated
        .score
        .voices
        .iter()
        .map(|voice| voice.voice_type)
        .collect();
    let prepared = PreparedFree {
        request: search,
        profile: profile.clone(),
        seed: plan.part(at).seed,
        repeat: request.repeat.into(),
        warnings: Vec::new(),
    };
    free_summary(&prepared, std::slice::from_ref(&result.generated), lang)
}

/// The cadence plan of each movement of `m` ([`cadence_plan`]), its measures counted through the
/// movement: every part's internal cadences and its ending in its last measure. A part not set
/// shows as „…“, and so do the parts after it in its movement, whose measures are not known.
pub fn cadence_plans_of(m: &Mass, lang: Lang) -> Vec<String> {
    let plan = &m.plan;
    let mut lines = Vec::new();
    for (mi, movement) in plan.movements.iter().enumerate() {
        let name = |step: Step| degree_name(movement.mode, step, lang);
        // The measures before the part, while every part before it is set.
        let mut before = Some(0);
        let mut parts = Vec::new();
        for (si, section) in movement.sections.iter().enumerate() {
            for (pi, part) in section.parts.iter().enumerate() {
                let at = PartRef {
                    movement: mi,
                    section: si,
                    part: pi,
                };
                let cadences = match (before, m.result(at)) {
                    (Some(first), Some(result)) => {
                        let score = &result.generated.score;
                        let measures = score.measure_count();
                        before = Some(first + measures);
                        Some(PartCadences {
                            internal: score
                                .cadences
                                .iter()
                                .map(|c| (first + c.measure + 1, name(c.target)))
                                .collect(),
                            ending: (first + measures, name(part.ending)),
                        })
                    }
                    _ => {
                        before = None;
                        None
                    }
                };
                parts.push((part.join, cadences));
            }
        }
        if !parts.is_empty() {
            lines.push(cadence_plan(movement.movement.title(), &parts, lang));
        }
    }
    lines
}

/// The warnings of the set parts of `m`, each once, in the order of the mass (§ 4.4): a
/// fallback names its part or its section, a warning of the head motif or of the chant its
/// movement or none (chant IV once for the mass); another warning of a part's setting follows the
/// part's address.
pub fn mass_warnings(m: &Mass, lang: Lang) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for ((at, _), result) in m.plan.parts().zip(&m.results) {
        let Some(result) = result else {
            continue;
        };
        for warning in &result.warnings {
            let message = warning.message(lang);
            let line = match warning {
                Warning::LengthFallback { .. }
                | Warning::ScoringFallback { .. }
                | Warning::MotifFallback { .. }
                | Warning::NoMotif { .. }
                | Warning::ChantMode { .. }
                | Warning::ChantIV => message,
                _ => format!("{}: {message}", m.plan.address(at)),
            };
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
    }
    lines
}

/// The line with the totals of `m` (§ 7): its parts set of those planned and how many of them
/// came from `from` (`taken`), its measures and cost, the time its searches took, and the number
/// of its `warnings`.
pub fn mass_totals(m: &Mass, taken: usize, from: &str, warnings: usize, lang: Lang) -> String {
    let set: Vec<&PartResult> = m.results.iter().flatten().collect();
    let planned = m.results.len();
    let measures: usize = set
        .iter()
        .map(|result| result.generated.score.measure_count())
        .sum();
    let cost: u32 = set.iter().map(|result| result.generated.cost).sum();
    let seconds: f64 = set.iter().map(|result| f64::from(result.seconds)).sum();
    let parts = match (lang, planned) {
        (Lang::De, 1) => format!("{} von 1 Teil", set.len()),
        (Lang::De, _) => format!("{} von {planned} Teilen", set.len()),
        (Lang::En, 1) => format!("{} of 1 part", set.len()),
        (Lang::En, _) => format!("{} of {planned} parts", set.len()),
    };
    let taken = match taken {
        0 => String::new(),
        n => format!(" ({n} {} {from})", lang.pick("aus", "from")),
    };
    let warnings = match (lang, warnings) {
        (Lang::De, 0) => "keine Warnung".to_string(),
        (Lang::De, 1) => "1 Warnung".to_string(),
        (Lang::De, n) => format!("{n} Warnungen"),
        (Lang::En, 0) => "no warning".to_string(),
        (Lang::En, 1) => "1 warning".to_string(),
        (Lang::En, n) => format!("{n} warnings"),
    };
    let time = clock(seconds);
    match lang {
        Lang::De => format!(
            "Gesamt: {parts}{taken}, {measures} Takte, Kosten {cost}, Suchzeit {time}, {warnings}"
        ),
        Lang::En => format!(
            "Total: {parts}{taken}, {measures} measures, cost {cost}, search time {time}, \
             {warnings}"
        ),
    }
}

/// How the parts a mass lacks are set: the remark under a section that lacks one, and the end of
/// the report of an incomplete mass. Another seed for a part comes with `--only`: a seed given
/// to `--resume` alone would plan another mass ([`another_mass`]).
fn resume_hint(lang: Lang) -> &'static str {
    lang.pick(
        "fux mass --resume mit denselben Angaben setzt fehlende Teile nach, einen gescheiterten \
         Teil mit einer seiner Abhilfen; einem einzelnen Teil gibt --only TEIL --seed N einen \
         anderen Seed.",
        "fux mass --resume with the same options sets missing parts, a part that failed with one \
         of its remedies; --only PART --seed N gives a single part another seed.",
    )
}

/// Why a part of the mass could not be set, as `fux mass` tells it: the message of `error`,
/// where it is a part that no attempt set ([`GenerateError::MassPart`]) with its remedy of
/// another seed for this part alone, as `--only` gives it (`--only kyrie/2 --seed N`): a seed
/// given to `--resume` would plan another mass ([`another_mass`]).
fn failure_message(error: &GenerateError, lang: Lang) -> String {
    let message = error.message(lang);
    let GenerateError::MassPart { part, .. } = error else {
        return message;
    };
    let (any, own) = match lang {
        Lang::De => (
            "ein anderer Seed (--seed)",
            format!("ein anderer Seed (--only {part} --seed N)"),
        ),
        Lang::En => (
            "another seed (--seed)",
            format!("another seed (--only {part} --seed N)"),
        ),
    };
    message.replacen(any, &own, 1)
}

/// The widest line of a remark under a section in the book, in characters: about the width of
/// an A4 page in the font of the remarks, with room for wide letters.
const REMARK_WIDTH: usize = 85;

/// `text` in lines of at most `width` characters, broken between words; a word wider than
/// `width` has a line of its own.
fn wrapped(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// The line on a mass that lacks parts: which, and how to set them; `None` for a complete mass.
fn incomplete(m: &Mass, lang: Lang) -> Option<String> {
    let missing: Vec<String> = m
        .plan
        .parts()
        .zip(&m.results)
        .filter(|(_, result)| result.is_none())
        .map(|((at, _), _)| m.plan.address(at))
        .collect();
    if missing.is_empty() {
        return None;
    }
    Some(format!(
        "{}: {}. {}",
        lang.pick("Unvollständig, es fehlen", "Incomplete, missing"),
        missing.join(", "),
        resume_hint(lang)
    ))
}

/// The remarks under the section `section` of the movement `movement` of `m` where it lacks
/// parts (those of the Osanna it repeats for an Osanna ut supra): for each part missing why, the
/// error of its search where `failed` has one (which names the part and what may help), and how
/// `--resume` sets them; each in lines that the page has room for ([`REMARK_WIDTH`]), since the
/// book writes a remark as one line.
fn missing_remarks(
    m: &Mass,
    failed: &[(PartRef, String)],
    movement: usize,
    section: usize,
    lang: Lang,
) -> Vec<String> {
    let plan = &m.plan;
    let own = &plan.movements[movement].sections[section];
    let (from, repeated) = own.ut_supra.unwrap_or((movement, section));
    let parts = plan.movements[from].sections[repeated].parts.len();
    let mut remarks: Vec<String> = (0..parts)
        .map(|part| PartRef {
            movement: from,
            section: repeated,
            part,
        })
        .filter(|&at| m.result(at).is_none())
        .map(|at| match failed.iter().find(|(place, _)| *place == at) {
            Some((_, error)) => error.clone(),
            None => {
                let (address, why) = (plan.address(at), not_set(lang));
                match lang {
                    Lang::De => format!("Teil {address} fehlt: {why}"),
                    Lang::En => format!("Part {address} is missing: {why}"),
                }
            }
        })
        .collect();
    remarks.push(resume_hint(lang).to_string());
    remarks
        .iter()
        .flat_map(|remark| wrapped(remark, REMARK_WIDTH))
        .collect()
}

/// Why a part the search did not try is missing.
fn not_set(lang: Lang) -> &'static str {
    lang.pick(
        "nicht gesetzt, und seine Datei fehlt oder passt nicht zur Messe",
        "not set, and its file is missing or does not fit the mass",
    )
}

/// The file of the movement `movement` of a mass written to `base`, with `extension`:
/// `NAME-3-credo.musicxml`, numbered as the movements of a mass are (Kyrie 1 … Agnus Dei 6),
/// whichever of them are set, and named by its id (§ 6).
pub fn movement_path(base: &Path, movement: Movement, extension: &str) -> PathBuf {
    let mut name = base.as_os_str().to_owned();
    name.push(format!(
        "-{}-{}.{extension}",
        movement as usize + 1,
        movement.id()
    ));
    PathBuf::from(name)
}

/// The movement `movement` of `m` as its files keep it: its score ([`movement_score`]), its
/// voices named in `lang`, and the fields of its mass and its parts ([`part_fields`]) with the
/// positions of the syllables of each part ([`FIELD_LYRICS`], numbered as the parts are), from
/// which `fux check` repeats the report of every part ([`check_movement`]). `None` where a part
/// of the movement is not set.
pub fn movement_file(
    m: &Mass,
    movement: usize,
    lang: Lang,
) -> Option<(Score, Vec<(String, String)>)> {
    let plan = &m.plan;
    let mut score = movement_score(m, movement)?;
    let names = voice_names(&plan.movements[movement].voices, lang);
    for (voice, name) in score.voices.iter_mut().zip(&names) {
        voice.name = name.clone();
    }
    let mut fields = part_fields(m, movement);
    let places = plan.parts().filter(|(at, _)| at.movement == movement);
    for (n, (at, _)) in places.enumerate() {
        let result = m.result(at)?;
        fields.push((
            format!("{FIELD_LYRICS}.{}", n + 1),
            lyrics_field(&result.generated.score),
        ));
    }
    Some((score, fields))
}

/// Writes the mass `m` (§§ 6 and 7, D16): first `NAME.txt` with `report`, all that `fux mass`
/// printed; then for each movement whose parts are all set its MusicXML file
/// ([`movement_file`], [`fux_export::to_movement_musicxml`]) and its MIDI file
/// ([`fux_export::to_movement_midi`]), `NAME-3-credo.musicxml` and `NAME-3-credo.mid`
/// ([`movement_path`]); last the book ([`fux_export::mass_book`]) and its graphics, so that a
/// failure of LilyPond loses no other file. The book shows a section that lacks a part with a
/// rest in the staves of its voices and a remark per missing part: why it is missing (the error
/// of its search where `failed` has one), and how `--resume` sets it. A movement that lacks a
/// part has no MusicXML and MIDI file: its sections cannot be numbered, nor its parts checked
/// from it; a file of it from an earlier mass under `NAME` is removed (in the formats written),
/// so that the files agree with the book, and so is the file of a movement that the mass does not
/// set (`--movements`). Nothing else is removed: no file in a format not written, no file that
/// `fux mass` does not write (`NAME-3-credo.mscz`). `NAME` and the formats are those of
/// `output`, else [`DEFAULT_BASE`] and [`DEFAULT_FORMATS`]; the clefs, the brackets over the
/// entries, the tempo and the sound are those of `output` too, the Mensurstriche, the intonation
/// and the voices that rest through a section those of `notation`. Returns the files written:
/// the book and its graphics, the MusicXML files, the MIDI files, `NAME.txt`.
pub fn write_mass(
    m: &Mass,
    failed: &[(PartRef, String)],
    report: &[String],
    output: &OutputArgs,
    notation: &MassNotation,
    lang: Lang,
) -> Result<Vec<PathBuf>> {
    write_files(m, failed, report, output, notation, true, lang)
}

/// Writes the mass `m` as [`write_mass`] does, but without `remove_unset` the files of the
/// movements that `m` does not set stay as they are (`--rewrite`, [`rewrite`]): their parts may be
/// missing from `NAME.teile/`, so that such a file keeps the last copy of their notes.
fn write_files(
    m: &Mass,
    failed: &[(PartRef, String)],
    report: &[String],
    output: &OutputArgs,
    notation: &MassNotation,
    remove_unset: bool,
    lang: Lang,
) -> Result<Vec<PathBuf>> {
    let base = output::base_of(output, DEFAULT_BASE);
    if let Some(dir) = base.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| unwritable(dir, lang))?;
    }
    let txt = with_extension(&base, "txt");
    let mut text = report.join("\n");
    text.push('\n');
    std::fs::write(&txt, text).with_context(|| unwritable(&txt, lang))?;
    let formats = output::formats_of(output, &DEFAULT_FORMATS);
    let plan = &m.plan;
    let mut choir = plan.choir.clone();
    choir.sort();
    let choir = abbreviated(&choir);
    let byline = format!(
        "fux-kontrapunkt, {} {}",
        lang.pick("Seed", "seed"),
        plan.seed
    );
    let movement_options = MovementOptions {
        musicxml: output.musicxml_options(lang),
        mensurstriche: notation.mensurstriche,
        hide_tacet: notation.hide_tacet,
    };
    let kinds: Vec<(Format, &str)> = [(Format::Musicxml, "musicxml"), (Format::Midi, "mid")]
        .into_iter()
        .filter(|(format, _)| formats.contains(format))
        .collect();
    let (mut musicxml, mut midi) = (Vec::new(), Vec::new());
    for (mi, movement) in plan.movements.iter().enumerate() {
        if kinds.is_empty() {
            break;
        }
        let title = movement.movement.title();
        let file = movement_file(m, mi, lang);
        for &(format, extension) in &kinds {
            let path = movement_path(&base, movement.movement, extension);
            let Some((score, fields)) = &file else {
                // A file of an earlier mass, which the book no longer shows; where there is
                // none, or it cannot go, nothing is lost.
                std::fs::remove_file(&path).ok();
                continue;
            };
            let bytes = if format == Format::Musicxml {
                let heading = MovementHeading {
                    mass: plan.title.clone(),
                    number: movement.movement as usize + 1,
                    title: title.to_string(),
                    byline: Some(format!("{}, {choir} · {byline}", movement.mode.name(lang))),
                    intonation: movement
                        .intonation
                        .filter(|_| notation.intonation)
                        .map(str::to_string),
                };
                to_movement_musicxml(score, &heading, &movement_options, fields).into_bytes()
            } else {
                let name = format!("{} – {title}", plan.title);
                to_movement_midi(score, &name, &output.midi_options())
            };
            std::fs::write(&path, bytes).with_context(|| unwritable(&path, lang))?;
            if format == Format::Musicxml {
                musicxml.push(path);
            } else {
                midi.push(path);
            }
        }
    }
    // The files of the movements this mass does not set, from an earlier mass under `NAME`: the
    // book does not show them either. Their parts stay in `NAME.teile/`.
    if remove_unset {
        let unset = Movement::ALL
            .into_iter()
            .filter(|&movement| !plan.movements.iter().any(|set| set.movement == movement));
        for movement in unset {
            for &(_, extension) in &kinds {
                std::fs::remove_file(movement_path(&base, movement, extension)).ok();
            }
        }
    }
    let (mode, _) = mode_name(plan, lang);
    let book = MassBookOptions {
        subtitle: Some(format!("{mode}, {choir}")),
        byline: Some(byline),
        intonation: notation.intonation,
    };
    let options = LilypondOptions {
        mensurstriche: notation.mensurstriche,
        ..output.lilypond_options(lang)
    };
    let missing = |movement, section| missing_remarks(m, failed, movement, section, lang);
    let source = || mass_book(m, &book, &missing, &options);
    let mut files = output::write_lilypond(source, &base, formats, lang)?;
    files.extend(musicxml);
    files.extend(midi);
    files.push(txt);
    Ok(files)
}

/// The part of a mass that the file with `fields` keeps (`fux-part`), for `fux check`: `None`
/// for the file of anything else; an error in the user's language where the field cannot be
/// read.
pub fn part_of_file(fields: &[(String, String)], lang: Lang) -> Result<Option<PartField>> {
    let Some((_, value)) = fields.iter().find(|(name, _)| name == FIELD_PART) else {
        return Ok(None);
    };
    value
        .parse()
        .map(Some)
        .map_err(|error: FieldError| sentence(error.message(lang)))
}

/// The part and the profile with which `fux check` checks the file with `fields`. The file of a
/// part of a mass (`fux-part`) is checked as the part was set (D14), under the profile of a mass
/// on `base`: its overlay, then the single overrides `rules`, then those of `repeat`
/// ([`mass_profile`]). Any other file has no part and is checked as before, under `base` with
/// `rules` and `repeat`.
pub fn check_profile(
    fields: &[(String, String)],
    base: &Profile,
    rules: &[String],
    repeat: Repeat,
    lang: Lang,
) -> Result<(Option<PartField>, Profile)> {
    let part = part_of_file(fields, lang)?;
    if part.is_some() {
        return Ok((part, mass_profile(base, rules, repeat)?));
    }
    let mut profile = base.clone();
    for spec in rules {
        profile
            .apply_override(spec)
            .with_context(|| format!("--rule {spec}"))?;
    }
    Ok((None, repeat.apply(&profile)?))
}

/// The field of the file of a part with the position in its text that each of its syllables
/// sings, voice by voice: `0,1,2,3,4,5,4,5,6|…`, `-` for a syllable that fits none. The printed
/// syllables do not always say it: where a text repeats a word (San-ctus, San-ctus, San-ctus), a
/// word restated reads as the next one, and the text rules would judge another underlay than the
/// search did. So `fux check` and `--resume` take the positions from the file (D14).
pub const FIELD_LYRICS: &str = "fux-lyrics";

/// The value of [`FIELD_LYRICS`] for `score`: the lyric of every note that has one.
pub fn lyrics_field(score: &Score) -> String {
    let voices: Vec<String> = score
        .voices
        .iter()
        .map(|voice| {
            let positions: Vec<String> = voice
                .notes
                .iter()
                .filter_map(|note| note.lyric)
                .map(|lyric| {
                    lyric
                        .index()
                        .map_or_else(|| "-".to_string(), |i| i.to_string())
                })
                .collect();
            positions.join(",")
        })
        .collect();
    voices.join("|")
}

/// Gives the syllables of `score`, its lyrics aligned with its text, the positions that `value`
/// of [`FIELD_LYRICS`] keeps, and returns whether it could; where the field does not fit the
/// score (other voices, other syllables, positions beyond its text) the alignment stays.
pub fn restore_lyrics(score: &mut Score, value: &str) -> bool {
    let Some(syllables) = score.text.as_ref().map(|sung| sung.text.syllables.len()) else {
        return false;
    };
    let voices: Vec<&str> = value.split('|').collect();
    if voices.len() != score.voices.len() {
        return false;
    }
    let mut restored = Vec::new();
    for (voice, positions) in score.voices.iter().zip(voices) {
        let positions: Option<Vec<Lyric>> = positions
            .split(',')
            .filter(|position| !position.is_empty())
            .map(|position| match position {
                "-" => Some(Lyric::UNKNOWN),
                n => n.parse().ok().filter(|&i| i < syllables).map(Lyric::new),
            })
            .collect();
        match positions {
            Some(positions)
                if positions.len() == voice.notes.iter().filter(|n| n.lyric.is_some()).count() =>
            {
                restored.push(positions);
            }
            _ => return false,
        }
    }
    for (voice, positions) in score.voices.iter_mut().zip(restored) {
        let notes = voice.notes.iter_mut().filter(|note| note.lyric.is_some());
        for (note, lyric) in notes.zip(positions) {
            note.lyric = Some(lyric);
        }
    }
    true
}

/// Marks as musica ficta every note of `score` tied from a note of musica ficta at its pitch, as
/// the search set it: MusicXML marks the first note of a tie only, so that the file gives the
/// notes after it back without the mark, and the book would print them otherwise than for the
/// part as it was set.
fn restore_tied_ficta(score: &mut Score) {
    for voice in &mut score.voices {
        for i in 1..voice.notes.len() {
            let before = &voice.notes[i - 1];
            if before.tie && before.ficta && before.pitch == voice.notes[i].pitch {
                voice.notes[i].ficta = true;
            }
        }
    }
}

/// A part of a mass as its file with `fields` gives it back, its lyrics aligned with its text,
/// as the part was set: the notes tied from a note of musica ficta are ficta too, which the file
/// marks on the first note of a tie only; where it sings the file's own text, the text goes on
/// after it where it is not the last part of its section (`cont=1`, D8), and its syllables sing
/// the positions the file keeps ([`FIELD_LYRICS`]). For `fux check` and `--resume`.
pub fn restore_part(score: &mut Score, part: &PartField, fields: &[(String, String)]) {
    restore_tied_ficta(score);
    let field = |name: &str| {
        fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, value)| value.as_str())
    };
    let Some(sung) = score
        .text
        .as_mut()
        .filter(|sung| field(FIELD_TEXT) == Some(sung.text.spec().as_str()))
    else {
        return;
    };
    sung.text = Arc::new(Text {
        continued: part.continued,
        ..(*sung.text).clone()
    });
    if let Some(lyrics) = field(FIELD_LYRICS) {
        restore_lyrics(score, lyrics);
    }
}

/// The movement of a mass whose file has `fields` ([`movement_file`]): the one whose text its
/// first part sings (`fux-text.1`); `None` for the file of anything else.
pub fn movement_of_file(fields: &[(String, String)]) -> Option<Movement> {
    let name = format!("{FIELD_TEXT}.1");
    let (_, text) = fields.iter().find(|(field, _)| *field == name)?;
    mass::preset::movement_of(text.split(['+', ':']).next()?.trim())
}

/// The score that `fux render` writes of `score`, read from a file with `fields`: a single score
/// as before phase 14 (D15), and a score in sections (the file of a mass movement, a file of
/// another program with rehearsal marks after double bars) as a movement of the book of a mass
/// ends, with fermatas on its last notes as on those of its sections before; that of a movement
/// under the movement's title before the mode („Gloria – dorisch“).
pub fn rendered_sheet<'a>(
    score: &'a Score,
    fields: &[(String, String)],
    lang: Lang,
) -> fux_export::Sheet<'a> {
    let sheet = fux_export::Sheet::new(score);
    if score.sections.is_empty() {
        return sheet;
    }
    fux_export::Sheet {
        piece: movement_of_file(fields)
            .map(|movement| format!("{} – {}", movement.title(), score.mode.name(lang))),
        end: fux_export::SectionEnd::Movement,
        ..sheet
    }
}

/// A part of the file of a mass movement, or a section of another file in sections, as `fux
/// check` judges it: on its own, its measures counted in the file (§ 6 „Check“, D14).
#[derive(Clone, Debug)]
pub struct CheckedPiece {
    /// How the report names it: „Teil credo/2: credo:5-7, Takte 32–58“, „Abschnitt 2: Christe,
    /// Takte 28–71“.
    pub heading: String,
    /// The measures of the file before it, from which the report counts its measures.
    pub before: usize,
    /// Its score as checked: its voices with their notes, its text with the positions of its
    /// syllables, its internal cadences, its plan of imitation.
    pub score: Score,
    /// What the check says of its text and its imitation before the violations.
    pub lines: Vec<String>,
    /// Its violations under the profile of the check.
    pub report: Report,
}

/// The check of a file of several parts or sections ([`check_movement`]).
#[derive(Clone, Debug)]
pub struct MovementCheck {
    /// The parts, or the sections, in order.
    pub pieces: Vec<CheckedPiece>,
    /// The profile they are checked under: the mass's for the parts of a movement
    /// ([`mass_profile`]), the one chosen for the sections of another file.
    pub profile: Profile,
    /// Whether the pieces are the parts of a mass movement; else sections.
    pub parts: bool,
}

/// What `fux check` checks in the file read as `read`, with `fields`, where the file has several
/// parts or sections (§ 6 „Check“), each piece on its own; `None` for any other file, which is
/// checked whole as before.
///
/// The file of a mass movement (`fux-part.1`, …) gives its parts back ([`split_movement`]), and
/// each is checked as the file of the part is (D14): under the profile of the mass on `base`, with
/// the single overrides `rules` and those of `repeat` ([`mass_profile`]); its syllables aligned
/// with its text (`fux-text.N`, or `text` where given), going on after it and singing the
/// positions they sang ([`restore_part`]); its plan of imitation as `imitation` says, the last
/// point ending on the part's ending. So every part repeats the report of its search. Another
/// file in two sections or more, from the double bars and rehearsal marks of its notation
/// ([`Score::sections`]), is checked section by section, each with the voices that sing in it,
/// under `base` with `rules` and `repeat`, its syllables aligned with `text` or the text
/// recognised. The lines of every piece count the measures of the file.
#[allow(clippy::too_many_arguments)]
pub fn check_movement(
    read: &ReadScore,
    fields: &[(String, String)],
    text: Option<&str>,
    base: &Profile,
    rules: &[String],
    repeat: Repeat,
    imitation: ImitationSource,
    lang: Lang,
) -> Result<Option<MovementCheck>> {
    let failed = |error: FieldError| sentence(error.message(lang));
    let split = split_movement(&read.score, fields).map_err(failed)?;
    if split.is_empty() {
        return checked_sections(read, fields, text, base, rules, repeat, imitation, lang);
    }
    let kept = movement_parts(fields).map_err(failed)?;
    let profile = mass_profile(base, rules, repeat)?;
    let checker = Checker::new(&profile);
    let remedy = lang.pick("Abhilfe: --imitation aus.", "Remedy: --imitation off.");
    let mut pieces = Vec::with_capacity(split.len());
    for (n, (at, mut score, before)) in split.into_iter().enumerate() {
        let number = n + 1;
        let address = format!("{}/{number}", Movement::ALL[at.movement].id());
        let part = &kept[n];
        let own = fields_of_part(fields, number);
        let spec = own
            .iter()
            .find(|(name, _)| name == FIELD_TEXT)
            .map(|(_, value)| value.as_str());
        let stored = score.imitation.take().map(|plan| Ok((*plan).clone()));
        let named = || format!("{} {address}", lang.pick("Teil", "Part"));
        let mut lines = align_lyrics(&mut score, text.or(spec), lang).with_context(named)?;
        restore_part(&mut score, part, &own);
        let ending = part.ending;
        lines.extend(
            attach_after(&mut score, stored, imitation, remedy, ending, before, lang)
                .with_context(named)?,
        );
        let label = part_label_of(&kept, n, spec.unwrap_or_default());
        let measures = (before + 1, before + score.measure_count());
        pieces.push(CheckedPiece {
            heading: piece_heading(&named(), &label, measures, lang),
            before,
            report: checker.check(&score),
            score,
            lines,
        });
    }
    Ok(Some(MovementCheck {
        pieces,
        profile,
        parts: true,
    }))
}

/// How the report of a check names the part numbered `n` (from 0) of a movement whose file keeps
/// `parts`, as the lines of a mass name it ([`part_label`]): a section of one part by its title,
/// a part after a Break by its label, any other by its text `spec`.
fn part_label_of(parts: &[PartField], n: usize, spec: &str) -> String {
    let begins = |part: &PartField| matches!(part.join, None | Some(SectionKind::Section));
    let part = &parts[n];
    let alone = begins(part) && parts.get(n + 1).is_none_or(begins);
    if part.join == Some(SectionKind::Break) || (alone && !part.title.is_empty()) {
        part.title.clone()
    } else {
        spec.to_string()
    }
}

/// The heading of a piece in the report of a check: „Teil credo/2: credo:5-7, Takte 32–58“, its
/// measures counted in the file.
fn piece_heading(name: &str, label: &str, (first, last): (usize, usize), lang: Lang) -> String {
    let measures = match lang {
        Lang::De => format!("Takte {first}–{last}"),
        Lang::En => format!("measures {first}–{last}"),
    };
    match label {
        "" => format!("{name}: {measures}"),
        label => format!("{name}: {label}, {measures}"),
    }
}

/// The sections of a file in two sections or more ([`check_movement`]), each on its own;
/// `None` for a file in one section at most, or one in whose sections no voice sings.
#[allow(clippy::too_many_arguments)]
fn checked_sections(
    read: &ReadScore,
    fields: &[(String, String)],
    text: Option<&str>,
    base: &Profile,
    rules: &[String],
    repeat: Repeat,
    imitation: ImitationSource,
    lang: Lang,
) -> Result<Option<MovementCheck>> {
    let whole = &read.score;
    let starts: Vec<&SectionMark> = whole
        .sections
        .iter()
        .filter(|mark| mark.kind == SectionKind::Section)
        .collect();
    if starts.len() < 2 {
        return Ok(None);
    }
    let (_, profile) = check_profile(fields, base, rules, repeat, lang)?;
    let checker = Checker::new(&profile);
    let measures = whole.measure_count();
    let mut pieces = Vec::with_capacity(starts.len());
    for (i, mark) in starts.iter().enumerate() {
        let before = mark.measure;
        let end = starts.get(i + 1).map_or(measures, |next| next.measure);
        let name = format!("{} {}", lang.pick("Abschnitt", "Section"), i + 1);
        let Some(mut score) = section_of(whole, before, end) else {
            continue;
        };
        score
            .validate()
            .map_err(|error| anyhow!("{name}: {}", error.message(lang, score.meter)))?;
        let mut lines = align_lyrics(&mut score, text, lang).with_context(|| name.clone())?;
        let ending = score.mode.final_step();
        lines.extend(
            attach_after(&mut score, None, imitation, "", ending, before, lang)
                .with_context(|| name.clone())?,
        );
        let label = mark.title.clone().unwrap_or_default();
        let last = before + score.measure_count();
        pieces.push(CheckedPiece {
            heading: piece_heading(&name, &label, (before + 1, last), lang),
            before,
            report: checker.check(&score),
            score,
            lines,
        });
    }
    Ok((!pieces.is_empty()).then_some(MovementCheck {
        pieces,
        profile,
        parts: false,
    }))
}

/// The measures `first` to `end` (from 0, `end` not included) of `score` as a score of their own:
/// the voices that sing in them, with their notes there, the internal cadences that the section
/// has room for (one in its last measures is the cadence that ends it) and the marks of Breaks and
/// seams there; `None` where no voice sings in them. A note tied over the double bar into the next
/// section ends the section untied, as a section checked on its own ends.
fn section_of(score: &Score, first: usize, end: usize) -> Option<Score> {
    let per = score.meter.ticks_per_measure();
    let (from, to) = (first as u32 * per, end as u32 * per);
    let within = |tick: u32| from <= tick && tick < to;
    let voices: Vec<Voice> = score
        .voices
        .iter()
        .filter(|voice| {
            voice
                .timed()
                .any(|(tick, note)| within(tick) && !note.is_rest())
        })
        .map(|voice| {
            let mut notes: Vec<Note> = voice
                .timed()
                .filter(|&(tick, _)| within(tick))
                .map(|(_, note)| note.clone())
                .collect();
            if let Some(last) = notes.last_mut() {
                last.tie = false;
            }
            Voice {
                notes,
                ..voice.clone()
            }
        })
        .collect();
    if voices.is_empty() {
        return None;
    }
    let mut section = Score::new(score.mode, voices);
    section.meter = score.meter;
    // As `Score::validate` asks: a measure before the cadence, and its final cadence after it.
    section.cadences = score
        .cadences
        .iter()
        .filter(|cadence| first < cadence.measure && cadence.measure + 3 <= end)
        .map(|cadence| Cadence {
            measure: cadence.measure - first,
            ..*cadence
        })
        .collect();
    section.sections = score
        .sections
        .iter()
        .filter(|mark| {
            mark.kind != SectionKind::Section && first < mark.measure && mark.measure < end
        })
        .map(|mark| SectionMark {
            measure: mark.measure - first,
            ..mark.clone()
        })
        .collect();
    Some(section)
}

/// Prints the check of a file in parts or sections ([`check_movement`]): for each piece its
/// heading, then indented the lines on its text and its imitation, its violations and the line
/// with its prohibitions and cost, as `fux check` prints them for a file of its own but with the
/// measures of the file; last the totals. With `mark` writes the pieces with their violations
/// marked, a score each, its measures numbered as in the file, as `--output` and `--format` say.
/// Returns the number of prohibitions.
pub fn print_movement_check(
    out: &mut impl Write,
    check: &MovementCheck,
    mark: bool,
    output: &OutputArgs,
    lang: Lang,
) -> Result<usize> {
    let checker = Checker::new(&check.profile);
    let mut all = Report::default();
    for piece in &check.pieces {
        writeln!(out, "{}", piece.heading)?;
        for line in &piece.lines {
            writeln!(out, "  {line}")?;
        }
        let (score, report) = (&piece.score, &piece.report);
        print_violations_after(out, score, &checker, report, piece.before, lang, "  ")?;
        writeln!(out, "  {}", report_line(report, lang))?;
        all.violations.extend(report.violations.iter().cloned());
    }
    let pieces = if check.parts {
        counted(
            check.pieces.len(),
            ("Teil", "Teilen"),
            ("part", "parts"),
            lang,
        )
    } else {
        counted(
            check.pieces.len(),
            ("Abschnitt", "Abschnitten"),
            ("section", "sections"),
            lang,
        )
    };
    let totals = report_line(&all, lang);
    let totals = totals.trim_end_matches('.');
    match lang {
        Lang::De => writeln!(out, "Gesamt: {totals} in {pieces}.")?,
        Lang::En => writeln!(out, "Total: {totals} in {pieces}.")?,
    }
    if mark {
        let sheets: Vec<_> = check
            .pieces
            .iter()
            .map(|piece| fux_export::Sheet {
                piece: Some(piece.heading.clone()),
                ..marked_sheet_after(&piece.score, &piece.report, &checker, piece.before, lang)
            })
            .collect();
        let files = output::write(None, &sheets, output, "out/pruefung", lang)?;
        print_files(out, &files, lang)?;
    }
    Ok(all.forbidden())
}

/// The widest status line, in characters, so that it fits a terminal of 80 columns: a wider one
/// would break, and writing it anew in place would leave its first row behind.
const STATUS_WIDTH: usize = 79;

/// The status while several parts are searched at once (§ 4.5, K6): each part by its address
/// with the measure its search has reached (0 before it tells one) and the measures the plan
/// expects, as given (in the order of the mass), then the time the mass still takes: „kyrie/3 T.
/// 12/≈30 · gloria/2 T. 4/≈38 · noch etwa 3:40“. Where the parts would make the line wider than
/// 79 characters, which a terminal of 80 columns has room for, those beyond are counted instead
/// („+2“). One part has the line of [`PartLine::running`].
pub fn running_parts(parts: &[(String, u32, usize)], remaining: Option<f64>, lang: Lang) -> String {
    let at = lang.pick("T.", "m.");
    let left = remaining.map_or_else(String::new, |remaining| match lang {
        Lang::De => format!(" · noch etwa {}", clock(remaining)),
        Lang::En => format!(" · about {} left", clock(remaining)),
    });
    let items: Vec<String> = parts
        .iter()
        .map(|(address, measure, measures)| format!("{address} {at} {measure}/≈{measures}"))
        .collect();
    let mut shown = items.len();
    loop {
        let mut line = items[..shown].join(" · ");
        if shown < items.len() {
            line.push_str(&format!(" · +{}", items.len() - shown));
        }
        line.push_str(&left);
        if shown <= 1 || line.chars().count() <= STATUS_WIDTH {
            return line;
        }
        shown -= 1;
    }
}

/// The line on a terminal that tells how far the searches of the parts being set have come
/// (§ 4.5), written anew in place four times a second; the lines of the parts go above it. Where
/// stderr is no terminal there is none.
struct Status {
    terminal: bool,
    shown: Mutex<Shown>,
    tick: Condvar,
}

/// What the status line shows, and what the time the mass still takes comes from.
#[derive(Default)]
struct Shown {
    /// The parts being searched, in the order of the mass.
    running: Vec<Running>,
    /// The parts still to begin, in the order they begin, the order of the pool of the web
    /// version ([`start_order`]), each with the seconds the plan expects of it beside the others
    /// and how it stands to the head motif.
    waiting: Vec<(PartRef, EtaPart)>,
    /// The parts set at once.
    jobs: usize,
    /// How long the parts searched took against their estimates, for the time still needed.
    eta: Eta,
    /// The line stands on the terminal.
    visible: bool,
    /// The parts are set: the line goes.
    done: bool,
}

impl Shown {
    /// The part `at`, expected to take `estimate` seconds, is set after `seconds`, or failed
    /// after the time since it began: it runs no more, and the time it took counts for the
    /// others ([`Eta::record`]).
    fn end(&mut self, at: PartRef, estimate: f64, seconds: Option<f64>) {
        let Some(k) = self.running.iter().position(|part| part.at == at) else {
            return;
        };
        let part = self.running.remove(k);
        let seconds = seconds.unwrap_or_else(|| part.started.elapsed().as_secs_f64());
        self.eta.record(estimate, seconds);
    }

    /// The seconds the mass still takes ([`Eta::remaining`]).
    fn left(&self) -> f64 {
        let running: Vec<(EtaPart, f64)> = self
            .running
            .iter()
            .map(|part| (part.estimate, part.started.elapsed().as_secs_f64()))
            .collect();
        let waiting: Vec<EtaPart> = self.waiting.iter().map(|&(_, part)| part).collect();
        self.eta.remaining(self.jobs, &running, &waiting)
    }
}

/// A part being searched, for the status line.
struct Running {
    at: PartRef,
    line: PartLine,
    /// Its address: `credo/3`.
    address: String,
    /// The measures it is expected to have, and the seconds it is expected to take beside the
    /// others, with how it stands to the head motif.
    measures: usize,
    estimate: EtaPart,
    started: Instant,
}

/// Ends the status line where it goes out of scope ([`Status::finish`]): when the parts are set,
/// and while a panic of their search unwinds.
struct Finish<'a>(&'a Status);

impl Drop for Finish<'_> {
    fn drop(&mut self) {
        self.0.finish();
    }
}

impl Status {
    fn new() -> Status {
        Status {
            terminal: std::io::stderr().is_terminal(),
            shown: Mutex::default(),
            tick: Condvar::new(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Shown> {
        self.shown.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A line of its own on stderr, above the status line.
    fn print(&self, line: &str) {
        let mut shown = self.lock();
        let mut err = std::io::stderr().lock();
        if std::mem::take(&mut shown.visible) {
            let _ = write!(err, "\r\x1b[K");
        }
        let _ = writeln!(err, "{line}");
    }

    /// The parts are set: [`Status::show`] clears the line and ends.
    fn finish(&self) {
        self.lock().done = true;
        self.tick.notify_all();
    }

    /// Writes the status line anew every quarter of a second, with the measure each search has
    /// reached (`control`), until [`Status::finish`]: the line of the part being searched, or
    /// that of the parts being searched at once ([`running_parts`]).
    fn show(&self, control: &MassControl, lang: Lang) {
        let mut shown = self.lock();
        while !shown.done {
            let quarter = std::time::Duration::from_millis(250);
            shown = self
                .tick
                .wait_timeout(shown, quarter)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
            if shown.done {
                break;
            }
            let remaining = Some(shown.left());
            let measure = |part: &Running| control.reached(part.at).unwrap_or(0);
            let line = match shown.running.as_slice() {
                [] => continue,
                [part] => part
                    .line
                    .running(measure(part), part.measures, remaining, lang),
                parts => {
                    let parts: Vec<(String, u32, usize)> = parts
                        .iter()
                        .map(|part| (part.address.clone(), measure(part), part.measures))
                        .collect();
                    running_parts(&parts, remaining, lang)
                }
            };
            let mut err = std::io::stderr().lock();
            let _ = write!(err, "\r{line}\x1b[K");
            let _ = err.flush();
            shown.visible = true;
        }
        if std::mem::take(&mut shown.visible) {
            let _ = write!(std::io::stderr(), "\r\x1b[K");
        }
    }

    /// Runs `work`, the setting of the parts, with the status line where stderr is a terminal:
    /// a thread writes it ([`Status::show`]) until `work` returns, or panics. The scope waits for
    /// the thread before it passes a panic on, so the line ends then too ([`Finish`]), and the
    /// program with the panic.
    fn during<R>(&self, control: &MassControl, lang: Lang, work: impl FnOnce() -> R) -> R {
        std::thread::scope(|scope| {
            let _finish = Finish(self);
            if self.terminal {
                scope.spawn(|| self.show(control, lang));
            }
            work()
        })
    }
}

/// How a part came into the mass this time.
enum Source {
    /// Searched: set, or failed with why.
    Searched(Option<String>),
    /// Taken from its file.
    Taken,
    /// Neither searched nor taken (`--only` without `--resume`).
    Left,
}

/// The line on stderr of a part searched, kept until the parts before it have theirs, so that
/// the lines come in the order of the mass whichever search ends first.
enum Told {
    /// Set: its line, but for the time the mass still takes, which it gets when it is printed.
    Set {
        line: PartLine,
        measures: usize,
        cost: u32,
        seconds: f64,
    },
    /// No attempt set it: its line with why.
    Failed(String),
}

impl Told {
    /// The line, with the seconds the mass still takes where it has them.
    fn line(&self, remaining: Option<f64>, lang: Lang) -> String {
        match self {
            Told::Set {
                line,
                measures,
                cost,
                seconds,
            } => line.finished(*measures, *cost, *seconds, remaining, lang),
            Told::Failed(line) => line.clone(),
        }
    }
}

/// `fux mass`: plans the mass and checks every option; with `--plan` prints the plan without a
/// search (with `--details` a line per part). Otherwise prints the plan and sets the parts (all
/// of them; with `--resume` those its files lack; with `--only` those it names), `--jobs` of them
/// at once (else [`default_jobs`], [`run_mass`]), telling each on stderr in the order of the mass
/// once it and the parts before it are set (§ 7), and writing each to `NAME.teile/` as soon as it
/// is set (D16); then prints a line per part (with `--details` its summary), the cadence plan of
/// every movement, the totals and the warnings, and writes the files of the mass
/// ([`write_mass`]). `--resume` that would set a part again over its file in a movement whose
/// parts are all there, since the options plan it otherwise, sets nothing and fails with the
/// parts and what differs. `--rewrite` writes the mass anew from its parts as they are, setting
/// nothing ([`mass_of_parts`]). Returns the exit code ([`exit_code`]): 0 only for a complete mass.
pub fn mass(
    out: &mut impl Write,
    options: &MassOptions,
    base: &Profile,
    rules: &[String],
    output: &OutputArgs,
    lang: Lang,
) -> Result<i32> {
    if options.rewrite {
        return rewrite(out, options, base, rules, output, lang);
    }
    let (mut request, profile) = prepare_mass(options, base, rules, lang)?;
    let path = output::base_of(output, DEFAULT_BASE);
    let dir = parts_dir(&path);
    // --only: the parts it names are set again, the others come from their files.
    let again = !options.only.is_empty();
    // The mass that --resume and --only go on with, as the files of its parts record it.
    let record = (options.resume || again)
        .then(|| recorded_mass(&dir))
        .flatten();
    if let Some(record) = &record
        && (again || options.seed.is_none())
    {
        request = recorded_request(request, record);
        // The head motif of the mass recorded comes from its Kyrie I: the plan needs the Kyrie.
        if record.motif && !request.movements.contains(&Movement::Kyrie) {
            return Err(kyrie_of_the_motif(&dir, lang));
        }
    }
    let mut plan = mass_plan(&request, lang)?;
    let only = only_parts(&plan, &options.only, lang)?;
    // --only with a seed: the parts it names take it, as --plan --details shows them.
    if let (true, Some(seed)) = (again, options.seed) {
        for &at in &only {
            reseed(&mut plan, at, seed);
        }
    }
    let jobs = options.jobs.map_or_else(default_jobs, usize::from);
    let mut report: Vec<String> = mass_summary(&plan, jobs, lang)
        .lines()
        .map(str::to_string)
        .collect();
    if options.plan {
        if options.details {
            report.extend(plan_details(&plan, lang));
        }
        for line in &report {
            writeln!(out, "{line}")?;
        }
        return Ok(EXIT_COMPLETE);
    }
    if again && record.is_none() {
        bail!(
            "{}",
            match lang {
                Lang::De => format!(
                    "--only setzt Teile einer Messe in {} neu, aber dort ist keine; ohne --only \
                     setzt fux mass die ganze Messe.",
                    dir.display()
                ),
                Lang::En => format!(
                    "--only sets parts of a mass in {} again, but there is none; without --only \
                     fux mass sets the whole mass.",
                    dir.display()
                ),
            }
        );
    }
    // --resume with a seed of its own: the options must plan the mass whose parts are kept.
    if options.resume
        && !again
        && options.seed.is_some()
        && let Some(record) = &record
        && MassField::of(&plan) != *record
    {
        return Err(another_mass(&dir, record, &MassField::of(&plan), lang));
    }
    let plan = Arc::new(plan);
    let searched = part_profile(&request, &profile);
    // What the files of the parts keep, for --resume and --only, and the parts set again: those
    // --only names and, where Kyrie I is set again, every part that takes its head motif
    // (MASS-MOTIF). A part taken from its file took the motif that Kyrie I gives, where Kyrie I is
    // taken from its file. Kyrie I comes first.
    let kyrie_i = plan.motif_source();
    let mut motif: Option<Option<Motif>> = None;
    let mut kyrie_i_again = false;
    let mut renewed = Vec::new();
    let mut files: Vec<PartFile> = Vec::new();
    for (at, part) in plan.parts() {
        let renew = only.contains(&at) || (part.motif && kyrie_i_again);
        let kept = (options.resume || again) && !renew;
        let file = if kept {
            let given = motif.as_ref().map(Option::as_ref);
            let file = part_path(&path, &plan, at);
            taken_part(&file, &plan, at, &searched, given, lang)
        } else {
            PartFile::Missing
        };
        if kyrie_i == Some(at) {
            match &file {
                PartFile::Fits(result) => motif = Some(result.motif.clone()),
                _ => kyrie_i_again = renew || options.resume || !again,
            }
        }
        if renew {
            renewed.push(at);
        }
        files.push(file);
    }
    // --resume with other options than the mass was set with: it would replace its parts.
    if options.resume {
        let stale = stale_parts(&plan, &files, &only);
        if !stale.is_empty() {
            return Err(replaced_parts(&dir, &plan, &stale, lang));
        }
    }
    for line in &report {
        writeln!(out, "{line}")?;
    }
    let from = dir.display().to_string();
    let status = Status::new();
    // The parts the files keep, and those to set.
    let mut m = Mass::new(Arc::clone(&plan));
    let mut sources = Vec::new();
    let mut todo = Vec::new();
    for ((at, _), file) in plan.parts().zip(files) {
        let renew = renewed.contains(&at);
        match file {
            PartFile::Fits(result) => {
                let score = &result.generated.score;
                let line = PartLine::set(&plan, &result, lang);
                status.print(&line.taken(
                    score.measure_count(),
                    result.generated.cost,
                    &from,
                    lang,
                ));
                m.insert(*result);
                sources.push(Source::Taken);
            }
            _ if renew || options.resume || !again => {
                todo.push(at);
                sources.push(Source::Searched(None));
            }
            _ => sources.push(Source::Left),
        }
    }
    // The parts set at once, and the seconds the plan expects of each part to set beside the
    // others; the parts still to begin, in the order they begin.
    let jobs = jobs.clamp(1, todo.len().max(1));
    let factor = f64::from(tables::estimate_jobs(jobs));
    let estimates: Vec<f64> = todo
        .iter()
        .map(|&at| f64::from(plan.part(at).estimate.seconds) * factor)
        .collect();
    let place = |at: PartRef| todo.iter().position(|&part| part == at).unwrap_or(0);
    {
        let mut shown = status.lock();
        shown.jobs = jobs;
        shown.waiting = start_order(&plan, &todo, jobs)
            .into_iter()
            .map(|at| (at, EtaPart::of(&plan, at, estimates[place(at)])))
            .collect();
    }
    let places: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
    let xml = output.musicxml_options(lang);
    let control = MassControl::new();
    // The line of each part searched, once it is set or failed, and the next to print.
    let mut told: Vec<Option<Told>> = todo.iter().map(|_| None).collect();
    let mut next = 0;
    let mut written = 0;
    let mut failure = None;
    let m = status.during(&control, lang, || {
        let mut on = |event: MassEvent| {
            match event {
                MassEvent::Started { at, .. } => {
                    let i = place(at);
                    let mut shown = status.lock();
                    shown.waiting.retain(|&(part, _)| part != at);
                    let k = shown.running.partition_point(|part| place(part.at) < i);
                    let part = Running {
                        at,
                        line: PartLine::of(&plan, at, lang),
                        address: plan.address(at),
                        measures: plan.part(at).estimate.measures,
                        estimate: EtaPart::of(&plan, at, estimates[i]),
                        started: Instant::now(),
                    };
                    shown.running.insert(k, part);
                    return ControlFlow::Continue(());
                }
                MassEvent::Finished(result) => {
                    let i = place(result.at);
                    let seconds = f64::from(result.seconds);
                    status.lock().end(result.at, estimates[i], Some(seconds));
                    // The file first: what is set is kept, whatever follows (D16).
                    if let Err(error) = write_part(&plan, &result, &path, &xml, lang) {
                        failure = Some(error);
                        return ControlFlow::Break(());
                    }
                    written += 1;
                    let generated = &result.generated;
                    told[i] = Some(Told::Set {
                        line: PartLine::set(&plan, &result, lang),
                        measures: generated.score.measure_count(),
                        cost: generated.cost,
                        seconds,
                    });
                }
                MassEvent::Failed { at, attempt, error } => {
                    let i = place(at);
                    status.lock().end(at, estimates[i], None);
                    let why = failure_message(&error, lang);
                    let line = PartLine::of(&plan, at, lang).failed(attempt, &why, lang);
                    if let Some(place) = places.iter().position(|&place| place == at) {
                        sources[place] = Source::Searched(Some(why));
                    }
                    told[i] = Some(Told::Failed(line));
                }
                MassEvent::Planned(_) | MassEvent::Cancelled => return ControlFlow::Continue(()),
            }
            // The lines of the parts set or failed, in the order of the mass, each but the last
            // with the time the mass still takes.
            while let Some(Some(line)) = told.get(next) {
                let remaining = (next + 1 < todo.len()).then(|| status.lock().left());
                status.print(&line.line(remaining, lang));
                next += 1;
            }
            ControlFlow::Continue(())
        };
        run_mass(&request, m, &todo, &profile, Some(&control), jobs, &mut on)
    });
    // After a cancel the lines of the parts set behind one that is not.
    for line in told[next..].iter().flatten() {
        status.print(&line.line(None, lang));
    }
    if let Some(error) = failure {
        return Err(error);
    }
    // The report: a line per part, the cadence plans, the totals and the warnings.
    let printed = report.len();
    let mut failed = Vec::new();
    for ((at, result), source) in places.iter().zip(&m.results).zip(&sources) {
        let Some(result) = result else {
            let line = PartLine::of(&plan, *at, lang);
            match source {
                Source::Searched(Some(error)) => {
                    report.push(line.missing(error, lang));
                    failed.push((*at, error.clone()));
                }
                _ => report.push(line.missing(not_set(lang), lang)),
            }
            continue;
        };
        let generated = &result.generated;
        let measures = generated.score.measure_count();
        let line = PartLine::set(&plan, result, lang);
        report.push(match source {
            Source::Taken => line.taken(measures, generated.cost, &from, lang),
            _ => line.finished(
                measures,
                generated.cost,
                f64::from(result.seconds),
                None,
                lang,
            ),
        });
        if options.details {
            let summary = part_summary(&request, &plan, result, &profile, lang);
            report.extend(summary.iter().map(|line| format!("  {line}")));
        }
    }
    report.extend(cadence_plans_of(&m, lang));
    let warnings = mass_warnings(&m, lang);
    let taken = sources
        .iter()
        .filter(|source| matches!(source, Source::Taken))
        .count();
    report.push(mass_totals(&m, taken, &from, warnings.len(), lang));
    let warning = lang.pick("Warnung", "Warning");
    report.extend(warnings.iter().map(|line| format!("{warning}: {line}")));
    report.extend(incomplete(&m, lang));
    for line in &report[printed..] {
        writeln!(out, "{line}")?;
    }
    let mut files = write_mass(&m, &failed, &report, output, &options.notation, lang)?;
    if written > 0 {
        files.push(dir);
    }
    print_files(out, &files, lang)?;
    let set = m.results.iter().flatten().count();
    Ok(exit_code(set, m.results.len()))
}

/// `fux mass --rewrite` (docs/naechste-schritte.md, D9): writes the files of the mass whose parts
/// `NAME.teile/` keeps anew from them, as they are, without a search and without comparing their
/// plan and profile with what the options plan ([`mass_of_parts`]): the mass that the newest of
/// the files names ([`recorded_mass`]), in the movements that `--movements` chooses, by default
/// every movement of which the folder keeps parts, and so with all six named, which clap does not
/// tell from the default; each part checked again under the profile of today (`--profile`,
/// `--rule`, `--repeat`) and written with the notation, the formats, the title, the language and
/// the accents of the command line ([`write_mass`]). The options of the plan and the seed, which
/// cannot change a mass set already, are refused ([`planned_options`]). The report is that of
/// `fux mass` with every part from its file, its second line saying that the mass was written
/// anew, the summaries of `--details` naming the repetition of the check, since the files do not
/// keep the `--repeat` of the setting (D10 (a)), and the warnings of the parts followed by those
/// of the rewrite ([`rewrite_warnings`]) and one for each movement of which the folder keeps no
/// part, though an earlier writing left its file ([`Folder::unwritten`]); nothing goes to stderr.
/// The files of the movements it does not write stay as they are: their parts may be missing from
/// the folder. Fails, writing nothing, where the folder keeps no mass or not every part of the
/// movements chosen as it was set ([`KeptError`]); the error names the way on and the files of
/// movements that may keep the last copy of a part ([`not_rewritten`]). Returns 0, also where a
/// part breaks a prohibition of today's profile.
fn rewrite(
    out: &mut impl Write,
    options: &MassOptions,
    base: &Profile,
    rules: &[String],
    output: &OutputArgs,
    lang: Lang,
) -> Result<i32> {
    let path = output::base_of(output, DEFAULT_BASE);
    let dir = parts_dir(&path);
    let refused = planned_options(options);
    if !refused.is_empty() {
        return Err(rewrite_refused(&dir, &refused, lang));
    }
    let (request, profile) = prepare_mass(options, base, rules, lang)?;
    let Some(record) = recorded_mass(&dir) else {
        return Err(no_mass(&dir, lang));
    };
    let kept = kept_parts(&dir, &record, lang)?;
    // Without --movements, or with all six (clap gives both alike), every movement whose parts
    // the folder keeps.
    let movements: Vec<Movement> = if request.movements.len() == Movement::ALL.len() {
        Movement::ALL
            .into_iter()
            .filter(|&movement| kept.iter().any(|part| part.movement == movement))
            .collect()
    } else {
        request.movements.clone()
    };
    if movements.is_empty() {
        return Err(no_mass(&dir, lang));
    }
    let checked = part_profile(&request, &profile);
    let hash = profile_hash(&checked);
    let earlier = movement_files(&path);
    let held: Vec<Held> = kept.iter().map(|part| Held::of(part, hash)).collect();
    let folder = Folder {
        dir: &dir,
        record: &record,
        held: &held,
        chosen: &movements,
        files: &earlier,
    };
    let unwritten = folder.unwritten();
    let accents = options.singing.accents;
    let title = options.title.clone();
    let m = mass_of_parts(&record, &movements, kept, title, accents, &checked)
        .map_err(|error| not_rewritten(&folder, &error, lang))?;
    let plan = Arc::clone(&m.plan);
    let from = dir.display().to_string();
    let anew = match lang {
        Lang::De => format!("aus {from} neu geschrieben, ohne Suche"),
        Lang::En => format!("written anew from {from}, without a search"),
    };
    let mut report = plan_lines(&plan, anew, lang);
    // The summaries read the texts of the parts as their files divide and print them.
    let summarized = MassRequest {
        text: plan.text,
        accents: plan.accents,
        ..request
    };
    for result in m.results.iter().flatten() {
        let generated = &result.generated;
        let line = PartLine::set(&plan, result, lang);
        let measures = generated.score.measure_count();
        report.push(line.taken(measures, generated.cost, &from, lang));
        if options.details {
            let summary = part_summary(&summarized, &plan, result, &profile, lang);
            report.extend(summary.iter().map(|line| format!("  {line}")));
        }
    }
    report.extend(cadence_plans_of(&m, lang));
    let mut warnings = mass_warnings(&m, lang);
    warnings.extend(rewrite_warnings(&m, hash, lang));
    warnings.extend(
        unwritten
            .iter()
            .map(|(movement, files)| unwritten_warning(&dir, *movement, files, lang)),
    );
    report.push(mass_totals(
        &m,
        m.results.len(),
        &from,
        warnings.len(),
        lang,
    ));
    let warning = lang.pick("Warnung", "Warning");
    report.extend(warnings.iter().map(|line| format!("{warning}: {line}")));
    for line in &report {
        writeln!(out, "{line}")?;
    }
    let files = write_files(&m, &[], &report, output, &options.notation, false, lang)?;
    print_files(out, &files, lang)?;
    Ok(EXIT_COMPLETE)
}

/// The MusicXML and MIDI files of the movements of a mass written to `base` that an earlier
/// writing left (`NAME-3-credo.musicxml`, `.mid`; [`movement_path`]), by movement. They may keep
/// the last copy of parts that `NAME.teile/` lacks. Those of a movement of which it keeps no part
/// ([`Folder::unwritten`]) `--rewrite` leaves as they are, with a warning ([`unwritten_warning`]);
/// where it fails, its error names them ([`unwritten_files`]), and any of these files makes every
/// run of `fux mass` it names write and remove no movement file ([`Run::lean`]). A movement whose
/// first part a cancel stopped has no file and stays unseen, as the files of the parts do not
/// record the movements of the mass.
fn movement_files(base: &Path) -> Vec<(Movement, Vec<PathBuf>)> {
    Movement::ALL
        .into_iter()
        .map(|movement| {
            let files: Vec<PathBuf> = ["musicxml", "mid"]
                .into_iter()
                .map(|extension| movement_path(base, movement, extension))
                .filter(|file| file.exists())
                .collect();
            (movement, files)
        })
        .filter(|(_, files)| !files.is_empty())
        .collect()
}

/// The warning of `--rewrite` for a movement of which the folder `dir` keeps no part, though its
/// files `files` are there ([`Folder::unwritten`]), where it writes the mass.
fn unwritten_warning(dir: &Path, movement: Movement, files: &[PathBuf], lang: Lang) -> String {
    let (dir, title, files) = (dir.display(), movement.title(), file_names(files));
    match lang {
        Lang::De => format!(
            "In {dir} liegt kein Teil des {title}: --rewrite schreibt die Messe ohne diesen Satz \
             und lässt {files} unverändert"
        ),
        Lang::En => format!(
            "{dir} holds no part of the {title}: --rewrite writes the mass without this movement \
             and leaves {files} unchanged"
        ),
    }
}

/// The options of the plan and the seed that `options` give otherwise than by default, as the
/// command line names them, in the order of `fux mass --help`, which `--rewrite` refuses: the
/// mass, its plan and its seeds come from the files of its parts. So are `--plan`, `--only`,
/// `--resume` and `--jobs`, which plan or set parts or say how many are set at once.
fn planned_options(options: &MassOptions) -> Vec<&'static str> {
    let plain = MassOptions::default();
    [
        (
            options.preset.parse::<mass::Preset>().ok() != Some(mass::Preset::default()),
            "--preset",
        ),
        (options.voices != plain.voices, "--voices"),
        (options.parts.is_some(), "--parts"),
        (options.mode.is_some(), "--mode"),
        (options.scoring.is_some(), "--scoring"),
        (options.imitation.is_some(), "--imitation"),
        (options.endings.is_some(), "--endings"),
        (
            parse_chunk(&options.chunk).ok() != Some(Some(tables::CHUNK)),
            "--chunk",
        ),
        (
            options.lengths.parse::<mass::Lengths>().ok() != Some(mass::Lengths::default()),
            "--lengths",
        ),
        (options.motif, "--motif"),
        (parse_chant(&options.chant).ok() != Some(None), "--chant"),
        (
            options.singing.eleison != plain.singing.eleison,
            "--eleison",
        ),
        (options.seed.is_some(), "--seed"),
        (options.beam != plain.beam, "--beam"),
        (options.noise != plain.noise, "--noise"),
        (options.plan, "--plan"),
        (!options.only.is_empty(), "--only"),
        (options.resume, "--resume"),
        (options.jobs.is_some(), "--jobs"),
    ]
    .into_iter()
    .filter_map(|(given, flag)| given.then_some(flag))
    .collect()
}

/// The error of `--rewrite` with the options of the plan or the seed `refused`
/// ([`planned_options`]): the mass whose parts the folder `dir` keeps is written as it was set,
/// which they cannot change; the error names them and the ways to other parts or another mass.
fn rewrite_refused(dir: &Path, refused: &[&str], lang: Lang) -> anyhow::Error {
    let (dir, refused) = (dir.display(), refused.join(", "));
    anyhow!(
        "{}",
        match lang {
            Lang::De => format!(
                "--rewrite schreibt die Messe aus ihren Teilen in {dir} neu, wie sie gesetzt \
                 wurde: Form, Plan und Seeds kommen aus den Dateien, und nichts wird gesetzt. \
                 --rewrite nimmt {refused} nicht an; Teile mit anderen Angaben setzt fux mass \
                 --only neu, eine neue Messe fux mass ohne --rewrite."
            ),
            Lang::En => format!(
                "--rewrite writes the mass anew from its parts in {dir} as it was set: form, plan \
                 and seeds come from the files, and nothing is set. Leave out {refused}; fux mass \
                 --only sets parts again with other options, fux mass without --rewrite sets a \
                 new mass."
            ),
        }
    )
}

/// The error of `--rewrite` where the folder `dir` keeps no part of a mass.
fn no_mass(dir: &Path, lang: Lang) -> anyhow::Error {
    let dir = dir.display();
    anyhow!(
        "{}",
        match lang {
            Lang::De => format!(
                "In {dir} liegen keine Teile einer Messe; --rewrite schreibt eine Messe aus den \
                 Teilen neu, die fux mass dort beim Setzen ablegt."
            ),
            Lang::En => format!(
                "{dir} holds no parts of a mass; --rewrite writes a mass anew from the parts that \
                 fux mass keeps there as it sets them."
            ),
        }
    )
}

/// A part that the folder of a mass keeps, as the ways on of a failed `--rewrite` read it
/// ([`not_rewritten`]): its movement and its number in it, its ending and the ending of the part
/// before it as its file keeps them (`end=`, `before=`), the mode of its movement, and whether
/// this version of the generator set it under the rule profile of the check (`fits`): only then
/// do `--resume` and `--only` with the options of its setting take it from its file
/// ([`taken_part`]).
#[derive(Clone, Copy, Debug)]
struct Held {
    movement: Movement,
    number: usize,
    ending: Step,
    before: Option<Step>,
    mode: ModeSpec,
    fits: bool,
}

impl Held {
    /// What the ways on read of `part`, checked under the profile numbered `hash`.
    fn of(part: &KeptPart, hash: u64) -> Held {
        Held {
            movement: part.movement,
            number: part.number,
            ending: part.field.ending,
            before: part.field.before,
            mode: part.score.mode,
            fits: part.field.profile == hash && part.field.generator == mass::GENERATOR,
        }
    }
}

/// The folder `dir` of a mass that `--rewrite` cannot write anew, as the ways on read it
/// ([`not_rewritten`]): the mass its files name (`record`), the parts it keeps (`held`), the
/// movements `--rewrite` was to write (`chosen`), and the files of the movements of the mass that
/// are there (`files`, [`movement_files`]).
struct Folder<'a> {
    dir: &'a Path,
    record: &'a MassField,
    held: &'a [Held],
    chosen: &'a [Movement],
    files: &'a [(Movement, Vec<PathBuf>)],
}

/// An item of `--endings` that a way on names (`kyrie/1=A`), after the movement and the number of
/// its part, by which the items go in the order of the mass.
type Pin = (Movement, usize, String);

impl Folder<'_> {
    /// The part numbered `number` of `movement`, where the folder keeps it.
    fn part(&self, movement: Movement, number: usize) -> Option<&Held> {
        self.held
            .iter()
            .find(|held| (held.movement, held.number) == (movement, number))
    }

    /// The movements of which the folder keeps parts.
    fn kept(&self) -> Vec<Movement> {
        ordered(self.held.iter().map(|held| held.movement))
    }

    /// The files of the movements of which the folder keeps no part, though they are there: the
    /// last copy of their parts, it may be.
    fn unwritten(&self) -> Vec<&(Movement, Vec<PathBuf>)> {
        let kept = self.kept();
        self.files
            .iter()
            .filter(|(movement, _)| !kept.contains(movement))
            .collect()
    }

    /// The files of `movement` that are there.
    fn files_of(&self, movement: Movement) -> &[PathBuf] {
        self.files
            .iter()
            .find(|(other, _)| *other == movement)
            .map_or(&[], |(_, files)| files.as_slice())
    }

    /// `movements` as a run of `fux mass` names them in `--movements`, in the order of the mass:
    /// with the Kyrie in a mass with the head motif ([`kyrie_of_the_motif`]), and with the Sanctus
    /// where one of them repeats its Osanna (the Benedictus of the form normal, MASS-FORM), as
    /// `fux mass` refuses the run without them.
    fn run_movements(&self, movements: impl IntoIterator<Item = Movement>) -> Vec<Movement> {
        let movements: Vec<Movement> = movements.into_iter().collect();
        let repeats = movements.iter().any(|&movement| {
            mass::preset::form(self.record.preset, movement)
                .sections
                .iter()
                .any(|section| section.ut_supra.is_some())
        });
        let kyrie = self.record.motif.then_some(Movement::Kyrie);
        let sanctus = repeats.then_some(Movement::Sanctus);
        ordered(movements.into_iter().chain(kyrie).chain(sanctus))
    }

    /// The movements of a run of `fux mass` that sets parts for `--rewrite`, besides `more`:
    /// those chosen of which the folder keeps parts, with those that `fux mass` needs beside them
    /// ([`Folder::run_movements`]).
    fn movements(&self, more: &[Movement]) -> Vec<Movement> {
        let kept = self.kept();
        let chosen = self
            .chosen
            .iter()
            .copied()
            .filter(|movement| kept.contains(movement));
        self.run_movements(chosen.chain(more.iter().copied()))
    }

    /// Whether another version of the generator or another rule profile than the check's set a
    /// part kept of `movements`, which `--resume` with the options of the setting would then not
    /// take from its file, but set again or refuse ([`stale_parts`]).
    fn changed(&self, movements: &[Movement]) -> bool {
        self.held
            .iter()
            .any(|held| movements.contains(&held.movement) && !held.fits)
    }

    /// Whether the folder keeps Kyrie I as this version set it under the profile of the check, so
    /// that `--only` takes the head motif from its file.
    fn kyrie_i_fits(&self) -> bool {
        self.part(Movement::Kyrie, 1).is_some_and(|held| held.fits)
    }

    /// Whether a run of `fux mass`, which names the Kyrie in a mass with the head motif, sets
    /// Kyrie I anew, as the folder keeps no file of it: the new Kyrie I gives its motif to every
    /// part that takes it (MASS-MOTIF), after a change perhaps another than the parts kept took.
    fn kyrie_i_anew(&self) -> bool {
        self.record.motif && self.part(Movement::Kyrie, 1).is_none()
    }

    /// The sections of the form of `movement`, which name its parts in `--only` without a plan;
    /// an Osanna ut supra has none.
    fn sections(&self, movement: Movement) -> String {
        let ids: Vec<&str> = mass::preset::form(self.record.preset, movement)
            .sections
            .iter()
            .filter(|section| section.ut_supra.is_none())
            .map(|section| section.id)
            .collect();
        ids.join(",")
    }

    /// The item of `--endings` that keeps the ending of the part before the part `number` of
    /// `movement` as its file has it, so that a run that sets the part plans it after that
    /// ending, as `--rewrite` asks ([`KeptError::Before`]); none for the first part of a movement.
    fn ending_before(&self, movement: Movement, number: usize, lang: Lang) -> Option<Pin> {
        let before = self.part(movement, number.checked_sub(1)?)?;
        Some(pin(
            movement,
            before.number,
            before.mode,
            before.ending,
            lang,
        ))
    }

    /// The item of `--endings` that ends the part `number` of `movement` on the ending that the
    /// file of the part after it was set after (`before=`), so that a run that sets the part ends
    /// it there; none where the folder does not keep that part or its file lacks the item (the
    /// files of phase 14a).
    fn ending_of(&self, movement: Movement, number: usize, lang: Lang) -> Option<Pin> {
        let after = self.part(movement, number + 1)?;
        Some(pin(movement, number, after.mode, after.before?, lang))
    }

    /// The items of `--endings` of a run that sets Kyrie I again, and with it the first part of
    /// each of `movements` that takes its head motif (MASS-MOTIF): each to end where the part after
    /// it was set after ([`Folder::ending_of`]).
    fn motif_endings(&self, movements: &[Movement], lang: Lang) -> Vec<Pin> {
        movements
            .iter()
            .filter(|&&movement| movement == Movement::Kyrie || takes_motif(movement))
            .filter_map(|&movement| self.ending_of(movement, 1, lang))
            .collect()
    }
}

/// The item of `--endings` that ends the part `number` of `movement`, in `mode`, on `step`,
/// spelled as `fux mass --plan` spells the degree (`kyrie/1=A`; `H` for B in German).
fn pin(movement: Movement, number: usize, mode: ModeSpec, step: Step, lang: Lang) -> Pin {
    let item = format!(
        "{}/{number}={}",
        movement.id(),
        degree_name(mode, step, lang)
    );
    (movement, number, item)
}

/// `movements` in the order of the mass, each once.
fn ordered(movements: impl IntoIterator<Item = Movement>) -> Vec<Movement> {
    let movements: Vec<Movement> = movements.into_iter().collect();
    Movement::ALL
        .into_iter()
        .filter(|movement| movements.contains(movement))
        .collect()
}

/// Whether the first part of `movement` takes the head motif of a mass with one where it begins in
/// imitation (MASS-MOTIF): in Gloria, Credo, Sanctus and Agnus Dei I.
fn takes_motif(movement: Movement) -> bool {
    matches!(
        movement,
        Movement::Gloria | Movement::Credo | Movement::Sanctus | Movement::Agnus
    )
}

/// The movement and the number of the part at `address` (`credo/5`), as [`KeptError`] names it.
fn address_of(address: &str) -> Option<(Movement, usize)> {
    let (movement, number) = address.split_once('/')?;
    Some((movement.parse().ok()?, number.parse().ok()?))
}

/// A run of `fux mass` that a failed `--rewrite` names as its way on ([`way_on`]).
struct Run {
    /// What `--only` names, as the command line writes it; `None` for `--resume`.
    only: Option<String>,
    /// Its movements, which it names, since the files do not keep those of the mass (D10 (a)).
    movements: Vec<Movement>,
    /// The items for the end of `--endings`, in the order of the mass ([`Folder::ending_before`],
    /// [`Folder::ending_of`]).
    endings: Vec<String>,
    /// The movements whose parts it sets though the folder keeps none of them.
    sets: Vec<Movement>,
    /// `-f ly`: it writes and removes no file of a movement ([`write_files`]), as it is named
    /// wherever a file of a movement of the mass is there ([`movement_files`]), which may keep the
    /// last copy of parts that the folder lacks, whether the error names them or not.
    lean: bool,
}

impl Run {
    /// `fux mass --resume` with `movements`, which sets the parts they lack in `folder`.
    fn resume(folder: &Folder<'_>, movements: Vec<Movement>) -> Run {
        Run {
            only: None,
            sets: movements.clone(),
            movements,
            endings: Vec::new(),
            lean: !folder.files.is_empty(),
        }
    }

    /// `fux mass --only` with `parts` and `movements`, setting the parts it names with the
    /// endings `pins`, each part once.
    fn only(
        folder: &Folder<'_>,
        parts: String,
        movements: Vec<Movement>,
        mut pins: Vec<Pin>,
    ) -> Run {
        pins.sort();
        pins.dedup_by(|a, b| (a.0, a.1) == (b.0, b.1));
        Run {
            only: Some(parts),
            movements,
            endings: pins.into_iter().map(|(_, _, item)| item).collect(),
            sets: Vec::new(),
            lean: !folder.files.is_empty(),
        }
    }

    /// The run as the command line names it, without the other options of the setting:
    /// `fux mass --only kyrie/2 --movements kyrie -f ly`.
    fn command(&self) -> String {
        let named = match &self.only {
            Some(parts) => format!("--only {parts}"),
            None => "--resume".to_string(),
        };
        let movements: Vec<&str> = self
            .movements
            .iter()
            .map(|movement| movement.id())
            .collect();
        let lean = if self.lean { " -f ly" } else { "" };
        format!("fux mass {named} --movements {}{lean}", movements.join(","))
    }

    /// The sentence that names its items for the end of `--endings`, which the setting's
    /// `--endings` may name otherwise (a later item wins, and the option is given once); nothing
    /// without items.
    fn pinned(&self, lang: Lang) -> String {
        if self.endings.is_empty() {
            return String::new();
        }
        let items = self.endings.join(";");
        let one = self.endings.len() == 1;
        match lang {
            Lang::De => format!(
                " Ans Ende von --endings gehören dabei {items}: {}, die die Dateien halten.",
                if one {
                    "die Schlussstufe"
                } else {
                    "die Schlussstufen"
                }
            ),
            Lang::En => format!(
                " At the end of --endings, add {items}: {} the files keep.",
                if one { "the ending" } else { "the endings" }
            ),
        }
    }
}

/// „with the other options of the setting“, as a way on names them besides its own.
fn setting(lang: Lang) -> &'static str {
    lang.pick(
        "mit den übrigen Angaben des Setzens",
        "with the other options of the setting",
    )
}

/// That `--only` takes no `--seed`, which would give the parts it sets a seed of their own.
fn unseeded(lang: Lang) -> &'static str {
    lang.pick(" (ohne --seed)", " (without --seed)")
}

/// Why a way on names `--only` where another version or profile set parts kept.
fn resumed(lang: Lang) -> &'static str {
    lang.pick(
        " Nach einer Änderung von Programm oder Regelprofil nähme --resume die übrigen Teile nicht \
         mehr aus ihren Dateien.",
        " After a change of program or rule profile, --resume would no longer take the other parts \
         from their files.",
    )
}

/// What `--only` does with the parts that do not fit, and what follows it.
fn left(lang: Lang) -> &'static str {
    lang.pick(
        " Teile, die nicht passen, lässt --only, wie sie sind; dann endet es unvollständig \
         (Rückgabewert 3), und statt seines Hinweises auf --resume schreibt --rewrite danach die \
         Messe.",
        " --only leaves the parts that do not fit as they are; it then ends incomplete (exit code \
         3), and instead of its hint at --resume, --rewrite then writes the mass.",
    )
}

/// Why a way on that sets a part with the head motif sets Kyrie I again.
fn fitting(lang: Lang) -> &'static str {
    lang.pick(
        " Das Kopfmotiv nehmen --resume und --only nur aus der Datei eines Kyrie I, das passt.",
        " --resume and --only take the head motif only from the file of a Kyrie I that fits.",
    )
}

/// What a run of `fux mass` that sets Kyrie I again does (MASS-MOTIF): it sets `what` and Kyrie I,
/// and with it every part that takes its head motif, whose files it replaces.
fn renewal(what: Option<&str>, lang: Lang) -> String {
    match (lang, what) {
        (Lang::De, None) => "setzt das Kyrie I und mit ihm jeden Teil mit dem Kopfmotiv neu und \
                             ersetzt deren Dateien"
            .to_string(),
        (Lang::De, Some(what)) => format!(
            "setzt {what} und das Kyrie I und mit ihm jeden Teil mit dem Kopfmotiv neu und ersetzt \
             deren Dateien"
        ),
        (Lang::En, None) => "sets Kyrie I again and with it every part that takes its head motif, \
                             replacing their files"
            .to_string(),
        (Lang::En, Some(what)) => format!(
            "sets {what}, and Kyrie I again and with it every part that takes its head motif, \
             replacing their files"
        ),
    }
}

/// What a run of `fux mass` that sets the Kyrie of a mass with the head motif, of which the folder
/// keeps no part, does (MASS-MOTIF): it sets `what` and the Kyrie, and with its Kyrie I every part
/// that takes the motif, whose files it replaces.
fn kyrie_renewal(what: Option<&str>, lang: Lang) -> String {
    match (lang, what) {
        (Lang::De, None) => "setzt das Kyrie und mit dessen Kyrie I jeden Teil mit dem Kopfmotiv \
                             neu und ersetzt deren Dateien"
            .to_string(),
        (Lang::De, Some(what)) => format!(
            "setzt {what} und das Kyrie und mit dessen Kyrie I jeden Teil mit dem Kopfmotiv neu und \
             ersetzt deren Dateien"
        ),
        (Lang::En, None) => "sets the Kyrie, and with its Kyrie I every part that takes the head \
                             motif again, replacing their files"
            .to_string(),
        (Lang::En, Some(what)) => format!(
            "sets {what} and the Kyrie, and with its Kyrie I every part that takes the head motif \
             again, replacing their files"
        ),
    }
}

/// Why a run that sets the Kyrie of a mass with the head motif sets every part that takes the
/// motif again, in every movement of which the folder keeps parts.
fn new_motif(lang: Lang) -> &'static str {
    lang.pick(
        " Ein neues Kyrie I kann ein anderes Kopfmotiv geben, als die Teile in den Dateien nahmen.",
        " A new Kyrie I may give another head motif than the parts in the files took.",
    )
}

/// `movements`, each with `article` before its title, as a sentence lists them: „das Gloria, das
/// Credo und das Sanctus“.
fn listed(movements: &[Movement], article: &str, lang: Lang) -> String {
    let named: Vec<String> = movements
        .iter()
        .map(|movement| format!("{article} {}", movement.title()))
        .collect();
    match named.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} {} {last}", rest.join(", "), lang.pick("und", "and")),
        None => String::new(),
    }
}

/// The names of `files`, as a message lists them.
fn file_names(files: &[PathBuf]) -> String {
    let names: Vec<String> = files
        .iter()
        .map(|file| file.display().to_string())
        .collect();
    names.join(", ")
}

/// The error of `--rewrite` where the parts that `folder` keeps do not make the mass as it was set
/// (`error`): why; where a part is missing and the files of its movement are there, that these may
/// keep its last copy ([`last_copy`]); how to go on, where something helps ([`way_on`]); the files
/// of the movements of which the folder keeps no part, and where the way on is a run of
/// `fux mass` and a file of a movement is there, that its `-f ly` keeps them
/// ([`unwritten_files`]); and where a movement lacks parts, that `--movements` can leave it out.
fn not_rewritten(folder: &Folder<'_>, error: &KeptError, lang: Lang) -> anyhow::Error {
    let (dir, seed) = (folder.dir.display(), folder.record.seed);
    let why = sentence(error.message(lang));
    let copy = match error {
        KeptError::Missing { part, later } => last_copy(folder, part, *later, lang),
        _ => String::new(),
    };
    let (remedy, run) = way_on(folder, error, lang);
    let files = unwritten_files(folder, run.as_ref(), lang);
    let chosen = match error {
        KeptError::NoParts { .. } | KeptError::Missing { .. } => lang.pick(
            " Mit --movements schreibt --rewrite nur Sätze, deren Teile alle vorliegen, und lässt \
             die Dateien der übrigen unverändert.",
            " With --movements, --rewrite writes only movements whose parts are all there and \
             leaves the files of the others unchanged.",
        ),
        _ => "",
    };
    anyhow!(
        "{}",
        match lang {
            Lang::De => format!(
                "--rewrite kann die Messe mit Seed {seed} in {dir} nicht neu schreiben. \
                 {why}{copy}{remedy}{files}{chosen}"
            ),
            Lang::En => format!(
                "--rewrite cannot write the mass with seed {seed} in {dir} anew. \
                 {why}{copy}{remedy}{files}{chosen}"
            ),
        }
    )
}

/// How a `--rewrite` of `folder` that failed with `error` goes on, as a text that begins with a
/// space, and the run of `fux mass` it names, where something helps: nothing where joins, titles or
/// texts do not make the sections of the form, the parts of a movement do not go together, or a
/// file does not give back its part.
///
/// A run names its movements, which the files do not keep (D10 (a)): those chosen of which the
/// folder keeps parts, with those that `fux mass` needs beside them, the Kyrie in a mass with the
/// head motif ([`kyrie_of_the_motif`]) and the Sanctus beside the Benedictus of the form normal,
/// whose Osanna ut supra repeats its Osanna ([`Folder::run_movements`]); and `-f ly` wherever a
/// file of a movement of the mass is there, so that it writes and removes none ([`Run::lean`]):
/// such a file may keep the last copy of a part that the folder lacks, whether the error names it
/// or not. Where this version of the generator set every part kept of its movements under the
/// rule profile of the check, `--resume` with the options of the setting sets the missing parts, a
/// part that took another head motif than Kyrie I gives, and a movement without parts (with the
/// Kyrie, whose parts it takes from their files). Otherwise it would set those parts again or
/// refuse ([`stale_parts`]): `--only` sets the parts by their addresses, a movement by its
/// sections, without `--seed`, and leaves the parts that do not fit, so that it ends incomplete
/// and `--rewrite` follows. Of the parts missing at the end of a movement it names the first,
/// since the files do not say how many today's plan has after it; `fux mass --plan --details`
/// lists them, and the next `--rewrite` names the next one missing. `--only` plans the parts it
/// sets anew, after the endings the plan of today draws: so that they fit the parts kept, the way
/// on names the endings that these keep for the end of `--endings` ([`Run::pinned`]), the ending
/// of the part before and the ending that the part after was set after. A part with the head motif
/// takes it only from a Kyrie I that fits, and a run that sets Kyrie I anew gives the motif anew to
/// every part that takes it: such a run is `--resume` only where every part kept, of every
/// movement, fits; otherwise `--only` sets Kyrie I again, or the Kyrie where the folder keeps no
/// part of it, and with it every part that takes its motif in the movements the folder keeps, each
/// to end where the part after it was set after ([`Folder::motif_endings`]). A part set after
/// another ending of the part before it `--only` sets again with the options that part was set
/// with, whatever changed: `--resume` refuses it, as its movement is whole and one of the two
/// parts differs from the plan ([`taken_part`]). The Osanna ut supra wants the Sanctus: in
/// `--movements` where the folder keeps its parts, else set as a movement without parts.
fn way_on(folder: &Folder<'_>, error: &KeptError, lang: Lang) -> (String, Option<Run>) {
    match error {
        KeptError::NoParts { movement } => movement_way(folder, *movement, lang),
        KeptError::Missing { part, later } => missing_way(folder, part, *later, lang),
        KeptError::Before { part } => before_way(folder, part, lang),
        KeptError::Motif { part } => motif_way(folder, part, lang),
        KeptError::UtSupra if folder.kept().contains(&Movement::Sanctus) => {
            let text = lang.pick(
                " Mit dem Sanctus in --movements schreibt --rewrite beide.",
                " With the Sanctus in --movements, --rewrite writes both.",
            );
            (text.to_string(), None)
        }
        KeptError::UtSupra => movement_way(folder, Movement::Sanctus, lang),
        KeptError::Twice { .. } => {
            let text = lang.pick(
                " Die Dateien einer Messe mit anderen Angaben (etwa --chunk) gehören in einen \
                 anderen Ordner.",
                " The files of a mass with other options (such as --chunk) belong in another \
                 folder.",
            );
            (text.to_string(), None)
        }
        _ => (String::new(), None),
    }
}

/// The way on where the folder keeps no part of `movement`, which `--rewrite` was to write
/// ([`KeptError::NoParts`]), or of the Sanctus whose Osanna the Osanna ut supra repeats
/// ([`KeptError::UtSupra`]): `--resume` with the movement and those that `fux mass` needs beside
/// it ([`Folder::run_movements`]), where the parts kept of these fit, and where it sets Kyrie I
/// anew ([`Folder::kyrie_i_anew`]), every part kept; else `--only` with the sections of its form,
/// and with Kyrie I, or the Kyrie, and every movement kept where the movement may open with the
/// motif and Kyrie I does not fit, or the folder keeps no part of the Kyrie ([`way_on`]).
fn movement_way(folder: &Folder<'_>, movement: Movement, lang: Lang) -> (String, Option<Run>) {
    let title = movement.title();
    let kept = folder.kept();
    let movements = folder.run_movements([movement]);
    // A run that sets Kyrie I anew gives its motif anew to every part that takes it, in whichever
    // movement: the parts kept of all of them must fit.
    let checked = if folder.kyrie_i_anew() {
        &kept
    } else {
        &movements
    };
    // Where the folder keeps no part of the Kyrie, its sections stand for Kyrie I.
    let kyrie = folder.record.motif && !kept.contains(&Movement::Kyrie);
    let (setting, unseeded) = (setting(lang), unseeded(lang));
    if !folder.changed(checked) {
        let run = Run::resume(folder, movements.clone());
        let (taken, set): (Vec<Movement>, Vec<Movement>) = movements
            .iter()
            .copied()
            .filter(|&other| other != movement)
            .partition(|other| kept.contains(other));
        let named: Vec<Movement> = std::iter::once(movement).chain(set).collect();
        // Where it sets the missing Kyrie I beside parts it takes from their files, of the Kyrie or
        // of a movement that may open with the motif, it says that it sets Kyrie I and with it
        // every part that takes the motif.
        let renews = folder.kyrie_i_anew()
            && taken
                .iter()
                .any(|&other| other == Movement::Kyrie || takes_motif(other));
        let command = run.command();
        let text = match lang {
            Lang::De => {
                let named = listed(&named, "das", lang);
                let taken = if taken.is_empty() {
                    String::new()
                } else {
                    let taken = listed(&taken, "des", lang);
                    format!(" und nimmt die Teile {taken} aus ihren Dateien")
                };
                if renews {
                    format!(
                        " {command} {setting} setzt {named}{taken}; mit dem fehlenden Kyrie I \
                         setzt es jeden Teil mit dem Kopfmotiv neu."
                    )
                } else {
                    format!(
                        " {command} {setting} setzt {named}{taken}, ohne Teile anderer Sätze zu \
                         ersetzen."
                    )
                }
            }
            Lang::En => {
                let named = listed(&named, "the", lang);
                let taken = if taken.is_empty() {
                    String::new()
                } else {
                    let taken = listed(&taken, "the", lang);
                    format!(" and takes the parts of {taken} from their files")
                };
                if renews {
                    format!(
                        " {command} {setting} sets {named}{taken}; with the missing Kyrie I, it \
                         sets every part that takes its head motif again."
                    )
                } else {
                    format!(
                        " {command} {setting} sets {named}{taken} without replacing parts of \
                         other movements."
                    )
                }
            }
        };
        return (text, Some(run));
    }
    if !kyrie && (!takes_motif(movement) || folder.kyrie_i_fits()) {
        let mut run = Run::only(folder, folder.sections(movement), movements, Vec::new());
        run.sets = vec![movement];
        let command = run.command();
        let text = match lang {
            Lang::De => format!(
                " {command} {setting}{unseeded} setzt das {title}, ohne Teile anderer Sätze zu \
                 ersetzen."
            ),
            Lang::En => format!(
                " {command} {setting}{unseeded} sets the {title} without replacing parts of other \
                 movements."
            ),
        };
        return (format!("{text}{}{}", resumed(lang), left(lang)), Some(run));
    }
    let movements = folder.run_movements(kept.iter().copied().chain(movements));
    let pins = folder.motif_endings(&movements, lang);
    let sections = folder.sections(movement);
    let (named, sets) = match (kyrie, movement) {
        (true, Movement::Kyrie) => (sections, vec![movement]),
        (true, _) => (
            format!("{},{sections}", folder.sections(Movement::Kyrie)),
            vec![Movement::Kyrie, movement],
        ),
        (false, _) => (format!("kyrie/1,{sections}"), vec![movement]),
    };
    let mut run = Run::only(folder, named, movements, pins);
    run.sets = sets;
    let what = (movement != Movement::Kyrie).then(|| match lang {
        Lang::De => format!("das {title}"),
        Lang::En => format!("the {title}"),
    });
    let (renewed, why) = if kyrie {
        (kyrie_renewal(what.as_deref(), lang), new_motif(lang))
    } else {
        (renewal(what.as_deref(), lang), fitting(lang))
    };
    let text = format!(
        " {} {setting}{unseeded} {renewed}.{}{}{why}{}",
        run.command(),
        run.pinned(lang),
        resumed(lang),
        left(lang)
    );
    (text, Some(run))
}

/// The way on where the part at `part` is missing, or with `later` the parts from it on
/// ([`KeptError::Missing`]): `--resume` where the parts kept of its movements fit, and where it
/// sets Kyrie I anew ([`Folder::kyrie_i_anew`]), every part kept; else `--only` with the part,
/// after the ending of the part before and, where the part after is kept, ending where that was
/// set after; with Kyrie I, or the sections of the Kyrie where the folder keeps none of it, where
/// it is Kyrie I of a mass with the head motif, or a part that may open with the motif while
/// Kyrie I does not fit ([`way_on`]). Of parts missing at the end of a movement, `--only` names
/// the first: the error says that `fux mass --plan --details` lists the others.
fn missing_way(folder: &Folder<'_>, part: &str, later: bool, lang: Lang) -> (String, Option<Run>) {
    let Some((movement, number)) = address_of(part) else {
        return (String::new(), None);
    };
    let movements = folder.movements(&[]);
    let source = folder.record.motif && (movement, number) == (Movement::Kyrie, 1);
    let setting = setting(lang);
    // A run that sets Kyrie I anew gives its motif anew, also to the parts kept outside the run.
    let checked = if folder.kyrie_i_anew() {
        folder.kept()
    } else {
        movements.clone()
    };
    if !folder.changed(&checked) {
        let run = Run::resume(folder, movements);
        let scope = if folder.kyrie_i_anew() {
            lang.pick(
                " Da das Kyrie I fehlt, setzt es mit ihm jeden Teil mit dem Kopfmotiv neu.",
                " As Kyrie I is missing, it sets with it every part that takes its head motif \
                 again.",
            )
        } else {
            ""
        };
        let command = run.command();
        let text = match lang {
            Lang::De => format!(" Fehlende Teile setzt {command} {setting} nach.{scope}"),
            Lang::En => format!(" {command} {setting} sets missing parts.{scope}"),
        };
        return (text, Some(run));
    }
    let mut pins: Vec<Pin> = folder
        .ending_before(movement, number, lang)
        .into_iter()
        .collect();
    if !later {
        pins.extend(folder.ending_of(movement, number, lang));
    }
    let unseeded = unseeded(lang);
    let takes = folder.record.motif && number == 1 && takes_motif(movement);
    if source || (takes && !folder.kyrie_i_fits()) {
        let movements = folder.movements(&folder.kept());
        pins.extend(folder.motif_endings(&movements, lang));
        // Where the folder keeps no part of the Kyrie, its sections stand for Kyrie I.
        let whole = !folder.kept().contains(&Movement::Kyrie);
        let kyrie_i = if whole {
            folder.sections(Movement::Kyrie)
        } else {
            "kyrie/1".to_string()
        };
        let (named, what, why) = if source {
            (kyrie_i, None, "")
        } else {
            (format!("{kyrie_i},{part}"), Some(part), fitting(lang))
        };
        let renewed = if whole {
            kyrie_renewal(what, lang)
        } else {
            renewal(what, lang)
        };
        let mut run = Run::only(folder, named, movements, pins);
        if whole {
            run.sets = vec![Movement::Kyrie];
        }
        let text = format!(
            " {} {setting}{unseeded} {renewed}.{}{}{why}{}",
            run.command(),
            run.pinned(lang),
            resumed(lang),
            left(lang)
        );
        return (text, Some(run));
    }
    let run = Run::only(folder, part.to_string(), movements, pins);
    let command = run.command();
    let text = match (lang, later) {
        (Lang::De, false) => format!(" {command} {setting}{unseeded} setzt ihn nach."),
        (Lang::De, true) => format!(" {command} {setting}{unseeded} setzt {part} nach."),
        (Lang::En, false) => format!(" {command} {setting}{unseeded} sets it."),
        (Lang::En, true) => format!(" {command} {setting}{unseeded} sets {part}."),
    };
    // How many parts today's plan has after it, the files do not say.
    let title = movement.title();
    let after = match (lang, later) {
        (_, false) => String::new(),
        (Lang::De, true) => format!(
            " Die Teile des {title} danach nennt fux mass --plan --details mit denselben Angaben; \
             mit ihren Adressen in --only setzt es auch sie, sonst nennt das nächste --rewrite den \
             nächsten fehlenden Teil."
        ),
        (Lang::En, true) => format!(
            " fux mass --plan --details with the same options lists the parts of the {title} \
             after it; with their addresses in --only, it sets them as well, else the next \
             --rewrite names the next part missing."
        ),
    };
    let text = format!(
        "{text}{}{after}{}{}",
        run.pinned(lang),
        resumed(lang),
        left(lang)
    );
    (text, Some(run))
}

/// Where the part at `part` is missing, or with `later` the parts from it on
/// ([`KeptError::Missing`]), and the files of its movement are there ([`Folder::files_of`]): that
/// these may keep it as it was set, as its last copy, which `--rewrite` writes anew once it is set
/// again, and that a copy elsewhere keeps it. The runs of the way on write and remove no movement
/// file ([`Run::lean`]).
fn last_copy(folder: &Folder<'_>, part: &str, later: bool, lang: Lang) -> String {
    let files = address_of(part).map_or(&[][..], |(movement, _)| folder.files_of(movement));
    if files.is_empty() {
        return String::new();
    }
    let files = file_names(files);
    match (lang, later) {
        (Lang::De, false) => format!(
            " Wie er gesetzt wurde, kann der Teil nur noch in {files} stehen: Ist er neu gesetzt, \
             schreibt --rewrite diese Dateien neu; eine Kopie an anderem Ort behält ihn."
        ),
        (Lang::De, true) => format!(
            " Wie sie gesetzt wurden, können diese Teile nur noch in {files} stehen: Sind sie neu \
             gesetzt, schreibt --rewrite diese Dateien neu; eine Kopie an anderem Ort behält sie."
        ),
        (Lang::En, false) => format!(
            " As it was set, the part may now stand only in {files}: once it is set again, \
             --rewrite writes these files anew; a copy elsewhere keeps it."
        ),
        (Lang::En, true) => format!(
            " As they were set, these parts may now stand only in {files}: once they are set \
             again, --rewrite writes these files anew; a copy elsewhere keeps them."
        ),
    }
}

/// The way on where the part at `part` was set after another ending of the part before it than
/// that part's file keeps ([`KeptError::Before`]): `--only` with the part and the options the part
/// before it was set with, after its ending and, where the part after it is kept, ending where
/// that was set after ([`way_on`]).
fn before_way(folder: &Folder<'_>, part: &str, lang: Lang) -> (String, Option<Run>) {
    let Some((movement, number)) = address_of(part) else {
        return (String::new(), None);
    };
    let movements = folder.movements(&[]);
    let left = if folder.changed(&movements) {
        left(lang)
    } else {
        ""
    };
    let pins = [
        folder.ending_before(movement, number, lang),
        folder.ending_of(movement, number, lang),
    ];
    let run = Run::only(
        folder,
        part.to_string(),
        movements,
        pins.into_iter().flatten().collect(),
    );
    let command = run.command();
    let text = match lang {
        Lang::De => format!(
            " Den Teil setzt {command} mit den übrigen Angaben neu, mit denen der Teil davor \
             gesetzt wurde (ohne --seed)."
        ),
        Lang::En => format!(
            " {command} with the other options the part before it was set with (without --seed) \
             sets it again."
        ),
    };
    (format!("{text}{}{left}", run.pinned(lang)), Some(run))
}

/// The way on where the part at `part` took another head motif than Kyrie I in the files gives
/// ([`KeptError::Motif`]): `--resume` where the parts kept of its movements fit; else `--only`
/// with the part where Kyrie I fits, whose motif it takes, or with Kyrie I, which sets every part
/// with the motif again ([`way_on`]).
fn motif_way(folder: &Folder<'_>, part: &str, lang: Lang) -> (String, Option<Run>) {
    let Some((movement, number)) = address_of(part) else {
        return (String::new(), None);
    };
    let movements = folder.movements(&[]);
    let (setting, unseeded) = (setting(lang), unseeded(lang));
    if !folder.changed(&movements) {
        let run = Run::resume(folder, movements);
        let command = run.command();
        let text = match lang {
            Lang::De => format!(" Einen solchen Teil setzt {command} {setting} neu."),
            Lang::En => format!(" {command} {setting} sets such a part again."),
        };
        return (text, Some(run));
    }
    if folder.kyrie_i_fits() {
        let pins = [
            folder.ending_before(movement, number, lang),
            folder.ending_of(movement, number, lang),
        ];
        let run = Run::only(
            folder,
            part.to_string(),
            movements,
            pins.into_iter().flatten().collect(),
        );
        let command = run.command();
        let text = match lang {
            Lang::De => format!(
                " {command} {setting}{unseeded} setzt ihn mit dem Kopfmotiv aus der Datei des \
                 Kyrie I neu."
            ),
            Lang::En => format!(
                " {command} {setting}{unseeded} sets it again with the head motif from the file of \
                 Kyrie I."
            ),
        };
        let text = format!("{text}{}{}{}", run.pinned(lang), resumed(lang), left(lang));
        return (text, Some(run));
    }
    let movements = folder.movements(&folder.kept());
    let pins = folder.motif_endings(&movements, lang);
    let run = Run::only(folder, "kyrie/1".to_string(), movements, pins);
    let reason = lang.pick(
        " Nach einer Änderung von Programm oder Regelprofil nehmen --resume und --only das \
         Kopfmotiv nicht mehr aus der Datei des Kyrie I;",
        " After a change of program or rule profile, --resume and --only no longer take the head \
         motif from the file of Kyrie I;",
    );
    let text = format!(
        "{reason} {} {setting}{unseeded} {}.{}{}",
        run.command(),
        renewal(None, lang),
        run.pinned(lang),
        left(lang)
    );
    (text, Some(run))
}

/// The sentences of a failed `--rewrite` on the files of the movements: those of the movements of
/// which `folder` keeps no part, though they are there ([`Folder::unwritten`]), which `--rewrite`
/// leaves as they are and `fux mass` replaces or removes where it writes the files of movements
/// ([`write_files`]); where the way on names `run` and a file of a movement is there, that its
/// `-f ly` keeps them ([`Run::lean`]), and for a movement without parts whose parts it sets, that
/// the next `--rewrite` writes its files from them unless `--movements` leaves it out.
fn unwritten_files(folder: &Folder<'_>, run: Option<&Run>, lang: Lang) -> String {
    let unwritten = folder.unwritten();
    let dir = folder.dir.display();
    let mut text = String::new();
    if let Some(((first, files), others)) = unwritten.split_first() {
        text = match lang {
            Lang::De => format!(
                " In {dir} liegt kein Teil des {}, nur {}",
                first.title(),
                file_names(files)
            ),
            Lang::En => format!(
                " {dir} holds no part of the {}, only {}",
                first.title(),
                file_names(files)
            ),
        };
        for (movement, files) in others {
            text.push_str(&match lang {
                Lang::De => format!(
                    ", und keiner des {}, nur {}",
                    movement.title(),
                    file_names(files)
                ),
                Lang::En => format!(
                    ", and none of the {}, only {}",
                    movement.title(),
                    file_names(files)
                ),
            });
        }
        text.push_str(lang.pick(
            ": --rewrite lässt sie unverändert, fux mass ersetzt oder löscht sie, wo es \
             Satzdateien schreibt.",
            ": --rewrite leaves them unchanged, fux mass replaces or removes them where it writes \
             movement files.",
        ));
    }
    let Some(run) = run.filter(|run| run.lean) else {
        return text;
    };
    if unwritten.is_empty() {
        text.push_str(&match lang {
            Lang::De => format!(
                " Mit -f ly allein (ohne musicxml und midi) schreibt und löscht fux mass keine \
                 Satzdatei, und --rewrite schreibt danach die Messe: Satzdateien können Teile \
                 halten, die in {dir} fehlen."
            ),
            Lang::En => format!(
                " With -f ly alone (without musicxml and midi), fux mass writes and removes no \
                 movement file, and --rewrite then writes the mass: movement files may keep parts \
                 that {dir} lacks."
            ),
        });
    } else {
        text.push_str(lang.pick(
            " Mit -f ly allein (ohne musicxml und midi) schreibt und löscht fux mass keine \
             Satzdatei, und --rewrite schreibt danach die übrigen Sätze.",
            " With -f ly alone (without musicxml and midi), fux mass writes and removes no \
             movement file, and --rewrite then writes the other movements.",
        ));
    }
    let set = unwritten
        .iter()
        .filter(|(movement, _)| run.sets.contains(movement));
    for (movement, _) in set {
        let title = movement.title();
        text.push_str(&match lang {
            Lang::De => format!(
                " Aus den neuen Teilen des {title} schreibt das nächste --rewrite auch seine \
                 Dateien neu; ohne das {title} in --movements lässt es sie unverändert."
            ),
            Lang::En => format!(
                " From the new parts of the {title}, the next --rewrite writes its files anew; \
                 without the {title} in --movements, it leaves them unchanged."
            ),
        });
    }
    text
}

/// The movement and the number in it of the part whose file in `NAME.teile/` is named `name`
/// ([`part_file_name`]: `3-02-credo_5-7.musicxml`); `None` for a file named otherwise.
fn part_of_name(name: &str) -> Option<(Movement, usize)> {
    let stem = name.strip_suffix(".musicxml")?;
    let (movement, rest) = stem.split_once('-')?;
    let (number, _) = rest.split_once('-')?;
    let movement = movement.parse::<usize>().ok()?.checked_sub(1)?;
    let number = number.parse::<usize>().ok().filter(|&n| n >= 1)?;
    Some((*Movement::ALL.get(movement)?, number))
}

/// The parts of the mass `record` that the folder `dir` keeps, for `--rewrite` ([`KeptPart`]):
/// every file named as the file of a part ([`part_of_name`]) whose field names the mass `record`,
/// read as `--resume` reads it ([`taken_part`]): its lyrics aligned with its text and singing the
/// positions it keeps ([`restore_part`]), its plan of imitation attached. Files of another mass
/// and files named otherwise stay out. A file so named that cannot be read, or whose fields
/// cannot, fails with its name, and so do two files of one part.
fn kept_parts(dir: &Path, record: &MassField, lang: Lang) -> Result<Vec<KeptPart>> {
    let mut files: Vec<(Movement, usize, PathBuf)> = std::fs::read_dir(dir)
        .with_context(|| dir.display().to_string())?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let (movement, number) = part_of_name(path.file_name()?.to_str()?)?;
            Some((movement, number, path))
        })
        .collect();
    files.sort();
    let mut kept: Vec<KeptPart> = Vec::new();
    let mut held: Vec<(Movement, usize, PathBuf)> = Vec::new();
    for (movement, number, path) in files {
        let named = || path.display().to_string();
        let text = std::fs::read_to_string(&path).with_context(named)?;
        let (read, fields) = musicxml_text_fields(&text, None, lang).with_context(named)?;
        let field = |name: &str| {
            fields
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, value)| value.as_str())
        };
        let mass = field(FIELD_MASS).and_then(|value| value.parse::<MassField>().ok());
        if mass.as_ref() != Some(record) {
            continue;
        }
        let part: PartField = match field(FIELD_PART) {
            Some(value) => value
                .parse()
                .map_err(|error: FieldError| sentence(error.message(lang)))
                .with_context(named)?,
            None => bail!(
                "{}: {}",
                named(),
                match lang {
                    Lang::De => format!("Der Datei fehlt das Feld {FIELD_PART}."),
                    Lang::En => format!("The file lacks the field {FIELD_PART}."),
                }
            ),
        };
        let Some(spec) = read.text.as_deref() else {
            bail!(
                "{}: {}",
                named(),
                lang.pick(
                    "Die Datei nennt keinen Text (fux-text).",
                    "The file names no text (fux-text)."
                )
            );
        };
        let mut score = read.score;
        align_lyrics(&mut score, Some(spec), lang).with_context(named)?;
        restore_part(&mut score, &part, &fields);
        attach(&mut score, read.imitation, ImitationSource::File, "", lang).with_context(named)?;
        if let Some((_, _, other)) = held.iter().find(|(m, n, _)| (*m, *n) == (movement, number)) {
            let (other, address) = (other.display(), format!("{}/{number}", movement.id()));
            bail!(
                "{}",
                match lang {
                    Lang::De => format!(
                        "{other} und {} halten beide den Teil {address} der Messe; die Dateien \
                         einer Messe mit anderen Angaben (etwa --chunk) gehören in einen anderen \
                         Ordner.",
                        named()
                    ),
                    Lang::En => format!(
                        "{other} and {} both hold part {address} of the mass; the files of a mass \
                         with other options (such as --chunk) belong in another folder.",
                        named()
                    ),
                }
            );
        }
        held.push((movement, number, path.clone()));
        kept.push(KeptPart {
            movement,
            number,
            field: part,
            score,
        });
    }
    Ok(kept)
}

/// The warnings of a mass written anew from its parts (`--rewrite`), besides those of its parts:
/// the parts that another rule profile than the one numbered `hash`, which checks them now, or
/// another version of the generator set, so that the check under today's profile may give them
/// other costs than their setting did, here and in `fux check` of the files of the movements; and
/// the parts that break a prohibition of today's profile, each with how many.
fn rewrite_warnings(m: &Mass, hash: u64, lang: Lang) -> Vec<String> {
    let mut other = Vec::new();
    let mut forbidden = Vec::new();
    for ((at, _), result) in m.plan.parts().zip(&m.results) {
        let Some(result) = result else {
            continue;
        };
        let address = m.plan.address(at);
        let elsewhere = result
            .kept
            .as_ref()
            .is_some_and(|kept| kept.profile != hash || kept.generator != mass::GENERATOR);
        if elsewhere {
            other.push(address.clone());
        }
        let broken = result.generated.report.forbidden();
        if broken > 0 {
            let counted = counted(
                broken,
                ("Verbot", "Verbote"),
                ("prohibition", "prohibitions"),
                lang,
            );
            forbidden.push(format!("{address} ({counted})"));
        }
    }
    let mut lines = Vec::new();
    if !other.is_empty() {
        let parts = match (lang, other.len()) {
            (_, n) if n == m.results.len() => lang.pick("alle Teile", "every part").to_string(),
            (Lang::De, 1) => format!("den Teil {}", other[0]),
            (Lang::De, _) => format!("die Teile {}", other.join(", ")),
            (Lang::En, 1) => format!("part {}", other[0]),
            (Lang::En, _) => format!("parts {}", other.join(", ")),
        };
        lines.push(match lang {
            Lang::De => format!(
                "Ein anderes Regelprofil oder eine andere Version des Generators setzte {parts}: \
                 Die Kosten hier und in fux check der Satzdateien sind die einer Prüfung mit dem \
                 heutigen Profil und können von denen beim Setzen abweichen"
            ),
            Lang::En => format!(
                "another rule profile or version of the generator set {parts}: the costs here and \
                 in fux check of the movement files are those of a check under today's profile \
                 and may differ from those of their setting"
            ),
        });
    }
    if !forbidden.is_empty() {
        let parts = forbidden.join(", ");
        lines.push(match lang {
            Lang::De => format!(
                "Unter dem heutigen Profil brechen Teile Verbote: {parts}; fux check der \
                 Satzdateien meldet sie und endet mit 2"
            ),
            Lang::En => format!(
                "under today's profile, parts break prohibitions: {parts}; fux check of the \
                 movement files reports them and ends with 2"
            ),
        });
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use fux_core::mass::{Lengths, Override, Preset};
    use fux_core::rules::Level;
    use fux_core::text::Eleison;
    use fux_core::{Role, Species};

    /// The options as the command line reads them.
    #[derive(Parser)]
    struct Line {
        #[command(flatten)]
        options: MassOptions,
    }

    fn parsed(args: &[&str]) -> MassOptions {
        Line::parse_from(std::iter::once("mass").chain(args.iter().copied())).options
    }

    fn prepared(options: MassOptions, lang: Lang) -> Result<(MassRequest, Profile)> {
        let options = MassOptions {
            seed: Some(1),
            ..options
        };
        prepare_mass(&options, &Profile::fux(), &[], lang)
    }

    /// The message of a failed preparation or plan.
    fn refused(options: MassOptions, lang: Lang) -> String {
        match prepared(options, lang) {
            Err(error) => format!("{error:#}"),
            Ok((request, _)) => format!("{:#}", mass_plan(&request, lang).unwrap_err()),
        }
    }

    /// The defaults of the command line are the defaults of the options and of the mass: a
    /// Missa brevis in the Dorian mode for S A T B.
    #[test]
    fn defaults_are_the_mass_defaults() {
        assert_eq!(parsed(&[]), MassOptions::default());
        let (request, _) = prepared(MassOptions::default(), Lang::De).unwrap();
        assert_eq!(request, MassRequest::default());
        // Every option reaches the request.
        let options = parsed(&[
            "--preset",
            "normal",
            "--movements",
            "kyrie, agnus",
            "--parts",
            "B T A S",
            "--mode",
            "phrygisch",
            "--scoring",
            "agnus3=S A T B",
            "--imitation",
            "aus; christe=an",
            "--endings",
            "christe=A",
            "--chunk",
            "aus",
            "--lengths",
            "natürlich",
            "--repeat",
            "keine",
            "--accents",
            "aus",
            "--eleison",
            "3",
            "--beam",
            "100",
            "--noise",
            "0",
            "--title",
            "Missa prima",
        ]);
        let (request, profile) = prepared(options, Lang::De).unwrap();
        use VoiceType::*;
        assert_eq!(
            request,
            MassRequest {
                preset: Preset::Normal,
                mode: Some("phrygian".parse().unwrap()),
                choir: vec![Soprano, Alto, Tenor, Bass],
                movements: vec![Movement::Kyrie, Movement::Agnus],
                overrides: vec![
                    Override::Scoring {
                        at: "agnus3".into(),
                        voices: vec![Soprano, Alto, Tenor, Bass]
                    },
                    Override::Imitation {
                        at: String::new(),
                        toggle: fux_core::imitation::Toggle::Off
                    },
                    Override::Imitation {
                        at: "christe".into(),
                        toggle: fux_core::imitation::Toggle::On
                    },
                    Override::Ending {
                        at: "christe".into(),
                        step: Step::A
                    },
                ],
                lengths: Lengths::Natural,
                chunk: None,
                text: TextOptions {
                    eleison: Eleison::Three
                },
                repeat: mass::Repeat::None,
                accents: false,
                seed: 1,
                beam: 100,
                noise: 0.0,
                title: Some("Missa prima".into()),
                ..MassRequest::default()
            }
        );
        assert_eq!((request.chant, request.motif), (None, false));
        assert_eq!(
            profile.settings("TXT-REPEAT").level("long"),
            Level::Forbidden
        );
        // The voices alone give the usual choir.
        let (request, _) = prepared(parsed(&["--voices", "5"]), Lang::De).unwrap();
        assert_eq!(request.choir, mass::choir(5));
    }

    /// The notation of a mass (§ 1.4): bar lines through the staves, the intonation printed, the
    /// voices that rest through a section shown, unless the options say otherwise, in German or
    /// in English words; another word is refused with the words it takes.
    #[test]
    fn notation_options() {
        assert_eq!(
            MassNotation::default(),
            MassNotation {
                mensurstriche: false,
                intonation: true,
                hide_tacet: false,
            }
        );
        let changed = MassNotation {
            mensurstriche: true,
            intonation: false,
            hide_tacet: true,
        };
        for args in [
            [
                "--mensurstriche",
                "--intonation",
                "aus",
                "--tacet",
                "verbergen",
            ],
            ["--mensurstriche", "--intonation", "off", "--tacet", "hide"],
        ] {
            assert_eq!(parsed(&args).notation, changed);
        }
        let shown = parsed(&["--intonation", "text", "--tacet", "zeigen"]);
        assert_eq!(shown.notation, MassNotation::default());
        assert_eq!(
            parsed(&["--tacet", "show"]).notation,
            MassNotation::default()
        );
        assert_eq!(
            intonation("an").unwrap_err(),
            "\"an\": text or aus (off)".to_string()
        );
        assert_eq!(
            tacet("aus").unwrap_err(),
            "\"aus\": zeigen (show) or verbergen (hide)".to_string()
        );
        assert!(Line::try_parse_from(["mass", "--tacet", "nie"]).is_err());
    }

    /// The output options as the command line reads them.
    #[derive(Parser)]
    struct Out {
        #[command(flatten)]
        output: OutputArgs,
    }

    /// The book of a mass whose parts are all missing (§ 6): every section with a rest in its
    /// staves and the remarks on its missing parts, the intonation over the Gloria, the bar lines
    /// through the staves; with the other notation without the intonation, with Mensurstriche.
    /// A movement that lacks parts has no file of its own, and the file of an earlier mass under
    /// its name is removed, as is that of a movement the mass does not set; a file in a format
    /// not written, or one that `fux mass` does not write, stays. The files come back in their
    /// order: the book, the report.
    #[test]
    fn the_book_of_a_mass_without_parts() {
        let options = parsed(&["--movements", "kyrie,gloria", "--voices", "3"]);
        let (request, _) = prepared(options, Lang::De).unwrap();
        let m = Mass::new(Arc::new(mass_plan(&request, Lang::De).unwrap()));
        let dir = std::env::temp_dir().join(format!("fux-cli-buch-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let base = dir.join("messe");
        let stale = movement_path(&base, Movement::Gloria, "musicxml");
        assert_eq!(stale, dir.join("messe-2-gloria.musicxml"));
        // Of an earlier mass with a Credo, and MuseScore's file of it.
        let credo =
            ["musicxml", "mid", "mscz"].map(|kind| movement_path(&base, Movement::Credo, kind));
        for file in credo.iter().chain([&stale]) {
            std::fs::write(file, "").unwrap();
        }
        let path = base.to_str().unwrap();
        let output = Out::parse_from(["x", "-o", path, "-f", "ly,musicxml,midi"]).output;
        let report = ["Bericht".to_string()];
        let notation = MassNotation::default();
        let files = write_mass(&m, &[], &report, &output, &notation, Lang::De).unwrap();
        assert_eq!(
            files,
            [with_extension(&base, "ly"), with_extension(&base, "txt")]
        );
        assert!(!stale.exists());
        assert_eq!(credo.map(|file| file.exists()), [false, false, true]);
        // Without MIDI the MIDI file of a movement not set stays.
        let agnus = movement_path(&base, Movement::Agnus, "mid");
        std::fs::write(&agnus, "").unwrap();
        let without = Out::parse_from(["x", "-o", path, "-f", "ly,musicxml"]).output;
        write_mass(&m, &[], &report, &without, &notation, Lang::De).unwrap();
        assert!(agnus.exists());
        assert_eq!(
            std::fs::read_to_string(with_extension(&base, "txt")).unwrap(),
            "Bericht\n"
        );
        let ly = || std::fs::read_to_string(with_extension(&base, "ly")).unwrap();
        let book = ly();
        assert!(book.contains("\\italic \"Intonatio: Glória in excélsis Deo.\""));
        assert!(book.contains("\\line { \"Teil gloria/1 fehlt: nicht gesetzt"));
        assert!(!book.contains("measureBarType"));
        let other = MassNotation {
            mensurstriche: true,
            intonation: false,
            hide_tacet: true,
        };
        write_mass(&m, &[], &report, &output, &other, Lang::De).unwrap();
        let book = ly();
        assert!(!book.contains("Intonatio") && book.contains("measureBarType = \"-span|\""));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The overlay judges parts without a plan of imitation at the levels of parts with one; a
    /// single override comes after it, `--repeat` last.
    #[test]
    fn the_profile_of_a_mass() {
        let fux = Profile::fux();
        let overlay = mass_profile(&fux, &[], Repeat::Auto).unwrap();
        let changes: Vec<String> = overlay
            .to_toml(Some(&fux))
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with("name"))
            .map(str::to_string)
            .collect();
        assert_eq!(
            changes,
            [
                "[IMI-SILENCE]",
                "unplanned_thin = \"P1\"",
                "[IMI-HELD]",
                "unplanned_struck = \"V\"",
                "unplanned_passing = \"P3\"",
            ]
        );
        // The levels follow the base.
        let mut base = Profile::fux();
        base.apply_override("IMI-HELD.passing=P2").unwrap();
        let level =
            |profile: &Profile, rule: &str, param: &str| profile.settings(rule).level(param);
        assert_eq!(
            level(
                &mass_profile(&base, &[], Repeat::Auto).unwrap(),
                "IMI-HELD",
                "unplanned_passing"
            ),
            Level::P2
        );
        let rules = ["IMI-HELD.unplanned_struck=P4".to_string()];
        let profile = mass_profile(&fux, &rules, Repeat::Free).unwrap();
        assert_eq!(level(&profile, "IMI-HELD", "unplanned_struck"), Level::P4);
        assert_eq!(level(&profile, "IMI-SILENCE", "unplanned_thin"), Level::P1);
        assert_eq!(level(&profile, "TXT-REPEAT", "long"), Level::Off);
        // A rule switched off is off for the parts without a plan too; another level for the
        // whole rule keeps the overlay's.
        let off = ["IMI-HELD=aus".to_string(), "IMI-SILENCE=P2".to_string()];
        let profile = mass_profile(&fux, &off, Repeat::Auto).unwrap();
        assert!(profile.settings("IMI-HELD").is_off());
        assert_eq!(level(&profile, "IMI-SILENCE", "thin"), Level::P2);
        assert_eq!(level(&profile, "IMI-SILENCE", "unplanned_thin"), Level::P1);
        let wrong = ["IMI-HELD.nirgends=V".to_string()];
        let error = mass_profile(&fux, &wrong, Repeat::Auto).unwrap_err();
        assert!(
            format!("{error:#}").starts_with("--rule IMI-HELD.nirgends=V: "),
            "{error:#}"
        );
    }

    /// What the options cannot have is told in the user's language, before any search: the
    /// form, the movements, the size of the parts, the voices, and what the plan does not have.
    #[test]
    fn options_fail_in_both_languages() {
        let with = |args: &[&str], lang| refused(parsed(args), lang);
        for (args, de, en) in [
            (
                &["--preset", "kurz"][..],
                "Unbekannte Form der Messe „kurz“; die Formen sind brevis und normal.",
                "Unknown form of the mass \"kurz\"; the forms are brevis and normal.",
            ),
            (
                &["--movements", "kyrie,glroia"],
                "Unbekannter Satz „glroia“; die Sätze sind kyrie, gloria, credo, sanctus, \
                 benedictus, agnus.",
                "Unknown movement \"glroia\"; the movements are kyrie, gloria, credo, sanctus, \
                 benedictus, agnus.",
            ),
            (
                &["--chunk", "10"],
                "Die Größe der Teile „10“ ist weder eine Zahl von 20 bis 200 Silben noch „aus“.",
                "The size of the parts \"10\" is neither a number of 20 to 200 syllables nor \
                 \"aus\" (off).",
            ),
            (
                &["--lengths", "lang"],
                "Unbekannte Länge „lang“; die Längen sind palestrina und natürlich.",
                "Unknown lengths \"lang\"; the lengths are palestrina and natürlich (natural).",
            ),
            (
                &["--scoring", "benedictus S A T"],
                "--scoring: „benedictus S A T“ hat nicht die Form Abschnitt=Stimmen.",
                "--scoring: \"benedictus S A T\" is not of the form section=voices.",
            ),
            (
                &["--endings", "christe=E"],
                "„christe“: Ein Teil endet nicht auf E, sondern auf der Finalis, der Quinte oder \
                 der Quarte des Modus mit einer Klausel: D, A oder G.",
                "\"christe\": a part ends not on E but on the final, the fifth or the fourth of \
                 the mode with a clausula: D, A or G.",
            ),
            (
                &["--imitation", "credo/13=an"],
                "„credo/13“: Das Credo hat 12 Teile.",
                "\"credo/13\": the Credo has 12 parts.",
            ),
            (
                &["--parts", "S A T B Bar T B"],
                "Die Besetzung braucht zwei bis sechs Stimmen.",
                "The parts need two to six voices.",
            ),
        ] {
            assert_eq!(with(args, Lang::De), de, "{args:?}");
            assert_eq!(with(args, Lang::En), en, "{args:?}");
        }
        // A section that the form does not have lists those it has.
        let unknown = with(&["--scoring", "pleni=S A T"], Lang::De);
        assert!(
            unknown.starts_with(
                "Unbekannte Stelle „pleni“; gültig sind Satz/Teil wie credo/3 und kyrie, \
                 christe, kyrie2, gloria, qui-tollis, credo,"
            ),
            "{unknown}"
        );
    }

    /// `--only` names parts by their addresses, each once in the order of the mass.
    #[test]
    fn only_names_parts() {
        let (request, _) = prepared(MassOptions::default(), Lang::De).unwrap();
        let plan = mass_plan(&request, Lang::De).unwrap();
        let only = |names: &[&str], lang| {
            let names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
            only_parts(&plan, &names, lang).map(|parts| {
                parts
                    .into_iter()
                    .map(|at| plan.address(at))
                    .collect::<Vec<_>>()
            })
        };
        assert_eq!(
            only(&["incarnatus", "kyrie/2", " ", "credo/5"], Lang::De).unwrap(),
            ["kyrie/2", "credo/5"]
        );
        assert_eq!(
            only(&["Et incarnatus est", "christe"], Lang::De).unwrap(),
            ["kyrie/2", "credo/5"]
        );
        assert_eq!(only(&["benedictus"], Lang::De).unwrap(), ["benedictus/1"]);
        assert!(only(&[], Lang::De).unwrap().is_empty());
        let error = |names: &[&str], lang| format!("{:#}", only(names, lang).unwrap_err());
        assert_eq!(
            error(&["credo/13"], Lang::De),
            "--only: „credo/13“: Das Credo hat 12 Teile."
        );
        assert_eq!(
            error(&["pleni"], Lang::En).split(';').next().unwrap(),
            "--only: Unknown place \"pleni\""
        );
    }

    #[test]
    fn numbers_and_times() {
        assert_eq!(decimal(8.84, Lang::De), "8,8");
        assert_eq!(decimal(8.84, Lang::En), "8.8");
        assert_eq!(decimal(12.0, Lang::De), "12,0");
        assert_eq!(clock(220.0), "3:40");
        assert_eq!(clock(59.6), "1:00");
        assert_eq!(clock(5.0), "0:05");
        assert_eq!(clock(3725.0), "1:02:05");
        assert_eq!(clock(-3.0), "0:00");
        assert_eq!(about(0.2, Lang::De), "etwa 1 s");
        assert_eq!(about(42.4, Lang::En), "about 42 s");
        assert_eq!(about(59.6, Lang::De), "etwa 1 min");
        assert_eq!(about(330.0, Lang::De), "etwa 6 min");
    }

    fn credo_part() -> PartLine {
        PartLine {
            number: 7,
            total: 26,
            movement: "Credo".into(),
            label: "credo:5-7".into(),
            opening: Opening::Tutti,
            voices: "S A T B".into(),
            ending: "D".into(),
            seed: 4711,
        }
    }

    /// The lines of a part while the mass is set, as § 7 of the design has them.
    #[test]
    fn progress_lines() {
        let part = credo_part();
        assert_eq!(
            part.finished(31, 71, 8.8, Some(220.0), Lang::De),
            "[ 7/26] Credo · credo:5-7 (gemeinsam), S A T B, Schluss D, Seed 4711: 31 Takte, \
             Kosten 71, 8,8 s – noch etwa 3:40"
        );
        assert_eq!(
            part.finished(31, 71, 8.8, Some(220.0), Lang::En),
            "[ 7/26] Credo · credo:5-7 (together), S A T B, ending D, seed 4711: 31 measures, \
             cost 71, 8.8 s – about 3:40 left"
        );
        let last = PartLine {
            number: 26,
            label: "Agnus Dei II".into(),
            opening: Opening::Imitative,
            voices: "S A T T B".into(),
            ..credo_part()
        };
        assert_eq!(
            last.finished(40, 12, 61.25, None, Lang::De),
            "[26/26] Credo · Agnus Dei II (Imitation), S A T T B, Schluss D, Seed 4711: 40 \
             Takte, Kosten 12, 61,2 s"
        );
        let few = PartLine {
            number: 2,
            total: 3,
            ..credo_part()
        };
        assert!(few.heading(Lang::De).starts_with("[2/3] Credo · "));
        assert_eq!(
            part.failed(0, "Keine Lösung mit Imitation.", Lang::De),
            "[ 7/26] Credo · credo:5-7 (gemeinsam), S A T B, Schluss D, Seed 4711: Versuch 1 \
             gescheitert: Keine Lösung mit Imitation."
        );
        assert!(
            part.failed(2, "No solution.", Lang::En)
                .ends_with(": attempt 3 failed: No solution.")
        );
        assert_eq!(
            part.running(23, 40, Some(220.0), Lang::De),
            "Teil 7/26 · Credo · Takt 23/≈40 · noch etwa 3:40"
        );
        assert_eq!(
            part.running(23, 40, None, Lang::En),
            "Part 7/26 · Credo · measure 23/≈40"
        );
        assert_eq!(
            part.taken(31, 71, "out/messe.teile", Lang::De),
            "[ 7/26] Credo · credo:5-7 (gemeinsam), S A T B, Schluss D, Seed 4711: 31 Takte, \
             Kosten 71, aus out/messe.teile"
        );
        assert_eq!(
            part.missing("not set.", Lang::En),
            "[ 7/26] Credo · credo:5-7 (together), S A T B, ending D, seed 4711: missing: not set."
        );
    }

    /// The lines name the parts of a plan as `--plan` does: a section of one part by its title,
    /// a Break by its label, other parts by their text.
    #[test]
    fn parts_of_a_plan() {
        let (request, _) = prepared(MassOptions::default(), Lang::De).unwrap();
        let plan = mass_plan(&request, Lang::De).unwrap();
        let line = |at: &str| PartLine::of(&plan, plan.find(at).unwrap()[0], Lang::De);
        let christe = line("christe");
        assert_eq!(
            (
                christe.number,
                christe.total,
                christe.movement.as_str(),
                christe.label.as_str(),
                christe.voices.as_str()
            ),
            (2, 26, "Kyrie", "Christe", "S A T B")
        );
        assert_eq!(line("incarnatus").label, "Et incarnatus est");
        assert_eq!(line("credo/2").label, "credo:5-7");
        assert_eq!(line("sanctus").label, "Sanctus");
        let agnus = line("agnus3");
        assert_eq!((agnus.number, agnus.voices.as_str()), (26, "S A T T B"));
        assert_eq!(agnus.ending, "D");
        let at = plan.find("credo/2").unwrap()[0];
        assert_eq!(line("credo/2").seed, plan.part(at).seed);
        assert_eq!(part_label(&plan, at), "credo:5-7");
        // The details of a plan give the addresses.
        let details = plan_details(&plan, Lang::De);
        assert_eq!(details.len(), 26);
        assert!(
            details[13].starts_with("credo/5: Et incarnatus est (gemeinsam), S A T B, Schluss "),
            "{}",
            details[13]
        );
        assert!(details[13].contains("; geschätzt "), "{}", details[13]);
    }

    /// A part set in the choir of the mass after a fallback of the scoring (Agnus Dei II) names
    /// the voices it sings, not those of its section.
    #[test]
    fn lines_name_the_voices_sung() {
        let (request, _) = prepared(MassOptions::default(), Lang::De).unwrap();
        let plan = mass_plan(&request, Lang::De).unwrap();
        let at = plan.find("agnus3").unwrap()[0];
        let mode = plan.movements[at.movement].mode;
        let result = PartResult {
            at,
            seed: 1,
            attempt: 3,
            generated: fux_core::generate::Generated {
                score: Score::new(mode, Vec::new()),
                cost: 0,
                report: fux_core::rules::Report::default(),
                warnings: Vec::new(),
                subjects: Vec::new(),
            },
            seconds: 0.0,
            warnings: Vec::new(),
            voices: vec![0, 1, 2, 4],
            profile: 0,
            motif: None,
            kept: None,
        };
        assert_eq!(PartLine::of(&plan, at, Lang::De).voices, "S A T T B");
        let line = PartLine::set(&plan, &result, Lang::De);
        assert_eq!(line.voices, "S A T B");
        assert_eq!(line.label, "Agnus Dei II");
    }

    /// A mass whose movements take the modes of their chants (phase 14c) names the mode of each
    /// movement instead of one for the mass.
    #[test]
    fn summary_of_a_mass_in_several_modes() {
        let request = MassRequest {
            chant: Some(mass::ChantMass::IX),
            movements: vec![Movement::Kyrie, Movement::Gloria],
            ..MassRequest::default()
        };
        let plan = mass_plan(&request, Lang::De).unwrap();
        let summary = mass_summary(&plan, 1, Lang::De);
        let lines: Vec<&str> = summary.lines().collect();
        assert!(
            lines[0].starts_with("Missa brevis – Modus je Satz, S A T B, Seed 1: 2 Sätze, "),
            "{summary}"
        );
        assert!(
            lines[2].starts_with("Kyrie (dorisch): Kyrie I "),
            "{summary}"
        );
        assert!(
            lines[3].starts_with("Gloria (mixolydisch): gloria:1-5 "),
            "{summary}"
        );
        // One mode: no mode per movement.
        let one = MassRequest::default();
        let summary = mass_summary(&mass_plan(&one, Lang::En).unwrap(), 1, Lang::En);
        assert!(summary.starts_with("Missa brevis – Dorian, S A T B, seed 1: 6 movements, "));
        assert!(
            summary.contains("\nKyrie: Kyrie I (imitation) → D ‖ "),
            "{summary}"
        );
        assert!(!summary.ends_with('\n'));
    }

    /// The native estimate (K6): one part at a time the sum of the parts' estimates, as before;
    /// several at once the pool's, each part longer beside the others, and less than the sum,
    /// but never more than the sum, also for a few small parts. More jobs than parts count as
    /// one per part.
    #[test]
    fn the_native_estimate_is_the_pools() {
        let plan = mass_plan(&MassRequest::default(), Lang::De).unwrap();
        let sum = plan.estimate().seconds;
        assert_eq!(native_seconds(&plan, 1), f64::from(sum));
        assert_eq!(native_seconds(&plan, 0), f64::from(sum));
        let four = native_seconds(&plan, 4);
        let factor = tables::estimate_jobs(4);
        assert_eq!(four, f64::from(plan.pool_seconds(4, factor)));
        assert!(factor > 1.0 && four < f64::from(sum) && four > f64::from(sum) / 4.0);
        assert!(native_seconds(&plan, 2) > four);
        assert_eq!(tables::estimate_jobs(1), 1.0);
        let (_, kyrie) = kyrie_a3();
        let parts = kyrie.part_count();
        assert_eq!(native_seconds(&kyrie, 8), native_seconds(&kyrie, parts));
        let (_, small, _) = motif_mass();
        let alone = f64::from(small.estimate().seconds);
        assert!(f64::from(small.pool_seconds(4, factor)) > alone);
        assert_eq!(native_seconds(&small, 4), alone);
        // The first lines of the plan name it.
        let line = mass_summary(&plan, 4, Lang::De);
        let line = line.lines().nth(1).unwrap();
        assert!(
            line.contains(&format!(", {} (Browser", about(four, Lang::De))),
            "{line}"
        );
    }

    /// Several parts at once begin in the order of the pool of the web version (K12 (a)), which
    /// the native estimate and the browser's model: over a whole mass that of
    /// `MassPlan::pool_order`, over the parts of a run the same rule on them; one part at a time
    /// in the order of the mass.
    #[test]
    fn parts_begin_in_the_order_of_the_pool() {
        let plan = mass_plan(&MassRequest::default(), Lang::De).unwrap();
        let all: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
        assert_eq!(start_order(&plan, &all, 4), plan.pool_order(4));
        assert_eq!(start_order(&plan, &all, 1), all);
        let credo: Vec<PartRef> = all.iter().copied().filter(|at| at.movement == 2).collect();
        assert_eq!(start_order(&plan, &credo, 3), plan.pool_order_of(&credo, 3));
    }

    #[test]
    fn time_left() {
        // A part expected to take `seconds`, without the head motif; one running for `elapsed`.
        let part = |seconds| EtaPart {
            seconds,
            ..EtaPart::default()
        };
        let run = |seconds, elapsed| (part(seconds), elapsed);
        let mut eta = Eta::default();
        assert_eq!(eta.ratio(), 1.0);
        assert_eq!(eta.remaining(1, &[], &[part(60.0), part(40.0)]), 100.0);
        // Twice as long as estimated: the rest takes twice as long too.
        eta.record(10.0, 20.0);
        eta.record(30.0, 60.0);
        assert_eq!(eta.ratio(), 2.0);
        assert_eq!(eta.remaining(1, &[], &[part(60.0), part(40.0)]), 200.0);
        // A part running for 50 s of its 30 s estimate, scaled to 60 s.
        let waiting = [part(60.0), part(40.0)];
        assert_eq!(eta.remaining(1, &[run(30.0, 50.0)], &waiting), 210.0);
        assert_eq!(eta.remaining(1, &[run(30.0, 70.0)], &[]), 0.0);
        // Longer parts weigh more than short ones.
        let mut eta = Eta::default();
        eta.record(1.0, 3.0);
        eta.record(99.0, 99.0);
        assert!((eta.ratio() - 1.02).abs() < 1e-9);
        // Two parts at once: the parts waiting go, in their order, to the job free first; the
        // mass ends with the last of them.
        let eta = Eta::default();
        let waiting = [part(60.0), part(40.0), part(30.0)];
        assert_eq!(eta.remaining(2, &[], &waiting), 70.0);
        let waiting = [part(40.0), part(10.0)];
        assert_eq!(eta.remaining(2, &[run(50.0, 20.0)], &waiting), 40.0);
        let running = [run(50.0, 20.0), run(10.0, 0.0)];
        assert_eq!(eta.remaining(2, &running, &[part(40.0)]), 50.0);
        // More running than jobs (the jobs of a run before): each goes on to its end.
        assert_eq!(eta.remaining(1, &[run(5.0, 0.0), run(9.0, 0.0)], &[]), 9.0);
        assert_eq!(eta.remaining(4, &[], &[]), 0.0);
        // The head motif, as `schedule()` of the page has it: the parts that open with it wait
        // until Kyrie I, which gives it, ends. Four jobs, Kyrie I running for 11.25 s more and
        // four parts with the motif: 11.25 + 42 s, not 42 s as without it.
        let kyrie = |seconds| EtaPart {
            seconds,
            source: true,
            waits: false,
        };
        let opening = |seconds| EtaPart {
            seconds,
            waits: true,
            ..EtaPart::default()
        };
        let waiting = [opening(42.0), opening(36.5), opening(33.5), opening(25.25)];
        assert_eq!(eta.remaining(4, &[(kyrie(11.25), 0.0)], &waiting), 53.25);
        // Without Kyrie I in the run the motif is known: nothing waits.
        assert_eq!(eta.remaining(4, &[], &waiting), 42.0);
        // Kyrie I still to begin goes first; a part without the motif takes the job beside it,
        // the part with the motif follows Kyrie I.
        let waiting = [kyrie(10.0), opening(30.0), part(20.0)];
        assert_eq!(eta.remaining(2, &[], &waiting), 40.0);
        // Kyrie I set: the part with the motif begins at once.
        assert_eq!(eta.remaining(2, &[], &waiting[1..]), 30.0);
    }

    /// The status of several parts at once names each by its address with its measures, in the
    /// order given, and the time left; a part beyond the width of a terminal is counted instead.
    #[test]
    fn several_parts_running() {
        let part = |address: &str, measure, measures| (address.to_string(), measure, measures);
        assert_eq!(
            running_parts(
                &[part("kyrie/3", 12, 30), part("gloria/2", 4, 38)],
                Some(220.0),
                Lang::De
            ),
            "kyrie/3 T. 12/≈30 · gloria/2 T. 4/≈38 · noch etwa 3:40"
        );
        assert_eq!(
            running_parts(&[part("credo/11", 0, 35)], None, Lang::En),
            "credo/11 m. 0/≈35"
        );
        let four = [
            part("kyrie/1", 12, 30),
            part("kyrie/2", 4, 27),
            part("kyrie/3", 1, 30),
            part("gloria/1", 2, 54),
        ];
        let line = running_parts(&four[..3], Some(220.0), Lang::De);
        assert_eq!(
            line,
            "kyrie/1 T. 12/≈30 · kyrie/2 T. 4/≈27 · kyrie/3 T. 1/≈30 · noch etwa 3:40"
        );
        let line = running_parts(&four, Some(220.0), Lang::De);
        assert_eq!(
            line,
            "kyrie/1 T. 12/≈30 · kyrie/2 T. 4/≈27 · kyrie/3 T. 1/≈30 · +1 · noch etwa 3:40"
        );
        assert!(line.chars().count() <= STATUS_WIDTH, "{line}");
        let long = [
            part("credo/10", 23, 28),
            part("credo/11", 4, 35),
            part("sanctus/1", 12, 62),
            part("benedictus/1", 1, 46),
        ];
        assert_eq!(
            running_parts(&long, Some(80.0), Lang::En),
            "credo/10 m. 23/≈28 · credo/11 m. 4/≈35 · +2 · about 1:20 left"
        );
    }

    /// The parts set at once by default: half the threads of the machine, one to four.
    #[test]
    fn jobs_by_default() {
        let jobs = default_jobs();
        assert!((1..=JOBS_MAX).contains(&jobs), "{jobs}");
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
        assert_eq!(jobs, (threads / 2).clamp(1, JOBS_MAX));
    }

    #[test]
    fn cadence_plans() {
        let part = |internal: &[(usize, &str)], ending: (usize, &str)| {
            Some(PartCadences {
                internal: internal.iter().map(|&(m, t)| (m, t.to_string())).collect(),
                ending: (ending.0, ending.1.to_string()),
            })
        };
        let credo = [
            (Join::Start, part(&[(7, "A")], (12, "D"))),
            (Join::Seam, part(&[(18, "G")], (25, "A"))),
            (Join::Break("Et incarnatus est"), part(&[], (31, "A"))),
            (Join::Seam, None),
            (Join::Section, part(&[(37, "F")], (44, "D"))),
        ];
        assert_eq!(
            cadence_plan("Credo", &credo, Lang::De),
            "Credo – Kadenzplan: T. 7 A · T. 12 D (Naht) · T. 18 G · T. 25 A | T. 31 A (Naht) · \
             … ‖ T. 37 F · T. 44 D"
        );
        assert_eq!(
            cadence_plan("Kyrie", &credo[..1], Lang::En),
            "Kyrie – cadence plan: m. 7 A · m. 12 D"
        );
    }

    #[test]
    fn files_of_the_parts() {
        let base = Path::new("out/messe");
        assert_eq!(parts_dir(base), Path::new("out/messe.teile"));
        assert_eq!(
            part_file_name(Movement::Credo, 2, "credo:5-7"),
            "3-02-credo_5-7.musicxml"
        );
        assert_eq!(
            part_file_name(Movement::Sanctus, 1, "sanctus+pleni+osanna"),
            "4-01-sanctus+pleni+osanna.musicxml"
        );
        let (request, _) = prepared(parsed(&["--movements", "credo"]), Lang::De).unwrap();
        let plan = mass_plan(&request, Lang::De).unwrap();
        let at = plan.find("incarnatus").unwrap()[0];
        assert_eq!(
            part_path(base, &plan, at),
            Path::new("out/messe.teile/3-05-incarnatus.musicxml")
        );
        // Every part has a file of its own.
        let (request, _) = prepared(MassOptions::default(), Lang::De).unwrap();
        let plan = mass_plan(&request, Lang::De).unwrap();
        let mut files: Vec<PathBuf> = plan
            .parts()
            .map(|(at, _)| part_path(base, &plan, at))
            .collect();
        files.sort();
        files.dedup();
        assert_eq!(files.len(), plan.part_count());
        // The name of the file of a part names its movement and its number in it (`--rewrite`).
        assert_eq!(
            part_of_name("3-02-credo_5-7.musicxml"),
            Some((Movement::Credo, 2))
        );
        let name = part_file_name(Movement::Agnus, 12, "agnus3");
        assert_eq!(part_of_name(&name), Some((Movement::Agnus, 12)));
        for other in [
            "messe-1-kyrie.musicxml",
            "7-01-kyrie.musicxml",
            "1-00-kyrie.musicxml",
            "1-01-kyrie.tmp",
            "kyrie.musicxml",
        ] {
            assert_eq!(part_of_name(other), None, "{other}");
        }
    }

    /// `--rewrite` refuses the options of the plan and the seed, by their names on the command
    /// line, where they differ from the defaults (a value that reads as the default does not);
    /// the options of the notation, the report and the profile of the check it takes.
    #[test]
    fn rewrite_refuses_the_options_of_the_plan() {
        let allowed = [
            "--rewrite",
            "--mensurstriche",
            "--title",
            "Missa",
            "--details",
            "--movements",
            "kyrie",
            "--accents",
            "aus",
            "--repeat",
            "frei",
        ];
        assert!(planned_options(&parsed(&allowed)).is_empty());
        let defaults = [
            "--preset",
            "Brevis",
            "--chunk",
            "45",
            "--lengths",
            "Palestrina",
            "--chant",
            "keiner",
            "--voices",
            "4",
        ];
        assert!(planned_options(&parsed(&defaults)).is_empty());
        let planned = [
            "--preset",
            "normal",
            "--chunk",
            "aus",
            "--eleison",
            "3",
            "--noise",
            "1",
            "--resume",
            "--only",
            "christe",
            "--jobs",
            "2",
        ];
        assert_eq!(
            planned_options(&parsed(&planned)),
            [
                "--preset",
                "--chunk",
                "--eleison",
                "--noise",
                "--only",
                "--resume",
                "--jobs"
            ]
        );
    }

    /// The ways on that `--rewrite` names where the parts kept do not make the mass, in either
    /// language ([`way_on`]), each with its movements, the Kyrie in a mass with the head motif:
    /// `--resume` with the options of the setting for missing parts, a movement without parts and
    /// a part with another head motif, where every part kept of the movements of the run fits this
    /// program and the profile of the check; otherwise `--only` with the part missing or the
    /// sections of the movement, with Kyrie I where a part with the head motif would not find it in
    /// a file that fits; `--only` with a part set after another ending of the part before it,
    /// whatever changed; every `--only` with the endings that the files around its parts keep for
    /// the end of `--endings`; and where the folder keeps no part of a movement whose files are
    /// there, those files and `-f ly`. They name the folder, not `NAME.teile/`.
    #[test]
    fn remedies_of_a_mass_not_rewritten() {
        use Movement::{Agnus, Benedictus, Credo, Gloria, Kyrie, Sanctus};
        let dir = Path::new("out/messe.teile");
        let dorian: ModeSpec = "dorisch".parse().unwrap();
        let record = |items: &str| -> MassField {
            format!("preset={items};mode=dorian;choir=A,T,B;principal=A;seed=1")
                .parse()
                .unwrap()
        };
        let (plain, motif) = (record("brevis"), record("brevis;motif=1"));
        // The Kyrie, D A D, and the Credo without its fifth part, A G D A | D, after G.
        let parts = |changed: &[Movement], without: &[(Movement, usize)]| -> Vec<Held> {
            [
                (Kyrie, 1, Step::D, None),
                (Kyrie, 2, Step::A, Some(Step::D)),
                (Kyrie, 3, Step::D, Some(Step::A)),
                (Credo, 1, Step::A, None),
                (Credo, 2, Step::G, Some(Step::A)),
                (Credo, 3, Step::D, Some(Step::G)),
                (Credo, 4, Step::A, Some(Step::D)),
                (Credo, 6, Step::D, Some(Step::G)),
            ]
            .into_iter()
            .filter(|&(movement, number, ..)| !without.contains(&(movement, number)))
            .map(|(movement, number, ending, before)| Held {
                movement,
                number,
                ending,
                before,
                mode: dorian,
                fits: !changed.contains(&movement),
            })
            .collect()
        };
        let both = [Kyrie, Credo];
        let told = |record: &MassField,
                    held: &[Held],
                    chosen: &[Movement],
                    files: &[(Movement, Vec<PathBuf>)],
                    error: &KeptError,
                    lang: Lang| {
            let folder = Folder {
                dir,
                record,
                held,
                chosen,
                files,
            };
            not_rewritten(&folder, error, lang).to_string()
        };
        let missing = |part: &str, later| KeptError::Missing {
            part: part.into(),
            later,
        };
        let fits = parts(&[], &[]);
        let changed = parts(&[Kyrie, Credo], &[]);
        // A missing part: --resume where the parts of its movements fit, else --only with the
        // endings around it.
        assert_eq!(
            told(
                &plain,
                &fits,
                &both,
                &[],
                &missing("credo/5", false),
                Lang::De
            ),
            "--rewrite kann die Messe mit Seed 1 in out/messe.teile nicht neu schreiben. Teil \
             credo/5 fehlt. Fehlende Teile setzt fux mass --resume --movements kyrie,credo mit den \
             übrigen Angaben des Setzens nach. Mit --movements schreibt --rewrite nur Sätze, deren \
             Teile alle vorliegen, und lässt die Dateien der übrigen unverändert."
        );
        let credo_changed = parts(&[Credo], &[]);
        let alone = told(
            &plain,
            &credo_changed,
            &[Kyrie],
            &[],
            &missing("kyrie/2", false),
            Lang::De,
        );
        assert!(alone.contains(" --resume --movements kyrie "), "{alone}");
        assert_eq!(
            told(
                &plain,
                &changed,
                &both,
                &[],
                &missing("credo/5", false),
                Lang::De
            ),
            "--rewrite kann die Messe mit Seed 1 in out/messe.teile nicht neu schreiben. Teil \
             credo/5 fehlt. fux mass --only credo/5 --movements kyrie,credo mit den übrigen \
             Angaben des Setzens (ohne --seed) setzt ihn nach. Ans Ende von --endings gehören \
             dabei credo/4=A;credo/5=G: die Schlussstufen, die die Dateien halten. Nach einer \
             Änderung von Programm oder Regelprofil nähme --resume die übrigen Teile nicht mehr \
             aus ihren Dateien. Teile, die nicht passen, lässt --only, wie sie sind; dann endet es \
             unvollständig (Rückgabewert 3), und statt seines Hinweises auf --resume schreibt \
             --rewrite danach die Messe. Mit --movements schreibt --rewrite nur Sätze, deren Teile \
             alle vorliegen, und lässt die Dateien der übrigen unverändert."
        );
        assert_eq!(
            told(
                &plain,
                &changed,
                &both,
                &[],
                &missing("credo/5", false),
                Lang::En
            ),
            "--rewrite cannot write the mass with seed 1 in out/messe.teile anew. Part credo/5 is \
             missing. fux mass --only credo/5 --movements kyrie,credo with the other options of \
             the setting (without --seed) sets it. At the end of --endings, add \
             credo/4=A;credo/5=G: the endings the files keep. After a change of program or rule \
             profile, --resume would no longer take the other parts from their files. --only \
             leaves the parts that do not fit as they are; it then ends incomplete (exit code 3), \
             and instead of its hint at --resume, --rewrite then writes the mass. With \
             --movements, --rewrite writes only movements whose parts are all there and leaves the \
             files of the others unchanged."
        );
        // The parts missing at the end of a movement: --only with the first, as the files do not
        // say how many today's plan has after it, which --plan --details lists.
        for (lang, named) in [
            (
                Lang::De,
                " fux mass --only credo/7 --movements kyrie,credo mit den übrigen Angaben des \
                 Setzens (ohne --seed) setzt credo/7 nach. Ans Ende von --endings gehören dabei \
                 credo/6=D: die Schlussstufe, die die Dateien halten. Die Teile des Credo danach \
                 nennt fux mass --plan --details mit denselben Angaben; mit ihren Adressen in \
                 --only setzt es auch sie, sonst nennt das nächste --rewrite den nächsten \
                 fehlenden Teil. ",
            ),
            (
                Lang::En,
                " fux mass --only credo/7 --movements kyrie,credo with the other options of the \
                 setting (without --seed) sets credo/7. At the end of --endings, add credo/6=D: \
                 the ending the files keep. fux mass --plan --details with the same options lists \
                 the parts of the Credo after it; with their addresses in --only, it sets them as \
                 well, else the next --rewrite names the next part missing. ",
            ),
        ] {
            let text = told(
                &plain,
                &changed,
                &both,
                &[],
                &missing("credo/7", true),
                lang,
            );
            assert!(text.contains(named) && !text.contains('…'), "{text}");
        }
        // A part set after another ending of the part before it: --only, whatever changed.
        let before = KeptError::Before {
            part: "kyrie/2".into(),
        };
        for (lang, named) in [
            (
                Lang::De,
                " Den Teil setzt fux mass --only kyrie/2 --movements kyrie,credo mit den übrigen \
                 Angaben neu, mit denen der Teil davor gesetzt wurde (ohne --seed). Ans Ende von \
                 --endings gehören dabei kyrie/1=D;kyrie/2=A: die Schlussstufen, die die Dateien \
                 halten.",
            ),
            (
                Lang::En,
                " fux mass --only kyrie/2 --movements kyrie,credo with the other options the part \
                 before it was set with (without --seed) sets it again. At the end of --endings, \
                 add kyrie/1=D;kyrie/2=A: the endings the files keep.",
            ),
        ] {
            let kept = told(&plain, &fits, &both, &[], &before, lang);
            assert!(kept.ends_with(named), "{kept}");
            let other = told(&plain, &changed, &both, &[], &before, lang);
            assert!(
                other.contains(named) && other.ends_with(left(lang)),
                "{other}"
            );
        }
        // A movement without parts, in a mass without the head motif: --resume, whatever changed.
        let gloria = KeptError::NoParts { movement: Gloria };
        for held in [&fits, &changed] {
            let text = told(
                &plain,
                held,
                &[Kyrie, Gloria, Credo],
                &[],
                &gloria,
                Lang::De,
            );
            assert!(
                text.contains(
                    ". fux mass --resume --movements gloria mit den übrigen Angaben des Setzens \
                     setzt das Gloria, ohne Teile anderer Sätze zu ersetzen. "
                ),
                "{text}"
            );
        }
        // The head motif: a run names the Kyrie; Kyrie I missing sets every part with the motif.
        let no_kyrie_i = parts(&[], &[(Kyrie, 1)]);
        let text = told(
            &motif,
            &no_kyrie_i,
            &both,
            &[],
            &missing("kyrie/1", false),
            Lang::De,
        );
        assert!(
            text.contains(
                " Fehlende Teile setzt fux mass --resume --movements kyrie,credo mit den übrigen \
                 Angaben des Setzens nach. Da das Kyrie I fehlt, setzt es mit ihm jeden Teil mit \
                 dem Kopfmotiv neu. "
            ),
            "{text}"
        );
        let text = told(
            &motif,
            &no_kyrie_i,
            &both,
            &[],
            &missing("kyrie/1", false),
            Lang::En,
        );
        assert!(
            text.contains(
                " As Kyrie I is missing, it sets with it every part that takes its head motif \
                 again. "
            ),
            "{text}"
        );
        let other = parts(&[Kyrie, Credo], &[(Kyrie, 1)]);
        for (lang, named) in [
            (
                Lang::De,
                " fux mass --only kyrie/1 --movements kyrie,credo mit den übrigen Angaben des \
                 Setzens (ohne --seed) setzt das Kyrie I und mit ihm jeden Teil mit dem Kopfmotiv \
                 neu und ersetzt deren Dateien. Ans Ende von --endings gehören dabei \
                 kyrie/1=D;credo/1=A: ",
            ),
            (
                Lang::En,
                " fux mass --only kyrie/1 --movements kyrie,credo with the other options of the \
                 setting (without --seed) sets Kyrie I again and with it every part that takes its \
                 head motif, replacing their files. At the end of --endings, add \
                 kyrie/1=D;credo/1=A: ",
            ),
        ] {
            let text = told(&motif, &other, &both, &[], &missing("kyrie/1", false), lang);
            assert!(text.contains(named), "{text}");
        }
        // A part with the head motif missing: with Kyrie I where its file does not fit.
        let first = missing("credo/1", false);
        let no_credo_i = parts(&[Kyrie, Credo], &[(Credo, 1)]);
        for (lang, named) in [
            (
                Lang::De,
                " fux mass --only kyrie/1,credo/1 --movements kyrie,credo mit den übrigen Angaben \
                 des Setzens (ohne --seed) setzt credo/1 und das Kyrie I und mit ihm jeden Teil \
                 mit dem Kopfmotiv neu und ersetzt deren Dateien. Ans Ende von --endings gehören \
                 dabei kyrie/1=D;credo/1=A: ",
            ),
            (
                Lang::En,
                " fux mass --only kyrie/1,credo/1 --movements kyrie,credo with the other options \
                 of the setting (without --seed) sets credo/1, and Kyrie I again and with it every \
                 part that takes its head motif, replacing their files. At the end of --endings, \
                 add kyrie/1=D;credo/1=A: ",
            ),
        ] {
            let text = told(&motif, &no_credo_i, &both, &[], &first, lang);
            assert!(
                text.contains(named) && text.contains(fitting(lang)),
                "{text}"
            );
        }
        let credo_alone = parts(&[Credo], &[(Credo, 1)]);
        let text = told(&motif, &credo_alone, &both, &[], &first, Lang::De);
        assert!(
            text.contains(" fux mass --only credo/1 --movements kyrie,credo mit ")
                && !text.contains("kyrie/1"),
            "{text}"
        );
        // A movement without parts with the head motif: with the Kyrie, from its files where they
        // fit; else with its sections, and with Kyrie I where it may open with the motif.
        let text = told(&motif, &fits, &[Gloria], &[], &gloria, Lang::De);
        assert!(
            text.contains(
                " fux mass --resume --movements kyrie,gloria mit den übrigen Angaben des Setzens \
                 setzt das Gloria und nimmt die Teile des Kyrie aus ihren Dateien, ohne Teile \
                 anderer Sätze zu ersetzen. "
            ),
            "{text}"
        );
        for (lang, named) in [
            (
                Lang::De,
                " fux mass --only kyrie/1,gloria --movements kyrie,gloria,credo mit den übrigen \
                 Angaben des Setzens (ohne --seed) setzt das Gloria und das Kyrie I und mit ihm \
                 jeden Teil mit dem Kopfmotiv neu und ersetzt deren Dateien. Ans Ende von \
                 --endings gehören dabei kyrie/1=D;credo/1=A: ",
            ),
            (
                Lang::En,
                " fux mass --only kyrie/1,gloria --movements kyrie,gloria,credo with the other \
                 options of the setting (without --seed) sets the Gloria, and Kyrie I again and \
                 with it every part that takes its head motif, replacing their files. At the end \
                 of --endings, add kyrie/1=D;credo/1=A: ",
            ),
        ] {
            let text = told(&motif, &changed, &[Gloria], &[], &gloria, lang);
            assert!(text.contains(named) && text.contains(left(lang)), "{text}");
        }
        let benedictus = KeptError::NoParts {
            movement: Benedictus,
        };
        let text = told(&motif, &changed, &[Benedictus], &[], &benedictus, Lang::De);
        assert!(
            text.contains(
                " fux mass --only benedictus,osanna2 --movements kyrie,benedictus mit den übrigen \
                 Angaben des Setzens (ohne --seed) setzt das Benedictus, ohne Teile anderer Sätze \
                 zu ersetzen. "
            ),
            "{text}"
        );
        // Another head motif: --resume, --only with the part where Kyrie I fits, else Kyrie I.
        let took = KeptError::Motif {
            part: "credo/1".into(),
        };
        let text = told(&motif, &fits, &both, &[], &took, Lang::De);
        assert!(
            text.ends_with(
                " Einen solchen Teil setzt fux mass --resume --movements kyrie,credo mit den \
                 übrigen Angaben des Setzens neu."
            ),
            "{text}"
        );
        let text = told(&motif, &credo_changed, &both, &[], &took, Lang::En);
        assert!(
            text.contains(
                " fux mass --only credo/1 --movements kyrie,credo with the other options of the \
                 setting (without --seed) sets it again with the head motif from the file of \
                 Kyrie I. At the end of --endings, add credo/1=A: "
            ),
            "{text}"
        );
        let text = told(&motif, &changed, &both, &[], &took, Lang::De);
        assert!(
            text.contains(
                " Nach einer Änderung von Programm oder Regelprofil nehmen --resume und --only das \
                 Kopfmotiv nicht mehr aus der Datei des Kyrie I; fux mass --only kyrie/1 \
                 --movements kyrie,credo mit den übrigen Angaben des Setzens (ohne --seed) setzt \
                 das Kyrie I und mit ihm jeden Teil mit dem Kopfmotiv neu und ersetzt deren \
                 Dateien. Ans Ende von --endings gehören dabei kyrie/1=D;credo/1=A: "
            ),
            "{text}"
        );
        // The Osanna ut supra: the Sanctus in --movements where its parts are there, else set.
        let normal = record("normal");
        assert_eq!(
            told(
                &normal,
                &fits,
                &[Benedictus],
                &[],
                &KeptError::UtSupra,
                Lang::De
            ),
            "--rewrite kann die Messe mit Seed 1 in out/messe.teile nicht neu schreiben. Das \
             Osanna ut supra des Benedictus wiederholt das Osanna des Sanctus, und das fehlt. fux \
             mass --resume --movements sanctus mit den übrigen Angaben des Setzens setzt das \
             Sanctus, ohne Teile anderer Sätze zu ersetzen."
        );
        assert_eq!(
            told(
                &normal,
                &changed,
                &[Benedictus],
                &[],
                &KeptError::UtSupra,
                Lang::En
            ),
            "--rewrite cannot write the mass with seed 1 in out/messe.teile anew. The Osanna ut \
             supra of the Benedictus repeats the Osanna of the Sanctus, which is missing. fux mass \
             --resume --movements sanctus with the other options of the setting sets the Sanctus \
             without replacing parts of other movements."
        );
        let mut sanctus = fits.clone();
        sanctus.push(Held {
            movement: Sanctus,
            number: 1,
            ending: Step::D,
            before: None,
            mode: dorian,
            fits: true,
        });
        let text = told(
            &normal,
            &sanctus,
            &[Benedictus],
            &[],
            &KeptError::UtSupra,
            Lang::De,
        );
        assert!(
            text.ends_with(". Mit dem Sanctus in --movements schreibt --rewrite beide."),
            "{text}"
        );
        // The files of a movement without parts: named, and kept by -f ly.
        let agnus: Vec<PathBuf> = ["musicxml", "mid"]
            .into_iter()
            .map(|kind| PathBuf::from(format!("out/messe-6-agnus.{kind}")))
            .collect();
        let unwritten = [(Agnus, agnus)];
        assert_eq!(
            told(
                &plain,
                &fits,
                &both,
                &unwritten,
                &missing("credo/5", false),
                Lang::De
            ),
            "--rewrite kann die Messe mit Seed 1 in out/messe.teile nicht neu schreiben. Teil \
             credo/5 fehlt. Fehlende Teile setzt fux mass --resume --movements kyrie,credo -f ly \
             mit den übrigen Angaben des Setzens nach. In out/messe.teile liegt kein Teil des \
             Agnus Dei, nur out/messe-6-agnus.musicxml, out/messe-6-agnus.mid: --rewrite lässt sie \
             unverändert, fux mass ersetzt oder löscht sie, wo es Satzdateien schreibt. Mit -f ly \
             allein (ohne musicxml und midi) schreibt und löscht fux mass keine Satzdatei, und \
             --rewrite schreibt danach die übrigen Sätze. Mit --movements schreibt --rewrite nur \
             Sätze, deren Teile alle vorliegen, und lässt die Dateien der übrigen unverändert."
        );
        assert_eq!(
            told(
                &plain,
                &changed,
                &both,
                &unwritten,
                &missing("credo/5", false),
                Lang::En
            ),
            "--rewrite cannot write the mass with seed 1 in out/messe.teile anew. Part credo/5 is \
             missing. fux mass --only credo/5 --movements kyrie,credo -f ly with the other options \
             of the setting (without --seed) sets it. At the end of --endings, add \
             credo/4=A;credo/5=G: the endings the files keep. After a change of program or rule \
             profile, --resume would no longer take the other parts from their files. --only \
             leaves the parts that do not fit as they are; it then ends incomplete (exit code 3), \
             and instead of its hint at --resume, --rewrite then writes the mass. \
             out/messe.teile holds no part of the Agnus Dei, only out/messe-6-agnus.musicxml, \
             out/messe-6-agnus.mid: --rewrite leaves them unchanged, fux mass replaces or removes \
             them where it writes movement files. With -f ly alone (without musicxml and midi), \
             fux mass writes and removes no movement file, and --rewrite then writes the other \
             movements. With --movements, --rewrite writes only movements whose parts are all \
             there and leaves the files of the others unchanged."
        );
        let agnus_none = KeptError::NoParts { movement: Agnus };
        for (lang, named) in [
            (
                Lang::De,
                " Aus den neuen Teilen des Agnus Dei schreibt das nächste --rewrite auch seine \
                 Dateien neu; ohne das Agnus Dei in --movements lässt es sie unverändert. ",
            ),
            (
                Lang::En,
                " From the new parts of the Agnus Dei, the next --rewrite writes its files anew; \
                 without the Agnus Dei in --movements, it leaves them unchanged. ",
            ),
        ] {
            let text = told(
                &plain,
                &fits,
                &[Kyrie, Agnus],
                &unwritten,
                &agnus_none,
                lang,
            );
            assert!(
                text.contains(" fux mass --resume --movements agnus -f ly ")
                    && text.contains(named),
                "{text}"
            );
        }
        // The Benedictus of the form normal: every run names the Sanctus as well, whose Osanna
        // its Osanna ut supra repeats, as fux mass refuses it otherwise.
        let held = |kept: &[(Movement, usize)], fits: bool| -> Vec<Held> {
            kept.iter()
                .map(|&(movement, number)| Held {
                    movement,
                    number,
                    ending: Step::D,
                    before: (number > 1).then_some(Step::D),
                    mode: dorian,
                    fits,
                })
                .collect()
        };
        let sanctus_parts = [(Sanctus, 1), (Sanctus, 2), (Sanctus, 3)];
        let no_benedictus = KeptError::NoParts {
            movement: Benedictus,
        };
        let chosen = [Sanctus, Benedictus];
        assert_eq!(
            told(
                &normal,
                &held(&sanctus_parts, true),
                &chosen,
                &[],
                &no_benedictus,
                Lang::De
            ),
            "--rewrite kann die Messe mit Seed 1 in out/messe.teile nicht neu schreiben. Vom \
             Benedictus liegt kein Teil vor. fux mass --resume --movements sanctus,benedictus mit \
             den übrigen Angaben des Setzens setzt das Benedictus und nimmt die Teile des Sanctus \
             aus ihren Dateien, ohne Teile anderer Sätze zu ersetzen. Mit --movements schreibt \
             --rewrite nur Sätze, deren Teile alle vorliegen, und lässt die Dateien der übrigen \
             unverändert."
        );
        assert_eq!(
            told(
                &normal,
                &held(&sanctus_parts, false),
                &chosen,
                &[],
                &no_benedictus,
                Lang::En
            ),
            "--rewrite cannot write the mass with seed 1 in out/messe.teile anew. No part of the \
             Benedictus is there. fux mass --only benedictus --movements sanctus,benedictus with \
             the other options of the setting (without --seed) sets the Benedictus without \
             replacing parts of other movements. After a change of program or rule profile, \
             --resume would no longer take the other parts from their files. --only leaves the \
             parts that do not fit as they are; it then ends incomplete (exit code 3), and instead \
             of its hint at --resume, --rewrite then writes the mass. With --movements, --rewrite \
             writes only movements whose parts are all there and leaves the files of the others \
             unchanged."
        );
        let text = told(&normal, &fits, &[Benedictus], &[], &no_benedictus, Lang::De);
        assert!(
            text.contains(
                " fux mass --resume --movements sanctus,benedictus mit den übrigen Angaben des \
                 Setzens setzt das Benedictus und das Sanctus, ohne Teile anderer Sätze zu \
                 ersetzen. "
            ),
            "{text}"
        );
        let benedictus_2 = [(Benedictus, 2)];
        let first = missing("benedictus/1", false);
        let text = told(
            &normal,
            &held(&benedictus_2, true),
            &[Benedictus],
            &[],
            &first,
            Lang::De,
        );
        assert!(
            text.contains(
                " Fehlende Teile setzt fux mass --resume --movements sanctus,benedictus mit den \
                 übrigen Angaben des Setzens nach. "
            ),
            "{text}"
        );
        let text = told(
            &normal,
            &held(&benedictus_2, false),
            &[Benedictus],
            &[],
            &first,
            Lang::En,
        );
        assert!(
            text.contains(
                " fux mass --only benedictus/1 --movements sanctus,benedictus with the other \
                 options of the setting (without --seed) sets it. At the end of --endings, add \
                 benedictus/1=D: "
            ),
            "{text}"
        );
        // With the head motif: the Kyrie too, and with Kyrie I set anew, the parts of the Sanctus
        // that take its motif.
        let motif_normal = record("normal;motif=1");
        let mut renewed = held(&[(Kyrie, 2), (Kyrie, 3)], true);
        renewed.extend(held(&[(Benedictus, 1)], false));
        let text = told(
            &motif_normal,
            &renewed,
            &[Kyrie, Benedictus],
            &[],
            &missing("kyrie/1", false),
            Lang::De,
        );
        assert!(
            text.contains(
                " fux mass --only kyrie/1 --movements kyrie,sanctus,benedictus mit den übrigen \
                 Angaben des Setzens (ohne --seed) setzt das Kyrie I und mit ihm jeden Teil mit \
                 dem Kopfmotiv neu und ersetzt deren Dateien. "
            ),
            "{text}"
        );
        for (lang, named) in [
            (
                Lang::De,
                " fux mass --resume --movements kyrie,sanctus,benedictus mit den übrigen Angaben \
                 des Setzens setzt das Benedictus und das Kyrie und nimmt die Teile des Sanctus \
                 aus ihren Dateien; mit dem fehlenden Kyrie I setzt es jeden Teil mit dem \
                 Kopfmotiv neu. ",
            ),
            (
                Lang::En,
                " fux mass --resume --movements kyrie,sanctus,benedictus with the other options of \
                 the setting sets the Benedictus and the Kyrie and takes the parts of the Sanctus \
                 from their files; with the missing Kyrie I, it sets every part that takes its \
                 head motif again. ",
            ),
        ] {
            let text = told(
                &motif_normal,
                &held(&sanctus_parts, true),
                &[Benedictus],
                &[],
                &no_benedictus,
                lang,
            );
            assert!(text.contains(named), "{text}");
        }
        // The Kyrie of a mass with the head motif without parts: a run sets Kyrie I anew, whose
        // motif every part kept that takes it must take. --resume sets the Kyrie where every part
        // kept fits; otherwise --only sets it, and with its Kyrie I every part that takes the
        // motif, in every movement kept, at once.
        let no_kyrie = KeptError::NoParts { movement: Kyrie };
        let kyrie_parts = [(Kyrie, 1), (Kyrie, 2), (Kyrie, 3)];
        let text = told(
            &motif,
            &parts(&[], &kyrie_parts),
            &[Kyrie],
            &[],
            &no_kyrie,
            Lang::De,
        );
        assert!(
            text.contains(
                ". fux mass --resume --movements kyrie mit den übrigen Angaben des Setzens setzt \
                 das Kyrie, ohne Teile anderer Sätze zu ersetzen. "
            ),
            "{text}"
        );
        let credo_other = parts(&[Credo], &kyrie_parts);
        for (lang, named) in [
            (
                Lang::De,
                " Vom Kyrie liegt kein Teil vor. fux mass --only kyrie,christe,kyrie2 --movements \
                 kyrie,credo mit den übrigen Angaben des Setzens (ohne --seed) setzt das Kyrie und \
                 mit dessen Kyrie I jeden Teil mit dem Kopfmotiv neu und ersetzt deren Dateien. \
                 Ans Ende von --endings gehören dabei credo/1=A: die Schlussstufe, die die Dateien \
                 halten. Nach einer Änderung von Programm oder Regelprofil nähme --resume die \
                 übrigen Teile nicht mehr aus ihren Dateien. Ein neues Kyrie I kann ein anderes \
                 Kopfmotiv geben, als die Teile in den Dateien nahmen. Teile, die nicht passen, ",
            ),
            (
                Lang::En,
                " No part of the Kyrie is there. fux mass --only kyrie,christe,kyrie2 --movements \
                 kyrie,credo with the other options of the setting (without --seed) sets the \
                 Kyrie, and with its Kyrie I every part that takes the head motif again, replacing \
                 their files. At the end of --endings, add credo/1=A: the ending the files keep. \
                 After a change of program or rule profile, --resume would no longer take the \
                 other parts from their files. A new Kyrie I may give another head motif than the \
                 parts in the files took. --only leaves ",
            ),
        ] {
            let text = told(&motif, &credo_other, &[Kyrie, Credo], &[], &no_kyrie, lang);
            assert!(text.contains(named), "{text}");
        }
        let text = told(&motif, &credo_other, &[Gloria], &[], &gloria, Lang::De);
        assert!(
            text.contains(
                " fux mass --only kyrie,christe,kyrie2,gloria --movements kyrie,gloria,credo mit \
                 den übrigen Angaben des Setzens (ohne --seed) setzt das Gloria und das Kyrie und \
                 mit dessen Kyrie I jeden Teil mit dem Kopfmotiv neu und ersetzt deren Dateien. \
                 Ans Ende von --endings gehören dabei credo/1=A: "
            ),
            "{text}"
        );
        let text = told(
            &motif,
            &parts(&[], &kyrie_parts),
            &[Gloria],
            &[],
            &gloria,
            Lang::De,
        );
        assert!(
            text.contains(
                " fux mass --resume --movements kyrie,gloria mit den übrigen Angaben des Setzens \
                 setzt das Gloria und das Kyrie, ohne Teile anderer Sätze zu ersetzen. "
            ),
            "{text}"
        );
        // Kyrie I missing, the Credo not chosen but set by another version: the new Kyrie I gives
        // the motif anew to the Credo too.
        let text = told(
            &motif,
            &parts(&[Credo], &[(Kyrie, 1)]),
            &[Kyrie],
            &[],
            &missing("kyrie/1", false),
            Lang::De,
        );
        assert!(
            text.contains(
                " fux mass --only kyrie/1 --movements kyrie,credo mit den übrigen Angaben des \
                 Setzens (ohne --seed) setzt das Kyrie I und mit ihm jeden Teil mit dem Kopfmotiv \
                 neu "
            ),
            "{text}"
        );
        // Kyrie I set anew by a run for a part of another movement, as the folder keeps no file
        // of it: --resume only where the parts kept of every movement fit, and where the folder
        // keeps no part of the Kyrie, the sections of the Kyrie stand for Kyrie I.
        let without_kyrie_i = parts(&[], &[(Kyrie, 1)]);
        let text = told(&motif, &without_kyrie_i, &[Gloria], &[], &gloria, Lang::De);
        assert!(
            text.contains(
                " fux mass --resume --movements kyrie,gloria mit den übrigen Angaben des Setzens \
                 setzt das Gloria und nimmt die Teile des Kyrie aus ihren Dateien; mit dem \
                 fehlenden Kyrie I setzt es jeden Teil mit dem Kopfmotiv neu. "
            ),
            "{text}"
        );
        let credo_5 = missing("credo/5", false);
        let no_kyrie_parts = parts(&[], &[(Kyrie, 1), (Kyrie, 2), (Kyrie, 3), (Credo, 5)]);
        let text = told(&motif, &no_kyrie_parts, &[Credo], &[], &credo_5, Lang::En);
        assert!(
            text.contains(
                " fux mass --resume --movements kyrie,credo with the other options of the setting \
                 sets missing parts. As Kyrie I is missing, it sets with it every part that takes \
                 its head motif again. "
            ),
            "{text}"
        );
        let mut agnus_other = no_kyrie_parts.clone();
        agnus_other.extend(held(&[(Agnus, 1), (Agnus, 2)], false));
        let text = told(&motif, &agnus_other, &[Credo], &[], &credo_5, Lang::De);
        assert!(
            text.contains(
                " Teil credo/5 fehlt. fux mass --only credo/5 --movements kyrie,credo mit den \
                 übrigen Angaben des Setzens (ohne --seed) setzt ihn nach. "
            ),
            "{text}"
        );
        let credo_1 = missing("credo/1", false);
        let no_credo_i = parts(&[Credo], &[(Kyrie, 1), (Kyrie, 2), (Kyrie, 3), (Credo, 1)]);
        for (lang, named) in [
            (
                Lang::De,
                " fux mass --only kyrie,christe,kyrie2,credo/1 --movements kyrie,credo mit den \
                 übrigen Angaben des Setzens (ohne --seed) setzt credo/1 und das Kyrie und mit \
                 dessen Kyrie I jeden Teil mit dem Kopfmotiv neu und ersetzt deren Dateien. Ans \
                 Ende von --endings gehören dabei credo/1=A: ",
            ),
            (
                Lang::En,
                " fux mass --only kyrie,christe,kyrie2,credo/1 --movements kyrie,credo with the \
                 other options of the setting (without --seed) sets credo/1 and the Kyrie, and \
                 with its Kyrie I every part that takes the head motif again, replacing their \
                 files. At the end of --endings, add credo/1=A: ",
            ),
        ] {
            let text = told(&motif, &no_credo_i, &[Credo], &[], &credo_1, lang);
            assert!(
                text.contains(named) && text.contains(fitting(lang)),
                "{text}"
            );
        }
        // A part missing whose movement's files are there: they may keep its last copy, which
        // --rewrite writes anew once the part is set again; and every run of fux mass keeps the
        // files of the movements with -f ly.
        let credo: Vec<PathBuf> = ["musicxml", "mid"]
            .into_iter()
            .map(|kind| PathBuf::from(format!("out/messe-3-credo.{kind}")))
            .collect();
        let credo_files = [(Credo, credo)];
        assert_eq!(
            told(
                &plain,
                &fits,
                &both,
                &credo_files,
                &missing("credo/5", false),
                Lang::De
            ),
            "--rewrite kann die Messe mit Seed 1 in out/messe.teile nicht neu schreiben. Teil \
             credo/5 fehlt. Wie er gesetzt wurde, kann der Teil nur noch in \
             out/messe-3-credo.musicxml, out/messe-3-credo.mid stehen: Ist er neu gesetzt, \
             schreibt --rewrite diese Dateien neu; eine Kopie an anderem Ort behält ihn. Fehlende \
             Teile setzt fux mass --resume --movements kyrie,credo -f ly mit den übrigen Angaben \
             des Setzens nach. Mit -f ly allein (ohne musicxml und midi) schreibt und löscht fux \
             mass keine Satzdatei, und --rewrite schreibt danach die Messe: Satzdateien können \
             Teile halten, die in out/messe.teile fehlen. Mit --movements schreibt --rewrite nur \
             Sätze, deren Teile alle vorliegen, und lässt die Dateien der übrigen unverändert."
        );
        assert_eq!(
            told(
                &plain,
                &changed,
                &both,
                &credo_files,
                &missing("credo/5", false),
                Lang::En
            ),
            "--rewrite cannot write the mass with seed 1 in out/messe.teile anew. Part credo/5 is \
             missing. As it was set, the part may now stand only in out/messe-3-credo.musicxml, \
             out/messe-3-credo.mid: once it is set again, --rewrite writes these files anew; a \
             copy elsewhere keeps it. fux mass --only credo/5 --movements kyrie,credo -f ly with \
             the other options of the setting (without --seed) sets it. At the end of --endings, \
             add credo/4=A;credo/5=G: the endings the files keep. After a change of program or \
             rule profile, --resume would no longer take the other parts from their files. --only \
             leaves the parts that do not fit as they are; it then ends incomplete (exit code 3), \
             and instead of its hint at --resume, --rewrite then writes the mass. With -f ly alone \
             (without musicxml and midi), fux mass writes and removes no movement file, and \
             --rewrite then writes the mass: movement files may keep parts that out/messe.teile \
             lacks. With --movements, --rewrite writes only movements whose parts are all there \
             and leaves the files of the others unchanged."
        );
        for (lang, named) in [
            (
                Lang::De,
                " Die Teile ab credo/7 fehlen. Wie sie gesetzt wurden, können diese Teile nur noch \
                 in out/messe-3-credo.musicxml, out/messe-3-credo.mid stehen: Sind sie neu \
                 gesetzt, schreibt --rewrite diese Dateien neu; eine Kopie an anderem Ort behält \
                 sie. fux mass --only credo/7 --movements kyrie,credo -f ly mit ",
            ),
            (
                Lang::En,
                " The parts from credo/7 on are missing. As they were set, these parts may now \
                 stand only in out/messe-3-credo.musicxml, out/messe-3-credo.mid: once they are \
                 set again, --rewrite writes these files anew; a copy elsewhere keeps them. fux \
                 mass --only credo/7 --movements kyrie,credo -f ly with ",
            ),
        ] {
            let text = told(
                &plain,
                &changed,
                &both,
                &credo_files,
                &missing("credo/7", true),
                lang,
            );
            assert!(text.contains(named), "{text}");
        }
        // The files of another movement whose parts the folder keeps: -f ly all the same.
        let kyrie_files = [(Kyrie, vec![PathBuf::from("out/messe-1-kyrie.musicxml")])];
        let text = told(
            &plain,
            &changed,
            &both,
            &kyrie_files,
            &missing("credo/5", false),
            Lang::De,
        );
        assert!(
            text.contains(
                " Teil credo/5 fehlt. fux mass --only credo/5 --movements kyrie,credo -f ly mit "
            ) && text.contains(" Satzdateien können Teile halten, die in out/messe.teile fehlen. ")
                && !text.contains("messe-1-kyrie"),
            "{text}"
        );
        let text = told(&plain, &fits, &both, &kyrie_files, &before, Lang::En);
        assert!(
            text.contains(" fux mass --only kyrie/2 --movements kyrie,credo -f ly with the other ")
                && text.ends_with(" movement files may keep parts that out/messe.teile lacks."),
            "{text}"
        );
        // No way on but the files.
        let form = KeptError::Form {
            part: "credo/2".into(),
        };
        let text = told(&plain, &fits, &both, &unwritten, &form, Lang::De);
        assert!(
            text.ends_with(", wo es Satzdateien schreibt.")
                && !text.contains("fux mass --")
                && !text.contains("-f ly"),
            "{text}"
        );
        assert!(
            told(&plain, &fits, &both, &[], &form, Lang::En)
                .ends_with(" or follows its last section.")
        );
        let twice = KeptError::Twice {
            part: "credo/2".into(),
        };
        assert!(
            told(&plain, &fits, &both, &[], &twice, Lang::De)
                .ends_with(" gehören in einen anderen Ordner.")
        );
    }

    #[test]
    fn exit_codes() {
        assert_eq!(exit_code(26, 26), EXIT_COMPLETE);
        assert_eq!(exit_code(25, 26), EXIT_INCOMPLETE);
        assert_eq!(exit_code(0, 26), EXIT_INCOMPLETE);
        assert_ne!(EXIT_INCOMPLETE, 1);
        assert_ne!(EXIT_INCOMPLETE, 2);
    }

    /// The Kyrie of a mass for A T B, seed 1, with its plan.
    fn kyrie_a3() -> (MassRequest, MassPlan) {
        let options = parsed(&["--movements", "kyrie", "--voices", "3"]);
        let (request, _) = prepared(options, Lang::De).unwrap();
        let plan = mass_plan(&request, Lang::De).unwrap();
        (request, plan)
    }

    /// What the file of the first part of a movement keeps of it, beginning in imitation: its
    /// search seeded with `seed` at `attempt`, under the profile numbered `profile`, by this
    /// version of the generator.
    fn first_field(
        plan: &MassPlan,
        at: PartRef,
        seed: u64,
        attempt: u8,
        profile: u64,
    ) -> PartField {
        let section = &plan.movements[at.movement].sections[at.section];
        let voices: Vec<String> = section.voices.iter().map(usize::to_string).collect();
        format!(
            "bars=1-20;voices={};join=start;title={};end={};start=0;open=imit;cont=0;seed={seed};\
             attempt={attempt};points=;profile={profile:016x};generator={}",
            voices.join(","),
            section.title,
            plan.part(at).ending.letter(),
            mass::GENERATOR
        )
        .parse()
        .unwrap()
    }

    /// A file keeps its part for a plan that gives the part the same place, ending and start,
    /// under the same profile, the seed of its search being the plan's for its attempt (as the
    /// mass layer seeds the attempts) or one of its own (`--only … --seed`), set by this version
    /// of the generator; another ending, profile or version does not fit, and the version is what
    /// differs. A file without the version was set by version 1.
    #[test]
    fn files_keep_their_parts() {
        let (_, plan) = kyrie_a3();
        let at = plan.find("kyrie/1").unwrap()[0];
        let seed = plan.part(at).seed;
        let planned = first_field(&plan, at, attempt_seed(seed, 1), 1, 7);
        assert!(planned.fits(&plan, at, 7) && fits(&planned, &plan, at, 7));
        assert!(differences(&planned, &plan, at, 7).is_empty());
        let own = first_field(&plan, at, attempt_seed(5, 0), 0, 7);
        assert!(!own.fits(&plan, at, 7) && fits(&own, &plan, at, 7));
        assert!(!fits(&own, &plan, at, 8));
        let other = PartField {
            generator: mass::GENERATOR + 1,
            ..own.clone()
        };
        assert!(!fits(&other, &plan, at, 7));
        assert_eq!(differences(&other, &plan, at, 7), [Difference::Generator]);
        let older = PartField {
            generator: mass::GENERATOR + 1,
            profile: 8,
            ..own.clone()
        };
        assert_eq!(
            differences(&older, &plan, at, 7),
            [Difference::Profile, Difference::Generator]
        );
        let item = format!(";generator={}", mass::GENERATOR);
        let unversioned: PartField = own.to_string().replace(&item, "").parse().unwrap();
        assert_eq!(unversioned.generator, 1);
        assert_eq!(fits(&unversioned, &plan, at, 7), mass::GENERATOR == 1);
        let elsewhere = PartField {
            ending: Step::E,
            ..own
        };
        assert!(!fits(&elsewhere, &plan, at, 7));
        assert_eq!(
            attempt_seed(u64::MAX, 2),
            Rng::new(1).next_u64(),
            "the seeds wrap"
        );
    }

    /// `--resume` and `--only` go on with the mass the files record: its form, mode, choir and
    /// seed, whatever the options say, so that the plan is the recorded mass's. The Dorian of a
    /// mass without a mode plans as the Dorian named.
    #[test]
    fn a_recorded_mass_goes_on() {
        let options = parsed(&["--preset", "normal", "--voices", "3", "--mode", "phrygisch"]);
        let (request, _) = prepared(options, Lang::De).unwrap();
        let recorded = MassRequest { seed: 7, ..request };
        let plan = mass_plan(&recorded, Lang::De).unwrap();
        let record = MassField::of(&plan);
        let (other, _) = prepared(MassOptions::default(), Lang::De).unwrap();
        let going_on = recorded_request(other, &record);
        assert_eq!(going_on, recorded);
        assert_eq!(mass_plan(&going_on, Lang::De).unwrap(), plan);
        let plain = mass_plan(&MassRequest::default(), Lang::De).unwrap();
        let dorian = MassField::of(&plain);
        assert_eq!(dorian.mode, "dorian");
        let again = recorded_request(MassRequest::default(), &dorian);
        assert_eq!(again.mode, Some("dorian".parse().unwrap()));
        assert_eq!(mass_plan(&again, Lang::De).unwrap(), plain);
    }

    /// `--only … --seed` gives the parts it names a seed of their own, for their search and for
    /// their chain of internal cadences; the other parts keep theirs.
    #[test]
    fn a_part_takes_a_seed_of_its_own() {
        let (_, mut plan) = kyrie_a3();
        let before = plan.clone();
        let at = plan.find("christe").unwrap()[0];
        reseed(&mut plan, at, 5);
        assert_eq!(plan.part(at).seed, 5);
        assert!(matches!(
            plan.part(at).chain,
            CadenceChain::Weighted { seed: 5, .. }
        ));
        for (place, part) in plan.parts() {
            if place != at {
                assert_eq!(part, before.part(place));
            }
        }
    }

    /// `fux render` writes a single score as before; a score in sections ends with the fermatas
    /// of a movement, and the file of a movement, which its first part's text names, under the
    /// movement's title.
    #[test]
    fn the_rendered_sheet() {
        let field = |text: &str| vec![(format!("{FIELD_TEXT}.1"), text.to_string())];
        assert_eq!(movement_of_file(&field("kyrie")), Some(Movement::Kyrie));
        assert_eq!(movement_of_file(&field("credo:5-7")), Some(Movement::Credo));
        let osanna = field("sanctus+pleni+osanna");
        assert_eq!(movement_of_file(&osanna), Some(Movement::Sanctus));
        assert_eq!(movement_of_file(&field("Ave Maria")), None);
        let single = vec![(FIELD_TEXT.to_string(), "kyrie".to_string())];
        assert_eq!(movement_of_file(&single), None);
        let note = Note::new(Pitch::new(Step::D, 0, 4), fux_core::Duration::WHOLE);
        let voice = Voice::new(
            VoiceType::Alto,
            Role::Counterpoint(Species::Free),
            vec![note; 4],
        );
        let mut score = Score::new("dorian".parse().unwrap(), vec![voice]);
        let sheet = rendered_sheet(&score, &field("kyrie"), Lang::De);
        assert_eq!(
            (sheet.piece, sheet.end),
            (None, fux_export::SectionEnd::Plain)
        );
        score.sections = [0, 2]
            .map(|measure| SectionMark {
                measure,
                kind: SectionKind::Section,
                title: Some("A".into()),
            })
            .to_vec();
        let sheet = rendered_sheet(&score, &field("kyrie"), Lang::De);
        assert_eq!(sheet.piece.as_deref(), Some("Kyrie – dorisch"));
        assert_eq!(sheet.end, fux_export::SectionEnd::Movement);
        let sheet = rendered_sheet(&score, &[], Lang::En);
        assert_eq!(
            (sheet.piece, sheet.end),
            (None, fux_export::SectionEnd::Movement)
        );
    }

    /// `fux check` of the file of a part judges it as it was set: under the overlay of the mass,
    /// `--rule` after it and `--repeat` last, its text going on where the part's went on. Other
    /// files are checked as before; a field that cannot be read is an error in either language.
    #[test]
    fn the_check_of_a_part() {
        let fux = Profile::fux();
        let rules = ["IMI-HELD.unplanned_passing=P4".to_string()];
        let level = |profile: &Profile, param| profile.settings("IMI-HELD").level(param);
        let (part, profile) = check_profile(&[], &fux, &rules, Repeat::Free, Lang::De).unwrap();
        assert!(part.is_none());
        assert_eq!(level(&profile, "unplanned_struck"), Level::Off);
        assert_eq!(level(&profile, "unplanned_passing"), Level::P4);
        assert_eq!(profile.settings("TXT-REPEAT").level("long"), Level::Off);
        let field = "bars=1-20;voices=0,1,2;join=seam;title=;end=A;start=4;open=imit;cont=1;\
                     seed=9;attempt=0;points=;profile=0000000000000001";
        let fields = [
            ("fux-mode".to_string(), "dorian".to_string()),
            (FIELD_PART.to_string(), field.to_string()),
        ];
        let (part, profile) = check_profile(&fields, &fux, &rules, Repeat::Auto, Lang::De).unwrap();
        let part = part.unwrap();
        assert_eq!((part.ending, part.continued), (Step::A, true));
        let mass = mass_profile(&fux, &rules, Repeat::Auto).unwrap();
        assert_eq!(profile.to_toml(None), mass.to_toml(None));
        assert_ne!(level(&profile, "unplanned_struck"), Level::Off);
        let broken = [(FIELD_PART.to_string(), "bars=x".to_string())];
        let error = |lang| format!("{:#}", part_of_file(&broken, lang).unwrap_err());
        assert_eq!(
            error(Lang::De),
            "Feld fux-part: „bars=x“ lässt sich nicht lesen."
        );
        assert_eq!(
            error(Lang::En),
            "Field fux-part: \"bars=x\" cannot be read."
        );
    }

    /// A part comes back from its file as it was set where it sings the file's own text: the
    /// text goes on as its field says, and a word the text repeats (San-ctus, San-ctus, San-ctus)
    /// sings the position it sang, which the printed syllables alone do not tell. A field that
    /// does not fit the score leaves the alignment as it is.
    #[test]
    fn parts_come_back_as_set() {
        let note = |syllable: &&str| Note {
            syllable: Some(syllable.to_string()),
            ..Note::new(Pitch::new(Step::D, 0, 3), fux_core::Duration::WHOLE)
        };
        let words = [
            "San-", "ctus,", "San-", "ctus,", "San-", "ctus", "San-", "ctus",
        ];
        let notes = words.iter().map(note).collect();
        let voice = Voice::new(VoiceType::Tenor, Role::Counterpoint(Species::Free), notes);
        let read = || {
            let mut score = Score::new("dorian".parse().unwrap(), vec![voice.clone()]);
            align_lyrics(&mut score, Some("sanctus"), Lang::De).unwrap();
            score
        };
        // The alignment takes the restated word for another unit than the one sung.
        let set = "0,1,2,3,4,5,4,5";
        let aligned = lyrics_field(&read());
        assert_ne!(aligned, set);
        let part: PartField = "bars=1-8;voices=0;join=start;title=Sanctus;end=D;start=0;\
                               open=imit;cont=1;seed=9;attempt=0;points=;profile=1"
            .parse()
            .unwrap();
        let fields = |text: &str| {
            [
                (FIELD_TEXT.to_string(), text.to_string()),
                (FIELD_LYRICS.to_string(), set.to_string()),
            ]
        };
        let mut score = read();
        restore_part(&mut score, &part, &fields("sanctus"));
        assert_eq!(lyrics_field(&score), set);
        assert!(score.text.as_ref().unwrap().text.continued);
        // Another text: the score stays as aligned.
        let mut other = read();
        restore_part(&mut other, &part, &fields("kyrie"));
        assert_eq!(lyrics_field(&other), aligned);
        assert!(!other.text.as_ref().unwrap().text.continued);
        for misfit in ["0,1,2", "0,1,2,3,4,5,4,99", "0,1,2,3,4,5,4,5|0", "0,1,x"] {
            let mut score = read();
            assert!(!restore_lyrics(&mut score, misfit), "{misfit}");
            assert_eq!(lyrics_field(&score), aligned, "{misfit}");
        }
        let mut unknown = read();
        assert!(restore_lyrics(&mut unknown, "0,1,2,3,-,5,4,5"));
        assert_eq!(lyrics_field(&unknown), "0,1,2,3,-,5,4,5");
    }

    /// The report of a mass that lacks parts: the totals count what is set, the cadence plans
    /// show a part not set as „…“, the end names the parts missing, and so does a remark under
    /// each section that lacks one, with why and how `--resume` sets it, in lines the page has
    /// room for.
    #[test]
    fn a_mass_with_parts_missing() {
        let (_, plan) = kyrie_a3();
        let m = Mass::new(Arc::new(plan));
        assert_eq!(
            mass_totals(&m, 0, "out/messe.teile", 0, Lang::De),
            "Gesamt: 0 von 3 Teilen, 0 Takte, Kosten 0, Suchzeit 0:00, keine Warnung"
        );
        assert_eq!(
            mass_totals(&m, 2, "x.teile", 1, Lang::En),
            "Total: 0 of 3 parts (2 from x.teile), 0 measures, cost 0, search time 0:00, 1 \
             warning"
        );
        assert_eq!(
            cadence_plans_of(&m, Lang::De),
            ["Kyrie – Kadenzplan: … ‖ … ‖ …"]
        );
        assert!(mass_warnings(&m, Lang::De).is_empty());
        assert_eq!(
            incomplete(&m, Lang::De).unwrap(),
            format!(
                "Unvollständig, es fehlen: kyrie/1, kyrie/2, kyrie/3. {}",
                resume_hint(Lang::De)
            )
        );
        let christe = m.plan.find("christe").unwrap()[0];
        let failed = failure_message(&christe_failed(), Lang::De);
        let remarks = missing_remarks(&m, &[(christe, failed.clone())], 0, 1, Lang::De);
        assert_eq!(
            remarks,
            [
                "Teil kyrie/2 („Chri-ste e-lé-i-son.“) ließ sich in 3 Versuchen nicht setzen. Keine",
                "Lösung: Die Regeln verbieten jede Fortsetzung in Takt 1 (vor allem CAD-START).",
                "Abhilfe: ein anderer Seed (--only kyrie/2 --seed N), --imitation kyrie/2=aus oder",
                "--endings kyrie/2=D.",
                "fux mass --resume mit denselben Angaben setzt fehlende Teile nach, einen",
                "gescheiterten Teil mit einer seiner Abhilfen; einem einzelnen Teil gibt --only TEIL",
                "--seed N einen anderen Seed.",
            ]
        );
        assert_eq!(remarks[..4].join(" "), failed);
        assert_eq!(remarks[4..].join(" "), resume_hint(Lang::De));
        let remarks = missing_remarks(&m, &[], 0, 0, Lang::En);
        assert_eq!(
            remarks[0],
            "Part kyrie/1 is missing: not set, and its file is missing or does not fit the mass"
        );
        assert_eq!(remarks[1..].join(" "), resume_hint(Lang::En));
        assert!(
            remarks
                .iter()
                .all(|line| line.chars().count() <= REMARK_WIDTH),
            "{remarks:?}"
        );
    }

    /// The Christe of a Kyrie that no attempt could set, as the mass layer tells it.
    fn christe_failed() -> GenerateError {
        GenerateError::MassPart {
            part: "kyrie/2".into(),
            words: "Chri-ste e-lé-i-son.".into(),
            attempts: 3,
            error: Box::new(GenerateError::NoCounterpoint {
                measure: 1,
                rules: vec!["CAD-START"],
            }),
            remedies: vec![
                "--imitation kyrie/2=aus".into(),
                "--endings kyrie/2=D".into(),
            ],
        }
    }

    /// A part that failed gets another seed through `--only`, which keeps the other parts: a
    /// seed given to `--resume` would plan another mass. Other errors are told as they are.
    #[test]
    fn failures_name_the_seed_of_their_part() {
        assert_eq!(
            failure_message(&christe_failed(), Lang::De),
            "Teil kyrie/2 („Chri-ste e-lé-i-son.“) ließ sich in 3 Versuchen nicht setzen. Keine \
             Lösung: Die Regeln verbieten jede Fortsetzung in Takt 1 (vor allem CAD-START). \
             Abhilfe: ein anderer Seed (--only kyrie/2 --seed N), --imitation kyrie/2=aus oder \
             --endings kyrie/2=D."
        );
        assert_eq!(
            failure_message(&christe_failed(), Lang::En),
            "Part kyrie/2 (\"Chri-ste e-lé-i-son.\") could not be set in 3 attempts. No \
             solution: the rules forbid every continuation in measure 1 (mostly CAD-START). \
             Remedies: another seed (--only kyrie/2 --seed N), --imitation kyrie/2=aus or \
             --endings kyrie/2=D."
        );
        let mut alone = christe_failed();
        if let GenerateError::MassPart { remedies, .. } = &mut alone {
            remedies.clear();
        }
        assert!(
            failure_message(&alone, Lang::De)
                .ends_with(" Abhilfe: ein anderer Seed (--only kyrie/2 --seed N).")
        );
        let other = GenerateError::NoSolution { position: 3 };
        for lang in [Lang::De, Lang::En] {
            assert_eq!(failure_message(&other, lang), other.message(lang));
        }
    }

    /// The lines of a remark break between words, within the width; a word wider than the
    /// width has a line of its own.
    #[test]
    fn remarks_break_between_words() {
        assert_eq!(wrapped("Teil kyrie/1 fehlt.", 85), ["Teil kyrie/1 fehlt."]);
        assert_eq!(wrapped("ein  zwei drei vier", 9), ["ein zwei", "drei vier"]);
        assert_eq!(
            wrapped("Lösung: ä ö ü", 10),
            ["Lösung: ä", "ö ü"],
            "characters, not bytes"
        );
        assert_eq!(
            wrapped("a out/messe.teile/3-02-credo_5-7.musicxml b", 10),
            ["a", "out/messe.teile/3-02-credo_5-7.musicxml", "b"]
        );
        assert!(wrapped(" ", 10).is_empty());
    }

    /// `--resume` with a seed whose options plan another mass than the one in the folder is
    /// refused in the user's language, naming the seed recorded and how to go on, and the head
    /// motif or the chant where the two masses differ in them besides their seeds.
    #[test]
    fn another_mass_is_refused() {
        let (_, plan) = kyrie_a3();
        let record = MassField::of(&plan);
        let dir = Path::new("out/messe.teile");
        let planned = MassField {
            seed: 2,
            ..record.clone()
        };
        assert_eq!(
            another_mass(dir, &record, &planned, Lang::De).to_string(),
            "In out/messe.teile liegen Teile der Messe mit Seed 1, die --resume mit diesen \
             Angaben (Seed 2) durch Teile einer anderen Messe ersetzen würde. Ohne --seed setzt \
             --resume die Messe dort fort; einem einzelnen Teil gibt --only TEIL --seed N einen \
             anderen Seed; eine neue Messe setzt fux mass ohne --resume oder mit einem anderen -o."
        );
        assert_eq!(
            another_mass(dir, &record, &planned, Lang::En).to_string(),
            "out/messe.teile holds parts of the mass with seed 1, which --resume with these \
             options (seed 2) would replace with parts of another mass. Without --seed, --resume \
             goes on with the mass there; --only PART --seed N gives a single part another seed; \
             fux mass without --resume, or with another -o, sets a new mass."
        );
        // The same seed, without the head motif of the mass recorded, or on a chant.
        let motif = MassField {
            motif: true,
            ..record.clone()
        };
        assert_eq!(
            another_mass(dir, &motif, &record, Lang::De).to_string(),
            "In out/messe.teile liegen Teile der Messe mit Seed 1 und Kopfmotiv, die --resume mit \
             diesen Angaben (Seed 1, ohne Kopfmotiv) durch Teile einer anderen Messe ersetzen \
             würde. Ohne --seed setzt --resume die Messe dort fort; einem einzelnen Teil gibt \
             --only TEIL --seed N einen anderen Seed; eine neue Messe setzt fux mass ohne \
             --resume oder mit einem anderen -o."
        );
        let message = another_mass(dir, &record, &motif, Lang::En).to_string();
        assert!(
            message.starts_with(
                "out/messe.teile holds parts of the mass with seed 1 without the head motif, \
                 which --resume with these options (seed 1, with the head motif) would replace"
            ),
            "{message}"
        );
        let chant = MassField {
            chant: Some(mass::ChantMass::IV),
            ..record.clone()
        };
        let message = another_mass(dir, &record, &chant, Lang::De).to_string();
        assert!(
            message.starts_with(
                "In out/messe.teile liegen Teile der Messe mit Seed 1 ohne Choral, die --resume \
                 mit diesen Angaben (Seed 1, auf Choral IV) durch"
            ),
            "{message}"
        );
    }

    /// A file keeps its part for the mass it names (`fux-mass`), its seed included: the plan of
    /// another seed, whose part fits the file but for the seed of its mass, takes nothing from
    /// it, so that `--resume` never puts a part of one mass into another. The plan of the same
    /// mass with other options, or another profile, takes nothing from it either, and says what
    /// differs.
    #[test]
    fn files_keep_the_parts_of_their_mass() {
        let args = [
            "--movements",
            "kyrie",
            "--voices",
            "3",
            "--beam",
            "40",
            "--imitation",
            "kyrie2=aus",
        ];
        let (request, profile) = prepared(parsed(&args), Lang::De).unwrap();
        let plan = mass_plan(&request, Lang::De).unwrap();
        let at = plan.find("kyrie2").unwrap()[0];
        let result = generate_part(&request, &plan, at, &profile, None, None).unwrap();
        let dir = std::env::temp_dir().join(format!("fux-cli-teile-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        let base = dir.join("messe");
        let options = MusicXmlOptions::default();
        let path = write_part(&plan, &result, &base, &options, Lang::De).unwrap();
        let searched = part_profile(&request, &profile);
        let PartFile::Fits(taken) = taken_part(&path, &plan, at, &searched, None, Lang::De) else {
            panic!("the part is not taken from its file");
        };
        assert_eq!(taken.generated.cost, result.generated.cost);
        let other = mass_plan(
            &MassRequest {
                seed: 2,
                ..request.clone()
            },
            Lang::De,
        )
        .unwrap();
        let read = from_musicxml(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let part: PartField = read.field(FIELD_PART).unwrap().parse().unwrap();
        assert!(fits(&part, &other, at, profile_hash(&searched)));
        let taken = taken_part(&path, &other, at, &searched, None, Lang::De);
        assert!(matches!(taken, PartFile::Missing));
        // Kyrie II in imitation, as it is planned without `--imitation kyrie2=aus`.
        let (imitative, _) = prepared(parsed(&args[..6]), Lang::De).unwrap();
        let imitative = mass_plan(&imitative, Lang::De).unwrap();
        let taken = taken_part(&path, &imitative, at, &searched, None, Lang::De);
        assert!(
            matches!(&taken, PartFile::Differs(found) if found.contains(&Difference::Opening)),
            "{:?}",
            match taken {
                PartFile::Differs(found) => found,
                _ => Vec::new(),
            }
        );
        let (_, without) = prepared(
            parsed(&[&args[..], &["--repeat", "keine"]].concat()),
            Lang::De,
        )
        .unwrap();
        let without = part_profile(&request, &without);
        let taken = taken_part(&path, &plan, at, &without, None, Lang::De);
        assert!(matches!(&taken, PartFile::Differs(found) if found == &[Difference::Profile]));
        // Eleison in three syllables (`--eleison 3`).
        let three = [&args[..], &["--eleison", "3"]].concat();
        let (three, _) = prepared(parsed(&three), Lang::De).unwrap();
        let three = mass_plan(&three, Lang::De).unwrap();
        let taken = taken_part(&path, &three, at, &searched, None, Lang::De);
        assert!(matches!(&taken, PartFile::Differs(found) if found == &[Difference::Syllables]));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// `--resume` replaces no part whose file keeps it in a movement that the files keep whole,
    /// where the options plan it otherwise; it sets such a part again in a movement that lacks a
    /// part, or one that `--only` sets again, as after the remedy for a part that failed.
    #[test]
    fn resume_replaces_no_part_of_a_whole_movement() {
        let (_, plan) = kyrie_a3();
        let places: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
        let differs = || PartFile::Differs(vec![Difference::Ending]);
        let files = [differs(), differs(), differs()];
        let stale = stale_parts(&plan, &files, &[]);
        assert_eq!(stale.iter().map(|(at, _)| *at).collect::<Vec<_>>(), places);
        assert!(stale_parts(&plan, &files, &places[..1]).is_empty());
        let files = [differs(), PartFile::Missing, differs()];
        assert!(stale_parts(&plan, &files, &[]).is_empty());
        let message = |lang| {
            let found = [Difference::Opening, Difference::Ending];
            let stale = [(places[2], &found[..])];
            replaced_parts(Path::new("out/messe.teile"), &plan, &stale, lang).to_string()
        };
        assert_eq!(
            message(Lang::De),
            "In out/messe.teile liegen Teile dieser Messe, deren Dateien nicht zu diesen Angaben \
             passen: kyrie/3 (Imitation, Schlussstufe). --resume würde sie neu setzen und ihre \
             Dateien ersetzen. Mit den Angaben, mit denen die Messe gesetzt wurde (--scoring, \
             --imitation, --endings, --chunk, --lengths, --eleison, --profile, --rule, --repeat), \
             nimmt --resume sie aus ihren Dateien; --only kyrie/3 setzt sie mit diesen Angaben \
             neu; --rewrite schreibt die Messe aus den Teilen neu, wie sie sind, ohne zu setzen; \
             eine neue Messe setzt fux mass ohne --resume oder mit einem anderen -o."
        );
        assert_eq!(
            message(Lang::En),
            "out/messe.teile holds parts of this mass whose files do not fit these options: \
             kyrie/3 (imitation, ending). --resume would set them again and replace their files. \
             With the options the mass was set with (--scoring, --imitation, --endings, --chunk, \
             --lengths, --eleison, --profile, --rule, --repeat), --resume takes them from their \
             files; --only kyrie/3 sets them again with these options; --rewrite writes the mass \
             anew from the parts as they are, without setting any; fux mass without --resume, or \
             with another -o, sets a new mass."
        );
        // A part of another version of the generator besides it: no option makes that one fit.
        let other = [Difference::Generator];
        let found = [Difference::Opening, Difference::Ending];
        let stale = [(places[1], &other[..]), (places[2], &found[..])];
        let both = replaced_parts(Path::new("out/messe.teile"), &plan, &stale, Lang::De);
        assert_eq!(
            both.to_string(),
            "In out/messe.teile liegen Teile dieser Messe, deren Dateien nicht zu diesen Angaben \
             passen: kyrie/2 (Generatorversion), kyrie/3 (Imitation, Schlussstufe). --resume \
             würde sie neu setzen und ihre Dateien ersetzen. Mit den Angaben, mit denen die Messe \
             gesetzt wurde (--scoring, --imitation, --endings, --chunk, --lengths, --eleison, \
             --profile, --rule, --repeat), nimmt --resume sie aus ihren Dateien, außer denen \
             einer anderen Generatorversion, die mit keinen Angaben passen: Sie stammen von einem \
             Programm, das aus demselben Plan andere Noten setzen kann; --only kyrie/2,kyrie/3 \
             setzt sie mit diesen Angaben neu; --rewrite schreibt die Messe aus den Teilen neu, \
             wie sie sind, ohne zu setzen; eine neue Messe setzt fux mass ohne --resume oder mit \
             einem anderen -o."
        );
        let both = replaced_parts(Path::new("out/messe.teile"), &plan, &stale, Lang::En);
        assert!(
            both.to_string().contains(
                "--resume takes them from their files, except those of another generator \
                 version, which fit with no options: they come from a program that may set other \
                 notes from the same plan; --only kyrie/2,kyrie/3 sets them"
            ),
            "{both}"
        );
    }

    /// A note tied from a note of musica ficta is ficta too where a part comes back from its
    /// file, as the search set it, though MusicXML marks the first note of a tie only.
    #[test]
    fn tied_ficta_comes_back() {
        let note = |step, alter, tie| Note {
            tie,
            ..Note::new(Pitch::new(step, alter, 3), fux_core::Duration::HALF).with_ficta(alter != 0)
        };
        let notes = vec![
            note(Step::B, -1, true),
            note(Step::B, -1, true),
            note(Step::B, -1, false),
            note(Step::A, 0, true),
            note(Step::A, 0, false),
            note(Step::B, -1, false),
        ];
        let voice = Voice::new(VoiceType::Tenor, Role::Counterpoint(Species::Free), notes);
        let score = Score::new("dorian".parse().unwrap(), vec![voice]);
        let xml = fux_export::to_musicxml(&score, &MusicXmlOptions::default());
        let (read, _) = musicxml_text_fields(&xml, None, Lang::De).unwrap();
        let ficta = |score: &Score| -> Vec<bool> {
            score.voices[0]
                .notes
                .iter()
                .map(|note| note.ficta)
                .collect()
        };
        let mut restored = read.score;
        assert_eq!(ficta(&restored), [true, false, false, false, false, true]);
        restore_tied_ficta(&mut restored);
        assert_eq!(ficta(&restored), ficta(&score));
    }

    /// Each part being set has a control of its own, so that the status line counts the measures
    /// of its search alone, from none; a part not being set has none. A cancel stops the search
    /// of every part being set and of every part that would begin after it.
    #[test]
    fn each_part_has_a_control_of_its_own() {
        let (_, plan) = kyrie_a3();
        let parts: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
        let control = MassControl::new();
        assert_eq!(control.reached(parts[0]), None);
        let first = control.begin(parts[0]);
        let second = control.begin(parts[1]);
        assert_eq!(control.reached(parts[0]), Some(0));
        assert!(!Arc::ptr_eq(&first, &second));
        control.end(parts[0]);
        assert_eq!(control.reached(parts[0]), None);
        assert_eq!(control.reached(parts[1]), Some(0));
        control.cancel();
        assert!(control.is_cancelled() && second.is_cancelled() && !first.is_cancelled());
        assert!(control.begin(parts[2]).is_cancelled());
    }

    /// A panic of the search ends the status line too, so that the panic is passed on and the
    /// program ends, instead of waiting for the thread of the line for ever.
    #[test]
    fn a_panic_ends_the_status_line() {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let status = Status {
                terminal: true,
                shown: Mutex::default(),
                tick: Condvar::new(),
            };
            let control = MassControl::new();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                status.during(&control, Lang::De, || panic!("the search fails"))
            }));
            let _ = sender.send(outcome.is_err());
        });
        let passed_on = receiver.recv_timeout(std::time::Duration::from_secs(20));
        assert_eq!(passed_on, Ok(true));
    }

    /// A cancel before a part begins sets nothing, and the run tells of it once; `on` breaking
    /// off at the start of a part, after the seed of its first attempt, ends it alike; with one
    /// part at a time as with several.
    #[test]
    fn a_cancelled_run_sets_nothing() {
        let (request, plan) = kyrie_a3();
        let profile = mass::profile(&Profile::fux());
        let parts: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
        let first = attempt_seed(plan.part(parts[0]).seed, 0);
        let plan = Arc::new(plan);
        for jobs in [1, 3] {
            let control = MassControl::new();
            control.cancel();
            let mut told = Vec::new();
            let m = run_mass(
                &request,
                Mass::new(Arc::clone(&plan)),
                &parts,
                &profile,
                Some(&control),
                jobs,
                &mut |event| {
                    told.push(format!("{event:?}"));
                    ControlFlow::Continue(())
                },
            );
            assert!(m.results.iter().all(Option::is_none) && !m.complete);
            assert_eq!(told, ["Cancelled"]);
            let mut told = Vec::new();
            let m = run_mass(
                &request,
                Mass::new(Arc::clone(&plan)),
                &parts,
                &profile,
                None,
                jobs,
                &mut |event| {
                    told.push(match event {
                        MassEvent::Started { at, attempt, seed } => {
                            format!("{} {attempt} {}", at.part, seed == first)
                        }
                        other => format!("{other:?}"),
                    });
                    ControlFlow::Break(())
                },
            );
            assert!(m.results.iter().all(Option::is_none));
            assert_eq!(told, ["0 0 true", "Cancelled"], "{jobs} jobs");
        }
    }

    /// A small mass with the head motif: a Kyrie and an Agnus Dei in two voices, every part in
    /// imitation, with a narrow search (each part a second or less); its Kyrie I gives a motif,
    /// which Agnus Dei I takes.
    fn motif_mass() -> (MassRequest, MassPlan, Profile) {
        let options = MassOptions {
            seed: Some(6),
            ..parsed(&[
                "--movements",
                "kyrie,agnus",
                "--voices",
                "2",
                "--beam",
                "40",
                "--lengths",
                "natürlich",
                "--imitation",
                "an",
                "--motif",
                "an",
            ])
        };
        let (request, profile) = prepare_mass(&options, &Profile::fux(), &[], Lang::De).unwrap();
        let plan = mass_plan(&request, Lang::De).unwrap();
        (request, plan, profile)
    }

    /// What a part set came to, to compare runs: its seed, attempt, cost, motif and notes.
    fn set_as(result: &PartResult) -> String {
        let generated = &result.generated;
        format!(
            "{} {} {} {:?} {}",
            result.seed,
            result.attempt,
            generated.cost,
            result.motif,
            fux_export::to_musicxml(&generated.score, &MusicXmlOptions::default())
        )
    }

    /// Parts set at once are the parts set one after the other (D13): every part keeps its seed,
    /// and Agnus Dei I, which opens with the head motif, begins only once Kyrie I is set and takes
    /// its motif. One job tells the parts in the order of the mass, each begun once the one
    /// before it is set; several tell each part's start before its end, each start the first part
    /// still waiting in the order of the pool of the web version that may begin, and never more
    /// parts running than jobs.
    #[test]
    fn parts_at_once_are_the_parts_one_after_the_other() {
        let (request, plan, profile) = motif_mass();
        let parts: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
        let (kyrie, agnus) = (
            plan.motif_source().unwrap(),
            plan.find("agnus/1").unwrap()[0],
        );
        assert!(plan.part(agnus).motif);
        let plan = Arc::new(plan);
        let mut runs = Vec::new();
        for jobs in [1, 4] {
            let mut told: Vec<(char, PartRef)> = Vec::new();
            let m = run_mass(
                &request,
                Mass::new(Arc::clone(&plan)),
                &parts,
                &profile,
                None,
                jobs,
                &mut |event| {
                    told.push(match event {
                        MassEvent::Started { at, .. } => ('S', at),
                        MassEvent::Finished(result) => ('F', result.at),
                        other => panic!("{other:?}"),
                    });
                    ControlFlow::Continue(())
                },
            );
            assert!(m.complete, "{jobs} jobs");
            let position = |what: char, at: PartRef| {
                told.iter().position(|&event| event == (what, at)).unwrap()
            };
            assert!(
                position('F', kyrie) < position('S', agnus),
                "{jobs} jobs: {told:?}"
            );
            assert_eq!(
                m.result(agnus).unwrap().motif,
                m.result(kyrie).unwrap().motif
            );
            let mut running = 0;
            for &(what, at) in &told {
                running += if what == 'S' { 1 } else { -1 };
                assert!(
                    running <= jobs.min(parts.len()) as i32,
                    "{jobs} jobs: {told:?}"
                );
                assert!(position('S', at) < position('F', at));
            }
            // Each start is the first part of start_order still waiting that may begin, one with
            // the head motif only once Kyrie I is set: the run follows the order the estimate
            // models, however the searches end.
            let mut waiting = start_order(&plan, &parts, jobs.min(parts.len()));
            let mut motif = false;
            for &(what, at) in &told {
                if what == 'S' {
                    let next = waiting
                        .iter()
                        .position(|&part| motif || !plan.part(part).motif)
                        .unwrap();
                    assert_eq!(waiting.remove(next), at, "{jobs} jobs: {told:?}");
                } else {
                    motif |= at == kyrie;
                }
            }
            assert!(waiting.is_empty(), "{jobs} jobs: {told:?}");
            if jobs == 1 {
                let in_order: Vec<(char, PartRef)> = parts
                    .iter()
                    .flat_map(|&at| [('S', at), ('F', at)])
                    .collect();
                assert_eq!(told, in_order);
            } else {
                let begun: Vec<PartRef> = told
                    .iter()
                    .filter(|(what, _)| *what == 'S')
                    .map(|&(_, at)| at)
                    .collect();
                assert_eq!(begun[0], kyrie, "Kyrie I gives the motif: it goes first");
                let order = start_order(&plan, &parts, jobs);
                assert_eq!(order, plan.pool_order(jobs), "the pool's order");
                assert_eq!(order[0], kyrie);
                assert_ne!(order, parts, "the pool's order, not the mass's");
            }
            runs.push(
                m.results
                    .iter()
                    .map(|r| set_as(r.as_ref().unwrap()))
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(runs[1], runs[0]);
    }

    /// A cancel stops every part being set, and no other begins: cancelled as the second part
    /// begins, the third never does, and the run ends with the parts set before the cancel
    /// reached them. `on` breaking off at the first part set ends the run alike, and is told
    /// nothing after it but the end.
    #[test]
    fn a_cancel_stops_every_part() {
        let (request, plan, profile) = motif_mass();
        let parts: Vec<PartRef> = plan.parts().map(|(at, _)| at).collect();
        let plan = Arc::new(plan);
        let control = MassControl::new();
        let mut told: Vec<String> = Vec::new();
        let m = run_mass(
            &request,
            Mass::new(Arc::clone(&plan)),
            &parts,
            &profile,
            Some(&control),
            3,
            &mut |event| {
                if let MassEvent::Started { .. } = event
                    && told.len() == 1
                {
                    control.cancel();
                }
                told.push(match event {
                    MassEvent::Started { at, .. } => format!("S {}", plan.address(at)),
                    MassEvent::Finished(result) => format!("F {}", plan.address(result.at)),
                    other => format!("{other:?}"),
                });
                ControlFlow::Continue(())
            },
        );
        let begun = told.iter().filter(|line| line.starts_with("S ")).count();
        assert_eq!(begun, 2, "{told:?}");
        assert_eq!(told.last().unwrap(), "Cancelled", "{told:?}");
        let set = told.iter().filter(|line| line.starts_with("F ")).count();
        assert_eq!(m.results.iter().flatten().count(), set, "{told:?}");
        assert!(!m.complete);
        let mut told: Vec<String> = Vec::new();
        let m = run_mass(
            &request,
            Mass::new(Arc::clone(&plan)),
            &parts,
            &profile,
            None,
            3,
            &mut |event| {
                let flow = match &event {
                    MassEvent::Finished(_) => ControlFlow::Break(()),
                    _ => ControlFlow::Continue(()),
                };
                told.push(match event {
                    MassEvent::Started { .. } => "S".to_string(),
                    MassEvent::Finished(_) => "F".to_string(),
                    other => format!("{other:?}"),
                });
                flow
            },
        );
        assert_eq!(told, ["S", "S", "S", "F", "Cancelled"]);
        assert!(!m.complete && m.results.iter().flatten().count() >= 1);
    }
}
