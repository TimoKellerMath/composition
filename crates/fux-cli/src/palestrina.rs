//! Palestrina's masses as a corpus (references/datasets/palestrina-masses): listing them and
//! counting how often each rule fires in them, as `fux rules --corpus` does for Fux.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use fux_core::Lang;
use fux_core::kern::{KernPiece, parse_kern};
use fux_core::rules::{Checker, Level, Profile, all_rules};

/// Where the corpus lies: `$FUX_PALESTRINA`, else in the repository the program was built from.
pub fn corpus_dir() -> PathBuf {
    std::env::var_os("FUX_PALESTRINA").map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../references/datasets/palestrina-masses/kern")
        },
        PathBuf::from,
    )
}

/// Every movement of the corpus that reads, by file name.
pub fn read_corpus(lang: Lang) -> Result<Vec<(String, KernPiece)>> {
    let dir = corpus_dir();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .with_context(|| {
            format!(
                "{}: {} (python3 tools/fetch_palestrina.py, {} FUX_PALESTRINA)",
                dir.display(),
                lang.pick("nicht lesbar", "not readable"),
                lang.pick("oder", "or")
            )
        })?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "krn"))
        .collect();
    files.sort();
    let mut pieces = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path)?;
        if let Ok(piece) = parse_kern(&text)
            && piece.score.validate().is_ok()
        {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            pieces.push((name.into_owned(), piece));
        }
    }
    Ok(pieces)
}

/// The movements the calibration uses: in 4/2 with three to six voices (docs/regeln.md § 16).
pub fn calibration_set(pieces: Vec<(String, KernPiece)>) -> Vec<(String, KernPiece)> {
    pieces
        .into_iter()
        .filter(|(_, p)| p.written_meter.numerator == 4 && (3..=6).contains(&p.score.voices.len()))
        .collect()
}

/// `fux corpus --palestrina`: the movements and sections of movements with mass, movement,
/// section, voices, mode and measures.
pub fn print_corpus(out: &mut impl Write, lang: Lang) -> Result<()> {
    writeln!(
        out,
        "{:<22} {:<38} {:<16} {:<20} {:>7} {:<26} {:>6}",
        lang.pick("Datei", "File"),
        lang.pick("Messe", "Mass"),
        lang.pick("Satz", "Movement"),
        lang.pick("Abschnitt", "Section"),
        lang.pick("Stimmen", "Voices"),
        lang.pick("Modus", "Mode"),
        lang.pick("Takte", "Bars")
    )?;
    for (name, piece) in read_corpus(lang)? {
        let movement = piece.movement.as_deref().unwrap_or("");
        let section = piece.section.as_deref().unwrap_or("");
        let mass: String = piece
            .work
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(38)
            .collect();
        writeln!(
            out,
            "{name:<22} {mass:<38} {movement:<16} {section:<20} {:>7} {:<26} {:>6}",
            piece.score.voices.len(),
            piece.score.mode.name(lang),
            piece.score.measure_count()
        )?;
    }
    Ok(())
}

/// How often each rule fires in the calibration set: (all, prohibitions).
pub fn counts(profile: &Profile, lang: Lang) -> Result<BTreeMap<&'static str, (usize, usize)>> {
    let checker = Checker::new(profile);
    let mut counts: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    for (_, piece) in calibration_set(read_corpus(lang)?) {
        for v in checker.check(&piece.score).violations {
            let entry = counts.entry(v.rule).or_default();
            entry.0 += 1;
            if v.level == Level::Forbidden {
                entry.1 += 1;
            }
        }
    }
    Ok(counts)
}

/// `fux rules --corpus palestrina`.
pub fn rule_corpus(out: &mut impl Write, profile: &Profile, lang: Lang) -> Result<()> {
    let counts = counts(profile, lang)?;
    writeln!(
        out,
        "{}",
        lang.pick(
            "Verstöße in Palestrinas Messen (Sätze in 4/2, drei bis sechs Stimmen)",
            "Violations in Palestrina's masses (movements in 4/2, three to six voices)"
        )
    )?;
    writeln!(
        out,
        "{:<22} {:>9} {:>9}",
        "",
        lang.pick("gesamt", "total"),
        lang.pick("Verbote", "forbidden")
    )?;
    for rule in all_rules() {
        let id = rule.info().id;
        let (all, hard) = counts.get(id).copied().unwrap_or_default();
        writeln!(out, "{id:<22} {all:>9} {hard:>9}")?;
    }
    Ok(())
}
