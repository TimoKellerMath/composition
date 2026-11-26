//! Notes, voices and scores.

use std::num::NonZeroU16;
use std::ops::Range;
use std::sync::Arc;

use thiserror::Error;

use crate::clef::VoiceType;
use crate::interval::Interval;
use crate::lang::Lang;
use crate::mode::{Clausula, ModeSpec};
use crate::pitch::{Pitch, Step};
use crate::text::{MAX_SYLLABLES, Text};
use crate::time::{Duration, Meter};

/// A note or a rest. Notes never cross a barline; longer values are written as tied notes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// `None` for a rest.
    pub pitch: Option<Pitch>,
    pub duration: Duration,
    /// Tied to the next note, which must have the same pitch.
    pub tie: bool,
    /// The alteration is musica ficta, printed as an editorial accidental above the note.
    pub ficta: bool,
    /// Text syllable sung on this note, as printed: ending in `-` where the word goes on.
    pub syllable: Option<String>,
    /// The syllable of the score's text that begins on this note. Notes without one continue
    /// the syllable before them; see [`Score::fill_syllables`] for the printed form.
    pub lyric: Option<Lyric>,
}

impl Note {
    pub fn new(pitch: Pitch, duration: Duration) -> Note {
        Note {
            pitch: Some(pitch),
            duration,
            tie: false,
            ficta: false,
            syllable: None,
            lyric: None,
        }
    }

    pub fn rest(duration: Duration) -> Note {
        Note {
            pitch: None,
            duration,
            tie: false,
            ficta: false,
            syllable: None,
            lyric: None,
        }
    }

    pub fn tied(self) -> Note {
        Note { tie: true, ..self }
    }

    pub fn with_ficta(self, ficta: bool) -> Note {
        Note { ficta, ..self }
    }

    pub fn is_rest(&self) -> bool {
        self.pitch.is_none()
    }
}

/// A syllable of the score's text, by its position: what a note sings, without the string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Lyric(NonZeroU16);

impl Lyric {
    /// An imported syllable that matches no position of the text.
    pub const UNKNOWN: Lyric = Lyric(NonZeroU16::MAX);

    /// The syllable at `position` in the text, counted from 0.
    ///
    /// # Panics
    ///
    /// If `position` is not below [`MAX_SYLLABLES`], the most syllables a text can have.
    pub const fn new(position: usize) -> Lyric {
        assert!(
            position < MAX_SYLLABLES,
            "a position beyond the longest possible text"
        );
        // The position plus one: zero is left free for `Option`, `u16::MAX` for UNKNOWN.
        match NonZeroU16::new(position as u16 + 1) {
            Some(n) => Lyric(n),
            None => unreachable!(),
        }
    }

    /// The position in the text, counted from 0; `None` for [`Lyric::UNKNOWN`].
    pub const fn index(self) -> Option<usize> {
        if self.0.get() == u16::MAX {
            None
        } else {
            Some(self.0.get() as usize - 1)
        }
    }
}

/// The text a score sets, and which of its voices sing it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreText {
    pub text: Arc<Text>,
    /// Per voice, in score order: whether it sings the text.
    pub voices: Vec<bool>,
}

/// The five species of counterpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Species {
    /// Note against note.
    First,
    /// Two half notes against one.
    Second,
    /// Four quarter notes against one.
    Third,
    /// Syncopation.
    Fourth,
    /// Florid counterpoint.
    Fifth,
    /// Free counterpoint (freier Satz): florid like the fifth species, but in every voice and
    /// in Palestrina's rhythm, without a cantus firmus.
    Free,
}

impl Species {
    /// The five species of Fux.
    pub const ALL: [Species; 5] = [
        Species::First,
        Species::Second,
        Species::Third,
        Species::Fourth,
        Species::Fifth,
    ];

    /// 1 to 5 for Fux's species, 0 for free counterpoint.
    pub const fn number(self) -> u8 {
        match self {
            Species::Free => 0,
            _ => self as u8 + 1,
        }
    }

    pub fn from_number(number: u8) -> Option<Species> {
        Self::ALL.get(usize::from(number).checked_sub(1)?).copied()
    }

    /// "2. Gattung" or "freier Satz".
    pub fn name(self, lang: Lang) -> String {
        match self {
            Species::Free => lang.pick("freier Satz", "free counterpoint").to_string(),
            _ => format!("{}. {}", self.number(), lang.pick("Gattung", "species")),
        }
    }

    /// Whether the voice is florid: the fifth species or free counterpoint.
    pub const fn is_florid(self) -> bool {
        matches!(self, Species::Fifth | Species::Free)
    }
}

/// What a voice does in the score.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    CantusFirmus,
    Counterpoint(Species),
}

/// One voice of a score.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voice {
    pub name: String,
    pub voice_type: VoiceType,
    pub role: Role,
    pub notes: Vec<Note>,
}

/// A pitch sounding from `start` to `end` (in ticks), merged from tied notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sounding {
    pub pitch: Pitch,
    pub start: u32,
    pub end: u32,
    pub ficta: bool,
    /// Index of the first note in the voice.
    pub note: usize,
}

impl Voice {
    /// A voice named after its type.
    pub fn new(voice_type: VoiceType, role: Role, notes: Vec<Note>) -> Voice {
        Voice {
            name: voice_type.name(Lang::De).to_string(),
            voice_type,
            role,
            notes,
        }
    }

    pub fn is_cantus_firmus(&self) -> bool {
        self.role == Role::CantusFirmus
    }

    pub fn total_ticks(&self) -> u32 {
        self.notes.iter().map(|n| n.duration.ticks()).sum()
    }

    /// Notes with their start tick.
    pub fn timed(&self) -> impl Iterator<Item = (u32, &Note)> {
        self.notes.iter().scan(0, |tick, note| {
            let start = *tick;
            *tick += note.duration.ticks();
            Some((start, note))
        })
    }

    /// Sounding pitches in order, with tied notes merged into one.
    pub fn soundings(&self) -> Vec<Sounding> {
        let mut out: Vec<Sounding> = Vec::with_capacity(self.notes.len());
        let mut tied = false;
        for (index, (start, note)) in self.timed().enumerate() {
            let end = start + note.duration.ticks();
            match note.pitch {
                None => tied = false,
                Some(pitch) => {
                    match out.last_mut() {
                        Some(last) if tied && last.end == start && last.pitch == pitch => {
                            last.end = end;
                        }
                        _ => out.push(Sounding {
                            pitch,
                            start,
                            end,
                            ficta: note.ficta,
                            note: index,
                        }),
                    }
                    tied = note.tie;
                }
            }
        }
        out
    }

    /// The pitches of all sounding notes, tied notes counted once.
    pub fn pitches(&self) -> Vec<Pitch> {
        self.soundings().into_iter().map(|s| s.pitch).collect()
    }
}

/// An internal cadence: the clausula arrives on `target` at the start of `measure` (from 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cadence {
    pub measure: usize,
    pub target: Step,
}

/// The imitation a setting was planned with (phase 13): its points in order, each with the
/// entries of its subject. Where an entry stands and what its head sings says everything else:
/// its interval to the leader, the kind of answer and the degree it begins on follow from its
/// notes, so the plan cannot contradict itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreImitation {
    pub points: Vec<ScorePoint>,
}

/// A point of imitation: words of the text that the voices take up one after the other, each
/// with the head of the subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScorePoint {
    /// The point's syllables, by their positions in the text.
    pub words: Range<u16>,
    /// The entries in time order; the first is the leader's.
    pub entries: Vec<ScoreEntry>,
    /// The measure (from 0) of the internal cadence that ends the point; `None` for the last
    /// point, which the final cadence ends.
    pub goal: Option<usize>,
}

/// An entry of a point: a voice that sings the head of the subject from `start` on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreEntry {
    /// The voice, by its index in the score.
    pub voice: usize,
    /// The tick of the head's first attack.
    pub start: u32,
    /// The least silence before the entry, in ticks; 0 for the voice's first note, before which
    /// it is silent from the start.
    pub rest: u32,
    /// The head as the voice sings it: each pitch with its value in ticks, tied notes merged. A
    /// planned head has four notes or more.
    pub notes: Vec<(Pitch, u32)>,
}

impl ScoreImitation {
    /// The syllables of `text` after which a voice may begin its sentence again in this plan,
    /// besides the ends of units: the last before each point with words of its own that begins
    /// within a unit ([`gate_restarts`](crate::text::underlay::gate_restarts)).
    pub fn restarts(&self, text: &Text) -> Vec<u16> {
        crate::text::underlay::gate_restarts(text, self.points.iter().map(|p| &p.words))
    }

    /// Where voice `voice` may take up the words of the points that set words of their own: for
    /// each point after the first whose words go beyond those of every point before it, its first
    /// such syllable and a tick. That is the voice's own first entry in the point, before which it
    /// sings the words before them or rests; for a voice without an entry there, the point's
    /// second entry, so that the leader brings the new words alone and they are answered before a
    /// free voice joins in. The points that sing earlier words again (a tail or a restatement)
    /// bind nothing ([`Gates`](crate::text::underlay::Gates)).
    pub fn gates(&self, voice: usize) -> Vec<(u16, u32)> {
        let mut out = Vec::new();
        // The syllables before this one belong to points before.
        let mut set = 0;
        for (p, point) in self.points.iter().enumerate() {
            let own = point.words.start.max(set);
            set = set.max(point.words.end);
            if p == 0 || own >= point.words.end {
                continue;
            }
            let entered = point
                .entries
                .iter()
                .filter(|e| e.voice == voice)
                .map(|e| e.start)
                .min();
            let joined = point
                .entries
                .get(1)
                .or_else(|| point.entries.first())
                .map(|e| e.start);
            if let Some(tick) = entered.or(joined) {
                out.push((own, tick));
            }
        }
        out
    }

    /// The plan as one line of text, for a `miscellaneous-field` of MusicXML; [`parse`] reads it
    /// back. Each point gives two fields, separated by `|`: its syllables with the measure of
    /// its goal, as in `p0-7g11` (syllables 0 to 6, the cadence in measure 11 from 0) or `p3-7`
    /// without a goal, and its entries, separated by `;`. An entry is written as
    /// `voice@start/rest:` followed by its head, such as `1@0/0:D4.8,E4.4,F4.4,G4.8`: pitch and
    /// value of each note, separated by commas.
    ///
    /// [`parse`]: ScoreImitation::parse
    pub fn spec(&self) -> String {
        let mut fields = Vec::new();
        for point in &self.points {
            let goal = point.goal.map_or(String::new(), |goal| format!("g{goal}"));
            fields.push(format!("p{}-{}{goal}", point.words.start, point.words.end));
            let entries: Vec<String> = point
                .entries
                .iter()
                .map(|entry| {
                    let notes: Vec<String> = entry
                        .notes
                        .iter()
                        .map(|(pitch, ticks)| format!("{pitch}.{ticks}"))
                        .collect();
                    let (voice, start, rest) = (entry.voice, entry.start, entry.rest);
                    format!("{voice}@{start}/{rest}:{}", notes.join(","))
                })
                .collect();
            fields.push(entries.join(";"));
        }
        fields.join("|")
    }

    /// Reads a plan as [`spec`](ScoreImitation::spec) writes it. It checks only the form; whether
    /// the plan fits a score, [`validate`](ScoreImitation::validate) checks.
    pub fn parse(s: &str) -> Result<ScoreImitation, ScoreError> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(ScoreImitation { points: Vec::new() });
        }
        // An empty part is named by the whole plan.
        let syntax = |part: &str| ScoreError::ImitationSyntax {
            part: if part.is_empty() { s } else { part }.to_string(),
        };
        let fields: Vec<&str> = s.split('|').map(str::trim).collect();
        let mut points = Vec::new();
        for pair in fields.chunks(2) {
            // A point's syllables and its goal, and its entries.
            let &[head, entries] = pair else {
                return Err(syntax(pair[0]));
            };
            let (words, goal) = point_head(head).ok_or_else(|| syntax(head))?;
            let entries = if entries.is_empty() {
                Vec::new()
            } else {
                entries
                    .split(';')
                    .map(|entry| parse_entry(entry.trim()).map_err(syntax))
                    .collect::<Result<_, _>>()?
            };
            points.push(ScorePoint {
                words,
                entries,
                goal,
            });
        }
        Ok(ScoreImitation { points })
    }

    /// Checks that the plan fits the score: every point names syllables of the text (any, if the
    /// score has none) and has entries in time order, its leader not before the leader of the
    /// point before; every entry is in a voice of the score, with a head of notes that last, from
    /// a start no earlier than its rest and up to the end of the score at most; and the goals lie
    /// in the score, each after the one before.
    pub fn validate(&self, score: &Score) -> Result<(), ScoreError> {
        let total = u64::from(score.total_ticks());
        let measures = score.measure_count();
        let syllables = score.text.as_ref().map(|sung| sung.text.syllables.len());
        let (mut leader, mut goal_before) = (0, None);
        for (p, point) in self.points.iter().enumerate() {
            let words = &point.words;
            if words.is_empty() || syllables.is_some_and(|n| usize::from(words.end) > n) {
                return Err(ScoreError::ImitationWords { point: p });
            }
            let in_order = point.entries.windows(2).all(|w| w[0].start <= w[1].start);
            match point.entries.first() {
                Some(first) if in_order && first.start >= leader => leader = first.start,
                _ => return Err(ScoreError::ImitationOrder { point: p }),
            }
            for (e, entry) in point.entries.iter().enumerate() {
                if entry.voice >= score.voices.len() {
                    return Err(ScoreError::ImitationVoice {
                        point: p,
                        entry: e,
                        voice: entry.voice,
                    });
                }
                let ticks: u64 = entry.notes.iter().map(|&(_, t)| u64::from(t)).sum();
                if entry.notes.is_empty()
                    || entry.notes.iter().any(|&(_, t)| t == 0)
                    || entry.rest > entry.start
                    || u64::from(entry.start) + ticks > total
                {
                    return Err(ScoreError::ImitationEntry { point: p, entry: e });
                }
            }
            if let Some(goal) = point.goal {
                if goal >= measures || goal_before.is_some_and(|before| goal <= before) {
                    return Err(ScoreError::ImitationGoal {
                        point: p,
                        measure: goal,
                    });
                }
                goal_before = Some(goal);
            }
        }
        Ok(())
    }
}

/// The first field of a point in [`ScoreImitation::spec`]: its syllables and its goal, such as
/// `p0-7g11`.
fn point_head(field: &str) -> Option<(Range<u16>, Option<usize>)> {
    let rest = field.strip_prefix('p')?;
    let (words, goal) = match rest.split_once('g') {
        Some((words, goal)) => (words, Some(goal.parse().ok()?)),
        None => (rest, None),
    };
    let (from, to) = words.split_once('-')?;
    Some((from.parse().ok()?..to.parse().ok()?, goal))
}

/// An entry in [`ScoreImitation::spec`], such as `1@0/0:D4.8,E4.4,F4.4,G4.8`; fails with the
/// part that cannot be read.
fn parse_entry(entry: &str) -> Result<ScoreEntry, &str> {
    let (voice, start, rest, notes) = entry_place(entry).ok_or(entry)?;
    let notes = notes
        .split(',')
        .map(|note| {
            let note = note.trim();
            // Where a note is missing, the entry is named.
            let part = if note.is_empty() { entry } else { note };
            let (pitch, ticks) = note.split_once('.').ok_or(part)?;
            Ok((
                pitch.parse().map_err(|_| note)?,
                ticks.parse().map_err(|_| note)?,
            ))
        })
        .collect::<Result<_, &str>>()?;
    Ok(ScoreEntry {
        voice,
        start,
        rest,
        notes,
    })
}

/// The voice, the start and the rest of an entry in [`ScoreImitation::spec`], and its notes as
/// they are written.
fn entry_place(entry: &str) -> Option<(usize, u32, u32, &str)> {
    let (place, notes) = entry.split_once(':')?;
    let (voice, timing) = place.split_once('@')?;
    let (start, rest) = timing.split_once('/')?;
    Some((
        voice.trim().parse().ok()?,
        start.trim().parse().ok()?,
        rest.trim().parse().ok()?,
        notes,
    ))
}

/// Where a section of a mass movement begins, or a part within a section (phase 14): a movement
/// joins its sections at double bars and the parts of a long section at held chords, after
/// which every voice rests before the next part begins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionMark {
    /// The measure (from 0) it begins in.
    pub measure: usize,
    pub kind: SectionKind,
    /// The title of a section (Christe), the label of a break (Et incarnatus est); a seam has
    /// none.
    pub title: Option<String>,
}

/// How a section or a part of a mass movement joins what comes before it (phase 14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SectionKind {
    /// A new section after a double bar, with its own voices: Christe, Kyrie II, Agnus Dei II.
    Section,
    /// A part after a held chord within a section that the books name, where the voices rest a
    /// whole measure and begin anew on the downbeat: Qui tollis, Et incarnatus est, Crucifixus
    /// in the Missa brevis of 1570.
    Break,
    /// A part after an unnamed held chord within a section, after a minim's breath: where a long
    /// text is cut into parts at the end of a sentence.
    Seam,
}

/// A piece: a mode, a meter and any number of voices listed from top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Score {
    pub title: Option<String>,
    pub mode: ModeSpec,
    pub meter: Meter,
    pub voices: Vec<Voice>,
    /// Internal cadences; the final cadence is implied.
    pub cadences: Vec<Cadence>,
    /// The text the voices sing, if any. Without it the text rules stay silent, and the notes'
    /// lyrics mean nothing.
    pub text: Option<ScoreText>,
    /// The imitation the setting was planned with, if any (phase 13). Without it the IMI rules
    /// stay silent.
    pub imitation: Option<Arc<ScoreImitation>>,
    /// Where the sections of a mass movement and the parts within them begin, in order (phase
    /// 14). Empty for every other score.
    pub sections: Vec<SectionMark>,
}

/// The notes sounding at one onset of a score.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slice {
    pub tick: u32,
    /// Per voice: what sounds at `tick` (`None` during a rest); up to six voices without a heap
    /// allocation.
    pub sounding: smallvec::SmallVec<[Option<Sounding>; 6]>,
}

impl Slice {
    /// Whether the voice starts a new pitch at this tick.
    pub fn attacks(&self, voice: usize) -> bool {
        matches!(self.sounding[voice], Some(s) if s.start == self.tick)
    }

    /// The lowest sounding voice and its pitch; on equal pitch the lower voice in the score.
    pub fn lowest(&self) -> Option<(usize, Sounding)> {
        self.sounding
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|s| (i, s)))
            .min_by_key(|(i, s)| (s.pitch.midi(), s.pitch.diatonic(), usize::MAX - i))
    }
}

/// Problems that make a score unusable.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ScoreError {
    #[error("the score has no voices")]
    Empty,
    #[error("voice {voice} lasts {ticks} ticks, the first voice {expected}")]
    LengthMismatch {
        voice: usize,
        ticks: u32,
        expected: u32,
    },
    #[error("the score is {ticks} ticks long, not a whole number of {per_measure}-tick measures")]
    IncompleteMeasure { ticks: u32, per_measure: u32 },
    #[error("voice {voice}, note {note}: crosses the barline after measure {measure}")]
    CrossesBarline {
        voice: usize,
        note: usize,
        measure: usize,
    },
    #[error("voice {voice}, note {note}: tied to a rest, a different pitch or nothing")]
    BadTie { voice: usize, note: usize },
    #[error("internal cadence in measure {}: outside the measures 2 to {last}", measure + 1)]
    CadenceOutside { measure: usize, last: usize },
    #[error("internal cadence in measure {}: no clausula on {target:?} in this mode", measure + 1)]
    CadenceTarget { measure: usize, target: Step },
    #[error("the text is given for {voices} voices, but the score has {expected}")]
    TextVoices { voices: usize, expected: usize },
    #[error(
        "voice {voice}, note {note}: no syllable {position} in a text of {syllables} syllables"
    )]
    LyricOutside {
        voice: usize,
        note: usize,
        /// The lyric's position, counted from 0.
        position: usize,
        /// The number of syllables of the text.
        syllables: usize,
    },
    /// The text form of an imitation plan ([`ScoreImitation::spec`]) that cannot be read.
    #[error("the imitation plan cannot be read at \"{part}\"")]
    ImitationSyntax {
        /// The part that cannot be read.
        part: String,
    },
    /// A point of the imitation plan whose words are no syllables of the text, or none. Points
    /// and entries of the plan are counted from 0.
    #[error(
        "the words of point {} of the imitation plan are not syllables of the text",
        point + 1
    )]
    ImitationWords { point: usize },
    /// A point of the imitation plan without entries, with its entries out of time order or with
    /// its leader before the leader of the point before it.
    #[error(
        "the entries of point {} of the imitation plan are missing or out of time order",
        point + 1
    )]
    ImitationOrder { point: usize },
    /// An entry of the imitation plan in a voice that the score does not have.
    #[error(
        "entry {} of point {} of the imitation plan: the score has no voice {}",
        entry + 1,
        point + 1,
        voice + 1
    )]
    ImitationVoice {
        point: usize,
        entry: usize,
        voice: usize,
    },
    /// An entry of the imitation plan without a head, with a note that does not last, with more
    /// rest than the score has before it, or lasting beyond the end of the score.
    #[error(
        "entry {} of point {} of the imitation plan does not fit into the score",
        entry + 1,
        point + 1
    )]
    ImitationEntry { point: usize, entry: usize },
    /// A point of the imitation plan whose goal (a measure from 0) lies outside the score or not
    /// after the goal before it.
    #[error(
        "point {} of the imitation plan: its cadence in measure {} lies outside the score or not after the one before",
        point + 1,
        measure + 1
    )]
    ImitationGoal { point: usize, measure: usize },
    /// A mark of a section or a part (phase 14) in a measure (from 0) that the score does not
    /// have. Marks are counted from 0.
    #[error(
        "section mark {} stands in measure {}, but the score has only {measures} measures",
        mark + 1,
        measure + 1
    )]
    SectionOutside {
        mark: usize,
        measure: usize,
        /// The measures of the score.
        measures: usize,
    },
    /// A mark of a section or a part (phase 14) whose measure is not after that of the mark
    /// before it.
    #[error(
        "section mark {} in measure {} does not come after the one before it",
        mark + 1,
        measure + 1
    )]
    SectionOrder { mark: usize, measure: usize },
}

impl ScoreError {
    /// Message for users: voices and notes counted from 1, lengths in measures.
    pub fn message(&self, lang: Lang, meter: Meter) -> String {
        let measures = |ticks: u32| measures_text(ticks, meter.ticks_per_measure());
        match self {
            ScoreError::Empty => lang
                .pick("Der Satz hat keine Stimmen.", "The score has no voices.")
                .into(),
            ScoreError::LengthMismatch {
                voice,
                ticks,
                expected,
            } => match lang {
                Lang::De => format!(
                    "Stimme {} dauert {} Takte, Stimme 1 aber {} Takte.",
                    voice + 1,
                    measures(*ticks),
                    measures(*expected)
                ),
                Lang::En => format!(
                    "Voice {} lasts {} measures, but voice 1 lasts {}.",
                    voice + 1,
                    measures(*ticks),
                    measures(*expected)
                ),
            },
            ScoreError::IncompleteMeasure { ticks, .. } => match lang {
                Lang::De => format!(
                    "Die Stimmen dauern {} Takte; der letzte Takt ist unvollständig.",
                    measures(*ticks)
                ),
                Lang::En => format!(
                    "The voices last {} measures; the last measure is incomplete.",
                    measures(*ticks)
                ),
            },
            ScoreError::CrossesBarline {
                voice,
                note,
                measure,
            } => match lang {
                Lang::De => format!(
                    "Stimme {}, Note {}: reicht über den Taktstrich nach Takt {measure}; als übergebundene Noten schreiben.",
                    voice + 1,
                    note + 1
                ),
                Lang::En => format!(
                    "Voice {}, note {}: crosses the barline after measure {measure}; write it as tied notes.",
                    voice + 1,
                    note + 1
                ),
            },
            ScoreError::CadenceOutside { measure, last } => match lang {
                Lang::De => format!(
                    "Binnenschluss in Takt {}: Er muss zwischen Takt 2 und Takt {last} liegen.",
                    measure + 1
                ),
                Lang::En => format!(
                    "Internal cadence in measure {}: it must lie between measures 2 and {last}.",
                    measure + 1
                ),
            },
            ScoreError::CadenceTarget { measure, target } => match lang {
                Lang::De => format!(
                    "Binnenschluss in Takt {}: Auf {} ist in diesem Modus keine Klausel möglich.",
                    measure + 1,
                    target.name(lang)
                ),
                Lang::En => format!(
                    "Internal cadence in measure {}: no clausula on {} in this mode.",
                    measure + 1,
                    target.name(lang)
                ),
            },
            ScoreError::BadTie { voice, note } => match lang {
                Lang::De => format!(
                    "Stimme {}, Note {}: Haltebogen zu einer Pause, einer anderen Tonhöhe oder ins Leere.",
                    voice + 1,
                    note + 1
                ),
                Lang::En => format!(
                    "Voice {}, note {}: tie to a rest, a different pitch or nothing.",
                    voice + 1,
                    note + 1
                ),
            },
            ScoreError::TextVoices { voices, expected } => match lang {
                Lang::De => format!(
                    "Der Text ist für {voices} Stimmen angegeben, der Satz hat aber {expected}."
                ),
                Lang::En => {
                    format!("The text is given for {voices} voices, but the score has {expected}.")
                }
            },
            ScoreError::LyricOutside {
                voice,
                note,
                position,
                syllables,
            } => match lang {
                Lang::De => format!(
                    "Stimme {}, Note {}: Der Text hat keine {}. Silbe, nur {syllables} Silben.",
                    voice + 1,
                    note + 1,
                    position + 1
                ),
                Lang::En => format!(
                    "Voice {}, note {}: the text has no syllable {}, only {syllables}.",
                    voice + 1,
                    note + 1,
                    position + 1
                ),
            },
            ScoreError::ImitationSyntax { part } => match lang {
                Lang::De => format!("Der Imitationsplan ist an der Stelle „{part}“ nicht lesbar."),
                Lang::En => format!("The imitation plan cannot be read at \"{part}\"."),
            },
            ScoreError::ImitationWords { point } => match lang {
                Lang::De => format!(
                    "Die Worte von Punkt {} des Imitationsplans sind keine Silben des Textes.",
                    point + 1
                ),
                Lang::En => format!(
                    "The words of point {} of the imitation plan are not syllables of the text.",
                    point + 1
                ),
            },
            ScoreError::ImitationOrder { point } => match lang {
                Lang::De => format!(
                    "Die Einsätze von Punkt {} des Imitationsplans fehlen oder stehen nicht in \
                     zeitlicher Reihenfolge.",
                    point + 1
                ),
                Lang::En => format!(
                    "The entries of point {} of the imitation plan are missing or out of time \
                     order.",
                    point + 1
                ),
            },
            ScoreError::ImitationVoice {
                point,
                entry,
                voice,
            } => match lang {
                Lang::De => format!(
                    "Einsatz {} von Punkt {} des Imitationsplans: Der Satz hat keine Stimme {}.",
                    entry + 1,
                    point + 1,
                    voice + 1
                ),
                Lang::En => format!(
                    "Entry {} of point {} of the imitation plan: the score has no voice {}.",
                    entry + 1,
                    point + 1,
                    voice + 1
                ),
            },
            ScoreError::ImitationEntry { point, entry } => match lang {
                Lang::De => format!(
                    "Einsatz {} von Punkt {} des Imitationsplans passt nicht in den Satz.",
                    entry + 1,
                    point + 1
                ),
                Lang::En => format!(
                    "Entry {} of point {} of the imitation plan does not fit into the score.",
                    entry + 1,
                    point + 1
                ),
            },
            ScoreError::ImitationGoal { point, measure } => match lang {
                Lang::De => format!(
                    "Punkt {} des Imitationsplans: Sein Binnenschluss in Takt {} liegt außerhalb \
                     des Satzes oder nicht nach dem vorigen.",
                    point + 1,
                    measure + 1
                ),
                Lang::En => format!(
                    "Point {} of the imitation plan: its cadence in measure {} lies outside the \
                     score or not after the one before.",
                    point + 1,
                    measure + 1
                ),
            },
            ScoreError::SectionOutside {
                mark,
                measure,
                measures,
            } => match lang {
                Lang::De => format!(
                    "Die {}. Abschnittsmarke steht in Takt {}, der Satz hat aber nur {measures} \
                     Takte.",
                    mark + 1,
                    measure + 1
                ),
                Lang::En => format!(
                    "Section mark {} stands in measure {}, but the score has only {measures} \
                     measures.",
                    mark + 1,
                    measure + 1
                ),
            },
            ScoreError::SectionOrder { mark, measure } => match lang {
                Lang::De => format!(
                    "Die {}. Abschnittsmarke in Takt {} steht nicht nach der vorigen.",
                    mark + 1,
                    measure + 1
                ),
                Lang::En => format!(
                    "Section mark {} in measure {} does not come after the one before it.",
                    mark + 1,
                    measure + 1
                ),
            },
        }
    }
}

/// Ticks as measures, e.g. `3` or `2 1/2`.
fn measures_text(ticks: u32, per_measure: u32) -> String {
    let (whole, rest) = (ticks / per_measure, ticks % per_measure);
    if rest == 0 {
        return whole.to_string();
    }
    let divisor = gcd(rest, per_measure);
    let fraction = format!("{}/{}", rest / divisor, per_measure / divisor);
    if whole == 0 {
        fraction
    } else {
        format!("{whole} {fraction}")
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

impl Score {
    pub fn new(mode: ModeSpec, voices: Vec<Voice>) -> Score {
        Score {
            title: None,
            mode,
            meter: Meter::default(),
            voices,
            cadences: Vec::new(),
            text: None,
            imitation: None,
            sections: Vec::new(),
        }
    }

    pub fn total_ticks(&self) -> u32 {
        self.voices
            .iter()
            .map(Voice::total_ticks)
            .max()
            .unwrap_or(0)
    }

    pub fn measure_count(&self) -> usize {
        self.total_ticks().div_ceil(self.meter.ticks_per_measure()) as usize
    }

    /// Start of a note, by voice and index in the voice.
    pub fn note_start(&self, voice: usize, note: usize) -> Option<u32> {
        self.voices
            .get(voice)?
            .timed()
            .nth(note)
            .map(|(start, _)| start)
    }

    /// Whether the score is free counterpoint: every voice free, no cantus firmus.
    pub fn is_free(&self) -> bool {
        !self.voices.is_empty()
            && self
                .voices
                .iter()
                .all(|v| v.role == Role::Counterpoint(Species::Free))
    }

    /// Index and voice of the cantus firmus, if there is one.
    pub fn cantus_firmus(&self) -> Option<(usize, &Voice)> {
        self.voices
            .iter()
            .enumerate()
            .find(|(_, v)| v.is_cantus_firmus())
    }

    /// Checks that all voices have the same whole number of measures, that no note crosses a
    /// barline and that ties connect equal pitches. With a text, it also checks that the text
    /// says of every voice whether it sings and that every lyric is a syllable of the text or
    /// [`Lyric::UNKNOWN`]. Without a text the lyrics are not checked: nothing reads them. With an
    /// imitation plan, it checks that the plan fits ([`ScoreImitation::validate`]); with marks of
    /// sections, that each lies in a measure of the score after the one before it.
    pub fn validate(&self) -> Result<(), ScoreError> {
        let first = self.voices.first().ok_or(ScoreError::Empty)?;
        let expected = first.total_ticks();
        let per_measure = self.meter.ticks_per_measure();
        if expected % per_measure != 0 {
            return Err(ScoreError::IncompleteMeasure {
                ticks: expected,
                per_measure,
            });
        }
        for (v, voice) in self.voices.iter().enumerate() {
            let ticks = voice.total_ticks();
            if ticks != expected {
                return Err(ScoreError::LengthMismatch {
                    voice: v,
                    ticks,
                    expected,
                });
            }
            for (n, (start, note)) in voice.timed().enumerate() {
                let end = start + note.duration.ticks();
                let (measure, _) = self.meter.locate(start);
                if end > (measure as u32 + 1) * per_measure {
                    return Err(ScoreError::CrossesBarline {
                        voice: v,
                        note: n,
                        measure: measure + 1,
                    });
                }
                if note.tie {
                    let next = voice.notes.get(n + 1).and_then(|next| next.pitch);
                    if note.pitch.is_none() || next != note.pitch {
                        return Err(ScoreError::BadTie { voice: v, note: n });
                    }
                }
            }
        }
        // An internal cadence needs its penultimate measure and must end before the final
        // cadence begins.
        let measures = (expected / per_measure) as usize;
        for cadence in &self.cadences {
            let last = measures.saturating_sub(2);
            if cadence.measure < 1 || cadence.measure + 3 > measures {
                return Err(ScoreError::CadenceOutside {
                    measure: cadence.measure,
                    last,
                });
            }
            if self.mode.clausula(cadence.target).is_none() {
                return Err(ScoreError::CadenceTarget {
                    measure: cadence.measure,
                    target: cadence.target,
                });
            }
        }
        for (m, mark) in self.sections.iter().enumerate() {
            if mark.measure >= measures {
                return Err(ScoreError::SectionOutside {
                    mark: m,
                    measure: mark.measure,
                    measures,
                });
            }
            if m > 0 && mark.measure <= self.sections[m - 1].measure {
                return Err(ScoreError::SectionOrder {
                    mark: m,
                    measure: mark.measure,
                });
            }
        }
        self.validate_text()?;
        match &self.imitation {
            Some(imitation) => imitation.validate(self),
            None => Ok(()),
        }
    }

    /// The checks of [`Score::validate`] that concern the text.
    fn validate_text(&self) -> Result<(), ScoreError> {
        let Some(sung) = &self.text else {
            return Ok(());
        };
        if sung.voices.len() != self.voices.len() {
            return Err(ScoreError::TextVoices {
                voices: sung.voices.len(),
                expected: self.voices.len(),
            });
        }
        let syllables = sung.text.syllables.len();
        for (v, voice) in self.voices.iter().enumerate() {
            for (n, note) in voice.notes.iter().enumerate() {
                if let Some(position) = note.lyric.and_then(Lyric::index)
                    && position >= syllables
                {
                    return Err(ScoreError::LyricOutside {
                        voice: v,
                        note: n,
                        position,
                        syllables,
                    });
                }
            }
        }
        Ok(())
    }

    /// Writes the syllables of the voices that sing the text from their lyrics, as they are
    /// printed under the notes (the convention of [`crate::gabc::ChantNote::syllable`]): the book
    /// form, with the acute only if `accents` is set, a `-` if the word goes on, and the word's
    /// punctuation after its last syllable. A note without a lyric gets no syllable; one whose
    /// lyric is [`Lyric::UNKNOWN`] or outside the text keeps its own. Other voices, and scores
    /// without a text, stay as they are.
    pub fn fill_syllables(&mut self, accents: bool) {
        let Some(sung) = &self.text else {
            return;
        };
        let text = &sung.text;
        for (voice, _) in self
            .voices
            .iter_mut()
            .zip(&sung.voices)
            .filter(|(_, sings)| **sings)
        {
            for note in &mut voice.notes {
                match note.lyric.map(Lyric::index) {
                    None => note.syllable = None,
                    Some(Some(i)) if i < text.syllables.len() => {
                        note.syllable = Some(text.syllable_label(i, accents));
                    }
                    Some(_) => {}
                }
            }
        }
    }

    /// The syllables of a voice as one line, with spaces between the words, such as
    /// `Ký-ri-e e-lé-i-son. e-lé-i-son.` for Kyrie I with its tail restated; `None` if the voice
    /// has no syllables. It reads the printed syllables, so a generated score needs
    /// [`Score::fill_syllables`] first.
    pub fn voice_lyrics(&self, v: usize) -> Option<String> {
        let syllables = self.voices.get(v)?.notes.iter().filter_map(|note| {
            let syllable = note.syllable.as_deref()?.trim();
            (!syllable.is_empty()).then_some(syllable)
        });
        let mut line = String::new();
        for syllable in syllables {
            if !line.is_empty() && !line.ends_with('-') {
                line.push(' ');
            }
            line.push_str(syllable);
        }
        (!line.is_empty()).then_some(line)
    }

    /// Raises the cantus firmus' note before a cadence where it rises by a whole step into a
    /// target with a leading tone (C–D becomes C♯–D), as singers did, and marks it as ficta. Then
    /// the cantus carries the discant's clausula itself. Concerns chant melodies; Fux's cantus
    /// fall into their final.
    pub fn raise_cantus_leading_tones(&mut self) {
        let per = self.meter.ticks_per_measure();
        let total = self.total_ticks();
        let mode = self.mode;
        let mut goals: Vec<(u32, Step)> = self
            .cadences
            .iter()
            .map(|c| (c.measure as u32 * per, c.target))
            .collect();
        if total >= per {
            goals.push((total - per, mode.final_step()));
        }
        for voice in self.voices.iter_mut().filter(|v| v.is_cantus_firmus()) {
            let starts: Vec<u32> = voice.timed().map(|(start, _)| start).collect();
            for &(goal, target) in &goals {
                if mode.clausula(target) != Some(Clausula::Leading) {
                    continue;
                }
                let Some(i) = starts.iter().position(|&start| start == goal) else {
                    continue;
                };
                let (Some(before), Some(to)) = (
                    i.checked_sub(1).and_then(|b| voice.notes[b].pitch),
                    voice.notes[i].pitch,
                ) else {
                    continue;
                };
                let step = Interval::between(before, to);
                if to.step == target && step.steps == 1 && step.semitones == 2 {
                    let note = &mut voice.notes[i - 1];
                    note.pitch = Some(to.transpose(-Interval::MINOR_SECOND));
                    note.ficta = true;
                }
            }
        }
    }

    /// The measures (from 0) in which internal cadences arrive, in order.
    pub fn cadence_goals(&self) -> Vec<usize> {
        let mut goals: Vec<usize> = self.cadences.iter().map(|c| c.measure).collect();
        goals.sort_unstable();
        goals.dedup();
        goals
    }

    /// One slice for every tick at which any voice starts a note or a rest.
    pub fn slices(&self) -> Vec<Slice> {
        let soundings: Vec<Vec<Sounding>> = self.voices.iter().map(Voice::soundings).collect();
        self.slices_from(&soundings)
    }

    /// The slices, from the voices' sounding notes as [`Voice::soundings`] gives them.
    pub fn slices_from(&self, soundings: &[Vec<Sounding>]) -> Vec<Slice> {
        // Every voice's onsets are already in order: merged, they give the ticks in order, each
        // once.
        let mut onsets: Vec<std::iter::Peekable<_>> = self
            .voices
            .iter()
            .map(|v| v.timed().map(|(start, _)| start).peekable())
            .collect();
        let mut ticks: Vec<u32> =
            Vec::with_capacity(self.voices.iter().map(|v| v.notes.len()).max().unwrap_or(0) * 2);
        while let Some(tick) = onsets.iter_mut().filter_map(|o| o.peek().copied()).min() {
            ticks.push(tick);
            for o in &mut onsets {
                while o.next_if_eq(&tick).is_some() {}
            }
        }
        let mut cursors = vec![0; soundings.len()];
        ticks
            .into_iter()
            .map(|tick| Slice {
                tick,
                sounding: soundings
                    .iter()
                    .zip(cursors.iter_mut())
                    .map(|(line, cursor)| {
                        while line.get(*cursor).is_some_and(|s| s.end <= tick) {
                            *cursor += 1;
                        }
                        line.get(*cursor).filter(|s| s.start <= tick).copied()
                    })
                    .collect(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::Mode;

    fn note(s: &str, d: Duration) -> Note {
        Note::new(s.parse().unwrap(), d)
    }

    fn two_voices() -> Score {
        let cf = Voice::new(
            VoiceType::Alto,
            Role::CantusFirmus,
            vec![note("D4", Duration::WHOLE), note("E4", Duration::WHOLE)],
        );
        let cp = Voice::new(
            VoiceType::Soprano,
            Role::Counterpoint(Species::Fourth),
            vec![
                Note::rest(Duration::HALF),
                note("A4", Duration::HALF).tied(),
                note("A4", Duration::HALF),
                note("G4", Duration::HALF),
            ],
        );
        Score::new(ModeSpec::new(Mode::Dorian), vec![cp, cf])
    }

    #[test]
    fn validation() {
        let mut score = two_voices();
        assert_eq!(score.validate(), Ok(()));
        assert_eq!(score.measure_count(), 2);
        score.voices[0].notes[2].pitch = Some("B4".parse().unwrap());
        assert_eq!(
            score.validate(),
            Err(ScoreError::BadTie { voice: 0, note: 1 })
        );
        let mut score = two_voices();
        score.voices[1].notes.push(note("D4", Duration::WHOLE));
        assert_eq!(
            score.validate(),
            Err(ScoreError::LengthMismatch {
                voice: 1,
                ticks: 24,
                expected: 16
            })
        );
        let mut score = two_voices();
        score.voices[0].notes[0] = Note::rest(Duration::DOTTED_HALF);
        score.voices[0].notes[3].duration = Duration::QUARTER;
        assert_eq!(
            score.validate(),
            Err(ScoreError::CrossesBarline {
                voice: 0,
                note: 1,
                measure: 1
            })
        );
    }

    #[test]
    fn user_messages() {
        let mut score = two_voices();
        score.voices[1].notes.push(note("D4", Duration::HALF));
        let error = score.validate().unwrap_err();
        assert_eq!(
            error.message(Lang::De, score.meter),
            "Stimme 2 dauert 2 1/2 Takte, Stimme 1 aber 2 Takte."
        );
        assert_eq!(
            error.message(Lang::En, score.meter),
            "Voice 2 lasts 2 1/2 measures, but voice 1 lasts 2."
        );
    }

    #[test]
    fn soundings_merge_ties() {
        let score = two_voices();
        let s = score.voices[0].soundings();
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].start, s[0].end), (4, 12));
        assert_eq!(score.voices[0].pitches().len(), 2);
    }

    #[test]
    fn slices_and_lowest_voice() {
        let score = two_voices();
        let slices = score.slices();
        assert_eq!(
            slices.iter().map(|s| s.tick).collect::<Vec<_>>(),
            [0, 4, 8, 12]
        );
        assert_eq!(slices[0].sounding[0], None);
        assert!(slices[1].attacks(0));
        assert!(!slices[2].attacks(0));
        assert!(slices[2].attacks(1));
        assert_eq!(slices[2].lowest().unwrap().0, 1);
    }

    #[test]
    fn species_numbers() {
        assert_eq!(Species::Fourth.number(), 4);
        assert_eq!(Species::from_number(3), Some(Species::Third));
        assert_eq!(Species::from_number(0), None);
        assert_eq!(Species::from_number(6), None);
    }

    /// Kyrie I in the upper voice: Ký-ri-e with a melisma on e, the tail e-lé-i-son, a rest and
    /// the tail once more, its i held over the barline. The cantus below has a text of its own
    /// and does not sing the Kyrie.
    fn kyrie_setting() -> Score {
        use crate::parse::parse_lilypond;
        use crate::text::{TextOptions, resolve};

        let notes = |s: &str| parse_lilypond(s, Duration::WHOLE).unwrap();
        let mut upper = notes("d'2 e' f' g'4 f' e'2 f' g' a' r a' g' f'~ f' e'");
        // One position in the text or `_` per note.
        let lyrics = "0 1 2 _ _ 3 4 5 6 _ 3 4 5 _ 6";
        assert_eq!(upper.len(), lyrics.split(' ').count());
        for (note, lyric) in upper.iter_mut().zip(lyrics.split(' ')) {
            note.lyric = lyric.parse().ok().map(Lyric::new);
        }
        let mut cantus = notes("d1 f e d e f d");
        cantus[0].syllable = Some("A-".into());
        cantus[1].syllable = Some("men.".into());
        let mut score = Score::new(
            ModeSpec::new(Mode::Dorian),
            vec![
                Voice::new(
                    VoiceType::Soprano,
                    Role::Counterpoint(Species::Fifth),
                    upper,
                ),
                Voice::new(VoiceType::Bass, Role::CantusFirmus, cantus),
            ],
        );
        let (text, _) = resolve("kyrie", &TextOptions::default()).unwrap();
        score.text = Some(ScoreText {
            text: Arc::new(text),
            voices: vec![true, false],
        });
        score
    }

    /// The syllables of a voice, one or `_` per note.
    fn syllables(voice: &Voice) -> String {
        let all: Vec<&str> = voice
            .notes
            .iter()
            .map(|n| n.syllable.as_deref().unwrap_or("_"))
            .collect();
        all.join(" ")
    }

    #[test]
    fn lyrics_are_positions() {
        assert_eq!(Lyric::new(0).index(), Some(0));
        assert_eq!(Lyric::new(6).index(), Some(6));
        let last = Lyric::new(MAX_SYLLABLES - 1);
        assert_eq!(last.index(), Some(MAX_SYLLABLES - 1));
        assert_ne!(last, Lyric::UNKNOWN);
        assert_eq!(Lyric::UNKNOWN.index(), None);
        // A note pays two bytes for its lyric, which fit into its padding: the search clones
        // notes all the time, so a note must not grow.
        assert_eq!(std::mem::size_of::<Option<Lyric>>(), 2);
        assert!(std::mem::size_of::<Note>() <= 40);
    }

    #[test]
    #[should_panic(expected = "beyond the longest possible text")]
    fn lyrics_lie_within_the_longest_text() {
        Lyric::new(MAX_SYLLABLES);
    }

    #[test]
    fn syllables_from_lyrics() {
        let mut score = kyrie_setting();
        // A syllable left on a note without a lyric goes.
        score.voices[0].notes[3].syllable = Some("al-".into());
        score.fill_syllables(true);
        assert_eq!(
            syllables(&score.voices[0]),
            "Ký- ri- e _ _ e- lé- i- son. _ e- lé- i- _ son."
        );
        assert_eq!(
            score.voice_lyrics(0).as_deref(),
            Some("Ký-ri-e e-lé-i-son. e-lé-i-son.")
        );
        // The cantus does not sing the text and keeps its own.
        assert_eq!(syllables(&score.voices[1]), "A- men. _ _ _ _ _");
        assert_eq!(score.voice_lyrics(1).as_deref(), Some("A-men."));
        assert_eq!(score.voice_lyrics(2), None);

        // Without accents, and back.
        score.fill_syllables(false);
        assert_eq!(
            syllables(&score.voices[0]),
            "Ky- ri- e _ _ e- le- i- son. _ e- le- i- _ son."
        );
        assert_eq!(
            score.voice_lyrics(0).as_deref(),
            Some("Ky-ri-e e-le-i-son. e-le-i-son.")
        );
        let unaccented = score.clone();
        score.fill_syllables(true);
        score.fill_syllables(false);
        assert_eq!(score, unaccented);

        // An unknown lyric keeps the syllable read from a file.
        score.voices[0].notes[3].lyric = Some(Lyric::UNKNOWN);
        score.voices[0].notes[3].syllable = Some("al-".into());
        score.fill_syllables(true);
        assert_eq!(
            syllables(&score.voices[0]),
            "Ký- ri- e al- _ e- lé- i- son. _ e- lé- i- _ son."
        );
        assert_eq!(
            score.voice_lyrics(0).as_deref(),
            Some("Ký-ri-e al-e-lé-i-son. e-lé-i-son.")
        );
    }

    /// Imported scores may start a voice with a melisma and put lyrics on a tie continuation or
    /// a rest, where generated ones never do; the syllables are written all the same, so that
    /// TXT-ONSET can name them.
    #[test]
    fn syllables_where_imports_put_them() {
        use crate::parse::parse_lilypond;
        use crate::text::{TextOptions, resolve};

        let mut notes = parse_lilypond("r2 d'2 e' f'~ f' r g' a' g'1", Duration::WHOLE).unwrap();
        // One position in the text or `_` per note: nothing on the rest before the entry and on
        // the first attack, then Ký on e', ri on the tied f', e on its continuation, the tail's
        // e on the rest.
        let lyrics = "_ _ 0 1 2 3 4 5 6";
        assert_eq!(notes.len(), lyrics.split(' ').count());
        for (note, lyric) in notes.iter_mut().zip(lyrics.split(' ')) {
            note.lyric = lyric.parse().ok().map(Lyric::new);
        }
        assert!(notes[3].tie && notes[5].is_rest());
        let mut score = Score::new(
            ModeSpec::new(Mode::Dorian),
            vec![Voice::new(
                VoiceType::Soprano,
                Role::Counterpoint(Species::Fifth),
                notes,
            )],
        );
        let (text, _) = resolve("kyrie", &TextOptions::default()).unwrap();
        score.text = Some(ScoreText {
            text: Arc::new(text),
            voices: vec![true],
        });
        assert_eq!(score.validate(), Ok(()));

        score.fill_syllables(true);
        assert_eq!(syllables(&score.voices[0]), "_ _ Ký- ri- e e- lé- i- son.");
        assert_eq!(
            score.voice_lyrics(0).as_deref(),
            Some("Ký-ri-e e-lé-i-son.")
        );
        score.fill_syllables(false);
        assert_eq!(syllables(&score.voices[0]), "_ _ Ky- ri- e e- le- i- son.");
        assert_eq!(
            score.voice_lyrics(0).as_deref(),
            Some("Ky-ri-e e-le-i-son.")
        );
    }

    #[test]
    fn lyrics_of_a_voice() {
        let mut score = two_voices();
        assert_eq!(score.voice_lyrics(0), None);
        // Syllables as a chant gives them, with blanks and empty ones from a file.
        for (note, syllable) in score.voices[1].notes.iter_mut().zip(["Sanc-", "tus,"]) {
            note.syllable = Some(syllable.into());
        }
        assert_eq!(score.voice_lyrics(1).as_deref(), Some("Sanc-tus,"));
        score.voices[0].notes[1].syllable = Some(" Pax ".into());
        score.voices[0].notes[2].syllable = Some(String::new());
        score.voices[0].notes[3].syllable = Some("ti-".into());
        assert_eq!(score.voice_lyrics(0).as_deref(), Some("Pax ti-"));
    }

    #[test]
    fn validation_of_the_text() {
        let mut score = kyrie_setting();
        assert_eq!(score.validate(), Ok(()));
        // Unknown lyrics are valid, in any voice.
        score.voices[0].notes[3].lyric = Some(Lyric::UNKNOWN);
        score.voices[1].notes[6].lyric = Some(Lyric::UNKNOWN);
        assert_eq!(score.validate(), Ok(()));

        let mut outside = score.clone();
        outside.voices[1].notes[2].lyric = Some(Lyric::new(7));
        let error = outside.validate().unwrap_err();
        assert_eq!(
            error,
            ScoreError::LyricOutside {
                voice: 1,
                note: 2,
                position: 7,
                syllables: 7
            }
        );
        assert_eq!(
            error.message(Lang::De, score.meter),
            "Stimme 2, Note 3: Der Text hat keine 8. Silbe, nur 7 Silben."
        );
        assert_eq!(
            error.message(Lang::En, score.meter),
            "Voice 2, note 3: the text has no syllable 8, only 7."
        );

        for (voices, expected) in [(vec![true], 1), (vec![true, false, true], 3)] {
            let mut wrong = score.clone();
            wrong.text.as_mut().unwrap().voices = voices;
            assert_eq!(
                wrong.validate(),
                Err(ScoreError::TextVoices {
                    voices: expected,
                    expected: 2
                })
            );
        }
        let mut wrong = score.clone();
        wrong.text.as_mut().unwrap().voices.push(true);
        let error = wrong.validate().unwrap_err();
        assert_eq!(
            error.message(Lang::De, score.meter),
            "Der Text ist für 3 Stimmen angegeben, der Satz hat aber 2."
        );
        assert_eq!(
            error.message(Lang::En, score.meter),
            "The text is given for 3 voices, but the score has 2."
        );
    }

    #[test]
    fn untexted_scores_stay_unchanged() {
        let mut score = two_voices();
        assert_eq!(score.text, None);
        assert!(
            score
                .voices
                .iter()
                .flat_map(|v| &v.notes)
                .all(|n| n.lyric.is_none())
        );
        score.voices[1].notes[0].syllable = Some("Ky-".into());
        score.voices[1].notes[1].syllable = Some("ri".into());
        let before = score.clone();
        score.fill_syllables(true);
        assert_eq!(score, before);
        assert_eq!(score.validate(), Ok(()));
        assert_eq!(score.voice_lyrics(1).as_deref(), Some("Ky-ri"));
        // Without the text nothing reads the lyrics, so they are not checked.
        score.voices[1].notes[1].lyric = Some(Lyric::new(40));
        assert_eq!(score.validate(), Ok(()));
        let before = score.clone();
        score.fill_syllables(false);
        assert_eq!(score, before);
    }

    /// A head as `pitch.ticks`, such as `D4.8 E4.4`.
    fn head(notes: &str) -> Vec<(Pitch, u32)> {
        notes
            .split_whitespace()
            .map(|note| {
                let (pitch, ticks) = note.split_once('.').unwrap();
                (pitch.parse().unwrap(), ticks.parse().unwrap())
            })
            .collect()
    }

    fn entry(voice: usize, start: u32, rest: u32, notes: &str) -> ScoreEntry {
        ScoreEntry {
            voice,
            start,
            rest,
            notes: head(notes),
        }
    }

    /// Kyrie I in three voices: the first point with every voice and its goal in measure 11
    /// (from 0), then the tail e-lé-i-son in two voices, which rest before they enter again.
    fn kyrie_imitation() -> ScoreImitation {
        ScoreImitation {
            points: vec![
                ScorePoint {
                    words: 0..7,
                    entries: vec![
                        entry(1, 0, 0, "D4.8 E4.4 F4.4 G4.8"),
                        entry(0, 8, 0, "A4.8 B4.4 C5.4 D5.8"),
                        entry(2, 20, 0, "D3.4 E3.4 F3.4 G3.8"),
                    ],
                    goal: Some(11),
                },
                ScorePoint {
                    words: 3..7,
                    entries: vec![
                        entry(0, 92, 4, "A4.8 C#5.4 Bb4.4 A4.8"),
                        entry(2, 100, 8, "D3.8 F3.4 E3.4 D3.8"),
                    ],
                    goal: None,
                },
            ],
        }
    }

    /// Three voices of sixteen measures that sing Kyrie I, with the plan of
    /// [`kyrie_imitation`] and its cadence; the notes do not matter.
    fn imitated_score() -> Score {
        use crate::text::{TextOptions, resolve};

        let voices = [VoiceType::Alto, VoiceType::Tenor, VoiceType::Bass]
            .into_iter()
            .map(|voice_type| {
                let rests = vec![Note::rest(Duration::WHOLE); 16];
                Voice::new(voice_type, Role::Counterpoint(Species::Free), rests)
            })
            .collect();
        let mut score = Score::new(ModeSpec::new(Mode::Dorian), voices);
        assert_eq!(score.imitation, None);
        score.cadences = vec![Cadence {
            measure: 11,
            target: Step::A,
        }];
        let (text, _) = resolve("kyrie", &TextOptions::default()).unwrap();
        score.text = Some(ScoreText {
            text: Arc::new(text),
            voices: vec![true; 3],
        });
        score.imitation = Some(Arc::new(kyrie_imitation()));
        score
    }

    /// The marks of the sections and parts of a mass movement lie in its measures, each after the
    /// one before; every other score has none.
    #[test]
    fn section_marks_lie_in_order_in_the_score() {
        let mark = |measure: usize, kind: SectionKind, title: Option<&str>| SectionMark {
            measure,
            kind,
            title: title.map(String::from),
        };
        let mut score = imitated_score();
        assert_eq!((score.sections.len(), score.measure_count()), (0, 16));
        score.sections = vec![
            mark(0, SectionKind::Section, Some("Kyrie I")),
            mark(5, SectionKind::Seam, None),
            mark(9, SectionKind::Break, Some("Et incarnatus est")),
            mark(15, SectionKind::Section, Some("Christe")),
        ];
        assert_eq!(score.validate(), Ok(()));
        let meter = score.meter;
        let fails = |measures: &[usize], error: ScoreError, de: &str, en: &str| {
            let mut wrong = score.clone();
            wrong.sections = measures
                .iter()
                .map(|&measure| mark(measure, SectionKind::Seam, None))
                .collect();
            assert_eq!(wrong.validate(), Err(error.clone()), "{measures:?}");
            assert_eq!(error.message(Lang::De, meter), de);
            assert_eq!(error.message(Lang::En, meter), en);
        };
        fails(
            &[0, 16],
            ScoreError::SectionOutside {
                mark: 1,
                measure: 16,
                measures: 16,
            },
            "Die 2. Abschnittsmarke steht in Takt 17, der Satz hat aber nur 16 Takte.",
            "Section mark 2 stands in measure 17, but the score has only 16 measures.",
        );
        let order = ScoreError::SectionOrder {
            mark: 1,
            measure: 4,
        };
        let (de, en) = (
            "Die 2. Abschnittsmarke in Takt 5 steht nicht nach der vorigen.",
            "Section mark 2 in measure 5 does not come after the one before it.",
        );
        fails(&[4, 4], order.clone(), de, en);
        fails(&[6, 4, 8], order.clone(), de, en);
        assert_eq!(
            order.to_string(),
            "section mark 2 in measure 5 does not come after the one before it"
        );
    }

    /// The text form of a plan reads back as the plan, also with blanks between its parts, and
    /// so do a plan without points and a point without entries.
    #[test]
    fn score_imitation_round_trip() {
        let plan = kyrie_imitation();
        let spec = plan.spec();
        assert_eq!(
            spec,
            "p0-7g11|1@0/0:D4.8,E4.4,F4.4,G4.8;0@8/0:A4.8,B4.4,C5.4,D5.8;2@20/0:D3.4,E3.4,F3.4,\
             G3.8|p3-7|0@92/4:A4.8,C#5.4,Bb4.4,A4.8;2@100/8:D3.8,F3.4,E3.4,D3.8"
        );
        assert_eq!(ScoreImitation::parse(&spec), Ok(plan.clone()));
        let spaced = spec
            .replace('|', " | ")
            .replace(';', " ;\n")
            .replace(',', ", ");
        assert_eq!(ScoreImitation::parse(&format!(" {spaced}\n")), Ok(plan));
        let empty = ScoreImitation { points: Vec::new() };
        assert_eq!(empty.spec(), "");
        assert_eq!(ScoreImitation::parse(" "), Ok(empty));
        let bare = ScoreImitation {
            points: vec![ScorePoint {
                words: 0..7,
                entries: Vec::new(),
                goal: None,
            }],
        };
        assert_eq!(bare.spec(), "p0-7|");
        assert_eq!(ScoreImitation::parse(&bare.spec()), Ok(bare));
    }

    /// A text form that cannot be read names the part that fails, in both languages.
    #[test]
    fn unreadable_imitations_name_their_part() {
        for (spec, part) in [
            ("p0-7g11", "p0-7g11"),
            ("p0-7g11|1@0/0:D4.8|p3-7", "p3-7"),
            ("q0-7|1@0/0:D4.8", "q0-7"),
            ("p0-7gx|1@0/0:D4.8", "p0-7gx"),
            ("p0|1@0/0:D4.8", "p0"),
            ("p0-7|1@0:D4.8", "1@0:D4.8"),
            ("p0-7|x@0/0:D4.8", "x@0/0:D4.8"),
            ("p0-7|1@0/0:D4.8,X4.4", "X4.4"),
            ("p0-7|1@0/0:D4.8,E4", "E4"),
            ("p0-7|1@0/0:D4.8,E4.-4", "E4.-4"),
            ("p0-7|1@0/0:", "1@0/0:"),
            ("p0-7|1@0/0:D4.8;", "p0-7|1@0/0:D4.8;"),
            ("|", "|"),
        ] {
            assert_eq!(
                ScoreImitation::parse(spec),
                Err(ScoreError::ImitationSyntax { part: part.into() }),
                "{spec}"
            );
        }
        let error = ScoreImitation::parse("p0-7|1@0/0:D4.8,E4").unwrap_err();
        assert_eq!(
            error.message(Lang::De, Meter::default()),
            "Der Imitationsplan ist an der Stelle „E4“ nicht lesbar."
        );
        assert_eq!(
            error.message(Lang::En, Meter::default()),
            "The imitation plan cannot be read at \"E4\"."
        );
    }

    /// A plan fits its score: syllables of the text, entries in time order in voices of the
    /// score, heads within the score and after their rests, goals in order. Validating the
    /// score validates its plan.
    #[test]
    fn imitations_fit_their_score() {
        let score = imitated_score();
        assert_eq!(score.validate(), Ok(()));
        let meter = score.meter;
        // The plan changed by `change`: the error, and its messages.
        let fails =
            |change: &dyn Fn(&mut ScoreImitation), error: ScoreError, de: &str, en: &str| {
                let mut changed = score.clone();
                let mut plan = kyrie_imitation();
                change(&mut plan);
                assert_eq!(plan.validate(&changed), Err(error.clone()), "{en}");
                changed.imitation = Some(Arc::new(plan));
                assert_eq!(changed.validate(), Err(error.clone()), "{en}");
                assert_eq!(error.message(Lang::De, meter), de);
                assert_eq!(error.message(Lang::En, meter), en);
            };
        let words = ScoreError::ImitationWords { point: 1 };
        let (de, en) = (
            "Die Worte von Punkt 2 des Imitationsplans sind keine Silben des Textes.",
            "The words of point 2 of the imitation plan are not syllables of the text.",
        );
        fails(&|plan| plan.points[1].words = 3..8, words.clone(), de, en);
        fails(&|plan| plan.points[1].words = 4..4, words, de, en);
        // Without a text the words are not checked, as long as there are some.
        let mut untexted = score.clone();
        untexted.text = None;
        let mut plan = kyrie_imitation();
        plan.points[1].words = 3..80;
        assert_eq!(plan.validate(&untexted), Ok(()));

        let order = |point| ScoreError::ImitationOrder { point };
        fails(
            &|plan| plan.points[0].entries.swap(0, 1),
            order(0),
            "Die Einsätze von Punkt 1 des Imitationsplans fehlen oder stehen nicht in zeitlicher \
             Reihenfolge.",
            "The entries of point 1 of the imitation plan are missing or out of time order.",
        );
        fails(
            &|plan| plan.points[1].entries.clear(),
            order(1),
            "Die Einsätze von Punkt 2 des Imitationsplans fehlen oder stehen nicht in zeitlicher \
             Reihenfolge.",
            "The entries of point 2 of the imitation plan are missing or out of time order.",
        );
        // The tail first: the leader of the point after it enters before its own.
        fails(
            &|plan| plan.points.reverse(),
            order(1),
            "Die Einsätze von Punkt 2 des Imitationsplans fehlen oder stehen nicht in zeitlicher \
             Reihenfolge.",
            "The entries of point 2 of the imitation plan are missing or out of time order.",
        );
        fails(
            &|plan| plan.points[0].entries[2].voice = 3,
            ScoreError::ImitationVoice {
                point: 0,
                entry: 2,
                voice: 3,
            },
            "Einsatz 3 von Punkt 1 des Imitationsplans: Der Satz hat keine Stimme 4.",
            "Entry 3 of point 1 of the imitation plan: the score has no voice 4.",
        );
        let entry = ScoreError::ImitationEntry { point: 1, entry: 1 };
        let (de, en) = (
            "Einsatz 2 von Punkt 2 des Imitationsplans passt nicht in den Satz.",
            "Entry 2 of point 2 of the imitation plan does not fit into the score.",
        );
        fails(
            &|plan| plan.points[1].entries[1].notes.clear(),
            entry.clone(),
            de,
            en,
        );
        fails(
            &|plan| plan.points[1].entries[1].notes[2].1 = 0,
            entry.clone(),
            de,
            en,
        );
        fails(
            &|plan| plan.points[1].entries[1].rest = 101,
            entry.clone(),
            de,
            en,
        );
        // The last head ends with the score, and not after it.
        let mut plan = kyrie_imitation();
        plan.points[1].entries[1].start = 104;
        assert_eq!(plan.validate(&score), Ok(()));
        fails(&|plan| plan.points[1].entries[1].start = 105, entry, de, en);
        fails(
            &|plan| plan.points[0].goal = Some(16),
            ScoreError::ImitationGoal {
                point: 0,
                measure: 16,
            },
            "Punkt 1 des Imitationsplans: Sein Binnenschluss in Takt 17 liegt außerhalb des \
             Satzes oder nicht nach dem vorigen.",
            "Point 1 of the imitation plan: its cadence in measure 17 lies outside the score or \
             not after the one before.",
        );
        fails(
            &|plan| plan.points[1].goal = Some(11),
            ScoreError::ImitationGoal {
                point: 1,
                measure: 11,
            },
            "Punkt 2 des Imitationsplans: Sein Binnenschluss in Takt 12 liegt außerhalb des \
             Satzes oder nicht nach dem vorigen.",
            "Point 2 of the imitation plan: its cadence in measure 12 lies outside the score or \
             not after the one before.",
        );
    }
}
