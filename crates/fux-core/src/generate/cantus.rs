//! Cantus firmus generation: a beam search over the notes, checked by the rule engine.

use super::counterpoint::{CounterpointRequest, NewVoice, generate_counterpoint};
use super::rng::Rng;
use super::{GenerateError, Generated};
use crate::clef::VoiceType;
use crate::mode::ModeSpec;
use crate::pitch::Pitch;
use crate::rules::{Analysis, Checker, Level, Profile, Window};
use crate::score::{Note, Role, Score, Species, Voice};
use crate::time::{Duration, TICKS_PER_WHOLE};

/// What to generate.
#[derive(Clone, Debug)]
pub struct CantusRequest {
    pub mode: ModeSpec,
    /// Number of notes.
    pub length: usize,
    pub voice_type: VoiceType,
    pub seed: u64,
    /// Number of distinct cantus firmi to return, best first.
    pub count: usize,
    /// Partial melodies kept per step; larger is slower and finds cheaper results.
    pub beam: usize,
    /// Random noise added to the costs when ranking, for variety (0 = always the cheapest).
    pub noise: f64,
}

impl CantusRequest {
    pub fn new(mode: ModeSpec, length: usize) -> CantusRequest {
        CantusRequest {
            mode,
            length,
            voice_type: VoiceType::Alto,
            seed: 1,
            count: 1,
            beam: 300,
            noise: 3.0,
        }
    }
}

/// The final for a voice: its octave is chosen as in Fux's examples, where the finals lie
/// between four steps below and two steps above the middle of the voice's free range.
pub fn final_for(mode: ModeSpec, voice_type: VoiceType) -> Pitch {
    let (low, high) = voice_type.free_range();
    let center = (low.diatonic() + high.diatonic()) / 2;
    (-1..=9)
        .map(|octave| mode.final_pitch(octave))
        .find(|f| (center - 4..=center + 2).contains(&f.diatonic()))
        .unwrap_or_else(|| mode.final_pitch(4))
}

/// Pitches the search may use: the scale pitches of the requested ambitus (authentic: final to
/// octave; plagal: fourth below to fifth above the final), at most three steps outside the staff.
/// Fux's authentic cantus firmi stay within this octave.
fn candidates(mode: ModeSpec, final_pitch: Pitch, voice_type: VoiceType) -> Vec<Pitch> {
    let (low, high) = mode.range(final_pitch.octave);
    let clef = voice_type.clef();
    (low.diatonic()..=high.diatonic())
        .map(|d| mode.scale_pitch(d))
        .filter(|p| clef.steps_outside(*p).abs() <= 3)
        .collect()
}

#[derive(Clone)]
struct State {
    pitches: Vec<Pitch>,
    cost: u32,
    /// Cost plus accumulated noise; the beam keeps the lowest keys.
    key: f64,
}

fn score_of(mode: ModeSpec, voice_type: VoiceType, pitches: &[Pitch]) -> Score {
    let notes = pitches
        .iter()
        .map(|&p| Note::new(p, Duration::WHOLE))
        .collect();
    Score::new(
        mode,
        vec![Voice::new(voice_type, Role::CantusFirmus, notes)],
    )
}

/// Generates cantus firmi in a mode. Every result breaks no prohibition of the profile.
pub fn generate_cantus(
    request: &CantusRequest,
    profile: &Profile,
) -> Result<Vec<Generated>, GenerateError> {
    let length = request.length;
    if length < 3 {
        return Err(GenerateError::Invalid(format!(
            "a cantus needs at least 3 notes, not {length}"
        )));
    }
    let mode = request.mode;
    let voice_type = request.voice_type;
    let checker = Checker::new(profile);
    let final_pitch = final_for(mode, voice_type);
    let penultimate = mode.scale_pitch(final_pitch.diatonic() + 1);
    let pool = candidates(mode, final_pitch, voice_type);
    let mut rng = Rng::new(request.seed);
    let mut beam = vec![State {
        pitches: vec![final_pitch],
        cost: 0,
        key: 0.0,
    }];
    let mut violations = Vec::new();
    for position in 1..length {
        let choices: &[Pitch] = if position == length - 1 {
            std::slice::from_ref(&final_pitch)
        } else if position == length - 2 {
            std::slice::from_ref(&penultimate)
        } else {
            &pool
        };
        let complete = position == length - 1;
        let from = position as u32 * TICKS_PER_WHOLE;
        let mut next = Vec::with_capacity(beam.len() * choices.len());
        for state in &beam {
            for &pitch in choices {
                let mut pitches = state.pitches.clone();
                pitches.push(pitch);
                let score = score_of(mode, voice_type, &pitches);
                // Rules about the whole piece report where the problem lies, which can be before
                // the newest note; the last step therefore checks everything.
                let cost = if complete {
                    let report = checker.check(&score);
                    if report.forbidden() > 0 {
                        continue;
                    }
                    report.cost()
                } else {
                    let analysis = Analysis::new(&score);
                    violations.clear();
                    let window = Window::partial(from, from);
                    checker.check_window(&score, &analysis, window, &[true], &mut violations);
                    if violations.iter().any(|v| v.is_forbidden()) {
                        continue;
                    }
                    state.cost + violations.iter().map(|v| v.level.cost()).sum::<u32>()
                };
                let noise = rng.next_f64() * request.noise;
                next.push(State {
                    pitches,
                    key: state.key + f64::from(cost - state.cost.min(cost)) + noise,
                    cost,
                });
            }
        }
        if next.is_empty() {
            return Err(GenerateError::NoSolution { position });
        }
        next.sort_by(|a, b| a.key.total_cmp(&b.key));
        next.truncate(request.beam.max(1));
        beam = next;
    }
    beam.sort_by(|a, b| a.cost.cmp(&b.cost).then(a.key.total_cmp(&b.key)));
    let solvable = checker.level("CF-SOLVABLE", "level");
    let mut results: Vec<Generated> = Vec::new();
    for state in beam {
        if results.len() >= request.count.max(1) {
            break;
        }
        let score = score_of(mode, voice_type, &state.pitches);
        if results
            .iter()
            .any(|r| r.score.voices[0].notes == score.voices[0].notes)
        {
            continue;
        }
        let mut cost_extra = 0;
        if solvable != Level::Off && !is_solvable(&score, profile) {
            if solvable == Level::Forbidden {
                continue;
            }
            cost_extra = solvable.cost();
        }
        let report = checker.check(&score);
        results.push(Generated {
            cost: report.cost() + cost_extra,
            report,
            score,
            warnings: Vec::new(),
            subjects: Vec::new(),
        });
    }
    if results.is_empty() {
        return Err(GenerateError::NoSolution { position: length });
    }
    Ok(results)
}

/// Whether a first-species counterpoint exists above and below the cantus (where the voice has
/// a neighbor), found with a small search.
fn is_solvable(score: &Score, profile: &Profile) -> bool {
    let cantus = score.voices[0].voice_type;
    [true, false]
        .into_iter()
        .filter_map(|above| cantus.neighbor(above))
        .all(|voice_type| {
            let request = CounterpointRequest {
                beam: 30,
                noise: 0.0,
                ..CounterpointRequest::new(
                    score.clone(),
                    vec![NewVoice {
                        voice_type,
                        species: Species::First,
                    }],
                )
            };
            generate_counterpoint(&request, profile).is_ok()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::Mode;

    #[test]
    fn finals_lie_where_fux_puts_them() {
        let finals: Vec<String> = Mode::ALL
            .into_iter()
            .map(|m| final_for(ModeSpec::new(m), VoiceType::Alto).to_string())
            .collect();
        assert_eq!(finals, ["D4", "E4", "F3", "G3", "A3", "C4"]);
        let bass = final_for(ModeSpec::new(Mode::Dorian), VoiceType::Bass);
        assert_eq!(bass.to_string(), "D3");
    }

    #[test]
    fn generated_cantus_follow_the_rules() {
        let profile = Profile::fux();
        let request = CantusRequest {
            count: 3,
            ..CantusRequest::new(ModeSpec::new(Mode::Dorian), 11)
        };
        let results = generate_cantus(&request, &profile).unwrap();
        assert_eq!(results.len(), 3);
        for r in &results {
            assert_eq!(r.report.forbidden(), 0);
            let pitches = r.score.voices[0].pitches();
            assert_eq!(pitches.len(), 11);
            assert_eq!(pitches[0], pitches[10]);
        }
        assert!(results[0].cost <= results[1].cost);
    }

    #[test]
    fn same_seed_same_cantus() {
        let profile = Profile::fux();
        let request = CantusRequest::new(ModeSpec::new(Mode::Phrygian), 10);
        let a = generate_cantus(&request, &profile).unwrap();
        let b = generate_cantus(&request, &profile).unwrap();
        assert_eq!(a[0].score, b[0].score);
        let other = CantusRequest {
            seed: 99,
            ..request.clone()
        };
        let c = generate_cantus(&other, &profile).unwrap();
        assert_ne!(a[0].score, c[0].score);
    }

    #[test]
    fn too_short_is_invalid() {
        let request = CantusRequest::new(ModeSpec::new(Mode::Dorian), 2);
        assert!(generate_cantus(&request, &Profile::fux()).is_err());
    }
}
