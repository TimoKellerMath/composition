//! T5: Bach's first answers against the rule of `answer`.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::algomus;
use crate::answer::{self, AnswerKind};
use crate::key::Key;
use crate::pitch::Pitch;
use crate::score::{Note, Piece};
use crate::study::{find, load, name};
use crate::subject::{self, Exposition};

/// A tone as its degree in the key, with `#` or `b` where it leaves the major or natural minor
/// scale: `5`, `#4`, `#7`.
pub fn degree_name(key: Key, pitch: Pitch) -> String {
    let class = pitch.lof();
    let degree = key.degree_of(class);
    let alter = (class.0 - key.degree(degree).0) / 7;
    let sign = if alter > 0 { "#" } else { "b" };
    format!(
        "{}{}",
        sign.repeat(alter.unsigned_abs() as usize),
        degree + 1
    )
}

/// How an answer tone lies to its subject tone, octaves aside.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Region {
    /// A perfect fifth above.
    Fifth,
    /// A perfect fourth above.
    Fourth,
    /// A fifth by its letters, but augmented or diminished: an altered tone of a real answer.
    AlteredFifth,
    /// A fourth by its letters, but augmented or diminished.
    AlteredFourth,
    Other,
}

pub fn region(subject: Pitch, answer: Pitch) -> Region {
    let iv = subject.to(answer);
    match (iv.steps.rem_euclid(7), iv.semis.rem_euclid(12)) {
        (4, 7) => Region::Fifth,
        (3, 5) => Region::Fourth,
        (4, _) => Region::AlteredFifth,
        (3, _) => Region::AlteredFourth,
        _ => Region::Other,
    }
}

impl Region {
    const fn mark(self) -> char {
        match self {
            Region::Fifth => '5',
            Region::Fourth => '4',
            Region::AlteredFifth => 'q',
            Region::AlteredFourth => 'v',
            Region::Other => '?',
        }
    }
}

/// Bach's answer by its tones: real if every tone is a fifth above its subject tone by its
/// letters, tonal if some tone is a fourth above; `exact` if no tone is altered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Written {
    pub kind: Option<AnswerKind>,
    pub exact: bool,
}

pub fn written(exposition: &Exposition) -> Written {
    let regions: Vec<Region> = exposition
        .subject
        .iter()
        .zip(&exposition.answer)
        .map(|(s, a)| region(s.pitch, a.pitch))
        .collect();
    let fifth = |r: &Region| matches!(r, Region::Fifth | Region::AlteredFifth);
    let fourth = |r: &Region| matches!(r, Region::Fourth | Region::AlteredFourth);
    let kind = if regions.iter().all(fifth) {
        Some(AnswerKind::Real)
    } else if regions.iter().all(|r| fifth(r) || fourth(r)) {
        Some(AnswerKind::Tonal)
    } else {
        None
    };
    Written {
        kind,
        exact: regions
            .iter()
            .all(|r| matches!(r, Region::Fifth | Region::Fourth)),
    }
}

fn line(key: Key, notes: &[Note]) -> String {
    notes
        .iter()
        .map(|n| format!("{}:{}", n.pitch, degree_name(key, n.pitch)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn kind_name(kind: Option<AnswerKind>) -> &'static str {
    match kind {
        Some(AnswerKind::Real) => "real",
        Some(AnswerKind::Tonal) => "tonal",
        None => "other",
    }
}

/// The result of one fugue.
#[derive(Clone, Debug)]
pub struct Compared {
    pub tones: usize,
    pub written: Written,
    pub rule: AnswerKind,
    /// Positions (from 0) where the rule's tone is not Bach's, by spelled pitch class.
    pub differing: Vec<usize>,
    pub modulates: bool,
}

impl Compared {
    pub fn kind_agrees(&self) -> bool {
        self.written.kind == Some(self.rule)
    }

    pub fn all_agree(&self) -> bool {
        self.differing.is_empty()
    }
}

pub fn compare(exposition: &Exposition, key: Key) -> (Compared, answer::Answer) {
    let rule = answer::answer(&exposition.subject, key);
    let differing = rule
        .notes
        .iter()
        .zip(&exposition.answer)
        .enumerate()
        .filter(|(_, (r, b))| r.pitch.lof() != b.pitch.lof())
        .map(|(i, _)| i)
        .collect();
    let compared = Compared {
        tones: exposition.subject.len(),
        written: written(exposition),
        rule: rule.kind,
        differing,
        modulates: answer::modulates(&exposition.subject, key),
    };
    (compared, rule)
}

#[derive(Clone, Copy, Debug, Default)]
struct Totals {
    fugues: usize,
    kind: usize,
    all: usize,
    tones: usize,
    tones_agree: usize,
    real: usize,
    tonal: usize,
}

impl Totals {
    fn add(&mut self, c: &Compared) {
        self.fugues += 1;
        self.kind += c.kind_agrees() as usize;
        self.all += c.all_agree() as usize;
        self.tones += c.tones;
        self.tones_agree += c.tones - c.differing.len();
        self.real += (c.written.kind == Some(AnswerKind::Real)) as usize;
        self.tonal += (c.written.kind == Some(AnswerKind::Tonal)) as usize;
    }

    fn print(&self, label: &str) {
        println!(
            "{label:28} {:2} fugues; Bach real {:2}, tonal {:2}; kind agreed {:2}; all tones agreed {:2}; tones agreed {} of {}",
            self.fugues, self.real, self.tonal, self.kind, self.all, self.tones_agree, self.tones
        );
    }
}

fn print_fugue(piece: &Piece, e: &Exposition, source: &str) -> Compared {
    let (c, rule) = compare(e, piece.key);
    let marks: String = e
        .subject
        .iter()
        .zip(&e.answer)
        .map(|(s, a)| region(s.pitch, a.pitch).mark())
        .collect();
    let rule_marks: String = rule
        .fourth
        .iter()
        .map(|f| if *f { '4' } else { '5' })
        .collect();
    println!(
        "{:8} {:3} {:6} S v{} A v{} {:2} tones  Bach {:5}{} rule {:5}{} {}",
        piece.name,
        piece.key.to_string(),
        source,
        e.subject_voice,
        e.answer_voice,
        c.tones,
        kind_name(c.written.kind),
        if c.written.exact { " " } else { "*" },
        kind_name(Some(c.rule)),
        if c.modulates { " (mod.)" } else { "       " },
        if c.all_agree() {
            "all tones agree".to_string()
        } else {
            format!("{} tones differ", c.differing.len())
        }
    );
    if !c.all_agree() || !c.written.exact {
        println!("         Bach {marks}");
        println!("         rule {rule_marks}");
        let list: Vec<String> = c
            .differing
            .iter()
            .map(|i| {
                format!(
                    "{}: {} ({}) Bach {} rule {}",
                    i + 1,
                    e.subject[*i].pitch.lof(),
                    degree_name(piece.key, e.subject[*i].pitch),
                    e.answer[*i].pitch.lof(),
                    rule.notes[*i].pitch.lof()
                )
            })
            .collect();
        if !list.is_empty() {
            println!("         {}", list.join("; "));
        }
    }
    c
}

/// The fields of one line of a CSV file; a field in double quotes may hold commas.
fn csv_fields(line: &str) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                fields.last_mut().expect("one field").push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => fields.push(String::new()),
            other => fields.last_mut().expect("one field").push(other),
        }
    }
    fields
}

/// Answer kinds from a published table: a CSV file whose first line names the columns `book`,
/// `number` and `answer_prout` (`real` or `tonal`).
pub fn published(path: &Path) -> Result<BTreeMap<(usize, usize), AnswerKind>> {
    let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let mut lines = text.lines();
    let header = csv_fields(lines.next().unwrap_or(""));
    let column = |name: &str| {
        header
            .iter()
            .position(|h| h == name)
            .with_context(|| format!("{}: no column {name}", path.display()))
    };
    let (book, number, kind) = (column("book")?, column("number")?, column("answer_prout")?);
    let mut out = BTreeMap::new();
    for line in lines {
        let fields = csv_fields(line);
        let field = |i: usize| fields.get(i).map_or("", |f| f.trim());
        let kind = match field(kind) {
            "real" => AnswerKind::Real,
            "tonal" => AnswerKind::Tonal,
            _ => continue,
        };
        if let (Ok(book), Ok(number)) = (field(book).parse(), field(number).parse()) {
            out.insert((book, number), kind);
        }
    }
    Ok(out)
}

/// T5. `book` 1 or 2 limits the run to one book; the rule was frozen on book 1 before book 2
/// was looked at. `table` is a published table of answer kinds to check Bach's answers as read
/// from the notes.
pub fn run(data: &Path, book: Option<usize>, table: Option<&Path>) -> Result<()> {
    let pieces = load(data)?;
    println!(
        "Per fugue: source of the subject's length (labels: Algomus, entry: the heuristic of \
         `subject::from_entries`), voices of subject and answer, number of tones, Bach's answer \
         (* with an altered tone), the rule's answer (mod.: the subject closes in the dominant \
         key). Marks per tone: 5 a fifth above, 4 a fourth above, q and v the same with an \
         alteration.\n"
    );
    let mut all = Totals::default();
    let mut kinds: BTreeMap<(usize, usize), Option<AnswerKind>> = BTreeMap::new();
    for b in [1, 2] {
        if book.is_some_and(|only| only != b) {
            continue;
        }
        let mut totals = Totals::default();
        for number in 1..=24 {
            let piece = find(&pieces, &name(b, number))?;
            let labels = if b == 1 {
                algomus::read_for(data, number).ok()
            } else {
                None
            };
            let source = if labels.is_some() { "labels" } else { "entry" };
            match subject::exposition(piece, labels.as_deref()) {
                Some(e) => {
                    let c = print_fugue(piece, &e, source);
                    totals.add(&c);
                    all.add(&c);
                    kinds.insert((b, number), c.written.kind);
                }
                None => println!("{:8} no exposition found", piece.name),
            }
        }
        println!();
        totals.print(if b == 1 { "WTC I" } else { "WTC II" });
        println!();
    }
    if book.is_none() {
        all.print("all 48");
        println!();
    }
    if let Some(path) = table {
        let table = published(path)?;
        let mut same = 0;
        let mut differing = Vec::new();
        for (fugue, kind) in &kinds {
            match table.get(fugue) {
                Some(published) if Some(*published) == *kind => same += 1,
                Some(published) => differing.push(format!(
                    "{} (notes {}, table {})",
                    name(fugue.0, fugue.1),
                    kind_name(*kind),
                    kind_name(Some(*published))
                )),
                None => differing.push(format!("{} (not in the table)", name(fugue.0, fugue.1))),
            }
        }
        println!(
            "Bach's answers as read from the notes against the published table (Prout 1910): \
             the same kind in {same} of {} fugues{}",
            kinds.len(),
            if differing.is_empty() {
                String::new()
            } else {
                format!("; differing: {}", differing.join(", "))
            }
        );
        println!();
    }
    if book != Some(2) {
        heuristic(data, &pieces)?;
    }
    Ok(())
}

/// The heuristic for the subject's length against the Algomus labels on WTC I, and what the
/// comparison of the answers would say with it.
fn heuristic(data: &Path, pieces: &[crate::kern::KernPiece]) -> Result<()> {
    println!("The heuristic for the subject's length on WTC I (labels against entries):");
    let mut same = 0;
    let mut counted = 0;
    let mut with_labels = Totals::default();
    let mut with_entries = Totals::default();
    for number in 1..=24 {
        let piece = find(pieces, &name(1, number))?;
        let Ok(labels) = algomus::read_for(data, number) else {
            continue;
        };
        let (Some(by_labels), Some(by_entries)) = (
            subject::from_labels(piece, &labels),
            subject::from_entries(piece),
        ) else {
            continue;
        };
        counted += 1;
        let agree = by_labels.subject == by_entries.subject;
        same += agree as usize;
        let a = compare(&by_labels, piece.key).0;
        let b = compare(&by_entries, piece.key).0;
        with_labels.add(&a);
        with_entries.add(&b);
        if !agree {
            println!(
                "  {:8} labels {:2} tones, entries {:2} tones; with the entries: kind {}, tones {}",
                piece.name,
                by_labels.subject.len(),
                by_entries.subject.len(),
                if b.kind_agrees() { "agrees" } else { "differs" },
                if b.all_agree() { "agree" } else { "differ" }
            );
        }
    }
    println!("  same subject in {same} of {counted} fugues");
    with_labels.print("  WTC I with the labels");
    with_entries.print("  WTC I with the heuristic");
    Ok(())
}

/// Prints subject and answer of every fugue of a book tone by tone, with what the labels and
/// the heuristic say about the length.
pub fn explore(data: &Path, book: usize) -> Result<()> {
    let pieces = load(data)?;
    for number in 1..=24 {
        let piece = find(&pieces, &name(book, number))?;
        let labels = if book == 1 {
            algomus::read_for(data, number).ok()
        } else {
            None
        };
        let Some(e) = subject::exposition(piece, labels.as_deref()) else {
            println!("\n{} no exposition found", piece.name);
            continue;
        };
        println!(
            "\n{} {} {}  {} tones, answer enters at bar {} +{}",
            piece.name,
            piece.key,
            piece.meter,
            e.subject.len(),
            piece.bar_of(e.answer[0].on).0,
            piece.bar_of(e.answer[0].on).1
        );
        // Three further tones of both voices show what follows the subject.
        let take = |voice: usize, first: &Note, count: usize| -> Vec<Note> {
            let notes = &piece.voices[voice].notes;
            let start = notes.iter().position(|n| n.on == first.on).unwrap_or(0);
            notes[start..(start + count).min(notes.len())].to_vec()
        };
        let s = take(e.subject_voice, &e.subject[0], e.subject.len() + 3);
        let a = take(e.answer_voice, &e.answer[0], e.answer.len() + 3);
        println!("  S v{}: {}", e.subject_voice, line(piece.key, &s));
        println!("  A v{}: {}", e.answer_voice, line(piece.key, &a));
        let marks: String = s
            .iter()
            .zip(&a)
            .map(|(x, y)| region(x.pitch, y.pitch).mark())
            .collect();
        println!("  marks: {marks}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_and_degrees() {
        let c5 = Pitch::new(0, 0, 5);
        assert_eq!(region(c5, Pitch::new(4, 0, 5)), Region::Fifth);
        // A fifth above, octaves aside: the G below.
        assert_eq!(region(c5, Pitch::new(4, 0, 4)), Region::Fifth);
        assert_eq!(region(c5, Pitch::new(3, 0, 5)), Region::Fourth);
        assert_eq!(region(c5, Pitch::new(4, 1, 5)), Region::AlteredFifth);
        assert_eq!(region(c5, Pitch::new(3, 1, 4)), Region::AlteredFourth);
        assert_eq!(region(c5, Pitch::new(2, 0, 5)), Region::Other);
        let key = Key::parse("c").unwrap();
        assert_eq!(degree_name(key, Pitch::new(6, 0, 4)), "#7");
        assert_eq!(degree_name(key, Pitch::new(4, 0, 4)), "5");
        assert_eq!(degree_name(key, Pitch::new(1, -1, 4)), "b2");
    }

    #[test]
    fn csv_lines() {
        assert_eq!(
            csv_fields(r#"1,2,847,C minor,"p. 14: the answer, which is ""tonal""",x"#),
            [
                "1",
                "2",
                "847",
                "C minor",
                "p. 14: the answer, which is \"tonal\"",
                "x"
            ]
        );
    }

    /// The counts of `docs/design/bach-fugen/machbarkeit/laeufe/t5-antworten.txt`: per book
    /// Bach's real and tonal answers, the fugues where the rule gives the same kind and those
    /// where it gives every tone. Without the data nothing is checked.
    #[test]
    fn the_48_answers() {
        let data =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../references/datasets/bach-fugen");
        let Ok(pieces) = load(&data) else {
            return;
        };
        if pieces.len() != 48 {
            return;
        }
        let mut found = Vec::new();
        for book in [1, 2] {
            let mut totals = Totals::default();
            for number in 1..=24 {
                let piece = find(&pieces, &name(book, number)).unwrap();
                let labels = if book == 1 {
                    algomus::read_for(&data, number).ok()
                } else {
                    None
                };
                let e = subject::exposition(piece, labels.as_deref()).unwrap();
                totals.add(&compare(&e, piece.key).0);
            }
            found.push((totals.real, totals.tonal, totals.kind, totals.all));
        }
        assert_eq!(found, [(9, 15, 21, 21), (11, 13, 23, 17)]);
    }
}
