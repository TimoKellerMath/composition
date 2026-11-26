//! The style measures of protocol T4: per piece a fixed list of numbers, computed from the notes
//! with the estimated chords, and their bands over a corpus (10th to 90th percentile).

use crate::chord;
use crate::figures::{Figure, Reading, note_census};
use crate::key::Harmony;
use crate::proxy::dissonant;
use crate::score::{Note, Piece};
use crate::time::{QUARTER, SIXTEENTH, Tick};

/// Names of the measures, in the order of [`measures`]. German, as they appear in the report.
pub const NAMES: [&str; 29] = [
    "Notenwerte: Sechzehntel und kürzer",
    "Notenwerte: Achtel",
    "Notenwerte: Viertel und länger",
    "Noten je Stimme und Viertel",
    "Melodik: Tonwiederholung",
    "Melodik: Sekunde",
    "Melodik: Terz",
    "Melodik: Quarte und Quinte",
    "Melodik: größere Sprünge",
    "Melodik: chromatischer Halbton",
    "Melodik: Tritonussprung",
    "Melodik: Richtungswechsel",
    "Schläge mit angeschlagener Dissonanz",
    "Schläge mit Dissonanz",
    "Akkordbezug: Akkordton",
    "Akkordbezug: Durchgang",
    "Akkordbezug: Wechselnote",
    "Akkordbezug: Vorhalt",
    "Akkordbezug: übrige Figuren",
    "Akkordbezug: unerklärt",
    "Satz: Pausenanteil",
    "Satz: Sechzehntel-Raster besetzt",
    "Satz: gleichzeitige Anschläge",
    "Parallelen je 100 Schläge",
    "Stimmkreuzungen je 100 Schläge",
    "Außenstimmen: Gegenbewegung",
    "Außenstimmen: Seitenbewegung",
    "Außenstimmen: gleiche Richtung",
    "Klang: vollständiger Dreiklang",
];

pub const COUNT: usize = NAMES.len();

/// Index of the measure the protocol singles out: beats with a struck dissonance.
pub const STRUCK_DISSONANCE: usize = 12;
/// Index of the unexplained share.
pub const UNEXPLAINED: usize = 19;

fn share(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

/// Pairs of notes of two voices that begin together after both voices moved, with the notes
/// before them: the places where parallels can occur.
fn parallels(piece: &Piece) -> usize {
    let mut count = 0;
    for a in 0..piece.voices.len() {
        for b in a + 1..piece.voices.len() {
            let (upper, lower) = (&piece.voices[a], &piece.voices[b]);
            for (i, note) in upper.notes.iter().enumerate() {
                let Some(before) = upper.before(i) else {
                    continue;
                };
                let Some(j) = lower.index_at(note.on) else {
                    continue;
                };
                let other = &lower.notes[j];
                let Some(other_before) = lower.before(j) else {
                    continue;
                };
                if other.on != note.on {
                    continue;
                }
                let now = other.pitch.to(note.pitch);
                let then = other_before.pitch.to(before.pitch);
                let moved = before.pitch.to(note.pitch).dir();
                if now.is_perfect()
                    && then.is_perfect()
                    && now.simple() == then.simple()
                    && moved != 0
                    && moved == other_before.pitch.to(other.pitch).dir()
                {
                    count += 1;
                }
            }
        }
    }
    count
}

/// The measures of a piece, with the harmony estimated from its notes.
pub fn measures(piece: &Piece) -> [f64; COUNT] {
    let harmony = chord::estimate(piece);
    measures_with(piece, &harmony)
}

/// The measures of a piece against a given harmony (only the group „Akkordbezug“ depends on it).
pub fn measures_with(piece: &Piece, harmony: &Harmony) -> [f64; COUNT] {
    let mut out = [0.0; COUNT];
    let notes: Vec<&Note> = piece.voices.iter().flat_map(|v| &v.notes).collect();
    let total = notes.len();
    let start = notes.iter().map(|n| n.on).min().unwrap_or(0);
    let length = (piece.end - start).max(1);

    out[0] = share(notes.iter().filter(|n| n.dur() <= SIXTEENTH).count(), total);
    out[1] = share(
        notes
            .iter()
            .filter(|n| n.dur() > SIXTEENTH && n.dur() <= 2 * SIXTEENTH)
            .count(),
        total,
    );
    out[2] = share(notes.iter().filter(|n| n.dur() >= QUARTER).count(), total);
    out[3] = total as f64 / (piece.voices.len() as f64 * length as f64 / QUARTER as f64);

    // Melodic intervals between notes that follow each other without a rest.
    let mut intervals = 0;
    let mut kinds = [0usize; 7];
    let mut turns = 0;
    let mut turn_places = 0;
    for voice in &piece.voices {
        for i in 1..voice.notes.len() {
            let Some(before) = voice.before(i) else {
                continue;
            };
            let iv = before.pitch.to(voice.notes[i].pitch);
            intervals += 1;
            let steps = iv.steps.abs();
            let kind = if iv.is_unison() {
                0
            } else if iv.is_step() {
                1
            } else if steps == 2 {
                2
            } else if steps == 3 || steps == 4 {
                3
            } else {
                4
            };
            kinds[kind] += 1;
            if iv.steps == 0 && iv.semis.abs() == 1 {
                kinds[5] += 1;
            }
            if iv.semis.abs() == 6 {
                kinds[6] += 1;
            }
            if let Some(after) = voice.after(i) {
                let next = voice.notes[i].pitch.to(after.pitch);
                if iv.dir() != 0 && next.dir() != 0 {
                    turn_places += 1;
                    if iv.dir() != next.dir() {
                        turns += 1;
                    }
                }
            }
        }
    }
    for (k, count) in kinds.iter().enumerate() {
        out[4 + k] = share(*count, intervals);
    }
    out[11] = share(turns, turn_places);

    // Beats: dissonance, crossings, outer voices, sonority.
    let mut beats = 0;
    let mut beats_sounding = 0;
    let mut struck = 0;
    let mut any = 0;
    let mut crossings = 0;
    let mut full = 0;
    let mut three = 0;
    let mut motion = [0usize; 3];
    let mut before_outer: Option<(Note, Note)> = None;
    let last = piece.voices.len() - 1;
    for t in piece.beats() {
        if t < start {
            continue;
        }
        beats += 1;
        let sounding: Vec<(usize, Note)> = piece
            .voices
            .iter()
            .enumerate()
            .filter_map(|(v, voice)| voice.at(t).map(|n| (v, *n)))
            .collect();
        if sounding.len() >= 2 {
            beats_sounding += 1;
            let lowest = sounding
                .iter()
                .map(|s| s.1.pitch)
                .min()
                .expect("two sounding");
            let mut hit = false;
            let mut hit_struck = false;
            for a in 0..sounding.len() {
                for b in a + 1..sounding.len() {
                    let (x, y) = (sounding[a].1, sounding[b].1);
                    if dissonant(x.pitch, y.pitch, lowest) {
                        hit = true;
                        hit_struck |= x.on == t && y.on == t;
                    }
                    // The voice written above sounds below.
                    if x.pitch.midi < y.pitch.midi {
                        crossings += 1;
                    }
                }
            }
            any += hit as usize;
            struck += hit_struck as usize;
        }
        if sounding.len() >= 3 {
            three += 1;
            let mask: u16 = sounding.iter().fold(0, |m, s| m | 1 << s.1.pitch.pc());
            let triad = (0..12u16).any(|root| {
                [[0, 4, 7], [0, 3, 7], [0, 3, 6]]
                    .iter()
                    .any(|tones| tones.iter().all(|t| mask & (1 << ((root + t) % 12)) != 0))
            });
            full += triad as usize;
        }
        let outer = match (piece.voices[0].at(t), piece.voices[last].at(t)) {
            (Some(top), Some(bottom)) if last > 0 => Some((*top, *bottom)),
            _ => None,
        };
        if let (Some((top_before, bottom_before)), Some((top, bottom))) = (before_outer, outer) {
            let up = top_before.pitch.to(top.pitch).dir();
            let down = bottom_before.pitch.to(bottom.pitch).dir();
            if up != 0 || down != 0 {
                let kind = if up == 0 || down == 0 {
                    1
                } else if up == down {
                    2
                } else {
                    0
                };
                motion[kind] += 1;
            }
        }
        before_outer = outer;
    }
    out[12] = share(struck, beats_sounding);
    out[13] = share(any, beats_sounding);
    out[23] = 100.0 * share(parallels(piece), beats);
    out[24] = 100.0 * share(crossings, beats);
    let moves: usize = motion.iter().sum();
    for (k, count) in motion.iter().enumerate() {
        out[25 + k] = share(*count, moves);
    }
    out[28] = share(full, three);

    // Every tone against its chord.
    let census = note_census(Reading { piece, harmony });
    let read = census.read();
    let held = |f: Figure| census.held.get(&f).copied().unwrap_or(0);
    out[14] = share(census.count(Figure::ChordTone), read);
    out[15] = share(census.count(Figure::Passing), read);
    out[16] = share(census.count(Figure::Neighbour), read);
    out[17] = share(
        census.count(Figure::Suspension) + held(Figure::Suspension),
        read,
    );
    let others: usize = [
        Figure::Appoggiatura,
        Figure::Anticipation,
        Figure::Escape,
        Figure::Cambiata,
        Figure::DoubleNeighbour,
        Figure::Pedal,
        Figure::Held,
        Figure::Delayed,
    ]
    .into_iter()
    .map(|f| census.count(f) + held(f))
    .sum();
    out[18] = share(others, read);
    out[19] = share(
        census.count(Figure::Unexplained) + held(Figure::Unexplained),
        read,
    );

    // Texture.
    let sounding: Tick = notes.iter().map(|n| n.off.min(piece.end) - n.on).sum();
    out[20] = 1.0 - sounding as f64 / (piece.voices.len() as f64 * length as f64);
    let slots = (length / SIXTEENTH).max(1) as usize;
    let mut onsets = vec![0u8; slots + 1];
    for note in &notes {
        let slot = ((note.on - start) / SIXTEENTH) as usize;
        onsets[slot.min(slots)] += 1;
    }
    out[21] = share(onsets[..slots].iter().filter(|c| **c > 0).count(), slots);
    let together: usize =
        piece
            .voices
            .iter()
            .enumerate()
            .map(|(v, voice)| {
                voice
                    .notes
                    .iter()
                    .filter(|n| {
                        piece.voices.iter().enumerate().any(|(w, other)| {
                            w != v && other.at(n.on).is_some_and(|m| m.on == n.on)
                        })
                    })
                    .count()
            })
            .sum();
    out[22] = share(together, total);
    out
}

/// The `q`-quantile of the values by linear interpolation.
pub fn quantile(values: &[f64], q: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    if sorted.is_empty() {
        return 0.0;
    }
    let at = q * (sorted.len() - 1) as f64;
    let low = at.floor() as usize;
    let high = at.ceil() as usize;
    sorted[low] + (sorted[high] - sorted[low]) * (at - low as f64)
}

/// Bands of the measures over a corpus.
#[derive(Clone, Debug)]
pub struct Bands {
    pub low: [f64; COUNT],
    pub high: [f64; COUNT],
    pub median: [f64; COUNT],
}

impl Bands {
    /// The 10th and 90th percentiles of each measure over the rows.
    pub fn of(rows: &[[f64; COUNT]]) -> Bands {
        let mut bands = Bands {
            low: [0.0; COUNT],
            high: [0.0; COUNT],
            median: [0.0; COUNT],
        };
        for k in 0..COUNT {
            let values: Vec<f64> = rows.iter().map(|r| r[k]).collect();
            bands.low[k] = quantile(&values, 0.1);
            bands.high[k] = quantile(&values, 0.9);
            bands.median[k] = quantile(&values, 0.5);
        }
        bands
    }

    pub fn contains(&self, k: usize, value: f64) -> bool {
        // A small tolerance keeps a value on the edge inside in spite of rounding.
        value >= self.low[k] - 1e-12 && value <= self.high[k] + 1e-12
    }

    /// How many measures of a row lie inside their bands.
    pub fn hits(&self, row: &[f64; COUNT]) -> usize {
        (0..COUNT).filter(|k| self.contains(*k, row[*k])).count()
    }
}

/// The yardstick of T4: for each piece of the corpus the number of measures inside the bands of
/// the other pieces, and the 10th percentile of these numbers.
pub fn yardstick(rows: &[[f64; COUNT]]) -> (Vec<usize>, f64) {
    let hits: Vec<usize> = (0..rows.len())
        .map(|i| {
            let others: Vec<[f64; COUNT]> = rows
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, r)| *r)
                .collect();
            Bands::of(&others).hits(&rows[i])
        })
        .collect();
    let values: Vec<f64> = hits.iter().map(|h| *h as f64).collect();
    let mark = quantile(&values, 0.1);
    (hits, mark)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
    use crate::pitch::Pitch;
    use crate::score::Voice;
    use crate::time::Meter;

    #[test]
    fn quantiles_and_bands() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(quantile(&values, 0.5), 3.0);
        assert!((quantile(&values, 0.1) - 1.4).abs() < 1e-12);
        let rows: Vec<[f64; COUNT]> = (0..11).map(|i| [i as f64; COUNT]).collect();
        let bands = Bands::of(&rows);
        assert_eq!((bands.low[0], bands.high[0]), (1.0, 9.0));
        assert_eq!(bands.hits(&[5.0; COUNT]), COUNT);
        assert_eq!(bands.hits(&[10.0; COUNT]), 0);
    }

    #[test]
    fn measures_of_two_voices() {
        // Parallel fifths in quarters: C G, D A, E B; the lower voice repeats nothing.
        let line = |steps: &[i8], octave: i8| {
            let mut voice = Voice::default();
            for (i, step) in steps.iter().enumerate() {
                let on = 24 * i as Tick;
                voice.push(on, on + 24, Pitch::new(*step, 0, octave));
            }
            voice
        };
        let piece = Piece::new(
            "fifths",
            Key::parse("C").unwrap(),
            Meter::COMMON,
            vec![line(&[4, 5, 6], 4), line(&[0, 1, 2], 4)],
        );
        let m = measures(&piece);
        assert_eq!(m[2], 1.0);
        assert_eq!(m[5], 1.0);
        assert!((m[3] - 1.0).abs() < 1e-12);
        // Two parallel fifths on three beats.
        assert!((m[23] - 100.0 * 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(m[27], 1.0);
        assert_eq!(m[22], 1.0);
        assert_eq!(m[20], 0.0);
    }
}
