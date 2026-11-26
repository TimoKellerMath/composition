//! Generators: cantus firmi (phase 3), counterpoint (from phase 4), free counterpoint (phase 11),
//! imitation (phase 13), parts of a mass (phase 14).

mod cantus;
mod counterpoint;
mod free;
mod imitation;
mod rng;

use std::sync::Arc;

pub use cantus::{CantusRequest, final_for, generate_cantus};
pub(crate) use counterpoint::sung_profile;
pub use counterpoint::{
    CounterpointRequest, Fixed, FixedRole, MAX_WIDENING, NewVoice, SungText, choir_with,
    default_choir, generate_counterpoint, pitch_pool, pitch_pool_ending, pitch_pool_with,
};
pub(crate) use free::CADENCE_SPACING;
#[cfg(test)]
pub(crate) use free::free_search;
pub use free::{
    Budget, FreeRequest, FreeText, MIN_MEASURES, PartContext, SearchControl, cadence_formula,
    cadences_at, default_cadences, generate_free, part_endings, regular_cadences,
};
pub use rng::Rng;
pub(crate) use rng::{fingerprint, splitmix64, splitmix64_inverse};

use thiserror::Error;

use crate::imitation::chant::ChantFallback;
use crate::imitation::{OptionError, Soggetto};
use crate::lang::Lang;
use crate::mass::MassOptionError;
use crate::mode::ModeSpec;
use crate::pitch::{Pitch, Step};
use crate::rules::Report;
use crate::score::Score;

/// A generated score with its cost under the profile used.
#[derive(Clone, Debug)]
pub struct Generated {
    pub score: Score,
    /// Sum of the costs of all violated preferences.
    pub cost: u32,
    /// All violations of the finished score.
    pub report: Report,
    /// What the generator did otherwise than the request asked, for the user to read.
    pub warnings: Vec<Warning>,
    /// The subject of each point of an imitative setting, as the plan it was set by has them
    /// (for the head motif of a mass); empty for a setting without imitation.
    pub subjects: Vec<Arc<Soggetto>>,
}

/// Something a generator did otherwise than the request asked, which the result notes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Warning {
    /// A point of an imitation on chant whose phrase gave no usable head took a subject from its
    /// words.
    ChantFallback(ChantFallback),
    /// The first point of a movement of a mass, which was to take the head motif of Kyrie I,
    /// found no plan with it and took a subject from its words (phase 14, MASS-MOTIF). The
    /// movement by its title.
    MotifFallback { movement: String },
    /// The first point of a movement of a mass, which was to take the head motif of Kyrie I,
    /// took a subject from its words, since Kyrie I gave none: its first subject has not five to
    /// ten tones, or it begins together or is not set (phase 14, MASS-MOTIF). The movement by its
    /// title.
    NoMotif { movement: String },
    /// A movement of a mass on chant in one mode for the whole mass (`--mode` with `--chant`,
    /// D12) whose chant's mode has another final: the chant's degrees are sung from the final of
    /// the mass. The movement by its title, the modes of the chant and of the mass.
    ChantMode {
        movement: String,
        chant: ModeSpec,
        mass: ModeSpec,
    },
    /// A movement of a mass on chant IV other than the Kyrie, which takes the chant of Mass IX
    /// or Credo I: the Kyriale has Kyrie IV alone (D12).
    ChantIV,
    /// A part of a mass lengthened by two restated points (`--lengths palestrina`, MASS-LENGTH)
    /// that no attempt could set, set with the points its imitation plans by itself. The part by
    /// its address (`benedictus/1`).
    LengthFallback { part: String },
    /// A section of a mass with a voice added to the choir (Agnus Dei II, MASS-SCORING) that no
    /// attempt could set, set in the choir of the mass. The section by its id (`agnus3`).
    ScoringFallback { section: String },
    /// A movement of a mass with a head motif whose first part begins together instead of in
    /// imitation, so that it carries no motif (MASS-MOTIF): with two or six voices every part
    /// does, and `--imitation` can make any part begin together. The movement by its title.
    MotifTogether { movement: String },
}

impl Warning {
    /// Message for users in their language.
    pub fn message(&self, lang: Lang) -> String {
        match (self, lang) {
            (Warning::ChantFallback(fallback), _) => fallback.message(lang),
            (Warning::MotifFallback { movement }, Lang::De) => format!(
                "{movement}: Mit dem Kopfmotiv fand der erste Punkt keinen Plan; sein Soggetto \
                 kommt aus dem Text"
            ),
            (Warning::MotifFallback { movement }, Lang::En) => format!(
                "{movement}: the first point found no plan with the head motif; its subject comes \
                 from the text"
            ),
            (Warning::NoMotif { movement }, Lang::De) => format!(
                "{movement}: Das Kyrie I gab kein Kopfmotiv (es beginnt gemeinsam, fehlt, oder \
                 sein erstes Soggetto hat nicht 5 bis 10 Töne); der erste Punkt nimmt sein \
                 Soggetto aus dem Text"
            ),
            (Warning::NoMotif { movement }, Lang::En) => format!(
                "{movement}: Kyrie I gave no head motif (it begins together, is missing, or its \
                 first subject has not 5 to 10 tones); the first point takes its subject from the \
                 text"
            ),
            (
                Warning::ChantMode {
                    movement,
                    chant,
                    mass,
                },
                _,
            ) => {
                let name = |mode: &ModeSpec| {
                    let step = mode.final_step();
                    Pitch::new(step, mode.scale_alter(step), 4).class_name(lang)
                };
                let (theirs, ours) = (name(chant), name(mass));
                match lang {
                    Lang::De => format!(
                        "{movement}: Der Choral hat die Finalis {theirs}, die Messe {ours}; seine \
                         Stufen werden von {ours} aus gesungen"
                    ),
                    Lang::En => format!(
                        "{movement}: the chant's final is {theirs}, the mass's {ours}; its degrees \
                         are sung from {ours}"
                    ),
                }
            }
            (Warning::ChantIV, Lang::De) => "Choral IV: Das Kyriale hat nur ein Kyrie IV; die \
                                             übrigen Sätze paraphrasieren die Choräle der Messe IX \
                                             und das Credo I"
                .to_string(),
            (Warning::ChantIV, Lang::En) => "Chant IV: the Kyriale has a Kyrie IV only; the other \
                                             movements paraphrase the chants of Mass IX and Credo I"
                .to_string(),
            (Warning::LengthFallback { part }, Lang::De) => format!(
                "Teil {part}: mit zwei Imitationspunkten mehr nicht zu setzen; gesetzt mit den \
                 Punkten, die seine Imitation selbst plant (wie --lengths natürlich)"
            ),
            (Warning::LengthFallback { part }, Lang::En) => format!(
                "Part {part}: not to be set with two more points of imitation; set with the \
                 points its imitation plans by itself (as --lengths natürlich)"
            ),
            (Warning::ScoringFallback { section }, Lang::De) => format!(
                "Abschnitt {section}: mit der hinzugefügten Stimme nicht zu setzen; gesetzt in \
                 der Besetzung der Messe"
            ),
            (Warning::ScoringFallback { section }, Lang::En) => format!(
                "Section {section}: not to be set with the added voice; set in the choir of the \
                 mass"
            ),
            (Warning::MotifTogether { movement }, Lang::De) => format!(
                "{movement}: Der Satz beginnt gemeinsam und trägt deshalb kein Kopfmotiv; nur ein \
                 Anfang in Imitation nimmt es auf"
            ),
            (Warning::MotifTogether { movement }, Lang::En) => format!(
                "{movement}: the movement begins together and so carries no head motif; only an \
                 opening in imitation takes it up"
            ),
        }
    }
}

/// Why nothing could be generated.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum GenerateError {
    #[error("no solution: the rules forbid every continuation at note {position}")]
    NoSolution { position: usize },
    #[error("no solution: the rules forbid every continuation in measure {measure} (mostly {})", rules.join(", "))]
    NoCounterpoint {
        measure: usize,
        /// The rules that cut off the most candidates there, most frequent first.
        rules: Vec<&'static str>,
    },
    /// The search was cancelled through its [`SearchControl`].
    #[error("the search was cancelled")]
    Cancelled,
    #[error("invalid request: {0}")]
    Invalid(String),
    /// A text that does not fit into the given length: at least this many measures leave every
    /// voice a minim beat for each syllable.
    #[error("the text needs at least {measures} measures")]
    TextTooLong { measures: usize },
    /// Free counterpoint with fewer than two voices.
    #[error("free counterpoint needs two voices or more")]
    TooFewVoices,
    /// A free setting shorter than its least length.
    #[error("free counterpoint needs {least} measures or more")]
    TooShort { least: usize },
    /// An internal cadence that leaves no room for its clausula: before the third measure, or
    /// less than three measures after the cadence before it or before the final cadence. The
    /// measure (from 1) its target arrives in.
    #[error(
        "the internal cadence in measure {measure} leaves no room for its clausula: the first comes in measure 3 or later, and three measures lie between two cadences and before the final one"
    )]
    CadenceTooClose { measure: usize },
    /// An internal cadence on a degree of the mode with a diminished fifth above.
    #[error("no cadence on {} in this mode (diminished fifth)", target.letter())]
    NoClausula { target: Step },
    /// A part asked to end on a degree of `mode` other than those a part may end on: the final,
    /// or the fifth or the fourth where the mode has a clausula on them
    /// ([`part_endings`]).
    #[error("{}", ending_message(*mode, *target, Lang::En))]
    Ending { mode: ModeSpec, target: Step },
    /// A part asked to begin after a silence of `start` ticks, other than none, a minim or a
    /// whole measure.
    #[error("{}", start_message(*start, Lang::En))]
    Start { start: u32 },
    /// An opening chord in an imitative setting.
    #[error(
        "an opening chord brings in every voice at once, an imitation one voice after the other; they do not go together"
    )]
    ChordWithImitation,
    /// An opening chord with another number of pitches than the setting has voices.
    #[error("the opening chord has {pitches} pitches for {voices} voices")]
    ChordVoices { pitches: usize, voices: usize },
    /// A text without syllables.
    #[error("a text without syllables")]
    EmptyText,
    /// An imitation that nothing planned and searched could set: the point it fails in (from 1)
    /// with its words, the measure (from 1) and the rules that cut off the most there, most
    /// frequent first.
    #[error(
        "no solution with imitation: point {point} (\"{words}\") fails in measure {measure}{}",
        mostly(rules, Lang::En)
    )]
    NoImitation {
        point: usize,
        words: String,
        measure: usize,
        rules: Vec<&'static str>,
    },
    /// An imitation that does not fit into the given length: the shortest plan found, compressed
    /// and planned anew, has this many measures.
    #[error("no plan of the imitation fits the given length; the shortest has {measures} measures")]
    ImitationTooLong { measures: usize },
    /// Imitation without a text.
    #[error("imitation needs a text: its points follow the words")]
    ImitationNeedsText,
    /// Imitation with given internal cadences: it places them itself, at the ends of its points.
    #[error(
        "imitation places the internal cadences itself, at the ends of its points; they cannot be given"
    )]
    ImitationCadences,
    /// A given subject that cannot be imitated: the subject (from 1), the note (from 1) at which
    /// it fails and the rule it breaks there.
    #[error("the given subject {subject} fails at note {note} ({rule})")]
    SubjectInvalid {
        subject: usize,
        note: usize,
        rule: &'static str,
    },
    /// A phrase (from 1) that the chant does not have, and how many it has.
    #[error("the chant has no phrase {phrase}, only {phrases}")]
    NoChantPhrase { phrase: usize, phrases: usize },
    /// A number of points of imitation that the text does not allow: it has room for one to
    /// `most`.
    #[error("{}", point_count(*most, Lang::En))]
    PointCount { most: usize },
    /// An option of the imitation that the request cannot have.
    #[error("{}", .0.message(Lang::En))]
    ImitationOption(OptionError),
    /// An option of a mass that the plan cannot have (phase 14): it fails before any part is
    /// searched.
    #[error("{}", .0.message(Lang::En))]
    MassOption(MassOptionError),
    /// A part of a mass that no attempt could set, its fallbacks included (phase 14): the part by
    /// its address (`credo/3`) and the beginning and the end of its words, the attempts made, the
    /// error of the last (with the measure and the rules where the search ended) and the options
    /// that may help besides another seed, as the command line writes them
    /// (`--imitation credo/3=aus`, `--endings credo/3=D`, `--chunk 30`).
    #[error(
        "{}",
        part_failure(part, words, *attempts, error, remedies, Lang::En)
    )]
    MassPart {
        part: String,
        words: String,
        attempts: u8,
        error: Box<GenerateError>,
        remedies: Vec<String>,
    },
}

impl GenerateError {
    /// Message for users in their language.
    pub fn message(&self, lang: Lang) -> String {
        match (self, lang) {
            (GenerateError::NoSolution { position }, Lang::De) => {
                format!("Keine Lösung: Die Regeln verbieten jede Fortsetzung bei Note {position}.")
            }
            (GenerateError::NoCounterpoint { measure, rules }, Lang::De) => format!(
                "Keine Lösung: Die Regeln verbieten jede Fortsetzung in Takt {measure} (vor allem {}).",
                rules.join(", ")
            ),
            (GenerateError::Cancelled, Lang::De) => "Die Suche wurde abgebrochen.".to_string(),
            (GenerateError::Invalid(reason), Lang::De) => format!("Ungültige Anfrage: {reason}"),
            (GenerateError::TextTooLong { measures }, Lang::De) => {
                format!("Der Text braucht mindestens {measures} Takte.")
            }
            (GenerateError::TooFewVoices, Lang::De) => {
                "Ein freier Satz braucht zwei Stimmen oder mehr.".to_string()
            }
            (GenerateError::TooShort { least }, Lang::De) => {
                format!("Ein freier Satz braucht mindestens {least} Takte.")
            }
            (GenerateError::CadenceTooClose { measure }, Lang::De) => format!(
                "Der Binnenschluss in Takt {measure} lässt seiner Klausel keinen Platz: Der erste \
                 kommt frühestens in Takt 3, und zwischen zwei Binnenschlüssen und vor dem \
                 Schlusstakt liegen mindestens drei Takte."
            ),
            (GenerateError::NoClausula { target }, Lang::De) => format!(
                "Kein Binnenschluss auf {} in diesem Modus (verminderte Quinte).",
                target.letter()
            ),
            (GenerateError::Ending { mode, target }, Lang::De) => {
                format!("{}.", ending_message(*mode, *target, Lang::De))
            }
            (GenerateError::Start { start }, Lang::De) => {
                format!("{}.", start_message(*start, Lang::De))
            }
            (GenerateError::ChordWithImitation, Lang::De) => {
                "Ein Anfangsklang lässt alle Stimmen zugleich einsetzen, eine Imitation eine nach \
                 der anderen; beides zusammen geht nicht."
                    .to_string()
            }
            (GenerateError::ChordVoices { pitches, voices }, Lang::De) => {
                format!("Der Anfangsklang hat {pitches} Töne für {voices} Stimmen.")
            }
            (GenerateError::EmptyText, Lang::De) => "Ein Text ohne Silben.".to_string(),
            (
                GenerateError::NoImitation {
                    point,
                    words,
                    measure,
                    rules,
                },
                Lang::De,
            ) => format!(
                "Keine Lösung mit Imitation: Punkt {point} („{words}“) scheitert in Takt {measure}{}.",
                mostly(rules, Lang::De)
            ),
            (GenerateError::ImitationTooLong { measures }, Lang::De) => format!(
                "Kein Plan der Imitation passt in die gegebene Länge; der kürzeste hat {measures} Takte."
            ),
            (GenerateError::ImitationNeedsText, Lang::De) => {
                "Imitation braucht einen Text, denn ihre Punkte folgen den Worten.".to_string()
            }
            (GenerateError::ImitationCadences, Lang::De) => {
                "Die Imitation setzt die Binnenschlüsse selbst, an die Enden ihrer Punkte; sie \
                 lassen sich nicht vorgeben."
                    .to_string()
            }
            (
                GenerateError::SubjectInvalid {
                    subject,
                    note,
                    rule,
                },
                Lang::De,
            ) => format!("Das vorgegebene Soggetto {subject} scheitert an Note {note} ({rule})."),
            (GenerateError::NoChantPhrase { phrase, phrases }, Lang::De) => {
                format!("Der Choral hat keine Phrase {phrase}, nur {phrases}.")
            }
            (GenerateError::PointCount { most }, Lang::De) => {
                format!("{}.", point_count(*most, Lang::De))
            }
            (GenerateError::ImitationOption(option), Lang::De) => {
                format!("{}.", option.message(Lang::De))
            }
            (GenerateError::MassOption(option), Lang::De) => {
                format!("{}.", option.message(Lang::De))
            }
            (
                GenerateError::MassPart {
                    part,
                    words,
                    attempts,
                    error,
                    remedies,
                },
                Lang::De,
            ) => format!(
                "{}.",
                part_failure(part, words, *attempts, error, remedies, Lang::De)
            ),
            (error, Lang::En) => format!("{}{}", capitalized(&error.to_string()), "."),
        }
    }
}

/// Why a part of a mass could not be set, for a message without its full stop: the part and its
/// words, the attempts, the message of the last attempt's error (with its measure and rules) and
/// what may help: another seed first, then `remedies`.
fn part_failure(
    part: &str,
    words: &str,
    attempts: u8,
    error: &GenerateError,
    remedies: &[String],
    lang: Lang,
) -> String {
    let why = error.message(lang);
    let mut options = vec![
        lang.pick("ein anderer Seed (--seed)", "another seed (--seed)")
            .to_string(),
    ];
    options.extend(remedies.iter().cloned());
    let list = match options.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} {} {last}", rest.join(", "), lang.pick("oder", "or")),
        None => String::new(),
    };
    match lang {
        Lang::De => format!(
            "Teil {part} („{words}“) ließ sich in {attempts} Versuchen nicht setzen. {why} \
             Abhilfe: {list}"
        ),
        Lang::En => format!(
            "part {part} (\"{words}\") could not be set in {attempts} attempts. {why} Remedies: \
             {list}"
        ),
    }
}

/// Why a part cannot end on `target` in `mode`, for a message without its full stop: the degrees
/// it may end on instead, spelled in the mode.
fn ending_message(mode: ModeSpec, target: Step, lang: Lang) -> String {
    let name = |step: Step| Pitch::new(step, mode.scale_alter(step), 4).class_name(lang);
    let names: Vec<String> = part_endings(mode).into_iter().map(name).collect();
    let allowed = match names.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} {} {last}", rest.join(", "), lang.pick("oder", "or")),
        None => String::new(),
    };
    match lang {
        Lang::De => format!(
            "Ein Satz in diesem Modus schließt auf {allowed} (Finalis, Quinte oder Quarte mit \
             einer Klausel), nicht auf {}",
            name(target)
        ),
        Lang::En => format!(
            "a setting in this mode ends on {allowed} (the final, or the fifth or the fourth with \
             a clausula), not on {}",
            name(target)
        ),
    }
}

/// Why a part cannot begin after `start` ticks, for a message without its full stop.
fn start_message(start: u32, lang: Lang) -> String {
    let measures = f64::from(start) / 8.0;
    match lang {
        Lang::De => format!(
            "Ein Satz beginnt sofort, nach einer Halben oder nach einem ganzen Takt Pause, nicht \
             nach {} Takten",
            measures.to_string().replace('.', ",")
        ),
        Lang::En => format!(
            "a setting begins at once, after a minim or after a whole measure's rest, not after \
             {measures} measures"
        ),
    }
}

/// How many points of imitation a text has room for, for a message.
fn point_count(most: usize, lang: Lang) -> String {
    match (most, lang) {
        (1, Lang::De) => "Der Text hat nur für einen Imitationspunkt Platz".to_string(),
        (1, Lang::En) => "the text has room for one point of imitation only".to_string(),
        (most, Lang::De) => format!("Der Text hat Platz für 1 bis {most} Imitationspunkte"),
        (most, Lang::En) => format!("the text has room for 1 to {most} points of imitation"),
    }
}

/// The rules that cut off the most, for a message: ` (mostly MEL-TRITONE, TXT-ORDER)`, or nothing
/// without rules.
fn mostly(rules: &[&str], lang: Lang) -> String {
    if rules.is_empty() {
        return String::new();
    }
    format!(
        " ({} {})",
        lang.pick("vor allem", "mostly"),
        rules.join(", ")
    )
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The errors of imitation speak German and English.
    #[test]
    fn imitation_errors_in_both_languages() {
        let failed = |rules: &[&'static str]| GenerateError::NoImitation {
            point: 2,
            words: "e-lé-i-son.".into(),
            measure: 14,
            rules: rules.to_vec(),
        };
        for (error, de, en) in [
            (
                failed(&["TXT-ORDER", "MEL-LEAP"]),
                "Keine Lösung mit Imitation: Punkt 2 („e-lé-i-son.“) scheitert in Takt 14 (vor \
                 allem TXT-ORDER, MEL-LEAP).",
                "No solution with imitation: point 2 (\"e-lé-i-son.\") fails in measure 14 \
                 (mostly TXT-ORDER, MEL-LEAP).",
            ),
            (
                failed(&[]),
                "Keine Lösung mit Imitation: Punkt 2 („e-lé-i-son.“) scheitert in Takt 14.",
                "No solution with imitation: point 2 (\"e-lé-i-son.\") fails in measure 14.",
            ),
            (
                GenerateError::ImitationTooLong { measures: 31 },
                "Kein Plan der Imitation passt in die gegebene Länge; der kürzeste hat 31 Takte.",
                "No plan of the imitation fits the given length; the shortest has 31 measures.",
            ),
            (
                GenerateError::ImitationNeedsText,
                "Imitation braucht einen Text, denn ihre Punkte folgen den Worten.",
                "Imitation needs a text: its points follow the words.",
            ),
            (
                GenerateError::ImitationCadences,
                "Die Imitation setzt die Binnenschlüsse selbst, an die Enden ihrer Punkte; sie \
                 lassen sich nicht vorgeben.",
                "Imitation places the internal cadences itself, at the ends of its points; they \
                 cannot be given.",
            ),
            (
                GenerateError::SubjectInvalid {
                    subject: 1,
                    note: 3,
                    rule: "MEL-TRITONE",
                },
                "Das vorgegebene Soggetto 1 scheitert an Note 3 (MEL-TRITONE).",
                "The given subject 1 fails at note 3 (MEL-TRITONE).",
            ),
            (
                GenerateError::NoChantPhrase {
                    phrase: 7,
                    phrases: 5,
                },
                "Der Choral hat keine Phrase 7, nur 5.",
                "The chant has no phrase 7, only 5.",
            ),
            (
                GenerateError::PointCount { most: 6 },
                "Der Text hat Platz für 1 bis 6 Imitationspunkte.",
                "The text has room for 1 to 6 points of imitation.",
            ),
            (
                GenerateError::PointCount { most: 1 },
                "Der Text hat nur für einen Imitationspunkt Platz.",
                "The text has room for one point of imitation only.",
            ),
        ] {
            assert_eq!(error.message(Lang::De), de);
            assert_eq!(error.message(Lang::En), en);
        }
    }

    /// The errors of the parts of a mass (phase 14) speak German and English; an ending names
    /// the degrees the mode allows as they are spelled in it: B♭ is B in German.
    #[test]
    fn part_errors_in_both_languages() {
        let mode = |name: &str| -> ModeSpec { name.parse().unwrap() };
        for (error, de, en) in [
            (
                GenerateError::Cancelled,
                "Die Suche wurde abgebrochen.",
                "The search was cancelled.",
            ),
            (
                GenerateError::Ending {
                    mode: mode("ionian-transposed"),
                    target: Step::D,
                },
                "Ein Satz in diesem Modus schließt auf F, C oder B (Finalis, Quinte oder Quarte \
                 mit einer Klausel), nicht auf D.",
                "A setting in this mode ends on F, C or Bb (the final, or the fifth or the fourth \
                 with a clausula), not on D.",
            ),
            (
                GenerateError::Ending {
                    mode: mode("hypophrygian"),
                    target: Step::C,
                },
                "Ein Satz in diesem Modus schließt auf E oder A (Finalis, Quinte oder Quarte mit \
                 einer Klausel), nicht auf C.",
                "A setting in this mode ends on E or A (the final, or the fifth or the fourth \
                 with a clausula), not on C.",
            ),
            (
                GenerateError::Ending {
                    mode: mode("lydian"),
                    target: Step::B,
                },
                "Ein Satz in diesem Modus schließt auf F oder C (Finalis, Quinte oder Quarte mit \
                 einer Klausel), nicht auf H.",
                "A setting in this mode ends on F or C (the final, or the fifth or the fourth \
                 with a clausula), not on B.",
            ),
            (
                GenerateError::Start { start: 2 },
                "Ein Satz beginnt sofort, nach einer Halben oder nach einem ganzen Takt Pause, \
                 nicht nach 0,25 Takten.",
                "A setting begins at once, after a minim or after a whole measure's rest, not \
                 after 0.25 measures.",
            ),
            (
                GenerateError::ChordWithImitation,
                "Ein Anfangsklang lässt alle Stimmen zugleich einsetzen, eine Imitation eine nach \
                 der anderen; beides zusammen geht nicht.",
                "An opening chord brings in every voice at once, an imitation one voice after the \
                 other; they do not go together.",
            ),
            (
                GenerateError::ChordVoices {
                    pitches: 3,
                    voices: 4,
                },
                "Der Anfangsklang hat 3 Töne für 4 Stimmen.",
                "The opening chord has 3 pitches for 4 voices.",
            ),
        ] {
            assert_eq!(error.message(Lang::De), de);
            assert_eq!(error.message(Lang::En), en);
        }
    }

    /// The errors and warnings of a mass speak German and English: an option of the mass as a
    /// sentence of the generator, and a part that no attempt set with its place, its words, the
    /// last attempt's error and the options that may help, another seed first.
    #[test]
    fn mass_errors_and_warnings_in_both_languages() {
        let option = GenerateError::MassOption(MassOptionError::Voices { voices: 7 });
        assert_eq!(
            option.message(Lang::De),
            "Eine Messe braucht zwei bis sechs Stimmen, nicht 7."
        );
        assert_eq!(
            option.message(Lang::En),
            "A mass needs two to six voices, not 7."
        );
        let failed = GenerateError::MassPart {
            part: "credo/3".into(),
            words: "Et in u-num Dó-mi-num … sǽ-cu-la.".into(),
            attempts: 3,
            error: Box::new(GenerateError::NoCounterpoint {
                measure: 23,
                rules: vec!["TXT-ORDER"],
            }),
            remedies: vec![
                "--imitation credo/3=aus".into(),
                "--endings credo/3=D".into(),
                "--chunk 30".into(),
            ],
        };
        assert_eq!(
            failed.message(Lang::De),
            "Teil credo/3 („Et in u-num Dó-mi-num … sǽ-cu-la.“) ließ sich in 3 Versuchen nicht \
             setzen. Keine Lösung: Die Regeln verbieten jede Fortsetzung in Takt 23 (vor allem \
             TXT-ORDER). Abhilfe: ein anderer Seed (--seed), --imitation credo/3=aus, --endings \
             credo/3=D oder --chunk 30."
        );
        assert_eq!(
            failed.message(Lang::En),
            "Part credo/3 (\"Et in u-num Dó-mi-num … sǽ-cu-la.\") could not be set in 3 \
             attempts. No solution: the rules forbid every continuation in measure 23 (mostly \
             TXT-ORDER). Remedies: another seed (--seed), --imitation credo/3=aus, --endings \
             credo/3=D or --chunk 30."
        );
        let alone = GenerateError::MassPart {
            part: "kyrie/2".into(),
            words: "Chri-ste e-lé-i-son.".into(),
            attempts: 6,
            error: Box::new(GenerateError::Cancelled),
            remedies: Vec::new(),
        };
        assert!(
            alone
                .message(Lang::En)
                .ends_with("Remedies: another seed (--seed).")
        );
        for (warning, de, en) in [
            (
                Warning::LengthFallback {
                    part: "benedictus/1".into(),
                },
                "Teil benedictus/1: mit zwei Imitationspunkten mehr nicht zu setzen; gesetzt mit \
                 den Punkten, die seine Imitation selbst plant (wie --lengths natürlich)",
                "Part benedictus/1: not to be set with two more points of imitation; set with the \
                 points its imitation plans by itself (as --lengths natürlich)",
            ),
            (
                Warning::ScoringFallback {
                    section: "agnus3".into(),
                },
                "Abschnitt agnus3: mit der hinzugefügten Stimme nicht zu setzen; gesetzt in der \
                 Besetzung der Messe",
                "Section agnus3: not to be set with the added voice; set in the choir of the mass",
            ),
            (
                Warning::MotifFallback {
                    movement: "Gloria".into(),
                },
                "Gloria: Mit dem Kopfmotiv fand der erste Punkt keinen Plan; sein Soggetto kommt \
                 aus dem Text",
                "Gloria: the first point found no plan with the head motif; its subject comes \
                 from the text",
            ),
        ] {
            assert_eq!(warning.message(Lang::De), de);
            assert_eq!(warning.message(Lang::En), en);
        }
    }
}
