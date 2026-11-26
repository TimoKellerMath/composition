//! Imitation in free settings (phase 13, docs/regeln.md § 18): the options of `fux free` that ask
//! for it and shape its points, the request they make, lines describing the points of a plan and
//! of a setting, and the plan or the points recognised that a check of a score takes.

use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use fux_core::gabc::{Chant, parse_gabc};
use fux_core::generate::{FreeRequest, GenerateError};
use fux_core::imitation::chant::Kyriale;
use fux_core::imitation::detect::detect;
use fux_core::imitation::planner::plan_imitation;
use fux_core::imitation::points::spelled;
use fux_core::imitation::{
    AnswerChoice, Imitation, ImitationPlan, OptionError, Origin, PointKind, Relation,
    SubjectSource, Toggle,
};
use fux_core::text::latin::strip_acute;
use fux_core::text::underlay::Choice;
use fux_core::text::{OWN_ID, Text};
use fux_core::{
    Cadence, Duration, Lang, Meter, ModeSpec, Note, Pitch, Score, ScoreEntry, ScoreError,
    ScoreImitation, VoiceType, parse_notes,
};

use crate::free::PreparedFree;

/// Whether a free setting imitates (`--imitation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImitationMode {
    /// Where every voice sings a text, with three to five voices, the length and the internal
    /// cadences the text plans.
    #[default]
    Auto,
    On,
    Off,
}

impl FromStr for ImitationMode {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.trim().to_lowercase().as_str() {
            "auto" => Ok(ImitationMode::Auto),
            "an" | "on" => Ok(ImitationMode::On),
            "aus" | "off" => Ok(ImitationMode::Off),
            _ => Err(format!("{text:?}: auto, an (on) or aus (off)")),
        }
    }
}

/// A choice the planner makes unless it is given (`--restate`, `--pairs`): `auto`, `an` (`on`)
/// or `aus` (`off`).
pub fn toggle(text: &str) -> Result<Toggle, String> {
    match text.trim().to_lowercase().as_str() {
        "auto" => Ok(Toggle::Auto),
        "an" | "on" => Ok(Toggle::On),
        "aus" | "off" => Ok(Toggle::Off),
        _ => Err(format!("{text:?}: auto, an (on) or aus (off)")),
    }
}

/// How the entries answer the leader (`--answer`): `auto`, `real` or `modal`.
pub fn answer_choice(text: &str) -> Result<AnswerChoice, String> {
    match text.trim().to_lowercase().as_str() {
        "auto" => Ok(AnswerChoice::Auto),
        "real" => Ok(AnswerChoice::Real),
        "modal" => Ok(AnswerChoice::Modal),
        _ => Err(format!("{text:?}: auto, real or modal")),
    }
}

/// A number, or `auto` for the planner's choice (`--points`, `--head`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoCount(pub Option<usize>);

impl FromStr for AutoCount {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.trim() {
            "auto" | "" => Ok(AutoCount(None)),
            number => number
                .parse::<usize>()
                .ok()
                .filter(|&n| n >= 1)
                .map(|n| AutoCount(Some(n)))
                .ok_or_else(|| format!("{text:?}: auto or a number from 1")),
        }
    }
}

/// The notes of every head (`--head`): `auto` or a number from 4.
pub fn head_notes(text: &str) -> Result<AutoCount, String> {
    match text.parse::<AutoCount>()? {
        AutoCount(Some(n)) if n < 4 => Err(format!("{text:?}: auto or a number from 4")),
        count => Ok(count),
    }
}

/// The voices of each point after the first (`--point-voices`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PointVoices {
    /// Two to all, by the shares of Palestrina's later points.
    #[default]
    Auto,
    /// Every voice.
    All,
    /// This many, two at least.
    Count(usize),
}

impl FromStr for PointVoices {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.trim().to_lowercase().as_str() {
            "auto" | "" => Ok(PointVoices::Auto),
            "alle" | "all" => Ok(PointVoices::All),
            number => number
                .parse::<usize>()
                .ok()
                .filter(|&n| n >= 2)
                .map(PointVoices::Count)
                .ok_or_else(|| format!("{text:?}: auto, alle (all) or a number from 2")),
        }
    }
}

/// A voice as `--order` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceRef {
    /// By its type (`T`), where several voices have it with their number among them from the
    /// top (`T1`, `T2`).
    Type(VoiceType, Option<usize>),
    /// By its number from the top (`1`).
    Number(usize),
}

impl FromStr for VoiceRef {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let error = || format!("{text:?}: a voice such as S, A, T1, B or its number from the top");
        if let Ok(number) = text.parse::<usize>() {
            return (number >= 1)
                .then_some(VoiceRef::Number(number))
                .ok_or_else(error);
        }
        let digits = text.len() - text.trim_end_matches(|c: char| c.is_ascii_digit()).len();
        if digits > 0 && digits < text.len() {
            let (name, number) = text.split_at(text.len() - digits);
            if let (Ok(voice_type), Ok(number)) = (name.parse::<VoiceType>(), number.parse()) {
                return (number >= 1)
                    .then_some(VoiceRef::Type(voice_type, Some(number)))
                    .ok_or_else(error);
            }
        }
        text.parse::<VoiceType>()
            .map(|voice_type| VoiceRef::Type(voice_type, None))
            .map_err(|_| error())
    }
}

/// Splits a list of points at `;`: one group per point, `None` for an empty group or `auto`,
/// which leave the point to the planner.
fn groups<T>(
    text: &str,
    item: impl Fn(&str) -> Result<T, String>,
    separators: &[char],
) -> Result<Vec<Option<Vec<T>>>, String> {
    if text.trim().is_empty() || text.trim().eq_ignore_ascii_case("auto") {
        return Ok(Vec::new());
    }
    text.split(';')
        .map(|group| {
            let group = group.trim();
            if group.is_empty() || group.eq_ignore_ascii_case("auto") {
                return Ok(None);
            }
            group
                .split(|c: char| c.is_whitespace() || separators.contains(&c))
                .filter(|t| !t.is_empty())
                .map(&item)
                .collect::<Result<Vec<T>, String>>()
                .map(Some)
        })
        .collect()
}

/// The order of the entries of each point (`--order "A S T B; T B A"`): per point its voices,
/// the leader first, the points separated by `;`. `auto` or nothing leaves a point's order to
/// the planner; the first point's voices not named follow in an order the planner draws.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Orders(pub Vec<Option<Vec<VoiceRef>>>);

impl FromStr for Orders {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        groups(text, str::parse::<VoiceRef>, &[',']).map(Orders)
    }
}

impl Orders {
    /// The orders as voice indices of `choir`, the voices from the top. A voice named twice or
    /// a later point of one voice is the planner's to refuse ([`OptionError`]), in each
    /// frontend's words.
    pub fn resolve(&self, choir: &[VoiceType], lang: Lang) -> Result<Vec<Option<Vec<usize>>>> {
        self.0
            .iter()
            .map(|group| {
                let Some(group) = group else {
                    return Ok(None);
                };
                let order = group
                    .iter()
                    .map(|&voice| voice_index(voice, choir))
                    .collect::<Result<Vec<usize>, ChoiceError>>()
                    .map_err(|error| ChoiceFailure { error, lang })?;
                Ok(Some(order))
            })
            .collect()
    }
}

/// How a frontend names the choices of an imitation in its messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terms {
    /// The command line: `--order`, `--subject`.
    Cli,
    /// The window: the fields of the group „Imitation“.
    Window,
    /// The web version: the fields of the page.
    Page,
}

/// A choice of the imitation that the choir or the text cannot have, found before the planner
/// ([`OptionError`] are the planner's): each frontend words it with a remedy
/// ([`ChoiceError::message`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChoiceError {
    /// A voice by its number from the top that the choir does not have.
    NoNumber { number: usize, voices: usize },
    /// A voice type the parts do not have.
    NoType { name: String },
    /// A voice type that several voices have, named without its number among them.
    Ambiguous { name: String, count: usize },
    /// A numbered voice type beyond those the parts have.
    NoNumbered {
        name: String,
        number: usize,
        count: usize,
    },
    /// No chant of the Kyriale sings the text (its name, or `None` for an own text).
    NoChant { text: Option<String> },
}

impl ChoiceError {
    /// The message with its remedy, in the terms of a frontend.
    pub fn message(&self, lang: Lang, terms: Terms) -> String {
        let order = match (terms, lang) {
            (Terms::Cli, _) => "--order",
            (_, Lang::De) => "Reihenfolge",
            (_, Lang::En) => "Order",
        };
        let order_remedy = match (terms, lang) {
            (Terms::Cli, Lang::De) => "Abhilfe: eine andere --order oder --order auto.",
            (Terms::Cli, Lang::En) => "Remedies: another --order or --order auto.",
            (_, Lang::De) => "Abhilfe: eine andere Reihenfolge, oder keine.",
            (_, Lang::En) => "Remedies: another order, or none.",
        };
        let problem = match (self, lang) {
            (ChoiceError::NoNumber { number, voices }, Lang::De) => {
                format!("{order}: Stimme {number} gibt es nicht; der Satz hat {voices}.")
            }
            (ChoiceError::NoNumber { number, voices }, Lang::En) => {
                format!("{order}: there is no voice {number}; the setting has {voices}.")
            }
            (ChoiceError::NoType { name }, Lang::De) => {
                format!("{order}: Die Besetzung hat keine Stimme {name}.")
            }
            (ChoiceError::NoType { name }, Lang::En) => {
                format!("{order}: the parts have no voice {name}.")
            }
            (ChoiceError::Ambiguous { name, count }, Lang::De) => format!(
                "{order}: {name} gibt es {count}-mal; {name}1 bis {name}{count} nennen sie von oben."
            ),
            (ChoiceError::Ambiguous { name, count }, Lang::En) => format!(
                "{order}: there are {count} voices {name}; {name}1 to {name}{count} name them from the top."
            ),
            (
                ChoiceError::NoNumbered {
                    name,
                    number,
                    count,
                },
                Lang::De,
            ) => format!("{order}: {name}{number} gibt es nicht, nur {name}1 bis {name}{count}."),
            (
                ChoiceError::NoNumbered {
                    name,
                    number,
                    count,
                },
                Lang::En,
            ) => format!("{order}: there is no {name}{number}, only {name}1 to {name}{count}."),
            (ChoiceError::NoChant { text }, _) => {
                let text = match (text, lang) {
                    (Some(name), _) => name.clone(),
                    (None, Lang::De) => "den eigenen Text".to_string(),
                    (None, Lang::En) => "an own text".to_string(),
                };
                let remedy = match (terms, lang) {
                    (Terms::Cli, Lang::De) => {
                        "Abhilfe: --subject chant:DATEI mit einer GABC-Datei, ein Choral des Kyriale wie chant:kyrie-iv, oder --subject text."
                    }
                    (Terms::Cli, Lang::En) => {
                        "Remedies: --subject chant:FILE with a GABC file, a chant of the Kyriale such as chant:kyrie-iv, or --subject text."
                    }
                    (Terms::Window, Lang::De) => {
                        "Abhilfe: Soggetto „Choral aus Datei“ oder „aus dem Text“."
                    }
                    (Terms::Window, Lang::En) => {
                        "Remedies: subject “chant from a file” or “from the text”."
                    }
                    (Terms::Page, Lang::De) => {
                        "Abhilfe: einen Choral als GABC eingeben, oder das Soggetto aus dem Text."
                    }
                    (Terms::Page, Lang::En) => {
                        "Remedies: a chant entered as GABC, or the subject from the text."
                    }
                };
                return match lang {
                    Lang::De => format!("Für {text} hat das Kyriale keinen Choral. {remedy}"),
                    Lang::En => format!("The Kyriale has no chant for {text}. {remedy}"),
                };
            }
        };
        format!("{problem} {order_remedy}")
    }
}

/// A [`ChoiceError`] as an error of the command line, in the user's language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceFailure {
    pub error: ChoiceError,
    pub lang: Lang,
}

impl std::fmt::Display for ChoiceFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.error.message(self.lang, Terms::Cli))
    }
}

impl std::error::Error for ChoiceFailure {}

/// The message of an error of `fux free`'s preparation in the terms of a window or page: a
/// [`ChoiceFailure`] in their words, any other as it is.
pub fn form_message(error: &anyhow::Error, lang: Lang, terms: Terms) -> String {
    match error.downcast_ref::<ChoiceFailure>() {
        Some(failure) => failure.error.message(lang, terms),
        None => format!("{error:#}"),
    }
}

/// The index in `choir` of the voice `voice` names.
fn voice_index(voice: VoiceRef, choir: &[VoiceType]) -> Result<usize, ChoiceError> {
    let n = choir.len();
    match voice {
        VoiceRef::Number(number) if number <= n => Ok(number - 1),
        VoiceRef::Number(number) => Err(ChoiceError::NoNumber { number, voices: n }),
        VoiceRef::Type(voice_type, number) => {
            let alike: Vec<usize> = (0..n).filter(|&v| choir[v] == voice_type).collect();
            let name = voice_type.abbreviation().to_string();
            match (number, alike.as_slice()) {
                (None, [single]) => Ok(*single),
                (Some(k), alike) if k >= 1 && k <= alike.len() => Ok(alike[k - 1]),
                (_, []) => Err(ChoiceError::NoType { name }),
                (None, alike) => Err(ChoiceError::Ambiguous {
                    name,
                    count: alike.len(),
                }),
                (Some(k), alike) => Err(ChoiceError::NoNumbered {
                    name,
                    number: k,
                    count: alike.len(),
                }),
            }
        }
    }
}

/// The interval of each entry after the first to the leader's, per point (`--intervals "d 0 d;
/// 0"`): `0` the same degree, at the unison or an octave; `d` the other main degree of the mode,
/// the fifth against the final and back, the final against the fourth; `+n` or `-n` another
/// distance in diatonic steps (`+2` a third up). Points are separated by `;`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Intervals(pub Vec<Option<Vec<Relation>>>);

impl FromStr for Intervals {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let relation = |item: &str| -> Result<Relation, String> {
            let error = || format!("{item:?}: 0, d or a signed number of steps such as +2 or -3");
            match item.to_lowercase().as_str() {
                "0" => Ok(Relation::Same),
                "d" => Ok(Relation::Dominant),
                signed if signed.starts_with(['+', '-']) => {
                    let steps: i8 = signed
                        .trim_start_matches('+')
                        .parse()
                        .map_err(|_| error())?;
                    Ok(if steps % 7 == 0 {
                        Relation::Same
                    } else {
                        Relation::Steps(steps)
                    })
                }
                _ => Err(error()),
            }
        };
        groups(text, relation, &[',']).map(Intervals)
    }
}

/// The delays between the entries of each point in measures, halves allowed (`--delays "1 2
/// 1.5; 1"`): from each entry to the next, the points separated by `;`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Delays(pub Vec<Option<Vec<u32>>>);

impl FromStr for Delays {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let delay = |item: &str| -> Result<u32, String> {
            // A comma would be a decimal comma or a separator: neither is guessed.
            if item.contains(',') {
                return Err(format!(
                    "{item:?}: a comma is ambiguous; write 1.5 for one delay of a measure and a \
                     half, or 1 5 for two delays"
                ));
            }
            let measures: f64 = item
                .parse()
                .map_err(|_| format!("{item:?}: measures such as 1, 2 or 1.5"))?;
            let halves = measures * 2.0;
            if halves < 1.0 || halves.fract() != 0.0 || halves > 64.0 {
                return Err(format!(
                    "{item:?}: half a measure to 32 measures, in halves such as 1.5"
                ));
            }
            Ok(halves as u32 * Duration::HALF.ticks())
        };
        // The delays are separated by spaces.
        groups(text, delay, &[]).map(Delays)
    }
}

/// Which chant gives the subjects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChantRef {
    /// The chant of the Kyriale that sings the section of the Ordinary (Mass IX, Credo I).
    Kyriale,
    /// A chant of the Kyriale by its key (`kyrie-ix`, `kyrie-iv`) or a GABC file.
    Named(String),
    /// A chant as GABC text, as the web version, which has no files, gives it.
    Gabc(String),
}

/// Where the subjects come from (`--subject`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SubjectChoice {
    /// Built from the words of each point.
    #[default]
    Text,
    /// The notes of a chant's phrases, found by the words unless the phrases (from 1) are named.
    Chant {
        chant: ChantRef,
        phrases: Option<Vec<usize>>,
    },
    /// Heads given as notes, as note names or in LilyPond, for the first points in order; the
    /// later points build theirs from the text.
    Notes(Vec<Vec<Note>>),
}

/// Phrases as `chant:DATEI:2,3` names them, from 1: numbers and ranges such as `2-4`.
pub fn phrase_list(text: &str) -> Option<Vec<usize>> {
    let mut phrases = Vec::new();
    for item in text.split(',').map(str::trim) {
        match item.split_once('-') {
            Some((from, to)) => {
                let (from, to): (usize, usize) =
                    (from.trim().parse().ok()?, to.trim().parse().ok()?);
                if from < 1 || to < from {
                    return None;
                }
                phrases.extend(from..=to);
            }
            None => phrases.push(item.parse().ok().filter(|&n| n >= 1)?),
        }
    }
    (!phrases.is_empty()).then_some(phrases)
}

impl FromStr for SubjectChoice {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let trimmed = text.trim();
        let lower = trimmed.to_lowercase();
        if lower.is_empty() || lower == "text" {
            return Ok(SubjectChoice::Text);
        }
        if lower == "chant" || lower == "choral" {
            return Ok(SubjectChoice::Chant {
                chant: ChantRef::Kyriale,
                phrases: None,
            });
        }
        let named = ["chant:", "choral:"]
            .iter()
            .find(|prefix| lower.starts_with(*prefix))
            .map(|prefix| &trimmed[prefix.len()..]);
        if let Some(rest) = named {
            let (name, phrases) = match rest.rsplit_once(':') {
                Some((name, list)) => match phrase_list(list) {
                    Some(phrases) => (name, Some(phrases)),
                    None if list.trim().is_empty() => (name, None),
                    None => (rest, None),
                },
                None => (rest, None),
            };
            let chant = match name.trim() {
                "" => ChantRef::Kyriale,
                name => ChantRef::Named(name.to_string()),
            };
            return Ok(SubjectChoice::Chant { chant, phrases });
        }
        // As the other note options: note names (wholes) or LilyPond.
        let heads = trimmed
            .split('|')
            .map(|head| {
                parse_notes(head, Duration::WHOLE, Meter::ALLA_BREVE)
                    .map_err(|e| format!("{head:?}: {e}"))
                    .and_then(|notes| {
                        if notes.is_empty() {
                            Err(format!(
                                "{trimmed:?}: text, chant, chant:FILE[:PHRASES] or notes such as \"d'1 f'2 e'2 d'1\""
                            ))
                        } else {
                            Ok(notes)
                        }
                    })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(SubjectChoice::Notes(heads))
    }
}

/// The imitation of `fux free`: whether it imitates and what it asks the planner for, as the
/// window and the web version also give them. Whatever it leaves at `auto` the planner chooses.
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct ImitationOptions {
    /// Imitation: auto (an mit Text, 3–5 Stimmen, automatischen Binnenschlüssen und automatischer
    /// Länge), an, aus
    #[arg(
        id = "imitation",
        long = "imitation",
        value_name = "AUTO|AN|AUS",
        default_value = "auto"
    )]
    pub mode: ImitationMode,
    /// Soggetto: text (aus den Worten), chant (Choral des Kyriale zum Abschnitt; auch choral),
    /// chant:DATEI oder chant:DATEI:PHRASEN (GABC-Datei oder kyrie-ix, kyrie-iv …; Phrasen ab 1,
    /// z. B. 2,3 oder 2-4; chant::2,3 mit dem Choral des Kyriale) oder eigene Noten wie
    /// "d'1 f'2 e'2 d'1" oder "D4 F4 E4 D4" (Notennamen als Ganze), | zwischen den Köpfen
    #[arg(long, value_name = "QUELLE", default_value = "text")]
    pub subject: SubjectChoice,
    /// Anzahl der Imitationspunkte: auto (einer je Textteil) oder eine Zahl
    #[arg(long, value_name = "AUTO|N", default_value = "auto")]
    pub points: AutoCount,
    /// Den zweiten Durchgang eines kurzen Textes als Wiederholung von Punkt 1 in engerer
    /// Engführung statt mit eigenem Kopf auf dem Schlussteil: auto, an, aus
    #[arg(long, value_name = "AUTO|AN|AUS", default_value = "auto", value_parser = toggle)]
    pub restate: Toggle,
    /// Stimmen der Punkte nach dem ersten: auto (zwei bis alle, meist zwei, wie bei Palestrina),
    /// alle oder eine Zahl
    #[arg(long, value_name = "AUTO|ALLE|N", default_value = "auto")]
    pub point_voices: PointVoices,
    /// Reihenfolge der Einsätze, der Führer zuerst, je Punkt, getrennt durch ;, z. B.
    /// "A S T B; T B" (gleiche Stimmlagen als T1, T2 von oben, oder Nummern von oben; in Punkt 1
    /// folgen nicht genannte Stimmen in gezogener Folge)
    #[arg(long, value_name = "STIMMEN", default_value = "auto")]
    pub order: Orders,
    /// Intervalle der späteren Einsätze zum Führer je Punkt: 0 gleiche Stufe, d andere
    /// Hauptstufe (Quinte oder Quarte), +n/-n Stufen, z. B. "d 0 d; 0" oder "-3 d"
    #[arg(
        long,
        value_name = "INTERVALLE",
        default_value = "auto",
        allow_hyphen_values = true
    )]
    pub intervals: Intervals,
    /// Abstände der Einsätze je Punkt in Takten, Halbe mit Punkt (1.5), durch Leerzeichen
    /// getrennt, z. B. "1 2 1.5; 1"
    #[arg(long, value_name = "TAKTE", default_value = "auto")]
    pub delays: Delays,
    /// Ab vier Stimmen paarweise Einsätze (der 3. und 4. wiederholen den 1. und 2. eine Oktave
    /// tiefer oder höher): auto, an, aus
    #[arg(long, value_name = "AUTO|AN|AUS", default_value = "auto", value_parser = toggle)]
    pub pairs: Toggle,
    /// Antwort: auto (real, bisweilen modal), real oder modal (Quarte und Quinte getauscht)
    #[arg(long, value_name = "WIE", default_value = "auto", value_parser = answer_choice)]
    pub answer: AnswerChoice,
    /// Töne jedes Kopfes: auto oder eine Zahl ab 4
    #[arg(long, value_name = "AUTO|N", default_value = "auto", value_parser = head_notes)]
    pub head: AutoCount,
    /// Nur den Plan der Imitation zeigen, ohne Suche: Punkte, Köpfe und Einsätze
    #[arg(long = "plan")]
    pub plan_only: bool,
}

impl ImitationOptions {
    /// Whether a free setting imitates: with `an` always, with `aus` never, with `auto` where
    /// every voice sings a text, with three to five voices, the internal cadences that the text
    /// plans (imitation places them itself, at the ends of its points) and the length the plan
    /// gives: a given length keeps a setting of phase 12, as `--imitation an` does not.
    pub fn applies(
        &self,
        texted: bool,
        voices: usize,
        auto_cadences: bool,
        auto_length: bool,
    ) -> bool {
        match self.mode {
            ImitationMode::On => true,
            ImitationMode::Off => false,
            ImitationMode::Auto => {
                texted && (3..=5).contains(&voices) && auto_cadences && auto_length
            }
        }
    }

    /// Whether a choice besides `--imitation` and `--plan` is made.
    pub fn shapes(&self) -> bool {
        *self
            != ImitationOptions {
                mode: self.mode,
                plan_only: self.plan_only,
                ..ImitationOptions::default()
            }
    }

    /// The chant whose phrases give the subjects, for a setting of `text`; `None` for subjects
    /// from the words or from notes. The chant of the Kyriale is the one of the text's section; a
    /// named one is a chant of the Kyriale by its key or a GABC file.
    pub fn chant(&self, text: &Text, lang: Lang) -> Result<Option<Arc<Chant>>> {
        let SubjectChoice::Chant { chant, .. } = &self.subject else {
            return Ok(None);
        };
        let chant = match chant {
            ChantRef::Kyriale => Kyriale::for_section(&text.id)
                .map(Kyriale::chant)
                .ok_or_else(|| ChoiceFailure {
                    error: ChoiceError::NoChant {
                        text: (text.id != OWN_ID).then(|| text.name(lang).to_string()),
                    },
                    lang,
                })?,
            ChantRef::Named(name) => match Kyriale::from_key(name.trim()) {
                Some(kyriale) => kyriale.chant(),
                None => {
                    let gabc = std::fs::read_to_string(name).map_err(|_| {
                        let keys: Vec<&str> = Kyriale::ALL.iter().map(|k| k.key()).collect();
                        anyhow!(
                            "{name}: {}; {} {}",
                            lang.pick("nicht lesbar", "not readable"),
                            lang.pick(
                                "die Choräle des Kyriale sind",
                                "the chants of the Kyriale are"
                            ),
                            keys.join(", ")
                        )
                    })?;
                    Arc::new(parse_gabc(&gabc).map_err(|e| anyhow!("{name}: {e}"))?)
                }
            },
            ChantRef::Gabc(gabc) => Arc::new(parse_gabc(gabc).map_err(|e| anyhow!("GABC: {e}"))?),
        };
        Ok(Some(chant))
    }

    /// What the request asks the planner for, for `choir` (the voices from the top) singing
    /// `text`, with the subjects from `chant` where they come from one ([`Self::chant`]).
    pub fn imitation(
        &self,
        text: &Text,
        chant: Option<Arc<Chant>>,
        choir: &[VoiceType],
        lang: Lang,
    ) -> Result<Imitation> {
        let n = choir.len();
        let subject = match (&self.subject, chant) {
            (SubjectChoice::Text, _) => SubjectSource::Text,
            (SubjectChoice::Chant { phrases, .. }, Some(chant)) => SubjectSource::Chant {
                chant,
                phrases: phrases
                    .as_ref()
                    .map(|list| list.iter().map(|&p| p - 1).collect()),
            },
            (SubjectChoice::Chant { .. }, None) => match self.chant(text, lang)? {
                Some(chant) => return self.imitation(text, Some(chant), choir, lang),
                None => SubjectSource::Text,
            },
            (SubjectChoice::Notes(heads), _) => SubjectSource::Given(heads.clone()),
        };
        // A count the choir cannot give is the planner's to refuse (`OptionError::LaterVoices`).
        let later_voices = match self.point_voices {
            PointVoices::Auto => None,
            PointVoices::All => Some(n),
            PointVoices::Count(count) => Some(count),
        };
        Ok(Imitation {
            subject,
            points: self.points.0,
            later_voices,
            restate: self.restate,
            orders: self.order.resolve(choir, lang)?,
            relations: self.intervals.0.clone(),
            delays: self.delays.0.clone(),
            pairs: self.pairs,
            answer: self.answer,
            head: self.head.0,
        })
    }
}

/// The mode of a setting whose subjects come from `chant`: the mode its number names
/// ([`Chant::numbered_mode`]), in which the chant's phrases end on the final.
pub fn chant_mode(chant: &Chant) -> Option<ModeSpec> {
    chant.numbered_mode().ok()
}

/// What the lines on the points of a plan or a setting need besides the points: the mode, the
/// text, the names of the voices, the cadences, the ending and the length.
#[derive(Clone, Debug)]
pub struct Setting<'a> {
    pub mode: ModeSpec,
    pub text: Option<&'a Text>,
    /// The words keep the acutes of the liturgical books (`--accents`).
    pub accents: bool,
    /// The names of the voices from the top, as the summary shows them.
    pub names: Vec<String>,
    pub cadences: &'a [Cadence],
    /// The degree of the final cadence, which ends the last point: the final of the mode, or the
    /// ending planned for a part (phase 14).
    pub ending: fux_core::Step,
    pub measures: usize,
    pub ticks_per_measure: u32,
    /// The measures before the setting where it is a part of a larger whole, such as a part of a
    /// mass in the file of its movement (phase 14): the lines count its measures on from there.
    /// 0 for a setting on its own.
    pub before: usize,
}

impl<'a> Setting<'a> {
    /// The setting of a score, ending on the final of its mode.
    pub fn of(score: &'a Score, lang: Lang) -> Setting<'a> {
        Setting {
            mode: score.mode,
            text: score.text.as_ref().map(|sung| sung.text.as_ref()),
            accents: true,
            names: voice_names(
                score.voices.iter().map(|v| (v.voice_type, v.name.as_str())),
                lang,
            ),
            cadences: &score.cadences,
            ending: score.mode.final_step(),
            measures: score.measure_count(),
            ticks_per_measure: score.meter.ticks_per_measure(),
            before: 0,
        }
    }

    /// Where a tick of the setting lies, counted in the measures of the whole it belongs to
    /// ([`Setting::before`]): „T. 2“, „T. 2½“.
    fn position(&self, tick: u32, lang: Lang) -> String {
        let before = u32::try_from(self.before).unwrap_or(u32::MAX);
        let tick = tick.saturating_add(before.saturating_mul(self.ticks_per_measure));
        position(tick, self.ticks_per_measure, lang)
    }
}

/// The names of voices as the summary shows them: a voice named after its type by default gets
/// its type's name in `lang`, numbered from the top among voices of one type (`Tenor 1`), as
/// [`crate::commands::localize_names`] names them; others keep their names.
pub fn voice_names<'n>(
    voices: impl Iterator<Item = (VoiceType, &'n str)>,
    lang: Lang,
) -> Vec<String> {
    let voices: Vec<(VoiceType, &str)> = voices.collect();
    let mut seen: Vec<VoiceType> = Vec::new();
    voices
        .iter()
        .map(|&(voice_type, name)| {
            if name != voice_type.name(Lang::De) {
                return name.to_string();
            }
            let alike = voices.iter().filter(|(t, _)| *t == voice_type).count();
            seen.push(voice_type);
            let localized = voice_type.name(lang);
            if alike > 1 {
                let number = seen.iter().filter(|t| **t == voice_type).count();
                format!("{localized} {number}")
            } else {
                localized.to_string()
            }
        })
        .collect()
}

/// The names of the voices of a choir from the top, as [`voice_names`] gives them.
pub fn choir_names(choir: &[VoiceType], lang: Lang) -> Vec<String> {
    voice_names(choir.iter().map(|&t| (t, t.name(Lang::De))), lang)
}

/// Where an entry begins: „T. 2“, on the minim beat „T. 2½“, elsewhere with the quarter.
fn position(tick: u32, per_measure: u32, lang: Lang) -> String {
    let measure = tick / per_measure + 1;
    let within = tick % per_measure;
    let prefix = lang.pick("T.", "m.");
    match within {
        0 => format!("{prefix} {measure}"),
        w if 2 * w == per_measure => format!("{prefix} {measure}½"),
        w => match lang {
            Lang::De => format!("{prefix} {measure}, Viertel {}", w / 2 + 1),
            Lang::En => format!("{prefix} {measure}, beat {}", w / 2 + 1),
        },
    }
}

/// The interval from a leader's first note to an entry's: „Quinte“, „Unterquarte“, „Einklang“;
/// "fifth above", "fourth below", "unison".
pub fn interval_name(leader: Pitch, entry: Pitch, lang: Lang) -> String {
    const DE: [&str; 15] = [
        "Einklang",
        "Sekunde",
        "Terz",
        "Quarte",
        "Quinte",
        "Sexte",
        "Septime",
        "Oktave",
        "None",
        "Dezime",
        "Undezime",
        "Duodezime",
        "Tredezime",
        "Quartdezime",
        "Doppeloktave",
    ];
    const EN: [&str; 15] = [
        "unison",
        "second",
        "third",
        "fourth",
        "fifth",
        "sixth",
        "seventh",
        "octave",
        "ninth",
        "tenth",
        "eleventh",
        "twelfth",
        "thirteenth",
        "fourteenth",
        "double octave",
    ];
    let steps = entry.diatonic() - leader.diatonic();
    let size = steps.unsigned_abs() as usize;
    let Some(name) = (match lang {
        Lang::De => DE.get(size),
        Lang::En => EN.get(size),
    }) else {
        return match (lang, steps > 0) {
            (Lang::De, true) => format!("{size} Stufen höher"),
            (Lang::De, false) => format!("{size} Stufen tiefer"),
            (Lang::En, true) => format!("{size} steps above"),
            (Lang::En, false) => format!("{size} steps below"),
        };
    };
    match (steps.signum(), lang) {
        (0, _) | (1, Lang::De) => name.to_string(),
        (_, Lang::De) => format!("Unter{}", name.to_lowercase()),
        (1, Lang::En) => format!("{name} above"),
        (_, Lang::En) => format!("{name} below"),
    }
}

/// How an entry takes up the leader's head, by its diatonic intervals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Answered {
    Real,
    /// A fourth and a fifth exchanged in the first two intervals.
    Modal,
    /// Another change.
    Changed,
}

/// How `entry` answers `leader`: with the same intervals, with a fourth for a fifth or a fifth
/// for a fourth in its first two (a modal answer), or otherwise changed.
fn answered(leader: &ScoreEntry, entry: &ScoreEntry) -> Answered {
    let steps = |e: &ScoreEntry| -> Vec<i32> {
        e.notes
            .windows(2)
            .map(|w| w[1].0.diatonic() - w[0].0.diatonic())
            .collect()
    };
    let (theirs, ours) = (steps(leader), steps(entry));
    match theirs.iter().zip(&ours).position(|(a, b)| a != b) {
        None => Answered::Real,
        Some(k)
            if k < 2
                && theirs[k].signum() == ours[k].signum()
                && matches!((theirs[k].abs(), ours[k].abs()), (3, 4) | (4, 3)) =>
        {
            Answered::Modal
        }
        Some(_) => Answered::Changed,
    }
}

/// Words in quotes: „…“, "…".
fn quoted(words: &str, lang: Lang) -> String {
    match lang {
        Lang::De => format!("„{words}“"),
        Lang::En => format!("\"{words}\""),
    }
}

/// The name of a cadence's target, as the summaries write it.
fn target_name(mode: ModeSpec, target: fux_core::Step, lang: Lang) -> String {
    Pitch::new(target, mode.scale_alter(target), 4).class_name(lang)
}

/// One line per point of `plan`: its words, its entries (voice, measure, first note, interval to
/// the leader), its head, its answers and the cadence that ends it: „Punkt 1 („Ký-ri-e
/// e-lé-i-son.“): Alt T. 1 D4 · Sopran T. 2 A4 (Quinte) · …; Kopf 6 Töne, real; Binnenschluss
/// T. 12 auf A“.
pub fn point_lines(plan: &ScoreImitation, setting: &Setting, lang: Lang) -> Vec<String> {
    plan.points
        .iter()
        .enumerate()
        .map(|(p, point)| {
            let words = match setting.text {
                Some(text) if usize::from(point.words.end) <= text.syllables.len() => {
                    let words = spelled(text, &point.words);
                    if setting.accents {
                        words
                    } else {
                        strip_acute(&words)
                    }
                }
                _ => match lang {
                    Lang::De => format!("Silben {}–{}", point.words.start + 1, point.words.end),
                    Lang::En => format!("syllables {}–{}", point.words.start + 1, point.words.end),
                },
            };
            let leader = point.entries.first();
            let entries: Vec<String> = point
                .entries
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let name = setting
                        .names
                        .get(entry.voice)
                        .cloned()
                        .unwrap_or_else(|| (entry.voice + 1).to_string());
                    let at = setting.position(entry.start, lang);
                    let first = entry.notes.first().map(|(pitch, _)| *pitch);
                    let mut item = format!("{name} {at}");
                    if let Some(first) = first {
                        item.push_str(&format!(" {}", first.name(lang)));
                    }
                    let leader_first = leader.and_then(|l| l.notes.first()).map(|(p, _)| *p);
                    if let (true, Some(leader), Some(first)) = (i > 0, leader_first, first) {
                        item.push_str(&format!(" ({})", interval_name(leader, first, lang)));
                    }
                    item
                })
                .collect();
            let counts: Vec<usize> = point.entries.iter().map(|e| e.notes.len()).collect();
            let (least, most) = (
                counts.iter().copied().min().unwrap_or(0),
                counts.iter().copied().max().unwrap_or(0),
            );
            let notes = if least == most {
                most.to_string()
            } else {
                format!("{least}–{most}")
            };
            let mut kinds: Vec<Answered> = match leader {
                Some(leader) => point.entries[1..]
                    .iter()
                    .map(|e| answered(leader, e))
                    .collect(),
                None => Vec::new(),
            };
            kinds.sort();
            kinds.dedup();
            let kinds: Vec<&str> = kinds
                .iter()
                .map(|kind| match kind {
                    Answered::Real => "real",
                    Answered::Modal => "modal",
                    Answered::Changed => lang.pick("verändert", "changed"),
                })
                .collect();
            let answers = match kinds.as_slice() {
                [] => "real".to_string(),
                [one] => one.to_string(),
                [rest @ .., last] => {
                    format!("{} {} {last}", rest.join(", "), lang.pick("und", "and"))
                }
            };
            let head = match lang {
                Lang::De => format!("Kopf {notes} Töne, {answers}"),
                Lang::En => format!("head {notes} notes, {answers}"),
            };
            let ending = match point.goal {
                Some(goal) => {
                    let target = setting
                        .cadences
                        .iter()
                        .find(|c| c.measure == goal)
                        .map(|c| target_name(setting.mode, c.target, lang));
                    let at = format!("{} {}", lang.pick("T.", "m."), setting.before + goal + 1);
                    match (target, lang) {
                        (Some(target), Lang::De) => format!("Binnenschluss {at} auf {target}"),
                        (Some(target), Lang::En) => format!("internal cadence {at} on {target}"),
                        (None, Lang::De) => format!("Binnenschluss {at}"),
                        (None, Lang::En) => format!("internal cadence {at}"),
                    }
                }
                None => {
                    let final_name = target_name(setting.mode, setting.ending, lang);
                    let at = format!(
                        "{} {}",
                        lang.pick("T.", "m."),
                        setting.before + setting.measures
                    );
                    match lang {
                        Lang::De => format!("Schluss {at} auf {final_name}"),
                        Lang::En => format!("final cadence {at} on {final_name}"),
                    }
                }
            };
            format!(
                "{} {} ({}): {}; {head}; {ending}",
                lang.pick("Punkt", "Point"),
                p + 1,
                quoted(&words, lang),
                entries.join(" · ")
            )
        })
        .collect()
}

/// A value in LilyPond: `1`, `2.`, `\breve`.
fn lilypond_value(ticks: u32) -> String {
    match ticks {
        1 => "8".into(),
        2 => "4".into(),
        3 => "4.".into(),
        4 => "2".into(),
        6 => "2.".into(),
        8 => "1".into(),
        12 => "1.".into(),
        16 => "\\breve".into(),
        24 => "\\breve.".into(),
        t => format!("8*{t}"),
    }
}

/// The lines of a plan before its search: per point the line of [`point_lines`], then where its
/// subject comes from with the leader's head and the syllables it begins, and every entry's head
/// as the voice sings it, in LilyPond.
pub fn plan_lines(
    plan: &ImitationPlan,
    setting: &Setting,
    accents: bool,
    lang: Lang,
) -> Vec<String> {
    let kept = plan.score_imitation();
    let mut lines = Vec::new();
    for (point, line) in plan.points.iter().zip(point_lines(&kept, setting, lang)) {
        lines.push(line);
        let kind = match point.kind {
            PointKind::New => String::new(),
            PointKind::Tail => lang.pick("Schlussteil; ", "tail; ").to_string(),
            PointKind::Restatement { of } => match lang {
                Lang::De => format!("Wiederholung von Punkt {} in engerer Engführung; ", of + 1),
                Lang::En => format!("point {} restated in closer stretto; ", of + 1),
            },
        };
        let origin = match point.subject.origin {
            Origin::Text => lang
                .pick("Soggetto aus dem Text", "subject from the text")
                .to_string(),
            Origin::Chant { phrase } => match lang {
                Lang::De => format!("Soggetto aus dem Choral, Phrase {}", phrase + 1),
                Lang::En => format!("subject from the chant, phrase {}", phrase + 1),
            },
            Origin::Given => lang
                .pick("vorgegebenes Soggetto", "given subject")
                .to_string(),
            Origin::Motif => lang
                .pick("Soggetto aus dem Kopfmotiv", "subject from the head motif")
                .to_string(),
        };
        let syllable = |sung: Choice| match (sung, setting.text) {
            (Choice::Start(y), Some(text)) if usize::from(y) < text.syllables.len() => {
                Some(quoted(&text.syllable_label(usize::from(y), accents), lang))
            }
            _ => None,
        };
        if let Some(leader) = point.entries.first() {
            let head: Vec<String> = leader
                .notes
                .iter()
                .map(|&(pitch, ticks, sung)| {
                    let note = format!("{}{}", pitch.to_lilypond(), lilypond_value(ticks));
                    match syllable(sung) {
                        Some(syllable) => format!("{note} {syllable}"),
                        None => note,
                    }
                })
                .collect();
            let label = capitalized(&format!("{kind}{origin}"));
            lines.push(format!("  {label}: {}", head.join(" ")));
        }
        for entry in &point.entries {
            let name = setting
                .names
                .get(entry.voice)
                .cloned()
                .unwrap_or_else(|| (entry.voice + 1).to_string());
            let notes: Vec<String> = entry
                .notes
                .iter()
                .map(|&(pitch, ticks, _)| {
                    format!("{}{}", pitch.to_lilypond(), lilypond_value(ticks))
                })
                .collect();
            lines.push(format!(
                "  {name} {}: {}",
                setting.position(entry.start, lang),
                notes.join(" ")
            ));
        }
    }
    lines
}

/// A text with its first letter in capitals.
fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The plan of a prepared imitative request, as `fux free --plan` shows it, without a search: a
/// line on the mode, the voices, the length and the seed, the text, the internal cadences, the
/// lines of [`plan_lines`] and the warnings of points whose chant gave no subject. `None` where
/// the request does not imitate; an error where the planner finds no plan.
pub fn plan_summary(
    prepared: &PreparedFree,
    lang: Lang,
) -> Option<Result<Vec<String>, GenerateError>> {
    let request = &prepared.request;
    let imitation = request.imitation.as_ref()?;
    let sung = request.text.as_ref()?;
    Some((|| {
        let planner = plan_imitation(request, imitation, &prepared.profile)?;
        let plan = planner.plan();
        let mut choir = request.voices.clone();
        choir.sort();
        let setting = Setting {
            mode: request.mode,
            text: Some(&sung.text),
            accents: sung.accents,
            names: choir_names(&choir, lang),
            cadences: &plan.cadences,
            ending: plan.ending,
            measures: plan.measures,
            ticks_per_measure: fux_core::Meter::ALLA_BREVE.ticks_per_measure(),
            before: 0,
        };
        let mut lines = vec![
            format!(
                "{} – {}, {} {}, {} {}, seed {}",
                request.mode.name(lang),
                lang.pick("Plan der Imitation", "plan of the imitation"),
                choir.len(),
                lang.pick("Stimmen", "voices"),
                plan.measures,
                lang.pick("Takte", "measures"),
                prepared.seed
            ),
            crate::free::text_line(&sung.text, prepared.repeat, lang),
        ];
        if !plan.cadences.is_empty() {
            let list: Vec<String> = plan
                .cadences
                .iter()
                .map(|c| {
                    format!(
                        "{} {} {} {}",
                        lang.pick("Takt", "measure"),
                        c.measure + 1,
                        lang.pick("auf", "on"),
                        target_name(request.mode, c.target, lang)
                    )
                })
                .collect();
            lines.push(format!(
                "{}: {}",
                lang.pick("Binnenschlüsse", "Internal cadences"),
                list.join(", ")
            ));
        }
        lines.extend(plan_lines(&plan, &setting, sung.accents, lang));
        let warning = lang.pick("Warnung", "Warning");
        lines.extend(
            planner
                .fallbacks()
                .iter()
                .map(|fallback| format!("{warning}: {}", fallback.message(lang))),
        );
        lines.push(
            lang.pick(
                "Die Suche probt diesen Plan zuerst; scheitert die Probe, plant sie einen Punkt neu, und die Länge kann sich ändern. Die Zusammenfassung des Satzes nennt den Plan, den er singt.",
                "The search probes this plan first; where the probe fails, it plans a point anew, and the length may change. The summary of the setting gives the plan it sings.",
            )
            .to_string(),
        );
        Ok(lines)
    })())
}

/// Where a check of a score takes its imitation from (`fux check --imitation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImitationSource {
    /// The plan that a file of fux-kontrapunkt keeps, which the IMI rules judge.
    #[default]
    File,
    /// The points the detector finds, listed as information only, without the IMI rules.
    Detect,
    /// None: no IMI rules.
    Off,
}

impl FromStr for ImitationSource {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.trim().to_lowercase().as_str() {
            "datei" | "file" => Ok(ImitationSource::File),
            "erkennen" | "detect" | "recognise" | "recognize" => Ok(ImitationSource::Detect),
            "aus" | "off" => Ok(ImitationSource::Off),
            _ => Err(format!(
                "{text:?}: datei (file), erkennen (detect) or aus (off)"
            )),
        }
    }
}

/// Gives a score to check the imitation `source` says, once its lyrics have their text: the plan
/// `stored` that its file keeps, if it fits the score, so that the IMI rules judge the entries as
/// the search did; or no plan, with the points the detector finds (entries after rests of a minim
/// or longer) listed as information only; or none. Returns lines for the user: the points of the
/// plan, or those found. A stored plan that cannot be read or does not fit the score is an error
/// that says what may help (`remedy`).
pub fn attach(
    score: &mut Score,
    stored: Option<Result<ScoreImitation, ScoreError>>,
    source: ImitationSource,
    remedy: &str,
    lang: Lang,
) -> Result<Vec<String>> {
    let ending = score.mode.final_step();
    attach_ending(score, stored, source, remedy, ending, lang)
}

/// Gives a score its imitation as [`attach`] does, for a setting that ends on `ending` instead
/// of the final, as a part of a mass may (phase 14): the last point of its plan ends there.
pub fn attach_ending(
    score: &mut Score,
    stored: Option<Result<ScoreImitation, ScoreError>>,
    source: ImitationSource,
    remedy: &str,
    ending: fux_core::Step,
    lang: Lang,
) -> Result<Vec<String>> {
    attach_after(score, stored, source, remedy, ending, 0, lang)
}

/// Gives a score its imitation as [`attach_ending`] does, for a score that follows `before`
/// measures of a larger whole, a part of a mass in the file of its movement (phase 14): its lines
/// count the measures on from there ([`Setting::before`]).
pub fn attach_after(
    score: &mut Score,
    stored: Option<Result<ScoreImitation, ScoreError>>,
    source: ImitationSource,
    remedy: &str,
    ending: fux_core::Step,
    before: usize,
    lang: Lang,
) -> Result<Vec<String>> {
    score.imitation = None;
    match source {
        ImitationSource::Off => Ok(Vec::new()),
        ImitationSource::Detect => Ok(detected_lines_after(score, before, lang)),
        ImitationSource::File => {
            let Some(stored) = stored else {
                return Ok(Vec::new());
            };
            let meter = score.meter;
            let unusable = |error: ScoreError| {
                let message = format!(
                    "{}: {}",
                    lang.pick(
                        "Der Imitationsplan der Datei passt nicht zum Satz",
                        "The imitation plan of the file does not fit the score"
                    ),
                    error.message(lang, meter)
                );
                match remedy {
                    "" => anyhow!("{message}"),
                    remedy => anyhow!("{message} {remedy}"),
                }
            };
            let plan = stored.map_err(unusable)?;
            plan.validate(score).map_err(unusable)?;
            let mut lines = vec![match lang {
                Lang::De => format!(
                    "Imitation nach dem Plan der Datei: {} {}",
                    plan.points.len(),
                    if plan.points.len() == 1 {
                        "Punkt"
                    } else {
                        "Punkte"
                    }
                ),
                Lang::En => format!(
                    "Imitation as the file plans it: {} {}",
                    plan.points.len(),
                    if plan.points.len() == 1 {
                        "point"
                    } else {
                        "points"
                    }
                ),
            }];
            let setting = Setting {
                ending,
                before,
                ..Setting::of(score, lang)
            };
            lines.extend(point_lines(&plan, &setting, lang));
            score.imitation = Some(Arc::new(plan));
            Ok(lines)
        }
    }
}

/// The points of imitation that the detector finds in a score, entries after rests of a minim
/// or longer, as lines for the user: information only, never cases of the IMI rules.
pub fn detected_lines(score: &Score, lang: Lang) -> Vec<String> {
    detected_lines_after(score, 0, lang)
}

/// The lines of [`detected_lines`] for a score that follows `before` measures of a larger whole,
/// counting its measures on from there.
pub fn detected_lines_after(score: &Score, before: usize, lang: Lang) -> Vec<String> {
    let found = detect(score, Duration::HALF.ticks());
    let points: Vec<_> = found.iter().filter(|p| p.entries.len() >= 2).collect();
    if points.is_empty() {
        return vec![
            lang.pick("Keine Imitation erkannt.", "No imitation recognised.")
                .to_string(),
        ];
    }
    let setting = Setting {
        before,
        ..Setting::of(score, lang)
    };
    let sections = found.iter().map(|p| p.section).max().unwrap_or(0) + 1;
    let mut lines = vec![match lang {
        Lang::De => format!(
            "Imitation erkannt, nur zur Information: {} {}",
            points.len(),
            if points.len() == 1 { "Punkt" } else { "Punkte" }
        ),
        Lang::En => format!(
            "Imitation recognised, as information only: {} {}",
            points.len(),
            if points.len() == 1 { "point" } else { "points" }
        ),
    }];
    for (i, point) in points.iter().enumerate() {
        let leader = point.entries[0].notes.first().map(|s| s.pitch);
        let entries: Vec<String> = point
            .entries
            .iter()
            .enumerate()
            .map(|(k, entry)| {
                let name = setting
                    .names
                    .get(entry.voice)
                    .cloned()
                    .unwrap_or_else(|| (entry.voice + 1).to_string());
                let mut item = format!("{name} {}", setting.position(entry.start, lang));
                if let Some(first) = entry.notes.first() {
                    item.push_str(&format!(" {}", first.pitch.name(lang)));
                    if let (true, Some(leader)) = (k > 0, leader) {
                        item.push_str(&format!(" ({})", interval_name(leader, first.pitch, lang)));
                    }
                }
                item
            })
            .collect();
        let matched = point.entries.iter().map(|e| e.matched).max().unwrap_or(0);
        let section = if sections > 1 {
            match lang {
                Lang::De => format!(" (Abschnitt {})", point.section + 1),
                Lang::En => format!(" (section {})", point.section + 1),
            }
        } else {
            String::new()
        };
        lines.push(match lang {
            Lang::De => format!(
                "Punkt {}{section}: {}; bis zu {matched} gleiche Töne",
                i + 1,
                entries.join(" · ")
            ),
            Lang::En => format!(
                "Point {}{section}: {}; up to {matched} notes alike",
                i + 1,
                entries.join(" · ")
            ),
        });
    }
    lines
}

/// What an imitative request gives that the planner would otherwise choose, so that the
/// remedies of a failure can name it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Given {
    pub length: bool,
    pub head: bool,
    pub order: bool,
    pub intervals: bool,
}

impl Given {
    /// What `request` gives.
    pub fn of(request: &FreeRequest) -> Given {
        let imitation = request.imitation.as_ref();
        let any = |lists: &[Option<Vec<usize>>]| lists.iter().any(Option::is_some);
        Given {
            length: request.text.as_ref().is_some_and(|sung| !sung.auto_length),
            head: imitation.is_some_and(|i| i.head.is_some()),
            order: imitation.is_some_and(|i| any(&i.orders)),
            intervals: imitation.is_some_and(|i| i.relations.iter().any(Option::is_some)),
        }
    }

    /// The remedies that set back what was given, in the words of `names` (head, order,
    /// intervals, length), each led by a comma.
    pub fn undo(&self, names: [&str; 4]) -> String {
        [self.head, self.order, self.intervals, self.length]
            .into_iter()
            .zip(names)
            .filter(|(given, _)| *given)
            .map(|(_, name)| format!(", {name}"))
            .collect()
    }
}

/// The remedies for a failed imitative request, in the words of the command line, with what
/// the request gives (`given`); `None` for errors of other kinds.
pub fn remedies(error: &GenerateError, given: Given, lang: Lang) -> Option<String> {
    let undo = given.undo([
        "--head auto",
        "--order auto",
        "--intervals auto",
        "--length auto",
    ]);
    Some(match (error, lang) {
        (GenerateError::NoImitation { .. }, Lang::De) => format!(
            "Abhilfe: ein anderer --seed, andere --delays, weniger --point-voices{undo} oder --imitation aus."
        ),
        (GenerateError::NoImitation { .. }, Lang::En) => format!(
            "Remedies: another --seed, other --delays, fewer --point-voices{undo} or --imitation off."
        ),
        (GenerateError::ImitationTooLong { measures }, Lang::De) => {
            format!("Abhilfe: --length {measures} oder auto, oder --imitation aus.")
        }
        (GenerateError::ImitationTooLong { measures }, Lang::En) => {
            format!("Remedies: --length {measures} or auto, or --imitation off.")
        }
        (GenerateError::ImitationNeedsText, _) => lang
            .pick(
                "Abhilfe: ein Text (--text) oder --imitation aus.",
                "Remedies: a text (--text) or --imitation off.",
            )
            .into(),
        (GenerateError::ImitationCadences, _) => lang
            .pick(
                "Abhilfe: --cadences auto oder --imitation aus.",
                "Remedies: --cadences auto or --imitation off.",
            )
            .into(),
        (GenerateError::SubjectInvalid { .. }, _) => lang
            .pick(
                "Abhilfe: ein anderes Soggetto (--subject) oder --subject text.",
                "Remedies: another subject (--subject) or --subject text.",
            )
            .into(),
        (GenerateError::NoChantPhrase { phrases, .. }, Lang::De) => format!(
            "Abhilfe: Phrasen von 1 bis {phrases} (chant:DATEI:1,2) oder keine, dann nach den Worten."
        ),
        (GenerateError::NoChantPhrase { phrases, .. }, Lang::En) => format!(
            "Remedies: phrases from 1 to {phrases} (chant:FILE:1,2) or none, then by the words."
        ),
        (GenerateError::PointCount { most }, Lang::De) => {
            format!("Abhilfe: --points auto oder höchstens {most}.")
        }
        (GenerateError::PointCount { most }, Lang::En) => {
            format!("Remedies: --points auto or {most} at most.")
        }
        (GenerateError::ImitationOption(option), _) => option_remedy(option, lang),
        _ => return None,
    })
}

/// What may help against an option of the imitation that the request cannot have, in the words
/// of the command line.
fn option_remedy(option: &OptionError, lang: Lang) -> String {
    match (option, lang) {
        (OptionError::NoPoint { points, .. }, Lang::De) => format!(
            "Abhilfe: --order, --intervals und --delays für höchstens {points} Punkte, durch ; getrennt."
        ),
        (OptionError::NoPoint { points, .. }, Lang::En) => format!(
            "Remedies: --order, --intervals and --delays for {points} points at most, separated by ;."
        ),
        (
            OptionError::NoVoice { .. }
            | OptionError::VoiceTwice { .. }
            | OptionError::FewVoices { .. },
            _,
        ) => lang
            .pick(
                "Abhilfe: eine andere --order oder --order auto.",
                "Remedies: another --order or --order auto.",
            )
            .into(),
        (OptionError::TooMany { .. }, _) => lang
            .pick(
                "Abhilfe: weniger Werte für diesen Punkt in --delays und --intervals, oder mehr Stimmen für ihn (--point-voices, --order).",
                "Remedies: fewer values for this point in --delays and --intervals, or more voices for it (--point-voices, --order).",
            )
            .into(),
        (OptionError::Delay { .. }, _) => lang
            .pick(
                "Abhilfe: --delays in Takten ab einer Halben, Halbe erlaubt, z. B. 1 1.5.",
                "Remedies: --delays in measures from a minim on, halves allowed, such as 1 1.5.",
            )
            .into(),
        (OptionError::LaterVoices { most, .. }, Lang::De) => {
            format!("Abhilfe: --point-voices auto, alle oder 2 bis {most}.")
        }
        (OptionError::LaterVoices { most, .. }, Lang::En) => {
            format!("Remedies: --point-voices auto, all or 2 to {most}.")
        }
        (OptionError::Head { most, .. }, Lang::De) => {
            format!("Abhilfe: --head auto oder 4 bis {most}.")
        }
        (OptionError::Head { most, .. }, Lang::En) => {
            format!("Remedies: --head auto or 4 to {most}.")
        }
        (OptionError::Subjects { points, .. }, Lang::De) => format!(
            "Abhilfe: höchstens {points} Köpfe in --subject oder Phrasen in --subject chant:DATEI:PHRASEN, oder --restate aus."
        ),
        (OptionError::Subjects { points, .. }, Lang::En) => format!(
            "Remedies: {points} heads in --subject or phrases in --subject chant:FILE:PHRASES at most, or --restate off."
        ),
        (OptionError::Restate, _) => lang
            .pick(
                "Abhilfe: --restate auto oder aus.",
                "Remedies: --restate auto or off.",
            )
            .into(),
        (OptionError::Relation { .. }, _) => lang
            .pick(
                "Abhilfe: andere --intervals oder --intervals auto.",
                "Remedies: other --intervals or --intervals auto.",
            )
            .into(),
    }
}

#[cfg(test)]
mod tests {
    use fux_core::imitation::chant::Kyriale;
    use fux_core::text::Eleison;
    use fux_core::{Role, ScorePoint, Species, Voice};

    use super::*;
    use crate::text::text_of;

    fn kyrie() -> Text {
        text_of("kyrie", Eleison::Four, Lang::De)
            .unwrap()
            .unwrap()
            .0
    }

    #[test]
    fn imitation_options_parse() {
        use VoiceType::*;
        assert_eq!("auto".parse(), Ok(ImitationMode::Auto));
        assert_eq!("AN".parse(), Ok(ImitationMode::On));
        assert_eq!("off".parse(), Ok(ImitationMode::Off));
        assert!("ja".parse::<ImitationMode>().is_err());
        assert_eq!(toggle("aus"), Ok(Toggle::Off));
        assert!(toggle("vielleicht").is_err());
        assert_eq!(answer_choice("Modal"), Ok(AnswerChoice::Modal));
        assert!(answer_choice("tonal").is_err());
        assert_eq!("auto".parse(), Ok(AutoCount(None)));
        assert_eq!("3".parse(), Ok(AutoCount(Some(3))));
        assert!("0".parse::<AutoCount>().is_err());
        assert_eq!(head_notes("6"), Ok(AutoCount(Some(6))));
        assert!(head_notes("3").is_err());
        assert_eq!("alle".parse(), Ok(PointVoices::All));
        assert_eq!("2".parse(), Ok(PointVoices::Count(2)));
        assert!("1".parse::<PointVoices>().is_err());
        // Orders: by type, numbered among voices of one type, or by number.
        assert_eq!(
            "A S T1 4; auto; t2 b".parse(),
            Ok(Orders(vec![
                Some(vec![
                    VoiceRef::Type(Alto, None),
                    VoiceRef::Type(Soprano, None),
                    VoiceRef::Type(Tenor, Some(1)),
                    VoiceRef::Number(4),
                ]),
                None,
                Some(vec![
                    VoiceRef::Type(Tenor, Some(2)),
                    VoiceRef::Type(Bass, None)
                ]),
            ]))
        );
        assert_eq!("auto".parse(), Ok(Orders::default()));
        assert!("A X".parse::<Orders>().is_err());
        // Intervals: the same degree, the other main degree, steps.
        assert_eq!(
            "d 0 d; 0, +2 -3 +7".parse(),
            Ok(Intervals(vec![
                Some(vec![Relation::Dominant, Relation::Same, Relation::Dominant]),
                Some(vec![
                    Relation::Same,
                    Relation::Steps(2),
                    Relation::Steps(-3),
                    Relation::Same
                ]),
            ]))
        );
        assert!("4".parse::<Intervals>().is_err());
        // Delays in measures, halves allowed; a comma is neither guessed as a decimal comma nor
        // as a separator.
        assert_eq!(
            "1 2 1.5; ; 0.5".parse(),
            Ok(Delays(vec![Some(vec![8, 16, 12]), None, Some(vec![4])]))
        );
        assert!("0,5".parse::<Delays>().is_err());
        assert!("1,2".parse::<Delays>().is_err());
        assert!("1.25".parse::<Delays>().is_err());
        assert!("0".parse::<Delays>().is_err());
    }

    #[test]
    fn subjects_parse() {
        assert_eq!("text".parse(), Ok(SubjectChoice::Text));
        assert_eq!(
            "chant".parse(),
            Ok(SubjectChoice::Chant {
                chant: ChantRef::Kyriale,
                phrases: None
            })
        );
        assert_eq!(
            "chant:kyrie-iv".parse(),
            Ok(SubjectChoice::Chant {
                chant: ChantRef::Named("kyrie-iv".into()),
                phrases: None
            })
        );
        assert_eq!(
            "chant:hymnen/iste.gabc:2,4-5".parse(),
            Ok(SubjectChoice::Chant {
                chant: ChantRef::Named("hymnen/iste.gabc".into()),
                phrases: Some(vec![2, 4, 5])
            })
        );
        assert_eq!(
            "chant::3".parse(),
            Ok(SubjectChoice::Chant {
                chant: ChantRef::Kyriale,
                phrases: Some(vec![3])
            })
        );
        let SubjectChoice::Notes(heads) = "d'1 d'2 a'2 g'4 f'4 e'2 d'1 | a'1 g'2".parse().unwrap()
        else {
            panic!("notes");
        };
        assert_eq!(heads.len(), 2);
        assert_eq!(heads[0].len(), 7);
        assert_eq!(heads[1][1].duration, Duration::HALF);
        assert!("x'1".parse::<SubjectChoice>().is_err());
        assert!("d'1 |".parse::<SubjectChoice>().is_err());
        assert_eq!(phrase_list("1-3,5"), Some(vec![1, 2, 3, 5]));
        assert_eq!(phrase_list("0"), None);
    }

    #[test]
    fn options_make_the_request() {
        let text = kyrie();
        let satb = [
            VoiceType::Soprano,
            VoiceType::Alto,
            VoiceType::Tenor,
            VoiceType::Bass,
        ];
        let options = ImitationOptions::default();
        assert!(options.applies(true, 4, true, true));
        assert!(!options.applies(true, 2, true, true) && !options.applies(true, 6, true, true));
        assert!(!options.applies(false, 4, true, true) && !options.applies(true, 4, false, true));
        assert!(!options.applies(true, 4, true, false));
        assert!(!options.shapes());
        let imitation = options.imitation(&text, None, &satb, Lang::De).unwrap();
        assert_eq!(imitation, Imitation::default());
        let options = ImitationOptions {
            mode: ImitationMode::On,
            subject: "chant".parse().unwrap(),
            points: AutoCount(Some(2)),
            point_voices: PointVoices::All,
            order: "A S T B; T B".parse().unwrap(),
            intervals: "d 0 d".parse().unwrap(),
            delays: "1 2 1.5".parse().unwrap(),
            pairs: Toggle::Off,
            answer: AnswerChoice::Real,
            head: AutoCount(Some(5)),
            ..ImitationOptions::default()
        };
        assert!(options.shapes() && options.applies(false, 6, false, false));
        let chant = options.chant(&text, Lang::De).unwrap().unwrap();
        assert_eq!(chant, Kyriale::KyrieIX.chant());
        assert_eq!(
            chant_mode(&chant).map(|m| m.id()).as_deref(),
            Some("dorian")
        );
        let imitation = options
            .imitation(&text, Some(chant), &satb, Lang::De)
            .unwrap();
        assert!(matches!(
            imitation.subject,
            SubjectSource::Chant { phrases: None, .. }
        ));
        assert_eq!(imitation.points, Some(2));
        assert_eq!(imitation.later_voices, Some(4));
        assert_eq!(imitation.orders, [Some(vec![1, 0, 2, 3]), Some(vec![2, 3])]);
        assert_eq!(imitation.delays, [Some(vec![8, 16, 12])]);
        assert_eq!(
            (imitation.pairs, imitation.answer, imitation.head),
            (Toggle::Off, AnswerChoice::Real, Some(5))
        );
        // Voices that the choir does not have, or not once.
        let error = |order: &str, lang| {
            ImitationOptions {
                order: order.parse().unwrap(),
                ..ImitationOptions::default()
            }
            .imitation(&text, None, &satb, lang)
            .unwrap_err()
            .to_string()
        };
        assert_eq!(
            error("Mz", Lang::En),
            "--order: the parts have no voice Mz. Remedies: another --order or --order auto."
        );
        assert_eq!(
            error("5", Lang::De),
            "--order: Stimme 5 gibt es nicht; der Satz hat 4. Abhilfe: eine andere --order oder --order auto."
        );
        // In the words of a window, and a voice twice or a later point of one voice left to the
        // planner, which refuses them in each frontend's words.
        let failure = ImitationOptions {
            order: "5".parse().unwrap(),
            ..ImitationOptions::default()
        }
        .imitation(&text, None, &satb, Lang::De)
        .unwrap_err();
        assert_eq!(
            form_message(&failure, Lang::De, Terms::Window),
            "Reihenfolge: Stimme 5 gibt es nicht; der Satz hat 4. Abhilfe: eine andere Reihenfolge, oder keine."
        );
        for (order, orders) in [
            ("A A", vec![Some(vec![1, 1])]),
            ("S; T", vec![Some(vec![0]), Some(vec![2])]),
        ] {
            let imitation = ImitationOptions {
                order: order.parse().unwrap(),
                ..ImitationOptions::default()
            }
            .imitation(&text, None, &satb, Lang::De)
            .unwrap();
            assert_eq!(imitation.orders, orders, "{order}");
        }
        let five = [
            VoiceType::Soprano,
            VoiceType::Alto,
            VoiceType::Tenor,
            VoiceType::Tenor,
            VoiceType::Bass,
        ];
        let tenors = ImitationOptions {
            order: "T2 T1".parse().unwrap(),
            ..ImitationOptions::default()
        };
        assert_eq!(
            tenors
                .imitation(&text, None, &five, Lang::De)
                .unwrap()
                .orders,
            [Some(vec![3, 2])]
        );
        let ambiguous = ImitationOptions {
            order: "T".parse().unwrap(),
            ..ImitationOptions::default()
        };
        assert_eq!(
            ambiguous
                .imitation(&text, None, &five, Lang::De)
                .unwrap_err()
                .to_string(),
            "--order: T gibt es 2-mal; T1 bis T2 nennen sie von oben. Abhilfe: eine andere --order oder --order auto."
        );
        // More later voices than the choir has: the planner's `LaterVoices`.
        let too_many = ImitationOptions {
            point_voices: PointVoices::Count(5),
            ..ImitationOptions::default()
        };
        assert_eq!(
            too_many
                .imitation(&text, None, &satb, Lang::De)
                .unwrap()
                .later_voices,
            Some(5)
        );
        // An own text has no chant in the Kyriale.
        let own = text_of("Ad te le-vá-vi.", Eleison::Four, Lang::En)
            .unwrap()
            .unwrap()
            .0;
        let chant = ImitationOptions {
            subject: "chant".parse().unwrap(),
            ..ImitationOptions::default()
        };
        assert_eq!(
            chant.chant(&own, Lang::En).unwrap_err().to_string(),
            "The Kyriale has no chant for an own text. Remedies: --subject chant:FILE with a GABC \
             file, a chant of the Kyriale such as chant:kyrie-iv, or --subject text."
        );
        let failure = chant.chant(&own, Lang::De).unwrap_err();
        assert_eq!(
            form_message(&failure, Lang::De, Terms::Page),
            "Für den eigenen Text hat das Kyriale keinen Choral. Abhilfe: einen Choral als GABC \
             eingeben, oder das Soggetto aus dem Text."
        );
        let missing = ImitationOptions {
            subject: "chant:nirgends.gabc".parse().unwrap(),
            ..ImitationOptions::default()
        };
        assert!(
            missing
                .chant(&text, Lang::De)
                .unwrap_err()
                .to_string()
                .starts_with("nirgends.gabc: nicht lesbar; die Choräle des Kyriale sind kyrie-ix"),
        );
    }

    /// The line of a point names its words, its entries with their intervals to the leader, its
    /// head, its answers and its cadence.
    #[test]
    fn lines_of_points() {
        let text = kyrie();
        let entry = |voice, start, head: &[&str]| ScoreEntry {
            voice,
            start,
            rest: 0,
            notes: head
                .iter()
                .map(|n| {
                    let (pitch, ticks) = n.split_once('.').unwrap();
                    (pitch.parse().unwrap(), ticks.parse().unwrap())
                })
                .collect(),
        };
        let plan = ScoreImitation {
            points: vec![
                ScorePoint {
                    words: 0..7,
                    entries: vec![
                        entry(1, 0, &["D4.8", "D4.4", "A4.4", "G4.4", "F4.4", "E4.8"]),
                        entry(0, 8, &["A4.8", "A4.4", "E5.4", "D5.4", "C5.4", "B4.8"]),
                        entry(2, 28, &["A3.8", "A3.4", "E4.4", "D4.4"]),
                        entry(3, 32, &["D3.8", "D3.4", "A3.4", "G3.4", "F3.4", "E3.8"]),
                    ],
                    goal: Some(11),
                },
                ScorePoint {
                    words: 3..7,
                    entries: vec![
                        entry(2, 100, &["A3.8", "C4.4", "B3.4", "A3.8"]),
                        entry(0, 108, &["D5.8", "F5.4", "E5.4", "D5.8"]),
                    ],
                    goal: None,
                },
            ],
        };
        let mode: ModeSpec = "dorian".parse().unwrap();
        let cadences = [Cadence {
            measure: 11,
            target: fux_core::Step::A,
        }];
        let satb = [
            VoiceType::Soprano,
            VoiceType::Alto,
            VoiceType::Tenor,
            VoiceType::Bass,
        ];
        let setting = |lang| Setting {
            mode,
            text: Some(&text),
            accents: true,
            names: choir_names(&satb, lang),
            cadences: &cadences,
            ending: fux_core::Step::D,
            measures: 20,
            ticks_per_measure: 8,
            before: 0,
        };
        assert_eq!(
            point_lines(&plan, &setting(Lang::De), Lang::De),
            [
                "Punkt 1 („Ký-ri-e e-lé-i-son.“): Alt T. 1 D4 · Sopran T. 2 A4 (Quinte) · Tenor T. 4½ A3 (Unterquarte) · Bass T. 5 D3 (Unteroktave); Kopf 4–6 Töne, real; Binnenschluss T. 12 auf A",
                "Punkt 2 („e-lé-i-son.“): Tenor T. 13½ A3 · Sopran T. 14½ D5 (Undezime); Kopf 4 Töne, real; Schluss T. 20 auf D",
            ]
        );
        assert_eq!(
            point_lines(&plan, &setting(Lang::En), Lang::En)[0],
            "Point 1 (\"Ký-ri-e e-lé-i-son.\"): Alto m. 1 D4 · Soprano m. 2 A4 (fifth above) · Tenor m. 4½ A3 (fourth below) · Bass m. 5 D3 (octave below); head 4–6 notes, real; internal cadence m. 12 on A"
        );
        // A modal answer, a fourth for the leader's fifth in the second interval, and another
        // change.
        let mut modal = plan.clone();
        modal.points[0].entries[1] = entry(0, 8, &["A4.8", "A4.4", "D5.4", "C5.4"]);
        modal.points[0].entries[2] = entry(2, 28, &["A3.8", "A3.4", "C4.4"]);
        let line = &point_lines(&modal, &setting(Lang::De), Lang::De)[0];
        assert!(
            line.contains("; Kopf 3–6 Töne, real, modal und verändert; "),
            "{line}"
        );
        // A part that ends on the fifth (phase 14): the last point closes there.
        let on_the_fifth = Setting {
            ending: fux_core::Step::A,
            ..setting(Lang::En)
        };
        assert!(
            point_lines(&plan, &on_the_fifth, Lang::En)[1].ends_with("; final cadence m. 20 on A"),
            "{:?}",
            point_lines(&plan, &on_the_fifth, Lang::En)
        );
        // A part of a mass in the file of its movement, after 30 measures (phase 14): the
        // measures are those of the movement.
        let in_its_movement = Setting {
            before: 30,
            ..setting(Lang::De)
        };
        assert_eq!(
            point_lines(&plan, &in_its_movement, Lang::De),
            [
                "Punkt 1 („Ký-ri-e e-lé-i-son.“): Alt T. 31 D4 · Sopran T. 32 A4 (Quinte) · Tenor T. 34½ A3 (Unterquarte) · Bass T. 35 D3 (Unteroktave); Kopf 4–6 Töne, real; Binnenschluss T. 42 auf A",
                "Punkt 2 („e-lé-i-son.“): Tenor T. 43½ A3 · Sopran T. 44½ D5 (Undezime); Kopf 4 Töne, real; Schluss T. 50 auf D",
            ]
        );
        // Without the acutes, as --accents aus prints the words; pitches by their German names.
        let plain = Setting {
            accents: false,
            ..setting(Lang::De)
        };
        let mut flat = plan.clone();
        flat.points[1].entries[1] = entry(0, 116, &["Bb4.8", "A4.4", "B4.4", "C5.8"]);
        let lines = point_lines(&flat, &plain, Lang::De);
        assert!(
            lines[0].starts_with("Punkt 1 („Ky-ri-e e-le-i-son.“): "),
            "{}",
            lines[0]
        );
        assert!(
            lines[1].contains(" · Sopran T. 15½ B4 (None)"),
            "{}",
            lines[1]
        );
        // Numbered voices of one type, and names of their own.
        let voices = [
            (VoiceType::Tenor, "Tenor"),
            (VoiceType::Tenor, "Tenor"),
            (VoiceType::Bass, "Bassus"),
        ];
        assert_eq!(
            voice_names(voices.into_iter(), Lang::En),
            ["Tenor 1", "Tenor 2", "Bassus"]
        );
    }

    #[test]
    fn intervals_and_positions() {
        let p = |s: &str| s.parse::<Pitch>().unwrap();
        assert_eq!(interval_name(p("D4"), p("D4"), Lang::De), "Einklang");
        assert_eq!(interval_name(p("D4"), p("A4"), Lang::De), "Quinte");
        assert_eq!(interval_name(p("D4"), p("G2"), Lang::De), "Unterduodezime");
        assert_eq!(interval_name(p("A4"), p("D4"), Lang::En), "fifth below");
        assert_eq!(interval_name(p("D3"), p("D6"), Lang::En), "21 steps above");
        assert_eq!(position(0, 8, Lang::De), "T. 1");
        assert_eq!(position(12, 8, Lang::En), "m. 2½");
        assert_eq!(position(18, 8, Lang::De), "T. 3, Viertel 2");
    }

    /// A check takes the plan its file keeps where it fits, else says what may help; detection
    /// lists what it finds and plans nothing.
    #[test]
    fn checks_take_the_plan_of_the_file() {
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
        let head = |start, pitches: &[&str], values: &[u32]| ScoreEntry {
            voice: usize::from(start > 0) ^ 1,
            start,
            rest: 0,
            notes: pitches
                .iter()
                .zip(values)
                .map(|(p, &t)| (p.parse().unwrap(), t))
                .collect(),
        };
        let plan = ScoreImitation {
            points: vec![ScorePoint {
                words: 0..7,
                entries: vec![
                    head(0, &["D4", "D4", "C4", "Bb3", "A3"], &[8, 4, 4, 4, 4]),
                    head(8, &["A4", "A4", "G4", "F4", "E4"], &[8, 4, 4, 4, 4]),
                ],
                goal: None,
            }],
        };
        let stored = Some(Ok(plan.clone()));
        let lines = attach(
            &mut score,
            stored.clone(),
            ImitationSource::File,
            "",
            Lang::De,
        )
        .unwrap();
        assert_eq!(lines[0], "Imitation nach dem Plan der Datei: 1 Punkt");
        assert!(
            lines[1].starts_with("Punkt 1 („Silben 1–7“): Alt T. 1 D4 · Sopran T. 2 A4 (Quinte); "),
            "{lines:?}"
        );
        assert_eq!(score.imitation.as_deref(), Some(&plan));
        // Detection plans nothing; off neither.
        let found = attach(
            &mut score,
            stored.clone(),
            ImitationSource::Detect,
            "",
            Lang::En,
        )
        .unwrap();
        assert!(score.imitation.is_none());
        assert_eq!(
            found[0],
            "Imitation recognised, as information only: 1 point"
        );
        assert!(
            found[1].starts_with("Point 1: Alto m. 1 D4 · Soprano m. 2 A4 (fifth above); up to "),
            "{found:?}"
        );
        assert!(
            attach(&mut score, stored, ImitationSource::Off, "", Lang::De)
                .unwrap()
                .is_empty()
        );
        // A plan that does not fit: an entry in a voice the score lacks.
        let mut misfit = plan.clone();
        misfit.points[0].entries[1].voice = 5;
        let error = attach(
            &mut score,
            Some(Ok(misfit)),
            ImitationSource::File,
            "Abhilfe: --imitation aus.",
            Lang::De,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.starts_with("Der Imitationsplan der Datei passt nicht zum Satz: ")
                && error.ends_with(" Abhilfe: --imitation aus."),
            "{error}"
        );
        assert!(score.imitation.is_none());
        assert_eq!("erkennen".parse(), Ok(ImitationSource::Detect));
        assert!("ja".parse::<ImitationSource>().is_err());
    }
}
