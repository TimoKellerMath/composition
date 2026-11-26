//! From what is given about a piece (tones, places of silence, ranges) to the task of the
//! machine and back from its result to a piece: the rhythm of the free voices is drawn from a
//! table of cells, tick by tick, the pitches are searched.

use crate::cells::{self, Cell, Slot, Table};
use crate::engine::{Event, Invertible, Kind, Machine, Outcome, Part, Tally, Task, Tones};
use crate::key::{Harmony, Key};
use crate::melody;
use crate::pitch::Pitch;
use crate::rng::Rng;
use crate::rules::{Profile, Rule};
use crate::score::{Note, Piece, Voice};
use crate::time::{Meter, Tick};

#[derive(Clone, Debug)]
pub struct VoiceDraft {
    pub low: Pitch,
    pub high: Pitch,
    /// The given tones.
    pub given: Vec<Note>,
    /// Where the voice is given: inside these spans only the given tones sound.
    pub spans: Vec<(Tick, Tick)>,
    /// Where the voice is free but silent.
    pub silent: Vec<(Tick, Tick)>,
    /// Given tones that begin inside these spans are tones of a pedal point.
    pub pedals: Vec<(Tick, Tick)>,
    /// Figures the voice has where it is free: their rhythm stands, their steps are wishes.
    pub figures: Vec<Figure>,
}

/// A figure for a free voice: from `at` to `until` tones begin at the `onsets` (ticks from
/// `at`) and last to the next; before the first of them the voice rests. `steps` are the
/// diatonic steps wished for into each of them (none: whatever the search likes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Figure {
    pub at: Tick,
    pub until: Tick,
    pub onsets: Vec<Tick>,
    pub steps: Vec<Option<i8>>,
}

#[derive(Clone, Debug)]
pub struct Draft {
    pub name: String,
    pub key: Key,
    pub meter: Meter,
    pub origin: Tick,
    pub end: Tick,
    pub voices: Vec<VoiceDraft>,
    /// From here on every sounding free voice holds one tone to the end.
    pub final_hold: Option<Tick>,
    pub invertible: Vec<Invertible>,
    /// The outer voices take the root of the chord at the final hold.
    pub root_at_hold: bool,
    /// No free tone before the final hold lasts longer than this.
    pub longest: Option<Tick>,
    /// A free voice does not fall silent right after a given tone: it strikes a tone of its own
    /// first (a voice does not break off where its entry ends), and that tone resolves the given
    /// one if it is foreign to its chord and any tone can. And it sounds in the final hold, also
    /// after a rest.
    pub goes_on: bool,
}

fn inside(spans: &[(Tick, Tick)], t: Tick) -> bool {
    spans.iter().any(|(from, until)| *from <= t && t < *until)
}

impl Draft {
    /// Tick of the first cell: the first pulse on the bar grid at or before tick 0.
    pub fn start(&self) -> Tick {
        let unit = self.meter.unit();
        self.origin - (self.origin + unit - 1).div_euclid(unit) * unit
    }

    /// Ticks from the start to the end, in whole pulses.
    fn ticks(&self) -> usize {
        let unit = self.meter.unit();
        let pulses = (self.end - self.start() + unit - 1) / unit;
        (pulses * unit) as usize
    }

    /// What is known of voice `v` before any rhythm is drawn: the ticks of its given spans and
    /// of its silences; `None` where it is free.
    fn known(&self, v: usize) -> Vec<Option<Slot>> {
        let voice = &self.voices[v];
        let line = Voice {
            notes: voice.given.clone(),
        };
        let start = self.start();
        (0..self.ticks())
            .map(|i| {
                let t = start + i as Tick;
                if inside(&voice.spans, t) {
                    Some(match line.at(t) {
                        Some(note) if note.on == t => Slot::Onset,
                        Some(_) => Slot::Sustain,
                        None => Slot::Rest,
                    })
                } else if inside(&voice.silent, t) || t < 0 {
                    Some(Slot::Rest)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Draws the rhythm of the free voices and their wishes and returns the events of every
    /// voice.
    pub fn plan(&self, table: &Table, intervals: &melody::Table, rng: &mut Rng) -> Vec<Part> {
        let per = table.pulse as usize;
        debug_assert_eq!(table.pulse, self.meter.unit());
        let count = self.ticks();
        let pulses = count / per;
        let start = self.start();
        let mut known: Vec<Vec<Option<Slot>>> =
            (0..self.voices.len()).map(|v| self.known(v)).collect();
        // The figures: their rhythm stands where the voice is free for the whole of one.
        let mut wished: Vec<Vec<(Tick, Option<i8>)>> = vec![Vec::new(); self.voices.len()];
        for (v, voice) in self.voices.iter().enumerate() {
            for figure in &voice.figures {
                if figure.at < start || figure.until <= figure.at {
                    continue;
                }
                let (a, b) = (
                    (figure.at - start) as usize,
                    (figure.until - start) as usize,
                );
                let Some(lead) = figure.onsets.first().map(|o| *o as usize) else {
                    continue;
                };
                if b > count
                    || known[v][a..b].iter().any(Option::is_some)
                    || figure.onsets.iter().any(|o| *o < 0 || a + *o as usize >= b)
                {
                    continue;
                }
                for (k, slot) in known[v][a..b].iter_mut().enumerate() {
                    *slot = Some(if k < lead { Slot::Rest } else { Slot::Sustain });
                }
                for (k, onset) in figure.onsets.iter().enumerate() {
                    known[v][a + *onset as usize] = Some(Slot::Onset);
                    wished[v].push((figure.at + onset, figure.steps.get(k).copied().flatten()));
                }
            }
        }
        let known = known;
        // The ticks of every voice as far as they are settled; free ticks count as silence until
        // their voice has been drawn.
        let mut settled: Vec<Vec<Slot>> = known
            .iter()
            .map(|line| line.iter().map(|s| s.unwrap_or(Slot::Rest)).collect())
            .collect();
        // Triplets only where the given tones have some: otherwise every free tone begins on
        // the grid of thirty-second notes, so that every value can be written.
        let off_grid = |t: Tick| (t - self.origin).rem_euclid(3) != 0;
        let triplets = self
            .voices
            .iter()
            .any(|v| v.given.iter().any(|n| off_grid(n.on) || off_grid(n.off)));
        let cell_at =
            |line: &[Slot], i: usize| -> Cell { cells::cell(&line[i * per..(i + 1) * per]) };
        let mut fallback = vec![Slot::Sustain; per];
        fallback[0] = Slot::Onset;
        let fallback = cells::cell(&fallback);
        // Pulse by pulse through the voices: a voice sees what the others do in this pulse as
        // far as it is settled, and for those still to be drawn what they did in the pulse
        // before.
        let silent = cells::cell(&vec![Slot::Rest; per]);
        let mut before: Vec<Cell> = vec![silent; self.voices.len()];
        for i in 0..pulses {
            let range = i * per..(i + 1) * per;
            for v in 0..self.voices.len() {
                if known[v][range.clone()].iter().any(Option::is_none) {
                    let beside = cells::activity(
                        (0..self.voices.len()).filter(|w| *w != v).map(|w| {
                            let settled_now =
                                w < v || known[w][range.clone()].iter().all(Option::is_some);
                            if settled_now {
                                cell_at(&settled[w], i)
                            } else {
                                before[w]
                            }
                        }),
                        table.pulse,
                    );
                    let drawn = table
                        .draw(
                            before[v],
                            beside,
                            |c| triplets || cells::binary(c, table.pulse),
                            rng,
                        )
                        .unwrap_or(fallback);
                    for (k, s) in range.clone().enumerate() {
                        if known[v][s].is_none() {
                            settled[v][s] = cells::slot(drawn, k);
                        }
                    }
                }
                // A continuation needs a tone before it.
                for s in range.clone() {
                    if settled[v][s] == Slot::Sustain && (s == 0 || settled[v][s - 1] == Slot::Rest)
                    {
                        settled[v][s] = Slot::Rest;
                    }
                }
            }
            for v in 0..self.voices.len() {
                before[v] = cell_at(&settled[v], i);
            }
        }
        for v in 0..self.voices.len() {
            if self.goes_on {
                // Where a rest was drawn right after a given tone, the voice strikes a tone of
                // its own first: to the end of the beat, or of the next if that is too near.
                let beat = self.meter.beat();
                for s in 1..count {
                    let after_given = matches!(known[v][s - 1], Some(Slot::Onset | Slot::Sustain));
                    let t = start + s as Tick;
                    if settled[v][s] != Slot::Rest
                        || known[v][s].is_some()
                        || !after_given
                        || off_grid(t)
                    {
                        continue;
                    }
                    let in_beat = (t - self.origin).rem_euclid(beat);
                    let mut left = beat - in_beat;
                    if left < crate::time::SIXTEENTH {
                        left += beat;
                    }
                    settled[v][s] = Slot::Onset;
                    for k in s + 1..(s + left as usize).min(count) {
                        if settled[v][k] != Slot::Rest || known[v][k].is_some() {
                            break;
                        }
                        settled[v][k] = Slot::Sustain;
                    }
                }
            }
            if let Some(longest) = self.longest {
                // A free tone that would last longer is struck again.
                let limit = longest.max(1) as usize;
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
                let first = (hold - start).max(0) as usize;
                let free = (first..count).all(|s| known[v][s].is_none());
                if v == 0 && self.root_at_hold && free && first < count {
                    // The top voice arrives in the close: where a rest was drawn for the end of
                    // the beat before it, the voice strikes a tone there, from which it can step
                    // into its last.
                    let from = first.saturating_sub(self.meter.beat() as usize);
                    let mut s = first;
                    while s > from && settled[v][s - 1] == Slot::Rest && known[v][s - 1].is_none() {
                        s -= 1;
                    }
                    while s < first && off_grid(start + s as Tick) {
                        s += 1;
                    }
                    if s < first {
                        settled[v][s] = Slot::Onset;
                        for slot in &mut settled[v][s + 1..first] {
                            *slot = Slot::Sustain;
                        }
                    }
                }
                // A voice that goes on closes with the others, whatever rest was drawn for it.
                let sounding = self.goes_on || (first > 0 && settled[v][first - 1] != Slot::Rest);
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
                    notes: voice.given.clone(),
                };
                let mut events: Vec<Event> = Vec::new();
                // A tone is in force, and it is a given one.
                let mut sounding = false;
                let mut given = false;
                for (s, slot) in settled[v].iter().enumerate() {
                    let t = start + s as Tick;
                    if t >= self.end {
                        break;
                    }
                    match (inside(&voice.spans, t), *slot) {
                        (true, Slot::Onset) => {
                            if let Some(note) = line.at(t) {
                                let mut event = Event::given(t, note.pitch);
                                event.pedal = inside(&voice.pedals, t);
                                events.push(event);
                                sounding = true;
                                given = true;
                            }
                        }
                        (true, Slot::Sustain) => {}
                        (_, Slot::Rest) => {
                            if sounding {
                                events.push(Event::rest(t));
                                sounding = false;
                            }
                        }
                        (false, Slot::Onset) => {
                            events.push(Event::free(t, None));
                            sounding = true;
                            given = false;
                        }
                        // A free voice does not lengthen a given tone: it strikes anew, or
                        // falls silent where no written value begins.
                        (false, Slot::Sustain) => {
                            if sounding && given {
                                if triplets || !off_grid(t) {
                                    events.push(Event::free(t, None));
                                    given = false;
                                } else {
                                    events.push(Event::rest(t));
                                    sounding = false;
                                }
                            }
                        }
                    }
                }
                let mut after = melody::CLASSES;
                for e in 0..events.len() {
                    match events[e].kind {
                        Kind::Free => {
                            let end = events.get(e + 1).map_or(self.end, |n| n.at);
                            let wish = intervals.draw(after, end - events[e].at, rng);
                            after = wish.class as usize;
                            events[e].wish = Some(wish);
                        }
                        Kind::Rest => after = melody::CLASSES,
                        // After a given tone the line has moved, by what interval is open.
                        Kind::Given(_) => after = 1,
                    }
                }
                // The tones of a figure wish for its steps.
                for (at, steps) in &wished[v] {
                    if let Some(event) = events
                        .iter_mut()
                        .find(|e| e.at == *at && e.kind == Kind::Free)
                        && let Some(wish) = event.wish.as_mut()
                    {
                        wish.steps = *steps;
                    }
                }
                // The top voice comes to the close by steps: the tone before its last wishes
                // for one as well, whatever was drawn (a leap drawn there once sent the voice
                // an octave down and under its neighbour in the final chord).
                if v == 0
                    && self.root_at_hold
                    && let Some(hold) = self.final_hold
                    && let Some(event) = events
                        .iter_mut()
                        .rev()
                        .find(|e| e.at < hold && e.kind == Kind::Free)
                    && let Some(wish) = event.wish.as_mut()
                {
                    wish.class = 1;
                    wish.steps = None;
                }
                Part {
                    low: voice.low,
                    high: voice.high,
                    events,
                }
            })
            .collect()
    }

    /// The task of the machine for these events.
    pub fn task<'a>(
        &self,
        parts: Vec<Part>,
        harmony: &'a Harmony,
        profile: &'a Profile,
        beam: usize,
        seed: u64,
    ) -> Task<'a> {
        Task {
            meter: self.meter,
            origin: self.origin,
            end: self.end,
            harmony,
            parts,
            profile,
            beam,
            seed,
            invertible: self.invertible.clone(),
            root_at: self.final_hold.filter(|_| self.root_at_hold),
            resolve_given: self.goes_on,
        }
    }

    /// The piece that the tones of a setting stand for: every tone from its event to the next
    /// event of its voice.
    pub fn realize(&self, parts: &[Part], tones: &Tones) -> Piece {
        let voices: Vec<Voice> = parts
            .iter()
            .enumerate()
            .map(|(v, part)| {
                let mut notes: Vec<Note> = Vec::new();
                for (e, event) in part.events.iter().enumerate() {
                    let pitch = match event.kind {
                        Kind::Rest => continue,
                        Kind::Given(p) => p,
                        Kind::Free => match tones[v][e] {
                            Some(p) => p,
                            None => continue,
                        },
                    };
                    let off = part.events.get(e + 1).map_or(self.end, |n| n.at);
                    if off > event.at {
                        notes.push(Note {
                            on: event.at,
                            off: off.min(self.end),
                            pitch,
                        });
                    }
                }
                Voice { notes }
            })
            .collect();
        let mut piece = Piece::new(&self.name, self.key, self.meter, voices);
        piece.origin = self.origin;
        piece.end = self.end;
        piece
    }

    /// Whether a tone of a realized piece was given.
    pub fn is_given(&self, v: usize, note: &Note) -> bool {
        self.voices[v]
            .given
            .iter()
            .any(|g| g.on == note.on && g.pitch == note.pitch)
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub seed: u64,
    pub beam: usize,
    /// How often a new rhythm is drawn after a dead end or a fault at a junction.
    pub attempts: usize,
    pub profile: Profile,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            seed: 1,
            beam: 400,
            attempts: 12,
            profile: Profile::bach(),
        }
    }
}

/// Where a search ended, and the rule that forbade most there.
pub type DeadEnd = (Tick, Option<Rule>);

#[derive(Clone, Debug)]
pub struct Setting {
    pub piece: Piece,
    pub parts: Vec<Part>,
    pub tones: Tones,
    pub tally: Tally,
    /// Searches run.
    pub attempts: usize,
    pub dead_ends: Vec<DeadEnd>,
    pub seconds: f64,
}

/// Draws rhythms and searches until a setting without a fault at a junction is found or the
/// attempts are used up; then the setting with the fewest such faults is returned.
pub fn set(
    draft: &Draft,
    harmony: &Harmony,
    table: &Table,
    intervals: &melody::Table,
    options: &Options,
) -> Result<Setting, Vec<DeadEnd>> {
    let begun = crate::clock::Stopwatch::start();
    let mut dead_ends = Vec::new();
    let mut best: Option<Setting> = None;
    let mut searches = 0;
    for attempt in 0..options.attempts {
        searches += 1;
        let seed = options.seed.wrapping_add(1000 * attempt as u64);
        let mut rng = Rng::new(seed);
        let parts = draft.plan(table, intervals, &mut rng);
        let task = draft.task(parts, harmony, &options.profile, options.beam, seed);
        match Machine::new(&task).search() {
            Outcome::Found(found) => {
                let better = best.as_ref().is_none_or(|b| {
                    (found.tally.count.junction, found.tally.total())
                        < (b.tally.count.junction, b.tally.total())
                });
                if better {
                    best = Some(Setting {
                        piece: draft.realize(&task.parts, &found.tones),
                        parts: task.parts,
                        tones: found.tones,
                        tally: found.tally,
                        attempts: 0,
                        dead_ends: Vec::new(),
                        seconds: 0.0,
                    });
                }
                if best.as_ref().is_some_and(|b| b.tally.count.junction == 0) {
                    break;
                }
            }
            Outcome::DeadEnd(at, reasons) => {
                dead_ends.push((at, reasons.first().map(|r| r.0)));
            }
        }
    }
    match best {
        Some(mut setting) => {
            setting.attempts = searches;
            setting.dead_ends = dead_ends;
            setting.seconds = begun.seconds();
            Ok(setting)
        }
        None => Err(dead_ends),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::parse;

    /// A figure gives a free voice its rhythm and wishes for its steps; where the voice is
    /// taken the figure does not stand.
    #[test]
    fn a_figure_sets_the_rhythm_and_wishes_for_its_steps() {
        let key = Key::parse("C").unwrap();
        let meter = Meter::COMMON;
        let subject = parse("c''4 d'' e'' f'' | g''1 | c''1").unwrap();
        let low = Pitch::new(0, 0, 3);
        let high = Pitch::new(0, 0, 5);
        let figure = |at: Tick| Figure {
            at,
            until: at + 48,
            onsets: vec![12, 24, 36],
            steps: vec![None, Some(1), Some(-2)],
        };
        let draft = Draft {
            name: "figure".to_string(),
            key,
            meter,
            origin: 0,
            end: 288,
            voices: vec![
                VoiceDraft {
                    low: Pitch::new(0, 0, 4),
                    high: Pitch::new(0, 0, 6),
                    given: subject.clone(),
                    spans: vec![(0, 288)],
                    silent: Vec::new(),
                    pedals: Vec::new(),
                    figures: Vec::new(),
                },
                VoiceDraft {
                    low,
                    high,
                    given: parse("c'4").unwrap(),
                    spans: vec![(0, 24)],
                    silent: Vec::new(),
                    pedals: Vec::new(),
                    // The first stands, the second lies over the given tone and does not.
                    figures: vec![figure(96), figure(0)],
                },
            ],
            final_hold: None,
            invertible: Vec::new(),
            root_at_hold: false,
            longest: None,
            goes_on: false,
        };
        let tables = crate::tables::embedded();
        let (table, intervals, _) = tables.nearest_cells(meter.unit(), [0.3, 0.4, 0.3], "", 8);
        let parts = draft.plan(&table, &intervals, &mut Rng::new(1));
        let events = &parts[1].events;
        let at = |t: Tick| events.iter().find(|e| e.at == t);
        // A rest to the first tone, then the three tones, the last up to the end of the unit.
        assert!(at(96).is_some_and(|e| e.kind == Kind::Rest), "{events:?}");
        for (tick, steps) in [(108, None), (120, Some(1)), (132, Some(-2))] {
            let event = at(tick).unwrap_or_else(|| panic!("no event at {tick}: {events:?}"));
            assert_eq!(event.kind, Kind::Free, "{tick}");
            assert_eq!(event.wish.and_then(|w| w.steps), steps, "{tick}");
        }
        assert!(
            events.iter().all(|e| e.at <= 132 || e.at >= 144),
            "{events:?}"
        );
        // No wish of a figure anywhere else.
        assert!(
            events
                .iter()
                .filter(|e| e.at < 96 || e.at >= 144)
                .all(|e| e.wish.is_none_or(|w| w.steps.is_none()))
        );
    }
}
