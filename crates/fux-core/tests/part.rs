//! Parts of a mass section (phase 14, docs/regeln.md § 19): free settings that end on the fifth
//! or the fourth, begin after a breath or a measure's rest, open with a chord in every voice, and
//! stop when they are cancelled. Every part is a texted setting like any other (`assert_texted`,
//! and `assert_imitated` with imitation); besides, its bass ends on the planned ending, no voice
//! sounds before its start, and its opening chord sounds as planned. The matrix of endings in
//! every mode runs in release:
//! `cargo test --release -p fux-core --test part -- --ignored --nocapture`.

mod common;

use std::sync::Arc;
use std::time::{Duration as Elapsed, Instant};

use common::{assert_detected, assert_imitated, assert_texted, choir};
use fux_core::generate::{
    Budget, FreeRequest, FreeText, GenerateError, Generated, PartContext, SearchControl,
    generate_free, part_endings,
};
use fux_core::imitation::Imitation;
use fux_core::rules::Profile;
use fux_core::text::{Text, TextOptions, resolve};
use fux_core::{Ambitus, Lang, Mode, ModeSpec, Pitch, Step};

/// A text as `fux free --text` takes it: a section of the Ordinary or an own Latin text.
fn text(spec: &str) -> Arc<Text> {
    Arc::new(resolve(spec, &TextOptions::default()).unwrap().0)
}

fn mode(name: &str) -> ModeSpec {
    name.parse().unwrap()
}

/// The pitches of a chord from the top, such as `A4 F4 A3 D3`.
fn chord(pitches: &str) -> Vec<Pitch> {
    pitches.split(' ').map(|p| p.parse().unwrap()).collect()
}

/// The width of the search in the tests that run in every debug build: what each test checks
/// (the ending, the start, the chord, the cancel) does not depend on it, and at the full width of
/// 200 their nine searches took about five times as long. The release tests keep the full width,
/// as the mass searches with it.
const DEBUG_BEAM: usize = 40;

/// A part of `voices` voices that sing `spec` in `mode`, its length and cadences planned from the
/// text, as `part`, imitative where `imitative` says so.
fn request(
    spec: &str,
    mode: ModeSpec,
    voices: usize,
    seed: u64,
    part: PartContext,
    imitative: bool,
) -> FreeRequest {
    FreeRequest {
        seed,
        text: Some(FreeText::new(text(spec))),
        imitation: imitative.then(Imitation::default),
        part,
        ..FreeRequest::new(mode, choir(voices), 0)
    }
}

/// The acceptance of a part: a texted setting by every oracle, imitated as planned where it has
/// imitation; its bass ends on the part's ending; no voice sounds before its start; and where it
/// opens with a chord, every voice strikes its pitch at the start on the first syllable and holds
/// it a semibreve (tied over the barline after a breath), or longer, tied on as the last note of
/// an entry's head may be.
fn accept(generated: &Generated, request: &FreeRequest, label: &str) {
    let sung = &request.text.as_ref().expect("a text").text;
    let score = &generated.score;
    assert_texted(generated, sung, label);
    if request.imitation.is_some() {
        assert_imitated(score, label);
        assert_detected(score, label);
        assert!(!generated.subjects.is_empty(), "{label}: no subjects");
    } else {
        assert_eq!(score.imitation, None, "{label}");
    }
    let part = &request.part;
    let ending = part.ending_in(request.mode);
    let bass = score.voices.last().unwrap().pitches();
    assert_eq!(
        bass.last().unwrap().step,
        ending,
        "{label}: the bass's last note"
    );
    for (v, voice) in score.voices.iter().enumerate() {
        for (start, note) in voice.timed() {
            if start < part.start {
                assert!(note.is_rest(), "{label}, voice {v}: {note:?} at {start}");
            }
        }
        let Some(chord) = &part.chord else {
            continue;
        };
        // The first note: the chord's pitch at the start, on the first syllable, a semibreve at
        // least.
        let (n, (start, first)) = voice
            .timed()
            .enumerate()
            .find(|(_, (_, note))| !note.is_rest())
            .unwrap_or_else(|| panic!("{label}, voice {v}: silent"));
        assert_eq!(
            (start, first.pitch),
            (part.start, Some(chord[v])),
            "{label}, voice {v}"
        );
        assert_eq!(
            first.lyric.and_then(|l| l.index()),
            Some(0),
            "{label}, voice {v}"
        );
        let mut held = first.duration.ticks();
        let mut tied = first.tie;
        for note in &voice.notes[n + 1..] {
            if !tied {
                break;
            }
            assert_eq!(note.pitch, first.pitch, "{label}, voice {v}: tied on");
            held += note.duration.ticks();
            tied = note.tie;
        }
        assert!(
            held >= 8,
            "{label}, voice {v}: the chord's note lasts {held} ticks"
        );
    }
}

/// Generates `request` and accepts its first result; returns it with the seconds it took.
fn accepted(request: &FreeRequest, label: &str) -> (Generated, f64) {
    let clock = Instant::now();
    let results =
        generate_free(request, &Profile::fux()).unwrap_or_else(|e| panic!("{label}: {e}"));
    let seconds = clock.elapsed().as_secs_f64();
    accept(&results[0], request, label);
    println!(
        "{label}: {} measures, cost {}, {seconds:.1} s",
        results[0].score.measure_count(),
        results[0].cost
    );
    (results[0].clone(), seconds)
}

/// The Christe for three voices without imitation ends on the fifth A of the Dorian mode, and on
/// A in the Phrygian mode, where a third in its last chord is the major one, C♯: the bass ends
/// there, nothing is forbidden.
#[test]
fn ending_on_the_fifth_and_in_phrygian_a() {
    for (name, seed) in [("dorian", 1), ("phrygian", 1)] {
        let part = PartContext {
            ending: Some(Step::A),
            ..PartContext::default()
        };
        let request = FreeRequest {
            beam: DEBUG_BEAM,
            ..request("christe", mode(name), 3, seed, part, false)
        };
        let label = format!("christe, {name}, on A");
        let (generated, _) = accepted(&request, &label);
        // The final cadence is a cadence on A, as the checker reads it from the bass.
        let last: Vec<Pitch> = generated
            .score
            .voices
            .iter()
            .map(|v| *v.pitches().last().unwrap())
            .collect();
        assert!(
            last.iter()
                .all(|p| matches!(p.step, Step::A | Step::E | Step::C)),
            "{label}: {last:?}"
        );
        assert!(
            last.iter().all(|p| p.step != Step::C || p.alter == 1),
            "{label}: a major third over A, {last:?}"
        );
    }
}

/// No voice sounds before a part's start: after a minim's breath and after a measure's rest,
/// without imitation (every voice rests up to the start) and with it (the leader of the first
/// point enters at the start).
#[test]
fn start_silences_every_voice() {
    for (start, imitative, seed) in [(4, false, 1), (8, false, 2), (4, true, 1), (8, true, 1)] {
        let part = PartContext {
            start,
            ..PartContext::default()
        };
        let request = FreeRequest {
            beam: DEBUG_BEAM,
            ..request("kyrie", mode("dorian"), 3, seed, part, imitative)
        };
        let label = format!("kyrie, start {start}, imitation {imitative}");
        let (generated, _) = accepted(&request, &label);
        if imitative {
            let plan = generated.score.imitation.as_deref().unwrap();
            assert_eq!(plan.points[0].entries[0].start, start, "{label}");
        } else {
            // Some voice begins right at the start.
            let begins = generated.score.voices.iter().any(|voice| {
                voice
                    .timed()
                    .any(|(at, note)| at == start && !note.is_rest())
            });
            assert!(begins, "{label}: no voice begins at the start");
        }
    }
}

/// An opening chord sounds as planned: every voice strikes its pitch at the start, at once,
/// after a breath (tied over the barline) and after a measure's rest, a root-position triad of
/// the final with the third and, from four voices on, the fifth; in two voices the fifth over the
/// final, the perfect consonance that CAD-START asks of a two-voice setting, as the opening chord
/// of a mass part in two voices has it (with the third there, F4 over D4, no setting begins).
#[test]
fn tutti_chord_sounds_as_planned() {
    for (start, voices, pitches, ending, seed) in [
        (0, 3, "F4 A3 D3", None, 1),
        (4, 3, "F4 A3 D3", Some(Step::A), 1),
        (8, 4, "A4 F4 A3 D3", None, 1),
        (0, 2, "A4 D4", None, 1),
        (4, 2, "A4 D4", None, 1),
    ] {
        let part = PartContext {
            start,
            chord: Some(chord(pitches)),
            ending,
            ..PartContext::default()
        };
        let request = FreeRequest {
            beam: DEBUG_BEAM,
            ..request("christe", mode("dorian"), voices, seed, part, false)
        };
        let label = format!("christe, {voices} voices, chord {pitches} at {start}");
        accepted(&request, &label);
    }
    // The third over the final in two voices breaks CAD-START at once.
    let third = PartContext {
        chord: Some(chord("F4 D4")),
        ..PartContext::default()
    };
    let request = FreeRequest {
        beam: DEBUG_BEAM,
        ..request("christe", mode("dorian"), 2, 1, third, false)
    };
    match generate_free(&request, &Profile::fux()) {
        Err(GenerateError::NoCounterpoint { measure, rules }) => {
            assert_eq!(measure, 1);
            assert!(rules.contains(&"CAD-START"), "{rules:?}");
        }
        other => panic!("{other:?}"),
    }
}

/// A cancelled part fails at once, without imitation as with it, and says so in both languages;
/// a control that is not cancelled changes nothing and tells the last measure the search
/// reached.
#[test]
fn cancel_returns_cancelled() {
    let control = Arc::new(SearchControl::new());
    let part = PartContext {
        ending: Some(Step::A),
        start: 4,
        ..PartContext::default()
    };
    let plain = FreeRequest {
        beam: 40,
        ..request("kyrie", mode("dorian"), 3, 1, part, false)
    };
    let controlled = FreeRequest {
        control: Some(Arc::clone(&control)),
        ..plain.clone()
    };
    let without = generate_free(&plain, &Profile::fux()).unwrap();
    let with = generate_free(&controlled, &Profile::fux()).unwrap();
    assert_eq!(with[0].score, without[0].score);
    assert_eq!(with[0].cost, without[0].cost);
    assert_eq!(control.reached() as usize, with[0].score.measure_count());
    control.cancel();
    for imitative in [false, true] {
        let request = FreeRequest {
            imitation: imitative.then(Imitation::default),
            ..controlled.clone()
        };
        let error = generate_free(&request, &Profile::fux()).unwrap_err();
        assert_eq!(error, GenerateError::Cancelled, "imitation {imitative}");
        assert_eq!(error.message(Lang::De), "Die Suche wurde abgebrochen.");
        assert_eq!(error.message(Lang::En), "The search was cancelled.");
    }
}

/// A search cancelled while it runs stops within a step: an imitative setting of the first
/// sentences of the Gloria in four voices, cancelled once its search has reached a measure (after
/// its imitation is planned), returns [`GenerateError::Cancelled`] at once (natively well within
/// a second; the bound leaves room for a busy machine), and the control tells the measure it
/// reached.
#[test]
fn cancel_stops_a_running_search() {
    let control = Arc::new(SearchControl::new());
    let request = FreeRequest {
        control: Some(Arc::clone(&control)),
        ..request(
            "gloria:1-5",
            mode("dorian"),
            4,
            1,
            PartContext::default(),
            true,
        )
    };
    let (result, latency, reached) = std::thread::scope(|scope| {
        let search = scope.spawn(|| {
            let result = generate_free(&request, &Profile::fux());
            (result, Instant::now())
        });
        // The search is running once it tells a measure it reached.
        let waiting = Instant::now();
        while control.reached() == 0 && !search.is_finished() {
            assert!(
                waiting.elapsed() < Elapsed::from_secs(120),
                "no step in two minutes"
            );
            std::thread::sleep(Elapsed::from_millis(5));
        }
        let reached = control.reached();
        let cancelled = Instant::now();
        control.cancel();
        let (result, ended) = search.join().unwrap();
        (result, ended.saturating_duration_since(cancelled), reached)
    });
    assert!(reached > 0, "the search ended before its first step");
    assert_eq!(result.unwrap_err(), GenerateError::Cancelled);
    assert!(latency < Elapsed::from_secs(3), "{latency:?}");
    assert!(control.reached() >= reached);
    println!("cancelled within {latency:?}, at measure {reached}");
}

/// Times the search with and without a control, which it reads once per step: the Kyrie for four
/// voices, seeds 1 to 3, the least of four rounds, each pair in turns of order (the one without
/// a control first in even rounds), against the noise of a busy machine. Both find the same
/// settings; prints the times per seed and their ratio. A step evaluates a beam of candidates,
/// each with a check of the score so far, so two relaxed atomic operations per step should not
/// show; the ratio says whether they do, and nothing asserts it, as a bound failed on a busy
/// machine.
#[test]
#[ignore = "timing: cargo test --release -p fux-core --test part -- --ignored --nocapture"]
fn control_timing() {
    const ROUNDS: usize = 4;
    let mut least = [[f64::INFINITY; 3]; 2];
    for round in 0..ROUNDS {
        for seed in 1..=3 {
            let order = if round % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            };
            let mut found = Vec::new();
            for controlled in order {
                let request = FreeRequest {
                    control: controlled.then(|| Arc::new(SearchControl::new())),
                    ..request(
                        "kyrie",
                        mode("dorian"),
                        4,
                        seed,
                        PartContext::default(),
                        false,
                    )
                };
                let clock = Instant::now();
                let results = generate_free(&request, &Profile::fux()).unwrap();
                let seconds = clock.elapsed().as_secs_f64();
                let (k, s) = (usize::from(controlled), seed as usize - 1);
                least[k][s] = least[k][s].min(seconds);
                found.push(results[0].score.clone());
            }
            assert_eq!(found[0], found[1], "seed {seed}");
        }
    }
    for (s, (without, with)) in least[0].iter().zip(&least[1]).enumerate() {
        println!(
            "seed {}: without a control {without:.2} s, with one {with:.2} s",
            s + 1
        );
    }
    let total = |k: usize| least[k].iter().sum::<f64>();
    println!(
        "without a control {:.2} s, with one {:.2} s: {:.3} times",
        total(0),
        total(1),
        total(1) / total(0)
    );
}

/// The twelve variants of the modes: each mode untransposed and transposed, authentic and plagal
/// in turn.
fn mode_variants() -> Vec<ModeSpec> {
    Mode::ALL
        .into_iter()
        .enumerate()
        .flat_map(|(m, mode)| {
            [false, true].map(|transposed| {
                let ambitus = if (m + usize::from(transposed)).is_multiple_of(2) {
                    Ambitus::Authentic
                } else {
                    Ambitus::Plagal
                };
                ModeSpec::new(mode)
                    .with_ambitus(ambitus)
                    .with_transposition(transposed)
            })
        })
        .collect()
}

/// The Christe for four voices, with and without imitation, in the twelve variants of the
/// modes, on every ending besides the final that the mode allows (the fifth and the fourth with a
/// clausula), seeds 1 and 2, with the portable budget of the mass: every part is found and
/// accepted, its bass on the ending. Prints a line per part and the failures at the end.
#[test]
#[ignore = "slow: cargo test --release -p fux-core --test part -- --ignored --nocapture"]
fn part_endings_in_every_mode() {
    let clock = Instant::now();
    let mut failures = Vec::new();
    let mut count = 0;
    let mut seconds = [0.0; 2];
    for mode in mode_variants() {
        for &ending in &part_endings(mode)[1..] {
            for imitative in [false, true] {
                for seed in 1..=2 {
                    count += 1;
                    let part = PartContext {
                        ending: Some(ending),
                        ..PartContext::default()
                    };
                    let request = FreeRequest {
                        budget: Budget::PORTABLE,
                        ..request("christe", mode, 4, seed, part, imitative)
                    };
                    let label = format!(
                        "christe, {}, on {}, imitation {imitative}, seed {seed}",
                        mode.id(),
                        ending.letter()
                    );
                    let start = Instant::now();
                    let results = match generate_free(&request, &Profile::fux()) {
                        Ok(results) => results,
                        Err(error) => {
                            println!("{label}: {error}");
                            failures.push(format!("{label}: {error}"));
                            continue;
                        }
                    };
                    let took = start.elapsed().as_secs_f64();
                    seconds[usize::from(imitative)] += took;
                    let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        accept(&results[0], &request, &label)
                    }));
                    if let Err(panic) = checked {
                        let message = panic
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_default();
                        failures.push(message);
                        continue;
                    }
                    println!(
                        "{label}: {} measures, cost {}, {took:.1} s",
                        results[0].score.measure_count(),
                        results[0].cost
                    );
                }
            }
        }
    }
    println!(
        "{} of {count} parts found and accepted in {:.0} s ({:.0} s without imitation, {:.0} s \
         with)",
        count - failures.len(),
        clock.elapsed().as_secs_f64(),
        seconds[0],
        seconds[1]
    );
    assert!(failures.is_empty(), "{failures:#?}");
}
