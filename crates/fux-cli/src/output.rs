//! Writing LilyPond, PDF, SVG, PNG and MIDI files.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use fux_core::Lang;
use fux_export::{
    BookPart, ClefStyle, Graphic, LilypondOptions, MidiOptions, MusicXmlOptions, Sheet, book,
    document, render, to_midi, to_musicxml,
};

/// Output formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Ly,
    Pdf,
    Svg,
    Png,
    Midi,
    Musicxml,
}

/// Clefs on the staves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Clefs {
    /// Violin-, oktavierter Violin- und Bassschlüssel
    Modern,
    /// C- und F-Schlüssel wie bei Fux
    Original,
}

/// Where and how to write the result.
#[derive(Args, Debug)]
pub struct OutputArgs {
    /// Ausgabepfad ohne Endung; fehlende Ordner werden angelegt
    /// [Standard: out/satz, out/cantus, out/pruefung bzw. out/messe]
    #[arg(short, long, value_name = "PFAD")]
    pub output: Option<PathBuf>,
    /// Formate, kommagetrennt: ly, pdf, svg, png, midi, musicxml [Standard: ly,pdf,midi; beim
    /// Übungsheft ly,pdf; bei der Messe ly,pdf,midi,musicxml]
    #[arg(short, long, value_enum, value_delimiter = ',')]
    pub format: Vec<Format>,
    /// Schlüssel im Notenbild
    #[arg(long, value_enum, default_value_t = Clefs::Modern)]
    pub clefs: Clefs,
    /// Intervallziffern zwischen den Systemen (zweistimmig) bzw. unter dem Bass
    #[arg(long)]
    pub figures: bool,
    /// Tempo in Halben pro Minute
    #[arg(long, default_value_t = 72)]
    pub tempo: u32,
    /// MIDI-Klang nach General MIDI, 1–128 (53 = Choir Aahs, 20 = Kirchenorgel)
    #[arg(long, default_value_t = 53, value_parser = clap::value_parser!(u8).range(1..=128))]
    pub program: u8,
    /// Klammern mit der Nummer des Punktes über den Einsätzen einer Imitation (LilyPond,
    /// MusicXML): an oder aus
    #[arg(
        long,
        value_name = "AN|AUS",
        default_value = "an",
        action = clap::ArgAction::Set,
        value_parser = crate::free::on_off
    )]
    pub marks: bool,
}

impl OutputArgs {
    pub fn lilypond_options(&self, lang: Lang) -> LilypondOptions {
        LilypondOptions {
            clefs: match self.clefs {
                Clefs::Modern => ClefStyle::Modern,
                Clefs::Original => ClefStyle::Original,
            },
            interval_figures: self.figures,
            lang,
            entry_marks: self.marks,
            mensurstriche: false,
        }
    }

    pub fn midi_options(&self) -> MidiOptions {
        MidiOptions {
            tempo: self.tempo,
            program: self.program,
            ..MidiOptions::default()
        }
    }

    /// MusicXML with the clefs and brackets of these arguments, the tempo and program of their
    /// MIDI file and its head in `lang`, without a seed.
    pub fn musicxml_options(&self, lang: Lang) -> MusicXmlOptions {
        MusicXmlOptions {
            clefs: self.lilypond_options(lang).clefs,
            entry_marks: self.marks,
            tempo: self.tempo,
            program: self.program,
            lang,
            seed: None,
        }
    }
}

/// Writes all requested formats for the sheets to `--output` or else `default_base`, and returns
/// the written files.
pub fn write(
    title: Option<&str>,
    sheets: &[Sheet],
    args: &OutputArgs,
    default_base: &str,
    lang: Lang,
) -> Result<Vec<PathBuf>> {
    let source = || document(title, sheets, &args.lilypond_options(lang));
    let formats = [Format::Ly, Format::Pdf, Format::Midi];
    write_source(source, sheets, args, &formats, default_base, lang)
}

/// Writes a book in parts like [`write()`]: LilyPond and its graphics with all parts, MIDI and
/// MusicXML for each sheet that is not an exercise; LilyPond and PDF unless `--format` says
/// otherwise.
pub fn write_book(
    title: &str,
    subtitle: Option<&str>,
    parts: &[BookPart],
    args: &OutputArgs,
    default_base: &str,
    lang: Lang,
) -> Result<Vec<PathBuf>> {
    let played: Vec<Sheet> = parts
        .iter()
        .flat_map(|part| &part.sheets)
        .filter(|sheet| !sheet.exercise)
        .cloned()
        .collect();
    let source = || book(title, subtitle, parts, &args.lilypond_options(lang));
    let formats = [Format::Ly, Format::Pdf];
    write_source(source, &played, args, &formats, default_base, lang)
}

/// The base of the files written for `args`: `--output`, or else `default_base`.
pub fn base_of(args: &OutputArgs, default_base: &str) -> PathBuf {
    args.output
        .clone()
        .unwrap_or_else(|| PathBuf::from(default_base))
}

/// The formats to write for `args`: those of `--format`, or else `default_formats`.
pub fn formats_of<'a>(args: &'a OutputArgs, default_formats: &'a [Format]) -> &'a [Format] {
    if args.format.is_empty() {
        default_formats
    } else {
        &args.format
    }
}

/// „PATH nicht schreibbar“, for an error.
pub fn unwritable(path: &Path, lang: Lang) -> String {
    format!(
        "{} {}",
        path.display(),
        lang.pick("nicht schreibbar", "not writable")
    )
}

/// Writes the LilyPond `source` (made only if needed) to `base`.ly where `formats` ask for it or
/// for a graphic, and renders the graphics they ask for. Returns the files, LilyPond first. Since
/// LilyPond may fail, the other files are written before (D16).
pub fn write_lilypond(
    source: impl FnOnce() -> String,
    base: &Path,
    formats: &[Format],
    lang: Lang,
) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    let graphics: Vec<Graphic> = formats
        .iter()
        .filter_map(|f| match f {
            Format::Pdf => Some(Graphic::Pdf),
            Format::Svg => Some(Graphic::Svg),
            Format::Png => Some(Graphic::Png),
            Format::Ly | Format::Midi | Format::Musicxml => None,
        })
        .collect();
    if formats.contains(&Format::Ly) || !graphics.is_empty() {
        let ly = with_extension(base, "ly");
        std::fs::write(&ly, source()).with_context(|| unwritable(&ly, lang))?;
        written.push(ly.clone());
        for graphic in graphics {
            written.extend(render(&ly, base, graphic)?);
        }
    }
    Ok(written)
}

/// Writes the LilyPond `source` (made only if needed) with its graphics, and MIDI and MusicXML
/// for each sheet, the MusicXML with the seed of the sheet: in the formats of `args`, or else in
/// `default_formats`. Every file is written before LilyPond runs, so that a failure of LilyPond
/// loses none of them. The files come back in the order LilyPond, graphics, MusicXML, MIDI.
fn write_source(
    source: impl FnOnce() -> String,
    sheets: &[Sheet],
    args: &OutputArgs,
    default_formats: &[Format],
    default_base: &str,
    lang: Lang,
) -> Result<Vec<PathBuf>> {
    let formats = formats_of(args, default_formats);
    let unwritable = |path: &Path| unwritable(path, lang);
    let base = &base_of(args, default_base);
    if let Some(dir) = base.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| unwritable(dir))?;
    }
    let mut played = Vec::new();
    if formats.contains(&Format::Musicxml) {
        let options = args.musicxml_options(lang);
        for (i, sheet) in sheets.iter().enumerate() {
            let path = if sheets.len() == 1 {
                with_extension(base, "musicxml")
            } else {
                with_extension(base, &format!("{}.musicxml", i + 1))
            };
            let options = MusicXmlOptions {
                seed: sheet.seed,
                ..options
            };
            std::fs::write(&path, to_musicxml(sheet.score, &options))
                .with_context(|| unwritable(&path))?;
            played.push(path);
        }
    }
    if formats.contains(&Format::Midi) {
        for (i, sheet) in sheets.iter().enumerate() {
            let path = if sheets.len() == 1 {
                with_extension(base, "mid")
            } else {
                with_extension(base, &format!("{}.mid", i + 1))
            };
            std::fs::write(&path, to_midi(sheet.score, &args.midi_options()))
                .with_context(|| unwritable(&path))?;
            played.push(path);
        }
    }
    let mut written = write_lilypond(source, base, formats, lang)?;
    written.extend(played);
    Ok(written)
}

/// `out/satz` + `ly` → `out/satz.ly`; keeps dots in the file name.
pub fn with_extension(base: &Path, extension: &str) -> PathBuf {
    let mut name = base.as_os_str().to_owned();
    name.push(".");
    name.push(extension);
    PathBuf::from(name)
}
