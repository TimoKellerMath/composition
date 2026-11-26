//! Reads all of Fux's cantus firmi and examples and checks them against the data model.

use std::collections::BTreeSet;

use fux_core::corpus::{Cantus, fux_cantus, fux_examples};
use fux_core::{Interval, Mode, Pitch, Role, Species};

fn cantus_by_id(id: &str) -> Cantus {
    fux_cantus()
        .into_iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("no cantus {id}"))
}

/// Equal up to transposition by whole octaves.
fn same_melody(a: &[Pitch], b: &[Pitch]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.same_class(*y) && (x.diatonic() - y.diatonic()) == (a[0].diatonic() - b[0].diatonic())
        })
        && (a[0].diatonic() - b[0].diatonic()) % 7 == 0
}

#[test]
fn cantus_firmi_follow_their_modes() {
    let cantus = fux_cantus();
    assert_eq!(cantus.len(), 8);
    for c in &cantus {
        let mode = c.mode;
        let first = c.notes[0];
        let last = *c.notes.last().unwrap();
        let penultimate = c.notes[c.notes.len() - 2];
        assert!(first.same_class(mode.final_pitch(4)), "{}: start", c.id);
        assert!(last.same_class(mode.final_pitch(4)), "{}: end", c.id);
        assert_eq!(mode.degree(penultimate), 2, "{}: penultimate", c.id);
        // The cantus falls to the final by a whole step, in the Phrygian by a semitone (F-E).
        let expected = if mode.mode == Mode::Phrygian {
            -Interval::MINOR_SECOND
        } else {
            -Interval::MAJOR_SECOND
        };
        assert_eq!(
            Interval::between(penultimate, last),
            expected,
            "{}: cadence 2-1",
            c.id
        );
        assert!(c.notes.iter().all(|&p| mode.is_scale_pitch(p)), "{}", c.id);
        assert!((9..=14).contains(&c.notes.len()), "{}: length", c.id);
    }
}

#[test]
fn all_examples_are_valid_scores() {
    let examples = fux_examples();
    let count = |voices| examples.iter().filter(|e| e.parts.len() == voices).count();
    assert_eq!((count(2), count(3), count(4)), (46, 45, 32));
    for example in &examples {
        let score = example.score();
        score
            .validate()
            .unwrap_or_else(|e| panic!("Fig. {}: {e}", example.figure));
        assert_eq!(score.cantus_firmus().unwrap().0, example.cf_part);
    }
}

#[test]
fn cantus_parts_match_cantus_file() {
    for example in fux_examples() {
        let score = example.score();
        let (_, cf) = score.cantus_firmus().unwrap();
        let expected = cantus_by_id(&example.cantus);
        assert!(
            same_melody(&cf.pitches(), &expected.notes),
            "Fig. {}: cantus differs from {}",
            example.figure,
            example.cantus
        );
        assert_eq!(expected.mode, example.mode, "Fig. {}", example.figure);
    }
}

#[test]
fn detected_species_match_the_source() {
    // Fux lets the whole-note voices of some four-part examples move in halves (Mann, p. 129).
    let split_whole_notes = ["195", "197", "202", "203"];
    for example in fux_examples() {
        let named: BTreeSet<Species> = example.species.iter().copied().collect();
        let detected: BTreeSet<Species> = example
            .score()
            .voices
            .iter()
            .filter_map(|v| match v.role {
                Role::Counterpoint(s) if s != Species::First => Some(s),
                _ => None,
            })
            .collect();
        let expected: BTreeSet<Species> = named
            .iter()
            .copied()
            .filter(|s| *s != Species::First)
            .collect();
        if split_whole_notes.contains(&example.figure.as_str()) {
            assert!(detected.is_superset(&expected), "Fig. {}", example.figure);
        } else {
            assert_eq!(detected, expected, "Fig. {}", example.figure);
        }
    }
}

#[test]
fn figure_5_intervals() {
    let example = fux_examples()
        .into_iter()
        .find(|e| e.figure == "5")
        .unwrap();
    let score = example.score();
    let intervals: Vec<String> = score
        .slices()
        .iter()
        .map(|s| {
            let upper = s.sounding[0].unwrap().pitch;
            let lower = s.sounding[1].unwrap().pitch;
            Interval::between(lower, upper).short_name()
        })
        .collect();
    assert_eq!(
        intervals,
        [
            "P5", "M3", "m3", "P5", "M3", "P5", "m3", "M3", "M6", "M6", "P8"
        ]
    );
}

#[test]
fn subsemitonium_in_two_part_cadences() {
    // In every two-part example the voice rising to the final reaches it by a semitone,
    // except in the Phrygian mode (Ch. 54; FIC-SUBSEMITONIUM).
    for example in fux_examples().into_iter().filter(|e| e.parts.len() == 2) {
        let score = example.score();
        let cp = 1 - example.cf_part;
        let pitches = score.voices[cp].pitches();
        let (penultimate, last) = (pitches[pitches.len() - 2], pitches[pitches.len() - 1]);
        let step = Interval::between(penultimate, last);
        match score.mode.subsemitonium(last.octave) {
            Some(_) => assert_eq!(step, Interval::MINOR_SECOND, "Fig. {}", example.figure),
            None => assert_eq!(step, Interval::MAJOR_SECOND, "Fig. {}", example.figure),
        }
    }
}
