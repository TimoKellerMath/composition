//! Pieces as monophonic voices of notes with onset and end; rests are the gaps.

use crate::key::Key;
use crate::pitch::Pitch;
use crate::time::{Meter, Tick};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Note {
    pub on: Tick,
    pub off: Tick,
    pub pitch: Pitch,
}

impl Note {
    pub const fn dur(&self) -> Tick {
        self.off - self.on
    }
}

/// One voice: notes in time order without overlap.
#[derive(Clone, Debug, Default)]
pub struct Voice {
    pub notes: Vec<Note>,
}

impl Voice {
    /// Index of the note sounding at `t`.
    pub fn index_at(&self, t: Tick) -> Option<usize> {
        let i = self.notes.partition_point(|n| n.on <= t);
        (i > 0 && self.notes[i - 1].off > t).then(|| i - 1)
    }

    pub fn at(&self, t: Tick) -> Option<&Note> {
        self.index_at(t).map(|i| &self.notes[i])
    }

    /// The note before `i` if it ends where `i` begins.
    pub fn before(&self, i: usize) -> Option<&Note> {
        let prev = self.notes.get(i.checked_sub(1)?)?;
        (prev.off == self.notes[i].on).then_some(prev)
    }

    /// The note after `i` if it begins where `i` ends.
    pub fn after(&self, i: usize) -> Option<&Note> {
        let next = self.notes.get(i + 1)?;
        (next.on == self.notes[i].off).then_some(next)
    }

    /// Whether note `i` repeats the pitch of the note directly before it.
    pub fn repeats(&self, i: usize) -> bool {
        self.before(i)
            .is_some_and(|p| p.pitch == self.notes[i].pitch)
    }

    pub fn end(&self) -> Tick {
        self.notes.last().map_or(0, |n| n.off)
    }

    /// Appends a note; a note that touches the last one with the same pitch stays separate.
    pub fn push(&mut self, on: Tick, off: Tick, pitch: Pitch) {
        debug_assert!(self.notes.last().is_none_or(|n| n.off <= on) && on < off);
        self.notes.push(Note { on, off, pitch });
    }

    /// The notes that begin in `[from, until)`.
    pub fn range(&self, from: Tick, until: Tick) -> &[Note] {
        let a = self.notes.partition_point(|n| n.on < from);
        let b = self.notes.partition_point(|n| n.on < until);
        &self.notes[a..b]
    }
}

#[derive(Clone, Debug)]
pub struct Piece {
    pub name: String,
    pub key: Key,
    pub meter: Meter,
    /// Tick of the first barline: 0, or the length of the upbeat.
    pub origin: Tick,
    /// Number of the bar that begins at `origin`.
    pub first_bar: i32,
    /// The voices from the top down.
    pub voices: Vec<Voice>,
    /// Sounding lines outside the voices: second parts of a divided voice and further chord tones.
    pub extras: Vec<Voice>,
    pub end: Tick,
}

impl Piece {
    pub fn new(name: &str, key: Key, meter: Meter, voices: Vec<Voice>) -> Piece {
        let end = voices.iter().map(Voice::end).max().unwrap_or(0);
        Piece {
            name: name.to_string(),
            key,
            meter,
            origin: 0,
            first_bar: 1,
            voices,
            extras: Vec::new(),
            end,
        }
    }

    /// Position inside its bar.
    pub fn pos(&self, t: Tick) -> Tick {
        (t - self.origin).rem_euclid(self.meter.bar())
    }

    pub fn weight(&self, t: Tick) -> u8 {
        self.meter.weight(self.pos(t))
    }

    /// Bar number (as notated) and position inside the bar.
    pub fn bar_of(&self, t: Tick) -> (i32, Tick) {
        let bar = (t - self.origin).div_euclid(self.meter.bar());
        (self.first_bar + bar, self.pos(t))
    }

    /// Tick of a notated bar number plus an offset.
    pub fn tick_of(&self, bar: i32, offset: Tick) -> Tick {
        self.origin + (bar - self.first_bar) * self.meter.bar() + offset
    }

    /// All ticks of the grid with the given step that lie on it relative to the barlines.
    pub fn grid(&self, step: Tick) -> impl Iterator<Item = Tick> + '_ {
        let first = self.origin.rem_euclid(step);
        (0..)
            .map(move |k| first + k * step)
            .take_while(|t| *t < self.end)
    }

    pub fn beats(&self) -> impl Iterator<Item = Tick> + '_ {
        self.grid(self.meter.beat())
    }

    /// The lowest sounding pitch among the voices at `t`, with its voice.
    pub fn bass_at(&self, t: Tick) -> Option<(usize, Note)> {
        self.voices
            .iter()
            .enumerate()
            .filter_map(|(v, voice)| voice.at(t).map(|n| (v, *n)))
            .min_by_key(|(v, n)| (n.pitch, std::cmp::Reverse(*v)))
    }

    pub fn note_count(&self) -> usize {
        self.voices.iter().map(|v| v.notes.len()).sum()
    }
}
