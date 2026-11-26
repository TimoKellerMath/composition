//! Phase B7: the bass. Which tone of its chord sounds lowest where a chord begins, in Bach's
//! analysed fugues and in the program's, counted alike; and what the analyses say of the bass
//! of a chord and of the way from one bass to the next.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::bass::{Lowest, lowest};
use crate::key::Kind;
use crate::roman;

/// One line of the table.
fn line(low: &Lowest, name: &str) -> String {
    let share = |n: usize, of: usize| {
        if of == 0 {
            "     -".to_string()
        } else {
            format!("{:5.1}%", 100.0 * n as f64 / of as f64)
        }
    };
    format!(
        "{name:16} {:>6}  {} {} {} {} {}   {:>6} {}",
        low.onsets,
        share(low.members[0], low.onsets),
        share(low.members[1], low.onsets),
        share(low.members[2], low.onsets),
        share(low.members[3], low.onsets),
        share(low.members[4], low.onsets),
        low.named,
        share(low.met, low.named),
    )
}

const HEAD: &str = "                 onsets    root  third  fifth seventh  other    named    met";

/// What the analyses of a corpus say of the bass.
#[derive(Clone, Default, Debug)]
pub struct Figures {
    pub labels: usize,
    /// By kind of chord: how often root, third, fifth and seventh are the bass.
    pub by_kind: BTreeMap<String, [usize; 4]>,
    /// From one bass to the next, in semitones upwards (0 to 11).
    pub ways: [usize; 12],
    /// The same after a seventh in the bass.
    pub after_seventh: [usize; 12],
}

/// The group of a kind of chord in the tables.
pub fn group(kind: Kind) -> &'static str {
    match kind {
        Kind::Maj => "major triad",
        Kind::Min => "minor triad",
        Kind::Dim | Kind::Aug => "dim./aug. triad",
        Kind::Dom7 | Kind::Dom9 | Kind::DomMin9 => "dominant seventh",
        Kind::HalfDim7 | Kind::Dim7 => "(half-)dim. seventh",
        _ => "other seventh",
    }
}

impl Figures {
    pub fn learn(&mut self, analysis: &roman::Analysis) {
        let mut before: Option<(u8, usize)> = None;
        for label in &analysis.labels {
            let Some(chord) = roman::chord(&label.figure, label.key) else {
                before = None;
                continue;
            };
            let Some(bass) = chord.bass else {
                before = None;
                continue;
            };
            let Some(member) = chord.tones().position(|t| t == bass).filter(|m| *m < 4) else {
                before = None;
                continue;
            };
            self.labels += 1;
            self.by_kind
                .entry(group(chord.kind).to_string())
                .or_default()[member] += 1;
            if let Some((pc, last)) = before {
                let way = (bass.pc() + 12 - pc) % 12;
                self.ways[way as usize] += 1;
                if last == 3 {
                    self.after_seventh[way as usize] += 1;
                }
            }
            before = Some((bass.pc(), member));
        }
    }

    fn print(&self, name: &str) {
        println!("{name}: {} chords", self.labels);
        let mut all = [0usize; 4];
        for (kind, counts) in &self.by_kind {
            let n: usize = counts.iter().sum();
            for (a, c) in all.iter_mut().zip(counts) {
                *a += c;
            }
            println!(
                "  {kind:22} {n:>6}  root {:5.1}%  third {:5.1}%  fifth {:5.1}%  seventh {:5.1}%",
                100.0 * counts[0] as f64 / n as f64,
                100.0 * counts[1] as f64 / n as f64,
                100.0 * counts[2] as f64 / n as f64,
                100.0 * counts[3] as f64 / n as f64,
            );
        }
        let n: usize = all.iter().sum();
        println!(
            "  {:22} {n:>6}  root {:5.1}%  third {:5.1}%  fifth {:5.1}%  seventh {:5.1}%",
            "all",
            100.0 * all[0] as f64 / n as f64,
            100.0 * all[1] as f64 / n as f64,
            100.0 * all[2] as f64 / n as f64,
            100.0 * all[3] as f64 / n as f64,
        );
        let line = |ways: &[usize; 12]| -> String {
            let n: usize = ways.iter().sum();
            if n == 0 {
                return "none".to_string();
            }
            let share = |k: &[usize]| -> f64 {
                100.0 * k.iter().map(|i| ways[*i]).sum::<usize>() as f64 / n as f64
            };
            format!(
                "{n}: same {:.1}%, step up {:.1}%, step down {:.1}%, third up {:.1}%, third down {:.1}%, fourth up {:.1}%, fourth down {:.1}%, tritone {:.1}%",
                share(&[0]),
                share(&[1, 2]),
                share(&[10, 11]),
                share(&[3, 4]),
                share(&[8, 9]),
                share(&[5]),
                share(&[7]),
                share(&[6]),
            )
        };
        println!("  from one bass to the next, {}", line(&self.ways));
        println!(
            "  after a seventh in the bass, {}",
            line(&self.after_seventh)
        );
    }
}

/// The analyses of a directory.
fn analyses_in(dir: &Path) -> Result<Vec<roman::Analysis>> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| dir.display().to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let text = String::from_utf8_lossy(&std::fs::read(&file)?).into_owned();
        if let Ok(analysis) = roman::parse(&text) {
            out.push(analysis);
        }
    }
    Ok(out)
}

/// What the three corpora of analyses say of the bass.
pub fn figures(data: &Path) -> Result<[(&'static str, Figures); 3]> {
    let wir = data.join("wir");
    let mut out = [
        ("the seven fugues", Figures::default()),
        ("the 24 preludes of WTC I", Figures::default()),
        ("the 371 chorales", Figures::default()),
    ];
    for (k, dir) in [".", "praeludien", "choraele"].into_iter().enumerate() {
        for analysis in analyses_in(&wir.join(dir))? {
            out[k].1.learn(&analysis);
        }
    }
    Ok(out)
}

/// Bach alone: the bass in the analyses, and the lowest tone of the seven analysed fugues
/// where a chord of the analysis begins.
pub fn bach(data: &Path) -> Result<()> {
    println!("The bass in the analyses of When in Rome (figures of the analysts):");
    for (name, figures) in figures(data)? {
        figures.print(name);
    }
    println!();
    println!(
        "The lowest sounding tone where a chord of Tymoczko's analysis begins (two voices and more):"
    );
    println!("{HEAD}");
    let read = crate::study::load(data)?;
    let mut total = Lowest::default();
    for (piece, harmony, _) in crate::study::analyses(data, &read)? {
        let count = lowest(&piece, &harmony);
        println!("{}", line(&count, &piece.name));
        total.add(&count);
    }
    println!("{}", line(&total, "all seven"));
    Ok(())
}

/// The program: the lowest tone where a chord of the scaffold begins, fugue by fugue.
pub fn run(
    data: &Path,
    set: &str,
    voices: &[usize],
    options: &crate::compose::Options,
) -> Result<()> {
    let subjects = match set {
        "fresh" | "frisch" => crate::b4::fresh(data)?,
        other => crate::b3::subjects(other)?,
    };
    let (results, mark) = crate::b3::compose_all(data, &subjects, voices, &[1], options)?;
    println!(
        "Programm: Menge {set} (Profil {}, Beam {})",
        options.profile.name, options.beam
    );
    println!("{HEAD}   K1 K2 K3 K4 K5");
    let mut total = Lowest::default();
    let mut by_voices: BTreeMap<usize, Lowest> = BTreeMap::new();
    let mut k = [0usize; 5];
    let mut made = 0;
    for (index, count, _, result) in &results {
        let name = format!("{} {count}", subjects[*index].name);
        match result {
            Ok((fugue, verdict)) => {
                let low = lowest(&fugue.piece, &fugue.harmony);
                let marks = [
                    verdict.k1(),
                    verdict.k2(),
                    verdict.k3(),
                    verdict.k4(mark),
                    verdict.k5(),
                ];
                for (sum, ok) in k.iter_mut().zip(marks) {
                    *sum += ok as usize;
                }
                made += 1;
                println!(
                    "{}   {}",
                    line(&low, &name),
                    marks.map(|ok| if ok { "+ " } else { "- " }).concat()
                );
                total.add(&low);
                by_voices.entry(*count).or_default().add(&low);
            }
            Err(error) => println!("{name:16} keine Fuge: {error:#}"),
        }
    }
    for (count, low) in &by_voices {
        println!("{}", line(low, &format!("{count} Stimmen")));
    }
    println!("{}", line(&total, "alle"));
    println!(
        "{made} Fugen von {}: K1 {} K2 {} K3 {} K4 {} K5 {}",
        results.len(),
        k[0],
        k[1],
        k[2],
        k[3],
        k[4]
    );
    Ok(())
}
