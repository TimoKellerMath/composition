//! Minimal examples for every rule: one that triggers it, one that does not.

use super::{Checker, Level, Profile, Violation, Window};
use crate::clef::VoiceType;
use crate::parse::{parse_lilypond, parse_pitches};
use crate::score::{Note, Role, Score, Species, Voice};
use crate::time::Duration;

fn cantus_voice(notes: &str) -> Voice {
    let notes = parse_pitches(notes)
        .unwrap()
        .into_iter()
        .map(|p| Note::new(p, Duration::WHOLE))
        .collect();
    Voice::new(VoiceType::Alto, Role::CantusFirmus, notes)
}

/// A cantus alone, in the given mode.
fn cantus(mode: &str, notes: &str) -> Score {
    Score::new(mode.parse().unwrap(), vec![cantus_voice(notes)])
}

/// A counterpoint (soprano, LilyPond notation) above a cantus (scientific names).
fn above(species: Species, cf: &str, cp: &str) -> Score {
    let cp = parse_lilypond(cp, Duration::WHOLE).unwrap();
    let score = Score::new(
        "dorian".parse().unwrap(),
        vec![
            Voice::new(VoiceType::Soprano, Role::Counterpoint(species), cp),
            cantus_voice(cf),
        ],
    );
    score.validate().unwrap();
    score
}

fn violations(score: &Score) -> Vec<Violation> {
    Checker::new(&Profile::fux()).check(score).violations
}

/// Levels with which the rule fired.
fn fired(score: &Score, rule: &str) -> Vec<Level> {
    violations(score)
        .into_iter()
        .filter(|v| v.rule == rule)
        .map(|v| v.level)
        .collect()
}

const DORIAN: &str = "D4 F4 E4 D4 G4 F4 A4 G4 F4 E4 D4";

#[test]
fn fux_dorian_cantus_is_clean() {
    let forbidden: Vec<_> = violations(&cantus("dorian", DORIAN))
        .into_iter()
        .filter(|v| v.is_forbidden())
        .collect();
    assert!(forbidden.is_empty(), "{forbidden:?}");
}

#[test]
fn cantus_start_end_penultimate() {
    let score = cantus("dorian", "E4 F4 G4 A4 G4 F4 E4 C4");
    assert_eq!(fired(&score, "CF-START"), [Level::Forbidden]);
    assert_eq!(fired(&score, "CF-END"), [Level::Forbidden]);
    assert_eq!(fired(&score, "CF-PENULT"), [Level::Forbidden]);
    let score = cantus("dorian", "D4 F4 E4 G4 A4 F4 G4 D4");
    assert_eq!(fired(&score, "CF-PENULT"), [Level::Forbidden]);
    assert!(fired(&cantus("dorian", DORIAN), "CF-PENULT").is_empty());
}

#[test]
fn cantus_length() {
    assert_eq!(
        fired(&cantus("dorian", "D4 F4 G4 E4 D4"), "CF-LENGTH"),
        [Level::Forbidden]
    );
    assert_eq!(
        fired(&cantus("dorian", "D4 F4 G4 A4 F4 E4 D4"), "CF-LENGTH"),
        [Level::P1]
    );
    assert!(fired(&cantus("dorian", DORIAN), "CF-LENGTH").is_empty());
}

#[test]
fn cantus_scale_range_repeat() {
    let score = cantus("dorian", "D4 F#4 G4 A4 G4 F4 E4 D4");
    assert_eq!(fired(&score, "CF-DIATONIC"), [Level::Forbidden]);
    let score = cantus("dorian", "D4 F4 A4 C5 F5 D5 B4 A4 F4 E4 D4");
    assert_eq!(fired(&score, "CF-RANGE"), [Level::P3]);
    let score = cantus("dorian", "D4 F4 A4 C5 G5 D5 B4 A4 F4 E4 D4");
    assert_eq!(fired(&score, "CF-RANGE"), [Level::Forbidden, Level::P3]);
    let score = cantus("dorian", "D4 F4 F4 G4 A4 G4 F4 E4 D4");
    assert_eq!(fired(&score, "CF-REPEAT"), [Level::Forbidden]);
}

#[test]
fn cantus_climax_and_motif() {
    let score = cantus("dorian", "D4 F4 E4 G4 A4 G4 A4 F4 E4 D4");
    assert_eq!(fired(&score, "CF-CLIMAX"), [Level::Forbidden]);
    let score = cantus("dorian", "D4 F4 E4 G4 A4 G4 F4 E4 D4");
    assert!(fired(&score, "CF-CLIMAX").is_empty());
    // Highest note at 12 % of the length: too early.
    let score = cantus("dorian", "D4 A4 G4 F4 G4 F4 E4 D4");
    assert_eq!(fired(&score, "CF-CLIMAX"), [Level::P2]);
    let score = cantus("dorian", "D4 F4 E4 G4 A4 D4 F4 E4 G4 F4 E4 D4");
    assert_eq!(fired(&score, "CF-MOTIF"), [Level::P2]);
    assert!(fired(&cantus("dorian", DORIAN), "CF-MOTIF").is_empty());
}

#[test]
fn cantus_leaps_and_variety() {
    // D-G fourth (P1), F-A and D-F thirds (free), A-D fifth (P2).
    let score = cantus("dorian", "D4 G4 F4 A4 D4 F4 E4 D4");
    let mut levels = fired(&score, "CF-LEAP-COST");
    levels.sort_by_key(|l| l.cost());
    assert_eq!(levels, [Level::P1, Level::P2]);
    let score = cantus("dorian", "D4 F4 E4 F4 G4 A4 G4 F4 E4 D4");
    assert_eq!(fired(&score, "CF-VARIETY"), [Level::P1, Level::P1]);
    let score = cantus("dorian", "D4 F4 G4 F4 A4 F4 G4 F4 E4 D4");
    assert!(fired(&score, "CF-VARIETY").contains(&Level::P1));
    assert!(fired(&cantus("dorian", DORIAN), "CF-VARIETY").is_empty());
}

#[test]
fn cantus_ambitus() {
    // Fux's Phrygian cantus reaches A3 below the final E4: inside the plagal ambitus.
    let phrygian = cantus("phrygian", "E4 C4 D4 C4 A3 A4 G4 E4 F4 E4");
    assert!(fired(&phrygian, "CF-AMBITUS").is_empty());
    let low = cantus("dorian", "D4 F4 E4 C4 F3 G3 A3 E4 D4");
    assert_eq!(fired(&low, "CF-AMBITUS"), [Level::P2]);
}

#[test]
fn melodic_intervals() {
    let score = cantus("dorian", "D4 F4 B4 A4 G4 F4 E4 D4");
    assert_eq!(fired(&score, "MEL-TRITONE"), [Level::Forbidden]);
    let score = cantus("dorian", "D4 B4 A4 G4 F4 E4 D4");
    assert_eq!(fired(&score, "MEL-MAX-LEAP"), [Level::Forbidden]);
    let score = cantus("dorian", "D4 F4 D5 C5 A4 G4 F4 E4 D4");
    assert_eq!(fired(&score, "MEL-MAX-LEAP"), [Level::Forbidden]);
    let score = cantus("dorian", "D4 F4 A4 F4 G4 F4 E4 D4");
    assert!(fired(&score, "MEL-MAX-LEAP").is_empty());
}

#[test]
fn chromatic_step_two_voices() {
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 g' gis' a'");
    assert_eq!(fired(&score, "MEL-AUG-DIM"), [Level::Forbidden]);
}

#[test]
fn octave_leaps() {
    let score = above(Species::First, "D4 F4 E4 D4", "a1 a'1 g'1 a'1");
    assert_eq!(fired(&score, "MEL-OCTAVE"), [Level::Forbidden]);
    // Second species, voices a third apart on the downbeat: free.
    let score = above(Species::Second, "D4 F4 E4 D4", "f'2 f2 a2 c'2 g2 e'2 d'1");
    assert!(fired(&score, "MEL-OCTAVE").is_empty());
    // Voices a fifth apart: costs P3.
    let score = above(Species::Second, "D4 F4 E4 D4", "a'2 a2 c'2 e'2 g'2 e'2 d'1");
    assert_eq!(fired(&score, "MEL-OCTAVE"), [Level::P3]);
}

#[test]
fn repeated_notes() {
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 a' g' a'");
    assert_eq!(fired(&score, "MEL-REPEAT"), [Level::P1]);
    let score = above(
        Species::Second,
        "D4 F4 E4 D4",
        "r2 a'2 a'2 c''2 b'2 cis''2 d''1",
    );
    assert_eq!(fired(&score, "MEL-REPEAT"), [Level::Forbidden]);
    // Fifth species: a quarter anticipating a syncopation costs less than other repetitions.
    let score = above(
        Species::Fifth,
        "D4 F4 E4 D4",
        "r2 a'2~ a'4 g'4 g'2~ g'4 f'4 e'4 cis''4 d''1",
    );
    assert_eq!(fired(&score, "MEL-REPEAT"), [Level::P2]);
}

/// Free counterpoint: a note repeated after a black note costs MEL-REPEAT `free_black`, another
/// repeat `free`, or `free_texted` in a voice that sings; the anticipation keeps its level. The
/// third note of one pitch in a row or a later one costs at least `free_run`, whether the voice
/// sings or not.
#[test]
fn repeated_notes_in_free_counterpoint() {
    use crate::score::ScoreText;
    use crate::text::{Text, TextOptions};
    let repeats = |profile: &Profile, score: &Score| -> Vec<(u32, Level)> {
        Checker::new(profile)
            .check(score)
            .violations
            .into_iter()
            .filter(|v| v.rule == "MEL-REPEAT")
            .map(|v| (v.tick, v.level))
            .collect()
    };
    // A quarter repeated as a quarter (tick 2), a half as a quarter (8), the anticipation (12),
    // an eighth as an eighth (19), the third note in a row after it (20), a half as a half
    // (28), the third half in a row (32).
    let cp = "a'4 a'4 b'2 b'4 c''4 c''2 d''4 c''8 c''8 c''2 d''2 d''2 d''2 e''2";
    let free = above(Species::Free, "D4 F4 E4 F4 D4", cp);
    let fux = Profile::fux();
    let level = |param| fux.settings("MEL-REPEAT").level(param);
    assert_eq!((level("free"), level("free_black")), (Level::P1, Level::P1));
    assert_eq!(
        (level("free_texted"), level("free_run")),
        (Level::Off, Level::P1)
    );
    let mut profile = Profile::fux();
    profile.apply_override("MEL-REPEAT.free_black=P3").unwrap();
    profile.apply_override("MEL-REPEAT.free=P2").unwrap();
    profile.apply_override("MEL-REPEAT.free_run=P4").unwrap();
    const P1: Level = Level::P1;
    const P2: Level = Level::P2;
    const P3: Level = Level::P3;
    const P4: Level = Level::P4;
    const V: Level = Level::Forbidden;
    assert_eq!(
        repeats(&profile, &free),
        [
            (2, P3),
            (8, P2),
            (12, P2),
            (19, P3),
            (20, P4),
            (28, P2),
            (32, P4)
        ]
    );
    // Fux's profile prices them as before there were texts: `free_run` at P1 is no stricter
    // than `free` and `free_black` there.
    let mut runs = Profile::fux();
    runs.apply_override("MEL-REPEAT.free_run=aus").unwrap();
    assert_eq!(repeats(&runs, &free), repeats(&fux, &free));
    assert_eq!(
        repeats(&fux, &free),
        [
            (2, P1),
            (8, P1),
            (12, P2),
            (19, P1),
            (20, P1),
            (28, P1),
            (32, P1)
        ]
    );
    // Sung, the repeats of white notes take `free_texted` (`aus`).
    let mut sung = free.clone();
    sung.text = Some(ScoreText {
        text: std::sync::Arc::new(Text::own("pax.", &TextOptions::default()).unwrap().0),
        voices: vec![true, false],
    });
    assert_eq!(
        repeats(&profile, &sung),
        [(2, P3), (12, P2), (19, P3), (20, P4), (32, P4)]
    );
    // A run never costs less than its repeat: under a forbidden `free_texted` it stays
    // forbidden, and with `free_run` off the repeats keep their levels.
    profile.apply_override("MEL-REPEAT.free_texted=V").unwrap();
    let strict = repeats(&profile, &sung);
    assert_eq!(
        (strict[1], strict[5], strict[6]),
        ((8, V), (28, V), (32, V))
    );
    profile.apply_override("MEL-REPEAT.free_run=aus").unwrap();
    assert_eq!(repeats(&profile, &free)[3..5], [(19, P3), (20, P3)]);
    // The fifth species keeps `g5` and the anticipation.
    let fifth = above(Species::Fifth, "D4 F4 E4 F4 D4", cp);
    let levels: Vec<Level> = repeats(&profile, &fifth).iter().map(|r| r.1).collect();
    assert_eq!(levels, [P3, P3, P2, P3, P3, P3, P3]);
}

#[test]
fn leap_patterns() {
    let score = above(Species::First, "D4 F4 E4 D4 G4", "d'1 a'1 b'1 c''1 b'1");
    assert_eq!(fired(&score, "MEL-LEAP-RECOVERY"), [Level::P2]);
    let score = above(Species::First, "D4 F4 E4 D4", "c'1 e'1 g'1 f'1");
    assert!(fired(&score, "MEL-CONSEC-LEAPS").is_empty());
    let score = above(Species::First, "D4 F4 E4 D4", "c'1 f'1 bes'1 a'1");
    assert_eq!(fired(&score, "MEL-CONSEC-LEAPS"), [Level::P3]);
    let score = above(Species::First, "D4 F4 E4 D4", "c'1 g'1 e''1 d''1");
    assert_eq!(fired(&score, "MEL-CONSEC-LEAPS"), [Level::Forbidden]);
    // A triad with the larger leap above: a third, then a fourth upwards, or a fourth, then a
    // third downwards. Fux uses it now and then, Palestrina too; Jeppesen prefers the larger
    // below.
    let jeppesen = |score: &Score| -> Vec<Level> {
        Checker::new(&Profile::jeppesen())
            .check(score)
            .violations
            .into_iter()
            .filter(|v| v.rule == "MEL-CONSEC-LEAPS")
            .map(|v| v.level)
            .collect()
    };
    for larger_above in ["a'1 c'' f''", "f''1 c'' a'"] {
        let score = above(Species::First, "D4 F4 D4", larger_above);
        assert_eq!(
            fired(&score, "MEL-CONSEC-LEAPS"),
            [Level::P2],
            "{larger_above}"
        );
        assert_eq!(jeppesen(&score), [Level::P2], "{larger_above}");
    }
    for larger_below in ["a'1 d'' f''", "f''1 d'' a'"] {
        let score = above(Species::First, "D4 F4 D4", larger_below);
        assert!(jeppesen(&score).is_empty(), "{larger_below}");
    }
}

#[test]
fn tritone_outline() {
    let score = above(
        Species::First,
        "D4 F4 E4 D4 G4 F4",
        "f'1 g'1 a'1 b'1 a'1 a'1",
    );
    assert_eq!(fired(&score, "MEL-TRITONE-OUTLINE"), [Level::P2]);
    let score = above(
        Species::First,
        "D4 F4 E4 D4 G4 F4",
        "f'1 g'1 a'1 b'1 c''1 a'1",
    );
    assert!(fired(&score, "MEL-TRITONE-OUTLINE").is_empty());
}

#[test]
fn variety_and_climax() {
    let score = above(Species::Third, "D4 D4", "d''4 c'' d'' c'' d''1");
    assert!(!fired(&score, "MEL-VARIETY").is_empty());
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 c''1 g'1 c''1");
    assert_eq!(fired(&score, "MEL-CLIMAX"), [Level::P1]);
}

#[test]
fn voice_range() {
    // The soprano clef C1 ends on D5; F5 lies two steps above.
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 d''1 f''1 d''1");
    assert_eq!(fired(&score, "VOI-RANGE"), [Level::P2]);
}

#[test]
fn window_limits_reports() {
    // The search only wants what the newest note adds.
    let score = cantus("dorian", "D4 F4 B4 A4 G4 F4 E4 D4");
    let checker = Checker::new(&Profile::fux());
    let analysis = super::Analysis::new(&score);
    let mut early = Vec::new();
    checker.check_window(
        &score,
        &analysis,
        Window::partial(16, 16),
        &[true],
        &mut early,
    );
    assert!(early.iter().any(|v| v.rule == "MEL-TRITONE"));
    let mut late = Vec::new();
    checker.check_window(
        &score,
        &analysis,
        Window::partial(24, 56),
        &[true],
        &mut late,
    );
    assert!(late.iter().all(|v| v.rule != "MEL-TRITONE"));
    // Rules about the whole piece wait for the end.
    assert!(late.iter().all(|v| v.rule != "CF-END"));
    let mut none = Vec::new();
    checker.check_window(&score, &analysis, Window::ALL, &[false], &mut none);
    assert!(none.is_empty());
}

#[test]
fn sonorities() {
    // D4 against G4 is a fourth: a dissonance in two parts.
    let score = above(Species::First, "D4 F4 E4 D4", "g'1 a'1 c''1 d''1");
    assert!(fired(&score, "HAR-CONSONANCE").contains(&Level::Forbidden));
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 f'1 g'1 d''1");
    assert_eq!(fired(&score, "HAR-UNISON"), [Level::Forbidden]);
    let score = above(Species::First, "D4 F4 G4 E4 D4", "a'1 a'1 b'1 cis''1 d''1");
    assert!(fired(&score, "HAR-PERFECT-COST").is_empty());
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 c''1 g'1 d''1");
    assert!(fired(&score, "HAR-PERFECT-COST").contains(&Level::P1));
}

#[test]
fn parallel_and_hidden_perfect_consonances() {
    // D-A to F-C: parallel fifths.
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 c''1 cis''1 d''1");
    assert_eq!(fired(&score, "MOT-DIRECT-PERFECT"), [Level::Forbidden]);
    // D-F (third) to F-F (octave), both voices rising: hidden octave.
    let score = above(Species::First, "D4 F4 E4 D4", "f'1 f''1 cis''1 d''1");
    assert_eq!(fired(&score, "MOT-DIRECT-PERFECT"), [Level::Forbidden]);
    let clean = above(Species::First, "D4 F4 E4 D4", "a'1 a'1 cis''1 d''1");
    assert!(fired(&clean, "MOT-DIRECT-PERFECT").is_empty());
}

#[test]
fn cadences() {
    let good = above(Species::First, "D4 F4 E4 D4", "a'1 a'1 cis''1 d''1");
    for rule in ["CAD-START", "CAD-END", "CAD-PENULT", "FIC-SUBSEMITONIUM"] {
        assert!(fired(&good, rule).is_empty(), "{rule}");
    }
    // A fifth before the end instead of the major sixth.
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 a'1 b'1 d''1");
    assert_eq!(fired(&score, "CAD-PENULT"), [Level::Forbidden]);
    // C natural instead of the leading tone C sharp.
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 a'1 c''1 d''1");
    assert_eq!(fired(&score, "FIC-SUBSEMITONIUM"), [Level::Forbidden]);
    // The counterpoint below must start on the final: a fifth below is wrong.
    let below = Score::new(
        "dorian".parse().unwrap(),
        vec![
            cantus_voice("D4 F4 E4 D4"),
            Voice::new(
                VoiceType::Tenor,
                Role::Counterpoint(Species::First),
                parse_lilypond("g1 a1 cis'1 d'1", Duration::WHOLE).unwrap(),
            ),
        ],
    );
    assert_eq!(fired(&below, "CAD-START"), [Level::Forbidden]);
}

#[test]
fn ficta_scope() {
    // F sharp that does not rise.
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 fis'1 cis''1 d''1");
    assert!(fired(&score, "FIC-SCOPE").contains(&Level::Forbidden));
    // B flat in the Dorian mode: allowed at a small cost.
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 bes'1 cis''1 d''1");
    assert!(fired(&score, "FIC-SCOPE").is_empty());
    assert_eq!(fired(&score, "FIC-BFLAT"), [Level::P1]);
}

#[test]
fn voices_relative_to_each_other() {
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 e'1 cis''1 d''1");
    assert_eq!(fired(&score, "VOI-CROSSING"), [Level::P2]);
    // A5 over F4 is a tenth: fine. D6 over F4 is a thirteenth: too wide.
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 a''1 cis''1 d''1");
    assert!(fired(&score, "VOI-SPACING").is_empty());
    let score = above(Species::First, "D4 F4 E4 D4", "a'1 d'''1 cis''1 d''1");
    assert!(fired(&score, "VOI-SPACING").contains(&Level::Forbidden));
}
