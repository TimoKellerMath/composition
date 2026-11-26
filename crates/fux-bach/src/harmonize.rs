//! A harmony for a given line: per pulse the chord that fits its tones and follows the chord
//! before it as chords follow each other in analysed fugues. The transitions are counted from
//! Roman numeral analyses; the search is the usual best path.
//!
//! This is the scaffold generator of the prototype for entries of the subject. Progressions
//! between entries are written out by `fugue`.

use std::collections::BTreeMap;

use crate::chord::weights;
use crate::key::{Chord, Harmony, Key, Kind};
use crate::pitch::Lof;
use crate::roman;
use crate::score::{Note, Piece, Voice};
use crate::time::{Meter, Tick};

/// A chord relative to a key: the degree of its root, its distance from the natural degree in
/// semitones (the raised sixth and seventh degrees of minor are +1), and its type.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Rel {
    pub degree: u8,
    pub alter: i8,
    pub kind: Kind,
}

impl Rel {
    pub const fn new(degree: u8, alter: i8, kind: Kind) -> Rel {
        Rel {
            degree,
            alter,
            kind,
        }
    }

    pub fn of(chord: Chord, key: Key) -> Rel {
        let degree = key.degree_of(chord.root);
        Rel {
            degree: degree as u8,
            alter: (chord.root.0 - key.degree(degree).0) / 7,
            kind: chord.kind,
        }
    }

    pub fn in_key(self, key: Key) -> Chord {
        Chord::new(
            Lof(key.degree(self.degree as usize).0 + 7 * self.alter),
            self.kind,
        )
    }
}

/// How often one chord follows another, counted per mode of the key.
#[derive(Clone, Debug, Default)]
pub struct Transitions {
    /// Index 0 major, 1 minor.
    pairs: [BTreeMap<(Rel, Rel), f64>; 2],
    firsts: [BTreeMap<Rel, f64>; 2],
}

impl Transitions {
    /// Counts the successions of chords inside one key in an analysis.
    pub fn learn(&mut self, analysis: &roman::Analysis) {
        let mut before: Option<(Key, Rel)> = None;
        for label in &analysis.labels {
            let Some(chord) = roman::chord(&label.figure, label.key) else {
                continue;
            };
            let rel = Rel::of(chord, label.key);
            let mode = label.key.minor as usize;
            if let Some((key, last)) = before
                && key == label.key
                && last != rel
            {
                *self.pairs[mode].entry((last, rel)).or_default() += 1.0;
            }
            *self.firsts[mode].entry(rel).or_default() += 1.0;
            before = Some((label.key, rel));
        }
    }

    /// The chords seen at least `least` times in keys of this mode.
    pub fn vocabulary(&self, minor: bool, least: f64) -> Vec<Rel> {
        self.firsts[minor as usize]
            .iter()
            .filter(|(_, count)| **count >= least)
            .map(|(rel, _)| *rel)
            .collect()
    }

    /// The logarithm of the smoothed share of `to` among what follows `from`.
    pub fn log(&self, minor: bool, from: Rel, to: Rel, vocabulary: usize) -> f64 {
        let mode = minor as usize;
        let pair = self.pairs[mode].get(&(from, to)).copied().unwrap_or(0.0);
        let total: f64 = self.pairs[mode]
            .range((from, Rel::new(0, -9, Kind::Maj))..=(from, Rel::new(9, 9, Kind::Ger)))
            .map(|(_, c)| c)
            .sum();
        ((pair + 0.5) / (total + 0.5 * vocabulary as f64)).ln()
    }

    pub fn size(&self) -> usize {
        self.pairs.iter().flat_map(|m| m.values()).sum::<f64>() as usize
    }

    /// Everything counted, as rows of numbers: successions as (mode, degree, alteration and kind
    /// of the first chord, the same of the second, count), first chords as (mode, degree,
    /// alteration, kind, count). A kind is its place in `Kind::ALL`.
    pub fn rows(&self) -> (Vec<[f64; 8]>, Vec<[f64; 5]>) {
        let kind = |k: Kind| Kind::ALL.iter().position(|x| *x == k).unwrap_or(0) as f64;
        let mut pairs = Vec::new();
        let mut firsts = Vec::new();
        for mode in 0..2 {
            for ((a, b), count) in &self.pairs[mode] {
                pairs.push([
                    mode as f64,
                    a.degree as f64,
                    a.alter as f64,
                    kind(a.kind),
                    b.degree as f64,
                    b.alter as f64,
                    kind(b.kind),
                    *count,
                ]);
            }
            for (a, count) in &self.firsts[mode] {
                firsts.push([
                    mode as f64,
                    a.degree as f64,
                    a.alter as f64,
                    kind(a.kind),
                    *count,
                ]);
            }
        }
        (pairs, firsts)
    }

    /// The table that `rows` came from.
    pub fn from_rows(pairs: &[[f64; 8]], firsts: &[[f64; 5]]) -> Transitions {
        let rel = |d: f64, a: f64, k: f64| {
            Rel::new(
                d as u8,
                a as i8,
                Kind::ALL[(k as usize).min(Kind::ALL.len() - 1)],
            )
        };
        let mut out = Transitions::default();
        for row in pairs {
            let mode = (row[0] as usize).min(1);
            *out.pairs[mode]
                .entry((rel(row[1], row[2], row[3]), rel(row[4], row[5], row[6])))
                .or_default() += row[7];
        }
        for row in firsts {
            let mode = (row[0] as usize).min(1);
            *out.firsts[mode]
                .entry(rel(row[1], row[2], row[3]))
                .or_default() += row[4];
        }
        out
    }
}

/// What a harmonisation should begin and end with.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ends {
    /// The first chord is the tonic.
    pub tonic_first: bool,
    /// The last chord is the tonic.
    pub tonic_last: bool,
}

/// The chords for one or more lines in `key`, one per pulse from `from` to `until`, as spans.
#[allow(clippy::too_many_arguments)]
pub fn harmonize(
    transitions: &Transitions,
    lines: &[&[Note]],
    key: Key,
    meter: Meter,
    origin: Tick,
    from: Tick,
    until: Tick,
    ends: Ends,
) -> Vec<(Tick, Chord)> {
    let pulse = meter.pulse();
    let voices = lines
        .iter()
        .map(|notes| Voice {
            notes: notes.to_vec(),
        })
        .collect();
    let mut piece = Piece::new("lines", key, meter, voices);
    piece.origin = origin;
    piece.end = until;
    let vocabulary = transitions.vocabulary(key.minor, 3.0);
    let tonic = Rel::new(0, 0, if key.minor { Kind::Min } else { Kind::Maj });
    let count = ((until - from + pulse - 1) / pulse).max(1) as usize;
    // How well each chord covers the tones of each pulse; a pulse without a tone says nothing.
    let fit: Vec<Vec<f64>> = (0..count)
        .map(|i| {
            let t = from + i as Tick * pulse;
            let w = weights(&piece, t, t + pulse);
            let total: f32 = w.iter().sum();
            vocabulary
                .iter()
                .map(|rel| {
                    if total <= 0.0 {
                        return 0.0;
                    }
                    let mask = rel.in_key(key).mask();
                    let inside: f32 = (0..12)
                        .filter(|pc| mask & (1 << pc) != 0)
                        .map(|pc| w[pc])
                        .sum();
                    ((2.0 * inside - total) / total) as f64
                })
                .collect()
        })
        .collect();
    const FIT: f64 = 4.0;
    const STAY: f64 = -0.7;
    let n = vocabulary.len();
    let mut best: Vec<f64> = (0..n)
        .map(|k| {
            let start = if vocabulary[k] == tonic {
                0.0
            } else if ends.tonic_first {
                -50.0
            } else {
                -2.0
            };
            start + FIT * fit[0][k]
        })
        .collect();
    let mut back: Vec<Vec<usize>> = Vec::new();
    for row in fit.iter().skip(1) {
        let mut next = vec![f64::NEG_INFINITY; n];
        let mut from_state = vec![0; n];
        for k in 0..n {
            for p in 0..n {
                let step = if p == k {
                    STAY
                } else {
                    transitions.log(key.minor, vocabulary[p], vocabulary[k], n)
                };
                if best[p] + step > next[k] {
                    next[k] = best[p] + step;
                    from_state[k] = p;
                }
            }
            next[k] += FIT * row[k];
        }
        best = next;
        back.push(from_state);
    }
    if ends.tonic_last {
        for (k, value) in best.iter_mut().enumerate() {
            if vocabulary[k] != tonic {
                *value -= 50.0;
            }
        }
    }
    let mut state = (0..n)
        .max_by(|a, b| best[*a].total_cmp(&best[*b]))
        .unwrap_or(0);
    let mut path = vec![state; count];
    for (i, from_state) in back.iter().enumerate().rev() {
        state = from_state[state];
        path[i] = state;
    }
    path.into_iter()
        .enumerate()
        .map(|(i, k)| (from + i as Tick * pulse, vocabulary[k].in_key(key)))
        .collect()
}

/// Appends chords to a harmony.
pub fn extend(harmony: &mut Harmony, chords: &[(Tick, Chord)], key: Key) {
    for (tick, chord) in chords {
        harmony.push(*tick, *chord, key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::Pitch;

    fn learned() -> Transitions {
        // A small analysis with the usual cadences in major and minor.
        let text = "Time Signature: 4/4\n\
            m1 C: I b2 IV b3 V7 b4 I\n\
            m2 I b2 ii6 b3 V b4 I\n\
            m3 I b2 vi b3 ii b4 V7\n\
            m4 I b2 IV b3 V b4 I\n\
            m5 I b2 V6 b3 I b4 IV\n\
            m6 V7 b2 I b3 ii b4 V\n\
            m7 I b2 vi b3 IV b4 V\n\
            m8 I\n";
        let mut transitions = Transitions::default();
        transitions.learn(&roman::parse(text).unwrap());
        transitions
    }

    #[test]
    fn relative_chords() {
        let c_minor = Key::parse("c").unwrap();
        let g7 = Chord::new(Lof(1), Kind::Dom7);
        let rel = Rel::of(g7, c_minor);
        assert_eq!(rel, Rel::new(4, 0, Kind::Dom7));
        assert_eq!(rel.in_key(Key::parse("a").unwrap()).to_string(), "E7");
        let b_dim = Chord::new(Lof(5), Kind::Dim);
        assert_eq!(Rel::of(b_dim, c_minor), Rel::new(6, 1, Kind::Dim));
        assert_eq!(
            Rel::new(6, 1, Kind::Dim)
                .in_key(Key::parse("g").unwrap())
                .to_string(),
            "F#o"
        );
    }

    #[test]
    fn a_scale_gets_a_cadence() {
        let transitions = learned();
        assert!(transitions.size() > 20);
        // C E G E | F A G B | C: quarters in C major.
        let steps = [
            (0, 4),
            (2, 4),
            (4, 4),
            (2, 4),
            (3, 4),
            (5, 4),
            (4, 4),
            (6, 3),
            (0, 4),
        ];
        let notes: Vec<Note> = steps
            .iter()
            .enumerate()
            .map(|(i, (step, octave))| Note {
                on: 24 * i as Tick,
                off: 24 * i as Tick + 24,
                pitch: Pitch::new(*step, 0, *octave),
            })
            .collect();
        let key = Key::parse("C").unwrap();
        let chords = harmonize(
            &transitions,
            &[&notes],
            key,
            Meter::COMMON,
            0,
            0,
            216,
            Ends {
                tonic_first: true,
                tonic_last: true,
            },
        );
        let names: Vec<String> = chords.iter().map(|c| c.1.to_string()).collect();
        assert_eq!(names.len(), 9);
        assert_eq!(names[0], "C");
        assert_eq!(names[8], "C");
        // The leading tone before the end stands on a dominant.
        assert!(names[7].starts_with('G'), "{names:?}");
        assert!(
            names[4].starts_with('F') || names[4].starts_with('D'),
            "{names:?}"
        );
    }
}
