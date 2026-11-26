//! What counts as a dissonance between two notes, for the rules of the species.
//!
//! In two parts every interval but a consonance is a dissonance. With more parts a note is
//! measured against the bass, the lowest sounding voice, where the fourth is a dissonance too;
//! between upper voices fourths and the diminished fifth are free (Fux; Mann 71). Two more points
//! matter with several moving voices:
//!
//! - **Who answers for a clash.** A note that carries the dissonance itself excuses the other:
//!   a suspension (a note held over the downbeat), or a shorter note that began off the beat like
//!   the other one. In two parts the cantus never moves within the measure, so the counterpoint
//!   always answers.
//! - **The delayed leading tone.** From three voices on, in the penultimate measure of a cadence,
//!   the bass may bring the leading tone late, down by step from the target held over the
//!   downbeat, against a diminished fifth in an upper voice (Gallon/Bitsch rule 54; Fux, Fig. 123,
//!   144, 151, 156). HAR-DISSONANCE reports its use; the species rules take it as consonant.

use super::Ctx;
use super::analysis::{CadencePoint, harmonic, species};
use crate::interval::Interval;
use crate::mode::Clausula;
use crate::score::{Sounding, Species};

/// How a note clashes with another voice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Clash {
    /// A perfect fourth above the bass with three or more voices: a 6/4 sonority.
    FourthAboveBass,
    Dissonance,
}

/// Fourths and the diminished fifth: free between upper voices (with the diminished fourth,
/// which sounds like a third; Fig. 183).
pub(super) fn free_between_upper_voices(interval: Interval) -> bool {
    let s = interval.simple();
    s.steps == 3 || (s.steps == 4 && s.semitones == 6)
}

/// The cadence whose delayed leading tone `bass_note` (in voice `bass`) is: three or more voices,
/// and the bass reaches the leading tone of the target in the penultimate measure after the
/// downbeat, down by step from the target held over it (Fig. 123, 144, 151, 156, 197, 203).
pub(super) fn delayed_leading_tone<'a>(
    ctx: &'a Ctx<'a>,
    bass: usize,
    bass_note: Sounding,
) -> Option<&'a CadencePoint> {
    if ctx.analysis.voice_count() < 3 {
        return None;
    }
    let per = ctx.analysis.per_measure;
    let cadence = ctx.analysis.cadences.iter().find(|c| {
        let penultimate = c.goal.saturating_sub(per);
        penultimate < bass_note.start && bass_note.start < c.goal
    })?;
    if cadence.clausula != Clausula::Leading {
        return None;
    }
    let target = cadence.target_pitch(ctx.score.mode, bass_note.pitch.octave);
    if (target.midi() - bass_note.pitch.midi()).rem_euclid(12) != 1 {
        return None;
    }
    let line = &ctx.analysis.lines[bass];
    let i = line.partition_point(|s| s.start < bass_note.start);
    let held = *line.get(i.checked_sub(1)?)?;
    (held.end == bass_note.start
        && held.start < cadence.goal - per
        && Interval::between(held.pitch, bass_note.pitch).steps == -1)
        .then_some(cadence)
}

/// Whether the tritone between the bass note and an upper voice at `tick` is the tolerated one
/// against the delayed leading tone in the penultimate measure of a cadence. In free
/// counterpoint, whose cadences are not marked, the figure itself shows it.
pub(super) fn cadence_tritone(
    ctx: &Ctx,
    bass: usize,
    bass_note: Sounding,
    interval: Interval,
    tick: u32,
) -> bool {
    interval.is_tritone()
        && (delayed_leading_tone(ctx, bass, bass_note).is_some_and(|c| {
            let penultimate = c.goal - ctx.analysis.per_measure;
            penultimate <= tick && tick < c.goal
        }) || free_delayed_leading_tone(ctx, bass, bass_note))
}

/// The delayed leading tone of a free bass: a semitone down from the note it held over the
/// downbeat, later in the measure (Palestrina's cadences need no marks).
pub(super) fn free_delayed_leading_tone(ctx: &Ctx, bass: usize, bass_note: Sounding) -> bool {
    if ctx.analysis.voice_count() < 3 || species(ctx.score, bass) != Some(Species::Free) {
        return false;
    }
    let per = ctx.analysis.per_measure;
    let line = &ctx.analysis.lines[bass];
    let i = line.partition_point(|s| s.start < bass_note.start);
    let Some(held) = i.checked_sub(1).map(|j| line[j]) else {
        return false;
    };
    let downbeat = bass_note.start / per * per;
    let down = Interval::between(held.pitch, bass_note.pitch);
    held.end == bass_note.start
        && held.start < downbeat
        && bass_note.start > downbeat
        && down.steps == -1
        && down.semitones == -1
}

/// Whether `note` of a free voice repeats the note before it: prepared like a suspension, for
/// Palestrina strikes a suspended note again for a new syllable.
pub(super) fn repeated(ctx: &Ctx, voice: usize, note: Sounding) -> bool {
    if species(ctx.score, voice) != Some(Species::Free) {
        return false;
    }
    let line = &ctx.analysis.lines[voice];
    let i = line.partition_point(|s| s.start < note.start);
    i > 0 && line[i - 1].end == note.start && line[i - 1].pitch == note.pitch
}

/// Whether `note` has to answer for its clash with `other` at `tick`: not if `other` carries the
/// dissonance itself, being held over this measure's downbeat while `note` is not, or being
/// shorter while both began off the beat. The lengths count up to the barline: a tie into the
/// next measure is not yet known when a note begins.
fn answers_for(ctx: &Ctx, note: Sounding, other: Sounding, tick: u32) -> bool {
    let per = ctx.analysis.per_measure;
    let downbeat = tick / per * per;
    if other.start < downbeat && note.start >= downbeat {
        return false;
    }
    let off_beat = |n: Sounding| !n.start.is_multiple_of(per);
    let length = |n: Sounding| n.end.min(ctx.analysis.next_downbeat(n.start)) - n.start;
    !(off_beat(note) && off_beat(other) && length(other) < length(note))
}

/// How `note` (in `voice`) clashes with `other` (in voice `u`) at `tick`, whoever answers for
/// it; `None` for no clash. `bass` is the bass at the tick from three voices on, `None` in two
/// parts.
fn clash_between(
    ctx: &Ctx,
    (voice, note): (usize, Sounding),
    (u, other): (usize, Sounding),
    bass: Option<usize>,
    tick: u32,
) -> Option<Clash> {
    let interval = harmonic(note, other);
    if interval.is_consonant() {
        return None;
    }
    if ctx.analysis.voice_count() == 2 {
        return Some(Clash::Dissonance);
    }
    if bass == Some(u) || bass == Some(voice) {
        let (b, b_note) = if bass == Some(u) {
            (u, other)
        } else {
            (voice, note)
        };
        if cadence_tritone(ctx, b, b_note, interval, tick) {
            return None;
        }
        if bass == Some(u) && interval.is_perfect_fourth() {
            Some(Clash::FourthAboveBass)
        } else {
            Some(Clash::Dissonance)
        }
    } else if free_between_upper_voices(interval) {
        None
    } else {
        Some(Clash::Dissonance)
    }
}

/// How `note` (in `voice`) clashes with the note of voice `u` at `tick`, whoever answers for
/// it: a dissonance, or a fourth above `u` as the bass. `None` for no clash, and while the bass
/// is unknown (from three voices on).
pub(super) fn clash_with(
    ctx: &Ctx,
    voice: usize,
    note: Sounding,
    u: usize,
    tick: u32,
) -> Option<(Sounding, Clash)> {
    let slice = ctx.analysis.slice_at(tick)?;
    let other = slice.sounding.get(u).copied().flatten()?;
    let bass = if ctx.analysis.voice_count() == 2 {
        None
    } else {
        Some(ctx.analysis.bass_at(tick)?.0)
    };
    clash_between(ctx, (voice, note), (u, other), bass, tick).map(|clash| (other, clash))
}

/// The worst clash of `note` (in `voice`) with another voice at `tick`, with that voice and its
/// note. Voices the search has not yet set at the tick do not clash; while the bass is unknown,
/// nothing does.
pub(super) fn clash_at(
    ctx: &Ctx,
    voice: usize,
    note: Sounding,
    tick: u32,
) -> Option<(usize, Sounding, Clash)> {
    let slice = ctx.analysis.slice_at(tick)?;
    let two = ctx.analysis.voice_count() == 2;
    let bass = if two {
        None
    } else {
        Some(ctx.analysis.bass_at(tick)?.0)
    };
    let mut worst: Option<(usize, Sounding, Clash)> = None;
    for (u, other) in slice.sounding.iter().enumerate() {
        let Some(other) = *other else { continue };
        if u == voice {
            continue;
        }
        let Some(clash) = clash_between(ctx, (voice, note), (u, other), bass, tick) else {
            continue;
        };
        if !two && !answers_for(ctx, note, other, tick) {
            continue;
        }
        if worst.is_none_or(|(_, _, w)| clash > w) {
            worst = Some((u, other, clash));
        }
    }
    worst
}

/// A voice that `note` clashes with at `tick`.
pub(super) fn dissonance_at(
    ctx: &Ctx,
    voice: usize,
    note: Sounding,
    tick: u32,
) -> Option<(usize, Sounding)> {
    clash_at(ctx, voice, note, tick).map(|(u, other, _)| (u, other))
}

/// Whether `note` clashes with another voice where it begins.
pub(super) fn is_dissonant(ctx: &Ctx, voice: usize, note: Sounding) -> bool {
    clash_at(ctx, voice, note, note.start).is_some()
}
