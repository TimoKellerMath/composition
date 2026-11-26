//! LilyPond, MIDI and MusicXML export of Fux's examples, of a small score with every feature, of
//! a texted one and of an imitative one.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use fux_core::corpus::{fux_cantus, fux_examples};
use fux_core::{
    Duration, Lang, Mode, ModeSpec, Note, Pitch, Role, Score, ScoreEntry, ScoreImitation,
    ScorePoint, Species, Voice, VoiceType, parse_lilypond,
};
use fux_export::{
    ClefStyle, Color, FIELD_IMITATION, Graphic, Highlight, LilypondOptions, MidiOptions,
    MusicXmlOptions, Sheet, document, from_musicxml, lilypond_version, render, to_lilypond,
    to_midi, to_musicxml,
};
use midly::{MidiMessage, Smf, TrackEventKind};

fn example(figure: &str) -> Score {
    fux_examples()
        .into_iter()
        .find(|e| e.figure == figure)
        .unwrap_or_else(|| panic!("no Fig. {figure}"))
        .score()
}

/// Three voices with ficta, ties, rests, syllables and marked notes.
fn feature_score() -> Score {
    let notes = |s: &str| parse_lilypond(s, Duration::WHOLE).unwrap();
    let mut top = notes("r2 a'2~ a'4 g'8 f' e'4 d' cis''1 d''1");
    top[7].ficta = true;
    let mut cf = notes("d'1 f'1 e'1 d'1");
    for (note, syllable) in cf.iter_mut().zip(["Ky", "ri", "e", "son"]) {
        note.syllable = Some(syllable.to_string());
    }
    let bass = notes("d1 d1 a,1 d1");
    let mut score = Score::new(
        ModeSpec::new(Mode::Dorian),
        vec![
            Voice::new(VoiceType::Soprano, Role::Counterpoint(Species::Fifth), top),
            Voice::new(VoiceType::Alto, Role::CantusFirmus, cf),
            Voice::new(VoiceType::Bass, Role::Counterpoint(Species::First), bass),
        ],
    );
    score.title = Some("Test \"Kyrie\"".to_string());
    score.validate().unwrap();
    score
}

#[test]
fn lilypond_figure_5() {
    let options = LilypondOptions {
        interval_figures: true,
        ..LilypondOptions::default()
    };
    insta::assert_snapshot!(to_lilypond(&example("5"), &options));
}

/// The feature score with a marked note, a subtitle and a remark.
fn feature_sheet(score: &Score) -> Sheet<'_> {
    let mut sheet = Sheet::new(score);
    sheet.subtitle = Some("5. Gattung".to_string());
    sheet.highlights.push(Highlight {
        voice: 0,
        note: 3,
        color: Color::Red,
        label: Some("MEL-TEST".to_string()),
    });
    sheet.remarks.push("Takt 2: Beispielbemerkung".to_string());
    sheet
}

fn original_clefs_with_figures() -> LilypondOptions {
    LilypondOptions {
        clefs: ClefStyle::Original,
        interval_figures: true,
        ..LilypondOptions::default()
    }
}

#[test]
fn lilypond_features() {
    let score = feature_score();
    let sheet = feature_sheet(&score);
    insta::assert_snapshot!(document(
        Some("Sammlung"),
        &[sheet],
        &original_clefs_with_figures()
    ));
}

/// Notes from LilyPond input with one syllable or `_` each, rests and tied notes included.
fn sung(music: &str, syllables: &str) -> Vec<Note> {
    let mut notes = parse_lilypond(music, Duration::WHOLE).unwrap();
    let syllables: Vec<&str> = syllables.split_whitespace().collect();
    assert_eq!(notes.len(), syllables.len(), "{music}");
    for (note, syllable) in notes.iter_mut().zip(syllables) {
        note.syllable = (syllable != "_").then(|| syllable.to_string());
    }
    notes
}

/// Two voices singing Kyrie eleison, with or without the accents of the books: melismas within
/// words and at their ends, across a tie and up to a rest.
fn kyrie(accents: bool) -> Score {
    let text = |syllables: &str| {
        if accents {
            syllables.to_string()
        } else {
            syllables.replace('ý', "y").replace('é', "e")
        }
    };
    let soprano = sung(
        "a'2 d''2 | c''2 b'4 a'4 | b'2 r2 | d''2 c''4 b'4 | c''2 d''2~ | d''4 b'4 cis''2 | d''1",
        &text("Ký- ri- e _ _ _ _ e- lé- _ i- son. _ _ _ _"),
    );
    let tenor = sung(
        "r2 d'2 | f'2 g'2 | d'2 e'4 f'4 | g'2 a'2~ | a'4 g'4 f'2 | g'2 e'2 | d'1",
        &text("_ Ký- ri- e _ _ _ e- lé- _ _ i- son. _ _"),
    );
    let free = Role::Counterpoint(Species::Free);
    let mut score = Score::new(
        ModeSpec::new(Mode::Dorian),
        vec![
            Voice::new(VoiceType::Soprano, free, soprano),
            Voice::new(VoiceType::Tenor, free, tenor),
        ],
    );
    score.title = Some("Kyrie".to_string());
    score.validate().unwrap();
    score
}

/// The Kyrie with and without accents, with interval figures.
fn kyrie_document() -> String {
    let (accented, plain) = (kyrie(true), kyrie(false));
    let mut sheets = [Sheet::new(&accented), Sheet::new(&plain)];
    sheets[0].subtitle = Some("mit Akzenten".to_string());
    sheets[1].subtitle = Some("ohne Akzente".to_string());
    let options = LilypondOptions {
        interval_figures: true,
        ..LilypondOptions::default()
    };
    document(Some("Kyrie"), &sheets, &options)
}

/// Hyphens and extender lines; the interval figures between the staves go below the upper
/// voice's text, which stays next to its staff.
#[test]
fn lyrics_with_extenders() {
    let text = kyrie_document();
    assert!(
        text.contains(
            r#"\new Lyrics = "text0" \lyricsto "voice0" { "Ký" -- "ri" -- "e" __ _ _ _ "e" -- "lé" -- _ "i" -- "son." __ _ _ _ }"#
        ),
        "{text}"
    );
    assert!(
        text.contains(
            r#"\new Lyrics = "text1" \lyricsto "voice1" { "Ky" -- "ri" -- "e" __ _ _ _ "e" -- "le" -- _ "i" -- "son." __ _ _ }"#
        ),
        "{text}"
    );
    assert_eq!(
        text.matches(r#"\new Lyrics \with { alignBelowContext = "text0" }"#)
            .count(),
        2,
        "{text}"
    );
    // Once for all its scores, the document gives the hyphens room and keeps every line of text
    // clear of the staff below it, in LilyPond's staff size; unlike a book, it allows a page with
    // a single system.
    for setting in [
        "\\override LyricHyphen.minimum-distance = #0.5\n",
        "\\override VerticalAxisGroup.nonstaff-unrelatedstaff-spacing.minimum-distance = #7\n",
    ] {
        assert_eq!(text.matches(setting).count(), 1, "{setting}: {text}");
    }
    assert!(!text.contains("min-systems-per-page"), "{text}");
    assert!(!text.contains("set-global-staff-size"), "{text}");
    insta::assert_snapshot!(text);
}

/// Two voices of an imitation as its plan says: the alto takes up the soprano's head a measure
/// later at the fifth below, its last note tied over the barline; in the second point the alto
/// leads after a rest and the soprano follows on the minim beat.
fn imitative_score() -> Score {
    let notes = |src: &str| parse_lilypond(src, Duration::HALF).unwrap();
    let free = Role::Counterpoint(Species::Free);
    let mut score = Score::new(
        ModeSpec::new(Mode::Dorian),
        vec![
            Voice::new(
                VoiceType::Soprano,
                free,
                notes("a'1 a'2 bes'2 c''2 bes'2 a'1 g'1 f'1 r2 a'2 b'2 a'2"),
            ),
            Voice::new(
                VoiceType::Alto,
                free,
                notes("r1 d'1 d'2 e'2 f'1~ f'2 e'2 cis'2 r2 d'2 e'2 d'1"),
            ),
        ],
    );
    let entry = |voice, start, rest, head: &[(&str, u32)]| ScoreEntry {
        voice,
        start,
        rest,
        notes: head
            .iter()
            .map(|&(pitch, ticks)| (pitch.parse::<Pitch>().unwrap(), ticks))
            .collect(),
    };
    score.imitation = Some(Arc::new(ScoreImitation {
        points: vec![
            ScorePoint {
                words: 0..7,
                entries: vec![
                    entry(
                        0,
                        0,
                        0,
                        &[
                            ("A4", 8),
                            ("A4", 4),
                            ("Bb4", 4),
                            ("C5", 4),
                            ("Bb4", 4),
                            ("A4", 8),
                        ],
                    ),
                    entry(1, 8, 0, &[("D4", 8), ("D4", 4), ("E4", 4), ("F4", 12)]),
                ],
                goal: Some(5),
            },
            ScorePoint {
                words: 3..7,
                entries: vec![
                    entry(1, 48, 4, &[("D4", 4), ("E4", 4), ("D4", 8)]),
                    entry(0, 52, 4, &[("A4", 4), ("B4", 4), ("A4", 4)]),
                ],
                goal: None,
            },
        ],
    }));
    score.title = Some("Imitation".to_string());
    score.validate().unwrap();
    score
}

/// A bracket over each entry's head from its first note to its last, a tied one included, with
/// the number of its point over its first note; above the staff, by the engraver of the voices,
/// the number above the bracket. None without the marks.
#[test]
fn lilypond_entry_brackets() {
    let score = imitative_score();
    let text = to_lilypond(&score, &LilypondOptions::default());
    assert!(
        text.contains(
            "a'1\\startGroup^\\markup \\small \"1\" | a'2 bes'2 | c''2 bes'2 | a'1\\stopGroup"
        ),
        "{text}"
    );
    assert!(
        text.contains("f'1~ | f'2\\stopGroup e'2"),
        "the bracket ends on the tied note: {text}"
    );
    assert_eq!(text.matches("\\startGroup").count(), 4, "{text}");
    assert_eq!(text.matches("\\stopGroup").count(), 4, "{text}");
    assert_eq!(text.matches("\\startGroup^\\markup \\small \"").count(), 4);
    assert!(!text.contains("HorizontalBracketText"), "{text}");
    assert!(text.contains(
        "      \\consists \"Horizontal_bracket_engraver\"\n      \\override HorizontalBracket.direction = #UP\n      \\override TextScript.outside-staff-priority = #1000\n"
    ));
    insta::assert_snapshot!(text);
    let plain = to_lilypond(
        &score,
        &LilypondOptions {
            entry_marks: false,
            ..LilypondOptions::default()
        },
    );
    assert!(!plain.contains("startGroup") && plain.contains("\\layout { }"));
}

/// In MusicXML a bracket starts with the point's number as words above the first note of each
/// entry's head and stops above its last, both on the part's staff, in the order of the schema;
/// the plan travels in its field. Read back, the notes and the plan are those written.
#[test]
fn musicxml_entry_brackets() {
    let score = imitative_score();
    let xml = to_musicxml(&score, &MusicXmlOptions::default());
    let parsed = roxmltree::Document::parse_with_options(
        &xml,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .unwrap();
    let parts: Vec<_> = parsed
        .root_element()
        .children()
        .filter(|c| c.has_tag_name("part"))
        .collect();
    // Per part: the notes (by index) before which a bracket starts with its point, and stops.
    let brackets = [
        (vec![(0, 1), (9, 2)], vec![5, 11]),
        (vec![(1, 1), (9, 2)], vec![5, 11]),
    ];
    for (part, (starts, stops)) in parts.iter().zip(brackets) {
        let mut found_starts = Vec::new();
        let mut found_stops = Vec::new();
        let mut note = 0;
        for element in part
            .descendants()
            .filter(|n| n.has_tag_name("note") || n.has_tag_name("direction"))
        {
            if element.has_tag_name("note") {
                note += 1;
                continue;
            }
            assert_eq!(element.attribute("placement"), Some("above"));
            // The order of the schema: the direction types, then the voice and the staff, which
            // Verovio needs to close a bracket from the third staff on.
            let children: Vec<&str> = element
                .children()
                .filter(roxmltree::Node::is_element)
                .map(|c| c.tag_name().name())
                .collect();
            let bracket = element
                .descendants()
                .find(|n| n.has_tag_name("bracket"))
                .unwrap();
            assert_eq!(bracket.attribute("line-end"), Some("down"));
            let staff = element.children().find(|c| c.has_tag_name("staff"));
            assert_eq!(staff.and_then(|s| s.text()), Some("1"));
            match bracket.attribute("type") {
                Some("start") => {
                    assert_eq!(
                        children,
                        ["direction-type", "direction-type", "voice", "staff"]
                    );
                    let words = element
                        .descendants()
                        .find(|n| n.has_tag_name("words"))
                        .and_then(|w| w.text())
                        .unwrap();
                    found_starts.push((note, words.parse::<usize>().unwrap()));
                }
                Some("stop") => {
                    assert_eq!(children, ["direction-type", "voice", "staff"]);
                    found_stops.push(note);
                }
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(found_starts, starts);
        assert_eq!(found_stops, stops);
    }
    let read = from_musicxml(&xml).unwrap();
    for (part, voice) in read.parts.iter().zip(&score.voices) {
        assert_eq!(part.notes, voice.notes, "{}", voice.name);
    }
    assert_eq!(
        read.field(FIELD_IMITATION),
        score.imitation.as_ref().map(|plan| plan.spec()).as_deref()
    );
    assert_eq!(
        read.imitation().map(Result::unwrap),
        score.imitation.as_deref().cloned()
    );
    validate_musicxml(&xml, "imitation.musicxml");
    let plain = to_musicxml(
        &score,
        &MusicXmlOptions {
            entry_marks: false,
            ..MusicXmlOptions::default()
        },
    );
    assert!(!plain.contains("<direction") && plain.contains(FIELD_IMITATION));
}

/// A MusicXML file as a document, its document type allowed.
fn parse(xml: &str) -> roxmltree::Document<'_> {
    roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .unwrap()
}

/// The text of the first element named `name` below `node`.
fn text_of<'a>(node: roxmltree::Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.descendants()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
}

/// The head of a file: per credit its type and its words.
fn credits<'a>(doc: &'a roxmltree::Document<'_>) -> Vec<(&'a str, &'a str)> {
    doc.root_element()
        .children()
        .filter(|c| c.has_tag_name("credit"))
        .map(|c| {
            (
                text_of(c, "credit-type").unwrap(),
                text_of(c, "credit-words").unwrap(),
            )
        })
        .collect()
}

/// Fux's Dorian cantus firmus alone in the alto, a score of one voice as `fux cf` writes it.
fn cantus_firmus() -> Score {
    let cantus = fux_cantus().into_iter().find(|c| c.id == "dorian").unwrap();
    let notes = cantus
        .notes
        .iter()
        .map(|&pitch| Note::new(pitch, Duration::WHOLE))
        .collect();
    let mut score = Score::new(
        cantus.mode,
        vec![Voice::new(VoiceType::Alto, Role::CantusFirmus, notes)],
    );
    score.title = Some("Cantus firmus 1".to_string());
    score.validate().unwrap();
    score
}

/// A single score in MusicXML has the part list, the page and the tempo of the file of a
/// movement, with a head of its own (M1 (b)): its title as the work's title and on the first
/// page, under it its mode and its voices from the top in the language of the options, and with
/// a seed the line „fux-kontrapunkt, Seed N“ („seed“ in English); from two voices on a bracket
/// around its parts with the bar lines through the staves (a cantus firmus alone has none, as in
/// its LilyPond score); each voice a choir voice of its range under the abbreviation of its type,
/// on its MIDI channel with the program of the options; the tempo of the options, as many
/// quarters a minute as twice its halves, in the first measure of the first part, after its
/// attributes, and nowhere else.
#[test]
fn musicxml_head_and_choir_voices() {
    let score = feature_score();
    let xml = to_musicxml(&score, &MusicXmlOptions::default());
    validate_musicxml(&xml, "head.musicxml");
    let doc = parse(&xml);
    let root = doc.root_element();
    assert_eq!(text_of(root, "work-title"), Some("Test \"Kyrie\""));
    assert_eq!(
        (text_of(root, "millimeters"), text_of(root, "tenths")),
        (Some("7"), Some("40"))
    );
    assert_eq!(
        (text_of(root, "page-height"), text_of(root, "page-width")),
        (Some("1697"), Some("1200"))
    );
    assert_eq!(
        credits(&doc),
        [("title", "Test \"Kyrie\""), ("subtitle", "dorisch, S A B")]
    );
    // The part list: the bracket, then each voice as a choir voice.
    let list: Vec<roxmltree::Node> = root
        .descendants()
        .find(|n| n.has_tag_name("part-list"))
        .unwrap()
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    assert_eq!(list.len(), score.voices.len() + 2);
    let (start, stop) = (list[0], list[list.len() - 1]);
    assert_eq!(
        (start.tag_name().name(), start.attribute("type")),
        ("part-group", Some("start"))
    );
    assert_eq!(text_of(start, "group-symbol"), Some("bracket"));
    assert_eq!(text_of(start, "group-barline"), Some("yes"));
    assert_eq!(
        (stop.tag_name().name(), stop.attribute("type")),
        ("part-group", Some("stop"))
    );
    let parts: Vec<_> = list[1..list.len() - 1]
        .iter()
        .map(|part| {
            (
                text_of(*part, "part-name").unwrap(),
                text_of(*part, "part-abbreviation").unwrap(),
                text_of(*part, "instrument-name").unwrap(),
                text_of(*part, "instrument-sound").unwrap(),
                text_of(*part, "midi-channel").unwrap(),
                text_of(*part, "midi-program").unwrap(),
            )
        })
        .collect();
    assert_eq!(
        parts,
        [
            ("Sopran", "S", "Soprano", "voice.soprano", "1", "53"),
            ("Alt (C. f.)", "A", "Alto", "voice.alto", "2", "53"),
            ("Bass", "B", "Bass", "voice.bass", "3", "53"),
        ]
    );
    // The tempo: 72 halves a minute as in the MIDI file.
    let sounds: Vec<(String, Option<&str>)> = root
        .descendants()
        .filter(|n| n.has_tag_name("sound"))
        .map(|sound| {
            let measure = sound.parent().unwrap();
            let part = measure.parent().unwrap();
            let place = format!(
                "{} {}",
                part.attribute("id").unwrap(),
                measure.attribute("number").unwrap()
            );
            (place, sound.attribute("tempo"))
        })
        .collect();
    assert_eq!(sounds, [("P1 1".to_string(), Some("144"))]);
    assert!(xml.contains("      </attributes>\n      <sound tempo=\"144\"/>\n"));
    // With a seed, in English, another tempo and another program.
    let options = MusicXmlOptions {
        tempo: 60,
        program: 20,
        lang: Lang::En,
        seed: Some(4711),
        ..MusicXmlOptions::default()
    };
    let xml = to_musicxml(&score, &options);
    validate_musicxml(&xml, "head-en.musicxml");
    let doc = parse(&xml);
    assert_eq!(
        credits(&doc),
        [
            ("title", "Test \"Kyrie\""),
            ("subtitle", "Dorian, S A B"),
            ("composer", "fux-kontrapunkt, seed 4711")
        ]
    );
    assert_eq!(xml.matches("<midi-program>20</midi-program>").count(), 3);
    assert_eq!(xml.matches("<sound ").count(), 1);
    assert!(xml.contains("<sound tempo=\"120\"/>"));
    let german = to_musicxml(
        &score,
        &MusicXmlOptions {
            seed: Some(4711),
            ..MusicXmlOptions::default()
        },
    );
    assert!(german.contains(">fux-kontrapunkt, Seed 4711</credit-words>"));
    // Without a title, the subtitle stays.
    let mut untitled = score.clone();
    untitled.title = None;
    let xml = to_musicxml(&untitled, &MusicXmlOptions::default());
    assert!(!xml.contains("<work>"));
    assert_eq!(credits(&parse(&xml)), [("subtitle", "dorisch, S A B")]);
    // A single voice: its choir voice, head and tempo, but no bracket.
    let options = MusicXmlOptions {
        program: 20,
        seed: Some(7),
        ..MusicXmlOptions::default()
    };
    let xml = to_musicxml(&cantus_firmus(), &options);
    validate_musicxml(&xml, "cantus.musicxml");
    let doc = parse(&xml);
    assert_eq!(
        credits(&doc),
        [
            ("title", "Cantus firmus 1"),
            ("subtitle", "dorisch, A"),
            ("composer", "fux-kontrapunkt, Seed 7")
        ]
    );
    assert!(!xml.contains("<part-group"));
    let list: Vec<roxmltree::Node> = doc
        .descendants()
        .find(|n| n.has_tag_name("part-list"))
        .unwrap()
        .children()
        .filter(roxmltree::Node::is_element)
        .collect();
    assert_eq!(list.len(), 1);
    assert_eq!(
        [
            "part-name",
            "part-abbreviation",
            "instrument-name",
            "instrument-sound",
            "midi-channel",
            "midi-program"
        ]
        .map(|name| text_of(list[0], name).unwrap()),
        ["Alt (C. f.)", "A", "Alto", "voice.alto", "1", "20"]
    );
    assert_eq!(xml.matches("<score-instrument ").count(), 1);
    assert_eq!(xml.matches("<sound ").count(), 1);
    assert!(xml.contains("      </attributes>\n      <sound tempo=\"144\"/>\n"));
}

/// `xml`, the file of a single score, as fux-kontrapunkt wrote it before M1: without the staff of
/// the brackets over the entries, the page, the head, the bracket around the parts, the choir
/// voices and the tempo.
fn before_m1(xml: &str) -> String {
    let mut old = String::new();
    for line in xml.lines() {
        if line == "        <staff>1</staff>"
            || line.starts_with("  <defaults>")
            || line.starts_with("  <credit ")
            || line.starts_with("    <part-group ")
            || line.starts_with("      <sound tempo=")
        {
            continue;
        }
        let instruments = (
            line.find("<score-instrument "),
            line.find("</midi-instrument>"),
        );
        match instruments {
            (Some(from), Some(to)) => {
                old.push_str(&line[..from]);
                old.push_str(&line[to + "</midi-instrument>".len()..]);
            }
            _ => old.push_str(line),
        }
        old.push('\n');
    }
    old
}

/// The reader takes the file of a single score as it took the file fux-kontrapunkt wrote before
/// M1, without the staff of the brackets, the page, the head, the bracket around the parts, the
/// choir voices and the tempo: the same title, meter, mode, fields and parts, note by note; for
/// Fux's examples and a score with every feature, a texted and an imitative one and a cantus
/// firmus alone.
#[test]
fn musicxml_reads_as_before() {
    let mut scores: Vec<Score> = fux_examples().iter().map(|e| e.score()).collect();
    scores.extend([
        feature_score(),
        kyrie(true),
        imitative_score(),
        cantus_firmus(),
    ]);
    let options = MusicXmlOptions {
        seed: Some(1),
        ..MusicXmlOptions::default()
    };
    for score in &scores {
        let xml = to_musicxml(score, &options);
        let old = before_m1(&xml);
        for added in [
            "<staff>",
            "<defaults>",
            "<credit",
            "<part-group",
            "<score-instrument",
            "<sound",
        ] {
            assert!(!old.contains(added), "{added}: {old}");
        }
        assert_eq!(from_musicxml(&old).unwrap(), from_musicxml(&xml).unwrap());
    }
}

/// Validates MusicXML with xmllint against the MusicXML 4.0 schema, if `FUX_MUSICXML_XSD` names
/// a copy of `musicxml.xsd` (with `xml.xsd` and `xlink.xsd` beside it).
fn validate_musicxml(xml: &str, name: &str) {
    let Some(xsd) = std::env::var_os("FUX_MUSICXML_XSD") else {
        eprintln!("FUX_MUSICXML_XSD not set, skipping the schema validation");
        return;
    };
    let file = tmp_dir("musicxml").join(name);
    std::fs::write(&file, xml).unwrap();
    let output = Command::new("xmllint")
        .args(["--noout", "--nonet", "--schema"])
        .arg(xsd)
        .arg(&file)
        .output()
        .expect("xmllint");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Extender lines in MusicXML start in the lyric of a word-final syllable and stop in a lyric of
/// their own on the last attack of the melisma; rests and tied notes get no lyric. Read back, the
/// notes keep their syllables.
#[test]
fn musicxml_extend_round_trip() {
    let score = kyrie(true);
    let xml = to_musicxml(&score, &MusicXmlOptions::default());
    let parsed = roxmltree::Document::parse_with_options(
        &xml,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .unwrap();
    let parts: Vec<_> = parsed
        .root_element()
        .children()
        .filter(|c| c.has_tag_name("part"))
        .collect();
    // Per part, the notes where a line starts and where it stops.
    let lines = [([2, 11], [5, 15]), ([3, 12], [6, 14])];
    assert_eq!(parts.len(), lines.len());
    for (part, (starts, stops)) in parts.iter().zip(lines) {
        let notes: Vec<_> = part
            .descendants()
            .filter(|n| n.has_tag_name("note"))
            .collect();
        let extends = |kind: &str| -> Vec<usize> {
            (0..notes.len())
                .filter(|&i| {
                    notes[i]
                        .descendants()
                        .any(|e| e.has_tag_name("extend") && e.attribute("type") == Some(kind))
                })
                .collect()
        };
        assert_eq!(extends("start"), starts);
        assert_eq!(extends("stop"), stops);
        for note in &notes {
            let lyrics: Vec<_> = note
                .children()
                .filter(|c| c.has_tag_name("lyric"))
                .collect();
            let silent = note.children().any(|c| {
                c.has_tag_name("rest")
                    || (c.has_tag_name("tie") && c.attribute("type") == Some("stop"))
            });
            assert!(
                !silent || lyrics.is_empty(),
                "a lyric on a rest or a tied note"
            );
            for lyric in lyrics {
                // The order of the schema: syllabic, text, extend; or an extend alone.
                let children: Vec<&str> = lyric
                    .children()
                    .filter(roxmltree::Node::is_element)
                    .map(|c| c.tag_name().name())
                    .collect();
                assert!(
                    matches!(
                        children.as_slice(),
                        ["syllabic", "text"] | ["syllabic", "text", "extend"] | ["extend"]
                    ),
                    "{children:?}"
                );
            }
        }
    }
    let read = from_musicxml(&xml).unwrap();
    for (part, voice) in read.parts.iter().zip(&score.voices) {
        assert_eq!(part.notes, voice.notes, "{}", voice.name);
    }
    validate_musicxml(&xml, "kyrie.musicxml");
}

#[test]
fn midi_round_trip() {
    let score = example("82");
    let bytes = to_midi(&score, &MidiOptions::default());
    let smf = Smf::parse(&bytes).unwrap();
    assert_eq!(smf.tracks.len(), 1 + score.voices.len());
    for (voice, track) in score.voices.iter().zip(&smf.tracks[1..]) {
        let ons: Vec<u8> = track
            .iter()
            .filter_map(|e| match e.kind {
                TrackEventKind::Midi {
                    message: MidiMessage::NoteOn { key, .. },
                    ..
                } => Some(key.as_int()),
                _ => None,
            })
            .collect();
        let expected: Vec<u8> = voice.pitches().iter().map(|p| p.midi() as u8).collect();
        assert_eq!(ons, expected, "{}", voice.name);
    }
}

fn tmp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs only where LilyPond is installed (not in CI).
#[test]
fn lilypond_compiles_all_examples() {
    if lilypond_version().is_none() {
        eprintln!("LilyPond not installed, skipping");
        return;
    }
    let scores: Vec<Score> = fux_examples().iter().map(|e| e.score()).collect();
    let sheets: Vec<Sheet> = scores.iter().map(Sheet::new).collect();
    let options = LilypondOptions {
        interval_figures: true,
        ..LilypondOptions::default()
    };
    let dir = tmp_dir("all-examples");
    let ly = dir.join("gradus.ly");
    std::fs::write(&ly, document(Some("Gradus"), &sheets, &options)).unwrap();
    let pdf = render(&ly, &dir.join("gradus"), Graphic::Pdf).unwrap();
    assert_eq!(pdf, [dir.join("gradus.pdf")]);

    let feature = feature_score();
    let ly = dir.join("features.ly");
    let text = document(
        Some("Sammlung"),
        &[feature_sheet(&feature)],
        &original_clefs_with_figures(),
    );
    std::fs::write(&ly, text).unwrap();
    let svg = render(&ly, &dir.join("features"), Graphic::Svg).unwrap();
    assert!(!svg.is_empty());

    let ly = dir.join("kyrie.ly");
    std::fs::write(&ly, kyrie_document()).unwrap();
    let svg = render(&ly, &dir.join("kyrie"), Graphic::Svg).unwrap();
    assert!(!svg.is_empty());

    // The brackets over the entries of an imitation, without a warning.
    let ly = dir.join("imitation.ly");
    let score = imitative_score();
    std::fs::write(&ly, to_lilypond(&score, &LilypondOptions::default())).unwrap();
    let output = Command::new(fux_export::render::lilypond_command())
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .arg("--svg")
        .arg("-o")
        .arg(dir.join("imitation"))
        .arg(&ly)
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{log}");
    assert!(!log.to_lowercase().contains("warning"), "{log}");
}
