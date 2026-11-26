//! Roman numeral analyses in the RomanText format of When in Rome, as far as Tymoczko's seven
//! analyses of fugues use it: `m12 b2 V6/5 b2.66 b: i`, keys as `f#:`, half-diminished chords as
//! `/o`, major sevenths as `maj7`.
//!
//! An upper-case numeral with a seventh is read as a dominant seventh, `maj` makes the seventh
//! major, a lower-case numeral gives a minor seventh chord. In minor, `VI` and `VII` stand on the
//! natural degrees, `vi` and `vii` (minor or diminished) on the raised ones.

use anyhow::{Result, bail};

use crate::key::{Chord, Harmony, Key, Kind};
use crate::pitch::Lof;
use crate::score::Piece;
use crate::time::{Meter, Tick};

const NUMERALS: [(&str, usize); 7] = [
    ("VII", 6),
    ("III", 2),
    ("IV", 3),
    ("VI", 5),
    ("II", 1),
    ("V", 4),
    ("I", 0),
];

/// Degree, upper case and length of the numeral at the start of `text`.
fn numeral(text: &str) -> Option<(usize, bool, usize)> {
    NUMERALS.iter().find_map(|(name, degree)| {
        if text.starts_with(name) {
            Some((*degree, true, name.len()))
        } else if text.starts_with(&name.to_lowercase()) {
            Some((*degree, false, name.len()))
        } else {
            None
        }
    })
}

/// Leading `b` and `#` before a numeral: the alteration and the rest.
fn accidentals(text: &str) -> (i8, &str) {
    let mut alter = 0;
    let mut rest = text;
    loop {
        match rest.as_bytes().first() {
            Some(b'#') => alter += 1,
            Some(b'b') if rest.len() > 1 => alter -= 1,
            _ => return (alter, rest),
        }
        rest = &rest[1..];
    }
}

/// The root of a degree: in minor the sixth and seventh degrees are raised under a minor or
/// diminished chord.
fn root(key: Key, degree: usize, upper: bool, alter: i8) -> Lof {
    let base = if key.minor && !upper {
        key.raised(degree)
    } else {
        key.degree(degree)
    };
    base.up(7 * alter)
}

/// The chord of one figure such as `V6/5/V`, `vii/o7`, `IVmaj7`, `N6` or `Ger6/5` in `key`.
pub fn chord(figure: &str, key: Key) -> Option<Chord> {
    // The tonicized key comes last: `V7/V`, but not the slash of `6/5` or `/o`.
    if let Some(at) = figure.rfind('/') {
        let tail = &figure[at + 1..];
        let (alter, name) = accidentals(tail);
        if let Some((degree, upper, len)) = numeral(name)
            && len == name.len()
        {
            let tonic = root(key, degree, upper, alter);
            let target = Key {
                tonic,
                minor: !upper,
            };
            return chord(&figure[..at], target);
        }
    }
    let inversion = |digits: &str| -> Option<(bool, usize)> {
        Some(match digits {
            "" | "5" | "53" => (false, 0),
            "6" | "63" => (false, 1),
            "64" => (false, 2),
            "7" => (true, 0),
            "65" => (true, 1),
            "43" => (true, 2),
            "2" | "42" => (true, 3),
            _ => return None,
        })
    };
    let digits_of = |text: &str| -> String { text.chars().filter(char::is_ascii_digit).collect() };
    if let Some(rest) = figure.strip_prefix("Ger") {
        let raised_fourth = key.tonic.up(6);
        let (_, _) = inversion(&digits_of(rest))?;
        // The usual position has the flat sixth degree in the bass.
        return Some(Chord::new(raised_fourth, Kind::Ger).inverted(1));
    }
    if let Some(rest) = figure.strip_prefix('N') {
        let (_, inv) = inversion(&digits_of(rest))?;
        return Some(Chord::new(key.tonic.up(-5), Kind::Maj).inverted(inv));
    }
    if let Some(rest) = figure.strip_prefix("Cad") {
        let (_, _) = inversion(&digits_of(rest))?;
        let kind = if key.minor { Kind::Min } else { Kind::Maj };
        return Some(Chord::new(key.tonic, kind).inverted(2));
    }
    let (alter, rest) = accidentals(figure);
    let (degree, upper, len) = numeral(rest)?;
    let mut rest = &rest[len..];
    #[derive(PartialEq)]
    enum Triad {
        Plain,
        Dim,
        HalfDim,
        Aug,
    }
    let triad = if let Some(r) = rest.strip_prefix("/o") {
        rest = r;
        Triad::HalfDim
    } else if let Some(r) = rest.strip_prefix('o') {
        rest = r;
        Triad::Dim
    } else if let Some(r) = rest.strip_prefix('+') {
        rest = r;
        Triad::Aug
    } else {
        Triad::Plain
    };
    let major_seventh = rest.starts_with("maj");
    if major_seventh {
        rest = &rest[3..];
    }
    // Bracketed additions such as `[no5]` do not change the chord.
    let rest = rest.split('[').next().unwrap_or("");
    if !rest.chars().all(|c| c.is_ascii_digit() || c == '/') {
        return None;
    }
    let digits = digits_of(rest);
    let root = root(
        key,
        degree,
        upper && triad == Triad::Plain || triad == Triad::Aug,
        alter,
    );
    if digits == "9" {
        let kind = if key.minor { Kind::DomMin9 } else { Kind::Dom9 };
        return Some(Chord::new(root, kind).inverted(0));
    }
    let (seventh, inv) = inversion(&digits)?;
    let kind = match (&triad, seventh || major_seventh, upper) {
        (Triad::HalfDim, _, _) => Kind::HalfDim7,
        (Triad::Dim, false, _) => Kind::Dim,
        (Triad::Dim, true, _) => Kind::Dim7,
        (Triad::Aug, false, _) => Kind::Aug,
        (Triad::Aug, true, _) => Kind::AugMaj7,
        (Triad::Plain, false, true) => Kind::Maj,
        (Triad::Plain, false, false) => Kind::Min,
        (Triad::Plain, true, true) if major_seventh => Kind::Maj7,
        (Triad::Plain, true, true) => Kind::Dom7,
        (Triad::Plain, true, false) if major_seventh => Kind::MinMaj7,
        (Triad::Plain, true, false) => Kind::Min7,
    };
    // A half-diminished chord written with a triad figure is still a seventh chord.
    Some(Chord::new(root, kind).inverted(inv))
}

/// One label of an analysis.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub bar: i32,
    /// Offset in the bar in beats (`b2.66` is 1 + 2/3), as a fraction.
    pub beat: (i32, i32),
    pub key: Key,
    pub figure: String,
}

/// An analysis: its labels and what the header says about the meter.
#[derive(Clone, Debug, Default)]
pub struct Analysis {
    pub meter: Option<Meter>,
    /// „Slow 3/8“: the beat is the written value, not the dotted one.
    pub slow: bool,
    pub labels: Vec<Label>,
    /// Tokens that are neither a beat, a key nor a readable figure, with their bar.
    pub unread: Vec<(i32, String)>,
}

/// `2.66` as the fraction of beats after the first: (5, 3).
fn beat_fraction(text: &str) -> Option<(i32, i32)> {
    let value: f64 = text.parse().ok()?;
    if !(1.0..64.0).contains(&value) {
        return None;
    }
    let whole = value.floor();
    let part = value - whole;
    let (num, den) = [
        (0, 1),
        (1, 4),
        (1, 3),
        (1, 2),
        (2, 3),
        (3, 4),
        (1, 8),
        (3, 8),
        (5, 8),
        (7, 8),
    ]
    .into_iter()
    .min_by(|a, b| {
        let da = (part - a.0 as f64 / a.1 as f64).abs();
        let db = (part - b.0 as f64 / b.1 as f64).abs();
        da.total_cmp(&db)
    })?;
    Some(((whole as i32 - 1) * den + num, den))
}

pub fn parse(text: &str) -> Result<Analysis> {
    let mut out = Analysis::default();
    let mut key: Option<Key> = None;
    for line in text.lines() {
        let line = line.trim();
        let lower = line.to_lowercase();
        if let Some(rest) = lower.strip_prefix("time signature:") {
            let rest = rest.trim();
            out.slow |= rest.starts_with("slow");
            let meter = rest.trim_start_matches("slow").trim();
            if out.meter.is_none() {
                out.meter = Meter::parse(meter);
            }
            continue;
        }
        let Some(rest) = line.strip_prefix('m') else {
            continue;
        };
        let mut tokens = rest.split_whitespace();
        let Some(head) = tokens.next() else {
            continue;
        };
        // Alternative readings (`m10var1`) and copied ranges (`m5-6 = m1-2`) are not used.
        let Ok(bar) = head.parse::<i32>() else {
            continue;
        };
        let mut beat = (0, 1);
        for token in tokens {
            if let Some(b) = token.strip_prefix('b').and_then(beat_fraction) {
                beat = b;
            } else if let Some(name) = token.strip_suffix(':')
                && let Some(k) = Key::parse(name)
            {
                key = Some(k);
            } else if token == "||" || token == "|" {
                continue;
            } else {
                let Some(k) = key else {
                    bail!("bar {bar}: a figure before any key");
                };
                if chord(token, k).is_some() {
                    out.labels.push(Label {
                        bar,
                        beat,
                        key: k,
                        figure: token.to_string(),
                    });
                } else {
                    out.unread.push((bar, token.to_string()));
                }
            }
        }
    }
    Ok(out)
}

/// The analysis as the harmony of a piece: bars by their numbers, beats by the meter.
pub fn harmony(analysis: &Analysis, piece: &Piece) -> Harmony {
    let meter = analysis.meter.unwrap_or(piece.meter);
    let unit: Tick = if analysis.slow {
        crate::time::TPW / meter.den as Tick
    } else {
        meter.beat()
    };
    let mut harmony = Harmony::default();
    for label in &analysis.labels {
        let offset = (unit * label.beat.0 + label.beat.1 / 2) / label.beat.1;
        let tick = piece.tick_of(label.bar, offset);
        if let Some(chord) = chord(&label.figure, label.key) {
            harmony.push(tick.max(0), chord, label.key);
        }
    }
    harmony
}

/// The analysis on its own time line, without a score: bar 1 begins at tick 0, every bar has
/// the length of the meter in the header (4/4 if it names none).
pub fn timeline(analysis: &Analysis) -> Vec<(Tick, Chord, Key)> {
    let meter = analysis.meter.unwrap_or(Meter::COMMON);
    let unit: Tick = if analysis.slow {
        crate::time::TPW / meter.den as Tick
    } else {
        meter.beat()
    };
    let mut out: Vec<(Tick, Chord, Key)> = Vec::new();
    for label in &analysis.labels {
        let offset = (unit * label.beat.0 + label.beat.1 / 2) / label.beat.1;
        let tick = (label.bar - 1) * meter.bar() + offset;
        if let Some(chord) = chord(&label.figure, label.key) {
            // Bars are sometimes written twice (repeats, alternative endings): keep the order.
            if out.last().is_none_or(|l| l.0 < tick) {
                out.push((tick, chord, label.key));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(figure: &str, key: &str) -> String {
        let chord = chord(figure, Key::parse(key).unwrap()).unwrap();
        let tones: Vec<String> = chord.tones().map(|t| t.to_string()).collect();
        format!(
            "{} / {}",
            tones.join(" "),
            chord.bass.map_or("?".into(), |b| b.to_string())
        )
    }

    #[test]
    fn figures() {
        assert_eq!(names("V6/5", "A"), "E G# B D / G#");
        assert_eq!(names("viio6/V", "A"), "D# F# A / F#");
        assert_eq!(names("vii/o6/5/V", "A"), "D# F# A C# / F#");
        assert_eq!(names("IVmaj7", "A"), "D F# A C# / D");
        assert_eq!(names("ii2", "A"), "B D F# A / A");
        assert_eq!(names("V7/IV", "C"), "C E G Bb / C");
        assert_eq!(names("viio7", "c"), "B D F Ab / B");
        assert_eq!(names("VII", "c"), "Bb D F / Bb");
        assert_eq!(names("VI6", "c"), "Ab C Eb / C");
        assert_eq!(names("vi/o7", "c"), "A C Eb G / A");
        assert_eq!(names("III+6", "c"), "Eb G B / G");
        assert_eq!(names("iv64", "c"), "F Ab C / C");
        assert_eq!(names("IV7", "c"), "F A C Eb / F");
        assert_eq!(names("ii/o4/3", "c"), "D F Ab C / Ab");
        assert_eq!(names("N6", "c"), "Db F Ab / F");
        assert_eq!(names("Ger6/5", "c"), "F# Ab C Eb / Ab");
        assert_eq!(names("V9", "c"), "G B D F Ab / G");
        assert_eq!(names("V6/5/iv", "c"), "C E G Bb / E");
        assert_eq!(names("viio6/VII", "c"), "A C Eb / C");
        assert_eq!(names("v6", "c"), "G Bb D / Bb");
        assert!(chord("xyz", Key::parse("C").unwrap()).is_none());
    }

    #[test]
    fn lines() {
        let text = "Time Signature: Slow 3/8\n\nm0 b3 b: i\nm1 b3 V6\nNote: a note\nm7 V6/III b2 III b3 f#: V\nm8var1 I\nm9 b2.66 ? b3 i\n";
        let analysis = parse(text).unwrap();
        assert!(analysis.slow);
        assert_eq!(analysis.meter, Some(Meter { num: 3, den: 8 }));
        let got: Vec<(i32, (i32, i32), String, String)> = analysis
            .labels
            .iter()
            .map(|l| (l.bar, l.beat, l.key.to_string(), l.figure.clone()))
            .collect();
        assert_eq!(got[0], (0, (2, 1), "b".into(), "i".into()));
        assert_eq!(got[4], (7, (2, 1), "f#".into(), "V".into()));
        assert_eq!(got.len(), 6);
        assert_eq!(analysis.unread, [(9, "?".to_string())]);
        assert_eq!(beat_fraction("2.66"), Some((5, 3)));
        assert_eq!(beat_fraction("1.75"), Some((3, 4)));
    }
}
