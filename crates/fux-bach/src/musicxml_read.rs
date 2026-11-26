//! Reads a MusicXML file (score-partwise) as a piece: every voice of every part becomes a voice.
//! What the check needs is read (pitches, durations, ties, key, meter, upbeat); grace notes,
//! lyrics and everything about the looks are skipped.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};

use crate::key::Key;
use crate::pitch::{Lof, Pitch};
use crate::score::{Note, Piece, Voice};
use crate::time::{Meter, QUARTER, Tick};

/// What was read beside the piece.
#[derive(Clone, Debug, Default)]
pub struct Remarks {
    /// Tones of chords inside one voice that were left out (the highest tone stays).
    pub chord_tones: usize,
    pub grace_notes: usize,
}

fn child<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<roxmltree::Node<'a, 'a>> {
    node.children().find(|c| c.has_tag_name(name))
}

fn number<T: std::str::FromStr>(node: roxmltree::Node, name: &str) -> Option<T> {
    child(node, name)?.text()?.trim().parse().ok()
}

pub fn read(name: &str, text: &str) -> Result<(Piece, Remarks)> {
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..roxmltree::ParsingOptions::default()
    };
    let document =
        roxmltree::Document::parse_with_options(text, options).context("the file is not XML")?;
    let root = document.root_element();
    if !root.has_tag_name("score-partwise") {
        bail!(
            "only score-partwise MusicXML is read, this is <{}>",
            root.tag_name().name()
        );
    }
    let mut remarks = Remarks::default();
    let mut lines: BTreeMap<(usize, String), Vec<Note>> = BTreeMap::new();
    let mut key: Option<Key> = None;
    let mut meter: Option<Meter> = None;
    let mut origin: Tick = 0;
    let mut end: Tick = 0;
    for (p, part) in root
        .children()
        .filter(|c| c.has_tag_name("part"))
        .enumerate()
    {
        let mut divisions: i64 = 1;
        // Start of the measure and the cursor, in ticks times `divisions` (so nothing rounds).
        let mut start: i64 = 0;
        let ticks = |value: i64, divisions: i64| -> Result<Tick> {
            let scaled = value * QUARTER as i64;
            if scaled % divisions != 0 {
                bail!("a duration finer than the grid of 96 ticks to the whole note");
            }
            Ok((scaled / divisions) as Tick)
        };
        // Per voice: the tone a tie holds open.
        let mut open: BTreeMap<String, Pitch> = BTreeMap::new();
        for (m, measure) in part
            .children()
            .filter(|c| c.has_tag_name("measure"))
            .enumerate()
        {
            let mut cursor = start;
            let mut furthest = start;
            let mut last_onset = start;
            for item in measure.children().filter(roxmltree::Node::is_element) {
                match item.tag_name().name() {
                    "attributes" => {
                        if let Some(d) = number::<i64>(item, "divisions") {
                            // The cursor is in old divisions: rescale.
                            start = start * d / divisions;
                            cursor = cursor * d / divisions;
                            furthest = furthest * d / divisions;
                            last_onset = last_onset * d / divisions;
                            divisions = d.max(1);
                        }
                        if let Some(k) = child(item, "key")
                            && key.is_none()
                        {
                            let fifths: i8 = number(k, "fifths").unwrap_or(0);
                            let minor = child(k, "mode")
                                .and_then(|n| n.text())
                                .is_some_and(|t| t.trim() == "minor");
                            key = Some(if minor {
                                Key::minor(Lof(fifths + 3))
                            } else {
                                Key::major(Lof(fifths))
                            });
                        }
                        if let Some(t) = child(item, "time")
                            && meter.is_none()
                        {
                            meter = Some(Meter {
                                num: number(t, "beats").context("time without beats")?,
                                den: number(t, "beat-type").context("time without beat-type")?,
                            });
                        }
                    }
                    "backup" => cursor -= number::<i64>(item, "duration").unwrap_or(0),
                    "forward" => {
                        cursor += number::<i64>(item, "duration").unwrap_or(0);
                        furthest = furthest.max(cursor);
                    }
                    "note" => {
                        if child(item, "grace").is_some() {
                            remarks.grace_notes += 1;
                            continue;
                        }
                        let duration = number::<i64>(item, "duration").unwrap_or(0);
                        let in_chord = child(item, "chord").is_some();
                        let on = if in_chord { last_onset } else { cursor };
                        if !in_chord {
                            last_onset = cursor;
                            cursor += duration;
                            furthest = furthest.max(cursor);
                        }
                        let voice = child(item, "voice")
                            .and_then(|v| v.text())
                            .map_or("1".to_string(), |t| t.trim().to_string());
                        let Some(pitch) = child(item, "pitch") else {
                            open.remove(&voice);
                            continue;
                        };
                        let step = child(pitch, "step")
                            .and_then(|s| s.text())
                            .and_then(|s| "CDEFGAB".find(s.trim()))
                            .context("a pitch without step")?;
                        // Alterations may be written as decimals.
                        let alter = child(pitch, "alter")
                            .and_then(|a| a.text())
                            .and_then(|a| a.trim().parse::<f64>().ok())
                            .unwrap_or(0.0)
                            .round() as i8;
                        let octave: i8 =
                            number(pitch, "octave").context("a pitch without octave")?;
                        let pitch = Pitch::new(step as i8, alter, octave);
                        let (on, off) = (ticks(on, divisions)?, ticks(on + duration, divisions)?);
                        let ties: Vec<&str> = item
                            .children()
                            .filter(|c| c.has_tag_name("tie"))
                            .filter_map(|c| c.attribute("type"))
                            .collect();
                        let line = lines.entry((p, voice.clone())).or_default();
                        let continued = ties.contains(&"stop")
                            && open.get(&voice) == Some(&pitch)
                            && line.last().is_some_and(|n| n.off == on && n.pitch == pitch);
                        if continued {
                            if let Some(last) = line.last_mut() {
                                last.off = off;
                            }
                        } else if off > on {
                            line.push(Note { on, off, pitch });
                        }
                        if ties.contains(&"start") {
                            open.insert(voice, pitch);
                        } else {
                            open.remove(&voice);
                        }
                    }
                    _ => {}
                }
            }
            // An upbeat: a first measure shorter than the bar.
            if p == 0 && m == 0 {
                let length = ticks(furthest - start, divisions)?;
                if let Some(meter) = meter
                    && length < meter.bar()
                {
                    origin = length;
                }
            }
            start = furthest;
            end = end.max(ticks(furthest, divisions)?);
        }
    }
    let meter = meter.context("the file names no meter")?;
    if meter.den == 0 || crate::time::TPW % meter.den as Tick != 0 {
        bail!("meter {}/{} is not read", meter.num, meter.den);
    }
    let mut voices: Vec<Voice> = Vec::new();
    for (_, mut notes) in lines {
        notes.sort_by_key(|n| (n.on, std::cmp::Reverse(n.pitch)));
        // One tone at a time: of a chord the highest stays, a tone that reaches into the next is
        // cut.
        let mut line: Vec<Note> = Vec::new();
        for note in notes {
            match line.last_mut() {
                Some(last) if last.on == note.on => remarks.chord_tones += 1,
                Some(last) if last.off > note.on => {
                    last.off = note.on;
                    line.push(note);
                }
                _ => line.push(note),
            }
        }
        if !line.is_empty() {
            voices.push(Voice { notes: line });
        }
    }
    if voices.is_empty() {
        bail!("the file has no notes");
    }
    // From the top down by the mean pitch.
    let mean = |voice: &Voice| -> i64 {
        voice.notes.iter().map(|n| n.pitch.midi as i64).sum::<i64>() * 100
            / voice.notes.len() as i64
    };
    voices.sort_by_key(|v| std::cmp::Reverse(mean(v)));
    let mut piece = Piece::new(name, key.unwrap_or(Key::major(Lof(0))), meter, voices);
    piece.origin = origin;
    piece.end = piece.end.max(end);
    Ok((piece, remarks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::musicxml::musicxml;

    #[test]
    fn what_is_written_is_read() {
        // Two voices with a tie over the barline and an upbeat-free start.
        let key = Key::parse("g").unwrap();
        let mut upper = Voice::default();
        upper.push(0, 24, Pitch::new(4, 0, 4));
        upper.push(24, 120, Pitch::new(6, -1, 4));
        upper.push(120, 144, Pitch::new(5, 0, 4));
        upper.push(144, 192, Pitch::new(3, 1, 4));
        let mut lower = Voice::default();
        lower.push(0, 96, Pitch::new(4, 0, 3));
        lower.push(96, 108, Pitch::new(2, -1, 3));
        lower.push(108, 192, Pitch::new(1, 0, 3));
        let piece = Piece::new("round", key, Meter::COMMON, vec![upper, lower]);
        let xml = musicxml(&piece, "Round", "test", &[]);
        let (read, remarks) = read("round", &xml).unwrap();
        assert_eq!(remarks.chord_tones, 0);
        assert_eq!(read.key, piece.key);
        assert_eq!(read.meter, piece.meter);
        assert_eq!(read.voices.len(), 2);
        for (a, b) in read.voices.iter().zip(&piece.voices) {
            assert_eq!(a.notes, b.notes);
        }
        assert!(super::read("x", "<score-timewise/>").is_err());
    }

    /// The files of a fugue carry the fugue: read back from its MusicXML and its chord lines, it
    /// has the tones of the composer's piece, and the check, which then knows no given tone,
    /// finds no prohibition.
    #[test]
    fn a_fugue_read_back_from_its_files_checks_clean() {
        use crate::compose::{Options, compose};
        let subject = crate::themes::BUILTIN
            .iter()
            .find(|b| b.name == "bwv1080")
            .unwrap()
            .subject()
            .unwrap();
        let options = Options {
            beam: 60,
            ..Options::default()
        };
        let fugue = compose(&subject, &options).unwrap();
        let piece = &fugue.piece;
        let xml = musicxml(piece, "Fuge", "fux-fuge", &[]);
        let chords =
            crate::chordline::write(&fugue.harmony, piece, crate::scaffold::step_of(piece.meter));
        let (read, remarks) = read("fuge", &xml).unwrap();
        assert_eq!(remarks.chord_tones, 0);
        assert_eq!((read.key, read.meter), (piece.key, piece.meter));
        assert_eq!(read.voices.len(), piece.voices.len());
        assert!(read.note_count() > 300, "{} tones", read.note_count());
        for (a, b) in read.voices.iter().zip(&piece.voices) {
            assert_eq!(a.notes, b.notes);
        }
        // The chord lines carry the scaffold: the same chord in the same key on every step of
        // its grid (spans that hold a chord merge in the file, so their number is no measure).
        let harmony = crate::chordline::parse(&chords, &read).unwrap();
        let step = crate::scaffold::step_of(piece.meter);
        let mut at = piece.origin;
        while at < piece.end {
            let (written, back) = (fugue.harmony.at(at).unwrap(), harmony.at(at).unwrap());
            assert_eq!(
                (written.chord, written.key),
                (back.chord, back.key),
                "at {at}"
            );
            at += step;
        }
        let report = crate::check::check(&read, &harmony, &options.profile);
        assert_eq!(report.forbidden(), 0, "{:?}", report.forbidden_by_rule());
    }
}
