//! The runs of phase B3 (protocol in `docs/design/bach-fugen/b3/protokoll.md`).

use std::path::Path;

use anyhow::{Context, Result};

use crate::kern;
use crate::score::{Note, Piece};
use crate::study::load;
use crate::subject;
use crate::themes;

/// A subject as the first voice of a piece states it, with the distance to the answer: for
/// typing new subjects into the list that comes with the program.
pub fn extract(data: &Path, names: &[String]) -> Result<()> {
    let read = load(data)?;
    let mut pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
    for dir in ["opfer", "kdf"] {
        let (more, _) = crate::b1::readable(&data.join(dir))?;
        pieces.extend(more.into_iter().map(|r| r.piece));
    }
    for name in names {
        let piece = pieces
            .iter()
            .find(|p| &p.name == name)
            .with_context(|| format!("no piece {name}"))?;
        let exposition =
            subject::from_entries(piece).with_context(|| format!("no subject in {name}"))?;
        let first = exposition.subject[0].on;
        let answer = exposition.answer[0].on;
        let bar = first - piece.pos(first);
        let notes: Vec<Note> = exposition
            .subject
            .iter()
            .map(|n| Note {
                on: n.on - bar,
                off: n.off - bar,
                pitch: n.pitch,
            })
            .collect();
        println!(
            "{name} | {} | {} | period {} | voices {} | {} tones | {}",
            piece.key,
            piece.meter,
            answer - first,
            piece.voices.len(),
            notes.len(),
            themes::text(&notes)
        );
    }
    let _ = kern::read;
    Ok(())
}

/// The subject of a piece of the corpus as `extract` prints it: tones from the barline before
/// the first one, and the distance to the answer.
pub fn first_subject(piece: &Piece) -> Option<(Vec<Note>, crate::time::Tick)> {
    let exposition = subject::from_entries(piece)?;
    let first = exposition.subject[0].on;
    let answer = exposition.answer[0].on;
    let bar = first - piece.pos(first);
    let notes = exposition
        .subject
        .iter()
        .map(|n| Note {
            on: n.on - bar,
            off: n.off - bar,
            pitch: n.pitch,
        })
        .collect();
    Some((notes, answer - first))
}

/// The plans of Bach's fugues of WTC I as numbers: for every fugue with labels its voices and,
/// for each entry of the subject after the exposition, the voice from the top, the degree and
/// mode of its key in the home key, and the bars since the entry before it ended.
pub fn templates(data: &Path, out: &Path) -> Result<()> {
    let read = load(data)?;
    let mut fugues = Vec::new();
    for number in 1..=24usize {
        let Ok(labels) = crate::algomus::read_for(data, number) else {
            continue;
        };
        let piece = crate::study::find(&read, &crate::study::name(1, number))?;
        let bar = piece.meter.bar();
        let mut entries: Vec<&crate::algomus::Label> = labels
            .iter()
            .filter(|l| l.is_subject() && l.voice.is_some())
            .collect();
        entries.sort_by_key(|l| l.start);
        let voices = piece.voices.len();
        let Some(first) = entries.first() else {
            continue;
        };
        let length = first.end - first.start;
        let mut rows = Vec::new();
        for pair in entries.windows(2).skip(voices.saturating_sub(1)) {
            let (before, entry) = (pair[0], pair[1]);
            let key = crate::scaffold::key_of(entry.notes(piece), piece.key);
            let gap = (entry.start - (before.start + length)).div_euclid(bar);
            rows.push(serde_json::json!([
                entry.voice.unwrap_or(0),
                piece.key.degree_of(key.tonic),
                key.minor,
                gap.clamp(0, 8)
            ]));
        }
        fugues.push(serde_json::json!({
            "name": format!("wtk1-{number:02}"),
            "voices": voices,
            "minor": piece.key.minor,
            "entries": rows,
        }));
    }
    let count = fugues.len();
    let value = serde_json::json!({
        "note": "Counted by `bach-probe b3-templates` from the Algomus labels of WTC I (ODbL 1.0): the order of Bach's entries after the exposition, numbers only.",
        "fugues": fugues,
    });
    std::fs::write(out, serde_json::to_string(&value)? + "\n")
        .with_context(|| out.display().to_string())?;
    println!("{}: the plans of {count} fugues", out.display());
    Ok(())
}

/// The subjects of the development (the ten of the study) and of the test (ten more).
pub const DEVELOPMENT: [&str; 10] = [
    "wtk1-02", "wtk1-16", "bwv578", "bwv1080", "wtk2-02", "wtk1-21", "wtk1-01", "wtk2-20",
    "wtk1-07", "haendel",
];
pub const TEST: [&str; 10] = [
    "wtk2-01", "wtk2-05", "wtk2-12", "wtk2-16", "wtk2-18", "wtk2-11", "wtk2-21", "bwv1079",
    "wtk1-15", "wtk2-09",
];

/// What a fugue is judged by (K1 to K5 of the protocol).
pub struct Verdict {
    pub bars: i32,
    /// K2: prohibitions at junctions; given material against the scaffold; excused.
    pub junction: u32,
    pub own: u32,
    pub excused: u32,
    /// K3: every voice in the exposition, distinct keys of the middle entries other than the
    /// tonic, episodes with at least two statements of the motif (of all episodes), the ending.
    pub exposition: bool,
    pub keys: usize,
    pub episodes: (usize, usize),
    pub ending: &'static str,
    /// K4.
    pub hits: usize,
    pub struck_inside: bool,
    /// K5: planned entries, those of them the finder found, and further hits.
    pub entries: (usize, usize, usize),
    /// The measures outside their bands.
    pub outside: Vec<usize>,
    /// Those of them that lie above their band.
    pub above: Vec<usize>,
}

impl Verdict {
    pub fn k1(&self) -> bool {
        (30..=80).contains(&self.bars)
    }

    /// No prohibition at a searched tone or at a seam: neither into a given tone (`junction`)
    /// nor out of one (`excused`, a foreign given tone the searched tone after it leaves
    /// unresolved).
    pub fn k2(&self) -> bool {
        self.junction == 0 && self.excused == 0
    }

    pub fn k3(&self) -> bool {
        self.exposition
            && self.keys >= 2
            && self.episodes.0 == self.episodes.1
            && !self.ending.is_empty()
    }

    pub fn k4(&self, mark: f64) -> bool {
        self.hits as f64 >= mark
    }

    pub fn k5(&self) -> bool {
        self.entries.0 == self.entries.1
    }
}

pub fn judge(
    subject: &themes::Subject,
    fugue: &crate::compose::Fugue,
    bands: &crate::stats::Bands,
) -> Verdict {
    use crate::compose::What;
    let design = &fugue.design;
    let voices = fugue.piece.voices.len();
    let mut first: Vec<usize> = design
        .entries
        .iter()
        .filter(|e| e.group == 0)
        .map(|e| e.voice)
        .collect();
    first.sort_unstable();
    first.dedup();
    let last = design.entries.iter().map(|e| e.group).max().unwrap_or(0);
    let keys: std::collections::BTreeSet<String> = design
        .entries
        .iter()
        .filter(|e| e.group > 0 && e.group < last && e.key != subject.key)
        .map(|e| e.key.to_string())
        .collect();
    let with_motif = design
        .episodes
        .iter()
        .filter(|(from, until)| {
            fugue
                .given
                .iter()
                .filter(|g| g.what == What::Motif)
                .filter(|g| {
                    g.notes
                        .first()
                        .is_some_and(|n| n.on >= *from && n.on < *until)
                })
                .count()
                >= 2
        })
        .count();
    let dominant_pedal = design.pedals.iter().any(|p| p.degree == 4)
        && fugue.given.iter().any(|g| g.what == What::Pedal);
    let ending = if design.stretto.is_some() {
        "stretto"
    } else if dominant_pedal {
        "pedal"
    } else {
        ""
    };
    let hits = crate::finder::find(&fugue.piece, &subject.notes);
    let found = design
        .entries
        .iter()
        .filter(|e| hits.iter().any(|h| h.voice == e.voice && h.at == e.at))
        .count();
    let row = crate::stats::measures(&fugue.piece);
    Verdict {
        bars: design.bars(subject.meter),
        junction: fugue.tally.count.junction,
        own: fugue.tally.count.own,
        excused: fugue.tally.count.excused,
        exposition: first.len() == voices,
        keys: keys.len(),
        episodes: (with_motif, design.episodes.len()),
        ending,
        hits: bands.hits(&row),
        struck_inside: bands.contains(
            crate::stats::STRUCK_DISSONANCE,
            row[crate::stats::STRUCK_DISSONANCE],
        ),
        entries: (
            design.entries.len(),
            found,
            hits.len().saturating_sub(found),
        ),
        outside: (0..crate::stats::COUNT)
            .filter(|k| !bands.contains(*k, row[*k]))
            .collect(),
        above: (0..crate::stats::COUNT)
            .filter(|k| !bands.contains(*k, row[*k]) && row[*k] > bands.high[*k])
            .collect(),
    }
}

/// The subjects of a set by name (`dev`, `test`, `all`) or as a list of names.
pub fn subjects(set: &str) -> Result<Vec<themes::Subject>> {
    let names: Vec<String> = match set {
        "dev" | "entwicklung" => DEVELOPMENT.iter().map(|s| s.to_string()).collect(),
        "test" => TEST.iter().map(|s| s.to_string()).collect(),
        "all" | "alle" => DEVELOPMENT
            .iter()
            .chain(&TEST)
            .map(|s| s.to_string())
            .collect(),
        list => list.split(',').map(|s| s.trim().to_string()).collect(),
    };
    names
        .iter()
        .map(|name| {
            themes::BUILTIN
                .iter()
                .find(|b| b.name == name)
                .with_context(|| format!("no theme {name:?}"))?
                .subject()
        })
        .collect()
}

/// The plan of a fugue as text: sections with bar and key, and the given material.
pub fn describe(subject: &themes::Subject, fugue: &crate::compose::Fugue) -> String {
    use std::fmt::Write;
    let bar = subject.meter.bar();
    let mut out = String::new();
    for section in &fugue.design.sections {
        let entry = fugue
            .design
            .entries
            .iter()
            .find(|e| e.at == section.from && section.name != "Zwischenspiel");
        let _ = write!(out, "  T. {:3} {}", 1 + section.from / bar, section.name);
        if let Some(entry) = entry {
            let _ = write!(out, " in Stimme {} ({})", entry.voice + 1, entry.key);
        }
        out.push('\n');
    }
    let mut counts = std::collections::BTreeMap::new();
    for given in &fugue.given {
        *counts.entry(format!("{:?}", given.what)).or_insert(0usize) += 1;
    }
    let _ = writeln!(out, "  gegeben: {counts:?}");
    out
}

/// A fugue of a run: index of its subject, voices, seed, and the fugue with its verdict.
pub type Job = (usize, usize, u64, Result<(crate::compose::Fugue, Verdict)>);

/// The fugues on `subjects` for several numbers of voices and seeds, each with its verdict, and
/// the yardstick of K4.
pub fn compose_all(
    data: &Path,
    subjects: &[themes::Subject],
    voices: &[usize],
    seeds: &[u64],
    options: &crate::compose::Options,
) -> Result<(Vec<Job>, f64)> {
    use rayon::prelude::*;
    let read = load(data)?;
    let pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
    let rows = crate::study::corpus_measures(&read);
    let (_, mark) = crate::stats::yardstick(&rows);
    let jobs: Vec<(usize, usize, u64)> = (0..subjects.len())
        .flat_map(|s| {
            voices
                .iter()
                .flat_map(move |v| seeds.iter().map(move |seed| (s, *v, *seed)))
        })
        .collect();
    let results: Vec<Job> = jobs
        .par_iter()
        .map(|(s, v, seed)| {
            let subject = &subjects[*s];
            let outcome = (|| {
                let options = crate::compose::Options {
                    voices: *v,
                    seed: *seed,
                    ..options.clone()
                };
                let fugue = crate::compose::compose(subject, &options)?;
                let bands = match &subject.from {
                    Some(name) => crate::neusatz::bands_without(&pieces, &rows, name),
                    None => crate::stats::Bands::of(&rows),
                };
                let verdict = judge(subject, &fugue, &bands);
                Ok((fugue, verdict))
            })();
            (*s, *v, *seed, outcome)
        })
        .collect();
    Ok((results, mark))
}

/// K1 to K5 on a set of subjects for several numbers of voices: one line per fugue and the sums.
pub fn fugues(
    data: &Path,
    set: &str,
    voices: &[usize],
    seeds: &[u64],
    options: &crate::compose::Options,
    out: Option<&Path>,
    detail: bool,
) -> Result<()> {
    let subjects = match set {
        "fresh" | "frisch" => crate::b4::fresh(data)?,
        other => subjects(other)?,
    };
    let (results, mark) = compose_all(data, &subjects, voices, seeds, options)?;
    println!(
        "profile {}, beam {}, at most {} rhythms; yardstick {mark:.1} of {}",
        options.profile.name,
        options.beam,
        options.attempts,
        crate::stats::COUNT
    );
    println!(
        "theme     voices seed bars tones given  junct own excused tries   sec  keys episodes ending   hits struck  entries found extra  K1 K2 K3 K4 K5"
    );
    let mut sums = [0usize; 5];
    let mut all = 0;
    let mut count = 0;
    for (s, v, seed, outcome) in &results {
        let subject = &subjects[*s];
        match outcome {
            Err(error) => println!("{:9} {v:6} {seed:4}  no fugue: {error:#}", subject.name),
            Ok((fugue, verdict)) => {
                count += 1;
                let ks = [
                    verdict.k1(),
                    verdict.k2(),
                    verdict.k3(),
                    verdict.k4(mark),
                    verdict.k5(),
                ];
                for (k, ok) in ks.iter().enumerate() {
                    sums[k] += usize::from(*ok);
                }
                all += usize::from(ks.iter().all(|k| *k));
                let tones = fugue.piece.note_count();
                let given: usize = fugue.planned();
                let flag = |ok: bool| if ok { " +" } else { " -" };
                println!(
                    "{:9} {v:6} {seed:4} {:4} {tones:5} {:4.0}% {:6} {:3} {:7} {:2}+{:<2} {:5.1} {:5} {:4}/{:<3} {:8} {:4} {:6} {:8} {:5} {:5}  {}{}{}{}{}",
                    subject.name,
                    verdict.bars,
                    100.0 * given as f64 / tones.max(1) as f64,
                    verdict.junction,
                    verdict.own,
                    verdict.excused,
                    fugue.attempts.0,
                    fugue.attempts.1,
                    fugue.seconds,
                    verdict.keys,
                    verdict.episodes.0,
                    verdict.episodes.1,
                    verdict.ending,
                    verdict.hits,
                    if verdict.struck_inside { "in" } else { "out" },
                    verdict.entries.0,
                    verdict.entries.1,
                    verdict.entries.2,
                    flag(ks[0]),
                    flag(ks[1]),
                    flag(ks[2]),
                    flag(ks[3]),
                    flag(ks[4]),
                );
                if detail {
                    print!("{}", describe(subject, fugue));
                    let hits = crate::finder::find(&fugue.piece, &subject.notes);
                    for entry in &fugue.design.entries {
                        if !hits
                            .iter()
                            .any(|h| h.voice == entry.voice && h.at == entry.at)
                        {
                            println!(
                                "  not found: the entry of voice {} in bar {}",
                                entry.voice + 1,
                                1 + entry.at / subject.meter.bar()
                            );
                        }
                    }
                    // What the given material breaks against the scaffold.
                    let report = fugue.report(&options.profile);
                    for line in crate::check::lines(&report, &fugue.piece, false)
                        .iter()
                        .filter(|l| l.contains("(V)"))
                    {
                        println!("  {line}");
                    }
                }
                if let Some(dir) = out {
                    write(dir, subject, fugue, *v, *seed)?;
                }
            }
        }
    }
    let mut outside = [0usize; crate::stats::COUNT];
    let mut above = [0usize; crate::stats::COUNT];
    for (_, verdict) in results.iter().filter_map(|r| r.3.as_ref().ok()) {
        for k in &verdict.outside {
            outside[*k] += 1;
        }
        for k in &verdict.above {
            above[*k] += 1;
        }
    }
    println!("measures outside their band, by number of fugues (of them above it):");
    for (k, n) in outside.iter().enumerate() {
        if *n > 0 {
            println!(
                "  {n:2} ({:2})  [{k:2}] {}",
                above[k],
                crate::stats::NAMES[k]
            );
        }
    }
    println!(
        "{} fugues of {}: K1 complete and 30 to 80 bars {}, K2 no prohibition {}, K3 form {}, K4 at least {mark:.0} measures inside {}, K5 every entry found {}; all five: {all}",
        count,
        results.len(),
        sums[0],
        sums[1],
        sums[2],
        sums[3],
        sums[4]
    );
    Ok(())
}

/// Writes a fugue as LilyPond, MIDI and MusicXML; given tones are grey in the LilyPond score.
pub fn write(
    dir: &Path,
    subject: &themes::Subject,
    fugue: &crate::compose::Fugue,
    voices: usize,
    seed: u64,
) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let stem = format!("fuge-{}-{voices}st-s{seed}", subject.name);
    let title = format!("Fuge über {}", subject.title);
    let subtitle = format!("{voices} Stimmen, Seed {seed}: grau gegeben, schwarz gesucht");
    let grey = |v: usize, n: &Note| fugue.is_planned(v, n);
    std::fs::write(
        dir.join(format!("{stem}.ly")),
        crate::export::lilypond(&fugue.piece, &title, &subtitle, &[], &grey),
    )?;
    std::fs::write(
        dir.join(format!("{stem}.mid")),
        crate::export::midi(&fugue.piece, crate::export::tempo(&fugue.piece)),
    )?;
    let marks: Vec<(crate::time::Tick, String)> = fugue
        .design
        .sections
        .iter()
        .map(|s| (s.from, s.name.to_string()))
        .collect();
    std::fs::write(
        dir.join(format!("{stem}.musicxml")),
        crate::musicxml::musicxml(&fugue.piece, &title, "fux-fuge", &marks),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> Option<std::path::PathBuf> {
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../references/datasets/bach-fugen");
        dir.join("wtc").is_dir().then_some(dir)
    }

    /// The ten subjects added for phase B3 are the ones the corpus has.
    #[test]
    fn the_new_subjects_are_those_of_the_corpus() {
        let Some(data) = data() else {
            return;
        };
        let read = load(&data).unwrap();
        let mut pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
        let (more, _) = crate::b1::readable(&data.join("opfer")).unwrap();
        pieces.extend(more.into_iter().map(|r| r.piece));
        for (name, file) in [
            ("wtk2-01", "wtc2f01"),
            ("wtk2-05", "wtc2f05"),
            ("wtk2-12", "wtc2f12"),
            ("wtk2-16", "wtc2f16"),
            ("wtk2-18", "wtc2f18"),
            ("wtk2-11", "wtc2f11"),
            ("wtk2-21", "wtc2f21"),
            ("bwv1079", "offering-001"),
            ("wtk1-15", "wtc1f15"),
            ("wtk2-09", "wtc2f09"),
        ] {
            let typed = themes::BUILTIN
                .iter()
                .find(|b| b.name == name)
                .unwrap()
                .subject()
                .unwrap();
            let piece = pieces.iter().find(|p| p.name == file).unwrap();
            let (notes, period) = first_subject(piece).unwrap();
            assert_eq!(typed.key, piece.key, "{name}");
            assert_eq!(typed.meter, piece.meter, "{name}");
            assert_eq!(typed.period, period, "{name}");
            assert_eq!(themes::text(&typed.notes), themes::text(&notes), "{name}");
        }
    }
}
