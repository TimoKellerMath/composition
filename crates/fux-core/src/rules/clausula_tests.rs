//! Minimal examples for internal cadences (CAD-INNER-*).

use super::{Checker, Level, Profile};
use crate::clef::VoiceType;
use crate::parse::{parse_lilypond, parse_pitches};
use crate::pitch::Step;
use crate::score::{Cadence, Note, Role, Score, Species, Voice};
use crate::time::Duration;

/// The cantus falls from B3 into A3, where an internal cadence arrives, and ends on D4.
const CF: &str = "D4 C4 B3 A3 C4 E4 D4";
/// The soprano brings the leading tone G♯4 into the octave A4.
const SOPRANO: &str = "d''1 a' gis' a' e'' cis'' d''";

fn setting(soprano: &str, cadences: &[Cadence]) -> Score {
    let cantus = parse_pitches(CF)
        .unwrap()
        .into_iter()
        .map(|p| Note::new(p, Duration::WHOLE))
        .collect();
    let mut score = Score::new(
        "dorian".parse().unwrap(),
        vec![
            Voice::new(
                VoiceType::Soprano,
                Role::Counterpoint(Species::First),
                parse_lilypond(soprano, Duration::WHOLE).unwrap(),
            ),
            Voice::new(VoiceType::Alto, Role::CantusFirmus, cantus),
        ],
    );
    score.cadences = cadences.to_vec();
    score.validate().unwrap();
    score
}

const ON_A: Cadence = Cadence {
    measure: 3,
    target: Step::A,
};

/// Rules and levels that fire against the counterpoint.
fn fired(score: &Score) -> Vec<(&'static str, Level)> {
    Checker::new(&Profile::fux())
        .check_targets(score, &[true, false])
        .violations
        .into_iter()
        .filter(|v| v.level == Level::Forbidden || v.rule.starts_with("CAD-INNER"))
        .map(|v| (v.rule, v.level))
        .collect()
}

#[test]
fn an_internal_cadence_with_its_leading_tone() {
    assert_eq!(fired(&setting(SOPRANO, &[ON_A])), []);
    // Without the cadence marked, the leading tone costs as one outside the final cadence.
    let unmarked =
        Checker::new(&Profile::fux()).check_targets(&setting(SOPRANO, &[]), &[true, false]);
    assert!(
        unmarked
            .violations
            .iter()
            .any(|v| v.rule == "FIC-INNER-CADENCE")
    );
}

#[test]
fn internal_cadences_need_the_clausula_and_the_octave() {
    // G4 instead of G♯4: the plain step leaves no room for the leading tone.
    let plain = setting("d''1 a' g' a' e'' cis'' d''", &[ON_A]);
    assert_eq!(fired(&plain), [("CAD-INNER-CLAUSULA", Level::Forbidden)]);
    // C5 above A3 is a tenth, not the octave of the clausula.
    let tenth = setting("d''1 a' gis' c'' e'' cis'' d''", &[ON_A]);
    assert!(fired(&tenth).contains(&("CAD-INNER-GOAL", Level::Forbidden)));
}

#[test]
fn cadences_are_validated() {
    let mut score = setting(SOPRANO, &[]);
    score.cadences = vec![Cadence {
        measure: 5,
        target: Step::E,
    }];
    assert!(matches!(
        score.validate(),
        Err(crate::score::ScoreError::CadenceOutside { measure: 5, .. })
    ));
    score.cadences = vec![Cadence {
        measure: 2,
        target: Step::B,
    }];
    assert!(matches!(
        score.validate(),
        Err(crate::score::ScoreError::CadenceTarget { .. })
    ));
}
