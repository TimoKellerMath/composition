//! MusicXML on a piano staff: the voices as MusicXML voices on two staves, with ties, beams,
//! the accidentals a reader expects and marks above the staff. Verovio engraves it in the web
//! page, MuseScore opens it.

use std::collections::HashMap;
use std::fmt::Write;

use crate::export::written;
use crate::score::{Note, Piece};
use crate::time::Tick;

/// Divisions of a quarter note: one tick.
const DIVISIONS: Tick = 24;

/// A written value: its type, dots, and whether it is a triplet value.
struct Value {
    ticks: Tick,
    kind: &'static str,
    dots: usize,
    triplet: bool,
    /// Beams it carries: 1 for an eighth, 2 for a sixteenth, 3 for a thirty-second.
    beams: usize,
}

fn value(ticks: Tick, name: &str) -> Value {
    let triplet = name.contains("*2/3");
    let head = name.split('*').next().unwrap_or(name);
    let dots = head.matches('.').count();
    let (kind, beams) = match head.trim_end_matches('.') {
        "\\breve" => ("breve", 0),
        "1" => ("whole", 0),
        "2" => ("half", 0),
        "4" => ("quarter", 0),
        "8" => ("eighth", 1),
        "16" => ("16th", 2),
        "64" => ("64th", 4),
        _ => ("32nd", 3),
    };
    Value {
        ticks,
        kind,
        dots,
        triplet,
        beams,
    }
}

/// One element of a voice inside a measure.
struct Item {
    on: Tick,
    value: Value,
    /// The note it belongs to; a rest otherwise.
    note: Option<Note>,
    /// Tied to the item before, to the item after.
    tied: (bool, bool),
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The alteration that the signature of `fifths` gives a step.
fn signature_alter(step: i8, fifths: i8) -> i8 {
    const LOF: [i8; 7] = [0, 2, 4, -1, 1, 3, 5];
    let place = LOF[step as usize];
    if fifths > 0 && place + 1 < fifths {
        1
    } else if fifths < 0 && 5 - place < -fifths {
        -1
    } else {
        0
    }
}

fn accidental(alter: i8) -> &'static str {
    match alter {
        2 => "double-sharp",
        1 => "sharp",
        0 => "natural",
        -1 => "flat",
        _ => "flat-flat",
    }
}

/// The piece as MusicXML. `marks` are words above the upper staff at their ticks.
pub fn musicxml(piece: &Piece, title: &str, composer: &str, marks: &[(Tick, String)]) -> String {
    let bar = piece.meter.bar();
    let count = piece.voices.len();
    let fifths = piece.key.signature();
    let end = piece.origin + (piece.end - piece.origin + bar - 1) / bar * bar;
    let mut starts: Vec<Tick> = Vec::new();
    if piece.origin > 0 {
        starts.push(0);
    }
    let mut t = piece.origin;
    while t < end {
        starts.push(t);
        t += bar;
    }
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<!DOCTYPE score-partwise PUBLIC \"-//Recordare//DTD MusicXML 4.0 Partwise//EN\" \"http://www.musicxml.org/dtds/partwise.dtd\">\n");
    out.push_str("<score-partwise version=\"4.0\">\n");
    let _ = writeln!(
        out,
        "  <work><work-title>{}</work-title></work>",
        escape(title)
    );
    let _ = writeln!(
        out,
        "  <identification><creator type=\"composer\">{}</creator><encoding><software>fux-bach</software></encoding></identification>",
        escape(composer)
    );
    out.push_str(
        "  <part-list><score-part id=\"P1\"><part-name></part-name></score-part></part-list>\n",
    );
    out.push_str("  <part id=\"P1\">\n");
    // The staff of every voice, bar by bar: the middle voices go to the hand that reaches them.
    let hands = crate::hands::staves(piece, &starts);
    for (m, &from) in starts.iter().enumerate() {
        let staff_of = |v: usize| hands[v][m] as usize + 1;
        let until = starts.get(m + 1).copied().unwrap_or(end);
        let number = if piece.origin > 0 {
            m as i32
        } else {
            m as i32 + 1
        };
        let implicit = if piece.origin > 0 && m == 0 {
            " implicit=\"yes\""
        } else {
            ""
        };
        let _ = writeln!(out, "    <measure number=\"{number}\"{implicit}>");
        if m == 0 {
            let _ = writeln!(
                out,
                "      <attributes><divisions>{DIVISIONS}</divisions><key><fifths>{fifths}</fifths><mode>{}</mode></key><time><beats>{}</beats><beat-type>{}</beat-type></time><staves>2</staves><clef number=\"1\"><sign>G</sign><line>2</line></clef><clef number=\"2\"><sign>F</sign><line>4</line></clef></attributes>",
                if piece.key.minor { "minor" } else { "major" },
                piece.meter.num,
                piece.meter.den
            );
        }
        // The items of every voice in this measure.
        let mut voices: Vec<Vec<Item>> = Vec::new();
        for voice in &piece.voices {
            let mut items: Vec<Item> = Vec::new();
            let mut at = from;
            let first = voice.notes.partition_point(|n| n.off <= from);
            for note in voice.notes[first..].iter().take_while(|n| n.on < until) {
                let (a, b) = (note.on.max(from), note.off.min(until));
                if a > at {
                    let mut on = at;
                    for (ticks, name) in written(piece, at, a) {
                        items.push(Item {
                            on,
                            value: value(ticks, &name),
                            note: None,
                            tied: (false, false),
                        });
                        on += ticks;
                    }
                }
                let parts = written(piece, a, b);
                let last = parts.len() - 1;
                let mut on = a;
                for (i, (ticks, name)) in parts.into_iter().enumerate() {
                    items.push(Item {
                        on,
                        value: value(ticks, &name),
                        note: Some(*note),
                        tied: (i > 0 || note.on < a, i < last || note.off > b),
                    });
                    on += ticks;
                }
                at = b;
            }
            if at < until && !items.is_empty() {
                let mut on = at;
                for (ticks, name) in written(piece, at, until) {
                    items.push(Item {
                        on,
                        value: value(ticks, &name),
                        note: None,
                        tied: (false, false),
                    });
                    on += ticks;
                }
            }
            voices.push(items);
        }
        // Accidentals: per staff the tones in the order of time against the state of the measure.
        let mut shown: HashMap<(usize, Tick), bool> = HashMap::new();
        for staff in 1..=2 {
            let mut heads: Vec<(Tick, usize, Note)> = Vec::new();
            for (v, items) in voices.iter().enumerate() {
                if staff_of(v) != staff {
                    continue;
                }
                for item in items {
                    if let Some(note) = item.note
                        && !item.tied.0
                    {
                        heads.push((item.on, v, note));
                    }
                }
            }
            heads.sort_by_key(|h| (h.0, h.1));
            let mut state: HashMap<i16, i8> = HashMap::new();
            for (on, v, note) in heads {
                let alter = note.pitch.alter();
                let before = state
                    .get(&note.pitch.dia)
                    .copied()
                    .unwrap_or(signature_alter(note.pitch.step(), fifths));
                if alter != before {
                    shown.insert((v, on), true);
                }
                state.insert(note.pitch.dia, alter);
            }
        }
        // Words above the upper staff.
        for (tick, words) in marks {
            if *tick >= from && *tick < until {
                let _ = writeln!(
                    out,
                    "      <direction placement=\"above\"><direction-type><words font-style=\"italic\">{}</words></direction-type><offset>{}</offset><voice>1</voice><staff>1</staff></direction>",
                    escape(words),
                    tick - from
                );
            }
        }
        let silent_staff = |staff: usize| {
            voices
                .iter()
                .enumerate()
                .all(|(v, items)| staff_of(v) != staff || items.iter().all(|i| i.note.is_none()))
        };
        let mut written_any = false;
        for (v, items) in voices.iter().enumerate() {
            let staff = staff_of(v);
            let sounding = items.iter().any(|i| i.note.is_some());
            // A silent voice is skipped; a silent staff shows one rest for the measure.
            let first_of_staff = (0..count).find(|w| staff_of(*w) == staff) == Some(v);
            if !sounding && !(silent_staff(staff) && first_of_staff) {
                continue;
            }
            if written_any {
                let _ = writeln!(
                    out,
                    "      <backup><duration>{}</duration></backup>",
                    until - from
                );
            }
            written_any = true;
            if !sounding {
                let _ = writeln!(
                    out,
                    "      <note><rest measure=\"yes\"/><duration>{}</duration><voice>{}</voice><staff>{staff}</staff></note>",
                    until - from,
                    v + 1
                );
                continue;
            }
            let beat = piece.meter.beat();
            for (i, item) in items.iter().enumerate() {
                let mut text = String::from("      <note>");
                match item.note {
                    None => text.push_str("<rest/>"),
                    Some(note) => {
                        let step = b"CDEFGAB"[note.pitch.step() as usize] as char;
                        let _ = write!(text, "<pitch><step>{step}</step>");
                        if note.pitch.alter() != 0 {
                            let _ = write!(text, "<alter>{}</alter>", note.pitch.alter());
                        }
                        let _ = write!(text, "<octave>{}</octave></pitch>", note.pitch.octave());
                    }
                }
                let _ = write!(text, "<duration>{}</duration>", item.value.ticks);
                if item.note.is_some() {
                    if item.tied.0 {
                        text.push_str("<tie type=\"stop\"/>");
                    }
                    if item.tied.1 {
                        text.push_str("<tie type=\"start\"/>");
                    }
                }
                let _ = write!(
                    text,
                    "<voice>{}</voice><type>{}</type>",
                    v + 1,
                    item.value.kind
                );
                text.push_str(&"<dot/>".repeat(item.value.dots));
                if let Some(note) = item.note
                    && shown.contains_key(&(v, item.on))
                    && !item.tied.0
                {
                    let _ = write!(
                        text,
                        "<accidental>{}</accidental>",
                        accidental(note.pitch.alter())
                    );
                }
                if item.value.triplet {
                    text.push_str("<time-modification><actual-notes>3</actual-notes><normal-notes>2</normal-notes></time-modification>");
                }
                let _ = write!(text, "<staff>{staff}</staff>");
                // Beams: tones with flags that follow each other inside one beat.
                if item.note.is_some() && item.value.beams > 0 {
                    let joins = |a: &Item, b: &Item| {
                        a.note.is_some()
                            && b.note.is_some()
                            && a.value.beams > 0
                            && b.value.beams > 0
                            && a.on + a.value.ticks == b.on
                            && piece.pos(a.on) / beat == piece.pos(b.on) / beat
                    };
                    let before = (i > 0 && joins(&items[i - 1], item)).then(|| &items[i - 1]);
                    let after = items.get(i + 1).filter(|next| joins(item, next));
                    if before.is_some() || after.is_some() {
                        for level in 1..=item.value.beams {
                            let from_before = before.is_some_and(|b| b.value.beams >= level);
                            let to_after = after.is_some_and(|a| a.value.beams >= level);
                            let state = match (from_before, to_after) {
                                (true, true) => "continue",
                                (false, true) => "begin",
                                (true, false) => "end",
                                (false, false) if before.is_some() => "backward hook",
                                (false, false) => "forward hook",
                            };
                            let _ = write!(text, "<beam number=\"{level}\">{state}</beam>");
                        }
                    }
                }
                if item.note.is_some() && (item.tied.0 || item.tied.1) {
                    text.push_str("<notations>");
                    if item.tied.0 {
                        text.push_str("<tied type=\"stop\"/>");
                    }
                    if item.tied.1 {
                        text.push_str("<tied type=\"start\"/>");
                    }
                    text.push_str("</notations>");
                }
                text.push_str("</note>\n");
                out.push_str(&text);
            }
        }
        if m + 1 == starts.len() {
            out.push_str(
                "      <barline location=\"right\"><bar-style>light-heavy</bar-style></barline>\n",
            );
        }
        out.push_str("    </measure>\n");
    }
    out.push_str("  </part>\n</score-partwise>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
    use crate::pitch::Pitch;
    use crate::score::Voice;
    use crate::time::Meter;

    #[test]
    fn a_small_piece_is_written() {
        let mut upper = Voice::default();
        // B natural in C minor, then four sixteenths, then a tone tied over the barline.
        upper.push(0, 24, Pitch::new(6, 0, 4));
        for (i, step) in [0, 1, 2, 3].iter().enumerate() {
            let on = 24 + 6 * i as Tick;
            upper.push(
                on,
                on + 6,
                Pitch::new(*step, if *step == 2 { -1 } else { 0 }, 5),
            );
        }
        upper.push(48, 120, Pitch::new(4, 0, 5));
        let mut lower = Voice::default();
        lower.push(0, 192, Pitch::new(0, 0, 3));
        let piece = Piece::new(
            "small",
            Key::parse("c").unwrap(),
            Meter::COMMON,
            vec![upper, lower],
        );
        let xml = musicxml(&piece, "Probe & Test", "fux-bach", &[(0, "Thema".into())]);
        assert!(xml.contains("<fifths>-3</fifths><mode>minor</mode>"));
        assert!(xml.contains("<work-title>Probe &amp; Test</work-title>"));
        // The B natural needs its sign, the E flat of the signature none.
        assert!(xml.contains("<step>B</step><octave>4</octave></pitch><duration>24</duration><voice>1</voice><type>quarter</type><accidental>natural</accidental>"));
        assert!(!xml.contains("<accidental>flat</accidental>"));
        assert_eq!(xml.matches("<beam number=\"1\">begin</beam>").count(), 1);
        assert_eq!(xml.matches("<beam number=\"2\">continue</beam>").count(), 2);
        assert_eq!(xml.matches("<tie type=\"start\"/>").count(), 2);
        assert_eq!(xml.matches("<measure ").count(), 2);
        assert!(xml.contains("<words font-style=\"italic\">Thema</words>"));
        assert!(xml.contains("<backup><duration>96</duration></backup>"));
        assert_eq!(signature_alter(3, 1), 1);
        assert_eq!(signature_alter(6, -1), -1);
        assert_eq!(signature_alter(0, 2), 1);
        assert_eq!(signature_alter(2, -2), -1);
        assert_eq!(signature_alter(5, -2), 0);
    }
}
