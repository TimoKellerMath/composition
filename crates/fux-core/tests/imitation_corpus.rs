//! Points of imitation in Palestrina's masses (references/datasets/palestrina-masses): the
//! detector finds them as the study of the corpus measured them (docs/regeln.md § 18), so that
//! its figures for generated settings compare with his. The tests need the corpus and run with
//! `cargo test --release -p fux-core --test imitation_corpus -- --ignored --nocapture`.

mod common;

use std::collections::BTreeMap;
use std::ops::Range;

use common::{calibration_pieces, corpus_present};
use fux_core::ModeSpec;
use fux_core::imitation::check::pair_conflict;
use fux_core::imitation::detect::{FoundPoint, detect, sections};
use fux_core::imitation::{Origin, Soggetto, SubjectNote};
use fux_core::kern::KernPiece;
use fux_core::rules::{Checker, Profile};
use fux_core::text::underlay::Choice;

/// How a section of the corpus begins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Start {
    /// A movement: a file that holds a whole movement (it has no `!!!OMD`), or the first section
    /// of a divided one. Agnus II and Kyrie II in files of their own are movements too.
    Movement,
    /// A later section of a divided movement in a file of its own, such as a Christe or a
    /// Crucifixus.
    File,
    /// A section after a held chord within a file, such as the Christe of a whole Kyrie.
    Internal,
}

impl Start {
    fn name(self) -> &'static str {
        match self {
            Start::Movement => "movement starts",
            Start::File => "later file starts",
            Start::Internal => "internal breaks",
        }
    }
}

/// A section of a movement of the corpus, with the points of imitation found in it.
struct Section<'a> {
    file: &'a str,
    piece: &'a KernPiece,
    start: Start,
    /// Its ticks in the movement.
    span: Range<u32>,
    points: Vec<FoundPoint>,
}

impl Section<'_> {
    /// The point that opens the section, if it opens with imitation.
    fn opening(&self) -> Option<&FoundPoint> {
        self.points.iter().find(|p| p.opens)
    }
}

/// The sections of the movements, as the detector cuts them, with the points it finds for
/// entries after rests of at least `min_rest` ticks.
fn corpus_sections(pieces: &[(String, KernPiece)], min_rest: u32) -> Vec<Section<'_>> {
    let mut out = Vec::new();
    for (file, piece) in pieces {
        let mut points = detect(&piece.score, min_rest);
        for (index, span) in sections(&piece.score).into_iter().enumerate() {
            let start = match index {
                0 if piece
                    .section
                    .as_deref()
                    .is_none_or(|s| s == "First Section") =>
                {
                    Start::Movement
                }
                0 => Start::File,
                _ => Start::Internal,
            };
            let (here, later): (Vec<FoundPoint>, _) =
                points.into_iter().partition(|p| p.section == index);
            points = later;
            out.push(Section {
                file,
                piece,
                start,
                span,
                points: here,
            });
        }
    }
    out
}

fn percent(part: usize, whole: usize) -> f64 {
    100.0 * part as f64 / whole as f64
}

/// The study cut the 1156 movements of the calibration into 1474 sections: 678 movement starts,
/// 478 later file starts and 318 sections after a held chord within a file, 67 of the 74 whole
/// Kyries into Kyrie I, Christe and Kyrie II. Of the movement starts, 91.4 % open with a point of
/// imitation, counting entries after rests of a semibreve, and 90.6 % with those after minim
/// rests as well (79.1 and 69.5 % of the later file starts and the internal breaks). The detector
/// cuts the sections in the same places and finds the openings within two points. In fact it
/// finds the study's points themselves: 4847, 2480 of them with three voices or more, and 6738
/// and 3626 with minim rests; compared with the study's records, they agree entry by entry.
#[test]
#[ignore]
fn detector_reproduces_the_corpus_openings() {
    if !corpus_present() {
        return;
    }
    let pieces = calibration_pieces();
    assert_eq!(pieces.len(), 1156);
    let strict = corpus_sections(&pieces, 8);
    let minim = corpus_sections(&pieces, 4);
    assert_eq!(strict.len(), minim.len());
    // Per start: the sections, and those opening with imitation for both kinds of rest.
    let mut counts: BTreeMap<Start, [usize; 3]> = BTreeMap::new();
    for (s, m) in strict.iter().zip(&minim) {
        assert_eq!((s.file, &s.span), (m.file, &m.span));
        assert!(s.span.end <= s.piece.score.total_ticks(), "{}", s.file);
        let count = counts.entry(s.start).or_default();
        count[0] += 1;
        count[1] += usize::from(s.opening().is_some());
        count[2] += usize::from(m.opening().is_some());
    }
    println!(
        "{:<20} {:>8} {:>16} {:>16}",
        "sections", "count", "imitative (W)", "imitative (H)"
    );
    for (start, [all, by_semibreve, by_minim]) in &counts {
        println!(
            "{:<20} {all:>8} {:>15.1}% {:>15.1}%",
            start.name(),
            percent(*by_semibreve, *all),
            percent(*by_minim, *all)
        );
    }
    // All points, and those with three voices or more.
    let points = |sections: &[Section]| -> (usize, usize) {
        let all = sections.iter().flat_map(|s| &s.points);
        let larger = all.clone().filter(|p| p.voices().len() >= 3).count();
        (all.count(), larger)
    };
    let (by_semibreve, by_minim) = (points(&strict), points(&minim));
    println!("points (with three voices or more): {by_semibreve:?} (W), {by_minim:?} (H)");
    let whole_kyries: Vec<usize> = pieces
        .iter()
        .filter(|(_, piece)| piece.movement.as_deref() == Some("Kyrie") && piece.section.is_none())
        .map(|(_, piece)| sections(&piece.score).len())
        .collect();
    let cut = whole_kyries.iter().filter(|&&n| n == 3).count();
    println!(
        "whole Kyries: {}, cut into three sections: {cut}",
        whole_kyries.len()
    );
    assert_eq!(counts[&Start::Movement][0], 678);
    assert_eq!(counts[&Start::File][0], 478);
    assert_eq!(counts[&Start::Internal][0], 318);
    assert_eq!((whole_kyries.len(), cut), (74, 67));
    let [all, by_semibreve_opening, by_minim_opening] = counts[&Start::Movement];
    for (found, study) in [(by_semibreve_opening, 91.4), (by_minim_opening, 90.6)] {
        let share = percent(found, all);
        assert!(
            (share - study).abs() <= 2.0,
            "{share:.1} % of the movement starts imitative, the study {study} %"
        );
    }
    assert_eq!((by_semibreve, by_minim), ((4847, 2480), (6738, 3626)));
}

/// The heads of Palestrina's points as the subject builder would take them: the leader's notes
/// that its direct followers imitate, at most twelve, without inner rests and with values of a
/// quarter to a dotted breve; each with whether its point opens its section, the mode and the
/// number of voices.
fn leaders_heads(pieces: &[(String, KernPiece)]) -> Vec<(bool, ModeSpec, usize, Soggetto)> {
    let mut out = Vec::new();
    for (_, piece) in pieces {
        let score = &piece.score;
        for point in detect(score, 8) {
            let leader = &point.entries[0];
            let matched = point
                .entries
                .iter()
                .filter(|e| e.follows == Some(0))
                .map(|e| e.matched)
                .max()
                .unwrap_or(0)
                .min(12);
            if matched < 4 || leader.notes.len() < matched {
                continue;
            }
            let head = &leader.notes[..matched];
            let values: Vec<u32> = head.iter().map(|s| s.end - s.start).collect();
            if head.windows(2).any(|w| w[0].end != w[1].start)
                || values.iter().any(|v| !(2..=24).contains(v))
            {
                continue;
            }
            let first = head[0].pitch;
            let notes = head
                .iter()
                .zip(&values)
                .map(|(s, &ticks)| SubjectNote {
                    step: (s.pitch.diatonic() - first.diatonic()) as i8,
                    ticks: ticks as u8,
                    sung: Choice::Hold,
                    alter: 0,
                })
                .collect();
            let subject = Soggetto {
                degree: (score.mode.degree(first) - 1) as i8,
                notes,
                words: 0..1,
                origin: Origin::Text,
                answers: Vec::new(),
                stretto: Vec::new(),
                cost: 0,
            };
            out.push((point.opens, score.mode, score.voices.len(), subject));
        }
    }
    out
}

/// The stretto test of the subject builder (Fux, p. 247) keeps nearly all of Palestrina's
/// heads, the openings' and the later points', and charges most of them one or nothing: a
/// subject needs a few good strettos, as his have, not all of them.
#[test]
#[ignore]
fn stretto_test_keeps_palestrinas_heads() {
    if !corpus_present() {
        return;
    }
    let checker = Checker::new(&Profile::fux());
    let pieces = calibration_pieces();
    for opens in [true, false] {
        let mut costs: BTreeMap<Option<u32>, usize> = BTreeMap::new();
        for (_, mode, voices, mut subject) in
            leaders_heads(&pieces).into_iter().filter(|h| h.0 == opens)
        {
            *costs
                .entry(subject.test_stretto(&checker, mode, voices))
                .or_default() += 1;
        }
        let heads: usize = costs.values().sum();
        let kept = heads - costs.get(&None).copied().unwrap_or(0);
        let at_most_one: usize = [Some(0), Some(1)].iter().filter_map(|c| costs.get(c)).sum();
        let name = if opens { "openings" } else { "later points" };
        println!(
            "{name}: {heads} heads, kept {:.1} %, costs {costs:?}, one or less {:.1} %",
            percent(kept, heads),
            percent(at_most_one, heads)
        );
        assert!(percent(kept, heads) >= 97.0, "{name}");
        assert!(
            2 * at_most_one > heads,
            "{name}: the median head costs more than one"
        );
    }
}

/// The pair checks of the planner find in Palestrina's own openings almost nothing to object to:
/// the first two entries of the openings the detector finds (entries after rests of a
/// semibreve), as long as the other voices are silent, pass in 99.5 % at least.
#[test]
#[ignore]
fn pair_checks_accept_palestrinas_openings() {
    if !corpus_present() {
        return;
    }
    let (mut pairs, mut rejected) = (0, Vec::new());
    for (name, piece) in calibration_pieces() {
        let score = &piece.score;
        let choir: Vec<_> = score.voices.iter().map(|v| v.voice_type).collect();
        for point in detect(score, 8).iter().filter(|p| p.opens) {
            let leader = &point.entries[0];
            let Some(follower) = point
                .entries
                .iter()
                .find(|e| e.follows == Some(0) && e.voice != leader.voice)
            else {
                continue;
            };
            let notes = point
                .entries
                .iter()
                .filter(|e| e.follows == Some(0))
                .map(|e| e.matched)
                .max()
                .unwrap_or(0);
            if notes < 4 || leader.notes.len() < notes {
                continue;
            }
            // The other voices are silent until the first of them sounds.
            let cutoff = score
                .voices
                .iter()
                .enumerate()
                .filter(|&(v, _)| v != leader.voice && v != follower.voice)
                .filter_map(|(_, voice)| {
                    voice
                        .soundings()
                        .iter()
                        .find(|s| s.end > leader.start)
                        .map(|s| s.start)
                })
                .min()
                .unwrap_or(u32::MAX);
            if cutoff <= follower.start {
                continue;
            }
            // Each line up to there, without a rest inside.
            let line = |notes: &[fux_core::score::Sounding]| {
                let line: Vec<_> = notes.iter().copied().filter(|s| s.start < cutoff).collect();
                line.windows(2)
                    .all(|w| w[0].end == w[1].start)
                    .then_some(line)
            };
            let (Some(a), Some(b)) = (
                line(&leader.notes[..notes]),
                line(&follower.notes[..follower.matched]),
            ) else {
                continue;
            };
            pairs += 1;
            let lines = ((leader.voice, &a[..]), (follower.voice, &b[..]));
            if let Some((rule, tick)) = pair_conflict(&choir, lines, cutoff) {
                rejected.push(format!("{name}: {rule} at {tick}"));
            }
        }
    }
    let share = 100.0 * (pairs - rejected.len()) as f64 / pairs as f64;
    println!(
        "{} of {pairs} openings rejected ({share:.2} % pass)",
        rejected.len()
    );
    for case in &rejected {
        println!("  {case}");
    }
    assert!(pairs > 500, "{pairs} openings");
    assert!(share >= 99.5, "{share:.2} %");
}

/// Whether two pitches sounding together are dissonant: a second, a seventh, a tritone or an
/// altered interval, or a fourth whose lower note is `bass`, the lowest sounding pitch.
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

/// IMI-HELD judges in imitative settings what the rules of § 16 excuse: a note struck on the
/// downbeat against a note tied over it that does not go down by step, and a note struck later
/// against a tied note that is no suspension and is no figure. With a plan of one point without
/// entries attached, the test counts the same cases in Palestrina's movements against their
/// occasions (a dissonant note struck against a tied one, on the downbeat or after it), so that
/// the rule's levels follow the Leitlinie (docs/regeln.md § 16); and the downbeats after the last
/// first entry of a section on which exactly half of the voices sound, for IMI-SILENCE `thin`.
#[test]
#[ignore]
fn held_dissonances_and_half_textures_in_the_corpus() {
    if !corpus_present() {
        return;
    }
    use fux_core::rules::{Analysis, Level, ParamKind, all_rules};
    use fux_core::score::{ScoreImitation, ScorePoint};

    let pieces = calibration_pieces();
    let mut profile = Profile::fux();
    for rule in all_rules() {
        let id = rule.info().id;
        for param in rule.info().params.iter() {
            if param.kind == ParamKind::Level && id != "IMI-HELD" {
                profile
                    .apply_override(&format!("{id}.{}=aus", param.name))
                    .unwrap();
            }
        }
    }
    for param in ["struck", "passing"] {
        profile
            .apply_override(&format!("IMI-HELD.{param}=P1"))
            .unwrap();
    }
    let checker = Checker::new(&profile);
    let mut cases: BTreeMap<&'static str, usize> = BTreeMap::new();
    let (mut struck_occasions, mut later_occasions) = (0usize, 0usize);
    let mut voice_bars = 0u64;
    // Per number of voices: downbeats after the last first entry of a section, and those on
    // which exactly half of the voices sound.
    let mut half: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    for (name, piece) in &pieces {
        let mut score = piece.score.clone();
        score.imitation = Some(std::sync::Arc::new(ScoreImitation {
            points: vec![ScorePoint {
                words: 0..0,
                entries: Vec::new(),
                goal: None,
            }],
        }));
        for v in checker.check(&score).violations {
            assert_eq!(v.rule, "IMI-HELD", "{name}");
            assert_ne!(v.level, Level::Off);
            let fux_core::rules::Detail::Param(param) = v.detail else {
                panic!("{name}: {:?}", v.detail);
            };
            *cases.entry(param).or_default() += 1;
        }
        let analysis = Analysis::new(&score);
        let per = analysis.per_measure;
        let n = score.voices.len();
        voice_bars += u64::from(analysis.total / per) * n as u64;
        for slice in &analysis.slices {
            let Some(bass) = slice.lowest() else { continue };
            let downbeat = slice.tick / per * per;
            for (v, note) in slice.sounding.iter().enumerate() {
                let Some(note) = note.filter(|s| s.start == slice.tick && s.end - s.start > 1)
                else {
                    continue;
                };
                for other in slice.sounding.iter().enumerate().filter(|&(u, _)| u != v) {
                    let Some(held) = other.1.filter(|h| h.start < downbeat) else {
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
        if n >= 4 && n % 2 == 0 {
            for span in sections(&score) {
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
                let last_measure = span.end.saturating_sub(per);
                let entry = half.entry(n).or_default();
                let mut tick = entered.div_ceil(per) * per;
                while tick < last_measure {
                    let sounding = analysis
                        .lines
                        .iter()
                        .filter(|line| line.iter().any(|s| s.start <= tick && tick < s.end));
                    entry.0 += 1;
                    if 2 * sounding.count() == n {
                        entry.1 += 1;
                    }
                    tick += per;
                }
            }
        }
    }
    let struck = cases.get("struck").copied().unwrap_or(0);
    let passing = cases.get("passing").copied().unwrap_or(0);
    println!("{} movements, {voice_bars} voice-bars", pieces.len());
    println!(
        "IMI-HELD struck: {struck} of {struck_occasions} dissonant downbeat attacks against a tied note ({:.3} %; {:.3} per 100 voice-bars)",
        percent(struck, struck_occasions),
        100.0 * struck as f64 / voice_bars as f64
    );
    println!(
        "IMI-HELD passing: {passing} of {later_occasions} dissonant later attacks against a tied note ({:.3} %; {:.4} per 100 voice-bars)",
        percent(passing, later_occasions),
        100.0 * passing as f64 / voice_bars as f64
    );
    for (n, (downbeats, halves)) in &half {
        println!(
            "{n} voices: exactly half sounding on {halves} of {downbeats} downbeats after the last first entry ({:.1} %)",
            percent(*halves, *downbeats)
        );
    }
    // The levels of the profile by the Leitlinie: under about 0.1 % a prohibition, up to 2 % P3,
    // over 10 % P1.
    assert!(percent(struck, struck_occasions) < 0.1, "struck");
    assert!(
        (0.1..2.0).contains(&percent(passing, later_occasions)),
        "passing"
    );
    let (downbeats, halves) = half[&4];
    assert!(percent(halves, downbeats) > 10.0, "four voices, half");
}
