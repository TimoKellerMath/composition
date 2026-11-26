//! The bass of the scaffold: which tone of each chord lies lowest where the chord begins.
//!
//! The model is counted on the figures of Tymoczko's analyses of seven fugues of the
//! Well-Tempered Clavier (When in Rome): by kind of chord how often root, third, fifth and
//! seventh are the bass, and how often each way leads from one bass to the next. The plan draws
//! a bass for every chord under which a free voice is the lowest; where a given voice is the
//! lowest its tone is the bass, and the close has the root.

use serde_json::{Value, json};

use crate::draft::{Draft, VoiceDraft};
use crate::key::{Chord, Harmony, Kind};
use crate::pitch::Pitch;
use crate::rng::Rng;
use crate::roman;
use crate::score::Piece;
use crate::time::Tick;

/// The kinds of chords the model tells apart.
pub const GROUPS: [&str; 6] = [
    "major triad",
    "minor triad",
    "dim./aug. triad",
    "dominant seventh",
    "(half-)dim. seventh",
    "other seventh",
];

/// The group of a kind of chord.
pub fn group(kind: Kind) -> usize {
    match kind {
        Kind::Maj => 0,
        Kind::Min => 1,
        Kind::Dim | Kind::Aug => 2,
        Kind::Dom7 | Kind::Dom9 | Kind::DomMin9 => 3,
        Kind::HalfDim7 | Kind::Dim7 => 4,
        _ => 5,
    }
}

/// What the analyses say of the bass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    /// Chords counted.
    pub labels: f64,
    /// By group of chord: how often root, third, fifth and seventh are the bass.
    pub members: [[f64; 4]; 6],
    /// From one bass to the next, in semitones upwards (0 to 11).
    pub ways: [f64; 12],
    /// The same after a seventh in the bass.
    pub after_seventh: [f64; 12],
}

impl Model {
    /// Counts the basses of an analysis.
    pub fn learn(&mut self, analysis: &roman::Analysis) {
        let mut before: Option<(u8, usize)> = None;
        for label in &analysis.labels {
            let named = roman::chord(&label.figure, label.key).and_then(|chord| {
                let bass = chord.bass?;
                let member = chord.tones().position(|t| t == bass).filter(|m| *m < 4)?;
                Some((chord, bass, member))
            });
            let Some((chord, bass, member)) = named else {
                before = None;
                continue;
            };
            self.labels += 1.0;
            self.members[group(chord.kind)][member] += 1.0;
            if let Some((pc, last)) = before {
                let way = ((bass.pc() + 12 - pc) % 12) as usize;
                self.ways[way] += 1.0;
                if last == 3 {
                    self.after_seventh[way] += 1.0;
                }
            }
            before = Some((bass.pc(), member));
        }
    }

    pub fn is_empty(&self) -> bool {
        self.labels == 0.0
    }

    /// How much less likely than its root this tone is as the bass of the chord, as a
    /// logarithm (0 for the root, for a tone foreign to the chord and without a model).
    pub fn log_against_root(&self, chord: Chord, pc: u8) -> f64 {
        let Some(member) = chord.member(pc).filter(|m| *m < 4) else {
            return 0.0;
        };
        let counts = &self.members[group(chord.kind)];
        ((counts[member] + 0.5) / (counts[0] + 0.5)).ln()
    }

    pub fn to_json(&self) -> Value {
        json!({
            "labels": self.labels,
            "members": self.members,
            "ways": self.ways,
            "after_seventh": self.after_seventh,
        })
    }

    pub fn from_json(value: &Value) -> Model {
        let row = |value: &Value, out: &mut [f64]| {
            for (slot, number) in out.iter_mut().zip(value.as_array().into_iter().flatten()) {
                *slot = number.as_f64().unwrap_or(0.0);
            }
        };
        let mut model = Model {
            labels: value["labels"].as_f64().unwrap_or(0.0),
            ..Model::default()
        };
        for (k, members) in model.members.iter_mut().enumerate() {
            row(&value["members"][k], members);
        }
        row(&value["ways"], &mut model.ways);
        row(&value["after_seventh"], &mut model.after_seventh);
        model
    }
}

/// What sounds lowest at the onsets of the chords of a piece.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Lowest {
    /// Onsets of chords with at least two sounding voices.
    pub onsets: usize,
    /// The lowest tone as a member of the chord: root, third, fifth, seventh, none of these.
    pub members: [usize; 5],
    /// Onsets whose chord names its bass, and those of them where the lowest tone is it.
    pub named: usize,
    pub met: usize,
}

impl Lowest {
    pub fn add(&mut self, other: &Lowest) {
        self.onsets += other.onsets;
        for (mine, theirs) in self.members.iter_mut().zip(other.members) {
            *mine += theirs;
        }
        self.named += other.named;
        self.met += other.met;
    }
}

/// The lowest sounding tone at every onset of a chord, as the rule HAR-BASS reads it: with two
/// voices and more.
pub fn lowest(piece: &Piece, harmony: &Harmony) -> Lowest {
    let mut out = Lowest::default();
    for span in &harmony.spans {
        let sounding: Vec<_> = piece
            .voices
            .iter()
            .filter_map(|v| v.at(span.on))
            .map(|n| n.pitch)
            .collect();
        if sounding.len() < 2 {
            continue;
        }
        let low = sounding.iter().min().expect("two sounding");
        out.onsets += 1;
        let member = span.chord.member(low.pc()).filter(|m| *m < 4).unwrap_or(4);
        out.members[member] += 1;
        if let Some(bass) = span.chord.bass {
            out.named += 1;
            out.met += (bass.pc() == low.pc()) as usize;
        }
    }
    out
}

/// What is known of the lowest voice where a chord begins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fixed {
    /// Fewer than two voices sound: no bass is named.
    None,
    /// A given voice is the lowest: the pitch class of its tone.
    Tone(u8),
    /// The plan wants the root (the close).
    Root,
    /// A free voice is the lowest: the model chooses.
    Free,
}

/// A voice at a tick: silent (none), free (no tone yet), or with a given tone.
fn state(voice: &VoiceDraft, t: Tick) -> Option<Option<Pitch>> {
    let inside = |spans: &[(Tick, Tick)]| spans.iter().any(|(a, b)| *a <= t && t < *b);
    match voice.given.iter().find(|n| n.on <= t && t < n.off) {
        Some(note) => Some(Some(note.pitch)),
        None if inside(&voice.spans) || inside(&voice.silent) => None,
        None => Some(None),
    }
}

/// The voices that sound at a tick, the lowest of them (free: no tone yet), and the lowest
/// given tone among them. The voices come from the top down.
fn sounding(voices: &[VoiceDraft], t: Tick) -> (usize, Option<Option<Pitch>>, Option<Pitch>) {
    let mut count = 0;
    let mut lowest = None;
    let mut given_low: Option<Pitch> = None;
    for voice in voices {
        if let Some(state) = state(voice, t) {
            count += 1;
            lowest = Some(state);
            if let Some(pitch) = state {
                given_low = Some(given_low.map_or(pitch, |g| g.min(pitch)));
            }
        }
    }
    (count, lowest, given_low)
}

/// The lowest sounding tone at a tick if the lowest voice that sounds there is given, and not
/// with the tone of a pedal point (the chords over a pedal point are not chords on it).
pub fn lowest_given(voices: &[VoiceDraft], t: Tick) -> Option<Pitch> {
    let (_, lowest, given_low) = sounding(voices, t);
    let pitch = given_low.filter(|_| matches!(lowest, Some(Some(_))))?;
    let pedal = voices.iter().any(|voice| {
        voice.pedals.iter().any(|(a, b)| *a <= t && t < *b)
            && voice
                .given
                .iter()
                .any(|n| n.on <= t && t < n.off && n.pitch == pitch)
    });
    (!pedal).then_some(pitch)
}

/// What is known of the lowest voice at the onset of every chord of the harmony, from the
/// draft of a piece: its given tones, where its voices are silent, its close.
pub fn fixed(draft: &Draft, harmony: &Harmony) -> Vec<Fixed> {
    harmony
        .spans
        .iter()
        .map(|span| {
            let t = span.on;
            if t >= draft.end {
                return Fixed::None;
            }
            let (count, lowest, given_low) = sounding(&draft.voices, t);
            if count < 2 {
                return Fixed::None;
            }
            if draft.root_at_hold && draft.final_hold.is_some_and(|hold| t >= hold) {
                return Fixed::Root;
            }
            match lowest {
                Some(Some(_)) => Fixed::Tone(given_low.expect("a given tone sounds").pc()),
                Some(None) => Fixed::Free,
                None => Fixed::None,
            }
        })
        .collect()
}

/// A bass a chord may get: the member of the chord (none: a tone foreign to it or no tone at
/// all), its pitch class, its weight, and whether the plan chose it.
#[derive(Clone, Copy, Debug)]
struct State {
    member: Option<usize>,
    pc: Option<u8>,
    weight: f64,
    chosen: bool,
}

/// The weight of the way from one bass to the next.
fn way(model: &Model, a: &State, b: &State) -> f64 {
    let (Some(from), Some(to)) = (a.pc, b.pc) else {
        // A seventh the plan chooses needs a bass below it to go to.
        return if a.chosen && a.member == Some(3) {
            0.0
        } else {
            1.0
        };
    };
    let way = ((to + 12 - from) % 12) as usize;
    if a.member == Some(3) {
        // A seventh in the bass goes down by step; one the plan chooses has to.
        let down = way == 10 || way == 11;
        if a.chosen && !down {
            0.0
        } else {
            model.after_seventh[way] + if down { 1.0 } else { 0.05 }
        }
    } else {
        model.ways[way] + 0.5
    }
}

/// Names the bass of every chord: the given tone where a given voice is the lowest, the root
/// at the close, and else a tone drawn from the model (forward filtering, backward sampling),
/// so that the basses follow each other as in the analyses. The fifth of a triad is never
/// drawn, and a seventh only where the next bass lies a step below it.
pub fn assign(harmony: &mut Harmony, fixed: &[Fixed], model: &Model, seed: u64) {
    let count = harmony.spans.len();
    debug_assert_eq!(count, fixed.len());
    if count == 0 || model.is_empty() {
        return;
    }
    let states: Vec<Vec<State>> = harmony
        .spans
        .iter()
        .zip(fixed)
        .enumerate()
        .map(|(i, (span, fixed))| {
            let chord = span.chord;
            let tones: Vec<u8> = chord.tones().take(4).map(|t| t.pc()).collect();
            match fixed {
                Fixed::None => vec![State {
                    member: None,
                    pc: None,
                    weight: 1.0,
                    chosen: false,
                }],
                Fixed::Tone(pc) => vec![State {
                    member: tones.iter().position(|t| t == pc),
                    pc: Some(*pc),
                    weight: 1.0,
                    chosen: false,
                }],
                Fixed::Root => vec![State {
                    member: Some(0),
                    pc: Some(tones[0]),
                    weight: 1.0,
                    chosen: false,
                }],
                Fixed::Free => (0..tones.len())
                    // No six-four chord is planned, and no seventh in the bass at the end.
                    .filter(|m| !(tones.len() == 3 && *m == 2))
                    .filter(|m| !(*m == 3 && i + 1 == count))
                    .map(|m| State {
                        member: Some(m),
                        pc: Some(tones[m]),
                        weight: model.members[group(chord.kind)][m] + 0.5,
                        chosen: true,
                    })
                    .collect(),
            }
        })
        .collect();
    // Forward: the weight of every bass given all before it.
    let mut forward: Vec<Vec<f64>> = Vec::with_capacity(count);
    for (i, here) in states.iter().enumerate() {
        let mut row: Vec<f64> = here
            .iter()
            .map(|b| match i {
                0 => b.weight,
                _ => {
                    b.weight
                        * states[i - 1]
                            .iter()
                            .zip(&forward[i - 1])
                            .map(|(a, w)| w * way(model, a, b))
                            .sum::<f64>()
                }
            })
            .collect();
        let total: f64 = row.iter().sum();
        if total > 0.0 {
            for w in &mut row {
                *w /= total;
            }
        } else {
            // Nothing leads here (a given tone after a chosen seventh): the chain starts anew.
            for (w, b) in row.iter_mut().zip(here) {
                *w = b.weight;
            }
        }
        forward.push(row);
    }
    // Backward: draw the last bass, then each one given the one after it.
    let mut rng = Rng::new(seed ^ 0xBA55_0000_0000_0001);
    let mut picked = vec![0usize; count];
    for i in (0..count).rev() {
        let weights: Vec<f64> = states[i]
            .iter()
            .zip(&forward[i])
            .map(|(a, w)| {
                if i + 1 == count {
                    *w
                } else {
                    w * way(model, a, &states[i + 1][picked[i + 1]])
                }
            })
            .collect();
        picked[i] = rng
            .weighted(&weights)
            .or_else(|| rng.weighted(&forward[i]))
            .unwrap_or(0);
    }
    for (i, span) in harmony.spans.iter_mut().enumerate() {
        let state = states[i][picked[i]];
        span.chord.bass = state.member.and_then(|m| span.chord.tones().nth(m));
    }
}

/// The bass of every chord of the scaffold of a piece, from its draft.
pub fn plan(draft: &Draft, harmony: &mut Harmony, model: &Model, seed: u64) {
    let fixed = fixed(draft, harmony);
    assign(harmony, &fixed, model, seed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::{Chord, Key};
    use crate::pitch::Lof;
    use crate::score::Voice;
    use crate::themes::parse;
    use crate::time::Meter;

    /// A model with what the seven fugues roughly say.
    fn model() -> Model {
        let mut model = Model {
            labels: 1000.0,
            ..Model::default()
        };
        model.members[0] = [67.0, 30.0, 3.0, 0.0];
        model.members[1] = [63.0, 32.0, 5.0, 0.0];
        model.members[2] = [37.0, 59.0, 4.0, 0.0];
        model.members[3] = [50.0, 29.0, 5.0, 16.0];
        model.members[4] = [45.0, 27.0, 19.0, 9.0];
        model.members[5] = [60.0, 23.0, 8.0, 9.0];
        model.ways = [
            16.0, 8.0, 8.0, 3.0, 3.0, 22.0, 2.0, 4.0, 10.0, 11.0, 7.0, 7.0,
        ];
        model.after_seventh = [3.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 4.0, 2.0, 2.0, 25.0, 28.0];
        model
    }

    /// C, G7, C, F, G7, C over and over.
    fn harmony(bars: usize) -> Harmony {
        let key = Key::major(Lof(0));
        let chords = [
            Chord::new(Lof(0), Kind::Maj),
            Chord::new(Lof(1), Kind::Dom7),
            Chord::new(Lof(0), Kind::Maj),
            Chord::new(Lof(-1), Kind::Maj),
            Chord::new(Lof(1), Kind::Dom7),
            Chord::new(Lof(0), Kind::Maj),
        ];
        let mut harmony = Harmony::default();
        for k in 0..bars {
            // Spans of the same chord in a row stay apart: each is an onset.
            harmony.spans.push(crate::key::Span {
                on: 48 * k as i32,
                chord: chords[k % chords.len()],
                key,
            });
        }
        harmony
    }

    #[test]
    fn a_bass_for_every_chord_under_a_free_voice() {
        let mut harmony = harmony(600);
        let fixed = vec![Fixed::Free; harmony.spans.len()];
        assign(&mut harmony, &fixed, &model(), 1);
        let mut members = [0usize; 4];
        for (i, span) in harmony.spans.iter().enumerate() {
            let bass = span.chord.bass.expect("every chord names its bass");
            let member = span.chord.tones().position(|t| t == bass).unwrap();
            members[member] += 1;
            // No six-four chord is planned.
            assert!(!(span.chord.kind == Kind::Maj && member == 2));
            // A seventh in the bass goes down by step.
            if member == 3 {
                let next = harmony.spans[i + 1].chord.bass.unwrap();
                let way = (next.pc() + 12 - bass.pc()) % 12;
                assert!(way == 10 || way == 11, "seventh at {i}: way {way}");
            }
        }
        // Root position leads, the inversions come: about as the model has them.
        assert!(members[0] > 300 && members[1] > 100, "{members:?}");
        assert!(members[3] > 5, "{members:?}");
        // The same seed, the same bass; another seed, another one.
        let mut again = self::harmony(600);
        assign(&mut again, &fixed, &model(), 1);
        assert_eq!(again.spans, harmony.spans);
        let mut other = self::harmony(600);
        assign(&mut other, &fixed, &model(), 2);
        assert_ne!(other.spans, harmony.spans);
    }

    #[test]
    fn given_tones_and_the_close_stand() {
        let mut harmony = harmony(6);
        // The third of C, a tone foreign to G7, nothing, a free voice, the fifth of G7, the
        // root at the close.
        let fixed = [
            Fixed::Tone(4),
            Fixed::Tone(0),
            Fixed::None,
            Fixed::Free,
            Fixed::Tone(2),
            Fixed::Root,
        ];
        assign(&mut harmony, &fixed, &model(), 3);
        let basses: Vec<Option<u8>> = harmony
            .spans
            .iter()
            .map(|s| s.chord.bass.map(|b| b.pc()))
            .collect();
        assert_eq!(basses[0], Some(4));
        assert_eq!(basses[1], None);
        assert_eq!(basses[2], None);
        assert!(matches!(basses[3], Some(5) | Some(9)), "{basses:?}");
        assert_eq!(basses[4], Some(2));
        assert_eq!(basses[5], Some(0));
        // Without a model nothing is named.
        let mut plain = self::harmony(6);
        assign(&mut plain, &fixed, &Model::default(), 3);
        assert!(plain.spans.iter().all(|s| s.chord.bass.is_none()));
    }

    #[test]
    fn the_model_reads_what_it_writes() {
        let model = model();
        assert_eq!(Model::from_json(&model.to_json()), model);
        assert!(Model::from_json(&json!({})).is_empty());
    }

    #[test]
    fn the_lowest_tone_at_the_onsets() {
        // C major, then G major over its third, then a chord alone in one voice.
        let top = parse("e''2 d''2 | c''1").unwrap();
        let low = parse("c'2 b2 | r1").unwrap();
        let key = Key::major(Lof(0));
        let piece = Piece::new(
            "test",
            key,
            Meter::COMMON,
            vec![Voice { notes: top }, Voice { notes: low }],
        );
        let mut harmony = Harmony::default();
        harmony.push(0, Chord::new(Lof(0), Kind::Maj).inverted(0), key);
        harmony.push(48, Chord::new(Lof(1), Kind::Maj).inverted(0), key);
        harmony.push(96, Chord::new(Lof(0), Kind::Maj), key);
        let count = lowest(&piece, &harmony);
        assert_eq!(
            count,
            Lowest {
                onsets: 2,
                members: [1, 1, 0, 0, 0],
                named: 2,
                met: 1,
            }
        );
    }
}
