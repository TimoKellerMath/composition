//! Imitation (phase 13): points of imitation, planned before the search. A point sets some words
//! of the text, and its voices take them up one after the other, each with the head of the same
//! subject (soggetto), at the unison, the octave, the fourth or the fifth of the first entry.
//!
//! The planner fixes for each point its words, its subject, its entries (voice, tick, pitches,
//! values, the rest before and the syllables) and the internal cadence that ends it. The entries
//! become fixed notes of the search ([`ImitationPlan::fixed`]), which sets the free counterpoint
//! around them, and the plan travels in the score ([`ImitationPlan::score_imitation`]), where
//! the IMI rules judge the result against it.
//!
//! Ticks count eighths, eight to a 2/2 measure; voices are indexed in score order, from the top;
//! syllables by their positions in the text. The module has no I/O and builds for WebAssembly.

pub mod chant;
pub mod check;
pub mod detect;
pub mod planner;
pub mod points;
pub mod subject;
#[cfg(test)]
pub(crate) mod test_support;

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Range, RangeInclusive};
use std::str::FromStr;
use std::sync::Arc;

use crate::gabc::Chant;
use crate::generate::{Fixed, FixedRole, cadence_formula};
use crate::lang::Lang;
use crate::mode::ModeSpec;
use crate::pitch::{Pitch, Step};
use crate::score::{Cadence, Note, ScoreEntry, ScoreImitation, ScorePoint};
use crate::text::underlay::Choice;
use crate::time::{Duration, Meter};

/// What an imitative free setting asks for ([`FreeRequest::imitation`]). Whatever it leaves
/// open (`None`, empty or [`Toggle::Auto`]) the planner chooses from the text, the mode and
/// Palestrina's practice.
///
/// [`FreeRequest::imitation`]: crate::generate::FreeRequest::imitation
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Imitation {
    /// Where the subjects come from.
    pub subject: SubjectSource,
    /// The number of points; `None` plans one for each part of the text.
    pub points: Option<usize>,
    /// The number of voices in each point after the first, which has them all; `None` draws it
    /// by the sizes of Palestrina's later points in a choir of this size, mostly two.
    pub later_voices: Option<usize>,
    /// Whether a short text sung twice restates its first point, in closer stretto, instead of
    /// setting its last tail as a point with a subject of its own. `Auto` restates now and then.
    pub restate: Toggle,
    /// Per point, the voices in the order of their entries, the leader first; `None` or a
    /// missing point leaves the order to the planner.
    pub orders: Vec<Option<Vec<usize>>>,
    /// Per point, the relation of each entry after the first to the leader's.
    pub relations: Vec<Option<Vec<Relation>>>,
    /// Per point, the ticks from each entry to the next.
    pub delays: Vec<Option<Vec<u32>>>,
    /// Whether, from four voices on, the third and fourth entries repeat the first two an octave
    /// lower, as a pair. `Auto` does so now and then.
    pub pairs: Toggle,
    /// How the entries answer the leader.
    pub answer: AnswerChoice,
    /// The notes of every head, four or more; `None` keeps those of the subject.
    pub head: Option<usize>,
}

/// An option of an imitation that the request cannot have ([`Imitation`]): the request fails with
/// [`GenerateError::ImitationOption`](crate::generate::GenerateError::ImitationOption) rather than
/// the planner leaving it aside. Points and voices count from 1, voices from the top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OptionError {
    /// An option for a point that the text does not have.
    NoPoint { point: usize, points: usize },
    /// A voice that the choir does not have, in the order of a point.
    NoVoice {
        point: usize,
        voice: usize,
        voices: usize,
    },
    /// A voice named twice in the order of a point.
    VoiceTwice { point: usize, voice: usize },
    /// An order of a point after the first with fewer than two voices.
    FewVoices { point: usize },
    /// More delays or relations for a point than it has entries after the first.
    TooMany {
        point: usize,
        given: usize,
        most: usize,
    },
    /// A delay of a point off the minim beats, or none, which would let two voices enter
    /// together.
    Delay { point: usize, ticks: u32 },
    /// A number of voices of the points after the first that the choir cannot give: two at
    /// least, all of its voices at most.
    LaterVoices { voices: usize, most: usize },
    /// A head of fewer than four notes, or of more than a subject has (`most`: ten built from
    /// text or chant, twelve given).
    Head { notes: usize, most: usize },
    /// More subjects given (heads or a chant's phrases) than points take one of their own: a
    /// restatement takes its point's subject again.
    Subjects { given: usize, points: usize },
    /// A restatement of the first point in a text that is sung once.
    Restate,
    /// A relation of a point's entries to the leader that none of its subjects can be answered
    /// in.
    Relation { point: usize },
}

impl OptionError {
    /// Message for users in their language: a sentence without its full stop.
    pub fn message(&self, lang: Lang) -> String {
        match (self, lang) {
            (OptionError::NoPoint { point, points }, Lang::De) => {
                format!("Punkt {point} gibt es nicht, der Text hat {points} Punkte")
            }
            (OptionError::NoPoint { point, points }, Lang::En) => {
                format!("there is no point {point}, the text has {points} points")
            }
            (
                OptionError::NoVoice {
                    point,
                    voice,
                    voices,
                },
                Lang::De,
            ) => format!(
                "Die Reihenfolge von Punkt {point} nennt Stimme {voice}, der Satz hat {voices} Stimmen"
            ),
            (
                OptionError::NoVoice {
                    point,
                    voice,
                    voices,
                },
                Lang::En,
            ) => format!(
                "the order of point {point} names voice {voice}, but the setting has {voices} voices"
            ),
            (OptionError::VoiceTwice { point, voice }, Lang::De) => {
                format!("Die Reihenfolge von Punkt {point} nennt Stimme {voice} zweimal")
            }
            (OptionError::VoiceTwice { point, voice }, Lang::En) => {
                format!("the order of point {point} names voice {voice} twice")
            }
            (OptionError::FewVoices { point }, Lang::De) => {
                format!("Punkt {point} braucht mindestens zwei Stimmen")
            }
            (OptionError::FewVoices { point }, Lang::En) => {
                format!("point {point} needs two voices or more")
            }
            (OptionError::TooMany { point, given, most }, Lang::De) => format!(
                "Punkt {point} hat nach dem ersten Einsatz {most} Einsätze, aber {given} Angaben dafür"
            ),
            (OptionError::TooMany { point, given, most }, Lang::En) => format!(
                "point {point} has {most} entries after the first, but {given} values for them"
            ),
            (OptionError::Delay { point, ticks: 0 }, Lang::De) => format!(
                "Punkt {point}: Zwei Einsätze zugleich gibt es nicht; jeder folgt dem vorigen um \
                 eine Halbe oder mehr"
            ),
            (OptionError::Delay { point, ticks: 0 }, Lang::En) => format!(
                "point {point}: no two entries come together; each follows the one before by a \
                 minim or more"
            ),
            (OptionError::Delay { point, ticks }, Lang::De) => {
                let measures = (f64::from(*ticks) / 8.0).to_string().replace('.', ",");
                format!(
                    "Punkt {point}: Ein Abstand von {measures} Takten ist kein Vielfaches einer Halben"
                )
            }
            (OptionError::Delay { point, ticks }, Lang::En) => {
                let measures = f64::from(*ticks) / 8.0;
                format!("point {point}: a delay of {measures} measures is no multiple of a minim")
            }
            (OptionError::LaterVoices { voices, most }, Lang::De) => {
                format!("Spätere Punkte können nicht {voices} Stimmen haben, nur zwei bis {most}")
            }
            (OptionError::LaterVoices { voices, most }, Lang::En) => {
                format!("later points cannot have {voices} voices, only two to {most}")
            }
            (OptionError::Head { notes, most }, Lang::De) => {
                format!("Ein Kopf hat vier bis {most} Töne, nicht {notes}")
            }
            (OptionError::Head { notes, most }, Lang::En) => {
                format!("a head has four to {most} notes, not {notes}")
            }
            (OptionError::Subjects { given, points }, Lang::De) => format!(
                "Es sind {given} Soggetti gegeben (Köpfe oder Phrasen), aber nur {points} Punkte \
                 bringen ein eigenes"
            ),
            (OptionError::Subjects { given, points }, Lang::En) => format!(
                "{given} subjects are given (heads or phrases), but only {points} points take one \
                 of their own"
            ),
            (OptionError::Restate, Lang::De) => {
                "Der Text wird nur einmal gesungen; es gibt keinen Punkt zu wiederholen".to_string()
            }
            (OptionError::Restate, Lang::En) => {
                "the text is sung once only; there is no point to restate".to_string()
            }
            (OptionError::Relation { point }, Lang::De) => format!(
                "Punkt {point}: Kein Soggetto lässt sich in den verlangten Intervallen beantworten"
            ),
            (OptionError::Relation { point }, Lang::En) => {
                format!("point {point}: no subject can be answered at the intervals asked for")
            }
        }
    }
}

/// Where the subjects of the points come from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SubjectSource {
    /// Built from the words of each point: their declamation first, then the pitches.
    #[default]
    Text,
    /// The notes of a chant's phrases (a paraphrase), with a rhythm from the words.
    Chant {
        chant: Arc<Chant>,
        /// The phrases, counted from 0, for the points in order; `None` finds them by the words.
        phrases: Option<Vec<usize>>,
    },
    /// Heads given as notes, for the first points in order; the points after them build their
    /// subjects from the text.
    Given(Vec<Vec<Note>>),
    /// The head motif of a mass (phase 14, MASS-MOTIF) for the first point: its pitches kept,
    /// declaimed anew on the point's words through the path of the chant, with at most
    /// [`MOTIF_REPEATS`] of its notes struck again for syllables. Where the motif gives no
    /// subject, the point takes its subjects from its words, with a warning; the points after the
    /// first take theirs from their words.
    Motif(Arc<Motif>),
}

/// The notes a head may strike again to declaim its words on a head motif: two at most.
pub const MOTIF_REPEATS: usize = 2;

/// The head motif of a mass (phase 14, MASS-MOTIF, docs/design/phase-14/spec.md § 3.9): the
/// pitches of the subject of Kyrie I's first point, which the first points of Gloria, Credo,
/// Sanctus and Agnus Dei I take up, declaimed anew. Each tone is its degree in diatonic steps
/// above the final of the mode, the first within the octave above it, and its alteration against
/// the pitch of the scale on its step (−1 for the flattened mi that the head sang); a repeated
/// pitch counts once. A motif has five to ten tones ([`Motif::TONES`]).
///
/// As text it is its tones separated by commas, an alteration after the degree as `b` (flat) or
/// `#` (sharp): `0,2,3,4,5b,4` is D F G A B♭ A in the Dorian mode. So the files of a mass keep it
/// (`motif=` of `fux-part`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Motif {
    /// The tones: the degree above the final and the alteration against the scale.
    pub tones: Vec<(i8, i8)>,
}

impl Motif {
    /// The number of tones of a head motif: five to ten (MASS-MOTIF, spec § 3.9).
    pub const TONES: RangeInclusive<usize> = 5..=10;

    /// The motif of a head sung as `pitches` in `mode`: a pitch repeated at once counts once, the
    /// first tone lies within the octave above the final, and each alteration is counted against
    /// the scale of the mode. `None` where the head has not five to ten tones.
    pub fn of_pitches(mode: ModeSpec, pitches: &[Pitch]) -> Option<Motif> {
        let mut merged: Vec<Pitch> = Vec::with_capacity(pitches.len());
        for &pitch in pitches {
            if merged.last() != Some(&pitch) {
                merged.push(pitch);
            }
        }
        if !Motif::TONES.contains(&merged.len()) {
            return None;
        }
        let final_index = mode.final_step().index();
        let octave = (merged[0].diatonic() - final_index).div_euclid(7) * 7 + final_index;
        let tones = merged
            .iter()
            .map(|pitch| {
                let degree = (pitch.diatonic() - octave).clamp(-60, 60) as i8;
                (degree, pitch.alter - mode.scale_alter(pitch.step))
            })
            .collect();
        Some(Motif { tones })
    }

    /// The pitches of the motif in `mode`, its degrees counted from the final in octave
    /// `octave`.
    pub fn pitches(&self, mode: ModeSpec, octave: i8) -> Vec<Pitch> {
        let base = mode.final_pitch(octave).diatonic();
        self.tones
            .iter()
            .map(|&(degree, alter)| {
                let pitch = mode.scale_pitch(base + i32::from(degree));
                pitch.with_alter(pitch.alter + alter)
            })
            .collect()
    }
}

impl fmt::Display for Motif {
    /// Its tones separated by commas, each with `b` or `#` after an alteration: `0,2,3,4,5b,4`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tones: Vec<String> = self
            .tones
            .iter()
            .map(|&(degree, alter)| {
                let sign = match alter.cmp(&0) {
                    Ordering::Less => "b".repeat(usize::from(alter.unsigned_abs())),
                    Ordering::Equal => String::new(),
                    Ordering::Greater => "#".repeat(usize::from(alter.unsigned_abs())),
                };
                format!("{degree}{sign}")
            })
            .collect();
        f.write_str(&tones.join(","))
    }
}

/// A head motif written otherwise than [`Motif`]'s text form, or with fewer than five or more
/// than ten tones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MotifError {
    /// The text that could not be read.
    pub text: String,
}

impl fmt::Display for MotifError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\"{}\" is no head motif: five to ten degrees above the final, separated by commas, \
             each with b or # after an alteration",
            self.text
        )
    }
}

impl std::error::Error for MotifError {}

impl FromStr for Motif {
    type Err = MotifError;

    /// Reads the text form of [`Motif`]'s `Display`: `0,2,3,4,5b,4`.
    fn from_str(text: &str) -> Result<Motif, MotifError> {
        let error = || MotifError { text: text.into() };
        let tones = text
            .split(',')
            .map(|tone| {
                let tone = tone.trim();
                let digits = tone.trim_end_matches(['b', '#']);
                let signs = &tone[digits.len()..];
                let degree: i8 = digits.parse().ok()?;
                let flats = signs.matches('b').count();
                let sharps = signs.matches('#').count();
                if flats > 0 && sharps > 0 || flats + sharps > 2 {
                    return None;
                }
                Some((degree, sharps as i8 - flats as i8))
            })
            .collect::<Option<Vec<(i8, i8)>>>()
            .ok_or_else(error)?;
        if !Motif::TONES.contains(&tones.len()) {
            return Err(error());
        }
        Ok(Motif { tones })
    }
}

/// A choice that the planner makes, or that the request makes for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Toggle {
    /// The planner chooses.
    #[default]
    Auto,
    On,
    Off,
}

/// How the entries answer the leader's head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnswerChoice {
    /// Real answers; now and then a modal one where the head leaps between final and fifth, and
    /// always one where the real answer cannot be sung.
    #[default]
    Auto,
    /// Real answers only.
    Real,
    /// A modal answer wherever the head leaps between final and fifth.
    Modal,
}

/// The degree an entry begins on, against the leader's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Relation {
    /// The same degree, at the unison or an octave.
    Same,
    /// The other main degree of the mode: the fifth against the final and back, the final
    /// against the fourth; in the Phrygian mode A against E and back.
    Dominant,
    /// Another distance, in diatonic steps (only where the request asks for it).
    Steps(i8),
}

/// How an entry's head follows the leader's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Answer {
    /// With the leader's intervals, as diatonic steps on the scale of the mode.
    Real,
    /// A modal answer: where interval `at` (0 or 1, from the first note) joins final and fifth,
    /// the fifth and the fourth are exchanged, and the notes after it lie a step away.
    Modal { at: u8 },
}

/// How an entry sings its first note against the leader's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FirstNote {
    /// As long as the leader's.
    AsLeader,
    /// On the minim beat, a minim shorter (8 ticks to 4, 12 to 8, 16 to 12).
    Shortened,
    /// On the minim beat, as long as the leader's and tied over the barline.
    Syncopated,
}

/// What a point sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointKind {
    /// A part of the text, with a subject of its own.
    New,
    /// The last tail of the text once more (such as e-lé-i-son), with a subject of its own.
    Tail,
    /// The words and the subject of point `of` (from 0) once more, in closer stretto.
    Restatement { of: usize },
}

/// A note of a subject's head.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubjectNote {
    /// The diatonic steps from the first note.
    pub step: i8,
    /// The value in ticks.
    pub ticks: u8,
    /// What the note sings: `Start` where it begins a syllable, `Hold` in a melisma.
    pub sung: Choice,
    /// The alteration against the pitch of the scale on the note's step, as given: −1 a flat,
    /// +1 a sharp. Only a given head and a head motif have any; the other heads take their
    /// accidentals from the scale and its flattened mi ([`Soggetto::pitches`]).
    pub alter: i8,
}

/// Where a subject comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    Text,
    /// A phrase of the chant, counted from 0.
    Chant {
        phrase: usize,
    },
    Given,
    /// The head motif of a mass (phase 14): it keeps its pitches and their accidentals as a given
    /// head does ([`Soggetto::pitches`]).
    Motif,
}

/// A subject: the head that the voices of a point imitate, with the answers and the strettos
/// it allows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Soggetto {
    /// The degree of the first note, in diatonic steps above the final: 0 for the final, 4 for
    /// the fifth, 3 for the fourth.
    pub degree: i8,
    /// The head: 4 to 10 notes, a given one up to 12.
    pub notes: Vec<SubjectNote>,
    /// The syllables that the head declaims.
    pub words: Range<u16>,
    pub origin: Origin,
    /// The relations in which the head can be answered without a melodic prohibition, each with
    /// its answer.
    pub answers: Vec<(Relation, Answer)>,
    /// The strettos that pass the checks of two fixed lines: the follower's shift in diatonic
    /// steps, its delay in ticks and its first note.
    pub stretto: Vec<(i8, u32, FirstNote)>,
    /// What the subject costs.
    pub cost: u32,
}

/// An entry: a voice that sings the head of its point's subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The voice, by its index in the score.
    pub voice: usize,
    /// The tick of the head's first attack, on a minim beat.
    pub start: u32,
    /// The least silence before the entry, in ticks; 0 for the voice's first note, before which
    /// it is silent from the start.
    pub rest: u32,
    /// The head as this voice sings it: each pitch with its value in ticks, tied notes merged,
    /// and what it sings.
    pub notes: Vec<(Pitch, u32, Choice)>,
    /// The degree it begins on, against the leader's.
    pub relation: Relation,
    pub answer: Answer,
    pub first: FirstNote,
}

/// A point of imitation as planned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Point {
    /// The point's syllables, by their positions in the text.
    pub words: Range<u16>,
    pub kind: PointKind,
    pub subject: Arc<Soggetto>,
    /// The entries in time order; the first is the leader's.
    pub entries: Vec<Entry>,
    /// The measure (from 0) of the internal cadence that ends the point; `None` for the last
    /// point, which the final cadence ends.
    pub goal: Option<usize>,
}

/// The plan of an imitative setting: its points, its length and its cadences.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImitationPlan {
    pub points: Vec<Point>,
    /// The number of measures.
    pub measures: usize,
    /// The internal cadences, at the goals of the points.
    pub cadences: Vec<Cadence>,
    /// The planned pace: syllable starts per 100 minim beats, restatements included
    /// ([`SungText::rate`](crate::generate::SungText::rate)).
    pub rate: u16,
    /// What the plan costs.
    pub cost: u32,
    /// The target of the final cadence: the final of the mode, or for a part of a mass section
    /// (phase 14) the part's ending, the fifth or the fourth.
    pub ending: Step,
}

impl ImitationPlan {
    /// The fixed notes of the search for `voices` voices: the head of every entry, exact and
    /// split at the barlines, its first note an [`FixedRole::Entry`] and the others
    /// [`FixedRole::Subject`], with the syllables pinned at its attacks; and the formulas of
    /// every cadence, the final one on the plan's ending included, as [`cadence_formula`] writes
    /// them.
    pub fn fixed(&self, mode: ModeSpec, voices: usize) -> Vec<Fixed> {
        let meter = Meter::ALLA_BREVE;
        let per = meter.ticks_per_measure();
        let mut out = Vec::new();
        for entry in self.points.iter().flat_map(|point| &point.entries) {
            let mut tick = entry.start;
            for (i, &(pitch, ticks, sung)) in entry.notes.iter().enumerate() {
                let end = tick + ticks;
                let attack = tick;
                while tick < end {
                    let until = end.min((tick / per + 1) * per);
                    out.push(Fixed {
                        voice: entry.voice,
                        start: tick,
                        note: Note::new(pitch, Duration::from_ticks(until - tick))
                            .with_ficta(!mode.is_scale_pitch(pitch)),
                        tied_in: tick > attack,
                        any_octave: false,
                        role: if i == 0 && tick == attack {
                            FixedRole::Entry { rest: entry.rest }
                        } else {
                            FixedRole::Subject
                        },
                        sung: (tick == attack).then_some(sung),
                    });
                    tick = until;
                }
            }
        }
        let last = self.measures.saturating_sub(1);
        out.extend(cadence_formula(mode, self.ending, last, voices, meter));
        for cadence in &self.cadences {
            out.extend(cadence_formula(
                mode,
                cadence.target,
                cadence.measure,
                voices,
                meter,
            ));
        }
        out
    }

    /// The plan as the score keeps it: the points' words, entries and goals, each head with its
    /// pitches and values.
    pub fn score_imitation(&self) -> ScoreImitation {
        ScoreImitation {
            points: self
                .points
                .iter()
                .map(|point| ScorePoint {
                    words: point.words.clone(),
                    entries: point
                        .entries
                        .iter()
                        .map(|entry| ScoreEntry {
                            voice: entry.voice,
                            start: entry.start,
                            rest: entry.rest,
                            notes: entry.notes.iter().map(|&(p, t, _)| (p, t)).collect(),
                        })
                        .collect(),
                    goal: point.goal,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{FreeRequest, cadences_at};
    use crate::pitch::Step;
    use crate::score::ScoreError;

    fn dorian() -> ModeSpec {
        "dorian".parse().unwrap()
    }

    /// An entry of `voice` at `start` after `rest`, in the same degree as the leader's, its head
    /// as `pitch.ticks` with the syllable it begins after `=`, such as `A4.8=0 B4.4`.
    fn entry(voice: usize, start: u32, rest: u32, head: &str) -> Entry {
        let notes = head
            .split_whitespace()
            .map(|note| {
                let (note, sung) = match note.split_once('=') {
                    Some((note, y)) => (note, Choice::Start(y.parse().unwrap())),
                    None => (note, Choice::Hold),
                };
                let (pitch, ticks) = note.split_once('.').unwrap();
                (pitch.parse().unwrap(), ticks.parse().unwrap(), sung)
            })
            .collect();
        Entry {
            voice,
            start,
            rest,
            notes,
            relation: Relation::Same,
            answer: Answer::Real,
            first: FirstNote::AsLeader,
        }
    }

    /// The subject of a point as its leader sings it, declaiming `words`.
    fn subject(leader: &Entry, words: Range<u16>) -> Arc<Soggetto> {
        let first = leader.notes[0].0;
        let notes = leader
            .notes
            .iter()
            .map(|&(pitch, ticks, sung)| SubjectNote {
                step: (pitch.diatonic() - first.diatonic()) as i8,
                ticks: ticks as u8,
                sung,
                alter: 0,
            })
            .collect();
        Arc::new(Soggetto {
            degree: (first.diatonic() - Step::D.index()).rem_euclid(7) as i8,
            notes,
            words,
            origin: Origin::Text,
            answers: vec![
                (Relation::Same, Answer::Real),
                (Relation::Dominant, Answer::Real),
            ],
            stretto: vec![(-4, 12, FirstNote::Syncopated)],
            cost: 0,
        })
    }

    /// Kyrie I in three voices (alto, tenor, bass), Dorian, sixteen measures. The first point has
    /// every voice and ends in a cadence on A in measure 8 (from 0); the tenor enters on the
    /// minim beat, its first note tied over the barline. The tail e-lé-i-son follows in two
    /// voices: the tenor on the minim beat after the goal, the alto, which sang the cantizans,
    /// a measure later.
    fn kyrie_plan() -> ImitationPlan {
        let first = vec![
            entry(0, 0, 0, "A4.8=0 A4.4=1 B4.4=2 C5.12=3 B4.4"),
            Entry {
                relation: Relation::Dominant,
                first: FirstNote::Syncopated,
                ..entry(1, 12, 0, "D4.8=0 D4.4=1 E4.4=2 F4.12=3")
            },
            entry(2, 24, 0, "A2.8=0 A2.4=1 B2.4=2 C3.8=3"),
        ];
        let tail = vec![
            entry(1, 68, 4, "A3.8=3 C4.4=4 Bb3.4 A3.8=5"),
            Entry {
                relation: Relation::Dominant,
                ..entry(0, 76, 4, "D4.8=3 F4.4=4 E4.4 D4.8=5")
            },
        ];
        ImitationPlan {
            points: vec![
                Point {
                    words: 0..7,
                    kind: PointKind::New,
                    subject: subject(&first[0], 0..4),
                    entries: first,
                    goal: Some(8),
                },
                Point {
                    words: 3..7,
                    kind: PointKind::Tail,
                    subject: subject(&tail[0], 3..6),
                    entries: tail,
                    goal: None,
                },
            ],
            measures: 16,
            cadences: cadences_at(dorian(), &[8]),
            rate: 50,
            cost: 0,
            ending: Step::D,
        }
    }

    /// The notes of one voice among fixed notes, as `start:pitch.ticks` with `~` before a
    /// continuation, then the role (`F` formula, `E` entry with its rest, `S` subject) and the
    /// choice pinned (`=` and the syllable, `=_` for a melisma).
    fn notes_of(fixed: &[Fixed], voice: usize) -> Vec<String> {
        let mut notes: Vec<&Fixed> = fixed.iter().filter(|f| f.voice == voice).collect();
        notes.sort_by_key(|f| f.start);
        notes
            .iter()
            .map(|f| {
                let tie = if f.tied_in { "~" } else { "" };
                let role = match f.role {
                    FixedRole::Formula => "F".to_string(),
                    FixedRole::Entry { rest } => format!("E{rest}"),
                    FixedRole::Subject => "S".to_string(),
                };
                let sung = match f.sung {
                    Some(Choice::Start(y)) => format!("={y}"),
                    Some(Choice::Hold) => "=_".to_string(),
                    None => String::new(),
                };
                let pitch = f.note.pitch.unwrap();
                let ticks = f.note.duration.ticks();
                format!("{tie}{}:{pitch}.{ticks} {role}{sung}", f.start)
            })
            .collect()
    }

    /// The entries become exact fixed notes, split at the barlines: the first an entry with its
    /// rest, the others subject notes, the syllables pinned at the attacks and none on a
    /// continuation. The formulas of every cadence follow, as a search without imitation sets
    /// them.
    #[test]
    fn plans_give_their_fixed_notes() {
        let plan = kyrie_plan();
        let fixed = plan.fixed(dorian(), 3);
        assert_eq!(
            notes_of(&fixed, 0),
            [
                "0:A4.8 E0=0",
                "8:A4.4 S=1",
                "12:B4.4 S=2",
                "16:C5.8 S=3",
                "~24:C5.4 S",
                "28:B4.4 S=_",
                "~56:A4.4 F",
                "60:G#4.4 F",
                "64:A4.8 F",
                "76:D4.4 E4=3",
                "~80:D4.4 S",
                "84:F4.4 S=4",
                "88:E4.4 S=_",
                "92:D4.4 S=5",
                "~96:D4.4 S",
                "~112:D4.4 F",
                "116:C#4.4 F",
                "120:D4.8 F",
            ]
        );
        assert_eq!(
            notes_of(&fixed, 1),
            [
                "12:D4.4 E0=0",
                "~16:D4.4 S",
                "20:D4.4 S=1",
                "24:E4.4 S=2",
                "28:F4.4 S=3",
                "~32:F4.8 S",
                "68:A3.4 E4=3",
                "~72:A3.4 S",
                "76:C4.4 S=4",
                "80:Bb3.4 S=_",
                "84:A3.4 S=5",
                "~88:A3.4 S",
            ]
        );
        assert_eq!(
            notes_of(&fixed, 2),
            [
                "24:A2.8 E0=0",
                "32:A2.4 S=1",
                "36:B2.4 S=2",
                "40:C3.8 S=3",
                "56:E5.8 F",
                "64:A4.8 F",
                "112:A4.8 F",
                "120:D4.8 F",
            ]
        );
        // The heads are exact, B♭ outside the scale marked as ficta.
        let heads = fixed.iter().filter(|f| f.role != FixedRole::Formula);
        for f in heads {
            assert!(!f.any_octave);
            assert_eq!(f.note.ficta, f.note.pitch.unwrap().alter != 0, "{f:?}");
        }
        let formulas: Vec<Fixed> = fixed
            .iter()
            .filter(|f| f.role == FixedRole::Formula)
            .cloned()
            .collect();
        let mut expected = cadence_formula(dorian(), Step::D, 15, 3, Meter::ALLA_BREVE);
        expected.extend(cadence_formula(dorian(), Step::A, 8, 3, Meter::ALLA_BREVE));
        assert_eq!(formulas, expected);
    }

    /// The score keeps the plan's points, entries and goals, which its text form reads back,
    /// and which fit a setting of the plan's length that sings its text.
    #[test]
    fn plans_travel_in_the_score() {
        use crate::clef::VoiceType;
        use crate::score::{Role, Score, ScoreText, Species, Voice};
        use crate::text::{TextOptions, resolve};

        let plan = kyrie_plan();
        let kept = plan.score_imitation();
        assert_eq!(kept.points.len(), 2);
        assert_eq!(
            (kept.points[0].words.clone(), kept.points[0].goal),
            (0..7, Some(8))
        );
        assert_eq!(
            (kept.points[1].words.clone(), kept.points[1].goal),
            (3..7, None)
        );
        assert_eq!(kept.points[0].entries.len(), 3);
        assert_eq!(
            kept.points[1].entries[0],
            ScoreEntry {
                voice: 1,
                start: 68,
                rest: 4,
                notes: ["A3", "C4", "Bb3", "A3"]
                    .into_iter()
                    .map(|p| p.parse().unwrap())
                    .zip([8, 4, 4, 8])
                    .collect(),
            }
        );
        assert_eq!(ScoreImitation::parse(&kept.spec()), Ok(kept.clone()));

        let voices = [VoiceType::Alto, VoiceType::Tenor, VoiceType::Bass]
            .into_iter()
            .map(|voice_type| {
                let rests = vec![Note::rest(Duration::WHOLE); plan.measures];
                Voice::new(voice_type, Role::Counterpoint(Species::Free), rests)
            })
            .collect();
        let mut score = Score::new(dorian(), voices);
        assert_eq!(score.imitation, None);
        score.cadences = plan.cadences.clone();
        let (text, _) = resolve("kyrie", &TextOptions::default()).unwrap();
        score.text = Some(ScoreText {
            text: Arc::new(text),
            voices: vec![true; 3],
        });
        score.imitation = Some(Arc::new(kept));
        assert_eq!(score.validate(), Ok(()));
        // Without the bass, its entry has no voice.
        score.voices.truncate(2);
        score.text.as_mut().unwrap().voices.truncate(2);
        assert_eq!(
            score.validate(),
            Err(ScoreError::ImitationVoice {
                point: 0,
                entry: 2,
                voice: 2
            })
        );
    }

    /// A head motif is the pitches of a head (phase 14, MASS-MOTIF): a pitch repeated at once
    /// counts once, the first tone lies within the octave above the final, each alteration is
    /// counted against the scale; five to ten tones. Its text form reads back, and gives its
    /// pitches in any octave.
    #[test]
    fn motifs_keep_the_pitches_of_a_head() {
        let pitches = |names: &str| -> Vec<Pitch> {
            names
                .split_whitespace()
                .map(|name| name.parse().unwrap())
                .collect()
        };
        let head = pitches("A3 A3 F3 G3 A3 Bb3 Bb3 A3");
        let motif = Motif::of_pitches(dorian(), &head).unwrap();
        assert_eq!(
            motif.tones,
            [(4, 0), (2, 0), (3, 0), (4, 0), (5, -1), (4, 0)]
        );
        assert_eq!(motif.to_string(), "4,2,3,4,5b,4");
        assert_eq!("4,2,3,4,5b,4".parse::<Motif>(), Ok(motif.clone()));
        assert_eq!(
            motif.pitches(dorian(), 3),
            pitches("A3 F3 G3 A3 Bb3 A3"),
            "the pitches as sung, repeated ones once"
        );
        assert_eq!(motif.pitches(dorian(), 4)[0], "A4".parse().unwrap());
        // Below the final, and in a transposed mode, whose B♭ is its scale's.
        let low = Motif::of_pitches(dorian(), &pitches("D4 C4 A3 C4 D4")).unwrap();
        assert_eq!(low.to_string(), "0,-1,-3,-1,0");
        assert_eq!(low.pitches(dorian(), 2), pitches("D2 C2 A1 C2 D2"));
        let transposed: ModeSpec = "dorian-transposed".parse().unwrap();
        let flat = Motif::of_pitches(transposed, &pitches("G3 Bb3 C4 D4 Eb4 D4")).unwrap();
        assert_eq!(flat.to_string(), "0,2,3,4,5b,4");
        assert_eq!(flat.pitches(transposed, 3), pitches("G3 Bb3 C4 D4 Eb4 D4"));
        let sharp = Motif::of_pitches(dorian(), &pitches("D4 C#4 D4 E4 F4")).unwrap();
        assert_eq!(sharp.to_string(), "0,-1#,0,1,2");
        assert_eq!("0,-1#,0,1,2".parse::<Motif>(), Ok(sharp));
        // Five to ten tones.
        assert_eq!(
            Motif::of_pitches(dorian(), &pitches("D4 D4 F4 E4 D4")),
            None
        );
        let eleven = pitches("D4 E4 F4 G4 A4 G4 F4 E4 D4 E4 F4");
        assert_eq!(Motif::of_pitches(dorian(), &eleven), None);
        assert_eq!(
            Motif::of_pitches(dorian(), &eleven[..10]).map(|m| m.tones.len()),
            Some(10)
        );
        for text in [
            "",
            "0,2,3,4",
            "0,1,2,3,4,5,6,7,8,9,10",
            "0,2,x,4,5",
            "0,2b#,3,4,5",
        ] {
            assert_eq!(
                text.parse::<Motif>(),
                Err(MotifError { text: text.into() }),
                "{text:?}"
            );
        }
    }

    /// A request asks for imitation only where it says so, and leaves every choice to the
    /// planner unless it makes it.
    #[test]
    fn requests_leave_imitation_off() {
        assert_eq!(FreeRequest::new(dorian(), Vec::new(), 12).imitation, None);
        let imitation = Imitation::default();
        assert_eq!(imitation.subject, SubjectSource::Text);
        assert_eq!(
            (imitation.restate, imitation.pairs, imitation.answer),
            (Toggle::Auto, Toggle::Auto, AnswerChoice::Auto)
        );
        assert_eq!(
            (imitation.points, imitation.later_voices, imitation.head),
            (None, None, None)
        );
        assert!(imitation.orders.is_empty());
        assert!(imitation.relations.is_empty() && imitation.delays.is_empty());
    }
}
