//! Minimal examples for the rules of three or more voices.

use super::{Checker, Level, Profile};
use crate::clef::VoiceType;
use crate::parse::{parse_lilypond, parse_pitches};
use crate::score::{Note, Role, Score, Species, Voice};
use crate::time::Duration;

const CF: &str = "D4 F4 G4 E4 D4";
/// Soprano and bass of a good setting with the cantus in the alto: the clausula C♯5–D5 above
/// E4–D4, the bass A2–D3.
const SOPRANO: &str = "a'1 d'' e'' cis'' d''";
const BASS: &str = "d1 bes, g, a, d";

/// Soprano and bass in the first species around the cantus in the alto.
fn three(soprano: &str, bass: &str) -> Score {
    let voice = |voice_type, notes: &str| {
        Voice::new(
            voice_type,
            Role::Counterpoint(Species::First),
            parse_lilypond(notes, Duration::WHOLE).unwrap(),
        )
    };
    let cantus = parse_pitches(CF)
        .unwrap()
        .into_iter()
        .map(|p| Note::new(p, Duration::WHOLE))
        .collect();
    let score = Score::new(
        "dorian".parse().unwrap(),
        vec![
            voice(VoiceType::Soprano, soprano),
            Voice::new(VoiceType::Alto, Role::CantusFirmus, cantus),
            voice(VoiceType::Bass, bass),
        ],
    );
    score.validate().unwrap();
    score
}

/// Levels with which the rule fired against the two counterpoints.
fn fired(score: &Score, rule: &str) -> Vec<Level> {
    let targets = [true, false, true];
    Checker::new(&Profile::fux())
        .check_targets(score, &targets)
        .violations
        .into_iter()
        .filter(|v| v.rule == rule)
        .map(|v| v.level)
        .collect()
}

#[test]
fn a_good_setting_breaks_no_prohibition() {
    let targets = [true, false, true];
    let report = Checker::new(&Profile::fux()).check_targets(&three(SOPRANO, BASS), &targets);
    let forbidden: Vec<&str> = report
        .violations
        .iter()
        .filter(|v| v.is_forbidden())
        .map(|v| v.rule)
        .collect();
    assert!(forbidden.is_empty(), "{forbidden:?}");
}

#[test]
fn motion_counts_against_the_bass() {
    // Soprano and cantus rise together into measures 2 and 3, but both move against the bass:
    // only the close in similar motion counts.
    assert_eq!(fired(&three(SOPRANO, BASS), "MOT-PREFERENCE"), [Level::P2]);
}

#[test]
fn parallels_and_sonorities() {
    // F4 over B♭2, G4 over C3, E4 over A2: twice parallel twelfths between cantus and bass.
    let score = three(SOPRANO, "d1 bes, c a, d");
    assert_eq!(
        fired(&score, "TEX-PARALLEL"),
        [Level::Forbidden, Level::Forbidden]
    );
    assert!(fired(&three(SOPRANO, BASS), "TEX-PARALLEL").is_empty());
    // F5 and F4 over B♭2: no third.
    let score = three("a'1 f'' e'' cis'' d''", BASS);
    assert_eq!(fired(&score, "TEX-TRIAD"), [Level::P3]);
    assert!(fired(&three(SOPRANO, BASS), "TEX-TRIAD").is_empty());
}

/// The cantus of [`three`] in whole notes, as a free alto.
const ALTO: &str = "d'1 f' g' e' d'";

/// Soprano, alto and bass as in [`three`], but every voice free: free counterpoint.
fn free(soprano: &str, alto: &str, bass: &str) -> Score {
    let voice = |voice_type, notes: &str| {
        Voice::new(
            voice_type,
            Role::Counterpoint(Species::Free),
            parse_lilypond(notes, Duration::WHOLE).unwrap(),
        )
    };
    let score = Score::new(
        "dorian".parse().unwrap(),
        vec![
            voice(VoiceType::Soprano, soprano),
            voice(VoiceType::Alto, alto),
            voice(VoiceType::Bass, bass),
        ],
    );
    score.validate().unwrap();
    assert!(score.is_free());
    score
}

/// Levels with which the rule fired in a whole score with every voice a target, under the
/// standard profile with the overrides given.
fn fired_in(score: &Score, rule: &str, overrides: &[&str]) -> Vec<Level> {
    let mut profile = Profile::fux();
    for spec in overrides {
        profile.apply_override(spec).unwrap();
    }
    Checker::new(&profile)
        .check(score)
        .violations
        .into_iter()
        .filter(|v| v.rule == rule)
        .map(|v| v.level)
        .collect()
}

#[test]
fn kinds_of_sonorities() {
    use super::texture::{Kind, kind};
    assert_eq!(kind(&[2, 4]), Some(Kind::Complete));
    assert_eq!(kind(&[0, 2, 4]), Some(Kind::Complete));
    assert_eq!(kind(&[2, 5]), Some(Kind::Sixth));
    assert_eq!(kind(&[0, 2]), Some(Kind::NoFifth));
    assert_eq!(kind(&[0, 4]), Some(Kind::NoThird));
    assert_eq!(kind(&[0]), Some(Kind::NoThird));
    assert_eq!(kind(&[0, 5]), Some(Kind::BareSixth));
    assert_eq!(kind(&[5, 5]), Some(Kind::BareSixth));
    // A fifth with a sixth, a fourth or a seventh above the bass: dissonant.
    assert_eq!(kind(&[4, 5]), None);
    assert_eq!(kind(&[2, 3]), None);
    assert_eq!(kind(&[0, 6]), None);
}

#[test]
fn the_bare_sixth_costs_only_in_free_counterpoint() {
    // E5 and G4 over G2 in measure 3: a sixth and an octave without a third. The species leave
    // it free, as Fux writes it three times in his three-voice examples (Fig. 104, 115, 119).
    assert!(fired(&three(SOPRANO, BASS), "TEX-TRIAD").is_empty());
    let score = free(SOPRANO, ALTO, BASS);
    assert_eq!(fired_in(&score, "TEX-TRIAD", &[]), [Level::P2]);
    let off = ["TEX-TRIAD.free_bare_sixth=aus"];
    assert!(fired_in(&score, "TEX-TRIAD", &off).is_empty());
    // D5 instead of E5: a fifth and an octave without a third, also P2.
    let score = free("a'1 d'' d'' cis'' d''", ALTO, BASS);
    assert_eq!(fired_in(&score, "TEX-TRIAD", &off), [Level::P2]);
}

#[test]
fn the_third_of_the_final_chord_in_free_counterpoint() {
    // D3 D4 D5: the species leave the third free (`missing_third` aus), free counterpoint
    // prefers it (`free_missing_third`).
    assert!(fired(&three(SOPRANO, BASS), "TEX-FINAL-CHORD").is_empty());
    let bare = free(SOPRANO, ALTO, BASS);
    assert_eq!(fired_in(&bare, "TEX-FINAL-CHORD", &[]), [Level::P2]);
    let off = ["TEX-FINAL-CHORD.free_missing_third=aus"];
    assert!(fired_in(&bare, "TEX-FINAL-CHORD", &off).is_empty());
    let prefer = ["TEX-FINAL-CHORD.missing_third=P3"];
    assert_eq!(fired_in(&bare, "TEX-FINAL-CHORD", &prefer), [Level::P2]);
    // F♯4 in the alto: the major third.
    let major = free(SOPRANO, "d'1 f' g' e' fis'", BASS);
    assert!(fired_in(&major, "TEX-FINAL-CHORD", &[]).is_empty());
    // F4: the minor third is forbidden, and it is no major third.
    let minor = free(SOPRANO, "d'1 f' g' e' f'", BASS);
    assert_eq!(
        fired_in(&minor, "TEX-FINAL-CHORD", &[]),
        [Level::Forbidden, Level::P2]
    );
}

#[test]
fn only_free_counterpoint_names_the_missing_third() {
    use super::Detail;
    // The species report the case under the rule's title alone, as before `free_missing_third`;
    // the label of `missing_third` is a hint for its setting, not a message.
    let details = |score: &Score, targets: &[bool], overrides: &[&str]| {
        let mut profile = Profile::fux();
        for spec in overrides {
            profile.apply_override(spec).unwrap();
        }
        Checker::new(&profile)
            .check_targets(score, targets)
            .violations
            .into_iter()
            .filter(|v| v.rule == "TEX-FINAL-CHORD")
            .map(|v| v.detail)
            .collect::<Vec<_>>()
    };
    let prefer = ["TEX-FINAL-CHORD.missing_third=P3"];
    assert_eq!(
        details(&three(SOPRANO, BASS), &[true, false, true], &prefer),
        [Detail::None]
    );
    assert_eq!(
        details(&free(SOPRANO, ALTO, BASS), &[true; 3], &[]),
        [Detail::Param("free_missing_third")]
    );
}

#[test]
fn first_and_final_chords() {
    // B4 over D3 is a sixth: not a tone of the final's triad.
    let score = three("b'1 d'' e'' cis'' d''", BASS);
    assert_eq!(fired(&score, "TEX-FIRST-CHORD"), [Level::Forbidden]);
    // F5 over D3: the minor third in the final chord.
    let score = three("a'1 d'' e'' e'' f''", BASS);
    assert!(fired(&score, "TEX-FINAL-CHORD").contains(&Level::Forbidden));
    // C5 instead of the leading tone: no clausula.
    let score = three("a'1 d'' e'' c'' d''", BASS);
    assert_eq!(fired(&score, "TEX-CLAUSULA"), [Level::Forbidden]);
    assert!(fired(&three(SOPRANO, BASS), "TEX-CLAUSULA").is_empty());
}

#[test]
fn dissonances_between_upper_voices() {
    // G4 against the cantus F4: a second, though consonant with the bass B♭2.
    let score = three("a'1 g' b' cis'' d''", BASS);
    assert_eq!(fired(&score, "HAR-DISSONANCE"), [Level::Forbidden]);
    // B♭4 above F4 is a fourth, free between upper voices.
    let score = three("a'1 bes' d'' cis'' d''", BASS);
    assert!(fired(&score, "HAR-DISSONANCE").is_empty());
    // C5 above G2 is a fourth above the bass, judged by HAR-CONSONANCE.
    let score = three("a'1 d'' c'' cis'' d''", BASS);
    assert_eq!(fired(&score, "HAR-CONSONANCE"), [Level::Forbidden]);
    assert!(fired(&score, "HAR-DISSONANCE").is_empty());
    // F♯5 against F4.
    let score = three("a'1 fis'' e'' cis'' d''", BASS);
    assert!(fired(&score, "HAR-CHROMATIC-CLASH").contains(&Level::Forbidden));
}

#[test]
fn the_cadence() {
    // The bass on G2 under the leading tone: no cadential step, and an augmented fourth.
    let score = three("a'1 d'' e'' cis'' d''", "d1 bes, g, g, d");
    assert!(fired(&score, "TEX-PENULT").contains(&Level::Forbidden));
    // The leading tone in soprano and bass, in the bass not delayed.
    let score = three("a'1 d'' e'' cis'' d''", "d1 bes, g, cis d");
    assert_eq!(fired(&score, "TEX-CLAUSULA"), [Level::Forbidden]);
    assert_eq!(fired(&score, "TEX-PENULT"), [Level::Forbidden]);
    assert!(fired(&three(SOPRANO, BASS), "TEX-PENULT").is_empty());
}

/// Soprano, cantus in the alto and bass, each with its species.
fn setting(soprano: (Species, &str), cantus: &str, bass: (Species, &str)) -> Score {
    let voice = |voice_type, role, notes: &str| {
        Voice::new(
            voice_type,
            role,
            parse_lilypond(notes, Duration::WHOLE).unwrap(),
        )
    };
    let score = Score::new(
        "dorian".parse().unwrap(),
        vec![
            voice(VoiceType::Soprano, Role::Counterpoint(soprano.0), soprano.1),
            voice(VoiceType::Alto, Role::CantusFirmus, cantus),
            voice(VoiceType::Tenor, Role::Counterpoint(bass.0), bass.1),
        ],
    );
    score.validate().unwrap();
    score
}

#[test]
fn the_leading_tone_in_the_bass_only_late() {
    use Species::{First, Second};
    // Fig. 123: the bass holds the final over the downbeat and brings the leading tone late,
    // against the diminished fifth G4. HAR-DISSONANCE reports it with the level `aus`.
    let late = setting(
        (First, "d''1 a' g' fis'"),
        "d'1 f' e' d'",
        (Second, "r2 d'2 | d2 d'2~ | d'2 cis'2 | d'1"),
    );
    assert!(fired(&late, "TEX-PENULT").is_empty());
    assert!(fired(&late, "G2-PASSING").is_empty());
    // The leading tone on the downbeat: neither the bass nor the tritone of the passing G4
    // above it is tolerated.
    let early = setting(
        (Second, "r2 d''2 | a'2 b'2 | a'2 g'2 | fis'1"),
        "d'1 f' e' d'",
        (First, "d1 d cis d"),
    );
    assert_eq!(
        fired(&early, "TEX-PENULT"),
        [Level::Forbidden, Level::Forbidden]
    );
}
