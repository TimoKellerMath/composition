//! Two hands: which staff of the piano system a voice stands on, bar by bar. The top voice
//! stays above and the bottom voice below; a middle voice goes where the hand can reach it.

use crate::score::Piece;
use crate::time::Tick;

/// What a stretch beyond the octave costs per semitone and sampled moment.
const STRETCH: f64 = 4.0;
/// A hand that reaches over the other.
const CROSSED: f64 = 2.0;
/// A middle voice away from its usual staff, per bar.
const AWAY: f64 = 0.3;
/// A voice that changes staff at a bar line.
const CHANGE: f64 = 1.5;
/// Per semitone and sampled moment a tone lies below the B under middle C on the upper staff or
/// above the D over it on the lower: ledger lines.
const LEDGER: f64 = 0.15;

/// The usual staff of a voice: 0 the upper, 1 the lower.
pub fn usual(voice: usize, voices: usize) -> u8 {
    let upper = match voices {
        0 | 1 => 1,
        2 | 3 => voices - 1,
        _ => 2,
    };
    u8::from(voice >= upper)
}

/// For every voice the staff (0 upper, 1 lower) in every bar; `starts` are the first ticks of
/// the bars.
pub fn staves(piece: &Piece, starts: &[Tick]) -> Vec<Vec<u8>> {
    let voices = piece.voices.len();
    let fixed: Vec<Vec<u8>> = (0..voices)
        .map(|v| vec![usual(v, voices); starts.len()])
        .collect();
    if voices < 3 || starts.is_empty() {
        return fixed;
    }
    // The ways to give the voices to the hands: the top above, the bottom below, no voice
    // above one that is written higher.
    let ways: Vec<Vec<u8>> = (0..(1u32 << voices))
        .map(|bits| {
            (0..voices)
                .map(|v| (bits >> v & 1) as u8)
                .collect::<Vec<u8>>()
        })
        .filter(|w| w[0] == 0 && w[voices - 1] == 1 && w.windows(2).all(|p| p[0] <= p[1]))
        .collect();
    let step = (piece.meter.beat() / 2).max(1);
    let bar_cost = |from: Tick, until: Tick, way: &[u8]| -> f64 {
        let mut cost = 0.0;
        let mut t = from;
        while t < until {
            let mut reach = [(i16::MAX, i16::MIN); 2];
            for (v, voice) in piece.voices.iter().enumerate() {
                if let Some(note) = voice.at(t) {
                    let hand = &mut reach[way[v] as usize];
                    hand.0 = hand.0.min(note.pitch.midi);
                    hand.1 = hand.1.max(note.pitch.midi);
                    let beyond = if way[v] == 0 {
                        59 - note.pitch.midi
                    } else {
                        note.pitch.midi - 62
                    };
                    cost += LEDGER * f64::from(beyond.max(0));
                }
            }
            for (low, high) in reach {
                if high >= low {
                    cost += STRETCH * f64::from((high - low - 12).max(0));
                }
            }
            if reach[0].1 >= reach[0].0 && reach[1].1 >= reach[1].0 && reach[0].0 < reach[1].1 {
                cost += CROSSED;
            }
            t += step;
        }
        cost + AWAY
            * way
                .iter()
                .enumerate()
                .filter(|(v, s)| **s != usual(*v, voices))
                .count() as f64
    };
    // The cheapest way through the bars.
    let mut best: Vec<f64> = vec![0.0; ways.len()];
    let mut back: Vec<Vec<usize>> = Vec::with_capacity(starts.len());
    for (k, from) in starts.iter().enumerate() {
        let until = starts.get(k + 1).copied().unwrap_or(piece.end);
        let mut next = vec![f64::MAX; ways.len()];
        let mut from_way = vec![0; ways.len()];
        for (w, way) in ways.iter().enumerate() {
            let here = bar_cost(*from, until, way);
            for (p, before) in ways.iter().enumerate() {
                let changed = if k == 0 {
                    0
                } else {
                    way.iter().zip(before).filter(|(a, b)| a != b).count()
                };
                let total = best[p] + CHANGE * changed as f64 + here;
                if total < next[w] {
                    next[w] = total;
                    from_way[w] = p;
                }
            }
        }
        best = next;
        back.push(from_way);
    }
    let mut way = (0..ways.len())
        .min_by(|a, b| best[*a].total_cmp(&best[*b]))
        .unwrap_or(0);
    let mut out = fixed;
    for k in (0..starts.len()).rev() {
        for (v, line) in out.iter_mut().enumerate() {
            line[k] = ways[way][v];
        }
        way = back[k][way];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
    use crate::pitch::Pitch;
    use crate::score::Voice;
    use crate::time::Meter;

    #[test]
    fn a_middle_voice_goes_to_the_nearer_hand() {
        // Three voices in whole notes: the middle one near the top in bars 1 and 2, near the
        // bass in bars 3 and 4.
        let line = |tones: &[(i8, i8)]| {
            let mut voice = Voice::default();
            for (k, (step, octave)) in tones.iter().enumerate() {
                let on = 96 * k as Tick;
                voice.push(on, on + 96, Pitch::new(*step, 0, *octave));
            }
            voice
        };
        let piece = Piece::new(
            "hands",
            Key::parse("C").unwrap(),
            Meter::COMMON,
            vec![
                line(&[(0, 5), (0, 5), (0, 5), (0, 5)]),
                line(&[(2, 4), (2, 4), (2, 3), (2, 3)]),
                line(&[(0, 3), (0, 3), (0, 3), (0, 3)]),
            ],
        );
        let starts = [0, 96, 192, 288];
        let staves = staves(&piece, &starts);
        assert_eq!(staves[0], [0, 0, 0, 0]);
        assert_eq!(staves[1], [0, 0, 1, 1]);
        assert_eq!(staves[2], [1, 1, 1, 1]);
        assert_eq!(usual(1, 4), 0);
        assert_eq!(usual(2, 4), 1);
    }
}
