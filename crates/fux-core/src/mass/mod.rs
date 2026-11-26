//! The mass (phase 14): Kyrie, Gloria, Credo, Sanctus with Benedictus and Agnus Dei, planned as
//! independent sentence parts under the forms of Palestrina's masses (D1 of
//! docs/design/phase-14/spec.md). A part is one free setting: a range of the text, the voices of
//! its section, the degree it ends on, how it begins after the part before and a seed of its
//! own. No part reads another's notes, so a mass costs what its parts cost, one after the other
//! or side by side.
//!
//! [`plan_mass`] plans a mass without a search, in milliseconds: the forms of the movements
//! ([`preset`]), the voices of each section, the cut of long texts into parts at the ends of
//! sentences, the joins between the parts, their endings and restarts drawn by Palestrina's
//! shares ([`tables`]), the opening chord of a part that restarts in all voices together
//! ([`chord`]), the lengths of the short sections, the seeds and the estimates. What the plan
//! cannot have fails at once with a [`MassOptionError`]
//! ([`GenerateError::MassOption`](crate::generate::GenerateError::MassOption)), before any part
//! is searched.
//!
//! **Addresses.** The options that change single parts (`--imitation`, `--endings`, `--only`)
//! name them by `movement/n`, the n-th part of a movement counted from 1 (`credo/3`), by the id
//! of a section (`kyrie christe kyrie2 gloria qui-tollis credo incarnatus crucifixus spiritum
//! sanctus pleni osanna benedictus osanna2 agnus agnus3`, as far as the form has it), or by the
//! label of a Break, which names the part that begins there: its text's id (`qui-tollis`,
//! `incarnatus`, `crucifixus`) or its label as printed (`Et incarnatus est`). A section names
//! all its parts, for `--endings` its last one, whose ending is the section's; in
//! `--imitation` a value without an address names every part. `--scoring` takes sections only.
//! The items of an option apply in order, a later one over an earlier one.
//!
//! **Generation** ([`generate`]). Each part is set by itself as a free setting
//! ([`part_request`], [`generate_part`]), with three seeds and its fallbacks, under the profile of
//! the mass ([`profile`]); its subjects come from its words, from the chant of the Kyriale in a
//! mass on chant ([`part_chant`]), or for the first point of a later movement from the head motif
//! of Kyrie I ([`motif_of`], [`Motif`]), the one part that reads another's notes.
//! [`generate_mass`] sets the parts of a plan one after the other and reports what it does as
//! [`MassEvent`]s. [`join`] puts the parts of a section or a movement
//! together ([`section_score`], [`movement_score`]), writes and reads what a file keeps of a part
//! ([`PartField`], with the version of the generator that set it, [`GENERATOR`]) and takes the
//! parts of a movement's file apart again ([`split_movement`]). [`kept`] assembles a mass from
//! the files of its parts as they are, with the plan they were set in ([`mass_of_parts`],
//! `--rewrite`).
//!
//! The module has no I/O and builds for WebAssembly.

pub mod chord;
pub mod generate;
pub mod join;
pub mod kept;
pub mod plan;
pub mod preset;
pub mod tables;

use std::fmt;
use std::str::FromStr;

use crate::clef::VoiceType;
use crate::generate::{choir_with, part_endings};
use crate::imitation::Toggle;
use crate::lang::Lang;
use crate::mode::ModeSpec;
use crate::pitch::{Pitch, Step};
use crate::rules::Profile;
use crate::text::plan::CadenceChain;
use crate::text::{Text, TextOptions, resolve};

pub use crate::imitation::Motif;
pub use chord::opening_chord;
pub use generate::{
    Mass, MassEvent, PartResult, generate_mass, generate_part, motif_of, part_chant, part_profile,
    part_request, profile, profile_hash,
};
pub use join::{
    FIELD_MASS, FIELD_PART, FieldError, MassField, PartField, fields_of_part, mass_field,
    movement_parts, movement_score, movement_sections, part_field, part_fields, part_from_parts,
    section_score, split_movement,
};
pub use kept::{KeptError, KeptPart, mass_of_parts};
pub use plan::plan_mass;

/// The version of the generator, which the file of every part records (`generator=` of
/// `fux-part`, [`PartField::generator`]; docs/naechste-schritte.md, D9): `--resume` and `--only`
/// take a part from its file only where this version set it ([`PartField::fits`]), so that a mass
/// goes on only with parts that this program sets alike. Raise it with every change after which
/// the same request sets other notes than before, as the byte comparison of
/// `tools/compare_outputs.sh` shows them, also with a change that acts silently, without changing
/// the rule profile or the plan: the tables (the weights of the chain of internal cadences, the
/// shares of the endings and restarts), the formulas and the driver of the search. A file without
/// the item counts as version 1, the generator before the item was kept. Version 2 since L6, L3,
/// whose costs of a final chord without a major third and of the bare sixth in free
/// counterpoint set other notes in the parts of a mass.
pub const GENERATOR: u32 = 2;

/// A movement of the Ordinary, in the order of the mass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Movement {
    Kyrie,
    Gloria,
    Credo,
    Sanctus,
    Benedictus,
    Agnus,
}

impl Movement {
    /// All six, in the order of the mass.
    pub const ALL: [Movement; 6] = [
        Movement::Kyrie,
        Movement::Gloria,
        Movement::Credo,
        Movement::Sanctus,
        Movement::Benedictus,
        Movement::Agnus,
    ];

    /// The id by which options name it: `kyrie`, `gloria`, `credo`, `sanctus`, `benedictus`,
    /// `agnus`.
    pub const fn id(self) -> &'static str {
        match self {
            Movement::Kyrie => "kyrie",
            Movement::Gloria => "gloria",
            Movement::Credo => "credo",
            Movement::Sanctus => "sanctus",
            Movement::Benedictus => "benedictus",
            Movement::Agnus => "agnus",
        }
    }

    /// The title as editions print it, the same in every language: Kyrie, Gloria, Credo,
    /// Sanctus, Benedictus, Agnus Dei.
    pub const fn title(self) -> &'static str {
        match self {
            Movement::Kyrie => "Kyrie",
            Movement::Gloria => "Gloria",
            Movement::Credo => "Credo",
            Movement::Sanctus => "Sanctus",
            Movement::Benedictus => "Benedictus",
            Movement::Agnus => "Agnus Dei",
        }
    }
}

impl FromStr for Movement {
    type Err = MassOptionError;

    /// Accepts the id or the title in any case: `credo`, `Agnus Dei`, `agnus-dei`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = normalized(s);
        Movement::ALL
            .into_iter()
            .find(|m| name == m.id() || name == normalized(m.title()))
            .ok_or_else(|| MassOptionError::UnknownMovement {
                name: s.trim().into(),
            })
    }
}

impl fmt::Display for Movement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title())
    }
}

/// The forms of a mass (D18, [`preset`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Preset {
    /// After Palestrina's Missa brevis (1570) and the short four-voice masses: the Gloria with a
    /// Break at Qui tollis, the Credo undivided with Breaks at Et incarnatus and Crucifixus, the
    /// Sanctus through-composed, a reduced Benedictus with a new Osanna.
    #[default]
    Brevis,
    /// After the medians of his four-voice masses: Gloria and Credo in sections with a reduced
    /// Crucifixus, Sanctus, Pleni and Osanna apart, the Benedictus followed by „Osanna ut supra“.
    Normal,
}

impl Preset {
    /// The id by which options name it: `brevis`, `normal`.
    pub const fn id(self) -> &'static str {
        match self {
            Preset::Brevis => "brevis",
            Preset::Normal => "normal",
        }
    }

    /// The title of a mass in this form unless the request gives one.
    pub const fn title(self) -> &'static str {
        match self {
            Preset::Brevis => "Missa brevis",
            Preset::Normal => "Missa",
        }
    }
}

impl FromStr for Preset {
    type Err = MassOptionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalized(s).as_str() {
            "brevis" => Ok(Preset::Brevis),
            "normal" => Ok(Preset::Normal),
            _ => Err(MassOptionError::UnknownPreset {
                name: s.trim().into(),
            }),
        }
    }
}

/// The masses of the Kyriale whose chant the voices paraphrase (D12, MASS-CHANT).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChantMass {
    /// Mass IX *Cum jubilo* with Credo I.
    IX,
    /// Kyrie IV *Cunctipotens genitor Deus*; the other movements as IX.
    IV,
}

impl ChantMass {
    /// The id by which options name it: `ix`, `iv`.
    pub const fn id(self) -> &'static str {
        match self {
            ChantMass::IX => "ix",
            ChantMass::IV => "iv",
        }
    }
}

impl FromStr for ChantMass {
    type Err = MassOptionError;

    /// `ix` or `iv` in any case, or their numbers.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalized(s).as_str() {
            "ix" | "9" => Ok(ChantMass::IX),
            "iv" | "4" => Ok(ChantMass::IV),
            _ => Err(MassOptionError::UnknownChant {
                name: s.trim().into(),
            }),
        }
    }
}

/// How long the short sections are (D10, MASS-LENGTH).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Lengths {
    /// As the imitation plans them by themselves (phase 13).
    Natural,
    /// The Benedictus and Agnus Dei I with two points more, restated, nearer the lengths of
    /// Palestrina's (a generated Benedictus a 3 has 25–35 measures against his median of 67).
    #[default]
    Palestrina,
}

impl Lengths {
    /// The id by which options name it: `natürlich`, `palestrina`.
    pub const fn id(self) -> &'static str {
        match self {
            Lengths::Natural => "natürlich",
            Lengths::Palestrina => "palestrina",
        }
    }
}

impl FromStr for Lengths {
    type Err = MassOptionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalized(s).as_str() {
            "palestrina" => Ok(Lengths::Palestrina),
            "natürlich" | "natuerlich" | "naturlich" | "natural" => Ok(Lengths::Natural),
            _ => Err(MassOptionError::UnknownLengths {
                name: s.trim().into(),
            }),
        }
    }
}

/// How freely the voices restate their text (`--repeat`), as `fux free` takes it: TXT-REPEAT as
/// the profile has it, or overridden.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Repeat {
    /// As the profile has it: free in short texts, rare in long ones.
    #[default]
    Auto,
    /// Never: TXT-REPEAT `short` and `long` forbidden.
    None,
    /// Freely: TXT-REPEAT `short`, `long` and `again` off.
    Free,
}

impl Repeat {
    /// The overrides of the profile that this choice makes.
    pub fn overrides(self) -> &'static [&'static str] {
        match self {
            Repeat::Auto => &[],
            Repeat::None => &["TXT-REPEAT.short=V", "TXT-REPEAT.long=V"],
            Repeat::Free => &[
                "TXT-REPEAT.short=aus",
                "TXT-REPEAT.long=aus",
                "TXT-REPEAT.again=aus",
            ],
        }
    }

    /// Whether the voices may restate their text at all: a short text is then planned to be sung
    /// more than once, and its points of imitation may be restated.
    pub const fn restates(self) -> bool {
        !matches!(self, Repeat::None)
    }

    /// `profile` with TXT-REPEAT as this choice sets it.
    pub fn apply(self, profile: &Profile) -> Profile {
        let mut profile = profile.clone();
        for spec in self.overrides() {
            profile
                .apply_override(spec)
                .expect("the overrides of --repeat name parameters of TXT-REPEAT");
        }
        profile
    }

    /// The name for users: „auto“, „keine“, „frei“.
    pub fn name(self, lang: Lang) -> &'static str {
        match self {
            Repeat::Auto => "auto",
            Repeat::None => lang.pick("keine", "none"),
            Repeat::Free => lang.pick("frei", "free"),
        }
    }
}

impl FromStr for Repeat {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.trim().to_lowercase().as_str() {
            "auto" => Ok(Repeat::Auto),
            "keine" | "none" => Ok(Repeat::None),
            "frei" | "free" => Ok(Repeat::Free),
            _ => Err(format!("{text:?}: auto, keine (none) or frei (free)")),
        }
    }
}

/// How a part follows the part before it (D2, MASS-PARTS). The start of a part is the tick
/// before which no voice sounds ([`Join::start`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Join {
    /// The first part of a movement.
    Start,
    /// The first part of a section after another one: a double bar, the last chord before it
    /// under a fermata, its own scoring.
    Section,
    /// A Break inside a section, with its label as printed (`Et incarnatus est`): the part
    /// before ends in a held chord, then every voice rests a whole measure and all restart on
    /// the downbeat, as the 1570 Missa brevis does (Credo_14, ticks 808–816).
    Break(&'static str),
    /// An unlabelled join between sentence parts: a held chord, then a minim's breath, which no
    /// melodic pair, tie or planned entry crosses.
    Seam,
}

impl Join {
    /// The tick before which no voice of the part sounds: 0 at the start of a movement or a
    /// section, a whole measure (8) after a Break, a minim (4) after a seam.
    pub const fn start(self) -> u32 {
        match self {
            Join::Start | Join::Section => 0,
            Join::Break(_) => 8,
            Join::Seam => 4,
        }
    }

    /// The name for users: „Satzbeginn“, „Abschnitt“, the label of a Break, „Naht“.
    pub fn name(self, lang: Lang) -> &'static str {
        match self {
            Join::Start => lang.pick("Satzbeginn", "start"),
            Join::Section => lang.pick("Abschnitt", "section"),
            Join::Break(label) => label,
            Join::Seam => lang.pick("Naht", "seam"),
        }
    }
}

/// How a part begins (D6, MASS-RESTART).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Opening {
    /// In imitation, the leader of the first point at the part's start.
    Imitative,
    /// All voices together on a full triad of the final ([`opening_chord`]), in two voices on the
    /// final and its fifth or octave, a semibreve on the first syllable.
    Tutti,
}

impl Opening {
    /// The name for users: „Imitation“, „gemeinsam“.
    pub fn name(self, lang: Lang) -> &'static str {
        match self {
            Opening::Imitative => lang.pick("Imitation", "imitation"),
            Opening::Tutti => lang.pick("gemeinsam", "together"),
        }
    }
}

/// What a mass asks for. Whatever it leaves open the plan draws by Palestrina's practice.
#[derive(Clone, Debug, PartialEq)]
pub struct MassRequest {
    pub preset: Preset,
    /// One mode for the whole mass; `None` for the Dorian, or with a chant paraphrase for the
    /// mode of each movement's chant (D12).
    pub mode: Option<ModeSpec>,
    /// The voices of the full choir, from the top: two to six.
    pub choir: Vec<VoiceType>,
    /// The movements to set; the plan takes them in the order of the mass.
    pub movements: Vec<Movement>,
    /// Changes to the scoring, the restarts and the endings, in order: a later one wins.
    pub overrides: Vec<Override>,
    /// The chant of the Kyriale the voices paraphrase; `None` for subjects from the words.
    pub chant: Option<ChantMass>,
    /// Whether Gloria, Credo, Sanctus and Agnus Dei I open with the head motif of Kyrie I (D11).
    pub motif: bool,
    pub lengths: Lengths,
    /// The most syllables of a part of a long section (D3); `None` sets every stretch between
    /// Breaks in one part.
    pub chunk: Option<usize>,
    pub text: TextOptions,
    pub repeat: Repeat,
    /// The printed syllables keep the acute of the liturgical books.
    pub accents: bool,
    pub seed: u64,
    pub beam: usize,
    pub noise: f64,
    /// The title; `None` for the title of the form.
    pub title: Option<String>,
}

impl Default for MassRequest {
    /// The defaults of the options of `fux mass`: a Missa brevis in the Dorian mode for S A T B,
    /// all six movements, parts of at most 45 syllables, Palestrina's lengths; with seed 1,
    /// whereas `fux mass` without `--seed` takes its seed from the clock and prints it (a
    /// frontend that wants its defaults does the same).
    fn default() -> Self {
        MassRequest {
            preset: Preset::Brevis,
            mode: None,
            choir: choir(4),
            movements: Movement::ALL.to_vec(),
            overrides: Vec::new(),
            chant: None,
            motif: false,
            lengths: Lengths::Palestrina,
            chunk: Some(tables::CHUNK),
            text: TextOptions::default(),
            repeat: Repeat::Auto,
            accents: true,
            seed: 1,
            beam: 200,
            noise: 2.0,
            title: None,
        }
    }
}

/// A change to the plan at an address ([module documentation](self)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Override {
    /// The voices of a section, from the top (`--scoring benedictus=S A T`).
    Scoring { at: String, voices: Vec<VoiceType> },
    /// Whether parts begin in imitation (`--imitation credo/3=an`): `On` in imitation, `Off`
    /// together, `Auto` by the restart table. An empty address names every part.
    Imitation { at: String, toggle: Toggle },
    /// The degree a part ends on (`--endings christe=A`); a section's id names its last part.
    Ending { at: String, step: Step },
}

/// The plan of a mass: its movements with their sections and parts, every choice made.
#[derive(Clone, Debug, PartialEq)]
pub struct MassPlan {
    pub title: String,
    /// The principal degree beside the final (P), drawn once for the mass (§ 3.4), in the mode
    /// of its first movement.
    pub principal: Step,
    pub movements: Vec<MovementPlan>,
    /// The form of the mass.
    pub preset: Preset,
    /// The mode of the request as the draws read it: the id of its one mode, `kyriale-ix` or
    /// `kyriale-iv` where each movement takes the mode of its chant, else `dorian`.
    pub mode: String,
    /// The voices of the full choir, from the top.
    pub choir: Vec<VoiceType>,
    /// The seed of the draws and of the parts' seeds.
    pub seed: u64,
    /// How the parts divide their texts, as the request says: the syllables of the parts are
    /// counted so, and a part read back from a file sings its text so or does not fit.
    pub text: TextOptions,
    /// The printed syllables keep the acute of the liturgical books, as the request says; a
    /// part read back from a file prints them so.
    pub accents: bool,
    /// The chant of the Kyriale whose phrases the imitative parts paraphrase (MASS-CHANT), as the
    /// request says; `None` for subjects from the words.
    pub chant: Option<ChantMass>,
    /// The mass has a head motif (MASS-MOTIF), as the request says: Gloria, Credo, Sanctus and
    /// Agnus Dei I open with the motif of Kyrie I where they begin in imitation
    /// ([`PartPlan::motif`]). It is the mass's, whichever of its movements a plan sets and however
    /// they begin, so that the files of every run name the same mass ([`MassField`]) and Kyrie I
    /// keeps its motif for the movements set later ([`MassPlan::motif_source`]).
    pub motif: bool,
}

/// A movement as planned.
#[derive(Clone, Debug, PartialEq)]
pub struct MovementPlan {
    pub movement: Movement,
    pub mode: ModeSpec,
    /// The principal degree P in the mode of this movement.
    pub principal: Step,
    /// The voices of the movement, from the top: the union of its sections' voices.
    pub voices: Vec<VoiceType>,
    /// Their names, in German as a score names its voices; a type that sings twice is numbered
    /// („Tenor 1“, „Tenor 2“). [`voice_names`] gives them in another language.
    pub names: Vec<String>,
    /// The priest's intonation before the polyphony (Gloria, Credo), as the Graduale of 1961
    /// prints it.
    pub intonation: Option<&'static str>,
    /// Remarks for the performers, in German and English.
    pub notes: Vec<(&'static str, &'static str)>,
    pub sections: Vec<SectionPlan>,
}

impl MovementPlan {
    /// The remarks for the performers in the user's language.
    pub fn notes_in(&self, lang: Lang) -> Vec<&'static str> {
        self.notes
            .iter()
            .map(|&(de, en)| lang.pick(de, en))
            .collect()
    }

    /// The number of parts in the movement.
    pub fn part_count(&self) -> usize {
        self.sections.iter().map(|s| s.parts.len()).sum()
    }
}

/// A section as planned: a stretch of a movement between double bars, with its own scoring.
#[derive(Clone, Debug, PartialEq)]
pub struct SectionPlan {
    pub id: &'static str,
    pub title: &'static str,
    /// Its voices, as indices into the movement's voices: the k-th voice of a type is the k-th
    /// of that type in the movement.
    pub voices: Vec<usize>,
    pub parts: Vec<PartPlan>,
    /// „Osanna ut supra“: the section repeats this section (movement and section in the plan)
    /// and has no parts of its own.
    pub ut_supra: Option<(usize, usize)>,
}

/// A part as planned: one free setting.
#[derive(Clone, Debug, PartialEq)]
pub struct PartPlan {
    /// The text as a spec: a section of the Ordinary, some of its units, or several joined by
    /// `+` (`kyrie`, `credo:5-7`, `sanctus+pleni+osanna`).
    pub text: String,
    /// The syllables of the text.
    pub syllables: usize,
    pub join: Join,
    /// The degree of the final cadence (MASS-ENDING).
    pub ending: Step,
    pub opening: Opening,
    /// The tick before which no voice sounds ([`Join::start`]).
    pub start: u32,
    /// For a part that begins together: one pitch per voice of its section, from the top.
    pub chord: Option<Vec<Pitch>>,
    /// How its internal cadences choose their degrees (MASS-CADENCES): a chain seeded with the
    /// part's seed, weighted by Palestrina's cadences in the movement's mode with its principal
    /// degree, from the ending of the part before it in the movement.
    pub chain: CadenceChain,
    /// The text goes on after this part: every part but the last of its section (D8). In such a
    /// part TXT-REPEAT `long` also prices restating the last unit.
    pub continued: bool,
    /// The number of points of imitation; `None` for as many as the imitation plans by itself.
    pub points: Option<usize>,
    /// The first point takes the head motif of Kyrie I.
    pub motif: bool,
    /// The seed of its search and of its chain (MASS-SEED): `splitmix64(seed ^ fnv("mass|preset|
    /// mode|choir") ^ (m << 16 | s << 8 | p))` with m the movement in the order of the mass (Kyrie
    /// 0 … Agnus Dei 5), s the section in the movement and p the part in the section, so that a
    /// movement's parts keep their seeds whichever other movements are set, as its endings and
    /// restarts do, and masses with one seed in other modes or choirs set them otherwise.
    pub seed: u64,
    pub estimate: Estimate,
}

impl PartPlan {
    /// The id of the first text of the Ordinary the part sings (`credo` for `credo:5-7`,
    /// `sanctus` for `sanctus+pleni+osanna`): the section whose chant and pace it takes, and for
    /// a part that begins at a Break the address of the Break.
    pub fn first_text(&self) -> &str {
        self.text.split(['+', ':']).next().unwrap_or_default()
    }

    /// The text the part sings, with eleison divided as `options` say: its spec resolved, going
    /// on after the part where the part is not the last of its section ([`Text::continued`]).
    ///
    /// # Panics
    ///
    /// If the spec names no text of the Ordinary, which a plan's never does.
    pub fn sung_text(&self, options: &TextOptions) -> Text {
        let (mut text, _) = resolve(&self.text, options)
            .unwrap_or_else(|error| panic!("a part sings a text of the Ordinary: {error}"));
        text.continued = self.continued;
        text
    }
}

/// What a part or a mass is expected to take.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Estimate {
    pub measures: usize,
    /// Seconds of search natively, on eight cores; about five times as long in WebAssembly
    /// ([`tables::ESTIMATE_WASM`]).
    pub seconds: f32,
}

/// The place of a part in a plan: indices into the movements, the movement's sections and the
/// section's parts, from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PartRef {
    pub movement: usize,
    pub section: usize,
    pub part: usize,
}

/// What an address names in a plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    /// Every part (an empty address).
    All,
    /// One part, by its number in the movement or by the Break it begins at.
    Part(PartRef),
    /// A section: (movement, section).
    Section(usize, usize),
}

impl MassPlan {
    /// Every part in the order of the mass, with its place.
    pub fn parts(&self) -> impl Iterator<Item = (PartRef, &PartPlan)> {
        self.movements.iter().enumerate().flat_map(|(m, movement)| {
            movement
                .sections
                .iter()
                .enumerate()
                .flat_map(move |(s, section)| {
                    section.parts.iter().enumerate().map(move |(p, part)| {
                        let at = PartRef {
                            movement: m,
                            section: s,
                            part: p,
                        };
                        (at, part)
                    })
                })
        })
    }

    /// The part at a place.
    pub fn part(&self, at: PartRef) -> &PartPlan {
        &self.movements[at.movement].sections[at.section].parts[at.part]
    }

    /// The address of a part by its number in the movement: `credo/3`.
    pub fn address(&self, at: PartRef) -> String {
        let movement = &self.movements[at.movement];
        let before: usize = movement.sections[..at.section]
            .iter()
            .map(|s| s.parts.len())
            .sum();
        format!("{}/{}", movement.movement.id(), before + at.part + 1)
    }

    /// The parts an address names ([module documentation](self)): one part, every part of a
    /// section, or every part of the mass for an empty address. An „Osanna ut supra“ has no part
    /// of its own and fails as it fails the other options ([`MassOptionError::UtSupra`]): the
    /// Osanna it repeats is the Sanctus'.
    pub fn find(&self, at: &str) -> Result<Vec<PartRef>, MassOptionError> {
        Ok(match self.target(at)? {
            Target::All => self.parts().map(|(at, _)| at).collect(),
            Target::Part(part) => vec![part],
            Target::Section(m, s) if self.movements[m].sections[s].ut_supra.is_some() => {
                return Err(MassOptionError::UtSupra { at: at.into() });
            }
            Target::Section(m, s) => (0..self.movements[m].sections[s].parts.len())
                .map(|p| PartRef {
                    movement: m,
                    section: s,
                    part: p,
                })
                .collect(),
        })
    }

    /// The number of sections.
    pub fn section_count(&self) -> usize {
        self.movements.iter().map(|m| m.sections.len()).sum()
    }

    /// The number of parts.
    pub fn part_count(&self) -> usize {
        self.movements.iter().map(MovementPlan::part_count).sum()
    }

    /// The syllables of all parts.
    pub fn syllables(&self) -> usize {
        self.parts().map(|(_, part)| part.syllables).sum()
    }

    /// The estimate of the whole mass, its parts set one after the other.
    pub fn estimate(&self) -> Estimate {
        self.parts()
            .fold(Estimate::default(), |sum, (_, part)| Estimate {
                measures: sum.measures + part.estimate.measures,
                seconds: sum.seconds + part.estimate.seconds,
            })
    }

    /// Every part in the order in which the pool of the web version starts a whole mass on
    /// `workers` workers, and `fux mass` with `workers` parts at once: [`MassPlan::pool_order_of`]
    /// over all parts.
    pub fn pool_order(&self, workers: usize) -> Vec<PartRef> {
        let parts: Vec<PartRef> = self.parts().map(|(at, _)| at).collect();
        self.pool_order_of(&parts, workers)
    }

    /// The parts `parts` in the order in which the pool of the web version starts them on
    /// `workers` workers, each worker taking the first part of it that is ready once it is free
    /// (web/pool.js, `order` on the jobs of a run; K12 (a)), and in which `fux mass` begins them
    /// with `workers` parts at once (K6): Kyrie I first where it gives the head motif
    /// ([`MassPlan::motif_source`]) and is among them; then the `workers − 1` longest of the
    /// others, which bound the time of the whole run; then the rest movement by movement in the
    /// order of the mass, so that whole movements are done early, each movement's longest part
    /// first, except that the last [`tables::POOL_TAIL`] per worker go longest first, so that the
    /// workers finish about together. Of parts with equal estimates the one earlier in the mass
    /// comes first. Where the parts besides Kyrie I are no more than `workers − 1` and the tail,
    /// they all go longest first.
    pub fn pool_order_of(&self, parts: &[PartRef], workers: usize) -> Vec<PartRef> {
        let workers = workers.max(1);
        let source = self.motif_source().filter(|kyrie| parts.contains(kyrie));
        // Unscaled: the page orders the parts by their estimates times one factor, which keeps
        // their order, while such a product in f32 could round two estimates to one.
        let seconds = |at: &PartRef| self.part(*at).estimate.seconds;
        // PartRef orders as the mass does.
        let longer = |a: &PartRef, b: &PartRef| seconds(b).total_cmp(&seconds(a)).then(a.cmp(b));
        let mut longest: Vec<PartRef> = parts
            .iter()
            .copied()
            .filter(|&at| Some(at) != source)
            .collect();
        longest.sort_by(longer);
        let mut rest = longest.split_off((workers - 1).min(longest.len()));
        rest.sort_by(|a, b| a.movement.cmp(&b.movement).then_with(|| longer(a, b)));
        let end = rest.len().saturating_sub(tables::POOL_TAIL * workers);
        rest[end..].sort_by(longer);
        source.into_iter().chain(longest).chain(rest).collect()
    }

    /// The seconds the parts take on `workers` workers side by side, each part `factor` times as
    /// long as natively (about 5 in WebAssembly), as the pool of the web version sets them, and
    /// `fux mass` with `workers` parts at once (its estimate, with [`tables::estimate_jobs`] as
    /// the factor): each worker, once free, takes the first part of [`MassPlan::pool_order`]
    /// that is ready; a part that opens with the motif is ready once Kyrie I is set, and a worker
    /// with only such parts left waits for it.
    pub fn pool_seconds(&self, workers: usize, factor: f32) -> f32 {
        /// A part to set: its seconds, whether it gives the head motif, whether it waits for it.
        struct Job {
            seconds: f32,
            source: bool,
            waits: bool,
        }
        let source = self.motif_source();
        let mut waiting: Vec<Job> = self
            .pool_order(workers)
            .into_iter()
            .map(|at| Job {
                seconds: self.part(at).estimate.seconds * factor,
                source: source == Some(at),
                waits: self.part(at).motif,
            })
            .collect();
        let mut free = vec![0.0_f32; workers.max(1)];
        // When the head motif is known: once Kyrie I is set, at once without it.
        let mut motif = if waiting.iter().any(|job| job.source) {
            f32::INFINITY
        } else {
            0.0
        };
        while !waiting.is_empty() {
            // The worker free first, the first of equal ones.
            let (worker, &earliest) = free
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.total_cmp(b.1))
                .expect("one worker at least");
            let mut time = earliest;
            if !waiting.iter().any(|job| !job.waits || motif <= time) {
                time = time.max(motif);
            }
            let Some(i) = waiting.iter().position(|job| !job.waits || motif <= time) else {
                break;
            };
            let job = waiting.remove(i);
            free[worker] = time + job.seconds.max(0.0);
            if job.source {
                motif = free[worker];
            }
        }
        free.into_iter().fold(0.0, f32::max)
    }

    /// Kyrie I, whose first subject gives the head motif (MASS-MOTIF), in a mass with a head motif
    /// ([`MassPlan::motif`]) whose plan sets the Kyrie, also where no part of the plan takes the
    /// motif: its result and its file keep the motif for the movements set later (`--resume`);
    /// `None` in a mass without a head motif or a plan without the Kyrie.
    pub fn motif_source(&self) -> Option<PartRef> {
        if !self.motif {
            return None;
        }
        let movement = self
            .movements
            .iter()
            .position(|m| m.movement == Movement::Kyrie)?;
        Some(PartRef {
            movement,
            section: 0,
            part: 0,
        })
    }

    /// The previous part's ending in the movement, which the chain of internal cadences of a
    /// part starts from; `None` for the first part of a movement.
    pub fn ending_before(&self, at: PartRef) -> Option<Step> {
        let movement = &self.movements[at.movement];
        let mut before = None;
        for (s, section) in movement.sections.iter().enumerate() {
            for (p, part) in section.parts.iter().enumerate() {
                if (s, p) == (at.section, at.part) {
                    return before;
                }
                before = Some(part.ending);
            }
        }
        before
    }

    /// What an address names; see the [module documentation](self).
    pub(crate) fn target(&self, at: &str) -> Result<Target, MassOptionError> {
        let trimmed = at.trim();
        if trimmed.is_empty() {
            return Ok(Target::All);
        }
        if let Some((movement, number)) = trimmed.split_once('/') {
            let unknown = || self.unknown(at);
            let movement: Movement = movement.parse().map_err(|_| unknown())?;
            let number: usize = number.trim().parse().map_err(|_| unknown())?;
            let Some(m) = self.movements.iter().position(|p| p.movement == movement) else {
                return Err(MassOptionError::NotPlanned {
                    at: at.into(),
                    movement,
                });
            };
            let plan = &self.movements[m];
            let mut left = number.checked_sub(1);
            for (s, section) in plan.sections.iter().enumerate() {
                match left {
                    Some(p) if p < section.parts.len() => {
                        return Ok(Target::Part(PartRef {
                            movement: m,
                            section: s,
                            part: p,
                        }));
                    }
                    Some(p) => left = Some(p - section.parts.len()),
                    None => break,
                }
            }
            return Err(MassOptionError::NoPart {
                at: at.into(),
                movement,
                parts: plan.part_count(),
            });
        }
        let name = normalized(trimmed);
        for (m, movement) in self.movements.iter().enumerate() {
            for (s, section) in movement.sections.iter().enumerate() {
                if normalized(section.id) == name {
                    return Ok(Target::Section(m, s));
                }
            }
        }
        for (at_part, part) in self.parts() {
            if let Join::Break(label) = part.join
                && (normalized(part.first_text()) == name || normalized(label) == name)
            {
                return Ok(Target::Part(at_part));
            }
        }
        // A name of a movement the request leaves out, in either form.
        let elsewhere = preset::movement_of(&name.replace(' ', "-")).or_else(|| {
            [Preset::Brevis, Preset::Normal]
                .into_iter()
                .flat_map(|preset| Movement::ALL.map(|movement| preset::form(preset, movement)))
                .find(|form| {
                    form.sections.iter().any(|section| {
                        section
                            .pieces
                            .iter()
                            .any(|piece| piece.label.is_some_and(|l| normalized(l) == name))
                    })
                })
                .map(|form| form.movement)
        });
        match elsewhere {
            Some(movement) if !self.movements.iter().any(|p| p.movement == movement) => {
                Err(MassOptionError::NotPlanned {
                    at: at.into(),
                    movement,
                })
            }
            _ => Err(self.unknown(at)),
        }
    }

    /// The error for an address that names nothing, with the names that would.
    pub(crate) fn unknown(&self, at: &str) -> MassOptionError {
        let mut valid: Vec<String> = Vec::new();
        for movement in &self.movements {
            for section in &movement.sections {
                valid.push(section.id.to_string());
                for part in &section.parts {
                    if let Join::Break(_) = part.join {
                        valid.push(part.first_text().to_string());
                    }
                }
            }
        }
        MassOptionError::UnknownAddress {
            at: at.into(),
            valid,
        }
    }
}

/// The usual choir of a mass of `voices` voices, from the top: two S T, three A T B, four
/// S A T B, five S A T T B, six S S A T T B, as `fux free` sets them.
pub fn choir(voices: usize) -> Vec<VoiceType> {
    match voices {
        0..=2 => vec![VoiceType::Soprano, VoiceType::Tenor],
        n => choir_with(n, VoiceType::Alto, None),
    }
}

/// The names of voices in a language: a type that sings more than once is numbered from the top
/// („Tenor 1“, „Tenor 2“), as a score names its voices.
pub fn voice_names(voices: &[VoiceType], lang: Lang) -> Vec<String> {
    let mut seen = [0_usize; VoiceType::ALL.len()];
    voices
        .iter()
        .map(|&voice| {
            let name = voice.name(lang);
            if voices.iter().filter(|&&v| v == voice).count() > 1 {
                let number = &mut seen[voice as usize];
                *number += 1;
                format!("{name} {number}")
            } else {
                name.to_string()
            }
        })
        .collect()
}

/// The voices of a movement whose sections sing `sections` (the voice types of each, from the
/// top): every type as often as the section that has it most, from the top, so that the k-th voice
/// of a type in a section is the k-th of that type in the movement ([`voice_places`]).
pub(crate) fn movement_voices(sections: &[Vec<VoiceType>]) -> Vec<VoiceType> {
    let mut voices = Vec::new();
    for voice in VoiceType::ALL {
        let most = sections
            .iter()
            .map(|section| section.iter().filter(|&&v| v == voice).count())
            .max()
            .unwrap_or(0);
        voices.extend(std::iter::repeat_n(voice, most));
    }
    voices
}

/// The places of `kinds`, the voices of a section or a part from the top, among `voices`, those of
/// its movement: the k-th voice of a type is the k-th of that type in the movement (Tenor 1,
/// Tenor 2). `None` where the movement lacks one.
pub(crate) fn voice_places(voices: &[VoiceType], kinds: &[VoiceType]) -> Option<Vec<usize>> {
    let mut taken = [0_usize; VoiceType::ALL.len()];
    kinds
        .iter()
        .map(|&kind| {
            let k = &mut taken[kind as usize];
            let place = voices
                .iter()
                .enumerate()
                .filter(|&(_, &v)| v == kind)
                .nth(*k)
                .map(|(i, _)| i);
            *k += 1;
            place
        })
        .collect()
}

/// An option of a mass that names what the plan does not have, or asks what it cannot give. The
/// plan fails with it before any part is searched. Counts and parts count from 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MassOptionError {
    /// A choir of fewer than two or more than six voices.
    Voices {
        voices: usize,
    },
    /// No movement to set.
    NoMovement,
    /// A movement named twice.
    MovementTwice {
        movement: Movement,
    },
    UnknownMovement {
        name: String,
    },
    UnknownPreset {
        name: String,
    },
    UnknownLengths {
        name: String,
    },
    UnknownChant {
        name: String,
    },
    /// A size of the parts that is neither a number of syllables in [`tables::CHUNK_RANGE`] nor
    /// `aus`.
    Chunk {
        given: String,
    },
    /// An item of `--scoring`, `--imitation` or `--endings` that is not of the form
    /// `address=value`.
    Item {
        option: MassOption,
        item: String,
    },
    /// An address that names no part, section or Break of the plan; `valid` lists the sections
    /// and Breaks that it has.
    UnknownAddress {
        at: String,
        valid: Vec<String>,
    },
    /// An address in a movement that the request does not set.
    NotPlanned {
        at: String,
        movement: Movement,
    },
    /// A part beyond the parts of its movement.
    NoPart {
        at: String,
        movement: Movement,
        parts: usize,
    },
    /// `--scoring` for a part or a Break instead of a section.
    ScoringAddress {
        at: String,
    },
    /// A section of fewer than two or more than six voices.
    ScoringVoices {
        at: String,
        voices: usize,
    },
    UnknownVoice {
        at: String,
        name: String,
    },
    /// An `--imitation` value other than auto, an or aus.
    UnknownToggle {
        at: String,
        value: String,
    },
    /// An ending that is not a step of the scale.
    UnknownStep {
        at: String,
        value: String,
    },
    /// An ending on a step without a clausula: its fifth is diminished.
    EndingClausula {
        at: String,
        step: Step,
        mode: ModeSpec,
    },
    /// An ending on a degree other than the final, the fifth and the fourth.
    EndingDegree {
        at: String,
        step: Step,
        mode: ModeSpec,
    },
    /// An ending for the last part of a movement, which ends on the final.
    EndingLast {
        at: String,
        movement: Movement,
    },
    /// An ending other than the final for a part in two voices (CAD-END).
    EndingTwoVoices {
        at: String,
    },
    /// An option for the Osanna ut supra, which repeats the Osanna of the Sanctus.
    UtSupra {
        at: String,
    },
    /// The Benedictus of the form `normal`, whose Osanna ut supra repeats the Sanctus', without
    /// the Sanctus.
    UtSupraWithoutSanctus,
    /// The head motif together with a chant paraphrase.
    MotifWithChant,
    /// The head motif without the Kyrie it comes from.
    MotifWithoutKyrie,
}

/// The options of a mass that take addresses, for their messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MassOption {
    Scoring,
    Imitation,
    Endings,
}

impl MassOption {
    /// The option as the command line names it.
    pub const fn flag(self) -> &'static str {
        match self {
            MassOption::Scoring => "--scoring",
            MassOption::Imitation => "--imitation",
            MassOption::Endings => "--endings",
        }
    }

    /// The form of an item, for a message.
    fn form(self, lang: Lang) -> &'static str {
        match self {
            MassOption::Scoring => lang.pick("Abschnitt=Stimmen", "section=voices"),
            MassOption::Imitation => lang.pick("Stelle=auto|an|aus", "place=auto|on|off"),
            MassOption::Endings => lang.pick("Stelle=Stufe", "place=degree"),
        }
    }
}

/// The movements by their ids, for messages.
const MOVEMENT_IDS: &str = "kyrie, gloria, credo, sanctus, benedictus, agnus";

impl MassOptionError {
    /// Message for users in their language: a sentence without its full stop, as
    /// [`GenerateError`](crate::generate::GenerateError) completes it.
    pub fn message(&self, lang: Lang) -> String {
        let de = lang == Lang::De;
        match self {
            MassOptionError::Voices { voices } => match lang {
                Lang::De => format!("Eine Messe braucht zwei bis sechs Stimmen, nicht {voices}"),
                Lang::En => format!("a mass needs two to six voices, not {voices}"),
            },
            MassOptionError::NoMovement => match lang {
                Lang::De => format!("Kein Satz gewählt; die Sätze sind {MOVEMENT_IDS}"),
                Lang::En => format!("no movement chosen; the movements are {MOVEMENT_IDS}"),
            },
            MassOptionError::MovementTwice { movement } => match lang {
                Lang::De => format!("Der Satz {movement} ist zweimal genannt"),
                Lang::En => format!("the movement {movement} is named twice"),
            },
            MassOptionError::UnknownMovement { name } => match lang {
                Lang::De => format!("Unbekannter Satz „{name}“; die Sätze sind {MOVEMENT_IDS}"),
                Lang::En => {
                    format!("unknown movement \"{name}\"; the movements are {MOVEMENT_IDS}")
                }
            },
            MassOptionError::UnknownPreset { name } => match lang {
                Lang::De => {
                    format!("Unbekannte Form der Messe „{name}“; die Formen sind brevis und normal")
                }
                Lang::En => {
                    format!("unknown form of the mass \"{name}\"; the forms are brevis and normal")
                }
            },
            MassOptionError::UnknownLengths { name } => match lang {
                Lang::De => {
                    format!("Unbekannte Länge „{name}“; die Längen sind palestrina und natürlich")
                }
                Lang::En => format!(
                    "unknown lengths \"{name}\"; the lengths are palestrina and natürlich (natural)"
                ),
            },
            MassOptionError::UnknownChant { name } => match lang {
                Lang::De => {
                    format!("Unbekannter Choral „{name}“; zur Wahl stehen keiner, ix und iv")
                }
                Lang::En => {
                    format!("unknown chant \"{name}\"; the choices are keiner (none), ix and iv")
                }
            },
            MassOptionError::Chunk { given } => {
                let (low, high) = (tables::CHUNK_RANGE.start(), tables::CHUNK_RANGE.end());
                match lang {
                    Lang::De => format!(
                        "Die Größe der Teile „{given}“ ist weder eine Zahl von {low} bis {high} \
                         Silben noch „aus“"
                    ),
                    Lang::En => format!(
                        "the size of the parts \"{given}\" is neither a number of {low} to {high} \
                         syllables nor \"aus\" (off)"
                    ),
                }
            }
            MassOptionError::Item { option, item } => match lang {
                Lang::De => format!(
                    "{}: „{item}“ hat nicht die Form {}",
                    option.flag(),
                    option.form(lang)
                ),
                Lang::En => format!(
                    "{}: \"{item}\" is not of the form {}",
                    option.flag(),
                    option.form(lang)
                ),
            },
            MassOptionError::UnknownAddress { at, valid } => match lang {
                Lang::De => format!(
                    "Unbekannte Stelle „{at}“; gültig sind Satz/Teil wie credo/3 und {}",
                    valid.join(", ")
                ),
                Lang::En => format!(
                    "unknown place \"{at}\"; valid are movement/part such as credo/3 and {}",
                    valid.join(", ")
                ),
            },
            MassOptionError::NotPlanned { at, movement } => match lang {
                Lang::De => format!("„{at}“: Das {movement} wird nicht gesetzt"),
                Lang::En => format!("\"{at}\": the {movement} is not set"),
            },
            MassOptionError::NoPart {
                at,
                movement,
                parts,
            } => match (lang, parts) {
                (Lang::De, 1) => format!("„{at}“: Das {movement} hat nur einen Teil"),
                (Lang::De, _) => format!("„{at}“: Das {movement} hat {parts} Teile"),
                (Lang::En, 1) => format!("\"{at}\": the {movement} has one part only"),
                (Lang::En, _) => format!("\"{at}\": the {movement} has {parts} parts"),
            },
            MassOptionError::ScoringAddress { at } => match lang {
                Lang::De => {
                    format!("--scoring nimmt Abschnitte wie benedictus oder agnus3, nicht „{at}“")
                }
                Lang::En => {
                    format!("--scoring takes sections such as benedictus or agnus3, not \"{at}\"")
                }
            },
            MassOptionError::ScoringVoices { at, voices } => match lang {
                Lang::De => {
                    format!("„{at}“: Ein Abschnitt hat zwei bis sechs Stimmen, nicht {voices}")
                }
                Lang::En => format!("\"{at}\": a section has two to six voices, not {voices}"),
            },
            MassOptionError::UnknownVoice { at, name } => match lang {
                Lang::De => format!("„{at}“: Unbekannte Stimmlage „{name}“"),
                Lang::En => format!("\"{at}\": unknown voice type \"{name}\""),
            },
            MassOptionError::UnknownToggle { at, value } => match lang {
                Lang::De => format!("„{at}“: „{value}“ ist nicht auto, an oder aus"),
                Lang::En => format!("\"{at}\": \"{value}\" is not auto, an (on) or aus (off)"),
            },
            MassOptionError::UnknownStep { at, value } => match lang {
                Lang::De => format!("„{at}“: „{value}“ ist keine Stufe von A bis G"),
                Lang::En => format!("\"{at}\": \"{value}\" is no degree from A to G"),
            },
            MassOptionError::EndingClausula { at, step, mode } => {
                let name = step_name(*mode, *step, lang);
                match lang {
                    Lang::De => format!(
                        "„{at}“: Auf {name} gibt es in diesem Modus keine Klausel (verminderte \
                         Quinte)"
                    ),
                    Lang::En => {
                        format!("\"{at}\": no clausula on {name} in this mode (diminished fifth)")
                    }
                }
            }
            MassOptionError::EndingDegree { at, step, mode } => {
                let allowed: Vec<String> = part_endings(*mode)
                    .into_iter()
                    .map(|s| step_name(*mode, s, lang))
                    .collect();
                let list = listed(&allowed, lang.pick("oder", "or"));
                let name = step_name(*mode, *step, lang);
                if de {
                    format!(
                        "„{at}“: Ein Teil endet nicht auf {name}, sondern auf der Finalis, der \
                         Quinte oder der Quarte des Modus mit einer Klausel: {list}"
                    )
                } else {
                    format!(
                        "\"{at}\": a part ends not on {name} but on the final, the fifth or the \
                         fourth of the mode with a clausula: {list}"
                    )
                }
            }
            MassOptionError::EndingLast { at, movement } => match lang {
                Lang::De => format!("„{at}“: Der letzte Teil des {movement} endet auf der Finalis"),
                Lang::En => {
                    format!("\"{at}\": the last part of the {movement} ends on the final")
                }
            },
            MassOptionError::EndingTwoVoices { at } => match lang {
                Lang::De => format!("„{at}“: Ein zweistimmiger Teil endet auf der Finalis"),
                Lang::En => format!("\"{at}\": a part in two voices ends on the final"),
            },
            MassOptionError::UtSupra { at } => match lang {
                Lang::De => format!(
                    "„{at}“: Das Osanna ut supra wiederholt das Osanna des Sanctus und wird nicht \
                     eigens gesetzt"
                ),
                Lang::En => format!(
                    "\"{at}\": the Osanna ut supra repeats the Osanna of the Sanctus and is not \
                     set on its own"
                ),
            },
            MassOptionError::UtSupraWithoutSanctus => match lang {
                Lang::De => "Das Benedictus der Form normal schließt mit dem Osanna des Sanctus \
                             („Osanna ut supra“); dazu gehört das Sanctus"
                    .to_string(),
                Lang::En => "the Benedictus of the form normal ends with the Osanna of the \
                             Sanctus (\"Osanna ut supra\"); choose the Sanctus as well"
                    .to_string(),
            },
            MassOptionError::MotifWithChant => match lang {
                Lang::De => "Kopfmotiv und Choralparaphrase schließen einander aus".to_string(),
                Lang::En => {
                    "the head motif and the chant paraphrase exclude each other".to_string()
                }
            },
            MassOptionError::MotifWithoutKyrie => match lang {
                Lang::De => {
                    "Das Kopfmotiv kommt aus dem Kyrie I; dazu gehört das Kyrie".to_string()
                }
                Lang::En => {
                    "the head motif comes from Kyrie I; choose the Kyrie as well".to_string()
                }
            },
        }
    }
}

impl fmt::Display for MassOptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message(Lang::En))
    }
}

impl std::error::Error for MassOptionError {}

/// A step as the scale of a mode spells it: H and B in German, B and B♭ in English.
fn step_name(mode: ModeSpec, step: Step, lang: Lang) -> String {
    Pitch::new(step, mode.scale_alter(step), 4).class_name(lang)
}

/// Names joined by commas and a last word: „D, A oder G“.
fn listed(names: &[String], last: &str) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., tail] => format!("{} {last} {tail}", init.join(", ")),
    }
}

/// A name as addresses compare it: in lower case, hyphens and underscores as spaces, single
/// spaces.
fn normalized(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .replace(['-', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The items of an option: separated by `;`, empty ones left out.
fn items(text: &str) -> impl Iterator<Item = &str> {
    text.split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

/// The movements of `--movements`: ids or titles, separated by commas, or by spaces in a list
/// without commas (`kyrie gloria`; `kyrie, agnus dei`).
pub fn parse_movements(text: &str) -> Result<Vec<Movement>, MassOptionError> {
    let mut movements: Vec<Movement> = Vec::new();
    let names: Vec<&str> = if text.contains(',') {
        text.split(',').collect()
    } else {
        text.split_whitespace().collect()
    };
    for name in names.into_iter().filter(|name| !name.trim().is_empty()) {
        let movement: Movement = name.parse()?;
        if movements.contains(&movement) {
            return Err(MassOptionError::MovementTwice { movement });
        }
        movements.push(movement);
    }
    if movements.is_empty() {
        return Err(MassOptionError::NoMovement);
    }
    Ok(movements)
}

/// The size of the parts of `--chunk`: a number of syllables, or `aus` (`off`) for none.
pub fn parse_chunk(text: &str) -> Result<Option<usize>, MassOptionError> {
    let error = || MassOptionError::Chunk {
        given: text.trim().into(),
    };
    match normalized(text).as_str() {
        "aus" | "off" | "keine" | "none" => Ok(None),
        number => {
            let syllables: usize = number.parse().map_err(|_| error())?;
            if tables::CHUNK_RANGE.contains(&syllables) {
                Ok(Some(syllables))
            } else {
                Err(error())
            }
        }
    }
}

/// The chant of `--chant`: `keiner` (`none`), `ix` or `iv`.
pub fn parse_chant(text: &str) -> Result<Option<ChantMass>, MassOptionError> {
    match normalized(text).as_str() {
        "keiner" | "kein" | "none" | "aus" | "off" => Ok(None),
        _ => text.parse().map(Some),
    }
}

/// The overrides of `--scoring`: `section=voices` separated by `;`, the voices by spaces or
/// commas: `benedictus=S A T; agnus3=S A T T B`.
pub fn parse_scoring(text: &str) -> Result<Vec<Override>, MassOptionError> {
    items(text)
        .map(|item| {
            let Some((at, voices)) = item.split_once('=') else {
                return Err(MassOptionError::Item {
                    option: MassOption::Scoring,
                    item: item.into(),
                });
            };
            let at = at.trim().to_string();
            let mut parsed = voices
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|name| !name.is_empty())
                .map(|name| {
                    name.parse::<VoiceType>()
                        .map_err(|_| MassOptionError::UnknownVoice {
                            at: at.clone(),
                            name: name.into(),
                        })
                })
                .collect::<Result<Vec<VoiceType>, _>>()?;
            parsed.sort();
            Ok(Override::Scoring { at, voices: parsed })
        })
        .collect()
}

/// The overrides of `--imitation`: `place=auto|an|aus` separated by `;`, a value alone for every
/// part: `auto; incarnatus=aus; credo/3=an`.
pub fn parse_imitation(text: &str) -> Result<Vec<Override>, MassOptionError> {
    items(text)
        .map(|item| {
            let (at, value) = item.split_once('=').unwrap_or(("", item));
            let at = at.trim().to_string();
            let toggle = match normalized(value).as_str() {
                "auto" => Toggle::Auto,
                "an" | "on" => Toggle::On,
                "aus" | "off" => Toggle::Off,
                // A value alone that is none, such as a place without its value.
                _ if !item.contains('=') => {
                    return Err(MassOptionError::Item {
                        option: MassOption::Imitation,
                        item: item.into(),
                    });
                }
                _ => {
                    return Err(MassOptionError::UnknownToggle {
                        at,
                        value: value.trim().into(),
                    });
                }
            };
            Ok(Override::Imitation { at, toggle })
        })
        .collect()
}

/// The overrides of `--endings`: `place=degree` separated by `;`, the degree by its letter, a
/// flat after it being the mode's own (`B` or `Bb` for B♭ in a transposed mode; `H` for B):
/// `christe=A; gloria/3=D`. B is the only step a mode flattens, so a flat after another letter
/// (`Ab`, `Eb`) is no degree of any mode and fails here, rather than be read as the letter alone.
pub fn parse_endings(text: &str) -> Result<Vec<Override>, MassOptionError> {
    items(text)
        .map(|item| {
            let Some((at, value)) = item.split_once('=') else {
                return Err(MassOptionError::Item {
                    option: MassOption::Endings,
                    item: item.into(),
                });
            };
            let at = at.trim().to_string();
            let value = value.trim();
            let mut chars = value.chars();
            let step = chars
                .next()
                .and_then(|letter| match letter {
                    'h' | 'H' => Some(Step::B),
                    letter => Step::from_letter(letter),
                })
                .filter(|&step| match chars.as_str() {
                    "" => true,
                    "b" | "♭" => step == Step::B && !value.starts_with(['h', 'H']),
                    _ => false,
                });
            match step {
                Some(step) => Ok(Override::Ending { at, step }),
                None => Err(MassOptionError::UnknownStep {
                    at,
                    value: value.into(),
                }),
            }
        })
        .collect()
}
