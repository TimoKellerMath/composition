//! Chords as a plain text, bar by bar, for the check of a piece whose harmony is known:
//!
//! ```text
//! key d
//! 1: Dm | 2: A7/C# Dm | 3: Gm . A7 Dm
//! 5: key F  C7 F
//! ```
//!
//! `N:` begins bar N (as notated), the chords after it divide the bar into equal parts, a dot
//! holds the chord before it for one more part, a bar without an entry keeps the last chord.
//! `key X` (or `Tonart X`) sets the key from there on, lower case for minor; a dot after it holds
//! the chord in the new key. Chords are a root
//! (`C`, `F#`, `Bb`; B is the tone below C), a kind (nothing major, `m`, `o` or `dim`, `+`, `7`,
//! `m7`, `maj7`, `ø7` or `m7b5`, `o7` or `dim7`, `9`, `b9`) and after a slash the bass. `|` and
//! `#` comments are skipped.

use anyhow::{Context, Result, bail};

use crate::key::{Chord, Harmony, Key, Kind};
use crate::pitch::Lof;
use crate::score::Piece;
use crate::time::Tick;

/// `C`, `F#m7`, `Bbmaj7/D`, `Eø7`.
pub fn chord(symbol: &str) -> Option<Chord> {
    let (body, bass) = match symbol.split_once('/') {
        Some((body, bass)) => (body, Some(bass)),
        None => (symbol, None),
    };
    let letter = body.chars().next()?;
    if !('A'..='G').contains(&letter) {
        return None;
    }
    let accidentals: String = body[1..]
        .chars()
        .take_while(|c| matches!(c, '#' | 'b'))
        .collect();
    // `b9` belongs to the kind, not to the root.
    let accidentals =
        if body[1 + accidentals.len()..].starts_with('9') && accidentals.ends_with('b') {
            accidentals[..accidentals.len() - 1].to_string()
        } else {
            accidentals
        };
    let root = Lof::parse(&body[..1 + accidentals.len()])?;
    let kind = match &body[1 + accidentals.len()..] {
        "" | "maj" | "M" => Kind::Maj,
        "m" | "min" => Kind::Min,
        "o" | "dim" | "°" => Kind::Dim,
        "+" | "aug" => Kind::Aug,
        "7" => Kind::Dom7,
        "m7" | "min7" => Kind::Min7,
        "maj7" | "M7" => Kind::Maj7,
        "ø7" | "ø" | "m7b5" => Kind::HalfDim7,
        "o7" | "dim7" | "°7" => Kind::Dim7,
        "mmaj7" | "mM7" => Kind::MinMaj7,
        "+maj7" => Kind::AugMaj7,
        "9" => Kind::Dom9,
        "b9" | "7b9" => Kind::DomMin9,
        "Ger" => Kind::Ger,
        _ => return None,
    };
    let mut chord = Chord::new(root, kind);
    if let Some(bass) = bass {
        chord.bass = Some(Lof::parse(bass)?);
    }
    Some(chord)
}

/// A line up to its comment: a `#` that begins a word (a sharp follows its letter).
fn without_comment(line: &str) -> &str {
    let mut before = ' ';
    for (i, c) in line.char_indices() {
        if c == '#' && before.is_whitespace() {
            return &line[..i];
        }
        before = c;
    }
    line
}

/// The harmony a text of chord lines gives for a piece.
pub fn parse(text: &str, piece: &Piece) -> Result<Harmony> {
    let mut harmony = Harmony::default();
    let mut key = piece.key;
    // The bar being filled: its number and its parts so far (a chord, or `None` for a dot).
    let mut bar: Option<i32> = None;
    let mut parts: Vec<Option<(Chord, Key)>> = Vec::new();
    let mut last: Option<(Chord, Key)> = None;
    let flush = |bar: Option<i32>, parts: &mut Vec<Option<(Chord, Key)>>, harmony: &mut Harmony| {
        if let Some(number) = bar
            && !parts.is_empty()
        {
            let from = piece.tick_of(number, 0);
            let length = piece.meter.bar();
            for (k, part) in parts.iter().enumerate() {
                if let Some((chord, key)) = part {
                    let at: Tick = from + (length * k as Tick) / parts.len() as Tick;
                    harmony.push(at, *chord, *key);
                }
            }
        }
        parts.clear();
    };
    for (number, line) in text.lines().enumerate() {
        let line = without_comment(line);
        let mut tokens = line.split_whitespace().peekable();
        while let Some(token) = tokens.next() {
            let context = || format!("line {}: {token:?}", number + 1);
            if token == "|" {
                continue;
            }
            if token.eq_ignore_ascii_case("key") || token.eq_ignore_ascii_case("tonart") {
                let name = tokens.next().with_context(context)?;
                key = Key::parse(name)
                    .with_context(|| format!("line {}: no key {name:?}", number + 1))?;
                continue;
            }
            if let Some(digits) = token.strip_suffix(':')
                && let Ok(n) = digits.parse::<i32>()
            {
                flush(bar, &mut parts, &mut harmony);
                bar = Some(n);
                continue;
            }
            if bar.is_none() {
                bail!("{}: a chord before the first bar number", context());
            }
            if token == "." {
                let Some((held, in_key)) = last else {
                    bail!("{}: a dot before the first chord", context());
                };
                // A chord held into a new key begins again there, in that key.
                if in_key == key {
                    parts.push(None);
                } else {
                    last = Some((held, key));
                    parts.push(last);
                }
                continue;
            }
            let chord = chord(token).with_context(|| format!("{}: no chord", context()))?;
            last = Some((chord, key));
            parts.push(last);
        }
    }
    flush(bar, &mut parts, &mut harmony);
    if harmony.spans.is_empty() {
        bail!("the text names no chord");
    }
    harmony.spans.sort_by_key(|s| s.on);
    Ok(harmony)
}

/// A chord as `chord` reads it.
fn symbol(chord: Chord) -> String {
    let kind = match chord.kind {
        Kind::Maj => "",
        Kind::Min => "m",
        Kind::Dim => "o",
        Kind::Aug => "+",
        Kind::Dom7 => "7",
        Kind::Min7 => "m7",
        Kind::Maj7 => "maj7",
        Kind::HalfDim7 => "ø7",
        Kind::Dim7 => "o7",
        Kind::MinMaj7 => "mmaj7",
        Kind::AugMaj7 => "+maj7",
        Kind::Dom9 => "9",
        Kind::DomMin9 => "7b9",
        Kind::Ger => "Ger",
    };
    // A bass the harmony names is written, the root too: without a slash none is named.
    match chord.bass {
        Some(bass) => format!("{}{kind}/{bass}", chord.root),
        None => format!("{}{kind}", chord.root),
    }
}

/// A harmony as chord lines that `parse` reads back: one line per bar, the bar divided into
/// parts of `step` ticks, a dot where a chord holds. Chords that begin off this grid move to the
/// part they begin in.
pub fn write(harmony: &Harmony, piece: &Piece, step: Tick) -> String {
    let bar = piece.meter.bar();
    let parts = (bar / step.max(1)).max(1);
    let mut out = String::new();
    let mut key: Option<Key> = None;
    let mut before: Option<Chord> = None;
    let mut from = piece.origin;
    while from < piece.end {
        let (number, _) = piece.bar_of(from);
        let mut tokens: Vec<String> = Vec::new();
        for k in 0..parts {
            let Some(span) = harmony.at(from + k * bar / parts) else {
                continue;
            };
            // A chord that holds into a new key is named again there.
            let new_key = key != Some(span.key);
            if new_key {
                tokens.push(format!("key {}", span.key));
                key = Some(span.key);
            }
            if before == Some(span.chord) && !new_key && !tokens.is_empty() {
                tokens.push(".".to_string());
            } else {
                tokens.push(symbol(span.chord));
            }
            before = Some(span.chord);
        }
        // A bar that only holds its chord needs no line; one that holds it throughout needs
        // no dots.
        let chords: Vec<&String> = tokens.iter().filter(|t| !t.starts_with("key")).collect();
        if chords.len() as Tick == parts && chords[1..].iter().all(|t| *t == ".") {
            let first = chords[0].clone();
            tokens.retain(|t| t.starts_with("key"));
            tokens.push(first);
        }
        if !tokens.is_empty() {
            out += &format!("{number}: {}\n", tokens.join(" "));
        }
        from += bar;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::Voice;
    use crate::time::Meter;

    #[test]
    fn chords_by_symbol() {
        assert_eq!(chord("C"), Some(Chord::new(Lof(0), Kind::Maj)));
        assert_eq!(chord("F#m7"), Some(Chord::new(Lof(6), Kind::Min7)));
        assert_eq!(chord("Bbmaj7"), Some(Chord::new(Lof(-2), Kind::Maj7)));
        assert_eq!(chord("Eø7"), Some(Chord::new(Lof(4), Kind::HalfDim7)));
        assert_eq!(chord("Gb9"), Some(Chord::new(Lof(1), Kind::DomMin9)));
        assert_eq!(chord("Gb"), Some(Chord::new(Lof(-6), Kind::Maj)));
        let inverted = chord("A7/C#").unwrap();
        assert_eq!((inverted.root, inverted.bass), (Lof(3), Some(Lof(7))));
        assert_eq!(chord("H"), None);
        assert_eq!(chord("Cxyz"), None);
    }

    #[test]
    fn lines_give_a_harmony() {
        let key = Key::parse("d").unwrap();
        let mut voice = Voice::default();
        voice.push(0, 96 * 5, crate::pitch::Pitch::new(1, 0, 4));
        let piece = Piece::new("lines", key, Meter::COMMON, vec![voice]);
        let text = "key d  # the key\n1: Dm | 2: A7/C# Dm\n3: Gm . A7 Dm\n5: key F C7 F";
        let harmony = parse(text, &piece).unwrap();
        let starts: Vec<Tick> = harmony.spans.iter().map(|s| s.on).collect();
        assert_eq!(starts, [0, 96, 144, 192, 240, 264, 384, 432]);
        assert_eq!(harmony.spans[1].chord.root, Lof(3));
        assert_eq!(harmony.spans[4].chord.kind, Kind::Dom7);
        assert!(harmony.spans[5].key.minor);
        assert_eq!(harmony.spans[7].key, Key::parse("F").unwrap());
        // What is written is read back the same, with the bass the text names (since phase B7
        // a harmony can name the bass of its chords).
        let text = write(&harmony, &piece, 24);
        assert!(text.starts_with("1: key d Dm\n2: A7/C# . Dm .\n"), "{text}");
        let again = parse(&text, &piece).unwrap();
        assert_eq!(again.spans, harmony.spans);
        // A root named as the bass is written too: without a slash no bass is named.
        let mut named = harmony.clone();
        named.spans[0].chord = named.spans[0].chord.inverted(0);
        let text = write(&named, &piece, 24);
        assert!(text.starts_with("1: key d Dm/D\n"), "{text}");
        assert_eq!(parse(&text, &piece).unwrap().spans, named.spans);
        assert!(parse("Dm", &piece).is_err());
        assert!(parse("1: Xm", &piece).is_err());
    }

    /// A chord that holds while the key changes (a pivot chord) begins again in the new key, at
    /// the barline and inside a bar, written and read.
    #[test]
    fn a_chord_held_into_a_new_key() {
        let (d, a, c) = (
            Key::parse("d").unwrap(),
            Key::parse("a").unwrap(),
            Key::parse("C").unwrap(),
        );
        let mut voice = Voice::default();
        voice.push(0, 96 * 3, crate::pitch::Pitch::new(1, 0, 4));
        let piece = Piece::new("pivot", d, Meter::COMMON, vec![voice]);
        let (dm, e7, am) = (
            chord("Dm").unwrap(),
            chord("E7").unwrap(),
            chord("Am").unwrap(),
        );
        let mut harmony = Harmony::default();
        harmony.push(0, dm, d);
        harmony.push(96, dm, a);
        harmony.push(144, e7, a);
        harmony.push(192, am, a);
        harmony.push(240, am, c);
        let text = write(&harmony, &piece, 48);
        assert_eq!(text, "1: key d Dm\n2: key a Dm E7\n3: Am key C Am\n");
        let spans = |h: &Harmony| -> Vec<(Tick, Chord, Key)> {
            h.spans.iter().map(|s| (s.on, s.chord, s.key)).collect()
        };
        assert_eq!(spans(&parse(&text, &piece).unwrap()), spans(&harmony));
        // A dot after a key holds the chord in that key.
        let dotted = "1: key d Dm\n2: key a . E7\n3: Am key C .";
        assert_eq!(spans(&parse(dotted, &piece).unwrap()), spans(&harmony));
    }
}
