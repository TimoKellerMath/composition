//! MusicXML 4.0 (score-partwise): writing scores for notation programs, and reading the parts
//! of a score back.
//!
//! One part per voice, one MusicXML voice per part. Durations are counted in eighths
//! (`divisions` 2), the unit of `fux-core`. Musica ficta appears as an accidental above the note,
//! as in the LilyPond output; text syllables become lyrics; the entries of an imitation get
//! brackets with the numbers of their points, as directions above the staff.
//!
//! Every file has A4 pages with a head on the first, its voices as choir voices with the program
//! of the MIDI file, from two voices on in a bracket, and the tempo of the MIDI file, invisible,
//! in its first measure (M1 of docs/naechste-schritte.md): a single score ([`to_musicxml`]) under
//! its title, its mode and voices and, where it is known, its seed; the movement of a mass, a
//! file of its own ([`to_movement_musicxml`], phase 14, docs/design/phase-14/spec.md § 6), under
//! the titles of the mass and the movement, with its sections and its Breaks marked and the
//! fields of its parts, from which the reader gives the marks back ([`ImportedScore::sections`]).

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

use fux_core::mass::{movement_parts, movement_sections};
use fux_core::{
    Clef, Duration, Lang, Meter, Mode, Note, Pitch, Role, Score, ScoreError, ScoreImitation,
    SectionKind, SectionMark, Step, Voice, VoiceType,
};
use thiserror::Error;

use crate::lilypond::{ClefStyle, short_names};
use crate::lyrics::{self, Extend};
use crate::marks::{Mark, entry_marks};
use crate::midi::{MidiOptions, midi_channel};

/// Options for the MusicXML output.
#[derive(Clone, Copy, Debug)]
pub struct MusicXmlOptions {
    pub clefs: ClefStyle,
    /// A bracket over the head of every planned entry of a score that carries the plan of its
    /// imitation, with the number of its point as words (on by default).
    pub entry_marks: bool,
    /// The tempo in half notes per minute, as in the MIDI file ([`MidiOptions::tempo`], 72 by
    /// default): in the first measure twice as many quarter notes, as a `sound` without a
    /// direction (`<sound tempo="144"/>`), which notation programs play and do not print.
    pub tempo: u32,
    /// The General MIDI program of the voices, 1–128 as printed in GM lists, as in the MIDI file
    /// ([`MidiOptions::program`], 53 = Choir Aahs by default).
    pub program: u8,
    /// The language of the head of a single score ([`to_musicxml`]): of its mode under its title
    /// and of the line with its seed. The file of a movement takes its head as it is given.
    pub lang: Lang,
    /// The seed a single score was set with, where it is known: the line under its titles names
    /// it („fux-kontrapunkt, Seed 5“). Without it the file has no such line.
    pub seed: Option<u64>,
}

impl Default for MusicXmlOptions {
    fn default() -> Self {
        let midi = MidiOptions::default();
        MusicXmlOptions {
            clefs: ClefStyle::default(),
            entry_marks: true,
            tempo: midi.tempo,
            program: midi.program,
            lang: Lang::default(),
            seed: None,
        }
    }
}

/// Divisions of a quarter note: an eighth is one tick.
const DIVISIONS: u32 = 2;

/// The `miscellaneous-field` with the mode as `fux` names it (`hypophrygian`,
/// `dorian-transposed`): the key of MusicXML says neither plagal nor authentic.
pub const FIELD_MODE: &str = "fux-mode";

/// The `miscellaneous-field` with the internal cadences: the measure (from 1) the target arrives
/// in and the target, such as `13:A,22:F,26:G`, as `fux free --cadences` takes them.
pub const FIELD_CADENCES: &str = "fux-cadences";

/// The `miscellaneous-field` with the text the voices sing, as `fux check --text` takes it
/// (`kyrie`, `gloria:1-5`, an own text in full).
pub const FIELD_TEXT: &str = "fux-text";

/// The `miscellaneous-field` with the imitation the setting was planned with, as
/// [`ScoreImitation::spec`] writes it (`p0-7g11|1@0/0:D4.8,E4.4,…|p3-7|…`), so that a check of
/// the file judges the entries as the search did.
pub const FIELD_IMITATION: &str = "fux-imitation";

/// The score as MusicXML, with the part list, the page and the tempo of the file of a movement
/// ([`to_movement_musicxml`]), but without sections:
/// - the head: the score's title as the work's title; on the first of A4 pages (7 mm = 40
///   tenths) the title, under it the mode and the voices from the top in the language of
///   `options` („dorisch, S A T B“), and where `options` know the seed the line
///   „fux-kontrapunkt, Seed 5“ on the right;
/// - from two voices on a bracket around all parts, the bar lines through the staves and between
///   them; each voice a choir voice of its range on its MIDI channel with the program of
///   `options`, under the abbreviation of its type;
/// - the tempo of `options` in the first measure of the first part, invisible.
///
/// Besides the notes it keeps what a check of the file needs and MusicXML has no element for, in
/// `miscellaneous-field`s of the identification: the mode ([`FIELD_MODE`]), the internal
/// cadences ([`FIELD_CADENCES`]), the text the voices sing ([`FIELD_TEXT`]) and the plan of the
/// imitation ([`FIELD_IMITATION`]).
pub fn to_musicxml(score: &Score, options: &MusicXmlOptions) -> String {
    to_musicxml_with_fields(score, options, &[])
}

/// The score as MusicXML ([`to_musicxml`]) with more `miscellaneous-field`s after its own, each
/// as (name, value): what a file keeps besides its score, such as the mass and the part in the
/// file of a part of a mass (`fux-mass`, `fux-part`). [`ImportedScore::fields`] reads them back.
pub fn to_musicxml_with_fields(
    score: &Score,
    options: &MusicXmlOptions,
    extra: &[(String, String)],
) -> String {
    let mut out = String::new();
    write_head(&mut out);
    if let Some(title) = &score.title {
        let _ = writeln!(
            out,
            "  <work><work-title>{}</work-title></work>",
            escape(title)
        );
    }
    let mut fields = vec![(FIELD_MODE, score.mode.id())];
    if !score.cadences.is_empty() {
        let cadences: Vec<String> = score
            .cadences
            .iter()
            .map(|c| format!("{}:{}", c.measure + 1, c.target.letter()))
            .collect();
        fields.push((FIELD_CADENCES, cadences.join(",")));
    }
    if let Some(sung) = &score.text {
        fields.push((FIELD_TEXT, sung.text.spec()));
    }
    if let Some(imitation) = &score.imitation {
        fields.push((FIELD_IMITATION, imitation.spec()));
    }
    fields.extend(
        extra
            .iter()
            .map(|(name, value)| (name.as_str(), value.clone())),
    );
    write_identification(&mut out, &fields);
    out.push_str(DEFAULTS);
    let lang = options.lang;
    let byline = options
        .seed
        .map(|seed| format!("fux-kontrapunkt, {} {seed}", lang.pick("Seed", "seed")));
    write_credits(
        &mut out,
        score.title.as_deref().unwrap_or_default(),
        &mode_and_voices(score, lang),
        byline.as_deref().unwrap_or_default(),
    );
    let short: Vec<String> = score
        .voices
        .iter()
        .map(|voice| voice.voice_type.abbreviation().to_string())
        .collect();
    write_part_list(&mut out, score, &short, "yes", options.program);
    for (v, voice) in score.voices.iter().enumerate() {
        let _ = writeln!(out, "  <part id=\"P{}\">", v + 1);
        let marks = if options.entry_marks {
            entry_marks(score, v)
        } else {
            Vec::new()
        };
        let mut layout = PartLayout::single(score, &voice.notes);
        if v == 0 {
            layout.measures[0].tempo = Some(quarters_per_minute(options));
        }
        write_part(
            &mut out,
            score,
            &voice.notes,
            voice.voice_type.clef(),
            options,
            &marks,
            &layout,
        );
        out.push_str("  </part>\n");
    }
    out.push_str("</score-partwise>\n");
    out
}

/// The XML declaration, the document type and the root element's start tag.
fn write_head(out: &mut String) {
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n");
    out.push_str("<!DOCTYPE score-partwise PUBLIC \"-//Recordare//DTD MusicXML 4.0 Partwise//EN\" \"http://www.musicxml.org/dtds/partwise.dtd\">\n");
    out.push_str("<score-partwise version=\"4.0\">\n");
}

/// The identification: the software and the `miscellaneous-field`s (name, value).
fn write_identification(out: &mut String, fields: &[(&str, String)]) {
    let _ = write!(
        out,
        "  <identification><encoding><software>fux-kontrapunkt {}</software></encoding><miscellaneous>",
        env!("CARGO_PKG_VERSION")
    );
    for (name, value) in fields {
        let _ = write!(
            out,
            "<miscellaneous-field name=\"{}\">{}</miscellaneous-field>",
            escape(name),
            escape(value)
        );
    }
    out.push_str("</miscellaneous></identification>\n");
}

/// The name of a part: the voice's, and a cantus firmus says so.
fn part_name(voice: &Voice) -> String {
    if voice.role == Role::CantusFirmus {
        format!("{} (C. f.)", voice.name)
    } else {
        voice.name.clone()
    }
}

/// The subtitle of a single score: its mode in `lang` and the abbreviations of its voices from
/// the top, as a mass names its mode and choir („dorisch, S A T B“).
fn mode_and_voices(score: &Score, lang: Lang) -> String {
    let voices: Vec<&str> = score
        .voices
        .iter()
        .map(|voice| voice.voice_type.abbreviation())
        .collect();
    format!("{}, {}", score.mode.name(lang), voices.join(" "))
}

/// The tempo of `options` in quarter notes per minute: twice its half notes, as the MIDI file
/// sounds it.
fn quarters_per_minute(options: &MusicXmlOptions) -> u32 {
    options.tempo.max(1).saturating_mul(2)
}

/// The head on the first page: the title, the subtitle under it and a line on the right under
/// both, each where it is not empty, placed on the A4 page of [`DEFAULTS`].
fn write_credits(out: &mut String, title: &str, subtitle: &str, byline: &str) {
    let credits = [
        ("title", title, 600, 1650, "center", Some(22)),
        ("subtitle", subtitle, 600, 1590, "center", Some(16)),
        ("composer", byline, 1114, 1540, "right", None),
    ];
    for (kind, text, x, y, justify, size) in credits {
        if text.is_empty() {
            continue;
        }
        let size = size.map_or(String::new(), |s| format!(" font-size=\"{s}\""));
        let _ = writeln!(
            out,
            "  <credit page=\"1\"><credit-type>{kind}</credit-type><credit-words default-x=\"{x}\" default-y=\"{y}\" justify=\"{justify}\" valign=\"top\"{size}>{}</credit-words></credit>",
            escape(text)
        );
    }
}

/// The part list: from two voices on a bracket around all parts with the bar lines `barline`
/// (`yes` through the staves and between them, `Mensurstrich` only between them), and each voice
/// under its name and its short name in `short` a choir voice of its range ([`voice_sound`]) on
/// its MIDI channel, with the General MIDI program `program` (1–128). A single voice, such as a
/// cantus firmus, has no bracket, as in its LilyPond score: LilyPond prints none around one staff.
fn write_part_list(out: &mut String, score: &Score, short: &[String], barline: &str, program: u8) {
    let program = program.clamp(1, 128);
    let bracket = score.voices.len() > 1;
    out.push_str("  <part-list>\n");
    if bracket {
        let _ = writeln!(
            out,
            "    <part-group type=\"start\" number=\"1\"><group-symbol>bracket</group-symbol><group-barline>{barline}</group-barline></part-group>"
        );
    }
    for ((v, voice), short) in score.voices.iter().enumerate().zip(short) {
        let id = v + 1;
        let (sound, instrument) = voice_sound(voice.voice_type);
        let _ = writeln!(
            out,
            "    <score-part id=\"P{id}\"><part-name>{}</part-name><part-abbreviation>{}</part-abbreviation><score-instrument id=\"P{id}-I1\"><instrument-name>{instrument}</instrument-name><instrument-sound>{sound}</instrument-sound></score-instrument><midi-instrument id=\"P{id}-I1\"><midi-channel>{}</midi-channel><midi-program>{program}</midi-program></midi-instrument></score-part>",
            escape(&part_name(voice)),
            escape(short),
            midi_channel(v) + 1
        );
    }
    if bracket {
        out.push_str("    <part-group type=\"stop\" number=\"1\"/>\n");
    }
    out.push_str("  </part-list>\n");
}

/// What the head of the file of a mass movement says ([`to_movement_musicxml`]): the titles of
/// the mass and of the movement, a line under them, and the priest's intonation before the
/// polyphony.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MovementHeading {
    /// The title of the mass („Missa brevis“): the work's title, and on the first page the title.
    pub mass: String,
    /// The number of the movement in the mass, from 1 (`movement-number`; none for 0): Kyrie 1
    /// to Agnus Dei 6, as the files are numbered.
    pub number: usize,
    /// The title of the movement („Gloria“, `movement-title`), and on the first page the
    /// subtitle.
    pub title: String,
    /// A line on the first page, on the right under the titles, such as the book's („dorisch,
    /// S A T B · fux-kontrapunkt, Seed 4711“).
    pub byline: Option<String>,
    /// The intonation the priest sings before the polyphony (Gloria, Credo), as the Graduale
    /// prints it („Glória in excélsis Deo.“): in italics over the first measure, after
    /// „Intonatio:“.
    pub intonation: Option<String>,
}

/// Options for the file of a mass movement ([`to_movement_musicxml`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct MovementOptions {
    /// The clefs, the brackets over the entries, the tempo and the program, as in a single
    /// score; its language and seed do not count, the heading gives the lines of a movement.
    pub musicxml: MusicXmlOptions,
    /// Bar lines only between the staves (`group-barline` `Mensurstrich`), as Verovio draws
    /// them; MuseScore draws them through the staves. Otherwise through the staves and between
    /// them (`yes`).
    pub mensurstriche: bool,
    /// A voice that rests through a section is hidden there: `print-object="no"` in every
    /// measure of the section, as Verovio reads it measure by measure, and `yes` again where the
    /// voice sings. Otherwise it shows its whole-measure rests.
    pub hide_tacet: bool,
}

/// The page of every file: A4 at 7 mm a staff (40 tenths), margins of about 15 mm.
const DEFAULTS: &str = "  <defaults><scaling><millimeters>7</millimeters><tenths>40</tenths></scaling><page-layout><page-height>1697</page-height><page-width>1200</page-width><page-margins type=\"both\"><left-margin>86</left-margin><right-margin>86</right-margin><top-margin>86</top-margin><bottom-margin>86</bottom-margin></page-margins></page-layout></defaults>\n";

/// The movement of a mass as MusicXML, one file per movement (docs/design/phase-14/spec.md §
/// 6), from its score as `fux_core::mass::movement_score` joins it, its voices named in the
/// user's language:
/// - the head: the mass as the work's title, the movement's number and title, on the first page
///   the titles and the line of `heading`, an A4 page (7 mm = 40 tenths);
/// - a bracket around all parts (`group-barline` `yes`, or `Mensurstrich` with
///   [`MovementOptions::mensurstriche`]), and each voice a choir voice of its range
///   (`voice.soprano`, `voice.mezzo-soprano`, `voice.alto`, `voice.tenor`, `voice.baritone`,
///   `voice.bass`, with an English instrument name that matches it) on its MIDI channel with
///   the program of [`MusicXmlOptions::program`], its short name numbered where the movement
///   has two voices of its type (T1, T2), as its name is;
/// - the tempo of [`MusicXmlOptions::tempo`] in the first measure of the first part, as a
///   `sound` without a direction, which notation programs play and do not print;
/// - the sections by the score's marks ([`Score::sections`]): where one begins a new system
///   (after the first) and its title as a rehearsal mark, where one ends a double bar and a
///   fermata on every voice's last note, at the end of the movement the final bar line; the
///   label of a Break in italics in the measure where its part sings again, after the measure's
///   rest that opens it, and the intonation in italics over the first measure, all over the top
///   voice that sings in the section; a voice that does not sing in a section rests a whole
///   measure in each of its measures, hidden with [`MovementOptions::hide_tacet`];
/// - the brackets over the entries of its imitation, the points counted from 1 in each section,
///   as the book counts them;
/// - the fields: the mode ([`FIELD_MODE`]) and `fields`, those of the mass and of its parts
///   (`fux_core::mass::part_fields`: `fux-mass`, and for the part numbered N within the
///   movement `fux-part.N`, `fux-text.N`, `fux-cadences.N`, `fux-imitation.N`, relative to the
///   part), and any other a frontend numbers so, such as the positions of the syllables that
///   `fux check` reads; from them `fux_core::mass::split_movement` gives the parts back, and
///   `fux_core::mass::fields_of_part` the fields of each as the file of the part has them. Its
///   own text, cadences and plan, those of the whole movement, it does not keep.
///
/// A single score ([`to_musicxml`]) has the same part list, page and tempo, with a head of its
/// own, but no sections.
pub fn to_movement_musicxml(
    score: &Score,
    heading: &MovementHeading,
    options: &MovementOptions,
    fields: &[(String, String)],
) -> String {
    let mut out = String::new();
    write_head(&mut out);
    if !heading.mass.is_empty() {
        let _ = writeln!(
            out,
            "  <work><work-title>{}</work-title></work>",
            escape(&heading.mass)
        );
    }
    if heading.number > 0 {
        let _ = writeln!(
            out,
            "  <movement-number>{}</movement-number>",
            heading.number
        );
    }
    if !heading.title.is_empty() {
        let _ = writeln!(
            out,
            "  <movement-title>{}</movement-title>",
            escape(&heading.title)
        );
    }
    let mut kept = vec![(FIELD_MODE, score.mode.id())];
    kept.extend(
        fields
            .iter()
            .map(|(name, value)| (name.as_str(), value.clone())),
    );
    write_identification(&mut out, &kept);
    out.push_str(DEFAULTS);
    write_credits(
        &mut out,
        &heading.mass,
        &heading.title,
        heading.byline.as_deref().unwrap_or_default(),
    );
    let barline = if options.mensurstriche {
        "Mensurstrich"
    } else {
        "yes"
    };
    let kinds: Vec<VoiceType> = score.voices.iter().map(|voice| voice.voice_type).collect();
    write_part_list(
        &mut out,
        score,
        &short_names(&kinds),
        barline,
        options.musicxml.program,
    );
    let mut layouts = movement_layouts(score, heading, options);
    if let Some(first) = layouts.first_mut() {
        first.measures[0].tempo = Some(quarters_per_minute(&options.musicxml));
    }
    for ((v, voice), layout) in score.voices.iter().enumerate().zip(&layouts) {
        let _ = writeln!(out, "  <part id=\"P{}\">", v + 1);
        let marks = if options.musicxml.entry_marks {
            entry_marks(score, v)
        } else {
            Vec::new()
        };
        write_part(
            &mut out,
            score,
            &voice.notes,
            voice.voice_type.clef(),
            &options.musicxml,
            &marks,
            layout,
        );
        out.push_str("  </part>\n");
    }
    out.push_str("</score-partwise>\n");
    out
}

/// The sound of a voice in MusicXML's list of sounds, and an English instrument name that
/// matches it: MuseScore takes a known instrument name over the sound („Alto“ with
/// `voice.mezzo-soprano` imports as an alto).
fn voice_sound(voice: VoiceType) -> (&'static str, &'static str) {
    match voice {
        VoiceType::HighSoprano | VoiceType::Soprano => ("voice.soprano", "Soprano"),
        VoiceType::MezzoSoprano => ("voice.mezzo-soprano", "Mezzo-soprano"),
        VoiceType::Alto => ("voice.alto", "Alto"),
        VoiceType::Tenor => ("voice.tenor", "Tenor"),
        VoiceType::Baritone => ("voice.baritone", "Baritone"),
        VoiceType::Bass => ("voice.bass", "Bass"),
    }
}

/// What the measures of a part carry besides its notes and the brackets over its entries.
#[derive(Clone, Debug)]
struct PartLayout {
    /// The notes, by their index in the part, that have a fermata.
    fermatas: Vec<usize>,
    /// Per measure of the score.
    measures: Vec<MeasureLayout>,
    /// The number each point of the plan shows over its entries, by its index in the plan;
    /// empty for the index from 1.
    points: Vec<usize>,
}

/// What a measure of a part carries besides its notes.
#[derive(Clone, Debug, Default)]
struct MeasureLayout {
    /// A new system begins with it.
    new_system: bool,
    /// The staff is hidden (`false`) or shown again (`true`) in it.
    shown: Option<bool>,
    /// The tempo it sets, in quarter notes per minute, as a `sound` after its attributes: in a
    /// `direction` the schema wants a `direction-type`, which would print it. The first measure
    /// of the first part.
    tempo: Option<u32>,
    /// Directions above the staff before its notes, each written whole.
    directions: Vec<String>,
    /// The style of the bar line at its right end.
    barline: Option<&'static str>,
}

impl PartLayout {
    /// A part of a single score: the final bar line, and a fermata on its last note.
    fn single(score: &Score, notes: &[Note]) -> PartLayout {
        let mut measures = vec![MeasureLayout::default(); score.measure_count().max(1)];
        if let Some(last) = measures.last_mut() {
            last.barline = Some("light-heavy");
        }
        PartLayout {
            fermatas: notes.len().checked_sub(1).into_iter().collect(),
            measures,
            points: Vec::new(),
        }
    }

    /// The number the point at `point` (from 1) of the plan shows over its entries.
    fn number(&self, point: usize) -> usize {
        self.points.get(point - 1).copied().unwrap_or(point)
    }
}

/// The layout of the parts of a movement's file ([`to_movement_musicxml`]), from the marks of its
/// score: the sections between the marks of kind [`SectionKind::Section`], the whole score
/// where it has none.
fn movement_layouts(
    score: &Score,
    heading: &MovementHeading,
    options: &MovementOptions,
) -> Vec<PartLayout> {
    let per = score.meter.ticks_per_measure();
    let measures = score.measure_count().max(1);
    let mut starts = vec![0];
    starts.extend(
        score
            .sections
            .iter()
            .filter(|mark| mark.kind == SectionKind::Section)
            .map(|mark| mark.measure)
            .filter(|&measure| 0 < measure && measure < measures),
    );
    starts.sort_unstable();
    starts.dedup();
    let sections: Vec<(usize, usize)> = starts
        .iter()
        .enumerate()
        .map(|(s, &first)| (first, starts.get(s + 1).copied().unwrap_or(measures)))
        .collect();
    let section_of = |measure: usize| {
        sections
            .iter()
            .rposition(|&(first, _)| first <= measure)
            .unwrap_or(0)
    };
    // Per voice and section: whether it sounds nowhere in the section.
    let tacet: Vec<Vec<bool>> = score
        .voices
        .iter()
        .map(|voice| {
            sections
                .iter()
                .map(|&(first, end)| {
                    let (from, to) = (first as u32 * per, end as u32 * per);
                    voice
                        .timed()
                        .all(|(tick, note)| tick < from || tick >= to || note.is_rest())
                })
                .collect()
        })
        .collect();
    // The voice over which the directions of a section stand: its top voice that sings.
    let top: Vec<usize> = (0..sections.len())
        .map(|s| (0..score.voices.len()).find(|&v| !tacet[v][s]).unwrap_or(0))
        .collect();
    let points = point_numbers(score, &starts);
    let mut layouts: Vec<PartLayout> = score
        .voices
        .iter()
        .enumerate()
        .map(|(v, voice)| {
            let mut layout = vec![MeasureLayout::default(); measures];
            let timed: Vec<(u32, &Note)> = voice.timed().collect();
            let mut fermatas = Vec::new();
            for (s, &(first, end)) in sections.iter().enumerate() {
                if s > 0 {
                    layout[first].new_system = true;
                }
                layout[end - 1].barline = Some(if end == measures {
                    "light-heavy"
                } else {
                    "light-light"
                });
                if options.hide_tacet {
                    if tacet[v][s] {
                        for measure in &mut layout[first..end] {
                            measure.shown = Some(false);
                        }
                    } else if s > 0 && tacet[v][s - 1] {
                        layout[first].shown = Some(true);
                    }
                }
                // The voice's last note in the section, where it is one, holds.
                let (from, to) = (first as u32 * per, end as u32 * per);
                if let Some(last) = timed.iter().rposition(|&(tick, _)| tick < to)
                    && timed[last].0 >= from
                    && !timed[last].1.is_rest()
                {
                    fermatas.push(last);
                }
            }
            PartLayout {
                fermatas,
                measures: layout,
                points: points.clone(),
            }
        })
        .collect();
    if let (Some(intonation), Some(&v)) = (&heading.intonation, top.first())
        && let Some(layout) = layouts.get_mut(v)
    {
        layout.measures[0]
            .directions
            .push(italic_words(&format!("Intonatio: {intonation}")));
    }
    for mark in score.sections.iter().filter(|mark| mark.measure < measures) {
        let (direction, measure) = match (mark.kind, &mark.title) {
            (SectionKind::Section, Some(title)) => (rehearsal(title), mark.measure),
            (SectionKind::Break, Some(label)) => (italic_words(label), sung_from(score, mark)),
            _ => continue,
        };
        if let Some(layout) = layouts.get_mut(top[section_of(mark.measure)]) {
            layout.measures[measure].directions.push(direction);
        }
    }
    layouts
}

/// The measure in which a voice of `score` first sings from the measure of `mark` on, that
/// measure where none does: where the part after a Break sings again, after the measure's rest
/// that opens it, so that its label stays with the music it names where a line or a page breaks
/// after that rest.
fn sung_from(score: &Score, mark: &SectionMark) -> usize {
    let per = score.meter.ticks_per_measure();
    let from = mark.measure as u32 * per;
    score
        .voices
        .iter()
        .filter_map(|voice| {
            voice
                .timed()
                .find(|&(tick, note)| tick >= from && !note.is_rest())
                .map(|(tick, _)| tick)
        })
        .min()
        .map_or(mark.measure, |tick| (tick / per) as usize)
}

/// The number each point of the score's plan shows over its entries, by its index: counted from
/// 1 in each section that begins at a measure of `starts`, by the section its leader enters in,
/// as the book counts them, where each section is a score of its own.
fn point_numbers(score: &Score, starts: &[usize]) -> Vec<usize> {
    let Some(plan) = score.imitation.as_deref() else {
        return Vec::new();
    };
    let per = score.meter.ticks_per_measure();
    let mut numbers = Vec::with_capacity(plan.points.len());
    let mut current = (None, 0);
    for point in &plan.points {
        let leader = point.entries.first().map_or(0, |entry| entry.start);
        let section = starts
            .iter()
            .rposition(|&first| first as u32 * per <= leader);
        if section != current.0 {
            current = (section, 0);
        }
        current.1 += 1;
        numbers.push(current.1);
    }
    numbers
}

/// A direction above the staff with a rehearsal mark: the title of a section.
fn rehearsal(title: &str) -> String {
    format!(
        "      <direction placement=\"above\">\n        <direction-type><rehearsal>{}</rehearsal></direction-type>\n      </direction>\n",
        escape(title)
    )
}

/// A direction above the staff with words in italics: the label of a Break, the intonation.
fn italic_words(text: &str) -> String {
    format!(
        "      <direction placement=\"above\">\n        <direction-type><words font-style=\"italic\">{}</words></direction-type>\n      </direction>\n",
        escape(text)
    )
}

/// Writes the measures of a part with the brackets `marks` over its entries and what `layout`
/// adds to them.
fn write_part(
    out: &mut String,
    score: &Score,
    notes: &[Note],
    clef: Clef,
    options: &MusicXmlOptions,
    marks: &[Mark],
    layout: &PartLayout,
) {
    let per_measure = score.meter.ticks_per_measure();
    let key_alter = |step: Step| score.mode.scale_alter(step);
    let mut tick = 0;
    let mut index = 0;
    // Whether the last syllable went on to the next one.
    let mut word_goes_on = false;
    let extenders = lyrics::extenders(notes);
    for (m, measure) in layout.measures.iter().enumerate() {
        let _ = writeln!(out, "    <measure number=\"{}\">", m + 1);
        if measure.new_system {
            out.push_str("      <print new-system=\"yes\"/>\n");
        }
        if m == 0 {
            write_attributes(out, score, clef, options, measure.shown);
        } else if let Some(shown) = measure.shown {
            out.push_str("      <attributes>\n");
            write_staff_details(out, shown);
            out.push_str("      </attributes>\n");
        }
        if let Some(tempo) = measure.tempo {
            let _ = writeln!(out, "      <sound tempo=\"{tempo}\"/>");
        }
        for direction in &measure.directions {
            out.push_str(direction);
        }
        // The alteration in force per step and octave, as the key gives it and as accidentals
        // in this measure change it.
        let mut in_force: HashMap<(Step, i8), i8> = HashMap::new();
        let end = (m as u32 + 1) * per_measure;
        while index < notes.len() && tick < end {
            let note = &notes[index];
            let ticks = note.duration.ticks();
            // A bracket over an entry begins at its first note and ends at its last.
            if let Some(mark) = marks.iter().find(|m| m.first == index) {
                write_bracket(out, "start", Some(layout.number(mark.point)));
            }
            if marks.iter().any(|m| m.last == index) {
                write_bracket(out, "stop", None);
            }
            out.push_str("      <note>\n");
            match note.pitch {
                None if ticks == per_measure => out.push_str("        <rest measure=\"yes\"/>\n"),
                None => out.push_str("        <rest/>\n"),
                Some(p) => {
                    out.push_str("        <pitch>");
                    let _ = write!(out, "<step>{}</step>", p.step.letter());
                    if p.alter != 0 {
                        let _ = write!(out, "<alter>{}</alter>", p.alter);
                    }
                    let _ = writeln!(out, "<octave>{}</octave></pitch>", p.octave);
                }
            }
            let _ = writeln!(
                out,
                "        <duration>{}</duration>",
                ticks * DIVISIONS / 2
            );
            let tied_in = index > 0 && notes[index - 1].tie;
            if tied_in {
                out.push_str("        <tie type=\"stop\"/>\n");
            }
            if note.tie {
                out.push_str("        <tie type=\"start\"/>\n");
            }
            out.push_str("        <voice>1</voice>\n");
            let (kind, dot) = note_type(ticks);
            let _ = writeln!(out, "        <type>{kind}</type>");
            if dot {
                out.push_str("        <dot/>\n");
            }
            let mut notations = String::new();
            if let Some(p) = note.pitch {
                let current = *in_force
                    .get(&(p.step, p.octave))
                    .unwrap_or(&key_alter(p.step));
                if note.ficta && !tied_in {
                    let _ = write!(
                        notations,
                        "<accidental-mark placement=\"above\">{}</accidental-mark>",
                        accidental(p.alter)
                    );
                } else if p.alter != current && !tied_in {
                    let _ = writeln!(
                        out,
                        "        <accidental>{}</accidental>",
                        accidental(p.alter)
                    );
                }
                in_force.insert((p.step, p.octave), p.alter);
            }
            if tied_in {
                notations.push_str("<tied type=\"stop\"/>");
            }
            if note.tie {
                notations.push_str("<tied type=\"start\"/>");
            }
            if layout.fermatas.contains(&index) {
                notations.push_str("<fermata type=\"upright\"/>");
            }
            if !notations.is_empty() {
                let _ = writeln!(out, "        <notations>{notations}</notations>");
            }
            if let Some(syllable) = &note.syllable {
                let goes_on = lyrics::word_goes_on(syllable);
                let syllabic = match (word_goes_on, goes_on) {
                    (false, false) => "single",
                    (false, true) => "begin",
                    (true, true) => "middle",
                    (true, false) => "end",
                };
                // A melisma on a word-final syllable: the extender line starts here.
                let extend = if extenders[index] == Some(Extend::Start) {
                    "<extend type=\"start\"/>"
                } else {
                    ""
                };
                let _ = writeln!(
                    out,
                    "        <lyric number=\"1\"><syllabic>{syllabic}</syllabic><text>{}</text>{extend}</lyric>",
                    escape(syllable.trim_end_matches('-'))
                );
                word_goes_on = goes_on;
            } else if extenders[index] == Some(Extend::Stop) {
                out.push_str("        <lyric number=\"1\"><extend type=\"stop\"/></lyric>\n");
            }
            out.push_str("      </note>\n");
            tick += ticks;
            index += 1;
        }
        if let Some(style) = measure.barline {
            let _ = writeln!(
                out,
                "      <barline location=\"right\"><bar-style>{style}</bar-style></barline>"
            );
        }
        out.push_str("    </measure>\n");
    }
}

/// Hides a staff from here on (`print-object="no"`) or shows it again.
fn write_staff_details(out: &mut String, shown: bool) {
    let shown = if shown { "yes" } else { "no" };
    let _ = writeln!(out, "        <staff-details print-object=\"{shown}\"/>");
}

/// A direction above the staff that starts or stops the bracket over an entry at the note that
/// follows it; a start with the number of the entry's point as words. MusicXML 4.0 wants each in
/// a `direction-type` of its own, and `line-end` on every `bracket`. Start and stop name the
/// part's staff after its voice, in the order of the schema: Verovio (6.3) leaves a bracket
/// without a `staff` open from the third staff of a score on.
fn write_bracket(out: &mut String, kind: &str, point: Option<usize>) {
    out.push_str("      <direction placement=\"above\">\n");
    if let Some(point) = point {
        let _ = writeln!(
            out,
            "        <direction-type><words>{point}</words></direction-type>"
        );
    }
    let _ = writeln!(
        out,
        "        <direction-type><bracket type=\"{kind}\" number=\"1\" line-end=\"down\"/></direction-type>"
    );
    out.push_str("        <voice>1</voice>\n        <staff>1</staff>\n      </direction>\n");
}

/// The attributes of a part's first measure: divisions, key, time and clef, and where `shown`
/// says so the staff hidden or shown.
fn write_attributes(
    out: &mut String,
    score: &Score,
    clef: Clef,
    options: &MusicXmlOptions,
    shown: Option<bool>,
) {
    out.push_str("      <attributes>\n");
    let _ = writeln!(out, "        <divisions>{DIVISIONS}</divisions>");
    let fifths = -i32::from(score.mode.flats());
    let _ = writeln!(
        out,
        "        <key><fifths>{fifths}</fifths><mode>{}</mode></key>",
        score.mode.mode.id()
    );
    let symbol = if score.meter == Meter::ALLA_BREVE {
        " symbol=\"cut\""
    } else {
        ""
    };
    let _ = writeln!(
        out,
        "        <time{symbol}><beats>{}</beats><beat-type>{}</beat-type></time>",
        score.meter.numerator, score.meter.denominator
    );
    let (sign, line, octave_change) = match options.clefs {
        ClefStyle::Original => {
            let name = clef.name();
            let sign = &name[..1];
            (sign.to_string(), name[1..].to_string(), 0)
        }
        ClefStyle::Modern => match clef {
            Clef::C4 => ("G".to_string(), "2".to_string(), -1),
            Clef::F3 | Clef::F4 => ("F".to_string(), "4".to_string(), 0),
            _ => ("G".to_string(), "2".to_string(), 0),
        },
    };
    let _ = write!(out, "        <clef><sign>{sign}</sign><line>{line}</line>");
    if octave_change != 0 {
        let _ = write!(
            out,
            "<clef-octave-change>{octave_change}</clef-octave-change>"
        );
    }
    out.push_str("</clef>\n");
    if let Some(shown) = shown {
        write_staff_details(out, shown);
    }
    out.push_str("      </attributes>\n");
}

/// The MusicXML note type of a duration in ticks, and whether it is dotted.
fn note_type(ticks: u32) -> (&'static str, bool) {
    match ticks {
        16 => ("breve", false),
        12 => ("whole", true),
        8 => ("whole", false),
        6 => ("half", true),
        4 => ("half", false),
        3 => ("quarter", true),
        2 => ("quarter", false),
        _ => ("eighth", false),
    }
}

fn accidental(alter: i8) -> &'static str {
    match alter {
        2 => "double-sharp",
        1 => "sharp",
        0 => "natural",
        -1 => "flat",
        _ => "flat-flat",
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A part read from MusicXML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedPart {
    pub name: String,
    /// The clef of the first measure, if it is one of Fux's (G2, C1–C4, F3, F4).
    pub clef: Option<Clef>,
    pub notes: Vec<Note>,
}

/// The parts of a MusicXML score, with what it says about title, meter and mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedScore {
    pub title: Option<String>,
    pub meter: Meter,
    /// The mode named in the key, if any.
    pub mode: Option<Mode>,
    /// Flats (negative) or sharps of the key.
    pub fifths: i32,
    pub parts: Vec<ImportedPart>,
    /// The `miscellaneous-field`s of the identification as (name, value), such as those
    /// [`to_musicxml`] writes ([`FIELD_MODE`], [`FIELD_CADENCES`], [`FIELD_TEXT`],
    /// [`FIELD_IMITATION`]).
    pub fields: Vec<(String, String)>,
    /// Where the sections of a mass movement and the parts within them begin, as
    /// [`Score::sections`] has them (phase 14). In the file of a movement
    /// ([`to_movement_musicxml`]) from the fields of its parts (`fux-part.N`), where every part
    /// begins: a section with its title, a Break with its label, a seam; and from its notation a
    /// section where no part begins, an Osanna ut supra. In any other file from its notation
    /// alone: a section, titled by its rehearsal mark, where a rehearsal mark stands at the first
    /// measure or at a measure after a double bar (`light-light`), and one at the first measure
    /// where a later one begins. Empty for a file without these, as every single score is.
    pub sections: Vec<SectionMark>,
}

impl ImportedScore {
    /// The value of a `miscellaneous-field`, if the file has it.
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, value)| value.as_str())
    }

    /// The plan of the imitation the file keeps ([`FIELD_IMITATION`]): `None` without the
    /// field, an error if it cannot be read. Whether it fits the score built from the parts,
    /// [`Score::validate`] checks once the plan is in it.
    pub fn imitation(&self) -> Option<Result<ScoreImitation, ScoreError>> {
        self.field(FIELD_IMITATION).map(ScoreImitation::parse)
    }
}

/// Errors reading MusicXML.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MusicXmlError {
    #[error("not XML: {0}")]
    Xml(String),
    #[error("not a score-partwise document")]
    NotPartwise,
    #[error("part {part}, measure {measure}: {what}")]
    Unsupported {
        part: String,
        measure: String,
        what: String,
    },
}

/// The first child element with the given name.
fn child<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    node.children().find(|c| c.has_tag_name(name))
}

/// The trimmed text of an element.
fn text_of(node: Option<roxmltree::Node<'_, '_>>) -> Option<String> {
    node.and_then(|n| n.text()).map(|t| t.trim().to_string())
}

/// The syllable of a lyric, ending in `-` where the word goes on. A lyric without text, such as
/// the end of an extender line, gives none.
fn syllable_of(lyric: roxmltree::Node<'_, '_>) -> Option<String> {
    let text = text_of(child(lyric, "text")).filter(|t| !t.is_empty())?;
    Some(match text_of(child(lyric, "syllabic")).as_deref() {
        Some("begin" | "middle") => format!("{text}-"),
        _ => text,
    })
}

/// Reads the parts of a score-partwise document: from each part its first voice, without chords
/// and grace notes, and the first line of its lyrics, also on rests.
pub fn from_musicxml(text: &str) -> Result<ImportedScore, MusicXmlError> {
    let document = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .map_err(|e| MusicXmlError::Xml(e.to_string()))?;
    let root = document.root_element();
    if root.tag_name().name() != "score-partwise" {
        return Err(MusicXmlError::NotPartwise);
    }
    let title = text_of(child(root, "work").and_then(|w| child(w, "work-title")))
        .or_else(|| text_of(child(root, "movement-title")));
    let fields: Vec<(String, String)> = child(root, "identification")
        .and_then(|i| child(i, "miscellaneous"))
        .map(|m| {
            m.children()
                .filter(|c| c.has_tag_name("miscellaneous-field"))
                .filter_map(|f| {
                    let name = f.attribute("name")?.to_string();
                    Some((name, f.text().unwrap_or_default().trim().to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    let names: HashMap<String, String> = child(root, "part-list")
        .map(|list| {
            list.children()
                .filter(|c| c.has_tag_name("score-part"))
                .map(|p| {
                    (
                        p.attribute("id").unwrap_or_default().to_string(),
                        text_of(child(p, "part-name")).unwrap_or_default(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let mut meter = Meter::ALLA_BREVE;
    let mut mode = None;
    let mut fifths = 0;
    let mut parts = Vec::new();
    // What the measures say of sections, by their place in a part, in any part; and the
    // measures of the first part.
    let mut marked: Vec<Marked> = Vec::new();
    let mut measures: Option<usize> = None;
    for part in root.children().filter(|c| c.has_tag_name("part")) {
        let id = part.attribute("id").unwrap_or_default().to_string();
        let mut divisions: u32 = 1;
        let mut clef = None;
        let mut notes: Vec<Note> = Vec::new();
        let mut first_voice: Option<String> = None;
        // The lyric line read: that of the part's first lyric. A lyric without a number is on
        // the first line.
        let mut line: Option<&str> = None;
        let mut count = 0;
        for (m, measure) in part
            .children()
            .filter(|c| c.has_tag_name("measure"))
            .enumerate()
        {
            count = m + 1;
            if marked.len() <= m {
                marked.resize_with(m + 1, Marked::default);
            }
            marked[m].read(measure);
            let number = measure.attribute("number").unwrap_or_default().to_string();
            let unsupported = |what: &str| MusicXmlError::Unsupported {
                part: id.clone(),
                measure: number.clone(),
                what: what.to_string(),
            };
            for element in measure.children().filter(roxmltree::Node::is_element) {
                match element.tag_name().name() {
                    "attributes" => {
                        if let Some(d) = text_of(child(element, "divisions")) {
                            divisions = d
                                .parse()
                                .ok()
                                .filter(|&d| d > 0)
                                .ok_or_else(|| unsupported("divisions"))?;
                        }
                        if let Some(key) = child(element, "key") {
                            fifths = text_of(child(key, "fifths"))
                                .and_then(|f| f.parse().ok())
                                .unwrap_or(fifths);
                            mode = text_of(child(key, "mode"))
                                .and_then(|m| Mode::ALL.into_iter().find(|x| x.id() == m))
                                .or(mode);
                        }
                        if let Some(time) = child(element, "time") {
                            let beats = text_of(child(time, "beats")).and_then(|b| b.parse().ok());
                            let beat_type =
                                text_of(child(time, "beat-type")).and_then(|b| b.parse().ok());
                            if let (Some(numerator), Some(denominator)) = (beats, beat_type) {
                                meter = Meter {
                                    numerator,
                                    denominator,
                                };
                            }
                        }
                        if clef.is_none()
                            && let Some(c) = child(element, "clef")
                        {
                            let sign = text_of(child(c, "sign")).unwrap_or_default();
                            let line = text_of(child(c, "line")).unwrap_or_default();
                            clef = format!("{sign}{line}").parse::<Clef>().ok();
                        }
                    }
                    "note" => {
                        if child(element, "grace").is_some() || child(element, "chord").is_some() {
                            continue;
                        }
                        let voice = text_of(child(element, "voice")).unwrap_or_else(|| "1".into());
                        if first_voice.get_or_insert_with(|| voice.clone()) != &voice {
                            continue;
                        }
                        let duration: u32 = text_of(child(element, "duration"))
                            .and_then(|d| d.parse().ok())
                            .ok_or_else(|| unsupported("note without duration"))?;
                        if !(duration * 2).is_multiple_of(divisions) {
                            return Err(unsupported("a duration shorter than an eighth"));
                        }
                        let ticks = duration * 2 / divisions;
                        let pitch = match child(element, "pitch") {
                            Some(p) => {
                                let step = text_of(child(p, "step"))
                                    .and_then(|s| s.chars().next())
                                    .and_then(|c| Step::ALL.into_iter().find(|x| x.letter() == c))
                                    .ok_or_else(|| unsupported("pitch"))?;
                                let alter: f32 = text_of(child(p, "alter"))
                                    .and_then(|a| a.parse().ok())
                                    .unwrap_or(0.0);
                                let octave: i8 = text_of(child(p, "octave"))
                                    .and_then(|o| o.parse().ok())
                                    .ok_or_else(|| unsupported("octave"))?;
                                Some(Pitch::new(step, alter.round() as i8, octave))
                            }
                            None => None,
                        };
                        let tie = element
                            .children()
                            .any(|c| c.has_tag_name("tie") && c.attribute("type") == Some("start"));
                        let ficta = element.descendants().any(|c| {
                            c.has_tag_name("accidental-mark")
                                || (c.has_tag_name("accidental")
                                    && c.attribute("editorial") == Some("yes"))
                        });
                        let syllable = element
                            .children()
                            .filter(|c| c.has_tag_name("lyric"))
                            .find(|lyric| {
                                let number = lyric.attribute("number").unwrap_or("1");
                                *line.get_or_insert(number) == number
                            })
                            .and_then(syllable_of);
                        let duration = Duration::from_ticks(ticks);
                        notes.push(Note {
                            pitch,
                            duration,
                            tie: tie && pitch.is_some(),
                            ficta: ficta && pitch.is_some(),
                            syllable,
                            lyric: None,
                        });
                    }
                    "backup" | "forward" => {
                        // Other voices: the reader keeps the first one only.
                    }
                    _ => {}
                }
            }
        }
        measures.get_or_insert(count);
        parts.push(ImportedPart {
            name: names.get(&id).cloned().unwrap_or_default(),
            clef,
            notes,
        });
    }
    let sections = read_sections(&fields, &marked, measures.unwrap_or(0));
    Ok(ImportedScore {
        title,
        meter,
        mode,
        fifths,
        parts,
        fields,
        sections,
    })
}

/// What a measure says of the sections of a mass movement, in any of its parts.
#[derive(Clone, Debug, Default)]
struct Marked {
    /// The text of its first rehearsal mark.
    rehearsal: Option<String>,
    /// A double bar line (`light-light`) at its left end.
    double_before: bool,
    /// A double bar line at its right end.
    double_after: bool,
}

impl Marked {
    /// Adds what `measure` says.
    fn read(&mut self, measure: roxmltree::Node<'_, '_>) {
        for element in measure.children().filter(roxmltree::Node::is_element) {
            match element.tag_name().name() {
                "direction" if self.rehearsal.is_none() => {
                    self.rehearsal = element
                        .descendants()
                        .find(|d| d.has_tag_name("rehearsal"))
                        .and_then(|r| text_of(Some(r)))
                        .filter(|text| !text.is_empty());
                }
                "barline" => {
                    let double =
                        text_of(child(element, "bar-style")).as_deref() == Some("light-light");
                    match element.attribute("location").unwrap_or("right") {
                        "left" => self.double_before |= double,
                        "right" => self.double_after |= double,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

/// The marks of sections of a file with `fields` whose measures, `measures` of them, say what
/// `marked` has ([`ImportedScore::sections`]).
fn read_sections(
    fields: &[(String, String)],
    marked: &[Marked],
    measures: usize,
) -> Vec<SectionMark> {
    // From the notation: a section where a rehearsal mark stands at the start or after a double
    // bar, and one at the start where a later one begins.
    let mut starts: BTreeSet<usize> = BTreeSet::new();
    for (m, measure) in marked.iter().enumerate().take(measures) {
        let after_double = m == 0 || measure.double_before || marked[m - 1].double_after;
        if measure.rehearsal.is_some() && after_double {
            starts.insert(m);
        }
    }
    if !starts.is_empty() {
        starts.insert(0);
    }
    let notation = starts.into_iter().map(|m| SectionMark {
        measure: m,
        kind: SectionKind::Section,
        title: marked[m].rehearsal.clone(),
    });
    // From the fields of the parts of a movement, where the file has them and they can be read;
    // a section of the notation where no part begins, after them.
    let parts = movement_parts(fields).unwrap_or_default();
    let mut sections = movement_sections(&parts);
    sections.extend(notation.filter(|mark| {
        parts.is_empty()
            || !parts
                .iter()
                .any(|part| part.bars.contains(&(mark.measure + 1)))
    }));
    sections.retain(|mark| mark.measure < measures);
    sections.sort_by_key(|mark| mark.measure);
    sections.dedup_by_key(|mark| mark.measure);
    sections
}

#[cfg(test)]
mod tests {
    use super::*;
    use fux_core::corpus::fux_examples;

    /// Every example of Fux written and read again gives the same notes.
    #[test]
    fn examples_survive_the_round_trip() {
        for example in fux_examples() {
            let score = example.score();
            let xml = to_musicxml(&score, &MusicXmlOptions::default());
            let read =
                from_musicxml(&xml).unwrap_or_else(|e| panic!("Fig. {}: {e}", example.figure));
            assert_eq!(
                read.parts.len(),
                score.voices.len(),
                "Fig. {}",
                example.figure
            );
            assert_eq!(read.meter, score.meter);
            assert_eq!(read.mode, Some(score.mode.mode));
            for (part, voice) in read.parts.iter().zip(&score.voices) {
                assert_eq!(part.notes, voice.notes, "Fig. {}", example.figure);
            }
        }
    }

    #[test]
    fn ficta_lyrics_and_clefs() {
        let mut score = fux_examples()
            .into_iter()
            .find(|e| e.figure == "5")
            .unwrap()
            .score();
        // A ficta note and lyrics on the cantus.
        let cp = score
            .voices
            .iter()
            .position(|v| !v.is_cantus_firmus())
            .unwrap();
        let last = score.voices[cp].notes.len() - 2;
        score.voices[cp].notes[last].ficta = true;
        let cf = 1 - cp;
        score.voices[cf].notes[0].syllable = Some("Ky-".into());
        score.voices[cf].notes[1].syllable = Some("ri-".into());
        score.voices[cf].notes[2].syllable = Some("e".into());
        let xml = to_musicxml(
            &score,
            &MusicXmlOptions {
                clefs: ClefStyle::Original,
                ..MusicXmlOptions::default()
            },
        );
        assert!(xml.contains("<accidental-mark placement=\"above\">sharp</accidental-mark>"));
        assert!(xml.contains("<syllabic>begin</syllabic><text>Ky</text>"));
        assert!(xml.contains("<syllabic>middle</syllabic><text>ri</text>"));
        assert!(xml.contains("<syllabic>end</syllabic><text>e</text>"));
        assert!(
            xml.contains("<sign>C</sign><line>3</line>"),
            "the alto clef"
        );
        assert!(xml.contains("(C. f.)"));
        let read = from_musicxml(&xml).unwrap();
        assert!(read.parts[cp].notes[last].ficta);
        assert_eq!(read.parts[cf].notes[1].syllable.as_deref(), Some("ri-"));
        assert_eq!(read.parts[cf].clef, Some(Clef::C3));
        assert_eq!(
            from_musicxml("<score-timewise/>"),
            Err(MusicXmlError::NotPartwise)
        );
        assert!(matches!(from_musicxml("<a"), Err(MusicXmlError::Xml(_))));
        let zero = xml.replacen("<divisions>2</divisions>", "<divisions>0</divisions>", 1);
        assert!(matches!(
            from_musicxml(&zero),
            Err(MusicXmlError::Unsupported { what, .. }) if what == "divisions"
        ));
    }

    /// What a check of the file needs besides the notes travels in fields of the identification:
    /// the mode with its ambitus, the internal cadences and the text the voices sing.
    #[test]
    fn fields_for_the_check() {
        use fux_core::text::{TextOptions, resolve};
        use fux_core::{Cadence, ScoreText};
        use std::sync::Arc;

        let mut score = fux_examples()
            .into_iter()
            .find(|e| e.figure == "5")
            .unwrap()
            .score();
        let read = from_musicxml(&to_musicxml(&score, &MusicXmlOptions::default())).unwrap();
        assert_eq!(
            read.fields,
            [(FIELD_MODE.to_string(), "dorian".to_string())]
        );
        assert_eq!(read.field(FIELD_TEXT), None);
        score.mode = "hypodorian-transposed".parse().unwrap();
        score.cadences = vec![
            Cadence {
                measure: 4,
                target: Step::A,
            },
            Cadence {
                measure: 8,
                target: Step::F,
            },
        ];
        let voices = vec![true; score.voices.len()];
        for spec in ["gloria:1-5", "Ad te le-vá-vi | á-ni-mam / me-am."] {
            let (text, _) = resolve(spec, &TextOptions::default()).unwrap();
            score.text = Some(ScoreText {
                text: Arc::new(text),
                voices: voices.clone(),
            });
            let xml = to_musicxml(&score, &MusicXmlOptions::default());
            let read = from_musicxml(&xml).unwrap();
            assert_eq!(read.field(FIELD_MODE), Some("hypodorian-transposed"));
            assert_eq!(read.field(FIELD_CADENCES), Some("5:A,9:F"));
            assert_eq!(read.field(FIELD_TEXT), Some(spec));
        }
    }

    /// More fields follow the score's own, escaped, and read back in their order; the notes stay
    /// those of the file without them, and without them the file is the one of `to_musicxml`.
    #[test]
    fn more_fields_follow_the_own_ones() {
        let score = fux_examples()
            .into_iter()
            .find(|e| e.figure == "5")
            .unwrap()
            .score();
        let options = MusicXmlOptions::default();
        let plain = to_musicxml(&score, &options);
        assert_eq!(to_musicxml_with_fields(&score, &options, &[]), plain);
        let extra = vec![
            (
                "fux-mass".to_string(),
                "preset=brevis;mode=dorian;choir=S,A,T,B;principal=A;seed=1".to_string(),
            ),
            (
                "fux-part".to_string(),
                "bars=1-11;title=<Kyrie & \"Christe\">".to_string(),
            ),
        ];
        let xml = to_musicxml_with_fields(&score, &options, &extra);
        assert!(
            xml.contains(
                "<miscellaneous-field name=\"fux-mode\">dorian</miscellaneous-field>\
                 <miscellaneous-field name=\"fux-mass\">preset=brevis;"
            ),
            "{xml}"
        );
        assert!(
            xml.contains(
                "<miscellaneous-field name=\"fux-part\">bars=1-11;title=&lt;Kyrie &amp; \
                 &quot;Christe&quot;&gt;</miscellaneous-field>"
            ),
            "{xml}"
        );
        let read = from_musicxml(&xml).unwrap();
        let mut fields = vec![(FIELD_MODE.to_string(), "dorian".to_string())];
        fields.extend(extra.iter().cloned());
        assert_eq!(read.fields, fields);
        assert_eq!(read.field("fux-part"), Some(extra[1].1.as_str()));
        assert_eq!(read.parts, from_musicxml(&plain).unwrap().parts);
    }

    /// The plan of an imitation travels in a field of its own and reads back as it was, accidentals
    /// and ties included; it fits the score made of the parts read. A field that cannot be read
    /// says where.
    #[test]
    fn imitation_travels_in_its_field() {
        use fux_core::{
            Role, ScoreEntry, ScoreImitation, ScorePoint, Species, Voice, VoiceType, parse_lilypond,
        };
        use std::sync::Arc;

        let notes = |src: &str| parse_lilypond(src, Duration::HALF).unwrap();
        let free = |voice_type, src: &str| {
            Voice::new(voice_type, Role::Counterpoint(Species::Free), notes(src))
        };
        let mut score = Score::new(
            "dorian".parse().unwrap(),
            vec![
                free(
                    VoiceType::Soprano,
                    "a'1 a'2 bes'2 c''2 bes'2 a'1 g'1 a'1 r1 a'1",
                ),
                free(
                    VoiceType::Alto,
                    "r1 d'1 d'2 e'2 f'1~ f'2 e'2 cis'2 d'2 d'1 d'1",
                ),
            ],
        );
        let read = from_musicxml(&to_musicxml(&score, &MusicXmlOptions::default())).unwrap();
        assert_eq!(read.field(FIELD_IMITATION), None);
        assert_eq!(read.imitation(), None);
        let head = |src: &str| -> Vec<(Pitch, u32)> {
            let mut out: Vec<(Pitch, u32)> = Vec::new();
            let mut tied = false;
            for note in notes(src) {
                match out.last_mut() {
                    Some(last) if tied => last.1 += note.duration.ticks(),
                    _ => out.push((note.pitch.unwrap(), note.duration.ticks())),
                }
                tied = note.tie;
            }
            out
        };
        let entry = |voice, start, rest, src: &str| ScoreEntry {
            voice,
            start,
            rest,
            notes: head(src),
        };
        let plan = ScoreImitation {
            points: vec![
                ScorePoint {
                    words: 0..7,
                    entries: vec![
                        entry(0, 0, 0, "a'1 a'2 bes'2 c''2 bes'2 a'1"),
                        entry(1, 8, 0, "d'1 d'2 e'2 f'1~ f'2"),
                    ],
                    goal: Some(5),
                },
                ScorePoint {
                    words: 3..7,
                    entries: vec![entry(1, 44, 4, "d'2 d'1"), entry(0, 56, 8, "a'1")],
                    goal: None,
                },
            ],
        };
        score.imitation = Some(Arc::new(plan.clone()));
        score.validate().unwrap();
        let xml = to_musicxml(&score, &MusicXmlOptions::default());
        assert!(
            xml.contains(&format!(
                "<miscellaneous-field name=\"fux-imitation\">{}</miscellaneous-field>",
                plan.spec()
            )),
            "{xml}"
        );
        let read = from_musicxml(&xml).unwrap();
        assert_eq!(read.field(FIELD_IMITATION), Some(plan.spec().as_str()));
        assert_eq!(read.imitation(), Some(Ok(plan.clone())));
        // The score made of the parts read takes the plan back.
        let mut again = Score::new(
            score.mode,
            read.parts
                .iter()
                .zip(&score.voices)
                .map(|(part, voice)| Voice::new(voice.voice_type, voice.role, part.notes.clone()))
                .collect(),
        );
        again.imitation = read.imitation().map(|plan| Arc::new(plan.unwrap()));
        assert_eq!(again.validate(), Ok(()));
        assert_eq!(again.imitation, score.imitation);
        // A field written by hand that cannot be read.
        let broken = xml.replace(&plan.spec(), "p0-7|0@0/0:X4.8");
        assert_eq!(
            from_musicxml(&broken).unwrap().imitation(),
            Some(Err(ScoreError::ImitationSyntax {
                part: "X4.8".into()
            }))
        );
    }

    /// The first line of lyrics is read, also on rests; a lyric that only ends an extender line
    /// gives no syllable. A lyric without a number is on the first line.
    #[test]
    fn reading_lyrics() {
        let note = |content: &str, lyrics: &str| {
            format!(
                "<note>{content}<duration>2</duration><voice>1</voice><type>half</type>{lyrics}</note>"
            )
        };
        let d = "<pitch><step>D</step><octave>4</octave></pitch>";
        let notes = [
            note(
                d,
                "<lyric><syllabic>single</syllabic><text>e</text><extend type=\"start\"/></lyric>\
                 <lyric number=\"2\"><syllabic>begin</syllabic><text>Chri</text></lyric>",
            ),
            note(
                d,
                "<lyric number=\"1\"><extend type=\"stop\"/></lyric>\
                 <lyric number=\"2\"><syllabic>end</syllabic><text>ste</text></lyric>",
            ),
            note(
                "<rest/>",
                "<lyric number=\"1\"><syllabic>begin</syllabic><text>Ký</text></lyric>",
            ),
            note(d, "<lyric number=\"2\"><text>e</text></lyric>"),
        ];
        let xml = format!(
            "<score-partwise version=\"4.0\"><part id=\"P1\"><measure number=\"1\">\
             <attributes><divisions>1</divisions></attributes>{}</measure></part></score-partwise>",
            notes.concat()
        );
        let read = from_musicxml(&xml).unwrap();
        let syllables: Vec<Option<&str>> = read.parts[0]
            .notes
            .iter()
            .map(|n| n.syllable.as_deref())
            .collect();
        assert_eq!(syllables, [Some("e"), None, Some("Ký-"), None]);
        assert!(read.parts[0].notes[2].is_rest());
    }
}
