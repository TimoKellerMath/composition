//! The checks of a plan before any search (docs/regeln.md § 18): what the fixed notes of its
//! entries make certain, whatever the free counterpoint around them does. A plan that fails a
//! check is never searched; what the checks price counts in its cost.
//!
//! - **Rhythm** (C1): the measures that the rest before an entry and its head touch can still be
//!   written in a measure rhythm of free counterpoint that the profile does not forbid
//!   (FREE-RHYTHM).
//! - **Melody** (C2): each head checked alone in its voice and octave: no prohibition, and its
//!   costs count. The same check ranks the octaves an entry may take.
//! - **Text** (C3): the syllables pinned in a head, stepped through the underlay model from
//!   where the voice stands in the text before it, at the head's place in the measure and with
//!   its first note as the entry sings it: a shortened first note may give TXT-ACCENT `longer`.
//! - **Formulas and ends** (C4): no head note where its voice sings a cadence formula or holds
//!   the last measure, and none on the step of a clausula's semitone while that sounds.
//! - **Pairs** (C5): the certain conflicts of two fixed lines, below.
//! - **Skeleton** (C6): the checker on a score of all heads, with rests elsewhere and the plan
//!   attached. A prohibition decided at a tick where every voice is fixed or planned silent is
//!   certain, and so is every case of the IMI rules that depends on the plan alone; so is a
//!   dissonant figure (FREE-PASSING, FREE-RESOLUTION) whose note sounds while every voice is
//!   fixed or silent, or that answers for a second or a seventh with a fixed note, its voice
//!   fixed from the note before it to where the figure is decided; and the last note of a head
//!   that is dissonant where every other voice is fixed and cannot go on by a step, or stay,
//!   without a prohibition. Their costs count. The formulas stay out of the skeleton, since the
//!   search chooses their octaves; the pair checks take them.
//! - **Text capacity** (C7): every voice has the minim beats to begin the syllables that lie
//!   between the end of one head and the syllable its next entry begins, as the search counts
//!   them ([`Capacity`]); a syllable beyond four fifths of the beats costs one.
//! - **Texture** (C8): a downbeat on which fewer than half of the voices can sound, the others
//!   resting before their entries, costs what FREE-TEXTURE charges; one on which half of them
//!   can sound at most, in four or six voices, what IMI-SILENCE `thin` charges.
//!
//! **Pairs.** Between two fixed lines these conflicts are certain (a–h):
//!
//! - a: a tone against its own alteration, sounding together (HAR-CHROMATIC-CLASH);
//! - b: a second, seventh, tritone or altered interval struck together; from three voices on not
//!   where one note is a passing quarter off the downbeat and the other a longer note, which
//!   then does not answer for the clash (FREE-PASSING);
//! - c: a fourth struck together on a downbeat whose lower note is the bass: it lies in the
//!   lowest voice, or no other voice can sound below it, being silent, fixed above or unable to
//!   sing that low (HAR-CONSONANCE);
//! - d: a second or seventh struck on a downbeat against a held note, or a fourth struck below it
//!   as the bass, where the held note's line goes on but not a step down (FREE-RESOLUTION);
//!   within a choir also a fourth struck above a held note of the bass (IMI-HELD);
//! - e: an entry off the downbeat, dissonant with a note that sounds from before; within the
//!   choir also any note that enters unprepared (after a rest or by a leap) off the downbeat and
//!   answers for a second or a seventh with a held note or one struck with it, or for a fourth or
//!   a tritone against the bass, which FREE-PASSING forbids at once;
//! - f: an entry dissonant with a formula note that sounds, in whatever octave the search puts
//!   it, or a fourth above a formula note of the lowest voice;
//! - g: an entry a fourth above a held note of the bass;
//! - h: parallel unisons, fifths or octaves, both lines moving from fixed note to fixed note.
//!
//! `alone` judges two lines while every other voice is silent, as the first two entries of an
//! opening sound; the subject builder runs it for its stretto test (Fux, p. 247), on the leader
//! and one follower. Without other voices the lower of two notes is the bass; nothing is spared
//! as a passing quarter, and e judges entries only.

use std::sync::Arc;

use crate::clef::VoiceType;
use crate::generate::{pitch_pool_with, regular_cadences};
use crate::interval::Interval;
use crate::mode::ModeSpec;
use crate::pitch::{Pitch, Step};
use crate::rhythm::{self, Place, Token};
use crate::rules::{Analysis, Checker, Detail, Level, Profile, Violation, Window};
use crate::score::{Note, Role, Score, ScoreImitation, ScoreText, Sounding, Species, Voice};
use crate::text::Text;
use crate::text::underlay::{Capacity, Choice, Event, Found, Gates, Levels, Model, UState, gate};
use crate::time::Duration;

/// Ticks per 2/2 measure: a struck fourth and a suspension count on its downbeats.
const MEASURE: u32 = 8;

/// A quarter, the value of a passing note struck against a longer one.
const QUARTER: u32 = 2;

/// How far below its staff a voice sings at most, in diatonic steps, as the search's pitches
/// reach ([`crate::generate::pitch_pool`]).
const BELOW_STAFF: i32 = 3;

/// A note of a fixed line as it sounds, tied notes merged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Tone {
    pub pitch: Pitch,
    pub start: u32,
    pub end: u32,
}

/// A certain conflict between two fixed lines (C5 a–h), each named by its letter there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Conflict {
    /// a: a tone against its own alteration, sounding together.
    Chromatic,
    /// b: a second, seventh, tritone or altered interval struck together.
    Struck,
    /// c: a fourth struck together on a downbeat, where no voice sounds below it.
    Fourth,
    /// d: a second or seventh struck on a downbeat against a held note, or a fourth struck below
    /// it as the bass, where the held note's line goes on but does not step down.
    Unresolved,
    /// d: a fourth struck on a downbeat above a held note of the bass, where the held note's line
    /// goes on but does not step down.
    UnderFourth,
    /// e: an entry off the downbeat, dissonant with a note that sounds from before.
    Offbeat,
    /// f: an entry dissonant with a formula note, or a fourth above one in the lowest voice.
    Formula,
    /// g: an entry a fourth above a held note of the bass.
    OverBass,
    /// h: parallel unisons, fifths or octaves, both lines moving from fixed note to fixed note.
    Parallel,
}

impl Conflict {
    /// The rule that the conflict breaks in the search, for the diagnosis of a plan that finds
    /// none.
    pub(crate) fn rule(self) -> &'static str {
        match self {
            Conflict::Chromatic => "HAR-CHROMATIC-CLASH",
            Conflict::Struck => "HAR-DISSONANCE",
            Conflict::Fourth => "HAR-CONSONANCE",
            Conflict::Unresolved => "FREE-RESOLUTION",
            Conflict::UnderFourth => "IMI-HELD",
            Conflict::Offbeat | Conflict::Formula | Conflict::OverBass => "FREE-PASSING",
            Conflict::Parallel => "TEX-PARALLEL",
        }
    }
}

/// A certain conflict of two fixed lines: what it is, the tick it is decided at and, for each
/// line, the note before which the line would have to end to avoid it (0 for its first note,
/// which no line can do without).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Clash {
    pub conflict: Conflict,
    pub tick: u32,
    pub before: [usize; 2],
}

/// The interval between two pitches, upwards from the lower one.
fn between(a: Pitch, b: Pitch) -> Interval {
    if (a.diatonic(), a.midi()) <= (b.diatonic(), b.midi()) {
        Interval::between(a, b)
    } else {
        Interval::between(b, a)
    }
}

/// A second, a seventh, a tritone or an altered interval, in any octave.
fn dissonant(a: Pitch, b: Pitch) -> bool {
    let i = between(a, b).simple();
    matches!(i.steps, 1 | 6) || i.is_tritone() || !i.is_regular()
}

/// Whether `a` lies lower than `b`.
fn below(a: Pitch, b: Pitch) -> bool {
    (a.diatonic(), a.midi()) < (b.diatonic(), b.midi())
}

/// A fourth from `low` up to `high`, of any quality and in any octave, by its steps.
fn fourth_above(low: Pitch, high: Pitch) -> bool {
    below(low, high) && (high.step.index() - low.step.index()).rem_euclid(7) == 3
}

/// Whether tone `i` of a line is a passing quarter off the downbeat struck against `other`,
/// a longer note up to the barline: reached and left by step in one direction, as FREE-PASSING
/// allows it; the longer note does not answer for the clash then.
fn passing(line: &[Tone], i: usize, other: &Tone) -> bool {
    let note = line[i];
    let length = |t: &Tone| t.end.min((t.start / MEASURE + 1) * MEASURE) - t.start;
    let (Some(before), Some(after)) = (
        i.checked_sub(1)
            .map(|k| line[k])
            .filter(|b| b.end == note.start),
        line.get(i + 1).filter(|a| a.start == note.end),
    ) else {
        return false;
    };
    let into = note.pitch.diatonic() - before.pitch.diatonic();
    let out = after.pitch.diatonic() - note.pitch.diatonic();
    note.end - note.start == QUARTER
        && !note.start.is_multiple_of(MEASURE)
        && length(other) > QUARTER
        && into.abs() == 1
        && into == out
}

/// How the pair checks judge two lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Scope {
    /// Three voices or more: a passing quarter struck against a longer note is no conflict, and
    /// a note that does not answer for a clash with another (FREE-PASSING) is not judged by it.
    many: bool,
    /// Within a choir, not as the stretto test of two lines alone: a fourth, a tritone or a
    /// diminished fifth is dissonant only against the bass, free between upper voices; and the
    /// notes that enter unprepared within a line count too (e).
    choir: bool,
}

/// Whether a dissonance is one between any two voices, not only against the bass: a second, a
/// seventh, or an altered interval but the fourths and the diminished fifth, which are free
/// between upper voices.
pub(crate) fn harsh(a: Pitch, b: Pitch) -> bool {
    let i = between(a, b).simple();
    dissonant(a, b) && !(i.steps == 3 || (i.steps == 4 && i.semitones == 6))
}

/// Whether tone `k` of a line can be no dissonant figure: it enters after a rest, or it is
/// reached by a leap (FREE-PASSING).
fn unprepared(line: &[Tone], k: usize) -> bool {
    k.checked_sub(1).is_none_or(|before| {
        let before = line[before];
        before.end < line[k].start
            || (line[k].pitch.diatonic() - before.pitch.diatonic()).abs() >= 2
    })
}

/// Whether `note`, struck off the downbeat against `other`, has to answer for their clash
/// (FREE-PASSING): not if `other` is held over the downbeat of the note's measure, nor if both
/// begin off the beat and `other` is shorter up to the barline. With two voices every note
/// answers.
fn answers(note: &Tone, other: &Tone, many: bool) -> bool {
    let downbeat = note.start / MEASURE * MEASURE;
    let length = |t: &Tone| t.end.min(downbeat + MEASURE) - t.start;
    let off_beat = |t: &Tone| !t.start.is_multiple_of(MEASURE);
    !many
        || !((other.start < downbeat && note.start >= downbeat)
            || (off_beat(note) && off_beat(other) && length(other) < length(note)))
}

/// The first certain conflict of two lines (C5 a–e, g, h), the earliest and among those of one
/// tick the first by its letter. `bass(k, tick, pitch)` says whether `pitch`, the lower note at
/// `tick` and in line `k`, is the bass there.
fn first_clash(
    lines: [&[Tone]; 2],
    bass: impl Fn(usize, u32, Pitch) -> bool,
    scope: Scope,
) -> Option<Clash> {
    let many = scope.many;
    let mut found: Option<Clash> = None;
    let mut note = |conflict: Conflict, tick: u32, before: [usize; 2]| {
        if found.is_none_or(|f| (tick, conflict) < (f.tick, f.conflict)) {
            found = Some(Clash {
                conflict,
                tick,
                before,
            });
        }
    };
    for x_side in [0, 1] {
        let y_side = 1 - x_side;
        let (x_line, y_line) = (lines[x_side], lines[y_side]);
        let order = |x: usize, y: usize| if x_side == 0 { [x, y] } else { [y, x] };
        for (i, x) in x_line.iter().enumerate() {
            for (j, y) in y_line.iter().enumerate() {
                if x.start >= y.end || y.start >= x.end {
                    continue;
                }
                let tick = x.start.max(y.start);
                // a, once per pair of lines.
                if x_side == 0 && x.pitch.step == y.pitch.step && x.pitch.alter != y.pitch.alter {
                    note(Conflict::Chromatic, tick, order(i, j));
                }
                if x.start == y.start {
                    if x_side == 0 {
                        // b and c.
                        let (side, low) = if below(x.pitch, y.pitch) {
                            (x_side, x.pitch)
                        } else {
                            (y_side, y.pitch)
                        };
                        if dissonant(x.pitch, y.pitch) {
                            let figure = many && (passing(x_line, i, y) || passing(y_line, j, x));
                            let upper =
                                scope.choir && !harsh(x.pitch, y.pitch) && !bass(side, tick, low);
                            if !figure && !upper {
                                note(Conflict::Struck, tick, order(i, j));
                            }
                        } else if between(x.pitch, y.pitch).is_perfect_fourth()
                            && tick.is_multiple_of(MEASURE)
                        {
                            if bass(side, tick, low) {
                                note(Conflict::Fourth, tick, order(i, j));
                            }
                        } else if scope.choir
                            && between(x.pitch, y.pitch).simple().steps == 3
                            && !tick.is_multiple_of(MEASURE)
                        {
                            // A fourth over the bass struck together off the downbeat: both
                            // notes are dissonant, certain for one that enters unprepared and
                            // answers for it.
                            let (low, high) = if below(x.pitch, y.pitch) {
                                ((x_side, x_line, i), (y_side, y_line, j))
                            } else {
                                ((y_side, y_line, j), (x_side, x_line, i))
                            };
                            if bass(low.0, tick, low.1[low.2].pitch) {
                                for (one, other) in [(high, low), (low, high)] {
                                    let (line, k) = (one.1, one.2);
                                    if unprepared(line, k)
                                        && answers(&line[k], &other.1[other.2], many)
                                    {
                                        note(Conflict::OverBass, tick, order(i, j));
                                    }
                                }
                            }
                        }
                    }
                    continue;
                }
                // y struck while x is held.
                if x.start > y.start {
                    continue;
                }
                let t = y.start;
                // d: a suspension in x must step down where its line goes on; within a choir
                // so must a held bass under a fourth struck above it.
                let interval = between(x.pitch, y.pitch);
                let suspended = matches!(interval.simple().steps, 1 | 6)
                    || (interval.is_perfect_fourth()
                        && below(y.pitch, x.pitch)
                        && bass(y_side, t, y.pitch));
                let under_fourth = scope.choir
                    && interval.is_perfect_fourth()
                    && below(x.pitch, y.pitch)
                    && bass(x_side, t, x.pitch);
                if t.is_multiple_of(MEASURE) && (suspended || under_fourth) {
                    let next = x_line.get(i + 1).filter(|n| n.start == x.end);
                    if next.is_some_and(|n| n.pitch.diatonic() - x.pitch.diatonic() != -1) {
                        let conflict = if suspended {
                            Conflict::Unresolved
                        } else {
                            Conflict::UnderFourth
                        };
                        note(conflict, t, order(i + 1, j));
                    }
                }
                // The entry of y.
                if j == 0 {
                    // e.
                    if !t.is_multiple_of(MEASURE) && dissonant(x.pitch, y.pitch) {
                        note(Conflict::Offbeat, t, order(i, 0));
                    }
                    // g.
                    if fourth_above(x.pitch, y.pitch) && bass(x_side, t, x.pitch) {
                        note(Conflict::OverBass, t, order(i, 0));
                    }
                }
                // An entry above a held bass is g's.
                let entry_above = j == 0 && below(x.pitch, y.pitch);
                if scope.choir && !entry_above && !t.is_multiple_of(MEASURE) && answers(y, x, many)
                {
                    // e within a line: a dissonance reached by a leap can be no figure. A
                    // second or a seventh is one in any voices, a fourth or a tritone against
                    // the bass.
                    let (side, low) = if below(x.pitch, y.pitch) {
                        (x_side, x.pitch)
                    } else {
                        (y_side, y.pitch)
                    };
                    let fourth = between(x.pitch, y.pitch).simple().steps == 3;
                    let clash = harsh(x.pitch, y.pitch)
                        || ((fourth || dissonant(x.pitch, y.pitch)) && bass(side, t, low));
                    if clash && unprepared(y_line, j) {
                        note(Conflict::Offbeat, t, order(i, j));
                    }
                }
            }
        }
    }
    // h.
    let (a, b) = (lines[0], lines[1]);
    for (j, x2) in a.iter().enumerate().skip(1) {
        let x1 = a[j - 1];
        if x1.end != x2.start {
            continue;
        }
        let Some(k) = b.iter().position(|y| y.start == x2.start) else {
            continue;
        };
        if k == 0 || b[k - 1].end != b[k].start {
            continue;
        }
        let (y1, y2) = (b[k - 1], b[k]);
        let (dx, dy) = (
            x2.pitch.midi() - x1.pitch.midi(),
            y2.pitch.midi() - y1.pitch.midi(),
        );
        let (i1, i2) = (between(x1.pitch, y1.pitch), between(x2.pitch, y2.pitch));
        if dx != 0
            && dy != 0
            && dx.signum() == dy.signum()
            && i1.is_perfect_consonance()
            && i2.is_perfect_consonance()
            && i1.simple().steps == i2.simple().steps
        {
            note(Conflict::Parallel, x2.start, [j, k]);
        }
    }
    found
}

/// The first certain conflict of two fixed lines while every other voice is silent, with its
/// tick; `None` if they can sound together. Each line's first tone is an entry, after silence.
/// The checks are those of C5 except f, which concerns the formulas of cadences; the lower of
/// two notes is the bass, and nothing is exempt as a passing note.
pub(crate) fn alone(a: &[Tone], b: &[Tone]) -> Option<(Conflict, u32)> {
    let scope = Scope {
        many: false,
        choir: false,
    };
    first_clash([a, b], |_, _, _| true, scope).map(|c| (c.conflict, c.tick))
}

/// An entry as the checks see it: its voice, the least rest before it (0 for the voice's first
/// entry, before which it is silent from the start), its head as it sounds, tied notes merged,
/// and what each note of the head sings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Head {
    pub voice: usize,
    pub rest: u32,
    pub tones: Vec<Tone>,
    pub sung: Vec<Choice>,
}

impl Head {
    /// The tick of the entry.
    pub(crate) fn start(&self) -> u32 {
        self.tones[0].start
    }

    /// Where the head ends.
    pub(crate) fn end(&self) -> u32 {
        self.tones.last().map_or(self.start(), |t| t.end)
    }

    /// The head an octave higher (`steps` 7) or lower (−7).
    pub(crate) fn octave(&self, steps: i32) -> Head {
        let interval = crate::interval::Interval::new(steps, 12 * steps / 7);
        Head {
            tones: self
                .tones
                .iter()
                .map(|t| Tone {
                    pitch: t.pitch.transpose(interval),
                    ..*t
                })
                .collect(),
            ..self.clone()
        }
    }

    /// The diatonic steps from its lowest note to its highest.
    pub(crate) fn span(&self) -> i32 {
        let steps = || self.tones.iter().map(|t| t.pitch.diatonic());
        steps()
            .max()
            .zip(steps().min())
            .map_or(0, |(high, low)| high - low)
    }

    /// The silence the plan sets before the entry: from the start of the setting before a
    /// voice's first entry, the rest window before a later one.
    pub(crate) fn silence(&self) -> (u32, u32) {
        let start = self.start();
        if self.rest == 0 {
            (0, start)
        } else {
            (start - self.rest.min(start), start)
        }
    }

    /// Whether the plan has the voice silent at `tick` before this entry.
    fn silent_at(&self, tick: u32) -> bool {
        let (from, to) = self.silence();
        from <= tick && tick < to
    }

    /// The tone sounding at `tick`.
    fn at(&self, tick: u32) -> Option<&Tone> {
        self.tones.iter().find(|t| t.start <= tick && tick < t.end)
    }

    /// The syllable the entry begins.
    pub(crate) fn first_syllable(&self) -> Option<u16> {
        match self.sung.first() {
            Some(Choice::Start(y)) => Some(*y),
            _ => None,
        }
    }

    /// The last syllable the head begins, which the voice still sings where it ends.
    pub(crate) fn last_syllable(&self) -> Option<u16> {
        self.sung.iter().rev().find_map(|choice| match choice {
            Choice::Start(y) => Some(*y),
            Choice::Hold => None,
        })
    }

    /// The head ended after its first `notes` notes.
    pub(crate) fn cut(&self, notes: usize) -> Head {
        Head {
            tones: self.tones[..notes.min(self.tones.len())].to_vec(),
            sung: self.sung[..notes.min(self.sung.len())].to_vec(),
            ..self.clone()
        }
    }

    /// The head written in measures from the start of the entry's measure: the rest before the
    /// entry in its measure, then the tones, tied over the barlines. With each written note the
    /// index of the tone whose attack it is.
    fn written(&self, mode: ModeSpec) -> (Vec<Note>, Vec<Option<usize>>) {
        let start = self.start();
        let offset = start % MEASURE;
        let mut notes = Vec::new();
        let mut attacks = Vec::new();
        if offset > 0 {
            notes.push(Note::rest(Duration::from_ticks(offset)));
            attacks.push(None);
        }
        for (k, tone) in self.tones.iter().enumerate() {
            let ficta = !mode.is_scale_pitch(tone.pitch);
            let mut tick = tone.start;
            while tick < tone.end {
                let until = tone.end.min((tick / MEASURE + 1) * MEASURE);
                if tick > tone.start
                    && let Some(last) = notes.last_mut()
                {
                    last.tie = true;
                }
                notes.push(
                    Note::new(tone.pitch, Duration::from_ticks(until - tick)).with_ficta(ficta),
                );
                attacks.push((tick == tone.start).then_some(k));
                tick = until;
            }
        }
        (notes, attacks)
    }
}

/// A note of a cadence formula: its pitch in whatever octave, which the search chooses, where it
/// sounds, and whether it continues the note before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FormulaNote {
    pub voice: usize,
    pub pitch: Pitch,
    pub start: u32,
    pub end: u32,
    pub tied_in: bool,
}

/// What a voice does at a tick as far as the plan fixes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fixed<'a> {
    /// It rests before an entry.
    Silent,
    /// It sings a note of a head.
    Head(&'a Tone),
    /// It sings freely, or a formula note in an octave the search chooses.
    Free,
}

/// What a plan fixes of its voices, as the pair checks need it: the heads in a choir whose voices
/// sing down to three steps below their staves.
pub(crate) struct Frame<'a> {
    pub choir: &'a [VoiceType],
    pub heads: &'a [&'a Head],
}

impl Frame<'_> {
    /// What voice `v` does at `tick`.
    fn fixed(&self, v: usize, tick: u32) -> Fixed<'_> {
        let mut state = Fixed::Free;
        for head in self.heads.iter().filter(|h| h.voice == v) {
            if let Some(tone) = head.at(tick) {
                return Fixed::Head(tone);
            }
            if head.silent_at(tick) {
                state = Fixed::Silent;
            }
        }
        state
    }

    /// Whether `pitch` of voice `v` is the bass at `tick` against voice `other`: every other
    /// voice is silent there, fixed no lower or, unless `v` is the lowest voice of the choir,
    /// unable to sing below it.
    fn is_bass(&self, v: usize, other: usize, tick: u32, pitch: Pitch) -> bool {
        let lowest = self.choir.len().saturating_sub(1);
        (0..self.choir.len())
            .filter(|&w| w != v && w != other)
            .all(|w| match self.fixed(w, tick) {
                Fixed::Silent => true,
                Fixed::Head(tone) => !below(tone.pitch, pitch),
                Fixed::Free => {
                    let floor = self.choir[w].clef().bottom_line().diatonic() - BELOW_STAFF;
                    v == lowest || floor >= pitch.diatonic()
                }
            })
    }

    /// C5 a–e, g and h for two heads of the plan: the first certain conflict.
    pub(crate) fn clash(&self, a: &Head, b: &Head) -> Option<Clash> {
        let voices = [a.voice, b.voice];
        let scope = Scope {
            many: self.choir.len() >= 3,
            choir: true,
        };
        first_clash(
            [&a.tones, &b.tones],
            |side, tick, pitch| self.is_bass(voices[side], voices[1 - side], tick, pitch),
            scope,
        )
    }
}

/// C5 f: an entry dissonant by its pitch class with a formula note that sounds where it enters,
/// or a fourth above a formula note of the lowest voice. The tick of the entry if so.
pub(crate) fn against_formulas(
    head: &Head,
    formulas: &[FormulaNote],
    lowest: usize,
) -> Option<u32> {
    let entry = head.tones[0];
    formulas
        .iter()
        .filter(|f| f.voice != head.voice && f.start <= entry.start && entry.start < f.end)
        .any(|f| {
            let fourth = (entry.pitch.step.index() - f.pitch.step.index()).rem_euclid(7) == 3;
            dissonant(entry.pitch, f.pitch) || (f.voice == lowest && fourth)
        })
        .then_some(entry.start)
}

/// A line of a piece: its voice and its notes as they sound.
#[doc(hidden)]
pub type Line<'a> = (usize, &'a [Sounding]);

/// The first certain conflict that the pair checks (C5 a–e, g, h) find between two lines of a
/// piece in a choir `choir`, each given as its voice and its notes as they sound, while the other
/// voices are silent up to `until` and free from there: its rule and the tick it is decided at,
/// if that comes before `until`. For the test of the pair checks on Palestrina's openings
/// (`tests/imitation_corpus.rs`), which needs the planner's own checks.
#[doc(hidden)]
pub fn pair_conflict(
    choir: &[VoiceType],
    (a, b): (Line, Line),
    until: u32,
) -> Option<(&'static str, u32)> {
    let head = |(voice, notes): Line| Head {
        voice,
        rest: 0,
        tones: notes
            .iter()
            .map(|s| Tone {
                pitch: s.pitch,
                start: s.start,
                end: s.end,
            })
            .collect(),
        sung: vec![Choice::Hold; notes.len()],
    };
    let (a, b) = (head(a), head(b));
    let first = a.tones.first()?.pitch;
    // The other voices: silent from the start up to `until`, where they sound, whatever they
    // sing then.
    let others: Vec<Head> = (0..choir.len())
        .filter(|&v| v != a.voice && v != b.voice && until < u32::MAX)
        .map(|voice| Head {
            voice,
            rest: 0,
            tones: vec![Tone {
                pitch: first,
                start: until,
                end: until + 1,
            }],
            sung: vec![Choice::Hold],
        })
        .collect();
    let mut heads: Vec<&Head> = vec![&a, &b];
    heads.extend(&others);
    let frame = Frame {
        choir,
        heads: &heads,
    };
    frame
        .clash(&a, &b)
        .filter(|clash| clash.tick < until)
        .map(|clash| (clash.conflict.rule(), clash.tick))
}

/// The goal of a cadence as C4 needs it: the tick its target arrives at, the voices that sing
/// its formula and the step of its clausula's semitone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Goal {
    pub tick: u32,
    pub voices: Vec<usize>,
    pub semitone: Option<Step>,
}

/// C4: whether the heads keep clear of the cadences. No head note in a formula voice of a goal
/// from two measures before it to the end of the goal's measure, none in another voice on the
/// step of the clausula's semitone in the minim before the goal; for the final cadence, `last`,
/// none in a formula voice in the two measures before the last and none at all in the last.
pub(crate) fn clear_of_formulas(heads: &[&Head], goals: &[Goal], last: Option<&Goal>) -> bool {
    let touches = |head: &Head, from: u32, to: u32, step: Option<Step>| {
        head.tones
            .iter()
            .any(|t| t.start < to && from < t.end && step.is_none_or(|s| t.pitch.step == s))
    };
    heads.iter().all(|head| {
        goals.iter().all(|g| {
            if g.voices.contains(&head.voice) {
                !touches(
                    head,
                    g.tick.saturating_sub(2 * MEASURE),
                    g.tick + MEASURE,
                    None,
                )
            } else {
                g.semitone.is_none_or(|step| {
                    !touches(head, g.tick.saturating_sub(MEASURE / 2), g.tick, Some(step))
                })
            }
        }) && last.is_none_or(|g| {
            let formula = g.voices.contains(&head.voice)
                && touches(head, g.tick.saturating_sub(2 * MEASURE), g.tick, None);
            !formula && !touches(head, g.tick, g.tick + MEASURE, None)
        })
    })
}

/// C8: the downbeats from `from` up to `until` on which fewer than half of the `voices` can
/// sound, the others resting before an entry, as FREE-TEXTURE counts them from three voices on;
/// and those on which half of them can sound at most, as IMI-SILENCE `thin` counts them in four
/// and six voices after the last first entry.
pub(crate) fn thin_downbeats(heads: &[&Head], voices: usize, from: u32, until: u32) -> (u32, u32) {
    if voices < 3 {
        return (0, 0);
    }
    let (mut fewer, mut half) = (0, 0);
    for tick in (from.div_ceil(MEASURE)..until.div_ceil(MEASURE)).map(|m| m * MEASURE) {
        let silent = (0..voices)
            .filter(|&v| {
                heads
                    .iter()
                    .any(|h| h.voice == v && h.silent_at(tick) && h.at(tick).is_none())
            })
            .count();
        let sounding = voices - silent;
        if 2 * sounding < voices {
            fewer += 1;
        } else if 2 * sounding == voices && voices >= 4 {
            half += 1;
        }
    }
    (fewer, half)
}

/// The measure rhythms of free counterpoint that the profile does not forbid (FREE-RHYTHM).
#[derive(Clone, Debug)]
pub(crate) struct Rhythms {
    /// `None` where the profile allows every rhythm.
    patterns: Option<Vec<Vec<Token>>>,
}

impl Rhythms {
    pub(crate) fn new(checker: &Checker) -> Rhythms {
        let unlisted = checker.level("FREE-RHYTHM", "level");
        let patterns = (unlisted == Level::Forbidden).then(|| {
            rhythm::patterns(Species::Free, Place::Inner, true)
                .iter()
                .filter(|(_, name)| {
                    let level = name.map_or(Level::Off, |name| checker.level("FREE-RHYTHM", name));
                    level != Level::Forbidden
                })
                .map(|(pattern, _)| rhythm::tokens(pattern))
                .collect()
        });
        Rhythms { patterns }
    }

    /// Whether a measure can hold `tokens` from `offset` on, up to `end`: some rhythm the
    /// profile allows has them there, the tokens before and after `end` being free.
    fn fits(&self, offset: u32, tokens: &[Token], end: u32) -> bool {
        let Some(patterns) = &self.patterns else {
            return true;
        };
        patterns.iter().any(|pattern| {
            let mut at = 0;
            let mut i = 0;
            while at < offset && i < pattern.len() {
                at += pattern[i].ticks;
                i += 1;
            }
            if at != offset {
                return false;
            }
            for token in tokens {
                if pattern.get(i) != Some(token) {
                    return false;
                }
                at += token.ticks;
                i += 1;
            }
            at == end && (end < MEASURE || i == pattern.len())
        })
    }

    /// C1: whether the measures that the silence before an entry and its head touch can be
    /// written in rhythms of free counterpoint: from the barline before a voice's first entry,
    /// from the rest window before a later one, where the voice may also rest from the barline.
    pub(crate) fn fit(&self, head: &Head) -> bool {
        let start = head.start();
        let end = head.end();
        let window = match head.silence() {
            (0, _) => start - start % MEASURE,
            (from, _) => from,
        };
        (window / MEASURE..end.div_ceil(MEASURE)).all(|m| {
            let (low, high) = (m * MEASURE, (m + 1) * MEASURE);
            let (from, to) = (window.max(low), end.min(high));
            let tones = |tokens: &mut Vec<Token>| {
                for tone in head.tones.iter().filter(|t| t.start < to && from < t.end) {
                    let (a, b) = (tone.start.max(from), tone.end.min(to));
                    tokens.push(Token {
                        tied_in: tone.start < a,
                        rest: false,
                        ticks: b - a,
                    });
                }
            };
            let rest = |from: u32| Token {
                tied_in: false,
                rest: true,
                ticks: start.min(high) - from,
            };
            let mut tokens = Vec::new();
            if from < start {
                tokens.push(rest(from));
            }
            tones(&mut tokens);
            if self.fits(from - low, &tokens, to - low) {
                return true;
            }
            // A longer rest from the barline.
            if head.rest > 0 && low < from && from < start {
                let mut tokens = vec![rest(low)];
                tones(&mut tokens);
                return self.fits(0, &tokens, to - low);
            }
            false
        })
    }
}

/// What the checks need of a request: its text, mode and voices, the checkers with the
/// profile's levels and the underlay model of the text.
pub(crate) struct Checks {
    pub text: Arc<Text>,
    pub mode: ModeSpec,
    pub choir: Vec<VoiceType>,
    /// The profile of the search with its text, for the skeleton (C6).
    pub checker: Checker,
    /// The same for a head checked alone: as a voice's first entry, and as a later one, which
    /// FREE-ENTRY does not judge (C2).
    first: Checker,
    again: Checker,
    pub levels: Levels,
    pub model: Model,
    pub rhythms: Rhythms,
}

/// The rules of dissonant figures, which judge a note where it begins and decide by the notes
/// after it in its voice.
const FIGURES: [&str; 2] = ["FREE-PASSING", "FREE-RESOLUTION"];

/// Whether a case of the skeleton depends on the free notes of the search: what FREE-TEXTURE
/// counts, which C8 judges, the notes before an entry (IMI-REST `level` and `minim_before`) or
/// around it (IMI-OVERLAP `bare`), the silences between the heads (IMI-SILENCE), where the
/// skeleton rests and the search sings, and a dissonance against a held note (IMI-HELD), which
/// the free voices below and the ways on of the held note decide; pair check d takes the
/// certain ones.
fn free_dependent(v: &Violation) -> bool {
    matches!(
        (v.rule, v.detail),
        ("IMI-REST", Detail::Param("level" | "minim_before"))
            | ("IMI-OVERLAP", Detail::Param("bare"))
            | ("IMI-SILENCE", _)
            | ("IMI-HELD", _)
            | ("FREE-TEXTURE", _)
    )
}

/// The notes of a voice with a note of `pitch` and `ticks` from tick `at` on, where it rests.
fn with_note(notes: &[Note], at: u32, pitch: Pitch, ticks: u32, mode: ModeSpec) -> Vec<Note> {
    let mut out = Vec::with_capacity(notes.len() + 2);
    let mut tick = 0;
    for note in notes {
        let end = tick + note.duration.ticks();
        if note.is_rest() && tick <= at && at < end && at + ticks <= end {
            rests(&mut out, tick, at);
            out.push(
                Note::new(pitch, Duration::from_ticks(ticks))
                    .with_ficta(!mode.is_scale_pitch(pitch)),
            );
            rests(&mut out, at + ticks, end);
        } else {
            out.push(note.clone());
        }
        tick = end;
    }
    out
}

/// A rule's case at a tick in a voice, as the skeleton compares it with the cases of a head
/// checked alone.
type Priced = (usize, &'static str, u32, Detail);

impl Checks {
    /// The checks of a request for `choir` in `mode` on `text`, with the profile `sung` that the
    /// search of a texted request uses; `restarts` are the syllables after which the plan's
    /// voices may restate their sentence besides the ends of units
    /// ([`crate::text::underlay::gate_restarts`]).
    pub(crate) fn new(
        text: Arc<Text>,
        mode: ModeSpec,
        choir: &[VoiceType],
        sung: &Profile,
        restarts: &[u16],
    ) -> Self {
        let checker = Checker::new(sung);
        let mut again = sung.clone();
        again
            .apply_override("FREE-ENTRY=aus")
            .expect("a rule of the catalogue");
        Checks {
            model: Model::new(&text, mode, crate::time::Meter::ALLA_BREVE, &[])
                .with_restarts(&text, restarts),
            levels: Levels::from(&checker),
            rhythms: Rhythms::new(&checker),
            first: Checker::new(sung),
            again: Checker::new(&again),
            checker,
            text,
            mode,
            choir: choir.to_vec(),
        }
    }

    /// C2: the cases of a head checked alone in its voice, as a voice's `first` entry or a later
    /// one, at their ticks in the setting. Only what the head decides counts: the measure it ends
    /// in stays open.
    pub(crate) fn melody(&self, head: &Head, first: bool) -> Vec<Violation> {
        let (notes, _) = head.written(self.mode);
        let shift = head.start() - head.start() % MEASURE;
        let voice = Voice::new(
            self.choir[head.voice],
            Role::Counterpoint(Species::Free),
            notes,
        );
        let score = Score::new(self.mode, vec![voice]);
        let analysis = Analysis::new(&score);
        let checker = if first { &self.first } else { &self.again };
        let mut out = Vec::new();
        let window = Window::partial(0, head.end() - shift);
        checker.check_window(&score, &analysis, window, &[true], &mut out);
        for v in &mut out {
            v.voice = head.voice;
            v.tick += shift;
        }
        out
    }

    /// C3: what the syllables pinned in a head cost, stepped through the underlay model from
    /// `from`, where the voice stands in the text before it; the rule of a forbidden case.
    pub(crate) fn declaim(&self, head: &Head, from: UState) -> Result<u32, &'static str> {
        let (notes, attacks) = head.written(self.mode);
        let mut state = from;
        let mut found: Vec<Found> = Vec::new();
        let mut tick = 0;
        for (i, note) in notes.iter().enumerate() {
            if let Some(event) = Event::at(&notes, i, tick) {
                let choice = match (event, attacks[i]) {
                    (Event::Attack(_), Some(k)) => head.sung[k],
                    _ => Choice::Hold,
                };
                state = self.model.step(&state, &event, choice, &mut found);
            }
            tick += note.duration.ticks();
        }
        match self.levels.price(&found) {
            (_, Some(rule)) => Err(rule),
            (cost, None) => Ok(cost),
        }
    }

    /// C6: what the skeleton of `heads` with the plan `plan` attached costs from tick `from` on,
    /// in voices of `total` ticks; the rule of a certain prohibition: one decided where every
    /// voice is fixed or planned silent, or a dissonant figure (FREE-PASSING, FREE-RESOLUTION)
    /// whose note sounds where they all are and whose voice is fixed up to where it is decided,
    /// and any case of the IMI rules that depends on the plan alone. The cases of the heads
    /// checked alone (`alone`, as [`Checks::melody`] found them) count there, not here; neither
    /// does FREE-TEXTURE, which C8 judges, nor what depends on the free notes before an entry
    /// (IMI-REST `level` and `minim_before`) or around it (IMI-OVERLAP `bare`).
    pub(crate) fn skeleton(
        &self,
        heads: &[&Head],
        plan: &ScoreImitation,
        from: u32,
        total: u32,
        alone: &[Violation],
    ) -> Result<u32, &'static str> {
        let voices = self.choir.len();
        let lines: Vec<Vec<&Head>> = (0..voices)
            .map(|v| {
                let mut list: Vec<&Head> = heads.iter().copied().filter(|h| h.voice == v).collect();
                list.sort_by_key(|h| h.start());
                list
            })
            .collect();
        let notes: Vec<Vec<Note>> = lines
            .iter()
            .map(|list| {
                let mut notes = Vec::new();
                let mut tick = 0;
                for head in list {
                    rests(&mut notes, tick, head.start());
                    let (written, _) = head.written(self.mode);
                    let lead = usize::from(head.start() % MEASURE > 0);
                    notes.extend(written.into_iter().skip(lead));
                    tick = head.end();
                }
                rests(&mut notes, tick, total);
                notes
            })
            .collect();
        let score = self.skeleton_score(&notes, plan);
        let analysis = Analysis::new(&score);
        let mut out = Vec::new();
        let window = Window::partial(from, total);
        self.checker
            .check_window(&score, &analysis, window, &vec![true; voices], &mut out);
        // Where a voice is fixed or planned silent.
        let fixed = |v: usize, tick: u32| {
            lines[v]
                .iter()
                .any(|h| h.silent_at(tick) || (h.start() <= tick && tick < h.end()))
        };
        let all_fixed = |tick: u32| (0..voices).all(|v| fixed(v, tick));
        let priced: Vec<Priced> = alone
            .iter()
            .map(|v| (v.voice, v.rule, v.tick, v.detail))
            .collect();
        let mut cost = 0;
        for v in &out {
            if free_dependent(v) || priced.contains(&(v.voice, v.rule, v.tick, v.detail)) {
                continue;
            }
            let certain = v.rule.starts_with("IMI-") || all_fixed(v.tick) || {
                // A figure is judged where its dissonance sounds and decided where its voice
                // goes on. It is certain where every voice is fixed while the note sounds and
                // its voice up to there, or where the last note of a head finds no way on; and
                // where a note longer than a quarter, its voice fixed from the note before it up
                // to there, answers for a second or a seventh with a fixed note, which no voice
                // below makes consonant and only its own notes can make a figure of.
                let note = analysis.lines[v.voice].iter().find(|s| s.note == v.note);
                FIGURES.contains(&v.rule)
                    && note.is_some_and(|note| {
                        let own = (note.start.saturating_sub(1)..=v.tick)
                            .all(|tick| fixed(v.voice, tick));
                        let tone = Tone {
                            pitch: note.pitch,
                            start: note.start,
                            end: note.end,
                        };
                        let harsh_partner = (0..voices).filter(|&u| u != v.voice).any(|u| {
                            lines[u]
                                .iter()
                                .filter_map(|h| h.at(note.start))
                                .any(|other| {
                                    harsh(note.pitch, other.pitch)
                                        && answers(&tone, other, voices >= 3)
                                })
                        });
                        let all = (note.start..note.end).all(all_fixed);
                        (all && (note.start..=v.tick).all(|tick| fixed(v.voice, tick)))
                            || (own && harsh_partner && note.end - note.start > QUARTER)
                            || (all
                                && v.level == Level::Forbidden
                                && v.tick == note.end
                                && (0..voices).all(|u| u == v.voice || fixed(u, v.tick))
                                && lines[v.voice].iter().any(|h| h.end() == note.end)
                                && !self.goes_on(&notes, plan, v.voice, note))
                    })
            };
            if !certain {
                continue;
            }
            match v.level {
                Level::Forbidden => return Err(v.rule),
                level => cost += level.cost(),
            }
        }
        Ok(cost)
    }

    /// The skeleton score of the voices' `notes` with the plan attached and the text, which no
    /// voice sings yet.
    fn skeleton_score(&self, notes: &[Vec<Note>], plan: &ScoreImitation) -> Score {
        let voices = notes
            .iter()
            .enumerate()
            .map(|(v, notes)| {
                Voice::new(
                    self.choir[v],
                    Role::Counterpoint(Species::Free),
                    notes.clone(),
                )
            })
            .collect();
        let mut score = Score::new(self.mode, voices);
        score.text = Some(ScoreText {
            text: Arc::clone(&self.text),
            voices: vec![false; notes.len()],
        });
        score.imitation = Some(Arc::new(plan.clone()));
        score
    }

    /// Whether voice `v` can go on after `last`, the last note of a head, which is dissonant
    /// where every other voice is fixed: by some note a step away or the same, a third down
    /// after a quarter (the figures of FREE-PASSING), short or held to the barline, without a
    /// prohibition decided where it begins.
    fn goes_on(
        &self,
        notes: &[Vec<Note>],
        plan: &ScoreImitation,
        v: usize,
        last: &Sounding,
    ) -> bool {
        let at = last.end;
        let quarter = last.end - last.start <= QUARTER;
        let pool = pitch_pool_with(self.mode, self.choir[v], &regular_cadences(self.mode));
        let barline = (at / MEASURE + 1) * MEASURE;
        pool.iter()
            .filter(|p| {
                let steps = p.diatonic() - last.pitch.diatonic();
                (-1..=1).contains(&steps) || (quarter && steps == -2)
            })
            .any(|&pitch| {
                [QUARTER, barline - at].into_iter().any(|ticks| {
                    let mut voices = notes.to_vec();
                    voices[v] = with_note(&notes[v], at, pitch, ticks, self.mode);
                    let score = self.skeleton_score(&voices, plan);
                    let analysis = Analysis::new(&score);
                    let mut out = Vec::new();
                    let targets = vec![true; voices.len()];
                    self.checker.check_window(
                        &score,
                        &analysis,
                        Window::partial(at, at),
                        &targets,
                        &mut out,
                    );
                    !out.iter()
                        .any(|c| c.is_forbidden() && c.tick == at && !free_dependent(c))
                })
            })
    }

    /// The minim beats on which voice `v` can begin a syllable, as the search counts them
    /// ([`Capacity`]): not within the notes of its heads and its formulas after their attacks,
    /// nor in the silence before an entry, nor after the attack of its held last measure, in
    /// voices of `total` ticks.
    pub(crate) fn capacity(
        &self,
        v: usize,
        heads: &[&Head],
        formulas: &[FormulaNote],
        total: u32,
    ) -> Capacity {
        let mut held: Vec<(u32, u32, bool)> = Vec::new();
        for head in heads.iter().filter(|h| h.voice == v) {
            for tone in &head.tones {
                let mut tick = tone.start;
                while tick < tone.end {
                    let until = tone.end.min((tick / MEASURE + 1) * MEASURE);
                    held.push((tick, until - tick, tick > tone.start));
                    tick = until;
                }
            }
            let (from, to) = head.silence();
            if from < to {
                held.push((from, to - from, true));
            }
        }
        held.extend(
            formulas
                .iter()
                .filter(|f| f.voice == v)
                .map(|f| (f.start, f.end - f.start, f.tied_in)),
        );
        held.push((total.saturating_sub(MEASURE), MEASURE, false));
        Capacity::new(total, &held, &self.levels)
    }

    /// C7 from a voice's head `previous` to its next entry `next`: what it costs to begin the
    /// syllables between them on the minim beats the voice has there, among the `heads` and
    /// `formulas` of a setting of `total` ticks, where the syllables of the points that begin in
    /// between begin only from there on (`gates`); TXT-ORDER where it cannot reach the entry's
    /// syllable at all, TXT-COMPLETE where they do not fit.
    pub(crate) fn reach(
        &self,
        previous: &Head,
        next: &Head,
        heads: &[&Head],
        formulas: &[FormulaNote],
        total: u32,
        gates: &Gates,
    ) -> Result<u32, &'static str> {
        let (Some(z), Some(y)) = (previous.last_syllable(), next.first_syllable()) else {
            return Ok(0);
        };
        let state = self.model.after_rest(Some(usize::from(z)));
        let way = self.model.way(&state, y).ok_or("TXT-ORDER")?;
        let capacity = self.capacity(next.voice, heads, formulas, total);
        let places = |from: u32| capacity.at(from).saturating_sub(capacity.at(next.start()));
        let mut cost = crowding(way.len() as u32, places(previous.end())).ok_or("TXT-COMPLETE")?;
        for (needed, from) in gated(&way, gates, previous.end()) {
            cost = cost.max(crowding(needed, places(from)).ok_or("TXT-COMPLETE")?);
        }
        Ok(cost)
    }

    /// How many syllables a voice has to begin from the end of a head on syllable `last` up to
    /// syllable `next`, which its next entry begins, if it can at all (`None` when the voice has
    /// gone past it into a later sentence), with the rest of the text after `last` for
    /// `next = None`.
    pub(crate) fn needed(&self, last: u16, next: Option<u16>) -> Option<u32> {
        let state = self.model.after_rest(Some(usize::from(last)));
        match next {
            Some(y) => self.model.to_reach(&state, y),
            None => Some(self.model.need(&state)),
        }
    }
}

/// What the gates ask of a voice that begins the syllables of `way` one after the other from
/// tick `from` on: for each syllable whose gate lies later, the syllables from it on, which all
/// begin at the gate or later, with the gate's tick.
pub(crate) fn gated(way: &[u16], gates: &Gates, from: u32) -> Vec<(u32, u32)> {
    way.iter()
        .enumerate()
        .filter_map(|(i, &y)| {
            let tick = gate(Some(gates), y);
            (tick > from).then_some(((way.len() - i) as u32, tick))
        })
        .collect()
}

/// What C7 charges for `needed` syllables on `places` minim beats: none up to four fifths of
/// them, one for each syllable beyond; `None` if they do not fit.
fn crowding(needed: u32, places: u32) -> Option<u32> {
    (needed <= places).then(|| needed.saturating_sub(places * 4 / 5))
}

/// Rests from `from` to `to`, a measure at a time.
fn rests(out: &mut Vec<Note>, from: u32, to: u32) {
    let mut tick = from;
    while tick < to {
        let until = to.min((tick / MEASURE + 1) * MEASURE);
        out.push(Note::rest(Duration::from_ticks(until - tick)));
        tick = until;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::Rng;
    use crate::imitation::test_support::{head, line};

    fn conflict(a: (u32, &str), b: (u32, &str)) -> Option<(Conflict, u32)> {
        let (a, b) = (line(a.0, a.1), line(b.0, b.1));
        let found = alone(&a, &b);
        // Which line is the leader does not matter.
        assert_eq!(alone(&b, &a), found);
        found
    }

    /// One constructed conflict for each pair check, and its consonant counterpart.
    #[test]
    fn pair_checks_find_certain_conflicts() {
        use Conflict::*;
        // a: B against B♭.
        assert_eq!(
            conflict((0, "D4.8 B4.8"), (4, "Bb3.8")),
            Some((Chromatic, 8))
        );
        // b: a second struck together; a third is fine.
        assert_eq!(conflict((0, "D4.8 E4.8"), (8, "F4.8")), Some((Struck, 8)));
        assert_eq!(conflict((0, "D4.8 F4.8"), (8, "D4.8")), None);
        // c: a fourth struck together on a downbeat; on the minim beat it is not certain.
        assert_eq!(conflict((0, "D4.8 D4.8"), (8, "A3.8")), Some((Fourth, 8)));
        assert_eq!(conflict((0, "D4.4 D4.8"), (4, "A3.8")), None);
        // d: a seventh struck on the downbeat under a held note whose line leaps on; resolved
        // by a step down it is a suspension.
        assert_eq!(
            conflict((0, "C5.16 G4.8"), (8, "D4.8")),
            Some((Unresolved, 8))
        );
        assert_eq!(conflict((0, "C5.16 B4.8"), (8, "D4.8 G4.8")), None);
        // A fourth below a held note: the held note must resolve.
        assert_eq!(
            conflict((0, "D4.16 E4.8"), (8, "A3.8")),
            Some((Unresolved, 8))
        );
        assert_eq!(conflict((0, "D4.16 C4.8"), (8, "A3.8 A3.8")), None);

        // e: an entry on the minim beat a second against a held note.
        assert_eq!(conflict((0, "D4.16"), (4, "E4.8")), Some((Offbeat, 4)));
        // g: an entry a fourth above the held bass.
        assert_eq!(conflict((0, "D4.16"), (4, "G4.8")), Some((OverBass, 4)));
        assert_eq!(conflict((0, "D4.16"), (4, "A4.8")), None);
        // h: parallel fifths from note to note; contrary motion is fine.
        assert_eq!(
            conflict((0, "D4.8 E4.8"), (0, "A4.8 B4.8")),
            Some((Parallel, 8))
        );
        assert_eq!(conflict((0, "D4.8 E4.8"), (0, "A4.8 G4.8")), None);
    }

    /// The pair checks as the subject builder's stretto test first ran them, before the checks
    /// of a whole choir shared their code: two lines alone, the lower note always the bass.
    fn alone_before(a: &[Tone], b: &[Tone]) -> Option<(Conflict, u32)> {
        let mut found: Option<(u32, Conflict)> = None;
        let mut note = |tick: u32, conflict: Conflict| {
            if found.is_none_or(|f| (tick, conflict) < f) {
                found = Some((tick, conflict));
            }
        };
        for (x_line, y_line) in [(a, b), (b, a)] {
            for (i, x) in x_line.iter().enumerate() {
                for y in y_line {
                    if x.start >= y.end || y.start >= x.end {
                        continue;
                    }
                    let tick = x.start.max(y.start);
                    if std::ptr::eq(x_line, a)
                        && x.pitch.step == y.pitch.step
                        && x.pitch.alter != y.pitch.alter
                    {
                        note(tick, Conflict::Chromatic);
                    }
                    if x.start == y.start {
                        if std::ptr::eq(x_line, a) {
                            if dissonant(x.pitch, y.pitch) {
                                note(tick, Conflict::Struck);
                            } else if between(x.pitch, y.pitch).is_perfect_fourth()
                                && tick.is_multiple_of(MEASURE)
                            {
                                note(tick, Conflict::Fourth);
                            }
                        }
                        continue;
                    }
                    if x.start > y.start {
                        continue;
                    }
                    let t = y.start;
                    let suspended = matches!(between(x.pitch, y.pitch).simple().steps, 1 | 6)
                        || (between(x.pitch, y.pitch).is_perfect_fourth()
                            && below(y.pitch, x.pitch));
                    if t.is_multiple_of(MEASURE) && suspended {
                        let next = x_line.get(i + 1).filter(|n| n.start == x.end);
                        if next.is_some_and(|n| n.pitch.diatonic() - x.pitch.diatonic() != -1) {
                            note(t, Conflict::Unresolved);
                        }
                    }
                    if std::ptr::eq(y, &y_line[0]) {
                        if !t.is_multiple_of(MEASURE) && dissonant(x.pitch, y.pitch) {
                            note(t, Conflict::Offbeat);
                        }
                        if fourth_above(x.pitch, y.pitch) {
                            note(t, Conflict::OverBass);
                        }
                    }
                }
            }
        }
        for (j, x2) in a.iter().enumerate().skip(1) {
            let x1 = a[j - 1];
            if x1.end != x2.start {
                continue;
            }
            let Some(k) = b.iter().position(|y| y.start == x2.start) else {
                continue;
            };
            if k == 0 || b[k - 1].end != b[k].start {
                continue;
            }
            let (y1, y2) = (b[k - 1], b[k]);
            let (dx, dy) = (
                x2.pitch.midi() - x1.pitch.midi(),
                y2.pitch.midi() - y1.pitch.midi(),
            );
            let (i1, i2) = (between(x1.pitch, y1.pitch), between(x2.pitch, y2.pitch));
            if dx != 0
                && dy != 0
                && dx.signum() == dy.signum()
                && i1.is_perfect_consonance()
                && i2.is_perfect_consonance()
                && i1.simple().steps == i2.simple().steps
            {
                note(x2.start, Conflict::Parallel);
            }
        }
        found.map(|(tick, conflict)| (conflict, tick))
    }

    /// A random line: an entry on a minim beat, then values of free counterpoint on the scale of
    /// the transposed Dorian mode, now and then with the flattened mi.
    fn random_line(rng: &mut Rng) -> Vec<Tone> {
        let mode: ModeSpec = "dorian-transposed".parse().unwrap();
        let mut tick = (rng.next_u64() % 12) as u32 * 4;
        let mut d = 26 + (rng.next_u64() % 10) as i32;
        let notes = 4 + rng.next_u64() % 6;
        (0..notes)
            .map(|_| {
                let ticks = [2, 4, 4, 6, 8, 8, 12, 16][(rng.next_u64() % 8) as usize];
                d += [-2, -1, -1, 0, 1, 1, 2, 3, -3, 4][(rng.next_u64() % 10) as usize];
                let mut pitch = mode.scale_pitch(d);
                if rng.next_u64().is_multiple_of(8) {
                    pitch = pitch.with_alter(pitch.alter - 1);
                }
                let tone = Tone {
                    pitch,
                    start: tick,
                    end: tick + ticks,
                };
                tick += ticks;
                tone
            })
            .collect()
    }

    /// The stretto test of the subject builder judges two lines exactly as it did before the
    /// checks of a whole choir shared its code.
    #[test]
    fn alone_is_unchanged() {
        let mut rng = Rng::new(13);
        let mut found = 0;
        for _ in 0..20_000 {
            let (a, b) = (random_line(&mut rng), random_line(&mut rng));
            let before = alone_before(&a, &b);
            assert_eq!(alone(&a, &b), before, "{a:?} {b:?}");
            found += usize::from(before.is_some());
        }
        assert!(found > 5_000, "{found} conflicts");
    }

    /// The first conflict of heads `a` and `b` among `others` in `choir`.
    fn clash(choir: &[VoiceType], a: &Head, b: &Head, others: &[Head]) -> Option<(Conflict, u32)> {
        let mut heads: Vec<&Head> = vec![a, b];
        heads.extend(others);
        let frame = Frame {
            choir,
            heads: &heads,
        };
        let found = frame.clash(a, b).map(|c| (c.conflict, c.tick));
        assert_eq!(frame.clash(b, a).map(|c| (c.conflict, c.tick)), found);
        found
    }

    /// One constructed conflict for each pair check (C5 a–h) between the heads of a choir, and a
    /// counterpart the free voices can still make good: the lowest voice, the voices that can
    /// sound below a fourth, the passing quarter, a dissonance reached by a leap.
    #[test]
    fn pair_checks_find_constructed_conflicts() {
        use Conflict::*;
        use VoiceType::*;
        let satb = [Soprano, Alto, Tenor, Bass];
        let atb = [Alto, Tenor, Bass];
        // The tenor silent until it enters in the fourth measure.
        let tenor_later = head(2, 24, 0, "D3.8 E3.8 F3.8 D3.8");
        // a: B♮ against B♭.
        assert_eq!(
            clash(
                &satb,
                &head(0, 0, 0, "D5.8 B4.8"),
                &head(1, 4, 0, "Bb4.8"),
                &[]
            ),
            Some((Chromatic, 8))
        );
        // b: a second struck together, also between upper voices.
        assert_eq!(
            clash(
                &satb,
                &head(0, 0, 0, "D5.8 E5.8"),
                &head(1, 8, 0, "F4.8"),
                &[]
            ),
            Some((Struck, 8))
        );
        // A passing quarter struck off the downbeat against a longer note: from three voices
        // on the longer note does not answer for it; with two both do.
        let passing = head(0, 0, 0, "D5.4 C5.2 B4.2 A4.8");
        let held = head(1, 4, 0, "D4.4 F4.8");
        assert_eq!(clash(&satb, &passing, &held, &[]), None);
        assert_eq!(
            clash(&[Soprano, Alto], &passing, &held, &[]),
            Some((Struck, 4))
        );
        // c: a fourth struck on the downbeat over the lowest voice, or over another voice where
        // nothing can sound below it; where a free voice can, it is not certain.
        let upper = head(1, 0, 0, "A3.8 D4.8");
        assert_eq!(
            clash(&satb, &upper, &head(3, 8, 0, "A2.8"), &[]),
            Some((Fourth, 8))
        );
        assert_eq!(clash(&satb, &upper, &head(2, 8, 0, "A3.8"), &[]), None);
        assert_eq!(
            clash(
                &atb,
                &head(0, 0, 0, "A3.8 D4.8"),
                &head(1, 8, 0, "A3.8"),
                &[]
            ),
            None
        );
        let silent_bass = head(2, 32, 0, "D3.8 E3.8 F3.8 D3.8");
        assert_eq!(
            clash(
                &atb,
                &head(0, 0, 0, "A3.8 D4.8"),
                &head(1, 8, 0, "A3.8"),
                std::slice::from_ref(&silent_bass)
            ),
            Some((Fourth, 8))
        );
        // d: a seventh struck on the downbeat against a held note that leaps on; stepping down
        // it is a suspension.
        assert_eq!(
            clash(
                &satb,
                &head(0, 0, 0, "C5.16 G4.8"),
                &head(1, 8, 0, "D4.8"),
                &[]
            ),
            Some((Unresolved, 8))
        );
        assert_eq!(
            clash(
                &satb,
                &head(0, 0, 0, "C5.16 B4.8"),
                &head(1, 8, 0, "D4.8 G4.8"),
                &[]
            ),
            None
        );
        // d: a fourth struck above the tenor's held A3, the bass still silent: the tenor leaps
        // on; stepping down, it is free (IMI-HELD).
        assert_eq!(
            clash(
                &atb,
                &head(0, 4, 0, "C4.4 D4.8"),
                &head(1, 0, 0, "A3.16 C4.8"),
                std::slice::from_ref(&silent_bass)
            ),
            Some((UnderFourth, 8))
        );
        assert_eq!(
            clash(
                &atb,
                &head(0, 4, 0, "C4.4 D4.8"),
                &head(1, 0, 0, "A3.16 G3.8"),
                std::slice::from_ref(&silent_bass)
            ),
            None
        );
        // e: an entry on the minim beat a second against a held note; within a line a seventh
        // reached by a leap; reached by a step it may pass on.
        assert_eq!(
            clash(&satb, &head(0, 0, 0, "D5.16"), &head(1, 4, 0, "E4.8"), &[]),
            Some((Offbeat, 4))
        );
        let soprano = head(0, 48, 0, "E4.8 G4.8");
        assert_eq!(
            clash(&satb, &soprano, &head(2, 40, 0, "G3.8 A3.4 F3.4"), &[]),
            Some((Offbeat, 52))
        );
        assert_eq!(
            clash(&satb, &soprano, &head(2, 40, 0, "A3.8 G3.4 F3.4"), &[]),
            None
        );
        // g: an entry a fourth above a held note of the lowest voice; above a middle voice a
        // free bass may still sound below it.
        assert_eq!(
            clash(&satb, &head(3, 0, 0, "D3.16"), &head(1, 4, 0, "G3.8"), &[]),
            Some((OverBass, 4))
        );
        assert_eq!(
            clash(&satb, &head(2, 0, 0, "D3.16"), &head(1, 4, 0, "G3.8"), &[]),
            None
        );
        // h: parallel fifths from note to note.
        assert_eq!(
            clash(
                &atb,
                &head(0, 0, 0, "A3.8 B3.8"),
                &head(1, 0, 0, "D3.8 E3.8"),
                &[tenor_later]
            ),
            Some((Parallel, 8))
        );
        assert_eq!(
            clash(
                &atb,
                &head(0, 0, 0, "A3.8 B3.8"),
                &head(1, 0, 0, "D3.8 G3.8"),
                &[]
            ),
            None
        );
        // f: an entry against a formula note in whatever octave: dissonant, or a fourth above the
        // lowest voice's; consonant it is fine.
        let formula = |voice: usize, pitch: &str| FormulaNote {
            voice,
            pitch: pitch.parse().unwrap(),
            start: 48,
            end: 56,
            tied_in: false,
        };
        let goal = [formula(0, "A4"), formula(3, "A4")];
        assert_eq!(
            against_formulas(&head(1, 52, 4, "B3.8"), &goal, 3),
            Some(52)
        );
        assert_eq!(
            against_formulas(&head(1, 52, 4, "D4.8"), &goal, 3),
            Some(52)
        );
        assert_eq!(against_formulas(&head(1, 52, 4, "C4.8"), &goal, 3), None);
        assert_eq!(against_formulas(&head(1, 60, 4, "B3.8"), &goal, 3), None);
    }

    /// The measures that an entry's silence and head touch fit the rhythms of free counterpoint:
    /// a first entry on the minim beat, a re-entry after a minim's rest on the downbeat; not two
    /// eighths at a downbeat followed by a half.
    #[test]
    fn entries_fit_measure_rhythms() {
        let rhythms = Rhythms::new(&Checker::new(&Profile::fux()));
        assert!(rhythms.fit(&head(0, 12, 0, "D4.8 E4.4 F4.4 G4.8")));
        assert!(rhythms.fit(&head(0, 40, 4, "D4.8 E4.4 F4.4 G4.8")));
        assert!(rhythms.fit(&head(0, 44, 4, "D4.4 E4.6 F4.2 G4.8")));
        assert!(!rhythms.fit(&head(0, 40, 4, "D4.1 E4.1 F4.4 G4.2")));
        // Where the profile allows every rhythm, every head fits.
        let mut free = Profile::fux();
        free.apply_override("FREE-RHYTHM=aus").unwrap();
        let anything = Rhythms::new(&Checker::new(&free));
        assert!(anything.fit(&head(0, 40, 4, "D4.1 E4.1 F4.4 G4.2")));
    }
}
