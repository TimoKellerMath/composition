//! Minimal examples for the rules of the text underlay (TXT-*): for every parameter a texted
//! voice with one case of it and one without, with the tick at which the case is decided and the
//! note it marks; the rules' silence without a text; MEL-REPEAT in voices that sing; and the
//! checker's cost of the underlay the dynamic program finds.

use std::sync::Arc;

use super::{Checker, Detail, Level, ParamKind, Profile, Violation, all_rules};
use crate::clef::VoiceType;
use crate::gabc::parse_gabc;
use crate::generate::Rng;
use crate::interval::Interval;
use crate::pitch::Step;
use crate::score::{Cadence, Lyric, Note, Role, Score, ScoreText, Species, Voice};
use crate::text::test_support::{below, own, random_text, random_voice, section, sung};
use crate::text::underlay::{Goal, Levels, Model};
use crate::text::{Text, TextKind, TextOptions, resolve};
use crate::time::Meter;

/// An own text taken as a long one (Gloria, Credo).
fn long(src: &str) -> Text {
    let mut text = own(src);
    text.kind = TextKind::Long;
    text
}

fn kyrie() -> Text {
    section("kyrie")
}

/// A Dorian score of free voices that sing `text`, with internal cadences as (measure from 0,
/// target).
fn texted(text: &Text, voices: &[Vec<Note>], cadences: &[(usize, Step)]) -> Score {
    const TYPES: [VoiceType; 4] = [
        VoiceType::Soprano,
        VoiceType::Alto,
        VoiceType::Tenor,
        VoiceType::Bass,
    ];
    let voices = voices
        .iter()
        .zip(TYPES)
        .map(|(notes, voice_type)| {
            Voice::new(voice_type, Role::Counterpoint(Species::Free), notes.clone())
        })
        .collect();
    let mut score = Score::new("dorian".parse().unwrap(), voices);
    score.cadences = cadences
        .iter()
        .map(|&(measure, target)| Cadence { measure, target })
        .collect();
    score.text = Some(ScoreText {
        text: Arc::new(text.clone()),
        voices: vec![true; score.voices.len()],
    });
    score
}

/// The parameter whose level a violation of a TXT rule carries.
fn param(violation: &Violation) -> &'static str {
    match violation.detail {
        Detail::Param(name) => name,
        // Only TXT-MELISMA `long` counts the attacks.
        Detail::Count(_) => "long",
        other => panic!("{}: {other:?}", violation.rule),
    }
}

/// The cases of the TXT rules a check of the whole score reports: (rule, parameter, tick, marked
/// note).
fn txt(checker: &Checker, score: &Score) -> Vec<(&'static str, &'static str, u32, usize)> {
    checker
        .check(score)
        .violations
        .iter()
        .filter(|v| v.rule.starts_with("TXT-"))
        .map(|v| (v.rule, param(v), v.tick, v.note))
        .collect()
}

/// Fux's profile with every case of the TXT rules switched on: those free by default cost P1.
fn every_case() -> Profile {
    let mut profile = Profile::fux();
    for rule in all_rules() {
        let id = rule.info().id;
        if !id.starts_with("TXT-") {
            continue;
        }
        for param in rule.info().params.iter() {
            if param.kind == ParamKind::Level
                && profile.settings(id).level(param.name) == Level::Off
            {
                profile
                    .apply_override(&format!("{id}.{}=P1", param.name))
                    .unwrap();
            }
        }
    }
    profile
}

/// A voice that sings a text, in a score with internal cadences.
struct Setting {
    text: Text,
    voice: &'static str,
    cadences: Vec<(usize, Step)>,
}

impl Setting {
    fn score(&self) -> Score {
        let score = texted(&self.text, &[sung(self.voice)], &self.cadences);
        if let Err(e) = score.validate() {
            panic!("{}: {e}", self.voice);
        }
        score
    }
}

/// A parameter of a TXT rule: a voice with one case of it, decided at a tick on a note, and a
/// voice without it.
struct Example {
    rule: &'static str,
    param: &'static str,
    with: Setting,
    /// The tick and the marked note.
    at: (u32, usize),
    without: Setting,
}

fn example(
    (rule, param): (&'static str, &'static str),
    text: &Text,
    with: &'static str,
    at: (u32, usize),
    without: &'static str,
) -> Example {
    cadence_example((rule, param), (text, with, &[]), at, (text, without, &[]))
}

fn cadence_example(
    (rule, param): (&'static str, &'static str),
    (text, with, cadences): (&Text, &'static str, &[(usize, Step)]),
    at: (u32, usize),
    (other, without, other_cadences): (&Text, &'static str, &[(usize, Step)]),
) -> Example {
    Example {
        rule,
        param,
        with: Setting {
            text: text.clone(),
            voice: with,
            cadences: cadences.to_vec(),
        },
        at,
        without: Setting {
            text: other.clone(),
            voice: without,
            cadences: other_cadences.to_vec(),
        },
    }
}

/// One or more examples for every level parameter of the TXT rules. Measures are 8 ticks long, a
/// half note 4.
fn examples() -> Vec<Example> {
    let pax = own("pax pax pax pax.");
    let kyrie = kyrie();
    let ante = own("an-te De-um.");
    let broken = own("pax | pax pax pax.");
    let comma = own("pax pax, pax pax.");
    let stops = own("pax pax. pax pax.");
    let dominus = own("Dó-mi-nus pax.");
    let domine = own("Dó-mi-ne De-us.");
    let et = own("et in ter-ra pax.");
    let deus = own("De-us.");
    let long_comma = long("pax pax, pax pax.");
    let long_pax = long("pax pax pax pax.");
    let sanctus = section("sanctus");
    let amen = own("Glo-ri-a. A-men.");
    let goal_d = [(2, Step::D)];
    vec![
        // A new syllable on an eighth, on a white note and on a quarter off the minim beats, on a
        // rest and on a tie continuation.
        example(
            ("TXT-ONSET", "level"),
            &pax,
            "d'2=0 e'4 f'8=1 g'8 a'2 b'2",
            (6, 2),
            "d'2=0 e'4 f'8 g'8 a'2 b'2=1",
        ),
        example(
            ("TXT-ONSET", "level"),
            &pax,
            "d'4=0 e'2=1 f'4 g'1",
            (2, 1),
            "d'4=0 e'2 f'4 g'1=1",
        ),
        example(
            ("TXT-ONSET", "level"),
            &pax,
            "d'2=0 e'4 f'4=1 g'2 a'2",
            (6, 2),
            "d'2=0 e'4 f'4 g'2 a'2=1",
        ),
        example(
            ("TXT-ONSET", "level"),
            &pax,
            "d'2=0 r2=1 e'2=1 f'2",
            (4, 1),
            "d'2=0 r2 e'2=1 f'2",
        ),
        example(
            ("TXT-ONSET", "level"),
            &pax,
            "d'2=0 e'2~ e'2=1 f'2",
            (8, 2),
            "d'2=0 e'2~ e'2 f'2=1",
        ),
        example(
            ("TXT-ONSET", "run_upbeat"),
            &pax,
            "d'2=0 e'4=1 f'4 g'2 a'2",
            (4, 1),
            "d'2=0 e'4 f'4 g'2 a'2=1",
        ),
        // A texted quarter without a run, decided where the next note begins.
        example(
            ("TXT-ONSET", "single"),
            &pax,
            "d'1=0 e'4=1 f'2. g'2 a'2",
            (10, 1),
            "d'1=0 e'4=1 f'4 g'2 a'2 b'2",
        ),
        // The dactyl quarter of an-te De-um: a half or longer must follow it.
        example(
            ("TXT-ONSET", "single"),
            &ante,
            "d'2.=0 e'4=1 f'4=2 g'4 a'2 b'1=3",
            (8, 1),
            "d'2.=0 e'4=1 f'2=2 g'2 a'1=3",
        ),
        // The dactyl quarter takes the short middle syllable of Dó-mi-ne; any other syllable
        // there, such as a stressed one, costs.
        example(
            ("TXT-ONSET", "figure"),
            &domine,
            "d'2.=0 e'4=1 f'1=2 g'1=3 a'1=4",
            (6, 1),
            "d'2.=0 e'4 f'1=1 g'1=2 a'1=3 b'1=4",
        ),
        example(
            ("TXT-ONSET", "figure_accent"),
            &pax,
            "d'2.=0 e'4=1 f'2 g'2",
            (6, 1),
            "d'2.=0 e'4 f'2 g'2=1",
        ),
        // A restatement on a quarter, and on a quarter after a rest; on a half it is free.
        example(
            ("TXT-ONSET", "opening_black"),
            &pax,
            "d'1=0 e'1=1 f'1=2 g'1=3 a'4=0 b'4 c''2 b'1=1 a'1=2 g'1=3",
            (32, 4),
            "d'1=0 e'1=1 f'1=2 g'1=3 a'2=0 b'2 c''1=1 b'1=2 a'1=3",
        ),
        example(
            ("TXT-ONSET", "opening_black"),
            &pax,
            "d'1=0 e'1=1 f'1=2 g'1=3 r2 a'4=0 b'4 c''2 b'2=1 a'1=2 g'1=3",
            (36, 5),
            "d'1=0 e'1=1 f'1=2 g'1=3 r2 a'2=0 b'1=1 a'2=2 g'2 f'1=3",
        ),
        example(
            ("TXT-AFTER-BLACK", "level"),
            &pax,
            "d'2=0 e'4 f'4 g'2=1 a'2",
            (8, 3),
            "d'2=0 e'4 f'4 g'2 a'2=1",
        ),
        // The two licences: after the texted dactyl quarter, and the last syllable of a unit on a
        // half or longer (on a quarter it is none).
        example(
            ("TXT-AFTER-BLACK", "figure"),
            &pax,
            "d'2.=0 e'4=1 f'2=2 g'2",
            (8, 2),
            "d'2.=0 e'4=1 f'2 g'2=2",
        ),
        example(
            ("TXT-AFTER-BLACK", "unit_end"),
            &pax,
            "d'2=0 e'2=1 f'2=2 g'4 a'4 b'1=3",
            (16, 5),
            "d'2=0 e'2=1 f'2=2 g'4 a'4 b'4=3 a'4 g'2",
        ),
        // A half after the untexted quarter of a dotted half; after other quarters it is
        // `level`.
        example(
            ("TXT-AFTER-BLACK", "dotted"),
            &pax,
            "d'1=0 e'2.=1 f'4 g'2=2 a'2",
            (16, 3),
            "d'1=0 e'2=1 f'4 g'4 a'2=2 b'2",
        ),
        // A pitch struck again after a white note; after a black one it is an anticipation.
        example(
            ("TXT-REPEATED-PITCH", "level"),
            &pax,
            "d'2=0 d'2 e'1=1",
            (4, 1),
            "d'2=0 e'4 e'4 f'2 g'2=1",
        ),
        // After a rest, and at the voice's first note.
        example(
            ("TXT-ENTRY", "level"),
            &pax,
            "d'2=0 e'2=1 r2 f'2 g'1",
            (12, 3),
            "d'2=0 e'2=1 r2 f'2=2 g'1=3",
        ),
        example(
            ("TXT-ENTRY", "level"),
            &pax,
            "d'2 e'2=0 f'1=1",
            (0, 0),
            "d'2=0 e'2 f'1=1",
        ),
        example(
            ("TXT-ORDER", "level"),
            &pax,
            "d'2=0 e'2=2 f'1=3",
            (4, 1),
            "d'2=0 e'2=1 f'1=2",
        ),
        example(
            ("TXT-ORDER", "first"),
            &pax,
            "d'2=1 e'2=2 f'1=3",
            (0, 0),
            "d'2=0 e'2=1 f'1=2",
        ),
        example(
            ("TXT-REPEAT", "short"),
            &pax,
            "d'1=0 e'1=1 f'1=2 g'1=3 a'1=0 b'1=1 a'1=2 g'1=3",
            (32, 4),
            "d'1=0 e'1=1 f'1=2 g'1=3",
        ),
        // The last unit of a long text may be restated.
        example(
            ("TXT-REPEAT", "long"),
            &long_comma,
            "d'1=0 e'1=1 f'1=0 g'1=1 a'1=2 b'1=3",
            (16, 2),
            "d'1=0 e'1=1 f'1=2 g'1=3 a'1=2 b'1=3",
        ),
        // A third statement.
        example(
            ("TXT-REPEAT", "again"),
            &pax,
            "d'1=0 e'1=1 f'1=2 g'1=3 a'1=0 b'1=1 a'1=2 g'1=3 f'1=0 e'1=1 d'1=2 e'1=3",
            (64, 8),
            "d'1=0 e'1=1 f'1=2 g'1=3 a'1=0 b'1=1 a'1=2 g'1=3",
        ),
        // San-ctus, San-ctus, San-ctus: lone words. A-men. is a sentence of its own.
        cadence_example(
            ("TXT-REPEAT", "word"),
            (&sanctus, "d'1=0 e'1=1 f'1=2 g'1=3 a'1=2 b'1=3", &[]),
            (32, 4),
            (&amen, "d'1=0 e'1=1 f'1=2 g'1=3 a'1=4 b'1=3 a'1=4", &[]),
        ),
        example(
            ("TXT-REST", "word"),
            &dominus,
            "d'2=0 r2 e'2=1 f'2=2 g'1=3",
            (4, 1),
            "d'2=0 e'2=1 f'2=2 r2 g'1=3",
        ),
        // The marked note is the black one before the rest.
        example(
            ("TXT-REST", "short"),
            &pax,
            "d'2=0 e'4 f'4 r2 g'2=1 a'1",
            (8, 2),
            "d'2=0 e'4 f'4 g'2 r2 a'1=1",
        ),
        // At the voice's end, which the complete score shows.
        example(
            ("TXT-REST", "short"),
            &pax,
            "d'2=0 e'2=1 f'2=2 g'4=3 a'4",
            (16, 4),
            "d'2=0 e'2=1 f'2=2 g'2=3",
        ),
        // A rest at a marked break is free.
        example(
            ("TXT-REST", "clause"),
            &broken,
            "d'2=0 e'2=1 r2 f'2=2 g'1=3",
            (8, 2),
            "d'2=0 r2 e'2=1 f'2=2 g'1=3",
        ),
        // A rest that leaves only the last word of the unit; with two words left it is
        // `clause`.
        example(
            ("TXT-REST", "last_word"),
            &broken,
            "d'2=0 e'2=1 f'2=2 r2 g'1=3",
            (12, 3),
            "d'2=0 e'2=1 r2 f'2=2 g'1=3",
        ),
        // A rest after a comma and at a marked break; after a full stop it is free.
        cadence_example(
            ("TXT-REST", "comma"),
            (&comma, "d'1=0 e'2=1 r2 f'1=2 g'1=3", &[]),
            (12, 2),
            (&stops, "d'1=0 e'2=1 r2 f'1=2 g'1=3", &[]),
        ),
        example(
            ("TXT-REST", "comma"),
            &broken,
            "d'2=0 r2 e'2=1 f'2=2 g'1=3",
            (4, 1),
            "d'2=0 e'2=1 r2 f'2=2 g'1=3",
        ),
        // A rest longer than a half after a comma, decided by the rest that makes it so.
        example(
            ("TXT-REST", "long"),
            &comma,
            "d'1=0 e'2=1 r2 r2 f'2=2 g'1=3",
            (16, 3),
            "d'1=0 e'2=1 r2 f'2=2 g'2 a'1=3",
        ),
        // Decided once the score is complete, at the last sounding.
        example(
            ("TXT-COMPLETE", "level"),
            &pax,
            "d'1=0 e'1=1",
            (8, 1),
            "d'1=0 e'1=1 f'1=2 g'1=3",
        ),
        example(
            ("TXT-MELISMA", "unstressed"),
            &kyrie,
            "d'1=0 e'1=1 f'1=2 g'1 a'1=3 b'1=4 a'1=5 g'1=6",
            (24, 2),
            "d'1=0 e'1=1 f'1=2 g'1=3 a'1=4 b'1=5 a'1=6 g'1",
        ),
        example(
            ("TXT-MELISMA", "function_word"),
            &et,
            "d'1=0 e'1=1 f'1 g'1=2 a'1=3 b'1=4",
            (16, 1),
            "d'1=0 e'1=1 f'1=2 g'1 a'1=3 b'1=4",
        ),
        // Decided where a note follows one of the syllable's that is not black; a lead-in in
        // quarters is free.
        example(
            ("TXT-MELISMA", "last_syllable"),
            &pax,
            "d'1=0 e'1=1 f'1=2 g'2=3 a'2",
            (28, 3),
            "d'1=0 e'1=1 f'1=2 g'4=3 a'4 b'2",
        ),
        // The lead-in lies within the measure before the last note.
        example(
            ("TXT-MELISMA", "last_syllable"),
            &pax,
            "d'1=0 e'1=1 f'1=2 g'4=3 a'4 b'4 c''4 d''4 e''4 d''2 c''1",
            (34, 3),
            "d'1=0 e'1=1 f'1=2 g'4=3 a'4 b'4 c''4 d''1",
        ),
        // The eleventh note of a syllable in a short text, the ninth in a long one.
        example(
            ("TXT-MELISMA", "long"),
            &kyrie,
            "d'2=0 e'2 d'2 e'2 d'2 e'2 d'2 e'2 d'2 e'2 d'2 e'2=1",
            (40, 10),
            "d'2=0 e'2 d'2 e'2 d'2 e'2 d'2 e'2 d'2 e'2 d'2=1 e'2",
        ),
        example(
            ("TXT-MELISMA", "long"),
            &long_pax,
            "d'2=0 e'2 d'2 e'2 d'2 e'2 d'2 e'2 d'2 e'2=1 d'2 e'2",
            (32, 8),
            "d'2=0 e'2 d'2 e'2 d'2 e'2 d'2 e'2 d'2=1 e'2 d'2 e'2",
        ),
        example(
            ("TXT-ACCENT", "short_penult"),
            &kyrie,
            "d'1=0 e'2=1 f'2 g'1=2",
            (12, 1),
            "d'1=0 e'1=1 f'2=2 g'2",
        ),
        // Ký on a half, ri on a whole: decided where ri begins. On a half tied to a half it is
        // decided at the tie continuation.
        example(
            ("TXT-ACCENT", "longer"),
            &kyrie,
            "r2 d'2=0 e'1=1 f'1=2",
            (8, 2),
            "r2 d'2=0 e'2=1 f'2=2 g'1=3",
        ),
        example(
            ("TXT-ACCENT", "longer"),
            &kyrie,
            "d'2=0 e'2~=1 e'2 f'2=2 g'1=3",
            (8, 1),
            "d'1=0 e'2=1 f'2=2 g'1=3",
        ),
        // A long e- before lé.
        example(
            ("TXT-ACCENT", "opening"),
            &kyrie,
            "d'1=0 e'1=1 f'1=2 g'1=3 a'2=4 g'2=5 f'1=6",
            (36, 3),
            "d'1=0 e'1=1 f'1=2 g'2=3 a'2=4 b'2=5 a'2=6 g'1",
        ),
        example(
            ("TXT-ACCENT", "weak"),
            &deus,
            "r2 e'2=0 f'1=1",
            (8, 1),
            "e'1=0 f'1=1",
        ),
        example(
            ("TXT-LEAP", "octave"),
            &kyrie,
            "d'1=0 d''1=1",
            (8, 1),
            "d'1=0 f'1=1",
        ),
        example(
            ("TXT-LEAP", "sixth"),
            &kyrie,
            "d'1=0 b'1=1",
            (8, 1),
            "d'1=0 f'1=1",
        ),
        example(
            ("TXT-LEAP", "fifth"),
            &kyrie,
            "d'1=0 a'1=1",
            (8, 1),
            "d'1=0 f'1=1",
        ),
        // Between words a leap is free.
        example(
            ("TXT-LEAP", "fourth"),
            &kyrie,
            "d'1=0 g'1=1",
            (8, 1),
            "d'1=0 g'1 a'1=1",
        ),
        // The goal in measure 3: the voice reaches the final by a step on the ri of Kyrie; a
        // full stop there is free.
        cadence_example(
            ("TXT-CADENCE", "level"),
            (
                &kyrie,
                "f'1=0 e'1 d'1=1 e'1=2 f'1=3 e'1=4 d'1=5 e'1=6",
                &goal_d,
            ),
            (16, 2),
            (&stops, "f'1=0 e'1=1 d'1 e'1=2 f'1 e'1=3 d'1", &goal_d),
        ),
        // A principal cadence at a comma.
        cadence_example(
            ("TXT-CADENCE", "principal"),
            (&comma, "f'1=0 e'1=1 d'1 e'1=2 f'1 e'1=3 d'1", &goal_d),
            (16, 2),
            (&stops, "f'1=0 e'1=1 d'1 e'1=2 f'1 e'1=3 d'1", &goal_d),
        ),
        // At a comma on F, which is no principal degree of the Dorian mode; a goal the voice
        // does not reach costs nothing.
        cadence_example(
            ("TXT-CADENCE", "comma"),
            (
                &comma,
                "a'1=0 g'1=1 f'1 g'1=2 a'1 g'1=3 f'1",
                &[(2, Step::F)],
            ),
            (16, 2),
            (
                &comma,
                "a'1=0 g'1=1 f'1 g'1=2 a'1 g'1=3 f'1",
                &[(2, Step::A)],
            ),
        ),
        // Before a marked break as at a comma: on the e of Ký-ri-e before its tail.
        cadence_example(
            ("TXT-CADENCE", "comma"),
            (
                &kyrie,
                "a'1=0 g'1=1 f'1=2 g'1=3 a'1=4 g'1=5 f'1=6",
                &[(2, Step::F)],
            ),
            (16, 2),
            (
                &kyrie,
                "a'1=0 g'1 f'1=1 g'1=2 a'1=3 g'1=4 f'1=5 e'1=6",
                &[(2, Step::F)],
            ),
        ),
    ]
}

/// Every parameter of the TXT rules, in a minimal score: the case is reported once, by its rule
/// and parameter, at the tick at which it is decided and on the note it marks; the voice without
/// it has no such case.
#[test]
fn each_parameter_fires_once() {
    let checker = Checker::new(&every_case());
    let examples = examples();
    for rule in all_rules() {
        let id = rule.info().id;
        for param in rule.info().params.iter() {
            if id.starts_with("TXT-") && param.kind == ParamKind::Level {
                assert!(
                    examples
                        .iter()
                        .any(|e| (e.rule, e.param) == (id, param.name)),
                    "no example for {id}.{}",
                    param.name
                );
            }
        }
    }
    for e in &examples {
        let case = (e.rule, e.param);
        let found = txt(&checker, &e.with.score());
        let hits: Vec<(u32, usize)> = found
            .iter()
            .filter(|f| (f.0, f.1) == case)
            .map(|f| (f.2, f.3))
            .collect();
        assert_eq!(hits, [e.at], "{case:?} in {}: {found:?}", e.with.voice);
        let found = txt(&checker, &e.without.score());
        assert!(
            found.iter().all(|f| (f.0, f.1) != case),
            "{case:?} in {}: {found:?}",
            e.without.voice
        );
    }
}

/// The level of each case is its parameter's; TXT-MELISMA `long` counts only beyond its limit,
/// and its message names the attack.
#[test]
fn levels_and_messages() {
    use crate::lang::Lang;

    let pax = own("pax pax pax pax.");
    let score = texted(
        &pax,
        &[sung("d'2=0 e'4 f'4 g'2=1 a'2 r2 b'2=2 c''1=3")],
        &[],
    );
    let checker = Checker::new(&Profile::fux());
    let report = checker.check(&score);
    let txt: Vec<&Violation> = report
        .violations
        .iter()
        .filter(|v| v.rule.starts_with("TXT-"))
        .collect();
    let levels: Vec<(&str, &str, Level)> =
        txt.iter().map(|v| (v.rule, param(v), v.level)).collect();
    assert_eq!(
        levels,
        [
            ("TXT-AFTER-BLACK", "level", Level::Forbidden),
            ("TXT-REST", "clause", Level::P2),
        ]
    );
    // The main case of a rule says its title, the others what they are.
    assert_eq!(
        checker.message(txt[0], Lang::De),
        "Neue Silbe direkt nach Vierteln oder Achteln"
    );
    assert_eq!(
        checker.message(txt[1], Lang::En),
        "Rest inside a word or unit, or after too short a note: a rest after a word without \
         punctuation or a marked break (| or /)"
    );
    let score = texted(&pax, &[sung("d'2=0 d'2 e'1=1")], &[]);
    let report = checker.check(&score);
    let repeated = report
        .violations
        .iter()
        .find(|v| v.rule == "TXT-REPEATED-PITCH")
        .unwrap();
    assert_eq!(
        checker.message(repeated, Lang::De),
        "Tonwiederholung ohne neue Silbe"
    );
    // Eleven notes on Ký: only the eleventh costs, P1 with Fux's limit of 10.
    let kyrie = kyrie();
    let score = texted(
        &kyrie,
        &[sung("d'2=0 e'2 d'2 e'2 d'2 e'2 d'2 e'2 d'2 e'2 d'2 e'2=1")],
        &[],
    );
    let report = checker.check(&score);
    let long: Vec<&Violation> = report
        .violations
        .iter()
        .filter(|v| v.rule == "TXT-MELISMA")
        .collect();
    assert_eq!(long.len(), 1, "{long:?}");
    assert_eq!(
        (long[0].level, long[0].detail),
        (Level::P1, Detail::Count(11))
    );
    assert_eq!(
        checker.message(long[0], Lang::De),
        "Melisma an falscher Stelle oder zu lang: 11. Ton auf einer Silbe"
    );
    assert_eq!(
        checker.message(long[0], Lang::En),
        "Melisma misplaced or too long: note 11 on one syllable"
    );
    // With a limit of 8 the ninth note costs already, each further one again.
    let mut profile = Profile::fux();
    profile.apply_override("TXT-MELISMA.max_short=8").unwrap();
    profile.apply_override("TXT-MELISMA.long=P2").unwrap();
    let counts: Vec<(Level, Detail)> = Checker::new(&profile)
        .check(&score)
        .violations
        .iter()
        .filter(|v| v.rule == "TXT-MELISMA")
        .map(|v| (v.level, v.detail))
        .collect();
    assert_eq!(counts, [9, 10, 11].map(|n| (Level::P2, Detail::Count(n))));
}

/// Without a text in the score the TXT rules say nothing, nor on a voice that does not sing, on a
/// cantus firmus or on a voice the check does not count; neither on a chant cantus with its
/// syllables, which carries no text.
#[test]
fn txt_rules_are_silent_without_text() {
    let checker = Checker::new(&every_case());
    let silent = |score: &Score, targets: &[bool]| {
        checker
            .check_targets(score, targets)
            .violations
            .iter()
            .all(|v| !v.rule.starts_with("TXT-"))
    };
    let mut cases = 0;
    for e in examples() {
        for setting in [&e.with, &e.without] {
            let texted = setting.score();
            cases += txt(&checker, &texted).len();
            let mut score = texted.clone();
            score.text = None;
            assert!(silent(&score, &[true]), "{}", setting.voice);
            let mut score = texted.clone();
            score.text.as_mut().unwrap().voices = vec![false];
            assert!(silent(&score, &[true]), "{}", setting.voice);
            let mut score = texted.clone();
            score.voices[0].role = Role::CantusFirmus;
            assert!(silent(&score, &[true]), "{}", setting.voice);
            assert!(silent(&texted, &[false]), "{}", setting.voice);
        }
    }
    // The same scores with their text have cases.
    assert!(cases > 100, "{cases}");

    // A chant as cantus firmus, its syllables under the notes, with a voice above it.
    let chant = parse_gabc(include_str!(
        "../../../../tests/fixtures/chant/ave-maris-stella.gabc"
    ))
    .unwrap();
    let mode = chant.mode().unwrap();
    let cantus = chant.cantus_notes();
    assert!(cantus.iter().all(|n| n.lyric.is_none()));
    assert!(cantus.iter().filter(|n| n.syllable.is_some()).count() > 10);
    let above: Vec<Note> = cantus
        .iter()
        .map(|n| Note::new(n.pitch.unwrap().transpose(Interval::OCTAVE), n.duration))
        .collect();
    let mut score = Score::new(
        mode,
        vec![
            Voice::new(
                VoiceType::Soprano,
                Role::Counterpoint(Species::First),
                above,
            ),
            Voice::new(VoiceType::Alto, Role::CantusFirmus, cantus),
        ],
    );
    score.validate().unwrap();
    assert!(silent(&score, &[true, true]));
    // With a text that only the cantus sings, and with lyrics on its notes.
    let (text, _) = resolve("kyrie", &TextOptions::default()).unwrap();
    for (i, note) in score.voices[1].notes.iter_mut().take(7).enumerate() {
        note.lyric = Some(Lyric::new(i));
    }
    score.text = Some(ScoreText {
        text: Arc::new(text),
        voices: vec![false, true],
    });
    score.validate().unwrap();
    assert!(silent(&score, &[true, true]));
}

/// A repeated note in a voice that sings carries a new syllable: MEL-REPEAT takes the level of
/// `free_texted` there instead of `free`.
#[test]
fn mel_repeat_free_texted() {
    let pax = own("pax pax pax pax.");
    let repeats = |profile: &Profile, score: &Score| -> Vec<Level> {
        Checker::new(profile)
            .check(score)
            .violations
            .iter()
            .filter(|v| v.rule == "MEL-REPEAT")
            .map(|v| v.level)
            .collect()
    };
    let fux = Profile::fux();
    let mut strict = Profile::fux();
    strict.apply_override("MEL-REPEAT.free_texted=P2").unwrap();
    assert_eq!(fux.settings("MEL-REPEAT").level("free_texted"), Level::Off);
    let sings = texted(&pax, &[sung("d'2=0 d'2=1 e'1=2")], &[]);
    let mut untexted = sings.clone();
    untexted.text = None;
    let mut silent = sings.clone();
    silent.text.as_mut().unwrap().voices = vec![false];
    assert_eq!(repeats(&fux, &untexted), [Level::P1]);
    assert_eq!(repeats(&fux, &silent), [Level::P1]);
    assert_eq!(repeats(&fux, &sings), []);
    assert_eq!(repeats(&strict, &sings), [Level::P2]);
    assert_eq!(repeats(&strict, &untexted), [Level::P1]);
    // The anticipation keeps its own level.
    let anticipation = texted(&pax, &[sung("d'2=0 e'4 f'4 f'2 g'2=1")], &[]);
    let mut untexted = anticipation.clone();
    untexted.text = None;
    assert_eq!(repeats(&strict, &anticipation), [Level::P2]);
    assert_eq!(repeats(&fux, &anticipation), [Level::P2]);
    assert_eq!(repeats(&fux, &untexted), [Level::P2]);
    // Voices of the species keep their parameters.
    let mut first = sings.clone();
    first.voices[0].role = Role::Counterpoint(Species::First);
    assert_eq!(repeats(&strict, &first), [Level::P1]);
}

/// A voice that sings rests where its text lets it: FREE-REST takes the level of `texted` there
/// instead of `level`.
#[test]
fn free_rest_texted() {
    let pax = own("pax pax, pax pax.");
    let rests = |profile: &Profile, score: &Score| -> Vec<Level> {
        Checker::new(profile)
            .check(score)
            .violations
            .iter()
            .filter(|v| v.rule == "FREE-REST")
            .map(|v| v.level)
            .collect()
    };
    let mut profile = Profile::fux();
    profile.apply_override("FREE-REST.texted=P2").unwrap();
    let sings = texted(&pax, &[sung("d'1=0 e'1=1 r2 f'2=2 g'1=3")], &[]);
    let mut untexted = sings.clone();
    untexted.text = None;
    let mut silent = sings.clone();
    silent.text.as_mut().unwrap().voices = vec![false];
    assert_eq!(rests(&profile, &sings), [Level::P2]);
    assert_eq!(rests(&profile, &untexted), [Level::P1]);
    assert_eq!(rests(&profile, &silent), [Level::P1]);
    // Voices of the species have no FREE-REST.
    let mut first = sings.clone();
    first.voices[0].role = Role::Counterpoint(Species::First);
    assert_eq!(rests(&profile, &first), []);
}

/// Fux's profile with random levels for the cases of the TXT rules and random limits of
/// TXT-MELISMA `long`.
fn random_profile(rng: &mut Rng) -> Profile {
    const LEVELS: [&str; 5] = ["aus", "P1", "P2", "P3", "V"];
    let mut profile = Profile::fux();
    for rule in all_rules() {
        let id = rule.info().id;
        if !id.starts_with("TXT-") {
            continue;
        }
        for param in rule.info().params.iter() {
            let value = match param.kind {
                ParamKind::Level => LEVELS[below(rng, LEVELS.len())].to_string(),
                ParamKind::Int => (1 + below(rng, 5)).to_string(),
                ParamKind::Bool => continue,
            };
            profile
                .apply_override(&format!("{id}.{}={value}", param.name))
                .unwrap();
        }
    }
    profile
}

/// Random internal cadences, on degrees of the Dorian mode with a clausula.
fn random_cadences(rng: &mut Rng, measures: usize) -> Vec<Cadence> {
    const TARGETS: [Step; 5] = [Step::D, Step::A, Step::F, Step::G, Step::E];
    let mut cadences = Vec::new();
    for measure in 1..measures {
        if below(rng, 2) == 0 {
            cadences.push(Cadence {
                measure,
                target: TARGETS[below(rng, TARGETS.len())],
            });
        }
    }
    cadences
}

/// The checker prices the underlay the dynamic program finds exactly as the dynamic program does,
/// and finds no prohibition in it: the TXT rules report the model's cases with the same levels.
#[test]
fn dp_cost_equals_checker_cost() {
    let mut rng = Rng::new(17);
    let meter = Meter::ALLA_BREVE;
    let (mut underlaid, mut priced) = (0, 0);
    for case in 0..400 {
        let text = match case % 5 {
            0 => kyrie(),
            _ => random_text(&mut rng, 9),
        };
        let measures = 2 + below(&mut rng, 7);
        let notes = random_voice(&mut rng, measures, case % 3 != 0, true);
        let cadences = random_cadences(&mut rng, measures);
        let profile = match case % 2 {
            0 => Profile::fux(),
            _ => random_profile(&mut rng),
        };
        let checker = Checker::new(&profile);
        let levels = Levels::from(&checker);
        let mode = "dorian".parse().unwrap();
        let model = Model::new(&text, mode, meter, &Goal::of(&cadences, meter));
        let Some(path) = model.best(&levels, &notes) else {
            continue;
        };
        underlaid += 1;
        let mut voice = notes.clone();
        path.apply(&mut voice);
        let mut score = texted(&text, &[voice], &[]);
        score.cadences = cadences;
        let txt: Vec<Violation> = checker
            .check(&score)
            .violations
            .into_iter()
            .filter(|v| v.rule.starts_with("TXT-"))
            .collect();
        let context = || format!("case {case}: {} {txt:?}", text.render(true, true));
        assert!(txt.iter().all(|v| !v.is_forbidden()), "{}", context());
        let cost: u32 = txt.iter().map(|v| v.level.cost()).sum();
        assert_eq!(cost, path.cost, "{}", context());
        priced += usize::from(cost > 0);
    }
    assert!(
        underlaid >= 120 && priced >= 80,
        "{underlaid} underlaid, {priced} with a cost"
    );
}
