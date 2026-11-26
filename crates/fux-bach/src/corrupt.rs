//! Falsified versions of a piece for the counter-test of T3: one voice out of place.

use crate::score::{Piece, Voice};
use crate::time::Tick;

/// Voice `v` later by `by` ticks, cut at the end of the piece.
pub fn shifted(piece: &Piece, v: usize, by: Tick) -> Piece {
    let mut out = piece.clone();
    let voice = &mut out.voices[v];
    voice.notes.retain(|n| n.on + by < piece.end);
    for note in &mut voice.notes {
        note.on += by;
        note.off = (note.off + by).min(piece.end);
    }
    out
}

/// Voice `v` replaced by the notes of `donor` from its beginning, repeated or cut to the length
/// of the piece.
pub fn replaced(piece: &Piece, v: usize, donor: &Voice) -> Piece {
    let mut out = piece.clone();
    let mut voice = Voice::default();
    let length = donor.end();
    let mut base = 0;
    if length > 0 {
        'fill: loop {
            for note in &donor.notes {
                let on = base + note.on;
                if on >= piece.end {
                    break 'fill;
                }
                voice.push(on, (base + note.off).min(piece.end), note.pitch);
            }
            base += length;
        }
    }
    out.voices[v] = voice;
    out
}
