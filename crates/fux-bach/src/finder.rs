//! The subject finder: where a piece states a subject, from its notes alone. An entry is a run
//! of tones in one voice with the rhythm of the subject and its intervals, of which a tonal
//! answer may change a few by a step.

use crate::score::{Note, Piece};
use crate::time::Tick;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hit {
    pub voice: usize,
    /// The tick of the subject's tick 0: the bar line before its first tone.
    pub at: Tick,
    /// Diatonic steps above the subject as given.
    pub steps: i16,
}

/// Whether the tones repeat the subject: the same distances between onsets, every interval the
/// same number of steps, but for a few that differ by one (a tonal answer moves one or two tones
/// of the head, which changes up to four intervals; never more than a third of them).
fn matches(subject: &[Note], tones: &[Note]) -> bool {
    if tones.len() < subject.len() {
        return false;
    }
    let mut changed = 0;
    for k in 1..subject.len() {
        if subject[k].on - subject[k - 1].on != tones[k].on - tones[k - 1].on {
            return false;
        }
        let a = subject[k - 1].pitch.to(subject[k].pitch).steps;
        let b = tones[k - 1].pitch.to(tones[k].pitch).steps;
        match (a - b).abs() {
            0 => {}
            1 => changed += 1,
            _ => return false,
        }
    }
    changed <= 4 && 3 * changed <= subject.len()
}

/// Every entry of the subject in the piece.
pub fn find(piece: &Piece, subject: &[Note]) -> Vec<Hit> {
    let mut out = Vec::new();
    let Some(first) = subject.first() else {
        return out;
    };
    for (voice, line) in piece.voices.iter().enumerate() {
        for i in 0..line.notes.len() {
            if matches(subject, &line.notes[i..]) {
                out.push(Hit {
                    voice,
                    at: line.notes[i].on - first.on,
                    steps: first.pitch.to(line.notes[i].pitch).steps,
                });
            }
        }
    }
    out.sort_by_key(|h| (h.at, h.voice));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
    use crate::pitch::Iv;
    use crate::score::Voice;
    use crate::themes::parse;
    use crate::time::Meter;

    #[test]
    fn entries_are_found_where_they_stand() {
        let subject = parse("r8 c''16 b' c''8 g' as' c''16 b' c''8 d''").unwrap();
        // The subject in the upper voice from bar 2, a tonal answer (first leap a step smaller)
        // in the lower voice from bar 3, and a line that only begins like it.
        let mut upper: Vec<Note> = parse("c''4 d'' es'' f''").unwrap();
        upper.extend(subject.iter().map(|n| Note {
            on: n.on + 96,
            off: n.off + 96,
            pitch: n.pitch,
        }));
        let mut lower: Vec<Note> = parse("r1 r1").unwrap();
        let mut answer: Vec<Note> = subject
            .iter()
            .map(|n| Note {
                on: n.on + 192,
                off: n.off + 192,
                pitch: n.pitch.down(Iv::FOURTH),
            })
            .collect();
        answer[3].pitch = answer[3].pitch.up(Iv::new(2, 1));
        lower.append(&mut answer);
        let piece = Piece::new(
            "entries",
            Key::parse("c").unwrap(),
            Meter::COMMON,
            vec![Voice { notes: upper }, Voice { notes: lower }],
        );
        let hits = find(&piece, &subject);
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert_eq!((hits[0].voice, hits[0].at, hits[0].steps), (0, 96, 0));
        assert_eq!((hits[1].voice, hits[1].at, hits[1].steps), (1, 192, -3));
    }
}
