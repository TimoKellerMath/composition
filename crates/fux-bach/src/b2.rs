//! The runs of phase B2 (protocol in `docs/design/bach-fugen/b2/protokoll.md`): the tables of
//! the scaffold generator, its scaffolds under Bach's entries, and the counts Z6 to Z9.

use std::path::Path;

use anyhow::{Context, Result};
use rayon::prelude::*;

use crate::algomus::{self, Label};
use crate::check;
use crate::chord;
use crate::key::{Harmony, Key};
use crate::pitch::Pitch;
use crate::roman;
use crate::rules::{Level, Profile, Rule};
use crate::scaffold::{self, Cadence, CadenceKind, Model, Pedal, Plan};
use crate::score::{Note, Piece, Voice};
use crate::study::{analyses, find, load, name};
use crate::time::Tick;

/// The analyses of a directory, by file name.
fn read_analyses(dir: &Path) -> Result<Vec<(String, roman::Analysis)>> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| dir.display().to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let text = String::from_utf8_lossy(&std::fs::read(&file)?).into_owned();
        let stem = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        if let Ok(analysis) = roman::parse(&text) {
            out.push((stem, analysis));
        }
    }
    Ok(out)
}

/// The model counted on the analyses: successions from chorales, preludes and fugues, the places
/// where the chord changes from the keyboard pieces.
pub fn learn(data: &Path) -> Result<(Model, [usize; 3])> {
    let wir = data.join("wir");
    let mut model = Model::default();
    let mut counts = [0; 3];
    for (k, (dir, rhythm)) in [("choraele", false), ("praeludien", true), (".", true)]
        .into_iter()
        .enumerate()
    {
        for (_, analysis) in read_analyses(&wir.join(dir))? {
            model.learn(&analysis, rhythm);
            // The bass from the fugues alone: preludes and chorales lay it otherwise.
            if dir == "." {
                model.bass.learn(&analysis);
            }
            counts[k] += 1;
        }
    }
    Ok((model, counts))
}

/// Counts the model and writes it to the data directory of the crate.
pub fn tables(data: &Path, out: &Path) -> Result<()> {
    let (model, counts) = learn(data)?;
    std::fs::write(out, serde_json::to_string(&model.to_json())? + "\n")
        .with_context(|| out.display().to_string())?;
    println!(
        "{} chorales, {} preludes, {} fugues: {} successions; written to {}",
        counts[0],
        counts[1],
        counts[2],
        model.transitions.size(),
        out.display()
    );
    for (mode, name) in [(false, "major"), (true, "minor")] {
        let vocabulary = model.transitions.vocabulary(mode, 40.0);
        println!("{name}: {} chords seen at least 40 times", vocabulary.len());
    }
    println!(
        "the chord changes, by metric weight of the step (4 bar, 3 half bar, 2 beat, 1 half beat):"
    );
    for weight in (1..=4).rev() {
        let [changed, steps] = model.changes[weight];
        println!(
            "  {weight}: {:5.1} % of {} steps",
            100.0 * changed / steps.max(1.0),
            steps
        );
    }
    Ok(())
}

/// The key a label of Algomus names by its degree in the home key: `V`, `vi`, `III`.
pub fn key_of_degree(text: &str, home: Key) -> Option<Key> {
    let text = text.trim();
    let minor = text.chars().next()?.is_lowercase();
    let degree = match text.to_ascii_uppercase().as_str() {
        "I" => 0,
        "II" => 1,
        "III" => 2,
        "IV" => 3,
        "V" => 4,
        "VI" => 5,
        "VII" => 6,
        _ => return None,
    };
    Some(Key {
        tonic: home.degree(degree),
        minor,
    })
}

/// What the scaffold of a re-setting is made from: Bach's entries, and the cadences and pedal
/// points of the labels. Nothing of his chords.
pub fn plan_of(original: &Piece, labels: &[Label], given: &[Vec<Note>], seed: u64) -> Plan {
    let home = original.key;
    let cadences: Vec<Cadence> = labels
        .iter()
        .filter(|l| l.kind == "Cadence")
        .filter_map(|l| {
            let (degree, kind) = l.tag.split_once(':')?;
            Some(Cadence {
                at: l.start,
                key: key_of_degree(degree, home)?,
                kind: CadenceKind::parse(kind)?,
            })
        })
        .collect();
    let pedals: Vec<Pedal> = labels
        .iter()
        .filter(|l| l.kind == "Pedal")
        .filter_map(|l| {
            let degree = match l.tag.trim() {
                "I" | "Tonic pedal" => 0,
                "V" => 4,
                _ => return None,
            };
            Some(Pedal {
                from: l.start,
                until: l.end.min(original.end),
                key: home,
                degree,
            })
        })
        .collect();
    let keys = scaffold::key_plan(
        &entries(original, labels),
        home,
        original.meter,
        original.origin,
        original.end,
        &cadences,
    );
    Plan {
        key: home,
        meter: original.meter,
        origin: original.origin,
        end: original.end,
        lines: given.to_vec(),
        keys,
        cadences,
        pedals,
        must: Vec::new(),
        picardy: true,
        close: true,
        hold: None,
        lowest: Vec::new(),
        seed,
    }
}

/// Bach's entries of the subject, each with its tones.
pub fn entries(original: &Piece, labels: &[Label]) -> Vec<Vec<Note>> {
    let harmony = Harmony::default();
    crate::b1::draft(original, labels, &harmony)
        .voices
        .iter()
        .flat_map(|v| {
            v.spans.iter().map(|(from, until)| {
                v.given
                    .iter()
                    .filter(|n| n.on >= *from && n.on < *until)
                    .copied()
                    .collect::<Vec<Note>>()
            })
        })
        .collect()
}

/// A scaffold in numbers against the range of Tymoczko's seven analyses.
pub type Range = [(f64, f64); 6];

/// Z8, the reference: the six numbers of each of the seven analysed fugues and their range.
pub fn reference(data: &Path) -> Result<Range> {
    let read = load(data)?;
    let analysed = analyses(data, &read)?;
    let mut range = [(f64::MAX, f64::MIN); 6];
    println!(
        "fugue      {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "major", "minor", "dim.", "dom. 7", "other 7", "changes"
    );
    for (piece, harmony, _) in &analysed {
        let numbers = scaffold::profile(harmony, piece.meter, piece.end);
        println!(
            "{:10} {}",
            piece.name,
            numbers
                .iter()
                .enumerate()
                .map(|(k, n)| if k < 5 {
                    format!("{:8.1}%", 100.0 * n)
                } else {
                    format!("{n:9.2}")
                })
                .collect::<Vec<_>>()
                .join(" ")
        );
        for k in 0..6 {
            range[k] = (range[k].0.min(numbers[k]), range[k].1.max(numbers[k]));
        }
    }
    Ok(range)
}

/// What a scaffold is judged by.
pub struct Judged {
    pub number: usize,
    pub numbers: [f64; 6],
    /// Cadences of the plan, and how many of them the scaffold realises.
    pub cadences: (usize, usize),
    /// Given tones, and the prohibitions of the figures they break against the generated
    /// scaffold, against the estimated chords and (where there is one) against the analysis.
    pub given: usize,
    pub own: [Option<usize>; 3],
}

/// The entries of a fugue as a piece of their own.
fn given_piece(original: &Piece, given: &[Vec<Note>]) -> Piece {
    let voices: Vec<Voice> = given
        .iter()
        .map(|notes| Voice {
            notes: notes.clone(),
        })
        .collect();
    let mut piece = Piece::new(&original.name, original.key, original.meter, voices);
    piece.origin = original.origin;
    piece.end = original.end;
    piece
}

/// The prohibitions about figures that the tones of a piece break against a harmony.
fn figure_faults(piece: &Piece, harmony: &Harmony, profile: &Profile) -> usize {
    check::check(piece, harmony, profile)
        .findings
        .iter()
        .filter(|f| {
            f.level == Level::Forbidden
                && matches!(
                    f.rule,
                    Rule::FigUnresolved | Rule::FigTooLong | Rule::FigBeforeRest
                )
        })
        .count()
}

/// The given tones of a re-setting, voice by voice (as `b1::draft` keeps them).
pub fn given_lines(original: &Piece, labels: &[Label]) -> Vec<Vec<Note>> {
    let harmony = Harmony::default();
    crate::b1::draft(original, labels, &harmony)
        .voices
        .into_iter()
        .map(|v| v.given)
        .collect()
}

/// The generated scaffold of a fugue of WTC I.
pub fn scaffold_for(
    original: &Piece,
    labels: &[Label],
    model: &Model,
    seed: u64,
) -> (Plan, Harmony) {
    let given = given_lines(original, labels);
    let plan = plan_of(original, labels, &given, seed);
    let harmony = scaffold::generate(&plan, model);
    (plan, harmony)
}

/// Z7, Z8 and Z9 on a set of fugues.
pub fn scaffolds(data: &Path, numbers: &[usize], profile: &Profile, detail: bool) -> Result<()> {
    let read = load(data)?;
    let (model, _) = learn(data)?;
    println!("the seven analysed fugues (Tymoczko):");
    let range = reference(data)?;
    let analysed = analyses(data, &read)?;
    let judged: Vec<Result<Judged>> = numbers
        .par_iter()
        .map(|number| {
            let original = find(&read, &name(1, *number))?;
            let labels = algomus::read_for(data, *number)?;
            let (plan, harmony) = scaffold_for(original, &labels, &model, 1);
            let given = given_lines(original, &labels);
            let lines = given_piece(original, &given);
            // Cadences of the labels that lie inside the piece as read (the file of I/20 ends
            // seven bars early).
            let inside: Vec<&Cadence> = plan
                .cadences
                .iter()
                .filter(|c| c.at >= 0 && c.at < original.end)
                .collect();
            let realised = inside
                .iter()
                .filter(|c| {
                    let (key, goal) = c.goal(plan.key);
                    let dominant = harmony.chord_at(c.at - 1).is_some_and(|b| {
                        scaffold::is_dominant(crate::harmonize::Rel::of(b, key), key)
                    });
                    harmony
                        .chord_at(c.at)
                        .is_some_and(|x| x.mask() == goal.mask())
                        && (dominant != (c.kind == CadenceKind::Half))
                })
                .count();
            let tymoczko = analysed
                .iter()
                .find(|(piece, _, _)| piece.name == original.name)
                .map(|(_, harmony, _)| figure_faults(&lines, harmony, profile));
            if detail {
                println!("== I/{number}: keys and chords of the scaffold");
                let mut line = String::new();
                let mut key: Option<Key> = None;
                for span in &harmony.spans {
                    let (bar, pos) = original.bar_of(span.on);
                    if key != Some(span.key) {
                        line += &format!(" [{}]", span.key);
                        key = Some(span.key);
                    }
                    if pos == 0 {
                        line += &format!(" |{bar}");
                    }
                    line += &format!(" {}", span.chord);
                }
                println!("{line}");
            }
            Ok(Judged {
                number: *number,
                numbers: scaffold::profile(&harmony, original.meter, original.end),
                cadences: (inside.len(), realised),
                given: lines.note_count(),
                own: [
                    Some(figure_faults(&lines, &harmony, profile)),
                    Some(figure_faults(&lines, &chord::estimate(original), profile)),
                    tymoczko,
                ],
            })
        })
        .collect();
    println!("the generated scaffolds; * outside the range of the seven analyses:");
    println!(
        "fugue      {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}   cadences  given tones  faults: scaffold  estimate  Tymoczko",
        "major", "minor", "dim.", "dom. 7", "other 7", "changes"
    );
    let mut inside = [0usize; 6];
    let mut cadences = (0, 0);
    let mut given = 0;
    let mut own = [0usize; 2];
    let mut reference = (0usize, 0usize);
    let mut count = 0;
    for result in &judged {
        let j = match result {
            Ok(j) => j,
            Err(error) => {
                println!("{error}");
                continue;
            }
        };
        count += 1;
        let cells: Vec<String> = (0..6)
            .map(|k| {
                let ok = j.numbers[k] >= range[k].0 - 1e-9 && j.numbers[k] <= range[k].1 + 1e-9;
                inside[k] += usize::from(ok);
                let text = if k < 5 {
                    format!("{:7.1}%", 100.0 * j.numbers[k])
                } else {
                    format!("{:8.2}", j.numbers[k])
                };
                format!("{text}{}", if ok { " " } else { "*" })
            })
            .collect();
        cadences.0 += j.cadences.0;
        cadences.1 += j.cadences.1;
        given += j.given;
        own[0] += j.own[0].unwrap_or(0);
        own[1] += j.own[1].unwrap_or(0);
        if let Some(t) = j.own[2] {
            reference.0 += t;
            reference.1 += j.given;
        }
        println!(
            "I/{:<8} {}   {:2} of {:2} {:12} {:17} {:9} {:>9}",
            j.number,
            cells.join(" "),
            j.cadences.1,
            j.cadences.0,
            j.given,
            j.own[0].unwrap_or(0),
            j.own[1].unwrap_or(0),
            j.own[2].map_or("-".to_string(), |t| t.to_string())
        );
    }
    println!(
        "range      {}",
        (0..6)
            .map(|k| if k < 5 {
                format!("{:4.1}–{:4.1}%", 100.0 * range[k].0, 100.0 * range[k].1)
            } else {
                format!("{:4.2}–{:4.2}", range[k].0, range[k].1)
            })
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!(
        "Z8, scaffolds inside the range, of {count}: {}",
        (0..6)
            .map(|k| format!("{} {}", scaffold::PROFILE_NAMES[k], inside[k]))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("Z9, cadences realised: {} of {}", cadences.1, cadences.0);
    let share = |faults: usize, tones: usize| 100.0 * faults as f64 / tones.max(1) as f64;
    println!(
        "Z7, prohibitions of the figures broken by the given lines, per given tone: against the scaffold {:.2} % ({} of {}), against the estimated chords {:.2} % ({})",
        share(own[0], given),
        own[0],
        given,
        share(own[1], given),
        own[1]
    );
    // The reference of Z7: the entries of I/19 and I/22 against Tymoczko's chords.
    let mut faults = 0;
    let mut tones = 0;
    for number in [19, 22] {
        let original = find(&read, &name(1, number))?;
        let labels = algomus::read_for(data, number)?;
        let lines = given_piece(original, &given_lines(original, &labels));
        if let Some((_, harmony, _)) = analysed.iter().find(|(p, _, _)| p.name == original.name) {
            faults += figure_faults(&lines, harmony, profile);
            tones += lines.note_count();
        }
    }
    println!(
        "Z7, reference: the entries of I/19 and I/22 against Tymoczko's chords {:.2} % ({faults} of {tones})",
        share(faults, tones)
    );
    let _ = reference;
    Ok(())
}

/// The tones of the pedal points of a plan as given tones of the lowest voice, where it has no
/// entry: (voice, tone).
pub fn pedal_tones(plan: &Plan, original: &Piece, given: &[Vec<Note>]) -> Vec<Note> {
    let Some(lowest) = original.voices.len().checked_sub(1) else {
        return Vec::new();
    };
    let low = original.voices[lowest]
        .notes
        .iter()
        .map(|n| n.pitch)
        .min()
        .unwrap_or(Pitch::new(0, 0, 2));
    let first: Tick = original.voices[lowest]
        .notes
        .first()
        .map_or(original.end, |n| n.on);
    plan.pedals
        .iter()
        .filter(|p| p.from >= first)
        .filter(|p| {
            // Room for the tone: no entry of the voice within a beat of it.
            let beat = original.meter.beat();
            given[lowest]
                .iter()
                .all(|n| n.off + beat <= p.from || n.on >= p.until + beat)
        })
        .map(|p| Note {
            on: p.from,
            off: p.until,
            pitch: p.tone(low),
        })
        .collect()
}

/// The chords of the model by how often they were seen, for each mode.
pub fn vocabulary(data: &Path) -> Result<()> {
    let (model, _) = learn(data)?;
    let (_, firsts) = model.transitions.rows();
    for mode in 0..2 {
        let mut rows: Vec<&[f64; 5]> = firsts.iter().filter(|r| r[0] as usize == mode).collect();
        rows.sort_by(|a, b| b[4].total_cmp(&a[4]));
        let key = Key::parse(if mode == 0 { "C" } else { "a" }).context("key")?;
        let names: Vec<String> = rows
            .iter()
            .take(45)
            .map(|r| {
                let rel = crate::harmonize::Rel::new(
                    r[1] as u8,
                    r[2] as i8,
                    crate::key::Kind::ALL[r[3] as usize],
                );
                format!("{} {}", rel.in_key(key), r[4])
            })
            .collect();
        println!(
            "{}: {}",
            if mode == 0 { "C major" } else { "a minor" },
            names.join(", ")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> Option<std::path::PathBuf> {
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../references/datasets/bach-fugen");
        dir.join("wir/choraele").is_dir().then_some(dir)
    }

    /// The model that comes with the program is the one counted on the analyses.
    #[test]
    fn the_embedded_model_is_the_counted_one() {
        let Some(data) = data() else {
            return;
        };
        let (model, counts) = learn(&data).unwrap();
        assert_eq!(counts, [371, 23, 7]);
        let embedded = scaffold::embedded();
        assert_eq!(embedded.changes, model.changes);
        assert_eq!(embedded.transitions.rows(), model.transitions.rows());
    }

    /// Z9 on one fugue: every cadence of the labels stands in the scaffold, and the scaffold is
    /// made without Bach's chords.
    #[test]
    fn a_scaffold_realises_its_cadences() {
        let Some(data) = data() else {
            return;
        };
        let read = load(&data).unwrap();
        let original = find(&read, &name(1, 2)).unwrap();
        let labels = algomus::read_for(&data, 2).unwrap();
        let (plan, harmony) = scaffold_for(original, &labels, scaffold::embedded(), 1);
        assert_eq!(plan.cadences.len(), 4);
        for cadence in &plan.cadences {
            let (_, goal) = cadence.goal(plan.key);
            assert_eq!(
                harmony.chord_at(cadence.at).map(|c| c.mask()),
                Some(goal.mask()),
                "{cadence:?}"
            );
        }
        // The answer of I/2 stands in G minor.
        assert_eq!(harmony.key_at(original.tick_of(3, 48)), Key::parse("g"));
        assert_eq!(plan.pedals.len(), 1);
    }
}
