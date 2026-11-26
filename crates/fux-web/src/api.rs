//! Requests and answers: what the page asks for, and what it shows.

use std::fmt::Display;
use std::ops::RangeInclusive;
use std::str::FromStr;

use anyhow::{Result, anyhow, bail};
use fux_cli::commands::{
    CounterpointOptions, counterpoint_summary, localize_names, prepare_counterpoint,
    report_summary, violation_position_after, voice_text,
};
use fux_cli::free::{
    CadencePlan, FreeOptions, PreparedFree, Repeat, eleison, free_summary, parse_parts,
    prepare_free, result_warnings,
};
use fux_cli::imitation::{
    AutoCount, ChantRef, Given, ImitationMode, ImitationOptions, ImitationSource, SubjectChoice,
    Terms, answer_choice, attach_ending, form_message, phrase_list, plan_summary, toggle,
};
use fux_cli::input::{Cadences, ScoreInput, kern_text_score, musicxml_text_fields};
use fux_cli::mass::{MovementCheck, check_movement, mass_profile, part_of_file, restore_part};
use fux_cli::text::{
    DEFAULT_TEXT, align_lyrics, is_none, restates, text_label, text_of, unrecognised,
};
use fux_core::corpus::{fux_cantus, fux_examples};
use fux_core::generate::{
    CantusRequest, FreeRequest, GenerateError, Generated, generate_cantus, generate_free,
};
use fux_core::imitation::OptionError;
use fux_core::imitation::planner::plan_imitation;
use fux_core::rules::{BUILTIN_PROFILES, Checker, Profile, Report, all_rules};
use fux_core::text::latin::strip_acute;
use fux_core::text::plan::{automatic_length, plan};
use fux_core::text::{Eleison, SylFlags, Text, TextOptions, ordinary};
use fux_core::{Ambitus, Lang, Mode, ModeSpec, Score, VoiceType};
use fux_export::{
    ClefStyle, LilypondOptions, MidiOptions, MusicXmlOptions, to_lilypond, to_midi, to_musicxml,
};
use serde::{Deserialize, Serialize};

use crate::base64;
use crate::mass::{self, MassForm, MassView, PartAnswer, PartFile, ShownSection};

/// A request of the page. Which fields count depends on `command`; missing ones keep their
/// defaults.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Request {
    /// `info`, `cp` (counterpoint), `free` (free counterpoint), `cf` (cantus firmi), `check`,
    /// `example`, `text` (a text to sing as `fux text` shows it, with the length of a free
    /// setting of it), or for a mass (phase 14c) `mass_plan` (its plan, without a search),
    /// `mass_part` (one of its parts, as a worker of the page sets it) and `mass_join` (the files
    /// of its movements and its book from the parts set).
    pub command: String,
    /// `de` (default) or `en`.
    pub lang: String,
    /// `fux` (default) or `jeppesen`.
    pub profile: String,
    /// Changed rules as `ID=LEVEL` or `ID.PARAMETER=VALUE`.
    pub rules: Vec<String>,
    /// The cantus: note names, LilyPond or `fux:NAME`; for `cp` empty to generate one.
    pub cf: String,
    /// The mode, e.g. `dorian` or `hypophrygisch`; empty: from the cantus.
    pub mode: String,
    /// The voice type of the cantus, e.g. `A`; empty: from its range.
    pub cf_voice: String,
    /// 1 to 5, or 0 for free counterpoint around the cantus.
    pub species: u8,
    /// Voices with the cantus (`free`: all voices, default 4).
    pub voices: u8,
    /// Two voices: the counterpoint below the cantus.
    pub below: bool,
    /// The voice type of the counterpoint or of the species voice.
    pub voice: String,
    /// The voices from top to bottom, e.g. `S A T B`; `mass_join`: the parts of the mass set so
    /// far.
    pub parts: Parts,
    /// Length of a generated cantus (`free`, `text`: measures; 0 for as many as the text plans,
    /// 12 without a text).
    pub length: usize,
    /// Number of solutions.
    pub count: usize,
    pub seed: u64,
    /// Internal cadences as measures, e.g. `4,8`; empty for none (`free`, `text`: `auto` or empty
    /// for planned ones, with a text at ends of sense units, `none`, or measures with targets such
    /// as `7,12:F`).
    pub cadences: String,
    /// `free`, `text`: what the voices sing: a section of the Ordinary (`kyrie`, the default),
    /// some of its sense units (`gloria:1-5`), an own Latin text, or `none`. `cp` with species 0:
    /// what the free voices sing around the cantus, the same choices, but empty or `none` (the
    /// default) for no text. `check`: the text of the lyrics, as `fux check --text` takes it:
    /// empty to recognise the section, `none` to check no text.
    pub text: String,
    /// `free`, `cp`, `text`, `info`: how freely the voices restate the text: `auto` or empty (as
    /// the profile has it), `keine` (`none`) or `frei` (`free`); `check`: as the setting was made.
    pub repeat: String,
    /// `free`, `cp`, `text`: the printed syllables keep the acute of the liturgical books
    /// (default on).
    pub accents: Option<bool>,
    /// `free`, `cp`, `text`, `info`: eleison in 4 syllables (0, the default: e-lé-i-son, as in
    /// the books) or 3 (e-léi-son).
    pub eleison: u8,
    /// Further voices of a score to check, as `TYPE=NOTES`.
    pub others: Vec<String>,
    /// A MusicXML file to check, as text.
    pub musicxml: String,
    /// A Humdrum `**kern` file to check (free counterpoint), as text.
    pub kern: String,
    /// A figure of Fux's examples, e.g. `5`.
    pub figure: String,
    /// Fux's clefs in LilyPond and MusicXML.
    pub original_clefs: bool,
    /// Tempo in halves per minute (default 72) and General MIDI program 1–128 (default 53,
    /// choir), in MIDI and MusicXML.
    pub tempo: u32,
    pub program: u8,
    /// Brackets over the entries of an imitation in LilyPond and MusicXML (default on).
    pub marks: Option<bool>,
    /// `free`, `text`: whether the voices imitate each other, and how (phase 13).
    pub imitation: ImitationRequest,
    /// `mass_plan`, `mass_part`, `mass_join`: the mass as the page's form gives it.
    pub mass: MassForm,
    /// `mass_part`: the part to set, by its address in the plan (`credo/3`).
    pub part: String,
    /// `mass_part`: the head motif of Kyrie I as the answer for Kyrie I gives it
    /// ([`PartAnswer::motif`]), for a part that opens with it.
    pub motif: Option<String>,
    /// `mass_join`: the page's workers are still setting parts of the mass, so the parts not set
    /// yet go without saying: no line on the parts missing, and no hint how to set them.
    pub running: bool,
}

/// The `parts` of a request: the voices from top to bottom as text (`cp`, `free`, `text`: `S A
/// T B`), or the parts of a mass that the page's workers have set (`mass_join`).
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Parts {
    Voices(String),
    Set(Vec<PartFile>),
}

impl Default for Parts {
    fn default() -> Self {
        Parts::Voices(String::new())
    }
}

impl Parts {
    /// The voices as text; empty for the parts of a mass.
    pub fn voices(&self) -> &str {
        match self {
            Parts::Voices(voices) => voices,
            Parts::Set(_) => "",
        }
    }

    /// The parts of a mass; none for voices.
    pub fn set(&self) -> &[PartFile] {
        match self {
            Parts::Voices(_) => &[],
            Parts::Set(parts) => parts,
        }
    }
}

/// The imitation of a free setting as the page asks for it, as `fux free --imitation …` takes
/// it. Missing fields leave their choice to the planner.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ImitationRequest {
    /// `auto` or empty (with a text, three to five voices, automatic internal cadences and an
    /// automatic length), `an` (`on`) or `aus` (`off`).
    pub mode: String,
    /// Where the subjects come from: `text` or empty (from the words), `kyriale` (the chant of
    /// the Kyriale that sings the section; `chant:kyrie-iv` names one), a chant as GABC text, or
    /// heads as notes in LilyPond, `|` between them. With a chant, a mode left empty is the
    /// chant's.
    pub subject: String,
    /// The phrases of the chant from 1, such as `2,3`; empty to find them by the words.
    pub phrases: String,
    /// The number of points; 0 for automatic.
    pub points: usize,
    /// The voices of the later points: `auto` or empty, `alle` (`all`) or a number.
    pub later_voices: String,
    /// The order of the entries per point, the leader first, such as `A S T B; T B`.
    pub order: String,
    /// The intervals of the later entries to the leader per point: `0`, `d` or `±n` steps, such
    /// as `d 0 d; 0`.
    pub intervals: String,
    /// The delays between the entries per point in measures, such as `1 2 1.5; 1`.
    pub delays: String,
    /// Pairs of entries from four voices: `auto` or empty, `an`, `aus`.
    pub pairs: String,
    /// The second statement of a short text as point 1 restated: `auto` or empty, `an`, `aus`.
    pub restate: String,
    /// `auto` or empty, `real` or `modal`.
    pub answer: String,
    /// The notes of every head, 4 or more; 0 for automatic.
    pub head: usize,
    /// Only the plan of the imitation, without a search: its points, heads and entries as the
    /// summary, without results.
    pub plan_only: bool,
}

/// The answer: results to show, or an error.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Answer {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Lines describing the results.
    pub summary: Vec<String>,
    pub results: Vec<Entry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<Info>,
    /// `text`: the text, unless it is `none`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextView>,
    /// `text`: the length of a free setting with the request's choices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<PlanView>,
    /// `mass_plan`: the plan of the mass. Where the choices of the page's table of sections do
    /// not fit, with the error the plan without them, whose rows the page shows to change them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mass: Option<MassView>,
    /// `mass_part`: the part as set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<PartAnswer>,
    /// `mass_join`: the LilyPond source of the book of the mass, with what its parts have set
    /// (`results` are its movements whose parts are all set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book: Option<String>,
}

impl Answer {
    pub(crate) fn failed(error: String) -> Answer {
        Answer {
            ok: false,
            error: Some(error),
            ..Answer::default()
        }
    }

    pub(crate) fn results(summary: Vec<String>, results: Vec<Entry>) -> Answer {
        Answer {
            ok: true,
            summary,
            results,
            ..Answer::default()
        }
    }
}

/// One result: a score in all forms, with its violations.
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub label: String,
    pub cost: u32,
    pub forbidden: usize,
    /// The voices as text, e.g. `Sopran: D5 | A4 | C5`.
    pub lines: Vec<String>,
    /// What each voice sings, e.g. `Ký-ri-e e-lé-i-son. e-lé-i-son.`; `null` for a voice without
    /// text.
    pub lyrics: Vec<Option<String>>,
    pub violations: Vec<Finding>,
    pub musicxml: String,
    pub lilypond: String,
    /// A Standard MIDI File in base64.
    pub midi: String,
    /// The notes for playback.
    pub notes: Vec<Sound>,
    /// Eighths per measure.
    pub measure: u32,
    /// `mass_join`: the sections of the movement as the page shows them, each the score of the
    /// voices that sing in it, as the book prints it; none for every other result.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<ShownSection>,
    /// `mass_join`: the name of the movement's files without extension, as `fux mass` names
    /// them: `messe-3-credo`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// A rule violation as the page lists it.
#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub position: String,
    pub message: String,
    pub rule: String,
    pub level: String,
    pub forbidden: bool,
}

/// A note to play: voice, start and end in eighths, MIDI key. Tied notes are one sound.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Sound {
    pub voice: usize,
    pub start: u32,
    pub end: u32,
    pub key: i32,
}

/// What the page offers to choose from.
#[derive(Clone, Debug, Serialize)]
pub struct Info {
    pub cantus: Vec<Named>,
    pub modes: Vec<Named>,
    pub voices: Vec<Named>,
    pub examples: Vec<Named>,
    pub profiles: Vec<String>,
    pub rules: Vec<Named>,
    /// The sections of the Ordinary to sing in free counterpoint.
    pub texts: Vec<Section>,
}

/// An id to send back and a name to show.
#[derive(Clone, Debug, Serialize)]
pub struct Named {
    pub id: String,
    pub name: String,
}

/// A section of the Ordinary: its id, its names, its size and the length of a free setting.
#[derive(Clone, Debug, Serialize)]
pub struct Section {
    pub id: String,
    /// The name in the request's language.
    pub name: String,
    pub name_de: String,
    pub name_en: String,
    /// With eleison divided as the request says.
    pub syllables: usize,
    /// The sense units, as a range such as `gloria:1-5` counts them.
    pub units: usize,
    /// The automatic length of a free setting in measures: the text sung as often as the profile
    /// and the request's repetition allow.
    pub length: usize,
}

/// A text as the page previews it, like `fux text`.
#[derive(Clone, Debug, Serialize)]
pub struct TextView {
    /// `kyrie`, `gloria:1-5`, or `eigen` for an own text.
    pub id: String,
    pub name: String,
    /// The name with the size, e.g. „Kyrie I (7 Silben, 1 Sinnabschnitt)“.
    pub label: String,
    /// „kurz“ or „lang“: restated freely or rarely.
    pub kind: String,
    pub syllables: usize,
    pub words: usize,
    pub sentences: usize,
    /// The sense units, each as its syllables.
    pub units: Vec<Vec<SyllableView>>,
    /// The words before the marked breaks (`|`), where a voice may rest.
    pub breaks: Vec<Vec<SyllableView>>,
    /// The tails (`/`): ends of units that may be restated on their own.
    pub tails: Vec<Vec<SyllableView>>,
    /// Warnings in the request's language, such as uncertain accents.
    pub warnings: Vec<String>,
    /// The automatic length of a free setting in measures, as [`Section::length`].
    pub length: usize,
}

/// A syllable as the page shows a text.
#[derive(Clone, Debug, Serialize)]
pub struct SyllableView {
    /// The book form, with the acute unless the request turns the accents off: `lé`.
    pub text: String,
    /// The word is stressed on this syllable.
    pub stress: bool,
    /// What follows it in its line: `-` inside a word; after a word its punctuation and a space,
    /// ` | ` at a marked break or ` / ` before a tail; nothing more at the end of the line.
    pub after: String,
}

/// The length of a free setting with the choices of a request.
#[derive(Clone, Debug, Default, Serialize)]
pub struct PlanView {
    /// The measures: as many as the text plans unless the length is given, 12 without a text;
    /// with imitation, those of the plan for the request's seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measures: Option<usize>,
    /// Whether the voices imitate each other: the plan of the imitation sets the length then.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub imitation: bool,
    /// Why such a setting cannot be made, such as a length too short for the text, with what may
    /// help.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// Answers a request given as JSON, with JSON.
pub fn answer_json(text: &str) -> String {
    let answer = match serde_json::from_str::<Request>(text) {
        Ok(request) => answer(&request),
        Err(error) => Answer::failed(format!("request: {error}")),
    };
    serde_json::to_string(&answer).expect("answers serialize")
}

/// Answers a request.
pub fn answer(request: &Request) -> Answer {
    let lang = request.lang.parse().unwrap_or_default();
    respond(request, lang).unwrap_or_else(|error| Answer::failed(format!("{error:#}")))
}

fn respond(request: &Request, lang: Lang) -> Result<Answer> {
    let base = base_profile(request)?;
    let profile = profile(&base, request)?;
    match request.command.as_str() {
        "info" => Ok(Answer {
            ok: true,
            info: Some(info(request, &profile, lang)?),
            ..Answer::default()
        }),
        "cp" => counterpoint(request, &profile, lang),
        "free" => free(request, &profile, lang),
        "cf" => cantus(request, &profile, lang),
        "check" => check(request, &base, &profile, lang),
        "example" => example(request, &profile, lang),
        "text" => text(request, &profile, lang),
        // A mass lays its overlay over the profile before the changed rules (D7).
        "mass_plan" => mass::plan(request, &base, lang),
        "mass_part" => mass::part(request, &base, lang),
        "mass_join" => mass::join(request, &base, lang),
        other => bail!("unknown command {other:?}"),
    }
}

/// The named profile, without the changed rules.
fn base_profile(request: &Request) -> Result<Profile> {
    let name = match request.profile.trim() {
        "" => "fux",
        name => name,
    };
    Profile::builtin(name)
        .ok_or_else(|| anyhow!("unknown profile {name:?} ({})", BUILTIN_PROFILES.join(", ")))
}

/// The named profile `base` with the changed rules.
fn profile(base: &Profile, request: &Request) -> Result<Profile> {
    let mut profile = base.clone();
    for rule in rules(request) {
        profile
            .apply_override(&rule)
            .map_err(|e| anyhow!("{rule}: {e}"))?;
    }
    Ok(profile)
}

/// The changed rules of a request, without empty lines.
pub(crate) fn rules(request: &Request) -> Vec<String> {
    request
        .rules
        .iter()
        .filter(|rule| !rule.trim().is_empty())
        .cloned()
        .collect()
}

/// A field that may be empty.
pub(crate) fn optional<T>(text: &str) -> Result<Option<T>>
where
    T: FromStr,
    T::Err: Display,
{
    match text.trim() {
        "" => Ok(None),
        text => text.parse().map(Some).map_err(|e| anyhow!("{text}: {e}")),
    }
}

pub(crate) fn text_or_none(text: &str) -> Option<String> {
    Some(text.trim().to_string()).filter(|t| !t.is_empty())
}

/// The options of `fux cp` that a request gives. Free counterpoint (species 0) sings the text
/// the request names; without one, and in the species, the voices sing none.
fn counterpoint_options(request: &Request) -> Result<CounterpointOptions> {
    let cadences = match request.cadences.trim() {
        "" => Cadences::default(),
        text => text.parse().map_err(|e: String| anyhow!(e))?,
    };
    Ok(CounterpointOptions {
        cf: text_or_none(&request.cf),
        mode: optional(&request.mode)?,
        cf_voice: optional(&request.cf_voice)?,
        cadences,
        length: if request.length == 0 {
            11
        } else {
            request.length
        },
        species: request.species.min(5),
        voices: usize::from(request.voices.clamp(2, 6)),
        parts: text_or_none(request.parts.voices()),
        below: request.below,
        voice: optional(&request.voice)?,
        // Always given: the time is unknown in WebAssembly.
        seed: Some(request.seed),
        count: request.count.clamp(1, 10),
        beam: 200,
        noise: 2.0,
        details: false,
        text: text_or_none(&request.text).unwrap_or_else(|| "none".to_string()),
        repeat: optional(&request.repeat)?.unwrap_or_default(),
        accents: request.accents.unwrap_or(true),
        eleison: eleison_of(request)?,
    })
}

fn counterpoint(request: &Request, profile: &Profile, lang: Lang) -> Result<Answer> {
    let prepared = prepare_counterpoint(&counterpoint_options(request)?, profile, lang)?;
    let texted = prepared.request.text.is_some();
    let results = prepared
        .generate()
        .map_err(|e| anyhow!(counterpoint_error(&e, texted, lang)))?;
    let warning = lang.pick("Warnung", "Warning");
    let mut summary: Vec<String> = prepared
        .warnings
        .iter()
        .map(|w| format!("{warning}: {w}"))
        .collect();
    summary.extend(counterpoint_summary(&prepared, &results, lang));
    let title = lang.pick("Kontrapunkt", "Counterpoint");
    Ok(Answer::results(
        summary,
        numbered(results, title, &prepared.profile, request, lang),
    ))
}

/// The message of a failed counterpoint search; where the free voices sing a text, with what
/// may help, in the words of the page.
fn counterpoint_error(error: &GenerateError, texted: bool, lang: Lang) -> String {
    let message = error.message(lang);
    match (error, texted) {
        (GenerateError::NoCounterpoint { .. }, true) => format!(
            "{message} {}",
            lang.pick(
                "Abhilfe: Wiederholung frei, ein kürzerer Text, ein längerer Cantus oder andere Binnenschlüsse.",
                "Remedies: repetition free, a shorter text, a longer cantus or other internal cadences.",
            )
        ),
        _ => message,
    }
}

/// The text a request names, or the default text.
fn text_spec(request: &Request) -> String {
    text_or_none(&request.text).unwrap_or_else(|| DEFAULT_TEXT.to_string())
}

/// The division of eleison a request asks for: four syllables unless it says three.
fn eleison_of(request: &Request) -> Result<Eleison> {
    eleison_number(request.eleison)
}

/// The division of eleison as a number of syllables: four (also for 0) or three.
pub(crate) fn eleison_number(syllables: u8) -> Result<Eleison> {
    match syllables {
        0 => Ok(Eleison::Four),
        n => eleison(&n.to_string()).map_err(|e| anyhow!("eleison: {e}")),
    }
}

/// The profile with TXT-REPEAT as the request's repetition sets it, as `fux free` uses it.
fn with_repeat(profile: &Profile, request: &Request) -> Result<Profile> {
    let repeat: Repeat = optional(&request.repeat)?.unwrap_or_default();
    let mut profile = profile.clone();
    for spec in repeat.overrides() {
        profile
            .apply_override(spec)
            .map_err(|e| anyhow!("{spec}: {e}"))?;
    }
    Ok(profile)
}

/// The imitation options of `fux free` that a request gives.
fn imitation_options(imitation: &ImitationRequest) -> Result<ImitationOptions> {
    let field = |name: &'static str| move |e: String| anyhow!("imitation.{name}: {e}");
    let choice = |text: &str| match text.trim() {
        "" => "auto".to_string(),
        text => text.to_string(),
    };
    let phrases = match imitation.phrases.trim() {
        "" => None,
        list => Some(phrase_list(list).ok_or_else(|| {
            anyhow!("imitation.phrases: {list:?}: numbers from 1 such as 2,3 or 2-4")
        })?),
    };
    let subject = match imitation.subject.trim() {
        "" => SubjectChoice::Text,
        kyriale if kyriale.eq_ignore_ascii_case("kyriale") => SubjectChoice::Chant {
            chant: ChantRef::Kyriale,
            phrases,
        },
        // A chant pasted as GABC: its header ends with %%, its notes are in parentheses.
        gabc if gabc.contains("%%") || gabc.contains('(') => SubjectChoice::Chant {
            chant: ChantRef::Gabc(gabc.to_string()),
            phrases,
        },
        other => match other.parse().map_err(field("subject"))? {
            SubjectChoice::Chant {
                chant,
                phrases: given,
            } => SubjectChoice::Chant {
                chant,
                phrases: given.or(phrases),
            },
            subject => subject,
        },
    };
    Ok(ImitationOptions {
        mode: choice(&imitation.mode).parse().map_err(field("mode"))?,
        subject,
        points: AutoCount((imitation.points > 0).then_some(imitation.points)),
        restate: toggle(&choice(&imitation.restate)).map_err(field("restate"))?,
        point_voices: choice(&imitation.later_voices)
            .parse()
            .map_err(field("later_voices"))?,
        order: imitation.order.parse().map_err(field("order"))?,
        intervals: imitation.intervals.parse().map_err(field("intervals"))?,
        delays: imitation.delays.parse().map_err(field("delays"))?,
        pairs: toggle(&choice(&imitation.pairs)).map_err(field("pairs"))?,
        answer: answer_choice(&choice(&imitation.answer)).map_err(field("answer"))?,
        head: match imitation.head {
            0 => AutoCount(None),
            n if n >= 4 => AutoCount(Some(n)),
            n => bail!("imitation.head: {n}: 0 (auto) or a number from 4"),
        },
        plan_only: imitation.plan_only,
    })
}

/// The options of `fux free` that a request gives. An imitation asked for that cannot apply is
/// an error in the words of the page.
fn free_options(request: &Request, lang: Lang) -> Result<FreeOptions> {
    let cadences = match request.cadences.trim() {
        "" => CadencePlan::Auto,
        text => text.parse().map_err(|e: String| anyhow!(e))?,
    };
    // The imitation's fields count only where it can apply: switched off, the page hides them,
    // and where `auto` leaves it aside, a field it cannot read stops no setting.
    let mode: ImitationMode = match request.imitation.mode.trim() {
        "" => ImitationMode::Auto,
        mode => mode
            .parse()
            .map_err(|e: String| anyhow!("imitation.mode: {e}"))?,
    };
    let voices = match request.voices {
        0 => 4,
        n => usize::from(n.clamp(2, 6)),
    };
    let voice_count = text_or_none(request.parts.voices())
        .and_then(|parts| parse_parts(&parts, lang).ok())
        .map_or(voices, |parts| parts.len());
    let applies = ImitationOptions::default().applies(
        !is_none(&text_spec(request)),
        voice_count,
        cadences == CadencePlan::Auto,
        request.length == 0,
    );
    let imitation = match mode {
        ImitationMode::Off => ImitationOptions {
            mode,
            plan_only: request.imitation.plan_only,
            ..ImitationOptions::default()
        },
        ImitationMode::Auto if !applies => {
            imitation_options(&request.imitation).unwrap_or_else(|_| ImitationOptions {
                plan_only: request.imitation.plan_only,
                ..ImitationOptions::default()
            })
        }
        _ => imitation_options(&request.imitation)?,
    };
    if imitation.mode == ImitationMode::On {
        if is_none(&text_spec(request)) {
            bail!(
                "{}",
                lang.pick(
                    "Imitation braucht einen Text: einen Text wählen oder Imitation aus.",
                    "Imitation needs a text: choose one, or imitation off.",
                )
            );
        }
        if cadences != CadencePlan::Auto {
            bail!(
                "{}",
                lang.pick(
                    "Die Imitation setzt die Binnenschlüsse selbst: Binnenschlüsse auto oder Imitation aus.",
                    "Imitation places the internal cadences itself: internal cadences auto, or imitation off.",
                )
            );
        }
    }
    Ok(FreeOptions {
        mode: optional(&request.mode)?,
        voices,
        parts: text_or_none(request.parts.voices()),
        length: (request.length > 0).then_some(request.length),
        cadences,
        ending: None,
        text: text_spec(request),
        repeat: optional(&request.repeat)?.unwrap_or_default(),
        accents: request.accents.unwrap_or(true),
        eleison: eleison_of(request)?,
        // Always given: the time is unknown in WebAssembly.
        seed: Some(request.seed),
        count: request.count.clamp(1, 10),
        beam: 200,
        noise: 2.0,
        details: false,
        imitation,
    })
}

fn free(request: &Request, profile: &Profile, lang: Lang) -> Result<Answer> {
    let prepared = prepare_free(&free_options(request, lang)?, profile, lang)
        .map_err(|e| anyhow!(form_message(&e, lang, Terms::Page)))?;
    let warning = lang.pick("Warnung", "Warning");
    let mut summary: Vec<String> = prepared
        .warnings
        .iter()
        .map(|w| format!("{warning}: {w}"))
        .collect();
    if request.imitation.plan_only {
        // The plan alone, without a search.
        let lines = plan_summary(&prepared, lang).ok_or_else(|| {
            anyhow!(
                "{}",
                lang.pick(
                    "Dieser Satz imitiert nicht (auto: mit Text, drei bis fünf Stimmen, automatischen Binnenschlüssen und automatischer Länge); Imitation an schaltet sie ein.",
                    "This setting does not imitate (auto: with a text, three to five voices, automatic internal cadences and an automatic length); imitation on turns it on.",
                )
            )
        })?;
        summary.extend(lines.map_err(|e| anyhow!(free_error(&e, &prepared.request, lang)))?);
        return Ok(Answer::results(summary, Vec::new()));
    }
    let results = generate_free(&prepared.request, &prepared.profile)
        .map_err(|e| anyhow!(free_error(&e, &prepared.request, lang)))?;
    summary.extend(
        result_warnings(&results, lang)
            .into_iter()
            .map(|w| format!("{warning}: {w}")),
    );
    summary.extend(free_summary(&prepared, &results, lang));
    let title = lang.pick("Freier Satz", "Free counterpoint");
    Ok(Answer::results(
        summary,
        numbered(results, title, &prepared.profile, request, lang),
    ))
}

/// The message of a failed free search of `request`; for a texted or imitative setting with what
/// may help, in the words of the page.
fn free_error(error: &GenerateError, request: &FreeRequest, lang: Lang) -> String {
    let message = error.message(lang);
    let texted = request.text.is_some();
    if request.imitation.is_some()
        && let Some(hint) = imitation_remedies(error, request, lang)
    {
        return format!("{message} {hint}");
    }
    let hint = match (error, texted) {
        (GenerateError::TextTooLong { measures }, _) => match lang {
            Lang::De => format!(
                "Abhilfe: {measures} Takte oder automatisch, weniger Sinnabschnitte oder ein kürzerer Text, oder weniger Binnenschlüsse."
            ),
            Lang::En => format!(
                "Remedies: {measures} measures or automatic, fewer sense units or a shorter text, or fewer internal cadences."
            ),
        },
        (GenerateError::NoCounterpoint { .. }, true) => lang
            .pick(
                "Abhilfe: mehr Takte, Wiederholung frei, ein kürzerer Text oder andere Binnenschlüsse.",
                "Remedies: more measures, repetition free, a shorter text or other internal cadences.",
            )
            .to_string(),
        _ => return message,
    };
    format!("{message} {hint}")
}

/// What the page can change where an imitation fails, in the words of the page.
fn imitation_remedies(error: &GenerateError, request: &FreeRequest, lang: Lang) -> Option<String> {
    let undo = Given::of(request).undo(match lang {
        Lang::De => [
            "Kopf 0 (auto)",
            "Reihenfolge leer",
            "Intervalle leer",
            "Takte automatisch",
        ],
        Lang::En => [
            "head 0 (auto)",
            "order empty",
            "intervals empty",
            "measures automatic",
        ],
    });
    Some(match (error, lang) {
        (GenerateError::NoImitation { .. }, Lang::De) => format!(
            "Abhilfe: ein anderer Seed, andere Abstände, weniger Stimmen späterer Punkte{undo} oder Imitation aus."
        ),
        (GenerateError::NoImitation { .. }, Lang::En) => format!(
            "Remedies: another seed, other delays, fewer voices of later points{undo} or imitation off."
        ),
        (GenerateError::ImitationTooLong { measures }, Lang::De) => {
            format!("Abhilfe: {measures} Takte oder automatisch, oder Imitation aus.")
        }
        (GenerateError::ImitationTooLong { measures }, Lang::En) => {
            format!("Remedies: {measures} measures or automatic, or imitation off.")
        }
        (GenerateError::SubjectInvalid { .. }, _) => lang
            .pick(
                "Abhilfe: andere Noten für das Soggetto, oder das Soggetto aus dem Text.",
                "Remedies: other notes for the subject, or the subject from the text.",
            )
            .to_string(),
        (GenerateError::NoChantPhrase { phrases, .. }, Lang::De) => {
            format!("Abhilfe: Phrasen von 1 bis {phrases}, oder keine.")
        }
        (GenerateError::NoChantPhrase { phrases, .. }, Lang::En) => {
            format!("Remedies: phrases from 1 to {phrases}, or none.")
        }
        (GenerateError::PointCount { most }, Lang::De) => {
            format!("Abhilfe: Punkte auto oder höchstens {most}.")
        }
        (GenerateError::PointCount { most }, Lang::En) => {
            format!("Remedies: points auto or {most} at most.")
        }
        (GenerateError::ImitationOption(option), _) => option_remedy(option, lang),
        _ => return None,
    })
}

/// What the page can change where the imitation has an option the request cannot have, in the
/// words of the page.
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
        (OptionError::LaterVoices { most, .. }, Lang::De) => {
            format!("Abhilfe: Stimmen späterer Punkte auto, alle oder 2 bis {most}.")
        }
        (OptionError::LaterVoices { most, .. }, Lang::En) => {
            format!("Remedies: voices of later points auto, all or 2 to {most}.")
        }
        (OptionError::Head { most, .. }, Lang::De) => {
            format!("Abhilfe: Kopf 0 (auto) oder 4 bis {most} Töne.")
        }
        (OptionError::Head { most, .. }, Lang::En) => {
            format!("Remedies: head 0 (auto) or 4 to {most} notes.")
        }
        (OptionError::Subjects { points, .. }, Lang::De) => format!(
            "Abhilfe: höchstens {points} Köpfe oder Phrasen, oder Punkt 1 wiederholen aus."
        ),
        (OptionError::Subjects { points, .. }, Lang::En) => {
            format!("Remedies: {points} heads or phrases at most, or restate point 1 off.")
        }
        (OptionError::Restate, _) => lang
            .pick(
                "Abhilfe: Punkt 1 wiederholen auto oder aus.",
                "Remedies: restate point 1 auto or off.",
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

/// `text`: the text a request names, as the page previews it, and the length of a free setting
/// with the request's choices. A text that cannot be read is an error; a setting that cannot be
/// planned is a problem of the plan.
fn text(request: &Request, profile: &Profile, lang: Lang) -> Result<Answer> {
    let accents = request.accents.unwrap_or(true);
    let restating = with_repeat(profile, request)?;
    let text = text_of(&text_spec(request), eleison_of(request)?, lang)?
        .map(|(text, warnings)| text_view(&text, warnings, &restating, accents, lang));
    let plan = planned(request, profile, lang).unwrap_or_else(|error| PlanView {
        problem: Some(format!("{error:#}")),
        ..PlanView::default()
    });
    Ok(Answer {
        ok: true,
        text,
        plan: Some(plan),
        ..Answer::default()
    })
}

/// A text as the page previews it: its sense units, marked breaks and tails with the stressed
/// syllables, its warnings, and its automatic length under `profile`.
fn text_view(
    text: &Text,
    warnings: Vec<String>,
    profile: &Profile,
    accents: bool,
    lang: Lang,
) -> TextView {
    let flags = |s: usize| text.syllables[s].flags;
    let units = text
        .units
        .iter()
        .map(|unit| {
            line(
                text,
                usize::from(unit.first)..=usize::from(unit.last),
                accents,
            )
        })
        .collect();
    // As `fux text`: the word before each break that does not start a tail.
    let breaks = (1..text.syllables.len())
        .filter(|&s| flags(s - 1).contains(SylFlags::BREAK_AFTER))
        .filter(|&s| !flags(s).contains(SylFlags::TAIL_START))
        .map(|s| {
            let word = text.word_of(s - 1);
            let first = usize::from(word.first);
            line(text, first..=first + usize::from(word.len) - 1, accents)
        })
        .collect();
    let tails = (0..text.syllables.len())
        .filter(|&s| flags(s).contains(SylFlags::TAIL_START))
        .map(|s| line(text, s..=usize::from(text.unit_of(s).last), accents))
        .collect();
    TextView {
        id: text.id.clone(),
        name: text.name(lang).to_string(),
        label: text_label(text, lang),
        kind: text.kind.name(lang).to_string(),
        syllables: text.syllables.len(),
        words: text.words.len(),
        sentences: text.sentences().count(),
        units,
        breaks,
        tails,
        warnings,
        length: automatic_length(text, restates(profile, text)),
    }
}

/// Syllables of a text in a line, as the page shows them.
fn line(text: &Text, syllables: RangeInclusive<usize>, accents: bool) -> Vec<SyllableView> {
    let last = *syllables.end();
    syllables
        .map(|s| {
            let syllable = &text.syllables[s];
            let mut after = String::new();
            if !syllable.flags.contains(SylFlags::WORD_END) {
                after.push('-');
            } else {
                after.extend(text.word_of(s).punct);
                if s < last {
                    let broken = syllable.flags.contains(SylFlags::BREAK_AFTER);
                    let tail = text.syllables[s + 1].flags.contains(SylFlags::TAIL_START);
                    after.push_str(match (broken, tail) {
                        (true, true) => " / ",
                        (true, false) => " | ",
                        (false, _) => " ",
                    });
                }
            }
            SyllableView {
                text: if accents {
                    syllable.text.clone()
                } else {
                    strip_acute(&syllable.text)
                },
                stress: syllable.flags.contains(SylFlags::STRESS),
                after,
            }
        })
        .collect()
}

/// The length of a free setting with the request's choices, as `fux free` plans it, and why the
/// search would fail at once if the text does not fit. An imitation's length is the one of its
/// plan for the request's seed.
fn planned(request: &Request, profile: &Profile, lang: Lang) -> Result<PlanView> {
    let options = free_options(request, lang)?;
    let prepared = prepare_free(&options, profile, lang)
        .map_err(|e| anyhow!(form_message(&e, lang, Terms::Page)))?;
    if prepared.request.imitation.is_some() {
        return Ok(imitation_length(&prepared, lang));
    }
    let mode = prepared.request.mode;
    let problem = prepared.request.text.as_ref().and_then(|sung| {
        // The plan as `prepare_free` and the search make it: `auto` leaves the cadences to it.
        let given = match &options.cadences {
            CadencePlan::Auto => None,
            given => Some(given.cadences(mode, 0)),
        };
        plan(
            &sung.text,
            mode,
            prepared.request.voices.len(),
            options.length,
            given,
            restates(&prepared.profile, &sung.text),
        )
        .err()
    });
    Ok(PlanView {
        measures: Some(prepared.request.measures),
        problem: problem.map(|e| free_error(&e, &prepared.request, lang)),
        imitation: false,
    })
}

/// The length of a prepared imitative setting: that of its plan, or why it has none.
fn imitation_length(prepared: &PreparedFree, lang: Lang) -> PlanView {
    let request = &prepared.request;
    let planned = request
        .imitation
        .as_ref()
        .map(|imitation| plan_imitation(request, imitation, &prepared.profile));
    match planned {
        Some(Ok(planner)) => PlanView {
            measures: Some(planner.plan().measures),
            problem: None,
            imitation: true,
        },
        Some(Err(error)) => PlanView {
            measures: None,
            problem: Some(free_error(&error, request, lang)),
            imitation: true,
        },
        None => PlanView::default(),
    }
}

/// The solutions of a search as entries, titled and numbered.
fn numbered(
    results: Vec<Generated>,
    title: &str,
    profile: &Profile,
    request: &Request,
    lang: Lang,
) -> Vec<Entry> {
    results
        .into_iter()
        .enumerate()
        .map(|(i, mut generated)| {
            localize_names(&mut generated.score, lang);
            generated.score.title = Some(format!("{title} {}", i + 1));
            let label = format!(
                "{}. {} {}",
                i + 1,
                lang.pick("Kosten", "cost"),
                generated.cost
            );
            entry(
                label,
                &generated.score,
                &generated.report,
                profile,
                request,
                Some(request.seed),
                lang,
            )
        })
        .collect()
}

fn cantus(request: &Request, profile: &Profile, lang: Lang) -> Result<Answer> {
    let mode = optional(&request.mode)?.unwrap_or(ModeSpec::new(Mode::Dorian));
    let length = if request.length == 0 {
        11
    } else {
        request.length
    };
    let cantus_request = CantusRequest {
        voice_type: optional(&request.cf_voice)?.unwrap_or(VoiceType::Alto),
        seed: request.seed,
        count: request.count.clamp(1, 20),
        ..CantusRequest::new(mode, length)
    };
    let generated =
        generate_cantus(&cantus_request, profile).map_err(|e| anyhow!(e.message(lang)))?;
    let summary = vec![format!(
        "{} – {length} {}, seed {}",
        mode.name(lang),
        lang.pick("Töne", "notes"),
        request.seed
    )];
    let entries = generated
        .into_iter()
        .enumerate()
        .map(|(i, mut g)| {
            localize_names(&mut g.score, lang);
            g.score.title = Some(format!("Cantus firmus {}", i + 1));
            let label = format!("{}. {} {}", i + 1, lang.pick("Kosten", "cost"), g.cost);
            entry(
                label,
                &g.score,
                &g.report,
                profile,
                request,
                Some(request.seed),
                lang,
            )
        })
        .collect();
    Ok(Answer::results(summary, entries))
}

/// `check`: the violations of a score from notes, Humdrum or MusicXML. The file of a mass
/// movement is checked part by part, another file in two sections or more section by section
/// ([`movement_check`]); the file of a part of a mass as the part was set (D14), under the
/// profile of the mass on `base`, as `fux check` checks them.
fn check(request: &Request, base: &Profile, profile: &Profile, lang: Lang) -> Result<Answer> {
    let mode = optional(&request.mode)?;
    let repeat: Repeat = optional(&request.repeat)?.unwrap_or_default();
    // A MusicXML file of fux-kontrapunkt names the text of its lyrics and keeps the plan of its
    // imitation, and the file of a mass or of a part of one what its plan said.
    let mut stored = None;
    let mut imitation = None;
    let mut fields = Vec::new();
    let mut score = if !request.kern.trim().is_empty() {
        kern_text_score(&request.kern, mode)?
    } else if request.musicxml.trim().is_empty() {
        ScoreInput {
            mode,
            cf: text_or_none(&request.cf),
            cf_voice: optional(&request.cf_voice)?,
            voices: request
                .others
                .iter()
                .filter(|v| !v.trim().is_empty())
                .cloned()
                .collect(),
            ..ScoreInput::default()
        }
        .score(lang)?
    } else {
        let (read, kept) = musicxml_text_fields(&request.musicxml, mode, lang)?;
        let given = text_or_none(&request.text);
        let rules = rules(request);
        let movement = check_movement(
            &read,
            &kept,
            given.as_deref(),
            base,
            &rules,
            repeat,
            ImitationSource::File,
            lang,
        )?;
        if let Some(movement) = movement {
            return Ok(movement_check(read.score, &movement, request, lang));
        }
        stored = read.text;
        imitation = read.imitation;
        fields = kept;
        read.score
    };
    // Lyrics are checked with their text, as `fux check --text` checks them, and restatements
    // as the setting was made (`fux check --repeat`); the entries of an imitation as the file
    // plans them, unless the plan does not fit the notes. The file of a part of a mass sings as
    // the part was set: its text going on where it went on, its syllables at the positions they
    // sang, its last point ending on the part's ending.
    let part = part_of_file(&fields, lang)?;
    let text = text_or_none(&request.text).or(stored);
    let mut lines = align_lyrics(&mut score, text.as_deref(), lang)?;
    if let Some(part) = &part {
        restore_part(&mut score, part, &fields);
    }
    let ending = part
        .as_ref()
        .map_or_else(|| score.mode.final_step(), |part| part.ending);
    match attach_ending(
        &mut score,
        imitation,
        ImitationSource::File,
        "",
        ending,
        lang,
    ) {
        Ok(planned) => lines.extend(planned),
        Err(unfit) => lines.push(format!(
            "{}: {unfit:#}; {}",
            lang.pick("Warnung", "Warning"),
            lang.pick("geprüft ohne ihn", "checked without it")
        )),
    }
    let profile = match part {
        Some(_) => mass_profile(base, &rules(request), repeat)?,
        None => with_repeat(profile, request)?,
    };
    let label = lang.pick("Geprüfter Satz", "Checked score").to_string();
    let mut answer = checked(label, score, &profile, request, lang);
    answer.summary.splice(
        0..0,
        lines.into_iter().map(|line| in_page_words(line, lang)),
    );
    Ok(answer)
}

/// The check of a file in parts or sections (`fux_cli::mass::check_movement`) as one result, the
/// file `whole`: its notes, and the violations of its pieces in the measures of the file; in the
/// lines, as `fux check` prints them, for each piece its heading, the lines on its text and its
/// imitation and its prohibitions and cost, then the totals.
fn movement_check(
    mut whole: Score,
    check: &MovementCheck,
    request: &Request,
    lang: Lang,
) -> Answer {
    let checker = Checker::new(&check.profile);
    let mut summary = Vec::new();
    let mut all = Report::default();
    let mut violations = Vec::new();
    for piece in &check.pieces {
        summary.push(piece.heading.clone());
        summary.extend(
            piece
                .lines
                .iter()
                .map(|line| format!("  {}", in_page_words(line.clone(), lang))),
        );
        summary.push(format!("  {}", report_summary(&piece.report, lang)));
        violations.extend(findings_after(
            &piece.score,
            &piece.report,
            &checker,
            piece.before,
            lang,
        ));
        all.violations
            .extend(piece.report.violations.iter().cloned());
    }
    let pieces = check.pieces.len();
    let counted = match (lang, check.parts, pieces) {
        (Lang::De, true, 1) => "1 Teil".to_string(),
        (Lang::De, true, n) => format!("{n} Teilen"),
        (Lang::De, false, 1) => "1 Abschnitt".to_string(),
        (Lang::De, false, n) => format!("{n} Abschnitten"),
        (Lang::En, true, 1) => "1 part".to_string(),
        (Lang::En, true, n) => format!("{n} parts"),
        (Lang::En, false, 1) => "1 section".to_string(),
        (Lang::En, false, n) => format!("{n} sections"),
    };
    let totals = report_summary(&all, lang);
    let totals = totals.trim_end_matches('.');
    summary.push(match lang {
        Lang::De => format!("Gesamt: {totals} in {counted}."),
        Lang::En => format!("Total: {totals} in {counted}."),
    });
    localize_names(&mut whole, lang);
    let entry = Entry {
        label: lang.pick("Geprüfter Satz", "Checked score").to_string(),
        cost: all.cost(),
        forbidden: all.forbidden(),
        lines: voice_lines(&whole),
        lyrics: voice_lyrics(&whole),
        // Those of the pieces, each against its own notes.
        violations,
        musicxml: to_musicxml(&whole, &musicxml_options(request, lang)),
        lilypond: to_lilypond(&whole, &lilypond_options(request, lang)),
        midi: base64::encode(&to_midi(&whole, &midi_options(request))),
        notes: sounds(&whole),
        measure: whole.meter.ticks_per_measure(),
        sections: Vec::new(),
        file: None,
    };
    Answer::results(summary, vec![entry])
}

/// A line of [`align_lyrics`] in the words of the page, which has a field where `fux check` has
/// the option `--text`.
fn in_page_words(line: String, lang: Lang) -> String {
    if line == unrecognised(lang) {
        lang.pick(
            "Text nicht erkannt; im Feld „Text der Stimmen“ angeben",
            "Text not recognised; give it in the field “Text of the voices”",
        )
        .to_string()
    } else {
        line
    }
}

fn example(request: &Request, profile: &Profile, lang: Lang) -> Result<Answer> {
    let score = ScoreInput {
        example: text_or_none(&request.figure),
        ..ScoreInput::default()
    }
    .score(lang)?;
    let label = format!("Fig. {}", request.figure.trim());
    Ok(checked(label, score, profile, request, lang))
}

/// A score with all its violations.
fn checked(
    label: String,
    mut score: Score,
    profile: &Profile,
    request: &Request,
    lang: Lang,
) -> Answer {
    localize_names(&mut score, lang);
    let report = Checker::new(profile).check(&score);
    let summary = vec![report_summary(&report, lang)];
    let entry = entry(label, &score, &report, profile, request, None, lang);
    Answer::results(summary, vec![entry])
}

/// A score in all the forms the page needs; its MusicXML names `seed`, the seed it was set with,
/// where that is known.
fn entry(
    label: String,
    score: &Score,
    report: &Report,
    profile: &Profile,
    request: &Request,
    seed: Option<u64>,
    lang: Lang,
) -> Entry {
    let checker = Checker::new(profile);
    let musicxml = MusicXmlOptions {
        seed,
        ..musicxml_options(request, lang)
    };
    Entry {
        label,
        cost: report.cost(),
        forbidden: report.forbidden(),
        lines: voice_lines(score),
        lyrics: voice_lyrics(score),
        violations: findings_after(score, report, &checker, 0, lang),
        musicxml: to_musicxml(score, &musicxml),
        lilypond: to_lilypond(score, &lilypond_options(request, lang)),
        midi: base64::encode(&to_midi(score, &midi_options(request))),
        notes: sounds(score),
        measure: score.meter.ticks_per_measure(),
        sections: Vec::new(),
        file: None,
    }
}

/// The violations of `report` as the page lists them, in the order of the notes they mark; the
/// measures counted on from `before`, as in a part of a mass in its movement (phase 14).
pub(crate) fn findings_after(
    score: &Score,
    report: &Report,
    checker: &Checker,
    before: usize,
    lang: Lang,
) -> Vec<Finding> {
    let mut violations: Vec<_> = report.violations.iter().collect();
    violations.sort_by_key(|v| (score.note_start(v.voice, v.note).unwrap_or(v.tick), v.voice));
    violations
        .into_iter()
        .map(|v| Finding {
            position: violation_position_after(score, v, before, lang),
            message: checker.message(v, lang),
            rule: v.rule.to_string(),
            level: v.level.label(lang),
            forbidden: v.is_forbidden(),
        })
        .collect()
}

/// The voices as text, e.g. `Sopran: D5 | A4 | C5`.
pub(crate) fn voice_lines(score: &Score) -> Vec<String> {
    score
        .voices
        .iter()
        .map(|v| format!("{}: {}", v.name, voice_text(v, score.meter)))
        .collect()
}

/// What each voice sings; `None` for a voice without text.
pub(crate) fn voice_lyrics(score: &Score) -> Vec<Option<String>> {
    (0..score.voices.len())
        .map(|v| score.voice_lyrics(v))
        .collect()
}

/// The clefs a request asks for: Fux's, or modern ones.
fn clefs(request: &Request) -> ClefStyle {
    if request.original_clefs {
        ClefStyle::Original
    } else {
        ClefStyle::Modern
    }
}

/// MusicXML with the clefs and the brackets over the entries a request asks for, the tempo and
/// the program of its MIDI file ([`midi_options`]) and its head in `lang`, without a seed.
pub(crate) fn musicxml_options(request: &Request, lang: Lang) -> MusicXmlOptions {
    let midi = midi_options(request);
    MusicXmlOptions {
        clefs: clefs(request),
        entry_marks: request.marks.unwrap_or(true),
        tempo: midi.tempo,
        program: midi.program,
        lang,
        seed: None,
    }
}

/// LilyPond with the clefs and the brackets over the entries a request asks for, without
/// interval figures, in the language of the page.
pub(crate) fn lilypond_options(request: &Request, lang: Lang) -> LilypondOptions {
    LilypondOptions {
        clefs: clefs(request),
        interval_figures: false,
        lang,
        entry_marks: request.marks.unwrap_or(true),
        mensurstriche: false,
    }
}

/// MIDI with the tempo and the program a request asks for: 72 halves a minute and 53 (choir)
/// unless it says otherwise.
pub(crate) fn midi_options(request: &Request) -> MidiOptions {
    MidiOptions {
        tempo: if request.tempo == 0 {
            72
        } else {
            request.tempo
        },
        program: if request.program == 0 {
            53
        } else {
            request.program
        },
        ..MidiOptions::default()
    }
}

/// The notes of all voices for playback; a tie lengthens the note before.
pub(crate) fn sounds(score: &Score) -> Vec<Sound> {
    let mut sounds = Vec::new();
    for (v, voice) in score.voices.iter().enumerate() {
        let mut tied = false;
        for (start, note) in voice.timed() {
            let end = start + note.duration.ticks();
            match note.pitch {
                Some(_) if tied => {
                    if let Some(last) = sounds.last_mut() {
                        let last: &mut Sound = last;
                        last.end = end;
                    }
                }
                Some(pitch) => sounds.push(Sound {
                    voice: v,
                    start,
                    end,
                    key: pitch.midi(),
                }),
                None => {}
            }
            tied = note.tie && note.pitch.is_some();
        }
    }
    sounds
}

/// The choices of the page: Fux's cantus and examples, modes, voice types, profiles, rules, and
/// the sections of the Ordinary, with eleison divided and their lengths planned as the request
/// says.
fn info(request: &Request, profile: &Profile, lang: Lang) -> Result<Info> {
    let cantus = fux_cantus()
        .iter()
        .map(|c| {
            let notes: Vec<String> = c.notes.iter().map(ToString::to_string).collect();
            Named {
                id: format!("fux:{}", c.id),
                name: format!("{} – {}", c.mode.name(lang), notes.join(" ")),
            }
        })
        .collect();
    let mut modes = Vec::new();
    for mode in Mode::ALL {
        for ambitus in [Ambitus::Authentic, Ambitus::Plagal] {
            for transposed in [false, true] {
                let spec = ModeSpec::new(mode)
                    .with_ambitus(ambitus)
                    .with_transposition(transposed);
                modes.push(Named {
                    id: spec.id(),
                    name: spec.name(lang),
                });
            }
        }
    }
    let voices = VoiceType::ALL
        .iter()
        .map(|v| Named {
            id: v.abbreviation().to_string(),
            name: v.name(lang).to_string(),
        })
        .collect();
    let examples = fux_examples()
        .iter()
        .map(|e| {
            let species: Vec<String> = e.species.iter().map(|s| s.number().to_string()).collect();
            Named {
                id: e.figure.clone(),
                name: format!(
                    "Fig. {} – {}, {} {}, {}. {}",
                    e.figure,
                    e.mode.name(lang),
                    e.parts.len(),
                    lang.pick("Stimmen", "voices"),
                    species.join("+"),
                    lang.pick("Gattung", "species")
                ),
            }
        })
        .collect();
    let rules = all_rules()
        .iter()
        .map(|r| Named {
            id: r.info().id.to_string(),
            name: r.info().title(lang).to_string(),
        })
        .collect();
    let restating = with_repeat(profile, request)?;
    let options = TextOptions {
        eleison: eleison_of(request)?,
    };
    let texts = ordinary::sections(&options)
        .iter()
        .map(|text| Section {
            id: text.id.clone(),
            name: text.name(lang).to_string(),
            name_de: text.name_de.clone(),
            name_en: text.name_en.clone(),
            syllables: text.syllables.len(),
            units: text.units.len(),
            length: automatic_length(text, restates(&restating, text)),
        })
        .collect();
    Ok(Info {
        cantus,
        modes,
        voices,
        examples,
        profiles: BUILTIN_PROFILES.iter().map(ToString::to_string).collect(),
        rules,
        texts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(json: &str) -> serde_json::Value {
        serde_json::from_str(&answer_json(json)).unwrap()
    }

    #[test]
    fn counterpoint_in_all_forms() {
        let answer = ask(r#"{"command": "cp", "cf": "fux:dorian", "species": 2, "seed": 3}"#);
        assert_eq!(answer["ok"], true, "{answer}");
        assert!(
            answer["summary"][0]
                .as_str()
                .unwrap()
                .starts_with("dorisch – 2. Gattung, 2 Stimmen, seed 3")
        );
        let result = &answer["results"][0];
        assert_eq!(result["forbidden"], 0);
        let xml = result["musicxml"].as_str().unwrap();
        assert!(xml.contains("<score-partwise"));
        // Choir voices in the tempo of the MIDI file, under the seed of the request.
        assert!(
            xml.contains(">fux-kontrapunkt, Seed 3</credit-words>"),
            "{xml}"
        );
        assert!(xml.contains("<instrument-sound>voice.alto</instrument-sound>"));
        assert!(xml.contains("<sound tempo=\"144\"/>"));
        assert_eq!(xml.matches("<midi-program>53</midi-program>").count(), 2);
        let slower = ask(
            r#"{"command": "cp", "cf": "fux:dorian", "species": 2, "seed": 3, "tempo": 60, "program": 20}"#,
        );
        let xml = slower["results"][0]["musicxml"].as_str().unwrap();
        assert!(xml.contains("<sound tempo=\"120\"/>"), "{xml}");
        assert_eq!(xml.matches("<midi-program>20</midi-program>").count(), 2);
        assert!(result["lilypond"].as_str().unwrap().contains("\\score"));
        // "MThd" in base64.
        assert!(result["midi"].as_str().unwrap().starts_with("TVRoZA"));
        assert_eq!(result["measure"], 8);
        // The cantus in whole notes: 11 sounds of 8 eighths.
        let notes = result["notes"].as_array().unwrap();
        let cantus: Vec<_> = notes
            .iter()
            .filter(|n| n["end"].as_u64().unwrap() - n["start"].as_u64().unwrap() == 8)
            .collect();
        assert!(cantus.len() >= 11);
        assert!(result["lines"][0].as_str().unwrap().starts_with("Sopran: "));
    }

    #[test]
    fn free_counterpoint() {
        let answer = ask(
            r#"{"command": "free", "mode": "phrygian", "voices": 2, "length": 8, "count": 2, "seed": 5, "text": "none"}"#,
        );
        assert_eq!(answer["ok"], true, "{answer}");
        assert_eq!(
            answer["summary"][0],
            "phrygisch – freier Satz, 2 Stimmen, 8 Takte, seed 5"
        );
        let first = &answer["results"][0];
        assert_eq!(first["forbidden"], 0);
        assert_eq!(first["lines"].as_array().unwrap().len(), 2);
        assert_eq!(first["lyrics"], serde_json::json!([null, null]));
        assert!(!first["musicxml"].as_str().unwrap().contains("<lyric"));
        // Free voices around a cantus: species 0, without a text unless one is named.
        let around = ask(r#"{"command": "cp", "cf": "fux:dorian", "species": 0, "seed": 1}"#);
        assert_eq!(around["ok"], true, "{around}");
        assert!(
            around["summary"][0]
                .as_str()
                .unwrap()
                .starts_with("dorisch – freier Satz, 2 Stimmen")
        );
        assert_eq!(
            around["results"][0]["lyrics"],
            serde_json::json!([null, null])
        );
        let bad = ask(r#"{"command": "free", "cadences": "x"}"#);
        assert_eq!(bad["ok"], false);
    }

    #[test]
    fn free_counterpoint_on_a_text() {
        // Kyrie I, the default text, once, with e-léi-son and without accents: 8 measures.
        let answer = ask(
            r#"{"command": "free", "voices": 2, "seed": 1, "repeat": "keine", "eleison": 3, "accents": false}"#,
        );
        assert_eq!(answer["ok"], true, "{answer}");
        assert_eq!(
            answer["summary"],
            serde_json::json!([
                "dorisch – freier Satz, 2 Stimmen, 8 Takte, seed 1",
                "Text: Kyrie I (6 Silben, 1 Sinnabschnitt), Wiederholung keine"
            ])
        );
        let first = &answer["results"][0];
        assert_eq!(first["forbidden"], 0);
        assert_eq!(
            first["lyrics"],
            serde_json::json!(["Ky-ri-e e-lei-son.", "Ky-ri-e e-lei-son."])
        );
        let xml = first["musicxml"].as_str().unwrap();
        // Six syllables per voice, and an extender line ending under a tied note.
        assert_eq!(xml.matches("<lyric").count(), 13);
        assert_eq!(xml.matches("<extend type=\"stop\"/>").count(), 1);
        assert!(xml.contains("<text>Ky</text>") && !xml.contains("Ký"));
        assert!(first["lilypond"].as_str().unwrap().contains("\\lyricsto"));
        // An own text: its warnings come first.
        let own = ask(
            r#"{"command": "free", "voices": 2, "seed": 1, "repeat": "keine", "text": "Animam meam dedi.", "lang": "en"}"#,
        );
        assert_eq!(
            own["summary"][0],
            "Warning: Uncertain accent: Animam → Á-ni-mam; mark it with an acute",
            "{own}"
        );
        assert_eq!(
            own["summary"][2],
            "Text: Own text (7 syllables, 1 sense unit), repetition none"
        );
        assert_eq!(own["results"][0]["lyrics"][0], "Á-ni-mam me-am de-di.");
        // A given length too short for the text fails at once, with what may help; in four
        // voices, which imitate, the length of the imitation's plan.
        let short = ask(
            r#"{"command": "free", "text": "gloria:1-5", "length": 10, "imitation": {"mode": "aus"}}"#,
        );
        assert_eq!(
            short["error"],
            "Der Text braucht mindestens 21 Takte. Abhilfe: 21 Takte oder automatisch, weniger Sinnabschnitte oder ein kürzerer Text, oder weniger Binnenschlüsse."
        );
        // `auto` keeps a given length free of imitation; an imitation asked for says how long
        // its shortest plan is.
        let automatic = ask(r#"{"command": "free", "text": "gloria:1-5", "length": 10}"#);
        assert!(
            automatic["error"]
                .as_str()
                .unwrap()
                .starts_with("Der Text braucht mindestens "),
            "{automatic}"
        );
        let imitative = ask(
            r#"{"command": "free", "text": "gloria:1-5", "length": 10, "imitation": {"mode": "an"}}"#,
        );
        let error = imitative["error"].as_str().unwrap();
        assert!(
            error.starts_with(
                "Kein Plan der Imitation passt in die gegebene Länge; der kürzeste hat "
            ) && error.ends_with(" Takte oder automatisch, oder Imitation aus."),
            "{error}"
        );
        for bad in [
            r#"{"command": "free", "text": "kyrei"}"#,
            r#"{"command": "free", "repeat": "oft"}"#,
            r#"{"command": "free", "eleison": 5}"#,
        ] {
            assert_eq!(ask(bad)["ok"], false, "{bad}");
        }
    }

    /// Free voices around a cantus sing the text a `cp` request names, the cantus none.
    #[test]
    fn free_voices_around_a_cantus_sing_a_text() {
        let answer = ask(
            r#"{"command": "cp", "cf": "fux:dorian", "species": 0, "seed": 1, "text": "kyrie", "repeat": "keine", "eleison": 3, "accents": false}"#,
        );
        assert_eq!(answer["ok"], true, "{answer}");
        assert_eq!(
            answer["summary"],
            serde_json::json!([
                "dorisch – freier Satz, 2 Stimmen, seed 1",
                "C. f. (Alt): D4 | F4 | E4 | D4 | G4 | F4 | A4 | G4 | F4 | E4 | D4",
                "Text: Kyrie I (6 Silben, 1 Sinnabschnitt), Wiederholung keine"
            ])
        );
        let made = &answer["results"][0];
        assert_eq!(made["forbidden"], 0);
        assert_eq!(
            made["lyrics"],
            serde_json::json!(["Ky-ri-e e-lei-son.", null])
        );
        let lilypond = made["lilypond"].as_str().unwrap();
        assert!(
            lilypond.contains("\\lyricsto \"voice0\" { \"Ky\" --"),
            "{lilypond}"
        );
        assert_eq!(lilypond.matches("\\lyricsto").count(), 1, "{lilypond}");
        // Checked from its MusicXML file, which names its text, the soprano has the violations
        // it had; the check also reports those of the cantus (CF-…).
        let xml = made["musicxml"].as_str().unwrap();
        let checked = ask(
            &serde_json::json!({"command": "check", "musicxml": xml, "repeat": "keine"})
                .to_string(),
        );
        assert_eq!(
            checked["summary"][0],
            "Text: Kyrie I, e-léi-son; von 6 gesungenen Silben weichen 0 ab"
        );
        let found = &checked["results"][0];
        assert_eq!(found["forbidden"], 0, "{checked}");
        let free: Vec<&serde_json::Value> = found["violations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| !v["rule"].as_str().unwrap().starts_with("CF-"))
            .collect();
        let had: Vec<&serde_json::Value> = made["violations"].as_array().unwrap().iter().collect();
        assert_eq!(free, had);
        // The species sing no text; a text too long for the cantus fails at once.
        let species =
            ask(r#"{"command": "cp", "cf": "fux:dorian", "species": 1, "text": "kyrie"}"#);
        assert_eq!(
            species["error"],
            "Einen Text singen nur freie Gegenstimmen (Gattung 0)."
        );
        let long = ask(
            r#"{"command": "cp", "cf": "fux:dorian", "species": 0, "text": "gloria:1-5", "lang": "en"}"#,
        );
        assert!(
            long["error"]
                .as_str()
                .unwrap()
                .starts_with("The text needs at least 19 measures. The cantus has 11 measures."),
            "{long}"
        );
        for bad in [
            r#"{"command": "cp", "species": 0, "text": "kyrei"}"#,
            r#"{"command": "cp", "species": 0, "text": "kyrie", "repeat": "oft"}"#,
            r#"{"command": "cp", "species": 0, "text": "kyrie", "eleison": 5}"#,
        ] {
            assert_eq!(ask(bad)["ok"], false, "{bad}");
        }
    }

    #[test]
    fn lyrics_are_checked_with_their_text() {
        let answer = ask(
            r#"{"command": "free", "voices": 2, "seed": 1, "repeat": "keine", "eleison": 3, "accents": false}"#,
        );
        let xml = answer["results"][0]["musicxml"].as_str().unwrap();
        let check = |xml: &str, text: &str| {
            ask(&serde_json::json!({"command": "check", "musicxml": xml, "text": text}).to_string())
        };
        let rules = |answer: &serde_json::Value| -> Vec<String> {
            answer["results"][0]["violations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["rule"].as_str().unwrap().to_string())
                .collect()
        };
        // The file names its text, divided as the lyrics are: checked, nothing to report.
        let stored = check(xml, "");
        assert_eq!(
            stored["summary"],
            serde_json::json!([
                "Text: Kyrie I, e-léi-son; von 12 gesungenen Silben weichen 0 ab",
                "0 Verbot(e), Kosten 0 aus 0 Präferenz(en)."
            ])
        );
        assert_eq!(stored["results"][0]["lyrics"][1], "Ky-ri-e e-lei-son.");
        // Without the fields of the program, as another program writes it, the section is
        // recognised.
        let start = xml.find("<miscellaneous>").unwrap();
        let end = xml.find("</miscellaneous>").unwrap() + "</miscellaneous>".len();
        let foreign = format!("{}{}", &xml[..start], &xml[end..]);
        let recognised = check(&foreign, "");
        assert_eq!(
            recognised["summary"][0],
            "Text erkannt: Kyrie I, e-léi-son; von 12 gesungenen Silben weichen 0 ab"
        );
        let named = check(&foreign, "kyrie");
        assert_eq!(
            named["summary"][0],
            "Text: Kyrie I, e-léi-son; von 12 gesungenen Silben weichen 0 ab"
        );
        // Without the text rules, the repeated notes of the syllables cost (MEL-REPEAT).
        let none = check(xml, "none");
        assert!(!none["summary"][0].as_str().unwrap().starts_with("Text"));
        assert!(rules(&none).iter().all(|r| r == "MEL-REPEAT"), "{none}");
        // Without its first syllable, the lyrics are not recognised; with the text named, the
        // voice is reported where it enters.
        let start = foreign.find("<lyric").unwrap();
        let end = start + foreign[start..].find("</lyric>").unwrap() + "</lyric>".len();
        let falsified = format!("{}{}", &foreign[..start], &foreign[end..]);
        assert_eq!(
            check(&falsified, "")["summary"][0],
            "Text nicht erkannt; im Feld „Text der Stimmen“ angeben"
        );
        let reported = check(&falsified, "kyrie");
        assert_eq!(rules(&reported), ["TXT-ENTRY", "TXT-ORDER"]);
        assert_eq!(reported["results"][0]["forbidden"], 2);
        // A setting whose voices restate the text, checked with the repetition it was made with:
        // free as made, forbidden where the check says `keine`.
        let made = ask(r#"{"command": "free", "voices": 2, "seed": 1, "repeat": "frei"}"#);
        let made = &made["results"][0];
        let xml = made["musicxml"].as_str().unwrap();
        let again = |repeat: &str| {
            ask(
                &serde_json::json!({"command": "check", "musicxml": xml, "repeat": repeat})
                    .to_string(),
            )
        };
        let free = again("frei");
        assert_eq!(free["results"][0]["violations"], made["violations"]);
        let forbidden = again("keine");
        assert!(
            rules(&forbidden).iter().any(|r| r == "TXT-REPEAT"),
            "{forbidden}"
        );
        // Notes without lyrics, with a text named.
        let plain = ask(
            r#"{"command": "check", "cf": "D4 E4", "others": ["S=f'1 g'1"], "text": "kyrie", "lang": "en"}"#,
        );
        assert_eq!(
            plain["summary"][0],
            "Warning: No voice carries lyrics; the text rules are not checked."
        );
    }

    #[test]
    fn texts_as_the_page_previews_them() {
        let syllables = |line: &serde_json::Value| -> String {
            line.as_array()
                .unwrap()
                .iter()
                .map(|s| {
                    let text = s["text"].as_str().unwrap();
                    let text = if s["stress"] == true {
                        text.to_uppercase()
                    } else {
                        text.to_string()
                    };
                    text + s["after"].as_str().unwrap()
                })
                .collect()
        };
        let kyrie = ask(r#"{"command": "text"}"#);
        let text = &kyrie["text"];
        assert_eq!(text["id"], "kyrie");
        assert_eq!(text["label"], "Kyrie I (7 Silben, 1 Sinnabschnitt)");
        assert_eq!(text["kind"], "kurz");
        assert_eq!(
            [&text["syllables"], &text["words"], &text["sentences"]],
            [7, 2, 1]
        );
        assert_eq!(syllables(&text["units"][0]), "KÝ-ri-e / e-LÉ-i-son.");
        assert_eq!(text["breaks"], serde_json::json!([]));
        assert_eq!(syllables(&text["tails"][0]), "e-LÉ-i-son.");
        assert_eq!(text["length"], 16);
        // Four voices imitate: the plan of the imitation sets the length.
        assert_eq!(kyrie["plan"]["imitation"], true);
        assert!(kyrie["plan"]["measures"].as_u64().unwrap() > 16, "{kyrie}");
        let plain = ask(r#"{"command": "text", "imitation": {"mode": "aus"}}"#);
        assert_eq!(plain["plan"], serde_json::json!({"measures": 16}));
        // Without accents, in English: the stress stays.
        let benedictus =
            ask(r#"{"command": "text", "text": "benedictus", "accents": false, "lang": "en"}"#);
        let text = &benedictus["text"];
        assert_eq!(text["kind"], "short");
        assert_eq!(
            syllables(&text["units"][0]),
            "Be-ne-DI-ctus | qui VE-nit / in NO-mi-ne DO-mi-ni."
        );
        assert_eq!(syllables(&text["breaks"][0]), "Be-ne-DI-ctus");
        assert_eq!(syllables(&text["tails"][0]), "in NO-mi-ne DO-mi-ni.");
        // eleison in three syllables, sung once.
        let once = ask(r#"{"command": "text", "eleison": 3, "repeat": "keine", "voices": 2}"#);
        assert_eq!(syllables(&once["text"]["units"][0]), "KÝ-ri-e / e-LÉI-son.");
        assert_eq!(once["text"]["length"], 8);
        assert_eq!(once["plan"]["measures"], 8);
        // An own text with its warnings; a range of a section.
        let own = ask(r#"{"command": "text", "text": "Ad te levavi animam meam."}"#);
        assert_eq!(own["text"]["id"], "eigen");
        assert_eq!(
            syllables(&own["text"]["units"][0]),
            "Ad TE le-VÁ-vi Á-ni-mam ME-am."
        );
        assert_eq!(
            own["text"]["warnings"],
            serde_json::json!(["Akzent unsicher: animam → á-ni-mam; mit Akut angeben"])
        );
        let range = ask(r#"{"command": "text", "text": "gloria:1-5"}"#);
        assert_eq!(range["text"]["units"].as_array().unwrap().len(), 5);
        assert_eq!(range["text"]["length"], 32);
        assert_eq!(
            syllables(&range["text"]["units"][0]),
            "Et in TER-ra PAX | ho-MÍ-ni-bus BO-nae vo-lun-TÁ-tis."
        );
        // No text: the length of an untexted setting, or the given one.
        let none = ask(r#"{"command": "text", "text": "none"}"#);
        assert!(none["text"].is_null());
        assert_eq!(none["plan"], serde_json::json!({"measures": 12}));
        let given = ask(r#"{"command": "text", "text": "none", "length": 40}"#);
        assert_eq!(given["plan"]["measures"], 40);
        // A setting that cannot be planned: the text is shown, the plan says why.
        let short = ask(
            r#"{"command": "text", "text": "gloria:1-5", "length": 10, "imitation": {"mode": "aus"}}"#,
        );
        assert_eq!(short["plan"]["measures"], 10);
        assert!(
            short["plan"]["problem"]
                .as_str()
                .unwrap()
                .starts_with("Der Text braucht mindestens 21 Takte. Abhilfe: 21 Takte")
        );
        let imitative = ask(
            r#"{"command": "text", "text": "gloria:1-5", "length": 10, "imitation": {"mode": "an"}}"#,
        );
        assert!(imitative["plan"]["measures"].is_null());
        assert!(
            imitative["plan"]["problem"]
                .as_str()
                .unwrap()
                .starts_with("Kein Plan der Imitation passt in die gegebene Länge"),
            "{imitative}"
        );
        let cadences = ask(r#"{"command": "text", "cadences": "x"}"#);
        assert_eq!(cadences["text"]["id"], "kyrie");
        assert!(cadences["plan"]["measures"].is_null());
        assert!(
            cadences["plan"]["problem"]
                .as_str()
                .unwrap()
                .contains("auto, none")
        );
        // A text that cannot be read is an error.
        for bad in [
            r#"{"command": "text", "text": "kyrei"}"#,
            r#"{"command": "text", "text": "gloria:9-99"}"#,
            r#"{"command": "text", "text": "Ad te x7"}"#,
            r#"{"command": "text", "eleison": 2}"#,
        ] {
            let answer = ask(bad);
            assert_eq!(answer["ok"], false, "{bad}");
            assert!(answer["error"].as_str().is_some_and(|e| !e.is_empty()));
        }
    }

    /// The imitation of a free request: the choices the page gives, the plan alone, the length of
    /// the plan for the preview, a chant's subjects in the chant's mode, and errors in the words
    /// of the page.
    #[test]
    fn free_counterpoint_in_imitation() {
        let options = |json: &str| {
            let request: Request = serde_json::from_str(json).unwrap();
            imitation_options(&request.imitation)
        };
        let all = options(
            r#"{"imitation": {"mode": "an", "subject": "kyriale", "phrases": "2,3", "points": 2,
                "later_voices": "alle", "order": "A S T B", "intervals": "d 0 d",
                "delays": "1 2 1.5", "pairs": "aus", "restate": "an", "answer": "modal",
                "head": 5}}"#,
        )
        .unwrap();
        assert_eq!(all.mode, ImitationMode::On);
        assert_eq!(
            all.subject,
            SubjectChoice::Chant {
                chant: ChantRef::Kyriale,
                phrases: Some(vec![2, 3])
            }
        );
        assert_eq!(
            (all.points, all.head),
            (AutoCount(Some(2)), AutoCount(Some(5)))
        );
        assert_eq!(all.delays.0, [Some(vec![8, 16, 12])]);
        assert_eq!(options("{}").unwrap(), ImitationOptions::default());
        // A chant pasted as GABC, a chant of the Kyriale by its key, heads as notes.
        let gabc =
            options(r#"{"imitation": {"subject": "name:Amen;\n%%\n(c4) A(f)men(g)."}}"#).unwrap();
        assert!(matches!(
            gabc.subject,
            SubjectChoice::Chant {
                chant: ChantRef::Gabc(_),
                phrases: None
            }
        ));
        let key =
            options(r#"{"imitation": {"subject": "chant:kyrie-iv", "phrases": "1"}}"#).unwrap();
        assert_eq!(
            key.subject,
            SubjectChoice::Chant {
                chant: ChantRef::Named("kyrie-iv".into()),
                phrases: Some(vec![1])
            }
        );
        let notes =
            options(r#"{"imitation": {"subject": "d'1 d'2 a'2 g'4 f'4 e'2 d'1"}}"#).unwrap();
        assert!(matches!(notes.subject, SubjectChoice::Notes(ref heads) if heads[0].len() == 7));
        for bad in [
            r#"{"imitation": {"mode": "ja"}}"#,
            r#"{"imitation": {"head": 3}}"#,
            r#"{"imitation": {"delays": "x"}}"#,
            r#"{"imitation": {"order": "A X"}}"#,
            r#"{"imitation": {"subject": "kyriale", "phrases": "0"}}"#,
        ] {
            assert!(options(bad).is_err(), "{bad}");
        }
        // Fields of an imitation that is off, or that `auto` leaves aside, stop no setting; where
        // it applies they do, in the words of the page.
        for json in [
            r#"{"command": "free", "voices": 2, "text": "none", "length": 8, "seed": 1, "imitation": {"mode": "aus", "subject": "x'1", "order": "A X", "delays": "1.25"}}"#,
            r#"{"command": "free", "voices": 2, "text": "none", "length": 8, "seed": 1, "imitation": {"subject": "x'1", "intervals": "q"}}"#,
        ] {
            let answer = ask(json);
            assert_eq!(answer["ok"], true, "{json}: {answer}");
        }
        let preview = ask(
            r#"{"command": "text", "voices": 2, "imitation": {"mode": "aus", "delays": "1.25"}}"#,
        );
        assert!(preview["plan"]["problem"].is_null(), "{preview}");
        let order = ask(
            r#"{"command": "free", "voices": 3, "seed": 1, "imitation": {"order": "S A", "plan_only": true}}"#,
        );
        assert_eq!(
            order["error"],
            "Reihenfolge: Die Besetzung hat keine Stimme S. Abhilfe: eine andere Reihenfolge, oder keine."
        );
        let later = ask(
            r#"{"command": "free", "seed": 1, "imitation": {"later_voices": "5", "plan_only": true}}"#,
        );
        assert_eq!(
            later["error"],
            "Spätere Punkte können nicht 5 Stimmen haben, nur zwei bis 4. Abhilfe: Stimmen späterer Punkte auto, alle oder 2 bis 4."
        );
        // The plan alone, and its length in the preview of the text.
        let plan =
            ask(r#"{"command": "free", "voices": 3, "seed": 1, "imitation": {"plan_only": true}}"#);
        assert_eq!(plan["ok"], true, "{plan}");
        assert_eq!(plan["results"], serde_json::json!([]));
        let head = plan["summary"][0].as_str().unwrap();
        assert!(
            head.starts_with("dorisch – Plan der Imitation, 3 Stimmen, "),
            "{plan}"
        );
        let summary: Vec<&str> = plan["summary"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l.as_str().unwrap())
            .collect();
        assert!(
            summary
                .iter()
                .any(|l| l.starts_with("Punkt 1 („Ký-ri-e e-lé-i-son.“): ")),
            "{plan}"
        );
        let measures: u64 = head
            .split(", ")
            .find_map(|part| part.strip_suffix(" Takte"))
            .and_then(|n| n.parse().ok())
            .unwrap();
        let preview = ask(r#"{"command": "text", "voices": 3, "seed": 1}"#);
        assert_eq!(
            preview["plan"],
            serde_json::json!({"measures": measures, "imitation": true})
        );
        // A chant's subjects, in the chant's mode when the mode is left empty.
        let sanctus = ask(
            r#"{"command": "free", "voices": 4, "seed": 1, "text": "sanctus", "imitation": {"subject": "kyriale", "plan_only": true}}"#,
        );
        assert_eq!(sanctus["ok"], true, "{sanctus}");
        let chant = fux_core::imitation::chant::Kyriale::SanctusIX.chant();
        let mode = fux_cli::imitation::chant_mode(&chant).unwrap();
        assert!(
            sanctus["summary"][0]
                .as_str()
                .unwrap()
                .starts_with(&format!("{} – Plan der Imitation", mode.name(Lang::De))),
            "{sanctus}"
        );
        assert!(
            sanctus
                .to_string()
                .contains("Soggetto aus dem Choral, Phrase ")
        );
        // Errors: no imitation to plan, none without a text, none with given cadences.
        let plain = ask(r#"{"command": "free", "voices": 2, "imitation": {"plan_only": true}}"#);
        assert!(
            plain["error"]
                .as_str()
                .unwrap()
                .starts_with("Dieser Satz imitiert nicht"),
            "{plain}"
        );
        let untexted = ask(r#"{"command": "free", "text": "none", "imitation": {"mode": "an"}}"#);
        assert_eq!(
            untexted["error"],
            "Imitation braucht einen Text: einen Text wählen oder Imitation aus."
        );
        let given = ask(
            r#"{"command": "free", "cadences": "7", "imitation": {"mode": "an"}, "lang": "en"}"#,
        );
        assert_eq!(
            given["error"],
            "Imitation places the internal cadences itself: internal cadences auto, or imitation off."
        );
        // An option the planner refuses, with what the page can change: a restatement of a text
        // sung once.
        let once = ask(
            r#"{"command": "free", "seed": 1, "text": "gloria:1-5", "imitation": {"restate": "an", "plan_only": true}}"#,
        );
        assert_eq!(
            once["error"],
            "Der Text wird nur einmal gesungen; es gibt keinen Punkt zu wiederholen. Abhilfe: Punkt 1 wiederholen auto oder aus."
        );
    }

    /// A check takes the plan that the MusicXML file keeps and lists its points; a plan that does
    /// not fit the notes is left out, with a warning.
    #[test]
    fn imitation_is_checked_as_the_file_plans_it() {
        use fux_core::{Duration, Role, ScoreEntry, ScoreImitation, ScorePoint, Species, Voice};
        use std::sync::Arc;

        let notes = |src: &str| fux_core::parse_lilypond(src, Duration::HALF).unwrap();
        let free = |voice_type, src: &str| {
            Voice::new(voice_type, Role::Counterpoint(Species::Free), notes(src))
        };
        let mut score = Score::new(
            "dorian".parse().unwrap(),
            vec![
                free(VoiceType::Soprano, "r1 a'1 a'2 g'2 f'2 e'2 d'1 r1"),
                free(VoiceType::Alto, "d'1 d'2 c'2 bes2 a2 g1 a1 a1"),
            ],
        );
        let head = |voice, start, src: &[(&str, u32)]| ScoreEntry {
            voice,
            start,
            rest: 0,
            notes: src.iter().map(|&(p, t)| (p.parse().unwrap(), t)).collect(),
        };
        score.imitation = Some(Arc::new(ScoreImitation {
            points: vec![ScorePoint {
                words: 0..7,
                entries: vec![
                    head(1, 0, &[("D4", 8), ("D4", 4), ("C4", 4), ("Bb3", 4)]),
                    head(0, 8, &[("A4", 8), ("A4", 4), ("G4", 4), ("F4", 4)]),
                ],
                goal: None,
            }],
        }));
        let xml = to_musicxml(&score, &MusicXmlOptions::default());
        let check =
            |xml: &str| ask(&serde_json::json!({"command": "check", "musicxml": xml}).to_string());
        let checked = check(&xml);
        assert_eq!(
            checked["summary"][0], "Imitation nach dem Plan der Datei: 1 Punkt",
            "{checked}"
        );
        assert!(
            checked["summary"][1]
                .as_str()
                .unwrap()
                .starts_with("Punkt 1 („Silben 1–7“): Alt T. 1 D4 · Sopran T. 2 A4 (Quinte); "),
            "{checked}"
        );
        // The entry of a voice that the score does not have.
        let spec = score.imitation.as_ref().unwrap().spec();
        let broken = xml.replace(&spec, &spec.replacen(";0@8", ";5@8", 1));
        assert_ne!(broken, xml);
        let warned = check(&broken);
        assert_eq!(warned["ok"], true, "{warned}");
        assert!(
            warned["summary"][0]
                .as_str()
                .unwrap()
                .starts_with("Warnung: Der Imitationsplan der Datei passt nicht zum Satz: "),
            "{warned}"
        );
    }

    #[test]
    fn the_same_seed_gives_the_same_setting() {
        let request = r#"{"command": "cp", "cf": "fux:phrygian", "species": 3, "seed": 7}"#;
        assert_eq!(answer_json(request), answer_json(request));
    }

    #[test]
    fn cantus_check_and_examples() {
        let cantus = ask(
            r#"{"command": "cf", "mode": "hypomixolydisch", "length": 9, "count": 2, "seed": 1, "lang": "en"}"#,
        );
        assert_eq!(cantus["results"].as_array().unwrap().len(), 2, "{cantus}");
        assert!(cantus["summary"][0].as_str().unwrap().contains("9 notes"));
        // In English, under the seed of the request.
        let xml = cantus["results"][1]["musicxml"].as_str().unwrap();
        assert!(xml.contains(">Hypomixolydian, A</credit-words>"), "{xml}");
        assert!(
            xml.contains(">fux-kontrapunkt, seed 1</credit-words>"),
            "{xml}"
        );
        let check = ask(r#"{"command": "check", "cf": "D4 E4", "others": ["S=f'1 b'1"]}"#);
        let violations = check["results"][0]["violations"].as_array().unwrap();
        assert!(
            violations
                .iter()
                .any(|v| v["rule"] == "MEL-TRITONE" && v["forbidden"] == true),
            "{check}"
        );
        let example = ask(r#"{"command": "example", "figure": "5"}"#);
        assert_eq!(example["results"][0]["label"], "Fig. 5");
        assert!(
            example["summary"][0]
                .as_str()
                .unwrap()
                .starts_with("0 Verbot(e)")
        );
        // MusicXML read back from an answer; an example of Fux names no seed.
        let xml = example["results"][0]["musicxml"].as_str().unwrap();
        assert!(
            !xml.contains("<credit-type>composer</credit-type>"),
            "{xml}"
        );
        let request = serde_json::json!({"command": "check", "musicxml": xml});
        let again = ask(&request.to_string());
        assert_eq!(again["results"][0]["cost"], example["results"][0]["cost"]);
    }

    #[test]
    fn profiles_rules_and_errors() {
        let strict = ask(
            r#"{"command": "check", "cf": "D4 F4 D4", "others": ["S=a'1 c'' f''"], "profile": "jeppesen"}"#,
        );
        let rules: Vec<_> = strict["results"][0]["violations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["rule"].as_str().unwrap().to_string())
            .collect();
        assert!(rules.contains(&"MEL-CONSEC-LEAPS".to_string()), "{strict}");
        let off = ask(
            r#"{"command": "check", "cf": "D4 F4 D4", "others": ["S=a'1 c'' f''"], "rules": ["MEL-CONSEC-LEAPS=aus"]}"#,
        );
        assert!(!off.to_string().contains("MEL-CONSEC-LEAPS"));
        for bad in [
            r#"{"command": "nonsense"}"#,
            r#"{"command": "cp", "profile": "palestrina"}"#,
            r#"{"command": "cp", "cf": "X9"}"#,
            r#"{"command": "cp", "rules": ["NO-SUCH=V"]}"#,
            "not json",
        ] {
            let answer = ask(bad);
            assert_eq!(answer["ok"], false, "{bad}");
            assert!(answer["error"].as_str().is_some_and(|e| !e.is_empty()));
        }
    }

    #[test]
    fn humdrum_is_checked_as_free_counterpoint() {
        let kern = "**kern\t**kern\t**kern\n*M4/2\t*M4/2\t*M4/2\n1G\t1d\t1b\n1C\t1e\t1cc\n0G\t0d\t0b\n*-\t*-\t*-\n";
        let request = serde_json::json!({"command": "check", "kern": kern});
        let answer = ask(&request.to_string());
        assert_eq!(answer["ok"], true, "{answer}");
        assert_eq!(answer["results"][0]["lines"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn the_choices() {
        let info = ask(r#"{"command": "info"}"#)["info"].clone();
        assert_eq!(info["cantus"].as_array().unwrap().len(), 8);
        assert_eq!(info["modes"].as_array().unwrap().len(), 24);
        assert_eq!(info["examples"].as_array().unwrap().len(), 123);
        assert_eq!(info["profiles"], serde_json::json!(["fux", "jeppesen"]));
        let voices = info["voices"].as_array().unwrap();
        assert_eq!(voices.len(), 7);
        assert!(
            voices
                .iter()
                .any(|v| v["id"] == "S" && v["name"] == "Sopran")
        );
        // The sections of the Ordinary, Kyrie I sung twice in 16 measures.
        let texts = info["texts"].as_array().unwrap();
        assert_eq!(texts.len(), 17);
        assert_eq!(
            texts[0],
            serde_json::json!({"id": "kyrie", "name": "Kyrie I", "name_de": "Kyrie I",
                "name_en": "Kyrie I", "syllables": 7, "units": 1, "length": 16})
        );
        let gloria = &texts[3];
        assert_eq!(gloria["id"], "gloria");
        assert_eq!([&gloria["units"], &gloria["length"]], [13, 84]);
        let agnus = ask(r#"{"command": "info", "lang": "en"}"#)["info"]["texts"][15].clone();
        assert_eq!(agnus["name"], "Agnus Dei I and II");
        assert_eq!(agnus["name_de"], "Agnus Dei I und II");
        // With e-léi-son, and sung once.
        let kyrie = |request: &str| ask(request)["info"]["texts"][0].clone();
        let three = kyrie(r#"{"command": "info", "eleison": 3}"#);
        assert_eq!([&three["syllables"], &three["length"]], [6, 14]);
        let once = kyrie(r#"{"command": "info", "repeat": "keine"}"#);
        assert_eq!([&once["syllables"], &once["length"]], [7, 9]);
    }

    #[test]
    fn ties_sound_once() {
        let score = ScoreInput {
            cf: Some("D4 F4 E4 D4".into()),
            voices: vec!["S=r2 a'2~ a'2 d''2~ d''2 cis''2 d''1".into()],
            ..ScoreInput::default()
        }
        .score(Lang::De)
        .unwrap();
        let soprano: Vec<(u32, u32)> = sounds(&score)
            .iter()
            .filter(|s| s.voice == 0)
            .map(|s| (s.start, s.end))
            .collect();
        assert_eq!(soprano, [(4, 12), (12, 20), (20, 24), (24, 32)]);
    }
}
