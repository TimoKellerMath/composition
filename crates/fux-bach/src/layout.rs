//! From what is given about a piece (fixed tones, places of silence, ranges) to the plan of a
//! search and back from its result to a piece: the rhythm of the free voices is drawn from a
//! table of patterns, the pitches are searched.

use crate::key::{Harmony, Key};
use crate::melody;
use crate::pitch::Pitch;
use crate::rhythm::{self, Pattern, Slot, Table};
use crate::rng::Rng;
use crate::score::{Note, Piece, Voice};
use crate::search::{Costs, Ev, Found, Outcome, Task, VoicePlan, search};
use crate::time::{Meter, SIXTEENTH, Tick};

#[derive(Clone, Debug)]
pub struct VoiceLayout {
    pub low: Pitch,
    pub high: Pitch,
    /// The fixed tones.
    pub fixed: Vec<Note>,
    /// Where the voice is fixed: inside these spans only the fixed tones sound.
    pub spans: Vec<(Tick, Tick)>,
    /// Where the voice is free but silent.
    pub silent: Vec<(Tick, Tick)>,
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub name: String,
    pub key: Key,
    pub meter: Meter,
    pub origin: Tick,
    pub end: Tick,
    pub voices: Vec<VoiceLayout>,
    /// From here on every sounding free voice holds one tone to the end.
    pub final_hold: Option<Tick>,
    /// Pairs of voices (upper, lower) in double counterpoint at the octave between two ticks.
    pub invertible: Vec<(usize, usize, Tick, Tick)>,
    /// The lowest voice takes the root of the chord at the final hold.
    pub root_at_hold: bool,
    /// No free tone before the final hold lasts longer than this.
    pub longest: Option<Tick>,
    /// Refuse parallels into two fixed tones (see `search::Task::junctions`).
    pub junctions: bool,
}

fn inside(spans: &[(Tick, Tick)], t: Tick) -> bool {
    spans.iter().any(|(from, until)| *from <= t && t < *until)
}

impl Layout {
    /// Tick of slot 0: the first pulse on the bar grid at or before tick 0.
    pub fn start(&self) -> Tick {
        let pulse = self.meter.pulse();
        self.origin - (self.origin + pulse - 1).div_euclid(pulse) * pulse
    }

    pub fn slots(&self) -> usize {
        let pulse = self.meter.pulse();
        let pulses = (self.end - self.start() + pulse - 1) / pulse;
        (pulses * pulse / SIXTEENTH) as usize
    }

    /// What is known of voice `v` before any rhythm is drawn: the slots of its fixed spans and
    /// of its silences; `None` where it is free.
    fn known(&self, v: usize) -> Vec<Option<Slot>> {
        let voice = &self.voices[v];
        let line = Voice {
            notes: voice.fixed.clone(),
        };
        let start = self.start();
        (0..self.slots())
            .map(|s| {
                let t = start + s as Tick * SIXTEENTH;
                if inside(&voice.spans, t) {
                    Some(if !line.range(t, t + SIXTEENTH).is_empty() {
                        Slot::Onset
                    } else if line.at(t).is_some() {
                        Slot::Sustain
                    } else {
                        Slot::Rest
                    })
                } else if inside(&voice.silent, t) || t < 0 {
                    Some(Slot::Rest)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Draws the rhythm of the free voices and, with a table of intervals, their wishes, and
    /// returns the plan of every voice.
    pub fn plan(
        &self,
        table: &Table,
        intervals: Option<&melody::Table>,
        rng: &mut Rng,
    ) -> Vec<VoicePlan> {
        let per = table.slots;
        let count = self.slots();
        let pulses = count / per;
        let start = self.start();
        let known: Vec<Vec<Option<Slot>>> = (0..self.voices.len()).map(|v| self.known(v)).collect();
        // The slots of every voice as far as they are settled; free slots count as silence
        // until their voice has been drawn.
        let mut settled: Vec<Vec<Slot>> = known
            .iter()
            .map(|line| line.iter().map(|s| s.unwrap_or(Slot::Rest)).collect())
            .collect();
        let pattern_at =
            |line: &[Slot], i: usize| -> Pattern { rhythm::pattern(&line[i * per..(i + 1) * per]) };
        for v in 0..self.voices.len() {
            let mut before: Pattern = rhythm::pattern(&vec![Slot::Rest; per]);
            for i in 0..pulses {
                let range = i * per..(i + 1) * per;
                if known[v][range.clone()].iter().any(Option::is_none) {
                    let beside = rhythm::activity(
                        (0..self.voices.len())
                            .filter(|w| *w != v)
                            .map(|w| pattern_at(&settled[w], i)),
                        per,
                    );
                    let drawn = table
                        .draw(before, beside, |_| true, rng)
                        .unwrap_or(rhythm::pattern(&[Slot::Onset; 1]));
                    for (k, s) in range.clone().enumerate() {
                        if known[v][s].is_none() {
                            settled[v][s] = rhythm::slot(drawn, k);
                        }
                    }
                }
                // A continuation needs a tone before it.
                for s in range {
                    if settled[v][s] == Slot::Sustain && (s == 0 || settled[v][s - 1] == Slot::Rest)
                    {
                        settled[v][s] = Slot::Rest;
                    }
                }
                before = pattern_at(&settled[v], i);
            }
            if let Some(longest) = self.longest {
                // A free tone that would last longer is struck again.
                let limit = (longest / SIXTEENTH).max(1) as usize;
                let mut run = 0;
                for s in 0..count {
                    match settled[v][s] {
                        Slot::Sustain if known[v][s].is_none() => {
                            run += 1;
                            if run >= limit {
                                settled[v][s] = Slot::Onset;
                                run = 0;
                            }
                        }
                        _ => run = 0,
                    }
                }
            }
            if let Some(hold) = self.final_hold {
                let first = ((hold - start) / SIXTEENTH).max(0) as usize;
                let free = (first..count).all(|s| known[v][s].is_none());
                let sounding = first > 0 && settled[v][first - 1] != Slot::Rest;
                if free
                    && first < count
                    && (sounding || settled[v][first..].iter().any(|s| *s != Slot::Rest))
                {
                    settled[v][first] = Slot::Onset;
                    for slot in &mut settled[v][first + 1..] {
                        *slot = Slot::Sustain;
                    }
                }
            }
        }
        self.voices
            .iter()
            .enumerate()
            .map(|(v, voice)| {
                let line = Voice {
                    notes: voice.fixed.clone(),
                };
                let mut events: Vec<Ev> = Vec::with_capacity(count);
                for (s, slot) in settled[v].iter().enumerate().take(count) {
                    let t = start + s as Tick * SIXTEENTH;
                    let fixed = inside(&voice.spans, t);
                    let event = match (fixed, *slot) {
                        (true, Slot::Onset) => {
                            let note = line.range(t, t + SIXTEENTH).first().copied();
                            note.map_or(Ev::FixedRest, |n| Ev::Fixed(n.pitch))
                        }
                        (true, Slot::Sustain) => Ev::FixedSustain,
                        (true, Slot::Rest) => Ev::FixedRest,
                        (false, Slot::Onset) => Ev::Onset,
                        // A free voice does not lengthen a fixed tone.
                        (false, Slot::Sustain) if events.last().is_some_and(|e| e.is_fixed()) => {
                            Ev::Onset
                        }
                        (false, Slot::Sustain) => Ev::Sustain,
                        (false, Slot::Rest) => Ev::Rest,
                    };
                    events.push(event);
                }
                let mut wishes = vec![None; count];
                if let Some(intervals) = intervals {
                    let mut after = melody::CLASSES;
                    for s in 0..count {
                        match events[s] {
                            Ev::Onset => {
                                let mut k = s + 1;
                                while k < count && events[k] == Ev::Sustain {
                                    k += 1;
                                }
                                let wish = intervals.draw(after, (k - s) as Tick * SIXTEENTH, rng);
                                after = wish.class as usize;
                                wishes[s] = Some(wish);
                            }
                            Ev::Rest | Ev::FixedRest => after = melody::CLASSES,
                            // After a fixed tone the line has moved, by what interval is open.
                            Ev::Fixed(_) => after = 1,
                            _ => {}
                        }
                    }
                }
                VoicePlan {
                    low: voice.low,
                    high: voice.high,
                    events,
                    wishes,
                }
            })
            .collect()
    }

    /// The piece that a result of the search stands for: the fixed tones as given, the free
    /// ones from their onsets to the next event of their voice.
    pub fn realize(&self, plans: &[VoicePlan], found: &Found) -> Piece {
        let start = self.start();
        let voices: Vec<Voice> = self
            .voices
            .iter()
            .enumerate()
            .map(|(v, voice)| {
                let mut notes: Vec<Note> = voice.fixed.clone();
                for (s, pitch) in &found.onsets[v] {
                    if plans[v].events[*s] != Ev::Onset {
                        continue;
                    }
                    let mut k = s + 1;
                    while k < plans[v].events.len() && plans[v].events[k] == Ev::Sustain {
                        k += 1;
                    }
                    notes.push(Note {
                        on: start + *s as Tick * SIXTEENTH,
                        off: (start + k as Tick * SIXTEENTH).min(self.end),
                        pitch: *pitch,
                    });
                }
                notes.sort_by_key(|n| n.on);
                // A fixed tone that is not on the grid may reach into the slot of a free one.
                for i in 1..notes.len() {
                    if notes[i - 1].off > notes[i].on {
                        notes[i - 1].off = notes[i].on;
                    }
                }
                notes.retain(|n| n.off > n.on);
                Voice { notes }
            })
            .collect();
        let mut piece = Piece::new(&self.name, self.key, self.meter, voices);
        piece.origin = self.origin;
        piece.end = self.end;
        piece
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub seed: u64,
    pub beam: usize,
    /// How often a new rhythm is drawn after a dead end.
    pub attempts: usize,
    pub costs: Costs,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            seed: 1,
            beam: 400,
            attempts: 12,
            costs: Costs::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Setting {
    pub piece: Piece,
    pub cost: u32,
    /// Fixed tones after a free tone that broke a rule binding the free tones.
    pub forced: u32,
    /// Fixed tones that broke such a rule after a fixed tone or a rest.
    pub own: u32,
    /// Foreign fixed tones that the free tone after them did not resolve.
    pub excused: u32,
    /// Searches run, the last of them successful.
    pub attempts: usize,
    /// The slots at which the earlier searches ended, with the rule that refused most there.
    pub dead_ends: Vec<(usize, &'static str)>,
    pub seconds: f64,
}

/// Draws rhythms and searches until a setting is found or the attempts are used up.
pub fn set(
    layout: &Layout,
    harmony: &Harmony,
    table: &Table,
    intervals: Option<&melody::Table>,
    options: &Options,
) -> Result<Setting, Vec<(usize, &'static str)>> {
    let begun = crate::clock::Stopwatch::start();
    let mut dead_ends = Vec::new();
    for attempt in 0..options.attempts {
        let seed = options.seed.wrapping_add(1000 * attempt as u64);
        let mut rng = Rng::new(seed);
        let voices = layout.plan(table, intervals, &mut rng);
        let task = Task {
            meter: layout.meter,
            origin: layout.origin,
            start: layout.start(),
            harmony,
            voices,
            beam: options.beam,
            seed,
            costs: options.costs.clone(),
            invertible: layout
                .invertible
                .iter()
                .map(|(upper, lower, from, until)| {
                    let slot = |t: Tick| ((t - layout.start()) / SIXTEENTH).max(0) as usize;
                    (*upper, *lower, slot(*from), slot(*until))
                })
                .collect(),
            root_at: layout
                .final_hold
                .filter(|_| layout.root_at_hold)
                .map(|t| ((t - layout.start()) / SIXTEENTH).max(0) as usize),
            junctions: layout.junctions,
        };
        match search(&task) {
            Outcome::Found(found) => {
                return Ok(Setting {
                    piece: layout.realize(&task.voices, &found),
                    cost: found.cost,
                    forced: found.count.forced,
                    own: found.count.own,
                    excused: found.count.excused,
                    attempts: attempt + 1,
                    dead_ends,
                    seconds: begun.seconds(),
                });
            }
            Outcome::DeadEnd(slot, reasons) => {
                dead_ends.push((
                    slot,
                    reasons.first().map_or("nothing to choose from", |r| r.0),
                ));
            }
        }
    }
    Err(dead_ends)
}
