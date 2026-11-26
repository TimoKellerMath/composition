//! Motivic work, measured (protocol of phase B5). A figure is a run of consecutive tones of one
//! voice, read as the diatonic steps between them and the ticks from onset to onset: transposed
//! or mirrored it stays the same figure. Three shares of the free tones of a piece (those that
//! are neither subject nor countersubject) say how much of them is made of the subject's
//! figures, how much recurs, and how much is sequence.

use std::collections::{HashMap, HashSet};

use crate::score::{Note, Piece};
use crate::time::Tick;

/// The steps between consecutive tones and the ticks from onset to onset.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Figure {
    pub steps: Vec<i16>,
    pub spans: Vec<Tick>,
}

impl Figure {
    /// The figure of consecutive notes (the length of the last one does not count).
    pub fn of(notes: &[Note]) -> Figure {
        Figure {
            steps: notes
                .windows(2)
                .map(|w| w[1].pitch.dia - w[0].pitch.dia)
                .collect(),
            spans: notes.windows(2).map(|w| w[1].on - w[0].on).collect(),
        }
    }

    pub fn mirror(&self) -> Figure {
        Figure {
            steps: self.steps.iter().map(|s| -s).collect(),
            spans: self.spans.clone(),
        }
    }

    /// The figure or its mirror, whichever comes first: the same for both.
    pub fn canonical(&self) -> Figure {
        let mirror = self.mirror();
        if mirror < *self { mirror } else { self.clone() }
    }

    /// A figure without a shape of its own: the same step and the same value throughout (a
    /// scale, a repeated tone). It counts in no measure.
    pub fn plain(&self) -> bool {
        self.steps.windows(2).all(|w| w[0] == w[1]) && self.spans.windows(2).all(|w| w[0] == w[1])
    }
}

/// The figures of `n` tones in a line of material (a subject, a countersubject), with their
/// mirrors; tones on either side of a rest make no figure.
pub fn figures_of(line: &[Note], n: usize, out: &mut HashSet<Figure>) {
    for window in line.windows(n) {
        if window.windows(2).any(|w| w[0].off != w[1].on) {
            continue;
        }
        let figure = Figure::of(window);
        if !figure.plain() {
            out.insert(figure.canonical());
        }
    }
}

/// The three shares, as counts of free tones.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Measures {
    /// Free tones.
    pub free: usize,
    /// Those in a figure of the material.
    pub derived: usize,
    /// Those in a figure that the free tones have elsewhere as well.
    pub recurring: usize,
    /// Those in a figure that their own voice has again within two bars, at another pitch.
    pub sequence: usize,
}

pub const NAMES: [&str; 3] = ["Themenbezug", "Wiederkehr", "Sequenz"];

impl Measures {
    pub fn shares(&self) -> [f64; 3] {
        let free = self.free.max(1) as f64;
        [
            self.derived as f64 / free,
            self.recurring as f64 / free,
            self.sequence as f64 / free,
        ]
    }
}

/// A window of `n` free tones of a voice that follow one another without a rest.
struct Window {
    voice: usize,
    first: usize,
    on: Tick,
    dia: i16,
    figure: Figure,
}

/// Which tones of a piece count for each share, voice by voice and tone by tone.
#[derive(Clone, Debug, Default)]
pub struct Marks {
    pub free: Vec<Vec<bool>>,
    pub derived: Vec<Vec<bool>>,
    pub recurring: Vec<Vec<bool>>,
    pub sequence: Vec<Vec<bool>>,
}

/// The measures of a piece: `free` says which tones are free, `material` holds the lines whose
/// figures count as the subject's (subject and countersubject), `n` is the length of a figure in
/// tones.
pub fn measure(
    piece: &Piece,
    free: &dyn Fn(usize, &Note) -> bool,
    material: &[Vec<Note>],
    n: usize,
) -> Measures {
    let marks = marks(piece, free, material, n);
    let count = |marks: &Vec<Vec<bool>>| marks.iter().flatten().filter(|m| **m).count();
    Measures {
        free: count(&marks.free),
        derived: count(&marks.derived),
        recurring: count(&marks.recurring),
        sequence: count(&marks.sequence),
    }
}

/// The tones that count for each share.
pub fn marks(
    piece: &Piece,
    free: &dyn Fn(usize, &Note) -> bool,
    material: &[Vec<Note>],
    n: usize,
) -> Marks {
    let mut known: HashSet<Figure> = HashSet::new();
    for line in material {
        figures_of(line, n, &mut known);
    }
    let mut windows: Vec<Window> = Vec::new();
    let flags: Vec<Vec<bool>> = piece
        .voices
        .iter()
        .enumerate()
        .map(|(v, voice)| voice.notes.iter().map(|note| free(v, note)).collect())
        .collect();
    for (v, voice) in piece.voices.iter().enumerate() {
        for first in 0..voice.notes.len().saturating_sub(n - 1) {
            let notes = &voice.notes[first..first + n];
            if !flags[v][first..first + n].iter().all(|f| *f)
                || notes.windows(2).any(|w| w[0].off != w[1].on)
            {
                continue;
            }
            let figure = Figure::of(notes);
            if figure.plain() {
                continue;
            }
            windows.push(Window {
                voice: v,
                first,
                on: notes[0].on,
                dia: notes[0].pitch.dia,
                figure,
            });
        }
    }
    let mut derived: Vec<Vec<bool>> = flags.iter().map(|f| vec![false; f.len()]).collect();
    let mut recurring = derived.clone();
    let mut sequence = derived.clone();
    let mut by_figure: HashMap<Figure, Vec<usize>> = HashMap::new();
    for (k, window) in windows.iter().enumerate() {
        by_figure
            .entry(window.figure.canonical())
            .or_default()
            .push(k);
    }
    let two_bars = 2 * piece.meter.bar();
    for (k, window) in windows.iter().enumerate() {
        let mark = |flags: &mut Vec<Vec<bool>>| {
            for flag in &mut flags[window.voice][window.first..window.first + n] {
                *flag = true;
            }
        };
        let canonical = window.figure.canonical();
        if known.contains(&canonical) {
            mark(&mut derived);
        }
        let others = &by_figure[&canonical];
        let apart =
            |other: &Window| other.voice != window.voice || other.first.abs_diff(window.first) >= n;
        if others.iter().any(|j| *j != k && apart(&windows[*j])) {
            mark(&mut recurring);
        }
        if others.iter().any(|j| {
            let other = &windows[*j];
            *j != k
                && other.voice == window.voice
                && other.first.abs_diff(window.first) >= n
                && (other.on - window.on).abs() <= two_bars
                && other.figure == window.figure
                && other.dia != window.dia
        }) {
            mark(&mut sequence);
        }
    }
    Marks {
        free: flags,
        derived,
        recurring,
        sequence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
    use crate::score::Voice;
    use crate::themes::parse;
    use crate::time::Meter;

    fn piece(lines: &[&str]) -> Piece {
        let voices = lines
            .iter()
            .map(|line| Voice {
                notes: parse(line).unwrap(),
            })
            .collect();
        Piece::new("motifs", Key::parse("C").unwrap(), Meter::COMMON, voices)
    }

    #[test]
    fn figures_keep_their_shape_under_transposition_and_mirror() {
        let a = Figure::of(&parse("c'8 d' e'4 c'").unwrap());
        let b = Figure::of(&parse("g'8 a' b'4 g'").unwrap());
        let mirrored = Figure::of(&parse("c''8 b' a'4 c''").unwrap());
        let other = Figure::of(&parse("c'8 d' e'8 c'").unwrap());
        assert_eq!(a, b);
        assert_eq!(a.canonical(), mirrored.canonical());
        assert_ne!(a.canonical(), other.canonical());
        // A scale and a repeated tone have no shape of their own.
        assert!(Figure::of(&parse("c'8 d' e' f'").unwrap()).plain());
        assert!(Figure::of(&parse("c'8 c' c' c'").unwrap()).plain());
        assert!(!a.plain());
    }

    #[test]
    fn the_three_shares() {
        // The lower voice is all free. Its first bar has the figure of the subject (a step up, a
        // step up, a third down), a step lower in the second half: derived, recurring and a
        // sequence. Its second bar has a figure of its own, once.
        let subject = parse("c''8 d'' e''4 c''").unwrap();
        let piece = piece(&[
            "c''8 d'' e''4 c''4 r4 | r1",
            "e'8 f' g'4 e'8 d' e' f'4 d'8 | c'4 g' e'8 c' d'4",
        ]);
        let free = |v: usize, _: &Note| v == 1;
        let measures = measure(&piece, &free, std::slice::from_ref(&subject), 4);
        assert_eq!(measures.free, 13);
        // e' f' g' e' and d' e' f' d' are the subject's figure; the tones between belong to
        // windows of other shapes.
        assert_eq!(measures.derived, 8);
        // The sequence goes one tone further: f' g' e' d' comes again as e' f' d' c'.
        assert_eq!(measures.sequence, 9);
        assert_eq!(measures.recurring, 9);
        // With no material nothing is derived, and what recurs still recurs.
        let none = measure(&piece, &free, &[], 4);
        assert_eq!((none.derived, none.sequence), (0, 9));
        // Tones that are not free count nowhere.
        let nothing = measure(&piece, &|_, _| false, &[subject], 4);
        assert_eq!(nothing, Measures::default());
    }
}
