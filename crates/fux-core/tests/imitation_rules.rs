//! The rules of imitation (IMI-*) on the corpora and on imitative settings: without a plan they
//! report nothing on Fux's examples and Palestrina's movements, so the calibrations stay as they
//! were; on a setting whose entries are falsified they report each departure at its tick, and the
//! checker finds in the windows of the search and in growing prefixes of the voices exactly what
//! it finds in the whole score. The same holds for the parameters that judge settings without a
//! plan (phase 14), which the profile of a mass switches on.

mod common;

use std::collections::BTreeSet;
use std::sync::Arc;

use common::{
    assert_prefixes_collect, assert_windows_partition, calibration_pieces, corpus_present,
};

use fux_core::generate::Rng;
use fux_core::rhythm::{self, Place};
use fux_core::rules::{Analysis, Checker, Detail, Level, ParamKind, Profile, Window, all_rules};
use fux_core::text::{Text, TextOptions, ordinary};
use fux_core::{
    Cadence, Duration, Lyric, Note, Pitch, Role, Score, ScoreEntry, ScoreImitation, ScorePoint,
    ScoreText, Species, Step, Voice, VoiceType, parse_lilypond,
};

/// A checker of the IMI rules alone, with every case of a plan switched on: those free by default
/// cost P1. The parameters for settings without a plan stay off, as in the standard profile.
fn imi_rules() -> Checker {
    Checker::new(&imi_profile(false))
}

/// The same with the parameters for settings without a plan switched on too, at P1.
fn imi_rules_without_a_plan() -> Checker {
    Checker::new(&imi_profile(true))
}

/// Fux's profile with only the IMI rules, every case of them switched on: those free by default
/// cost P1, the parameters for settings without a plan (`unplanned_*`) only with `unplanned`.
fn imi_profile(unplanned: bool) -> Profile {
    let mut profile = Profile::fux();
    for rule in all_rules() {
        let id = rule.info().id;
        for param in rule.info().params.iter() {
            if param.kind != ParamKind::Level {
                continue;
            }
            let value = if !id.starts_with("IMI-") {
                "aus"
            } else if param.name.starts_with("unplanned_") && !unplanned {
                continue;
            } else if profile.settings(id).level(param.name) == Level::Off {
                "P1"
            } else {
                continue;
            };
            profile
                .apply_override(&format!("{id}.{}={value}", param.name))
                .unwrap();
        }
    }
    profile
}

/// The cases of the IMI rules in a score: (rule, parameter, tick, voice, marked note).
fn imi_cases(
    checker: &Checker,
    score: &Score,
) -> Vec<(&'static str, &'static str, u32, usize, usize)> {
    checker
        .check(score)
        .violations
        .iter()
        .filter(|v| v.rule.starts_with("IMI-"))
        .map(|v| {
            let Detail::Param(param) = v.detail else {
                panic!("{}: {:?}", v.rule, v.detail);
            };
            (v.rule, param, v.tick, v.voice, v.note)
        })
        .collect()
}

/// Neither corpus has a plan of imitation, so the IMI rules, every case of a plan switched on,
/// report nothing on Fux's 123 examples and Palestrina's 1156 movements: the calibrations of
/// both corpora (`calibration.rs`, `EXCEPTIONS` in `palestrina.rs`) stay as they were. The
/// parameters for settings without a plan, off in the standard profile, judge the corpus only in
/// the profile of a mass (`unplanned_held_and_thin_in_the_corpus` in `palestrina.rs`).
#[test]
fn imi_rules_are_silent_on_the_corpus() {
    if !corpus_present() {
        return;
    }
    let checker = imi_rules();
    let examples = fux_core::corpus::fux_examples();
    assert_eq!(examples.len(), 123);
    for example in &examples {
        let score = example.score();
        assert!(score.imitation.is_none());
        assert_eq!(imi_cases(&checker, &score), [], "Fig. {}", example.figure);
    }
    let movements = calibration_pieces();
    assert_eq!(movements.len(), 1156);
    for (name, piece) in &movements {
        assert!(piece.score.imitation.is_none(), "{name}");
        assert_eq!(imi_cases(&checker, &piece.score), [], "{name}");
    }
}

/// Notes in LilyPond notation, each with the index of the syllable it begins after `=`.
fn sung(src: &str) -> Vec<Note> {
    let mut lyrics = Vec::new();
    let tokens: Vec<&str> = src
        .split_whitespace()
        .map(|token| match token.split_once('=') {
            Some((note, syllable)) => {
                lyrics.push(Some(Lyric::new(syllable.parse().unwrap())));
                note
            }
            None => {
                lyrics.push(None);
                token
            }
        })
        .collect();
    let mut notes = parse_lilypond(&tokens.join(" "), Duration::HALF).unwrap();
    assert_eq!(notes.len(), lyrics.len(), "{src}");
    for (note, lyric) in notes.iter_mut().zip(lyrics) {
        note.lyric = lyric;
    }
    notes
}

/// A head in LilyPond notation as the plan keeps it: pitch and value, tied notes merged.
fn head(src: &str) -> Vec<(Pitch, u32)> {
    let mut out: Vec<(Pitch, u32)> = Vec::new();
    let mut tied = false;
    for note in parse_lilypond(src, Duration::HALF).unwrap() {
        match out.last_mut() {
            Some(last) if tied => last.1 += note.duration.ticks(),
            _ => out.push((note.pitch.unwrap(), note.duration.ticks())),
        }
        tied = note.tie;
    }
    out
}

fn kyrie() -> Text {
    ordinary::section("kyrie", &TextOptions::default())
        .unwrap()
        .clone()
}

/// Free voices from the soprano down that all sing `text`, with internal cadences.
fn texted(text: &Text, voices: Vec<Vec<Note>>, cadences: Vec<Cadence>) -> Score {
    const TYPES: [VoiceType; 4] = [
        VoiceType::Soprano,
        VoiceType::Alto,
        VoiceType::Tenor,
        VoiceType::Bass,
    ];
    let voices: Vec<Voice> = voices
        .into_iter()
        .zip(TYPES)
        .map(|(notes, voice_type)| Voice::new(voice_type, Role::Counterpoint(Species::Free), notes))
        .collect();
    let mut score = Score::new("dorian".parse().unwrap(), voices);
    score.cadences = cadences;
    score.text = Some(ScoreText {
        text: Arc::new(text.clone()),
        voices: vec![true; score.voices.len()],
    });
    score
}

const SOPRANO: &str =
    "a'1=0 a'2=1 b'2=2 c''2=3 b'2 a'1=4 g'1=5 a'1=6 r1 r1 a'1=3 c''2=4 b'2=5 a'1=6 a'1";
const ALTO: &str =
    "r1 d'1=0 d'2=1 e'2=2 f'2=3 e'2 d'2=4 c'2=5 d'1=6 r1 d'1=3 f'2=4 e'2=5 d'1=6 c'1 d'1";
const TENOR: &str = "r1 r1 a1=0 a2=1 b2=2 c'2=3 b2 a1=4 g1=5 a1=6 r1 d1=3 f2=4 e2=5 d1=6";

/// Kyrie I in three voices, twelve measures, as the plan of [`kyrie_plan`] has it: the soprano
/// leads, the alto answers a measure later at the fifth below, the tenor a measure after it at
/// the octave; after the cadence on D in measure 8 the alto leads the tail e-lé-i-son after a
/// rest, the soprano and the tenor follow it.
fn kyrie_setting(soprano: &str, alto: &str, tenor: &str) -> Score {
    let mut score = texted(
        &kyrie(),
        vec![sung(soprano), sung(alto), sung(tenor)],
        vec![Cadence {
            measure: 7,
            target: Step::D,
        }],
    );
    score.imitation = Some(Arc::new(kyrie_plan()));
    score.validate().unwrap();
    score
}

fn kyrie_plan() -> ScoreImitation {
    let entry = |voice, start, rest, notes: &str| ScoreEntry {
        voice,
        start,
        rest,
        notes: head(notes),
    };
    ScoreImitation {
        points: vec![
            ScorePoint {
                words: 0..7,
                entries: vec![
                    entry(0, 0, 0, "a'1 a'2 b'2 c''2 b'2 a'1"),
                    entry(1, 8, 0, "d'1 d'2 e'2 f'2 e'2 d'2"),
                    entry(2, 16, 0, "a1 a2 b2 c'2 b2 a1"),
                ],
                goal: Some(7),
            },
            ScorePoint {
                words: 3..7,
                entries: vec![
                    entry(1, 56, 4, "d'1 f'2 e'2 d'1"),
                    entry(0, 64, 4, "a'1 c''2 b'2 a'1"),
                    entry(2, 72, 4, "d1 f2 e2 d1"),
                ],
                goal: None,
            },
        ],
    }
}

/// The planned setting has no case; falsified, each departure is reported at its tick, in its
/// voice, on its note: a head note moved a step, a value cut short, a syllable moved to the next
/// note, a rest shortened to a quarter. Window by window and prefix by prefix the checker finds
/// the same, with the IMI rules alone and with all of Fux's profile.
#[test]
fn falsified_imitations_are_reported_at_their_tick() {
    let imi = imi_rules();
    let fux = Checker::new(&Profile::fux());
    let planned = kyrie_setting(SOPRANO, ALTO, TENOR);
    assert_eq!(imi_cases(&imi, &planned), []);
    type Case = (&'static str, &'static str, u32, usize, usize);
    let falsified: [(&str, Score, Vec<Case>); 4] = [
        // B3 in the tenor's head becomes C4.
        (
            "a moved head note",
            kyrie_setting(
                SOPRANO,
                ALTO,
                "r1 r1 a1=0 a2=1 c'2=2 c'2=3 b2 a1=4 g1=5 a1=6 r1 d1=3 f2=4 e2=5 d1=6",
            ),
            vec![("IMI-ENTRY", "level", 28, 2, 4)],
        ),
        // The soprano's C5 in the tail, a half, becomes a quarter and a passing D5.
        (
            "a changed value",
            kyrie_setting(
                "a'1=0 a'2=1 b'2=2 c''2=3 b'2 a'1=4 g'1=5 a'1=6 r1 r1 a'1=3 c''4=4 d''4 b'2=5 a'1=6 a'1",
                ALTO,
                TENOR,
            ),
            vec![("IMI-ENTRY", "rhythm", 74, 0, 11)],
        ),
        // The tenor sings i of e-lé-i-son on the head's last note instead of the one before.
        (
            "a moved syllable",
            kyrie_setting(
                SOPRANO,
                ALTO,
                "r1 r1 a1=0 a2=1 b2=2 c'2=3 b2 a1=4 g1=5 a1=6 r1 d1=3 f2=4 e2 d1=5",
            ),
            vec![
                ("IMI-ENTRY", "text", 84, 2, 13),
                ("IMI-ENTRY", "text", 88, 2, 14),
            ],
        ),
        // The tenor holds its A3 into the rest before the tail, which keeps a quarter.
        (
            "a shortened rest",
            kyrie_setting(
                SOPRANO,
                ALTO,
                "r1 r1 a1=0 a2=1 b2=2 c'2=3 b2 a1=4 g1=5 a1~=6 a2. r4 d1=3 f2=4 e2=5 d1=6",
            ),
            vec![("IMI-REST", "level", 72, 2, 12)],
        ),
    ];
    assert_windows_partition(&imi, &planned, "planned, IMI rules");
    assert_prefixes_collect(&imi, &planned, "planned, IMI rules");
    assert_windows_partition(&fux, &planned, "planned, Fux");
    assert_prefixes_collect(&fux, &planned, "planned, Fux");
    for (name, score, expected) in &falsified {
        assert_eq!(&imi_cases(&imi, score), expected, "{name}");
        // At the levels of the standard profile too.
        let report = fux.check(score);
        for &(rule, param, tick, voice, note) in expected {
            let found = report
                .violations
                .iter()
                .find(|v| (v.rule, v.tick, v.voice, v.note) == (rule, tick, voice, note))
                .unwrap_or_else(|| panic!("{name}: {rule}"));
            let level = Profile::fux().settings(rule).level(param);
            assert_eq!(found.level, level, "{name}");
        }
        assert_windows_partition(&imi, score, &format!("{name}, IMI rules"));
        assert_prefixes_collect(&imi, score, &format!("{name}, IMI rules"));
        assert_windows_partition(&fux, score, &format!("{name}, Fux"));
        assert_prefixes_collect(&fux, score, &format!("{name}, Fux"));
    }
    // In voices that do not sing, as in the search, the moved syllable is no case.
    let mut unsung = falsified[2].1.clone();
    unsung.text.as_mut().unwrap().voices = vec![false; 3];
    assert_eq!(imi_cases(&imi, &unsung), []);
}

/// Free voices from the soprano down, in the Dorian mode, without a text and without a plan of
/// imitation, as a part of a mass that begins together.
fn planless(voices: [&str; 4]) -> Score {
    const TYPES: [VoiceType; 4] = [
        VoiceType::Soprano,
        VoiceType::Alto,
        VoiceType::Tenor,
        VoiceType::Bass,
    ];
    let voices = voices
        .into_iter()
        .zip(TYPES)
        .map(|(notes, voice_type)| {
            let notes = parse_lilypond(notes, Duration::HALF).unwrap();
            Voice::new(voice_type, Role::Counterpoint(Species::Free), notes)
        })
        .collect();
    let score = Score::new("dorian".parse().unwrap(), voices);
    score.validate().unwrap();
    score
}

/// Ten measures in four voices that begin together, a chord on every downbeat, without ties and
/// rests: D minor, F, C, D minor, G, D minor twice, C, A minor, D minor.
const PLAIN: [&str; 4] = [
    "a'1 a'1 g'1 f'1 d'1 f'1 f'1 e'1 e'1 d'1",
    "f'1 f'1 e'1 d'1 b1 d'1 d'1 c'1 c'1 a1",
    "d'1 c'1 c'1 a1 g1 a1 a1 g1 a1 f1",
    "d1 f1 c1 d1 g,1 d1 d1 c1 a,1 d1",
];

/// The setting without a plan has no case of the IMI rules; falsified, each case of the
/// parameters for settings without a plan (phase 14) is reported at its tick, in its voice, on
/// its note: the bass held over into a 6/4 chord whose fourth is struck against it and leaving
/// by a leap, the bass held under a soprano that leaps into a seventh against it, soprano and
/// alto resting together. Window by window and prefix by prefix the checker finds the same, with
/// the IMI rules alone and with Fux's profile and the overlay of a mass, which prices the cases
/// as their counterparts with a plan.
#[test]
fn falsified_scores_reported_at_their_tick() {
    let imi = imi_rules_without_a_plan();
    let mass = fux_core::mass::profile(&Profile::fux());
    let fux = Checker::new(&mass);
    let plain = planless(PLAIN);
    assert_eq!(imi_cases(&imi, &plain), []);
    type Case = (&'static str, &'static str, u32, usize, usize);
    let falsified: [(&str, Score, Vec<Case>); 3] = [
        // The bass holds D3 into the G chord, under the tenor's G3, and leaps down to G2.
        (
            "a struck fourth",
            planless([
                PLAIN[0],
                PLAIN[1],
                PLAIN[2],
                "d1 f1 c1 d1~ d2 g,2 d1 d1 c1 a,1 d1",
            ]),
            vec![("IMI-HELD", "unplanned_struck", 36, 2, 4)],
        ),
        // The bass holds D3 into measure 7, consonant on its downbeat; the soprano leaps from F4
        // into C5, a seventh above it.
        (
            "a leap into a seventh",
            planless([
                "a'1 a'1 g'1 f'1 d'1 f'1 f'4 c''4 b'4 a'4 e'1 e'1 d'1",
                PLAIN[1],
                PLAIN[2],
                "d1 f1 c1 d1 g,1 d1~ d1 c1 a,1 d1",
            ]),
            vec![("IMI-HELD", "unplanned_passing", 50, 0, 7)],
        ),
        // Soprano and alto rest in measure 9.
        (
            "half of the choir resting",
            planless([
                "a'1 a'1 g'1 f'1 d'1 f'1 f'1 e'1 r1 d'1",
                "f'1 f'1 e'1 d'1 b1 d'1 d'1 c'1 r1 a1",
                PLAIN[2],
                PLAIN[3],
            ]),
            vec![("IMI-SILENCE", "unplanned_thin", 64, 0, 8)],
        ),
    ];
    for (checker, name) in [(&imi, "IMI rules"), (&fux, "Fux and the mass")] {
        assert_windows_partition(checker, &plain, &format!("plain, {name}"));
        assert_prefixes_collect(checker, &plain, &format!("plain, {name}"));
    }
    for (name, score, expected) in &falsified {
        assert_eq!(&imi_cases(&imi, score), expected, "{name}");
        // With a plan without points the same; the standard profile is silent.
        let mut empty = score.clone();
        empty.imitation = Some(Arc::new(ScoreImitation { points: Vec::new() }));
        assert_eq!(&imi_cases(&imi, &empty), expected, "{name}");
        assert_eq!(imi_cases(&imi_rules(), score), [], "{name}");
        // At the levels of the mass, those of the counterparts with a plan.
        let report = fux.check(score);
        for &(rule, param, tick, voice, note) in expected {
            let found = report
                .violations
                .iter()
                .find(|v| (v.rule, v.tick, v.voice, v.note) == (rule, tick, voice, note))
                .unwrap_or_else(|| panic!("{name}: {rule}"));
            let counterpart = param.trim_start_matches("unplanned_");
            assert_eq!(found.level, mass.settings(rule).level(param), "{name}");
            assert_eq!(
                found.level,
                mass.settings(rule).level(counterpart),
                "{name}"
            );
        }
        for (checker, label) in [(&imi, "IMI rules"), (&fux, "Fux and the mass")] {
            assert_windows_partition(checker, score, &format!("{name}, {label}"));
            assert_prefixes_collect(checker, score, &format!("{name}, {label}"));
        }
    }
}

/// A level for the whole rule, `--rule IMI-HELD=V` or `IMI-SILENCE=P1`, judges scores without a
/// plan as it did before the parameters for them came (phase 14, D15): it leaves those
/// parameters off, so the held fourth and the half choir of the falsified planless settings cost
/// nothing, as on every score of the corpora. The profile of a mass judges them, and a level for
/// the whole rule after it keeps the mass's levels for them, but the rule switched off
/// (`--rule IMI-HELD=aus`) is off for them too; an override that names them sets them.
#[test]
fn whole_rule_levels_leave_planless_scores_alone() {
    let held = planless([
        PLAIN[0],
        PLAIN[1],
        PLAIN[2],
        "d1 f1 c1 d1~ d2 g,2 d1 d1 c1 a,1 d1",
    ]);
    let thin = planless([
        "a'1 a'1 g'1 f'1 d'1 f'1 f'1 e'1 r1 d'1",
        "f'1 f'1 e'1 d'1 b1 d'1 d'1 c'1 r1 a1",
        PLAIN[2],
        PLAIN[3],
    ]);
    let with = |base: Profile, overrides: &[&str]| {
        let mut profile = base;
        for spec in overrides {
            profile.apply_override(spec).unwrap();
        }
        Checker::new(&profile)
    };
    let levels = ["IMI-HELD=V", "IMI-SILENCE=P1"];
    let fux = with(Profile::fux(), &levels);
    assert_eq!(imi_cases(&fux, &held), []);
    assert_eq!(imi_cases(&fux, &thin), []);
    let mass = with(fux_core::mass::profile(&Profile::fux()), &levels);
    assert_eq!(
        imi_cases(&mass, &held),
        [("IMI-HELD", "unplanned_struck", 36, 2, 4)]
    );
    assert_eq!(
        imi_cases(&mass, &thin),
        [("IMI-SILENCE", "unplanned_thin", 64, 0, 8)]
    );
    let off = with(
        fux_core::mass::profile(&Profile::fux()),
        &["IMI-HELD=aus", "IMI-SILENCE=aus"],
    );
    assert_eq!(imi_cases(&off, &held), []);
    assert_eq!(imi_cases(&off, &thin), []);
    let named = with(Profile::fux(), &["IMI-HELD.unplanned_struck=P2"]);
    assert_eq!(
        imi_cases(&named, &held),
        [("IMI-HELD", "unplanned_struck", 36, 2, 4)]
    );
    let report = named.check(&held);
    let found = report.violations.iter().find(|v| v.rule == "IMI-HELD");
    assert_eq!(found.map(|v| v.level), Some(Level::P2));
}

/// What the IMI rules add to the checks of the search: the planned Kyrie as the search sees it
/// (voices without lyrics, written up to each onset, checked since the onset before), with Fux's
/// profile, with its plan and without it; the rounds of the two alternate, so that a busy machine
/// slows both alike, and the least of fifteen counts. A check takes about 1.25 times as long with
/// the plan (8.2 µs against 6.6 µs in phase 14a, as before it): IMI-HELD and IMI-SILENCE take
/// 1.13 µs of the difference with a plan, and without one they return at once (0.02 µs, the
/// levels of their parameters for settings without a plan looked up). The bound of 1.2 set in
/// phase 13, when the rules took about 5 %, no longer held before phase 14 (1.25 to 1.27):
/// `cargo test --release -p fux-core --test imitation_rules -- --ignored --nocapture cost`.
#[test]
#[ignore = "timing: cargo test --release -- --ignored"]
fn imi_rules_cost_little() {
    let fux = Checker::new(&Profile::fux());
    let mut planned = kyrie_setting(SOPRANO, ALTO, TENOR);
    planned.text.as_mut().unwrap().voices = vec![false; 3];
    let mut unplanned = planned.clone();
    unplanned.imitation = None;
    let ticks: Vec<u32> = Analysis::new(&planned)
        .slices
        .iter()
        .map(|s| s.tick)
        .collect();
    // The prefixes with their analyses and windows, built once.
    let steps = |score: &Score| -> Vec<(Score, Analysis, Window)> {
        ticks
            .iter()
            .enumerate()
            .map(|(k, &tick)| {
                let mut part = score.clone();
                for voice in &mut part.voices {
                    let keep = voice
                        .timed()
                        .take_while(|(start, _)| *start <= tick)
                        .count();
                    voice.notes.truncate(keep);
                }
                let analysis = Analysis::new(&part);
                let from = if k == 0 { 0 } else { ticks[k - 1] + 1 };
                (part, analysis, Window::partial(from, tick))
            })
            .collect()
    };
    let targets = vec![true; 3];
    let round = |steps: &[(Score, Analysis, Window)]| {
        let started = std::time::Instant::now();
        let mut found = Vec::new();
        for _ in 0..500 {
            for (score, analysis, window) in steps {
                found.clear();
                fux.check_window(score, analysis, *window, &targets, &mut found);
            }
        }
        started.elapsed().as_secs_f64() / (500 * steps.len()) as f64
    };
    let (with, without) = (steps(&planned), steps(&unplanned));
    let (mut planned, mut unplanned) = (f64::INFINITY, f64::INFINITY);
    for _ in 0..15 {
        planned = planned.min(round(&with));
        unplanned = unplanned.min(round(&without));
    }
    let factor = planned / unplanned;
    println!(
        "a check of the search: {:.2} µs without the plan, {:.2} µs with it: {factor:.3} times",
        unplanned * 1e6,
        planned * 1e6
    );
    assert!(factor <= 1.3, "{factor:.3} times");
}

fn below(rng: &mut Rng, n: usize) -> usize {
    (rng.next_u64() % n as u64) as usize
}

/// A random voice of `measures` measures in Palestrina's measure rhythms, with rests and ties;
/// its last measure holds one note.
fn random_voice(rng: &mut Rng, measures: usize) -> Vec<Note> {
    const MOVES: [i32; 20] = [
        0, 0, 0, 1, 1, 1, 1, -1, -1, -1, -1, 2, -2, 2, -2, 3, -3, 4, -5, 7,
    ];
    let mut notes: Vec<Note> = Vec::new();
    let mut tied = false;
    let mut diatonic = 29;
    for m in 0..measures {
        let last = m + 1 == measures;
        let place = if last { Place::Last } else { Place::Inner };
        let patterns: Vec<&str> = rhythm::patterns(Species::Free, place, true)
            .iter()
            .map(|(pattern, _)| *pattern)
            .filter(|p| p.starts_with('⁀') == tied)
            .filter(|p| !last || matches!(*p, "1" | "⁀1"))
            .collect();
        let pick = below(rng, patterns.len()).min(below(rng, patterns.len()));
        for token in rhythm::tokens(patterns[pick]) {
            let duration = Duration::from_ticks(token.ticks);
            if token.tied_in {
                let held = notes.last_mut().unwrap();
                held.tie = true;
                let note = Note::new(held.pitch.unwrap(), duration);
                notes.push(note);
            } else if token.rest {
                notes.push(Note::rest(duration));
            } else {
                diatonic = (diatonic + MOVES[below(rng, MOVES.len())]).clamp(24, 36);
                notes.push(Note::new(Pitch::from_diatonic(diatonic), duration));
            }
        }
        tied = patterns[pick].ends_with('⁀');
    }
    notes
}

/// Lyrics on most attacks, mostly the next syllable, now and then a restart or one that is not
/// in the text.
fn random_lyrics(rng: &mut Rng, notes: &mut [Note], syllables: usize) {
    let mut next = 0;
    let mut tied = false;
    for note in notes {
        let attack = note.pitch.is_some() && !tied;
        tied = note.tie;
        if !attack || below(rng, 3) == 0 {
            continue;
        }
        note.lyric = Some(match below(rng, 12) {
            0 => Lyric::UNKNOWN,
            1 => Lyric::new(below(rng, syllables)),
            _ => {
                let y = next % syllables;
                next = y + 1;
                Lyric::new(y)
            }
        });
    }
}

/// A random plan for the voices: up to three points of up to four entries, each head read from
/// its voice where an attack begins it, now and then with a pitch moved a step, a value changed
/// or the start moved off it, so that every rule finds cases.
fn random_plan(rng: &mut Rng, score: &Score) -> ScoreImitation {
    let syllables = score.text.as_ref().unwrap().text.syllables.len() as u16;
    let measures = score.measure_count();
    let mut points: Vec<ScorePoint> = Vec::new();
    let mut leader = 0;
    for _ in 0..1 + below(rng, 3) {
        let mut entries: Vec<ScoreEntry> = (0..1 + below(rng, 4))
            .filter_map(|_| {
                let voice = below(rng, score.voices.len());
                let line = score.voices[voice].soundings();
                let first = below(rng, line.len().max(1));
                let length = 1 + below(rng, 8);
                let mut notes: Vec<(Pitch, u32)> = line
                    .iter()
                    .skip(first)
                    .take(length)
                    .map(|s| (s.pitch, s.end - s.start))
                    .collect();
                if notes.is_empty() {
                    return None;
                }
                let mut start = line[first].start;
                match below(rng, 8) {
                    0 => {
                        let k = below(rng, notes.len());
                        let moved = notes[k].0.diatonic() + [1, -1][below(rng, 2)];
                        notes[k].0 = Pitch::from_diatonic(moved);
                    }
                    1 => {
                        let k = below(rng, notes.len());
                        notes[k].1 = (notes[k].1 + 2 * below(rng, 3) as u32)
                            .saturating_sub(2)
                            .max(1);
                    }
                    2 => start += 2 * below(rng, 3) as u32,
                    _ => {}
                }
                let rest = [0, 4, 8][below(rng, 3)].min(start);
                Some(ScoreEntry {
                    voice,
                    start,
                    rest,
                    notes,
                })
            })
            .collect();
        entries.sort_by_key(|e| e.start);
        let Some(first) = entries.first() else {
            continue;
        };
        if first.start < leader {
            continue;
        }
        leader = first.start;
        let from = below(rng, syllables as usize) as u16;
        let goal = (below(rng, 2) == 0).then(|| below(rng, measures));
        points.push(ScorePoint {
            words: from..syllables,
            entries,
            goal,
        });
    }
    ScoreImitation { points }
}

/// Random scores of two to four voices with random lyrics and random plans: every case of the
/// IMI rules, in every window and every prefix exactly once, with the IMI rules alone and with
/// Fux's profile.
#[test]
fn random_imitations_check_window_by_window() {
    let imi = imi_rules();
    let fux = Checker::new(&Profile::fux());
    let text = kyrie();
    let mut rng = Rng::new(37);
    let mut cases = 0;
    let mut kinds: BTreeSet<(&str, &str)> = BTreeSet::new();
    for case in 0..200 {
        let measures = 3 + below(&mut rng, 14);
        let voices = (0..2 + below(&mut rng, 3))
            .map(|_| {
                let mut notes = random_voice(&mut rng, measures);
                random_lyrics(&mut rng, &mut notes, text.syllables.len());
                notes
            })
            .collect();
        let mut score = texted(&text, voices, Vec::new());
        // Now and then a voice that does not sing.
        for sings in &mut score.text.as_mut().unwrap().voices {
            *sings = below(&mut rng, 5) != 0;
        }
        score.imitation = Some(Arc::new(random_plan(&mut rng, &score)));
        let found = imi_cases(&imi, &score);
        cases += found.len();
        kinds.extend(found.iter().map(|f| (f.0, f.1)));
        let name = format!("case {case}: {}", score.imitation.as_ref().unwrap().spec());
        assert_windows_partition(&imi, &score, &name);
        assert_prefixes_collect(&imi, &score, &name);
        if case % 10 == 0 {
            assert_windows_partition(&fux, &score, &name);
            assert_prefixes_collect(&fux, &score, &name);
        }
    }
    // Random voices hardly sing a plain head; the examples of the rules' tests have one.
    println!("{cases} cases of {} kinds: {kinds:?}", kinds.len());
    assert!(cases >= 2000, "{cases} cases");
    assert!(kinds.len() >= 22, "{kinds:?}");
}
