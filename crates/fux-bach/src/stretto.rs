//! The stretto finder: at which distances a subject goes against itself. A distance holds if
//! the two entries, harmonised together by the scaffold, break no prohibition.

use crate::check;
use crate::compose::{fitted, ranges};
use crate::rules::Profile;
use crate::scaffold::{self, Plan};
use crate::score::{Note, Piece, Voice};
use crate::themes::Subject;
use crate::time::Tick;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stretto {
    /// Ticks from the first entry to the second.
    pub delay: Tick,
    /// What the check of the two entries costs.
    pub cost: u32,
}

/// The two entries of a stretto as a piece: the lowest voice first, the top voice `delay` later.
pub fn entries(subject: &Subject, voices: usize, delay: Tick) -> Piece {
    let ranges = ranges(voices);
    let low = fitted(&subject.notes, ranges[voices - 1]);
    let high: Vec<Note> = fitted(&subject.notes, ranges[0])
        .iter()
        .map(|n| Note {
            on: n.on + delay,
            off: n.off + delay,
            pitch: n.pitch,
        })
        .collect();
    let mut piece = Piece::new(
        &subject.name,
        subject.key,
        subject.meter,
        vec![Voice { notes: high }, Voice { notes: low }],
    );
    let bar = subject.meter.bar();
    piece.end = (piece.end + bar - 1) / bar * bar;
    piece
}

/// Every distance at which the subject goes against itself at the octave: steps of a bar (half
/// a bar where the bar has two halves and the subject's entries may stand on either), with at
/// least a third of the subject overlapping.
pub fn all(subject: &Subject, voices: usize, profile: &Profile) -> Vec<Stretto> {
    let meter = subject.meter;
    let bar = meter.bar();
    let span = subject.notes.last().map_or(0, |n| n.off);
    let halves = (bar / meter.beat()) % 2 == 0 && subject.period % bar != 0;
    let step = if halves { bar / 2 } else { bar };
    let mut out = Vec::new();
    let mut delay = step;
    while delay <= span - span / 3 && delay < subject.period {
        let piece = entries(subject, voices, delay);
        let plan = Plan {
            key: subject.key,
            meter,
            origin: 0,
            end: piece.end,
            lines: piece.voices.iter().map(|v| v.notes.clone()).collect(),
            keys: vec![(0, subject.key)],
            cadences: Vec::new(),
            pedals: Vec::new(),
            must: Vec::new(),
            picardy: false,
            close: false,
            hold: None,
            lowest: Vec::new(),
            seed: 1,
        };
        let harmony = scaffold::generate(&plan, scaffold::embedded());
        let report = check::check(&piece, &harmony, profile);
        if report.forbidden() == 0 {
            out.push(Stretto {
                delay,
                cost: report.cost(),
            });
        }
        delay += step;
    }
    out
}

/// The closest stretto of a subject, if it has one.
pub fn find(subject: &Subject, voices: usize, profile: &Profile) -> Option<Stretto> {
    all(subject, voices, profile).into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
    use crate::themes::parse;
    use crate::time::Meter;

    #[test]
    fn a_canon_has_a_stretto() {
        // A rising scale in thirds with itself a bar later: c d e f | e f g a | g.
        let subject = Subject {
            name: "scale".to_string(),
            title: String::new(),
            key: Key::parse("C").unwrap(),
            meter: Meter::COMMON,
            notes: parse("c'4 d' e' f' | e' f' g' a' | g'1").unwrap(),
            period: 288,
            from: None,
            development: false,
        };
        let found = all(&subject, 3, &Profile::bach());
        assert!(found.iter().any(|s| s.delay == 96), "{found:?}");
        let piece = entries(&subject, 3, 96);
        assert_eq!(piece.voices[0].notes[0].on, 96);
        assert!(piece.voices[1].notes[0].pitch < piece.voices[0].notes[0].pitch);
    }
}
