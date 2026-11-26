//! The opening chord of a part that restarts in all voices together (MASS-RESTART,
//! docs/design/phase-14/spec.md § 3.7). Without it a joint restart came out bare: none of the
//! ten plain restarts in the prototype's Dorian mass had a third, and two put all four voices on
//! one pitch class, because TEX-TRIAD skips the first measure and FREE-ENTRY prices a third. The
//! plan therefore fixes a full triad of the final (in two voices a perfect consonance over it),
//! which every voice sings at the part's start.
//!
//! The voicing is deterministic: the lowest voice on the root, each other voice on the chord
//! tone of its staff nearest the middle of its staff, so that the voices start where they move
//! freely; no voice below the one under it and no two adjacent upper voices more than an octave
//! apart; the third present, once only up to four voices, and the fifth from four voices on.
//! Two voices begin on a perfect consonance over the final instead, as CAD-START demands of
//! every two-voice setting (Fux ch. 54): the upper voice on the root or the fifth, never the
//! third. Where several voicings keep these rules, the one with the fewest unisons wins, then the
//! one with the fewest tones outside the octaves of the mode the voices move in (FREE-AMBITUS:
//! the Mixolydian bass on G2 rather than G3, whose octave reaches up to E3 only), then the one
//! nearest the middles of the staves.

use crate::clef::VoiceType;
use crate::mode::ModeSpec;
use crate::pitch::{Pitch, Step};
use crate::rules::modal_octave;

/// The widest interval between two adjacent upper voices, in semitones: an octave.
const SPACING: i32 = 12;

/// What a voicing must keep, from the strictest; where no voicing keeps every rule, the next
/// level gives up the last of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Strictness {
    /// Every rule.
    All,
    /// Without the spacing of the upper voices.
    Unspaced,
    /// Only the root in the lowest voice and no crossing.
    Ordered,
}

/// A chord tone a voice may take: its pitch, its distance from the middle of the voice's staff
/// in semitones, and whether it lies more than a step outside the voice's octave of the mode.
#[derive(Clone, Copy, Debug)]
struct Candidate {
    pitch: Pitch,
    distance: i32,
    outside: bool,
}

/// The chord on `root` (usually the final of `mode`) for `voices` from the top: one pitch per
/// voice, the root position triad with the mode's own third and fifth, spelled in the scale
/// (D F A in the Dorian, G B♭ D in the Dorian on G); for two voices the root and the fifth or
/// the octave above it, a perfect consonance.
pub fn opening_chord(mode: ModeSpec, voices: &[VoiceType], root: Step) -> Vec<Pitch> {
    if voices.is_empty() {
        return Vec::new();
    }
    let root_pitch = Pitch::new(root, mode.scale_alter(root), 4);
    let tones = [0, 2, 4].map(|steps| mode.scale_pitch(root_pitch.diatonic() + steps).step);
    let perfect = [tones[0], tones[2]];
    // The candidates of each voice: its chord tones within its staff, those within its octave of
    // the mode first, then nearest the middle, on a tie the lower; the lowest voice takes the
    // root only, the upper of two voices the root or the fifth.
    let candidates: Vec<Vec<Candidate>> = voices
        .iter()
        .enumerate()
        .map(|(v, &voice)| {
            let clef = voice.clef();
            let (bottom, top) = (clef.bottom_line().diatonic(), clef.top_line().diatonic());
            let middle = Pitch::from_diatonic((bottom + top) / 2).midi();
            let octave = modal_octave(mode, voice);
            let allowed: &[Step] = if v + 1 == voices.len() {
                &tones[..1]
            } else if voices.len() == 2 {
                &perfect
            } else {
                &tones
            };
            let mut found: Vec<Candidate> = (bottom..=top)
                .map(|d| mode.scale_pitch(d))
                .filter(|p| allowed.contains(&p.step))
                .map(|pitch| {
                    let d = pitch.diatonic();
                    Candidate {
                        pitch,
                        distance: (pitch.midi() - middle).abs(),
                        outside: d < octave - 1 || d > octave + 8,
                    }
                })
                .collect();
            found.sort_by_key(|c| (c.outside, c.distance, c.pitch.midi()));
            found
        })
        .collect();
    for strictness in [Strictness::All, Strictness::Unspaced, Strictness::Ordered] {
        let mut search = Search {
            candidates: &candidates,
            tones,
            strictness,
            chosen: Vec::with_capacity(voices.len()),
            best: None,
        };
        search.descend(0, (0, 0, 0));
        if let Some((_, chord)) = search.best {
            return chord;
        }
    }
    // A voice whose staff holds no root: the nearest tones, from the top.
    candidates
        .iter()
        .map(|c| c.first().map_or(root_pitch, |c| c.pitch))
        .collect()
}

/// What ranks a voicing, the least first: its unisons, its tones outside the octaves of the mode
/// and its distance from the middles of the staves.
type Rank = (usize, usize, i32);

/// The search of a voicing: every voice from the top takes one of its candidates.
struct Search<'a> {
    candidates: &'a [Vec<Candidate>],
    /// Root, third and fifth.
    tones: [Step; 3],
    strictness: Strictness,
    chosen: Vec<Pitch>,
    /// The best voicing so far, with its rank.
    best: Option<(Rank, Vec<Pitch>)>,
}

impl Search<'_> {
    fn descend(&mut self, v: usize, rank: Rank) {
        if self.best.as_ref().is_some_and(|(best, _)| *best <= rank) {
            return;
        }
        if v == self.candidates.len() {
            if self.complete() {
                self.best = Some((rank, self.chosen.clone()));
            }
            return;
        }
        let upper = self.candidates.len() - 1;
        let (unisons, outside, distance) = rank;
        for &candidate in &self.candidates[v] {
            let pitch = candidate.pitch;
            let unison = match self.chosen.last() {
                None => false,
                Some(above) => {
                    let gap = above.midi() - pitch.midi();
                    if gap < 0 {
                        continue;
                    }
                    let spaced = v == upper || gap <= SPACING;
                    if !spaced && self.strictness == Strictness::All {
                        continue;
                    }
                    gap == 0
                }
            };
            self.chosen.push(pitch);
            self.descend(
                v + 1,
                (
                    unisons + usize::from(unison),
                    outside + usize::from(candidate.outside),
                    distance + candidate.distance,
                ),
            );
            self.chosen.pop();
        }
    }

    /// Whether the chosen voicing has its third, once up to four voices, and from four voices
    /// its fifth; two voices have no third.
    fn complete(&self) -> bool {
        if self.strictness == Strictness::Ordered {
            return true;
        }
        let count = |step: Step| self.chosen.iter().filter(|p| p.step == step).count();
        let (thirds, fifths) = (count(self.tones[1]), count(self.tones[2]));
        match self.chosen.len() {
            2 => thirds == 0,
            voices => thirds >= 1 && (voices > 4 || thirds == 1) && (voices < 4 || fifths >= 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mass::choir;

    fn names(chord: &[Pitch]) -> Vec<String> {
        chord.iter().map(Pitch::to_string).collect()
    }

    #[test]
    fn dorian_chords_in_the_usual_choirs() {
        let dorian: ModeSpec = "dorian".parse().unwrap();
        let chord = |voices: usize| names(&opening_chord(dorian, &choir(voices), Step::D));
        assert_eq!(chord(4), ["F4", "D4", "A3", "D3"]);
        assert_eq!(chord(3), ["F4", "A3", "D3"]);
        // Two voices on a perfect consonance: the fifth above the final.
        assert_eq!(chord(2), ["A4", "D4"]);
        let g_dorian = dorian.with_transposition(true);
        let chord = opening_chord(g_dorian, &choir(4), Step::G);
        assert!(
            chord.iter().any(|p| p.to_string().starts_with("Bb")),
            "{chord:?}"
        );
        assert_eq!(chord[3].step, Step::G);
    }

    /// The bass of a chord on G stays in its octave of the mode: the Mixolydian bass moves
    /// between D2 and D3 (FREE-AMBITUS lets it reach E3), so it begins on G2, not on G3 in a
    /// chord packed into one octave. In the usual choirs of three and four voices no chord tone
    /// lies outside its voice's octave in the modes on G, nor in any other mode.
    #[test]
    fn mixolydian_bass_in_its_octave() {
        let mixolydian: ModeSpec = "mixolydian".parse().unwrap();
        let chord = |voices: usize| names(&opening_chord(mixolydian, &choir(voices), Step::G));
        assert_eq!(chord(4), ["G4", "D4", "B3", "G2"]);
        assert_eq!(chord(3), ["B3", "G3", "G2"]);
        for mode in crate::mode::Mode::ALL {
            for transposed in [false, true] {
                let mode = ModeSpec::new(mode).with_transposition(transposed);
                for voices in [choir(3), choir(4)] {
                    let chord = opening_chord(mode, &voices, mode.final_step());
                    for (pitch, &voice) in chord.iter().zip(&voices) {
                        let octave = modal_octave(mode, voice);
                        let d = pitch.diatonic();
                        assert!(
                            octave - 1 <= d && d <= octave + 8,
                            "{mode} {voice:?}: {pitch}"
                        );
                    }
                }
            }
        }
    }
}
