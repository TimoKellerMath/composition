//! The cantus generator over many seeds and all modes.

use fux_core::generate::{CantusRequest, generate_cantus};
use fux_core::rules::Profile;
use fux_core::{Ambitus, Mode, ModeSpec};

/// Acceptance test of phase 3: 100 seeds × 6 modes give valid cantus firmi.
#[test]
fn hundred_seeds_in_every_mode() {
    let mut profile = Profile::fux();
    // Solvability runs a counterpoint search per cantus; it has its own test below.
    profile.apply_override("CF-SOLVABLE=aus").unwrap();
    for mode in Mode::ALL {
        let mut distinct = std::collections::HashSet::new();
        for seed in 0..100 {
            let request = CantusRequest {
                seed,
                beam: 40,
                ..CantusRequest::new(ModeSpec::new(mode), 11)
            };
            let results = generate_cantus(&request, &profile)
                .unwrap_or_else(|e| panic!("{mode:?}, seed {seed}: {e}"));
            assert_eq!(results[0].report.forbidden(), 0, "{mode:?}, seed {seed}");
            distinct.insert(results[0].score.voices[0].pitches());
        }
        assert!(
            distinct.len() >= 20,
            "{mode:?}: only {} distinct",
            distinct.len()
        );
    }
}

#[test]
fn other_lengths_ambitus_and_transposition() {
    let profile = Profile::fux();
    for (mode, length) in [
        ("hypodorisch", 8),
        ("phrygisch", 14),
        ("dorisch-transponiert", 12),
        ("hypolydisch-transponiert", 10),
        ("ionisch", 16),
    ] {
        let spec: ModeSpec = mode.parse().unwrap();
        let request = CantusRequest::new(spec, length);
        let results = generate_cantus(&request, &profile).unwrap();
        let pitches = results[0].score.voices[0].pitches();
        assert_eq!(pitches.len(), length, "{mode}");
        assert_eq!(results[0].report.forbidden(), 0, "{mode}");
        let (low, high) = spec.range(pitches[0].octave);
        assert!(
            pitches
                .iter()
                .all(|p| (low.diatonic()..=high.diatonic()).contains(&p.diatonic())),
            "{mode}: {pitches:?}"
        );
        if spec.ambitus == Ambitus::Plagal {
            assert!(
                pitches.iter().any(|p| p.diatonic() < pitches[0].diatonic()),
                "{mode}"
            );
        }
    }
}

#[test]
fn generated_cantus_firmi_are_solvable() {
    let profile = Profile::fux();
    for mode in Mode::ALL {
        let request = CantusRequest {
            seed: 3,
            ..CantusRequest::new(ModeSpec::new(mode), 11)
        };
        let results = generate_cantus(&request, &profile).unwrap();
        assert_eq!(results[0].report.forbidden(), 0, "{mode:?}");
    }
}

/// Acceptance test of phase 4: first species above and below every cantus of Fux.
#[test]
fn first_species_for_every_fux_cantus() {
    species_for_every_fux_cantus(fux_core::Species::First, &Profile::fux());
}

/// Acceptance test of phase 5: second species above and below every cantus of Fux.
#[test]
fn second_species_for_every_fux_cantus() {
    species_for_every_fux_cantus(fux_core::Species::Second, &Profile::fux());
}

/// Acceptance test of phase 5: third species above and below every cantus of Fux.
#[test]
fn third_species_for_every_fux_cantus() {
    species_for_every_fux_cantus(fux_core::Species::Third, &Profile::fux());
}

/// Acceptance test of phase 5: fourth species above and below every cantus of Fux.
#[test]
fn fourth_species_for_every_fux_cantus() {
    species_for_every_fux_cantus(fux_core::Species::Fourth, &Profile::fux());
}

/// Acceptance test of phase 6: fifth species above and below every cantus of Fux.
#[test]
fn fifth_species_for_every_fux_cantus() {
    species_for_every_fux_cantus(fux_core::Species::Fifth, &Profile::fux());
}

/// Phase 9: Jeppesen's stricter profile in every species, above and below every cantus of Fux.
#[test]
fn every_species_after_jeppesen() {
    use fux_core::Species::*;
    for species in [First, Second, Third, Fourth, Fifth] {
        species_for_every_fux_cantus(species, &Profile::jeppesen());
    }
}

fn species_for_every_fux_cantus(species: fux_core::Species, profile: &Profile) {
    use fux_core::corpus::fux_cantus;
    use fux_core::generate::{CounterpointRequest, NewVoice, generate_counterpoint};
    use fux_core::{Duration, Note, Role, Score, Voice, VoiceType};

    for cantus in fux_cantus() {
        let voice_type = if VoiceType::Alto.misfit(&cantus.notes) == 0 {
            VoiceType::Alto
        } else {
            VoiceType::best_for(&cantus.notes)
        };
        let notes = cantus
            .notes
            .iter()
            .map(|&p| Note::new(p, Duration::WHOLE))
            .collect();
        let base = Score::new(
            cantus.mode,
            vec![Voice::new(voice_type, Role::CantusFirmus, notes)],
        );
        for above in [true, false] {
            let Some(cp) = voice_type.neighbor(above) else {
                continue;
            };
            let request = CounterpointRequest::new(
                base.clone(),
                vec![NewVoice {
                    voice_type: cp,
                    species,
                }],
            );
            let place = if above { "above" } else { "below" };
            let results = generate_counterpoint(&request, profile).unwrap_or_else(|e| {
                panic!("{species:?} {} {place} ({}): {e}", cantus.id, profile.name)
            });
            assert_eq!(results[0].report.forbidden(), 0, "{}", cantus.id);
            let score = &results[0].score;
            score.validate().unwrap();
            let voice = score.voices.iter().find(|v| !v.is_cantus_firmus()).unwrap();
            assert_eq!(voice.role, Role::Counterpoint(species));
        }
    }
}

/// A cantus of Fux in the given voice type, its octave chosen to fit.
fn cantus_in(id: &str, voice_type: fux_core::VoiceType) -> fux_core::Score {
    use fux_core::corpus::fux_cantus;
    use fux_core::{Duration, Note, Pitch, Role, Score, Voice};

    let cantus = fux_cantus().into_iter().find(|c| c.id == id).unwrap();
    let shift = voice_type.octave_shift_for(&cantus.notes) as i8;
    let notes = cantus
        .notes
        .iter()
        .map(|p| {
            Note::new(
                Pitch::new(p.step, p.alter, p.octave + shift),
                Duration::WHOLE,
            )
        })
        .collect();
    Score::new(
        cantus.mode,
        vec![Voice::new(voice_type, Role::CantusFirmus, notes)],
    )
}

/// A given voice keeps its text and lyrics through the search; the new voice does not sing.
#[test]
fn given_voices_keep_their_text() {
    use std::sync::Arc;

    use fux_core::generate::{CounterpointRequest, NewVoice, generate_counterpoint};
    use fux_core::text::{TextOptions, resolve};
    use fux_core::{Lyric, ScoreText, Species, VoiceType};

    let mut base = cantus_in("dorian", VoiceType::Alto);
    for (position, note) in base.voices[0].notes.iter_mut().take(7).enumerate() {
        note.lyric = Some(Lyric::new(position));
    }
    let (kyrie, _) = resolve("kyrie", &TextOptions::default()).unwrap();
    base.text = Some(ScoreText {
        text: Arc::new(kyrie),
        voices: vec![true],
    });
    base.fill_syllables(true);
    let request = CounterpointRequest::new(
        base.clone(),
        vec![NewVoice {
            voice_type: VoiceType::Soprano,
            species: Species::First,
        }],
    );
    let results = generate_counterpoint(&request, &Profile::fux()).unwrap();
    let score = &results[0].score;
    score.validate().unwrap();
    let sung = score.text.as_ref().unwrap();
    assert_eq!(sung.text, base.text.as_ref().unwrap().text);
    let cantus: Vec<bool> = score.voices.iter().map(|v| v.is_cantus_firmus()).collect();
    assert_eq!(sung.voices, cantus);
    let (c, voice) = score.cantus_firmus().unwrap();
    let lyrics: Vec<Option<Lyric>> = voice.notes.iter().map(|n| n.lyric).collect();
    let given: Vec<Option<Lyric>> = base.voices[0].notes.iter().map(|n| n.lyric).collect();
    assert_eq!(lyrics, given);
    assert_eq!(
        score.voice_lyrics(c).as_deref(),
        Some("Ký-ri-e e-lé-i-son.")
    );
    assert_eq!(score.voice_lyrics(1 - c), None);
}

/// Fux's scheme with `voices` voices: the cantus in the voice at `position` of the usual choir,
/// the highest other voice in `species`, the rest in whole notes.
fn fux_scheme(cantus: &str, voices: usize, species: fux_core::Species, position: usize) {
    use fux_core::generate::{CounterpointRequest, NewVoice, default_choir, generate_counterpoint};
    use fux_core::{Role, Species, VoiceType};

    let choir = default_choir(voices, VoiceType::Alto);
    let cantus_type = choir[position];
    let mut others = choir.clone();
    others.remove(position);
    let new: Vec<NewVoice> = others
        .iter()
        .enumerate()
        .map(|(i, &voice_type)| NewVoice {
            voice_type,
            species: if i == 0 { species } else { Species::First },
        })
        .collect();
    let request = CounterpointRequest::new(cantus_in(cantus, cantus_type), new);
    let label = format!("{cantus}, {voices} voices, {species:?}, cantus in {cantus_type:?}");
    let results =
        generate_counterpoint(&request, &Profile::fux()).unwrap_or_else(|e| panic!("{label}: {e}"));
    assert_eq!(results[0].report.forbidden(), 0, "{label}");
    let score = &results[0].score;
    score.validate().unwrap();
    assert_eq!(score.voices.len(), voices, "{label}");
    assert!(
        score
            .voices
            .iter()
            .any(|v| v.role == Role::Counterpoint(species)),
        "{label}"
    );
}

/// Acceptance tests of phase 7: three to six voices in Fux's scheme, the cantus in different
/// voices. The full matrix runs with `cargo test --release -- --ignored`.
#[test]
fn three_voices() {
    use fux_core::Species::*;
    fux_scheme("dorian", 3, First, 0);
    fux_scheme("phrygian", 3, Third, 1);
    fux_scheme("ionian", 3, Fifth, 2);
}

#[test]
fn four_voices() {
    use fux_core::Species::*;
    fux_scheme("mixolydian", 4, Second, 3);
    fux_scheme("aeolian", 4, Fourth, 0);
}

#[test]
fn five_voices() {
    use fux_core::Species::*;
    fux_scheme("lydian", 5, First, 4);
    fux_scheme("dorian", 5, Fourth, 2);
}

#[test]
fn six_voices() {
    use fux_core::Species::*;
    fux_scheme("dorian", 6, First, 0);
    fux_scheme("aeolian", 6, Second, 5);
}

#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn every_species_with_the_cantus_in_every_voice() {
    for voices in 3..=6 {
        for species in fux_core::Species::ALL {
            for position in 0..voices {
                fux_scheme("dorian", voices, species, position);
            }
        }
    }
}

/// A chant from `tests/fixtures/chant` as the cantus in `voice_type`, with internal cadences at
/// its half and full bars.
fn chant(file: &str, voice_type: fux_core::VoiceType) -> fux_core::Score {
    use fux_core::gabc::{CadenceBars, parse_gabc};
    use fux_core::{Note, Pitch, Role, Score, Voice};

    let path = format!(
        "{}/../../tests/fixtures/chant/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    let chant = parse_gabc(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mode = chant.mode().unwrap();
    let notes = chant.cantus_notes();
    let pitches: Vec<Pitch> = notes.iter().filter_map(|n| n.pitch).collect();
    let shift = voice_type.octave_shift_for(&pitches) as i8;
    let notes = notes
        .into_iter()
        .map(|n| Note {
            pitch: n
                .pitch
                .map(|p| Pitch::new(p.step, p.alter, p.octave + shift)),
            ..n
        })
        .collect();
    let mut score = Score::new(
        mode,
        vec![Voice::new(voice_type, Role::CantusFirmus, notes)],
    );
    score.cadences = chant.cadences(CadenceBars::Half, mode);
    score
}

/// A chant in the tenor of the usual choir of `voices`, the highest other voice in `species`.
fn chant_setting(file: &str, voices: usize, species: fux_core::Species) -> fux_core::Score {
    use fux_core::generate::{CounterpointRequest, NewVoice, default_choir, generate_counterpoint};
    use fux_core::{Species, VoiceType};

    let cantus = chant(file, VoiceType::Tenor);
    let mut others = default_choir(voices, VoiceType::Tenor);
    let position = others.iter().position(|&t| t == VoiceType::Tenor).unwrap();
    others.remove(position);
    let new: Vec<NewVoice> = others
        .iter()
        .enumerate()
        .map(|(i, &voice_type)| NewVoice {
            voice_type,
            species: if i == 0 { species } else { Species::First },
        })
        .collect();
    let label = format!("{file}, {voices} voices, {species:?}");
    let request = CounterpointRequest::new(cantus, new);
    let results =
        generate_counterpoint(&request, &Profile::fux()).unwrap_or_else(|e| panic!("{label}: {e}"));
    assert_eq!(results[0].report.forbidden(), 0, "{label}");
    results[0].score.validate().unwrap();
    results.into_iter().next().unwrap().score
}

/// Acceptance test of phase 7b: a hymn of 45 notes with two internal cadences in four voices.
#[test]
fn a_chant_in_four_voices() {
    let score = chant_setting("veni-creator.gabc", 4, fux_core::Species::First);
    assert_eq!(score.measure_count(), 45);
    assert_eq!(score.cadences.len(), 2);
    // The cantus rises F–G into the final: it gets the leading tone.
    let (_, cantus) = score.cantus_firmus().unwrap();
    let pitches: Vec<String> = cantus.pitches().iter().map(ToString::to_string).collect();
    assert_eq!(pitches[pitches.len() - 2..], ["F#3", "G3"]);
}

/// Three hymns in four to six voices with the voice in every species. Runs with
/// `cargo test --release -- --ignored`.
#[test]
#[ignore]
fn chants_in_four_to_six_voices() {
    for file in [
        "ave-maris-stella.gabc",
        "veni-creator.gabc",
        "pange-lingua.gabc",
    ] {
        for voices in 4..=6 {
            for species in fux_core::Species::ALL {
                chant_setting(file, voices, species);
            }
        }
    }
}
