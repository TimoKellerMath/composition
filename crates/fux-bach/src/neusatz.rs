//! T6: a fugue of WTC I set anew. The entries of the subject stay as Bach wrote them, every
//! other tone is searched over the chords estimated from the original.

use std::path::Path;

use anyhow::{Context, Result};

use crate::algomus::{self, Label};
use crate::chord;
use crate::export;
use crate::key::Harmony;
use crate::layout::{Layout, Options, Setting, VoiceLayout, set};
use crate::rhythm;
use crate::score::{Note, Piece};
use crate::stats::{self, Bands, COUNT, NAMES, STRUCK_DISSONANCE, UNEXPLAINED};
use crate::study::{corpus_measures, find, load, name, shown};
use crate::time::{SIXTEENTH, Tick};

/// What stays of the original.
#[derive(Clone, Copy, Debug, Default)]
pub struct Given {
    /// The countersubjects stay as well as the subject.
    pub countersubjects: bool,
    /// Where a voice of the original is silent for a whole pulse, the new one is silent too.
    pub rests: bool,
}

/// The layout of a re-setting: fixed entries, ranges, silences.
pub fn layout(original: &Piece, labels: &[Label], harmony: &Harmony, given: Given) -> Layout {
    let pulse = original.meter.pulse();
    let voices = original
        .voices
        .iter()
        .enumerate()
        .map(|(v, voice)| {
            let mut fixed: Vec<Note> = Vec::new();
            let mut spans: Vec<(Tick, Tick)> = Vec::new();
            for label in labels {
                let wanted =
                    label.is_subject() || (given.countersubjects && label.is_countersubject());
                if label.voice != Some(v) || !wanted {
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
                let new: Vec<Note> = notes
                    .iter()
                    .filter(|n| !fixed.contains(n))
                    .copied()
                    .collect();
                fixed.extend(new);
                spans.push(span);
            }
            fixed.sort_by_key(|n| n.on);
            let first = voice.notes.first().map_or(original.end, |n| n.on);
            let mut silent = vec![(Tick::MIN / 2, first)];
            if given.rests {
                for t in original.grid(pulse) {
                    let sounding = voice.at(t).is_some() || !voice.range(t, t + pulse).is_empty();
                    if !sounding {
                        silent.push((t, t + pulse));
                    }
                }
            }
            VoiceLayout {
                low: voice
                    .notes
                    .iter()
                    .map(|n| n.pitch)
                    .min()
                    .unwrap_or(crate::pitch::Pitch::new(0, 0, 4)),
                high: voice
                    .notes
                    .iter()
                    .map(|n| n.pitch)
                    .max()
                    .unwrap_or(crate::pitch::Pitch::new(0, 0, 4)),
                fixed,
                spans,
                silent,
            }
        })
        .collect();
    let last_chord = harmony.spans.last().map_or(original.end, |s| s.on);
    Layout {
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
        junctions: false,
    }
}

/// One re-setting with what is needed to judge it.
pub struct Run {
    pub setting: Setting,
    pub layout: Layout,
    pub harmony: Harmony,
    pub learned_from: Vec<String>,
    pub fixed_tones: usize,
}

pub fn run(
    pieces: &[Piece],
    original: &Piece,
    labels: &[Label],
    given: Given,
    options: &Options,
) -> Result<Run> {
    let harmony = chord::estimate(original);
    let layout = layout(original, labels, &harmony, given);
    let subject: Vec<Note> = layout.voices.iter().flat_map(|v| v.fixed.clone()).collect();
    let profile = rhythm::value_profile(subject.iter());
    let (table, intervals, learned_from) = rhythm::table_for(
        pieces.iter(),
        original.meter.pulse(),
        profile,
        &original.name,
        8,
    );
    let fixed_tones = subject.len();
    let setting = set(&layout, &harmony, &table, Some(&intervals), options).map_err(|dead| {
        let bars: Vec<String> = dead
            .iter()
            .map(|(s, why)| {
                format!(
                    "{} ({why})",
                    original.bar_of(layout.start() + *s as Tick * SIXTEENTH).0
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
    Ok(Run {
        setting,
        layout,
        harmony,
        learned_from,
        fixed_tones,
    })
}

/// Writes a piece as LilyPond and MIDI; tones for which `grey` holds are printed grey.
pub fn write(
    dir: &Path,
    stem: &str,
    piece: &Piece,
    title: &str,
    subtitle: &str,
    grey: &dyn Fn(usize, &Note) -> bool,
) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let ly = dir.join(format!("{stem}.ly"));
    std::fs::write(&ly, export::lilypond(piece, title, subtitle, &[], grey))
        .with_context(|| ly.display().to_string())?;
    let mid = dir.join(format!("{stem}.mid"));
    std::fs::write(&mid, export::midi(piece, export::tempo(piece)))
        .with_context(|| mid.display().to_string())?;
    Ok(())
}

/// The judgement of a setting against the bands of the other fugues.
pub struct Verdict {
    pub row: [f64; COUNT],
    pub hits: usize,
    pub struck_inside: bool,
    pub unexplained_inside: bool,
}

pub fn verdict(piece: &Piece, bands: &Bands) -> Verdict {
    let row = stats::measures(piece);
    Verdict {
        hits: bands.hits(&row),
        struck_inside: bands.contains(STRUCK_DISSONANCE, row[STRUCK_DISSONANCE]),
        unexplained_inside: row[UNEXPLAINED] <= bands.high[UNEXPLAINED] + 1e-12,
        row,
    }
}

/// The bands of all fugues but the named one.
pub fn bands_without(pieces: &[Piece], rows: &[[f64; COUNT]], except: &str) -> Bands {
    let others: Vec<[f64; COUNT]> = pieces
        .iter()
        .zip(rows)
        .filter(|(p, _)| p.name != except)
        .map(|(_, r)| *r)
        .collect();
    Bands::of(&others)
}

/// Sets one fugue anew, prints the measures beside the original's and the bands, and writes the
/// score.
#[allow(clippy::too_many_arguments)]
pub fn one(
    data: &Path,
    number: usize,
    given: Given,
    options: &Options,
    out: Option<&Path>,
    detail: bool,
) -> Result<()> {
    let read = load(data)?;
    let pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
    let rows = corpus_measures(&read);
    let original = find(&read, &name(1, number))?;
    let labels = algomus::read_for(data, number)?;
    let result = run(&pieces, original, &labels, given, options)?;
    let bands = bands_without(&pieces, &rows, &original.name);
    let (_, mark) = stats::yardstick(&rows);
    let new = verdict(&result.setting.piece, &bands);
    let old = verdict(original, &bands);
    let tones = result.setting.piece.note_count();
    println!(
        "{} seed {}: {} tones, {} of them fixed ({:.0} %); cost {}, forced {}, in the fixed lines {}, excused {}, {} attempt(s), {:.1} s; rhythm from {:?}",
        original.name,
        options.seed,
        tones,
        result.fixed_tones,
        100.0 * result.fixed_tones as f64 / tones.max(1) as f64,
        result.setting.cost,
        result.setting.forced,
        result.setting.own,
        result.setting.excused,
        result.setting.attempts,
        result.setting.seconds,
        result.learned_from
    );
    println!(
        "hits {} of {COUNT} (Bach's own {}; yardstick {mark:.1}); struck dissonance inside the band: {}; unexplained at most p90: {}",
        new.hits, old.hits, new.struck_inside, new.unexplained_inside
    );
    if detail {
        println!(
            "{:44} {:>8} {:>8} {:>8} {:>8}",
            "measure", "new", "Bach", "p10", "p90"
        );
        for k in 0..COUNT {
            println!(
                "{:44} {} {} {} {} {}",
                NAMES[k],
                shown(k, new.row[k]),
                shown(k, old.row[k]),
                shown(k, bands.low[k]),
                shown(k, bands.high[k]),
                if bands.contains(k, new.row[k]) {
                    ""
                } else {
                    "  outside"
                }
            );
        }
    }
    if detail {
        println!("parallels:");
        crate::study::list_parallels(&result.setting.piece);
        let fixed_only: Vec<Vec<Note>> = result
            .layout
            .voices
            .iter()
            .map(|v| v.fixed.clone())
            .collect();
        println!(
            "fixed tones per voice: {:?}",
            fixed_only.iter().map(Vec::len).collect::<Vec<_>>()
        );
        // The struck dissonant pairs on beats by figure, new and original.
        for (label, piece) in [("new", &result.setting.piece), ("Bach", original)] {
            let harmony = chord::estimate(piece);
            let census = crate::figures::pair_census(crate::figures::Reading {
                piece,
                harmony: &harmony,
            });
            let (total, rest) = census.struck();
            println!(
                "{label}: {} beats with two voices, {total} struck dissonant pairs, {rest} unexplained",
                census.beats_sounding
            );
            for figure in crate::figures::Figure::ALL {
                let counts: Vec<usize> =
                    [crate::figures::Layer::Free, crate::figures::Layer::Repeated]
                        .into_iter()
                        .flat_map(|l| {
                            [crate::figures::Pair::Bass, crate::figures::Pair::Upper]
                                .map(|p| census.of(l, Some(p), Some(figure)))
                        })
                        .collect();
                if counts.iter().sum::<usize>() > 0 {
                    println!(
                        "   {:22} bass {:3} upper {:3}  repeated: bass {:3} upper {:3}",
                        figure.german(),
                        counts[0],
                        counts[1],
                        counts[2],
                        counts[3]
                    );
                }
            }
        }
    }
    if let Some(dir) = out {
        let stem = format!(
            "neusatz-{}-{}{}-s{}",
            original.name,
            if given.countersubjects { "cs" } else { "s" },
            if given.rests { "-pausen" } else { "" },
            options.seed
        );
        let layout = &result.layout;
        let grey = |v: usize, n: &Note| layout.voices[v].fixed.contains(n);
        write(
            dir,
            &stem,
            &result.setting.piece,
            &format!("Neusatz von WTK I/{number}"),
            &format!(
                "Machbarkeitsstudie, Seed {}: grau Bachs Einsätze, schwarz gesucht",
                options.seed
            ),
            &grey,
        )?;
        println!("written: {}/{stem}.ly and .mid", dir.display());
    }
    Ok(())
}

/// A run with the verdicts on the new setting and on the original.
type Judged = (Run, Verdict, Verdict);

/// The fugues of T6 on which rules and costs are set, and those of the test.
pub const DEVELOPMENT: [usize; 2] = [2, 16];
pub const TEST: [usize; 6] = [6, 7, 13, 17, 21, 23];

/// T6 on a set of fugues with several seeds: one line per run and the sums.
pub fn many(
    data: &Path,
    numbers: &[usize],
    seeds: &[u64],
    given: Given,
    options: &Options,
    out: Option<&Path>,
) -> Result<()> {
    use rayon::prelude::*;
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
                let result = run(&pieces, original, &labels, given, &options)?;
                let bands = bands_without(&pieces, &rows, &original.name);
                let new = verdict(&result.setting.piece, &bands);
                let old = verdict(original, &bands);
                Ok((result, new, old))
            })();
            (*number, *seed, outcome)
        })
        .collect();
    let mut found = 0;
    let mut hits_ok = 0;
    let mut struck_ok = 0;
    let mut rest_ok = 0;
    let mut outside = [0usize; COUNT];
    println!(
        "fugue seed  tones fixed  cost forced own excused tries   sec  hits (Bach)  struck (Bach)  unexplained (Bach)  outside"
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
                let clean = result.setting.forced == 0;
                found += clean as usize;
                hits_ok += (new.hits as f64 >= mark) as usize;
                struck_ok += new.struck_inside as usize;
                rest_ok += new.unexplained_inside as usize;
                println!(
                    "I/{number:<2}  {seed:3}  {tones:5} {:4.0}% {:5} {:6} {:3} {:7} {:5} {:5.1}  {:4} ({:2})  {} ({})  {} ({})  {:?}",
                    100.0 * result.fixed_tones as f64 / tones.max(1) as f64,
                    result.setting.cost,
                    result.setting.forced,
                    result.setting.own,
                    result.setting.excused,
                    result.setting.attempts,
                    result.setting.seconds,
                    new.hits,
                    old.hits,
                    shown(STRUCK_DISSONANCE, new.row[STRUCK_DISSONANCE]),
                    shown(STRUCK_DISSONANCE, old.row[STRUCK_DISSONANCE]),
                    shown(UNEXPLAINED, new.row[UNEXPLAINED]),
                    shown(UNEXPLAINED, old.row[UNEXPLAINED]),
                    misses
                );
                if let Some(dir) = out {
                    let stem = format!(
                        "neusatz-{}-{}{}-s{seed}",
                        original.name,
                        if given.countersubjects { "cs" } else { "s" },
                        if given.rests { "-pausen" } else { "" },
                    );
                    let layout = &result.layout;
                    let grey = |v: usize, n: &Note| layout.voices[v].fixed.contains(n);
                    write(
                        dir,
                        &stem,
                        &result.setting.piece,
                        &format!("Neusatz von WTK I/{number}"),
                        &format!(
                            "Machbarkeitsstudie, Seed {seed}: grau Bachs Einsätze, schwarz gesucht"
                        ),
                        &grey,
                    )?;
                }
            }
        }
    }
    // How near each setting is to its original, against the other fugues: the mean distance
    // over the measures in units of their band width.
    let all = Bands::of(&rows);
    let distance = |a: &[f64; COUNT], b: &[f64; COUNT]| -> f64 {
        (0..COUNT)
            .map(|k| (a[k] - b[k]).abs() / (all.high[k] - all.low[k]).max(1e-9))
            .sum::<f64>()
            / COUNT as f64
    };
    println!(
        "distance to the original in band widths, and how many of the other 47 fugues are nearer to it:"
    );
    for (number, seed, outcome) in &results {
        if let Ok((_, new, old)) = outcome {
            let own = distance(&new.row, &old.row);
            let nearer = pieces
                .iter()
                .zip(&rows)
                .filter(|(p, r)| p.name != name(1, *number) && distance(r, &old.row) < own)
                .count();
            println!("  I/{number:<2} seed {seed}: {own:.2}, {nearer} nearer");
        }
    }
    let runs = results.len();
    println!(
        "{runs} runs: (a) a setting in which the free voices lead into every entry {found}, (b) hits at least {mark:.1}: {hits_ok}, (c) struck dissonance inside the band: {struck_ok}, (d) unexplained at most p90: {rest_ok}"
    );
    println!("measures outside their band, by number of runs:");
    for (k, count) in outside.iter().enumerate() {
        if *count > 0 {
            println!("  {count:2}  [{k:2}] {}", NAMES[k]);
        }
    }
    Ok(())
}
