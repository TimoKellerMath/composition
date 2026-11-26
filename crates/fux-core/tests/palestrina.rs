//! Palestrina's masses as a corpus for free counterpoint (references/datasets/palestrina-masses).
//!
//! With the standard profile his movements break the prohibitions only in the cases counted in
//! `EXCEPTIONS` (docs/regeln.md § 16). The ignored tests print the statistics behind the default
//! levels of free counterpoint (run them with `--release -- --ignored --nocapture`).

mod common;

use std::collections::BTreeMap;
use std::sync::OnceLock;

use common::{
    assert_prefixes_collect, assert_windows_partition, calibration_pieces, corpus_present, read_all,
};

/// All files are read except three with tuplets, and all scores are valid except four sections in
/// triple meter that end within a measure (they go on in the next section). The files carry no
/// text, so no score sings one.
#[test]
fn the_corpus_reads() {
    if !corpus_present() {
        return;
    }
    let all = read_all();
    assert_eq!(all.len(), 1318);
    let mut failed = Vec::new();
    for (name, result) in &all {
        match result {
            Ok(piece) => {
                assert_eq!(piece.score.text, None, "{name}");
                if let Err(e) = piece.score.validate() {
                    failed.push(name.as_str());
                    assert!(e.to_string().contains("12-tick measures"), "{name}: {e}");
                }
            }
            Err(e) => {
                failed.push(name.as_str());
                assert!(e.to_string().contains("the duration 3"), "{name}: {e}");
            }
        }
    }
    assert_eq!(
        failed,
        [
            "Agnus_III_b.krn",
            "Benedictus_66_b.krn",
            "Benedictus_71_b.krn",
            "Credo_10_e.krn",
            "Credo_66_g.krn",
            "Kyrie_89.krn",
            "Sanctus_rpt_01_c.krn"
        ]
    );
}

/// How often Palestrina breaks each prohibition of the standard profile in the 1156 movements of
/// the calibration: rare figures, editorial ficta of the edition and probably errors of the
/// encoding, each well below 0.1 % of the opportunities (docs/regeln.md § 16).
const EXCEPTIONS: &[(&str, usize)] = &[
    ("FREE-PASSING", 498),
    ("FIC-SCOPE", 349),
    ("TEX-PARALLEL", 222),
    ("FREE-RESOLUTION", 298),
    ("FREE-EIGHTHS", 96),
    ("TEX-PENULT", 75),
    ("HAR-CONSONANCE", 63),
    ("MEL-MAX-LEAP", 57),
    ("HAR-DISSONANCE", 42),
    ("TEX-CLAUSULA", 33),
    ("G3-ACCENT-LEAP", 29),
    ("MEL-CONSEC-LEAPS", 25),
    ("TEX-FINAL-CHORD", 15),
    ("HAR-CHROMATIC-CLASH", 11),
    ("G3-ACCENTED-PASSING", 7),
    ("MEL-MIN6-DOWN", 7),
    ("MEL-AUG-DIM", 2),
    ("MEL-TRITONE", 1),
];

#[test]
fn palestrina_breaks_only_the_counted_prohibitions() {
    if !corpus_present() {
        return;
    }
    let findings = findings();
    assert_eq!(findings.movements, 1156);
    let expected: BTreeMap<&str, usize> = EXCEPTIONS.iter().copied().collect();
    assert_eq!(findings.forbidden, expected);
}

/// The TXT rules look only at voices that sing a text, and a kern file has none: they report
/// nothing on any movement of the calibration, so they leave `EXCEPTIONS` as it was.
#[test]
fn txt_rules_are_silent_on_the_corpus() {
    if !corpus_present() {
        return;
    }
    let findings = findings();
    assert_eq!(findings.movements, 1156);
    assert_eq!(findings.text, Vec::<String>::new());
}

/// What the standard profile finds in the movements of the calibration, checked once for the
/// tests above.
struct Findings {
    movements: usize,
    /// How often each prohibition is broken.
    forbidden: BTreeMap<&'static str, usize>,
    /// Every case of the TXT rules, as „file: rule“.
    text: Vec<String>,
}

fn findings() -> &'static Findings {
    use fux_core::rules::{Checker, Profile};
    static FINDINGS: OnceLock<Findings> = OnceLock::new();
    FINDINGS.get_or_init(|| {
        let pieces = calibration_pieces();
        let checker = Checker::new(&Profile::fux());
        let mut forbidden: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut text = Vec::new();
        for (name, piece) in &pieces {
            for v in checker.check(&piece.score).violations {
                if v.rule.starts_with("TXT-") {
                    text.push(format!("{name}: {}", v.rule));
                }
                if v.is_forbidden() {
                    *forbidden.entry(v.rule).or_default() += 1;
                }
            }
        }
        Findings {
            movements: pieces.len(),
            forbidden,
            text,
        }
    })
}

/// The search checks window by window and sees the voices only up to the current tick; on free
/// counterpoint the rules must find the same as in the whole score. On short movements, so that
/// the test stays quick, with Fux's profile and with the profile of a mass (phase 14, D7), whose
/// parameters for settings without a plan of imitation (IMI-HELD `unplanned_*`, IMI-SILENCE
/// `unplanned_thin`) decide by what the voices sang before; on many movements in
/// [`window_by_window_on_many_with_the_mass_overlay`].
#[test]
fn palestrina_checks_window_by_window() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::{Checker, Profile};
    let fux = Checker::new(&Profile::fux());
    let mass = Checker::new(&fux_core::mass::profile(&Profile::fux()));
    let mut pieces = calibration_pieces();
    pieces.sort_by_key(|(name, p)| (p.score.measure_count() * p.score.voices.len(), name.clone()));
    let mut done = std::collections::BTreeSet::new();
    for (name, piece) in pieces {
        // The shortest movement for four, five and six voices each.
        if !done.insert(piece.score.voices.len()) {
            continue;
        }
        for checker in [&fux, &mass] {
            assert_windows_partition(checker, &piece.score, &name);
            assert_prefixes_collect(checker, &piece.score, &name);
        }
    }
    assert_eq!(done.len(), 4);
}

/// How often each rule fires in Palestrina's movements, with examples of prohibitions:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture report`.
#[test]
#[ignore]
fn report() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::{Checker, Profile};
    use std::collections::BTreeMap;
    let pieces = calibration_pieces();
    // Changed rules for experiments, e.g. OVERRIDES="FREE-PASSING.neighbor=P(13) …".
    let mut profile = match std::env::var("PROFILE").as_deref() {
        Ok(name) => Profile::builtin(name).unwrap(),
        Err(_) => Profile::fux(),
    };
    for spec in std::env::var("OVERRIDES")
        .unwrap_or_default()
        .split_whitespace()
    {
        profile.apply_override(spec).unwrap();
    }
    let checker = Checker::new(&profile);
    let started = std::time::Instant::now();
    let mut by_level: BTreeMap<(&str, u32), usize> = BTreeMap::new();
    let mut counts: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
    let mut examples: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut measures = 0;
    for (name, piece) in &pieces {
        let score = &piece.score;
        measures += score.measure_count() * score.voices.len();
        let report = checker.check(score);
        let mut seen: BTreeMap<&str, bool> = BTreeMap::new();
        for v in &report.violations {
            *by_level.entry((v.rule, v.level.cost())).or_default() += 1;
            let entry = counts.entry(v.rule).or_default();
            entry.0 += 1;
            if v.is_forbidden() {
                entry.1 += 1;
                let list = examples.entry(v.rule).or_default();
                if list.len() < std::env::var("EXAMPLES").map_or(6, |n| n.parse().unwrap()) {
                    let tick = score.note_start(v.voice, v.note).unwrap_or(v.tick);
                    let (m, at) = score.meter.locate(tick);
                    list.push(format!(
                        "{name} m{} +{at} {}: {}",
                        m + 1,
                        score.voices[v.voice].name,
                        checker.message(v, fux_core::Lang::En)
                    ));
                }
            }
            seen.insert(v.rule, true);
        }
        for rule in seen.keys() {
            counts.get_mut(rule).unwrap().2 += 1;
        }
    }
    println!(
        "{} movements, {measures} voice-measures, {:.1} s",
        pieces.len(),
        started.elapsed().as_secs_f64()
    );
    let mut rows: Vec<_> = counts.into_iter().collect();
    rows.sort_by_key(|(_, (all, hard, _))| std::cmp::Reverse((*hard, *all)));
    println!(
        "{:<26} {:>8} {:>8} {:>8}",
        "rule", "all", "forbidden", "pieces"
    );
    for (rule, (all, hard, files)) in &rows {
        println!("{rule:<26} {all:>8} {hard:>8} {files:>8}");
    }
    println!("\nby cost (forbidden: {}):", u32::MAX);
    for ((rule, cost), n) in &by_level {
        println!("  {rule:<26} {cost:>10} {n:>8}");
    }
    let only = std::env::var("RULE").ok();
    for (rule, list) in examples {
        if only.as_deref().is_some_and(|r| r != rule) {
            continue;
        }
        println!("\n{rule}:");
        for line in list {
            println!("  {line}");
        }
    }
}

/// The measure rhythms of Palestrina's voices with their counts, for the table of free
/// counterpoint: `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture rhythms`.
#[test]
#[ignore]
fn rhythms() {
    if !corpus_present() {
        return;
    }
    use fux_core::Note;
    use fux_core::rhythm::measure_rhythm;
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut last: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0;
    for (_, piece) in calibration_pieces() {
        let score = &piece.score;
        let per = score.meter.ticks_per_measure();
        for voice in &score.voices {
            let mut measures: Vec<(bool, Vec<&Note>)> = Vec::new();
            let mut tied = false;
            for (start, note) in voice.timed() {
                let m = (start / per) as usize;
                if measures.len() <= m {
                    measures.push((tied, Vec::new()));
                }
                measures[m].1.push(note);
                tied = note.tie;
            }
            for (tied_in, notes) in &measures {
                *counts.entry(measure_rhythm(notes, *tied_in)).or_default() += 1;
                total += 1;
            }
            if let Some((tied_in, notes)) = measures.last() {
                *last.entry(measure_rhythm(notes, *tied_in)).or_default() += 1;
            }
        }
    }
    println!("last measures: {last:?}");
    let mut rows: Vec<_> = counts.into_iter().collect();
    rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("{total} voice-measures, {} rhythms", rows.len());
    let mut sum = 0;
    for (pattern, n) in &rows {
        sum += n;
        println!(
            "{n:>7} {:>6.3}% {:>7.3}%  {pattern}",
            100.0 * *n as f64 / total as f64,
            100.0 * sum as f64 / total as f64
        );
    }
}

/// What kinds of dissonance G5-PASSING forbids in Palestrina's free voices, with examples:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture classify`.
#[test]
#[ignore]
fn classify_dissonances() {
    if !corpus_present() {
        return;
    }
    use fux_core::Interval;
    use fux_core::rules::{Analysis, Checker, Profile};
    use std::collections::BTreeMap;
    let profile = Profile::fux();
    let checker = Checker::new(&profile);
    let mut kinds: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    for (name, piece) in calibration_pieces() {
        let score = &piece.score;
        let analysis = Analysis::new(score);
        let report = checker.check(score);
        for v in report.violations.iter().filter(|v| {
            v.is_forbidden()
                && std::env::var("RULE").map_or(v.rule == "G5-PASSING", |r| v.rule == r)
        }) {
            let line = &analysis.lines[v.voice];
            let Some(i) = line.iter().position(|s| {
                s.note <= v.note
                    && v.note < s.note + 4
                    && s.start == score.note_start(v.voice, s.note).unwrap()
            }) else {
                continue;
            };
            // The sounding note that contains the written note.
            let start = score.note_start(v.voice, v.note).unwrap();
            let i = line
                .iter()
                .position(|s| s.start <= start && start < s.end)
                .unwrap_or(i);
            let note = line[i];
            let before = (i > 0 && line[i - 1].end == note.start).then(|| line[i - 1]);
            let after = line.get(i + 1).copied().filter(|n| n.start == note.end);
            let step = |a: Option<fux_core::Sounding>, b: Option<fux_core::Sounding>| match (a, b) {
                (Some(a), Some(b)) => {
                    let s = Interval::between(a.pitch, b.pitch).steps;
                    match s {
                        0 => "=".to_string(),
                        1 => "+1".into(),
                        -1 => "-1".into(),
                        s if s > 0 => "leap+".into(),
                        _ => "leap-".into(),
                    }
                }
                _ => "rest".into(),
            };
            let figure = format!("{}/{}", step(before, Some(note)), step(Some(note), after));
            let length = note.end - note.start;
            let at = note.start % 8;
            let (bass, b) = analysis.bass_at(note.start).unwrap();
            let held_bass = b.start < note.start;
            let over_bass = if bass == v.voice {
                "is bass".to_string()
            } else {
                Interval::between(b.pitch, note.pitch).simple().short_name()
            };
            // Dissonances against the other voices (not the bass).
            let slice = analysis.slice_at(note.start).unwrap();
            let mut others: Vec<String> = Vec::new();
            for (u, o) in slice.sounding.iter().enumerate() {
                let Some(o) = o else { continue };
                if u == v.voice || u == bass {
                    continue;
                }
                let iv = Interval::between(o.pitch, note.pitch);
                if !iv.is_consonant() {
                    let s = iv.abs().simple();
                    if !(s.steps == 3 || (s.steps == 4 && s.semitones == 6)) {
                        others.push(format!(
                            "{}{}",
                            s.short_name(),
                            if o.start < note.start { "h" } else { "" }
                        ));
                    }
                }
            }
            let kind = format!(
                "len {length} at {at} {figure} bass {over_bass}{} upper [{}]",
                if held_bass { " held" } else { "" },
                others.join(",")
            );
            let entry = kinds.entry(kind).or_default();
            entry.0 += 1;
            if entry.1.len() < 3 {
                let (m, _) = score.meter.locate(note.start);
                entry
                    .1
                    .push(format!("{name} {} {}", m + 1, score.voices[v.voice].name));
            }
        }
    }
    let mut rows: Vec<_> = kinds.into_iter().collect();
    rows.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (kind, (n, examples)) in rows.iter().take(60) {
        println!("{n:>5}  {kind}   {}", examples.join(" | "));
    }
}

/// How Palestrina's sections end: the degree of the last bass note above the final of the mode,
/// and the third of the final sonority, also by the voices of the movement; then how often
/// TEX-FINAL-CHORD finds a final chord without a major third (`free_missing_third`), which judges
/// the last sonority of the movement:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture endings`.
#[test]
#[ignore]
fn endings() {
    if !corpus_present() {
        return;
    }
    use fux_core::Interval;
    use fux_core::rules::{Checker, Profile};
    use std::collections::BTreeMap;
    let mut degrees: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut thirds: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_voices: BTreeMap<(usize, String), usize> = BTreeMap::new();
    let mut other: BTreeMap<String, usize> = BTreeMap::new();
    let mut profile = Profile::fux();
    profile
        .apply_override("TEX-FINAL-CHORD.free_missing_third=P(13)")
        .unwrap();
    let checker = Checker::new(&profile);
    let mut missing: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    let (mut silent, mut silent_cases) = (0, 0);
    for (_, piece) in calibration_pieces() {
        let score = &piece.score;
        let cases = checker
            .check(score)
            .violations
            .iter()
            .filter(|v| v.rule == "TEX-FINAL-CHORD" && v.level.cost() == 13)
            .count();
        let entry = missing.entry(score.voices.len()).or_default();
        entry.0 += 1;
        entry.1 += cases;
        // The last note of each voice that sounds at the end.
        let end = score.total_ticks();
        let last: Vec<fux_core::Pitch> = score
            .voices
            .iter()
            .filter_map(|v| {
                let (start, note) = v.timed().last()?;
                (start + note.duration.ticks() == end)
                    .then_some(note.pitch)
                    .flatten()
            })
            .collect();
        let Some(bass) = last.iter().min_by_key(|p| p.midi()).copied() else {
            silent += 1;
            silent_cases += cases;
            continue;
        };
        let final_pitch = score.mode.final_pitch(bass.octave);
        let degree = Interval::between(final_pitch, bass).simple().short_name();
        *degrees.entry((score.mode.id(), degree)).or_default() += 1;
        let mut third = "none".to_string();
        for p in &last {
            let s = Interval::between(bass, *p).simple();
            match (s.steps, s.semitones) {
                (0, 0) | (4, 7) => {}
                (2, 4) => third = "major".into(),
                (2, 3) => third = "minor".into(),
                _ => *other.entry(s.short_name()).or_default() += 1,
            }
        }
        *by_voices
            .entry((score.voices.len(), third.clone()))
            .or_default() += 1;
        *thirds.entry(third).or_default() += 1;
    }
    for ((mode, degree), n) in &degrees {
        println!("{mode:<24} {degree:<4} {n}");
    }
    println!("thirds {thirds:?}");
    println!("thirds by voices {by_voices:?}");
    println!("other intervals above the bass {other:?}");
    println!(
        "{silent} movements without a note at their end, with {silent_cases} cases of the rule"
    );
    for (voices, (movements, cases)) in &missing {
        println!(
            "{voices} voices: {cases} of {movements} final chords without a major third ({:.1} %)",
            100.0 * *cases as f64 / *movements as f64
        );
    }
}

/// What kinds of alterations FIC-SCOPE forbids in Palestrina, with examples:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture classify_ficta`.
#[test]
#[ignore]
fn classify_ficta() {
    if !corpus_present() {
        return;
    }
    use fux_core::Interval;
    use fux_core::rules::{Analysis, Checker, Profile};
    use std::collections::BTreeMap;
    let profile = Profile::fux();
    let checker = Checker::new(&profile);
    let mut kinds: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    for (name, piece) in calibration_pieces() {
        let score = &piece.score;
        let mode = score.mode;
        let analysis = Analysis::new(score);
        let report = checker.check(score);
        for v in report.violations.iter().filter(|v| v.rule == "FIC-SCOPE") {
            let start = score.note_start(v.voice, v.note).unwrap();
            let line = &analysis.lines[v.voice];
            let i = line
                .iter()
                .position(|s| s.start <= start && start < s.end)
                .unwrap();
            let note = line[i];
            let raised = note.pitch.alter > mode.scale_alter(note.pitch.step);
            let after = line.get(i + 1).copied().filter(|n| n.start == note.end);
            let next = match after {
                Some(a) => {
                    let s = Interval::between(note.pitch, a.pitch);
                    match (s.steps, s.semitones) {
                        (1, 1) => "up semitone",
                        (1, _) => "up step",
                        (-1, _) => "down step",
                        (0, 0) => "same",
                        (s, _) if s > 0 => "leap up",
                        _ => "leap down",
                    }
                }
                None => "rest/end",
            };
            let (bass, b) = analysis.bass_at(note.start).unwrap();
            let over_bass = if bass == v.voice {
                "bass".to_string()
            } else {
                Interval::between(b.pitch, note.pitch).simple().short_name()
            };
            let degree = Interval::between(mode.final_pitch(note.pitch.octave), note.pitch)
                .simple()
                .short_name();
            let last = i + 1 == line.len();
            let kind = format!(
                "{} {degree} next {next} over bass {over_bass}{}",
                if raised { "raised" } else { "lowered" },
                if last { " LAST" } else { "" }
            );
            let entry = kinds.entry(kind).or_default();
            entry.0 += 1;
            if entry.1.len() < 2 {
                let (m, _) = score.meter.locate(note.start);
                entry.1.push(format!(
                    "{name} {} {} {}",
                    m + 1,
                    score.voices[v.voice].name,
                    mode.id()
                ));
            }
        }
    }
    let mut rows: Vec<_> = kinds.into_iter().collect();
    rows.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (kind, (n, examples)) in rows.iter().take(45) {
        println!("{n:>5}  {kind}   {}", examples.join(" | "));
    }
}

/// Opportunities for the texture rules: downbeats on which three or more voices sound and all
/// strike, by number of voices sounding:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture opportunities`.
#[test]
#[ignore]
fn opportunities() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::Analysis;
    use std::collections::BTreeMap;
    let mut struck: BTreeMap<usize, usize> = BTreeMap::new();
    let mut downbeats: BTreeMap<usize, usize> = BTreeMap::new();
    for (_, piece) in calibration_pieces() {
        let score = &piece.score;
        let analysis = Analysis::new(score);
        let per = score.meter.ticks_per_measure();
        for slice in &analysis.slices {
            if !slice.tick.is_multiple_of(per) {
                continue;
            }
            let notes: Vec<_> = slice.sounding.iter().flatten().collect();
            *downbeats.entry(notes.len()).or_default() += 1;
            if notes.len() >= 3 && notes.iter().all(|n| n.start == slice.tick) {
                *struck.entry(score.voices.len()).or_default() += 1;
            }
        }
    }
    println!("struck downbeat sonorities by voices of the piece: {struck:?}");
    println!("downbeats by voices sounding: {downbeats:?}");
}

/// The sonorities TEX-TRIAD judges, on which the levels of its `free_*` parameters were counted:
/// downbeats of the inner measures on which three or more voices sound and all strike, by the
/// voices of the movement and by their kind over the bass, with each kind's share of all and of
/// the consonant ones; the same for the inner downbeats on which a voice holds, which the rule
/// does not judge; then the cases of the rule itself, each level at a cost of its own, which must
/// be as many as the struck sonorities of their kinds:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture struck_sonorities`.
#[test]
#[ignore]
fn struck_sonorities() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::{Analysis, Checker, Profile, harmonic};
    use std::collections::BTreeMap;
    const KINDS: [&str; 6] = [
        "complete",
        "sixth",
        "no fifth",
        "no third",
        "bare sixth",
        "dissonant",
    ];
    // As TEX-TRIAD's `kind` sorts the simple steps of the upper notes above the bass.
    let kind = |steps: &[i32]| {
        let has = |s: i32| steps.contains(&s);
        if steps.iter().any(|s| ![0, 2, 4, 5].contains(s)) || (has(4) && has(5)) {
            return 5;
        }
        match (has(2), has(4), has(5)) {
            (true, true, _) => 0,
            (true, false, true) => 1,
            (true, false, false) => 2,
            (false, _, false) => 3,
            (false, _, true) => 4,
        }
    };
    let pieces = calibration_pieces();
    let mut counts: BTreeMap<usize, [usize; 6]> = BTreeMap::new();
    let mut held: BTreeMap<usize, [usize; 6]> = BTreeMap::new();
    for (_, piece) in &pieces {
        let score = &piece.score;
        let analysis = Analysis::new(score);
        let per = score.meter.ticks_per_measure();
        for slice in &analysis.slices {
            let tick = slice.tick;
            if !tick.is_multiple_of(per)
                || analysis.in_first_measure(tick)
                || analysis.in_last_measure(tick)
            {
                continue;
            }
            let notes: Vec<(usize, fux_core::Sounding)> = slice
                .sounding
                .iter()
                .enumerate()
                .filter_map(|(v, s)| s.map(|s| (v, s)))
                .collect();
            if notes.len() < 3 {
                continue;
            }
            let Some(bass) = slice.lowest() else {
                continue;
            };
            let steps: Vec<i32> = notes
                .iter()
                .filter(|(v, _)| *v != bass.0)
                .map(|(_, s)| harmonic(bass.1, *s).simple().steps)
                .collect();
            let table = if notes.iter().all(|(_, s)| s.start == tick) {
                &mut counts
            } else {
                &mut held
            };
            table.entry(score.voices.len()).or_default()[kind(&steps)] += 1;
        }
    }
    for (what, table) in [("struck", &counts), ("held", &held)] {
        let mut four_and_more = [0; 6];
        for row in table.range(4..).map(|(_, row)| row) {
            for (sum, n) in four_and_more.iter_mut().zip(row) {
                *sum += n;
            }
        }
        let rows: Vec<(String, [usize; 6])> = table
            .iter()
            .map(|(voices, row)| (voices.to_string(), *row))
            .chain([("4+".to_string(), four_and_more)])
            .collect();
        println!(
            "voices {what:>6} consonant  {}: n (% of all / of the consonant)",
            KINDS.join(" / ")
        );
        for (voices, row) in &rows {
            let all: usize = row.iter().sum();
            let consonant = all - row[5];
            let share = |n: usize, of: usize| 100.0 * n as f64 / of as f64;
            let shares: Vec<String> = row[..5]
                .iter()
                .map(|&n| format!("{n} ({:.1} / {:.1} %)", share(n, all), share(n, consonant)))
                .collect();
            println!(
                "{voices:<6} {all:>6} {consonant:>9}  {} / {} ({:.1} % of all) dissonant",
                shares.join(" / "),
                row[5],
                share(row[5], all)
            );
        }
    }
    // The rule's cases, sixth chord, third with octaves, no third and bare sixth.
    let levels = [
        ("sixth", 3),
        ("free_no_fifth", 5),
        ("free_no_third", 7),
        ("free_bare_sixth", 11),
    ];
    let mut profile = Profile::fux();
    for (param, cost) in levels {
        profile
            .apply_override(&format!("TEX-TRIAD.{param}=P({cost})"))
            .unwrap();
    }
    let checker = Checker::new(&profile);
    let mut cases: BTreeMap<usize, [usize; 6]> = BTreeMap::new();
    for (_, piece) in &pieces {
        for v in checker.check(&piece.score).violations {
            if v.rule != "TEX-TRIAD" {
                continue;
            }
            let k = levels
                .iter()
                .position(|(_, cost)| *cost == v.level.cost())
                .unwrap();
            cases.entry(piece.score.voices.len()).or_default()[k + 1] += 1;
        }
    }
    for (voices, row) in &counts {
        let found = cases.get(voices).copied().unwrap_or_default();
        assert_eq!(found[1..5], row[1..5], "{voices} voices");
    }
}

/// Two leaps in the same direction that outline a triad, and how often the larger lies above:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture triad_leaps`.
#[test]
#[ignore]
fn triad_leaps() {
    if !corpus_present() {
        return;
    }
    use fux_core::Interval;
    let (mut all, mut larger_above) = (0, 0);
    for (_, piece) in calibration_pieces() {
        for voice in &piece.score.voices {
            let line = voice.soundings();
            for i in 2..line.len() {
                let (a, b, c) = (line[i - 2], line[i - 1], line[i]);
                if a.end != b.start || b.end != c.start {
                    continue;
                }
                let first = Interval::between(a.pitch, b.pitch);
                let second = Interval::between(b.pitch, c.pitch);
                if !(first.is_leap() && second.is_leap() && first.direction() == second.direction())
                {
                    continue;
                }
                let (lower, upper) = if first.direction() > 0 {
                    (first, second)
                } else {
                    (second, first)
                };
                let key = (
                    (lower.abs().steps, lower.abs().semitones),
                    (upper.abs().steps, upper.abs().semitones),
                );
                let triad = matches!(
                    key,
                    ((2, 4), (2, 3))
                        | ((2, 3), (2, 4))
                        | ((2, 3), (3, 5))
                        | ((2, 4), (3, 5))
                        | ((3, 5), (2, 4))
                        | ((3, 5), (2, 3))
                        | ((4, 7), (3, 5))
                        | ((3, 5), (4, 7))
                );
                if triad {
                    all += 1;
                    if upper.abs().steps > lower.abs().steps {
                        larger_above += 1;
                    }
                }
            }
        }
    }
    println!("triad leaps {all}, larger above {larger_above}");
}

/// The same on many movements (slow): `cargo test --release -p fux-core --test palestrina --
/// --ignored window_by_window_on_many`.
#[test]
#[ignore]
fn window_by_window_on_many() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::{Checker, Profile};
    let checker = Checker::new(&Profile::fux());
    let step = std::env::var("STEP").map_or(25, |s| s.parse().unwrap());
    for (name, piece) in calibration_pieces().into_iter().step_by(step) {
        assert_windows_partition(&checker, &piece.score, &name);
        assert_prefixes_collect(&checker, &piece.score, &name);
    }
}

/// The same with the profile of a mass (phase 14, D7, `fux_core::mass::profile`), whose held
/// notes and halves of the choir are judged without a plan too, as the movements of the corpus
/// are, at the levels their cases have with one (slow): `cargo test --release -p fux-core --test
/// palestrina -- --ignored window_by_window_on_many_with_the_mass_overlay`.
#[test]
#[ignore]
fn window_by_window_on_many_with_the_mass_overlay() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::{Checker, Profile};
    let checker = Checker::new(&fux_core::mass::profile(&Profile::fux()));
    let step = std::env::var("STEP").map_or(25, |s| s.parse().unwrap());
    let mut checked = 0;
    for (name, piece) in calibration_pieces().into_iter().step_by(step) {
        assert_windows_partition(&checker, &piece.score, &name);
        assert_prefixes_collect(&checker, &piece.score, &name);
        checked += 1;
    }
    println!("{checked} movements window by window and prefix by prefix");
}

/// Whether two pitches sounding together are dissonant: a second, a seventh, a tritone or an
/// altered interval, or a fourth whose lower note is `bass`, the lowest sounding pitch (as
/// `imitation_corpus.rs` counts the occasions of IMI-HELD).
fn dissonant_pair(a: fux_core::Pitch, b: fux_core::Pitch, bass: fux_core::Pitch) -> bool {
    let (low, high) = if (a.diatonic(), a.midi()) <= (b.diatonic(), b.midi()) {
        (a, b)
    } else {
        (b, a)
    };
    let interval = fux_core::Interval::between(low, high).simple();
    let fourth = interval.steps == 3 && interval.semitones == 5;
    matches!(interval.steps, 1 | 6)
        || interval.is_tritone()
        || (fourth && low == bass)
        || !(interval.is_consonant() || fourth || interval.is_tritone())
}

/// The parameters for settings without a plan (phase 14) on Palestrina's movements, which have
/// none. IMI-HELD finds without a plan exactly the cases it finds with a plan of one point
/// without entries, on which the levels of `struck` and `passing` were counted
/// (`held_dissonances_and_half_textures_in_the_corpus` in `imitation_corpus.rs`), against the
/// same occasions. IMI-SILENCE `unplanned_thin` finds exactly the downbeats after every voice's
/// first note, before the last measure, on which half of a choir of four or six sounds; the
/// share is counted per movement, as the rule judges a part of a mass, and per section, as
/// `thin` was counted after the last first entry of each:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture
/// unplanned_held_and_thin_in_the_corpus`.
#[test]
#[ignore]
fn unplanned_held_and_thin_in_the_corpus() {
    if !corpus_present() {
        return;
    }
    use fux_core::imitation::detect::sections;
    use fux_core::rules::{Analysis, Checker, Detail, ParamKind, Profile, Violation, all_rules};
    use fux_core::score::{ScoreImitation, ScorePoint};
    use std::sync::Arc;

    let started = std::time::Instant::now();
    // A checker of the named parameters alone, at P1.
    let only = |params: &[(&str, &str)]| {
        let mut profile = Profile::fux();
        for rule in all_rules() {
            let id = rule.info().id;
            for param in rule.info().params.iter() {
                if param.kind == ParamKind::Level {
                    let level = if params.contains(&(id, param.name)) {
                        "P1"
                    } else {
                        "aus"
                    };
                    let spec = format!("{id}.{}={level}", param.name);
                    profile.apply_override(&spec).unwrap();
                }
            }
        }
        Checker::new(&profile)
    };
    let unplanned = only(&[
        ("IMI-HELD", "unplanned_struck"),
        ("IMI-HELD", "unplanned_passing"),
        ("IMI-SILENCE", "unplanned_thin"),
    ]);
    let planned = only(&[("IMI-HELD", "struck"), ("IMI-HELD", "passing")]);
    let pointless = Arc::new(ScoreImitation {
        points: vec![ScorePoint {
            words: 0..0,
            entries: Vec::new(),
            goal: None,
        }],
    });
    // A case as (rule, parameter without `unplanned_`, tick, voice, note).
    let key = |v: &Violation| {
        let Detail::Param(param) = v.detail else {
            panic!("{}: {:?}", v.rule, v.detail);
        };
        let param = param.trim_start_matches("unplanned_");
        (v.rule, param, v.tick, v.voice, v.note)
    };
    let mut cases: BTreeMap<&'static str, usize> = BTreeMap::new();
    let (mut struck_occasions, mut later_occasions) = (0usize, 0usize);
    // Per number of voices: the downbeats judged and those with half of the choir, per movement
    // and per section.
    let mut by_movement: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    let mut by_section: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    let pieces = calibration_pieces();
    for (name, piece) in &pieces {
        let score = &piece.score;
        assert!(score.imitation.is_none(), "{name}");
        let found = unplanned.check(score).violations;
        for v in &found {
            let (_, param, ..) = key(v);
            *cases.entry(param).or_default() += 1;
        }
        let mut with_plan = score.clone();
        with_plan.imitation = Some(Arc::clone(&pointless));
        let held: Vec<_> = found
            .iter()
            .filter(|v| v.rule == "IMI-HELD")
            .map(key)
            .collect();
        let counterparts: Vec<_> = planned
            .check(&with_plan)
            .violations
            .iter()
            .map(key)
            .collect();
        assert_eq!(held, counterparts, "{name}");
        let analysis = Analysis::new(score);
        let per = analysis.per_measure;
        let n = score.voices.len();
        // The occasions of IMI-HELD: dissonant attacks against a note tied over the downbeat.
        for slice in &analysis.slices {
            let Some(bass) = slice.lowest() else { continue };
            let downbeat = slice.tick / per * per;
            for (v, note) in slice.sounding.iter().enumerate() {
                let Some(note) = note.filter(|s| s.start == slice.tick && s.end - s.start > 1)
                else {
                    continue;
                };
                for (_, held) in slice.sounding.iter().enumerate().filter(|&(u, _)| u != v) {
                    let Some(held) = held.filter(|h| h.start < downbeat) else {
                        continue;
                    };
                    if dissonant_pair(note.pitch, held.pitch, bass.1.pitch) {
                        if slice.tick == downbeat {
                            struck_occasions += 1;
                        } else {
                            later_occasions += 1;
                        }
                    }
                }
            }
        }
        if n < 4 || n % 2 != 0 {
            continue;
        }
        let sounding = |tick: u32| {
            analysis
                .lines
                .iter()
                .filter(|line| line.iter().any(|s| s.start <= tick && tick < s.end))
                .count()
        };
        // Per movement: from the latest first note on, as the rule judges.
        let firsts: Option<Vec<u32>> = analysis
            .lines
            .iter()
            .map(|line| line.first().map(|s| s.start))
            .collect();
        let thin = found.iter().filter(|v| v.rule == "IMI-SILENCE").count();
        let mut halves = 0;
        if let Some(entered) = firsts.and_then(|f| f.into_iter().max()) {
            let entry = by_movement.entry(n).or_default();
            let mut tick = entered.div_ceil(per) * per;
            while tick + per < analysis.total {
                entry.0 += 1;
                if 2 * sounding(tick) == n {
                    entry.1 += 1;
                    halves += 1;
                }
                tick += per;
            }
        }
        assert_eq!(thin, halves, "{name}");
        // Per section: from the last first entry of each, as `thin` was counted.
        for span in sections(score) {
            let firsts: Option<Vec<u32>> = analysis
                .lines
                .iter()
                .map(|line| {
                    line.iter()
                        .find(|s| s.start >= span.start && s.start < span.end)
                        .map(|s| s.start)
                })
                .collect();
            let Some(entered) = firsts.and_then(|f| f.into_iter().max()) else {
                continue;
            };
            let entry = by_section.entry(n).or_default();
            let mut tick = entered.div_ceil(per) * per;
            while tick < span.end.saturating_sub(per) {
                entry.0 += 1;
                if 2 * sounding(tick) == n {
                    entry.1 += 1;
                }
                tick += per;
            }
        }
    }
    let percent = |part: usize, whole: usize| 100.0 * part as f64 / whole as f64;
    let count = |param: &str| cases.get(param).copied().unwrap_or(0);
    let (struck, passing) = (count("struck"), count("passing"));
    println!("{} movements without a plan", pieces.len());
    println!(
        "IMI-HELD unplanned_struck: {struck} of {struck_occasions} dissonant downbeat attacks against a tied note ({:.3} %)",
        percent(struck, struck_occasions)
    );
    println!(
        "IMI-HELD unplanned_passing: {passing} of {later_occasions} dissonant later attacks against a tied note ({:.3} %)",
        percent(passing, later_occasions)
    );
    for (n, (downbeats, halves)) in &by_movement {
        let (sections, section_halves) = by_section[n];
        println!(
            "IMI-SILENCE unplanned_thin, {n} voices: {halves} of {downbeats} downbeats after every voice's first note ({:.1} %); per section after its last first entry {section_halves} of {sections} ({:.1} %)",
            percent(*halves, *downbeats),
            percent(section_halves, sections)
        );
    }
    println!("{:.1} s", started.elapsed().as_secs_f64());
    // The levels of the profile of a mass by the Leitlinie: under about 0.1 % a prohibition, up
    // to 2 % P3, over 10 % P1.
    assert!(percent(struck, struck_occasions) < 0.1, "struck");
    assert!(
        (0.1..2.0).contains(&percent(passing, later_occasions)),
        "passing"
    );
    let (downbeats, halves) = by_movement[&4];
    assert!(percent(halves, downbeats) > 10.0, "four voices, half");
}

/// Every raised note of the corpus by what follows and where it stands over the bass:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture raised_notes`.
#[test]
#[ignore]
fn raised_notes() {
    if !corpus_present() {
        return;
    }
    use fux_core::Interval;
    use fux_core::rules::Analysis;
    use std::collections::BTreeMap;
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0;
    let mut measures = 0;
    for (_, piece) in calibration_pieces() {
        let score = &piece.score;
        let mode = score.mode;
        let analysis = Analysis::new(score);
        measures += score.measure_count() * score.voices.len();
        for (v, line) in analysis.lines.iter().enumerate() {
            for (i, note) in line.iter().enumerate() {
                if note.pitch.alter <= mode.scale_alter(note.pitch.step) {
                    continue;
                }
                total += 1;
                let next = match line.get(i + 1).filter(|n| n.start == note.end) {
                    Some(a) => {
                        let s = Interval::between(note.pitch, a.pitch);
                        match (s.steps, s.semitones) {
                            (1, 1) => "up semitone",
                            (1, _) => "up step",
                            (-1, _) => "down step",
                            (0, 0) => "same",
                            (s, _) if s > 0 => "leap up",
                            _ => "leap down",
                        }
                    }
                    None => "rest/end",
                };
                let prev = match i
                    .checked_sub(1)
                    .map(|j| line[j])
                    .filter(|p| p.end == note.start)
                {
                    Some(p) => {
                        let s = Interval::between(p.pitch, note.pitch);
                        match (s.steps, s.semitones) {
                            (1, _) => "from below step",
                            (-1, _) => "from above step",
                            (0, 0) => "from same",
                            (s, _) if s > 0 => "from below leap",
                            _ => "from above leap",
                        }
                    }
                    None => "after rest",
                };
                let over_bass = match analysis.bass_at(note.start) {
                    Some((bass, _)) if bass == v => "bass".to_string(),
                    Some((_, b)) => Interval::between(b.pitch, note.pitch).simple().short_name(),
                    None => "-".into(),
                };
                let degree = Interval::between(mode.final_pitch(note.pitch.octave), note.pitch)
                    .simple()
                    .short_name();
                let last = if i + 1 == line.len() { " LAST" } else { "" };
                *kinds
                    .entry(format!(
                        "{degree} {prev} next {next} over {over_bass}{last}"
                    ))
                    .or_default() += 1;
            }
        }
    }
    println!("{total} raised notes in {measures} voice-measures");
    let mut by_next: BTreeMap<String, usize> = BTreeMap::new();
    for (kind, n) in &kinds {
        let next = kind
            .split(" next ")
            .nth(1)
            .unwrap_or("")
            .split(" over ")
            .next()
            .unwrap_or("");
        let over = if kind.contains(" over bass") {
            "in bass"
        } else if kind.contains(" over M3") || kind.contains(" over M10") {
            "M3 over bass"
        } else {
            "other"
        };
        *by_next.entry(format!("{next} / {over}")).or_default() += n;
    }
    for (k, n) in &by_next {
        println!("{n:>6}  {k}");
    }
    let mut rows: Vec<_> = kinds.into_iter().collect();
    rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (kind, n) in rows.iter().take(70) {
        println!("{n:>6}  {kind}");
    }
}

/// Where leading tones of the corpus (raised notes rising by a semitone) arrive, and whether a
/// second voice arrives with them on the octave or unison of the target, coming by step from
/// above or as the bass by a fourth or fifth:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture leading_tones`.
#[test]
#[ignore]
fn leading_tones() {
    if !corpus_present() {
        return;
    }
    use fux_core::Interval;
    use fux_core::rules::Analysis;
    use std::collections::BTreeMap;
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for (_, piece) in calibration_pieces() {
        let score = &piece.score;
        let mode = score.mode;
        let analysis = Analysis::new(score);
        let per = analysis.per_measure;
        for (v, line) in analysis.lines.iter().enumerate() {
            for (i, note) in line.iter().enumerate() {
                if note.pitch.alter <= mode.scale_alter(note.pitch.step) {
                    continue;
                }
                let Some(next) = line.get(i + 1).filter(|n| n.start == note.end) else {
                    continue;
                };
                let step = Interval::between(note.pitch, next.pitch);
                if (step.steps, step.semitones) != (1, 1) {
                    continue;
                }
                let at = next.start;
                let position = match at % per {
                    0 => "downbeat",
                    4 => "half",
                    _ => "quarter",
                };
                // Another voice reaching the target's pitch class at `at`.
                let mut partner = "none";
                for (w, other) in analysis.lines.iter().enumerate() {
                    if w == v {
                        continue;
                    }
                    let Some(j) = other.iter().position(|s| s.start == at) else {
                        continue;
                    };
                    if !other[j].pitch.same_class(next.pitch) || j == 0 {
                        continue;
                    }
                    let before = other[j - 1];
                    if before.end != at {
                        continue;
                    }
                    let came = Interval::between(before.pitch, other[j].pitch);
                    let kind = match (came.steps, came.semitones.abs()) {
                        (-1, _) => "tenorizans",
                        (s, _) if s.abs() == 3 || s.abs() == 4 => "basizans",
                        (0, 0) => "held",
                        _ => "other",
                    };
                    if partner == "none" || kind == "tenorizans" {
                        partner = kind;
                    }
                }
                let value = match note.end - note.start {
                    1 => "8",
                    2 => "4",
                    3 => "4.",
                    4 => "2",
                    6 => "2.",
                    8 => "1",
                    _ => "long",
                };
                *kinds
                    .entry(format!("{position:<8} {partner:<10} value {value}"))
                    .or_default() += 1;
            }
        }
    }
    let mut rows: Vec<_> = kinds.into_iter().collect();
    rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    let total: usize = rows.iter().map(|(_, n)| n).sum();
    println!("{total} leading tones");
    for (kind, n) in rows.iter().take(40) {
        println!("{n:>6}  {kind}");
    }
}

/// How long Palestrina's notes sound (ties merged), apart from each voice's last note, and how
/// voices begin: `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture
/// durations_and_entries`.
#[test]
#[ignore]
fn durations_and_entries() {
    if !corpus_present() {
        return;
    }
    use fux_core::Interval;
    use fux_core::rules::Analysis;
    use std::collections::BTreeMap;
    let mut lengths: BTreeMap<u32, usize> = BTreeMap::new();
    let mut entries: BTreeMap<String, usize> = BTreeMap::new();
    let mut first_values: BTreeMap<u32, usize> = BTreeMap::new();
    let mut notes = 0;
    for (_, piece) in calibration_pieces() {
        let score = &piece.score;
        let analysis = Analysis::new(score);
        // The degrees relative to the last note of the lowest voice: the encodings name modes
        // whose final is often a fifth away from where the movement ends.
        let Some(last) = analysis
            .lines
            .last()
            .and_then(|l| l.last())
            .map(|s| s.pitch)
        else {
            continue;
        };
        let final_pitch = |octave: i8| fux_core::Pitch::new(last.step, last.alter, octave);
        // Only movements that end on the final of the mode the encoding names.
        if std::env::var_os("ON_FINAL").is_some() && last.step != score.mode.final_step() {
            continue;
        }
        for line in &analysis.lines {
            for (i, s) in line.iter().enumerate() {
                if i + 1 < line.len() {
                    *lengths.entry(s.end - s.start).or_default() += 1;
                    notes += 1;
                }
                // An entry: the first note, or a note after a rest of a measure or more.
                let entry = i == 0 || s.start >= line[i - 1].end + analysis.per_measure;
                if entry {
                    let degree = Interval::between(final_pitch(s.pitch.octave), s.pitch).simple();
                    let degree = (
                        degree.steps.rem_euclid(7) + 1,
                        degree.semitones.rem_euclid(12),
                    );
                    *entries
                        .entry(format!(
                            "{} degree {} ({} st)",
                            if i == 0 { "first" } else { "re-entry" },
                            degree.0,
                            degree.1
                        ))
                        .or_default() += 1;
                    if i == 0 {
                        *first_values.entry(s.end - s.start).or_default() += 1;
                    }
                }
            }
        }
    }
    println!("{notes} notes (without the last of each voice), by length in eighths:");
    for (ticks, n) in &lengths {
        println!(
            "{ticks:>4} {n:>8} {:>8.3}%",
            100.0 * *n as f64 / notes as f64
        );
    }
    println!("entries:");
    let mut rows: Vec<_> = entries.into_iter().collect();
    rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (kind, n) in rows {
        println!("{n:>7}  {kind}");
    }
    println!("first notes by length: {first_values:?}");
}

/// The suspensions FREE-RESOLUTION forbids in the corpus, by how the note is held and left:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture held_dissonances`.
#[test]
#[ignore]
fn held_dissonances() {
    if !corpus_present() {
        return;
    }
    use fux_core::Interval;
    use fux_core::rules::{Analysis, Checker, Profile};
    use std::collections::BTreeMap;
    let checker = Checker::new(&Profile::fux());
    let mut kinds: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    for (name, piece) in calibration_pieces() {
        let score = &piece.score;
        let analysis = Analysis::new(score);
        let per = analysis.per_measure;
        for v in checker
            .check(score)
            .violations
            .iter()
            .filter(|v| v.rule == "FREE-RESOLUTION" && v.is_forbidden())
        {
            let start = score.note_start(v.voice, v.note).unwrap();
            let line = &analysis.lines[v.voice];
            let i = line
                .iter()
                .position(|s| s.start <= start && start < s.end)
                .unwrap();
            let note = line[i];
            let on_beat = note.start.is_multiple_of(per);
            let downbeats = (note.end - 1) / per - note.start / per;
            let after = line.get(i + 1).filter(|n| n.start == note.end);
            let next = match after {
                Some(a) => {
                    let s = Interval::between(note.pitch, a.pitch);
                    match s.steps {
                        -1 => "step down",
                        1 => "step up",
                        0 => "same",
                        s if s > 0 => "leap up",
                        _ => "leap down",
                    }
                }
                None => "rest/end",
            };
            let length = note.end - note.start;
            let kind = format!(
                "{} len {length:>2} over {downbeats} downbeat(s), next {next}",
                if on_beat { "on beat " } else { "off beat" }
            );
            let entry = kinds.entry(kind).or_default();
            entry.0 += 1;
            if entry.1.len() < 2 {
                let (m, _) = score.meter.locate(note.start);
                entry
                    .1
                    .push(format!("{name} m{} {}", m + 1, score.voices[v.voice].name));
            }
        }
    }
    let mut rows: Vec<_> = kinds.into_iter().collect();
    rows.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (kind, (n, examples)) in rows.iter().take(40) {
        println!("{n:>5}  {kind}   {}", examples.join(" | "));
    }
}

/// How soon a voice sings a step again with another alteration (B♭ … B), by notes between and
/// the time from one to the other:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture near_chromatic`.
#[test]
#[ignore]
fn near_chromatic() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::Analysis;
    use std::collections::BTreeMap;
    let mut by_notes: BTreeMap<usize, usize> = BTreeMap::new();
    let mut by_measures: BTreeMap<u32, usize> = BTreeMap::new();
    let mut examples: Vec<String> = Vec::new();
    let mut measures = 0;
    for (name, piece) in calibration_pieces() {
        let score = &piece.score;
        let analysis = Analysis::new(score);
        let per = analysis.per_measure;
        measures += score.measure_count() * score.voices.len();
        for (v, line) in analysis.lines.iter().enumerate() {
            for (i, a) in line.iter().enumerate() {
                // The nearest later note on the same step, within six notes.
                for (k, b) in line.iter().enumerate().skip(i + 1).take(6) {
                    if b.pitch.step != a.pitch.step {
                        continue;
                    }
                    if b.pitch.alter != a.pitch.alter {
                        *by_notes.entry(k - i - 1).or_default() += 1;
                        *by_measures.entry((b.start - a.end) / per).or_default() += 1;
                        if k - i == 2 && examples.len() < 8 {
                            let (m, _) = score.meter.locate(a.start);
                            examples.push(format!(
                                "{name} m{} {} {}–{}",
                                m + 1,
                                score.voices[v].name,
                                a.pitch,
                                b.pitch
                            ));
                        }
                    }
                    break;
                }
            }
        }
    }
    println!("{measures} voice-measures");
    println!("by notes between: {by_notes:?}");
    println!("by whole measures between: {by_measures:?}");
    println!("{examples:#?}");
}

/// Unisons in the final sonority of the corpus, and at downbeats inside the movements:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture final_unisons`.
#[test]
#[ignore]
fn final_unisons() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::Analysis;
    let (mut finals, mut with_unison, mut bass_unison) = (0, 0, 0);
    for (_, piece) in calibration_pieces() {
        let analysis = Analysis::new(&piece.score);
        let Some(last) = analysis.slices.last() else {
            continue;
        };
        let notes: Vec<_> = last.sounding.iter().flatten().collect();
        finals += 1;
        let unison = (0..notes.len())
            .any(|a| (a + 1..notes.len()).any(|b| notes[a].pitch == notes[b].pitch));
        if unison {
            with_unison += 1;
        }
        if let Some((_, bass)) = last.lowest()
            && notes.iter().filter(|n| n.pitch == bass.pitch).count() > 1
        {
            bass_unison += 1;
        }
    }
    println!(
        "{finals} final sonorities, {with_unison} with a unison, {bass_unison} with the bass doubled at the unison"
    );
}

/// How much Palestrina's voices rest between their first and last notes, by number of voices:
/// the share of that time, how long the rests are and how many voices sound at once:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture rests`.
#[test]
#[ignore]
fn rests() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::Analysis;
    use std::collections::BTreeMap;
    // Per number of voices: time inside the voices, time resting there, rests by length in
    // measures (rounded up), downbeats by voices sounding.
    let mut inside: BTreeMap<usize, (u64, u64)> = BTreeMap::new();
    let mut lengths: BTreeMap<usize, BTreeMap<u32, usize>> = BTreeMap::new();
    let mut sounding: BTreeMap<usize, BTreeMap<usize, usize>> = BTreeMap::new();
    let mut entries_after_rest: BTreeMap<u32, usize> = BTreeMap::new();
    for (_, piece) in calibration_pieces() {
        let score = &piece.score;
        let n = score.voices.len();
        let analysis = Analysis::new(score);
        let per = analysis.per_measure;
        for line in &analysis.lines {
            let (Some(first), Some(last)) = (line.first(), line.last()) else {
                continue;
            };
            let entry = inside.entry(n).or_default();
            entry.0 += u64::from(last.end - first.start);
            for pair in line.windows(2) {
                let gap = pair[1].start - pair[0].end;
                if gap > 0 {
                    entry.1 += u64::from(gap);
                    *lengths
                        .entry(n)
                        .or_default()
                        .entry(gap.div_ceil(per))
                        .or_default() += 1;
                    *entries_after_rest
                        .entry(pair[1].end - pair[1].start)
                        .or_default() += 1;
                }
            }
        }
        for slice in analysis.slices.iter().filter(|s| s.tick % per == 0) {
            let count = slice.sounding.iter().flatten().count();
            *sounding.entry(n).or_default().entry(count).or_default() += 1;
        }
    }
    for (n, (all, resting)) in &inside {
        println!(
            "{n} voices: resting {:.1} % of the time between first and last note",
            100.0 * *resting as f64 / *all as f64
        );
        println!("   rests by measures: {:?}", lengths[n]);
        println!("   downbeats by voices sounding: {:?}", sounding[n]);
    }
    println!("first note after a rest, by length in eighths: {entries_after_rest:?}");
}

/// Downbeats after every voice has entered and before the last measure on which fewer than half
/// of the voices or none sound, by number of voices:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture sparse`.
#[test]
#[ignore]
fn sparse() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::Analysis;
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<usize, (usize, usize, usize)> = BTreeMap::new();
    for (_, piece) in calibration_pieces() {
        let score = &piece.score;
        let n = score.voices.len();
        let analysis = Analysis::new(score);
        let per = analysis.per_measure;
        let entered = analysis
            .lines
            .iter()
            .filter_map(|l| l.first().map(|s| s.start))
            .max()
            .unwrap_or(0);
        let end = analysis.total.saturating_sub(per);
        let entry = counts.entry(n).or_default();
        for slice in analysis
            .slices
            .iter()
            .filter(|s| s.tick % per == 0 && s.tick >= entered && s.tick < end)
        {
            let sounding = slice.sounding.iter().flatten().count();
            entry.0 += 1;
            if 2 * sounding < n {
                entry.1 += 1;
            }
            if sounding == 0 {
                entry.2 += 1;
            }
        }
    }
    for (n, (all, sparse, silent)) in counts {
        println!(
            "{n} voices: {all} downbeats, fewer than half sounding {sparse} ({:.2} %), none {silent}",
            100.0 * sparse as f64 / all as f64
        );
    }
}

/// Bass notes held over a downbeat (begun on an earlier downbeat), against which an upper voice
/// sounds a dissonance there: by interval, by whether the upper voice is struck there, and by
/// how the bass goes on:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture held_bass`.
#[test]
#[ignore]
fn held_bass() {
    if !corpus_present() {
        return;
    }
    use fux_core::Interval;
    use fux_core::rules::Analysis;
    use std::collections::BTreeMap;
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for (_, piece) in calibration_pieces() {
        let analysis = Analysis::new(&piece.score);
        let per = analysis.per_measure;
        for slice in analysis.slices.iter().filter(|s| s.tick % per == 0) {
            let Some((bass, b)) = slice.lowest() else {
                continue;
            };
            if b.start >= slice.tick || b.start % per != 0 {
                continue;
            }
            let line = &analysis.lines[bass];
            let i = line.iter().position(|s| s.start == b.start).unwrap();
            let next = match line.get(i + 1).filter(|n| n.start == b.end) {
                Some(n) => match Interval::between(b.pitch, n.pitch).steps {
                    -1 => "step down",
                    1 => "step up",
                    0 => "same",
                    s if s > 0 => "leap up",
                    _ => "leap down",
                },
                None => "rest/end",
            };
            for (v, other) in slice.sounding.iter().enumerate() {
                let Some(other) = *other else { continue };
                if v == bass {
                    continue;
                }
                let interval = Interval::between(b.pitch, other.pitch).simple();
                let class = match (
                    interval.steps.rem_euclid(7),
                    interval.semitones.rem_euclid(12),
                ) {
                    (3, 5) => "4th",
                    (1, _) => "2nd/9th",
                    (6, _) => "7th",
                    (3, 6) | (4, 6) => "tritone",
                    _ => continue,
                };
                let struck = if other.start == slice.tick {
                    "struck"
                } else {
                    "held"
                };
                *kinds
                    .entry(format!("{class:<8} upper {struck:<6} bass next {next}"))
                    .or_default() += 1;
            }
        }
    }
    let mut rows: Vec<_> = kinds.into_iter().collect();
    rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (kind, n) in rows {
        println!("{n:>6}  {kind}");
    }
}

/// Measures of voice leading, for Palestrina's four-voice movements and for generated four-voice
/// free settings side by side, so that the generator's preferences can be weighed against him:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture style`.
#[test]
#[ignore]
fn style() {
    if !corpus_present() {
        return;
    }
    use fux_core::generate::{FreeRequest, default_cadences, generate_free};
    use fux_core::rules::{Analysis, Profile};
    use fux_core::{Interval, Mode, ModeSpec, Score, VoiceType};

    /// Shares in per cent: leaps among the steps and leaps of the top, inner and lowest voices,
    /// fourths, fifths and octaves of the lowest voice, returns to the note before last (a b a),
    /// oscillations (a b a b), notes by value, and the mean range of a voice over four measures.
    fn measures(scores: &[Score]) -> Vec<(String, f64)> {
        let mut leaps = [(0usize, 0usize); 3];
        // Per role: thirds, fourths, fifths, octaves, all moves.
        let mut sizes = [[0usize; 5]; 3];
        let (mut bass_moves, mut bass_frames) = (0usize, 0usize);
        let (mut notes, mut returns, mut oscillations) = (0usize, 0usize, 0usize);
        let (mut double_same, mut double_turn) = (0usize, 0usize);
        let mut values = [0usize; 5];
        let (mut ranges, mut windows) = (0i64, 0usize);
        for score in scores {
            let analysis = Analysis::new(score);
            let per = analysis.per_measure;
            let last = analysis.lines.len() - 1;
            for (v, line) in analysis.lines.iter().enumerate() {
                let role = if v == 0 {
                    0
                } else if v == last {
                    2
                } else {
                    1
                };
                for (i, pair) in line.windows(2).enumerate() {
                    if pair[0].end != pair[1].start || pair[0].pitch == pair[1].pitch {
                        continue;
                    }
                    let interval = Interval::between(pair[0].pitch, pair[1].pitch).abs();
                    leaps[role].1 += 1;
                    if interval.steps >= 2 {
                        leaps[role].0 += 1;
                    }
                    match interval.steps {
                        2 => sizes[role][0] += 1,
                        3 => sizes[role][1] += 1,
                        4 => sizes[role][2] += 1,
                        7 => sizes[role][3] += 1,
                        _ => {}
                    }
                    sizes[role][4] += 1;
                    if role == 2 {
                        bass_moves += 1;
                        if matches!(interval.steps, 3 | 4 | 7) {
                            bass_frames += 1;
                        }
                    }
                    if let Some(third) = line.get(i + 2).filter(|n| n.start == pair[1].end) {
                        notes += 1;
                        let next = Interval::between(pair[1].pitch, third.pitch);
                        if interval.steps >= 2 && next.abs().steps >= 2 {
                            let first = Interval::between(pair[0].pitch, pair[1].pitch);
                            if (first.steps > 0) == (next.steps > 0) {
                                double_same += 1;
                            } else {
                                double_turn += 1;
                            }
                        }
                        if third.pitch == pair[0].pitch {
                            returns += 1;
                            if line
                                .get(i + 3)
                                .is_some_and(|n| n.start == third.end && n.pitch == pair[1].pitch)
                            {
                                oscillations += 1;
                            }
                        }
                    }
                }
                for s in line {
                    let value = match s.end - s.start {
                        0..=1 => 0,
                        2..=3 => 1,
                        4..=6 => 2,
                        7..=8 => 3,
                        _ => 4,
                    };
                    values[value] += 1;
                }
                let span = 4 * per;
                let mut from = 0;
                while from + span <= analysis.total {
                    let inside: Vec<i32> = line
                        .iter()
                        .filter(|s| s.start >= from && s.start < from + span)
                        .map(|s| s.pitch.diatonic())
                        .collect();
                    if let (Some(lo), Some(hi)) = (inside.iter().min(), inside.iter().max()) {
                        ranges += i64::from(hi - lo);
                        windows += 1;
                    }
                    from += span;
                }
            }
        }
        let share = |a: usize, b: usize| 100.0 * a as f64 / b.max(1) as f64;
        let all: usize = values.iter().sum();
        let moves: usize = sizes.iter().map(|r| r.iter().sum::<usize>()).sum();
        let mut rows: Vec<(String, f64)> = Vec::new();
        for (role, name) in ["top", "inner", "lowest"].iter().enumerate() {
            let total: usize = sizes[role].iter().sum();
            for (i, size) in ["3rd", "4th", "5th", "8ve"].iter().enumerate() {
                rows.push((
                    format!("  {name} {size}"),
                    share(sizes[role][i], sizes[role][4]),
                ));
            }
            let _ = total;
        }
        let _ = moves;
        vec![
            ("leaps, top voice".into(), share(leaps[0].0, leaps[0].1)),
            ("leaps, inner voices".into(), share(leaps[1].0, leaps[1].1)),
            ("leaps, lowest voice".into(), share(leaps[2].0, leaps[2].1)),
            (
                "lowest voice by 4th/5th/8ve".into(),
                share(bass_frames, bass_moves),
            ),
            ("returns a b a".into(), share(returns, notes)),
            ("oscillations a b a b".into(), share(oscillations, notes)),
            (
                "two leaps, same direction".into(),
                share(double_same, notes),
            ),
            ("two leaps, turning".into(), share(double_turn, notes)),
            ("eighths".into(), share(values[0], all)),
            ("quarters".into(), share(values[1], all)),
            ("halves".into(), share(values[2], all)),
            ("wholes".into(), share(values[3], all)),
            ("longer".into(), share(values[4], all)),
            (
                "range in 4 measures (steps)".into(),
                ranges as f64 / windows.max(1) as f64,
            ),
        ]
        .into_iter()
        .chain(rows)
        .collect()
    }

    let palestrina: Vec<Score> = calibration_pieces()
        .into_iter()
        .map(|(_, p)| p.score)
        .filter(|s| s.voices.len() == 4)
        .collect();
    let voices = vec![
        VoiceType::Soprano,
        VoiceType::Alto,
        VoiceType::Tenor,
        VoiceType::Bass,
    ];
    let mut generated = Vec::new();
    for mode in [Mode::Dorian, Mode::Phrygian, Mode::Mixolydian, Mode::Ionian] {
        for seed in 1..=3 {
            let mode = ModeSpec::new(mode);
            let request = FreeRequest {
                cadences: default_cadences(mode, 16),
                seed,
                ..FreeRequest::new(mode, voices.clone(), 16)
            };
            let mut profile = Profile::fux();
            for rule in std::env::var("OVERRIDES")
                .unwrap_or_default()
                .split_whitespace()
            {
                profile.apply_override(rule).unwrap();
            }
            if let Ok(mut results) = generate_free(&request, &profile) {
                generated.push(results.remove(0).score);
            }
        }
    }
    let a = measures(&palestrina);
    let b = measures(&generated);
    println!(
        "{:<32} {:>10} {:>10}   ({} and {} settings)",
        "",
        "Palestrina",
        "generated",
        palestrina.len(),
        generated.len()
    );
    for ((name, x), (_, y)) in a.iter().zip(&b) {
        println!("{name:<32} {x:>10.1} {y:>10.1}");
    }
}

/// Whether Palestrina's voices keep to the octave of their mode: for each movement that ends on
/// the final of its mode, the tenor decides authentic (final to octave) or plagal (fourth below
/// to fifth above); then every voice is measured against its octave, the tenor's and soprano's
/// ambitus or the other one, placed where its notes lie:
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture ambitus`.
#[test]
#[ignore]
fn ambitus() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::Analysis;
    use std::collections::BTreeMap;
    // Per voice name: notes, notes more than a step outside the principal (P) and the collateral
    // (C) octave.
    let mut counts: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    let mut tenors: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, piece) in calibration_pieces() {
        let score = &piece.score;
        let analysis = Analysis::new(score);
        let final_step = score.mode.final_step();
        let Some(last) = analysis.lines.last().and_then(|l| l.last()) else {
            continue;
        };
        if last.pitch.step != final_step {
            continue;
        }
        // Lowest octave start (diatonic index of a final) that fits most notes of a line in the
        // octave beginning `offset` steps above a final.
        let outside = |line: &[fux_core::Sounding], offset: i32| -> (usize, i32) {
            let finals = (0..9).map(|o| fux_core::Pitch::new(final_step, 0, o).diatonic() + offset);
            finals
                .map(|low| {
                    let out = line
                        .iter()
                        .filter(|s| {
                            let d = s.pitch.diatonic();
                            d < low - 1 || d > low + 8
                        })
                        .count();
                    (out, low)
                })
                .min()
                .unwrap()
        };
        let Some(tenor) = score
            .voices
            .iter()
            .position(|v| v.name.to_lowercase().starts_with("tenor"))
        else {
            continue;
        };
        let line = &analysis.lines[tenor];
        let (authentic, _) = outside(line, 0);
        let (plagal, _) = outside(line, -3);
        let principal = if authentic <= plagal { 0 } else { -3 };
        *tenors
            .entry(if principal == 0 {
                "authentic"
            } else {
                "plagal"
            })
            .or_default() += 1;
        let collateral = if principal == 0 { -3 } else { 0 };
        for (v, voice) in score.voices.iter().enumerate() {
            let name: String = voice
                .name
                .chars()
                .take_while(|c| c.is_alphabetic())
                .collect();
            let line = &analysis.lines[v];
            let entry = counts.entry(name).or_default();
            entry.0 += line.len();
            entry.1 += outside(line, principal).0;
            entry.2 += outside(line, collateral).0;
        }
    }
    println!("tenors: {tenors:?}");
    println!(
        "{:<10} {:>8} {:>16} {:>16}",
        "voice", "notes", "outside P (%)", "outside C (%)"
    );
    for (name, (notes, p, c)) in counts {
        if notes < 1000 {
            continue;
        }
        println!(
            "{name:<10} {notes:>8} {:>16.2} {:>16.2}",
            100.0 * p as f64 / notes as f64,
            100.0 * c as f64 / notes as f64
        );
    }
}

/// Palestrina's melodic intervals by size, for the top, inner and lowest voices, as shares of
/// all moves (repeated notes not counted):
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture intervals`.
#[test]
#[ignore]
fn intervals() {
    if !corpus_present() {
        return;
    }
    use fux_core::Interval;
    use fux_core::rules::Analysis;
    let names = [
        "step", "third", "fourth", "fifth", "sixth", "seventh", "octave", "beyond",
    ];
    let mut counts = [[0usize; 8]; 3];
    for (_, piece) in calibration_pieces() {
        let analysis = Analysis::new(&piece.score);
        let last = analysis.lines.len() - 1;
        for (v, line) in analysis.lines.iter().enumerate() {
            let role = if v == 0 {
                0
            } else if v == last {
                2
            } else {
                1
            };
            for pair in line.windows(2) {
                if pair[0].end != pair[1].start || pair[0].pitch == pair[1].pitch {
                    continue;
                }
                let steps = Interval::between(pair[0].pitch, pair[1].pitch).abs().steps;
                let index = match steps {
                    0 | 1 => 0,
                    2..=7 => steps as usize - 1,
                    _ => 7,
                };
                counts[role][index] += 1;
            }
        }
    }
    for (role, name) in ["top", "inner", "lowest"].iter().enumerate() {
        let all: usize = counts[role].iter().sum();
        let shares: Vec<String> = names
            .iter()
            .zip(counts[role])
            .map(|(n, c)| format!("{n} {:.2} %", 100.0 * c as f64 / all as f64))
            .collect();
        println!("{name}: {}", shares.join(", "));
    }
}

/// The sections whose voices each sing one short text of the Ordinary, by the movement and the
/// section their files name (as the design's judge chose them): Kyrie I and the Christe of the
/// divided Kyries, Kyrie II where it has a file of its own, Agnus I, and the Benedictus without
/// its Hosanna. (Movement, section, text.)
const SUNG_SECTIONS: [(&str, Option<&str>, &str); 5] = [
    ("Kyrie", Some("First Section"), "kyrie"),
    ("Kyrie", Some("Christe"), "christe"),
    ("Kyrie II", None, "kyrie2"),
    ("Agnus I", None, "agnus"),
    ("Benedictus", Some("First Section"), "benedictus"),
];

/// Whether Palestrina's music lets each of his voices carry its text by the rules of the
/// underlay: the automaton of the search runs over every voice of the sections of
/// [`SUNG_SECTIONS`] with the section's text, at Fux's levels, without the capacity bound and
/// without a limit on its states. A voice admits an underlay if the automaton accepts at its end:
/// some underlay of the whole text breaks no prohibition. A text with eleison is tried with four
/// syllables (the books) and with three (e-léi-son). His sections are often long and his voices
/// may leave out words; this measures only whether the rules let his music carry the text. At
/// least 90 % of the voices must admit an underlay with the books' eleison (the design's model:
/// 983 of 1037, 95 %). Prints the share per section and eleison, the rules that leave the other
/// voices without a state, and examples of each (`EXAMPLES`, default 3):
/// `cargo test --release -p fux-core --test palestrina -- --ignored --nocapture admit`.
#[test]
#[ignore]
fn palestrina_voices_admit_an_underlay() {
    if !corpus_present() {
        return;
    }
    use fux_core::Note;
    use fux_core::rules::{Checker, Profile};
    use fux_core::text::underlay::{Automaton, Levels, Model};
    use fux_core::text::{Eleison, TextOptions, resolve};

    /// Where the automaton is left without a state: the rule that cut the most states there and
    /// the note; `None` if it accepts at the voice's end.
    fn blocked(automaton: &Automaton, notes: &[Note]) -> Option<(&'static str, usize)> {
        let mut set = Automaton::start();
        for i in 0..notes.len() {
            match automaton.advance(&set, &notes[..=i]) {
                Ok(next) => set = next,
                Err(rule) => return Some((rule, i)),
            }
        }
        None
    }

    /// Eleison in four syllables, as the books divide it, and in three.
    const ELEISON: [(u8, Eleison); 2] = [(4, Eleison::Four), (3, Eleison::Three)];
    let started = std::time::Instant::now();
    let levels = Levels::from(&Checker::new(&Profile::fux()));
    let examples = std::env::var("EXAMPLES").map_or(3, |n| n.parse().unwrap());
    let label = |s: usize| match SUNG_SECTIONS[s] {
        (movement, Some(section), _) => format!("{movement} / {section}"),
        (movement, None, _) => movement.to_string(),
    };
    // Per section and syllables of eleison: the voices that admit an underlay, and all voices.
    // A text without eleison counts for both.
    let mut rates: BTreeMap<(usize, u8), (usize, usize)> = BTreeMap::new();
    let mut with_eleison = [false; SUNG_SECTIONS.len()];
    // Per syllables of eleison and rule: the voices the rule leaves without a state.
    let mut blocking: BTreeMap<(u8, &str), Vec<String>> = BTreeMap::new();
    // The voices that admit an underlay with either eleison, and all voices.
    let (mut either, mut voices) = (0, 0);
    for (name, result) in read_all() {
        let Ok(piece) = result else {
            continue;
        };
        let Some(s) = SUNG_SECTIONS.iter().position(|&(movement, section, _)| {
            piece.movement.as_deref() == Some(movement) && piece.section.as_deref() == section
        }) else {
            continue;
        };
        let score = &piece.score;
        if score.validate().is_err() {
            continue;
        }
        let texts = ELEISON.map(|(_, eleison)| {
            resolve(SUNG_SECTIONS[s].2, &TextOptions { eleison })
                .unwrap()
                .0
        });
        with_eleison[s] = texts[0] != texts[1];
        let models = texts.map(|text| Model::new(&text, score.mode, score.meter, &[]));
        for (v, voice) in score.voices.iter().enumerate() {
            let mut admitted = false;
            let mut outcome = None;
            for (k, &(syllables, _)) in ELEISON.iter().enumerate() {
                if k == 0 || with_eleison[s] {
                    let automaton = Automaton {
                        model: &models[k],
                        levels: &levels,
                        total: voice.total_ticks(),
                        cap: None,
                        limit: None,
                        pins: None,
                        gates: None,
                    };
                    outcome = blocked(&automaton, &voice.notes);
                }
                let rate = rates.entry((s, syllables)).or_default();
                rate.1 += 1;
                match outcome {
                    None => {
                        rate.0 += 1;
                        admitted = true;
                    }
                    Some((rule, note)) => {
                        let (m, at) = score.meter.locate(score.note_start(v, note).unwrap());
                        blocking.entry((syllables, rule)).or_default().push(format!(
                            "{name} ({}) {} m{} +{at}",
                            label(s),
                            voice.name,
                            m + 1
                        ));
                    }
                }
            }
            voices += 1;
            either += usize::from(admitted);
        }
    }
    let share = |(admit, all): (usize, usize)| 100.0 * admit as f64 / all.max(1) as f64;
    println!(
        "{:<26} {:>7} {:>6} {:>6} {:>6}",
        "section", "eleison", "voices", "admit", "%"
    );
    for (s, &eleison_in_text) in with_eleison.iter().enumerate() {
        for (syllables, _) in ELEISON {
            let Some(&rate) = rates.get(&(s, syllables)) else {
                continue;
            };
            let eleison = match (eleison_in_text, syllables) {
                (true, _) => syllables.to_string(),
                (false, 4) => "-".to_string(),
                (false, _) => continue,
            };
            println!(
                "{:<26} {eleison:>7} {:>6} {:>6} {:>6.1}",
                label(s),
                rate.1,
                rate.0,
                share(rate)
            );
        }
    }
    let total = |syllables: u8| {
        rates
            .iter()
            .filter(|((_, e), _)| *e == syllables)
            .fold((0, 0), |(a, n), (_, &(admit, all))| (a + admit, n + all))
    };
    for (syllables, _) in ELEISON {
        let rate = total(syllables);
        println!(
            "all, eleison in {syllables} syllables: {}/{} ({:.1} %)",
            rate.0,
            rate.1,
            share(rate)
        );
    }
    println!(
        "all, either eleison: {either}/{voices} ({:.1} %); {:.1} s",
        share((either, voices)),
        started.elapsed().as_secs_f64()
    );
    for (syllables, _) in ELEISON {
        let mut rows: Vec<(&str, &Vec<String>)> = blocking
            .iter()
            .filter(|((e, _), _)| *e == syllables)
            .map(|((_, rule), list)| (*rule, list))
            .collect();
        rows.sort_by_key(|(rule, list)| (std::cmp::Reverse(list.len()), *rule));
        println!("\nvoices without an underlay by rule, eleison in {syllables} syllables:");
        for (rule, list) in rows {
            println!("{rule:<22} {:>4}", list.len());
            for line in list.iter().take(examples) {
                println!("  {line}");
            }
        }
    }
    assert_eq!(voices, 1037);
    let books = total(4);
    assert!(
        10 * books.0 >= 9 * books.1,
        "{} of {} voices admit an underlay",
        books.0,
        books.1
    );
}
