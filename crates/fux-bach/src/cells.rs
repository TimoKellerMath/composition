//! Rhythm as cells: what a voice does in one pulse, tick by tick (a tone begins, a tone goes on,
//! silence). A table counts the cells of a set of fugues, each after the cell before it in the
//! same voice and beside what the other voices do; from it the rhythm of a free voice is drawn.
//!
//! The cells are those of `rhythm`, on ticks instead of sixteenths: thirty-second notes and
//! triplets of the corpus keep their places.

use std::collections::BTreeMap;

use crate::rng::Rng;
use crate::score::{Piece, Voice};
use crate::time::{SIXTEENTH, Tick};

/// The state of one tick.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Slot {
    Sustain,
    Onset,
    Rest,
}

/// A cell: the ticks of a pulse as digits to the base 3, the first tick lowest.
pub type Cell = u64;

/// The longest pulse whose cells fit.
pub const LONGEST: Tick = 40;

pub fn cell(slots: &[Slot]) -> Cell {
    debug_assert!(slots.len() <= LONGEST as usize);
    slots.iter().rev().fold(0, |c, s| 3 * c + *s as u64)
}

pub fn slot(cell: Cell, k: usize) -> Slot {
    match cell / 3u64.pow(k as u32) % 3 {
        0 => Slot::Sustain,
        1 => Slot::Onset,
        _ => Slot::Rest,
    }
}

/// A cell as text, run by run: `o12o12` two eighths, `-6o18` a tone going on for a sixteenth
/// and a dotted eighth, `r24` a quarter rest.
pub fn text(cell: Cell, pulse: Tick) -> String {
    let mut out = String::new();
    let mut k = 0;
    while k < pulse as usize {
        let first = slot(cell, k);
        let mut run = 1;
        while k + run < pulse as usize
            && slot(cell, k + run) == Slot::Sustain
            && first != Slot::Rest
            || k + run < pulse as usize && first == Slot::Rest && slot(cell, k + run) == Slot::Rest
        {
            run += 1;
        }
        out.push(match first {
            Slot::Sustain => '-',
            Slot::Onset => 'o',
            Slot::Rest => 'r',
        });
        out += &run.to_string();
        k += run;
    }
    out
}

/// The cell a text of `text` stands for.
pub fn parse(text: &str) -> Option<Cell> {
    let mut slots: Vec<Slot> = Vec::new();
    let mut rest = text;
    while let Some(letter) = rest.chars().next() {
        let digits: String = rest[1..].chars().take_while(char::is_ascii_digit).collect();
        let run: usize = digits.parse().ok()?;
        let (first, then) = match letter {
            '-' => (Slot::Sustain, Slot::Sustain),
            'o' => (Slot::Onset, Slot::Sustain),
            'r' => (Slot::Rest, Slot::Rest),
            _ => return None,
        };
        if run == 0 {
            return None;
        }
        slots.push(first);
        slots.extend(std::iter::repeat_n(then, run - 1));
        rest = &rest[1 + digits.len()..];
    }
    (slots.len() <= LONGEST as usize).then(|| cell(&slots))
}

/// The ticks of a voice from `from` on: an onset where a tone begins, a continuation where one
/// sounds, silence otherwise.
pub fn slots_of(voice: &Voice, from: Tick, count: usize) -> Vec<Slot> {
    (0..count)
        .map(|k| {
            let t = from + k as Tick;
            match voice.at(t) {
                Some(note) if note.on == t => Slot::Onset,
                Some(_) => Slot::Sustain,
                None => Slot::Rest,
            }
        })
        .collect()
}

/// The cells of every voice of a piece, pulse by pulse from the first pulse on the bar grid.
pub fn cells_of(piece: &Piece) -> Vec<Vec<Cell>> {
    let pulse = piece.meter.unit();
    let starts: Vec<Tick> = piece.grid(pulse).collect();
    piece
        .voices
        .iter()
        .map(|voice| {
            starts
                .iter()
                .map(|t| cell(&slots_of(voice, *t, pulse as usize)))
                .collect()
        })
        .collect()
}

/// Whether everything in a cell begins on the grid of thirty-second notes: no triplets.
pub fn binary(cell: Cell, pulse: Tick) -> bool {
    (0..pulse as usize).all(|k| {
        let begins = match slot(cell, k) {
            Slot::Onset => true,
            Slot::Rest => k == 0 || slot(cell, k - 1) != Slot::Rest,
            Slot::Sustain => false,
        };
        !begins || k % 3 == 0
    })
}

fn onsets(cell: Cell, pulse: Tick) -> usize {
    (0..pulse as usize)
        .filter(|k| slot(cell, *k) == Slot::Onset)
        .count()
}

fn silent(cell: Cell, pulse: Tick) -> bool {
    (0..pulse as usize).all(|k| slot(cell, k) == Slot::Rest)
}

/// What the other voices do in a pulse: 0 nothing struck, 1 up to half the sixteenths struck in
/// the busiest of them, 2 more.
pub fn activity(others: impl Iterator<Item = Cell>, pulse: Tick) -> u8 {
    match others.map(|c| onsets(c, pulse)).max().unwrap_or(0) {
        0 => 0,
        n if 2 * n <= (pulse / SIXTEENTH) as usize => 1,
        _ => 2,
    }
}

type Counts = BTreeMap<Cell, f64>;

/// Counts of cells in their contexts.
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub pulse: Tick,
    after_beside: BTreeMap<(Cell, u8), Counts>,
    after: BTreeMap<Cell, Counts>,
    beside: BTreeMap<u8, Counts>,
    all: Counts,
}

impl Table {
    pub fn new(pulse: Tick) -> Table {
        assert!(pulse > 0 && pulse <= LONGEST);
        Table {
            pulse,
            ..Table::default()
        }
    }

    /// Adds the cells of a piece with the same pulse.
    pub fn learn(&mut self, piece: &Piece) {
        let voices = cells_of(piece);
        let pulses = voices.first().map_or(0, Vec::len);
        for (v, line) in voices.iter().enumerate() {
            // Before its first tone and after its last a voice is not part of the texture.
            let Some(first) = line.iter().position(|c| !silent(*c, self.pulse)) else {
                continue;
            };
            let last = line
                .iter()
                .rposition(|c| !silent(*c, self.pulse))
                .unwrap_or(first);
            for i in first + 1..=last.min(pulses - 1) {
                let beside = activity(
                    voices
                        .iter()
                        .enumerate()
                        .filter(|(w, _)| *w != v)
                        .map(|(_, l)| l[i]),
                    self.pulse,
                );
                self.count(line[i - 1], beside, line[i], 1.0);
            }
        }
    }

    pub fn size(&self) -> usize {
        self.all.values().sum::<f64>() as usize
    }

    fn count(&mut self, before: Cell, beside: u8, now: Cell, times: f64) {
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

    /// Everything counted, as (cell before, activity beside, cell, count).
    pub fn counts(&self) -> Vec<(Cell, u8, Cell, f64)> {
        self.after_beside
            .iter()
            .flat_map(|((before, beside), row)| {
                row.iter()
                    .map(|(now, count)| (*before, *beside, *now, *count))
            })
            .collect()
    }

    pub fn from_counts(pulse: Tick, counts: &[(Cell, u8, Cell, f64)]) -> Table {
        let mut table = Table::new(pulse);
        for (before, beside, now, times) in counts {
            table.count(*before, *beside, *now, *times);
        }
        table
    }

    pub fn add(&mut self, other: &Table) {
        for (before, beside, now, times) in other.counts() {
            self.count(before, beside, now, times);
        }
    }

    /// Draws the cell of a pulse after `before` in the same voice, beside the given activity of
    /// the other voices. The four contexts are mixed, the most specific one weighing most where
    /// it has been seen often.
    pub fn draw(
        &self,
        before: Cell,
        beside: u8,
        allow: impl Fn(Cell) -> bool,
        rng: &mut Rng,
    ) -> Option<Cell> {
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
            for (c, count) in level.iter() {
                *mixed.entry(*c).or_default() += weight * count / seen;
            }
        }
        let sounding = slot(before, self.pulse as usize - 1) != Slot::Rest;
        let options: Vec<(Cell, f64)> = mixed
            .into_iter()
            // A cell that begins with a continuation needs a sounding tone before it.
            .filter(|(c, _)| (slot(*c, 0) != Slot::Sustain || sounding) && allow(*c))
            .collect();
        let weights: Vec<f64> = options.iter().map(|o| o.1).collect();
        rng.weighted(&weights).map(|i| options[i].0)
    }
}

/// The shares of note values in the subject of a fugue (its first entry), or in the whole piece
/// where no subject is found.
pub fn subject_profile(piece: &Piece) -> [f64; 3] {
    match crate::subject::from_entries(piece) {
        Some(exposition) => crate::rhythm::value_profile(exposition.subject.iter()),
        None => crate::rhythm::value_profile(piece.voices.iter().flat_map(|v| &v.notes)),
    }
}

/// The tables of cells and of intervals from the `nearest` pieces with the same pulse whose
/// subjects are closest to `profile` in their note values, without the piece named `except`:
/// like is compared with like, since the voices around a subject move faster than it does.
pub fn table_for<'a>(
    pieces: impl Iterator<Item = &'a Piece>,
    pulse: Tick,
    profile: [f64; 3],
    except: &str,
    nearest: usize,
) -> (Table, crate::melody::Table, Vec<String>) {
    let mut ranked: Vec<(f64, &Piece)> = pieces
        .filter(|p| p.meter.unit() == pulse && p.name != except)
        .map(|p| {
            let own = subject_profile(p);
            let distance: f64 = own.iter().zip(profile).map(|(a, b)| (a - b).abs()).sum();
            (distance, p)
        })
        .collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut table = Table::new(pulse);
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
    fn cells_read_and_print() {
        let mut slots = vec![Slot::Sustain; 24];
        slots[0] = Slot::Onset;
        slots[12] = Slot::Onset;
        let two = cell(&slots);
        assert_eq!(text(two, 24), "o12o12");
        assert_eq!(parse("o12o12"), Some(two));
        for line in ["-6o18", "r24", "o3o3o6r12", "-36", "o4o4o4o12"] {
            let pulse: Tick = if line == "-36" { 36 } else { 24 };
            assert_eq!(text(parse(line).unwrap(), pulse), line);
        }
        assert_eq!(parse("x3"), None);
        assert!(binary(parse("o3o3o6r12").unwrap(), 24));
        assert!(!binary(parse("o8o8o8").unwrap(), 24));
        assert!(!binary(parse("o4r20").unwrap(), 24));
        assert_eq!(onsets(parse("o3o3o6r12").unwrap(), 24), 3);
        assert!(silent(parse("r24").unwrap(), 24));
    }

    #[test]
    fn a_table_repeats_what_it_saw() {
        // One voice in eighths with a pair of thirty-seconds, one in quarters.
        let c = Pitch::new(0, 0, 4);
        let mut fast = Voice::default();
        let mut slow = Voice::default();
        for i in 0..32 {
            fast.push(12 * i, 12 * i + 9, c);
            fast.push(12 * i + 9, 12 * i + 12, c);
        }
        for i in 0..16 {
            slow.push(24 * i, 24 * i + 24, c);
        }
        let piece = Piece::new(
            "two",
            Key::parse("C").unwrap(),
            Meter::COMMON,
            vec![fast, slow],
        );
        let lines = cells_of(&piece);
        assert_eq!(text(lines[0][3], 24), "o9o3o9o3");
        assert_eq!(text(lines[1][3], 24), "o24");
        let mut table = Table::new(24);
        table.learn(&piece);
        assert_eq!(table.size(), 30);
        let mut rng = Rng::new(1);
        let drawn = table.draw(lines[0][0], 1, |_| true, &mut rng).unwrap();
        assert_eq!(text(drawn, 24), "o9o3o9o3");
        let again = Table::from_counts(24, &table.counts());
        assert_eq!(again.counts(), table.counts());
    }
}
