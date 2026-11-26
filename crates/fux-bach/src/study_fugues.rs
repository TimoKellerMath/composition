//! T7: fugues on the ten subjects of the protocol, judged against the bands.

use std::path::Path;

use anyhow::{Context, Result};

use crate::fugue::Fugue;
use crate::harmonize::Transitions;
use crate::layout::Options;
use crate::neusatz::{bands_without, verdict, write};
use crate::rhythm;
use crate::roman;
use crate::score::{Note, Piece};
use crate::stats::{self, COUNT, NAMES, STRUCK_DISSONANCE, UNEXPLAINED};
use crate::study::{corpus_measures, load, shown};
use crate::themes::{self, Subject};

/// The successions of chords in Tymoczko's seven analyses.
pub fn transitions(data: &Path) -> Result<Transitions> {
    let mut out = Transitions::default();
    let dir = data.join("wir");
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .with_context(|| dir.display().to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    paths.sort();
    for path in paths {
        let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
        out.learn(&roman::parse(&text)?);
    }
    Ok(out)
}

/// A fugue with the names of the fugues its tables were counted on.
type Composed = (Fugue, Vec<String>);

/// A fugue on one subject with the tables of the fugues nearest to it.
pub fn one(subject: &Subject, options: &Options) -> Result<Composed> {
    crate::fugue::compose_embedded(subject, options)
}

/// T7: every subject of the list (or the one named) with the given seeds.
pub fn t7(
    data: &Path,
    only: Option<&str>,
    seeds: &[u64],
    options: &Options,
    out: Option<&Path>,
    detail: bool,
) -> Result<()> {
    use rayon::prelude::*;
    let read = load(data)?;
    let pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
    let rows = corpus_measures(&read);
    let (_, mark) = stats::yardstick(&rows);
    let subjects: Vec<Subject> = themes::LIST
        .iter()
        .filter(|e| only.is_none_or(|name| e.name == name))
        .map(|e| themes::subject(e, &pieces, data))
        .collect::<Result<_>>()?;
    let jobs: Vec<(&Subject, u64)> = subjects
        .iter()
        .flat_map(|s| seeds.iter().map(move |seed| (s, *seed)))
        .collect();
    let results: Vec<(&Subject, u64, Result<Composed>)> = jobs
        .par_iter()
        .map(|(subject, seed)| {
            let options = Options {
                seed: *seed,
                ..options.clone()
            };
            (*subject, *seed, one(subject, &options))
        })
        .collect();
    println!(
        "theme     role  seed  answer  bars tones fixed  forced own excused tries   sec  hits  struck  unexplained  outside"
    );
    let mut found = 0;
    let mut tested = 0;
    let mut passed: std::collections::BTreeMap<&str, [usize; 4]> = Default::default();
    for (subject, seed, outcome) in &results {
        let role = if subject.development { "dev " } else { "test" };
        let entry = passed.entry(subject.name.as_str()).or_default();
        entry[3] += 1;
        match outcome {
            Err(error) => println!("{:9} {role} {seed:4}  no fugue: {error}", subject.name),
            Ok((fugue, learned_from)) => {
                let piece = &fugue.setting.piece;
                let bands = bands_without(&pieces, &rows, subject.from.as_deref().unwrap_or(""));
                let v = verdict(piece, &bands);
                let tones = piece.note_count();
                let fixed: usize = fugue.layout.voices.iter().map(|l| l.fixed.len()).sum();
                let misses: Vec<usize> = (0..COUNT)
                    .filter(|k| !bands.contains(*k, v.row[*k]))
                    .collect();
                let clean = fugue.setting.forced == 0;
                entry[0] += clean as usize;
                entry[1] += (v.hits as f64 >= mark) as usize;
                entry[2] += v.struck_inside as usize;
                found += 1;
                tested += !subject.development as usize;
                println!(
                    "{:9} {role} {seed:4}  {:6} {:5} {tones:5} {:4.0}% {:6} {:3} {:7} {:2}+{:<2} {:5.1}  {:4}  {}  {}  {:?}",
                    subject.name,
                    format!("{:?}", fugue.answer.kind).to_lowercase(),
                    piece.end / piece.meter.bar(),
                    100.0 * fixed as f64 / tones.max(1) as f64,
                    fugue.setting.forced,
                    fugue.setting.own,
                    fugue.setting.excused,
                    fugue.first_attempts,
                    fugue.setting.attempts,
                    fugue.seconds,
                    v.hits,
                    shown(STRUCK_DISSONANCE, v.row[STRUCK_DISSONANCE]),
                    shown(UNEXPLAINED, v.row[UNEXPLAINED]),
                    misses
                );
                if detail {
                    println!("   parallels:");
                    crate::study::list_parallels(piece);
                    println!("   rhythm and intervals from {learned_from:?}");
                    println!(
                        "   sections: {}",
                        fugue
                            .sections
                            .iter()
                            .map(|s| format!("{} T. {}", s.name, 1 + s.from / piece.meter.bar()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    let chords: Vec<String> = fugue
                        .harmony
                        .spans
                        .iter()
                        .map(|s| format!("{}:{}", 1 + s.on / piece.meter.bar(), s.chord))
                        .collect();
                    println!("   chords: {}", chords.join(" "));
                    for k in 0..COUNT {
                        if !bands.contains(k, v.row[k]) {
                            println!(
                                "   outside: {} {} (band {} to {})",
                                NAMES[k],
                                shown(k, v.row[k]),
                                shown(k, bands.low[k]),
                                shown(k, bands.high[k])
                            );
                        }
                    }
                }
                if let Some(dir) = out {
                    let layout = &fugue.layout;
                    let grey = |voice: usize, n: &Note| layout.voices[voice].fixed.contains(n);
                    write(
                        dir,
                        &format!("fuge-{}-s{seed}", subject.name),
                        piece,
                        &format!("Fuge über {}", subject.title),
                        &format!(
                            "Machbarkeitsstudie, Seed {seed}: grau Thema, Antwort und Kontrasubjekt, schwarz gesucht"
                        ),
                        &grey,
                    )?;
                }
            }
        }
    }
    println!(
        "{found} of {} runs found a fugue ({tested} of them on test themes)",
        results.len()
    );
    println!(
        "per theme: runs without a fault at an entry / hits at least {mark:.1} / struck dissonance inside the band / runs"
    );
    for (name, counts) in &passed {
        println!(
            "  {name:9} {} / {} / {} / {}",
            counts[0], counts[1], counts[2], counts[3]
        );
    }
    Ok(())
}

/// T8: the time of a four-voice search without fixed tones over 30, 60 and 100 bars of 4/4 on a
/// cadence that repeats every two bars.
pub fn t8(data: &Path, options: &Options) -> Result<()> {
    use crate::harmonize::Rel;
    use crate::key::{Harmony, Key, Kind};
    use crate::layout::{Layout, VoiceLayout, set};
    use crate::pitch::Pitch;
    use crate::time::Meter;
    let read = load(data)?;
    let pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
    let (table, intervals, _) = rhythm::table_for(pieces.iter(), 24, [0.5, 0.35, 0.15], "", 8);
    let key = Key::parse("C").context("key")?;
    let meter = Meter::COMMON;
    let cycle = [
        Rel::new(0, 0, Kind::Maj),
        Rel::new(5, 0, Kind::Min),
        Rel::new(1, 0, Kind::Min7),
        Rel::new(4, 0, Kind::Dom7),
    ];
    let ranges = [
        (Pitch::new(0, 0, 4), Pitch::new(0, 0, 6)),
        (Pitch::new(4, 0, 3), Pitch::new(2, 0, 5)),
        (Pitch::new(0, 0, 3), Pitch::new(4, 0, 4)),
        (Pitch::new(0, 0, 2), Pitch::new(2, 0, 4)),
    ];
    println!("bars  beam  tones  cost  tries  seconds  seconds per bar");
    for bars in [30, 60, 100] {
        let end = bars * meter.bar();
        let mut harmony = Harmony::default();
        for half in 0..2 * bars {
            let rel = if half >= 2 * bars - 2 {
                cycle[0]
            } else {
                cycle[(half % 4) as usize]
            };
            harmony.push(half * 48, rel.in_key(key), key);
        }
        let layout = Layout {
            name: format!("{bars} bars"),
            key,
            meter,
            origin: 0,
            end,
            voices: ranges
                .iter()
                .map(|(low, high)| VoiceLayout {
                    low: *low,
                    high: *high,
                    fixed: Vec::new(),
                    spans: Vec::new(),
                    silent: Vec::new(),
                })
                .collect(),
            final_hold: Some(end - meter.bar()),
            invertible: Vec::new(),
            root_at_hold: true,
            longest: Some(meter.bar()),
            junctions: true,
        };
        match set(&layout, &harmony, &table, Some(&intervals), options) {
            Ok(setting) => println!(
                "{bars:4} {:5} {:6} {:5} {:6} {:8.1} {:8.2}",
                options.beam,
                setting.piece.note_count(),
                setting.cost,
                setting.attempts,
                setting.seconds,
                setting.seconds / bars as f64
            ),
            Err(dead) => println!("{bars:4}  no setting: {dead:?}"),
        }
    }
    Ok(())
}

/// Prints the subjects of the list as typed text: name, key, meter, period, tones.
pub fn print_themes(data: &Path) -> Result<()> {
    let read = load(data)?;
    let pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
    for entry in &themes::LIST {
        let subject = themes::subject(entry, &pieces, data)?;
        let typed = themes::BUILTIN
            .iter()
            .find(|b| b.name == entry.name)
            .map(|b| b.subject())
            .transpose()?;
        let same = typed.is_some_and(|t| {
            t.notes == subject.notes
                && t.key == subject.key
                && t.meter == subject.meter
                && t.period == subject.period
        });
        println!(
            "{} {} | {} | {} | {} | {} | {}",
            if same {
                "same as typed"
            } else {
                "DIFFERS FROM TYPED"
            },
            subject.name,
            subject.title,
            subject.key,
            subject.meter,
            subject.period,
            themes::text(&subject.notes)
        );
    }
    Ok(())
}
