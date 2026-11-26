//! A beam search for the free voices over a given harmony.
//!
//! Given are, per voice and slot of a grid of sixteenths, what happens (a fixed tone, a free
//! onset, a continuation, silence) and, per slot, the chord and key. The search chooses the
//! pitch of every free onset. A tone foreign to the chord must be one of the figures of
//! `figures` (passing tone, neighbour, suspension, appoggiatura and their relatives), which the
//! state of a voice carries as a duty for its next tone; parallels, crossings and clashes of
//! two foreign tones are forbidden, the rest is priced.

use std::collections::HashSet;

use crate::key::{Chord, Harmony, Key};
use crate::melody::{self, Wish};
use crate::pitch::Pitch;
use crate::rng::Rng;
use crate::time::{Meter, SIXTEENTH, Tick};

pub const MAX_VOICES: usize = 6;

/// What a voice does in one slot.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Ev {
    Rest,
    Sustain,
    Onset,
    Fixed(Pitch),
    FixedSustain,
    FixedRest,
}

impl Ev {
    pub const fn is_fixed(self) -> bool {
        matches!(self, Ev::Fixed(_) | Ev::FixedSustain | Ev::FixedRest)
    }

    pub const fn sounds(self) -> bool {
        !matches!(self, Ev::Rest | Ev::FixedRest)
    }
}

#[derive(Clone, Debug)]
pub struct VoicePlan {
    pub low: Pitch,
    pub high: Pitch,
    pub events: Vec<Ev>,
    /// For free onsets: the interval the line would like to make (empty: no wishes).
    pub wishes: Vec<Option<Wish>>,
}

/// Prices of what is allowed but not free. One unit is the smallest preference.
#[derive(Clone, Debug)]
pub struct Costs {
    /// Per class of interval away from the wished one, at most twice.
    pub off_class: u32,
    /// The line turns where it wished to go on, or the other way round.
    pub off_turn: u32,
    pub repeat: u32,
    pub third: u32,
    pub fourth: u32,
    pub sixth: u32,
    pub octave: u32,
    pub seventh: u32,
    pub altered: u32,
    pub chromatic: u32,
    pub leap_on: u32,
    pub arpeggio: u32,
    pub no_recovery: u32,
    pub back: u32,
    pub trill: u32,
    pub seventh_leaves: u32,
    pub leading_leaves: u32,
    pub edge: u32,
    pub accented: u32,
    pub appoggiatura: u32,
    pub escape: u32,
    pub group: u32,
    pub long_foreign: u32,
    pub lying_on: u32,
    pub crossing: u32,
    pub direct: u32,
    pub unison: u32,
    pub doubled: u32,
    pub harsh: u32,
    pub no_third: u32,
    pub six_four: u32,
    pub wide: u32,
    pub forced: u32,
    pub excused: u32,
    pub far: u32,
    /// Two tones struck together on a beat form a dissonance (the HAR-CONSONANCE of this style).
    pub struck_dissonance: u32,
    /// A tone foreign to the chord is struck again.
    pub restrike: u32,
}

impl Default for Costs {
    fn default() -> Costs {
        Costs {
            off_class: 2,
            off_turn: 1,
            repeat: 3,
            third: 1,
            fourth: 2,
            sixth: 4,
            octave: 4,
            seventh: 8,
            altered: 0,
            chromatic: 3,
            leap_on: 4,
            arpeggio: 1,
            no_recovery: 1,
            back: 1,
            trill: 3,
            seventh_leaves: 4,
            leading_leaves: 2,
            edge: 1,
            accented: 2,
            appoggiatura: 2,
            escape: 3,
            group: 1,
            long_foreign: 4,
            lying_on: 1,
            crossing: 4,
            direct: 2,
            unison: 1,
            doubled: 4,
            harsh: 2,
            no_third: 2,
            six_four: 3,
            wide: 1,
            forced: 20,
            excused: 6,
            far: 1,
            struck_dissonance: 1,
            restrike: 2,
        }
    }
}

pub struct Task<'a> {
    pub meter: Meter,
    /// Tick of the first barline, for the metric weights.
    pub origin: Tick,
    /// Tick of slot 0.
    pub start: Tick,
    pub harmony: &'a Harmony,
    pub voices: Vec<VoicePlan>,
    pub beam: usize,
    pub seed: u64,
    pub costs: Costs,
    /// Pairs of voices (upper, lower) that must stand in double counterpoint at the octave from
    /// one slot to another: on beats no fifth or fourth between them, and not more than an
    /// octave apart.
    pub invertible: Vec<(usize, usize, usize, usize)>,
    /// At this slot the lowest voice takes the root of the chord.
    pub root_at: Option<usize>,
    /// Parallels are also refused between two fixed tones if a free tone leads into one of
    /// them (off in the re-settings of T6, which were counted without it).
    pub junctions: bool,
}

/// How the sounding tone of a voice was reached.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Arrive {
    /// After a rest or at the beginning.
    Start,
    /// The same pitch as before, struck again or held into a new chord.
    Repeat,
    Step(i8),
    /// Direction, and whether it was a fourth or more.
    Leap(i8, bool),
}

/// What a second foreign tone in a row still owes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Chain {
    None,
    /// A second passing tone: one more step in this direction to a chord tone.
    Pass(i8),
    /// The second neighbour of a group: a step in this direction back to this diatonic pitch.
    Group(i16, i8),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Line {
    cur: Option<Pitch>,
    /// The tone before `cur` without a rest between.
    prev: Option<Pitch>,
    /// The last tone that sounded, across rests.
    last: Option<Pitch>,
    /// 0 a chord tone, 1 a foreign tone, 2 the second foreign tone in a row.
    foreign: u8,
    arrive: Arrive,
    chain: Chain,
    /// `cur` is the seventh of its chord.
    seventh: bool,
    /// How often in a row the voice has returned to the tone before the last.
    turns: u8,
    /// `cur` is a fixed tone: what it owes as a foreign tone does not bind a free tone after it.
    fixed: bool,
}

const SILENT: Line = Line {
    cur: None,
    prev: None,
    last: None,
    foreign: 0,
    arrive: Arrive::Start,
    chain: Chain::None,
    seventh: false,
    turns: 0,
    fixed: false,
};

struct SlotInfo {
    chord: Option<Chord>,
    mask: u16,
    key: Key,
    weight: u8,
    /// The chord differs from the one of the slot before.
    changed: bool,
    dominant: bool,
}

impl SlotInfo {
    fn has(&self, pitch: Pitch) -> bool {
        self.chord.is_none() || self.mask & (1 << pitch.pc()) != 0
    }
}

#[derive(Clone)]
struct State {
    lines: [Line; MAX_VOICES],
    cost: u32,
    forced: Count,
    /// Cost plus what the tones still have to travel to the next fixed entries.
    rank: u32,
    tie: u32,
}

/// A state while the voices of one slot are placed: lines, cost, forced, what was placed.
type Partial = ([Line; MAX_VOICES], u32, Count, [Option<Pitch>; MAX_VOICES]);

#[derive(Clone, Copy)]
struct Back {
    parent: u32,
    placed: [Option<Pitch>; MAX_VOICES],
}

/// The result of a search: per voice the onsets as (slot, pitch), fixed tones included.
#[derive(Clone, Debug)]
pub struct Found {
    pub onsets: Vec<Vec<(usize, Pitch)>>,
    pub cost: u32,
    pub count: Count,
}

#[derive(Clone, Debug)]
pub enum Outcome {
    Found(Found),
    /// No candidate survived this slot; with it, how often each rule turned a candidate down
    /// there.
    DeadEnd(usize, Vec<(&'static str, usize)>),
}

/// Why a candidate is turned down.
type Refused = &'static str;

/// Rules that gave way to a fixed tone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Count {
    /// A fixed tone after a free one broke a rule that binds the free tones: the free voice did
    /// not lead into the entry.
    pub forced: u32,
    /// A fixed tone after a fixed tone or a rest broke such a rule: the given line itself does
    /// what the figures do not explain against the given chords.
    pub own: u32,
    /// A free tone did not resolve the foreign fixed tone before it.
    pub excused: u32,
}

struct Search<'a> {
    task: &'a Task<'a>,
    slots: Vec<SlotInfo>,
    /// Per voice and slot: for an onset, how many slots its tone lasts.
    lengths: Vec<Vec<u16>>,
    /// Per voice and slot: the next fixed tone within two beats, as (slots ahead, pitch).
    targets: Vec<Vec<Option<(u16, Pitch)>>>,
    count: usize,
}

impl Search<'_> {
    fn costs(&self) -> &Costs {
        &self.task.costs
    }

    /// The pitches written on the diatonic step `dia` that the key or the chord offer.
    fn spellings(&self, dia: i16, info: &SlotInfo, out: &mut Vec<Pitch>) {
        let step = dia.rem_euclid(7) as i8;
        let octave = dia.div_euclid(7) as i8;
        let degree = (step - info.key.tonic.step()).rem_euclid(7) as usize;
        let mut push = |class: crate::pitch::Lof| {
            if class.step() == step {
                let p = Pitch::new(step, class.alter(), octave);
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        };
        push(info.key.degree(degree));
        push(info.key.raised(degree));
        if let Some(chord) = info.chord {
            for tone in chord.tones() {
                push(tone);
            }
        }
    }

    fn candidates(&self, v: usize, line: &Line, s: usize, out: &mut Vec<Pitch>) {
        let info = &self.slots[s];
        out.clear();
        let plan = &self.task.voices[v];
        let centre = Pitch {
            midi: ((plan.low.midi as i32 + plan.high.midi as i32) / 2) as i16,
            dia: ((plan.low.dia as i32 + plan.high.dia as i32) / 2) as i16,
        };
        let from = line.cur.or(line.last).unwrap_or(centre);
        let reach = if line.cur.is_some() { 12 } else { 9 };
        let in_range = |p: Pitch| p.midi >= plan.low.midi && p.midi <= plan.high.midi;
        // Chord tones within reach.
        let mut chord_tones: Vec<Pitch> = Vec::new();
        if let Some(chord) = info.chord {
            for tone in chord.tones() {
                for octave in 0..9 {
                    let p = Pitch::new(tone.step(), tone.alter(), octave);
                    if in_range(p) && (p.midi - from.midi).abs() <= reach {
                        chord_tones.push(p);
                    }
                }
            }
        } else {
            // Before the first chord: the tones of the key.
            for dia in from.dia - 7..=from.dia + 7 {
                self.spellings(dia, info, &mut chord_tones);
            }
            chord_tones.retain(|p| in_range(*p));
        }
        out.extend(&chord_tones);
        if let Some(y) = line.cur {
            // Steps, the same tone, and the third across for a group of neighbours.
            for d in [-2i16, -1, 0, 1, 2] {
                self.spellings(y.dia + d, info, out);
            }
            // The semitone below, as a chromatic neighbour.
            let below = Pitch {
                midi: y.midi - 1,
                dia: y.dia - 1,
            };
            if !out.contains(&below) && y.to(below).is_step() {
                out.push(below);
            }
        }
        if info.weight >= 1 && line.foreign == 0 {
            // Appoggiaturas: the neighbours of the chord tones nearby.
            let mut near: Vec<Pitch> = Vec::new();
            for c in &chord_tones {
                if (c.midi - from.midi).abs() <= 9 {
                    self.spellings(c.dia + 1, info, &mut near);
                    self.spellings(c.dia - 1, info, &mut near);
                }
            }
            for p in near {
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
        if self.task.root_at == Some(s)
            && v + 1 == self.count
            && let Some(chord) = info.chord
        {
            out.retain(|p| p.pc() == chord.root.pc());
        }
        // No spelling further from the key than the chromatic neighbours of its scale.
        let centre = info.key.tonic.0 - if info.key.minor { 3 } else { 0 };
        out.retain(|p| in_range(*p) && (centre - 6..=centre + 10).contains(&p.lof().0));
    }

    /// The price of the interval from `y` to `z` in a voice, `None` if it is forbidden.
    fn melodic(
        &self,
        line: &Line,
        y: Pitch,
        z: Pitch,
        z_chord: bool,
        info: &SlotInfo,
        wish: Option<Wish>,
    ) -> Result<u32, Refused> {
        let c = self.costs();
        let b = y.to(z);
        let mut cost = 0;
        // With a wish the size of the interval is paid by its distance from the wish, without one
        // by the fixed prices of the sizes.
        let sized = wish.is_none();
        if let Some(wish) = wish {
            let away = (melody::class(b) as i32 - wish.class as i32).unsigned_abs();
            cost += c.off_class * away.min(2);
            if let Some(x) = line.prev {
                let before = x.to(y).dir();
                if before != 0 && b.dir() != 0 && (before != b.dir()) != wish.turn {
                    cost += c.off_turn;
                }
            }
        }
        if b.is_unison() {
            if sized {
                cost += c.repeat;
            }
        } else if b.is_step() {
            if b.steps == 0 {
                cost += c.chromatic;
            }
        } else {
            if b.semis.abs() > 12 || b.steps == 0 {
                return Err("leap beyond the octave");
            }
            let both_chord = z_chord && info.has(y);
            if b.is_altered() {
                // Tritones and diminished leaps only inside a dominant chord.
                if !(both_chord && info.dominant) {
                    return Err("augmented or diminished leap");
                }
                cost += c.altered;
            } else {
                let price = match b.steps.abs() {
                    2 => c.third,
                    3 | 4 => c.fourth,
                    5 => c.sixth,
                    6 if both_chord => c.seventh,
                    7 => c.octave,
                    _ => return Err("leap of a seventh"),
                };
                if sized || b.steps.abs() == 6 {
                    cost += price;
                }
            }
            if let Arrive::Leap(d, _) = line.arrive
                && d == b.dir()
            {
                cost += if both_chord { c.arpeggio } else { c.leap_on };
            }
        }
        if let Arrive::Leap(d, true) = line.arrive
            && !(b.is_step() && b.dir() == -d)
        {
            cost += c.no_recovery;
        }
        if Some(z) == line.prev && !b.is_unison() {
            cost += c.back + if line.turns >= 1 { c.trill } else { 0 };
        }
        if line.seventh && !(b.is_step() && b.dir() < 0) && !b.is_unison() {
            cost += c.seventh_leaves;
        }
        // The leading tone of a dominant that gives way to another chord goes up a step.
        if y.lof() == info.key.raised(6)
            && line.foreign == 0
            && info.changed
            && !info.dominant
            && !(b.is_step() && b.dir() > 0)
        {
            cost += c.leading_leaves;
        }
        Ok(cost)
    }

    /// Voice `v` strikes `z` at slot `s`. Returns its new state and the price, or `None` if the
    /// tone is forbidden. A fixed tone is never forbidden: what it breaks is counted and priced.
    fn place(
        &self,
        v: usize,
        line: &Line,
        z: Pitch,
        s: usize,
        fixed: bool,
    ) -> Result<(Line, u32, Count), Refused> {
        let c = self.costs();
        let info = &self.slots[s];
        let plan = &self.task.voices[v];
        let z_chord = info.has(z);
        let mut cost = 0;
        let mut excused = 0;
        let mut why: Option<Refused> = None;
        let mut refuse = |reason: Refused, holds: bool| {
            if !holds && why.is_none() {
                why = Some(reason);
            }
        };
        let mut foreign = 0;
        let mut chain = Chain::None;
        let arrive;
        match line.cur {
            Some(y) => {
                let b = y.to(z);
                arrive = if b.is_unison() {
                    Arrive::Repeat
                } else if b.is_step() {
                    Arrive::Step(b.dir())
                } else {
                    Arrive::Leap(b.dir(), b.steps.abs() >= 3)
                };
                let wish = if fixed {
                    None
                } else {
                    plan.wishes.get(s).copied().flatten()
                };
                match self.melodic(line, y, z, z_chord, info, wish) {
                    Ok(price) => cost += price,
                    Err(reason) => refuse(reason, false),
                }
                if line.foreign > 0 {
                    // What the foreign tone before owes.
                    let step_in = |d: i8| b.is_step() && b.dir() == d;
                    let paid = match (line.foreign, line.arrive, line.chain) {
                        (2, _, Chain::Pass(d)) => step_in(d) && z_chord,
                        (2, _, Chain::Group(dia, d)) => step_in(d) && z.dia == dia && z_chord,
                        (_, Arrive::Step(d), _) => {
                            if step_in(d) {
                                if !z_chord {
                                    foreign = 2;
                                    chain = Chain::Pass(d);
                                }
                                true
                            } else if step_in(-d) {
                                z_chord && line.prev.is_some_and(|x| x.dia == z.dia)
                            } else if b.steps == -2 * d as i16 && !z_chord {
                                match line.prev {
                                    Some(x) => {
                                        foreign = 2;
                                        chain = Chain::Group(x.dia, d);
                                        cost += c.group;
                                        true
                                    }
                                    None => false,
                                }
                            } else if z_chord && !b.is_step() && !b.is_unison() {
                                // An escape tone, or a cambiata by a third in the same direction.
                                cost += c.escape;
                                b.dir() == -d || b.steps.abs() == 2
                            } else {
                                false
                            }
                        }
                        (_, Arrive::Leap(..) | Arrive::Start, _) => b.is_step() && z_chord,
                        (_, Arrive::Repeat, _) => {
                            if b.is_unison() {
                                if !z_chord {
                                    foreign = 1;
                                    cost += c.restrike;
                                }
                                true
                            } else {
                                // A suspension resolves by step, downwards for choice.
                                if b.dir() > 0 {
                                    cost += 1;
                                }
                                b.is_step() && z_chord
                            }
                        }
                    };
                    if !paid && line.fixed && !fixed {
                        // The fixed tone before is one the figures do not explain.
                        excused = 1;
                        cost += c.excused;
                        foreign = if z_chord { 0 } else { 1 };
                        chain = Chain::None;
                    } else {
                        refuse("foreign tone not resolved", paid);
                    }
                } else if !z_chord {
                    foreign = 1;
                    match arrive {
                        Arrive::Repeat => cost += c.restrike,
                        Arrive::Step(_) => {
                            // On a beat a foreign tone costs, on a strong beat more.
                            cost += match info.weight {
                                0 | 1 => 0,
                                2 => c.accented,
                                _ => c.accented + c.accented / 2,
                            };
                        }
                        Arrive::Leap(..) | Arrive::Start => {
                            cost += c.appoggiatura;
                            refuse("appoggiatura off the beat", info.weight >= 1);
                        }
                    }
                }
            }
            None => {
                arrive = Arrive::Start;
                if !z_chord {
                    foreign = 1;
                    cost += c.appoggiatura;
                    refuse("appoggiatura off the beat", info.weight >= 1);
                }
                if let Some(last) = line.last {
                    let far = (last.midi - z.midi).abs();
                    if far > 12 {
                        refuse("entry far from the last tone", false);
                    } else if far > 7 {
                        cost += c.fourth;
                    }
                }
            }
        }
        if foreign > 0 {
            // A foreign tone is short and does not stand before a rest.
            let length = self.lengths[v][s] as Tick * SIXTEENTH;
            if length > self.task.meter.beat() {
                cost += c.long_foreign;
                refuse("long foreign tone", length <= 2 * self.task.meter.beat());
            }
        }
        // No tone is foreign to the chord when its voice falls silent.
        let after = s + self.lengths[v][s] as usize;
        if plan.events.get(after).is_none_or(|e| !e.sounds()) {
            refuse("foreign tone before a rest", self.slots[after - 1].has(z));
        }
        if z.midi - plan.low.midi < 2 || plan.high.midi - z.midi < 2 {
            cost += c.edge;
        }
        refuse(
            "out of range",
            z.midi >= plan.low.midi && z.midi <= plan.high.midi,
        );
        let mut forced = 0;
        let mut own = 0;
        if let Some(reason) = why {
            if !fixed {
                return Err(reason);
            }
            if line.fixed || line.cur.is_none() {
                own = 1;
            } else {
                forced = 1;
            }
            cost = c.forced;
            if !z_chord && foreign == 0 {
                foreign = 1;
            }
        }
        let seventh = z_chord && info.chord.is_some_and(|ch| ch.member(z.pc()) == Some(3));
        let turns = if Some(z) == line.prev && line.cur != Some(z) {
            line.turns.saturating_add(1)
        } else {
            0
        };
        Ok((
            Line {
                cur: Some(z),
                prev: line.cur,
                last: Some(z),
                foreign,
                arrive,
                chain,
                seventh,
                turns,
                fixed,
            },
            cost,
            Count {
                forced,
                own,
                excused,
            },
        ))
    }

    /// The price of two voices against each other after slot `s`, `None` if forbidden. `upper`
    /// is the voice written above. Only pairs in which a free voice strikes are judged.
    #[allow(clippy::too_many_arguments)]
    fn pair(
        &self,
        upper: usize,
        lower: usize,
        before: &[Line; MAX_VOICES],
        after: &[Line; MAX_VOICES],
        struck: u8,
        free: u8,
        s: usize,
    ) -> Result<u32, Refused> {
        let c = self.costs();
        let info = &self.slots[s];
        let (Some(a), Some(b)) = (after[upper].cur, after[lower].cur) else {
            return Ok(0);
        };
        let bit = |v: usize| struck & (1 << v) != 0;
        if free & ((1 << upper) | (1 << lower)) == 0 {
            // Two fixed tones: only the way a free tone leads into them can still be judged.
            let led = |v: usize| before[v].cur.is_some() && !before[v].fixed;
            if self.task.junctions
                && bit(upper)
                && bit(lower)
                && (led(upper) || led(lower))
                && let (Some(a0), Some(b0)) = (before[upper].cur, before[lower].cur)
            {
                let (now, then) = (b.to(a), b0.to(a0));
                let (da, db) = (a0.to(a).dir(), b0.to(b).dir());
                if now.is_perfect()
                    && then.is_perfect()
                    && now.simple() == then.simple()
                    && da != 0
                    && da == db
                {
                    return Err("parallel fifths or octaves");
                }
            }
            return Ok(0);
        }
        let both_free = free & (1 << upper) != 0 && free & (1 << lower) != 0;
        let mut cost = 0;
        if self
            .task
            .invertible
            .iter()
            .any(|(u, l, from, until)| *u == upper && *l == lower && (*from..*until).contains(&s))
        {
            let simple = b.to(a).simple();
            if info.weight >= 2 && (simple == (4, 7) || simple == (3, 5)) {
                return Err("fifth in double counterpoint");
            }
            if a.midi - b.midi > 12 || a.midi < b.midi {
                return Err("double counterpoint wider than an octave");
            }
        }
        if a.midi < b.midi {
            // Against a fixed voice a crossing only costs: the fixed voices may cross themselves
            // and leave no room between them.
            let depth = (b.midi - a.midi) as u32;
            if depth > 4 && both_free {
                return Err("crossing");
            }
            cost += c.crossing * (1 + depth / 3);
        }
        let now = b.to(a);
        if bit(upper) && bit(lower) {
            if let (Some(a0), Some(b0)) = (before[upper].cur, before[lower].cur) {
                let then = b0.to(a0);
                let (da, db) = (a0.to(a).dir(), b0.to(b).dir());
                if now.is_perfect() && da != 0 && da == db {
                    if then.is_perfect() && now.simple() == then.simple() {
                        return Err("parallel fifths or octaves");
                    }
                    // A perfect interval reached in similar motion with a leap above.
                    if !a0.to(a).is_step() {
                        cost += c.direct;
                    }
                }
            }
            if after[upper].foreign > 0 && after[lower].foreign > 0 && !now.is_consonant(true) {
                return Err("two foreign tones clash");
            }
            if info.weight >= 2 {
                let lowest = after[..self.count].iter().filter_map(|l| l.cur).min();
                if !now.is_consonant(Some(b) != lowest) {
                    cost += c.struck_dissonance;
                }
            }
            if a.midi == b.midi {
                cost += c.unison;
            }
        }
        if a.pc() == b.pc() {
            let leading = info.key.raised(6).pc() == a.pc() && info.dominant;
            if leading || (after[upper].seventh && after[lower].seventh) {
                cost += c.doubled;
            }
            if after[upper].foreign > 0 && after[lower].foreign > 0 {
                cost += c.doubled;
            }
        }
        // A struck foreign tone a semitone away from a chord tone.
        let semis = (a.midi - b.midi).rem_euclid(12);
        if (semis == 1 || semis == 11)
            && ((bit(upper) && after[upper].foreign > 0)
                || (bit(lower) && after[lower].foreign > 0))
        {
            cost += c.harsh;
        }
        Ok(cost)
    }

    /// The price of the whole sound on a beat on which a free voice strikes.
    fn sound(&self, after: &[Line; MAX_VOICES], info: &SlotInfo) -> u32 {
        let c = self.costs();
        let Some(chord) = info.chord else {
            return 0;
        };
        let sounding: Vec<Pitch> = after[..self.count].iter().filter_map(|l| l.cur).collect();
        if sounding.len() < 3 {
            return 0;
        }
        let mut cost = 0;
        let has = |member: usize| {
            chord
                .tones()
                .nth(member)
                .is_some_and(|t| sounding.iter().any(|p| p.pc() == t.pc()))
        };
        if !has(1) {
            cost += c.no_third;
        }
        let lowest = sounding.iter().min().expect("three sounding");
        if !chord.kind.is_seventh() && chord.member(lowest.pc()) == Some(2) {
            cost += c.six_four;
        }
        let mut sorted = sounding.clone();
        sorted.sort();
        if sorted
            .windows(2)
            .skip(1)
            .any(|w| w[1].midi - w[0].midi > 19)
        {
            cost += c.wide;
        }
        cost
    }

    fn run(&self) -> Outcome {
        let n = self.count;
        let mut rng = Rng::new(self.task.seed);
        let mut beam: Vec<State> = vec![State {
            lines: [SILENT; MAX_VOICES],
            cost: 0,
            forced: Count::default(),
            rank: 0,
            tie: 0,
        }];
        let mut backs: Vec<Vec<Back>> = Vec::with_capacity(self.slots.len());
        let mut candidates: Vec<Pitch> = Vec::new();
        for s in 0..self.slots.len() {
            let info = &self.slots[s];
            let events: Vec<Ev> = (0..n).map(|v| self.task.voices[v].events[s]).collect();
            let mut struck = 0u8;
            let mut free = 0u8;
            // Fixed voices first, then the free ones from the bottom up.
            let mut order: Vec<usize> = Vec::new();
            for (v, event) in events.iter().enumerate() {
                if matches!(event, Ev::Fixed(_)) {
                    order.push(v);
                    struck |= 1 << v;
                }
            }
            for v in (0..n).rev() {
                if events[v] == Ev::Onset {
                    order.push(v);
                    struck |= 1 << v;
                    free |= 1 << v;
                }
            }
            let mut next: Vec<(State, Back)> = Vec::new();
            let mut refused: std::cell::RefCell<Vec<(Refused, usize)>> = Default::default();
            let note = |reason: Refused| {
                let mut list = refused.borrow_mut();
                match list.iter_mut().find(|(r, _)| *r == reason) {
                    Some(entry) => entry.1 += 1,
                    None => list.push((reason, 1)),
                }
            };
            for (index, state) in beam.iter().enumerate() {
                let mut lines = state.lines;
                let mut cost = state.cost;
                let mut forced = state.forced;
                let mut alive = true;
                for v in 0..n {
                    match events[v] {
                        Ev::Rest | Ev::FixedRest => {
                            if lines[v].cur.is_some() && lines[v].foreign > 0 {
                                if events[v] == Ev::FixedRest {
                                    if lines[v].fixed {
                                        forced.own += 1;
                                    } else {
                                        forced.forced += 1;
                                    }
                                    cost += self.costs().forced;
                                } else {
                                    alive = false;
                                    note("rest after a foreign tone");
                                }
                            }
                            lines[v] = Line {
                                last: lines[v].last,
                                ..SILENT
                            };
                        }
                        Ev::Sustain | Ev::FixedSustain => {
                            if let Some(y) = lines[v].cur
                                && info.changed
                            {
                                let inside = info.has(y);
                                if lines[v].foreign == 0 && !inside {
                                    // A held tone becomes a suspension.
                                    lines[v].foreign = 1;
                                    lines[v].arrive = Arrive::Repeat;
                                    lines[v].chain = Chain::None;
                                } else if lines[v].foreign > 0 && inside {
                                    lines[v].foreign = 0;
                                    lines[v].chain = Chain::None;
                                } else if lines[v].foreign > 0 {
                                    cost += self.costs().lying_on;
                                }
                                lines[v].seventh = false;
                            }
                        }
                        _ => {}
                    }
                }
                if !alive {
                    continue;
                }
                // Partial states of this parent: lines, cost, forced, what was placed.
                let mut partial: Vec<Partial> = vec![(lines, cost, forced, [None; MAX_VOICES])];
                let mut decided: u8 = !struck;
                for &v in &order {
                    let mut grown: Vec<Partial> = Vec::new();
                    for (lines, cost, forced, placed) in &partial {
                        let try_place = |z: Pitch, fixed: bool, grown: &mut Vec<Partial>| {
                            let (line, price, f) = match self.place(v, &lines[v], z, s, fixed) {
                                Ok(placed) => placed,
                                Err(reason) => {
                                    note(reason);
                                    return;
                                }
                            };
                            let mut after = *lines;
                            after[v] = line;
                            let mut total = *cost + price;
                            for w in 0..n {
                                if w == v || decided & (1 << w) == 0 {
                                    continue;
                                }
                                let (upper, lower) = if v < w { (v, w) } else { (w, v) };
                                match self.pair(upper, lower, &state.lines, &after, struck, free, s)
                                {
                                    Ok(price) => total += price,
                                    Err(reason) => {
                                        note(reason);
                                        return;
                                    }
                                }
                            }
                            let mut placed = *placed;
                            placed[v] = Some(z);
                            let count = Count {
                                forced: forced.forced + f.forced,
                                own: forced.own + f.own,
                                excused: forced.excused + f.excused,
                            };
                            grown.push((after, total, count, placed));
                        };
                        match events[v] {
                            Ev::Fixed(z) => try_place(z, true, &mut grown),
                            _ => {
                                self.candidates(v, &lines[v], s, &mut candidates);
                                for z in candidates.iter().copied() {
                                    try_place(z, false, &mut grown);
                                }
                            }
                        }
                    }
                    decided |= 1 << v;
                    if grown.is_empty()
                        && index == 0
                        && std::env::var_os("FUX_BACH_TRACE").is_some()
                    {
                        eprintln!(
                            "slot {s}: voice {v} has no tone; {} candidates {:?}; partial {}",
                            candidates.len(),
                            candidates.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
                            partial.len()
                        );
                    }
                    grown.sort_by_key(|g| g.1);
                    grown.truncate(48);
                    partial = grown;
                    if partial.is_empty() {
                        break;
                    }
                }
                for (lines, mut cost, forced, placed) in partial {
                    if free != 0 && info.weight >= 2 {
                        cost += self.sound(&lines, info);
                    }
                    // How far the voices still are from their next fixed tones.
                    let mut rank = cost;
                    for v in 0..n {
                        if let (Some((ahead, target)), Some(cur)) =
                            (self.targets[v][s], lines[v].cur)
                        {
                            let far = (cur.midi - target.midi).unsigned_abs() as u32;
                            let free_way = 2 + 2 * ahead as u32;
                            rank += self.costs().far * far.saturating_sub(free_way);
                        }
                    }
                    next.push((
                        State {
                            lines,
                            cost,
                            forced,
                            rank,
                            tie: rng.bits() as u32,
                        },
                        Back {
                            parent: index as u32,
                            placed,
                        },
                    ));
                }
            }
            if next.is_empty() {
                if std::env::var_os("FUX_BACH_TRACE").is_some() {
                    eprintln!(
                        "dead end at slot {s}: events {events:?}, beam {}",
                        beam.len()
                    );
                    for state in beam.iter().take(3) {
                        for line in &state.lines[..n] {
                            eprintln!("   {:?}", line);
                        }
                        eprintln!("   --");
                    }
                }
                let mut reasons = std::mem::take(refused.get_mut());
                reasons.sort_by_key(|r| std::cmp::Reverse(r.1));
                return Outcome::DeadEnd(s, reasons);
            }
            // Of states with the same lines only the cheapest goes on.
            next.sort_by_key(|(state, _)| (state.rank, state.tie));
            let mut seen: HashSet<[Line; MAX_VOICES]> = HashSet::new();
            let mut kept: Vec<(State, Back)> = Vec::new();
            for entry in next {
                if kept.len() >= self.task.beam {
                    break;
                }
                if seen.insert(entry.0.lines) {
                    kept.push(entry);
                }
            }
            beam = kept.iter().map(|k| k.0.clone()).collect();
            backs.push(kept.iter().map(|k| k.1).collect());
        }
        // No voice may end on a foreign tone.
        let Some((best, state)) = beam
            .iter()
            .enumerate()
            .filter(|(_, st)| {
                st.lines[..n]
                    .iter()
                    .all(|l| l.cur.is_none() || l.foreign == 0)
            })
            .min_by_key(|(_, st)| (st.cost, st.tie))
        else {
            return Outcome::DeadEnd(
                self.slots.len(),
                vec![("foreign tone at the end", beam.len())],
            );
        };
        let mut onsets: Vec<Vec<(usize, Pitch)>> = vec![Vec::new(); n];
        let mut index = best;
        for s in (0..self.slots.len()).rev() {
            let back = &backs[s][index];
            for v in 0..n {
                if let Some(p) = back.placed[v] {
                    onsets[v].push((s, p));
                }
            }
            index = back.parent as usize;
        }
        for line in &mut onsets {
            line.reverse();
        }
        Outcome::Found(Found {
            onsets,
            cost: state.cost,
            count: state.forced,
        })
    }
}

pub fn search(task: &Task) -> Outcome {
    let count = task.voices.len();
    assert!(count <= MAX_VOICES && count > 0);
    let slot_count = task.voices[0].events.len();
    let mut slots: Vec<SlotInfo> = Vec::with_capacity(slot_count);
    let mut before: Option<Chord> = None;
    for s in 0..slot_count {
        let tick = task.start + s as Tick * SIXTEENTH;
        let span = task.harmony.at(tick);
        let chord = span.map(|sp| sp.chord);
        let pos = (tick - task.origin).rem_euclid(task.meter.bar());
        slots.push(SlotInfo {
            chord,
            mask: chord.map_or(0, Chord::mask),
            key: span.map_or_else(
                || {
                    task.harmony
                        .spans
                        .first()
                        .map_or(Key::major(crate::pitch::Lof(0)), |sp| sp.key)
                },
                |sp| sp.key,
            ),
            weight: task.meter.weight(pos),
            changed: s > 0 && chord.map(Chord::mask) != before.map(Chord::mask),
            dominant: chord.is_some_and(|c| c.kind.is_dominant()),
        });
        before = chord;
    }
    let mut lengths = vec![vec![0u16; slot_count]; count];
    let mut targets = vec![vec![None; slot_count]; count];
    let horizon = (2 * task.meter.beat() / SIXTEENTH) as usize;
    for (v, plan) in task.voices.iter().enumerate() {
        for s in 0..slot_count {
            if matches!(plan.events[s], Ev::Onset | Ev::Fixed(_)) {
                let mut k = s + 1;
                while k < slot_count && matches!(plan.events[k], Ev::Sustain | Ev::FixedSustain) {
                    k += 1;
                }
                lengths[v][s] = (k - s) as u16;
            }
            if !plan.events[s].is_fixed() {
                targets[v][s] = (1..=horizon).find_map(|ahead| match plan.events.get(s + ahead) {
                    Some(Ev::Fixed(p)) => Some((ahead as u16, *p)),
                    _ => None,
                });
            }
        }
    }
    Search {
        task,
        slots,
        lengths,
        targets,
        count,
    }
    .run()
}
