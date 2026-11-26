//! Texts to sing (phase 12): the text of `fux free`, `fux text`, and the text of the lyrics of a
//! score to check.

use std::io::Write;
use std::ops::RangeInclusive;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use fux_core::rules::{Level, Profile};
use fux_core::text::align::{align, find_text, fit};
use fux_core::text::plan::automatic_length;
use fux_core::text::{Eleison, SylFlags, Text, TextKind, TextOptions, ordinary, resolve};
use fux_core::{Lang, Score};

/// The text of `fux free` unless another is given: Kyrie I.
pub const DEFAULT_TEXT: &str = "kyrie";

/// Whether a text as the user gives it asks for none: `none` or `keine`, in any case.
pub fn is_none(spec: &str) -> bool {
    let spec = spec.trim();
    spec.eq_ignore_ascii_case("none") || spec.eq_ignore_ascii_case("keine")
}

/// The text a user names: a section of the Ordinary (`kyrie`), some of its sense units
/// (`gloria:1-5`) or an own Latin text, with eleison divided as `eleison` says; with its warnings
/// (uncertain accents) in the user's language. `None` for `none`. An unknown section is an error
/// that lists the sections.
pub fn text_of(spec: &str, eleison: Eleison, lang: Lang) -> Result<Option<(Text, Vec<String>)>> {
    if is_none(spec) {
        return Ok(None);
    }
    let (text, warnings) =
        resolve(spec, &TextOptions { eleison }).map_err(|e| anyhow!("{}", e.message(lang)))?;
    let warnings = warnings.iter().map(|w| w.message(lang)).collect();
    Ok(Some((text, warnings)))
}

/// Whether the voices may restate `text` under `profile`: TXT-REPEAT `short` or `long`, by the
/// kind of the text, is not forbidden. The planner counts the statements of a text the same way.
pub fn restates(profile: &Profile, text: &Text) -> bool {
    let kind = match text.kind {
        TextKind::Short => "short",
        TextKind::Long => "long",
    };
    profile.settings("TXT-REPEAT").level(kind) != Level::Forbidden
}

/// A number with its noun: „1 Silbe“, „7 Silben“.
fn counted(n: usize, de: (&str, &str), en: (&str, &str), lang: Lang) -> String {
    let (one, many) = match lang {
        Lang::De => de,
        Lang::En => en,
    };
    format!("{n} {}", if n == 1 { one } else { many })
}

fn syllables(n: usize, lang: Lang) -> String {
    counted(n, ("Silbe", "Silben"), ("syllable", "syllables"), lang)
}

fn units(n: usize, lang: Lang) -> String {
    counted(
        n,
        ("Sinnabschnitt", "Sinnabschnitte"),
        ("sense unit", "sense units"),
        lang,
    )
}

/// A text with its size: „Kyrie I (7 Silben, 1 Sinnabschnitt)“.
pub fn text_label(text: &Text, lang: Lang) -> String {
    format!(
        "{} ({}, {})",
        text.name(lang),
        syllables(text.syllables.len(), lang),
        units(text.units.len(), lang)
    )
}

/// The position in the text that a voice sings at `tick`: the lyric of the last note starting at
/// or before it, unless a rest came after that note. `None` where the voice is silent or sings no
/// known syllable.
pub fn sung_at(score: &Score, voice: usize, tick: u32) -> Option<usize> {
    let mut sung = None;
    for (start, note) in score.voices.get(voice)?.timed() {
        if start > tick {
            break;
        }
        if note.is_rest() {
            sung = None;
        } else if let Some(lyric) = note.lyric {
            sung = lyric.index();
        }
    }
    sung
}

/// What a voice that sings the score's text has sung of its current word at `tick`, as printed
/// under the notes: „e-lé-i-son.“ when it reaches the end of the word there, „e-lé-“ inside it.
/// `None` where it sings nothing.
pub fn sung_word(score: &Score, voice: usize, tick: u32, accents: bool) -> Option<String> {
    let text = &score.text.as_ref()?.text;
    let position = sung_at(score, voice, tick).filter(|&p| p < text.syllables.len())?;
    let first = usize::from(text.word_of(position).first);
    Some(
        (first..=position)
            .map(|i| text.syllable_label(i, accents))
            .collect(),
    )
}

/// A syllable as `fux text` shows it: in capitals if its word is stressed on it.
fn shown(text: &Text, syl: usize) -> String {
    let syllable = &text.syllables[syl];
    if syllable.flags.contains(SylFlags::STRESS) {
        syllable.text.to_uppercase()
    } else {
        syllable.text.clone()
    }
}

/// Words of a text as `fux text` shows them: the syllables joined by hyphens, the stressed ones in
/// capitals, with the punctuation, and between the words the markers `/` (a tail begins) and `|`
/// (a break).
fn shown_words(text: &Text, words: RangeInclusive<usize>) -> String {
    let first_word = *words.start();
    let mut out = String::new();
    for w in words {
        let word = &text.words[w];
        let first = usize::from(word.first);
        if w > first_word {
            let broken = text.syllables[first - 1]
                .flags
                .contains(SylFlags::BREAK_AFTER);
            let tail = text.syllables[first].flags.contains(SylFlags::TAIL_START);
            out.push_str(match (broken, tail) {
                (true, true) => " / ",
                (true, false) => " | ",
                (false, _) => " ",
            });
        }
        let parts: Vec<String> = word.range().map(|i| shown(text, i)).collect();
        out.push_str(&parts.join("-"));
        out.extend(word.punct);
    }
    out
}

/// The words from the one of syllable `from` to the one of syllable `to`.
fn words_of(text: &Text, from: usize, to: usize) -> RangeInclusive<usize> {
    usize::from(text.syllables[from].word)..=usize::from(text.syllables[to].word)
}

/// `fux text ID|TEXT`: the sense units with the stressed syllables in capitals, the breaks and
/// tails, the warnings, and the length a free setting gets from the text.
pub fn show_text(out: &mut impl Write, spec: &str, eleison: Eleison, lang: Lang) -> Result<()> {
    let Some((text, warnings)) = text_of(spec, eleison, lang)? else {
        writeln!(
            out,
            "{}",
            lang.pick(
                "Kein Text: Die Stimmen singen ohne Text.",
                "No text: the voices sing without text."
            )
        )?;
        return Ok(());
    };
    let sentences = text.sentences().count();
    writeln!(
        out,
        "{} ({}), {}: {}, {}, {}, {}",
        text.name(lang),
        text.id,
        text.kind.name(lang),
        syllables(text.syllables.len(), lang),
        counted(
            text.words.len(),
            ("Wort", "Wörter"),
            ("word", "words"),
            lang
        ),
        units(text.units.len(), lang),
        counted(
            sentences,
            ("Satz", "Sätze"),
            ("sentence", "sentences"),
            lang
        )
    )?;
    for (u, unit) in text.units.iter().enumerate() {
        let line = shown_words(
            &text,
            words_of(&text, usize::from(unit.first), usize::from(unit.last)),
        );
        writeln!(out, "{:>3}  {line}", u + 1)?;
    }
    let none = lang.pick("keine", "none");
    let breaks: Vec<String> = (0..text.syllables.len().saturating_sub(1))
        .filter(|&s| {
            text.syllables[s].flags.contains(SylFlags::BREAK_AFTER)
                && !text.syllables[s + 1].flags.contains(SylFlags::TAIL_START)
        })
        .map(|s| {
            let w = usize::from(text.syllables[s].word);
            format!(
                "{} {}",
                lang.pick("nach", "after"),
                shown_words(&text, w..=w)
            )
        })
        .collect();
    writeln!(
        out,
        "{} (|): {}",
        lang.pick("Zäsuren", "Breaks"),
        if breaks.is_empty() {
            none.to_string()
        } else {
            breaks.join(", ")
        }
    )?;
    let tails: Vec<String> = (0..text.syllables.len())
        .filter(|&s| text.syllables[s].flags.contains(SylFlags::TAIL_START))
        .map(|s| shown_words(&text, words_of(&text, s, usize::from(text.unit_of(s).last))))
        .collect();
    writeln!(
        out,
        "{} (/): {}",
        lang.pick("Schlussteile", "Tails"),
        if tails.is_empty() {
            none.to_string()
        } else {
            tails.join(" ")
        }
    )?;
    writeln!(
        out,
        "{}",
        lang.pick(
            "Betonte Silben in Großbuchstaben. An einer Zäsur darf eine Stimme pausieren; ein Schlussteil darf für sich wiederholt werden.",
            "Stressed syllables in capitals. A voice may rest at a break; a tail may be restated on its own."
        )
    )?;
    for warning in &warnings {
        writeln!(out, "{}: {warning}", lang.pick("Warnung", "Warning"))?;
    }
    let length = automatic_length(&text, true);
    let once = automatic_length(&text, false);
    write!(
        out,
        "{}: {length} {}",
        lang.pick("Automatische Länge", "Automatic length"),
        lang.pick("Takte", "measures")
    )?;
    if once != length {
        write!(
            out,
            "; {} {once} {}",
            lang.pick(
                "ohne Wiederholung (--repeat keine)",
                "without repetition (--repeat none)"
            ),
            lang.pick("Takte", "measures")
        )?;
    }
    writeln!(out)?;
    Ok(())
}

/// `fux text --list`: the sections of the Ordinary with their syllables, sense units and
/// automatic lengths.
pub fn list_texts(out: &mut impl Write, eleison: Eleison, lang: Lang) -> Result<()> {
    writeln!(
        out,
        "{:<12} {:<24} {:<5} {:>6} {:>15} {:>6}",
        lang.pick("Abschnitt", "Section"),
        "Name",
        lang.pick("Art", "Kind"),
        lang.pick("Silben", "Syll."),
        lang.pick("Sinnabschnitte", "Sense units"),
        lang.pick("Takte", "Bars")
    )?;
    for text in ordinary::sections(&TextOptions { eleison }) {
        writeln!(
            out,
            "{:<12} {:<24} {:<5} {:>6} {:>15} {:>6}",
            text.id,
            text.name(lang),
            text.kind.name(lang),
            text.syllables.len(),
            text.units.len(),
            automatic_length(text, true)
        )?;
    }
    writeln!(
        out,
        "{}",
        lang.pick(
            "Takte: automatische Länge eines freien Satzes. ID:A-B wählt die Sinnabschnitte A bis B (gloria:1-5); ein eigener lateinischer Text steht in Anführungszeichen.",
            "Bars: automatic length of a free setting. ID:A-B selects the sense units A to B (gloria:1-5); an own Latin text goes in quotes."
        )
    )?;
    Ok(())
}

/// How many syllables the voices of a score sing: those of the voices besides a cantus firmus,
/// as [`align`] counts them.
fn sung_syllables(score: &Score) -> usize {
    // Any text counts the syllables of the score alike.
    let any = &ordinary::sections(&TextOptions::default())[0];
    fit(score, any).syllables
}

/// The line of [`align_lyrics`] when it recognises no text, in the words of the command line;
/// the GUI and the web version say where they take the text instead.
pub fn unrecognised(lang: Lang) -> &'static str {
    lang.pick(
        "Text nicht erkannt; mit --text angeben",
        "Text not recognised; give it with --text",
    )
}

/// Aligns the lyrics of a score with their text, so that a check checks the text rules: with the
/// text `spec` names (a section, a range or an own Latin text, eleison divided as the lyrics fit
/// better), with none for `none`, and without `spec` with the section of the Ordinary, or the
/// units of it, that the lyrics set, if one is recognised. A score without lyrics stays as it
/// is. Returns lines for the user: which text, and the syllables that do not fit it.
pub fn align_lyrics(score: &mut Score, spec: Option<&str>, lang: Lang) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    let warning = |message: String| format!("{}: {message}", lang.pick("Warnung", "Warning"));
    let mut text_warnings = Vec::new();
    let (text, eleison, recognised) = match spec {
        Some(spec) if is_none(spec) => {
            score.text = None;
            for note in score.voices.iter_mut().flat_map(|v| v.notes.iter_mut()) {
                note.lyric = None;
            }
            return Ok(lines);
        }
        Some(spec) => {
            let read = |eleison| {
                resolve(spec, &TextOptions { eleison }).map_err(|e| anyhow!("{}", e.message(lang)))
            };
            let (four, warnings) = read(Eleison::Four)?;
            let (three, _) = read(Eleison::Three)?;
            text_warnings = warnings.iter().map(|w| warning(w.message(lang))).collect();
            if three.syllables != four.syllables
                && fit(score, &three).unmatched < fit(score, &four).unmatched
            {
                (Arc::new(three), Eleison::Three, false)
            } else {
                (Arc::new(four), Eleison::Four, false)
            }
        }
        None => {
            if sung_syllables(score) == 0 {
                return Ok(lines);
            }
            let Some((text, eleison, _)) = find_text(score) else {
                lines.push(unrecognised(lang).to_string());
                return Ok(lines);
            };
            (text, eleison, true)
        }
    };
    let fit = fit(score, &text);
    if fit.syllables > 0 {
        let mut name = text.name(lang).to_string();
        if eleison == Eleison::Three {
            name.push_str(", e-léi-son");
        }
        let head = if recognised {
            lang.pick("Text erkannt", "Text recognised")
        } else {
            "Text"
        };
        lines.push(match lang {
            Lang::De => format!(
                "{head}: {name}; von {} gesungenen Silben weichen {} ab",
                fit.syllables, fit.unmatched
            ),
            Lang::En => format!(
                "{head}: {name}; {} of {} sung syllables unmatched",
                fit.unmatched, fit.syllables
            ),
        });
    }
    lines.extend(text_warnings);
    for aligned in align(score, text) {
        lines.push(warning(aligned.message(lang)));
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use fux_core::{Duration, Lyric, Note, Pitch, Role, Species, Step, Voice, VoiceType};

    use super::*;

    fn kyrie() -> Text {
        text_of("kyrie", Eleison::Four, Lang::De)
            .unwrap()
            .unwrap()
            .0
    }

    /// A free voice in halves, one per syllable token; `_` holds the syllable before.
    fn voice(voice_type: VoiceType, tokens: &[&str]) -> Voice {
        let notes = tokens
            .iter()
            .enumerate()
            .map(|(i, &token)| {
                let step = if i % 2 == 0 { Step::D } else { Step::E };
                let mut note = Note::new(Pitch::new(step, 0, 4), Duration::HALF);
                if token != "_" {
                    note.syllable = Some(token.to_string());
                }
                note
            })
            .collect();
        Voice::new(voice_type, Role::Counterpoint(Species::Free), notes)
    }

    fn sung(tokens: &[&str]) -> Score {
        let mode = "dorian".parse().unwrap();
        Score::new(
            mode,
            vec![
                voice(VoiceType::Soprano, tokens),
                voice(VoiceType::Tenor, tokens),
            ],
        )
    }

    const KYRIE: [&str; 8] = ["Ký-", "ri-", "e", "e-", "lé-", "i-", "son.", "_"];

    #[test]
    fn texts_by_name_own_or_none() {
        assert!(is_none("none") && is_none(" Keine ") && !is_none("kyrie"));
        assert!(text_of("none", Eleison::Four, Lang::De).unwrap().is_none());
        let text = kyrie();
        assert_eq!(
            text_label(&text, Lang::De),
            "Kyrie I (7 Silben, 1 Sinnabschnitt)"
        );
        assert_eq!(
            text_label(&text, Lang::En),
            "Kyrie I (7 syllables, 1 sense unit)"
        );
        let (three, _) = text_of("kyrie", Eleison::Three, Lang::De).unwrap().unwrap();
        assert_eq!(three.syllables.len(), 6);
        let (own, warnings) = text_of("Ad te levavi animam meam.", Eleison::Four, Lang::De)
            .unwrap()
            .unwrap();
        assert_eq!(own.id, "eigen");
        assert_eq!(
            warnings,
            ["Akzent unsicher: animam → á-ni-mam; mit Akut angeben"]
        );
        let unknown = text_of("kyrei", Eleison::Four, Lang::De).unwrap_err();
        let message = unknown.to_string();
        assert!(
            message
                .starts_with("Unbekannter Abschnitt „kyrei“; die Abschnitte sind kyrie, christe"),
            "{message}"
        );
    }

    #[test]
    fn restatements_follow_the_profile() {
        let mut profile = Profile::fux();
        let gloria = text_of("gloria", Eleison::Four, Lang::De)
            .unwrap()
            .unwrap()
            .0;
        assert!(restates(&profile, &kyrie()) && restates(&profile, &gloria));
        profile.apply_override("TXT-REPEAT.short=V").unwrap();
        assert!(!restates(&profile, &kyrie()) && restates(&profile, &gloria));
    }

    #[test]
    fn showing_a_text() {
        let show = |spec: &str, lang| {
            let mut out = Vec::new();
            show_text(&mut out, spec, Eleison::Four, lang).unwrap();
            String::from_utf8(out).unwrap()
        };
        let kyrie = show("kyrie", Lang::De);
        assert_eq!(
            kyrie,
            "Kyrie I (kyrie), kurz: 7 Silben, 2 Wörter, 1 Sinnabschnitt, 1 Satz\n  1  KÝ-ri-e / e-LÉ-i-son.\nZäsuren (|): keine\nSchlussteile (/): e-LÉ-i-son.\nBetonte Silben in Großbuchstaben. An einer Zäsur darf eine Stimme pausieren; ein Schlussteil darf für sich wiederholt werden.\nAutomatische Länge: 16 Takte; ohne Wiederholung (--repeat keine) 9 Takte\n"
        );
        let benedictus = show("benedictus", Lang::En);
        assert!(
            benedictus
                .contains("Breaks (|): after Be-ne-DÍ-ctus\nTails (/): in NÓ-mi-ne DÓ-mi-ni.\n"),
            "{benedictus}"
        );
        let gloria = show("gloria:1-5", Lang::De);
        assert!(gloria.contains("36 Silben") && gloria.contains("5 Sinnabschnitte"));
        assert!(
            gloria.contains("  1  Et in TER-ra PAX | ho-MÍ-ni-bus"),
            "{gloria}"
        );
        assert!(
            gloria.ends_with("Automatische Länge: 32 Takte\n"),
            "{gloria}"
        );
        assert!(show("none", Lang::De).starts_with("Kein Text"));
    }

    #[test]
    fn listing_the_sections() {
        let mut out = Vec::new();
        list_texts(&mut out, Eleison::Three, Lang::De).unwrap();
        let list = String::from_utf8(out).unwrap();
        // A header, the 17 sections and a note.
        assert_eq!(list.lines().count(), 1 + 17 + 1, "{list}");
        // With e-léi-son, Kyrie I has six syllables, sung twice in 12 measures and the slack.
        let kyrie = list.lines().nth(1).unwrap();
        assert_eq!(
            kyrie.split_whitespace().collect::<Vec<_>>(),
            ["kyrie", "Kyrie", "I", "kurz", "6", "1", "14"]
        );
    }

    #[test]
    fn the_word_sung_at_a_tick() {
        let mut score = sung(&KYRIE);
        assert!(align_lyrics(&mut score, None, Lang::De).unwrap()[0].starts_with("Text erkannt"));
        score.fill_syllables(true);
        // Halves: Ký at 0, ri at 4, e at 8 (measure 2), e at 12, lé at 16, i at 20, son at 24.
        assert_eq!(sung_word(&score, 0, 0, true).as_deref(), Some("Ký-"));
        assert_eq!(sung_word(&score, 0, 18, false).as_deref(), Some("e-le-"));
        assert_eq!(
            sung_word(&score, 1, 28, true).as_deref(),
            Some("e-lé-i-son.")
        );
        assert_eq!(sung_at(&score, 1, 28), Some(6));
    }

    #[test]
    fn lyrics_are_aligned_for_the_check() {
        // Found without --text.
        let mut score = sung(&KYRIE);
        let lines = align_lyrics(&mut score, None, Lang::De).unwrap();
        assert_eq!(
            lines,
            ["Text erkannt: Kyrie I; von 14 gesungenen Silben weichen 0 ab"]
        );
        let sung_text = score.text.as_ref().unwrap();
        assert_eq!(sung_text.voices, [true, true]);
        let placed = |n: &Note| n.lyric.and_then(Lyric::index).is_some();
        assert!(score.voices[0].notes[..7].iter().all(placed));
        assert!(!placed(&score.voices[0].notes[7]));
        // Named, in English: Jeppesen's e-lei-son takes the division in three syllables.
        let mut three = sung(&["Ky-", "ri-", "e", "e-", "lei-", "son.", "_", "_"]);
        let lines = align_lyrics(&mut three, Some("Kyrie"), Lang::En).unwrap();
        assert_eq!(
            lines,
            ["Text: Kyrie I, e-léi-son; 0 of 12 sung syllables unmatched"]
        );
        assert_eq!(three.text.as_ref().unwrap().text.syllables.len(), 6);
        // `none` checks no text.
        let lines = align_lyrics(&mut score, Some("none"), Lang::De).unwrap();
        assert!(lines.is_empty());
        assert!(score.text.is_none() && score.voices[0].notes.iter().all(|n| n.lyric.is_none()));
        // Lyrics of no section.
        let mut motet = sung(&["Ad", "te", "le-", "va-", "vi", "_", "_", "_"]);
        let lines = align_lyrics(&mut motet, None, Lang::De).unwrap();
        assert_eq!(lines, ["Text nicht erkannt; mit --text angeben"]);
        assert!(motet.text.is_none());
        let lines = align_lyrics(&mut motet, Some("Ad te le-vá-vi."), Lang::De).unwrap();
        assert_eq!(
            lines,
            ["Text: Eigener Text; von 10 gesungenen Silben weichen 0 ab"]
        );
        // Without lyrics nothing is said, unless a text is named.
        let mut plain = sung(&["_"; 8]);
        assert!(align_lyrics(&mut plain, None, Lang::De).unwrap().is_empty());
        let lines = align_lyrics(&mut plain, Some("kyrie"), Lang::De).unwrap();
        assert_eq!(
            lines,
            ["Warnung: Keine Stimme trägt Text; die Textregeln bleiben ungeprüft."]
        );
        assert!(plain.text.is_none());
        // Syllables that fit nowhere are named.
        let mut odd = sung(&["Ký-", "ri-", "e", "xx", "e-", "lé-", "i-", "son."]);
        let lines = align_lyrics(&mut odd, Some("kyrie"), Lang::De).unwrap();
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert_eq!(
            lines[0],
            "Text: Kyrie I; von 16 gesungenen Silben weichen 2 ab"
        );
        assert_eq!(
            lines[1],
            "Warnung: Sopran: 1 Silbe nicht im Text: „xx“ (Takt 2)."
        );
    }
}
