//! Facts about a score that many rules need, computed once per check.

use crate::interval::Interval;
use crate::mode::{Clausula, ModeSpec};
use crate::pitch::{Pitch, Step};
use crate::score::{Role, Score, Slice, Sounding, Species};

/// Sounding notes per voice (ties merged), the slices at every onset, and basic facts.
pub struct Analysis {
    /// Per voice, the sounding notes in order.
    pub lines: Vec<Vec<Sounding>>,
    /// What sounds at each onset of any voice.
    pub slices: Vec<Slice>,
    /// Ticks per measure.
    pub per_measure: u32,
    /// Length of the longest voice in ticks.
    pub total: u32,
    /// Per voice, where its written notes end. The search fills voices up to the current tick.
    pub ends: Vec<u32>,
    /// The internal cadences marked in the score and the final cadence, in order.
    pub cadences: Vec<CadencePoint>,
}

/// A cadence: an internal one marked in the score, or the final cadence at the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CadencePoint {
    /// Start of the measure the clausula arrives in.
    pub goal: u32,
    pub target: Step,
    pub clausula: Clausula,
    /// The final cadence.
    pub last: bool,
}

impl CadencePoint {
    /// The target in the given octave, spelled in the scale.
    pub fn target_pitch(&self, mode: ModeSpec, octave: i8) -> Pitch {
        Pitch::new(self.target, mode.scale_alter(self.target), octave)
    }

    /// Whether `pitch` is the target in some octave.
    pub fn is_target(&self, mode: ModeSpec, pitch: Pitch) -> bool {
        pitch.same_class(self.target_pitch(mode, pitch.octave))
    }
}

/// The valid internal cadences of a score (see [`Score::validate`]) and its final cadence. The
/// cadences of free counterpoint stand also beyond the end of an incomplete score: the search
/// sets the voices measure by measure towards them.
fn cadence_points(score: &Score, per_measure: u32, total: u32) -> Vec<CadencePoint> {
    let measures = (total / per_measure) as usize;
    let free = score.is_free();
    let mut points: Vec<CadencePoint> = score
        .cadences
        .iter()
        .filter(|c| c.measure >= 1 && (free || c.measure + 3 <= measures))
        .filter_map(|c| {
            Some(CadencePoint {
                goal: c.measure as u32 * per_measure,
                target: c.target,
                clausula: score.mode.clausula(c.target)?,
                last: false,
            })
        })
        .collect();
    points.sort_by_key(|c| c.goal);
    points.dedup_by_key(|c| c.goal);
    // The final cadence of free counterpoint lies in the notes; the rules look for it once the
    // score is complete (see `free_final_cadence`).
    if free {
        return points;
    }
    let final_step = score.mode.final_step();
    if let (Some(clausula), true) = (score.mode.clausula(final_step), measures >= 2) {
        points.push(CadencePoint {
            goal: total - per_measure,
            target: final_step,
            clausula,
            last: true,
        });
    }
    points
}

/// The final cadence of free counterpoint, found in the notes: its target is the last note of the
/// lowest voice (a section may end on another degree than the final), its goal the last downbeat
/// on which a voice reaches the target by the semitone of the clausula. What follows is a coda
/// over the final, as at the end of many of Palestrina's movements. Without such a clausula the
/// goal is the last measure, where the rules miss it.
pub fn free_final_cadence(score: &Score, per_measure: u32, total: u32) -> Option<CadencePoint> {
    if total < 2 * per_measure {
        return None;
    }
    let bass = score
        .voices
        .iter()
        .filter_map(|v| {
            let (start, note) = v.timed().last()?;
            (start + note.duration.ticks() == total)
                .then_some(note.pitch)
                .flatten()
        })
        .min_by_key(|p| (p.midi(), p.diatonic()))?;
    let target = bass.step;
    let clausula = score.mode.clausula(target)?;
    let point = CadencePoint {
        goal: total - per_measure,
        target,
        clausula,
        last: true,
    };
    let arrival = score
        .voices
        .iter()
        .flat_map(|v| {
            let line = v.soundings();
            (1..line.len())
                .filter(|&i| line[i - 1].end == line[i].start)
                .map(|i| (line[i - 1], line[i]))
                .collect::<Vec<_>>()
        })
        .filter(|(a, b)| {
            let step = Interval::between(a.pitch, b.pitch);
            let semitone = match clausula {
                Clausula::Leading => step.steps == 1 && step.semitones == 1,
                Clausula::Phrygian => step.steps == -1 && step.semitones == -1,
            };
            b.start.is_multiple_of(per_measure)
                && b.start < total
                && semitone
                && point.is_target(score.mode, b.pitch)
        })
        .map(|(_, b)| b.start)
        .max();
    Some(CadencePoint {
        goal: arrival.unwrap_or(point.goal),
        ..point
    })
}

/// How two voices move from one slice to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// In opposite directions.
    Contrary,
    /// One voice keeps its pitch.
    Oblique,
    /// In the same direction (parallel motion included).
    Similar,
    /// Neither voice changes its pitch.
    Static,
}

/// Two voices at two consecutive slices.
#[derive(Clone, Copy, Debug)]
pub struct Transition {
    pub a1: Sounding,
    pub a2: Sounding,
    pub b1: Sounding,
    pub b2: Sounding,
    /// Tick of the second slice.
    pub tick: u32,
}

impl Transition {
    pub fn motion(&self) -> Motion {
        let da = self.a2.pitch.midi() - self.a1.pitch.midi();
        let db = self.b2.pitch.midi() - self.b1.pitch.midi();
        match (da, db) {
            (0, 0) => Motion::Static,
            (0, _) | (_, 0) => Motion::Oblique,
            _ if (da > 0) == (db > 0) => Motion::Similar,
            _ => Motion::Contrary,
        }
    }

    pub fn before(&self) -> Interval {
        harmonic(self.a1, self.b1)
    }

    pub fn after(&self) -> Interval {
        harmonic(self.a2, self.b2)
    }
}

/// The interval between two sounding notes, measured upwards from the lower one.
pub fn harmonic(a: Sounding, b: Sounding) -> Interval {
    let (low, high) =
        if (a.pitch.diatonic(), a.pitch.midi()) <= (b.pitch.diatonic(), b.pitch.midi()) {
            (a, b)
        } else {
            (b, a)
        };
    Interval::between(low.pitch, high.pitch)
}

impl Analysis {
    pub fn new(score: &Score) -> Analysis {
        let lines: Vec<Vec<Sounding>> = score.voices.iter().map(|v| v.soundings()).collect();
        let per_measure = score.meter.ticks_per_measure();
        let total = score.total_ticks();
        Analysis {
            slices: score.slices_from(&lines),
            lines,
            per_measure,
            total,
            ends: score.voices.iter().map(|v| v.total_ticks()).collect(),
            cadences: cadence_points(score, per_measure, total),
        }
    }

    /// The internal cadences.
    pub fn inner_cadences(&self) -> impl Iterator<Item = &CadencePoint> {
        self.cadences.iter().filter(|c| !c.last)
    }

    /// Whether every voice has its note at `tick`. While the search sets the voices of a tick
    /// one after the other, the sonority there is incomplete, and rules about the whole sonority
    /// wait.
    pub fn complete_at(&self, tick: u32) -> bool {
        self.ends.iter().all(|&end| end > tick || end >= self.total)
    }

    /// Whether voice `v` has its note at `tick` (or has ended with the piece).
    fn present_at(&self, v: usize, tick: u32) -> bool {
        self.ends[v] > tick || self.ends[v] >= self.total
    }

    /// The bass at `tick` once it is certain: the lowest sounding voice, if every voice listed
    /// below it already has its note there. The search sets the voices of a tick from the bottom
    /// up, so the bass is known before the voices above it.
    pub fn bass_at(&self, tick: u32) -> Option<(usize, Sounding)> {
        let slice = self.slice_at(tick)?;
        let (bass, note) = slice.lowest()?;
        (bass + 1..self.voice_count())
            .all(|v| self.present_at(v, tick))
            .then_some((bass, note))
    }

    pub fn voice_count(&self) -> usize {
        self.lines.len()
    }

    /// Consecutive notes of a voice that follow each other without a rest: (index of the second
    /// in the line, first, second).
    pub fn melodic_pairs(&self, voice: usize) -> impl Iterator<Item = (usize, Sounding, Sounding)> {
        let line = &self.lines[voice];
        line.windows(2)
            .enumerate()
            .filter(|(_, w)| w[0].end == w[1].start)
            .map(|(i, w)| (i + 1, w[0], w[1]))
    }

    /// Measure (from 0) of a tick.
    pub fn measure(&self, tick: u32) -> usize {
        (tick / self.per_measure) as usize
    }

    pub fn is_downbeat(&self, tick: u32) -> bool {
        tick.is_multiple_of(self.per_measure)
    }

    /// Whether the tick lies in the first measure.
    pub fn in_first_measure(&self, tick: u32) -> bool {
        tick < self.per_measure
    }

    /// Whether the tick lies in the last measure.
    pub fn in_last_measure(&self, tick: u32) -> bool {
        tick + self.per_measure >= self.total
    }

    /// Voices `a` and `b` at slice `i - 1` and slice `i`, if both sound in both and at least one
    /// of them starts a note at slice `i`.
    pub fn transition(&self, i: usize, a: usize, b: usize) -> Option<Transition> {
        let (s1, s2) = (self.slices.get(i.checked_sub(1)?)?, self.slices.get(i)?);
        let (a1, a2, b1, b2) = (
            s1.sounding[a]?,
            s2.sounding[a]?,
            s1.sounding[b]?,
            s2.sounding[b]?,
        );
        if a2.start != s2.tick && b2.start != s2.tick {
            return None;
        }
        Some(Transition {
            a1,
            a2,
            b1,
            b2,
            tick: s2.tick,
        })
    }

    /// What sounds at a tick: the last slice at or before it.
    pub fn slice_at(&self, tick: u32) -> Option<&Slice> {
        let after = self.slices.partition_point(|s| s.tick <= tick);
        self.slices.get(after.checked_sub(1)?)
    }

    /// The first downbeat after `tick`.
    pub fn next_downbeat(&self, tick: u32) -> u32 {
        (tick / self.per_measure + 1) * self.per_measure
    }

    /// Indices of the slices on downbeats.
    pub fn downbeat_slices(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.slices.len()).filter(|&i| self.is_downbeat(self.slices[i].tick))
    }
}

/// The species of a voice, or `None` for the cantus firmus.
pub fn species(score: &Score, voice: usize) -> Option<Species> {
    match score.voices[voice].role {
        Role::CantusFirmus => None,
        Role::Counterpoint(s) => Some(s),
    }
}
