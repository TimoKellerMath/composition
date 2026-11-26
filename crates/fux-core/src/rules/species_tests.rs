//! Minimal examples for the rules of the second to fourth species: one that triggers the rule,
//! one that does not.

use super::{Checker, Level, Profile, Violation};
use crate::clef::VoiceType;
use crate::parse::{parse_lilypond, parse_pitches};
use crate::score::{Note, Role, Score, Species, Voice};
use crate::time::Duration;

fn cantus(notes: &str) -> Voice {
    let notes = parse_pitches(notes)
        .unwrap()
        .into_iter()
        .map(|p| Note::new(p, Duration::WHOLE))
        .collect();
    Voice::new(VoiceType::Alto, Role::CantusFirmus, notes)
}

/// A counterpoint (LilyPond notation) in the given voice above or below a cantus.
fn score(species: Species, cf: &str, voice_type: VoiceType, cp: &str) -> Score {
    let cp = Voice::new(
        voice_type,
        Role::Counterpoint(species),
        parse_lilypond(cp, Duration::WHOLE).unwrap(),
    );
    let voices = if voice_type < VoiceType::Alto {
        vec![cp, cantus(cf)]
    } else {
        vec![cantus(cf), cp]
    };
    let score = Score::new("dorian".parse().unwrap(), voices);
    score.validate().unwrap();
    score
}

fn above(species: Species, cf: &str, cp: &str) -> Score {
    score(species, cf, VoiceType::Soprano, cp)
}

fn below(species: Species, cf: &str, cp: &str) -> Score {
    score(species, cf, VoiceType::Tenor, cp)
}

fn violations_with(profile: &Profile, score: &Score) -> Vec<Violation> {
    Checker::new(profile).check(score).violations
}

/// Levels with which the rule fired.
fn fired(score: &Score, rule: &str) -> Vec<Level> {
    violations_with(&Profile::fux(), score)
        .into_iter()
        .filter(|v| v.rule == rule)
        .map(|v| v.level)
        .collect()
}

/// The prohibitions the counterpoint breaks (the short cantus does not count).
fn forbidden(score: &Score) -> Vec<&'static str> {
    let targets: Vec<bool> = score.voices.iter().map(|v| !v.is_cantus_firmus()).collect();
    Checker::new(&Profile::fux())
        .check_targets(score, &targets)
        .violations
        .into_iter()
        .filter(|v| v.is_forbidden())
        .map(|v| v.rule)
        .collect()
}

const CF: &str = "D4 F4 E4 D4";

#[test]
fn good_examples_break_no_prohibition() {
    let second = above(Species::Second, CF, "r2 a'2 | d''2 a'2 | b'2 cis''2 | d''1");
    assert!(forbidden(&second).is_empty(), "{:?}", forbidden(&second));
    let third = above(
        Species::Third,
        CF,
        "d''4 c'' b' a' | d'' a' c'' a' | g' a' b' cis'' | d''1",
    );
    assert!(forbidden(&third).is_empty(), "{:?}", forbidden(&third));
    let fourth = above(
        Species::Fourth,
        CF,
        "r2 a'2~ | a'2 d''2~ | d''2 cis''2 | d''1",
    );
    assert!(forbidden(&fourth).is_empty(), "{:?}", forbidden(&fourth));
}

#[test]
fn rhythms() {
    // Two half notes without the rest at the start cost a little.
    let score = above(
        Species::Second,
        CF,
        "a'2 f'2 | d''2 a'2 | b'2 cis''2 | d''1",
    );
    assert_eq!(fired(&score, "G2-RHYTHM"), [Level::P2]);
    let score = above(Species::Second, CF, "r2 a'2 | d''1 | b'2 cis''2 | d''1");
    assert_eq!(fired(&score, "G2-RHYTHM"), [Level::Forbidden]);
    let score = above(
        Species::Third,
        CF,
        "r4 a'4 g' f' | f' a' c'' d'' | c'' a' b' cis'' | d''1",
    );
    assert_eq!(fired(&score, "G3-RHYTHM"), [Level::Forbidden]);
    let score = above(
        Species::Fourth,
        CF,
        "a'2 a'2~ | a'2 d''2~ | d''2 cis''2 | d''1",
    );
    assert_eq!(fired(&score, "G4-RHYTHM"), [Level::Forbidden]);
}

#[test]
fn second_species_dissonance_only_as_passing_tone() {
    // G4 against F4 is a second, reached by a leap.
    let score = above(Species::Second, CF, "r2 a'2 | d''2 g'2 | b'2 cis''2 | d''1");
    assert_eq!(fired(&score, "G2-PASSING"), [Level::Forbidden]);
    // A4–G4–F4 passes through the second G4 against F4.
    let score = above(Species::Second, "D4 F4 D4", "r2 f'2 | a'2 g'2 | f'1");
    assert!(fired(&score, "G2-PASSING").is_empty());
}

#[test]
fn second_species_skip_of_a_third_does_not_hide_fifths() {
    // A4 (fifth) – F4 – B4 (fifth): the third does not separate the fifths.
    let score = above(Species::Second, "D4 E4", "a'2 f'2 | b'1");
    assert_eq!(fired(&score, "G2-DOWNBEAT-MOTION"), [Level::Forbidden]);
    // A skip of a fourth does.
    let score = above(Species::Second, "D4 E4", "a'2 e'2 | b'1");
    assert!(fired(&score, "G2-DOWNBEAT-MOTION").is_empty());
}

#[test]
fn third_species_dissonances() {
    // C5 against D4 is a seventh, reached by a leap.
    let score = above(Species::Third, "D4 D4", "a'4 c'' b' a' | a'1");
    assert_eq!(fired(&score, "G3-PASSING"), [Level::Forbidden]);
    // Passing tones G4 and E4.
    let score = above(Species::Third, "D4 D4", "a'4 g' f' e' | d'1");
    assert!(fired(&score, "G3-PASSING").is_empty());
    // The neighbour G4 between two F4.
    let score = above(Species::Third, "D4 D4", "f'4 g' f' e' | d'1");
    assert_eq!(fired(&score, "G3-PASSING"), [Level::Forbidden]);
    // D5–C5–A4–F4 begins like a cambiata but goes on down: decided by its fourth note, where
    // the search knows it.
    let score = above(Species::Third, "D4 D4", "d''4 c'' a' f' | d'1");
    let ticks: Vec<u32> = violations_with(&Profile::fux(), &score)
        .into_iter()
        .filter(|v| v.rule == "G3-PASSING")
        .map(|v| v.tick)
        .collect();
    assert_eq!(ticks, [6]);
    // Nota cambiata D5–C5–A4–B4: free, but counted when the profile asks for it.
    let score = above(Species::Third, "D4 D4", "d''4 c'' a' b' | a'1");
    assert!(fired(&score, "G3-PASSING").is_empty());
    let mut profile = Profile::fux();
    profile.apply_override("G3-CAMBIATA=P1").unwrap();
    let cambiata: Vec<Level> = violations_with(&profile, &score)
        .into_iter()
        .filter(|v| v.rule == "G3-CAMBIATA")
        .map(|v| v.level)
        .collect();
    assert_eq!(cambiata, [Level::P1]);
}

/// Levels with which the rule fired under Jeppesen's profile.
fn fired_jeppesen(score: &Score, rule: &str) -> Vec<Level> {
    violations_with(&Profile::jeppesen(), score)
        .into_iter()
        .filter(|v| v.rule == rule)
        .map(|v| v.level)
        .collect()
}

#[test]
fn neighbour_notes_after_fux_and_jeppesen() {
    // The lower neighbour E4 between two F4 on the second quarter: Fux forbids it in two
    // voices, Jeppesen allows it.
    let lower = above(Species::Third, "D4 D4", "f'4 e' f' g' | a'1");
    assert_eq!(fired(&lower, "G3-PASSING"), [Level::Forbidden]);
    assert!(fired_jeppesen(&lower, "G3-PASSING").is_empty());
    // The upper neighbour G4 between two F4: forbidden in both.
    let upper = above(Species::Third, "D4 D4", "f'4 g' f' e' | d'1");
    assert_eq!(fired_jeppesen(&upper, "G3-PASSING"), [Level::Forbidden]);
    // On the third quarter a neighbour note is an accented dissonance.
    let accented = above(Species::Third, "D4 D4", "a'4 f' e' f' | d'1");
    assert_eq!(fired_jeppesen(&accented, "G3-PASSING"), [Level::Forbidden]);
}

#[test]
fn accented_quarters_after_jeppesen() {
    // C5 against D4 on the third quarter, passing upwards: free for Fux, not for Jeppesen.
    let rising = above(Species::Third, "D4 F4", "a'4 b' c'' d'' | c''1");
    assert!(fired(&rising, "G3-ACCENTED-PASSING").is_empty());
    assert_eq!(
        fired_jeppesen(&rising, "G3-ACCENTED-PASSING"),
        [Level::Forbidden]
    );
    let falling = above(Species::Third, "D4 F4", "f''4 d'' c'' b' | c''1");
    assert!(fired_jeppesen(&falling, "G3-ACCENTED-PASSING").is_empty());
    // A leap up from the accented third quarter (A4–D5), not from the second (F4–A4).
    let leap = above(Species::Third, "D4 D4", "a'4 b' a' d'' | d''1");
    assert!(fired(&leap, "G3-ACCENT-LEAP").is_empty());
    assert_eq!(fired_jeppesen(&leap, "G3-ACCENT-LEAP"), [Level::Forbidden]);
    let unaccented = above(Species::Third, "D4 D4", "a'4 f' a' b' | a'1");
    assert!(fired_jeppesen(&unaccented, "G3-ACCENT-LEAP").is_empty());
    // In the fifth species, too: A4 on the downbeat of measure 6 leaps up to C5.
    let good = above(Species::Fifth, CF5, &GOOD5.join(" | "));
    assert_eq!(fired_jeppesen(&good, "G3-ACCENT-LEAP"), [Level::Forbidden]);
    assert!(fired_jeppesen(&fifth(6, "a'4 b'4 d''2~"), "G3-ACCENT-LEAP").is_empty());
}

#[test]
fn third_species_third_quarter_and_runs() {
    // Over B3 the fourth E4 and the diminished fifth F4 follow each other.
    let score = above(Species::Third, "B3 B3", "d'4 e' f' g' | a'1");
    assert_eq!(fired(&score, "G3-THIRD-QUARTER"), [Level::Forbidden]);
    assert_eq!(fired(&score, "G3-RUN"), [Level::Forbidden]);
    let score = above(Species::Third, "D4 D4", "f'4 a' g' f' | d'1");
    assert!(fired(&score, "G3-THIRD-QUARTER").is_empty());
    let score = above(Species::Third, "D4 D4", "a'4 g' f' e' | d'1");
    assert!(fired(&score, "G3-RUN").is_empty());
}

#[test]
fn fifths_on_consecutive_downbeats() {
    let score = above(Species::Third, "D4 E4", "a'4 f' g' a' | b'1");
    assert_eq!(fired(&score, "G3-ACCENT-PARALLELS"), [Level::P2]);
    let score = above(Species::Third, "D4 E4", "a'4 f' e' f' | g'1");
    assert!(fired(&score, "G3-ACCENT-PARALLELS").is_empty());
}

#[test]
fn fourth_species_suspensions() {
    // The fourth G4 over D4 on the upbeat.
    let score = above(Species::Fourth, "D4 E4", "r2 g'2~ | g'1");
    assert_eq!(fired(&score, "G4-ARSIS"), [Level::Forbidden]);
    // A4 over E4 is a fourth: down to G4 (4–3) is right, up to B4 is not.
    let score = above(Species::Fourth, "D4 E4 D4", "r2 a'2~ | a'2 g'2 | f'1");
    assert!(fired(&score, "G4-RESOLUTION").is_empty());
    let score = above(Species::Fourth, "D4 E4 D4", "r2 a'2~ | a'2 b'2 | a'1");
    assert_eq!(fired(&score, "G4-RESOLUTION"), [Level::Forbidden]);
    // Below the cantus, the seventh F3 under E4 would resolve into the octave.
    let score = below(Species::Fourth, "D4 E4 D4", "r2 f2~ | f2 e2 | d1");
    assert_eq!(fired(&score, "G4-SUSPENSIONS"), [Level::Forbidden]);
    // Octave D5 over D4, then the ninth D5 over C4: hidden octaves.
    let score = above(Species::Fourth, "D4 C4 B3", "r2 d''2~ | d''2 c''2 | b'1");
    assert_eq!(fired(&score, "G4-OCTAVES"), [Level::Forbidden]);
}

#[test]
fn fourth_species_broken_syncopation() {
    let score = above(
        Species::Fourth,
        CF,
        "r2 a'2 | d''2 a'2~ | a'2 cis''2 | d''1",
    );
    assert_eq!(fired(&score, "G4-BREAK"), [Level::P4]);
}

#[test]
fn cadences_of_the_species() {
    // Second species: a third (G4 over E4) on the penultimate downbeat instead of the fifth.
    let score = above(Species::Second, CF, "r2 a'2 | d''2 a'2 | g'2 cis''2 | d''1");
    assert_eq!(fired(&score, "CAD-PENULT-G2"), [Level::Forbidden]);
    let score = above(Species::Second, CF, "r2 a'2 | d''2 a'2 | b'2 cis''2 | d''1");
    assert!(fired(&score, "CAD-PENULT-G2").is_empty());
    // Third species below the cantus: the first quarter should be the minor third C#4.
    let score = below(Species::Third, CF, "d4 e f g | a g f e | a b c' cis' | d'1");
    assert_eq!(fired(&score, "CAD-PENULT-G3"), [Level::P2]);
    // Fourth species: the consonant B4 over E4 instead of the seventh D5.
    let score = above(
        Species::Fourth,
        CF,
        "r2 a'2~ | a'2 b'2~ | b'2 cis''2 | d''1",
    );
    assert_eq!(fired(&score, "CAD-PENULT-G4"), [Level::Forbidden]);
}

/// A cantus of eight notes for the fifth species, and a counterpoint above it in which every
/// measure but the given one is Fux's.
const CF5: &str = "D4 F4 E4 D4 G4 F4 E4 D4";
const GOOD5: [&str; 8] = [
    "r2 a'2~",
    "a'4 c''4 d''2~",
    "d''4 c''8 b'8 c''2~",
    "c''4 b'4 a'4 b'4",
    "e''4 d''4 c''4 b'4",
    "a'4 c''4 d''2~",
    "d''2 cis''2",
    "d''1",
];

fn fifth(measure: usize, replacement: &str) -> Score {
    let mut measures = GOOD5.to_vec();
    measures[measure - 1] = replacement;
    above(Species::Fifth, CF5, &measures.join(" | "))
}

#[test]
fn fifth_species_good_example() {
    let score = above(Species::Fifth, CF5, &GOOD5.join(" | "));
    assert!(forbidden(&score).is_empty(), "{:?}", forbidden(&score));
    // Measures 3 (⁀4 8 8 2⁀) and 5 (4 4 4 4) are rhythms Fux uses 3 to 9 times.
    assert_eq!(fired(&score, "G5-RHYTHM"), [Level::P1, Level::P1]);
    let messages: Vec<String> = Checker::new(&Profile::fux())
        .check(&score)
        .violations
        .iter()
        .filter(|v| v.rule == "G5-RHYTHM")
        .map(|v| Checker::new(&Profile::fux()).message(v, crate::lang::Lang::De))
        .collect();
    assert_eq!(
        messages[0],
        "5. Gattung, Taktrhythmus ⁀4 8 8 2⁀: bei Fux 3- bis 9-mal belegt"
    );
}

#[test]
fn fifth_species_rhythms() {
    // A dotted half and a quarter: Fux uses it only with three or more voices.
    assert_eq!(
        fired(&fifth(5, "e''2. b'4"), "G5-RHYTHM"),
        [Level::P1, Level::Forbidden]
    );
    // Without the tie at the start.
    let score = fifth(1, "r2 a'2");
    assert!(fired(&score, "G5-RHYTHM").contains(&Level::P1));
    // The penultimate measure must hold the syncopation.
    let score = fifth(7, "d''4 c''4 cis''2");
    assert!(fired(&score, "G5-RHYTHM").contains(&Level::Forbidden));
}

#[test]
fn fifth_species_dissonances() {
    // C5 against G4 is a fourth, reached by a skip.
    let score = fifth(5, "e''4 c''4 d''4 b'4");
    assert_eq!(fired(&score, "G5-PASSING"), [Level::Forbidden]);
    // A syncopation that begins with the tritone B4 against F4.
    let score = above(
        Species::Fifth,
        CF5,
        "r2 a'2~ | a'4 c''4 d''2~ | d''4 c''8 b'8 c''2~ | c''4 b'4 a'4 b'4 \
         | e''4 d''4 c''4 b'4 | a'4 c''4 b'2~ | b'2 cis''2 | d''1",
    );
    assert!(fired(&score, "G5-PASSING").contains(&Level::Forbidden));
    // The lower neighbour C5 between two D5 off the beat is fine, the upper F5 on the beat not.
    assert!(fired(&fifth(5, "e''4 d''8 c''8 d''4 b'4"), "G5-EIGHTHS").is_empty());
    assert_eq!(
        fired(&fifth(5, "e''4 f''8 e''8 d''4 b'4"), "G5-EIGHTHS"),
        [Level::Forbidden]
    );
    // An eighth reached by a skip of a fourth.
    let score = fifth(3, "d''4 a'8 b'8 c''2~");
    assert_eq!(fired(&score, "G5-EIGHTHS"), [Level::Forbidden]);
}

#[test]
fn fifth_species_resolutions() {
    // D5 against E4 (a seventh) resolves at once, over two eighths or after a skip down.
    for resolved in ["d''4 c''8 b'8 c''2~", "d''4 c''4 c''2~", "d''4 g'4 c''2~"] {
        assert!(
            fired(&fifth(3, resolved), "G5-ORNAMENTED-RES").is_empty(),
            "{resolved}"
        );
    }
    // An ornament upwards, and no resolution at all.
    for unresolved in ["d''4 e''4 c''2~", "d''4 a'4 b'2"] {
        assert_eq!(
            fired(&fifth(3, unresolved), "G5-ORNAMENTED-RES"),
            [Level::Forbidden],
            "{unresolved}"
        );
    }
}

#[test]
fn fifth_species_variety_and_cadence() {
    // Three measures without a tie, two of them repeating the rhythm before.
    let score = above(
        Species::Fifth,
        CF5,
        "r2 a'2~ | a'4 c''4 d''2 | e''4 d''4 c''4 b'4 | a'4 b'4 c''4 d''4 | e''4 d''4 c''4 b'4 \
         | a'4 c''4 d''2~ | d''2 cis''2 | d''1",
    );
    assert_eq!(
        fired(&score, "G5-VARIETY"),
        [Level::P2, Level::P2, Level::P2]
    );
    // The fifth B4 over E4 instead of the seventh D5 before the cadence.
    let score = above(
        Species::Fifth,
        CF5,
        "r2 a'2~ | a'4 c''4 d''2~ | d''4 c''8 b'8 c''2~ | c''4 b'4 a'4 b'4 \
         | e''4 d''4 c''4 b'4 | a'4 b'4 c''4 b'4~ | b'2 cis''2 | d''1",
    );
    assert_eq!(fired(&score, "CAD-PENULT-G5"), [Level::Forbidden]);
}
