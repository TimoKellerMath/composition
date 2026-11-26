//! Humdrum `**kern` as in the encodings of Palestrina's masses
//! (`references/datasets/palestrina-masses`): one spine per voice, the lowest first, with clef,
//! key signature, mode (`*G:mix`) and meter; notes like `[2.b-` (tied dotted half B♭4) or `0r`
//! (breve rest). The voices become free counterpoint (no cantus firmus).
//!
//! Measures of 4/2 (a breve) are read as two measures of 2/2, the unit of the species; a note
//! across the middle of a breve measure becomes two tied notes. Other meters keep their measures.
//! Tuplets, spine splits and chords are not read.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::clef::VoiceType;
use crate::mode::{Mode, ModeSpec};
use crate::pitch::{Pitch, Step};
use crate::score::{Note, Role, Score, Species, Voice};
use crate::time::{Duration, Meter, TICKS_PER_WHOLE};

/// A piece read from `**kern`, with what its reference records say about it.
///
/// As Humdrum defines the records, `!!!OTL` titles what the file holds and `!!!OMD` names the
/// movement. A file that holds a whole movement has only `!!!OTL` („Kyrie“); a file that holds
/// one section of a divided movement has both: `!!!OMD: Credo` with `!!!OTL: Crucifixus`.
#[derive(Clone, Debug)]
pub struct KernPiece {
    /// The work (`!!!OPR`), e.g. „Missa Ad fugam“.
    pub work: Option<String>,
    /// The movement: `!!!OMD`, or `!!!OTL` if the file has no `!!!OMD`; e.g. „Kyrie“, „Credo“.
    pub movement: Option<String>,
    /// The section of a divided movement: `!!!OTL` beside `!!!OMD`, e.g. „Christe“, „Crucifixus“
    /// or „First Section“; `None` for a whole movement.
    pub section: Option<String>,
    /// The meter as written; a score from 4/2 counts in 2/2.
    pub written_meter: Meter,
    /// Whether the encoding names the mode (otherwise it comes from the final of the bass).
    pub mode_given: bool,
    pub score: Score,
}

/// Why a `**kern` text cannot be read.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum KernError {
    #[error("no **kern spine")]
    NoKern,
    #[error("line {line}: {what} is not supported")]
    Unsupported { line: usize, what: String },
    #[error("line {line}: cannot read {token:?}")]
    Token { line: usize, token: String },
    #[error("the voices are not equally long: {0}")]
    Lengths(String),
}

/// A note or rest as written in one spine, before the measures are laid out.
struct Written {
    pitch: Option<Pitch>,
    ticks: u32,
    /// Tied to the next note.
    tie: bool,
}

/// What the interpretations say about one spine.
#[derive(Default)]
struct SpineInfo {
    name: Option<String>,
    instrument: Option<String>,
    mode: Option<Mode>,
    flats: Option<usize>,
    meter: Option<Meter>,
}

/// Reads a piece.
pub fn parse_kern(text: &str) -> Result<KernPiece, KernError> {
    let mut records: BTreeMap<String, String> = BTreeMap::new();
    let mut spines: Option<usize> = None;
    let mut info: Vec<SpineInfo> = Vec::new();
    let mut notes: Vec<Vec<Written>> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if let Some(record) = line.strip_prefix("!!!") {
            if let Some((key, value)) = record.split_once(':') {
                records
                    .entry(key.trim().to_string())
                    .or_insert_with(|| value.trim().to_string());
            }
            continue;
        }
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if line.starts_with("**") {
            if fields.iter().any(|f| *f != "**kern") {
                return Err(KernError::Unsupported {
                    line: number,
                    what: "a spine other than **kern".into(),
                });
            }
            spines = Some(fields.len());
            info = (0..fields.len()).map(|_| SpineInfo::default()).collect();
            notes = (0..fields.len()).map(|_| Vec::new()).collect();
            continue;
        }
        let Some(count) = spines else {
            continue;
        };
        if fields.len() != count {
            return Err(KernError::Unsupported {
                line: number,
                what: format!("{} fields for {count} spines", fields.len()),
            });
        }
        if line.starts_with('!') {
            // Local comments: the first one per spine names the voice, e.g. „!Bassus“.
            for (spine, field) in fields.iter().enumerate() {
                let name = field.trim_start_matches('!').trim();
                if !name.is_empty() && info[spine].name.is_none() {
                    info[spine].name = Some(name.to_string());
                }
            }
            continue;
        }
        if line.starts_with('*') {
            for (spine, field) in fields.iter().enumerate() {
                interpret(field, &mut info[spine], number)?;
            }
            continue;
        }
        if line.starts_with('=') {
            continue;
        }
        for (spine, field) in fields.iter().enumerate() {
            if *field == "." {
                continue;
            }
            notes[spine].push(written(field, number)?);
        }
    }
    let count = spines.ok_or(KernError::NoKern)?;
    let written_meter = info
        .iter()
        .find_map(|i| i.meter)
        .unwrap_or(Meter::ALLA_BREVE);
    let meter = if written_meter
        == (Meter {
            numerator: 4,
            denominator: 2,
        }) {
        Meter::ALLA_BREVE
    } else {
        written_meter
    };
    let per_measure = meter.ticks_per_measure();
    // The voices from the top down; kern lists them from the bottom up.
    let mut voices: Vec<Voice> = Vec::new();
    for spine in (0..count).rev() {
        let laid = lay_out(&notes[spine], per_measure);
        let pitches: Vec<Pitch> = laid.iter().filter_map(|n| n.pitch).collect();
        let mut voice = Voice::new(
            VoiceType::best_for(&pitches),
            Role::Counterpoint(Species::Free),
            laid,
        );
        if let Some(name) = &info[spine].name {
            voice.name = name.clone();
        }
        voices.push(voice);
    }
    let lengths: Vec<u32> = voices.iter().map(Voice::total_ticks).collect();
    if lengths.windows(2).any(|w| w[0] != w[1]) {
        let list: Vec<String> = lengths.iter().map(ToString::to_string).collect();
        return Err(KernError::Lengths(list.join(", ")));
    }
    let flats = info.iter().find_map(|i| i.flats).unwrap_or(0);
    let named = info.iter().find_map(|i| i.mode);
    let mode = match named {
        Some(mode) => ModeSpec::new(mode),
        None => mode_of_final(&voices, flats),
    }
    .with_transposition(flats == 1);
    let mut score = Score::new(mode, voices);
    score.meter = meter;
    let (movement, section) = match (records.get("OMD"), records.get("OTL")) {
        (Some(movement), section) => (Some(movement.clone()), section.cloned()),
        (None, movement) => (movement.cloned(), None),
    };
    let work = records.get("OPR").cloned();
    // „Missa Ad fugam – Credo – Crucifixus“.
    let parts: Vec<&str> = [&work, &movement, &section]
        .into_iter()
        .filter_map(|part| part.as_deref())
        .collect();
    score.title = (!parts.is_empty()).then(|| parts.join(" – "));
    for voice in &mut score.voices {
        for note in &mut voice.notes {
            if let Some(pitch) = note.pitch {
                note.ficta = !mode.is_scale_pitch(pitch);
            }
        }
    }
    Ok(KernPiece {
        work,
        movement,
        section,
        written_meter,
        mode_given: named.is_some(),
        score,
    })
}

/// Takes an interpretation of one spine into account.
fn interpret(field: &str, info: &mut SpineInfo, line: usize) -> Result<(), KernError> {
    match field {
        "*" | "*-" => {}
        "*^" | "*v" | "*x" | "*+" => {
            return Err(KernError::Unsupported {
                line,
                what: format!("the spine manipulator {field}"),
            });
        }
        _ => {
            if let Some(instrument) = field.strip_prefix("*I") {
                info.instrument = Some(instrument.to_string());
            } else if let Some(signature) = field.strip_prefix("*k[") {
                let signature = signature.trim_end_matches(']');
                if signature.contains('#') {
                    return Err(KernError::Unsupported {
                        line,
                        what: format!("the key signature {field}"),
                    });
                }
                info.flats = Some(signature.matches('-').count());
            } else if let Some(meter) = field.strip_prefix("*M") {
                if let Some((n, d)) = meter.split_once('/')
                    && let (Ok(numerator), Ok(denominator)) = (n.parse(), d.parse())
                {
                    info.meter = Some(Meter {
                        numerator,
                        denominator,
                    });
                }
            } else if let Some((_, mode)) = field[1..].split_once(':') {
                info.mode = match mode {
                    "dor" => Some(Mode::Dorian),
                    "phr" => Some(Mode::Phrygian),
                    "lyd" => Some(Mode::Lydian),
                    "mix" => Some(Mode::Mixolydian),
                    "aeo" => Some(Mode::Aeolian),
                    "ion" => Some(Mode::Ionian),
                    _ => info.mode,
                };
            }
        }
    }
    Ok(())
}

/// Reads one note or rest, e.g. `[2.b-`, `4cc#`, `1E_` or `0r`.
fn written(token: &str, line: usize) -> Result<Written, KernError> {
    let bad = || KernError::Token {
        line,
        token: token.to_string(),
    };
    if token.contains(' ') {
        return Err(KernError::Unsupported {
            line,
            what: "a chord".into(),
        });
    }
    let mut rest = token;
    let tie_start = rest.starts_with('[');
    rest = rest.trim_start_matches(['[', '(', '{']);
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    rest = &rest[digits.len()..];
    let dots = rest.chars().take_while(|c| *c == '.').count();
    rest = &rest[dots..];
    // `0` is a breve, `00` a long; other values divide the whole note.
    let mut ticks = match digits.as_str() {
        "0" => 2 * TICKS_PER_WHOLE,
        "00" => 4 * TICKS_PER_WHOLE,
        "" => return Err(bad()),
        number => {
            let value: u32 = number.parse().map_err(|_| bad())?;
            if value == 0 || !TICKS_PER_WHOLE.is_multiple_of(value) {
                return Err(KernError::Unsupported {
                    line,
                    what: format!("the duration {number} (a tuplet or shorter than an eighth)"),
                });
            }
            TICKS_PER_WHOLE / value
        }
    };
    let mut added = ticks;
    for _ in 0..dots {
        if !added.is_multiple_of(2) {
            return Err(bad());
        }
        added /= 2;
        ticks += added;
    }
    let letter = rest.chars().next().ok_or_else(bad)?;
    let pitch = if letter == 'r' {
        rest = rest.trim_start_matches('r');
        None
    } else {
        let step = Step::from_letter(letter.to_ascii_uppercase()).ok_or_else(bad)?;
        let repeated = rest.chars().take_while(|c| *c == letter).count();
        rest = &rest[repeated..];
        let octave = if letter.is_ascii_lowercase() {
            3 + repeated as i8
        } else {
            4 - repeated as i8
        };
        let mut alter = 0i8;
        while let Some(c) = rest.chars().next() {
            match c {
                '#' => alter += 1,
                '-' => alter -= 1,
                'n' => {}
                _ => break,
            }
            rest = &rest[1..];
        }
        Some(Pitch::new(step, alter, octave))
    };
    // The tie into the next note: `[` starts one, `_` goes on, `]` ends it.
    let mut tie = tie_start;
    for c in rest.chars() {
        match c {
            '_' => tie = true,
            ']' => tie = false,
            ';' | ')' | '}' | 'L' | 'J' | '\'' | 'y' | 'X' => {}
            _ => return Err(bad()),
        }
    }
    Ok(Written {
        pitch,
        ticks,
        tie: tie && pitch.is_some(),
    })
}

/// The notes of a spine in measures of `per_measure` ticks: a note across a barline becomes tied
/// notes, a rest across one several rests; rests that follow each other within a measure become
/// one.
fn lay_out(written: &[Written], per_measure: u32) -> Vec<Note> {
    let mut notes: Vec<Note> = Vec::new();
    let mut at = 0;
    for w in written {
        let mut left = w.ticks;
        while left > 0 {
            let room = per_measure - at % per_measure;
            let ticks = left.min(room);
            left -= ticks;
            let in_measure = !at.is_multiple_of(per_measure);
            at += ticks;
            match w.pitch {
                Some(pitch) => {
                    let mut note = Note::new(pitch, Duration::from_ticks(ticks));
                    note.tie = left > 0 || w.tie;
                    notes.push(note);
                }
                None => match notes.last_mut() {
                    Some(last) if last.is_rest() && in_measure => {
                        last.duration = Duration::from_ticks(last.duration.ticks() + ticks);
                    }
                    _ => notes.push(Note::rest(Duration::from_ticks(ticks))),
                },
            }
        }
    }
    notes
}

/// The mode from the final of the lowest voice, for an encoding that does not name it.
fn mode_of_final(voices: &[Voice], flats: usize) -> ModeSpec {
    let last = voices
        .last()
        .and_then(|v| v.notes.iter().rev().find_map(|n| n.pitch));
    let Some(last) = last else {
        return ModeSpec::new(Mode::Dorian);
    };
    // With one flat the finals lie a fourth higher (G for the Dorian mode).
    let step = if flats == 1 {
        last.transpose(-crate::interval::Interval::PERFECT_FOURTH)
            .step
    } else {
        last.step
    };
    Mode::from_final(step).map_or(ModeSpec::new(Mode::Dorian), ModeSpec::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KYRIE: &str = "!!!OPR: Missa Exemplum
!!!OTL: Kyrie
**kern\t**kern
*Ibass\t*Icant
!Bassus\t!Cantus
*clefF4\t*clefG2
*k[]\t*k[]
*G:mix\t*G:mix
*M4/2\t*M4/2
=1\t=1
0r\t1d
.\t[1g
=2\t=2
1G\t2g]
.\t2a
2.c\t1b
4B\t.
=3\t=3
0G\t0g
==\t==
*-\t*-
";

    #[test]
    fn a_small_piece() {
        let piece = parse_kern(KYRIE).unwrap();
        assert_eq!(piece.work.as_deref(), Some("Missa Exemplum"));
        assert_eq!(piece.movement.as_deref(), Some("Kyrie"));
        assert_eq!(piece.section, None);
        assert_eq!(
            piece.written_meter,
            Meter {
                numerator: 4,
                denominator: 2
            }
        );
        let score = &piece.score;
        score.validate().unwrap();
        assert_eq!(score.meter, Meter::ALLA_BREVE);
        assert_eq!(score.mode, ModeSpec::new(Mode::Mixolydian));
        assert_eq!(score.title.as_deref(), Some("Missa Exemplum – Kyrie"));
        // The cantus on top; the tied G5 of measure 1 goes on into measure 2.
        let cantus = &score.voices[0];
        assert_eq!(cantus.name, "Cantus");
        assert_eq!(cantus.role, Role::Counterpoint(Species::Free));
        let text: Vec<String> = cantus.notes.iter().map(note_text).collect();
        assert_eq!(
            text,
            ["D4:8", "G4:8~", "G4:4", "A4:4", "B4:8", "G4:8~", "G4:8"]
        );
        // The breve rest becomes two whole rests; the breve G3 two tied wholes.
        let bass: Vec<String> = score.voices[1].notes.iter().map(note_text).collect();
        assert_eq!(
            bass,
            ["r:8", "r:8", "G3:8", "C4:6", "B3:2", "G3:8~", "G3:8"]
        );
    }

    /// A file that holds a section of a divided movement names the movement in `!!!OMD` and the
    /// section in `!!!OTL`; a file that holds a whole movement names it in `!!!OTL` alone.
    #[test]
    fn movements_and_sections() {
        let read =
            |records: &str| parse_kern(&format!("{records}**kern\n*M4/2\n0G\n*-\n")).unwrap();
        let piece = read("!!!OPR: Missa Exemplum\n!!!OMD: Credo\n!!!OTL: Crucifixus\n");
        assert_eq!(piece.movement.as_deref(), Some("Credo"));
        assert_eq!(piece.section.as_deref(), Some("Crucifixus"));
        assert_eq!(
            piece.score.title.as_deref(),
            Some("Missa Exemplum – Credo – Crucifixus")
        );
        let piece = read("!!!OTL: Agnus I\n");
        assert_eq!(piece.movement.as_deref(), Some("Agnus I"));
        assert_eq!(piece.section, None);
        assert_eq!(piece.score.title.as_deref(), Some("Agnus I"));
        let piece = read("!!!OMD: Benedictus\n");
        assert_eq!(piece.movement.as_deref(), Some("Benedictus"));
        assert_eq!(piece.section, None);
        let piece = read("");
        assert_eq!(
            (piece.work, piece.movement, piece.section, piece.score.title),
            (None, None, None, None)
        );
    }

    fn note_text(note: &Note) -> String {
        let pitch = note.pitch.map_or("r".to_string(), |p| p.to_string());
        let tie = if note.tie { "~" } else { "" };
        format!("{pitch}:{}{tie}", note.duration.ticks())
    }

    #[test]
    fn tokens() {
        let w = written("[2.b-", 1).unwrap();
        assert_eq!(
            (w.pitch, w.ticks, w.tie),
            (Some("Bb4".parse().unwrap()), 6, true)
        );
        let w = written("4cc#", 1).unwrap();
        assert_eq!(w.pitch, Some("C#5".parse().unwrap()));
        let w = written("1EE_", 1).unwrap();
        assert_eq!((w.pitch, w.tie), (Some("E2".parse().unwrap()), true));
        assert_eq!(written("00r", 1).unwrap().ticks, 32);
        assert!(matches!(
            written("3c", 1),
            Err(KernError::Unsupported { .. })
        ));
        assert!(matches!(written("4x", 1), Err(KernError::Token { .. })));
    }

    #[test]
    fn errors() {
        assert_eq!(parse_kern("!!!OTL: x\n").unwrap_err(), KernError::NoKern);
        let split = "**kern\n*^\n";
        assert!(matches!(
            parse_kern(split),
            Err(KernError::Unsupported { line: 2, .. })
        ));
        let uneven = "**kern\t**kern\n1c\t0c\n*-\t*-\n";
        assert!(matches!(parse_kern(uneven), Err(KernError::Lengths(_))));
    }

    #[test]
    fn rests_within_a_measure_become_one() {
        let text = "**kern\n*M4/2\n2r\n2r\n4r\n4r\n2c\n*-\n";
        let piece = parse_kern(text).unwrap();
        let notes: Vec<String> = piece.score.voices[0].notes.iter().map(note_text).collect();
        assert_eq!(notes, ["r:8", "r:4", "C4:4"]);
    }

    #[test]
    fn the_mode_from_the_final_with_a_flat() {
        let text = "**kern\n*k[b-]\n*M4/2\n0G\n*-\n";
        let piece = parse_kern(text).unwrap();
        assert!(!piece.mode_given);
        assert_eq!(
            piece.score.mode,
            ModeSpec::new(Mode::Dorian).with_transposition(true)
        );
    }
}
