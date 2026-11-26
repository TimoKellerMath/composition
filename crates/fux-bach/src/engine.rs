//! The machine of the style: one transition for the search and for the check.
//!
//! Time is in ticks. Every voice is a list of events (a given tone, a free tone, a rest); the
//! moments of a task are the ticks at which a voice has an event or the chord changes. At a
//! moment the voices that fall silent are cleared, the held tones are read against a new chord
//! (`hold`), every struck tone is read in its voice (`place`), every pair of voices of which one
//! strikes against each other (`pair`), and on a beat the whole sound (`sound`). Each of these
//! reports what it finds to a sink with the rule's identifier; the level of the rule in the
//! profile decides whether a finding costs or forbids.
//!
//! The search walks the moments with a beam of states and chooses the free tones; a finding of a
//! forbidden rule on a free tone ends a candidate. The check (`replay`) walks the same moments
//! with every tone known and collects the findings. Both call the same four functions, so what
//! the search priced and what the check reports are the same by construction; the tests compare
//! the two sums run by run.

use std::collections::HashSet;

use crate::key::{Chord, Harmony, Key};
use crate::melody::{self, Wish};
use crate::pitch::{Lof, Pitch};
use crate::rng::Rng;
use crate::rules::{Bearer, Level, Params, Profile, Rule};
use crate::time::{Meter, Tick};

pub const MAX_VOICES: usize = 6;
const NONE: u32 = u32::MAX;

/// What begins in a voice.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    Rest,
    /// A tone whose pitch the search chooses.
    Free,
    Given(Pitch),
}

#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub at: Tick,
    pub kind: Kind,
    /// For a free tone: the interval the line would like to make.
    pub wish: Option<Wish>,
    /// The tone belongs to a pedal point: it owes nothing to the chords above it.
    pub pedal: bool,
}

impl Event {
    pub const fn rest(at: Tick) -> Event {
        Event {
            at,
            kind: Kind::Rest,
            wish: None,
            pedal: false,
        }
    }

    pub const fn given(at: Tick, pitch: Pitch) -> Event {
        Event {
            at,
            kind: Kind::Given(pitch),
            wish: None,
            pedal: false,
        }
    }

    pub const fn free(at: Tick, wish: Option<Wish>) -> Event {
        Event {
            at,
            kind: Kind::Free,
            wish,
            pedal: false,
        }
    }
}

/// One voice: its range for free tones and its events in time order, each lasting until the
/// next.
#[derive(Clone, Debug)]
pub struct Part {
    pub low: Pitch,
    pub high: Pitch,
    pub events: Vec<Event>,
}

/// Two voices (upper, lower) that stand in double counterpoint at the octave between two ticks:
/// on beats no fifth or fourth between them, and not more than an octave apart.
#[derive(Clone, Copy, Debug)]
pub struct Invertible {
    pub upper: usize,
    pub lower: usize,
    pub from: Tick,
    pub until: Tick,
    /// Semitones the two voices may lie apart: 12 for inversion at the octave, 24 at the
    /// fifteenth (for a subject whose compass is wider than an octave).
    pub span: i16,
}

pub struct Task<'a> {
    pub meter: Meter,
    /// Tick of the first barline, for the metric weights.
    pub origin: Tick,
    pub end: Tick,
    pub harmony: &'a Harmony,
    pub parts: Vec<Part>,
    pub profile: &'a Profile,
    pub beam: usize,
    pub seed: u64,
    pub invertible: Vec<Invertible>,
    /// At this tick, the close of a piece, the outer voices take the root of the chord.
    pub root_at: Option<Tick>,
    /// A searched tone after a foreign given tone resolves it wherever one can: the excuse is
    /// only for where none does.
    pub resolve_given: bool,
}

/// What a finding is reported to. The search listens only for what forbade a candidate, the
/// check collects everything, the count on a corpus also the opportunities.
pub trait Sink {
    /// A place where the rule could have been broken.
    fn chance(&mut self, _rule: Rule) {}
    /// The rule was broken at `at` by voice `voice` (against `other`).
    fn found(
        &mut self,
        _rule: Rule,
        _level: Level,
        _at: Tick,
        _voice: usize,
        _other: Option<usize>,
        _bearer: Bearer,
    ) {
    }
}

pub struct Quiet;

impl Sink for Quiet {}

/// How often each rule forbade a free tone.
struct Refusals([u32; Rule::COUNT]);

impl Sink for Refusals {
    fn found(
        &mut self,
        rule: Rule,
        level: Level,
        _at: Tick,
        _voice: usize,
        _other: Option<usize>,
        bearer: Bearer,
    ) {
        if level == Level::Forbidden && bearer == Bearer::Free {
            self.0[rule as usize] += 1;
        }
    }
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

/// The state of a voice: what the next tone has to know of the tones before it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Line {
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
    /// `cur` is a given tone.
    given: bool,
    /// `cur` lies from the chord before into this one and ends within a beat of the change.
    late: bool,
    pedal: bool,
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
    given: false,
    late: false,
    pedal: false,
};

type Lines = [Line; MAX_VOICES];

/// Given tones that broke a prohibition, by who answers for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Count {
    /// A given tone after a free one: the free voice did not lead into the entry.
    pub junction: u32,
    /// A given tone after a given tone or a rest: the given line itself.
    pub own: u32,
    /// A free tone did not resolve the foreign given tone before it.
    pub excused: u32,
}

impl Count {
    fn add(&mut self, other: Count) {
        self.junction += other.junction;
        self.own += other.own;
        self.excused += other.excused;
    }
}

/// What a step adds up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// The prices of the rules.
    pub cost: u32,
    /// The prices of the plan: wishes not granted, the edge of a range, given tones that broke
    /// a prohibition.
    pub plan: u32,
    /// A free tone broke a prohibition.
    pub stop: bool,
    pub count: Count,
}

impl Tally {
    fn add(&mut self, other: Tally) {
        self.cost += other.cost;
        self.plan += other.plan;
        self.stop |= other.stop;
        self.count.add(other.count);
    }

    pub const fn total(&self) -> u32 {
        self.cost + self.plan
    }
}

struct Moment {
    at: Tick,
    chord: Option<Chord>,
    mask: u16,
    key: Key,
    weight: u8,
    /// The chord differs from the one of the moment before.
    changed: bool,
    /// A span of the harmony begins here.
    onset: bool,
    dominant: bool,
    /// Per voice: the event that begins here.
    begins: [u32; MAX_VOICES],
    /// Per voice: the event in force.
    current: [u32; MAX_VOICES],
    /// Per voice: its next given tone within two beats, as (ticks ahead, pitch).
    target: [Option<(Tick, Pitch)>; MAX_VOICES],
}

impl Moment {
    fn has(&self, pitch: Pitch) -> bool {
        self.chord.is_none() || self.mask & (1 << pitch.pc()) != 0
    }
}

/// What is known of an event before any pitch is chosen.
struct Shape {
    /// Where the next event of the voice begins.
    end: Tick,
    /// After it the voice is silent.
    then_silent: bool,
    /// The chord at its last tick.
    last_mask: Option<u16>,
    /// The chord at its onset differs from the one at the onset of the tone before.
    changed_since: bool,
    /// Onset of the event before it.
    before: Tick,
}

pub struct Machine<'a> {
    task: &'a Task<'a>,
    levels: Vec<Level>,
    params: &'a Params,
    moments: Vec<Moment>,
    shapes: Vec<Vec<Shape>>,
    count: usize,
    beat: Tick,
}

/// The tones of a setting: per voice and event the pitch of a tone, `None` for a rest.
pub type Tones = Vec<Vec<Option<Pitch>>>;

#[derive(Clone, Debug)]
pub struct Found {
    pub tones: Tones,
    pub tally: Tally,
}

#[derive(Clone, Debug)]
pub enum Outcome {
    Found(Found),
    /// No candidate survived this tick; with it, how often each rule forbade a candidate there.
    DeadEnd(Tick, Vec<(Rule, usize)>),
}

#[derive(Clone)]
struct State {
    lines: Lines,
    tally: Tally,
    /// Total price plus what the tones still have to travel to their next given tones.
    rank: u32,
    tie: u32,
}

/// A state while the voices of one moment are placed.
type Partial = (Lines, Tally, [Option<Pitch>; MAX_VOICES]);

#[derive(Clone, Copy)]
struct Back {
    parent: u32,
    placed: [Option<Pitch>; MAX_VOICES],
}

impl<'a> Machine<'a> {
    pub fn new(task: &'a Task<'a>) -> Machine<'a> {
        let count = task.parts.len();
        assert!(count > 0 && count <= MAX_VOICES);
        let mut ticks: Vec<Tick> = task
            .parts
            .iter()
            .flat_map(|p| p.events.iter().map(|e| e.at))
            .collect();
        let first = ticks.iter().copied().min().unwrap_or(0);
        ticks.extend(
            task.harmony
                .spans
                .iter()
                .map(|s| s.on)
                .filter(|t| *t > first && *t < task.end),
        );
        ticks.sort_unstable();
        ticks.dedup();
        let home = task
            .harmony
            .spans
            .first()
            .map_or(Key::major(Lof(0)), |s| s.key);
        let horizon = 2 * task.meter.beat();
        let mut moments: Vec<Moment> = Vec::with_capacity(ticks.len());
        let mut next = [0usize; MAX_VOICES];
        let mut before: Option<u16> = None;
        for (i, at) in ticks.iter().copied().enumerate() {
            let span = task.harmony.at(at);
            let chord = span.map(|s| s.chord);
            let mask = chord.map(Chord::mask);
            let mut begins = [NONE; MAX_VOICES];
            let mut current = [NONE; MAX_VOICES];
            let mut target = [None; MAX_VOICES];
            for (v, part) in task.parts.iter().enumerate() {
                if part.events.get(next[v]).is_some_and(|e| e.at == at) {
                    begins[v] = next[v] as u32;
                    next[v] += 1;
                }
                if next[v] > 0 {
                    current[v] = (next[v] - 1) as u32;
                }
                target[v] = part.events[next[v]..]
                    .iter()
                    .take_while(|e| e.at - at <= horizon)
                    .find_map(|e| match e.kind {
                        Kind::Given(p) => Some((e.at - at, p)),
                        _ => None,
                    });
            }
            moments.push(Moment {
                at,
                chord,
                mask: mask.unwrap_or(0),
                key: span.map_or(home, |s| s.key),
                weight: task
                    .meter
                    .weight((at - task.origin).rem_euclid(task.meter.bar())),
                changed: i > 0 && mask != before,
                onset: span.is_some_and(|s| s.on == at || i == 0),
                dominant: chord.is_some_and(|c| c.kind.is_dominant()),
                begins,
                current,
                target,
            });
            before = mask;
        }
        let mask_at = |t: Tick| task.harmony.at(t).map(|s| s.chord.mask());
        let shapes = task
            .parts
            .iter()
            .map(|part| {
                part.events
                    .iter()
                    .enumerate()
                    .map(|(e, event)| {
                        let after = part.events.get(e + 1);
                        let end = after.map_or(task.end, |a| a.at).max(event.at + 1);
                        let before = if e > 0 {
                            part.events[e - 1].at
                        } else {
                            event.at
                        };
                        Shape {
                            end,
                            then_silent: after.is_none_or(|a| a.kind == Kind::Rest),
                            last_mask: mask_at(end - 1),
                            changed_since: mask_at(event.at) != mask_at(before),
                            before,
                        }
                    })
                    .collect()
            })
            .collect();
        Machine {
            task,
            levels: Rule::ALL.iter().map(|r| task.profile.level(*r)).collect(),
            params: &task.profile.params,
            moments,
            shapes,
            count,
            beat: task.meter.beat(),
        }
    }

    /// A finding: priced, or forbidden and then borne by whoever answers for it.
    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn judge<S: Sink>(
        &self,
        rule: Rule,
        times: (u32, u32),
        at: Tick,
        voice: usize,
        other: Option<usize>,
        bearer: Bearer,
        tally: &mut Tally,
        sink: &mut S,
    ) {
        let level = self.levels[rule as usize];
        match level {
            Level::Off => {}
            Level::Cost(n) => tally.cost += n * times.0 / times.1,
            Level::Forbidden => match bearer {
                Bearer::Free => tally.stop = true,
                Bearer::Junction => {
                    tally.plan += self.params.given_fault;
                    tally.count.junction += 1;
                }
                Bearer::Given | Bearer::Excused => {
                    tally.plan += self.params.given_fault;
                    tally.count.own += 1;
                }
            },
        }
        sink.found(rule, level, at, voice, other, bearer);
    }

    /// The pitches written on the diatonic step `dia` that the key or the chord offer.
    fn spellings(&self, dia: i16, m: &Moment, out: &mut Vec<Pitch>) {
        let step = dia.rem_euclid(7) as i8;
        let octave = dia.div_euclid(7) as i8;
        let degree = (step - m.key.tonic.step()).rem_euclid(7) as usize;
        let mut push = |class: Lof| {
            if class.step() == step {
                let p = Pitch::new(step, class.alter(), octave);
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        };
        push(m.key.degree(degree));
        push(m.key.raised(degree));
        if let Some(chord) = m.chord {
            for tone in chord.tones() {
                push(tone);
            }
        }
    }

    /// The pitches a free tone of voice `v` may take at moment `m`.
    fn candidates(&self, m: &Moment, v: usize, line: &Line, out: &mut Vec<Pitch>) {
        out.clear();
        let part = &self.task.parts[v];
        let centre = Pitch {
            midi: ((part.low.midi as i32 + part.high.midi as i32) / 2) as i16,
            dia: ((part.low.dia as i32 + part.high.dia as i32) / 2) as i16,
        };
        let from = line.cur.or(line.last).unwrap_or(centre);
        let reach = if line.cur.is_some() {
            self.params.reach
        } else {
            self.params.reach_after_rest
        };
        let in_range = |p: Pitch| p.midi >= part.low.midi && p.midi <= part.high.midi;
        let mut chord_tones: Vec<Pitch> = Vec::new();
        if let Some(chord) = m.chord {
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
                self.spellings(dia, m, &mut chord_tones);
            }
            chord_tones.retain(|p| in_range(*p));
        }
        out.extend(&chord_tones);
        if let Some(y) = line.cur {
            // Steps, the same tone, and the third across for a group of neighbours.
            for d in [-2i16, -1, 0, 1, 2] {
                self.spellings(y.dia + d, m, out);
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
        let weak = self.levels[Rule::FigAppoggiaturaWeak as usize] != Level::Forbidden;
        if (m.weight >= 1 || weak) && line.foreign == 0 {
            // Appoggiaturas: the neighbours of the chord tones nearby.
            let mut near: Vec<Pitch> = Vec::new();
            for c in &chord_tones {
                if (c.midi - from.midi).abs() <= 9 {
                    self.spellings(c.dia + 1, m, &mut near);
                    self.spellings(c.dia - 1, m, &mut near);
                }
            }
            for p in near {
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
        // The close: the outer voices take the root (Bach's top voice ends on the tonic in 43 of
        // 47 fugues of the Well-Tempered Clavier, on the third in 4, never on the fifth).
        if self.task.root_at == Some(m.at)
            && (v == 0 || v + 1 == self.count)
            && let Some(chord) = m.chord
        {
            out.retain(|p| p.pc() == chord.root.pc());
        }
        // No spelling further from the key than the chromatic neighbours of its scale.
        let centre = m.key.tonic.0 - if m.key.minor { 3 } else { 0 };
        out.retain(|p| in_range(*p) && (centre - 6..=centre + 10).contains(&p.lof().0));
    }

    /// The interval from `y` to `z` in a voice.
    #[allow(clippy::too_many_arguments)]
    fn melodic<S: Sink>(
        &self,
        m: &Moment,
        line: &Line,
        y: Pitch,
        z: Pitch,
        z_chord: bool,
        v: usize,
        bearer: Bearer,
        tally: &mut Tally,
        sink: &mut S,
    ) {
        let b = y.to(z);
        let mut judge = |rule: Rule, tally: &mut Tally| {
            self.judge(rule, (1, 1), m.at, v, None, bearer, tally, sink)
        };
        if b.is_unison() {
            judge(Rule::MelRepeat, tally);
        } else if b.is_step() {
            if b.steps == 0 {
                judge(Rule::MelChromatic, tally);
            }
        } else {
            let both_chord = z_chord && m.has(y);
            if b.semis.abs() > 12 || b.steps == 0 || b.steps.abs() > 7 {
                judge(Rule::MelLeapBig, tally);
            } else if b.is_altered() {
                // Tritones and diminished leaps only inside a dominant chord.
                if both_chord && m.dominant {
                    judge(Rule::MelAlteredIn, tally);
                } else {
                    judge(Rule::MelAlteredOut, tally);
                }
            } else if b.steps.abs() == 6 {
                if both_chord {
                    judge(Rule::MelSeventhIn, tally);
                } else {
                    judge(Rule::MelSeventhOut, tally);
                }
            }
            if let Arrive::Leap(d, _) = line.arrive
                && d == b.dir()
            {
                if both_chord {
                    judge(Rule::MelArpeggio, tally);
                } else {
                    judge(Rule::MelLeapsSame, tally);
                }
            }
        }
        if let Arrive::Leap(d, true) = line.arrive
            && !(b.is_step() && b.dir() == -d)
        {
            judge(Rule::MelRecovery, tally);
        }
        if Some(z) == line.prev && !b.is_unison() {
            judge(Rule::MelBack, tally);
            if line.turns >= 1 {
                judge(Rule::MelTrill, tally);
            }
        }
        if line.seventh && !(b.is_step() && b.dir() < 0) && !b.is_unison() {
            judge(Rule::HarSeventhResolve, tally);
        }
        // The leading tone of a dominant that gives way to another chord goes up a step.
        if y.lof() == m.key.raised(6)
            && line.foreign == 0
            && m.changed
            && !m.dominant
            && !(b.is_step() && b.dir() > 0)
        {
            judge(Rule::MelLeading, tally);
        }
    }

    /// The opportunities of the melodic rules at the interval from `y` to `z` (for the count on
    /// a corpus; the search does not call it).
    fn melodic_chances<S: Sink>(&self, m: &Moment, line: &Line, y: Pitch, z: Pitch, sink: &mut S) {
        let b = y.to(z);
        for rule in [
            Rule::MelRepeat,
            Rule::MelChromatic,
            Rule::MelLeapBig,
            Rule::MelAlteredOut,
            Rule::MelAlteredIn,
            Rule::MelSeventhOut,
            Rule::MelSeventhIn,
        ] {
            sink.chance(rule);
        }
        if !b.is_unison() && !b.is_step() && matches!(line.arrive, Arrive::Leap(..)) {
            sink.chance(Rule::MelLeapsSame);
            sink.chance(Rule::MelArpeggio);
        }
        if matches!(line.arrive, Arrive::Leap(_, true)) {
            sink.chance(Rule::MelRecovery);
        }
        if line.prev.is_some() && !b.is_unison() {
            sink.chance(Rule::MelBack);
            sink.chance(Rule::MelTrill);
        }
        if line.seventh && !b.is_unison() {
            sink.chance(Rule::HarSeventhResolve);
        }
        if y.lof() == m.key.raised(6) && line.foreign == 0 && m.changed && !m.dominant {
            sink.chance(Rule::MelLeading);
        }
    }

    /// Voice `v` strikes `z` with its event `e` at moment `m`: its new state and what was found.
    #[allow(clippy::too_many_arguments)]
    fn place<S: Sink>(
        &self,
        m: &Moment,
        v: usize,
        e: usize,
        line: &Line,
        z: Pitch,
        census: bool,
        sink: &mut S,
    ) -> (Line, Tally) {
        let p = self.params;
        let part = &self.task.parts[v];
        let event = &part.events[e];
        let shape = &self.shapes[v][e];
        let free = event.kind == Kind::Free;
        let bearer = if free {
            Bearer::Free
        } else if line.cur.is_some() && !line.given {
            Bearer::Junction
        } else {
            Bearer::Given
        };
        let pedal = event.pedal;
        let z_chord = pedal || m.has(z);
        let at = m.at;
        let length = shape.end - at;
        let mut tally = Tally::default();
        macro_rules! judge {
            ($rule:expr) => {
                self.judge($rule, (1, 1), at, v, None, bearer, &mut tally, sink)
            };
            ($rule:expr, $at:expr) => {
                self.judge($rule, (1, 1), $at, v, None, bearer, &mut tally, sink)
            };
        }
        let mut foreign = 0u8;
        let mut chain = Chain::None;
        let mut late = false;
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
                if free {
                    // The top voice steps into the close, whatever was drawn for it (Bach's
                    // does in 46 of 47 fugues of the Well-Tempered Clavier).
                    let closing = v == 0 && self.task.root_at == Some(m.at);
                    let wish = if closing {
                        event.wish.map(|w| Wish { class: 1, ..w })
                    } else {
                        event.wish
                    };
                    if let Some(wish) = wish {
                        if let Some(steps) = wish.steps.filter(|_| !closing) {
                            // A tone of a figure: the step itself is wished for.
                            let away = (b.steps - steps as i16).unsigned_abs() as u32;
                            tally.plan += p.figure * away.min(2);
                        } else {
                            let away = (melody::class(b) as i32 - wish.class as i32).unsigned_abs();
                            let times = if closing { p.close } else { 1 };
                            tally.plan += p.wish_class * away.min(2) * times;
                            if let Some(x) = line.prev {
                                let before = x.to(y).dir();
                                if before != 0 && b.dir() != 0 && (before != b.dir()) != wish.turn {
                                    tally.plan += p.wish_turn;
                                }
                            }
                        }
                    }
                } else if bearer == Bearer::Junction {
                    tally.plan += p.into_given[melody::class(b)];
                }
                if census {
                    self.melodic_chances(m, line, y, z, sink);
                }
                self.melodic(m, line, y, z, z_chord, v, bearer, &mut tally, sink);
                if line.foreign > 0 {
                    // What the foreign tone before owes.
                    if census {
                        for rule in [
                            Rule::FigUnresolved,
                            Rule::FigGroup,
                            Rule::FigEscape,
                            Rule::FigAnticipation,
                            Rule::FigDelayed,
                        ] {
                            sink.chance(rule);
                        }
                    }
                    let step_in = |d: i8| b.is_step() && b.dir() == d;
                    let mut paid = true;
                    match (line.foreign, line.arrive, line.chain) {
                        (2, _, Chain::Pass(d)) => paid = step_in(d) && z_chord,
                        (2, _, Chain::Group(dia, d)) => {
                            paid = step_in(d) && z.dia == dia && z_chord;
                        }
                        (_, Arrive::Repeat, _) => {
                            if b.is_unison() {
                                if !z_chord {
                                    foreign = 1;
                                    late = length <= self.beat;
                                    judge!(Rule::FigRestruck);
                                }
                            } else if b.is_step() && z_chord {
                                // A suspension resolves by step, downwards for choice.
                                if census {
                                    sink.chance(Rule::FigSuspensionUp);
                                }
                                if b.dir() > 0 {
                                    judge!(Rule::FigSuspensionUp);
                                }
                            } else if line.late && z_chord {
                                judge!(Rule::FigDelayed);
                            } else {
                                paid = false;
                            }
                        }
                        (_, _, _) if b.is_unison() => {
                            // The foreign tone again, as a tone of the chord that has come.
                            if z_chord && shape.changed_since {
                                judge!(Rule::FigAnticipation);
                            } else {
                                paid = false;
                            }
                        }
                        (_, Arrive::Step(d), _) => {
                            if step_in(d) {
                                if !z_chord {
                                    foreign = 2;
                                    chain = Chain::Pass(d);
                                }
                            } else if step_in(-d) {
                                paid = z_chord && line.prev.is_some_and(|x| x.dia == z.dia);
                            } else if b.steps == -2 * d as i16 && !z_chord && line.prev.is_some() {
                                foreign = 2;
                                chain = Chain::Group(line.prev.map_or(0, |x| x.dia), d);
                                judge!(Rule::FigGroup);
                            } else if z_chord
                                && !b.is_step()
                                && (b.dir() == -d || b.steps.abs() == 2)
                            {
                                // An escape tone, or a cambiata by a third in the same direction.
                                judge!(Rule::FigEscape);
                            } else {
                                paid = false;
                            }
                        }
                        (_, Arrive::Leap(..) | Arrive::Start, _) => paid = b.is_step() && z_chord,
                    }
                    if !paid {
                        // The finding stands at the foreign tone, not at the tone after it.
                        if line.given && free {
                            tally.plan += p.excused;
                            tally.count.excused += 1;
                            sink.found(
                                Rule::FigUnresolved,
                                self.levels[Rule::FigUnresolved as usize],
                                shape.before,
                                v,
                                None,
                                Bearer::Excused,
                            );
                        } else {
                            judge!(Rule::FigUnresolved, shape.before);
                        }
                        foreign = u8::from(!z_chord);
                        chain = Chain::None;
                    }
                } else if !z_chord {
                    foreign = 1;
                    match arrive {
                        Arrive::Repeat => {
                            late = length <= self.beat;
                            judge!(Rule::FigRestruck);
                        }
                        Arrive::Step(_) => match m.weight {
                            // On a beat a foreign tone costs, on a strong beat more.
                            0 | 1 => {}
                            2 => judge!(Rule::FigAccented),
                            _ => self.judge(
                                Rule::FigAccented,
                                (p.accented_strong_halves, 2),
                                at,
                                v,
                                None,
                                bearer,
                                &mut tally,
                                sink,
                            ),
                        },
                        Arrive::Leap(..) | Arrive::Start => {
                            if m.weight >= 1 {
                                judge!(Rule::FigAppoggiatura);
                            } else {
                                judge!(Rule::FigAppoggiaturaWeak);
                            }
                        }
                    }
                }
                if census {
                    match arrive {
                        Arrive::Repeat => sink.chance(Rule::FigRestruck),
                        Arrive::Step(_) if m.weight >= 2 => sink.chance(Rule::FigAccented),
                        Arrive::Leap(..) if m.weight >= 1 => sink.chance(Rule::FigAppoggiatura),
                        Arrive::Leap(..) => sink.chance(Rule::FigAppoggiaturaWeak),
                        _ => {}
                    }
                }
            }
            None => {
                arrive = Arrive::Start;
                if census {
                    sink.chance(if m.weight >= 1 {
                        Rule::FigAppoggiatura
                    } else {
                        Rule::FigAppoggiaturaWeak
                    });
                }
                if !z_chord {
                    foreign = 1;
                    if m.weight >= 1 {
                        judge!(Rule::FigAppoggiatura);
                    } else {
                        judge!(Rule::FigAppoggiaturaWeak);
                    }
                }
                if let Some(last) = line.last {
                    if census {
                        sink.chance(Rule::MelEntryFar);
                        sink.chance(Rule::MelEntryWide);
                    }
                    let far = (last.midi - z.midi).abs();
                    if far > 12 {
                        judge!(Rule::MelEntryFar);
                    } else if far > 7 {
                        judge!(Rule::MelEntryWide);
                    }
                }
            }
        }
        if foreign > 0 {
            // A foreign tone is short.
            if census {
                sink.chance(Rule::FigLong);
                sink.chance(Rule::FigTooLong);
            }
            if length > 2 * self.beat {
                judge!(Rule::FigTooLong);
            } else if length > self.beat {
                judge!(Rule::FigLong);
            }
        }
        // No tone is foreign to the chord when its voice falls silent.
        if shape.then_silent && !pedal {
            if census {
                sink.chance(Rule::FigBeforeRest);
            }
            if shape
                .last_mask
                .is_some_and(|mask| mask & (1 << z.pc()) == 0)
            {
                judge!(Rule::FigBeforeRest);
            }
        }
        if free {
            if z.midi - part.low.midi < 2 || part.high.midi - z.midi < 2 {
                tally.plan += p.edge;
            }
            if z.midi < part.low.midi || z.midi > part.high.midi {
                tally.stop = true;
            }
        }
        let seventh = !pedal
            && m.chord
                .is_some_and(|c| c.member(z.pc()) == Some(3) && m.mask & (1 << z.pc()) != 0);
        let turns = if Some(z) == line.prev && line.cur != Some(z) {
            line.turns.saturating_add(1)
        } else {
            0
        };
        (
            Line {
                cur: Some(z),
                prev: line.cur,
                last: Some(z),
                foreign,
                arrive,
                chain,
                seventh,
                turns,
                given: !free,
                late,
                pedal,
            },
            tally,
        )
    }

    /// A held tone under a new chord.
    fn hold<S: Sink>(
        &self,
        m: &Moment,
        v: usize,
        line: &mut Line,
        census: bool,
        tally: &mut Tally,
        sink: &mut S,
    ) {
        let Some(y) = line.cur else {
            return;
        };
        if line.pedal {
            return;
        }
        let inside = m.has(y);
        if line.foreign == 0 && !inside {
            // A held tone becomes a suspension.
            line.foreign = 1;
            line.arrive = Arrive::Repeat;
            line.chain = Chain::None;
            let end = self.shapes[v][m.current[v] as usize].end;
            line.late = end - m.at <= self.beat;
        } else if line.foreign > 0 {
            if census {
                sink.chance(Rule::FigLying);
            }
            if inside {
                line.foreign = 0;
                line.chain = Chain::None;
                line.late = false;
            } else {
                let bearer = if line.given {
                    Bearer::Given
                } else {
                    Bearer::Free
                };
                self.judge(Rule::FigLying, (1, 1), m.at, v, None, bearer, tally, sink);
            }
        }
        line.seventh = inside && m.chord.is_some_and(|c| c.member(y.pc()) == Some(3));
    }

    /// Two voices against each other at a moment at which at least one of them strikes. `upper`
    /// is the voice written above.
    #[allow(clippy::too_many_arguments)]
    fn pair<S: Sink>(
        &self,
        m: &Moment,
        upper: usize,
        lower: usize,
        before: &Lines,
        after: &Lines,
        struck: u8,
        census: bool,
        tally: &mut Tally,
        sink: &mut S,
    ) {
        let (Some(a), Some(b)) = (after[upper].cur, after[lower].cur) else {
            return;
        };
        let p = self.params;
        let (su, sl) = (struck & (1 << upper) != 0, struck & (1 << lower) != 0);
        let both_free = !after[upper].given && !after[lower].given;
        let bearer = if !after[upper].given || !after[lower].given {
            Bearer::Free
        } else {
            let led = |v: usize, s: bool| s && before[v].cur.is_some() && !before[v].given;
            if led(upper, su) || led(lower, sl) {
                Bearer::Junction
            } else {
                Bearer::Given
            }
        };
        let at = m.at;
        let mut judge = |rule: Rule, times: u32, tally: &mut Tally| {
            self.judge(
                rule,
                (times, 1),
                at,
                upper,
                Some(lower),
                bearer,
                tally,
                sink,
            )
        };
        if let Some(region) = self
            .task
            .invertible
            .iter()
            .find(|i| i.upper == upper && i.lower == lower && i.from <= at && at < i.until)
        {
            let simple = b.to(a).simple();
            if m.weight >= 2 && (simple == (4, 7) || simple == (3, 5)) {
                judge(Rule::DcFifth, 1, tally);
            }
            if a.midi - b.midi > region.span || a.midi < b.midi {
                judge(Rule::DcWide, 1, tally);
            }
        }
        if a.midi < b.midi {
            let depth = (b.midi - a.midi) as u32;
            // A free voice against a given one: a deep crossing only costs, since the given
            // voices may cross themselves and leave no room between them.
            let both_given = after[upper].given && after[lower].given;
            if depth > p.crossing_deep && (both_free || both_given) {
                judge(Rule::MotCrossingDeep, 1, tally);
            }
            // In the final chord the top voice is on top: a crossing there weighs like the
            // other faults of the close.
            let weight = if self.task.root_at == Some(at) {
                p.close
            } else {
                1
            };
            judge(
                Rule::MotCrossing,
                (1 + depth / p.crossing_per) * weight,
                tally,
            );
        }
        let now = b.to(a);
        let mut similar = false;
        if su && sl {
            if let (Some(a0), Some(b0)) = (before[upper].cur, before[lower].cur) {
                let then = b0.to(a0);
                let (da, db) = (a0.to(a).dir(), b0.to(b).dir());
                similar = da != 0 && da == db;
                if similar && now.is_perfect() {
                    if then.is_perfect() && now.simple() == then.simple() {
                        judge(Rule::MotParallel, 1, tally);
                    } else if !a0.to(a).is_step() {
                        // A perfect interval reached in similar motion with a leap above.
                        judge(Rule::MotDirect, 1, tally);
                    }
                }
                if similar && upper == 0 && lower + 1 == self.count {
                    judge(Rule::MotOuterSimilar, 1, tally);
                }
            }
            if after[upper].foreign > 0 && after[lower].foreign > 0 && !now.is_consonant(true) {
                judge(Rule::HarClash, 1, tally);
            }
            if a.midi == b.midi {
                judge(Rule::MotUnison, 1, tally);
            }
        }
        if a.pc() == b.pc() {
            if m.key.raised(6).pc() == a.pc() && m.dominant {
                judge(Rule::HarDoubledLeading, 1, tally);
            }
            if after[upper].seventh && after[lower].seventh {
                judge(Rule::HarDoubledSeventh, 1, tally);
            }
            if after[upper].foreign > 0 && after[lower].foreign > 0 {
                judge(Rule::HarDoubledForeign, 1, tally);
            }
        }
        // A struck foreign tone a semitone away from a sounding tone.
        let semis = (a.midi - b.midi).rem_euclid(12);
        let struck_foreign = (su && after[upper].foreign > 0) || (sl && after[lower].foreign > 0);
        if struck_foreign && (semis == 1 || semis == 11) {
            judge(Rule::HarHarsh, 1, tally);
        }
        if census {
            sink.chance(Rule::MotCrossing);
            sink.chance(Rule::MotCrossingDeep);
            if su && sl {
                sink.chance(Rule::MotUnison);
                if similar {
                    sink.chance(Rule::MotParallel);
                    sink.chance(Rule::MotDirect);
                }
                if upper == 0
                    && lower + 1 == self.count
                    && let (Some(a0), Some(b0)) = (before[upper].cur, before[lower].cur)
                    && a0 != a
                    && b0 != b
                {
                    sink.chance(Rule::MotOuterSimilar);
                }
                if after[upper].foreign > 0 && after[lower].foreign > 0 {
                    sink.chance(Rule::HarClash);
                }
            }
            if a.pc() == b.pc() {
                sink.chance(Rule::HarDoubledLeading);
                sink.chance(Rule::HarDoubledSeventh);
                sink.chance(Rule::HarDoubledForeign);
            }
            if struck_foreign {
                sink.chance(Rule::HarHarsh);
            }
        }
    }

    /// Where a chord begins that names its bass, the lowest sounding tone is that bass (with
    /// two voices and more; a chord that names none asks for nothing).
    fn bass<S: Sink>(
        &self,
        m: &Moment,
        after: &Lines,
        census: bool,
        tally: &mut Tally,
        sink: &mut S,
    ) {
        let Some(bass) = m.chord.and_then(|c| c.bass) else {
            return;
        };
        let n = self.count;
        let mut sounding = (0..n).filter_map(|v| after[v].cur.map(|p| (p, v)));
        let Some(first) = sounding.next() else {
            return;
        };
        let mut count = 1;
        let mut lowest = first;
        for tone in sounding {
            count += 1;
            if tone.0 < lowest.0 {
                lowest = tone;
            }
        }
        if count < 2 {
            return;
        }
        if census {
            sink.chance(Rule::HarBass);
        }
        if lowest.0.pc() != bass.pc() {
            let bearer = if after[lowest.1].given {
                Bearer::Given
            } else {
                Bearer::Free
            };
            self.judge(
                Rule::HarBass,
                (1, 1),
                m.at,
                lowest.1,
                None,
                bearer,
                tally,
                sink,
            );
        }
    }

    /// The whole sound on a beat on which a voice strikes.
    fn sound<S: Sink>(
        &self,
        m: &Moment,
        after: &Lines,
        struck: u8,
        census: bool,
        tally: &mut Tally,
        sink: &mut S,
    ) {
        let n = self.count;
        let free = (0..n).any(|v| struck & (1 << v) != 0 && !after[v].given);
        let bearer = if free { Bearer::Free } else { Bearer::Given };
        let Some(lowest) = after[..n].iter().filter_map(|l| l.cur).min() else {
            return;
        };
        let mut sounding = 0u32;
        let mut mask = 0u16;
        for line in &after[..n] {
            if let Some(p) = line.cur {
                sounding += 1;
                mask |= 1 << p.pc();
            }
        }
        // Two tones struck together in a dissonance; the fourth is one against the bass.
        for upper in 0..n {
            for lower in upper + 1..n {
                if struck & (1 << upper) == 0 || struck & (1 << lower) == 0 {
                    continue;
                }
                let (Some(a), Some(b)) = (after[upper].cur, after[lower].cur) else {
                    continue;
                };
                if census {
                    sink.chance(Rule::HarStruck);
                }
                if !b.to(a).is_consonant(a.min(b) != lowest) {
                    self.judge(
                        Rule::HarStruck,
                        (1, 1),
                        m.at,
                        upper,
                        Some(lower),
                        bearer,
                        tally,
                        sink,
                    );
                }
            }
        }
        if sounding < 3 {
            return;
        }
        let top = (0..n).find(|v| after[*v].cur.is_some()).unwrap_or(0);
        // The final chord weighs more: no fugue of the Well-Tempered Clavier ends on a fifth
        // without its third, and none for four voices on less than the triad.
        let close = self.task.root_at == Some(m.at);
        let times = if close { self.params.close } else { 1 };
        let mut judge = |rule: Rule, times: u32, tally: &mut Tally| {
            self.judge(rule, (times, 1), m.at, top, None, bearer, tally, sink)
        };
        // Any complete triad among the sounding tones, as the measure of the study counts it.
        let triad = (0..12u16).any(|root| {
            [[0, 4, 7], [0, 3, 7], [0, 3, 6]]
                .iter()
                .any(|tones| tones.iter().all(|t| mask & (1 << ((root + t) % 12)) != 0))
        });
        // At the close the outer voices hold the root: three voices cannot sound the triad
        // there (the rule counts as anywhere), four and more shall (it counts with the weight of
        // the close, however many voices it is read for elsewhere).
        let full = close && sounding >= 4;
        let read = full
            || (m.weight as u32 >= self.params.incomplete_weight
                && (self.params.incomplete_voices == 0
                    || self.params.incomplete_voices == sounding));
        if read && !triad {
            judge(Rule::HarIncomplete, if full { times } else { 1 }, tally);
        }
        if let Some(chord) = m.chord {
            let third = chord.tones().nth(1);
            if third.is_some_and(|t| mask & (1 << t.pc()) == 0) {
                judge(Rule::HarNoThird, times, tally);
            }
            if !chord.kind.is_seventh() && chord.member(lowest.pc()) == Some(2) {
                judge(Rule::HarSixFour, 1, tally);
            }
        }
        let mut sorted: Vec<i16> = after[..n]
            .iter()
            .filter_map(|l| l.cur.map(|p| p.midi))
            .collect();
        sorted.sort_unstable();
        if sorted
            .windows(2)
            .skip(1)
            .any(|w| (w[1] - w[0]) as u32 > self.params.wide)
        {
            judge(Rule::HarWide, 1, tally);
        }
        if census {
            for rule in [Rule::HarNoThird, Rule::HarSixFour, Rule::HarWide] {
                sink.chance(rule);
            }
            if read {
                sink.chance(Rule::HarIncomplete);
            }
        }
    }

    /// What happens at a moment before any tone is struck: voices fall silent, held tones meet
    /// a new chord.
    fn settle<S: Sink>(
        &self,
        m: &Moment,
        lines: &mut Lines,
        census: bool,
        tally: &mut Tally,
        sink: &mut S,
    ) {
        for v in 0..self.count {
            match m.begins[v] {
                NONE => {
                    if m.changed {
                        self.hold(m, v, &mut lines[v], census, tally, sink);
                    }
                }
                e => {
                    if self.task.parts[v].events[e as usize].kind == Kind::Rest {
                        lines[v] = Line {
                            last: lines[v].last,
                            ..SILENT
                        };
                    }
                }
            }
        }
    }

    /// The voices that strike at a moment: all of them, and the free ones.
    fn struck(&self, m: &Moment) -> (u8, u8) {
        let mut struck = 0u8;
        let mut free = 0u8;
        for v in 0..self.count {
            if m.begins[v] == NONE {
                continue;
            }
            match self.task.parts[v].events[m.begins[v] as usize].kind {
                Kind::Rest => {}
                Kind::Free => {
                    struck |= 1 << v;
                    free |= 1 << v;
                }
                Kind::Given(_) => struck |= 1 << v,
            }
        }
        (struck, free)
    }

    /// How far the free voices still are from their next given tones.
    fn distance(&self, m: &Moment, lines: &Lines) -> u32 {
        let mut far = 0;
        for v in 0..self.count {
            if let (Some((ahead, target)), Some(cur)) = (m.target[v], lines[v].cur)
                && !lines[v].given
            {
                let apart = (cur.midi - target.midi).unsigned_abs() as u32;
                let free_way = 2 + ahead as u32 / 3;
                far += self.params.far * apart.saturating_sub(free_way);
            }
        }
        far
    }

    /// The search: a beam of states over the moments.
    pub fn search(&self) -> Outcome {
        let n = self.count;
        let mut rng = Rng::new(self.task.seed);
        let mut beam: Vec<State> = vec![State {
            lines: [SILENT; MAX_VOICES],
            tally: Tally::default(),
            rank: 0,
            tie: 0,
        }];
        let mut backs: Vec<Vec<Back>> = Vec::with_capacity(self.moments.len());
        let mut candidates: Vec<Pitch> = Vec::new();
        let mut refusals = Refusals([0; Rule::COUNT]);
        for m in &self.moments {
            let (struck, free) = self.struck(m);
            // Given voices first, then the free ones from the bottom up.
            let mut order: Vec<usize> = (0..n)
                .filter(|v| struck & (1 << v) != 0 && free & (1 << v) == 0)
                .collect();
            order.extend((0..n).rev().filter(|v| free & (1 << v) != 0));
            refusals.0 = [0; Rule::COUNT];
            let mut next: Vec<(State, Back)> = Vec::new();
            for (index, state) in beam.iter().enumerate() {
                let mut lines = state.lines;
                let mut tally = state.tally;
                self.settle(m, &mut lines, false, &mut tally, &mut refusals);
                if tally.stop {
                    continue;
                }
                let mut partial: Vec<Partial> = vec![(lines, tally, [None; MAX_VOICES])];
                let mut decided: u8 = !struck;
                for &v in &order {
                    let e = m.begins[v] as usize;
                    let mut grown: Vec<Partial> = Vec::new();
                    for (lines, tally, placed) in &partial {
                        let mut try_place = |z: Pitch, grown: &mut Vec<Partial>| {
                            let (line, found) =
                                self.place(m, v, e, &lines[v], z, false, &mut refusals);
                            if found.stop {
                                return;
                            }
                            let mut after = *lines;
                            after[v] = line;
                            let mut total = *tally;
                            total.add(found);
                            for w in 0..n {
                                if w == v || decided & (1 << w) == 0 {
                                    continue;
                                }
                                let (upper, lower) = if v < w { (v, w) } else { (w, v) };
                                self.pair(
                                    m,
                                    upper,
                                    lower,
                                    &state.lines,
                                    &after,
                                    struck,
                                    false,
                                    &mut total,
                                    &mut refusals,
                                );
                                if total.stop {
                                    return;
                                }
                            }
                            let mut placed = *placed;
                            placed[v] = Some(z);
                            grown.push((after, total, placed));
                        };
                        match self.task.parts[v].events[e].kind {
                            Kind::Given(z) => try_place(z, &mut grown),
                            _ => {
                                self.candidates(m, v, &lines[v], &mut candidates);
                                let mut mine: Vec<Partial> = Vec::new();
                                for z in candidates.iter().copied() {
                                    try_place(z, &mut mine);
                                }
                                // A foreign given tone is resolved if any tone here can do it.
                                if self.task.resolve_given {
                                    let excused =
                                        |p: &Partial| p.1.count.excused > tally.count.excused;
                                    if mine.iter().any(|p| !excused(p)) {
                                        mine.retain(|p| !excused(p));
                                    }
                                }
                                grown.append(&mut mine);
                            }
                        }
                    }
                    decided |= 1 << v;
                    grown.sort_by_key(|g| g.1.total());
                    grown.truncate(48);
                    partial = grown;
                    if partial.is_empty() {
                        break;
                    }
                }
                for (lines, mut tally, placed) in partial {
                    if struck != 0 && m.weight >= 2 {
                        self.sound(m, &lines, struck, false, &mut tally, &mut refusals);
                        if tally.stop {
                            continue;
                        }
                    }
                    if m.onset {
                        self.bass(m, &lines, false, &mut tally, &mut refusals);
                        if tally.stop {
                            continue;
                        }
                    }
                    let rank = tally.total() + self.distance(m, &lines);
                    next.push((
                        State {
                            lines,
                            tally,
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
                let mut reasons: Vec<(Rule, usize)> = Rule::ALL
                    .iter()
                    .map(|r| (*r, refusals.0[*r as usize] as usize))
                    .filter(|r| r.1 > 0)
                    .collect();
                reasons.sort_by_key(|r| std::cmp::Reverse(r.1));
                return Outcome::DeadEnd(m.at, reasons);
            }
            // Of states with the same lines only the cheapest goes on.
            next.sort_by_key(|(state, _)| (state.rank, state.tie));
            let mut seen: HashSet<Lines> = HashSet::new();
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
        let Some((best, state)) = beam
            .iter()
            .enumerate()
            .min_by_key(|(_, st)| (st.tally.total(), st.tie))
        else {
            return Outcome::DeadEnd(self.task.end, Vec::new());
        };
        let mut tones: Tones = self
            .task
            .parts
            .iter()
            .map(|p| vec![None; p.events.len()])
            .collect();
        let mut index = best;
        for (m, back) in self.moments.iter().zip(&backs).rev() {
            let back = &back[index];
            for v in 0..n {
                if let Some(p) = back.placed[v] {
                    tones[v][m.begins[v] as usize] = Some(p);
                }
            }
            index = back.parent as usize;
        }
        Outcome::Found(Found {
            tones,
            tally: state.tally,
        })
    }

    /// The check: every tone is known, nothing ends the walk, everything found goes to the
    /// sink. With `census` the sink also hears of every opportunity.
    pub fn replay<S: Sink>(&self, tones: &Tones, census: bool, sink: &mut S) -> Tally {
        let n = self.count;
        let mut lines: Lines = [SILENT; MAX_VOICES];
        let mut total = Tally::default();
        for m in &self.moments {
            let (struck, _) = self.struck(m);
            let before = lines;
            let mut tally = Tally::default();
            self.settle(m, &mut lines, census, &mut tally, sink);
            for v in 0..n {
                if struck & (1 << v) == 0 {
                    continue;
                }
                let e = m.begins[v] as usize;
                let z = match self.task.parts[v].events[e].kind {
                    Kind::Given(z) => z,
                    _ => tones[v][e].expect("a free tone has a pitch"),
                };
                let (line, found) = self.place(m, v, e, &lines[v], z, census, sink);
                lines[v] = line;
                tally.add(found);
            }
            for upper in 0..n {
                for lower in upper + 1..n {
                    if struck & ((1 << upper) | (1 << lower)) != 0 {
                        self.pair(
                            m, upper, lower, &before, &lines, struck, census, &mut tally, sink,
                        );
                    }
                }
            }
            if struck != 0 && m.weight >= 2 {
                self.sound(m, &lines, struck, census, &mut tally, sink);
            }
            if m.onset {
                self.bass(m, &lines, census, &mut tally, sink);
            }
            total.add(tally);
        }
        total
    }
}

pub fn search(task: &Task) -> Outcome {
    Machine::new(task).search()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Kind as ChordKind;

    struct Collect(Vec<(Rule, Tick, usize)>);

    impl Sink for Collect {
        fn found(
            &mut self,
            rule: Rule,
            level: Level,
            at: Tick,
            voice: usize,
            _other: Option<usize>,
            _bearer: Bearer,
        ) {
            if level != Level::Off {
                self.0.push((rule, at, voice));
            }
        }
    }

    fn c_major() -> Harmony {
        let key = Key::parse("C").unwrap();
        let mut harmony = Harmony::default();
        harmony.push(0, Chord::new(Lof(0), ChordKind::Maj), key);
        harmony.push(96, Chord::new(Lof(1), ChordKind::Dom7), key);
        harmony.push(192, Chord::new(Lof(0), ChordKind::Maj), key);
        harmony
    }

    fn part(low: Pitch, high: Pitch, events: Vec<Event>) -> Part {
        Part { low, high, events }
    }

    #[test]
    fn the_search_and_the_check_agree() {
        let harmony = c_major();
        let profile = Profile::start();
        // A given bass in halves, a free voice in eighths above it.
        let bass = [(0, 0, 3), (48, 2, 3), (96, 4, 2), (144, 6, 2), (192, 0, 3)]
            .map(|(at, step, octave)| Event::given(at, Pitch::new(step, 0, octave)));
        let upper: Vec<Event> = (0..20).map(|i| Event::free(12 * i, None)).collect();
        let task = Task {
            meter: Meter::COMMON,
            origin: 0,
            end: 240,
            harmony: &harmony,
            parts: vec![
                part(Pitch::new(0, 0, 4), Pitch::new(0, 0, 6), upper),
                part(Pitch::new(0, 0, 2), Pitch::new(0, 0, 4), bass.to_vec()),
            ],
            profile: &profile,
            beam: 50,
            seed: 1,
            invertible: Vec::new(),
            root_at: None,
            resolve_given: false,
        };
        let machine = Machine::new(&task);
        let Outcome::Found(found) = machine.search() else {
            panic!("no setting");
        };
        assert!(found.tones[0].iter().all(Option::is_some));
        let mut sink = Collect(Vec::new());
        let again = machine.replay(&found.tones, false, &mut sink);
        assert_eq!(again, found.tally);
        assert!(!again.stop);
        // What the check reports adds up to what the search paid.
        let sum: u32 = sink
            .0
            .iter()
            .map(|(rule, ..)| profile.level(*rule).cost())
            .sum();
        assert!(sum <= again.cost);
    }

    #[test]
    fn a_given_line_is_read_not_pruned() {
        let harmony = c_major();
        let profile = Profile::start();
        // Parallel fifths between two given voices, and a foreign tone left by leap.
        let upper = [(0, 4, 4), (24, 5, 4), (48, 3, 4), (72, 0, 5)]
            .map(|(at, step, octave)| Event::given(at, Pitch::new(step, 0, octave)));
        let lower = [(0, 0, 4), (24, 1, 4), (48, 0, 4), (72, 2, 4)]
            .map(|(at, step, octave)| Event::given(at, Pitch::new(step, 0, octave)));
        let task = Task {
            meter: Meter::COMMON,
            origin: 0,
            end: 96,
            harmony: &harmony,
            parts: vec![
                part(Pitch::new(0, 0, 4), Pitch::new(0, 0, 6), upper.to_vec()),
                part(Pitch::new(0, 0, 3), Pitch::new(0, 0, 5), lower.to_vec()),
            ],
            profile: &profile,
            beam: 10,
            seed: 1,
            invertible: Vec::new(),
            root_at: None,
            resolve_given: false,
        };
        let machine = Machine::new(&task);
        let tones: Tones = vec![vec![None; 4], vec![None; 4]];
        let mut sink = Collect(Vec::new());
        let tally = machine.replay(&tones, false, &mut sink);
        assert!(sink.0.contains(&(Rule::MotParallel, 24, 0)));
        // The F of the upper voice is reached by leap and left by leap: at its own tick.
        assert!(sink.0.contains(&(Rule::FigUnresolved, 48, 0)));
        assert!(tally.count.own >= 2 && tally.count.junction == 0);
        // The same task as a search: one path, the same sums.
        let Outcome::Found(found) = machine.search() else {
            panic!("given tones never end a search");
        };
        assert_eq!(found.tally, tally);
    }
}
