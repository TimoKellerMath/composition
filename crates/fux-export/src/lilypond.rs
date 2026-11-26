//! LilyPond source for scores. The entries of an imitation get brackets above the staff
//! (`Horizontal_bracket_engraver`), the number of its point over the start of each. Every
//! document and book sets its text alike (`LAYOUT`): room for the hyphens, and the text of a
//! voice clear of the numbers and the ficta over the staff below it. A book allows no page with a
//! single system (`BOOK_PAPER`), a document does (`PAPER`), and so does a part of the workbook
//! with a single setting.
//!
//! **The book of a mass** (phase 14, docs/design/phase-14/spec.md § 6; [`mass_book`]) extends the
//! book of parts ([`book`]): a book part per movement under its centred heading, with the lines
//! under it ([`BookPart::intro`]: the intonation of the Gloria and the Credo, remarks for the
//! performers), and a score per section with only the staves of the voices that sing in it,
//! under its title ([`Sheet::piece`]), its measures numbered on through the movement
//! ([`Sheet::first_bar`]), ending in a double bar or, at the end of the movement, a final bar,
//! with a fermata on the last note of every voice ([`Sheet::end`]). The label of a Break stands
//! where its part sings again, over the barline after the measure's rest that opens it; a seam
//! has none. Two voices of a type are T1 and T2 on the systems after the first
//! ([`Sheet::short_names`]). With Mensurstriche ([`LilypondOptions::mensurstriche`]) the barlines
//! stand between the staves only, and notes tied over them are written as one where the rule of
//! [`crate::mensur`] allows, but for a tie into a note with a fermata.

use std::fmt::Write;

use fux_core::mass::{Mass, section_score, voice_names};
use fux_core::{
    Duration, Interval, Lang, Note, Role, Score, SectionKind, Slice, Species, Voice, VoiceType,
};

use crate::lyrics::{Extend, extenders, word_goes_on};
use crate::marks::{Mark, entry_marks};
use crate::mensur::{Mensur, Shown, merge_ties};

/// Clefs on the staves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClefStyle {
    /// Treble, octave treble and bass clef.
    #[default]
    Modern,
    /// The C and F clefs Fux uses, one per voice type.
    Original,
}

/// Color of a marked note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Red,
    Orange,
    Blue,
}

impl Color {
    fn scheme(self) -> &'static str {
        match self {
            Color::Red => "#red",
            Color::Orange => "#(x11-color 'DarkOrange)",
            Color::Blue => "#blue",
        }
    }
}

/// A note to mark, e.g. where the checker found a rule violation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Highlight {
    pub voice: usize,
    /// Index into the voice's notes.
    pub note: usize,
    pub color: Color,
    /// Short text above the note, e.g. a rule id.
    pub label: Option<String>,
}

/// Options for the LilyPond output.
#[derive(Clone, Debug)]
pub struct LilypondOptions {
    pub clefs: ClefStyle,
    /// Interval numbers between the staves (two voices) or under the bass (more voices).
    pub interval_figures: bool,
    /// Language of voice names and texts.
    pub lang: Lang,
    /// A bracket over the head of every planned entry of a score that carries the plan of its
    /// imitation, labelled with the number of its point (on by default).
    pub entry_marks: bool,
    /// Mensurstriche: barlines between the staves only, and notes tied over them written as one
    /// where [`crate::mensur`] allows (off by default; phase 14).
    pub mensurstriche: bool,
}

impl Default for LilypondOptions {
    fn default() -> Self {
        LilypondOptions {
            clefs: ClefStyle::default(),
            interval_figures: false,
            lang: Lang::default(),
            entry_marks: true,
            mensurstriche: false,
        }
    }
}

/// How a score ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SectionEnd {
    /// A final barline, as a single setting ends.
    #[default]
    Plain,
    /// A section of a mass movement that more sections follow: a double barline, and a fermata
    /// on the last note of every voice.
    Section,
    /// The last section of a mass movement: a final barline, and a fermata on the last note of
    /// every voice.
    Movement,
}

/// One score in a document, with marks and remarks printed below it.
#[derive(Clone, Debug)]
pub struct Sheet<'a> {
    pub score: &'a Score,
    pub subtitle: Option<String>,
    pub highlights: Vec<Highlight>,
    pub remarks: Vec<String>,
    /// An exercise: only the cantus firmus is printed, the other staves stay empty.
    pub exercise: bool,
    /// The heading of the score in place of its title, mode and subtitle: the title of a section
    /// of a mass („Christe“).
    pub piece: Option<String>,
    /// The number of its first measure, where a movement of a mass numbers its measures on
    /// through its sections; 1 by default.
    pub first_bar: usize,
    /// The voices whose staves are printed, by their indices in the score, from the top; `None`
    /// for all. A section of a mass leaves out the voices that rest throughout it.
    pub staves: Option<Vec<usize>>,
    /// How the score ends: a final barline by default.
    pub end: SectionEnd,
    /// The short names of the staves on the systems after the first, by the voices of the score;
    /// `None` for the abbreviations of their voice types. A mass numbers two voices of a type as
    /// it numbers their names: T1, T2.
    pub short_names: Option<Vec<String>>,
    /// The seed the score was set with, where it is known: the MusicXML file written of the sheet
    /// names it under its titles ([`crate::MusicXmlOptions::seed`]); the LilyPond source does
    /// not.
    pub seed: Option<u64>,
}

impl<'a> Sheet<'a> {
    pub fn new(score: &'a Score) -> Sheet<'a> {
        Sheet {
            score,
            subtitle: None,
            highlights: Vec::new(),
            remarks: Vec::new(),
            exercise: false,
            piece: None,
            first_bar: 1,
            staves: None,
            end: SectionEnd::default(),
            short_names: None,
            seed: None,
        }
    }
}

/// The short names of the voices of a mass movement, from the top, as its book and its file
/// print them on the systems after the first: the abbreviation of each voice type (S, A, T, B),
/// numbered where the movement has two voices of a type or more, as `voice_names` numbers their
/// names („Tenor 1“, „Tenor 2“): T1, T2.
pub(crate) fn short_names(voices: &[VoiceType]) -> Vec<String> {
    let mut seen = [0_usize; VoiceType::ALL.len()];
    voices
        .iter()
        .map(|&voice| {
            let abbreviation = voice.abbreviation();
            if voices.iter().filter(|&&v| v == voice).count() > 1 {
                let number = &mut seen[voice as usize];
                *number += 1;
                format!("{abbreviation}{number}")
            } else {
                abbreviation.to_string()
            }
        })
        .collect()
}

/// A part of a book: a heading and its scores, beginning on a new page.
#[derive(Clone, Debug)]
pub struct BookPart<'a> {
    pub heading: String,
    /// Lines under the heading, centred: the intonation of a Gloria or a Credo, remarks for the
    /// performers. Each is broken into lines that the page has room for.
    pub intro: Vec<String>,
    pub sheets: Vec<Sheet<'a>>,
}

/// LilyPond version the output is written for. Newer versions read it unchanged.
pub const LILYPOND_VERSION: &str = "2.24.0";

/// The widest line under the heading of a book part, in characters: about the width of an A4
/// page in italics, with room for wide letters.
const INTRO_WIDTH: usize = 90;

/// The paper of every document: A4. Unlike a book, a document allows a page with a single
/// system: LilyPond keeps `min-systems-per-page` on every page, the only one too, and would set a
/// score that fits on one system in two across the page.
const PAPER: &str = r#"\paper {
  #(set-paper-size "a4")
}
"#;

/// The paper of every book, of a mass and the workbook: A4, and no page with a single system,
/// which the end of a movement or of a mode would otherwise leave alone on its page.
const BOOK_PAPER: &str = r#"\paper {
  #(set-paper-size "a4")
  min-systems-per-page = #2
}
"#;

/// The paper of a part of the workbook with a single setting, inside the part: no minimum of
/// systems per page, as in a document. The part's page would otherwise have to hold two systems,
/// and LilyPond would set a setting that fits on one in two across the page.
const SINGLE_SETTING_PAPER: &str = r"\paper {
  min-systems-per-page = #0
}
";

/// The layout of every document and book, for all its scores, in each line of text (and of
/// interval figures): two syllables of a word at least half a staff space apart, so that
/// LilyPond prints the hyphen between them, which it leaves out where they come closer than 0.3
/// staff spaces, as after a dotted note or in the dense text of the Gloria and the Credo, and
/// then prints two syllables as one word under two notes; and at least 7 staff spaces from the
/// baseline of the line down to the middle line of the staff below it (`minimum-distance`
/// between their reference points), so that the numbers over the entries and the ficta of that
/// staff stay out of the line.
const LAYOUT: &str = r"\layout {
  \context {
    \Lyrics
    \override LyricHyphen.minimum-distance = #0.5
    \override VerticalAxisGroup.nonstaff-unrelatedstaff-spacing.minimum-distance = #7
  }
}
";

/// The staff size of the book of a mass, at the top level before its paper: 18 pt, a staff
/// about 6.3 mm high instead of 7.0 mm at LilyPond's 20 pt, so that the long movements take
/// fewer pages. Single scores and the workbook keep 20 pt.
const MASS_STAFF_SIZE: &str = "#(set-global-staff-size 18)\n";

/// Writes `paper` and, after an empty line, `LAYOUT`.
fn write_paper_and_layout(out: &mut String, paper: &str) {
    out.push_str(paper);
    out.push('\n');
    out.push_str(LAYOUT);
}

/// LilyPond source for a single score.
pub fn to_lilypond(score: &Score, options: &LilypondOptions) -> String {
    document(None, &[Sheet::new(score)], options)
}

/// LilyPond source for several scores in one file, e.g. alternative solutions, on the page of
/// every document (`PAPER`) with the layout of every document and book (`LAYOUT`).
pub fn document(title: Option<&str>, sheets: &[Sheet], options: &LilypondOptions) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\\version \"{LILYPOND_VERSION}\"");
    out.push_str("\\language \"nederlands\"\n\n");
    let single_title = match sheets {
        [sheet] => sheet.score.title.as_deref(),
        _ => None,
    };
    out.push_str("\\header {\n");
    if let Some(title) = title.or(single_title) {
        let _ = writeln!(out, "  title = \"{}\"", escape(title));
    }
    out.push_str("  tagline = ##f\n}\n\n");
    write_paper_and_layout(&mut out, PAPER);
    for sheet in sheets {
        out.push('\n');
        write_score(
            &mut out,
            sheet,
            options,
            title.is_some() || sheets.len() > 1,
        );
    }
    out
}

/// LilyPond source for a book: the title on the first page, then the parts, each beginning on a
/// new page under its heading and the lines of its intro. Its page is that of every book
/// (`BOOK_PAPER`: no page with a single system), its layout that of every document
/// ([`document`]); a part with a single setting is paged like a document.
pub fn book(
    title: &str,
    subtitle: Option<&str>,
    parts: &[BookPart],
    options: &LilypondOptions,
) -> String {
    write_book(title, subtitle, None, parts, options, false)
}

/// What the book of a mass prints besides its music and what its plan says ([`mass_book`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MassBookOptions {
    /// Under the title of the mass, such as its mode and choir („dorisch, S A T B“).
    pub subtitle: Option<String>,
    /// On the right under the subtitle, where a composer stands: „fux-kontrapunkt, Seed 1“.
    pub byline: Option<String>,
    /// The priest's intonation over the Gloria and the Credo („Intonatio: Glória in excélsis
    /// Deo.“; on by default).
    pub intonation: bool,
}

impl Default for MassBookOptions {
    fn default() -> Self {
        MassBookOptions {
            subtitle: None,
            byline: None,
            intonation: true,
        }
    }
}

/// LilyPond source for the book of the mass `m` (phase 14, docs/design/phase-14/spec.md § 6): the
/// title of its plan with the subtitle and the byline of `book` on the first page; a book part
/// per movement, beginning on a new page under its title, with the intonation of a Gloria or a
/// Credo („Intonatio: Glória in excélsis Deo.“, as `book` asks) and the remarks of the plan for
/// the performers in the language of `options` under it; a score per section ([`section_score`])
/// under its title, with the staves of the voices that sing in it, named as the movement names
/// them in that language („Tenor 1“, „Tenor 2“) and on the systems after the first by the
/// abbreviations of their types, numbered as the names are (T1, T2), its measures numbered on
/// through the movement, ending in a double bar, or a final bar at the end of the movement, with
/// fermatas on the last notes. The label of a Break stands where its part sings again, over the
/// barline after the measure's rest that opens it. A section that lacks a part shows a measure's
/// rest in the staves of its voices, counted as one measure by the numbers of the sections after
/// it, with the lines that `missing` gives for it (by movement and section, from 0: which parts
/// are missing and how to set them) under it. An Osanna ut supra shows the Osanna it repeats.
/// With [`LilypondOptions::mensurstriche`] the barlines stand between the staves. The book is
/// set in staves of 18 pt instead of LilyPond's 20 (`MASS_STAFF_SIZE`), on the page of every book
/// (`BOOK_PAPER`) with the layout of every document ([`document`]).
pub fn mass_book(
    m: &Mass,
    book: &MassBookOptions,
    missing: &dyn Fn(usize, usize) -> Vec<String>,
    options: &LilypondOptions,
) -> String {
    let plan = &m.plan;
    let lang = options.lang;
    // Per movement, the score of each section with what its sheet needs: the sheets borrow the
    // scores.
    let mut movements: Vec<Vec<MassSection>> = Vec::new();
    for (mi, movement) in plan.movements.iter().enumerate() {
        let names = voice_names(&movement.voices, lang);
        let short = short_names(&movement.voices);
        let last = movement.sections.len().saturating_sub(1);
        let mut first_bar = 1;
        let mut sections = Vec::new();
        for (si, section) in movement.sections.iter().enumerate() {
            let (mut score, staves, remarks) = match section_score(m, mi, si) {
                Some(score) => {
                    let staves = (0..score.voices.len())
                        .filter(|&v| score.voices[v].notes.iter().any(|n| !n.is_rest()))
                        .collect();
                    (score, staves, Vec::new())
                }
                None => {
                    let voices = movement
                        .voices
                        .iter()
                        .map(|&voice_type| {
                            Voice::new(
                                voice_type,
                                Role::Counterpoint(Species::Free),
                                vec![Note::rest(Duration::WHOLE)],
                            )
                        })
                        .collect();
                    let empty = Score::new(movement.mode, voices);
                    (empty, section.voices.clone(), missing(mi, si))
                }
            };
            for (voice, name) in score.voices.iter_mut().zip(&names) {
                voice.name = name.clone();
            }
            let measures = score.measure_count();
            sections.push(MassSection {
                score,
                title: section.title,
                first_bar,
                staves,
                remarks,
                end: if si == last {
                    SectionEnd::Movement
                } else {
                    SectionEnd::Section
                },
                short_names: short.clone(),
            });
            first_bar += measures;
        }
        movements.push(sections);
    }
    let parts: Vec<BookPart> = plan
        .movements
        .iter()
        .zip(&movements)
        .map(|(movement, sections)| {
            let mut intro: Vec<String> = Vec::new();
            if let (true, Some(intonation)) = (book.intonation, movement.intonation) {
                intro.push(format!("Intonatio: {intonation}"));
            }
            intro.extend(movement.notes_in(lang).into_iter().map(str::to_string));
            BookPart {
                heading: movement.movement.title().to_string(),
                intro,
                sheets: sections
                    .iter()
                    .map(|section| Sheet {
                        remarks: section.remarks.clone(),
                        piece: Some(section.title.to_string()),
                        first_bar: section.first_bar,
                        staves: Some(section.staves.clone()),
                        end: section.end,
                        short_names: Some(section.short_names.clone()),
                        ..Sheet::new(&section.score)
                    })
                    .collect(),
            }
        })
        .collect();
    write_book(
        &plan.title,
        book.subtitle.as_deref(),
        book.byline.as_deref(),
        &parts,
        options,
        true,
    )
}

/// A section of a mass as its book prints it ([`mass_book`]).
struct MassSection {
    score: Score,
    title: &'static str,
    first_bar: usize,
    staves: Vec<usize>,
    remarks: Vec<String>,
    end: SectionEnd,
    short_names: Vec<String>,
}

/// LilyPond source for a book: `title`, `subtitle` and `byline` (as `composer`) on the first
/// page, then the parts, each beginning on a new page under its heading and its intro, on the
/// page `BOOK_PAPER` with the layout `LAYOUT`; a part of the workbook with a single setting is
/// paged like a document (`SINGLE_SETTING_PAPER`). The book of a mass (`mass`) is set in the
/// staff size `MASS_STAFF_SIZE`.
fn write_book(
    title: &str,
    subtitle: Option<&str>,
    byline: Option<&str>,
    parts: &[BookPart],
    options: &LilypondOptions,
    mass: bool,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\\version \"{LILYPOND_VERSION}\"");
    out.push_str("\\language \"nederlands\"\n\n");
    if mass {
        out.push_str(MASS_STAFF_SIZE);
        out.push('\n');
    }
    out.push_str("\\header {\n  tagline = ##f\n}\n\n");
    write_paper_and_layout(&mut out, BOOK_PAPER);
    for (i, part) in parts.iter().enumerate() {
        out.push_str("\n\\bookpart {\n");
        if !mass && part.sheets.len() == 1 {
            out.push_str(SINGLE_SETTING_PAPER);
        }
        if i == 0 {
            let _ = writeln!(out, "\\header {{ title = \"{}\"", escape(title));
            if let Some(subtitle) = subtitle {
                let _ = writeln!(out, "  subtitle = \"{}\"", escape(subtitle));
            }
            if let Some(byline) = byline {
                let _ = writeln!(out, "  composer = \"{}\"", escape(byline));
            }
            out.push_str("}\n");
        }
        let _ = writeln!(
            out,
            "\\markup \\fill-line {{ \\override #'(font-size . 4) \\bold \"{}\" }}",
            escape(&part.heading)
        );
        for intro in &part.intro {
            let lines: Vec<String> = balanced(intro, INTRO_WIDTH)
                .iter()
                .map(|line| format!("\\italic \"{}\"", escape(line)))
                .collect();
            let _ = writeln!(
                out,
                "\\markup \\fill-line {{ \\center-column {{ {} }} }}",
                lines.join(" ")
            );
        }
        for sheet in &part.sheets {
            out.push('\n');
            write_score(&mut out, sheet, options, true);
        }
        out.push_str("}\n");
    }
    out
}

/// `text` in lines of at most `width` characters, broken between words; a word wider than
/// `width` has a line of its own.
fn wrapped(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// `text` in as few lines of at most `width` characters as [`wrapped`] gives, as even as they
/// can be: broken at the least width that needs no more lines, as centred lines look best.
fn balanced(text: &str, width: usize) -> Vec<String> {
    let mut best = wrapped(text, width);
    for narrower in (1..width).rev() {
        let lines = wrapped(text, narrower);
        if lines.len() > best.len() {
            break;
        }
        best = lines;
    }
    best
}

/// What a staff prints: its voice, its notes as printed, the brackets over its entries, the marks
/// of the score's sections it carries and where its notes take a fermata.
struct Staff<'a> {
    v: usize,
    voice: &'a Voice,
    /// The notes as printed, with Mensurstriche merged ([`crate::mensur`]).
    shown: &'a [Shown],
    /// The brackets over its entries, by the indices of the notes as printed.
    marks: &'a [Mark],
    /// Marks before the note that begins at a tick: the labels of Breaks and sections.
    labels: Vec<(u32, String)>,
    /// The ticks at which a note ending there takes a fermata.
    fermatas: &'a [u32],
    /// The first staff of the score, which numbers its first measure.
    first: bool,
}

fn write_score(out: &mut String, sheet: &Sheet, options: &LilypondOptions, piece_title: bool) {
    let score = sheet.score;
    out.push_str("\\score {\n");
    let piece = match &sheet.piece {
        Some(piece) => piece.clone(),
        None => {
            let mode_name = score.mode.name(options.lang);
            let mut piece = String::new();
            if piece_title && let Some(title) = &score.title {
                piece.push_str(title);
                piece.push_str(" – ");
            }
            piece.push_str(&mode_name);
            if let Some(subtitle) = &sheet.subtitle {
                piece.push_str(" – ");
                piece.push_str(subtitle);
            }
            piece
        }
    };
    let _ = writeln!(out, "  \\header {{ piece = \"{}\" }}", escape(&piece));
    out.push_str("  \\new StaffGroup <<\n");
    let figures = (options.interval_figures && !sheet.exercise).then(|| interval_figures(score));
    let voices = score.voices.len();
    let staves: Vec<usize> = match &sheet.staves {
        Some(staves) => staves.iter().copied().filter(|&v| v < voices).collect(),
        None => (0..voices).collect(),
    };
    let fermatas = fermata_ticks(score, sheet);
    let shown = if options.mensurstriche {
        merge_ties(score, &fermatas)
    } else {
        Mensur::plain(score)
    };
    // The brackets over the entries of the imitation, by voice, over the notes as printed.
    let marks: Vec<Vec<Mark>> = (0..voices)
        .map(|v| {
            if options.entry_marks && !sheet.exercise && staves.contains(&v) {
                shown_marks(&entry_marks(score, v), &shown, v)
            } else {
                Vec::new()
            }
        })
        .collect();
    let labels = section_labels(score, &shown, &staves);
    for (k, &v) in staves.iter().enumerate() {
        let staff = Staff {
            v,
            voice: &score.voices[v],
            shown: &shown.voices[v],
            marks: &marks[v],
            labels: labels
                .iter()
                .filter(|(voice, _, _)| *voice == v)
                .map(|(_, tick, label)| (*tick, label.clone()))
                .collect(),
            fermatas: &fermatas,
            first: k == 0,
        };
        let texted = write_staff(out, score, &staff, sheet, options);
        if let Some(figures) = &figures {
            let below_this = if staves.len() == 2 {
                k == 0
            } else {
                k + 1 == staves.len()
            };
            if below_this {
                // Below the voice's text, which stays next to its staff.
                let above = if texted { "text" } else { "staff" };
                let _ = writeln!(
                    out,
                    "    \\new Lyrics \\with {{ alignBelowContext = \"{above}{v}\" }} \\lyricmode {{\n      \\override LyricText.font-size = #-1\n      {figures}\n    }}"
                );
            }
        }
    }
    out.push_str("  >>\n");
    let mut contexts: Vec<&str> = Vec::new();
    if options.mensurstriche {
        // Barlines between the staves only; the double and final bars still run through them.
        contexts.push("    \\context {\n      \\Staff\n      measureBarType = \"-span|\"\n    }\n");
    }
    if marks.iter().any(|m| !m.is_empty()) {
        // The brackets go above the staff, away from the lyrics, and the numbers of their points
        // above them: a text script of a priority over the bracket's 800 is placed after it,
        // farther from the staff. So are the labels of highlights. (The 800 is LilyPond 2.26's;
        // in 2.24 the bracket has no outside-staff-priority.)
        contexts.push(
            "    \\context {\n      \\Voice\n      \\consists \"Horizontal_bracket_engraver\"\n      \\override HorizontalBracket.direction = #UP\n      \\override TextScript.outside-staff-priority = #1000\n    }\n",
        );
    }
    if contexts.is_empty() {
        out.push_str("  \\layout { }\n}\n");
    } else {
        out.push_str("  \\layout {\n");
        for context in contexts {
            out.push_str(context);
        }
        out.push_str("  }\n}\n");
    }
    if !sheet.remarks.is_empty() {
        out.push_str("\\markup \\column {\n");
        for remark in &sheet.remarks {
            let _ = writeln!(out, "  \\line {{ \"{}\" }}", escape(remark));
        }
        out.push_str("}\n");
    }
}

/// The brackets `marks` of voice `v` over its notes as printed in `shown`: from the note that
/// stands for a bracket's first note to the one that stands for its last. None where both are one
/// note as printed, or where a bracket would begin on the note where the one before it ends.
fn shown_marks(marks: &[Mark], shown: &Mensur, v: usize) -> Vec<Mark> {
    let mut kept: Vec<Mark> = Vec::with_capacity(marks.len());
    for mark in marks {
        let (Some(first), Some(last)) = (shown.shown(v, mark.first), shown.shown(v, mark.last))
        else {
            continue;
        };
        if last > first && kept.last().is_none_or(|before| before.last < first) {
            kept.push(Mark {
                first,
                last,
                point: mark.point,
            });
        }
    }
    kept
}

/// The ticks at which the notes of every staff of `sheet` take a fermata (phase 14): before a
/// section within its score, and at the end where the sheet ends a section or a movement.
fn fermata_ticks(score: &Score, sheet: &Sheet) -> Vec<u32> {
    let per_measure = score.meter.ticks_per_measure();
    let mut fermatas: Vec<u32> = score
        .sections
        .iter()
        .filter(|mark| mark.kind == SectionKind::Section && mark.measure > 0)
        .map(|mark| mark.measure as u32 * per_measure)
        .collect();
    if sheet.end != SectionEnd::Plain {
        fermatas.push(score.total_ticks());
    }
    fermatas
}

/// The marks of the sections of `score` (phase 14), as (voice, tick, LilyPond), each before the
/// note as printed of the first staff of `staves` that begins at its tick: a section within the
/// score after a double bar with its title in bold, at its measure; the label of a Break in
/// italics where its part sings again, at the first note of a staff from its measure on, over
/// the barline after the measure's rest that opens the part, so that the label stays with the
/// music it names where a line or a page breaks after that rest; a seam has none.
fn section_labels(score: &Score, shown: &Mensur, staves: &[usize]) -> Vec<(usize, u32, String)> {
    let per_measure = score.meter.ticks_per_measure();
    let mut labels = Vec::new();
    for mark in &score.sections {
        let mut tick = mark.measure as u32 * per_measure;
        let title = mark.title.as_deref().map(escape);
        let label = match (mark.kind, title) {
            (SectionKind::Break, Some(title)) => {
                let sung = staves
                    .iter()
                    .filter_map(|&v| {
                        shown.voices[v]
                            .iter()
                            .find(|s| s.start >= tick && !s.note.is_rest())
                            .map(|s| s.start)
                    })
                    .min();
                tick = sung.unwrap_or(tick);
                format!("\\mark \\markup \\italic \"{title}\"")
            }
            (SectionKind::Section, title) => {
                let mut label = String::new();
                if tick > 0 {
                    label.push_str("\\bar \"||\"");
                }
                if let Some(title) = title {
                    if !label.is_empty() {
                        label.push(' ');
                    }
                    let _ = write!(label, "\\mark \\markup \\bold \"{title}\"");
                }
                label
            }
            _ => String::new(),
        };
        let carrier = staves
            .iter()
            .copied()
            .find(|&v| shown.voices[v].iter().any(|s| s.start == tick));
        if let (false, Some(v)) = (label.is_empty(), carrier) {
            labels.push((v, tick, label));
        }
    }
    labels
}

/// Writes the staff of a voice with the brackets over its entries, and its text, if it has one;
/// returns whether it has.
fn write_staff(
    out: &mut String,
    score: &Score,
    staff: &Staff,
    sheet: &Sheet,
    options: &LilypondOptions,
) -> bool {
    let (v, voice) = (staff.v, staff.voice);
    let clef = voice.voice_type.clef();
    let clef_name = match options.clefs {
        ClefStyle::Modern => clef.modern_lilypond(),
        ClefStyle::Original => clef.lilypond(),
    };
    let name = escape(&voice.name);
    let instrument = if voice.role == Role::CantusFirmus {
        format!("\\markup \\center-column {{ \"{name}\" \\small \"C. f.\" }}")
    } else {
        format!("\"{name}\"")
    };
    let short = sheet
        .short_names
        .as_ref()
        .and_then(|names| names.get(v))
        .map_or(voice.voice_type.abbreviation(), String::as_str);
    let _ = writeln!(
        out,
        "    \\new Staff = \"staff{v}\" \\with {{ instrumentName = {instrument} shortInstrumentName = \"{}\" }}",
        escape(short)
    );
    let _ = writeln!(out, "    \\new Voice = \"voice{v}\" {{");
    if staff.first && sheet.first_bar != 1 {
        // The measures of a movement are numbered on through its sections.
        let _ = writeln!(
            out,
            "      \\set Score.currentBarNumber = #{}",
            sheet.first_bar
        );
    }
    let final_name = score.mode.final_pitch(3).to_lilypond();
    let _ = writeln!(
        out,
        "      \\key {final_name} \\{} \\time {} \\clef \"{clef_name}\"",
        score.mode.mode.id(),
        score.meter
    );
    let blank = sheet.exercise && voice.role != Role::CantusFirmus;
    if blank {
        // An empty staff as long as the voice, in eighths.
        let ticks = voice
            .timed()
            .last()
            .map_or(0, |(start, note)| start + note.duration.ticks());
        let _ = writeln!(out, "      s8*{ticks}");
    } else {
        let _ = writeln!(out, "      {}", music(score, staff, sheet));
    }
    let bar = match sheet.end {
        SectionEnd::Section => "||",
        SectionEnd::Plain | SectionEnd::Movement => "|.",
    };
    let _ = write!(out, "      \\bar \"{bar}\"\n    }}\n");
    let text = if blank { None } else { lyrics(&voice.notes) };
    if let Some(text) = &text {
        let _ = writeln!(
            out,
            "    \\new Lyrics = \"text{v}\" \\lyricsto \"voice{v}\" {{ {text} }}"
        );
    }
    text.is_some()
}

/// The lyrics of a voice for `\lyricsto`, if it has syllables: one item per attack, as rests and
/// tied notes take none. `--` joins the syllables of a word, `__` draws the extender line under a
/// word-final syllable sung to several notes, and `_` skips a further note of a syllable. Notes
/// merged for Mensurstriche are tied notes, so they sing the same items.
fn lyrics(notes: &[Note]) -> Option<String> {
    if notes.iter().all(|n| n.syllable.is_none()) {
        return None;
    }
    let mut items = Vec::new();
    let mut tied_in = false;
    for (note, extend) in notes.iter().zip(extenders(notes)) {
        let skip = tied_in || note.is_rest();
        tied_in = note.tie;
        if skip {
            continue;
        }
        items.push(match &note.syllable {
            Some(s) if word_goes_on(s) => format!("\"{}\" --", escape(s.trim_end_matches('-'))),
            Some(s) if extend == Some(Extend::Start) => format!("\"{}\" __", escape(s)),
            Some(s) => format!("\"{}\"", escape(s)),
            None => "_".to_string(),
        });
    }
    Some(items.join(" "))
}

/// The notes of one staff as printed, with bar checks, ficta, cautionary accidentals,
/// highlights, the brackets over its entries (`\startGroup` … `\stopGroup`, the number of the
/// point over the bracket's first note as a text script, which a broken bracket does not repeat
/// after the line break), the marks of the score's sections and fermatas.
fn music(score: &Score, staff: &Staff, sheet: &Sheet) -> String {
    let per_measure = score.meter.ticks_per_measure();
    let mut parts: Vec<String> = Vec::new();
    for (n, shown) in staff.shown.iter().enumerate() {
        let start = shown.start;
        if start > 0 && start % per_measure == 0 {
            parts.push("|".to_string());
        }
        let mut token = String::new();
        for (_, label) in staff.labels.iter().filter(|(tick, _)| *tick == start) {
            token.push_str(label);
            token.push(' ');
        }
        // The highlights of the notes it stands for.
        let highlights = || {
            sheet
                .highlights
                .iter()
                .filter(|h| h.voice == staff.v && shown.notes.contains(&h.note))
        };
        if let Some(h) = highlights().next() {
            let color = h.color.scheme();
            let _ = write!(
                token,
                "\\once \\override NoteHead.color = {color} \\once \\override Stem.color = {color} "
            );
        }
        let note = &shown.note;
        if note.ficta {
            token.push_str("\\once \\set suggestAccidentals = ##t ");
        }
        let opening = staff
            .marks
            .iter()
            .find(|m| m.first == n)
            .map_or(String::new(), |mark| {
                format!("\\startGroup^\\markup \\small \"{}\"", mark.point)
            });
        let mut closing = String::new();
        if staff.marks.iter().any(|m| m.last == n) {
            closing.push_str("\\stopGroup");
        }
        if !note.is_rest() && staff.fermatas.contains(&shown.end()) {
            closing.push_str("\\fermata");
        }
        token.push_str(&note_token(note, shown.cautionary, &opening, &closing));
        if let Some(label) = highlights().find_map(|h| h.label.as_ref()) {
            let _ = write!(token, "^\\markup \\tiny \"{}\"", escape(label));
        }
        parts.push(token);
    }
    parts.join(" ")
}

/// One note or rest; values without a single written form become tied notes. A cautionary
/// accidental (`?`) goes on the first of them, the post-event `opening` after the first,
/// `closing` after the last.
fn note_token(note: &Note, cautionary: bool, opening: &str, closing: &str) -> String {
    let pieces = Duration::split(note.duration.ticks());
    let mut out = Vec::new();
    for (i, piece) in pieces.iter().enumerate() {
        let value = piece.lilypond().unwrap_or("8");
        let last = i + 1 == pieces.len();
        let mut token = match note.pitch {
            Some(pitch) => {
                let tie = if !last || note.tie { "~" } else { "" };
                let caution = if cautionary && i == 0 { "?" } else { "" };
                format!("{}{caution}{value}{tie}", pitch.to_lilypond())
            }
            None => format!("r{value}"),
        };
        if i == 0 {
            token.push_str(opening);
        }
        if last {
            token.push_str(closing);
        }
        out.push(token);
    }
    out.join(" ")
}

/// Lyrics line with the interval numbers at every onset: the interval between the two voices,
/// or for more voices a column with each upper voice's interval to the bass.
fn interval_figures(score: &Score) -> String {
    let slices = score.slices();
    let end = score.total_ticks();
    let mut out = Vec::new();
    for (i, slice) in slices.iter().enumerate() {
        let next = slices.get(i + 1).map_or(end, |s| s.tick);
        let text = figure_text(slice, score.voices.len());
        let mut pieces = Duration::split(next - slice.tick).into_iter();
        if let Some(first) = pieces.next() {
            let value = first.lilypond().unwrap_or("8");
            out.push(match &text {
                Some(t) => format!("{t}{value}"),
                None => format!("\\skip {value}"),
            });
        }
        out.extend(pieces.map(|p| format!("\\skip {}", p.lilypond().unwrap_or("8"))));
    }
    out.join(" ")
}

fn figure_text(slice: &Slice, voices: usize) -> Option<String> {
    let sounding: Vec<(usize, fux_core::Sounding)> = slice
        .sounding
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.map(|s| (i, s)))
        .collect();
    if sounding.len() < 2 {
        return None;
    }
    if voices == 2 {
        let (a, b) = (sounding[0].1.pitch, sounding[1].1.pitch);
        return Some(format!("\"{}\"", Interval::between(a, b).number()));
    }
    let (bass_voice, bass) = slice.lowest()?;
    let mut numbers: Vec<u32> = sounding
        .iter()
        .filter(|(i, _)| *i != bass_voice)
        .map(
            |(_, s)| match Interval::between(bass.pitch, s.pitch).simple().number() {
                1 => 8,
                n => n,
            },
        )
        .collect();
    numbers.sort_unstable_by(|a, b| b.cmp(a));
    numbers.dedup();
    let column: Vec<String> = numbers.iter().map(|n| format!("\"{n}\"")).collect();
    Some(format!(
        "\\markup \\center-column {{ {} }}",
        column.join(" ")
    ))
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use fux_core::{SectionMark, VoiceType, parse_lilypond};

    use super::*;

    #[test]
    fn long_values_become_ties() {
        let note = Note::new("D4".parse().unwrap(), Duration::from_ticks(5));
        assert_eq!(note_token(&note, false, "", ""), "d'2~ d'8");
        assert_eq!(
            note_token(&Note::rest(Duration::from_ticks(7)), false, "", ""),
            "r2. r8"
        );
        assert_eq!(note_token(&note.clone().tied(), false, "", ""), "d'2~ d'8~");
        // A bracket opens at the first piece and closes at the last.
        assert_eq!(
            note_token(&note, false, "\\startGroup", "\\stopGroup"),
            "d'2~\\startGroup d'8\\stopGroup"
        );
        // A cautionary accidental goes on the first piece only.
        assert_eq!(
            note_token(&note, true, "", "\\fermata"),
            "d'?2~ d'8\\fermata"
        );
    }

    #[test]
    fn a_book_with_exercises() {
        let cantus = |s: &str| {
            fux_core::parse_lilypond(s, Duration::WHOLE)
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>()
        };
        let score = Score::new(
            "dorian".parse().unwrap(),
            vec![
                Voice::new(
                    fux_core::VoiceType::Soprano,
                    Role::Counterpoint(fux_core::Species::First),
                    cantus("a'1 g'1 a'1"),
                ),
                Voice::new(
                    fux_core::VoiceType::Alto,
                    Role::CantusFirmus,
                    cantus("d'1 e'1 d'1"),
                ),
            ],
        );
        let mut exercise = Sheet::new(&score);
        exercise.exercise = true;
        exercise.subtitle = Some("1. Gattung".into());
        let parts = [
            BookPart {
                heading: "Aufgaben".into(),
                intro: Vec::new(),
                sheets: vec![exercise],
            },
            BookPart {
                heading: "Lösungen".into(),
                intro: Vec::new(),
                sheets: vec![Sheet::new(&score)],
            },
        ];
        let options = LilypondOptions {
            interval_figures: true,
            ..LilypondOptions::default()
        };
        let text = book("Übungsheft", Some("Fux"), &parts, &options);
        assert_eq!(text.matches("\\bookpart").count(), 2);
        assert_eq!(text.matches("title = \"Übungsheft\"").count(), 1);
        assert!(text.contains("subtitle = \"Fux\""));
        // The exercise has an empty staff for the counterpoint and no figures; the solution
        // has both.
        assert_eq!(text.matches("s8*24").count(), 1, "{text}");
        assert_eq!(text.matches("a'1 | g'1 | a'1").count(), 1, "{text}");
        assert_eq!(text.matches("\\new Lyrics").count(), 1, "{text}");
        assert!(text.contains("piece = \"dorisch – 1. Gattung\""), "{text}");
        // Without an intro or a byline; with the page of every book and the layout of every
        // document, but in LilyPond's staff size, not in that of the book of a mass.
        assert!(
            !text.contains("composer") && !text.contains("\\italic"),
            "{text}"
        );
        assert!(
            text.contains(&format!(
                "  tagline = ##f\n}}\n\n{BOOK_PAPER}\n{LAYOUT}\n\\bookpart {{"
            )),
            "{text}"
        );
        assert!(text.contains("  min-systems-per-page = #2\n"), "{text}");
        assert!(!text.contains("set-global-staff-size"), "{text}");
        // Each of the two parts holds a single setting and is paged like a document; a part
        // with two keeps the minimum of the book.
        assert_eq!(
            text.matches(&format!("\\bookpart {{\n{SINGLE_SETTING_PAPER}"))
                .count(),
            2,
            "{text}"
        );
        let two = [BookPart {
            heading: "Dorisch".into(),
            intro: Vec::new(),
            sheets: vec![Sheet::new(&score), Sheet::new(&score)],
        }];
        let text = book("Übungsheft", None, &two, &options);
        assert!(
            text.contains(&format!(
                "{BOOK_PAPER}\n{LAYOUT}\n\\bookpart {{\n\\header {{ title"
            )),
            "{text}"
        );
        assert!(!text.contains("min-systems-per-page = #0"), "{text}");
    }

    /// Three voices of a movement in one score: the first section from the start, a Break with
    /// its label where its part sings again, a seam without one, a section after a double bar
    /// with fermatas before it. The alto rests throughout and is left out; the sheet numbers its
    /// measures from 7 and ends a movement. Its staves have the abbreviations of their voice types
    /// as short names, or those the sheet gives them.
    #[test]
    fn sections_within_a_score() {
        let notes = |src: &str| parse_lilypond(src, Duration::WHOLE).unwrap();
        let free = Role::Counterpoint(Species::Free);
        let mut score = Score::new(
            "dorian".parse().unwrap(),
            vec![
                Voice::new(
                    VoiceType::Soprano,
                    free,
                    notes("a'1 | r1 | d''1 | r2 a'2 | d''1"),
                ),
                Voice::new(VoiceType::Alto, free, notes("r1 | r1 | r1 | r1 | r1")),
                Voice::new(VoiceType::Bass, free, notes("d1 | r1 | d1 | r2 d2 | d1")),
            ],
        );
        let mark = |measure, kind, title: Option<&str>| SectionMark {
            measure,
            kind,
            title: title.map(str::to_string),
        };
        score.sections = vec![
            mark(0, SectionKind::Section, Some("Kyrie I")),
            mark(1, SectionKind::Break, Some("Qui tollis")),
            mark(3, SectionKind::Seam, None),
            mark(4, SectionKind::Section, Some("Christe")),
        ];
        score.validate().unwrap();
        let sheet = Sheet {
            first_bar: 7,
            staves: Some(vec![0, 2]),
            end: SectionEnd::Movement,
            piece: Some("Kyrie".into()),
            ..Sheet::new(&score)
        };
        let text = document(None, &[sheet], &LilypondOptions::default());
        assert!(text.contains("piece = \"Kyrie\""), "{text}");
        assert!(!text.contains("voice1"), "{text}");
        assert_eq!(text.matches("currentBarNumber = #7").count(), 1, "{text}");
        // The label of the Break stands where its part sings again, after its measure's rest.
        assert!(
            text.contains(
                "\\mark \\markup \\bold \"Kyrie I\" a'1 | r1 | \\mark \\markup \\italic \"Qui tollis\" d''1 | r2 a'2\\fermata | \\bar \"||\" \\mark \\markup \\bold \"Christe\" d''1\\fermata"
            ),
            "{text}"
        );
        assert!(
            text.contains("d1 | r1 | d1 | r2 d2\\fermata | d1\\fermata\n      \\bar \"|.\""),
            "{text}"
        );
        // A section that more sections follow ends in a double bar.
        let sheet = Sheet {
            end: SectionEnd::Section,
            ..Sheet::new(&score)
        };
        let text = document(None, &[sheet], &LilypondOptions::default());
        assert_eq!(text.matches("\\bar \"||\"\n").count(), 3, "{text}");
        assert!(!text.contains("currentBarNumber"), "{text}");
        assert_eq!(text.matches("shortInstrumentName = ").count(), 3);
        for short in ["S", "A", "B"] {
            assert!(
                text.contains(&format!("shortInstrumentName = \"{short}\" }}")),
                "{text}"
            );
        }
        // The short names a mass gives two voices of a type.
        assert_eq!(
            short_names(&[
                VoiceType::Soprano,
                VoiceType::Tenor,
                VoiceType::Tenor,
                VoiceType::Bass
            ]),
            ["S", "T1", "T2", "B"]
        );
        let sheet = Sheet {
            short_names: Some(short_names(&[
                VoiceType::Soprano,
                VoiceType::Soprano,
                VoiceType::Bass,
            ])),
            ..Sheet::new(&score)
        };
        let text = document(None, &[sheet], &LilypondOptions::default());
        for short in ["S1", "S2", "B"] {
            assert!(
                text.contains(&format!("shortInstrumentName = \"{short}\" }}")),
                "{text}"
            );
        }
    }

    /// With Mensurstriche a highlight of a note merged with the one before it marks the note as
    /// printed, with its label; without them the tied note itself.
    #[test]
    fn highlights_follow_merged_notes() {
        let notes = |src: &str| parse_lilypond(src, Duration::WHOLE).unwrap();
        let free = Role::Counterpoint(Species::Free);
        let score = Score::new(
            "dorian".parse().unwrap(),
            vec![
                Voice::new(
                    VoiceType::Soprano,
                    free,
                    notes("a'1 | g'2 a'2~ | a'2 g'2 | a'1"),
                ),
                Voice::new(VoiceType::Bass, free, notes("d1 | e1 | f2 g2 | d1")),
            ],
        );
        let sheet = Sheet {
            highlights: vec![Highlight {
                voice: 0,
                note: 3,
                color: Color::Blue,
                label: Some("MEL".into()),
            }],
            ..Sheet::new(&score)
        };
        let options = LilypondOptions {
            mensurstriche: true,
            ..LilypondOptions::default()
        };
        let text = document(None, &[sheet], &options);
        assert!(
            text.contains(
                "a'1 | g'2 \\once \\override NoteHead.color = #blue \\once \\override Stem.color = #blue a'1^\\markup \\tiny \"MEL\" g'2 | a'1"
            ),
            "{text}"
        );
        let plain = Sheet {
            highlights: vec![Highlight {
                voice: 0,
                note: 3,
                color: Color::Blue,
                label: None,
            }],
            ..Sheet::new(&score)
        };
        let text = document(None, &[plain], &LilypondOptions::default());
        assert!(
            text.contains("a'2~ | \\once \\override NoteHead.color = #blue \\once \\override Stem.color = #blue a'2 g'2"),
            "{text}"
        );
    }

    #[test]
    fn intro_lines_are_wrapped() {
        assert_eq!(wrapped("a bb ccc", 4), ["a bb", "ccc"]);
        assert_eq!(wrapped("abcdef g", 4), ["abcdef", "g"]);
        assert!(wrapped("  ", 4).is_empty());
        // As many lines as the width needs, as even as they can be.
        assert_eq!(wrapped("aa bb cc dd ee", 12), ["aa bb cc dd", "ee"]);
        assert_eq!(balanced("aa bb cc dd ee", 12), ["aa bb cc", "dd ee"]);
        assert_eq!(balanced("aa bb", 12), ["aa bb"]);
    }

    #[test]
    fn escaping() {
        assert_eq!(escape(r#"a "b" \c"#), r#"a \"b\" \\c"#);
    }
}
