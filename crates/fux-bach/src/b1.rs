//! The runs of phase B1 (protocol in `docs/design/bach-fugen/b1/protokoll.md`): the count of the
//! rules on Bach's fugues, the comparison of search step and check, and the re-setting of the
//! labelled fugues of WTC I with the machine of `engine`.

use std::path::Path;

use anyhow::{Context, Result};
use rayon::prelude::*;

use crate::algomus::{self, Label};
use crate::cells;
use crate::check::{self, Census};
use crate::chord;
use crate::draft::{Draft, Options, Setting, VoiceDraft, set};
use crate::engine::{Machine, Outcome};
use crate::key::Harmony;
use crate::neusatz::{Verdict, bands_without, verdict, write};
use crate::pitch::Pitch;
use crate::rhythm;
use crate::rules::{Level, Profile, Rule};
use crate::score::{Note, Piece};
use crate::stats::{self, COUNT, NAMES, STRUCK_DISSONANCE};
use crate::study::{analyses, corpus_measures, find, load, name, shown};
use crate::time::Tick;

/// The fugues of WTC I on which rules and prices are set, and those of the test.
pub const DEVELOPMENT: [usize; 11] = [2, 16, 6, 7, 13, 17, 21, 23, 10, 15, 22];
pub const TEST: [usize; 12] = [1, 3, 4, 5, 8, 9, 12, 14, 18, 19, 20, 24];

/// The measures the calibration singles out: repetition, outer voices in the same direction,
/// complete triad.
pub const SINGLED: [usize; 3] = [4, 27, 28];

fn percent(census: &Census, rule: Rule) -> String {
    match census.share(rule) {
        Some(share) => format!("{:6.2} %", 100.0 * share),
        None => "       -".to_string(),
    }
}

/// The kern files of a directory that the reader takes, and the names of the others.
pub fn readable(dir: &Path) -> Result<(Vec<crate::kern::KernPiece>, Vec<String>)> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| dir.display().to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "krn"))
        .collect();
    files.sort();
    let mut read = Vec::new();
    let mut unread = Vec::new();
    for file in &files {
        let text = String::from_utf8_lossy(&std::fs::read(file)?).into_owned();
        let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
        match crate::kern::read(stem, &text) {
            Ok(piece) => read.push(piece),
            Err(_) => unread.push(stem.to_string()),
        }
    }
    Ok((read, unread))
}

/// Z1: every rule counted on the 48 fugues (estimated chords), on the seven analysed fugues
/// (Tymoczko's chords) and on the Art of Fugue (estimated chords).
pub fn census(data: &Path, profile: &Profile) -> Result<()> {
    let read = load(data)?;
    let count = |pieces: Vec<(&Piece, Harmony)>| -> Census {
        let parts: Vec<Census> = pieces
            .par_iter()
            .map(|(piece, harmony)| check::census(piece, harmony, profile))
            .collect();
        let mut total = Census::default();
        for part in &parts {
            total.add(part);
        }
        total
    };
    let wtc = count(
        read.iter()
            .map(|r| (&r.piece, chord::estimate(&r.piece)))
            .collect(),
    );
    let analysed = analyses(data, &read)?;
    let seven_own = count(
        analysed
            .iter()
            .map(|(piece, _, _)| (piece, chord::estimate(piece)))
            .collect(),
    );
    let seven = count(
        analysed
            .iter()
            .map(|(piece, harmony, _)| (piece, harmony.clone()))
            .collect(),
    );
    let (art, unread) = readable(&data.join("kdf"))?;
    if !unread.is_empty() {
        println!("Art of Fugue, not read: {}", unread.join(", "));
    }
    let kdf = count(
        art.iter()
            .map(|r| (&r.piece, chord::estimate(&r.piece)))
            .collect(),
    );
    println!(
        "profile {}: findings per opportunity; WTC 48 fugues with estimated chords, the seven analysed fugues with estimated and with Tymoczko's chords, the Art of Fugue ({} pieces) with estimated chords",
        profile.name,
        art.len()
    );
    println!(
        "{:22} {:6} {:>9} {:>7} {:>8}   {:>8} {:>8}   {:>9} {:>7} {:>8}",
        "rule",
        "level",
        "WTC opp.",
        "found",
        "share",
        "7 est.",
        "7 Tym.",
        "KdF opp.",
        "found",
        "share"
    );
    for rule in Rule::ALL {
        let k = *rule as usize;
        let level = profile.level(*rule);
        // A prohibition whose share in the second corpus differs by more than the double and
        // more than a percentage point.
        let mut mark = "";
        if level == Level::Forbidden
            && let (Some(a), Some(b)) = (wtc.share(*rule), kdf.share(*rule))
            && (a - b).abs() > 0.01
            && (b > 2.0 * a || a > 2.0 * b)
        {
            mark = "  differs";
        }
        println!(
            "{:22} {:6} {:9} {:7} {}   {} {}   {:9} {:7} {}{}",
            rule.id(),
            level.to_string(),
            wtc.chances[k],
            wtc.found[k],
            percent(&wtc, *rule),
            percent(&seven_own, *rule),
            percent(&seven, *rule),
            kdf.chances[k],
            kdf.found[k],
            percent(&kdf, *rule),
            mark
        );
    }
    Ok(())
}

/// A fugue of Bach's as the check reads it: name, prohibitions broken by rule, cost, and whether
/// the search with every tone given adds up to the same.
type Original = (String, Vec<(Rule, usize)>, u32, bool);

/// The measures of a setting and of its original, and the counts of the rules on both.
type Rates = ([f64; COUNT], [f64; COUNT], Census, Census);

/// Z2 and Z3 on Bach's own fugues: what the check reports as forbidden, rule by rule, and
/// whether a search in which every tone is given adds up to the same as the check.
pub fn originals(data: &Path, profile: &Profile, detail: bool) -> Result<()> {
    let read = load(data)?;
    let rows: Vec<Original> = read
        .par_iter()
        .map(|r| {
            let harmony = chord::estimate(&r.piece);
            let (task, tones) = check::task_of(&r.piece, &harmony, profile);
            let report = check::report(&task, &tones);
            let same = match Machine::new(&task).search() {
                Outcome::Found(found) => found.tally == report.tally,
                Outcome::DeadEnd(..) => false,
            };
            (
                r.piece.name.clone(),
                report.forbidden_by_rule(),
                report.cost(),
                same,
            )
        })
        .collect();
    let mut sums = vec![0usize; Rule::COUNT];
    let mut differing = 0;
    for (piece, forbidden, cost, same) in &rows {
        for (rule, count) in forbidden {
            sums[*rule as usize] += count;
        }
        differing += usize::from(!*same);
        if detail {
            let list: Vec<String> = forbidden
                .iter()
                .map(|(rule, count)| format!("{} {count}", rule.id()))
                .collect();
            println!(
                "{piece}: cost {cost}; {}{}",
                list.join(", "),
                if *same { "" } else { "  SEARCH DIFFERS" }
            );
        }
    }
    println!(
        "profile {}: prohibitions broken by the 48 fugues against their estimated chords",
        profile.name
    );
    for rule in Rule::ALL {
        if sums[*rule as usize] > 0 {
            println!("  {:22} {:6}", rule.id(), sums[*rule as usize]);
        }
    }
    println!(
        "search with every tone given against the check of the notes: {} of {} fugues differ",
        differing,
        rows.len()
    );
    Ok(())
}

/// The draft of a re-setting: Bach's entries of the subject stay, the voices are silent until
/// their first tone and keep their ranges.
pub fn draft(original: &Piece, labels: &[Label], harmony: &Harmony) -> Draft {
    let voices = original
        .voices
        .iter()
        .enumerate()
        .map(|(v, voice)| {
            let mut given: Vec<Note> = Vec::new();
            let mut spans: Vec<(Tick, Tick)> = Vec::new();
            for label in labels {
                if label.voice != Some(v) || !label.is_subject() {
                    continue;
                }
                let notes = label.notes(original);
                let Some(last) = notes.last() else {
                    continue;
                };
                let span = (label.start, label.end.max(last.off));
                // Entries that overlap in one voice keep the earlier one.
                if spans
                    .iter()
                    .any(|(from, until)| span.0 < *until && *from < span.1)
                {
                    continue;
                }
                given.extend(
                    notes
                        .iter()
                        .filter(|n| !given.contains(n))
                        .copied()
                        .collect::<Vec<_>>(),
                );
                spans.push(span);
            }
            given.sort_by_key(|n| n.on);
            let first = voice.notes.first().map_or(original.end, |n| n.on);
            let middle = Pitch::new(0, 0, 4);
            VoiceDraft {
                figures: Vec::new(),
                low: voice.notes.iter().map(|n| n.pitch).min().unwrap_or(middle),
                high: voice.notes.iter().map(|n| n.pitch).max().unwrap_or(middle),
                given,
                spans,
                silent: vec![(Tick::MIN / 2, first)],
                pedals: Vec::new(),
            }
        })
        .collect();
    let last_chord = harmony.spans.last().map_or(original.end, |s| s.on);
    Draft {
        name: original.name.clone(),
        key: original.key,
        meter: original.meter,
        origin: original.origin,
        end: original.end,
        voices,
        final_hold: Some(last_chord.max(original.end - original.meter.bar())),
        invertible: Vec::new(),
        root_at_hold: false,
        longest: None,
        goes_on: false,
    }
}

/// A draft in which these groups of tones of the original stay (voice, tones), everything else
/// is free.
pub fn draft_with(original: &Piece, harmony: &Harmony, groups: &[(usize, Vec<Note>)]) -> Draft {
    let labels: Vec<Label> = groups
        .iter()
        .filter_map(|(voice, notes)| {
            Some(Label {
                kind: "S".to_string(),
                tag: String::new(),
                start: notes.first()?.on,
                end: notes.last()?.off,
                voice: Some(*voice),
            })
        })
        .collect();
    draft(original, &labels, harmony)
}

/// The machine on the fugues of WTC II, of which only the first two entries stay: no count of
/// the protocol, a look at meters, keys and voice counts the development fugues do not have.
pub fn robust(data: &Path, options: &Options) -> Result<()> {
    let read = load(data)?;
    let pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
    let rows = corpus_measures(&read);
    let (_, mark) = stats::yardstick(&rows);
    let second: Vec<&Piece> = pieces
        .iter()
        .filter(|p| p.name.starts_with("wtc2"))
        .collect();
    let lines: Vec<String> = second
        .par_iter()
        .map(|original| {
            let Some(exposition) = crate::subject::from_entries(original) else {
                return format!("{}: no exposition found", original.name);
            };
            let harmony = chord::estimate(original);
            let draft = draft_with(
                original,
                &harmony,
                &[
                    (exposition.subject_voice, exposition.subject.clone()),
                    (exposition.answer_voice, exposition.answer.clone()),
                ],
            );
            let profile = rhythm::value_profile(exposition.subject.iter());
            let (table, intervals, _) = cells::table_for(
                pieces.iter(),
                original.meter.unit(),
                profile,
                &original.name,
                8,
            );
            match set(&draft, &harmony, &table, &intervals, options) {
                Err(dead) => format!(
                    "{} {} {} voices: no setting, {} dead ends, first at bar {} ({})",
                    original.name,
                    original.meter,
                    original.voices.len(),
                    dead.len(),
                    dead.first().map_or(0, |d| original.bar_of(d.0).0),
                    dead.first()
                        .and_then(|d| d.1)
                        .map_or("nothing to choose from", |r| r.id())
                ),
                Ok(setting) => {
                    let bands = bands_without(&pieces, &rows, &original.name);
                    let new = verdict(&setting.piece, &bands);
                    let same = same_as_check(&draft, &harmony, &setting, options);
                    format!(
                        "{} {:5} {} voices {:5} tones: cost {:5} junction {} own {} tries {} {:5.1} s; hits {} (yardstick {mark:.0}), struck dissonance {}; check {}",
                        original.name,
                        original.meter.to_string(),
                        original.voices.len(),
                        setting.piece.note_count(),
                        setting.tally.cost,
                        setting.tally.count.junction,
                        setting.tally.count.own,
                        setting.attempts,
                        setting.seconds,
                        new.hits,
                        if new.struck_inside { "inside" } else { "outside" },
                        if same { "same" } else { "DIFFERS" }
                    )
                }
            }
        })
        .collect();
    println!(
        "profile {}: WTC II, the first two entries given",
        options.profile.name
    );
    for line in lines {
        println!("{line}");
    }
    Ok(())
}

/// One re-setting with what is needed to judge it.
pub struct Run {
    pub setting: Setting,
    pub draft: Draft,
    pub harmony: Harmony,
    pub learned_from: Vec<String>,
    pub given_tones: usize,
    /// Z3: the check of the finished piece adds up to what the search paid.
    pub same: bool,
}

/// Z3 for a setting: the tones read again by the machine, and the piece read from its notes
/// with the roles of the draft, give the sums of the search.
pub fn same_as_check(
    draft: &Draft,
    harmony: &Harmony,
    setting: &Setting,
    options: &Options,
) -> bool {
    let task = draft.task(setting.parts.clone(), harmony, &options.profile, 1, 0);
    let mut quiet = crate::engine::Quiet;
    let again = Machine::new(&task).replay(&setting.tones, false, &mut quiet);
    // From the notes: the events are built anew, the wishes and ranges of the plan are gone.
    let (parts, tones) = check::parts_of(
        &setting.piece,
        &|v, note| draft.is_given(v, note),
        &|v, i| {
            let on = setting.piece.voices[v].notes[i].on;
            draft.voices[v]
                .pedals
                .iter()
                .any(|(from, until)| *from <= on && on < *until)
        },
    );
    let fresh = draft.task(parts, harmony, &options.profile, 1, 0);
    let read = check::report(&fresh, &tones);
    again == setting.tally
        && read.tally.cost == setting.tally.cost
        && read.tally.count == setting.tally.count
        && !read.tally.stop
}

pub fn run(pieces: &[Piece], original: &Piece, labels: &[Label], options: &Options) -> Result<Run> {
    run_over(pieces, original, labels, options, None)
}

/// A re-setting over the chords estimated from the original, or over a scaffold generated from
/// the entries, cadences and pedal points alone (phase B2).
pub fn run_over(
    pieces: &[Piece],
    original: &Piece,
    labels: &[Label],
    options: &Options,
    model: Option<&crate::scaffold::Model>,
) -> Result<Run> {
    let (harmony, pedal_tones) = match model {
        None => (chord::estimate(original), Vec::new()),
        Some(model) => {
            let (plan, harmony) = crate::b2::scaffold_for(original, labels, model, options.seed);
            let given = crate::b2::given_lines(original, labels);
            let tones = crate::b2::pedal_tones(&plan, original, &given);
            (harmony, tones)
        }
    };
    let mut draft = draft(original, labels, &harmony);
    // Bach's entries are what the rhythm tables are chosen by; the pedal tones come on top.
    let subject: Vec<Note> = draft.voices.iter().flat_map(|v| v.given.clone()).collect();
    if let Some(lowest) = draft.voices.last_mut() {
        for tone in pedal_tones {
            lowest.given.push(tone);
            lowest.spans.push((tone.on, tone.off));
            lowest.pedals.push((tone.on, tone.off));
        }
        lowest.given.sort_by_key(|n| n.on);
    }
    let profile = rhythm::value_profile(subject.iter());
    let (table, intervals, learned_from) = cells::table_for(
        pieces.iter(),
        original.meter.unit(),
        profile,
        &original.name,
        8,
    );
    let given_tones = subject.len();
    let setting = set(&draft, &harmony, &table, &intervals, options).map_err(|dead| {
        let bars: Vec<String> = dead
            .iter()
            .map(|(at, why)| {
                format!(
                    "{} ({})",
                    original.bar_of(*at).0,
                    why.map_or("nothing to choose from", |r| r.id())
                )
            })
            .collect();
        anyhow::anyhow!(
            "{}: no setting in {} attempts; dead ends in bars {}",
            original.name,
            dead.len(),
            bars.join(", ")
        )
    })?;
    let same = same_as_check(&draft, &harmony, &setting, options);
    Ok(Run {
        setting,
        draft,
        harmony,
        learned_from,
        given_tones,
        same,
    })
}

type Judged = (Run, Verdict, Verdict);

/// Z4 and Z5 on a set of fugues with several seeds: one line per run and the sums.
pub fn many(
    data: &Path,
    numbers: &[usize],
    seeds: &[u64],
    options: &Options,
    out: Option<&Path>,
    detail: bool,
) -> Result<()> {
    many_over(data, numbers, seeds, options, out, detail, false)
}

/// As `many`; with `generated` over scaffolds generated from the plan instead of the chords
/// estimated from the original.
pub fn many_over(
    data: &Path,
    numbers: &[usize],
    seeds: &[u64],
    options: &Options,
    out: Option<&Path>,
    detail: bool,
    generated: bool,
) -> Result<()> {
    let model = if generated {
        Some(crate::b2::learn(data)?.0)
    } else {
        None
    };
    let model = model.as_ref();
    let read = load(data)?;
    let pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
    let rows = corpus_measures(&read);
    let (_, mark) = stats::yardstick(&rows);
    let jobs: Vec<(usize, u64)> = numbers
        .iter()
        .flat_map(|n| seeds.iter().map(move |s| (*n, *s)))
        .collect();
    let results: Vec<(usize, u64, Result<Judged>)> = jobs
        .par_iter()
        .map(|(number, seed)| {
            let outcome = (|| {
                let original = find(&read, &name(1, *number))?;
                let labels = algomus::read_for(data, *number)?;
                let options = Options {
                    seed: *seed,
                    ..options.clone()
                };
                let result = run_over(&pieces, original, &labels, &options, model)?;
                let bands = bands_without(&pieces, &rows, &original.name);
                let new = verdict(&result.setting.piece, &bands);
                let old = verdict(original, &bands);
                Ok((result, new, old))
            })();
            (*number, *seed, outcome)
        })
        .collect();
    let mut clean = 0;
    let mut hits_ok = 0;
    let mut struck_ok = 0;
    let mut passed = 0;
    let mut differing = 0;
    let mut singled = [0usize; 3];
    let mut outside = [0usize; COUNT];
    println!(
        "profile {}, beam {}, at most {} rhythms, chords {}",
        options.profile.name,
        options.beam,
        options.attempts,
        if generated {
            "from the generated scaffold"
        } else {
            "estimated from the original"
        }
    );
    println!(
        "fugue seed voices tones given  cost  plan junct own excused tries   sec  hits (Bach)  struck (Bach)  rep sim tri  check  outside"
    );
    for (number, seed, outcome) in &results {
        match outcome {
            Err(error) => println!("I/{number:<2}  {seed:3}  no setting: {error}"),
            Ok((result, new, old)) => {
                let original = find(&read, &name(1, *number))?;
                let bands = bands_without(&pieces, &rows, &original.name);
                let tones = result.setting.piece.note_count();
                let misses: Vec<usize> = (0..COUNT)
                    .filter(|k| !bands.contains(*k, new.row[*k]))
                    .collect();
                for k in &misses {
                    outside[*k] += 1;
                }
                let tally = result.setting.tally;
                let a = tally.count.junction == 0;
                let b = new.hits as f64 >= mark;
                let c = new.struck_inside;
                clean += usize::from(a);
                hits_ok += usize::from(b);
                struck_ok += usize::from(c);
                passed += usize::from(a && b && c);
                differing += usize::from(!result.same);
                let inside: Vec<bool> = SINGLED
                    .iter()
                    .map(|k| bands.contains(*k, new.row[*k]))
                    .collect();
                for (k, ok) in inside.iter().enumerate() {
                    singled[k] += usize::from(*ok);
                }
                let flag = |ok: bool| if ok { "  +" } else { "  -" };
                println!(
                    "I/{number:<2}  {seed:3} {:6} {tones:5} {:4.0}% {:5} {:5} {:5} {:3} {:7} {:5} {:5.1}  {:4} ({:2})  {} ({}) {}{}{}  {}  {:?}",
                    original.voices.len(),
                    100.0 * result.given_tones as f64 / tones.max(1) as f64,
                    tally.cost,
                    tally.plan,
                    tally.count.junction,
                    tally.count.own,
                    tally.count.excused,
                    result.setting.attempts,
                    result.setting.seconds,
                    new.hits,
                    old.hits,
                    shown(STRUCK_DISSONANCE, new.row[STRUCK_DISSONANCE]),
                    shown(STRUCK_DISSONANCE, old.row[STRUCK_DISSONANCE]),
                    flag(inside[0]),
                    flag(inside[1]),
                    flag(inside[2]),
                    if result.same { "same " } else { "DIFFERS" },
                    misses
                );
                if detail {
                    for k in SINGLED {
                        println!(
                            "        {:40} new {} Bach {} band {} to {}",
                            NAMES[k],
                            shown(k, new.row[k]),
                            shown(k, old.row[k]),
                            shown(k, bands.low[k]),
                            shown(k, bands.high[k])
                        );
                    }
                }
                if let Some(dir) = out {
                    let stem = format!("neusatz-{}-s{seed}", original.name);
                    let draft = &result.draft;
                    let grey = |v: usize, n: &Note| draft.is_given(v, n);
                    write(
                        dir,
                        &stem,
                        &result.setting.piece,
                        &format!("Neusatz von WTK I/{number}"),
                        &format!(
                            "Profil {}, Seed {seed}, Akkorde {}: grau gegeben, schwarz gesucht",
                            options.profile.name,
                            if generated {
                                "aus dem erzeugten Gerüst"
                            } else {
                                "aus dem Original geschätzt"
                            }
                        ),
                        &grey,
                    )?;
                    let xml = crate::musicxml::musicxml(
                        &result.setting.piece,
                        &format!("Neusatz von WTK I/{number}"),
                        "fux-bach (Themeneinsätze: J. S. Bach)",
                        &[],
                    );
                    std::fs::write(dir.join(format!("{stem}.musicxml")), xml)?;
                }
            }
        }
    }
    let runs = results.len();
    println!(
        "{runs} runs: (a) no prohibition at a searched tone or a junction {clean}, (b) hits at least {mark:.1}: {hits_ok}, (c) struck dissonance inside the band: {struck_ok}; all three: {passed} ({:.0} %)",
        100.0 * passed as f64 / runs.max(1) as f64
    );
    println!(
        "inside the band: repetition {}, outer voices in the same direction {}, complete triad {}",
        singled[0], singled[1], singled[2]
    );
    println!("search step against check: {differing} of {runs} runs differ");
    println!("measures outside their band, by number of runs:");
    for (k, count) in outside.iter().enumerate() {
        if *count > 0 {
            println!("  {count:2}  [{k:2}] {}", NAMES[k]);
        }
    }
    Ok(())
}

/// The measures of the runs of a set against Bach's medians: where the prices have to move.
pub fn rates(data: &Path, numbers: &[usize], seeds: &[u64], options: &Options) -> Result<()> {
    let read = load(data)?;
    let pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
    let rows = corpus_measures(&read);
    let all = stats::Bands::of(&rows);
    let jobs: Vec<(usize, u64)> = numbers
        .iter()
        .flat_map(|n| seeds.iter().map(move |s| (*n, *s)))
        .collect();
    let results: Vec<Option<Rates>> = jobs
        .par_iter()
        .map(|(number, seed)| {
            let original = find(&read, &name(1, *number)).ok()?;
            let labels = algomus::read_for(data, *number).ok()?;
            let options = Options {
                seed: *seed,
                ..options.clone()
            };
            let result = run(&pieces, original, &labels, &options).ok()?;
            let new_harmony = chord::estimate(&result.setting.piece);
            Some((
                stats::measures(&result.setting.piece),
                stats::measures(original),
                check::census(&result.setting.piece, &new_harmony, &options.profile),
                check::census(original, &result.harmony, &options.profile),
            ))
        })
        .collect();
    let found: Vec<&Rates> = results.iter().flatten().collect();
    println!(
        "profile {}: {} of {} runs with a setting; means of the measures, new against the originals of the same fugues, with the band of the 48",
        options.profile.name,
        found.len(),
        jobs.len()
    );
    for k in 0..COUNT {
        let mean = |pick: &dyn Fn(&Rates) -> f64| {
            found.iter().map(|r| pick(r)).sum::<f64>() / found.len().max(1) as f64
        };
        let new = mean(&|r| r.0[k]);
        let old = mean(&|r| r.1[k]);
        println!(
            "  [{k:2}] {:44} new {} Bach {}  band {} to {}{}",
            NAMES[k],
            shown(k, new),
            shown(k, old),
            shown(k, all.low[k]),
            shown(k, all.high[k]),
            if all.contains(k, new) {
                ""
            } else {
                "  outside"
            }
        );
    }
    let mut new = Census::default();
    let mut old = Census::default();
    for r in &found {
        new.add(&r.2);
        old.add(&r.3);
    }
    println!(
        "findings per opportunity, new against the originals (both against estimated chords):"
    );
    for rule in Rule::ALL {
        println!(
            "  {:22} {:6} new {} ({:6})  Bach {} ({:6})",
            rule.id(),
            options.profile.level(*rule).to_string(),
            percent(&new, *rule),
            new.found[*rule as usize],
            percent(&old, *rule),
            old.found[*rule as usize]
        );
    }
    Ok(())
}

/// How many cells of each fugue lie off the grid of thirty-second notes, and how many of those
/// on the grid of triplets.
pub fn cell_grids(data: &Path) -> Result<()> {
    let read = load(data)?;
    println!(
        "fugue      pulse  cells  off the 32nd grid  of those on the triplet grid  shortest tone"
    );
    for r in &read {
        let piece = &r.piece;
        let pulse = piece.meter.unit();
        let lines = cells::cells_of(piece);
        let mut total = 0;
        let mut off = 0;
        let mut triplet = 0;
        for cell in lines.iter().flatten() {
            total += 1;
            let changes: Vec<usize> = (0..pulse as usize)
                .filter(|k| cells::slot(*cell, *k) != cells::Slot::Sustain)
                .filter(|k| {
                    // The start of a run of silence, or an onset.
                    cells::slot(*cell, *k) == cells::Slot::Onset
                        || *k == 0
                        || cells::slot(*cell, *k - 1) != cells::Slot::Rest
                })
                .collect();
            if changes.iter().any(|k| k % 3 != 0) {
                off += 1;
                if changes.iter().all(|k| k % 4 == 0) {
                    triplet += 1;
                }
            }
        }
        let shortest = piece
            .voices
            .iter()
            .flat_map(|v| v.notes.iter().map(|n| n.dur()))
            .min()
            .unwrap_or(0);
        if off > 0 || shortest < 3 {
            println!(
                "{:10} {pulse:5} {total:6} {off:18} {triplet:29} {shortest:14}",
                piece.name
            );
        }
    }
    Ok(())
}

/// The numbers of a set by name (`dev`, `test`) or as a list (`2,16`).
pub fn numbers(set: &str) -> Result<Vec<usize>> {
    match set {
        "dev" | "entwicklung" => Ok(DEVELOPMENT.to_vec()),
        "test" => Ok(TEST.to_vec()),
        list => list
            .split(',')
            .map(|n| {
                n.trim()
                    .parse()
                    .with_context(|| format!("{n:?} is no number of a fugue"))
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The corpus, if it is there (the files are not part of every checkout).
    fn data() -> Option<std::path::PathBuf> {
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../references/datasets/bach-fugen");
        dir.join("wtc").is_dir().then_some(dir)
    }

    /// Z2 of the protocol: the 48 fugues break the prohibitions of the profile `bach` exactly as
    /// often as the catalogue (`docs/regeln-bach.md`) says, and a search in which every tone is
    /// given adds up to the same as the check.
    #[test]
    fn bach_breaks_the_counted_prohibitions() {
        let Some(data) = data() else {
            return;
        };
        let profile = Profile::bach();
        let read = load(&data).unwrap();
        assert_eq!(read.len(), 48);
        let mut sums = vec![0usize; Rule::COUNT];
        for r in &read {
            let harmony = chord::estimate(&r.piece);
            let (task, tones) = check::task_of(&r.piece, &harmony, &profile);
            let report = check::report(&task, &tones);
            for (rule, count) in report.forbidden_by_rule() {
                sums[rule as usize] += count;
            }
            let Outcome::Found(found) = Machine::new(&task).search() else {
                panic!("{}: given tones never end a search", r.piece.name);
            };
            assert_eq!(found.tally, report.tally, "{}", r.piece.name);
        }
        let counted: Vec<(&str, usize)> = Rule::ALL
            .iter()
            .filter(|r| sums[**r as usize] > 0)
            .map(|r| (r.id(), sums[*r as usize]))
            .collect();
        assert_eq!(
            counted,
            [
                ("MEL-LEAP-BIG", 87),
                ("FIG-UNRESOLVED", 333),
                ("FIG-TOO-LONG", 4),
                ("FIG-BEFORE-REST", 14),
                ("MOT-PARALLEL", 34),
                ("HAR-CLASH", 68),
            ]
        );
    }

    /// Z3 of the protocol on a generated setting: what the search paid step by step is what the
    /// check of the finished piece adds up to.
    #[test]
    fn a_setting_checks_as_it_was_searched() {
        let Some(data) = data() else {
            return;
        };
        let read = load(&data).unwrap();
        let pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
        let original = find(&read, &name(1, 2)).unwrap();
        let labels = algomus::read_for(&data, 2).unwrap();
        let options = Options {
            beam: 40,
            attempts: 4,
            ..Options::default()
        };
        let result = run(&pieces, original, &labels, &options).unwrap();
        assert!(result.same);
        assert_eq!(result.setting.tally.count.junction, 0);
        assert!(!result.setting.tally.stop);
        // Bach's entries are in the piece as given.
        let given = result
            .setting
            .piece
            .voices
            .iter()
            .enumerate()
            .flat_map(|(v, voice)| voice.notes.iter().map(move |n| (v, n)))
            .filter(|(v, n)| result.draft.is_given(*v, n))
            .count();
        assert_eq!(given, result.given_tones);
    }
}
