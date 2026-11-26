//! A simple chord and key estimate from the notes alone: per half beat the triad or seventh
//! chord that covers the sounding tones best, weighted by duration, smoothed over time; per bar
//! the key whose profile fits the tones around it.
//!
//! The estimate is a tool for checking and calibrating (T3, T4), not for composing: in a search
//! the chord is an input.

use crate::key::{Chord, Harmony, Key, Kind};
use crate::pitch::Lof;
use crate::score::Piece;
use crate::time::Tick;

/// Chord types the estimate chooses from, with the cost of preferring them over a plain triad.
const KINDS: [(Kind, f32); 8] = [
    (Kind::Maj, 0.0),
    (Kind::Min, 0.0),
    (Kind::Dim, 0.06),
    (Kind::Dom7, 0.05),
    (Kind::Min7, 0.14),
    (Kind::HalfDim7, 0.12),
    (Kind::Dim7, 0.10),
    (Kind::Maj7, 0.22),
];

/// What a tone outside the chord costs, relative to what a chord tone earns.
const OUTSIDE: f32 = 1.0;
const MISSING_ROOT: f32 = 0.05;
const MISSING_THIRD: f32 = 0.05;
/// Per chord tone that belongs to the key of the bar.
const IN_KEY: f32 = 0.03;
/// Cost of a change of chord from one step of the grid to the next.
const CHANGE: f32 = 0.20;
/// Cost of a change of key from one bar to the next.
const KEY_CHANGE: f32 = 0.08;

/// Key profiles after Temperley (2001), major and minor from the tonic upwards.
const PROFILE_MAJOR: [f32; 12] = [5.0, 2.0, 3.5, 2.0, 4.5, 4.0, 2.0, 4.5, 2.0, 3.5, 1.5, 4.0];
const PROFILE_MINOR: [f32; 12] = [5.0, 2.0, 3.5, 4.5, 2.0, 4.0, 2.0, 4.5, 3.5, 2.0, 1.5, 4.0];

/// Weight of every pitch class in `[from, until)`: sounding ticks, more for a tone struck at
/// `from`, for the lowest tone there and for a tone reached or left by leap, less for a tone off
/// the beat that is reached and left by step (the shape of a passing tone or neighbour).
pub fn weights(piece: &Piece, from: Tick, until: Tick) -> [f32; 12] {
    let mut w = [0.0f32; 12];
    let lowest = piece
        .voices
        .iter()
        .chain(&piece.extras)
        .filter_map(|v| v.at(from).map(|n| n.pitch))
        .min();
    let beat = piece.meter.beat();
    for voice in piece.voices.iter().chain(&piece.extras) {
        let first = voice.notes.partition_point(|n| n.off <= from);
        for (i, note) in voice.notes.iter().enumerate().skip(first) {
            if note.on >= until {
                break;
            }
            let overlap = (note.off.min(until) - note.on.max(from)) as f32;
            let mut factor = 1.0;
            if note.on == from {
                factor += 0.5;
            }
            if note.on <= from && Some(note.pitch) == lowest {
                factor += 0.5;
            }
            let arrive = voice.before(i).map(|b| b.pitch.to(note.pitch));
            let leave = voice.after(i).map(|a| note.pitch.to(a.pitch));
            let leap = |iv: Option<crate::pitch::Iv>| {
                iv.is_some_and(|iv| !iv.is_step() && !iv.is_unison())
            };
            let step = |iv: Option<crate::pitch::Iv>| iv.is_some_and(|iv| iv.is_step());
            if leap(arrive) || leap(leave) {
                factor += 0.5;
            } else if step(arrive) && step(leave) && note.dur() < beat {
                factor *= match piece.weight(note.on) {
                    0 => 0.4,
                    1 => 0.7,
                    _ => 1.0,
                };
            }
            w[note.pitch.pc() as usize] += overlap * factor;
        }
    }
    w
}

/// How likely a chord is by its degree in the key, as a bonus: tonic and dominant first.
fn degree_bonus(chord: Chord, key: Key) -> f32 {
    let degree = key.degree_of(chord.root);
    let diatonic = chord.tones().all(|t| key.contains(t));
    if !diatonic {
        return 0.0;
    }
    match (degree, chord.kind) {
        (0, Kind::Maj | Kind::Min) => 0.06,
        (4, Kind::Maj | Kind::Dom7) => 0.06,
        (3, Kind::Maj | Kind::Min) => 0.04,
        (6, Kind::Dim | Kind::Dim7 | Kind::HalfDim7) => 0.03,
        (1, Kind::Min | Kind::Dim | Kind::Min7 | Kind::HalfDim7) => 0.03,
        (5, Kind::Maj | Kind::Min) => 0.02,
        (2, Kind::Maj | Kind::Min) => 0.01,
        _ => 0.0,
    }
}

fn normalized(profile: [f32; 12]) -> [f32; 12] {
    let mean = profile.iter().sum::<f32>() / 12.0;
    let centred = profile.map(|p| p - mean);
    let norm = centred.iter().map(|p| p * p).sum::<f32>().sqrt();
    centred.map(|p| p / norm)
}

/// The best path through `scores[time][state]` when a change of state costs `change(a, b)`.
fn viterbi(scores: &[Vec<f32>], change: impl Fn(usize, usize) -> f32) -> Vec<usize> {
    let Some(first) = scores.first() else {
        return Vec::new();
    };
    let states = first.len();
    let mut best = first.clone();
    let mut back: Vec<Vec<usize>> = Vec::with_capacity(scores.len());
    for row in &scores[1..] {
        let mut next = vec![f32::NEG_INFINITY; states];
        let mut from = vec![0; states];
        // The best state to come from when the state changes, apart from the cost of the change.
        for (s, value) in next.iter_mut().enumerate() {
            for (p, before) in best.iter().enumerate() {
                let total = before - change(p, s);
                if total > *value {
                    *value = total;
                    from[s] = p;
                }
            }
            *value += row[s];
        }
        best = next;
        back.push(from);
    }
    let mut state = (0..states)
        .max_by(|a, b| best[*a].total_cmp(&best[*b]))
        .unwrap_or(0);
    let mut path = vec![state; scores.len()];
    for (t, from) in back.iter().enumerate().rev() {
        state = from[state];
        path[t] = state;
    }
    path
}

/// The 24 keys in a fixed order: index 2·pc for major, 2·pc + 1 for minor.
fn key_of(index: usize, home: Key) -> Key {
    let pc = (index / 2) as u8;
    let minor = index % 2 == 1;
    // Spelled next to the home key on the line of fifths; minor tonics lie three fifths above
    // their relative major.
    let centre = home.tonic.0 - if home.minor { 3 } else { 0 };
    let tonic = Lof::spell(pc, centre - 6 + if minor { 3 } else { 0 });
    Key { tonic, minor }
}

/// The key of every bar: the profile that fits the tones of the bar and its two neighbours.
pub fn keys(piece: &Piece) -> Vec<(Tick, Key)> {
    let bar = piece.meter.bar();
    let starts: Vec<Tick> = piece.grid(bar).collect();
    let major = normalized(PROFILE_MAJOR);
    let minor = normalized(PROFILE_MINOR);
    let scores: Vec<Vec<f32>> = starts
        .iter()
        .map(|&t| {
            let w = weights(piece, t - bar, t + 2 * bar);
            let total: f32 = w.iter().sum::<f32>().max(1.0);
            (0..24)
                .map(|k| {
                    let profile = if k % 2 == 1 { &minor } else { &major };
                    let fit: f32 = (0..12)
                        .map(|pc| w[pc] * profile[(pc + 12 - k / 2) % 12])
                        .sum();
                    let home = key_of(k, piece.key) == piece.key;
                    fit / total + if home { 0.02 } else { 0.0 }
                })
                .collect()
        })
        .collect();
    let path = viterbi(&scores, |a, b| if a == b { 0.0 } else { KEY_CHANGE });
    starts
        .into_iter()
        .zip(path)
        .map(|(t, k)| (t, key_of(k, piece.key)))
        .collect()
}

/// The harmony of a piece from its notes, with the key of each bar. The grid is the half beat,
/// in compound meters the third of the beat: on the development fugues the harmony of I/19 moves
/// in eighths, and the cost of a change keeps the chord of a beat together elsewhere.
pub fn estimate(piece: &Piece) -> Harmony {
    let meter = piece.meter;
    let step = if meter.compound() {
        meter.beat() / 3
    } else {
        meter.beat() / 2
    };
    estimate_on(piece, step, CHANGE)
}

/// The estimate on a grid of `step` ticks with the given cost of a change of chord.
pub fn estimate_on(piece: &Piece, step: Tick, change: f32) -> Harmony {
    let key_track = keys(piece);
    let key_at = |t: Tick| -> Key {
        let i = key_track.partition_point(|(start, _)| *start <= t);
        key_track
            .get(i.saturating_sub(1))
            .map_or(piece.key, |k| k.1)
    };
    let beat = step;
    let beats: Vec<Tick> = piece.grid(step).collect();
    let states = 12 * KINDS.len();
    let chord_of = |state: usize, key: Key| -> Chord {
        let (kind, _) = KINDS[state % KINDS.len()];
        let pc = (state / KINDS.len()) as u8;
        Chord::new(
            key.spell(pc, kind.is_dominant() && kind != Kind::Dom7),
            kind,
        )
    };
    let scores: Vec<Vec<f32>> = beats
        .iter()
        .map(|&t| {
            let w = weights(piece, t, t + beat);
            let total: f32 = w.iter().sum();
            let key = key_at(t);
            (0..states)
                .map(|state| {
                    if total <= 0.0 {
                        return 0.0;
                    }
                    let (kind, cost) = KINDS[state % KINDS.len()];
                    let chord = chord_of(state, key);
                    let mask = chord.mask();
                    let inside: f32 = (0..12)
                        .filter(|pc| mask & (1 << pc) != 0)
                        .map(|pc| w[pc])
                        .sum();
                    let mut score = (inside - OUTSIDE * (total - inside)) / total - cost;
                    let mut tones = chord.tones();
                    if tones.next().is_some_and(|r| w[r.pc() as usize] == 0.0) {
                        score -= MISSING_ROOT;
                    }
                    if tones.next().is_some_and(|t| w[t.pc() as usize] == 0.0) {
                        score -= MISSING_THIRD;
                    }
                    // A seventh chord needs its seventh.
                    if kind.is_seventh()
                        && chord
                            .tones()
                            .nth(3)
                            .is_some_and(|s| w[s.pc() as usize] == 0.0)
                    {
                        score -= 1.0;
                    }
                    score
                        + IN_KEY * chord.tones().filter(|t| key.contains(*t)).count() as f32
                        + degree_bonus(chord, key)
                })
                .collect()
        })
        .collect();
    let path = viterbi(&scores, |a, b| if a == b { 0.0 } else { change });
    let mut harmony = Harmony::default();
    for (&t, state) in beats.iter().zip(path) {
        let key = key_at(t);
        harmony.push(t, chord_of(state, key), key);
    }
    harmony
}

/// How two harmonies agree on the beats of a piece where both have a chord.
#[derive(Clone, Copy, Debug, Default)]
pub struct Agreement {
    pub beats: usize,
    pub same_root: usize,
    pub same_tones: usize,
    /// One chord contains the other (a triad and its seventh chord).
    pub nested: usize,
    pub same_key: usize,
}

impl std::ops::AddAssign for Agreement {
    fn add_assign(&mut self, o: Agreement) {
        self.beats += o.beats;
        self.same_root += o.same_root;
        self.same_tones += o.same_tones;
        self.nested += o.nested;
        self.same_key += o.same_key;
    }
}

pub fn agreement(piece: &Piece, a: &Harmony, b: &Harmony) -> Agreement {
    agreement_on(piece, a, b, piece.meter.beat())
}

pub fn agreement_on(piece: &Piece, a: &Harmony, b: &Harmony, step: Tick) -> Agreement {
    let mut out = Agreement::default();
    for t in piece.grid(step) {
        let (Some(x), Some(y)) = (a.at(t), b.at(t)) else {
            continue;
        };
        out.beats += 1;
        let (mx, my) = (x.chord.mask(), y.chord.mask());
        out.same_root += (x.chord.root.pc() == y.chord.root.pc()) as usize;
        out.same_tones += (mx == my) as usize;
        out.nested += (mx & my == mx || mx & my == my) as usize;
        out.same_key +=
            (x.key.tonic.pc() == y.key.tonic.pc() && x.key.minor == y.key.minor) as usize;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::Pitch;
    use crate::score::Voice;
    use crate::time::Meter;

    #[test]
    fn a_cadence_is_recognised() {
        // C major: I, IV, V7, I in half notes, three voices.
        let tones = [
            [(4, 4), (5, 4), (3, 4), (2, 4)],
            [(2, 4), (3, 4), (6, 3), (0, 4)],
            [(0, 3), (3, 2), (4, 2), (0, 3)],
        ];
        let voices = tones
            .iter()
            .map(|line| {
                let mut voice = Voice::default();
                for (i, (step, octave)) in line.iter().enumerate() {
                    let on = 48 * i as Tick;
                    voice.push(on, on + 48, Pitch::new(*step, 0, *octave));
                }
                voice
            })
            .collect();
        let piece = Piece::new("cadence", Key::parse("C").unwrap(), Meter::COMMON, voices);
        let harmony = estimate(&piece);
        let names: Vec<String> = harmony.spans.iter().map(|s| s.chord.to_string()).collect();
        assert_eq!(names, ["C", "F", "G7", "C"]);
        assert!(harmony.spans.iter().all(|s| s.key == piece.key));
    }
}
