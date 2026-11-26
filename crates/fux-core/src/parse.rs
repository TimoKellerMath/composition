//! Parsers for the two note input formats: scientific note names (`D4 F#4 Bb3`) for pitch lists,
//! and LilyPond absolute notation (`d'1 fis'2 bes4~ bes4`) for notes with durations.

use crate::error::{ParseError, ParseErrorKind};
use crate::pitch::{Pitch, Step, parse_scientific};
use crate::score::Note;
use crate::time::{Duration, Meter, TICKS_PER_WHOLE};

/// Splits input into whitespace-separated tokens with their byte offsets.
fn tokens(input: &str) -> impl Iterator<Item = (usize, &str)> {
    input
        .split_whitespace()
        .map(move |t| (t.as_ptr() as usize - input.as_ptr() as usize, t))
}

/// Parses pitches in scientific notation separated by whitespace, e.g. `D4 F4 E4 C#5 Bb3`.
/// Bar lines `|` are ignored.
pub fn parse_pitches(input: &str) -> Result<Vec<Pitch>, ParseError> {
    tokens(input)
        .filter(|(_, t)| *t != "|")
        .map(|(pos, t)| parse_scientific(t, pos))
        .collect()
}

/// Parses LilyPond absolute notation with Dutch note names (`c' = C4`, `cis`, `bes`, `es`, `as`).
///
/// Supported: notes and rests (`r`) with durations `1 2 4 8` and `\breve`, one dot, ties (`~`
/// attached or as a separate token), cautionary accidentals (`!`, `?`, ignored) and bar checks
/// `|` (ignored here, see [`parse_lilypond_measures`]). A note without a duration repeats the
/// previous duration; the first one defaults to `default`.
pub fn parse_lilypond(input: &str, default: Duration) -> Result<Vec<Note>, ParseError> {
    parse(input, default, None)
}

/// Like [`parse_lilypond`], but every bar check `|` must fall on a barline of `meter`.
pub fn parse_lilypond_measures(
    input: &str,
    default: Duration,
    meter: Meter,
) -> Result<Vec<Note>, ParseError> {
    parse(input, default, Some(meter))
}

/// Parses either input format: scientific note names start with an upper-case letter
/// (`D4 F4 E4`) and get the duration `default`; everything else is read as LilyPond
/// (`d'1 f' e'`), with bar checks against `meter`.
pub fn parse_notes(input: &str, default: Duration, meter: Meter) -> Result<Vec<Note>, ParseError> {
    let scientific = tokens(input)
        .find(|(_, t)| *t != "|")
        .is_some_and(|(_, t)| t.starts_with(|c: char| c.is_ascii_uppercase()));
    if scientific {
        Ok(parse_pitches(input)?
            .into_iter()
            .map(|p| Note::new(p, default))
            .collect())
    } else {
        parse_lilypond_measures(input, default, meter)
    }
}

fn parse(input: &str, default: Duration, meter: Option<Meter>) -> Result<Vec<Note>, ParseError> {
    let mut notes: Vec<Note> = Vec::new();
    let mut duration = default;
    let mut tick = 0;
    for (pos, token) in tokens(input) {
        match token {
            "|" => {
                if let Some(meter) = meter {
                    let into = tick % meter.ticks_per_measure();
                    if into != 0 {
                        return Err(ParseError::new(ParseErrorKind::BarCheck(into), token, pos));
                    }
                }
            }
            "~" => tie_last(&mut notes, token, pos)?,
            _ => {
                let (note, explicit) = parse_note(token, pos, duration)?;
                if let Some(d) = explicit {
                    duration = d;
                }
                tick += note.duration.ticks();
                notes.push(note);
            }
        }
    }
    Ok(notes)
}

fn tie_last(notes: &mut [Note], token: &str, pos: usize) -> Result<(), ParseError> {
    match notes.last_mut() {
        Some(last) if !last.is_rest() => {
            last.tie = true;
            Ok(())
        }
        _ => Err(ParseError::new(ParseErrorKind::DanglingTie, token, pos)),
    }
}

/// Parses one note or rest token; returns the note and its explicit duration, if any.
fn parse_note(
    token: &str,
    pos: usize,
    current: Duration,
) -> Result<(Note, Option<Duration>), ParseError> {
    let error = |kind| ParseError::new(kind, token, pos);
    let (body, tie) = match token.strip_suffix('~') {
        Some(body) => (body, true),
        None => (token, false),
    };
    let bytes = body.as_bytes();
    let (pitch, mut i) = if body.starts_with('r') {
        (None, 1)
    } else {
        let step = bytes
            .first()
            .and_then(|&b| {
                let c = b as char;
                c.is_ascii_lowercase()
                    .then(|| Step::from_letter(c))
                    .flatten()
            })
            .ok_or_else(|| error(ParseErrorKind::UnknownToken))?;
        let (alter, used) = dutch_accidental(step, &body[1..]);
        let mut i = 1 + used;
        let mut octave: i8 = 3;
        while let Some(&b) = bytes.get(i) {
            match b {
                b'\'' => octave += 1,
                b',' => octave -= 1,
                _ => break,
            }
            i += 1;
        }
        while matches!(bytes.get(i), Some(b'!' | b'?')) {
            i += 1;
        }
        (Some(Pitch::new(step, alter, octave)), i)
    };
    let (explicit, used) = parse_duration(&body[i..]).map_err(error)?;
    i += used;
    if i != body.len() {
        return Err(error(ParseErrorKind::UnknownToken));
    }
    let note = Note {
        pitch,
        duration: explicit.unwrap_or(current),
        tie: tie && pitch.is_some(),
        ficta: false,
        syllable: None,
        lyric: None,
    };
    if tie && pitch.is_none() {
        return Err(error(ParseErrorKind::DanglingTie));
    }
    Ok((note, explicit))
}

/// Dutch accidental suffix after the letter: `is`, `isis`, `es`, `eses`, and for E and A also
/// the short forms `s` and `ses`. Returns the alteration and the bytes consumed.
fn dutch_accidental(step: Step, rest: &str) -> (i8, usize) {
    const SUFFIXES: [(&str, i8); 4] = [("isis", 2), ("is", 1), ("eses", -2), ("es", -1)];
    for (suffix, alter) in SUFFIXES {
        if rest.starts_with(suffix) {
            return (alter, suffix.len());
        }
    }
    if matches!(step, Step::E | Step::A) {
        if rest.starts_with("ses") {
            return (-2, 3);
        }
        if rest.starts_with('s') {
            return (-1, 1);
        }
    }
    (0, 0)
}

/// Parses an optional duration (`1`, `2`, `4`, `8`, `\breve`, each with at most one dot).
fn parse_duration(s: &str) -> Result<(Option<Duration>, usize), ParseErrorKind> {
    let (base, mut used) = if s.starts_with("\\breve") {
        (2 * TICKS_PER_WHOLE, "\\breve".len())
    } else {
        let digits = s.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return Ok((None, 0));
        }
        let value: u32 = s[..digits]
            .parse()
            .map_err(|_| ParseErrorKind::UnsupportedDuration)?;
        if !matches!(value, 1 | 2 | 4 | 8) {
            return Err(ParseErrorKind::UnsupportedDuration);
        }
        (TICKS_PER_WHOLE / value, digits)
    };
    let mut ticks = base;
    if s[used..].starts_with('.') {
        if base % 2 != 0 {
            return Err(ParseErrorKind::UnsupportedDuration);
        }
        ticks += base / 2;
        used += 1;
    }
    Ok((Some(Duration::from_ticks(ticks)), used))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lily(s: &str) -> Vec<Note> {
        parse_lilypond(s, Duration::QUARTER).unwrap()
    }

    fn names(notes: &[Note]) -> Vec<String> {
        notes
            .iter()
            .map(|n| match n.pitch {
                Some(p) => format!("{p}:{}{}", n.duration, if n.tie { "~" } else { "" }),
                None => format!("r:{}", n.duration),
            })
            .collect()
    }

    #[test]
    fn pitch_lists() {
        let pitches = parse_pitches("D4 F4 E4 | C#5 Bb3").unwrap();
        let s: Vec<String> = pitches.iter().map(ToString::to_string).collect();
        assert_eq!(s, ["D4", "F4", "E4", "C#5", "Bb3"]);
        let err = parse_pitches("D4 X4").unwrap_err();
        assert_eq!((err.kind, err.position), (ParseErrorKind::UnknownToken, 3));
    }

    #[test]
    fn notes_rests_ties() {
        assert_eq!(
            names(&lily("r2 a'2~ | a'4 d'8 e' f'2")),
            ["r:2", "A4:2~", "A4:4", "D4:8", "E4:8", "F4:2"]
        );
        assert_eq!(names(&lily("c'4 ~ c'4")), ["C4:4~", "C4:4"]);
        assert_eq!(names(&lily("bes,2. es''4")), ["Bb2:2.", "Eb5:4"]);
        assert_eq!(names(&lily("d")), ["D3:4"]);
        assert_eq!(
            names(&lily("cis'!1 cis'?\\breve")),
            ["C#4:1", "C#4:\\breve"]
        );
    }

    #[test]
    fn dutch_names() {
        let p = |s: &str| lily(s)[0].pitch.unwrap().to_string();
        assert_eq!(p("es'"), "Eb4");
        assert_eq!(p("ees'"), "Eb4");
        assert_eq!(p("as"), "Ab3");
        assert_eq!(p("aes"), "Ab3");
        assert_eq!(p("eses"), "Ebb3");
        assert_eq!(p("ases"), "Abb3");
        assert_eq!(p("fisis"), "F##3");
        assert_eq!(p("beses"), "Bbb3");
    }

    #[test]
    fn lilypond_round_trip() {
        for s in ["C4", "C#5", "Bb3", "Eb4", "Ab2", "E2", "F##4", "Abb3"] {
            let pitch: Pitch = s.parse().unwrap();
            let parsed = lily(&pitch.to_lilypond())[0].pitch.unwrap();
            assert_eq!(parsed, pitch, "{s}");
        }
    }

    #[test]
    fn errors() {
        let err = |s: &str| parse_lilypond(s, Duration::QUARTER).unwrap_err().kind;
        assert_eq!(err("c'16"), ParseErrorKind::UnsupportedDuration);
        assert_eq!(err("c'8."), ParseErrorKind::UnsupportedDuration);
        assert_eq!(err("x4"), ParseErrorKind::UnknownToken);
        assert_eq!(err("c'4x"), ParseErrorKind::UnknownToken);
        assert_eq!(err("~ c'4"), ParseErrorKind::DanglingTie);
        assert_eq!(err("r4~"), ParseErrorKind::DanglingTie);
    }

    #[test]
    fn format_detection() {
        let meter = Meter::ALLA_BREVE;
        let sci = parse_notes("D4 F4 | E4", Duration::WHOLE, meter).unwrap();
        assert_eq!(names(&sci), ["D4:1", "F4:1", "E4:1"]);
        let lily = parse_notes("d'1 f' | e'", Duration::WHOLE, meter).unwrap();
        assert_eq!(names(&lily), ["D4:1", "F4:1", "E4:1"]);
        assert!(parse_notes("d'2 | e'1", Duration::WHOLE, meter).is_err());
    }

    #[test]
    fn bar_checks() {
        let meter = Meter::ALLA_BREVE;
        assert!(parse_lilypond_measures("d'1 | f'2 e'2 | d'1", Duration::WHOLE, meter).is_ok());
        let err = parse_lilypond_measures("d'1 | f'2 | e'2", Duration::WHOLE, meter).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::BarCheck(4));
        assert_eq!(err.position, 10);
    }
}
