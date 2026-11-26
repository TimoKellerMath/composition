//! The runs of phase B4 (protocol in `docs/design/bach-fugen/b4/protokoll.md`): the exposed
//! moments of Bach's labelled fugues and of the program's, counted alike.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::algomus::Label;
use crate::compose::Fugue;
use crate::moments::{self, Count, Marks, Tally};
use crate::scaffold::CadenceKind;
use crate::score::Piece;
use crate::themes::{self, Subject};
use crate::time::Tick;

/// The moments of one of Bach's fugues from its labels, and the number of cadences of each
/// kind. A label `S` ends where the last tone of the subject begins.
pub fn bach_marks(piece: &Piece, labels: &[Label]) -> (Marks, BTreeMap<String, usize>) {
    let mut marks = Marks::default();
    let mut kinds = BTreeMap::new();
    let end_of = |label: &Label| -> Tick {
        label
            .voice
            .and_then(|v| piece.voices.get(v))
            .and_then(|voice| voice.notes.iter().find(|n| n.on == label.end))
            .map_or(label.end, |n| n.off)
    };
    for label in labels {
        if label.kind == "Cadence" {
            let Some((degree, kind)) = label.tag.split_once(':') else {
                continue;
            };
            let Some(kind) = CadenceKind::parse(kind) else {
                continue;
            };
            *kinds.entry(kind.label().to_string()).or_insert(0) += 1;
            marks.arrivals.push(label.start);
            if matches!(kind, CadenceKind::Perfect | CadenceKind::RootImperfect)
                && let Some(key) = crate::b2::key_of_degree(degree, piece.key)
            {
                marks.cadences.push((label.start, key));
            }
        } else if label.kind.starts_with('S') && label.kind != "Structure" {
            marks.subject.push((label.start, end_of(label)));
            if label.is_subject()
                && let Some(voice) = label.voice
                && let Some(first) = label.notes(piece).first()
            {
                marks.entries.push((voice, first.on, end_of(label)));
            }
        }
    }
    (marks, kinds)
}

/// The moments of a fugue of the program from its plan: every cadence of the plan, every entry
/// from its first tone to its last. With `close` the final chord counts as a cadence, as the
/// protocol first had it: it stands over a pedal point that began with the cadence before, and
/// Bach's labels name that cadence, not the last chord.
pub fn fugue_marks(subject: &Subject, fugue: &Fugue, close: bool) -> Marks {
    let design = &fugue.design;
    let first = subject.notes.first().map_or(0, |n| n.on);
    let last = subject.notes.last().map_or(subject.period, |n| n.off);
    let mut marks = Marks::default();
    for cadence in &design.cadences {
        marks.arrivals.push(cadence.at);
        if matches!(
            cadence.kind,
            CadenceKind::Perfect | CadenceKind::RootImperfect
        ) {
            marks.cadences.push((cadence.at, cadence.key));
        }
    }
    if close {
        marks.arrivals.push(design.final_hold);
        marks.cadences.push((design.final_hold, subject.key));
    }
    for entry in &design.entries {
        marks
            .entries
            .push((entry.voice, entry.at + first, entry.at + last));
        marks.subject.push((entry.at + first, entry.at + last));
    }
    marks
}

/// The numbers of the fugues of WTC I whose subjects are fresh: labelled, and not among the
/// twenty subjects that come with the program.
pub const FRESH: [usize; 17] = [3, 4, 5, 6, 8, 9, 10, 12, 13, 14, 17, 18, 19, 20, 22, 23, 24];

/// The fresh subjects, as the first label `S` of their fugue bounds them.
pub fn fresh(data: &Path) -> Result<Vec<Subject>> {
    let read = crate::study::load(data)?;
    let pieces: Vec<Piece> = read.iter().map(|r| r.piece.clone()).collect();
    FRESH
        .iter()
        .map(|number| {
            let name: &'static str = Box::leak(format!("wtk1-{number:02}").into_boxed_str());
            let entry = themes::Entry {
                name,
                title: name,
                source: themes::Source::Wtc(1, *number),
                development: false,
            };
            themes::subject(&entry, &pieces, data)
        })
        .collect()
}

/// The tallies of a set of fugues by name.
pub type Rows = Vec<(String, Tally)>;

/// Where the cadences of a set stand (`moments::PLACES`), and for those outside the entries the
/// bars to the next entry.
#[derive(Clone, Debug, Default)]
pub struct Places {
    pub places: [u32; 4],
    pub ahead: Vec<i32>,
    /// The bars of the pieces read.
    pub bars: f64,
    /// `moments::doublings`.
    pub doublings: [u32; 4],
}

impl Places {
    fn read(&mut self, piece: &Piece, marks: &Marks) {
        let found = moments::places(piece, marks, &mut self.ahead);
        for k in 0..4 {
            self.places[k] += found[k];
        }
        self.bars += (piece.end - piece.origin) as f64 / piece.meter.bar() as f64;
        let found = moments::doublings(piece, marks);
        for k in 0..4 {
            self.doublings[k] += found[k];
        }
    }

    fn doubled(&self) -> String {
        let [all, bass, silent, first] = self.doublings;
        format!(
            "{all} Einsätze, davon verdoppelt der Bass {bass}, verstummt die Stimme binnen eines Takts {silent}, ist es der erste Einsatz der Stimme {first}"
        )
    }

    fn line(&self) -> String {
        let all: u32 = self.places.iter().sum();
        let mut parts: Vec<String> = moments::PLACES
            .iter()
            .zip(self.places)
            .map(|(name, n)| format!("{name} {n} ({:.0} %)", 100.0 * n as f64 / all.max(1) as f64))
            .collect();
        let near = self.ahead.iter().filter(|b| **b < 1).count();
        let next = self.ahead.iter().filter(|b| **b == 1).count();
        parts.push(format!(
            "von denen außerhalb bis zum nächsten Einsatz: unter einem Takt {near}, ein bis zwei Takte {next}, mehr {}, kein Einsatz mehr {}",
            self.ahead.len() - near - next,
            self.places[3] as usize - self.ahead.len()
        ));
        parts.push(format!(
            "je 100 Takte {:.1} ({all} in {:.0} Takten)",
            100.0 * all as f64 / self.bars.max(1.0),
            self.bars
        ));
        parts.join(", ")
    }
}

/// Bach's side: every labelled fugue of WTC I with its tally, the cadences by kind, and where
/// the cadences in root position stand.
pub fn bach(data: &Path) -> Result<(Rows, BTreeMap<String, usize>, Places)> {
    let read = crate::study::load(data)?;
    let mut rows = Vec::new();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut places = Places::default();
    for number in 1..=24usize {
        let Ok(labels) = crate::algomus::read_for(data, number) else {
            continue;
        };
        let name = crate::study::name(1, number);
        let piece = crate::study::find(&read, &name)?;
        let (marks, found) = bach_marks(piece, &labels);
        for (kind, n) in found {
            *kinds.entry(kind).or_insert(0) += n;
        }
        places.read(piece, &marks);
        rows.push((name, moments::count(piece, &marks)));
    }
    Ok((rows, kinds, places))
}

fn sum<'a>(rows: impl Iterator<Item = &'a Tally>) -> Tally {
    let mut all = Tally::default();
    for row in rows {
        all.add(row);
    }
    all
}

/// The counts of the protocol for Bach and for the program's fugues on a set of subjects, side
/// by side, with K1 to K5 of the fugues.
pub fn run(
    data: &Path,
    set: &str,
    voices: &[usize],
    options: &crate::compose::Options,
    close: bool,
    detail: bool,
) -> Result<()> {
    let (bach_rows, kinds, bach_places) = bach(data)?;
    let mut places = Places::default();
    let bach_all = sum(bach_rows.iter().map(|r| &r.1));
    let subjects = match set {
        "fresh" | "frisch" => fresh(data)?,
        other => crate::b3::subjects(other)?,
    };
    let (results, mark) = crate::b3::compose_all(data, &subjects, voices, &[1], options)?;
    let mut rows: Rows = Vec::new();
    let mut ks = [0usize; 5];
    let mut whole = 0;
    let mut failed = Vec::new();
    for (s, v, _, outcome) in &results {
        let subject = &subjects[*s];
        match outcome {
            Err(error) => failed.push(format!("{} {v}: {error:#}", subject.name)),
            Ok((fugue, verdict)) => {
                let flags = [
                    verdict.k1(),
                    verdict.k2(),
                    verdict.k3(),
                    verdict.k4(mark),
                    verdict.k5(),
                ];
                for (k, ok) in flags.iter().enumerate() {
                    ks[k] += usize::from(*ok);
                }
                whole += usize::from(flags.iter().all(|k| *k));
                let marks = fugue_marks(subject, fugue, close);
                places.read(&fugue.piece, &marks);
                rows.push((
                    format!("{}-{v}st", subject.name),
                    moments::count(&fugue.piece, &marks),
                ));
            }
        }
    }
    let ours = sum(rows.iter().map(|r| &r.1));
    println!(
        "Bach: {} Fugen von WTK I mit Etiketten; Kadenzen nach Art: {}",
        bach_rows.len(),
        kinds
            .iter()
            .map(|(k, n)| format!("{k} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "Programm: Menge {set}, {} von {} Fugen (Profil {}, Beam {}); der Schlussakkord zählt {}",
        rows.len(),
        results.len(),
        options.profile.name,
        options.beam,
        if close {
            "als Kadenz"
        } else {
            "nicht als Kadenz"
        }
    );
    for line in &failed {
        println!("  keine Fuge: {line}");
    }
    println!(
        "K1 {} K2 {} K3 {} K4 {} K5 {}, alle fünf {} von {}",
        ks[0],
        ks[1],
        ks[2],
        ks[3],
        ks[4],
        whole,
        results.len()
    );
    println!(
        "Stellen: Kadenzen Bach {} Programm {}, Einsätze {} und {}, Enden von Zwischenspielen {} und {}",
        bach_all.moments[0],
        ours.moments[0],
        bach_all.moments[1],
        ours.moments[1],
        bach_all.moments[2],
        ours.moments[2]
    );
    println!();
    println!(
        "{:17} {:>13} {:>7} {:>13} {:>7} {:>7}  {:6}  Aussage",
        "Zählung", "Bach", "", "Programm", "", "Abstand", "Befund"
    );
    let share = |hits: u32, chances: u32| -> String {
        if chances == 0 {
            "-".to_string()
        } else {
            format!("{:5.1} %", 100.0 * hits as f64 / chances as f64)
        }
    };
    let mut findings = 0;
    for count in Count::ALL {
        let k = *count as usize;
        let (a, b) = (
            (bach_all.hits[k], bach_all.chances[k]),
            (ours.hits[k], ours.chances[k]),
        );
        let gap = match (bach_all.share(*count), ours.share(*count)) {
            (Some(x), Some(y)) => format!("{:+6.1}", 100.0 * (y - x)),
            _ => "     -".to_string(),
        };
        let finding = moments::differs(a, b, 20.0);
        findings += usize::from(finding);
        println!(
            "{:17} {:>13} {:>7} {:>13} {:>7} {:>7}  {:6}  {}",
            count.id(),
            format!("{} von {}", a.0, a.1),
            share(a.0, a.1),
            format!("{} von {}", b.0, b.1),
            share(b.0, b.1),
            gap,
            if finding { "ja" } else { "" },
            count.german()
        );
    }
    println!();
    println!("{findings} Befunde von {} Zählungen.", Count::ALL.len());
    println!();
    println!("Wo die Kadenzen in Grundstellung ankommen (Bericht, keine Zählung des Protokolls):");
    println!("  Bach: {}", bach_places.line());
    println!("  Programm: {}", places.line());
    println!("Wer den ersten Ton eines Einsatzes verdoppelt (Bericht):");
    println!("  Bach: {}", bach_places.doubled());
    println!("  Programm: {}", places.doubled());
    if detail {
        println!();
        println!("Stellen je Fuge (Kadenzen, Einsätze, Enden von Zwischenspielen):");
        for (name, tally) in bach_rows.iter().chain(&rows) {
            println!(
                "  {name:14} {:3} {:3} {:3}",
                tally.moments[0], tally.moments[1], tally.moments[2]
            );
        }
    }
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

    /// Bach's labelled cadences in root position have the root in the bass: the labels, the
    /// reader and the count agree on where and in which key a cadence arrives.
    #[test]
    fn bachs_cadences_have_the_root_in_the_bass() {
        let Some(data) = data() else {
            return;
        };
        let (rows, kinds, places) = bach(&data).unwrap();
        assert_eq!(
            places.places.iter().sum::<u32>(),
            sum(rows.iter().map(|r| &r.1)).moments[0]
        );
        assert_eq!(rows.len(), 23);
        let all = sum(rows.iter().map(|r| &r.1));
        let labelled =
            kinds.get("PAC").copied().unwrap_or(0) + kinds.get("rIAC").copied().unwrap_or(0);
        assert!(
            all.moments[0] as usize <= labelled && all.moments[0] >= 80,
            "{:?}",
            all.moments
        );
        assert!(
            all.share(Count::KadBass).unwrap() > 0.9,
            "{:?}",
            all.share(Count::KadBass)
        );
    }

    #[test]
    fn the_fresh_subjects_are_read() {
        let Some(data) = data() else {
            return;
        };
        let subjects = fresh(&data).unwrap();
        assert_eq!(subjects.len(), FRESH.len());
        for subject in &subjects {
            assert!(
                subject.notes.len() >= 4 && subject.period > 0,
                "{}",
                subject.name
            );
            assert!(!themes::BUILTIN.iter().any(|b| b.name == subject.name));
        }
    }
}
