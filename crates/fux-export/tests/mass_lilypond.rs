//! The book of a mass in LilyPond (phase 14, docs/design/phase-14/spec.md §§ 6 and 8): a small
//! mass for three voices made by hand, with and without Mensurstriche, and the merging of ties
//! over barlines on the examples.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use fux_core::generate::Generated;
use fux_core::mass::{
    Join, Mass, MassPlan, MassRequest, Movement, PartRef, PartResult, parse_scoring, plan_mass,
};
use fux_core::rules::Report;
use fux_core::text::resolve;
use fux_core::{
    Duration, Lyric, ModeSpec, Pitch, Role, Score, ScoreEntry, ScoreImitation, ScorePoint,
    ScoreText, Species, Voice, VoiceType, parse_lilypond,
};
use fux_export::{
    ClefStyle, FIELD_MODE, LilypondOptions, MassBookOptions, Sheet, document, from_musicxml,
    lilypond_version, mass_book, merge_ties,
};

/// A voice of a part made by hand: its notes as LilyPond input, and per note `x` where it begins
/// a syllable, `F` where it does and is ficta, `f` for a ficta note within a syllable, `_` for
/// any other note, a tied one or a rest.
type Line = (&'static str, &'static str);

/// An entry of a point made by hand: its voice in the part, its start, and the pitches and
/// values of its head, tied notes merged.
type Head = (usize, u32, &'static [(&'static str, u32)]);

/// The part at `at` of `plan` sung by its section's voices as `lines` give them, from the top:
/// the text `spec`, each line singing it through; with an imitation of one point of the entries
/// `heads`, if there are any.
fn handmade(
    plan: &MassPlan,
    at: PartRef,
    spec: &str,
    lines: &[Line],
    heads: &[Head],
) -> PartResult {
    let movement = &plan.movements[at.movement];
    let voices = movement.sections[at.section].voices.clone();
    assert_eq!(voices.len(), lines.len(), "{spec}");
    let (text, _) = resolve(spec, &plan.text).unwrap();
    let syllables = text.syllables.len();
    let parts: Vec<Voice> = voices
        .iter()
        .zip(lines)
        .map(|(&v, &(music, marks))| {
            let mut notes = parse_lilypond(music, Duration::WHOLE).unwrap();
            let marks: Vec<char> = marks.split_whitespace().flat_map(str::chars).collect();
            assert_eq!(notes.len(), marks.len(), "{spec}: {music}");
            let mut sung = 0;
            for (note, mark) in notes.iter_mut().zip(marks) {
                note.ficta = matches!(mark, 'F' | 'f');
                if matches!(mark, 'x' | 'F') {
                    note.lyric = Some(Lyric::new(sung));
                    sung += 1;
                }
            }
            assert_eq!(sung, syllables, "{spec}: {music}");
            Voice::new(movement.voices[v], Role::Counterpoint(Species::Free), notes)
        })
        .collect();
    let mut score = Score::new(movement.mode, parts);
    score.text = Some(ScoreText {
        text: Arc::new(text),
        voices: vec![true; voices.len()],
    });
    if !heads.is_empty() {
        let entries = heads
            .iter()
            .map(|&(voice, start, head)| ScoreEntry {
                voice,
                start,
                rest: 0,
                notes: head
                    .iter()
                    .map(|&(pitch, ticks)| (pitch.parse::<Pitch>().unwrap(), ticks))
                    .collect(),
            })
            .collect();
        score.imitation = Some(Arc::new(ScoreImitation {
            points: vec![ScorePoint {
                words: 0..syllables as u16,
                entries,
                goal: None,
            }],
        }));
    }
    score.fill_syllables(true);
    score.validate().unwrap();
    PartResult {
        at,
        seed: 1,
        attempt: 0,
        generated: Generated {
            score,
            cost: 0,
            report: Report::default(),
            warnings: Vec::new(),
            subjects: Vec::new(),
        },
        seconds: 0.0,
        warnings: Vec::new(),
        voices,
        profile: 0,
        motif: None,
        kept: None,
    }
}

fn at(movement: usize, section: usize, part: usize) -> PartRef {
    PartRef {
        movement,
        section,
        part,
    }
}

/// A small Missa brevis for A T B in the Dorian mode, its parts made by hand: the Kyrie with a
/// Christe for A T and without its Kyrie II, which is missing; the Gloria in one section of
/// three short parts, the third after the Break at Qui tollis; the Agnus Dei with a second tenor
/// in Agnus Dei II.
fn small_mass() -> Mass {
    let request = MassRequest {
        choir: vec![VoiceType::Alto, VoiceType::Tenor, VoiceType::Bass],
        movements: vec![Movement::Kyrie, Movement::Gloria, Movement::Agnus],
        overrides: parse_scoring("christe=A T; agnus3=A T T B").unwrap(),
        seed: 1,
        ..MassRequest::default()
    };
    let mut plan = plan_mass(&request).unwrap();
    // Short texts in place of the Gloria's and the Agnus Dei's own.
    let gloria = &mut plan.movements[1].sections[0];
    let mut parts = vec![
        gloria.parts[0].clone(),
        gloria.parts[1].clone(),
        gloria.parts[3].clone(),
    ];
    assert_eq!(
        parts.iter().map(|p| p.join).collect::<Vec<_>>(),
        [Join::Start, Join::Seam, Join::Break("Qui tollis")]
    );
    for (part, (text, syllables)) in
        parts
            .iter_mut()
            .zip([("gloria:2", 4), ("gloria:3", 6), ("qui-tollis:1", 8)])
    {
        part.text = text.to_string();
        part.syllables = syllables;
    }
    parts[2].continued = false;
    gloria.parts = parts;
    let agnus = &mut plan.movements[2];
    for (section, text, syllables) in [(0, "agnus:1", 4), (1, "agnus3:3", 6)] {
        agnus.sections[section].parts[0].text = text.to_string();
        agnus.sections[section].parts[0].syllables = syllables;
    }
    assert_eq!(agnus.names, ["Alt", "Tenor 1", "Tenor 2", "Bass"]);
    assert_eq!(agnus.sections[0].voices, [0, 1, 3]);
    assert_eq!(plan.movements[0].sections[1].voices, [0, 1]);
    let plan = Arc::new(plan);
    let mut m = Mass::new(Arc::clone(&plan));
    // Kyrie I in imitation: the alto leads, the tenor answers a measure later, the bass after
    // two.
    m.insert(handmade(
        &plan,
        at(0, 0, 0),
        "kyrie",
        &[
            (
                "d'1 | f'2 g'2 | a'1~ | a'2 g'2 | f'2 e'2~ | e'4 d'4 d'2~ | d'2 cis'2 | d'1",
                "x x x _ _ x x _ _ _ x _ f x",
            ),
            (
                "r1 | a1 | c'2 d'2 | e'1~ | e'2 d'2~ | d'4 c'4 b2 | a1~ | a1",
                "_ x x x x _ x _ _ x x _",
            ),
            (
                "r1 | r1 | d1 | f2 g2 | a1~ | a2 g2 | a1 | d1",
                "_ _ x x x x _ x x x",
            ),
        ],
        &[
            (0, 0, &[("D4", 8), ("F4", 4), ("G4", 4), ("A4", 12)]),
            (1, 8, &[("A3", 8), ("C4", 4), ("D4", 4), ("E4", 12)]),
            (2, 16, &[("D3", 8), ("F3", 4), ("G3", 4), ("A3", 12)]),
        ],
    ));
    // The Christe for A T; the alto holds an F sharp in the staff over a barline, then sings F.
    m.insert(handmade(
        &plan,
        at(0, 1, 0),
        "christe",
        &[
            (
                "a'1 | g'2 f'2 | e'2 fis'2~ | fis'4 g'4 f'2 | e'1~ | e'1",
                "x x x x _ _ _ x x _",
            ),
            (
                "r1 | d'1 | c'2 a2 | d'2 c'2~ | c'2 b2 | a1",
                "_ x x x x x _ _ x",
            ),
        ],
        &[],
    ));
    // The Gloria: after the first part a seam (a minim's breath), then the Break at Qui tollis
    // (a measure's rest).
    m.insert(handmade(
        &plan,
        at(1, 0, 0),
        "gloria:2",
        &[
            ("a'2 a'2 | g'2 a'2~ | a'2 g'2 | a'1", "x x x x _ _ _"),
            ("d'2 d'2 | c'2 d'2~ | d'2 c'2 | d'1", "x x x x _ _ _"),
            ("d2 d2 | e2 f2 | g2 a2 | d1", "x x x x _ _ _"),
        ],
        &[],
    ));
    m.insert(handmade(
        &plan,
        at(1, 0, 1),
        "gloria:3",
        &[
            (
                "r2 a'2 | a'2 bes'2~ | bes'2 a'2 | g'2 a'2~ | a'1",
                "_ x x F f x x x _",
            ),
            (
                "r2 f'2 | f'2 d'2 | g'2 f'2~ | f'2 e'2 | d'1",
                "_ x x x x x _ x _",
            ),
            ("r2 d2 | d2 g2 | g2 d2 | g2 a2 | d1", "_ x x x x x x _ _"),
        ],
        &[],
    ));
    m.insert(handmade(
        &plan,
        at(1, 0, 2),
        "qui-tollis:1",
        &[
            (
                "r1 | a'2 a'2 | g'2 f'2 | e'2 d'2 | e'2 f'2~ | f'2 e'2 | d'1",
                "_ x x x x x x x x _ _ _",
            ),
            (
                "r1 | d'2 d'2 | d'2 a2 | c'2 b2 | c'2 a2 | a1~ | a1",
                "_ x x x x x x x x _ _",
            ),
            (
                "r1 | d2 f2 | g2 d2 | a2 g2 | c2 f2 | d1~ | d1",
                "_ x x x x x x x x _ _",
            ),
        ],
        &[],
    ));
    // Agnus Dei I for A T B, Agnus Dei II with both tenors.
    m.insert(handmade(
        &plan,
        at(2, 0, 0),
        "agnus:1",
        &[
            ("d'2 f'2 | g'2 a'2~ | a'2 g'2 | a'1", "x x x x _ _ _"),
            ("a2 d'2 | d'2 c'2~ | c'2 b2 | a1", "x x x x _ _ _"),
            ("d2 d2 | g2 a2 | f2 g2 | d1", "x x x x _ _ _"),
        ],
        &[],
    ));
    m.insert(handmade(
        &plan,
        at(2, 1, 0),
        "agnus3:3",
        &[
            (
                "a'1 | g'2 f'2 | e'2 d'2~ | d'2 cis'2 | d'1",
                "x x x x x _ f x",
            ),
            ("f'1 | e'2 d'2 | c'2 b2 | a1~ | a1", "x x x x x x _"),
            ("d'1 | c'2 a2 | g2 f2~ | f2 e2 | f1", "x x x x x _ x _"),
            ("d1 | e2 f2 | c2 d2 | a,1 | d1", "x x x x x x _"),
        ],
        &[],
    ));
    assert!(!m.complete);
    m
}

/// The lines under a section that lacks a part, as `fux mass` words them.
fn missing(m: &Mass) -> impl Fn(usize, usize) -> Vec<String> + '_ {
    move |movement, section| {
        let parts = m.plan.movements[movement].sections[section].parts.len();
        let mut lines: Vec<String> = (0..parts)
            .map(|part| at(movement, section, part))
            .filter(|&place| m.result(place).is_none())
            .map(|place| format!("Teil {} fehlt: nicht gesetzt.", m.plan.address(place)))
            .collect();
        lines.push("fux mass --resume mit denselben Angaben setzt fehlende Teile nach.".into());
        lines
    }
}

/// What `fux mass` prints on the first page of the small mass.
fn title_page() -> MassBookOptions {
    MassBookOptions {
        subtitle: Some("dorisch, A T B".into()),
        byline: Some("fux-kontrapunkt, Seed 1".into()),
        ..MassBookOptions::default()
    }
}

fn small_book(options: &LilypondOptions) -> String {
    let m = small_mass();
    mass_book(&m, &title_page(), &missing(&m), options)
}

/// The scores of a book, in order, each from `\score {` to the line before the next.
fn scores(text: &str) -> Vec<&str> {
    text.split("\\score {").skip(1).collect()
}

/// The book of the small mass (spec § 6): one book part per movement, the title, subtitle and
/// byline on the first page only, the centred headings with the intonation of the Gloria and the
/// remarks for the performers under them, no table of contents; a score per section under its
/// title, with the staves of the voices that sing in it and the movement's names, two tenors T1
/// and T2 on the systems after the first, its measures numbered on through the movement, a double
/// bar after a section that more follow and a final bar at the end of a movement, fermatas on the
/// last notes; the label of the Break where its part sings again, nothing at the seam; a section
/// that lacks a part with the remark that says so; staves of 18 pt, the page of every book (no
/// page with a single system) and the layout of every document (two syllables of a word far
/// enough apart for the hyphen between them, the text clear of the staff below it).
#[test]
fn a_small_mass_book() {
    let text = small_book(&LilypondOptions::default());
    assert!(text.starts_with("\\version \"2.24.0\"\n"));
    assert_eq!(text.matches("\\bookpart {").count(), 3);
    assert_eq!(text.matches("title = \"Missa brevis\"").count(), 1);
    assert_eq!(text.matches("subtitle = \"dorisch, A T B\"").count(), 1);
    assert_eq!(
        text.matches("composer = \"fux-kontrapunkt, Seed 1\"")
            .count(),
        1
    );
    assert!(!text.contains("tocItem"));
    for heading in ["Kyrie", "Gloria", "Agnus Dei"] {
        assert!(
            text.contains(&format!(
                "\\markup \\fill-line {{ \\override #'(font-size . 4) \\bold \"{heading}\" }}"
            )),
            "{heading}"
        );
    }
    assert!(text.contains(
        "\\markup \\fill-line { \\center-column { \\italic \"Intonatio: Glória in excélsis Deo.\" } }"
    ));
    // The remarks of the plan, broken into even lines.
    assert!(text.contains(
        "\\markup \\fill-line { \\center-column { \\italic \"Kyrie I, Christe und Kyrie II stehen je für drei Anrufungen\" \\italic \"(Wiederholung oder Wechsel mit dem Choral nach Ortsbrauch).\" } }"
    ), "{text}");
    let scores = scores(&text);
    let pieces: Vec<&str> = scores
        .iter()
        .map(|score| {
            let start = score.find("piece = \"").unwrap() + 9;
            &score[start..start + score[start..].find('"').unwrap()]
        })
        .collect();
    assert_eq!(
        pieces,
        [
            "Kyrie I",
            "Christe",
            "Kyrie II",
            "Et in terra pax",
            "Agnus Dei I",
            "Agnus Dei II"
        ]
    );
    // The staves of the voices that sing, by their names.
    let staves: Vec<Vec<&str>> = scores
        .iter()
        .map(|score| {
            score
                .split("instrumentName = \"")
                .skip(1)
                .map(|rest| &rest[..rest.find('"').unwrap()])
                .collect()
        })
        .collect();
    assert_eq!(
        staves,
        [
            vec!["Alt", "Tenor", "Bass"],
            vec!["Alt", "Tenor"],
            vec!["Alt", "Tenor", "Bass"],
            vec!["Alt", "Tenor", "Bass"],
            vec!["Alt", "Tenor 1", "Bass"],
            vec!["Alt", "Tenor 1", "Tenor 2", "Bass"],
        ]
    );
    // Kyrie I has 8 measures, the Christe 6; Agnus Dei I has 4.
    let first_bars: Vec<Option<&str>> = scores
        .iter()
        .map(|score| {
            score
                .split_once("\\set Score.currentBarNumber = #")
                .map(|(_, rest)| &rest[..rest.find('\n').unwrap()])
        })
        .collect();
    assert_eq!(
        first_bars,
        [None, Some("9"), Some("15"), None, None, Some("5")]
    );
    let bars: Vec<&str> = scores
        .iter()
        .map(|score| {
            let start = score.find("\\bar \"").unwrap() + 6;
            &score[start..start + score[start..].find('"').unwrap()]
        })
        .collect();
    assert_eq!(bars, ["||", "||", "|.", "|.", "||", "|."]);
    for (score, voices) in scores.iter().zip([3, 2, 0, 3, 3, 4]) {
        assert_eq!(score.matches("\\fermata").count(), voices, "{score}");
    }
    // The Break at Qui tollis is labelled where its part sings again, over the barline after its
    // measure's rest, in the alto; the seam is not.
    assert_eq!(text.matches("\\mark \\markup").count(), 1);
    assert!(
        scores[3].contains("| r1 | \\mark \\markup \\italic \"Qui tollis\" a'2 a'2 | g'2 f'2"),
        "{}",
        scores[3]
    );
    // On the systems after the first the two tenors of the Agnus Dei are T1 and T2, in both its
    // sections; the other voices have the abbreviations of their types.
    let short: Vec<Vec<&str>> = scores
        .iter()
        .map(|score| {
            score
                .split("shortInstrumentName = \"")
                .skip(1)
                .map(|rest| &rest[..rest.find('"').unwrap()])
                .collect()
        })
        .collect();
    assert_eq!(
        short,
        [
            vec!["A", "T", "B"],
            vec!["A", "T"],
            vec!["A", "T", "B"],
            vec!["A", "T", "B"],
            vec!["A", "T1", "B"],
            vec!["A", "T1", "T2", "B"],
        ]
    );
    // The staff size of the book of a mass at the top, before its paper; then the page of every
    // book and the layout of every document, once for all its scores.
    assert!(text.contains(
        "\\language \"nederlands\"\n\n#(set-global-staff-size 18)\n\n\\header {\n  tagline = ##f\n}\n\n\\paper {\n  #(set-paper-size \"a4\")\n  min-systems-per-page = #2\n}\n\n\\layout {\n  \\context {\n    \\Lyrics\n    \\override LyricHyphen.minimum-distance = #0.5\n    \\override VerticalAxisGroup.nonstaff-unrelatedstaff-spacing.minimum-distance = #7\n  }\n}\n\n\\bookpart {"
    ), "{text}");
    assert_eq!(text.matches("LyricHyphen").count(), 1);
    assert_eq!(text.matches("set-global-staff-size").count(), 1);
    // The Gloria has a single section and keeps the minimum of the book, unlike a part of the
    // workbook with a single setting.
    assert_eq!(text.matches("min-systems-per-page").count(), 1);
    // Kyrie II lacks its part: a measure's rest in the staves of its voices, and the remark.
    assert!(scores[2].contains("\\line { \"Teil kyrie/3 fehlt: nicht gesetzt.\" }"));
    assert_eq!(scores[2].matches("      r1\n").count(), 3, "{}", scores[2]);
    // The brackets over the entries of Kyrie I.
    assert_eq!(scores[0].matches("\\startGroup").count(), 3);
    assert!(!text.contains("measureBarType") && !text.contains("?"));
    insta::assert_snapshot!(text);
}

/// With Mensurstriche every score draws its barlines between the staves, and notes tied over a
/// barline are written as one: the alto's held A of Kyrie I with its bracket, the F sharp of the
/// Christe, after which the F in its measure has a cautionary natural, the ficta B flat of the
/// Gloria; but a last note keeps the tie into it, and so its fermata over the last measure. The
/// rest is as without them.
#[test]
fn a_small_mass_book_with_mensurstriche() {
    let options = LilypondOptions {
        mensurstriche: true,
        ..LilypondOptions::default()
    };
    let text = small_book(&options);
    let scores = scores(&text);
    for score in &scores {
        assert!(
            score.contains(
                "  \\layout {\n    \\context {\n      \\Staff\n      measureBarType = \"-span|\"\n    }\n"
            ),
            "{score}"
        );
    }
    // The brackets of Kyrie I stay, over the notes as printed.
    assert!(
        scores[0].contains(
            "d'1\\startGroup^\\markup \\small \"1\" | f'2 g'2 | a'1.\\stopGroup g'2 | f'2 e'2. d'4 d'2~ | d'2 \\once \\set suggestAccidentals = ##t cis'2 | d'1\\fermata"
        ),
        "{}",
        scores[0]
    );
    assert!(scores[0].contains("\\consists \"Horizontal_bracket_engraver\""));
    // The last note keeps the tie into it, so that its fermata stands over the last measure as
    // in the other voice.
    assert!(
        scores[1].contains("e'2 fis'2. g'4 f'?2 | e'1~ | e'1\\fermata"),
        "{}",
        scores[1]
    );
    assert!(scores[1].contains("c'1 b2 | a1\\fermata"), "{}", scores[1]);
    assert!(scores[0].contains("| a1~ | a1\\fermata"), "{}", scores[0]);
    assert!(
        scores[3].contains("a'2 \\once \\set suggestAccidentals = ##t bes'1 a'2 | g'2 a'1."),
        "{}",
        scores[3]
    );
    assert_eq!(text.matches("?").count(), 1);
    // The lyrics are those without Mensurstriche.
    let plain = small_book(&LilypondOptions::default());
    let lyrics = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|line| line.contains("\\lyricsto"))
            .map(str::to_string)
            .collect()
    };
    assert_eq!(lyrics(&text), lyrics(&plain));
    insta::assert_snapshot!(text);
}

/// Original clefs stay as they are with Mensurstriche.
#[test]
fn original_clefs_with_mensurstriche() {
    let text = small_book(&LilypondOptions {
        mensurstriche: true,
        clefs: ClefStyle::Original,
        ..LilypondOptions::default()
    });
    assert!(text.contains("\\clef \"alto\"") && text.contains("\\clef \"tenor\""));
    assert!(text.contains("measureBarType = \"-span|\""));
}

/// Without the intonation the Gloria has its heading alone; the voices are named in English
/// with the remarks for the performers.
#[test]
fn the_intonation_can_be_left_out() {
    let m = small_mass();
    let book = MassBookOptions {
        intonation: false,
        ..MassBookOptions::default()
    };
    let options = LilypondOptions {
        lang: fux_core::Lang::En,
        ..LilypondOptions::default()
    };
    let text = mass_book(&m, &book, &missing(&m), &options);
    assert!(
        !text.contains("Intonatio") && !text.contains("subtitle"),
        "{text}"
    );
    assert!(text.contains("\\italic \"Agnus Dei I serves for the first and second invocations,\""));
    assert!(text.contains("instrumentName = \"Tenor 2\"") && text.contains("\"Alto\""));
}

/// The examples in the repository.
fn examples() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "musicxml"))
        .collect();
    files.sort();
    files
}

/// The score of an example's MusicXML file, its voices in their ranges.
fn example_score(path: &Path) -> Score {
    let read = from_musicxml(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mode: ModeSpec = read.field(FIELD_MODE).unwrap().parse().unwrap();
    let voices = read
        .parts
        .iter()
        .map(|part| {
            let pitches: Vec<Pitch> = part.notes.iter().filter_map(|n| n.pitch).collect();
            let mut voice = Voice::new(
                VoiceType::best_for(&pitches),
                Role::Counterpoint(Species::Free),
                part.notes.clone(),
            );
            voice.name.clone_from(&part.name);
            voice
        })
        .collect();
    let mut score = Score::new(mode, voices);
    score.meter = read.meter;
    score.validate().unwrap();
    score
}

/// The ties over barlines merged and kept in the examples 01–08 with K = 4: 106 of 128 in 01, 02,
/// 03, 07 and 08 since the examples were set anew on 5.11.2026 (E6 (b)). The examples before gave
/// 104 of 125 there, as measured with the prototype of the research
/// (docs/design/phase-14/research/research_notation.json and the correction of
/// verify_notation.json), among them the reset at a barline whose ties cannot be merged, without
/// which their 03 would have merged 36 and kept 10.
#[test]
fn ties_merge_on_the_examples() {
    let counts: Vec<(String, usize, usize)> = examples()
        .iter()
        .take(8)
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy();
            let mensur = merge_ties(&example_score(path), &[]);
            (name[..2].to_string(), mensur.merged, mensur.kept)
        })
        .collect();
    let expected = [
        ("01", 13, 2),
        ("02", 13, 0),
        ("03", 37, 10),
        ("04", 19, 3),
        ("05", 22, 4),
        ("06", 10, 2),
        ("07", 31, 8),
        ("08", 12, 2),
    ];
    let expected: Vec<(String, usize, usize)> = expected
        .iter()
        .map(|&(name, merged, kept)| (name.to_string(), merged, kept))
        .collect();
    assert_eq!(counts, expected);
    let research = ["01", "02", "03", "07", "08"];
    let (merged, kept) = counts
        .iter()
        .filter(|(name, _, _)| research.contains(&name.as_str()))
        .fold((0, 0), |(m, k), (_, merged, kept)| (m + merged, k + kept));
    assert_eq!((merged, merged + kept), (106, 128));
}

fn tmp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Compiles `text` with LilyPond to SVG and asserts that it has no warning.
fn compiles(dir: &Path, name: &str, text: &str) {
    let ly = dir.join(format!("{name}.ly"));
    std::fs::write(&ly, text).unwrap();
    let output = Command::new(fux_export::render::lilypond_command())
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .arg("--svg")
        .arg("-o")
        .arg(dir.join(name))
        .arg(&ly)
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{name}: {log}");
    assert!(!log.to_lowercase().contains("warning"), "{name}: {log}");
}

/// The book compiles without a warning, without Mensurstriche and with them and original clefs;
/// so does the five-voice example 03 with Mensurstriche, whose systems ran off the page when
/// every tie was merged. Runs only where LilyPond is installed (not in CI).
#[test]
fn lilypond_compiles_the_mass_book() {
    if lilypond_version().is_none() {
        eprintln!("LilyPond not installed, skipping");
        return;
    }
    let dir = tmp_dir("mass-book");
    compiles(&dir, "plain", &small_book(&LilypondOptions::default()));
    let mensur = LilypondOptions {
        mensurstriche: true,
        clefs: ClefStyle::Original,
        ..LilypondOptions::default()
    };
    compiles(&dir, "mensur", &small_book(&mensur));
    let score = example_score(&examples()[2]);
    assert_eq!(score.voices.len(), 5);
    compiles(
        &dir,
        "example-03",
        &document(None, &[Sheet::new(&score)], &mensur),
    );
}
