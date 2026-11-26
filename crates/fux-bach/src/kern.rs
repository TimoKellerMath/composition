//! Humdrum `**kern` as in Huron's encoding of the 48 fugues: one spine per voice, the lowest
//! first, with sharps and flats in the signature, values down to thirty-seconds, triplets, ties,
//! divided spines (`*^`, `*v`), alternative readings (`*S/ossia`) and occasional chords.
//!
//! A voice stays monophonic: of a divided spine the first part goes on as the voice and the
//! second becomes an extra line; of a chord the first tone belongs to the voice. Alternative
//! readings are kept apart and do not sound.

use anyhow::{Context, Result, anyhow, bail};

use crate::key::Key;
use crate::pitch::Pitch;
use crate::score::{Piece, Voice};
use crate::time::{Meter, TPW, Tick};

/// What a file held, counted the way `scripts/wtc_stats.py` counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KernCounts {
    pub spines: usize,
    /// Note tokens that do not continue a tie, in every spine, part and chord.
    pub notes: usize,
    pub barlines: usize,
    /// Of `notes`: in second parts of divided spines (alternative readings included).
    pub divided: usize,
    /// Of `notes`: in alternative readings.
    pub ossia: usize,
    /// Of `notes`: further tones of chords.
    pub chord_tones: usize,
}

/// A piece with everything the file held.
#[derive(Clone, Debug)]
pub struct KernPiece {
    pub piece: Piece,
    pub counts: KernCounts,
    /// Every line of the file as its own voice, in the order of their first appearance: the
    /// spines from the bottom up, then each second part and alternative reading. This is how the
    /// Python scripts of the assessment see a file.
    pub lines: Vec<Voice>,
    /// Whether the barlines are evenly spaced.
    pub regular: bool,
    /// Tick of every data record with the tick of the barline before it.
    pub rows: Vec<(Tick, Tick)>,
}

struct Line {
    spine: Option<usize>,
    ossia: bool,
    voice: Voice,
    /// The further tones of chords, one line per place in the chord.
    chords: Vec<Voice>,
}

struct Column {
    line: usize,
    end: Tick,
}

/// Duration of a token in ticks: `8.` is a dotted eighth, `0` a breve.
fn duration(token: &str) -> Option<Tick> {
    let start = token.find(|c: char| c.is_ascii_digit())?;
    let digits: String = token[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let dots = token[start + digits.len()..]
        .chars()
        .take_while(|c| *c == '.')
        .count();
    let mut ticks = match digits.as_str() {
        "0" => 2 * TPW,
        "00" => 4 * TPW,
        number => {
            let value: Tick = number.parse().ok()?;
            if value == 0 || TPW % value != 0 {
                return None;
            }
            TPW / value
        }
    };
    let mut added = ticks;
    for _ in 0..dots {
        if added % 2 != 0 {
            return None;
        }
        added /= 2;
        ticks += added;
    }
    Some(ticks)
}

/// The pitch of a note token, `None` for a rest.
fn pitch(token: &str) -> Option<Pitch> {
    let start = token.find(|c: char| matches!(c, 'a'..='g' | 'A'..='G'))?;
    let letter = token[start..].chars().next()?;
    let count = token[start..].chars().take_while(|c| *c == letter).count() as i8;
    let step = "cdefgab".find(letter.to_ascii_lowercase())? as i8;
    let octave = if letter.is_ascii_lowercase() {
        3 + count
    } else {
        4 - count
    };
    let mut alter = 0;
    for c in token[start + count as usize..].chars() {
        match c {
            '#' => alter += 1,
            '-' => alter -= 1,
            _ => break,
        }
    }
    Some(Pitch::new(step, alter, octave))
}

fn is_note(token: &str) -> bool {
    !token.contains('r') && token.contains(|c: char| matches!(c, 'a'..='g' | 'A'..='G'))
}

fn continues_tie(token: &str) -> bool {
    token.contains(']') || token.contains('_')
}

pub fn read(name: &str, text: &str) -> Result<KernPiece> {
    let mut lines: Vec<Line> = Vec::new();
    let mut columns: Vec<Column> = Vec::new();
    let mut counts = KernCounts::default();
    let mut key = None;
    let mut meter = None;
    let mut barlines: Vec<(Tick, Option<i32>)> = Vec::new();
    let mut now: Tick = 0;
    let mut started = false;
    let mut rows: Vec<(Tick, Tick)> = Vec::new();
    let mut bar_start: Tick = 0;
    for (index, row) in text.lines().enumerate() {
        let number = index + 1;
        if row.is_empty() || row.starts_with('!') {
            continue;
        }
        // An empty field is a stray tabulator (I/24 has one in every record), not a spine.
        let fields: Vec<&str> = row.split('\t').filter(|f| !f.is_empty()).collect();
        if row.starts_with("**") {
            if started {
                bail!("{name}: line {number}: a second exclusive interpretation");
            }
            started = true;
            counts.spines = fields.iter().filter(|f| **f == "**kern").count();
            if counts.spines != fields.len() {
                bail!("{name}: line {number}: a spine other than **kern");
            }
            for spine in 0..fields.len() {
                lines.push(Line {
                    spine: Some(spine),
                    ossia: false,
                    voice: Voice::default(),
                    chords: Vec::new(),
                });
                columns.push(Column {
                    line: spine,
                    end: 0,
                });
            }
            continue;
        }
        if !started {
            continue;
        }
        if fields.len() != columns.len() {
            bail!(
                "{name}: line {number}: {} fields for {} columns",
                fields.len(),
                columns.len()
            );
        }
        if row.starts_with('*') {
            let mut next: Vec<Column> = Vec::with_capacity(columns.len() + 1);
            let mut joined = false;
            for (field, column) in fields.iter().zip(columns.drain(..)) {
                match *field {
                    "*^" | "*+" => {
                        let end = column.end;
                        next.push(column);
                        lines.push(Line {
                            spine: None,
                            ossia: false,
                            voice: Voice::default(),
                            chords: Vec::new(),
                        });
                        next.push(Column {
                            line: lines.len() - 1,
                            end,
                        });
                        joined = false;
                    }
                    "*v" => {
                        if !joined {
                            next.push(column);
                        }
                        joined = true;
                    }
                    other => {
                        joined = false;
                        if other == "*S/ossia" {
                            lines[column.line].ossia = true;
                        } else if let Some(m) = other.strip_prefix("*M")
                            && meter.is_none()
                            && let Some(m) = Meter::parse(m)
                        {
                            meter = Some(m);
                        } else if key.is_none()
                            && other.ends_with(':')
                            && let Some(k) = Key::parse(&other[1..])
                        {
                            key = Some(k);
                        }
                        next.push(column);
                    }
                }
            }
            columns = next;
            continue;
        }
        if row.starts_with('=') {
            counts.barlines += 1;
            bar_start = now;
            if !row.starts_with("==") {
                let digits: String = fields[0].chars().filter(char::is_ascii_digit).collect();
                barlines.push((now, digits.parse().ok()));
            }
            continue;
        }
        rows.push((now, bar_start));
        for (field, column) in fields.iter().zip(columns.iter_mut()) {
            if *field == "." {
                continue;
            }
            let line = &mut lines[column.line];
            for (part, token) in field.split(' ').filter(|t| !t.is_empty()).enumerate() {
                if token.contains('q') || token.contains('Q') {
                    continue;
                }
                let note = is_note(token);
                if note && !continues_tie(token) {
                    counts.notes += 1;
                    if part > 0 {
                        counts.chord_tones += 1;
                    } else if line.spine.is_none() {
                        counts.divided += 1;
                        if line.ossia {
                            counts.ossia += 1;
                        }
                    }
                }
                let ticks = duration(token)
                    .with_context(|| format!("{name}: line {number}: duration of {token:?}"))?;
                if part == 0 {
                    column.end = now + ticks;
                }
                if !note {
                    continue;
                }
                let p = pitch(token)
                    .ok_or_else(|| anyhow!("{name}: line {number}: pitch of {token:?}"))?;
                let target = if part == 0 {
                    &mut line.voice
                } else {
                    if line.chords.len() < part {
                        line.chords.resize_with(part, Voice::default);
                    }
                    &mut line.chords[part - 1]
                };
                match target.notes.last_mut() {
                    Some(last) if continues_tie(token) && last.pitch == p && last.off == now => {
                        last.off = now + ticks;
                    }
                    Some(last) if last.off > now => {
                        bail!("{name}: line {number}: {token:?} overlaps the note before it");
                    }
                    _ => target.push(now, now + ticks, p),
                }
            }
        }
        if let Some(next) = columns.iter().map(|c| c.end).filter(|e| *e > now).min() {
            now = next;
        }
    }
    if !started {
        bail!("{name}: no **kern spine");
    }
    let meter = meter.ok_or_else(|| anyhow!("{name}: no meter"))?;
    let key = key.ok_or_else(|| anyhow!("{name}: no key"))?;
    let end = lines.iter().map(|l| l.voice.end()).max().unwrap_or(0);
    let (origin, first_bar) = match barlines.first() {
        Some((tick, Some(number))) => (*tick, *number),
        Some((tick, None)) => (*tick, 1),
        None => (0, 1),
    };
    let regular = barlines.windows(2).all(|w| w[1].0 - w[0].0 == meter.bar());
    let mut voices: Vec<Voice> = Vec::new();
    let mut extras: Vec<Voice> = Vec::new();
    let all: Vec<Voice> = lines.iter().map(|l| l.voice.clone()).collect();
    for line in lines {
        if line.ossia {
            continue;
        }
        extras.extend(line.chords);
        match line.spine {
            Some(_) => voices.push(line.voice),
            None if !line.voice.notes.is_empty() => extras.push(line.voice),
            None => {}
        }
    }
    voices.reverse();
    let mut piece = Piece::new(name, key, meter, voices);
    piece.origin = origin;
    piece.first_bar = first_bar;
    piece.extras = extras;
    piece.end = end;
    Ok(KernPiece {
        piece,
        counts,
        lines: all,
        regular,
        rows,
    })
}

/// Reads every `.krn` file of a directory, in the order of the names.
pub fn read_dir(dir: &std::path::Path) -> Result<Vec<KernPiece>> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| dir.display().to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "krn"))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let text = std::fs::read(path).with_context(|| path.display().to_string())?;
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
            read(name, &String::from_utf8_lossy(&text))
        })
        .collect()
}

/// Writes the voices as plain `**kern` on a grid of eighths for `fux_core::kern::parse_kern`: no
/// ornaments, ties as `[`, `_`, `]`, the mode named after the key (`*C:ion`, `*a:aeo`). Fails if
/// a note does not lie on the grid.
pub fn write_plain(piece: &Piece) -> Result<String> {
    const EIGHTH: Tick = TPW / 8;
    // Eighths that one token can hold, with its kern value.
    const VALUES: [(Tick, &str); 8] = [
        (16, "0"),
        (12, "1."),
        (8, "1"),
        (6, "2."),
        (4, "2"),
        (3, "4."),
        (2, "4"),
        (1, "8"),
    ];
    let count = piece.voices.len();
    let end = piece.end;
    // Per voice the tokens by tick.
    let mut tokens: Vec<std::collections::BTreeMap<Tick, String>> = vec![Default::default(); count];
    for (v, voice) in piece.voices.iter().enumerate() {
        let mut at = 0;
        let mut put = |from: Tick, until: Tick, pitch: Option<Pitch>| -> Result<()> {
            if from % EIGHTH != 0 || until % EIGHTH != 0 {
                bail!(
                    "{}: a note off the grid of eighths at tick {from}",
                    piece.name
                );
            }
            let mut left = (until - from) / EIGHTH;
            let mut t = from;
            let mut first = true;
            while left > 0 {
                // Stay inside the bar so that every token is a value the reader accepts.
                let room = (piece.meter.bar() - piece.pos(t)) / EIGHTH;
                let (eighths, value) = VALUES
                    .into_iter()
                    .find(|(e, _)| *e <= left.min(room))
                    .unwrap_or((1, "8"));
                left -= eighths;
                let token = match pitch {
                    None => format!("{value}r"),
                    Some(p) => {
                        let open = if first && left > 0 { "[" } else { "" };
                        let close = match (first, left > 0) {
                            (true, _) => "",
                            (false, true) => "_",
                            (false, false) => "]",
                        };
                        format!("{open}{value}{}{close}", p.kern())
                    }
                };
                tokens[v].insert(t, token);
                t += eighths * EIGHTH;
                first = false;
            }
            Ok(())
        };
        for note in &voice.notes {
            if note.on > at {
                put(at, note.on, None)?;
            }
            put(note.on, note.off, Some(note.pitch))?;
            at = note.off;
        }
        if at < end {
            put(at, end, None)?;
        }
    }
    let flats = (-piece.key.signature()).max(0) as usize;
    if piece.key.signature() > 0 || flats > 1 {
        bail!(
            "{}: the signature of {} is outside the reader's range",
            piece.name,
            piece.key
        );
    }
    let signature = if flats == 1 { "*k[b-]" } else { "*k[]" };
    let mode = format!(
        "*{}:{}",
        piece.key,
        if piece.key.minor { "aeo" } else { "ion" }
    );
    let row = |cell: &dyn Fn(usize) -> String| -> String {
        // kern lists the voices from the bottom up.
        (0..count).rev().map(cell).collect::<Vec<_>>().join("\t")
    };
    let mut out = String::new();
    out.push_str(&format!("!!!OTL: {}\n", piece.name));
    out.push_str(&row(&|_| "**kern".into()));
    out.push('\n');
    out.push_str(&row(&|_| signature.into()));
    out.push('\n');
    out.push_str(&row(&|_| mode.clone()));
    out.push('\n');
    out.push_str(&row(&|_| format!("*M{}", piece.meter)));
    out.push('\n');
    let mut ticks: Vec<Tick> = tokens.iter().flat_map(|m| m.keys().copied()).collect();
    ticks.sort_unstable();
    ticks.dedup();
    for t in ticks {
        if piece.pos(t) == 0 && t >= piece.origin {
            let bar = piece.bar_of(t).0;
            out.push_str(&row(&|_| format!("={bar}")));
            out.push('\n');
        }
        out.push_str(&row(&|v| {
            tokens[v].get(&t).cloned().unwrap_or_else(|| ".".into())
        }));
        out.push('\n');
    }
    out.push_str(&row(&|_| "==".into()));
    out.push('\n');
    out.push_str(&row(&|_| "*-".into()));
    out.push('\n');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "!!!OTL: sample
**kern\t**kern
*k[b-e-a-]\t*k[b-e-a-]
*M4/4\t*M4/4
*c:\t*c:
8r\t8g
=1\t=1
4C\t16ccL
.\t16bnJ
.\t8cc 8ee-
[4E-\t8dd
.\t8ee-
*^\t*
2E-]\t4GG\t4.g#T
.\t4r\t.
.\t.\t8a-
*v\t*v\t*
=2\t=2
1C;\t1cc;
==\t==
*-\t*-
";

    #[test]
    fn reads_ties_divisions_and_chords() {
        let read = read("sample", SAMPLE).unwrap();
        let piece = &read.piece;
        assert_eq!(piece.key, Key::parse("c").unwrap());
        assert_eq!((piece.origin, piece.first_bar), (12, 1));
        assert_eq!(piece.voices.len(), 2);
        // The upper voice comes first.
        let upper: Vec<(Tick, Tick, String)> = piece.voices[0]
            .notes
            .iter()
            .map(|n| (n.on, n.off, n.pitch.to_string()))
            .collect();
        assert_eq!(upper[0], (0, 12, "G4".into()));
        assert_eq!(upper[2], (18, 24, "B4".into()));
        assert_eq!(upper[6], (60, 96, "G#4".into()));
        // The tied E flat of the lower voice is one note.
        let lower = &piece.voices[1].notes;
        assert_eq!((lower[1].on, lower[1].off), (36, 108));
        assert_eq!(read.counts.notes, 14);
        assert_eq!(read.counts.chord_tones, 1);
        assert_eq!(read.counts.divided, 1);
        assert_eq!(piece.extras.len(), 2);
        assert!(read.regular);
        assert_eq!(piece.end, 204);
    }

    #[cfg(feature = "tools")]
    #[test]
    fn plain_writing_is_read_by_the_checker() {
        let c = |step, octave| Pitch::new(step, 0, octave);
        let mut upper = Voice::default();
        upper.push(0, 96, c(0, 5));
        upper.push(96, 156, c(1, 5));
        upper.push(156, 192, c(0, 5));
        let mut lower = Voice::default();
        lower.push(48, 96, c(0, 4));
        lower.push(96, 192, c(0, 4));
        let piece = Piece::new(
            "plain",
            Key::parse("C").unwrap(),
            Meter { num: 2, den: 2 },
            vec![upper, lower],
        );
        let text = write_plain(&piece).unwrap();
        let parsed = fux_core::kern::parse_kern(&text).unwrap();
        assert_eq!(parsed.score.voices.len(), 2);
        assert_eq!(parsed.score.voices[0].total_ticks(), 16);
    }
}
