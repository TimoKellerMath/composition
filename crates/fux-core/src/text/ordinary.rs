//! The Ordinary of the Mass after the Graduale Romanum of 1961 (Mass VIII and Credo I), embedded
//! from `ordinary.txt` and parsed once for each division of eleison.
//!
//! Each section starts with a header line `[id] kind=short|long de="…" en="…"`, followed by its
//! text, one sense unit per line. Lines starting with `#` are comments.

use std::sync::OnceLock;

use super::{Eleison, PUNCTUATION, Text, TextKind, TextOptions};

const SOURCE: &str = include_str!("ordinary.txt");

static FOUR: OnceLock<Vec<Text>> = OnceLock::new();
static THREE: OnceLock<Vec<Text>> = OnceLock::new();

/// All sections in the order of the Mass: kyrie, christe, kyrie2, gloria, qui-tollis, credo,
/// incarnatus, crucifixus, resurrexit, spiritum, sanctus, pleni, osanna, benedictus, osanna2,
/// agnus (Agnus Dei I and II) and agnus3.
pub fn sections(options: &TextOptions) -> &'static [Text] {
    let cell = match options.eleison {
        Eleison::Four => &FOUR,
        Eleison::Three => &THREE,
    };
    cell.get_or_init(|| {
        raw_sections()
            .into_iter()
            .map(|raw| {
                let names = (raw.name_de, raw.name_en);
                match Text::parse(raw.id, names, raw.kind, &raw.body, options) {
                    Ok((text, _)) => text,
                    Err(error) => panic!("ordinary.txt, section {}: {error}", raw.id),
                }
            })
            .collect()
    })
}

/// The section with this id.
pub fn section(id: &str, options: &TextOptions) -> Option<&'static Text> {
    sections(options).iter().find(|text| text.id == id)
}

/// The intonation of the Gloria, which the celebrant sings before the choir begins with Et in
/// terra, as the Graduale Romanum of 1961 prints it: with the acutes of its words of three
/// syllables and more, and its punctuation.
pub const GLORIA_INTONATION: &str = "Glória in excélsis Deo.";

/// The intonation of the Credo, before Patrem omnipotentem, as the Graduale Romanum of 1961 prints
/// it (Credo I); the sentence goes on in the choir.
pub const CREDO_INTONATION: &str = "Credo in unum Deum,";

/// The intonation sung before a section of the Ordinary, by its id: the Gloria's before
/// `gloria`, the Credo's before `credo`; `None` for every other section. The polyphony begins
/// after it (docs/regeln.md § 17), so the book of a mass prints it above its movement (phase
/// 14).
pub fn intonation(id: &str) -> Option<&'static str> {
    match id {
        "gloria" => Some(GLORIA_INTONATION),
        "credo" => Some(CREDO_INTONATION),
        _ => None,
    }
}

/// The words of all sections as written in `ordinary.txt`, with hyphens and acutes but without
/// punctuation and markers: `Ký-ri-e`, `e-lé-i-son`, … The lexicon of the syllabifier reads them
/// here, because the parser calls the syllabifier.
pub fn words() -> impl Iterator<Item = &'static str> {
    SOURCE
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(['#', '[']))
        .flat_map(|line| line.split([' ', '\t', '/', '|']))
        .map(|token| token.trim_end_matches(PUNCTUATION))
        .filter(|word| !word.is_empty())
}

/// A section as written in `ordinary.txt`.
struct RawSection {
    id: &'static str,
    kind: TextKind,
    name_de: &'static str,
    name_en: &'static str,
    body: String,
}

fn raw_sections() -> Vec<RawSection> {
    let mut sections: Vec<RawSection> = Vec::new();
    for line in SOURCE.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            sections.push(
                header(line).unwrap_or_else(|| panic!("ordinary.txt: invalid header {line:?}")),
            );
        } else {
            let section = sections
                .last_mut()
                .expect("ordinary.txt: text before the first header");
            section.body.push_str(line);
            section.body.push('\n');
        }
    }
    sections
}

/// Reads `[id] kind=short|long de="…" en="…"`.
fn header(line: &'static str) -> Option<RawSection> {
    let (id, mut rest) = line.strip_prefix('[')?.split_once(']')?;
    let (mut kind, mut name_de, mut name_en) = (None, None, None);
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        let (key, value) = rest.split_once('=')?;
        let (value, after) = match value.strip_prefix('"') {
            Some(quoted) => quoted.split_once('"')?,
            None => value.split_once(' ').unwrap_or((value, "")),
        };
        match key {
            "kind" if value == "short" => kind = Some(TextKind::Short),
            "kind" if value == "long" => kind = Some(TextKind::Long),
            "de" => name_de = Some(value),
            "en" => name_en = Some(value),
            _ => return None,
        }
        rest = after;
    }
    Some(RawSection {
        id,
        kind: kind?,
        name_de: name_de?,
        name_en: name_en?,
        body: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{SylFlags, latin, resolve};

    /// Kind, syllables, words, units and sentences of every section, pinned so that any
    /// correction of the text shows up.
    const COUNTS: [(&str, TextKind, usize, usize, usize, usize); 17] = [
        ("kyrie", TextKind::Short, 7, 2, 1, 1),
        ("christe", TextKind::Short, 6, 2, 1, 1),
        ("kyrie2", TextKind::Short, 7, 2, 1, 1),
        ("gloria", TextKind::Long, 98, 40, 13, 9),
        ("qui-tollis", TextKind::Long, 89, 40, 13, 8),
        ("credo", TextKind::Long, 128, 58, 16, 6),
        ("incarnatus", TextKind::Long, 25, 13, 2, 1),
        ("crucifixus", TextKind::Long, 24, 11, 3, 1),
        ("resurrexit", TextKind::Long, 62, 29, 7, 3),
        ("spiritum", TextKind::Long, 120, 48, 12, 7),
        ("sanctus", TextKind::Short, 14, 6, 3, 1),
        ("pleni", TextKind::Short, 13, 7, 1, 1),
        ("osanna", TextKind::Short, 7, 3, 1, 1),
        ("benedictus", TextKind::Short, 14, 6, 1, 1),
        ("osanna2", TextKind::Short, 7, 3, 1, 1),
        ("agnus", TextKind::Short, 18, 8, 3, 1),
        ("agnus3", TextKind::Short, 18, 9, 3, 1),
    ];

    /// Syllables that start a tail and syllables followed by a break, per section.
    const MARKERS: [(&str, &[&str], &[&str]); 17] = [
        ("kyrie", &["e"], &["e"]),
        ("christe", &["e"], &["ste"]),
        ("kyrie2", &["e"], &["e"]),
        ("gloria", &[], &["pax", "bi", "li", "te"]),
        ("qui-tollis", &[], &["pe", "des"]),
        ("credo", &[], &["num", "tum", "tem"]),
        ("incarnatus", &[], &["est", "cto"]),
        ("crucifixus", &[], &["to"]),
        ("resurrexit", &[], &[]),
        ("spiritum", &[], &["o", "est", "ctam", "ma", "cto"]),
        ("sanctus", &["Dó"], &["ctus"]),
        ("pleni", &["gló"], &["li", "ra"]),
        ("osanna", &["in"], &["na"]),
        ("benedictus", &["in"], &["ctus", "nit"]),
        ("osanna2", &["in"], &["na"]),
        ("agnus", &[], &[]),
        ("agnus3", &[], &[]),
    ];

    #[test]
    fn ordinary_sections_parse() {
        let options = TextOptions::default();
        let sections = sections(&options);
        assert_eq!(sections.len(), COUNTS.len());
        for (text, &(id, kind, syllables, words, units, sentences)) in sections.iter().zip(&COUNTS)
        {
            assert_eq!(text.id, id);
            assert_eq!(text.kind, kind, "{id}");
            let counts = (
                text.syllables.len(),
                text.words.len(),
                text.units.len(),
                text.sentences().count(),
            );
            assert_eq!(counts, (syllables, words, units, sentences), "{id}");
            // Acutes only on words of three or more syllables, there on the stressed syllable.
            for word in &text.words {
                let acutes: Vec<usize> = word
                    .range()
                    .filter(|&i| latin::has_acute(&text.syllables[i].text))
                    .map(|i| i - usize::from(word.first))
                    .collect();
                let expected: Vec<usize> = match word.stress {
                    Some(k) if word.len >= 3 => vec![usize::from(k)],
                    _ => Vec::new(),
                };
                assert_eq!(acutes, expected, "{id}: {}", text.render(true, false));
            }
            assert!(
                text.syllables
                    .iter()
                    .all(|s| !s.flags.contains(SylFlags::UNCERTAIN)),
                "{id}"
            );
        }
        // The books' text parses without warnings.
        for raw in raw_sections() {
            let names = (raw.name_de, raw.name_en);
            let (_, warnings) = Text::parse(raw.id, names, raw.kind, &raw.body, &options).unwrap();
            assert!(warnings.is_empty(), "{}: {warnings:?}", raw.id);
        }
        // Et in terra … Glorificamus te.
        let (gloria, _) = resolve("gloria:1-5", &options).unwrap();
        assert_eq!((gloria.syllables.len(), gloria.units.len()), (36, 5));
    }

    #[test]
    fn markers_of_the_ordinary() {
        let options = TextOptions::default();
        for (text, &(id, tails, breaks)) in sections(&options).iter().zip(&MARKERS) {
            assert_eq!(text.id, id);
            let with = |flag: SylFlags, without: SylFlags| -> Vec<&str> {
                text.syllables
                    .iter()
                    .filter(|s| s.flags.contains(flag) && !s.flags.intersects(without))
                    .map(|s| s.text.as_str())
                    .collect()
            };
            assert_eq!(
                with(SylFlags::TAIL_START, SylFlags::UNIT_START),
                tails,
                "{id}"
            );
            assert_eq!(
                with(SylFlags::BREAK_AFTER, SylFlags::empty()),
                breaks,
                "{id}"
            );
        }
        // Rendered with its markers, every section gives back its text in ordinary.txt.
        for (text, raw) in sections(&options).iter().zip(raw_sections()) {
            let source: Vec<&str> = raw.body.split_whitespace().collect();
            assert_eq!(text.render(true, true), source.join(" "), "{}", raw.id);
        }
        let benedictus = section("benedictus", &options).unwrap();
        assert_eq!(
            benedictus.render(true, true),
            "Be-ne-dí-ctus | qui ve-nit / in nó-mi-ne Dó-mi-ni."
        );
        assert_eq!(
            benedictus.render(false, false),
            "Be-ne-di-ctus qui ve-nit in no-mi-ne Do-mi-ni."
        );
    }

    #[test]
    fn ordinary_text_is_pinned() {
        // FNV-1a over all sections with their markers: a correction of the text, which must be
        // checked against the Graduale of 1961, changes it.
        let all: Vec<String> = sections(&TextOptions::default())
            .iter()
            .map(|text| format!("[{}] {}", text.id, text.render(true, true)))
            .collect();
        let hash = crate::generate::fingerprint(&all.join("\n"));
        assert_eq!(hash, 0xd0c9_854c_c187_7672, "ordinary.txt changed");
    }

    #[test]
    fn words_as_written() {
        let parsed: Vec<String> = sections(&TextOptions::default())
            .iter()
            .flat_map(|text| {
                text.words.iter().map(|word| {
                    word.range()
                        .map(|i| text.syllables[i].text.as_str())
                        .collect::<Vec<_>>()
                        .join("-")
                })
            })
            .collect();
        assert_eq!(words().collect::<Vec<_>>(), parsed);
        assert_eq!(parsed.len(), 287);
        assert_eq!(words().next(), Some("Ký-ri-e"));
    }

    /// The intonations precede the sections that begin the Gloria and the Credo, and they are
    /// written as the sections are: parsed as own texts they keep their acutes and syllables and
    /// need no acute added.
    #[test]
    fn intonations() {
        assert_eq!(intonation("gloria"), Some(GLORIA_INTONATION));
        assert_eq!(intonation("credo"), Some(CREDO_INTONATION));
        for id in [
            "kyrie",
            "qui-tollis",
            "incarnatus",
            "sanctus",
            "gloria:1-5",
            "eigen",
        ] {
            assert_eq!(intonation(id), None, "{id}");
        }
        let options = TextOptions::default();
        for (src, divided) in [
            (GLORIA_INTONATION, "Gló-ri-a in ex-cél-sis De-o."),
            (CREDO_INTONATION, "Cre-do in u-num De-um,"),
        ] {
            let (text, warnings) = Text::own(src, &options).unwrap();
            assert!(warnings.is_empty(), "{src}");
            assert_eq!(text.render(true, false), divided);
            assert_eq!(divided.replace('-', ""), src);
        }
    }

    #[test]
    fn sections_by_id() {
        let three = TextOptions {
            eleison: Eleison::Three,
        };
        assert_eq!(section("credo", &three).unwrap().name_de, "Credo: Patrem");
        assert_eq!(
            section("agnus", &three).unwrap().name_en,
            "Agnus Dei I and II"
        );
        assert_eq!(section("kyrie", &three).unwrap().syllables.len(), 6);
        assert!(section("gloria:1-5", &three).is_none());
        assert!(section("Kyrie", &three).is_none());
    }
}
