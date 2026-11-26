//! The runs of phase B5 (protocol in `docs/design/bach-fugen/b5/protokoll.md`): motivic work,
//! measured alike on Bach's labelled fugues and on the program's.

use std::path::Path;

use anyhow::Result;

use crate::algomus::Label;
use crate::compose::{Fugue, What};
use crate::motifs::{self, Measures};
use crate::score::{Note, Piece};
use crate::stats::quantile;
use crate::themes::Subject;
use crate::time::Tick;

/// Where a label ends in its voice: a label of Algomus ends where its last tone begins.
fn end_of(piece: &Piece, label: &Label) -> Tick {
    label
        .voice
        .and_then(|v| piece.voices.get(v))
        .and_then(|voice| voice.notes.iter().find(|n| n.on == label.end))
        .map_or(label.end, |n| n.off)
}

/// The spans of each voice of one of Bach's fugues that a label marks as subject or
/// countersubject, in any form: what is not free.
pub fn bach_bound(piece: &Piece, labels: &[Label]) -> Vec<Vec<(Tick, Tick)>> {
    let mut bound = vec![Vec::new(); piece.voices.len()];
    for label in labels {
        let thematic = (label.kind.starts_with('S') && label.kind != "Structure")
            || label.kind.starts_with("CS");
        if let (true, Some(voice)) = (thematic, label.voice)
            && voice < bound.len()
        {
            bound[voice].push((label.start, end_of(piece, label)));
        }
    }
    bound
}

/// The material of one of Bach's fugues: its subject as the first label `S` bounds it, and its
/// first countersubject as the first label `CS1` does.
pub fn bach_material(piece: &Piece, labels: &[Label]) -> Vec<Vec<Note>> {
    let mut lines = Vec::new();
    if let Some(exposition) = crate::subject::from_labels(piece, labels) {
        lines.push(exposition.subject);
    }
    if let Some(label) = labels.iter().find(|l| l.kind == "CS1" && l.voice.is_some()) {
        let mut notes = label.notes(piece).to_vec();
        if let Some(last) = label
            .voice
            .and_then(|v| piece.voices.get(v))
            .and_then(|voice| voice.notes.iter().find(|n| n.on == label.end))
        {
            notes.push(*last);
        }
        if notes.len() >= 2 {
            lines.push(notes);
        }
    }
    lines
}

/// One of Bach's fugues measured: with its own material, and, as a control, with the material
/// of the fugue after it in the list (figures that are everyone's count there as well).
pub struct BachRow {
    pub name: String,
    pub own: Measures,
    pub other: Measures,
}

pub fn bach(data: &Path, n: usize) -> Result<Vec<BachRow>> {
    let read = crate::study::load(data)?;
    let mut fugues: Vec<(String, &Piece, Vec<Label>)> = Vec::new();
    for number in 1..=24usize {
        let Ok(labels) = crate::algomus::read_for(data, number) else {
            continue;
        };
        let name = crate::study::name(1, number);
        fugues.push((name.clone(), crate::study::find(&read, &name)?, labels));
    }
    let materials: Vec<Vec<Vec<Note>>> = fugues
        .iter()
        .map(|(_, piece, labels)| bach_material(piece, labels))
        .collect();
    let mut rows = Vec::new();
    for (k, (name, piece, labels)) in fugues.iter().enumerate() {
        let bound = bach_bound(piece, labels);
        let free = |v: usize, note: &Note| {
            !bound[v]
                .iter()
                .any(|(from, until)| *from <= note.on && note.on < *until)
        };
        rows.push(BachRow {
            name: name.clone(),
            own: motifs::measure(piece, &free, &materials[k], n),
            other: motifs::measure(piece, &free, &materials[(k + 1) % fugues.len()], n),
        });
    }
    Ok(rows)
}

/// A fugue of the program measured: free is what is neither subject, answer nor
/// countersubject; the material is the subject and the countersubject.
pub fn fugue(subject: &Subject, fugue: &Fugue, n: usize) -> Measures {
    let bound: Vec<(usize, Tick)> = fugue
        .given
        .iter()
        .filter(|g| matches!(g.what, What::Subject | What::Answer | What::Countersubject))
        .flat_map(|g| g.notes.iter().map(move |note| (g.voice, note.on)))
        .collect();
    let free = |v: usize, note: &Note| !bound.contains(&(v, note.on));
    let material = vec![subject.notes.clone(), fugue.countersubject.clone()];
    motifs::measure(&fugue.piece, &free, &material, n)
}

/// The band of a measure over Bach's fugues: 10th percentile, median, 90th percentile.
pub fn band(values: &[f64]) -> (f64, f64, f64) {
    (
        quantile(values, 0.1),
        quantile(values, 0.5),
        quantile(values, 0.9),
    )
}

/// Bach alone: the three shares for figures of four, five and six tones, with the fugue's own
/// material and with another fugue's. Says which length tells the subject's figures from
/// everyone's.
pub fn explore(data: &Path) -> Result<()> {
    for n in 4..=6 {
        let rows = bach(data, n)?;
        println!("Figuren aus {n} Tönen, {} Fugen von WTK I:", rows.len());
        println!(
            "  {:10} {:>6} {:>12} {:>12} {:>10} {:>8}",
            "Fuge", "frei", "Themenbezug", "fremdes Th.", "Wiederkehr", "Sequenz"
        );
        let mut columns: [Vec<f64>; 4] = Default::default();
        let mut above = 0;
        for row in &rows {
            let own = row.own.shares();
            let other = row.other.shares()[0];
            println!(
                "  {:10} {:6} {:11.1}% {:11.1}% {:9.1}% {:7.1}%",
                row.name,
                row.own.free,
                100.0 * own[0],
                100.0 * other,
                100.0 * own[1],
                100.0 * own[2]
            );
            above += usize::from(own[0] > other);
            columns[0].push(own[0]);
            columns[1].push(other);
            columns[2].push(own[1]);
            columns[3].push(own[2]);
        }
        for (name, values) in ["Themenbezug", "fremdes Thema", "Wiederkehr", "Sequenz"]
            .iter()
            .zip(&columns)
        {
            let (low, median, high) = band(values);
            println!(
                "  {name:14} 10. Perzentil {:5.1} %, Median {:5.1} %, 90. Perzentil {:5.1} %",
                100.0 * low,
                100.0 * median,
                100.0 * high
            );
        }
        println!(
            "  Themenbezug über dem des fremden Themas in {above} von {} Fugen",
            rows.len()
        );
        println!();
    }
    Ok(())
}

/// The length of a figure in tones that the protocol counts with.
pub const TONES: usize = 4;

/// Bach's bands of the three shares (10th percentile, first quartile, median, 90th percentile).
pub fn bands(data: &Path) -> Result<[[f64; 4]; 3]> {
    let rows = bach(data, TONES)?;
    let mut out = [[0.0; 4]; 3];
    for (k, band) in out.iter_mut().enumerate() {
        let values: Vec<f64> = rows.iter().map(|r| r.own.shares()[k]).collect();
        *band = [
            quantile(&values, 0.1),
            quantile(&values, 0.25),
            quantile(&values, 0.5),
            quantile(&values, 0.9),
        ];
    }
    Ok(out)
}

/// The three shares of the program's fugues on a set of subjects against Bach's bands, with
/// the conditions M1 to M3 of the protocol and K1 to K5.
pub fn run(
    data: &Path,
    set: &str,
    voices: &[usize],
    options: &crate::compose::Options,
) -> Result<()> {
    let bands = bands(data)?;
    let subjects = match set {
        "fresh" | "frisch" => crate::b4::fresh(data)?,
        other => crate::b3::subjects(other)?,
    };
    let (results, mark) = crate::b3::compose_all(data, &subjects, voices, &[1], options)?;
    println!(
        "Bachs Bänder (Figuren aus {TONES} Tönen, 23 Fugen): Themenbezug {:.1} bis {:.1} % (1. Quartil {:.1}, Median {:.1}), Wiederkehr {:.1} bis {:.1} % (Median {:.1}), Sequenz {:.1} bis {:.1} % (Median {:.1})",
        100.0 * bands[0][0],
        100.0 * bands[0][3],
        100.0 * bands[0][1],
        100.0 * bands[0][2],
        100.0 * bands[1][0],
        100.0 * bands[1][3],
        100.0 * bands[1][2],
        100.0 * bands[2][0],
        100.0 * bands[2][3],
        100.0 * bands[2][2],
    );
    println!(
        "Programm: Menge {set} (Profil {}, Beam {})",
        options.profile.name, options.beam
    );
    println!(
        "{:10} {:>7} {:>6} {:>12} {:>11} {:>8}  K1 K2 K3 K4 K5",
        "Thema", "Stimmen", "frei", "Themenbezug", "Wiederkehr", "Sequenz"
    );
    let inside =
        |k: usize, value: f64| value >= bands[k][0] - 1e-12 && value <= bands[k][3] + 1e-12;
    let mut within = [0usize; 3];
    let mut derived: Vec<f64> = Vec::new();
    let mut all: [Vec<f64>; 3] = Default::default();
    let mut ks = [0usize; 5];
    let mut whole = 0;
    let mut count = 0;
    for (s, v, _, outcome) in &results {
        let subject = &subjects[*s];
        match outcome {
            Err(error) => println!("{:10} {v:7}  keine Fuge: {error:#}", subject.name),
            Ok((made, verdict)) => {
                count += 1;
                let measures = fugue(subject, made, TONES);
                let shares = measures.shares();
                for k in 0..3 {
                    within[k] += usize::from(inside(k, shares[k]));
                    all[k].push(shares[k]);
                }
                derived.push(shares[0]);
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
                let flag = |ok: bool| if ok { " +" } else { " -" };
                let star = |k: usize| if inside(k, shares[k]) { ' ' } else { '*' };
                println!(
                    "{:10} {v:7} {:6} {:10.1} %{} {:9.1} %{} {:6.1} %{} {}{}{}{}{}",
                    subject.name,
                    measures.free,
                    100.0 * shares[0],
                    star(0),
                    100.0 * shares[1],
                    star(1),
                    100.0 * shares[2],
                    star(2),
                    flag(flags[0]),
                    flag(flags[1]),
                    flag(flags[2]),
                    flag(flags[3]),
                    flag(flags[4]),
                );
                if std::env::var_os("FUX_B5_ANATOMY").is_some() {
                    print!("{}", anatomy(subject, made));
                }
            }
        }
    }
    println!("(* außerhalb von Bachs Band)");
    let median = quantile(&derived, 0.5);
    println!(
        "{count} Fugen von {}: Wiederkehr im Band {} (M1), Sequenz im Band {} (M2), Themenbezug im Median {:.1} % (M3: {}), Themenbezug im Band {}",
        results.len(),
        within[1],
        within[2],
        100.0 * median,
        if median >= bands[0][1] - 1e-12 && median <= bands[0][3] + 1e-12 {
            "im Bereich"
        } else {
            "außerhalb"
        },
        within[0]
    );
    println!(
        "Mediane: Themenbezug {:.1} %, Wiederkehr {:.1} %, Sequenz {:.1} %",
        100.0 * quantile(&all[0], 0.5),
        100.0 * quantile(&all[1], 0.5),
        100.0 * quantile(&all[2], 0.5)
    );
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
    Ok(())
}

/// Where the free tones of a fugue of the program come from, and how many of each kind count
/// as recurring and as sequence: a look into what the shares are made of.
pub fn anatomy(subject: &Subject, made: &Fugue) -> String {
    use std::fmt::Write;
    let bound: Vec<(usize, Tick)> = made
        .given
        .iter()
        .filter(|g| matches!(g.what, What::Subject | What::Answer | What::Countersubject))
        .flat_map(|g| g.notes.iter().map(move |note| (g.voice, note.on)))
        .collect();
    let free = |v: usize, note: &Note| !bound.contains(&(v, note.on));
    let material = vec![subject.notes.clone(), made.countersubject.clone()];
    let marks = motifs::marks(&made.piece, &free, &material, TONES);
    let kind = |v: usize, note: &Note| -> &'static str {
        if let Some(group) = made
            .given
            .iter()
            .find(|g| g.voice == v && g.notes.iter().any(|n| n.on == note.on))
        {
            return match group.what {
                What::Motif => "Motiv",
                What::Sequence => "Glied",
                What::Pedal => "Orgelpunkt",
                _ => "Kadenzton",
            };
        }
        let bar = made.piece.meter.bar();
        if made.draft.voices[v]
            .figures
            .iter()
            .any(|f| f.at <= note.on && note.on < f.until)
        {
            "Figur"
        } else if note.on >= made.design.final_hold - 2 * bar {
            "gezogen, Schluss"
        } else if made
            .design
            .episodes
            .iter()
            .any(|(from, until)| *from <= note.on && note.on < *until)
        {
            "gezogen, Zwischenspiel"
        } else {
            "gezogen, sonst"
        }
    };
    let mut rows: std::collections::BTreeMap<&'static str, [usize; 4]> = Default::default();
    for (v, voice) in made.piece.voices.iter().enumerate() {
        for (i, note) in voice.notes.iter().enumerate() {
            if !marks.free[v][i] {
                continue;
            }
            let row = rows.entry(kind(v, note)).or_default();
            row[0] += 1;
            row[1] += usize::from(marks.derived[v][i]);
            row[2] += usize::from(marks.recurring[v][i]);
            row[3] += usize::from(marks.sequence[v][i]);
        }
    }
    let mut out = String::new();
    for (name, row) in rows {
        let _ = writeln!(
            out,
            "    {name:24} {:4} Töne: Themenbezug {:3}, Wiederkehr {:3}, Sequenz {:3}",
            row[0], row[1], row[2], row[3]
        );
    }
    out
}
