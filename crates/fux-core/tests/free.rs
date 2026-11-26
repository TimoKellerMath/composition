//! Free counterpoint (phase 11): settings without a cantus firmus in every mode, and free voices
//! around a cantus. The full matrix runs with `cargo test --release -- --ignored`.

mod common;

use fux_core::generate::{
    CounterpointRequest, FreeRequest, NewVoice, default_cadences, generate_counterpoint,
    generate_free,
};
use fux_core::rules::{Checker, Profile};
use fux_core::{Mode, ModeSpec, Role, Score, Species, VoiceType};

/// The usual free choir of `voices` voices (as `fux free` sets it).
fn choir(voices: usize) -> Vec<VoiceType> {
    use VoiceType::*;
    match voices {
        2 => vec![Soprano, Tenor],
        3 => vec![Alto, Tenor, Bass],
        4 => vec![Soprano, Alto, Tenor, Bass],
        5 => vec![Soprano, Alto, Tenor, Tenor, Bass],
        _ => vec![Soprano, Soprano, Alto, Tenor, Tenor, Bass],
    }
}

/// A free setting with the usual cadences; panics if it breaks a prohibition.
fn setting(mode: ModeSpec, voices: usize, measures: usize, beam: usize) -> Score {
    let request = FreeRequest {
        cadences: default_cadences(mode, measures),
        beam,
        ..FreeRequest::new(mode, choir(voices), measures)
    };
    let label = format!("{}, {voices} voices, {measures} measures", mode.id());
    let results =
        generate_free(&request, &Profile::fux()).unwrap_or_else(|e| panic!("{label}: {e}"));
    let generated = &results[0];
    assert_eq!(generated.report.forbidden(), 0, "{label}");
    let score = &generated.score;
    score.validate().unwrap();
    assert!(score.is_free(), "{label}");
    assert_eq!(score.voices.len(), voices, "{label}");
    assert_eq!(score.measure_count(), measures, "{label}");
    // The last sonority rests on the final in the bass.
    let bass = score.voices.last().unwrap().pitches();
    assert_eq!(bass.last().unwrap().step, mode.final_step(), "{label}");
    generated.score.clone()
}

/// Acceptance test of phase 11: two voices in all modes, untransposed and transposed, without a
/// prohibition.
#[test]
fn two_voices_in_every_mode() {
    for mode in Mode::ALL {
        for transposed in [false, true] {
            setting(
                ModeSpec::new(mode).with_transposition(transposed),
                2,
                10,
                100,
            );
        }
    }
}

/// Three and four voices with an internal cadence, with both kinds of clausula (Dorian:
/// leading tone; Phrygian: semitone from above).
#[test]
fn three_and_four_voices() {
    let dorian = ModeSpec::new(Mode::Dorian);
    let phrygian = ModeSpec::new(Mode::Phrygian);
    setting(dorian, 3, 10, 100);
    setting(phrygian, 3, 10, 100);
    setting(dorian, 4, 10, 100);
}

/// The search sees free voices only up to the current tick: the rules must decide everything as
/// they do in the whole score, window by window and prefix by prefix.
#[test]
fn generated_settings_check_window_by_window() {
    let checker = Checker::new(&Profile::fux());
    for (mode, voices) in [(Mode::Dorian, 2), (Mode::Aeolian, 3), (Mode::Mixolydian, 4)] {
        let score = setting(ModeSpec::new(mode), voices, 10, 60);
        let name = format!("{mode:?}, {voices} voices");
        common::assert_windows_partition(&checker, &score, &name);
        common::assert_prefixes_collect(&checker, &score, &name);
    }
}

/// Free voices around a cantus firmus in whole notes.
#[test]
fn free_voices_around_a_cantus() {
    use fux_core::corpus::fux_cantus;
    use fux_core::{Duration, Note, Pitch, Voice};

    let cantus = fux_cantus().into_iter().find(|c| c.id == "dorian").unwrap();
    let notes = cantus
        .notes
        .iter()
        .map(|p| Note::new(Pitch::new(p.step, p.alter, p.octave), Duration::WHOLE))
        .collect();
    let base = Score::new(
        cantus.mode,
        vec![Voice::new(VoiceType::Alto, Role::CantusFirmus, notes)],
    );
    let request = CounterpointRequest {
        beam: 100,
        ..CounterpointRequest::new(
            base,
            [VoiceType::Soprano, VoiceType::Tenor]
                .into_iter()
                .map(|voice_type| NewVoice {
                    voice_type,
                    species: Species::Free,
                })
                .collect(),
        )
    };
    let results = generate_counterpoint(&request, &Profile::fux()).unwrap();
    assert_eq!(results[0].report.forbidden(), 0);
    let score = &results[0].score;
    score.validate().unwrap();
    assert!(!score.is_free());
    assert_eq!(
        score
            .voices
            .iter()
            .filter(|v| v.role == Role::Counterpoint(Species::Free))
            .count(),
        2
    );
}

/// Every mode, plagal ones included, with two to four voices and twelve measures.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn every_mode_with_two_to_four_voices() {
    for mode in Mode::ALL {
        for ambitus in [fux_core::Ambitus::Authentic, fux_core::Ambitus::Plagal] {
            for transposed in [false, true] {
                for voices in 2..=4 {
                    let spec = ModeSpec::new(mode)
                        .with_ambitus(ambitus)
                        .with_transposition(transposed);
                    setting(spec, voices, 12, 200);
                }
            }
        }
    }
}

/// Five and six voices, with rests and the cadences of their modes.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn five_and_six_voices() {
    for (mode, voices) in [
        (Mode::Dorian, 5),
        (Mode::Phrygian, 5),
        (Mode::Ionian, 5),
        (Mode::Mixolydian, 6),
        (Mode::Aeolian, 6),
        (Mode::Lydian, 6),
    ] {
        setting(ModeSpec::new(mode), voices, 12, 200);
    }
}
