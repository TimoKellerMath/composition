//! The runs of the study as functions that print their tables.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::chord::{self, Agreement};
use crate::corrupt;
use crate::figures::{
    Figure, Layer, NoteCensus, Pair, PairCensus, Reading, note_census, pair_census,
};
use crate::kern::{self, KernPiece};
use crate::key::Harmony;
use crate::roman;
use crate::score::Piece;
use crate::time::EIGHTH;

pub fn load(data: &Path) -> Result<Vec<KernPiece>> {
    kern::read_dir(&data.join("wtc"))
}

/// `wtc2f07` for book 2, number 7.
pub fn name(book: usize, number: usize) -> String {
    format!("wtc{book}f{number:02}")
}

pub fn find<'a>(pieces: &'a [KernPiece], name: &str) -> Result<&'a Piece> {
    pieces
        .iter()
        .map(|p| &p.piece)
        .find(|p| p.name == name)
        .with_context(|| format!("no piece {name}"))
}

/// The other book's fugue of the same number, which is in the same key.
fn twin(pieces: &[KernPiece], piece: &Piece) -> Option<Piece> {
    let other = if piece.name.starts_with("wtc1") {
        piece.name.replacen("wtc1", "wtc2", 1)
    } else {
        piece.name.replacen("wtc2", "wtc1", 1)
    };
    find(pieces, &other).ok().cloned()
}

/// A fugue with the harmony of its analysis and the tokens of it that could not be read.
pub type Analysed = (Piece, Harmony, Vec<(i32, String)>);

/// Tymoczko's analyses: the piece and its harmony, with the tokens that could not be read.
pub fn analyses(data: &Path, pieces: &[KernPiece]) -> Result<Vec<Analysed>> {
    let mut out = Vec::new();
    for (file, book, number) in [
        ("I_19", 1, 19),
        ("II_16", 2, 16),
        ("I_22", 1, 22),
        ("II_07", 2, 7),
        ("II_11", 2, 11),
        ("II_23", 2, 23),
        ("II_24", 2, 24),
    ] {
        let path = data.join("wir").join(format!("{file}_fugue.txt"));
        let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
        let analysis = roman::parse(&text)?;
        let piece = find(pieces, &name(book, number))?.clone();
        let harmony = roman::harmony(&analysis, &piece);
        out.push((piece, harmony, analysis.unread));
    }
    Ok(out)
}

/// The fugues of T3 on which the figures were developed.
pub const DEVELOPMENT: [&str; 2] = ["wtc1f19", "wtc2f16"];

/// The three falsified versions of the counter-test.
pub fn falsified(pieces: &[KernPiece], piece: &Piece) -> Vec<(&'static str, Piece)> {
    let v = 1.min(piece.voices.len() - 1);
    let mut out = vec![
        ("G1", corrupt::shifted(piece, v, EIGHTH)),
        ("G2", corrupt::shifted(piece, v, piece.meter.beat())),
    ];
    if let Some(twin) = twin(pieces, piece) {
        let donor = &twin.voices[1.min(twin.voices.len() - 1)];
        out.push(("G3", corrupt::replaced(piece, v, donor)));
    }
    out
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        "   –  ".into()
    } else {
        format!("{:5.1}%", 100.0 * part as f64 / whole as f64)
    }
}

fn pair_line(label: &str, census: &PairCensus) -> String {
    let (total, rest) = census.struck();
    let held = census.of(Layer::AgainstHeld, None, None);
    let held_rest = census.of(Layer::AgainstHeld, None, Some(Figure::Unexplained));
    format!(
        "{label:22} beats {:5}  struck pairs {total:5}  unexplained {rest:4} {}  per 100 beats {:5.2}   against held {held:5}  unexplained {held_rest:4} {}",
        census.beats_sounding,
        percent(rest, total),
        100.0 * rest as f64 / census.beats_sounding.max(1) as f64,
        percent(held_rest, held)
    )
}

fn figure_table(census: &PairCensus) {
    println!("    figure                   A bass  A upper  B bass  B upper  C bass  C upper");
    for figure in Figure::ALL {
        let row: Vec<usize> = [Layer::Free, Layer::Repeated, Layer::AgainstHeld]
            .into_iter()
            .flat_map(|layer| {
                [Pair::Bass, Pair::Upper].map(|pair| census.of(layer, Some(pair), Some(figure)))
            })
            .collect();
        if row.iter().sum::<usize>() > 0 {
            println!(
                "    {:22} {:8} {:8} {:7} {:8} {:7} {:8}",
                figure.german(),
                row[0],
                row[1],
                row[2],
                row[3],
                row[4],
                row[5]
            );
        }
    }
}

/// Sums of the censuses of a set of pieces under one way of getting the harmony.
#[derive(Default)]
struct Sums {
    bach: PairCensus,
    falsified: BTreeMap<&'static str, PairCensus>,
}

impl Sums {
    fn add(&mut self, pieces: &[KernPiece], piece: &Piece, harmony: Option<&Harmony>) {
        let own;
        let h = match harmony {
            Some(h) => h,
            None => {
                own = chord::estimate(piece);
                &own
            }
        };
        self.bach.add(&pair_census(Reading { piece, harmony: h }));
        for (label, version) in falsified(pieces, piece) {
            // With the estimate, the chords are estimated anew on the falsified version.
            let own;
            let h = match harmony {
                Some(h) => h,
                None => {
                    own = chord::estimate(&version);
                    &own
                }
            };
            self.falsified
                .entry(label)
                .or_default()
                .add(&pair_census(Reading {
                    piece: &version,
                    harmony: h,
                }));
        }
    }

    fn print(&self, title: &str) {
        println!("{title}");
        println!("  {}", pair_line("Bach", &self.bach));
        let (total, rest) = self.bach.struck();
        let bach = rest as f64 / total.max(1) as f64;
        for (label, census) in &self.falsified {
            let (t, r) = census.struck();
            let share = r as f64 / t.max(1) as f64;
            let ratio = if bach > 0.0 {
                format!("{:5.2}", share / bach)
            } else if share > 0.0 {
                "  inf".to_string()
            } else {
                "    –".to_string()
            };
            println!("  {}  ratio to Bach {ratio}", pair_line(label, census));
        }
    }
}

/// T3: the dissonant pairs on beats, against Tymoczko's chords and against the estimate.
pub fn t3(data: &Path, detail: bool, development_only: bool) -> Result<()> {
    let pieces = load(data)?;
    let mut analyses = analyses(data, &pieces)?;
    if development_only {
        analyses.retain(|a| DEVELOPMENT.contains(&a.0.name.as_str()));
    }
    println!("== Tymoczko's analyses read");
    let mut all = Agreement::default();
    for (piece, harmony, unread) in &analyses {
        let own = chord::estimate(piece);
        let a = chord::agreement(piece, harmony, &own);
        all += a;
        println!(
            "{:8} {:3} chords, {} unread {:?}; estimate against it on {} beats: same root {}, same tones {}, nested {}, same key {}",
            piece.name,
            harmony.spans.len(),
            unread.len(),
            unread.iter().map(|u| u.1.as_str()).collect::<Vec<_>>(),
            a.beats,
            percent(a.same_root, a.beats),
            percent(a.same_tones, a.beats),
            percent(a.nested, a.beats),
            percent(a.same_key, a.beats)
        );
    }
    println!(
        "all seven: {} beats, same root {}, same tones {}, nested {}, same key {}",
        all.beats,
        percent(all.same_root, all.beats),
        percent(all.same_tones, all.beats),
        percent(all.nested, all.beats),
        percent(all.same_key, all.beats)
    );
    for (title, own) in [("Tymoczko's chords", false), ("estimated chords", true)] {
        println!("\n== {title}, per fugue");
        let mut development = Sums::default();
        let mut test = Sums::default();
        let mut seven = Sums::default();
        for (piece, harmony, _) in &analyses {
            let h = (!own).then_some(harmony);
            let mut one = Sums::default();
            one.add(&pieces, piece, h);
            one.print(&piece.name);
            if detail {
                figure_table(&one.bach);
            }
            if DEVELOPMENT.contains(&piece.name.as_str()) {
                development.add(&pieces, piece, h);
            } else {
                test.add(&pieces, piece, h);
            }
            seven.add(&pieces, piece, h);
        }
        println!("\n== {title}, sums");
        development.print("development (I/19, II/16)");
        test.print("test (I/22, II/7, II/11, II/23, II/24)");
        seven.print("all seven");
        figure_table(&seven.bach);
    }
    if development_only {
        return Ok(());
    }
    println!("\n== estimated chords, all 48");
    let mut all = Sums::default();
    for read in &pieces {
        all.add(&pieces, &read.piece, None);
    }
    all.print("all 48");
    figure_table(&all.bach);
    Ok(())
}

/// The tones of all 48 fugues by figure, with estimated chords.
pub fn notes(data: &Path) -> Result<()> {
    let pieces = load(data)?;
    let mut sum = NoteCensus::default();
    for read in &pieces {
        let harmony = chord::estimate(&read.piece);
        sum.add(&note_census(Reading {
            piece: &read.piece,
            harmony: &harmony,
        }));
    }
    println!(
        "{} tones, {} before the first chord",
        sum.notes, sum.without_chord
    );
    println!("figure                  on beats  half beats    finer     all   share");
    for figure in Figure::ALL {
        let row: Vec<usize> = [2u8, 1, 0]
            .iter()
            .map(|w| sum.struck.get(&(*w, figure)).copied().unwrap_or(0))
            .collect();
        let total: usize = row.iter().sum();
        println!(
            "{:22} {:9} {:11} {:8} {:7}  {}",
            figure.german(),
            row[0],
            row[1],
            row[2],
            total,
            percent(total, sum.read())
        );
    }
    println!("held tones foreign to the chord on a beat:");
    for (figure, count) in &sum.held {
        println!("  {:22} {count:6}", figure.german());
    }
    Ok(())
}

fn describe(piece: &Piece, v: usize, i: usize, t: crate::time::Tick) -> String {
    let voice = &piece.voices[v];
    let note = &voice.notes[i];
    let name =
        |n: Option<&crate::score::Note>| n.map_or("–".to_string(), |n| n.pitch.to_string());
    format!(
        "v{v} {} [{} {} {}]{}",
        note.pitch,
        name(voice.before(i)),
        if note.on == t { "struck" } else { "held" },
        name(voice.after(i)),
        if voice.repeats(i) { " rep" } else { "" }
    )
}

/// Lists the dissonant pairs on beats whose figure is `wanted`, with their surroundings.
pub fn list_pairs(piece: &Piece, harmony: &Harmony, wanted: Figure, limit: usize) {
    let reading = Reading { piece, harmony };
    let mut shown = 0;
    for t in piece.beats() {
        let sounding: Vec<(usize, usize)> = piece
            .voices
            .iter()
            .enumerate()
            .filter_map(|(v, voice)| voice.index_at(t).map(|i| (v, i)))
            .collect();
        if sounding.len() < 2 {
            continue;
        }
        let pitch = |s: &(usize, usize)| piece.voices[s.0].notes[s.1].pitch;
        let lowest = sounding.iter().map(pitch).min();
        for a in 0..sounding.len() {
            for b in a + 1..sounding.len() {
                let (x, y) = (sounding[a], sounding[b]);
                let (lo, hi) = if pitch(&x) <= pitch(&y) {
                    (pitch(&x), pitch(&y))
                } else {
                    (pitch(&y), pitch(&x))
                };
                let against_bass = Some(lo) == lowest;
                if lo.to(hi).is_consonant(!against_bass) {
                    continue;
                }
                let struck = |s: (usize, usize)| piece.voices[s.0].notes[s.1].on == t;
                if !struck(x) && !struck(y) {
                    continue;
                }
                let (Some(fx), Some(fy)) =
                    (reading.figure(x.0, x.1, t), reading.figure(y.0, y.1, t))
                else {
                    continue;
                };
                if fx.max(fy) != wanted || shown >= limit {
                    continue;
                }
                shown += 1;
                let (bar, pos) = piece.bar_of(t);
                let span = harmony.at(t);
                println!(
                    "  bar {bar:3} +{pos:3}  {:28} {:10?} | {:28} {:10?} | chord {} in {}",
                    describe(piece, x.0, x.1, t),
                    fx,
                    describe(piece, y.0, y.1, t),
                    fy,
                    span.map_or("?".into(), |s| s.chord.to_string()),
                    span.map_or("?".into(), |s| s.key.to_string())
                );
            }
        }
    }
}

/// Prints Tymoczko's chord and the estimate for every beat of the bars `from..=to`.
pub fn compare_chords(data: &Path, piece_name: &str, from: i32, to: i32) -> Result<()> {
    let pieces = load(data)?;
    let analyses = analyses(data, &pieces)?;
    let piece = find(&pieces, piece_name)?;
    let own = chord::estimate(piece);
    let theirs = analyses
        .iter()
        .find(|a| a.0.name == piece_name)
        .map(|a| &a.1);
    for t in piece.beats() {
        let (bar, pos) = piece.bar_of(t);
        if bar < from || bar > to {
            continue;
        }
        let tones: Vec<String> = piece
            .voices
            .iter()
            .map(|v| {
                v.range(t, t + piece.meter.beat())
                    .iter()
                    .map(|n| n.pitch.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        let label = |h: Option<&Harmony>| {
            h.and_then(|h| h.at(t))
                .map_or("–".to_string(), |s| format!("{} ({})", s.chord, s.key))
        };
        println!(
            "bar {bar:3} +{pos:3}  theirs {:16} own {:16} | {}",
            label(theirs),
            label(Some(&own)),
            tones.join(" | ")
        );
    }
    Ok(())
}

/// Lists the unexplained pairs of the development fugues under Tymoczko's chords.
pub fn list_development(data: &Path, own: bool, limit: usize) -> Result<()> {
    let pieces = load(data)?;
    for (piece, harmony, _) in analyses(data, &pieces)? {
        if !DEVELOPMENT.contains(&piece.name.as_str()) {
            continue;
        }
        println!("{}", piece.name);
        let estimate = chord::estimate(&piece);
        list_pairs(
            &piece,
            if own { &estimate } else { &harmony },
            Figure::Unexplained,
            limit,
        );
    }
    Ok(())
}

/// Agreement of the estimate with Tymoczko's chords on the development fugues for several grids
/// and costs of a change.
pub fn tune_estimate(data: &Path) -> Result<()> {
    let pieces = load(data)?;
    for (piece, harmony, _) in analyses(data, &pieces)? {
        if !DEVELOPMENT.contains(&piece.name.as_str()) {
            continue;
        }
        let meter = piece.meter;
        let sub = if meter.compound() {
            meter.beat() / 3
        } else {
            meter.beat() / 2
        };
        for (label, step) in [("beat", meter.beat()), ("sub", sub)] {
            for change in [0.05f32, 0.10, 0.15, 0.20, 0.30] {
                let own = chord::estimate_on(&piece, step, change);
                let on_beats = chord::agreement_on(&piece, &harmony, &own, meter.beat());
                let on_subs = chord::agreement_on(&piece, &harmony, &own, sub);
                println!(
                    "{} grid {label:4} change {change:.2}: {:4} chords; on beats root {} tones {} nested {}; on subdivisions root {} tones {} nested {}",
                    piece.name,
                    own.spans.len(),
                    percent(on_beats.same_root, on_beats.beats),
                    percent(on_beats.same_tones, on_beats.beats),
                    percent(on_beats.nested, on_beats.beats),
                    percent(on_subs.same_root, on_subs.beats),
                    percent(on_subs.same_tones, on_subs.beats),
                    percent(on_subs.nested, on_subs.beats)
                );
            }
        }
    }
    Ok(())
}

/// A piece moved to the key of the same mode with no accidental or one flat that lies nearest.
pub fn transposed_for_fux(piece: &Piece) -> Piece {
    use crate::pitch::{Lof, Pitch};
    let targets = if piece.key.minor {
        [Lof(3), Lof(2)]
    } else {
        [Lof(0), Lof(-1)]
    };
    let home = Pitch::new(piece.key.tonic.step(), piece.key.tonic.alter(), 4);
    let (target, iv) = targets
        .into_iter()
        .map(|t| (t, home.to(Pitch::nearest(t, home))))
        .min_by_key(|(_, iv)| iv.semis.abs())
        .expect("two targets");
    let mut out = piece.clone();
    out.key.tonic = target;
    for voice in out.voices.iter_mut().chain(out.extras.iter_mut()) {
        for note in &mut voice.notes {
            note.pitch = note.pitch.up(iv);
        }
    }
    out
}

/// T2: four fugues without sixteenths against today's rules (profile `jeppesen`).
pub fn t2(data: &Path) -> Result<()> {
    use fux_core::rules::{Checker, Profile};
    let pieces = load(data)?;
    let checker = Checker::new(&Profile::jeppesen());
    // Rule, then forbidden and costing violations.
    let mut rules: BTreeMap<&'static str, (usize, usize, u32)> = BTreeMap::new();
    let mut notes_all = 0;
    println!(
        "fugue     key  voices  notes  measures  forbidden  preferences   cost  forbidden per 100 notes"
    );
    for name in ["wtc1f04", "wtc1f22", "wtc2f09", "wtc2f22"] {
        let piece = transposed_for_fux(find(&pieces, name)?);
        let text = kern::write_plain(&piece)?;
        let parsed = fux_core::kern::parse_kern(&text).with_context(|| name.to_string())?;
        let score = &parsed.score;
        let report = checker.check(score);
        let notes: usize = score
            .voices
            .iter()
            .map(|v| v.notes.iter().filter(|n| n.pitch.is_some()).count())
            .sum();
        notes_all += notes;
        for violation in &report.violations {
            let entry = rules.entry(violation.rule).or_default();
            if violation.is_forbidden() {
                entry.0 += 1;
            } else {
                entry.1 += 1;
                entry.2 += violation.level.cost();
            }
        }
        let forbidden = report.forbidden();
        println!(
            "{name:9} {:4} {:6} {notes:6} {:9} {forbidden:10} {:12} {:6}  {:6.1}",
            piece.key.to_string(),
            score.voices.len(),
            score.total_ticks() / score.meter.ticks_per_measure(),
            report.violations.len() - forbidden,
            report.cost(),
            100.0 * forbidden as f64 / notes as f64
        );
    }
    println!("\nrule                      forbidden  per 100 notes  preferences   cost");
    let mut rows: Vec<_> = rules.into_iter().collect();
    rows.sort_by_key(|(_, (forbidden, costing, _))| std::cmp::Reverse((*forbidden, *costing)));
    for (rule, (forbidden, costing, cost)) in rows {
        println!(
            "{rule:25} {forbidden:9} {:14.2} {costing:12} {cost:6}",
            100.0 * forbidden as f64 / notes_all as f64
        );
    }
    Ok(())
}

/// The Algomus labels against the notes: per fugue the entries of the subject with their voice,
/// bar and number of notes.
pub fn labels(data: &Path) -> Result<()> {
    let pieces = load(data)?;
    for number in 1..=24 {
        let Ok(labels) = crate::algomus::read_for(data, number) else {
            println!("{}: no labels", name(1, number));
            continue;
        };
        let piece = find(&pieces, &name(1, number))?;
        let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
        for label in &labels {
            *kinds.entry(label.kind.as_str()).or_default() += 1;
        }
        let entries: Vec<String> = labels
            .iter()
            .filter(|l| l.is_subject())
            .map(|l| {
                let notes = l.notes(piece);
                format!(
                    "v{}@{}:{}n{}",
                    l.voice.unwrap_or(9),
                    piece.bar_of(l.start).0,
                    notes.len(),
                    notes
                        .first()
                        .map_or(String::new(), |n| format!("({})", n.pitch))
                )
            })
            .collect();
        println!(
            "{} {} {:?}\n    {}",
            piece.name,
            piece.voices.len(),
            kinds,
            entries.join(" ")
        );
    }
    Ok(())
}

/// The measures of all pieces of the corpus, in parallel.
pub fn corpus_measures(pieces: &[KernPiece]) -> Vec<[f64; crate::stats::COUNT]> {
    use rayon::prelude::*;
    pieces
        .par_iter()
        .map(|read| crate::stats::measures(&read.piece))
        .collect()
}

/// A value of a measure as it is printed: shares in percent, rates as they are.
pub fn shown(k: usize, value: f64) -> String {
    if matches!(k, 3 | 23 | 24) {
        format!("{value:7.2}")
    } else {
        format!("{:6.1}%", 100.0 * value)
    }
}

/// T4: the bands of the measures over the 48 fugues and the yardstick.
pub fn t4(data: &Path) -> Result<()> {
    use crate::stats::{Bands, COUNT, NAMES, yardstick};
    let pieces = load(data)?;
    let rows = corpus_measures(&pieces);
    let bands = Bands::of(&rows);
    println!(
        "{:44} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "measure", "min", "p10", "median", "p90", "max"
    );
    for k in 0..COUNT {
        let values: Vec<f64> = rows.iter().map(|r| r[k]).collect();
        println!(
            "{:44} {} {} {} {} {}",
            NAMES[k],
            shown(k, crate::stats::quantile(&values, 0.0)),
            shown(k, bands.low[k]),
            shown(k, bands.median[k]),
            shown(k, bands.high[k]),
            shown(k, crate::stats::quantile(&values, 1.0))
        );
    }
    let (hits, mark) = yardstick(&rows);
    println!("\nmeasures inside the bands of the other 47, per fugue:");
    for (read, hit) in pieces.iter().zip(&hits) {
        print!("{} {hit}  ", read.piece.name);
    }
    let values: Vec<f64> = hits.iter().map(|h| *h as f64).collect();
    println!(
        "\nmin {} p10 {mark:.1} median {} max {} of {COUNT}",
        crate::stats::quantile(&values, 0.0),
        crate::stats::quantile(&values, 0.5),
        crate::stats::quantile(&values, 1.0)
    );
    Ok(())
}

/// Lists the parallel fifths and octaves of a piece as the measure counts them.
pub fn list_parallels(piece: &Piece) {
    for a in 0..piece.voices.len() {
        for b in a + 1..piece.voices.len() {
            let (upper, lower) = (&piece.voices[a], &piece.voices[b]);
            for (i, note) in upper.notes.iter().enumerate() {
                let (Some(before), Some(j)) = (upper.before(i), lower.index_at(note.on)) else {
                    continue;
                };
                let other = &lower.notes[j];
                let Some(other_before) = lower.before(j) else {
                    continue;
                };
                if other.on != note.on {
                    continue;
                }
                let now = other.pitch.to(note.pitch);
                let then = other_before.pitch.to(before.pitch);
                let moved = before.pitch.to(note.pitch).dir();
                if now.is_perfect()
                    && then.is_perfect()
                    && now.simple() == then.simple()
                    && moved != 0
                    && moved == other_before.pitch.to(other.pitch).dir()
                {
                    let (bar, pos) = piece.bar_of(note.on);
                    println!(
                        "  bar {bar} +{pos}: v{a} {}→{} (from tick {}), v{b} {}→{} (from tick {})",
                        before.pitch,
                        note.pitch,
                        before.on,
                        other_before.pitch,
                        other.pitch,
                        other_before.on
                    );
                }
            }
        }
    }
}

/// T9: the measures of the pieces in other directories of the data against the bands of the 48.
pub fn t9(data: &Path, dirs: &[String]) -> Result<()> {
    use crate::stats::{Bands, COUNT, NAMES, measures, quantile, yardstick};
    let pieces = load(data)?;
    let rows = corpus_measures(&pieces);
    let bands = Bands::of(&rows);
    let (_, mark) = yardstick(&rows);
    for dir in dirs {
        let path = data.join(dir);
        let mut names: Vec<_> = std::fs::read_dir(&path)
            .with_context(|| path.display().to_string())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "krn"))
            .collect();
        names.sort();
        println!(
            "== {dir}: {} files; yardstick {mark:.1} of {COUNT}",
            names.len()
        );
        println!("file              voices meter key   tones  hits  outside");
        let mut hits_all = Vec::new();
        let mut outside = [0usize; COUNT];
        let mut tones = 0;
        for file in &names {
            let text = String::from_utf8_lossy(&std::fs::read(file)?).into_owned();
            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
            let title = text
                .lines()
                .find_map(|l| l.strip_prefix("!!!OTL:"))
                .map_or(String::new(), |t| t.trim().to_string());
            match kern::read(stem, &text) {
                Err(error) => println!("{stem:17} not read: {error}"),
                Ok(read) => {
                    let piece = &read.piece;
                    let row = measures(piece);
                    let hits = bands.hits(&row);
                    let misses: Vec<usize> = (0..COUNT)
                        .filter(|k| !bands.contains(*k, row[*k]))
                        .collect();
                    for k in &misses {
                        outside[*k] += 1;
                    }
                    tones += piece.note_count();
                    hits_all.push(hits as f64);
                    println!(
                        "{stem:17} {:6} {:5} {:4} {:6} {hits:5}  {misses:?}  {title}",
                        piece.voices.len(),
                        piece.meter.to_string(),
                        piece.key.to_string(),
                        piece.note_count()
                    );
                }
            }
        }
        println!(
            "{dir}: {tones} tones in {} pieces; hits: min {} median {} max {}",
            hits_all.len(),
            quantile(&hits_all, 0.0),
            quantile(&hits_all, 0.5),
            quantile(&hits_all, 1.0)
        );
        for (k, count) in outside.iter().enumerate() {
            if *count * 3 >= hits_all.len().max(1) {
                println!("  outside in {count} pieces: [{k}] {}", NAMES[k]);
            }
        }
    }
    Ok(())
}

/// Counts the tables of `tables.rs` on the corpus and writes them as JSON.
pub fn write_tables(data: &Path, out: &Path) -> Result<()> {
    use crate::tables::{Source, Tables};
    use crate::time::SIXTEENTH;
    let pieces = load(data)?;
    let rows = corpus_measures(&pieces);
    let (_, yardstick) = crate::stats::yardstick(&rows);
    let sources = pieces
        .iter()
        .map(|read| {
            let piece = &read.piece;
            let pulse = piece.meter.pulse();
            let mut rhythm = crate::rhythm::Table::new((pulse / SIXTEENTH) as usize);
            rhythm.learn(piece);
            let mut intervals = crate::melody::Table::default();
            intervals.learn(piece);
            let unit = piece.meter.unit();
            let mut cells = crate::cells::Table::new(unit);
            cells.learn(piece);
            Source {
                name: piece.name.clone(),
                meter: piece.meter,
                pulse,
                profile: crate::rhythm::value_profile(piece.voices.iter().flat_map(|v| &v.notes)),
                rhythm,
                intervals,
                unit,
                subject_profile: crate::cells::subject_profile(piece),
                cells,
            }
        })
        .collect();
    let tables = Tables {
        sources,
        transitions: crate::study_fugues::transitions(data)?,
        bands: crate::stats::Bands::of(&rows),
        yardstick,
    };
    let text = serde_json::to_string(&tables.to_json())?;
    std::fs::write(out, text + "\n").with_context(|| out.display().to_string())?;
    println!(
        "{}: {} fugues, {} successions of chords",
        out.display(),
        tables.sources.len(),
        tables.transitions.size()
    );
    Ok(())
}
