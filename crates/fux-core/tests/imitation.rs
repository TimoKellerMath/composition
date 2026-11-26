//! Imitative free settings (phase 13, docs/regeln.md § 18): the acceptance of the generator. Every
//! setting breaks no prohibition, sings its text in every voice by the rules of the underlay
//! (`assert_texted`, with the syllables the plan pins), sounds every planned entry as planned
//! (`assert_imitated`, which uses no code of the rules) and shows its points to the detector
//! (`assert_detected`); every fifth setting is also checked window by window and prefix by
//! prefix, as the search sees it. Further gates measure the success rate over seeds, the style of
//! the imitation against Palestrina's and the time against the texted search without imitation.
//! The tests run in release:
//! `cargo test --release -p fux-core --test imitation -- --ignored --nocapture --test-threads=1`.

mod common;

use std::sync::Arc;
use std::time::Instant;

use common::{
    assert_detected, assert_imitated, assert_prefixes_collect, assert_sung, assert_texted,
    assert_windows_partition, calibration_pieces, choir, corpus_present, text_rules,
};
use fux_core::gabc::{Chant, parse_gabc};
use fux_core::generate::{FreeRequest, FreeText, GenerateError, Generated, generate_free};
use fux_core::imitation::chant::Kyriale;
use fux_core::imitation::detect::{Answered, FoundPoint, detect};
use fux_core::imitation::{Imitation, SubjectSource};
use fux_core::rules::{Checker, Profile};
use fux_core::text::{Text, TextOptions, resolve};
use fux_core::{Ambitus, Duration, Mode, ModeSpec, Score, parse_lilypond};

/// A text as `fux free --text` takes it: a section of the Ordinary or an own Latin text.
fn text(spec: &str) -> Arc<Text> {
    Arc::new(resolve(spec, &TextOptions::default()).unwrap().0)
}

fn mode(name: &str) -> ModeSpec {
    name.parse().unwrap()
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

/// An imitative free setting of `voices` voices that all sing `spec`, with the length and the
/// cadences planned from the text and the beam of `fux free`.
fn request(
    spec: &str,
    mode: ModeSpec,
    voices: usize,
    seed: u64,
    imitation: Imitation,
) -> FreeRequest {
    FreeRequest {
        seed,
        text: Some(FreeText::new(text(spec))),
        imitation: Some(imitation),
        ..FreeRequest::new(mode, choir(voices), 0)
    }
}

/// The same request without imitation.
fn plain(request: &FreeRequest) -> FreeRequest {
    FreeRequest {
        imitation: None,
        ..request.clone()
    }
}

/// The acceptance of one result: every oracle, and with `windows` the checks window by window and
/// prefix by prefix, under Fux's profile and the TXT rules alone.
fn accept(generated: &Generated, request: &FreeRequest, label: &str, windows: bool) {
    let sung = &request.text.as_ref().expect("a text").text;
    let score = &generated.score;
    assert_texted(generated, sung, label);
    assert_imitated(score, label);
    assert_detected(score, label);
    // The internal cadences are the goals of the points.
    let plan = score.imitation.as_deref().expect("a plan");
    assert_eq!(
        score.cadences.len(),
        plan.points.len() - 1,
        "{label}: a cadence ends every point but the last"
    );
    // The heads lie within the voices' ranges: no head note is more than a step beyond its
    // staff where the head an octave higher or lower lies on it (the planner's tests check the
    // octave of every head; the rare head beyond is priced by VOI-RANGE).
    for entry in plan.points.iter().flat_map(|p| &p.entries) {
        let clef = score.voices[entry.voice].voice_type.clef();
        let near = |shift: i32| {
            entry.notes.iter().all(|&(p, _)| {
                let shifted = fux_core::Pitch::from_diatonic(p.diatonic() + shift);
                (-1..=1).contains(&clef.steps_outside(shifted))
            })
        };
        let beyond = !near(0);
        assert!(
            !beyond || !(near(7) || near(-7)),
            "{label}: voice {} at {} sings {:?} beyond its staff, where an octave lies on it",
            entry.voice,
            entry.start,
            entry.notes
        );
    }
    if windows {
        let fux = Checker::new(&Profile::fux());
        let text_rules = text_rules();
        assert_windows_partition(&fux, score, label);
        assert_prefixes_collect(&fux, score, label);
        assert_windows_partition(&text_rules, score, label);
        assert_prefixes_collect(&text_rules, score, label);
    }
}

/// Runs the requests one by one: each must find a setting that passes [`accept`], every fifth
/// checked window by window. Prints a line per setting, and at the end every failure.
fn accept_all(requests: &[(String, FreeRequest)]) {
    let clock = Instant::now();
    let mut failures = Vec::new();
    for (k, (label, request)) in requests.iter().enumerate() {
        let start = Instant::now();
        let results = match generate_free(request, &Profile::fux()) {
            Ok(results) => results,
            Err(error) => {
                println!("{label}: {error}");
                failures.push(format!("{label}: {error}"));
                continue;
            }
        };
        let generated = &results[0];
        let seconds = start.elapsed().as_secs_f64();
        let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            accept(generated, request, label, k % 5 == 0)
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
        let plan = generated.score.imitation.as_deref().expect("a plan");
        println!(
            "{label}: {} measures, {} points, cost {}, {seconds:.1} s",
            generated.score.measure_count(),
            plan.points.len(),
            generated.cost
        );
    }
    println!(
        "{} settings in {:.1} s",
        requests.len(),
        clock.elapsed().as_secs_f64()
    );
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The label of a request.
fn label(spec: &str, mode: ModeSpec, voices: usize, seed: u64) -> String {
    format!("{spec}, {}, {voices} voices, seed {seed}", mode.id())
}

/// Whether `check` panics on `score`.
fn rejects(check: fn(&Score, &str), score: &Score) -> bool {
    std::panic::catch_unwind(|| check(score, "falsified")).is_err()
}

/// The oracle of the imitation catches every departure from the plan: a head note sung at
/// another pitch, a planned value that the score does not have, a rest planned longer than the
/// silence before an entry, and a head note with another syllable than the leader's. A small
/// imitative Kyrie (three voices, beam 20) passes it as generated, and the detector finds its
/// points.
#[test]
fn the_oracle_finds_departures() {
    let request = FreeRequest {
        beam: 20,
        ..request("kyrie", mode("dorian"), 3, 1, Imitation::default())
    };
    let results = generate_free(&request, &Profile::fux()).unwrap();
    let score = &results[0].score;
    assert_imitated(score, "kyrie");
    assert_detected(score, "kyrie");
    let plan = score.imitation.as_deref().unwrap().clone();
    // The second entry of the first point and the written note of its third head note.
    let entry = plan.points[0].entries[1].clone();
    let tick = entry.start + entry.notes[0].1 + entry.notes[1].1;
    let index = score.voices[entry.voice]
        .timed()
        .position(|(start, _)| start == tick)
        .unwrap();
    // Another pitch.
    let mut moved = score.clone();
    let note = &mut moved.voices[entry.voice].notes[index];
    note.pitch = note
        .pitch
        .map(|p| fux_core::Pitch::from_diatonic(p.diatonic() + 1));
    assert!(
        rejects(assert_imitated, &moved),
        "a head note at another pitch"
    );
    // A planned value the score does not have.
    let mut planned = plan.clone();
    planned.points[0].entries[1].notes[1].1 += 2;
    let mut longer = score.clone();
    longer.imitation = Some(Arc::new(planned));
    assert!(rejects(assert_imitated, &longer), "another value");
    // A longer rest than the silence before a later entry.
    let mut planned = plan.clone();
    let later = &mut planned.points[1].entries[0];
    later.rest = later.start;
    let mut rested = score.clone();
    rested.imitation = Some(Arc::new(planned));
    assert!(rejects(assert_imitated, &rested), "a rest too short");
    // Another syllable on a head note than the leader's.
    let mut sung = score.clone();
    let note = &mut sung.voices[entry.voice].notes[index];
    note.lyric = match note.lyric {
        Some(_) => None,
        None => Some(fux_core::Lyric::new(0)),
    };
    assert!(rejects(assert_imitated, &sung), "another syllable");
}

/// Imitative Kyries in three and four voices, Dorian, seeds 1 and 2, as `fux free` asks for them
/// but with a beam of 40, as the golden test of the texted search has it: every planned entry
/// sounds as planned within the limits of every plan (by the oracle, which uses no code of the
/// rules); every voice enters in the first point and sings the whole text, with the syllables
/// written out; nothing is forbidden, also when Fux's profile checks the score again; and no
/// case of the heads as planned or of their rests.
#[test]
fn imitative_kyries_are_sung_as_planned() {
    let kyrie = text("kyrie");
    let fux = Checker::new(&Profile::fux());
    for voices in [3, 4] {
        for seed in [1, 2] {
            let label = format!("kyrie, dorian, {voices} voices, seed {seed}");
            let request = FreeRequest {
                beam: 40,
                ..request("kyrie", mode("dorian"), voices, seed, Imitation::default())
            };
            let results =
                generate_free(&request, &Profile::fux()).unwrap_or_else(|e| panic!("{label}: {e}"));
            let generated = &results[0];
            let score = &generated.score;
            assert_imitated(score, &label);
            let plan = score.imitation.as_deref().expect("the plan in the score");
            assert_eq!(score.cadences.len(), plan.points.len() - 1, "{label}");
            assert_eq!(generated.report.forbidden(), 0, "{label}");
            assert_eq!(fux.check(score).violations, generated.report.violations);
            // The only case of the heads and their rests the free notes can bring is a minim
            // before a rest (IMI-REST `minim_before`, a preference).
            let departing: Vec<_> = generated
                .report
                .violations
                .iter()
                .filter(|v| match v.rule {
                    "IMI-ENTRY" => true,
                    "IMI-REST" => v.detail != fux_core::rules::Detail::Param("minim_before"),
                    _ => false,
                })
                .collect();
            assert!(departing.is_empty(), "{label}: {departing:?}");
            for (v, voice) in score.voices.iter().enumerate() {
                let label = format!("{label}, voice {v}");
                assert_sung(&kyrie, &voice.notes, &[], &label);
                for note in &voice.notes {
                    assert_eq!(note.syllable.is_some(), note.lyric.is_some(), "{label}");
                }
            }
        }
    }
}

/// Kyrie I in the twelve variants of the modes with three, four and five voices, seed 1.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn imitative_kyries_in_every_mode() {
    let mut requests = Vec::new();
    for mode in mode_variants() {
        for voices in 3..=5 {
            let request = request("kyrie", mode, voices, 1, Imitation::default());
            requests.push((label("kyrie", mode, voices, 1), request));
        }
    }
    accept_all(&requests);
}

/// The other sections of the Ordinary and an own text in four voices and four modes; the Agnus
/// and the Benedictus in five voices; the Osanna and the Pleni in three.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn imitative_ordinary_and_own_texts() {
    let mut requests = Vec::new();
    for spec in [
        "christe",
        "agnus",
        "sanctus",
        "benedictus",
        "gloria:1-5",
        "Ad te le-vá-vi á-ni-mam me-am.",
    ] {
        for name in ["dorian", "phrygian", "mixolydian", "ionian-transposed"] {
            let request = request(spec, mode(name), 4, 1, Imitation::default());
            requests.push((label(spec, mode(name), 4, 1), request));
        }
    }
    for spec in ["agnus", "benedictus"] {
        for name in ["dorian", "aeolian"] {
            let request = request(spec, mode(name), 5, 1, Imitation::default());
            requests.push((label(spec, mode(name), 5, 1), request));
        }
    }
    for (spec, name) in [("osanna", "dorian"), ("pleni", "mixolydian")] {
        let request = request(spec, mode(name), 3, 1, Imitation::default());
        requests.push((label(spec, mode(name), 3, 1), request));
    }
    accept_all(&requests);
}

/// A chant's phrases as the subjects, found by the words.
fn chant_imitation(chant: Arc<Chant>) -> Imitation {
    Imitation {
        subject: SubjectSource::Chant {
            chant,
            phrases: None,
        },
        ..Imitation::default()
    }
}

/// Subjects from chant, each setting in the mode of its chant: Kyrie IX (Palestrina's Missa de
/// beata Virgine) on Kyrie I with three to five voices, on the Christe and on Kyrie II; Kyrie IV
/// on Kyrie I; Sanctus IX on the Sanctus; and the hymn Iste confessor on an own text, whose
/// points take its phrases in order.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn chant_subjects() {
    let iste = Arc::new(
        parse_gabc(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/chant/iste-confessor.gabc"
        )))
        .unwrap(),
    );
    let kyrie_ix = Kyriale::KyrieIX.chant();
    let mut settings: Vec<(&str, Arc<Chant>, usize)> = (3..=5)
        .map(|voices| ("kyrie", Arc::clone(&kyrie_ix), voices))
        .collect();
    settings.extend([
        ("christe", Arc::clone(&kyrie_ix), 4),
        ("kyrie2", Arc::clone(&kyrie_ix), 4),
        ("kyrie", Kyriale::KyrieIV.chant(), 4),
        ("sanctus", Kyriale::SanctusIX.chant(), 4),
        ("Lau-dá-te Dó-mi-num, / om-nes gen-tes.", iste, 4),
    ]);
    let mut requests = Vec::new();
    for (spec, chant, voices) in settings {
        let mode = chant.numbered_mode().unwrap();
        let request = request(spec, mode, voices, 1, chant_imitation(chant));
        requests.push((label(spec, mode, voices, 1), request));
    }
    accept_all(&requests);
}

/// Subjects given as notes, Dorian, four and five voices: the first point's and the tail's.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn given_subjects() {
    let heads: Vec<_> = ["d'1 d'2 a'2 g'4 f'4 e'2 d'1", "a'1 a'2 g'2 f'2 e'2 d'1"]
        .iter()
        .map(|head| parse_lilypond(head, Duration::HALF).unwrap())
        .collect();
    let imitation = Imitation {
        subject: SubjectSource::Given(heads),
        ..Imitation::default()
    };
    let mut requests = Vec::new();
    for voices in [4, 5] {
        let request = request("kyrie", mode("dorian"), voices, 1, imitation.clone());
        requests.push((label("kyrie", mode("dorian"), voices, 1), request));
    }
    accept_all(&requests);
}

/// How often the generator finds an imitative setting: Kyrie I and Agnus Dei I in four modes
/// with three to five voices and seeds 1 to 10, 97 % at least with three and four voices, 90 %
/// with five; no result breaks a prohibition, and every one keeps its plan.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn success_rate() {
    let clock = Instant::now();
    // Per number of voices: requests, settings found, seconds.
    let mut counts = [(0usize, 0usize, 0.0f64); 3];
    let mut failures = Vec::new();
    for spec in ["kyrie", "agnus"] {
        for name in ["dorian", "phrygian", "mixolydian", "aeolian"] {
            for voices in 3..=5 {
                for seed in 1..=10 {
                    let label = label(spec, mode(name), voices, seed);
                    let request = request(spec, mode(name), voices, seed, Imitation::default());
                    let start = Instant::now();
                    let result = generate_free(&request, &Profile::fux());
                    let seconds = start.elapsed().as_secs_f64();
                    let count = &mut counts[voices - 3];
                    count.0 += 1;
                    count.2 += seconds;
                    match result {
                        Ok(results) => {
                            count.1 += 1;
                            let generated = &results[0];
                            assert_eq!(generated.report.forbidden(), 0, "{label}");
                            assert_imitated(&generated.score, &label);
                            println!(
                                "{label}: {} measures, {seconds:.1} s",
                                generated.score.measure_count()
                            );
                        }
                        Err(error) => {
                            println!("{label}: {error} ({seconds:.1} s)");
                            failures.push(format!("{label}: {error}"));
                        }
                    }
                }
            }
        }
    }
    for (k, &(requests, found, seconds)) in counts.iter().enumerate() {
        println!(
            "{} voices: {found} of {requests} found ({:.1} %), {:.1} s each",
            k + 3,
            100.0 * found as f64 / requests as f64,
            seconds / requests as f64
        );
    }
    println!(
        "{} failures in {:.0} s: {failures:#?}",
        failures.len(),
        clock.elapsed().as_secs_f64()
    );
    for (k, &(requests, found, _)) in counts.iter().enumerate() {
        let least = if k + 3 == 5 { 0.90 } else { 0.97 };
        assert!(
            found as f64 >= least * requests as f64,
            "{} voices: {found} of {requests}",
            k + 3
        );
    }
}

/// The measures of imitation that the style test compares, from the points the detector finds
/// (entries after rests of a minim): in openings, the second entry's interval to the first, its
/// delay in four voices, the leader's degree, first note, first interval and the range of the
/// imitated head, complete four-voice openings in repeated duos; in all points the notes the
/// followers imitate, entries before the voice before has ended its head (stretto) and real
/// answers; the entries of later points on the minim beat.
#[derive(Default)]
struct Imitations {
    scores: usize,
    points: usize,
    /// Second entries of openings: at the fourth or the fifth, at the unison or the octave, all.
    second: [usize; 3],
    /// Openings of four voices: the second entry after a semibreve, all.
    close: [usize; 2],
    /// Leaders of openings on the final or the fifth (A in the Phrygian mode), all.
    main_degree: [usize; 2],
    /// Leaders of openings with a first note of a semibreve or longer, all.
    long_first: [usize; 2],
    /// Leaders of openings that strike their first note again, all.
    repeat: [usize; 2],
    /// The range of the imitated head of each opening's leader, in diatonic steps.
    ranges: Vec<i64>,
    /// The notes each follower imitates.
    matched: Vec<i64>,
    /// Consecutive entries of a point that come before the head before has ended, all.
    stretto: [usize; 2],
    /// Real answers among the followers, all.
    real: [usize; 2],
    /// Entries of later points on the minim beat, all.
    later_minim: [usize; 2],
    /// Complete four-voice openings in repeated duos, all.
    duos: [usize; 2],
}

/// The median of `values`, for an even number the mean of the two middle ones rounded down, as
/// the study of the corpus took it; 0 for none.
fn median(values: &[i64]) -> i64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    match sorted.len() {
        0 => 0,
        n if n % 2 == 1 => sorted[n / 2],
        n => (sorted[n / 2 - 1] + sorted[n / 2]).div_euclid(2),
    }
}

impl Imitations {
    /// Adds the points of `score` that the detector finds.
    fn add(&mut self, score: &Score) {
        self.scores += 1;
        let voices = score.voices.len();
        let main = |pitch: fux_core::Pitch| {
            let degree = score.mode.degree(pitch);
            degree == 1
                || match score.mode.mode {
                    Mode::Phrygian => degree == 4,
                    _ => degree == 5,
                }
        };
        for point in detect(score, 4) {
            self.add_point(&point, voices, &main);
        }
    }

    fn add_point(
        &mut self,
        point: &FoundPoint,
        voices: usize,
        main: &impl Fn(fux_core::Pitch) -> bool,
    ) {
        self.points += 1;
        let leader = &point.entries[0];
        // The first entry of each voice, in time order.
        let mut firsts = Vec::new();
        for entry in &point.entries {
            if !firsts
                .iter()
                .any(|e: &&fux_core::imitation::detect::FoundEntry| e.voice == entry.voice)
            {
                firsts.push(entry);
            }
        }
        let followers: Vec<_> = point
            .entries
            .iter()
            .filter(|e| e.follows.is_some())
            .collect();
        let matched: Vec<i64> = followers.iter().map(|e| e.matched as i64).collect();
        self.matched.extend(&matched);
        for follower in &followers {
            self.real[0] += usize::from(follower.answer == Answered::Real);
            self.real[1] += 1;
        }
        // The subject's length, as the study of the corpus took it: the median of the notes the
        // followers imitate, two at least.
        let length = (median(&matched).max(2) as usize).min(leader.notes.len());
        for pair in firsts.windows(2) {
            let head = &pair[0].notes[..length.min(pair[0].notes.len())];
            let last = head[head.len() - 1];
            let span = last.start - head[0].start + (last.end - last.start).min(8);
            self.stretto[0] += usize::from(pair[1].start - pair[0].start < span);
            self.stretto[1] += 1;
        }
        if !point.opens {
            for entry in &point.entries {
                self.later_minim[0] += usize::from(entry.start % 8 == 4);
                self.later_minim[1] += 1;
            }
            return;
        }
        let first = leader.notes[0];
        self.main_degree[0] += usize::from(main(first.pitch));
        self.main_degree[1] += 1;
        self.long_first[0] += usize::from(first.end - first.start >= 8);
        self.long_first[1] += 1;
        if let Some(next) = leader.notes.get(1) {
            self.repeat[0] += usize::from(next.pitch.diatonic() == first.pitch.diatonic());
            self.repeat[1] += 1;
        }
        let head = &leader.notes[..length];
        let steps: Vec<i32> = head.iter().map(|s| s.pitch.diatonic()).collect();
        self.ranges.push(i64::from(
            steps.iter().max().unwrap() - steps.iter().min().unwrap(),
        ));
        if let Some(second) = firsts.get(1) {
            let apart = (second.notes[0].pitch.diatonic() - first.pitch.diatonic()).rem_euclid(7);
            self.second[0] += usize::from(matches!(apart, 3 | 4));
            self.second[1] += usize::from(apart == 0);
            self.second[2] += 1;
            if voices == 4 {
                self.close[0] += usize::from(second.start - leader.start == 8);
                self.close[1] += 1;
            }
        }
        let starts: Vec<u32> = firsts.iter().map(|e| e.start).collect();
        if voices == 4 && firsts.len() == 4 && starts.windows(2).all(|w| w[0] < w[1]) {
            let gaps: Vec<u32> = starts.windows(2).map(|w| w[1] - w[0]).collect();
            let steps: Vec<i32> = firsts
                .windows(2)
                .map(|w| w[1].notes[0].pitch.diatonic() - w[0].notes[0].pitch.diatonic())
                .collect();
            let duo =
                gaps[0] == gaps[2] && gaps[1] > gaps[0] && (steps[0] - steps[2]).rem_euclid(7) == 0;
            self.duos[0] += usize::from(duo);
            self.duos[1] += 1;
        }
    }

    /// The measures as rows: name, value, and the target of generated settings.
    fn rows(&self) -> Vec<(&'static str, f64, (f64, f64))> {
        let share = |[a, b]: [usize; 2]| 100.0 * a as f64 / b.max(1) as f64;
        let all = self.second[2].max(1) as f64;
        vec![
            (
                "2nd entry at a 4th or 5th (%)",
                100.0 * self.second[0] as f64 / all,
                (65.0, 85.0),
            ),
            (
                "2nd entry at the unison or 8ve (%)",
                100.0 * self.second[1] as f64 / all,
                (10.0, 30.0),
            ),
            (
                "dt(1->2) = 8 ticks, 4 voices (%)",
                share(self.close),
                (25.0, 50.0),
            ),
            (
                "leader on the final or fifth (%)",
                share(self.main_degree),
                (65.0, 100.0),
            ),
            (
                "leader's first note >= a whole (%)",
                share(self.long_first),
                (90.0, 100.0),
            ),
            (
                "imitated notes (median)",
                median(&self.matched) as f64,
                (5.0, 8.0),
            ),
            (
                "head range (median, steps)",
                median(&self.ranges) as f64,
                (3.0, 5.0),
            ),
            ("stretto (%)", share(self.stretto), (50.0, 80.0)),
            ("real answers (%)", share(self.real), (85.0, 100.0)),
            (
                "later-point entries on the minim beat (%)",
                share(self.later_minim),
                (40.0, 100.0),
            ),
            (
                "repeated duos, complete 4-voice openings (%)",
                share(self.duos),
                (10.0, 35.0),
            ),
            (
                "first interval a repeat (%)",
                share(self.repeat),
                (15.0, 40.0),
            ),
        ]
    }
}

/// The style of the imitation against Palestrina's, measured by the same detector (entries after
/// rests of a minim) on his movements and on 72 settings: Kyrie I, the Christe, Agnus Dei I and
/// the Benedictus in four voices, four modes and seeds 1 to 3, and in three and five voices, three
/// modes and seed 1 (docs/regeln.md § 18). Each measure of the settings keeps its target.
#[test]
#[ignore = "slow: cargo test --release -- --ignored"]
fn imitation_style() {
    if !corpus_present() {
        return;
    }
    let clock = Instant::now();
    let mut palestrina = Imitations::default();
    for (_, piece) in calibration_pieces() {
        palestrina.add(&piece.score);
    }
    for (name, value, _) in palestrina.rows() {
        println!("Palestrina: {name:<46} {value:>8.1}");
    }
    let mut generated = Imitations::default();
    let mut failures = Vec::new();
    let modes = ["dorian", "phrygian", "mixolydian", "aeolian"];
    let mut requests = Vec::new();
    for spec in ["kyrie", "christe", "agnus", "benedictus"] {
        for name in modes {
            for seed in 1..=3 {
                requests.push((spec, name, 4, seed));
            }
        }
        for name in &modes[..3] {
            for voices in [3, 5] {
                requests.push((spec, name, voices, 1));
            }
        }
    }
    for (spec, name, voices, seed) in requests {
        let request = request(spec, mode(name), voices, seed, Imitation::default());
        match generate_free(&request, &Profile::fux()) {
            Ok(results) => generated.add(&results[0].score),
            Err(error) => failures.push(format!(
                "{}: {error}",
                label(spec, mode(name), voices, seed)
            )),
        }
    }
    println!(
        "{:<46} {:>10} {:>10}   target   ({} movements, {} points; {} settings, {} points)",
        "",
        "Palestrina",
        "generated",
        palestrina.scores,
        palestrina.points,
        generated.scores,
        generated.points
    );
    let mut missed = Vec::new();
    for ((name, theirs, _), (_, ours, (low, high))) in
        palestrina.rows().into_iter().zip(generated.rows())
    {
        let keeps = (low..=high).contains(&ours);
        println!(
            "{name:<46} {theirs:>10.1} {ours:>10.1}   {low}–{high}{}",
            if keeps { "" } else { "  missed" }
        );
        if !keeps {
            missed.push(name);
        }
    }
    println!(
        "{:.0} s; not found: {failures:?}",
        clock.elapsed().as_secs_f64()
    );
    assert!(failures.is_empty(), "{failures:?}");
    assert!(missed.is_empty(), "missed: {missed:?}");
}

/// The least seconds of `rounds` runs of `request`, with the outcome of the last: the measures
/// of the setting, or the error.
fn timed(request: &FreeRequest, rounds: usize) -> (f64, Result<usize, GenerateError>) {
    let profile = Profile::fux();
    let mut least = f64::INFINITY;
    let mut outcome = Ok(0);
    for _ in 0..rounds {
        let clock = Instant::now();
        let result = generate_free(request, &profile);
        least = least.min(clock.elapsed().as_secs_f64());
        outcome = result.map(|results| results[0].score.measure_count());
    }
    (least, outcome)
}

/// The timings of imitative settings against the texted search without imitation of the same
/// text, Dorian, three to five voices, seeds 1 and 2, one round each:
/// `cargo test --release -p fux-core --test imitation -- --ignored --nocapture imitation_timings`.
#[test]
#[ignore = "timing: cargo test --release -- --ignored"]
fn imitation_timings() {
    println!(
        "{:<12} {:>6} {:>18} {:>18} {:>7}",
        "text", "voices", "imitative s (m.)", "texted s (m.)", "factor"
    );
    for spec in ["kyrie", "christe", "agnus3", "gloria:1-5"] {
        for voices in 3..=5 {
            let (mut imitative, mut texted) = (0.0, 0.0);
            let (mut long, mut short) = (Vec::new(), Vec::new());
            for seed in 1..=2 {
                let request = request(spec, mode("dorian"), voices, seed, Imitation::default());
                let (seconds, outcome) = timed(&request, 1);
                imitative += seconds;
                long.push(outcome.map_or_else(|e| format!("({e})"), |m| m.to_string()));
                let (seconds, outcome) = timed(&plain(&request), 1);
                texted += seconds;
                short.push(outcome.map_or_else(|e| format!("({e})"), |m| m.to_string()));
            }
            println!(
                "{spec:<12} {voices:>6} {:>18} {:>18} {:>7.2}",
                format!("{imitative:.1} ({})", long.join("/")),
                format!("{texted:.1} ({})", short.join("/")),
                imitative / texted
            );
        }
    }
}

/// The guard of the plan: the imitative search is not much slower than the texted search
/// without imitation. Kyrie I in four voices, Dorian, seeds 1 to 6: each imitative setting
/// against the texted setting without imitation of the same length (the text then planned
/// into it), the least of two rounds each; the median of the six factors at most 1.5. The plan
/// sets the imitative setting's length (docs/regeln.md § 18), and per measure the imitative
/// search costs about as much as the texted one, so a factor against the texted setting of its
/// own planned length would follow the two seeds' plan lengths rather than the speed of the
/// search: with seeds 1 and 2 it moved between 1.85 and 2.67 as the drawn plans changed. Per
/// planned measure and over six seeds it measures the search. Prints the factors against the
/// texted setting of the text's own length too:
/// `cargo test --release -p fux-core --test imitation -- --ignored --nocapture fast_enough`.
#[test]
#[ignore = "timing: cargo test --release -- --ignored"]
fn imitative_search_is_fast_enough() {
    let (mut factors, mut natural) = (Vec::new(), (0.0, 0.0));
    for seed in 1..=6 {
        let request = request("kyrie", mode("dorian"), 4, seed, Imitation::default());
        let (imitative, outcome) = timed(&request, 2);
        let measures = outcome.unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        let mut same = plain(&request);
        same.measures = measures;
        same.text.as_mut().unwrap().auto_length = false;
        let (texted, outcome) = timed(&same, 2);
        assert_eq!(outcome, Ok(measures), "seed {seed}");
        let (own, outcome) = timed(&plain(&request), 1);
        let own_measures = outcome.unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        natural = (natural.0 + imitative, natural.1 + own);
        let factor = imitative / texted;
        println!(
            "seed {seed}: imitative {imitative:.2} s, texted of the same {measures} measures \
             {texted:.2} s ({factor:.2} times), of its own {own_measures} measures {own:.2} s"
        );
        factors.push(factor);
    }
    factors.sort_by(f64::total_cmp);
    let median = (factors[2] + factors[3]) / 2.0;
    println!(
        "median {median:.2} times the texted search of the same length; {:.2} times the texted \
         setting of the text's own length",
        natural.0 / natural.1
    );
    assert!(median <= 1.5, "{median:.2} times");
}
