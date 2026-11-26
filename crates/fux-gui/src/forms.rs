//! The inputs of the tasks as the window edits them, and their translation into the options of
//! the program `fux` (`fux_cli`), so that both build the same requests.

use std::sync::Arc;

use fux_cli::book::BookOptions;
use fux_cli::commands::{CounterpointOptions, least_cantus};
use fux_cli::free::{CadencePlan, FreeOptions, Repeat, SingingArgs, parse_parts, prepare_free};
use fux_cli::imitation::{
    AutoCount, ChantRef, ImitationMode, ImitationOptions, PointVoices, SubjectChoice, Terms,
    form_message, phrase_list,
};
use fux_cli::input::{Cadences, ScoreInput};
use fux_cli::mass::{MassNotation, MassOptions};
use fux_cli::text::{DEFAULT_TEXT, is_none, restates, text_of};
use fux_core::gabc::CadenceBars;
use fux_core::generate::{GenerateError, MIN_MEASURES};
use fux_core::imitation::{AnswerChoice, Toggle};
use fux_core::mass::{ChantMass, Lengths, MassRequest, Movement, Preset, preset, tables};
use fux_core::rules::Profile;
use fux_core::text::plan::plan;
use fux_core::text::{Eleison, OWN_ID, Text};
use fux_core::{Lang, ModeSpec, VoiceType};
use serde::{Deserialize, Serialize};

use crate::jobs::CheckJob;

/// [`FreeForm::text`] for the own text of [`FreeForm::own_text`].
pub const OWN_TEXT: &str = OWN_ID;

/// [`FreeForm::text`] for voices without a text, as before phase 12.
pub const NO_TEXT: &str = "none";

/// Where the cantus firmus of a setting comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceKind {
    /// One of Fux's cantus firmi.
    Fux,
    /// Notes typed in: note names (`D4 F4 E4`) or LilyPond (`d'1 f' e'`).
    Notes,
    /// A chant in a GABC file, or a melody in MusicXML.
    Gabc,
    /// A cantus generated in a mode.
    Generated,
}

impl SourceKind {
    pub const ALL: [SourceKind; 4] = [
        SourceKind::Fux,
        SourceKind::Notes,
        SourceKind::Gabc,
        SourceKind::Generated,
    ];

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            SourceKind::Fux => lang.pick("Cantus von Fux", "Cantus by Fux"),
            SourceKind::Notes => lang.pick("Eigene Noten", "Own notes"),
            SourceKind::Gabc => lang.pick("Datei (GABC, MusicXML)", "File (GABC, MusicXML)"),
            SourceKind::Generated => lang.pick("Erzeugen", "Generate"),
        }
    }
}

/// Where internal cadences go (`--cadences`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CadenceChoice {
    /// At the half and full bars of a chant.
    Auto,
    /// At full bars only.
    Full,
    /// At every bar from the quarter bar on.
    All,
    None,
    /// In the measures typed in.
    Measures,
}

impl CadenceChoice {
    pub const ALL: [CadenceChoice; 5] = [
        CadenceChoice::Auto,
        CadenceChoice::Full,
        CadenceChoice::All,
        CadenceChoice::None,
        CadenceChoice::Measures,
    ];

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            CadenceChoice::Auto => lang.pick("an Halb- und Ganzstrichen", "at half and full bars"),
            CadenceChoice::Full => lang.pick("nur an Ganzstrichen", "at full bars only"),
            CadenceChoice::All => lang.pick("auch an Viertelstrichen", "also at quarter bars"),
            CadenceChoice::None => lang.pick("keine", "none"),
            CadenceChoice::Measures => lang.pick("in den Takten …", "in the measures …"),
        }
    }

    /// The setting of `fux`, with `measures` such as `8, 15` for [`CadenceChoice::Measures`].
    fn cadences(self, measures: &str) -> Result<Cadences, String> {
        Ok(match self {
            CadenceChoice::Auto => Cadences::Bars(CadenceBars::Half),
            CadenceChoice::Full => Cadences::Bars(CadenceBars::Full),
            CadenceChoice::All => Cadences::Bars(CadenceBars::Quarter),
            CadenceChoice::None => Cadences::Bars(CadenceBars::None),
            CadenceChoice::Measures => measures.parse()?,
        })
    }
}

/// Stores a mode by its id (`dorian`, `hypophrygian-transposed`).
mod mode_id {
    use fux_core::ModeSpec;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(mode: &ModeSpec, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&mode.id())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<ModeSpec, D::Error> {
        let id = String::deserialize(d)?;
        id.parse().map_err(serde::de::Error::custom)
    }
}

/// Stores an optional mode by its id, or the empty string.
mod optional_mode_id {
    use fux_core::ModeSpec;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(mode: &Option<ModeSpec>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&mode.map(|m| m.id()).unwrap_or_default())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<ModeSpec>, D::Error> {
        let id = String::deserialize(d)?;
        if id.is_empty() {
            return Ok(None);
        }
        id.parse().map(Some).map_err(serde::de::Error::custom)
    }
}

/// Stores an optional voice type by its abbreviation, or the empty string.
mod optional_voice {
    use fux_core::VoiceType;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(voice: &Option<VoiceType>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(voice.map_or("", |v| v.abbreviation()))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<VoiceType>, D::Error> {
        let text = String::deserialize(d)?;
        if text.is_empty() {
            return Ok(None);
        }
        text.parse().map(Some).map_err(serde::de::Error::custom)
    }
}

/// Stores a voice type by its abbreviation.
mod voice {
    use fux_core::VoiceType;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(voice: &VoiceType, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(voice.abbreviation())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<VoiceType, D::Error> {
        let text = String::deserialize(d)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// Stores the repetition of a text as `--repeat` takes it: `auto`, `keine` or `frei`.
mod repeat_name {
    use fux_cli::free::Repeat;
    use fux_core::Lang;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(repeat: &Repeat, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(repeat.name(Lang::De))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Repeat, D::Error> {
        let text = String::deserialize(d)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// Stores the form of a mass as `--preset` takes it: `brevis` or `normal`.
mod preset_id {
    use fux_core::mass::Preset;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(preset: &Preset, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(preset.id())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Preset, D::Error> {
        let id = String::deserialize(d)?;
        id.parse().map_err(serde::de::Error::custom)
    }
}

/// Stores the lengths of a mass as `--lengths` takes them: `palestrina` or `natürlich`.
mod lengths_id {
    use fux_core::mass::Lengths;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(lengths: &Lengths, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(lengths.id())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Lengths, D::Error> {
        let id = String::deserialize(d)?;
        id.parse().map_err(serde::de::Error::custom)
    }
}

/// Stores the chant a mass paraphrases as `--chant` takes it: `ix` or `iv`, or the empty string
/// for none.
mod chant_id {
    use fux_core::mass::{ChantMass, parse_chant};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(chant: &Option<ChantMass>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(chant.map_or("", ChantMass::id))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<ChantMass>, D::Error> {
        let id = String::deserialize(d)?;
        if id.is_empty() {
            return Ok(None);
        }
        parse_chant(&id).map_err(serde::de::Error::custom)
    }
}

/// The inputs of `fux cp`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CounterpointForm {
    pub source: SourceKind,
    /// Id of Fux's cantus, e.g. `dorian`.
    pub fux_cantus: String,
    pub notes: String,
    /// Path of a GABC or MusicXML file.
    pub gabc: String,
    /// Mode and length of a generated cantus.
    #[serde(with = "mode_id")]
    pub generated_mode: ModeSpec,
    pub length: usize,
    /// The mode, if not taken from the cantus.
    #[serde(with = "optional_mode_id")]
    pub mode: Option<ModeSpec>,
    /// The cantus' voice, if not chosen by its range.
    #[serde(with = "optional_voice")]
    pub cf_voice: Option<VoiceType>,
    pub cadences: CadenceChoice,
    /// Measures with internal cadences, e.g. `8, 15`.
    pub cadence_measures: String,
    /// Number of voices with the cantus.
    pub voices: usize,
    /// Voice types from top to bottom, e.g. `S A T B`; empty for a usual choir.
    pub parts: String,
    pub species: u8,
    /// The voice in the species, if not the highest (or, in two voices, the neighbour).
    #[serde(with = "optional_voice")]
    pub voice: Option<VoiceType>,
    /// In two voices: the counterpoint below the cantus.
    pub below: bool,
    /// A new seed for every run.
    pub random_seed: bool,
    pub seed: u64,
    pub count: usize,
    pub beam: usize,
    pub noise: f64,
    /// What the free voices sing in free counterpoint (species 0), as [`FreeForm::text`] says
    /// it; [`NO_TEXT`] by default, also for settings stored before phase 13. The cantus keeps
    /// its own syllables, if it has any.
    pub text: String,
    /// An own Latin text, with hyphens and acutes where the syllabifier needs them.
    pub own_text: String,
    /// How freely the free voices restate the text (`--repeat`).
    #[serde(with = "repeat_name")]
    pub repeat: Repeat,
    /// The syllables keep the acutes of the liturgical books (`--accents`).
    pub accents: bool,
    /// eleison in three syllables, e-léi-son (`--eleison 3`).
    pub eleison3: bool,
}

impl Default for CounterpointForm {
    fn default() -> Self {
        CounterpointForm {
            source: SourceKind::Fux,
            fux_cantus: "dorian".into(),
            notes: "D4 F4 E4 D4 G4 F4 A4 G4 F4 E4 D4".into(),
            gabc: String::new(),
            generated_mode: "dorian".parse().expect("valid mode"),
            length: 11,
            mode: None,
            cf_voice: None,
            cadences: CadenceChoice::Auto,
            cadence_measures: String::new(),
            voices: 2,
            parts: String::new(),
            species: 1,
            voice: None,
            below: false,
            random_seed: true,
            seed: 1,
            count: 3,
            beam: 200,
            noise: 2.0,
            text: NO_TEXT.into(),
            own_text: String::new(),
            repeat: Repeat::Auto,
            accents: true,
            eleison3: false,
        }
    }
}

/// What the free voices of a counterpoint form will sing, as far as it is known before the
/// search.
#[derive(Clone, Debug)]
pub struct CounterpointPreview {
    /// The text the free voices sing; `None` without a text.
    pub text: Option<Arc<Text>>,
    /// Warnings about the text in the user's language, such as uncertain accents.
    pub warnings: Vec<String>,
    /// The fewest measures of a cantus around which the voices can sing the text.
    pub least: usize,
}

impl CounterpointForm {
    /// The text as `fux cp --text` takes it: a section, some of its sense units, the own text,
    /// or `none`.
    pub fn text_spec(&self) -> &str {
        spec_of(&self.text, &self.own_text)
    }

    /// The division of eleison.
    pub fn eleison(&self) -> Eleison {
        eleison_of(self.eleison3)
    }

    /// The text of the free voices with its warnings, and the cantus it needs. An error says
    /// what is wrong with the text.
    pub fn preview(&self, lang: Lang) -> Result<CounterpointPreview, String> {
        let sung = text_of(self.text_spec(), self.eleison(), lang).map_err(|e| format!("{e:#}"))?;
        Ok(match sung {
            None => CounterpointPreview {
                text: None,
                warnings: Vec::new(),
                least: MIN_MEASURES,
            },
            Some((text, warnings)) => {
                let mode = self.mode.unwrap_or(self.generated_mode);
                CounterpointPreview {
                    least: least_cantus(&text, mode),
                    text: Some(Arc::new(text)),
                    warnings,
                }
            }
        })
    }

    /// The options of `fux cp`; `seed` is the seed of this run. Only free counterpoint sings
    /// the text.
    pub fn options(&self, seed: u64) -> Result<CounterpointOptions, String> {
        let cf = match self.source {
            SourceKind::Fux => Some(format!("fux:{}", self.fux_cantus)),
            SourceKind::Notes => Some(self.notes.trim().to_string()),
            SourceKind::Gabc => Some(self.gabc.trim().to_string()),
            SourceKind::Generated => None,
        };
        let mode = match self.source {
            SourceKind::Generated => Some(self.generated_mode),
            _ => self.mode,
        };
        let parts = self.parts.trim();
        Ok(CounterpointOptions {
            cf,
            mode,
            cf_voice: self.cf_voice,
            cadences: self.cadences.cadences(&self.cadence_measures)?,
            length: self.length,
            species: self.species,
            voices: self.voices,
            parts: (!parts.is_empty()).then(|| parts.to_string()),
            below: self.below,
            voice: self.voice,
            seed: Some(seed),
            count: self.count,
            beam: self.beam,
            noise: self.noise,
            details: false,
            text: if self.species == 0 {
                self.text_spec().to_string()
            } else {
                NO_TEXT.to_string()
            },
            repeat: self.repeat,
            accents: self.accents,
            eleison: self.eleison(),
        })
    }
}

/// The text as `--text` takes it from the fields of a form: the own text for [`OWN_TEXT`],
/// otherwise the text chosen.
fn spec_of<'a>(text: &'a str, own_text: &'a str) -> &'a str {
    if text == OWN_TEXT { own_text } else { text }
}

/// The division of eleison: three syllables if `three`, otherwise the four of the books.
fn eleison_of(three: bool) -> Eleison {
    if three { Eleison::Three } else { Eleison::Four }
}

/// A choice that the planner makes unless the form makes it (`auto`, `an`, `aus`): whether the
/// voices imitate, whether a short text sung twice restates its first point, whether four voices
/// enter in pairs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Switch {
    #[default]
    Auto,
    On,
    Off,
}

impl Switch {
    pub const ALL: [Switch; 3] = [Switch::Auto, Switch::On, Switch::Off];

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Switch::Auto => "auto",
            Switch::On => lang.pick("an", "on"),
            Switch::Off => lang.pick("aus", "off"),
        }
    }

    fn toggle(self) -> Toggle {
        match self {
            Switch::Auto => Toggle::Auto,
            Switch::On => Toggle::On,
            Switch::Off => Toggle::Off,
        }
    }
}

/// How the entries of a point answer its leader (`--answer`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnswerKind {
    /// Real, now and then modal.
    #[default]
    Auto,
    Real,
    Modal,
}

impl AnswerKind {
    pub const ALL: [AnswerKind; 3] = [AnswerKind::Auto, AnswerKind::Real, AnswerKind::Modal];

    pub fn label(self) -> &'static str {
        match self {
            AnswerKind::Auto => "auto",
            AnswerKind::Real => "real",
            AnswerKind::Modal => "modal",
        }
    }
}

/// Where the subjects of the points come from (`--subject`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubjectKind {
    /// From the words of each point.
    #[default]
    Text,
    /// The chant of the Kyriale that sings the section of the Ordinary.
    Kyriale,
    /// A chant in a GABC file.
    File,
    /// Heads typed in as notes.
    Notes,
}

impl SubjectKind {
    pub const ALL: [SubjectKind; 4] = [
        SubjectKind::Text,
        SubjectKind::Kyriale,
        SubjectKind::File,
        SubjectKind::Notes,
    ];

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            SubjectKind::Text => lang.pick("aus dem Text", "from the text"),
            SubjectKind::Kyriale => lang.pick("Choral des Kyriale", "chant of the Kyriale"),
            SubjectKind::File => lang.pick("Choral aus Datei (GABC)", "chant from a file (GABC)"),
            SubjectKind::Notes => lang.pick("eigene Noten", "own notes"),
        }
    }
}

/// The voices of the points after the first (`--point-voices`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaterVoices {
    /// Two to all, by the shares of Palestrina's later points.
    #[default]
    Auto,
    All,
    Count(usize),
}

impl LaterVoices {
    pub fn label(self, lang: Lang) -> String {
        match self {
            LaterVoices::Auto => "auto".into(),
            LaterVoices::All => lang.pick("alle", "all").into(),
            LaterVoices::Count(n) => n.to_string(),
        }
    }
}

/// The imitation of a free setting as the window edits it (`fux free --imitation …`). Settings
/// stored before phase 13 lack it and get these defaults: imitation where it applies, everything
/// else as the planner chooses.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImitationForm {
    /// `auto`: with a text, three to five voices, automatic internal cadences and an automatic
    /// length.
    pub mode: Switch,
    pub subject: SubjectKind,
    /// The GABC file of [`SubjectKind::File`].
    pub chant_file: String,
    /// The phrases of the chant from 1, such as `2, 3` or `2-4`; empty to find them by the
    /// words.
    pub phrases: String,
    /// A chant's subjects are sung in the chant's mode instead of the form's.
    pub chant_mode: bool,
    /// The heads of [`SubjectKind::Notes`] in LilyPond, `|` between them.
    pub notes: String,
    /// The number of points; 0 for automatic.
    pub points: usize,
    /// The second statement of a short text as a restatement of the first point.
    pub restate: Switch,
    pub later_voices: LaterVoices,
    /// The order of the entries per point, such as `A S T B; T B`; empty for automatic.
    pub order: String,
    /// The intervals of the later entries to the leader per point, such as `d 0 d; 0`.
    pub intervals: String,
    /// The delays between the entries per point in measures, such as `1 2 1.5; 1`.
    pub delays: String,
    pub pairs: Switch,
    pub answer: AnswerKind,
    /// The notes of every head; 0 for automatic.
    pub head: usize,
}

impl Default for ImitationForm {
    fn default() -> Self {
        ImitationForm {
            mode: Switch::Auto,
            subject: SubjectKind::Text,
            chant_file: String::new(),
            phrases: String::new(),
            chant_mode: true,
            notes: String::new(),
            points: 0,
            restate: Switch::Auto,
            later_voices: LaterVoices::Auto,
            order: String::new(),
            intervals: String::new(),
            delays: String::new(),
            pairs: Switch::Auto,
            answer: AnswerKind::Auto,
            head: 0,
        }
    }
}

impl ImitationForm {
    /// Whether the subjects come from a chant.
    pub fn chant_subject(&self) -> bool {
        matches!(self.subject, SubjectKind::Kyriale | SubjectKind::File)
    }

    /// The imitation options of `fux free`, or what is wrong with them, named by the labels of
    /// the form.
    pub fn options(&self, lang: Lang) -> Result<ImitationOptions, String> {
        let field = |label: &str, error: String| format!("{label}: {error}");
        let phrases = match self.phrases.trim() {
            "" => None,
            list => Some(phrase_list(list).ok_or_else(|| {
                field(
                    lang.pick("Phrasen", "Phrases"),
                    lang.pick(
                        "Nummern ab 1 wie 2, 3 oder 2-4",
                        "numbers from 1 such as 2, 3 or 2-4",
                    )
                    .to_string(),
                )
            })?),
        };
        let subject = match self.subject {
            SubjectKind::Text => SubjectChoice::Text,
            SubjectKind::Kyriale => SubjectChoice::Chant {
                chant: ChantRef::Kyriale,
                phrases,
            },
            SubjectKind::File => match self.chant_file.trim() {
                "" => {
                    return Err(lang
                        .pick(
                            "Soggetto: bitte eine GABC-Datei wählen.",
                            "Subject: please choose a GABC file.",
                        )
                        .to_string());
                }
                file => SubjectChoice::Chant {
                    chant: ChantRef::Named(file.to_string()),
                    phrases,
                },
            },
            SubjectKind::Notes => match self.notes.trim() {
                "" => {
                    return Err(lang
                        .pick(
                            "Soggetto: bitte Noten eingeben, etwa d'1 d'2 a'2 g'4 f'4 e'2 d'1.",
                            "Subject: please enter notes, such as d'1 d'2 a'2 g'4 f'4 e'2 d'1.",
                        )
                        .to_string());
                }
                notes => match notes.parse::<SubjectChoice>() {
                    Ok(SubjectChoice::Notes(heads)) => SubjectChoice::Notes(heads),
                    Ok(_) => return Err(field("Soggetto", format!("{notes:?}"))),
                    Err(error) => return Err(field("Soggetto", error)),
                },
            },
        };
        Ok(ImitationOptions {
            mode: match self.mode {
                Switch::Auto => ImitationMode::Auto,
                Switch::On => ImitationMode::On,
                Switch::Off => ImitationMode::Off,
            },
            subject,
            points: AutoCount((self.points > 0).then_some(self.points)),
            restate: self.restate.toggle(),
            point_voices: match self.later_voices {
                LaterVoices::Auto => PointVoices::Auto,
                LaterVoices::All => PointVoices::All,
                LaterVoices::Count(n) => PointVoices::Count(n.max(2)),
            },
            order: self
                .order
                .parse()
                .map_err(|e| field(lang.pick("Reihenfolge", "Order"), e))?,
            intervals: self
                .intervals
                .parse()
                .map_err(|e| field(lang.pick("Intervalle", "Intervals"), e))?,
            delays: self
                .delays
                .parse()
                .map_err(|e| field(lang.pick("Abstände", "Delays"), e))?,
            pairs: self.pairs.toggle(),
            answer: match self.answer {
                AnswerKind::Auto => AnswerChoice::Auto,
                AnswerKind::Real => AnswerChoice::Real,
                AnswerKind::Modal => AnswerChoice::Modal,
            },
            head: AutoCount((self.head >= 4).then_some(self.head)),
            plan_only: false,
        })
    }
}

/// The inputs of `fux free`. Settings stored before phase 12 lack the text; they get Kyrie I
/// with the length it plans.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FreeForm {
    #[serde(with = "mode_id")]
    pub mode: ModeSpec,
    pub voices: usize,
    /// Voice types from top to bottom, e.g. `S A T B`; empty for the usual choir.
    pub parts: String,
    /// The number of measures, unless `length_auto`.
    pub length: usize,
    /// As many measures as the text needs, 12 without a text (`--length auto`).
    pub length_auto: bool,
    /// Internal cadences: `auto`, `none`, or measures such as `7, 12:F`.
    pub cadences: String,
    /// What every voice sings: a section of the Ordinary (`kyrie`) or some of its sense units
    /// (`gloria:1-5`), [`OWN_TEXT`] for `own_text`, or [`NO_TEXT`].
    pub text: String,
    /// An own Latin text, with hyphens and acutes where the syllabifier needs them.
    pub own_text: String,
    /// How freely the voices restate the text (`--repeat`).
    #[serde(with = "repeat_name")]
    pub repeat: Repeat,
    /// The syllables keep the acutes of the liturgical books (`--accents`).
    pub accents: bool,
    /// eleison in three syllables, e-léi-son, instead of the four of the books (`--eleison 3`).
    pub eleison3: bool,
    pub random_seed: bool,
    pub seed: u64,
    pub count: usize,
    pub beam: usize,
    pub noise: f64,
    /// How the voices imitate each other (phase 13).
    pub imitation: ImitationForm,
}

impl Default for FreeForm {
    fn default() -> Self {
        FreeForm {
            mode: "dorian".parse().expect("valid mode"),
            voices: 4,
            parts: String::new(),
            length: 12,
            length_auto: true,
            cadences: "auto".into(),
            text: DEFAULT_TEXT.into(),
            own_text: String::new(),
            repeat: Repeat::Auto,
            accents: true,
            eleison3: false,
            random_seed: true,
            seed: 1,
            count: 3,
            beam: 200,
            noise: 2.0,
            imitation: ImitationForm::default(),
        }
    }
}

/// What a free setting of the form will be, as far as it is known before the search.
#[derive(Clone, Debug)]
pub struct FreePreview {
    /// The text every voice sings; `None` without a text.
    pub text: Option<Arc<Text>>,
    /// Warnings about the text in the user's language, such as uncertain accents.
    pub warnings: Vec<String>,
    /// The length planned from the text and the other settings; 12 measures without a text.
    pub automatic: usize,
    /// The least length for the text, if the given one is too short for it.
    pub least: Option<usize>,
}

impl FreeForm {
    /// The text as `fux free --text` takes it: a section, some of its sense units, the own text,
    /// or `none`.
    pub fn text_spec(&self) -> &str {
        spec_of(&self.text, &self.own_text)
    }

    /// The division of eleison.
    pub fn eleison(&self) -> Eleison {
        eleison_of(self.eleison3)
    }

    /// The options of `fux free`; `seed` is the seed of this run. An imitation that cannot apply
    /// as the form asks for it is an error in the words of the form.
    pub fn options(&self, seed: u64, lang: Lang) -> Result<FreeOptions, String> {
        let cadences = match self.cadences.trim() {
            "" => CadencePlan::Auto,
            text => text.parse()?,
        };
        // The imitation's inputs count only where it can apply: switched off, its group is hidden,
        // and where `auto` leaves it aside, an input it cannot read stops no setting.
        let auto_cadences = cadences == CadencePlan::Auto;
        let imitation = match self.imitation.mode {
            Switch::Off => ImitationOptions {
                mode: ImitationMode::Off,
                ..ImitationOptions::default()
            },
            Switch::Auto
                if !ImitationOptions::default().applies(
                    self.texted(),
                    self.voice_count(lang),
                    auto_cadences,
                    self.length_auto,
                ) =>
            {
                self.imitation.options(lang).unwrap_or_default()
            }
            _ => self.imitation.options(lang)?,
        };
        if imitation.mode == ImitationMode::On {
            if !self.texted() {
                return Err(lang
                    .pick(
                        "Imitation braucht einen Text: einen Text wählen oder Imitation „aus“.",
                        "Imitation needs a text: choose one, or imitation “off”.",
                    )
                    .to_string());
            }
            if cadences != CadencePlan::Auto {
                return Err(lang
                    .pick(
                        "Die Imitation setzt die Binnenschlüsse selbst: Binnenschlüsse „auto“ oder Imitation „aus“.",
                        "Imitation places the internal cadences itself: internal cadences “auto”, or imitation “off”.",
                    )
                    .to_string());
            }
        }
        let imitating = imitation.applies(
            self.texted(),
            self.voice_count(lang),
            auto_cadences,
            self.length_auto,
        );
        // A chant's subjects in the chant's mode, if the form asks for it.
        let mode = if imitating && self.imitation.chant_subject() && self.imitation.chant_mode {
            None
        } else {
            Some(self.mode)
        };
        Ok(FreeOptions {
            mode,
            cadences,
            imitation,
            ..self.text_options(seed)
        })
    }

    /// Whether the voices sing a text.
    fn texted(&self) -> bool {
        let spec = self.text_spec();
        !spec.trim().is_empty() && !is_none(spec)
    }

    /// The number of voices: those of the parts, if they can be read, else the number given.
    pub fn voice_count(&self, lang: Lang) -> usize {
        match self.parts.trim() {
            "" => self.voices,
            parts => parse_parts(parts, lang).map_or(self.voices, |parts| parts.len()),
        }
    }

    /// Whether the voices of the form imitate each other.
    pub fn imitates(&self, lang: Lang) -> bool {
        let auto_cadences = matches!(self.cadences.trim(), "" | "auto");
        self.imitation.options(lang).is_ok_and(|imitation| {
            imitation.applies(
                self.texted(),
                self.voice_count(lang),
                auto_cadences,
                self.length_auto,
            )
        })
    }

    /// The options of `fux free` without the internal cadences, which stay automatic, and without
    /// imitation.
    fn text_options(&self, seed: u64) -> FreeOptions {
        let parts = self.parts.trim();
        FreeOptions {
            mode: Some(self.mode),
            voices: self.voices,
            parts: (!parts.is_empty()).then(|| parts.to_string()),
            length: (!self.length_auto).then_some(self.length),
            cadences: CadencePlan::Auto,
            text: self.text_spec().to_string(),
            repeat: self.repeat,
            accents: self.accents,
            eleison: self.eleison(),
            seed: Some(seed),
            count: self.count,
            beam: self.beam,
            noise: self.noise,
            imitation: ImitationOptions {
                mode: ImitationMode::Off,
                ..ImitationOptions::default()
            },
            ..FreeOptions::default()
        }
    }

    /// The text with its warnings and the lengths, as `fux free` would plan them. An error says
    /// what is wrong with the text; invalid parts or cadences count as not given here, since the
    /// run reports them.
    pub fn preview(&self, profile: &Profile, lang: Lang) -> Result<FreePreview, String> {
        let mut options = self.text_options(0);
        options.length = None;
        if let Some(parts) = &options.parts
            && parse_parts(parts, lang).is_err()
        {
            options.parts = None;
        }
        options.cadences = self.cadences.trim().parse().unwrap_or_default();
        let prepared = prepare_free(&options, profile, lang)
            .map_err(|e| form_message(&e, lang, Terms::Window))?;
        let request = &prepared.request;
        let least = match (&request.text, self.length_auto) {
            (Some(sung), false) => {
                let given = match &options.cadences {
                    CadencePlan::Auto => None,
                    given => Some(given.cadences(self.mode, 0)),
                };
                let planned = plan(
                    &sung.text,
                    self.mode,
                    request.voices.len(),
                    Some(self.length),
                    given,
                    restates(&prepared.profile, &sung.text),
                );
                match planned {
                    Err(GenerateError::TextTooLong { measures }) => Some(measures),
                    _ => None,
                }
            }
            _ => None,
        };
        Ok(FreePreview {
            text: request.text.as_ref().map(|sung| Arc::clone(&sung.text)),
            warnings: prepared.warnings,
            automatic: request.measures,
            least,
        })
    }
}

/// The inputs of `fux cf`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CantusForm {
    #[serde(with = "mode_id")]
    pub mode: ModeSpec,
    pub length: usize,
    #[serde(with = "voice")]
    pub voice_type: VoiceType,
    pub count: usize,
    pub random_seed: bool,
    pub seed: u64,
}

impl Default for CantusForm {
    fn default() -> Self {
        CantusForm {
            mode: "dorian".parse().expect("valid mode"),
            length: 11,
            voice_type: VoiceType::Alto,
            count: 5,
            random_seed: true,
            seed: 1,
        }
    }
}

/// The inputs of `fux book`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BookForm {
    /// All twelve modes instead of the six of Fux's cantus.
    pub all_modes: bool,
    /// The species 1 to 5 to set.
    pub species: [bool; 5],
    /// The numbers of voices 2 to 6.
    pub voices: [bool; 5],
    /// A generated cantus in every mode instead of Fux's.
    pub generate: bool,
    pub length: usize,
    pub exercises: bool,
    pub random_seed: bool,
    pub seed: u64,
}

impl Default for BookForm {
    fn default() -> Self {
        BookForm {
            all_modes: false,
            species: [true; 5],
            voices: [true, false, false, false, false],
            generate: false,
            length: 11,
            exercises: true,
            random_seed: true,
            seed: 1,
        }
    }
}

impl BookForm {
    pub fn options(&self, seed: u64) -> BookOptions {
        let chosen = |flags: &[bool; 5], first: u8| -> Vec<u8> {
            (first..)
                .zip(flags)
                .filter(|(_, on)| **on)
                .map(|(n, _)| n)
                .collect()
        };
        BookOptions {
            modes: if self.all_modes {
                vec!["alle".into()]
            } else {
                Vec::new()
            },
            species: chosen(&self.species, 1),
            voices: chosen(&self.voices, 2),
            generate: self.generate,
            length: self.length,
            exercises: self.exercises,
            seed: Some(seed),
            ..BookOptions::default()
        }
    }
}

/// What the form of a mass changes in one of its sections (`fux mass --scoring`, `--imitation`,
/// `--endings` with the section's id): its voices, how its parts begin, the degree it ends on.
/// A section the movements chosen do not have keeps its changes for when they have it again.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SectionChoice {
    /// The id of the section, by which the options name it: `christe`, `agnus3`.
    pub id: String,
    /// Its voices from the top, such as `S A T`; empty for those of Palestrina's practice.
    pub voices: String,
    /// Whether its parts begin in imitation: `auto` by Palestrina's practice.
    pub imitation: Switch,
    /// The degree its last part ends on, by its letter (`A`); empty to draw it.
    pub ending: String,
}

impl SectionChoice {
    /// Whether it changes nothing.
    pub fn is_empty(&self) -> bool {
        self.voices.trim().is_empty() && self.imitation == Switch::Auto && self.ending.is_empty()
    }
}

/// The inputs of `fux mass` (phase 14), stored like the other forms: the form of the mass, its
/// mode, choir and movements, the changes per section, how long texts are cut, the lengths, the
/// chant and the head motif (phase 14c), how the voices sing, the search, and how the files print
/// the mass. Settings stored before phase 14 lack it and get the defaults of `fux mass`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MassForm {
    #[serde(with = "preset_id")]
    pub preset: Preset,
    /// The mode of the mass, unless each movement takes its chant's (`chant_mode`).
    #[serde(with = "mode_id")]
    pub mode: ModeSpec,
    /// With a chant, each movement in the mode of its chant („nach dem Choral“, `fux mass
    /// --chant` without `--mode`) instead of `mode`.
    pub chant_mode: bool,
    pub voices: usize,
    /// Voice types from top to bottom, e.g. `S A T B`; empty for the usual choir.
    pub parts: String,
    /// The movements to set, in the order of [`Movement::ALL`].
    pub movements: [bool; 6],
    /// The changes per section; a section without one has no entry.
    pub sections: Vec<SectionChoice>,
    /// Changes of single parts as `--imitation` writes them, after those of the sections:
    /// `credo/3=aus`.
    pub imitation: String,
    /// Endings of single parts as `--endings` writes them, after those of the sections:
    /// `gloria/3=D`.
    pub endings: String,
    /// The most syllables of a part of Gloria and Credo (`--chunk`), unless `whole`.
    pub chunk: usize,
    /// A part from Break to Break, uncut (`--chunk aus`).
    pub whole: bool,
    #[serde(with = "lengths_id")]
    pub lengths: Lengths,
    /// The chant of the Kyriale the voices paraphrase; `None` for subjects from the words.
    #[serde(with = "chant_id")]
    pub chant: Option<ChantMass>,
    /// Gloria, Credo, Sanctus and Agnus Dei I open with the head motif of Kyrie I.
    pub motif: bool,
    /// How freely the voices restate their text (`--repeat`).
    #[serde(with = "repeat_name")]
    pub repeat: Repeat,
    /// The syllables keep the acutes of the liturgical books (`--accents`).
    pub accents: bool,
    /// eleison in three syllables, e-léi-son (`--eleison 3`).
    pub eleison3: bool,
    /// The title of the mass; empty for that of its form.
    pub title: String,
    pub random_seed: bool,
    pub seed: u64,
    pub beam: usize,
    pub noise: f64,
    /// Bar lines between the staves only (`--mensurstriche`).
    pub mensurstriche: bool,
    /// The priest's intonation before Gloria and Credo (`--intonation text`).
    pub intonation: bool,
    /// The voices that rest through a section hidden there in MusicXML (`--tacet verbergen`).
    pub hide_tacet: bool,
}

impl Default for MassForm {
    /// As `fux mass` without options: a Missa brevis in the Dorian mode for S A T B, all six
    /// movements, parts of at most 45 syllables, Palestrina's lengths, no chant and no head motif.
    fn default() -> Self {
        let notation = MassNotation::default();
        MassForm {
            preset: Preset::default(),
            mode: "dorian".parse().expect("valid mode"),
            chant_mode: true,
            voices: 4,
            parts: String::new(),
            movements: [true; 6],
            sections: Vec::new(),
            imitation: String::new(),
            endings: String::new(),
            chunk: tables::CHUNK,
            whole: false,
            lengths: Lengths::default(),
            chant: None,
            motif: false,
            repeat: Repeat::Auto,
            accents: true,
            eleison3: false,
            title: String::new(),
            random_seed: true,
            seed: 1,
            beam: 200,
            noise: 2.0,
            mensurstriche: notation.mensurstriche,
            intonation: notation.intonation,
            hide_tacet: notation.hide_tacet,
        }
    }
}

impl MassForm {
    /// The movements chosen, in the order of the mass.
    pub fn chosen(&self) -> Vec<Movement> {
        Movement::ALL
            .into_iter()
            .zip(self.movements)
            .filter(|(_, on)| *on)
            .map(|(movement, _)| movement)
            .collect()
    }

    /// The sections of the movements chosen in the form chosen that the options can change, in
    /// the order of the mass: all but an Osanna ut supra, which repeats the Sanctus' Osanna.
    pub fn sections_shown(&self) -> Vec<(Movement, &'static preset::SectionForm)> {
        self.chosen()
            .into_iter()
            .flat_map(|movement| {
                preset::form(self.preset, movement)
                    .sections
                    .iter()
                    .filter(|section| section.ut_supra.is_none())
                    .map(move |section| (movement, section))
            })
            .collect()
    }

    /// The changes of the section `id`, if it has any.
    pub fn section(&self, id: &str) -> Option<&SectionChoice> {
        self.sections.iter().find(|choice| choice.id == id)
    }

    /// The changes of the section `id`, to edit; a new entry if it has none.
    pub fn section_mut(&mut self, id: &str) -> &mut SectionChoice {
        let i = match self.sections.iter().position(|choice| choice.id == id) {
            Some(i) => i,
            None => {
                self.sections.push(SectionChoice {
                    id: id.to_string(),
                    ..SectionChoice::default()
                });
                self.sections.len() - 1
            }
        };
        &mut self.sections[i]
    }

    /// Whether a chant gives each movement its mode („nach dem Choral“).
    pub fn modes_of_chant(&self) -> bool {
        self.chant.is_some() && self.chant_mode
    }

    /// How the files print the mass.
    pub fn notation(&self) -> MassNotation {
        MassNotation {
            mensurstriche: self.mensurstriche,
            intonation: self.intonation,
            hide_tacet: self.hide_tacet,
        }
    }

    /// The options of `fux mass`; `seed` is the seed of this run. The changes per section count
    /// for the sections of the movements chosen ([`MassForm::sections_shown`]), first those of
    /// the sections, then those of single parts. Without a movement chosen the movements are
    /// empty, which [`fux_cli::mass::prepare_mass`] refuses in the user's words. The chant and
    /// the head motif are `--chant` and `--motif`; the plan refuses them together
    /// (`jobs::prepare_mass`).
    pub fn options(&self, seed: u64) -> MassOptions {
        let shown: Vec<&'static str> = self
            .sections_shown()
            .into_iter()
            .map(|(_, section)| section.id)
            .collect();
        let changes: Vec<&SectionChoice> = shown
            .iter()
            .filter_map(|id| self.section(id))
            .filter(|choice| !choice.is_empty())
            .collect();
        let joined = |items: Vec<String>| (!items.is_empty()).then(|| items.join("; "));
        let scoring = changes
            .iter()
            .filter(|choice| !choice.voices.trim().is_empty())
            .map(|choice| format!("{}={}", choice.id, choice.voices.trim()))
            .collect();
        let mut imitation: Vec<String> = changes
            .iter()
            .filter(|choice| choice.imitation != Switch::Auto)
            .map(|choice| {
                let value = if choice.imitation == Switch::On {
                    "an"
                } else {
                    "aus"
                };
                format!("{}={value}", choice.id)
            })
            .collect();
        imitation.extend(own_items(&self.imitation));
        let mut endings: Vec<String> = changes
            .iter()
            .filter(|choice| !choice.ending.is_empty())
            .map(|choice| format!("{}={}", choice.id, choice.ending))
            .collect();
        endings.extend(own_items(&self.endings));
        let parts = self.parts.trim();
        let title = self.title.trim();
        MassOptions {
            preset: self.preset.id().to_string(),
            movements: self
                .chosen()
                .into_iter()
                .map(Movement::id)
                .collect::<Vec<_>>()
                .join(","),
            voices: self.voices.clamp(2, 6) as u8,
            parts: (!parts.is_empty()).then(|| parts.to_string()),
            mode: (!self.modes_of_chant()).then_some(self.mode),
            scoring: joined(scoring),
            imitation: joined(imitation),
            endings: joined(endings),
            chunk: if self.whole {
                "aus".to_string()
            } else {
                self.chunk.to_string()
            },
            lengths: self.lengths.id().to_string(),
            motif: self.motif,
            chant: self.chant.map_or("keiner", ChantMass::id).to_string(),
            singing: SingingArgs {
                repeat: self.repeat,
                accents: self.accents,
                eleison: eleison_of(self.eleison3),
            },
            seed: Some(seed),
            beam: self.beam,
            noise: self.noise,
            title: (!title.is_empty()).then(|| title.to_string()),
            notation: self.notation(),
            ..MassOptions::default()
        }
    }

    /// The request of `fux mass` from the form, and the profile its parts are set and checked
    /// with: `profile` with the overlay of the mass ([`fux_cli::mass::prepare_mass`]). What the
    /// options cannot have is an error in the user's language, before any plan.
    pub fn prepare(
        &self,
        seed: u64,
        profile: &Profile,
        lang: Lang,
    ) -> Result<(MassRequest, Profile), String> {
        let options = self.options(seed);
        fux_cli::mass::prepare_mass(&options, profile, &[], lang).map_err(|e| format!("{e:#}"))
    }
}

/// The items of a field that takes them as `--imitation` or `--endings` does, separated by `;`.
fn own_items(text: &str) -> Vec<String> {
    text.split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

/// One voice of a score to check.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckVoice {
    #[serde(with = "voice")]
    pub voice_type: VoiceType,
    /// Note names or LilyPond.
    pub notes: String,
}

/// A result with a text, taken over into the check form as MusicXML, which keeps its lyrics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextedScore {
    /// Its title, e.g. „Freier Satz 1“.
    pub title: String,
    pub voices: usize,
    pub musicxml: String,
}

/// The inputs of `fux check`: a cantus and further voices, or a result with its text.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CheckForm {
    /// The cantus: notes, `fux:NAME` or the path of a GABC or MusicXML file.
    pub cantus: String,
    #[serde(with = "optional_voice")]
    pub cf_voice: Option<VoiceType>,
    #[serde(with = "optional_mode_id")]
    pub mode: Option<ModeSpec>,
    pub cadences: CadenceChoice,
    pub cadence_measures: String,
    pub voices: Vec<CheckVoice>,
    /// The text of the lyrics as `fux check --text` takes it: a section, some of its sense
    /// units, an own text, or `none` for no text rules; empty for the text a MusicXML file of
    /// fux-kontrapunkt names, or else to recognise the section.
    pub text: String,
    /// How freely the voices could restate the text when the setting was made
    /// (`fux check --repeat`).
    #[serde(with = "repeat_name")]
    pub repeat: Repeat,
    /// A result with a text, checked instead of the cantus and the voices.
    pub texted: Option<TextedScore>,
}

impl Default for CheckForm {
    fn default() -> Self {
        CheckForm {
            cantus: "fux:dorian".into(),
            cf_voice: None,
            mode: None,
            cadences: CadenceChoice::Auto,
            cadence_measures: String::new(),
            voices: vec![CheckVoice {
                voice_type: VoiceType::Soprano,
                notes: "a'1 a' g' a' b' c'' c'' b' d'' cis'' d''".into(),
            }],
            text: String::new(),
            repeat: Repeat::Auto,
            texted: None,
        }
    }
}

impl CheckForm {
    /// The text of the lyrics; `None` to recognise it.
    pub fn text_spec(&self) -> Option<String> {
        let text = self.text.trim();
        (!text.is_empty()).then(|| text.to_string())
    }

    /// What `fux check` checks: the cantus and the voices, or the texted result taken over, with
    /// the mode, the cadences, the text and the repetition of the form.
    pub fn job(&self) -> Result<CheckJob, String> {
        Ok(CheckJob {
            input: self.input()?,
            musicxml: self.texted.as_ref().map(|t| t.musicxml.clone()),
            text: self.text_spec(),
            repeat: self.repeat,
        })
    }

    /// The input of `fux check`.
    pub fn input(&self) -> Result<ScoreInput, String> {
        let cantus = self.cantus.trim();
        Ok(ScoreInput {
            example: None,
            job: None,
            musicxml: None,
            kern: None,
            mode: self.mode,
            cf: (!cantus.is_empty()).then(|| cantus.to_string()),
            cf_voice: self.cf_voice,
            cadences: self.cadences.cadences(&self.cadence_measures)?,
            voices: self
                .voices
                .iter()
                .filter(|v| !v.notes.trim().is_empty())
                .map(|v| format!("{}={}", v.voice_type.abbreviation(), v.notes.trim()))
                .collect(),
            title: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fux_cli::imitation::{AutoCount, ChantRef, PointVoices, SubjectChoice};

    #[test]
    fn forms_become_options_of_fux() {
        let form = CounterpointForm {
            voices: 4,
            species: 3,
            parts: "  ".into(),
            ..CounterpointForm::default()
        };
        let options = form.options(7).unwrap();
        assert_eq!(options.cf.as_deref(), Some("fux:dorian"));
        assert_eq!(
            (options.voices, options.species, options.seed),
            (4, 3, Some(7))
        );
        assert_eq!(options.parts, None);
        let generated = CounterpointForm {
            source: SourceKind::Generated,
            ..CounterpointForm::default()
        };
        let options = generated.options(1).unwrap();
        assert_eq!(options.cf, None);
        assert_eq!(options.mode.map(|m| m.id()).as_deref(), Some("dorian"));
        let bad = CounterpointForm {
            cadences: CadenceChoice::Measures,
            cadence_measures: "8, x".into(),
            ..CounterpointForm::default()
        };
        assert!(bad.options(1).is_err());
        let input = CheckForm::default().input().unwrap();
        assert_eq!(input.voices, ["S=a'1 a' g' a' b' c'' c'' b' d'' cis'' d''"]);
        let free = FreeForm {
            cadences: "7, 12:F".into(),
            ..FreeForm::default()
        };
        let options = free.options(5, Lang::De).unwrap();
        // Kyrie I by default, as long as it needs.
        assert_eq!(
            (options.voices, options.length, options.seed),
            (4, None, Some(5))
        );
        assert_eq!(
            (options.text.as_str(), options.repeat, options.accents),
            ("kyrie", Repeat::Auto, true)
        );
        assert_eq!(options.eleison, Eleison::Four);
        assert_eq!(
            options.cadences,
            CadencePlan::Measures(vec![(7, None), (12, Some(fux_core::Step::F))])
        );
        let bad = FreeForm {
            cadences: "x".into(),
            ..FreeForm::default()
        };
        assert!(bad.options(1, Lang::De).is_err());
    }

    /// Free counterpoint around a cantus sings the text of the form, the species sing none; the
    /// preview shows the text and the cantus it needs.
    #[test]
    fn texts_become_options_of_fux_cp() {
        let kyrie = CounterpointForm {
            species: 0,
            text: "kyrie".into(),
            repeat: Repeat::None,
            accents: false,
            eleison3: true,
            ..CounterpointForm::default()
        };
        let options = kyrie.options(1).unwrap();
        assert_eq!(options.text, "kyrie");
        assert_eq!(
            (options.repeat, options.accents, options.eleison),
            (Repeat::None, false, Eleison::Three)
        );
        let species = CounterpointForm {
            species: 3,
            ..kyrie.clone()
        };
        assert_eq!(species.options(1).unwrap().text, NO_TEXT);
        // By default without a text, as before.
        assert_eq!(
            CounterpointForm::default().options(1).unwrap().text,
            NO_TEXT
        );
        let preview = kyrie.preview(Lang::De).unwrap();
        assert_eq!(preview.text.unwrap().syllables.len(), 6);
        assert_eq!(preview.least, MIN_MEASURES);
        let gloria = CounterpointForm {
            text: "gloria:1-5".into(),
            ..kyrie.clone()
        };
        assert_eq!(gloria.preview(Lang::De).unwrap().least, 19);
        let own = CounterpointForm {
            text: OWN_TEXT.into(),
            own_text: "Ad te levavi animam meam.".into(),
            ..kyrie.clone()
        };
        assert_eq!(own.options(1).unwrap().text, "Ad te levavi animam meam.");
        assert_eq!(
            own.preview(Lang::De).unwrap().warnings,
            ["Akzent unsicher: animam → á-ni-mam; mit Akut angeben"]
        );
        let none = CounterpointForm::default().preview(Lang::De).unwrap();
        assert!(none.text.is_none() && none.warnings.is_empty());
        let error = CounterpointForm {
            text: "kyrei".into(),
            ..kyrie
        }
        .preview(Lang::De)
        .unwrap_err();
        assert!(
            error.starts_with("Unbekannter Abschnitt „kyrei“"),
            "{error}"
        );
        // Settings stored before phase 13 have no text, and the text is kept.
        let mut storage = crate::Memory::default();
        eframe::Storage::set_string(
            &mut storage,
            "cp",
            "(source:Fux,fux_cantus:\"phrygian\",species:0,voices:3)".into(),
        );
        let stored: CounterpointForm = eframe::get_value(&storage, "cp").unwrap();
        assert_eq!(
            (stored.fux_cantus.as_str(), stored.species),
            ("phrygian", 0)
        );
        assert_eq!(stored.text, NO_TEXT);
        assert!(stored.accents && !stored.eleison3 && stored.repeat == Repeat::Auto);
        let texted = CounterpointForm {
            text: "christe".into(),
            repeat: Repeat::Free,
            ..stored
        };
        eframe::set_value(&mut storage, "cp", &texted);
        let text = eframe::Storage::get_string(&storage, "cp").unwrap();
        assert!(text.contains("repeat:\"frei\""), "{text}");
        let back: CounterpointForm = eframe::get_value(&storage, "cp").unwrap();
        assert_eq!((back.text.as_str(), back.repeat), ("christe", Repeat::Free));
    }

    #[test]
    fn texts_become_options_of_fux_free() {
        let own = FreeForm {
            text: OWN_TEXT.into(),
            own_text: "Ad te levavi.".into(),
            length_auto: false,
            length: 10,
            repeat: Repeat::Free,
            accents: false,
            eleison3: true,
            ..FreeForm::default()
        };
        let options = own.options(1, Lang::De).unwrap();
        assert_eq!(options.text, "Ad te levavi.");
        assert_eq!(options.length, Some(10));
        assert_eq!(
            (options.repeat, options.accents, options.eleison),
            (Repeat::Free, false, Eleison::Three)
        );
        let none = FreeForm {
            text: NO_TEXT.into(),
            ..FreeForm::default()
        };
        assert_eq!(none.options(1, Lang::De).unwrap().text, "none");
        assert_eq!(none.text_spec(), "none");
        let range = FreeForm {
            text: "gloria:1-5".into(),
            ..FreeForm::default()
        };
        assert_eq!(range.options(1, Lang::De).unwrap().text, "gloria:1-5");
    }

    #[test]
    fn the_preview_plans_the_text() {
        let profile = Profile::fux();
        let preview = |form: FreeForm| form.preview(&profile, Lang::De);
        // Kyrie I, sung twice in 16 measures.
        let kyrie = preview(FreeForm::default()).unwrap();
        let text = kyrie.text.as_ref().unwrap();
        assert_eq!((text.id.as_str(), text.syllables.len()), ("kyrie", 7));
        assert_eq!((kyrie.automatic, kyrie.least), (16, None));
        assert!(kyrie.warnings.is_empty());
        // e-léi-son, and Kyrie I sung once.
        let three = preview(FreeForm {
            eleison3: true,
            ..FreeForm::default()
        })
        .unwrap();
        assert_eq!(three.text.unwrap().syllables.len(), 6);
        assert_eq!(three.automatic, 14);
        let once = preview(FreeForm {
            repeat: Repeat::None,
            ..FreeForm::default()
        })
        .unwrap();
        assert_eq!(once.automatic, 9);
        // Without a text: 12 measures, as before phase 12.
        let none = preview(FreeForm {
            text: NO_TEXT.into(),
            length_auto: false,
            length: 8,
            ..FreeForm::default()
        })
        .unwrap();
        assert!(none.text.is_none());
        assert_eq!((none.automatic, none.least), (12, None));
        // An own text with an uncertain accent.
        let own = preview(FreeForm {
            text: OWN_TEXT.into(),
            own_text: "Ad te levavi animam meam.".into(),
            ..FreeForm::default()
        })
        .unwrap();
        assert_eq!(own.text.unwrap().id, OWN_ID);
        assert_eq!(
            own.warnings,
            ["Akzent unsicher: animam → á-ni-mam; mit Akut angeben"]
        );
        // A given length too short for the text, and invalid settings the run reports.
        let gloria = FreeForm {
            text: "gloria:1-5".into(),
            length_auto: false,
            length: 10,
            parts: "S X".into(),
            cadences: "x".into(),
            ..FreeForm::default()
        };
        let short = preview(gloria.clone()).unwrap();
        assert_eq!(short.automatic, 32);
        assert!(short.least.is_some_and(|least| least > 10), "{short:?}");
        let fits = preview(FreeForm {
            length: 40,
            ..gloria
        })
        .unwrap();
        assert_eq!(fits.least, None);
        // Texts that cannot be sung.
        let error = preview(FreeForm {
            text: OWN_TEXT.into(),
            own_text: "Ad te #levavi".into(),
            ..FreeForm::default()
        })
        .unwrap_err();
        assert!(error.starts_with("Unerlaubtes Zeichen „#“"), "{error}");
        let error = preview(FreeForm {
            text: "kyrei".into(),
            ..FreeForm::default()
        })
        .unwrap_err();
        assert!(
            error.starts_with("Unbekannter Abschnitt „kyrei“"),
            "{error}"
        );
        let error = preview(FreeForm {
            text: OWN_TEXT.into(),
            ..FreeForm::default()
        })
        .unwrap_err();
        assert_eq!(error, "Der Text ist leer.");
    }

    #[test]
    fn settings_of_phase_11_get_the_text() {
        let mut storage = crate::Memory::default();
        eframe::Storage::set_string(
            &mut storage,
            "free",
            "(mode:\"phrygian\",voices:3,parts:\"\",length:10,cadences:\"auto\",random_seed:false,seed:7,count:2,beam:200,noise:2.0)".into(),
        );
        eframe::Storage::set_string(
            &mut storage,
            "check",
            "(cantus:\"fux:dorian\",cf_voice:\"\",mode:\"\",cadences:Auto,cadence_measures:\"\",voices:[])".into(),
        );
        let free: FreeForm = eframe::get_value(&storage, "free").unwrap();
        assert_eq!((free.voices, free.length, free.seed), (3, 10, 7));
        assert_eq!(free.text, "kyrie");
        assert!(free.length_auto && free.accents && !free.eleison3);
        assert_eq!(free.repeat, Repeat::Auto);
        let check: CheckForm = eframe::get_value(&storage, "check").unwrap();
        assert!(check.text.is_empty() && check.texted.is_none());
        // The new settings are kept.
        let free = FreeForm {
            text: OWN_TEXT.into(),
            own_text: "Ad te le-vá-vi.".into(),
            repeat: Repeat::None,
            eleison3: true,
            ..free
        };
        eframe::set_value(&mut storage, "free", &free);
        let text = eframe::Storage::get_string(&storage, "free").unwrap();
        assert!(text.contains("repeat:\"keine\""), "{text}");
        let back: FreeForm = eframe::get_value(&storage, "free").unwrap();
        assert_eq!(
            (back.text_spec(), back.repeat, back.eleison()),
            ("Ad te le-vá-vi.", Repeat::None, Eleison::Three)
        );
    }

    /// Settings stored before phase 13 imitate where it applies, and the choices of the
    /// imitation are kept; they become the options of `fux free`.
    #[test]
    fn imitation_becomes_options_of_fux_free() {
        let mut storage = crate::Memory::default();
        eframe::Storage::set_string(
            &mut storage,
            "free",
            "(mode:\"phrygian\",voices:4,text:\"christe\",random_seed:false,seed:7)".into(),
        );
        let old: FreeForm = eframe::get_value(&storage, "free").unwrap();
        assert_eq!(old.imitation, ImitationForm::default());
        assert!(old.imitates(Lang::De));
        let options = old.options(7, Lang::De).unwrap();
        assert_eq!(options.imitation, ImitationOptions::default());
        assert_eq!(options.mode.map(|m| m.id()).as_deref(), Some("phrygian"));
        let form = FreeForm {
            imitation: ImitationForm {
                subject: SubjectKind::Kyriale,
                phrases: "2, 3".into(),
                points: 2,
                later_voices: LaterVoices::Count(3),
                order: "A S T B".into(),
                intervals: "d 0 d".into(),
                delays: "1 1.5 2".into(),
                pairs: Switch::Off,
                restate: Switch::On,
                answer: AnswerKind::Real,
                head: 5,
                ..ImitationForm::default()
            },
            ..old
        };
        eframe::set_value(&mut storage, "free", &form);
        let back: FreeForm = eframe::get_value(&storage, "free").unwrap();
        assert_eq!(back.imitation, form.imitation);
        let options = back.options(7, Lang::De).unwrap();
        let imitation = &options.imitation;
        assert_eq!(
            imitation.subject,
            SubjectChoice::Chant {
                chant: ChantRef::Kyriale,
                phrases: Some(vec![2, 3])
            }
        );
        assert_eq!(
            (imitation.points, imitation.head),
            (AutoCount(Some(2)), AutoCount(Some(5)))
        );
        assert_eq!(imitation.point_voices, PointVoices::Count(3));
        assert_eq!(imitation.delays.0, [Some(vec![8, 12, 16])]);
        assert_eq!(
            (imitation.pairs, imitation.restate, imitation.answer),
            (Toggle::Off, Toggle::On, AnswerChoice::Real)
        );
        // A chant's subjects in the chant's mode, unless the form says otherwise.
        assert_eq!(options.mode, None);
        let own_mode = FreeForm {
            imitation: ImitationForm {
                chant_mode: false,
                ..back.imitation.clone()
            },
            ..back.clone()
        };
        assert!(own_mode.options(7, Lang::De).unwrap().mode.is_some());
        // Two voices do not imitate, nor do voices without a text; „an“ asks for a text.
        let two = FreeForm {
            voices: 2,
            ..back.clone()
        };
        assert!(!two.imitates(Lang::De));
        assert!(two.options(7, Lang::De).unwrap().mode.is_some());
        let on = FreeForm {
            text: NO_TEXT.into(),
            imitation: ImitationForm {
                mode: Switch::On,
                ..ImitationForm::default()
            },
            ..back.clone()
        };
        assert_eq!(
            on.options(7, Lang::De).unwrap_err(),
            "Imitation braucht einen Text: einen Text wählen oder Imitation „aus“."
        );
        // Errors name the field.
        let bad = |imitation: ImitationForm| {
            FreeForm {
                imitation,
                ..back.clone()
            }
            .options(7, Lang::De)
            .unwrap_err()
        };
        assert!(
            bad(ImitationForm {
                order: "A X".into(),
                ..ImitationForm::default()
            })
            .starts_with("Reihenfolge: ")
        );
        assert!(
            bad(ImitationForm {
                subject: SubjectKind::File,
                ..ImitationForm::default()
            })
            .starts_with("Soggetto: bitte eine GABC-Datei wählen")
        );
        assert!(
            bad(ImitationForm {
                phrases: "0".into(),
                subject: SubjectKind::Kyriale,
                ..ImitationForm::default()
            })
            .starts_with("Phrasen: ")
        );
    }

    #[test]
    fn forms_are_stored_as_text() {
        let form = CounterpointForm {
            mode: Some("hypophrygian".parse().unwrap()),
            cf_voice: Some(VoiceType::Tenor),
            ..CounterpointForm::default()
        };
        let mut storage = crate::Memory::default();
        eframe::set_value(&mut storage, "cp", &form);
        let text = eframe::Storage::get_string(&storage, "cp").unwrap();
        assert!(
            text.contains("hypophrygian") && text.contains("\"T\""),
            "{text}"
        );
        let back: CounterpointForm = eframe::get_value(&storage, "cp").unwrap();
        assert_eq!(back.mode, form.mode);
        assert_eq!(back.cf_voice, form.cf_voice);
    }

    /// The mass form by default asks what `fux mass` asks without options; its table of
    /// sections, the changes of single parts and the other fields become the options of `fux
    /// mass`, the chant and the head motif `--chant` and `--motif`. A section of a movement not
    /// chosen keeps its changes, which count no more.
    #[test]
    fn mass_forms_become_options_of_fux_mass() {
        use fux_cli::mass::{mass_plan, prepare_mass};
        let fux = Profile::fux();
        let dorian: ModeSpec = "dorian".parse().unwrap();
        let options = MassForm::default().options(7);
        assert_eq!(
            options,
            MassOptions {
                mode: Some(dorian),
                seed: Some(7),
                ..MassOptions::default()
            }
        );
        let (request, profile) = MassForm::default().prepare(7, &fux, Lang::De).unwrap();
        assert_eq!(
            request,
            MassRequest {
                mode: Some(dorian),
                seed: 7,
                ..MassRequest::default()
            }
        );
        let (_, overlaid) = prepare_mass(&options, &fux, &[], Lang::De).unwrap();
        assert_eq!(profile.to_toml(None), overlaid.to_toml(None));
        // The mode of the request reads as the default's: the same plan.
        let default = MassRequest {
            seed: 7,
            ..MassRequest::default()
        };
        assert_eq!(
            mass_plan(&request, Lang::De).unwrap(),
            mass_plan(&default, Lang::De).unwrap()
        );
        let choice = |id: &str| SectionChoice {
            id: id.into(),
            ..SectionChoice::default()
        };
        let form = MassForm {
            movements: [true, false, false, false, false, true],
            sections: vec![
                SectionChoice {
                    voices: " S A T ".into(),
                    imitation: Switch::Off,
                    ending: "A".into(),
                    ..choice("christe")
                },
                SectionChoice {
                    voices: "S A T B".into(),
                    ..choice("agnus3")
                },
                SectionChoice {
                    imitation: Switch::On,
                    ..choice("credo")
                },
                choice("kyrie"),
            ],
            imitation: "kyrie2=an; ".into(),
            endings: " agnus=A".into(),
            whole: true,
            lengths: Lengths::Natural,
            repeat: Repeat::None,
            accents: false,
            eleison3: true,
            title: " Missa prima ".into(),
            beam: 40,
            noise: 0.0,
            mensurstriche: true,
            intonation: false,
            hide_tacet: true,
            ..MassForm::default()
        };
        let options = form.options(1);
        assert_eq!(options.movements, "kyrie,agnus");
        assert_eq!(
            options.scoring.as_deref(),
            Some("christe=S A T; agnus3=S A T B")
        );
        assert_eq!(options.imitation.as_deref(), Some("christe=aus; kyrie2=an"));
        assert_eq!(options.endings.as_deref(), Some("christe=A; agnus=A"));
        assert_eq!(
            (options.chunk.as_str(), options.lengths.as_str()),
            ("aus", "natürlich")
        );
        assert_eq!(
            (
                options.singing.repeat,
                options.singing.accents,
                options.singing.eleison
            ),
            (Repeat::None, false, Eleison::Three)
        );
        assert_eq!(options.title.as_deref(), Some("Missa prima"));
        assert_eq!((options.beam, options.noise), (40, 0.0));
        assert_eq!(
            options.notation,
            MassNotation {
                mensurstriche: true,
                intonation: false,
                hide_tacet: true
            }
        );
        let (request, _) = form.prepare(1, &fux, Lang::De).unwrap();
        let plan = mass_plan(&request, Lang::De).unwrap();
        let christe = &plan.movements[0].sections[1];
        assert_eq!((christe.id, christe.voices.len()), ("christe", 3));
        assert_eq!(christe.parts[0].ending, fux_core::Step::A);
        // A chant gives each movement its mode, unless the form gives one; with the head motif
        // the plan fails in the user's words.
        let chant = MassForm {
            chant: Some(ChantMass::IX),
            ..MassForm::default()
        };
        assert!(chant.modes_of_chant());
        assert_eq!(chant.options(1).mode, None);
        assert_eq!(
            (chant.options(1).chant.as_str(), chant.options(1).motif),
            ("ix", false)
        );
        let (request, _) = chant.prepare(1, &fux, Lang::De).unwrap();
        assert_eq!(
            (request.chant, request.motif, request.mode),
            (Some(ChantMass::IX), false, None)
        );
        let modes: std::collections::HashSet<String> = mass_plan(&request, Lang::De)
            .unwrap()
            .movements
            .iter()
            .map(|movement| movement.mode.id())
            .collect();
        assert!(modes.len() > 1, "{modes:?}");
        let one = MassForm {
            chant_mode: false,
            ..chant.clone()
        };
        assert_eq!(one.options(1).mode, Some(dorian));
        let both = MassForm {
            motif: true,
            ..chant
        };
        assert!(both.options(1).motif);
        let (request, _) = both.prepare(1, &fux, Lang::De).unwrap();
        assert_eq!((request.chant, request.motif), (Some(ChantMass::IX), true));
        let error = crate::jobs::prepare_mass(&both, 1, &fux, Lang::De).unwrap_err();
        assert!(
            error.starts_with("Kopfmotiv und Choralparaphrase schließen einander aus"),
            "{error}"
        );
        // What the options cannot have, in the user's words.
        let none = MassForm {
            movements: [false; 6],
            ..MassForm::default()
        };
        let error = none.prepare(1, &fux, Lang::De).unwrap_err();
        assert!(error.starts_with("Kein Satz gewählt"), "{error}");
        let parts = MassForm {
            parts: "S X".into(),
            ..MassForm::default()
        };
        assert!(parts.prepare(1, &fux, Lang::En).is_err());
    }

    /// The mass form is stored as text, with the ids of the options; settings stored before
    /// phase 14 get the defaults of `fux mass`.
    #[test]
    fn mass_forms_are_stored_as_text() {
        let mut storage = crate::Memory::default();
        eframe::Storage::set_string(&mut storage, "mass", "()".into());
        let old: MassForm = eframe::get_value(&storage, "mass").unwrap();
        assert_eq!(old, MassForm::default());
        let form = MassForm {
            preset: Preset::Normal,
            mode: "phrygian".parse().unwrap(),
            chant: Some(ChantMass::IV),
            chant_mode: false,
            lengths: Lengths::Natural,
            repeat: Repeat::Free,
            movements: [true, true, false, false, false, false],
            sections: vec![SectionChoice {
                id: "christe".into(),
                voices: "S A T".into(),
                imitation: Switch::On,
                ending: "A".into(),
            }],
            ..MassForm::default()
        };
        eframe::set_value(&mut storage, "mass", &form);
        let text = eframe::Storage::get_string(&storage, "mass").unwrap();
        for stored in [
            "preset:\"normal\"",
            "mode:\"phrygian\"",
            "chant:\"iv\"",
            "lengths:\"natürlich\"",
            "repeat:\"frei\"",
        ] {
            assert!(text.contains(stored), "{stored} in {text}");
        }
        let back: MassForm = eframe::get_value(&storage, "mass").unwrap();
        assert_eq!(back, form);
    }
}
