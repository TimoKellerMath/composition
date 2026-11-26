//! Rhythm as patterns per pulse (the quarter, in compound meters the dotted quarter) on a grid
//! of sixteenths: every slot of a pulse holds an onset, the continuation of a tone, or silence.
//! A table counts the patterns of a set of fugues, each after the pattern before it in the same
//! voice and beside what the other voices do; from it the rhythm of a free voice is drawn.

use std::collections::BTreeMap;

use crate::rng::Rng;
use crate::score::{Piece, Voice};
use crate::time::{SIXTEENTH, Tick};

/// The state of one slot.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Slot {
    Sustain,
    Onset,
    Rest,
}

/// A pattern: the slots of a pulse as digits to the base 3, the first slot lowest.
pub type Pattern = u16;

pub fn pattern(slots: &[Slot]) -> Pattern {
    slots.iter().rev().fold(0, |p, s| 3 * p + *s as u16)
}

pub fn slot(pattern: Pattern, k: usize) -> Slot {
    match pattern / 3u16.pow(k as u32) % 3 {
        0 => Slot::Sustain,
        1 => Slot::Onset,
        _ => Slot::Rest,
    }
}

/// `N-N-`, `RRN-`, `----`.
pub fn text(pattern: Pattern, slots: usize) -> String {
    (0..slots)
        .map(|k| match slot(pattern, k) {
            Slot::Sustain => '-',
            Slot::Onset => 'N',
            Slot::Rest => 'R',
        })
        .collect()
}

/// The slots of a voice from `from` on: an onset where a tone begins inside the slot, a
/// continuation where one sounds at its start, silence otherwise.
pub fn slots_of(voice: &Voice, from: Tick, count: usize) -> Vec<Slot> {
    (0..count)
        .map(|k| {
            let t = from + k as Tick * SIXTEENTH;
            if !voice.range(t, t + SIXTEENTH).is_empty() {
                Slot::Onset
            } else if voice.at(t).is_some() {
                Slot::Sustain
            } else {
                Slot::Rest
            }
        })
        .collect()
}

/// The patterns of every voice of a piece, pulse by pulse from the first pulse on the bar grid.
pub fn patterns_of(piece: &Piece) -> Vec<Vec<Pattern>> {
    let pulse = piece.meter.pulse();
    let per = (pulse / SIXTEENTH) as usize;
    let starts: Vec<Tick> = piece.grid(pulse).collect();
    piece
        .voices
        .iter()
        .map(|voice| {
            starts
                .iter()
                .map(|t| pattern(&slots_of(voice, *t, per)))
                .collect()
        })
        .collect()
}

fn onsets(pattern: Pattern, slots: usize) -> usize {
    (0..slots)
        .filter(|k| slot(pattern, *k) == Slot::Onset)
        .count()
}

fn silent(pattern: Pattern, slots: usize) -> bool {
    (0..slots).all(|k| slot(pattern, k) == Slot::Rest)
}

/// What the other voices do in a pulse: 0 nothing struck, 1 up to half the slots struck in the
/// busiest of them, 2 more.
pub fn activity(others: impl Iterator<Item = Pattern>, slots: usize) -> u8 {
    match others.map(|p| onsets(p, slots)).max().unwrap_or(0) {
        0 => 0,
        n if 2 * n <= slots => 1,
        _ => 2,
    }
}

type Counts = BTreeMap<Pattern, f64>;

/// Counts of patterns in their contexts.
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub slots: usize,
    after_beside: BTreeMap<(Pattern, u8), Counts>,
    after: BTreeMap<Pattern, Counts>,
    beside: BTreeMap<u8, Counts>,
    all: Counts,
}

impl Table {
    pub fn new(slots: usize) -> Table {
        Table {
            slots,
            ..Table::default()
        }
    }

    /// Adds the patterns of a piece with the same pulse.
    pub fn learn(&mut self, piece: &Piece) {
        let voices = patterns_of(piece);
        let pulses = voices.first().map_or(0, Vec::len);
        for (v, line) in voices.iter().enumerate() {
            // Before its first tone and after its last a voice is not part of the texture.
            let Some(first) = line.iter().position(|p| !silent(*p, self.slots)) else {
                continue;
            };
            let last = line
                .iter()
                .rposition(|p| !silent(*p, self.slots))
                .unwrap_or(first);
            for i in first + 1..=last.min(pulses - 1) {
                let beside = activity(
                    voices
                        .iter()
                        .enumerate()
                        .filter(|(w, _)| *w != v)
                        .map(|(_, l)| l[i]),
                    self.slots,
                );
                self.count(line[i - 1], beside, line[i], 1.0);
            }
        }
    }

    pub fn size(&self) -> usize {
        self.all.values().sum::<f64>() as usize
    }

    /// Counts one pattern in its context.
    fn count(&mut self, before: Pattern, beside: u8, now: Pattern, times: f64) {
        *self
            .after_beside
            .entry((before, beside))
            .or_default()
            .entry(now)
            .or_default() += times;
        *self
            .after
            .entry(before)
            .or_default()
            .entry(now)
            .or_default() += times;
        *self
            .beside
            .entry(beside)
            .or_default()
            .entry(now)
            .or_default() += times;
        *self.all.entry(now).or_default() += times;
    }

    /// Everything counted, as (pattern before, activity beside, pattern, count).
    pub fn counts(&self) -> Vec<(Pattern, u8, Pattern, f64)> {
        self.after_beside
            .iter()
            .flat_map(|((before, beside), row)| {
                row.iter()
                    .map(|(now, count)| (*before, *beside, *now, *count))
            })
            .collect()
    }

    /// A table from what `counts` gives.
    pub fn from_counts(slots: usize, counts: &[(Pattern, u8, Pattern, f64)]) -> Table {
        let mut table = Table::new(slots);
        for (before, beside, now, times) in counts {
            table.count(*before, *beside, *now, *times);
        }
        table
    }

    /// Adds the counts of another table with the same pulse.
    pub fn add(&mut self, other: &Table) {
        for (before, beside, now, times) in other.counts() {
            self.count(before, beside, now, times);
        }
    }

    /// Draws the pattern of a pulse after `before` in the same voice, beside the given activity
    /// of the other voices. `allow` filters the patterns (for instance: not silent). The four
    /// contexts are mixed, the most specific one weighing most where it has been seen often.
    pub fn draw(
        &self,
        before: Pattern,
        beside: u8,
        allow: impl Fn(Pattern) -> bool,
        rng: &mut Rng,
    ) -> Option<Pattern> {
        let empty = Counts::new();
        let levels = [
            self.after_beside.get(&(before, beside)).unwrap_or(&empty),
            self.after.get(&before).unwrap_or(&empty),
            self.beside.get(&beside).unwrap_or(&empty),
            &self.all,
        ];
        let mut mixed: Counts = Counts::new();
        let mut left = 1.0;
        for (i, level) in levels.iter().enumerate() {
            let seen: f64 = level.values().sum();
            if seen <= 0.0 {
                continue;
            }
            // A context seen n times gets the share n / (n + 8) of what is left.
            let weight = if i + 1 == levels.len() {
                left
            } else {
                left * seen / (seen + 8.0)
            };
            left -= weight;
            for (p, count) in level.iter() {
                *mixed.entry(*p).or_default() += weight * count / seen;
            }
        }
        let options: Vec<(Pattern, f64)> = mixed
            .into_iter()
            .filter(|(p, _)| {
                // A pattern that begins with a continuation needs a sounding tone before it.
                let continues = slot(*p, 0) == Slot::Sustain;
                let sounding = slot(before, self.slots - 1) != Slot::Rest;
                (!continues || sounding) && allow(*p)
            })
            .collect();
        let weights: Vec<f64> = options.iter().map(|o| o.1).collect();
        rng.weighted(&weights).map(|i| options[i].0)
    }
}

/// Shares of short (a sixteenth or less), middle and long (a quarter or more) values.
pub fn value_profile<'a>(notes: impl Iterator<Item = &'a crate::score::Note>) -> [f64; 3] {
    let mut counts = [0.0; 3];
    for note in notes {
        let k = if note.dur() <= SIXTEENTH {
            0
        } else if note.dur() < 4 * SIXTEENTH {
            1
        } else {
            2
        };
        counts[k] += 1.0;
    }
    let total: f64 = counts.iter().sum::<f64>().max(1.0);
    counts.map(|c| c / total)
}

/// The tables of patterns and of intervals from the `nearest` pieces with the same pulse whose
/// note values are closest to `profile`, without the piece named `except`.
pub fn table_for<'a>(
    pieces: impl Iterator<Item = &'a Piece>,
    pulse: Tick,
    profile: [f64; 3],
    except: &str,
    nearest: usize,
) -> (Table, crate::melody::Table, Vec<String>) {
    let mut ranked: Vec<(f64, &Piece)> = pieces
        .filter(|p| p.meter.pulse() == pulse && p.name != except)
        .map(|p| {
            let own = value_profile(p.voices.iter().flat_map(|v| &v.notes));
            let distance: f64 = own.iter().zip(profile).map(|(a, b)| (a - b).abs()).sum();
            (distance, p)
        })
        .collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut table = Table::new((pulse / SIXTEENTH) as usize);
    let mut intervals = crate::melody::Table::default();
    let mut names = Vec::new();
    for (_, piece) in ranked.into_iter().take(nearest) {
        table.learn(piece);
        intervals.learn(piece);
        names.push(piece.name.clone());
    }
    (table, intervals, names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
    use crate::pitch::Pitch;
    use crate::time::Meter;

    #[test]
    fn patterns_round_trip() {
        let slots = [Slot::Onset, Slot::Sustain, Slot::Rest, Slot::Onset];
        let p = pattern(&slots);
        assert_eq!(text(p, 4), "N-RN");
        assert_eq!((0..4).map(|k| slot(p, k)).collect::<Vec<_>>(), slots);
        assert_eq!(onsets(p, 4), 2);
        assert!(silent(pattern(&[Slot::Rest; 4]), 4));
    }

    #[test]
    fn a_table_repeats_what_it_saw() {
        // One voice in eighths, one in quarters.
        let c = Pitch::new(0, 0, 4);
        let mut eighths = Voice::default();
        let mut quarters = Voice::default();
        for i in 0..32 {
            eighths.push(12 * i, 12 * i + 12, c);
        }
        for i in 0..16 {
            quarters.push(24 * i, 24 * i + 24, c);
        }
        let piece = Piece::new(
            "two",
            Key::parse("C").unwrap(),
            Meter::COMMON,
            vec![eighths, quarters],
        );
        let lines = patterns_of(&piece);
        assert_eq!(text(lines[0][3], 4), "N-N-");
        assert_eq!(text(lines[1][3], 4), "N---");
        let mut table = Table::new(4);
        table.learn(&piece);
        assert_eq!(table.size(), 30);
        let mut rng = Rng::new(1);
        let drawn = table.draw(lines[0][0], 1, |_| true, &mut rng).unwrap();
        assert_eq!(text(drawn, 4), "N-N-");
        assert_eq!(value_profile(piece.voices[0].notes.iter()), [0.0, 1.0, 0.0]);
    }
}
