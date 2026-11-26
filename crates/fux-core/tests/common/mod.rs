//! Checks shared by the calibration tests: the checker finds the same violations window by
//! window and prefix by prefix as in the whole score, as the search needs it; a checker of the
//! text's rules alone; the reader of Palestrina's masses; and the oracles of texted and
//! imitative settings, which use no code of the rules.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use fux_core::generate::Generated;
use fux_core::imitation::detect::detect;
use fux_core::kern::{KernError, KernPiece, parse_kern};
use fux_core::rules::{Checker, Level, ParamKind, Profile, all_rules};
use fux_core::text::underlay::{Choice, Goal, Levels, Model};
use fux_core::text::{SylFlags, Text};
use fux_core::{Duration, Note, Score, VoiceType};

/// Where Palestrina's masses lie (references/datasets/palestrina-masses).
#[allow(dead_code)]
pub fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../references/datasets/palestrina-masses/kern")
}

/// Whether the data at `path` is there: Palestrina's masses (`python3 tools/fetch_palestrina.py`)
/// or Jeppesen's underlay, which not every checkout has. Without it a test that needs it says so
/// and returns; with `FUX_REQUIRE_CORPUS` set its absence fails the test instead, so that no
/// check against the corpus goes unseen.
#[allow(dead_code)]
pub fn present(path: &Path) -> bool {
    if path.exists() {
        return true;
    }
    assert!(
        std::env::var_os("FUX_REQUIRE_CORPUS").is_none_or(|v| v.is_empty() || v == "0"),
        "{} is missing, and FUX_REQUIRE_CORPUS is set",
        path.display()
    );
    println!(
        "skipped: {} is missing (python3 tools/fetch_palestrina.py)",
        path.display()
    );
    false
}

/// Whether Palestrina's masses are there (see [`present`]).
#[allow(dead_code)]
pub fn corpus_present() -> bool {
    present(&corpus_dir())
}

/// Every file of Palestrina's masses with what reading it gives, sorted by the file names.
#[allow(dead_code)]
pub fn read_all() -> Vec<(String, Result<KernPiece, KernError>)> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(corpus_dir())
        .expect("the corpus (python3 tools/fetch_palestrina.py)")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "krn"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&path).unwrap();
            (name, parse_kern(&text))
        })
        .collect()
}

/// The movements of Palestrina's masses that the calibration uses: in 4/2, three to six
/// voices, valid; sorted by their file names.
#[allow(dead_code)]
pub fn calibration_pieces() -> Vec<(String, KernPiece)> {
    read_all()
        .into_iter()
        .filter_map(|(name, result)| {
            let piece = result.ok()?;
            let voices = piece.score.voices.len();
            (piece.written_meter.numerator == 4
                && (3..=6).contains(&voices)
                && piece.score.validate().is_ok())
            .then_some((name, piece))
        })
        .collect()
}

/// A checker of the TXT rules alone, with every case switched on: those free by default cost
/// P1. Only the tests of texted scores use it.
#[allow(dead_code)]
pub fn text_rules() -> Checker {
    let mut profile = Profile::fux();
    for rule in all_rules() {
        let id = rule.info().id;
        for param in rule.info().params.iter() {
            if param.kind != ParamKind::Level {
                continue;
            }
            let value = if !id.starts_with("TXT-") {
                "aus"
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
    Checker::new(&profile)
}

/// The search checks a candidate only until the first rule that forbids it
/// ([`Checker::check_window_until_forbidden`]): it must find a prohibition exactly where the full
/// check of the window, `full`, does, and without one the same violations. That holds as long as
/// a rule forbids only through a level `Forbidden` among its own settings.
#[allow(dead_code)]
fn assert_until_forbidden_agrees(
    checker: &Checker,
    score: &Score,
    analysis: &fux_core::rules::Analysis,
    window: fux_core::rules::Window,
    targets: &[bool],
    full: &[fux_core::rules::Violation],
    name: &str,
) {
    let forbidden = full.iter().any(|v| v.is_forbidden());
    for costs in [true, false] {
        let mut quick = Vec::new();
        let found = checker
            .check_window_until_forbidden(score, analysis, window, targets, &mut quick, costs);
        assert_eq!(
            found, forbidden,
            "{name}: from {} until {}, costs {costs}: {full:?}",
            window.from, window.until
        );
        if costs && !forbidden {
            let key = |v: &fux_core::rules::Violation| (v.tick, v.voice, v.rule, v.note);
            let mut full = full.to_vec();
            full.sort_by_key(key);
            quick.sort_by_key(key);
            assert_eq!(
                quick, full,
                "{name}: from {} until {}",
                window.from, window.until
            );
        }
    }
}

/// The search checks one window per step. Over windows that cover the piece, the rules must
/// find exactly what a check of the whole score finds; skipping work before a window must not
/// lose anything.
#[allow(dead_code)]
pub fn assert_windows_partition(checker: &Checker, score: &Score, name: &str) {
    use fux_core::rules::{Analysis, Window};

    let whole = checker.check(score).violations;
    let analysis = Analysis::new(score);
    let ticks: Vec<u32> = analysis.slices.iter().map(|s| s.tick).collect();
    let targets = vec![true; score.voices.len()];
    let mut found = Vec::new();
    for (k, &tick) in ticks.iter().enumerate() {
        let from = if k == 0 { 0 } else { ticks[k - 1] + 1 };
        let until = if k + 1 == ticks.len() { u32::MAX } else { tick };
        let window = Window {
            from,
            until,
            complete: true,
        };
        let mut here = Vec::new();
        checker.check_window(score, &analysis, window, &targets, &mut here);
        assert_until_forbidden_agrees(checker, score, &analysis, window, &targets, &here, name);
        found.extend(here);
    }
    found.sort_by_key(|v| (v.tick, v.voice, v.rule, v.note));
    let mut whole = whole;
    whole.sort_by_key(|v| (v.tick, v.voice, v.rule, v.note));
    assert_eq!(found, whole, "{name}");
}

/// The search sees the voices it sets only up to the current tick, the cantus completely, and
/// counts the violations decided since the previous tick; rules about the whole piece wait for
/// the final check. Built up that way, a score must collect exactly what a check of the whole
/// score finds without those rules: a rule that reports a violation at a tick before it could be
/// decided would be missed, and the search would learn of it only at the end.
#[allow(dead_code)]
pub fn assert_prefixes_collect(checker: &Checker, score: &Score, name: &str) {
    use fux_core::Role;
    use fux_core::rules::{Analysis, Window};

    let prefix = |tick: u32| {
        let mut part = score.clone();
        for voice in part
            .voices
            .iter_mut()
            .filter(|v| v.role != Role::CantusFirmus)
        {
            let keep = voice
                .timed()
                .take_while(|(start, _)| *start <= tick)
                .count();
            voice.notes.truncate(keep);
        }
        part
    };
    let targets: Vec<bool> = score
        .voices
        .iter()
        .map(|v| v.role != Role::CantusFirmus)
        .collect();
    let mut whole = Vec::new();
    let open = Window {
        from: 0,
        until: u32::MAX,
        complete: false,
    };
    checker.check_window(score, &Analysis::new(score), open, &targets, &mut whole);
    let ticks: Vec<u32> = Analysis::new(score).slices.iter().map(|s| s.tick).collect();
    let mut found = Vec::new();
    for (k, &tick) in ticks.iter().enumerate() {
        let last = k + 1 == ticks.len();
        let part = prefix(tick);
        let analysis = Analysis::new(&part);
        let window = Window {
            from: if k == 0 { 0 } else { ticks[k - 1] + 1 },
            until: if last { u32::MAX } else { tick },
            complete: false,
        };
        let mut here = Vec::new();
        checker.check_window(&part, &analysis, window, &targets, &mut here);
        assert_until_forbidden_agrees(checker, &part, &analysis, window, &targets, &here, name);
        found.extend(here);
    }
    let key = |v: &fux_core::rules::Violation| (v.tick, v.voice, v.rule, v.note);
    found.sort_by_key(key);
    whole.sort_by_key(key);
    let missing: Vec<String> = whole
        .iter()
        .filter(|v| !found.contains(v))
        .map(|v| format!("{} at {} (voice {})", v.rule, v.tick, v.voice))
        .collect();
    let extra: Vec<String> = found
        .iter()
        .filter(|v| !whole.contains(v))
        .map(|v| format!("{} at {} (voice {})", v.rule, v.tick, v.voice))
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "{name}: missing {missing:?}, extra {extra:?}"
    );
}

/// The usual choir of `voices` voices, as `fux free` sets it, from the top.
#[allow(dead_code)]
pub fn choir(voices: usize) -> Vec<VoiceType> {
    use VoiceType::*;
    match voices {
        2 => vec![Soprano, Tenor],
        3 => vec![Alto, Tenor, Bass],
        4 => vec![Soprano, Alto, Tenor, Bass],
        5 => vec![Soprano, Alto, Tenor, Tenor, Bass],
        _ => vec![Soprano, Soprano, Alto, Tenor, Tenor, Bass],
    }
}

/// A texted result: no prohibition, also when the user's profile checks it again with every TXT
/// rule active, a valid score whose voices all sing the text, the syllables written out, and the
/// underlay of every voice as the oracle wants it ([`assert_sung`]) and the cheapest of its
/// notes ([`assert_cheapest`]).
#[allow(dead_code)]
pub fn assert_texted(generated: &Generated, text: &Text, label: &str) {
    let score = &generated.score;
    let forbidden: Vec<_> = generated
        .report
        .violations
        .iter()
        .filter(|v| v.is_forbidden())
        .collect();
    assert!(forbidden.is_empty(), "{label}: {forbidden:?}");
    score.validate().unwrap_or_else(|e| panic!("{label}: {e}"));
    assert!(score.is_free(), "{label}");
    let sung = score
        .text
        .as_ref()
        .unwrap_or_else(|| panic!("{label}: no text"));
    assert_eq!(*sung.text, *text, "{label}");
    assert_eq!(sung.voices, vec![true; score.voices.len()], "{label}");
    // The search priced the music under a copy of the profile; the profile itself finds the same.
    let report = Checker::new(&Profile::fux()).check(score);
    assert_eq!(report.violations, generated.report.violations, "{label}");
    assert_eq!(text_rules().check(score).forbidden(), 0, "{label}");
    let restated = gate_restarts(score, text);
    for (v, voice) in score.voices.iter().enumerate() {
        let label = format!("{label}, voice {v}");
        assert_sung(text, &voice.notes, &restated, &label);
        for (i, note) in voice.notes.iter().enumerate() {
            assert_eq!(
                note.syllable.is_some(),
                note.lyric.is_some(),
                "{label}, note {i}"
            );
        }
        let lyrics = score.voice_lyrics(v).unwrap();
        let last = text.syllable_label(text.syllables.len() - 1, true);
        assert!(lyrics.ends_with(&last), "{label}: {lyrics}");
    }
    assert_cheapest(generated, label);
}

/// Every voice of a result that sings the text and is no cantus firmus carries the cheapest
/// underlay of its notes under Fux's profile, as the dynamic program lays it, at the cost that
/// the TXT rules of the result's report find. In an imitative result the underlay keeps the
/// syllables that the plan pins at the attacks of its heads ([`planned_pins`]) and begins no
/// syllable of a point's own words before the point begins (the plan's gates).
#[allow(dead_code)]
pub fn assert_cheapest(generated: &Generated, label: &str) {
    let score = &generated.score;
    let sung = score.text.as_ref().expect("a text");
    let goals = Goal::of(&score.cadences, score.meter);
    let restarts = score
        .imitation
        .as_deref()
        .map(|plan| plan.restarts(&sung.text))
        .unwrap_or_default();
    let model = Model::new(&sung.text, score.mode, score.meter, &goals)
        .with_restarts(&sung.text, &restarts);
    let levels = Levels::from(&Checker::new(&Profile::fux()));
    let pins = planned_pins(score);
    let gates = |v: usize| {
        score
            .imitation
            .as_deref()
            .map(|plan| plan.gates(v))
            .filter(|gates| !gates.is_empty())
    };
    let lyrics = |notes: &[Note]| notes.iter().map(|n| n.lyric).collect::<Vec<_>>();
    let mut singing = 0;
    for (v, voice) in score.voices.iter().enumerate() {
        if !sung.voices[v] || voice.is_cantus_firmus() {
            continue;
        }
        singing += 1;
        let mut bare = voice.notes.clone();
        for note in &mut bare {
            note.lyric = None;
        }
        let pinned = (!pins[v].is_empty()).then_some(&pins[v][..]);
        let path = model
            .best_pinned(&levels, &bare, pinned, gates(v).as_deref())
            .unwrap_or_else(|| panic!("{label}, voice {v}: no underlay"));
        path.apply(&mut bare);
        assert_eq!(lyrics(&bare), lyrics(&voice.notes), "{label}, voice {v}");
        let cost: u32 = generated
            .report
            .violations
            .iter()
            .filter(|x| x.voice == v && x.rule.starts_with("TXT-"))
            .map(|x| x.level.cost())
            .sum();
        assert_eq!(cost, path.cost, "{label}, voice {v}");
    }
    assert!(singing > 0, "{label}: no voice sings");
}

/// Where a voice may begin its text again right after the last syllable of a sense unit `x`:
/// at the start of each unit or tail of the sentence, up to x's own unit.
#[allow(dead_code)]
pub fn restarts(text: &Text, x: usize) -> Vec<usize> {
    let unit = usize::from(text.syllables[x].unit);
    let first = usize::from(text.units[unit].sentence);
    let from = usize::from(text.units[first].first);
    let to = usize::from(text.units[unit].last);
    (from..=to)
        .filter(|&s| {
            let flags = text.syllables[s].flags;
            flags.contains(SylFlags::UNIT_START) || flags.contains(SylFlags::TAIL_START)
        })
        .collect()
}

/// The syllables of `text` after which a voice of an imitative `score` may restate its sentence
/// besides the ends of sense units, by the oracle's own count: the last syllable before each
/// point after the first whose words go beyond those of every point before it, unless it ends a
/// unit already.
#[allow(dead_code)]
pub fn gate_restarts(score: &Score, text: &Text) -> Vec<usize> {
    let Some(plan) = score.imitation.as_deref() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut set = 0;
    for (p, point) in plan.points.iter().enumerate() {
        let own = usize::from(point.words.start).max(set);
        set = set.max(usize::from(point.words.end));
        if p > 0
            && own > 0
            && own < usize::from(point.words.end)
            && !text.syllables[own - 1].flags.contains(SylFlags::UNIT_END)
        {
            out.push(own - 1);
        }
    }
    out
}

/// Where a voice may begin its text again right after syllable `x`, which ends no sense unit
/// but comes before a later point of an imitation: at the start of each unit or tail of its
/// sentence up to `x`.
#[allow(dead_code)]
pub fn restarts_within(text: &Text, x: usize) -> Vec<usize> {
    let unit = usize::from(text.syllables[x].unit);
    let first = usize::from(text.units[unit].sentence);
    let from = usize::from(text.units[first].first);
    (from..=x)
        .filter(|&s| {
            let flags = text.syllables[s].flags;
            flags.contains(SylFlags::UNIT_START) || flags.contains(SylFlags::TAIL_START)
        })
        .collect()
}

/// The oracle, which uses no code of the rules: the voice's first attack begins the text; each
/// new syllable is the next one or, right after the last syllable of a sense unit, the start of
/// a unit or tail of its sentence, and so after one of `restated` ([`gate_restarts`]) up to it;
/// every attack after a rest begins a syllable; no rest, tie continuation or eighth carries one;
/// and the last note sings the text's last syllable.
#[allow(dead_code)]
pub fn assert_sung(text: &Text, notes: &[Note], restated: &[usize], label: &str) {
    let mut current: Option<usize> = None;
    let mut entry = true;
    let mut tied = false;
    for (i, note) in notes.iter().enumerate() {
        let continued = tied;
        tied = note.tie;
        if note.is_rest() || continued || note.duration == Duration::EIGHTH {
            assert_eq!(note.lyric, None, "{label}: a syllable on note {i}");
        }
        if note.is_rest() {
            entry = true;
            continue;
        }
        if continued {
            continue;
        }
        match note.lyric {
            Some(lyric) => {
                let y = lyric
                    .index()
                    .unwrap_or_else(|| panic!("{label}: note {i} sings no syllable of the text"));
                let follows = match current {
                    None => y == 0,
                    Some(x) => {
                        let unit_end = text.syllables[x].flags.contains(SylFlags::UNIT_END);
                        y == x + 1
                            || (unit_end && restarts(text, x).contains(&y))
                            || (restated.contains(&x) && restarts_within(text, x).contains(&y))
                    }
                };
                assert!(
                    follows,
                    "{label}: syllable {y} after {current:?} on note {i}"
                );
                current = Some(y);
            }
            None => assert!(!entry, "{label}: note {i} enters without a syllable"),
        }
        entry = false;
    }
    assert!(
        notes.last().is_some_and(|n| !n.is_rest()),
        "{label}: the voice ends with a rest"
    );
    assert_eq!(
        current,
        Some(text.syllables.len() - 1),
        "{label}: the last note does not sing the last syllable"
    );
}

/// The written note of `voice` whose attack falls on `tick`, by its index: a pitch that is no
/// continuation of a tie.
fn attack_at(notes: &[Note], tick: u32) -> Option<usize> {
    let mut at = 0;
    for (i, note) in notes.iter().enumerate() {
        if at == tick {
            let continued = i > 0 && notes[i - 1].tie && !notes[i - 1].is_rest();
            return (!note.is_rest() && !continued).then_some(i);
        }
        if at > tick {
            return None;
        }
        at += note.duration.ticks();
    }
    None
}

/// What the first entry of each point of an imitative score that has a head note sings on it:
/// per point, per head note, the syllable its attack begins (`None` where it holds one), or
/// `None` where the voices do not sing. The entries of a point pin these syllables.
fn point_syllables(score: &Score) -> Vec<Vec<Option<Option<usize>>>> {
    let Some(plan) = score.imitation.as_deref() else {
        return Vec::new();
    };
    let sings = |v: usize| score.text.as_ref().is_some_and(|t| t.voices[v]);
    plan.points
        .iter()
        .map(|point| {
            let longest = point.entries.iter().map(|e| e.notes.len()).max();
            (0..longest.unwrap_or(0))
                .map(|k| {
                    let entry = point.entries.iter().find(|e| e.notes.len() > k)?;
                    if !sings(entry.voice) {
                        return None;
                    }
                    let notes = &score.voices.get(entry.voice)?.notes;
                    let tick = entry.start + entry.notes[..k].iter().map(|n| n.1).sum::<u32>();
                    let i = attack_at(notes, tick)?;
                    Some(notes[i].lyric.and_then(|l| l.index()))
                })
                .collect()
        })
        .collect()
}

/// The underlay choices that the plan of an imitative score pins in each voice, by tick: at each
/// attack of a head, the syllable that the first entry of its point with that head note begins
/// on it, or a melisma. Empty for a voice without heads and in a score without a plan.
#[allow(dead_code)]
pub fn planned_pins(score: &Score) -> Vec<Vec<(u32, Choice)>> {
    let mut pins = vec![Vec::new(); score.voices.len()];
    let Some(plan) = score.imitation.as_deref() else {
        return pins;
    };
    let syllables = point_syllables(score);
    for (point, sung) in plan.points.iter().zip(&syllables) {
        for entry in &point.entries {
            let mut tick = entry.start;
            for (k, &(_, ticks)) in entry.notes.iter().enumerate() {
                if let Some(Some(syllable)) = sung.get(k) {
                    let choice = match syllable {
                        Some(y) => Choice::Start(*y as u16),
                        None => Choice::Hold,
                    };
                    pins[entry.voice].push((tick, choice));
                }
                tick += ticks;
            }
        }
    }
    for voice in &mut pins {
        voice.sort_by_key(|&(tick, _)| tick);
    }
    pins
}

/// An imitative result keeps its plan, which its score carries; the oracle uses no code of the
/// rules (docs/regeln.md § 18). Every voice enters in the first point, the second entry of each
/// point follows the first within two breves, every head has four notes or more within an
/// octave, and every planned entry sounds as planned: on a minim beat, at its tick with its
/// pitches, values and ties (the last note may go on), after silence from the start for a
/// voice's first entry and after at least its rest of a minim or more for any other; and where
/// the voices sing, each attack of a head carries the syllable that the point's first entry with
/// that note carries, none where that one holds, the first beginning the point's words. No voice
/// begins the words of a point that no point before it set before its own entry in the point,
/// or without one before the point's second entry.
#[allow(dead_code)]
pub fn assert_imitated(score: &Score, label: &str) {
    let plan = score
        .imitation
        .as_deref()
        .unwrap_or_else(|| panic!("{label}: no plan in the score"));
    let n = score.voices.len();
    assert!(!plan.points.is_empty(), "{label}: a plan without points");
    let first: BTreeSet<usize> = plan.points[0].entries.iter().map(|e| e.voice).collect();
    assert_eq!(
        first,
        (0..n).collect::<BTreeSet<_>>(),
        "{label}: every voice in the first point"
    );
    let syllables = point_syllables(score);
    let mut entered = vec![false; n];
    let mut entries: Vec<_> = plan
        .points
        .iter()
        .enumerate()
        .flat_map(|(p, point)| point.entries.iter().map(move |e| (p, e)))
        .collect();
    entries.sort_by_key(|(_, e)| e.start);
    for (p, entry) in entries {
        let point = &plan.points[p];
        let at = format!(
            "{label}: point {}, voice {} at {}",
            p + 1,
            entry.voice,
            entry.start
        );
        assert!(entry.voice < n, "{at}: no such voice");
        assert_eq!(entry.start % 4, 0, "{at}: off the minim beats");
        assert!(entry.notes.len() >= 4, "{at}: {} notes", entry.notes.len());
        let steps = || entry.notes.iter().map(|&(pitch, _)| pitch.diatonic());
        let span = steps().max().unwrap() - steps().min().unwrap();
        assert!(span <= 7, "{at}: a head beyond an octave ({span} steps)");
        let again = std::mem::replace(&mut entered[entry.voice], true);
        assert_eq!(entry.rest == 0, !again, "{at}: rest {}", entry.rest);
        assert!(
            entry.rest == 0 || entry.rest >= 4,
            "{at}: rest {}",
            entry.rest
        );
        let notes = &score.voices[entry.voice].notes;
        // Silence before it: from the start for a first entry, for its rest at least otherwise.
        let silent_from = if entry.rest == 0 {
            0
        } else {
            entry.start - entry.rest
        };
        let mut tick = 0;
        for note in notes {
            let end = tick + note.duration.ticks();
            if tick < entry.start && end > silent_from {
                assert!(note.is_rest(), "{at}: sounds {:?} at {tick}", note.pitch);
            }
            tick = end;
        }
        // The head, note by note.
        let sings = score.text.as_ref().is_some_and(|t| t.voices[entry.voice]);
        let mut tick = entry.start;
        for (k, &(pitch, value)) in entry.notes.iter().enumerate() {
            let last = k + 1 == entry.notes.len();
            let i = attack_at(notes, tick)
                .unwrap_or_else(|| panic!("{at}: no attack of note {} at {tick}", k + 1));
            let (mut j, mut held) = (i, 0);
            loop {
                let note = &notes[j];
                assert_eq!(note.pitch, Some(pitch), "{at}: note {} at {tick}", k + 1);
                held += note.duration.ticks();
                if held >= value {
                    break;
                }
                assert!(note.tie, "{at}: note {} ends before its value", k + 1);
                j += 1;
                assert!(j < notes.len(), "{at}: the voice ends in its head");
            }
            assert_eq!(held, value, "{at}: note {} is not {value} ticks", k + 1);
            assert!(
                last || !notes[j].tie,
                "{at}: note {} lasts beyond its value",
                k + 1
            );
            if sings {
                let sung = notes[i].lyric.map(|l| {
                    l.index()
                        .unwrap_or_else(|| panic!("{at}: no syllable of the text"))
                });
                if let Some(wanted) = syllables[p][k] {
                    assert_eq!(
                        sung,
                        wanted,
                        "{at}: note {} sings otherwise than the point's first entry",
                        k + 1
                    );
                }
                if k == 0 {
                    assert_eq!(
                        sung,
                        Some(usize::from(point.words.start)),
                        "{at}: the entry does not begin the point's words"
                    );
                }
            }
            tick += value;
        }
    }
    for (p, point) in plan.points.iter().enumerate() {
        if let [leader, second, ..] = &point.entries[..] {
            assert!(
                second.start > leader.start && second.start - leader.start <= 32,
                "{label}: point {}, the second entry after {} ticks",
                p + 1,
                second.start - leader.start
            );
        }
    }
    // No voice begins the words of a point (those no point before it set) before its own entry
    // in the point, or without one before the point's second entry.
    let Some(text) = score.text.as_ref() else {
        return;
    };
    for (v, voice) in score.voices.iter().enumerate() {
        if !text.voices[v] {
            continue;
        }
        let mut set = 0;
        let mut gates = Vec::new();
        for (p, point) in plan.points.iter().enumerate() {
            let own = point.words.start.max(set)..point.words.end;
            set = set.max(point.words.end);
            let entry = point.entries.iter().find(|e| e.voice == v);
            let joins = entry.or(point.entries.get(1)).or(point.entries.first());
            if let Some(start) = joins.map(|e| e.start).filter(|_| p > 0 && !own.is_empty()) {
                gates.push((own, start, p));
            }
        }
        for (tick, note) in voice.timed() {
            let Some(y) = note.lyric.and_then(|l| l.index()) else {
                continue;
            };
            for (words, start, p) in &gates {
                assert!(
                    !words.contains(&(y as u16)) || tick >= *start,
                    "{label}: voice {v} begins syllable {y} at {tick}, before it takes up point \
                     {} at {start}",
                    p + 1
                );
            }
        }
    }
}

/// The detector, counting entries after rests of a minim, finds every point of an imitative
/// score's plan: one point that has each of its planned entries.
#[allow(dead_code)]
pub fn assert_detected(score: &Score, label: &str) {
    let plan = score.imitation.as_deref().expect("a plan in the score");
    let found = detect(score, 4);
    for (p, point) in plan.points.iter().enumerate() {
        let planned: Vec<(usize, u32)> = point.entries.iter().map(|e| (e.voice, e.start)).collect();
        let whole = found.iter().any(|f| {
            planned
                .iter()
                .all(|&(v, t)| f.entries.iter().any(|e| e.voice == v && e.start == t))
        });
        let seen: Vec<Vec<(usize, u32)>> = found
            .iter()
            .map(|f| f.entries.iter().map(|e| (e.voice, e.start)).collect())
            .collect();
        assert!(
            whole,
            "{label}: point {} with the entries {planned:?} is not found, only {seen:?}",
            p + 1
        );
    }
}
