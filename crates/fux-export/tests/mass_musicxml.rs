//! The files of the movements of a mass (phase 14b, docs/design/phase-14/spec.md § 6): MusicXML
//! with its head, its voices as choir voices in a bracket, its sections, Breaks and tacet voices
//! and the fields of its parts, read back with its sections and split into its parts; MIDI with
//! a marker per section and Break and a silent measure after each double bar.
//!
//! The parts are made by hand, so these tests run in debug. The files validate against the
//! MusicXML 4.0 schema where `FUX_MUSICXML_XSD` names a copy of `musicxml.xsd` (with `xml.xsd`
//! and `xlink.xsd` beside it); without it the validation is skipped. One ignored test sets a
//! small mass and does the same with its files, in release: `cargo test --release -p fux-export
//! --test mass_musicxml -- --ignored --nocapture`.

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use fux_core::generate::Generated;
use fux_core::mass::{
    FieldError, Mass, MassEvent, MassPlan, MassRequest, Movement, Opening, PartRef, PartResult,
    Preset, choir, generate_mass, movement_parts, movement_score, opening_chord, part_fields,
    plan_mass, profile, profile_hash, split_movement,
};
use fux_core::rules::{Checker, Profile};
use fux_core::text::TextOptions;
use fux_core::{
    Cadence, Duration, Lang, Lyric, Note, Role, Score, ScoreEntry, ScoreImitation, ScorePoint,
    ScoreText, SectionKind, SectionMark, Species, Voice, VoiceType,
};
use fux_export::{
    ClefStyle, FIELD_MODE, MidiOptions, MovementHeading, MovementOptions, MusicXmlOptions,
    from_musicxml, to_midi, to_movement_midi, to_movement_musicxml, to_musicxml,
};
use midly::{MetaMessage, MidiMessage, Smf, TrackEventKind};
use roxmltree::{Document, Node};

/// A part made by hand in the voices of its section: it sings its text a syllable a measure on
/// its pitch of the opening chord of those voices, after a measure's rest where it begins after
/// silence, and holds its last syllable a measure more; an internal cadence on the final where its
/// text is long enough; where it begins in imitation, a point in which every voice enters at its
/// first note with two of them.
fn handmade(plan: &MassPlan, at: PartRef, profile: &Profile) -> PartResult {
    let movement = &plan.movements[at.movement];
    let voices = movement.sections[at.section].voices.clone();
    let part = plan.part(at);
    let text = part.sung_text(&TextOptions::default());
    let kinds: Vec<VoiceType> = voices.iter().map(|&v| movement.voices[v]).collect();
    let chord = opening_chord(movement.mode, &kinds, movement.mode.final_step());
    let silent = usize::from(part.start > 0);
    let syllables = text.syllables.len();
    let lines: Vec<Voice> = kinds
        .iter()
        .zip(&chord)
        .map(|(&kind, &pitch)| {
            let mut notes = vec![Note::rest(Duration::WHOLE); silent];
            notes.extend((0..syllables).map(|i| Note {
                lyric: Some(Lyric::new(i)),
                ..Note::new(pitch, Duration::WHOLE)
            }));
            notes.push(Note::new(pitch, Duration::WHOLE));
            Voice::new(kind, Role::Counterpoint(Species::Free), notes)
        })
        .collect();
    let mut score = Score::new(movement.mode, lines);
    if syllables >= 6 {
        score.cadences = vec![Cadence {
            measure: silent + 3,
            target: movement.mode.final_step(),
        }];
    }
    score.text = Some(ScoreText {
        text: Arc::new(text),
        voices: vec![true; kinds.len()],
    });
    if part.opening == Opening::Imitative {
        let start = silent as u32 * 8;
        let entries = chord
            .iter()
            .enumerate()
            .map(|(voice, &pitch)| ScoreEntry {
                voice,
                start,
                rest: 0,
                notes: vec![(pitch, 8), (pitch, 8)],
            })
            .collect();
        score.imitation = Some(Arc::new(ScoreImitation {
            points: vec![ScorePoint {
                words: 0..2,
                entries,
                goal: None,
            }],
        }));
    }
    score.fill_syllables(true);
    score.validate().unwrap();
    let report = Checker::new(profile).check(&score);
    PartResult {
        at,
        seed: part.seed,
        attempt: 0,
        generated: Generated {
            score,
            cost: report.cost(),
            report,
            warnings: Vec::new(),
            subjects: Vec::new(),
        },
        seconds: 0.0,
        warnings: Vec::new(),
        voices,
        profile: profile_hash(profile),
        motif: None,
        kept: None,
    }
}

/// A mass of `request` whose every part is made by hand ([`handmade`]).
fn handmade_mass(request: &MassRequest) -> Mass {
    let profile = profile(&Profile::fux());
    let plan = Arc::new(plan_mass(request).unwrap());
    let mut mass = Mass::new(Arc::clone(&plan));
    for (at, _) in plan.parts() {
        mass.insert(handmade(&plan, at, &profile));
    }
    assert!(mass.complete);
    mass
}

/// The Missa brevis for S A T B: Kyrie in three sections, the Gloria with seams and the Break at
/// Qui tollis, the Benedictus without the bass, Agnus Dei I without the second tenor.
fn brevis() -> Mass {
    handmade_mass(&MassRequest::default())
}

/// The form normal for S A T B, whose Benedictus ends with the Osanna of the Sanctus ut supra.
fn normal() -> Mass {
    handmade_mass(&MassRequest {
        preset: Preset::Normal,
        movements: vec![Movement::Sanctus, Movement::Benedictus],
        ..MassRequest::default()
    })
}

/// The head of the file of the movement `m` of `mass`, as a frontend gives it.
fn heading(mass: &Mass, m: usize) -> MovementHeading {
    let plan = &mass.plan;
    let movement = &plan.movements[m];
    MovementHeading {
        mass: plan.title.clone(),
        number: movement.movement as usize + 1,
        title: movement.movement.title().to_string(),
        byline: Some(format!(
            "dorisch, S A T B · fux-kontrapunkt, Seed {}",
            plan.seed
        )),
        intonation: movement.intonation.map(str::to_string),
    }
}

/// The file of the movement `m` of `mass` with `options`, and its score.
fn movement_file(mass: &Mass, m: usize, options: &MovementOptions) -> (String, Score) {
    let score = movement_score(mass, m).unwrap();
    let xml = to_movement_musicxml(&score, &heading(mass, m), options, &part_fields(mass, m));
    (xml, score)
}

fn parse(xml: &str) -> Document<'_> {
    Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .unwrap()
}

/// The elements named `name` below `node`.
fn all<'a, 'input>(node: Node<'a, 'input>, name: &'a str) -> Vec<Node<'a, 'input>> {
    node.descendants()
        .filter(|n| n.has_tag_name(name))
        .collect()
}

/// The text of the first element named `name` below `node`.
fn text(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.descendants()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
        .map(str::to_string)
}

/// Per part, its measures.
fn measures<'a, 'input>(doc: &'a Document<'input>) -> Vec<Vec<Node<'a, 'input>>> {
    doc.root_element()
        .children()
        .filter(|c| c.has_tag_name("part"))
        .map(|part| {
            part.children()
                .filter(|c| c.has_tag_name("measure"))
                .collect()
        })
        .collect()
}

/// The measures (from 0) of a part that have an element named `name`, with its text.
fn found(measures: &[Node<'_, '_>], name: &str) -> Vec<(usize, String)> {
    measures
        .iter()
        .enumerate()
        .flat_map(|(m, measure)| {
            all(*measure, name)
                .into_iter()
                .map(move |n| (m, n.text().unwrap_or_default().to_string()))
        })
        .collect()
}

/// Every `sound` of the parts, `measures` of them, as (part, measure, its tempo), from 0.
fn tempi<'a>(measures: &[Vec<Node<'a, '_>>]) -> Vec<(usize, usize, Option<&'a str>)> {
    let mut tempi = Vec::new();
    for (v, part) in measures.iter().enumerate() {
        for (m, measure) in part.iter().enumerate() {
            for sound in all(*measure, "sound") {
                tempi.push((v, m, sound.attribute("tempo")));
            }
        }
    }
    tempi
}

/// Validates MusicXML with xmllint against the MusicXML 4.0 schema, if `FUX_MUSICXML_XSD` names
/// a copy of `musicxml.xsd` (with `xml.xsd` and `xlink.xsd` beside it).
fn validate_musicxml(xml: &str, name: &str) {
    let Some(xsd) = std::env::var_os("FUX_MUSICXML_XSD") else {
        eprintln!("FUX_MUSICXML_XSD not set, skipping the schema validation");
        return;
    };
    let file = tmp_dir("mass-musicxml").join(name);
    std::fs::write(&file, xml).unwrap();
    let output = Command::new("xmllint")
        .args(["--noout", "--nonet", "--schema"])
        .arg(xsd)
        .arg(&file)
        .output()
        .expect("xmllint");
    assert!(
        output.status.success(),
        "{name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn tmp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The sections of a movement: its first measure and the measure after its last.
fn sections(score: &Score) -> Vec<(usize, usize)> {
    let starts: Vec<usize> = score
        .sections
        .iter()
        .filter(|mark| mark.kind == SectionKind::Section)
        .map(|mark| mark.measure)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(s, &first)| {
            (
                first,
                starts.get(s + 1).copied().unwrap_or(score.measure_count()),
            )
        })
        .collect()
}

/// Whether voice `v` of `score` sounds nowhere in the measures `first..end`.
fn tacet(score: &Score, v: usize, (first, end): (usize, usize)) -> bool {
    score.voices[v]
        .timed()
        .all(|(tick, note)| tick < first as u32 * 8 || tick >= end as u32 * 8 || note.is_rest())
}

/// The file of a movement has the mass as the work, the movement's number and title, an A4 page,
/// the titles and the line under them on the first page; a bracket around its voices, each a
/// choir voice of its range on its MIDI channel with program 53; the tempo of its MIDI file, 72
/// halves a minute, as 144 quarters in the first measure of the first part, after its attributes;
/// where a section begins a new system (after the first) and its title as a rehearsal mark, where
/// one ends a double bar and a fermata on every singing voice's last note, the final bar line at
/// the end; the label of a Break, where its part sings again after the measure's rest that opens
/// it, and the intonation in italics, over the top voice; two voices of a type numbered in their
/// short names, as in their names (T1, T2); the brackets over the entries counted from 1 in each
/// section; the mode and the fields of the parts. With Mensurstriche the bracket has them, and
/// the program and the tempo are the options'; their language and seed change nothing. Every file
/// validates.
#[test]
fn movement_files_have_their_head_and_sections() {
    let mass = brevis();
    let plan = &mass.plan;
    for (m, movement) in plan.movements.iter().enumerate() {
        let label = movement.movement.id();
        let (xml, score) = movement_file(&mass, m, &MovementOptions::default());
        validate_musicxml(&xml, &format!("{}-{label}.musicxml", m + 1));
        let doc = parse(&xml);
        let root = doc.root_element();
        assert_eq!(text(root, "work-title").as_deref(), Some("Missa brevis"));
        assert_eq!(
            text(root, "movement-number"),
            Some((movement.movement as usize + 1).to_string())
        );
        assert_eq!(
            text(root, "movement-title").as_deref(),
            Some(movement.movement.title())
        );
        assert_eq!(text(root, "millimeters").as_deref(), Some("7"));
        assert_eq!(text(root, "tenths").as_deref(), Some("40"));
        assert_eq!(
            (text(root, "page-height"), text(root, "page-width")),
            (Some("1697".into()), Some("1200".into()))
        );
        let credits: Vec<(String, String)> = all(root, "credit")
            .iter()
            .map(|c| {
                assert_eq!(c.attribute("page"), Some("1"));
                (
                    text(*c, "credit-type").unwrap(),
                    text(*c, "credit-words").unwrap(),
                )
            })
            .collect();
        assert_eq!(
            credits,
            [
                ("title".into(), "Missa brevis".into()),
                ("subtitle".into(), movement.movement.title().into()),
                (
                    "composer".into(),
                    format!("dorisch, S A T B · fux-kontrapunkt, Seed {}", plan.seed)
                ),
            ]
        );
        // The part list: a bracket around every part, choir voices.
        let list = all(root, "part-list")[0];
        let children: Vec<Node> = list.children().filter(Node::is_element).collect();
        assert_eq!(children.len(), score.voices.len() + 2);
        let (start, stop) = (children[0], children[children.len() - 1]);
        assert_eq!(
            (start.attribute("type"), stop.attribute("type")),
            (Some("start"), Some("stop"))
        );
        assert_eq!(text(start, "group-symbol").as_deref(), Some("bracket"));
        assert_eq!(text(start, "group-barline").as_deref(), Some("yes"));
        for (v, (part, voice)) in children[1..children.len() - 1]
            .iter()
            .zip(&score.voices)
            .enumerate()
        {
            assert_eq!(text(*part, "part-name"), Some(voice.name.clone()));
            // The short name, numbered where the movement has two voices of the type.
            let kind = voice.voice_type;
            let mut short = kind.abbreviation().to_string();
            if score.voices.iter().filter(|v| v.voice_type == kind).count() > 1 {
                let number = score.voices[..=v]
                    .iter()
                    .filter(|v| v.voice_type == kind)
                    .count();
                short.push_str(&number.to_string());
            }
            assert_eq!(text(*part, "part-abbreviation"), Some(short), "{label}");
            let sound = match voice.voice_type {
                VoiceType::Soprano => ("voice.soprano", "Soprano"),
                VoiceType::Alto => ("voice.alto", "Alto"),
                VoiceType::Tenor => ("voice.tenor", "Tenor"),
                VoiceType::Bass => ("voice.bass", "Bass"),
                other => panic!("{other:?}"),
            };
            assert_eq!(
                (
                    text(*part, "instrument-sound"),
                    text(*part, "instrument-name")
                ),
                (Some(sound.0.into()), Some(sound.1.into()))
            );
            assert_eq!(text(*part, "midi-program").as_deref(), Some("53"));
            assert_eq!(text(*part, "midi-channel"), Some((v + 1).to_string()));
        }
        // The fields: the mode, then those of the parts.
        let fields: Vec<(String, String)> = all(root, "miscellaneous-field")
            .iter()
            .map(|f| {
                (
                    f.attribute("name").unwrap().to_string(),
                    f.text().unwrap_or_default().to_string(),
                )
            })
            .collect();
        let mut expected = vec![(FIELD_MODE.to_string(), score.mode.id())];
        expected.extend(part_fields(&mass, m));
        assert_eq!(fields, expected, "{label}");
        // The tempo, in the first measure of the first part only, after its attributes.
        let parts = measures(&doc);
        assert_eq!(tempi(&parts), [(0, 0, Some("144"))], "{label}");
        let first: Vec<&str> = parts[0][0]
            .children()
            .filter(Node::is_element)
            .map(|c| c.tag_name().name())
            .take(2)
            .collect();
        assert_eq!(first, ["attributes", "sound"], "{label}");
        // The sections.
        let ranges = sections(&score);
        let measure_count = score.measure_count();
        let section_starts: Vec<usize> = ranges.iter().map(|&(first, _)| first).collect();
        let top = |measure: usize| {
            let s = ranges
                .iter()
                .rposition(|&(first, _)| first <= measure)
                .unwrap();
            (0..score.voices.len())
                .find(|&v| !tacet(&score, v, ranges[s]))
                .unwrap()
        };
        let mut rehearsals: Vec<(usize, String)> = Vec::new();
        let mut words: Vec<(usize, String)> = Vec::new();
        // The points shown over the entries, by section and number.
        let mut numbers: std::collections::BTreeSet<(usize, usize)> = Default::default();
        for (v, part) in parts.iter().enumerate() {
            assert_eq!(part.len(), measure_count, "{label}");
            let systems: Vec<usize> = (0..measure_count)
                .filter(|&i| {
                    all(part[i], "print")
                        .iter()
                        .any(|p| p.attribute("new-system") == Some("yes"))
                })
                .collect();
            assert_eq!(systems, section_starts[1..], "{label}, voice {v}");
            let bars = found(part, "bar-style");
            let mut expected: Vec<(usize, String)> = ranges
                .iter()
                .map(|&(_, end)| (end - 1, "light-light".to_string()))
                .collect();
            expected.last_mut().unwrap().1 = "light-heavy".into();
            assert_eq!(bars, expected, "{label}, voice {v}");
            for (m, text) in found(part, "rehearsal") {
                assert_eq!(v, top(m), "{label}: the rehearsal mark over the top voice");
                rehearsals.push((m, text));
            }
            for (m, text) in found(part, "words") {
                if let Ok(number) = text.parse::<usize>() {
                    // The number of a point over its entries, counted in its section.
                    let s = ranges.iter().rposition(|&(first, _)| first <= m).unwrap();
                    numbers.insert((s, number));
                    continue;
                }
                assert_eq!(v, top(m), "{label}: the words over the top voice");
                words.push((m, text));
            }
            // Fermatas on the last note of each section the voice sings in.
            let notes: Vec<Node> = part.iter().flat_map(|m| all(*m, "note")).collect();
            let held: Vec<usize> = (0..notes.len())
                .filter(|&i| !all(notes[i], "fermata").is_empty())
                .collect();
            let timed: Vec<(u32, &Note)> = score.voices[v].timed().collect();
            let last: Vec<usize> = ranges
                .iter()
                .filter(|&&range| !tacet(&score, v, range))
                .map(|&(_, end)| {
                    timed
                        .iter()
                        .rposition(|&(tick, _)| tick < end as u32 * 8)
                        .unwrap()
                })
                .collect();
            assert_eq!(held, last, "{label}, voice {v}");
        }
        // Each section counts its points from 1.
        let mut points: std::collections::BTreeSet<(usize, usize)> = Default::default();
        let mut counted = vec![0; ranges.len()];
        for point in score.imitation.iter().flat_map(|plan| &plan.points) {
            let leader = point.entries[0].start as usize / 8;
            let s = ranges
                .iter()
                .rposition(|&(first, _)| first <= leader)
                .unwrap();
            counted[s] += 1;
            points.insert((s, counted[s]));
        }
        assert_eq!(numbers, points, "{label}");
        rehearsals.sort();
        words.sort();
        let titled: Vec<(usize, String)> = score
            .sections
            .iter()
            .filter(|mark| mark.kind == SectionKind::Section)
            .map(|mark| (mark.measure, mark.title.clone().unwrap()))
            .collect();
        assert_eq!(rehearsals, titled, "{label}");
        // The label of a Break stands where its part sings again, after the measure's rest
        // that opens it.
        let sung = |measure: usize| {
            score
                .voices
                .iter()
                .filter_map(|voice| {
                    voice
                        .timed()
                        .find(|&(tick, note)| tick >= measure as u32 * 8 && !note.is_rest())
                        .map(|(tick, _)| tick as usize / 8)
                })
                .min()
                .unwrap()
        };
        let mut labels: Vec<(usize, String)> = score
            .sections
            .iter()
            .filter(|mark| mark.kind == SectionKind::Break)
            .map(|mark| {
                assert_eq!(sung(mark.measure), mark.measure + 1, "{label}");
                (mark.measure + 1, mark.title.clone().unwrap())
            })
            .collect();
        if let Some(intonation) = movement.intonation {
            labels.push((0, format!("Intonatio: {intonation}")));
        }
        labels.sort();
        assert_eq!(words, labels, "{label}");
        assert!(
            all(root, "words")
                .iter()
                .filter(|w| w.text().is_some_and(|t| t.parse::<usize>().is_err()))
                .all(|w| w.attribute("font-style") == Some("italic"))
        );
    }
    // The Gloria: the intonation and the Break at Qui tollis.
    let (xml, _) = movement_file(&mass, 1, &MovementOptions::default());
    assert!(
        xml.contains("<words font-style=\"italic\">Intonatio: Glória in excélsis Deo.</words>")
    );
    assert!(xml.contains("<words font-style=\"italic\">Qui tollis</words>"));
    assert!(xml.contains("<rehearsal>Et in terra pax</rehearsal>"));
    // The Agnus Dei has two tenors, T1 and T2 on the systems after the first.
    let (xml, _) = movement_file(&mass, 5, &MovementOptions::default());
    for n in [1, 2] {
        assert!(
            xml.contains(&format!(
                "<part-name>Tenor {n}</part-name><part-abbreviation>T{n}</part-abbreviation>"
            )),
            "{xml}"
        );
    }
    // Mensurstriche, another program and another tempo.
    let options = MovementOptions {
        mensurstriche: true,
        musicxml: MusicXmlOptions {
            clefs: ClefStyle::Original,
            entry_marks: false,
            tempo: 60,
            program: 1,
            ..MusicXmlOptions::default()
        },
        ..MovementOptions::default()
    };
    let (xml, _) = movement_file(&mass, 0, &options);
    validate_musicxml(&xml, "kyrie-mensur.musicxml");
    let doc = parse(&xml);
    assert_eq!(
        text(doc.root_element(), "group-barline").as_deref(),
        Some("Mensurstrich")
    );
    assert!(
        all(doc.root_element(), "midi-program")
            .iter()
            .all(|p| p.text() == Some("1"))
    );
    assert_eq!(tempi(&measures(&doc)), [(0, 0, Some("120"))]);
    assert!(xml.contains("<sign>C</sign><line>1</line>"));
    assert!(!xml.contains("<bracket"));
    // The language and the seed of the options change nothing: the heading gives the lines.
    let english = MovementOptions {
        musicxml: MusicXmlOptions {
            lang: Lang::En,
            seed: Some(9),
            ..MusicXmlOptions::default()
        },
        ..MovementOptions::default()
    };
    assert_eq!(
        movement_file(&mass, 0, &english).0,
        movement_file(&mass, 0, &MovementOptions::default()).0
    );
}

/// A voice that does not sing in a section rests a whole measure in each of its measures; with
/// the tacet voices hidden it is hidden in every one of them and shown again where it sings, as
/// Verovio reads it measure by measure. Other voices and other measures say nothing of it.
#[test]
fn tacet_voices_rest_or_hide() {
    let mass = brevis();
    // The Benedictus without the bass, Agnus Dei I without the second tenor.
    for (m, voice) in [(4, 3), (5, 3)] {
        let (shown, score) = movement_file(&mass, m, &MovementOptions::default());
        let hidden_options = MovementOptions {
            hide_tacet: true,
            ..MovementOptions::default()
        };
        let (hidden, _) = movement_file(&mass, m, &hidden_options);
        validate_musicxml(&hidden, &format!("tacet-{m}.musicxml"));
        assert!(!shown.contains("staff-details"));
        let ranges = sections(&score);
        assert!(tacet(&score, voice, ranges[0]) && !tacet(&score, voice, ranges[1]));
        let (first, end) = ranges[0];
        let doc = parse(&shown);
        let part = &measures(&doc)[voice];
        for measure in &part[first..end] {
            let notes = all(*measure, "note");
            assert_eq!(notes.len(), 1);
            assert_eq!(all(notes[0], "rest")[0].attribute("measure"), Some("yes"));
        }
        let doc = parse(&hidden);
        for (v, part) in measures(&doc).iter().enumerate() {
            let details: Vec<(usize, Option<&str>)> = part
                .iter()
                .enumerate()
                .flat_map(|(i, measure)| {
                    all(*measure, "staff-details")
                        .into_iter()
                        .map(move |d| (i, d.attribute("print-object")))
                })
                .collect();
            let expected: Vec<(usize, Option<&str>)> = if v == voice {
                (first..end)
                    .map(|i| (i, Some("no")))
                    .chain([(end, Some("yes"))])
                    .collect()
            } else {
                Vec::new()
            };
            assert_eq!(details, expected, "movement {m}, voice {v}");
        }
        // The notes are those of the file that shows the voice.
        assert_eq!(
            from_musicxml(&hidden).unwrap().parts,
            from_musicxml(&shown).unwrap().parts
        );
    }
}

/// A file of a movement reads back as written: its parts with their notes and syllables, its
/// fields, and its sections and the parts' Breaks and seams from the fields of its parts, an
/// Osanna ut supra's from its notation. Without the fields its sections come from its rehearsal
/// marks and double bars. The parts of the movement come back from the score read and the
/// fields, each with its notes as set.
#[test]
fn movement_files_read_back() {
    for mass in [brevis(), normal()] {
        let plan = &mass.plan;
        for (m, movement) in plan.movements.iter().enumerate() {
            let label = format!("{} {}", plan.preset.id(), movement.movement.id());
            let (xml, score) = movement_file(&mass, m, &MovementOptions::default());
            let read = from_musicxml(&xml).unwrap();
            assert_eq!(read.title.as_deref(), Some(plan.title.as_str()));
            assert_eq!(read.meter, score.meter);
            assert_eq!(read.mode, Some(score.mode.mode));
            assert_eq!(read.parts.len(), score.voices.len());
            for (part, voice) in read.parts.iter().zip(&score.voices) {
                let written: Vec<Note> = voice
                    .notes
                    .iter()
                    .map(|note| Note {
                        lyric: None,
                        ..note.clone()
                    })
                    .collect();
                assert_eq!(part.notes, written, "{label}, {}", voice.name);
                assert_eq!(part.name, voice.name);
            }
            let mut fields = vec![(FIELD_MODE.to_string(), score.mode.id())];
            fields.extend(part_fields(&mass, m));
            assert_eq!(read.fields, fields, "{label}");
            assert_eq!(read.sections, score.sections, "{label}");
            // Without the fields: the sections from the notation.
            let start = xml.find("<miscellaneous>").unwrap();
            let end = xml.find("</miscellaneous>").unwrap();
            let bare = format!(
                "{}{}",
                &xml[..start],
                &xml[end + "</miscellaneous>".len()..]
            );
            let sections: Vec<SectionMark> = score
                .sections
                .iter()
                .filter(|mark| mark.kind == SectionKind::Section)
                .cloned()
                .collect();
            assert_eq!(from_musicxml(&bare).unwrap().sections, sections, "{label}");
            // The parts from the score read, as a frontend builds it, and the fields.
            let voices: Vec<Voice> = read
                .parts
                .iter()
                .zip(&score.voices)
                .map(|(part, voice)| {
                    let mut read = Voice::new(voice.voice_type, voice.role, part.notes.clone());
                    read.name = part.name.clone();
                    read
                })
                .collect();
            let mut again = Score::new(score.mode, voices);
            again.sections = read.sections.clone();
            again.validate().unwrap();
            let split = split_movement(&again, &read.fields).unwrap();
            let places: Vec<PartRef> = plan
                .parts()
                .filter(|(at, _)| at.movement == m)
                .map(|(at, _)| at)
                .collect();
            assert_eq!(split.len(), places.len(), "{label}");
            for ((_, part, _), &at) in split.iter().zip(&places) {
                let own = &mass.result(at).unwrap().generated.score;
                for (voice, original) in part.voices.iter().zip(&own.voices) {
                    let set: Vec<Note> = original
                        .notes
                        .iter()
                        .map(|note| Note {
                            lyric: None,
                            ..note.clone()
                        })
                        .collect();
                    assert_eq!(voice.notes, set, "{label}, {}", plan.address(at));
                }
                assert_eq!(part.cadences, own.cadences);
                assert_eq!(part.imitation, own.imitation);
            }
        }
    }
    // The Benedictus of the form normal ends with the Osanna ut supra, which no part of it sings.
    let mass = normal();
    let (xml, score) = movement_file(&mass, 1, &MovementOptions::default());
    let read = from_musicxml(&xml).unwrap();
    assert_eq!(
        read.sections.last().and_then(|mark| mark.title.as_deref()),
        Some("Osanna ut supra")
    );
    assert_eq!(read.sections, score.sections);
}

/// The measures of every part of a movement's file, as a notation program writes them after an
/// edit: with a measure's rest inserted before the measure `measure` (from 0), or with that
/// measure deleted.
fn edited(xml: &str, measure: usize, insert: bool) -> String {
    let start = format!("    <measure number=\"{}\">\n", measure + 1);
    let end = "    </measure>\n";
    let rest = "    <measure number=\"0\">\n      <note>\n        <rest measure=\"yes\"/>\n        <duration>8</duration>\n        <voice>1</voice>\n        <type>whole</type>\n      </note>\n    </measure>\n";
    let mut out = String::new();
    let mut parts = 0;
    let mut tail = xml;
    while let Some(at) = tail.find(&start) {
        out.push_str(&tail[..at]);
        let measure = &tail[at..];
        if insert {
            out.push_str(rest);
            out.push_str(&start);
            tail = &measure[start.len()..];
        } else {
            tail = &measure[measure.find(end).unwrap() + end.len()..];
        }
        parts += 1;
    }
    out.push_str(tail);
    assert_eq!(parts, xml.matches("<part id=").count());
    out
}

/// The parts of the movement's file `xml`, whose voices are those of `score`, as `fux check`
/// splits them.
fn split_file(xml: &str, score: &Score) -> Result<Vec<(PartRef, Score, usize)>, FieldError> {
    let read = from_musicxml(xml).unwrap();
    let voices: Vec<Voice> = read
        .parts
        .iter()
        .zip(&score.voices)
        .map(|(part, voice)| Voice::new(voice.voice_type, voice.role, part.notes.clone()))
        .collect();
    let mut again = Score::new(score.mode, voices);
    again.sections = read.sections.clone();
    split_movement(&again, &read.fields)
}

/// A file of a movement whose measures no longer fit the fields of its parts, as after an edit in
/// a notation program, does not split, and the error names a part (§ 6 „Check“): with a measure
/// inserted in the Christe the parts of the Kyrie end before the file does, and so they do
/// without the field of the last part; with a measure inserted in the Benedictus of the form
/// normal its Osanna ut supra no longer begins after the parts, and with one deleted there the
/// bass of the Osanna sounds in the measures of the Benedictus, which it does not sing.
#[test]
fn edited_movement_files_do_not_split() {
    let mass = brevis();
    let (xml, score) = movement_file(&mass, 0, &MovementOptions::default());
    assert_eq!(split_file(&xml, &score).unwrap().len(), 3);
    let fields = part_fields(&mass, 0);
    let parts = movement_parts(&fields).unwrap();
    let christe = *parts[1].bars.start();
    let measures = score.measure_count();
    assert_eq!(
        split_file(&edited(&xml, christe, true), &score),
        Err(FieldError::Unclaimed {
            part: "kyrie/3".into(),
            last: measures,
            measures: measures + 1,
            ut_supra: None,
        })
    );
    let third = &fields
        .iter()
        .find(|(name, _)| name == "fux-part.3")
        .unwrap()
        .1;
    let field = format!("<miscellaneous-field name=\"fux-part.3\">{third}</miscellaneous-field>");
    assert!(xml.contains(&field));
    assert_eq!(
        split_file(&xml.replacen(&field, "", 1), &score),
        Err(FieldError::Unclaimed {
            part: "kyrie/2".into(),
            last: *parts[1].bars.end(),
            measures,
            ut_supra: None,
        })
    );
    // The Benedictus of the form normal for S A T B, without the bass, and the Osanna ut supra.
    let mass = normal();
    let (xml, score) = movement_file(&mass, 1, &MovementOptions::default());
    let parts = movement_parts(&part_fields(&mass, 1)).unwrap();
    assert_eq!(parts.len(), 1);
    let last = *parts[0].bars.end();
    assert!(last < score.measure_count());
    assert_eq!(split_file(&xml, &score).unwrap().len(), 1);
    assert_eq!(
        split_file(&edited(&xml, 1, true), &score),
        Err(FieldError::Unclaimed {
            part: "benedictus/1".into(),
            last,
            measures: score.measure_count() + 1,
            ut_supra: Some("Osanna ut supra".into()),
        })
    );
    assert_eq!(
        split_file(&edited(&xml, 1, false), &score),
        Err(FieldError::Voices {
            part: "benedictus/1".into()
        })
    );
}

/// A single score reads back without sections, also where it has marks of sections, which only
/// the file of a movement writes: Fux's examples, all of two voices or more, have the part list
/// of a movement's file, a bracket around choir voices (since M1 (b); in 14b only the file of a
/// movement had them; a score of one voice has none, see musicxml_head_and_choir_voices), but
/// neither the number and title of a movement nor rehearsal marks, new systems or double bars,
/// with the marks as without them.
#[test]
fn single_scores_have_no_sections() {
    for example in fux_core::corpus::fux_examples() {
        let mut score = example.score();
        let plain = to_musicxml(&score, &MusicXmlOptions::default());
        assert!(from_musicxml(&plain).unwrap().sections.is_empty());
        assert_eq!(plain.matches("<part-group ").count(), 2);
        assert_eq!(
            plain.matches("<score-instrument ").count(),
            score.voices.len()
        );
        for movement in [
            "<movement-number>",
            "<movement-title>",
            "<rehearsal>",
            "new-system",
            "light-light",
        ] {
            assert!(!plain.contains(movement), "Fig. {}", example.figure);
        }
        if score.measure_count() > 2 {
            score.sections = vec![
                SectionMark {
                    measure: 0,
                    kind: SectionKind::Section,
                    title: Some("Kyrie I".into()),
                },
                SectionMark {
                    measure: 2,
                    kind: SectionKind::Section,
                    title: Some("Christe".into()),
                },
            ];
            assert_eq!(to_musicxml(&score, &MusicXmlOptions::default()), plain);
            assert_eq!(
                to_midi(&score, &MidiOptions::default()),
                to_midi(&example.score(), &MidiOptions::default())
            );
        }
    }
}

/// The MIDI file of a movement: format 1, a tempo track named after the mass and the movement
/// with the tempo (72 halves a minute), the meter and a marker where each section and each Break
/// begins, lasting as long as the movement, and a track per voice with program 53. After each
/// double bar every voice is silent a measure, so each section begins a measure later than in
/// the score for each double bar before it. A score without sections is the single score's
/// file.
#[test]
fn movement_midi_marks_sections_and_breaks() {
    let mass = brevis();
    let options = MidiOptions::default();
    // MIDI ticks of a measure.
    let measure = 1920;
    for (m, movement) in mass.plan.movements.iter().enumerate() {
        let score = movement_score(&mass, m).unwrap();
        let name = format!("Missa brevis – {}", movement.movement.title());
        let bytes = to_movement_midi(&score, &name, &options);
        let smf = Smf::parse(&bytes).unwrap();
        assert_eq!(smf.header.format, midly::Format::Parallel);
        assert_eq!(smf.tracks.len(), 1 + score.voices.len());
        // Where each measure of the score sounds: a measure later after each double bar.
        let starts: Vec<usize> = score
            .sections
            .iter()
            .filter(|mark| mark.kind == SectionKind::Section && mark.measure > 0)
            .map(|mark| mark.measure)
            .collect();
        let at = |measure_of_score: usize| -> u32 {
            let gaps = starts.iter().filter(|&&s| s <= measure_of_score).count();
            (measure_of_score + gaps) as u32 * measure
        };
        let mut time = 0;
        let mut names = Vec::new();
        let mut markers = Vec::new();
        let mut tempo = None;
        for event in &smf.tracks[0] {
            time += event.delta.as_int();
            match event.kind {
                TrackEventKind::Meta(MetaMessage::TrackName(n)) => {
                    names.push(String::from_utf8(n.to_vec()).unwrap());
                }
                TrackEventKind::Meta(MetaMessage::Marker(text)) => {
                    markers.push((time, String::from_utf8(text.to_vec()).unwrap()));
                }
                TrackEventKind::Meta(MetaMessage::Tempo(t)) => tempo = Some(t.as_int()),
                _ => {}
            }
        }
        assert_eq!(names, std::slice::from_ref(&name));
        assert_eq!(tempo, Some(416_666));
        let expected: Vec<(u32, String)> = score
            .sections
            .iter()
            .filter(|mark| mark.kind != SectionKind::Seam)
            .map(|mark| (at(mark.measure), mark.title.clone().unwrap()))
            .collect();
        assert_eq!(markers, expected, "{name}");
        // The tempo track lasts as long as the movement, the silent measures included.
        assert_eq!(time, at(score.measure_count() - 1) + measure, "{name}");
        // Every note sounds where its measure sounds; none in the silent measures.
        for (track, voice) in smf.tracks[1..].iter().zip(&score.voices) {
            let mut time = 0;
            let mut ons = Vec::new();
            let mut program = None;
            let mut last = 0;
            for event in track {
                time += event.delta.as_int();
                if let TrackEventKind::Midi { message, .. } = event.kind {
                    match message {
                        MidiMessage::ProgramChange { program: p } => program = Some(p.as_int()),
                        MidiMessage::NoteOn { .. } => ons.push(time),
                        MidiMessage::NoteOff { .. } => last = time,
                        _ => {}
                    }
                }
            }
            assert_eq!(program, Some(52), "{name}, {}", voice.name);
            let expected: Vec<u32> = voice
                .soundings()
                .iter()
                .map(|s| {
                    let measure_of_score = s.start as usize / 8;
                    at(measure_of_score) + (s.start % 8) * 240
                })
                .collect();
            assert_eq!(ons, expected, "{name}, {}", voice.name);
            if !ons.is_empty() {
                assert!(last <= at(score.measure_count() - 1) + measure);
            }
        }
        // The last note ends with the movement, the silent measures included.
        let end = smf.tracks[1..]
            .iter()
            .map(|track| track.iter().map(|e| e.delta.as_int()).sum::<u32>())
            .max()
            .unwrap();
        assert_eq!(end, at(score.measure_count() - 1) + measure, "{name}");
    }
    // The Kyrie: Kyrie I at the start, Christe and Kyrie II each a measure later.
    let kyrie = movement_score(&mass, 0).unwrap();
    let bytes = to_movement_midi(&kyrie, "Missa brevis – Kyrie", &options);
    let smf = Smf::parse(&bytes).unwrap();
    let markers: Vec<String> = smf.tracks[0]
        .iter()
        .filter_map(|e| match e.kind {
            TrackEventKind::Meta(MetaMessage::Marker(text)) => {
                Some(String::from_utf8(text.to_vec()).unwrap())
            }
            _ => None,
        })
        .collect();
    assert_eq!(markers, ["Kyrie I", "Christe", "Kyrie II"]);
    // Without sections the single score's file.
    let mut single = kyrie.clone();
    single.sections.clear();
    single.title = Some("Kyrie".into());
    assert_eq!(
        to_movement_midi(&single, "Kyrie", &options),
        to_midi(&single, &options)
    );
}

/// A small mass set in release, in its files (§ 8): Kyrie, Benedictus and Agnus Dei for S A T B,
/// Dorian, seed 1. Every movement's file validates against the schema (where
/// `FUX_MUSICXML_XSD` names it), with Mensurstriche and hidden tacet voices too, reads back with
/// its notes and sections, and gives its parts back as set; every MIDI file has its markers. The
/// files stay in the target directory's `tmp/mass-files` for a look.
#[test]
#[ignore = "release: sets a small mass, a few minutes"]
fn a_small_mass_in_its_files() {
    let request = MassRequest {
        movements: vec![Movement::Kyrie, Movement::Benedictus, Movement::Agnus],
        choir: choir(4),
        seed: 1,
        ..MassRequest::default()
    };
    let profile = profile(&Profile::fux());
    let clock = std::time::Instant::now();
    let mass = generate_mass(&request, &profile, None, &mut |event| {
        if let MassEvent::Finished(result) = &event {
            println!(
                "{:?}: {} measures, cost {}, {:.1} s",
                result.at,
                result.generated.score.measure_count(),
                result.generated.cost,
                result.seconds
            );
        }
        ControlFlow::Continue(())
    })
    .unwrap();
    println!("set in {:.0} s", clock.elapsed().as_secs_f64());
    assert!(mass.complete);
    let dir = tmp_dir("mass-files");
    let plan = &mass.plan;
    for (m, movement) in plan.movements.iter().enumerate() {
        let name = format!(
            "{}-{}",
            movement.movement as usize + 1,
            movement.movement.id()
        );
        for (suffix, options) in [
            ("", MovementOptions::default()),
            (
                "-mensur",
                MovementOptions {
                    mensurstriche: true,
                    hide_tacet: true,
                    ..MovementOptions::default()
                },
            ),
        ] {
            let (xml, score) = movement_file(&mass, m, &options);
            std::fs::write(dir.join(format!("{name}{suffix}.musicxml")), &xml).unwrap();
            validate_musicxml(&xml, &format!("{name}{suffix}.musicxml"));
            let read = from_musicxml(&xml).unwrap();
            assert_eq!(read.sections, score.sections, "{name}");
            let voices: Vec<Voice> = read
                .parts
                .iter()
                .zip(&score.voices)
                .map(|(part, voice)| Voice::new(voice.voice_type, voice.role, part.notes.clone()))
                .collect();
            let mut again = Score::new(score.mode, voices);
            again.sections = read.sections.clone();
            let split = split_movement(&again, &read.fields).unwrap();
            let places: Vec<PartRef> = plan
                .parts()
                .filter(|(at, _)| at.movement == m)
                .map(|(at, _)| at)
                .collect();
            assert_eq!(split.len(), places.len(), "{name}");
            for ((_, part, _), &at) in split.iter().zip(&places) {
                let own = &mass.result(at).unwrap().generated.score;
                for (voice, original) in part.voices.iter().zip(&own.voices) {
                    let set: Vec<Note> = original
                        .notes
                        .iter()
                        .map(|note| Note {
                            lyric: None,
                            ..note.clone()
                        })
                        .collect();
                    // A note tied from a note of musica ficta reads back without the mark, as
                    // MusicXML marks the first note of a tie only.
                    let read: Vec<(Option<fux_core::Pitch>, Duration, bool)> = voice
                        .notes
                        .iter()
                        .map(|n| (n.pitch, n.duration, n.tie))
                        .collect();
                    let written: Vec<(Option<fux_core::Pitch>, Duration, bool)> =
                        set.iter().map(|n| (n.pitch, n.duration, n.tie)).collect();
                    assert_eq!(read, written, "{name}, {}", plan.address(at));
                }
            }
        }
        let score = movement_score(&mass, m).unwrap();
        let midi = to_movement_midi(
            &score,
            &format!("{} – {}", plan.title, movement.movement.title()),
            &MidiOptions::default(),
        );
        std::fs::write(dir.join(format!("{name}.mid")), &midi).unwrap();
        let smf = Smf::parse(&midi).unwrap();
        let markers = smf.tracks[0]
            .iter()
            .filter(|e| matches!(e.kind, TrackEventKind::Meta(MetaMessage::Marker(_))))
            .count();
        let expected = score
            .sections
            .iter()
            .filter(|mark| mark.kind != SectionKind::Seam)
            .count();
        assert_eq!(markers, expected, "{name}");
    }
    println!("files in {}", dir.display());
}
