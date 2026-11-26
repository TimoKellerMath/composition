//! Calibration: the rules against Fux's own cantus firmi and examples.
//!
//! With the standard profile, Fux's examples must not break any prohibition, except the
//! deviations documented in `docs/regeln.md` § 13. The test also prints how often each rule
//! fires, which is the basis for the default levels (run with `--nocapture`).

mod common;

use std::collections::BTreeMap;

use common::{assert_prefixes_collect, assert_windows_partition};

use fux_core::corpus::{fux_cantus, fux_examples};
use fux_core::rules::{Checker, Profile};
use fux_core::{Duration, Lang, Note, Role, Score, Voice, VoiceType};

fn cantus_score(pitches: &[fux_core::Pitch], mode: fux_core::ModeSpec) -> Score {
    let notes: Vec<Note> = pitches
        .iter()
        .map(|&p| Note::new(p, Duration::WHOLE))
        .collect();
    let voice_type = VoiceType::best_for(pitches);
    Score::new(
        mode,
        vec![Voice::new(voice_type, Role::CantusFirmus, notes)],
    )
}

#[test]
fn fux_cantus_firmi_break_no_prohibition() {
    let checker = Checker::new(&Profile::fux());
    for cantus in fux_cantus() {
        let score = cantus_score(&cantus.notes, cantus.mode);
        let report = checker.check(&score);
        let forbidden: Vec<String> = report
            .violations
            .iter()
            .filter(|v| v.is_forbidden())
            .map(|v| format!("{}: {}", v.rule, checker.message(v, Lang::De)))
            .collect();
        assert!(forbidden.is_empty(), "{}: {forbidden:?}", cantus.id);
    }
}

#[test]
fn rule_frequencies_in_the_corpus() {
    let checker = Checker::new(&Profile::fux());
    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut forbidden = Vec::new();
    for example in fux_examples() {
        let score = example.score();
        for v in checker.check(&score).violations {
            let entry = counts.entry(v.rule).or_default();
            entry.0 += 1;
            if v.is_forbidden() {
                entry.1 += 1;
                let measure = v.tick / score.meter.ticks_per_measure() + 1;
                forbidden.push(format!(
                    "Fig. {} T{measure} {}: {}",
                    example.figure,
                    score.voices[v.voice].name,
                    checker.message(&v, Lang::De)
                ));
            }
        }
    }
    println!("rule: violations (forbidden)");
    for (rule, (all, hard)) in &counts {
        println!("{rule}: {all} ({hard})");
    }
    assert!(forbidden.is_empty(), "{}", forbidden.join("\n"));
}

/// An example and spoiled copies of it, in which notes of the counterpoints are moved: every
/// third note a whole step up, or every fourth a minor third down. Tied notes move together, so
/// the ties stay valid.
fn variants(score: &Score) -> Vec<(&'static str, Score)> {
    use fux_core::{Interval, Role};

    let spoil = |every: usize, at: usize, by: Interval| {
        let mut spoiled = score.clone();
        for voice in spoiled
            .voices
            .iter_mut()
            .filter(|v| v.role != Role::CantusFirmus)
        {
            let mut k = 0;
            for note in &mut voice.notes {
                if let Some(pitch) = note.pitch {
                    if k % every == at {
                        note.pitch = Some(pitch.transpose(by));
                    }
                    if !note.tie {
                        k += 1;
                    }
                }
            }
        }
        spoiled
    };
    vec![
        ("", score.clone()),
        (" (spoiled)", spoil(3, 1, Interval::MAJOR_SECOND)),
        (" (spoiled by thirds)", spoil(4, 2, -Interval::MINOR_THIRD)),
    ]
}

/// Fux's examples hardly break the rules, so the checks also run on spoiled copies of them.
#[test]
fn windows_partition_the_violations() {
    let checker = Checker::new(&Profile::fux());
    for example in fux_examples() {
        for (name, score) in variants(&example.score()) {
            assert_windows_partition(&checker, &score, &format!("Fig. {}{name}", example.figure));
        }
    }
}

#[test]
fn prefixes_collect_the_violations() {
    let checker = Checker::new(&Profile::fux());
    for example in fux_examples() {
        for (name, score) in variants(&example.score()) {
            assert_prefixes_collect(&checker, &score, &format!("Fig. {}{name}", example.figure));
        }
    }
}

/// The same for settings of a chant with internal cadences, which Fux's examples do not have.
#[test]
fn internal_cadences_are_decided_in_time() {
    use fux_core::gabc::{CadenceBars, parse_gabc};
    use fux_core::generate::{CounterpointRequest, NewVoice, generate_counterpoint};
    use fux_core::{Pitch, Species};

    let text = include_str!("../../../tests/fixtures/chant/ave-maris-stella.gabc");
    let chant = parse_gabc(text).unwrap();
    let mode = chant.mode().unwrap();
    let notes: Vec<Note> = chant
        .cantus_notes()
        .into_iter()
        .map(|n| {
            Note::new(
                n.pitch
                    .map(|p| Pitch::new(p.step, p.alter, p.octave + 1))
                    .unwrap(),
                Duration::WHOLE,
            )
        })
        .collect();
    let mut base = Score::new(
        mode,
        vec![Voice::new(VoiceType::Alto, Role::CantusFirmus, notes)],
    );
    base.cadences = chant.cadences(CadenceBars::Quarter, mode);
    assert_eq!(base.cadences.len(), 3);
    let checker = Checker::new(&Profile::fux());
    for species in [Species::First, Species::Third, Species::Fifth] {
        let request = CounterpointRequest::new(
            base.clone(),
            vec![
                NewVoice {
                    voice_type: VoiceType::Soprano,
                    species,
                },
                NewVoice {
                    voice_type: VoiceType::Bass,
                    species: Species::First,
                },
            ],
        );
        let results = generate_counterpoint(&request, &Profile::fux()).unwrap();
        for (name, score) in variants(&results[0].score) {
            let name = format!("Ave maris stella, {species:?}{name}");
            assert_windows_partition(&checker, &score, &name);
            assert_prefixes_collect(&checker, &score, &name);
        }
    }
}
