//! Melodic intervals as wishes: for every free onset a class of interval (repetition, step,
//! third, fourth or fifth, larger) and whether the line turns, drawn from a table that counts
//! what a set of fugues does after each class and for each note value. The search pays for
//! every wish it does not grant; so the shares of steps and leaps follow the corpus instead of
//! the cheapest path.

use crate::pitch::Iv;
use crate::rng::Rng;
use crate::score::Piece;
use crate::time::{QUARTER, SIXTEENTH, Tick};

pub const CLASSES: usize = 5;

/// 0 repetition, 1 step, 2 third, 3 fourth or fifth, 4 larger.
pub fn class(iv: Iv) -> usize {
    if iv.is_unison() {
        0
    } else if iv.is_step() {
        1
    } else {
        match iv.steps.abs() {
            0..=2 => 2,
            3 | 4 => 3,
            _ => 4,
        }
    }
}

/// 0 a sixteenth or less, 1 less than a quarter, 2 a quarter or more.
pub fn value(ticks: Tick) -> usize {
    if ticks <= SIXTEENTH {
        0
    } else if ticks < QUARTER {
        1
    } else {
        2
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Wish {
    pub class: u8,
    /// The line changes direction against the interval before.
    pub turn: bool,
    /// The very interval wished for, in diatonic steps: a tone of a figure. Then class and turn
    /// do not count.
    pub steps: Option<i8>,
}

/// Counts of (class, turn) after each class (index 5: after a rest) for each note value.
#[derive(Clone, Debug, Default)]
pub struct Table {
    counts: [[[f64; 2 * CLASSES]; CLASSES + 1]; 3],
}

impl Table {
    pub fn learn(&mut self, piece: &Piece) {
        for voice in &piece.voices {
            let mut before: Option<Iv> = None;
            for i in 1..voice.notes.len() {
                let Some(prev) = voice.before(i) else {
                    before = None;
                    continue;
                };
                let iv = prev.pitch.to(voice.notes[i].pitch);
                let turn =
                    before.is_some_and(|b| b.dir() != 0 && iv.dir() != 0 && b.dir() != iv.dir());
                let after = before.map_or(CLASSES, class);
                self.counts[value(voice.notes[i].dur())][after][2 * class(iv) + turn as usize] +=
                    1.0;
                before = Some(iv);
            }
        }
    }

    /// A wish for a tone of `ticks` after an interval of class `after` (5: after a rest).
    pub fn draw(&self, after: usize, ticks: Tick, rng: &mut Rng) -> Wish {
        let row = &self.counts[value(ticks)][after.min(CLASSES)];
        // Mixed with the counts over all values and classes, so that no row is empty.
        let mut weights = [0.0; 2 * CLASSES];
        let seen: f64 = row.iter().sum();
        for (k, weight) in weights.iter_mut().enumerate() {
            let all: f64 = self.counts.iter().flatten().map(|r| r[k]).sum();
            let total: f64 = self.counts.iter().flatten().flatten().sum::<f64>().max(1.0);
            *weight = (row[k] + 8.0 * all / total) / (seen + 8.0);
        }
        let k = rng.weighted(&weights).unwrap_or(2);
        Wish {
            class: (k / 2) as u8,
            turn: k % 2 == 1,
            steps: None,
        }
    }

    /// Everything counted, as one row of numbers: value, class before, then (class, turn).
    pub fn flat(&self) -> Vec<f64> {
        self.counts.iter().flatten().flatten().copied().collect()
    }

    /// A table from what `flat` gives.
    pub fn from_flat(numbers: &[f64]) -> Table {
        let mut table = Table::default();
        for (slot, number) in table.counts.iter_mut().flatten().flatten().zip(numbers) {
            *slot = *number;
        }
        table
    }

    pub fn add(&mut self, other: &Table) {
        for (slot, number) in self.counts.iter_mut().flatten().flatten().zip(other.flat()) {
            *slot += number;
        }
    }

    /// Shares of the classes over everything counted.
    pub fn shares(&self) -> [f64; CLASSES] {
        let mut out = [0.0; CLASSES];
        for row in self.counts.iter().flatten() {
            for (k, count) in row.iter().enumerate() {
                out[k / 2] += count;
            }
        }
        let total: f64 = out.iter().sum::<f64>().max(1.0);
        out.map(|c| c / total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
    use crate::pitch::Pitch;
    use crate::score::Voice;
    use crate::time::Meter;

    #[test]
    fn a_table_wishes_what_it_saw() {
        // A scale up and down in sixteenths: steps only, a turn at the top.
        let mut voice = Voice::default();
        let steps = [0, 1, 2, 3, 4, 3, 2, 1, 0];
        for (i, step) in steps.iter().enumerate() {
            let on = 6 * i as Tick;
            voice.push(on, on + 6, Pitch::new(*step, 0, 4));
        }
        let piece = Piece::new(
            "scale",
            Key::parse("C").unwrap(),
            Meter::COMMON,
            vec![voice],
        );
        let mut table = Table::default();
        table.learn(&piece);
        assert_eq!(table.shares(), [0.0, 1.0, 0.0, 0.0, 0.0]);
        let mut rng = Rng::new(3);
        let wishes: Vec<Wish> = (0..200).map(|_| table.draw(1, 6, &mut rng)).collect();
        assert!(wishes.iter().filter(|w| w.class == 1).count() > 180);
        let turns = wishes.iter().filter(|w| w.turn).count();
        assert!(turns > 5 && turns < 80);
        assert_eq!(class(Pitch::new(0, 0, 4).to(Pitch::new(4, 0, 4))), 3);
        assert_eq!(class(Pitch::new(0, 0, 4).to(Pitch::new(0, 0, 5))), 4);
    }
}
