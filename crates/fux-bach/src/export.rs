//! Scores for reading and listening: LilyPond on a piano staff and a MIDI file with one track
//! per voice. Plain on purpose: the product's exporters work on its own grid of eighths.

use crate::pitch::Pitch;
use crate::score::{Note, Piece};
use crate::time::{QUARTER, Tick};

/// Writable values in ticks with their LilyPond names, long to short.
const VALUES: [(Tick, &str); 12] = [
    (192, "\\breve"),
    (144, "1."),
    (96, "1"),
    (72, "2."),
    (48, "2"),
    (36, "4."),
    (24, "4"),
    (18, "8."),
    (12, "8"),
    (9, "16."),
    (6, "16"),
    (3, "32"),
];

/// The largest undotted value inside a value.
fn base(value: Tick) -> Tick {
    [192, 96, 48, 24, 12, 6, 3]
        .into_iter()
        .find(|b| *b <= value)
        .unwrap_or(3)
}

/// Splits `[from, until)` into written values: no value crosses a barline, and a tone that
/// begins inside a beat is first filled up to the next beat.
pub(crate) fn written(piece: &Piece, from: Tick, until: Tick) -> Vec<(Tick, String)> {
    let meter = piece.meter;
    let beat = meter.beat();
    // Triplet values: the plain value, scaled, without a bracket.
    if let Some(name) = [(16, "4*2/3"), (8, "8*2/3"), (4, "16*2/3"), (2, "32*2/3")]
        .into_iter()
        .find(|(v, _)| *v == until - from)
        .map(|(_, name)| name)
    {
        return vec![(until - from, name.to_string())];
    }
    let mut out = Vec::new();
    let mut at = from;
    while at < until {
        let pos = piece.pos(at);
        let in_beat = pos % beat;
        let limit = if in_beat != 0 {
            beat - in_beat
        } else {
            meter.bar() - pos
        };
        let room = (until - at).min(limit);
        let fits = |value: Tick| {
            let dotted = value != base(value);
            value <= room
                && if in_beat != 0 {
                    // Inside a beat a value begins on its own grid, a dotted one on the grid of
                    // twice its undotted value.
                    in_beat % if dotted { 2 * base(value) } else { value } == 0
                } else {
                    value <= beat || value % beat == 0 || !meter.compound()
                }
        };
        match VALUES.into_iter().find(|(v, _)| fits(*v)) {
            Some((value, name)) => {
                out.push((value, name.to_string()));
                at += value;
            }
            None => {
                // Off the binary grid: the largest triplet value that fits.
                let (value, name) = [
                    (16, "4*2/3"),
                    (8, "8*2/3"),
                    (4, "16*2/3"),
                    (2, "32*2/3"),
                    (1, "64*2/3"),
                ]
                .into_iter()
                .find(|(v, _)| *v <= room)
                .unwrap_or((1, "64*2/3"));
                out.push((value, name.to_string()));
                at += value;
            }
        }
    }
    out
}

/// The tones of a voice as LilyPond. `at_bar` gives what is to be said at the start of a bar
/// (a change of staff or of stem direction), `hidden` whether a bar in which the voice is silent
/// shows no rest.
fn voice_music(
    piece: &Piece,
    notes: &[Note],
    grey: &dyn Fn(&Note) -> bool,
    end: Tick,
    at_bar: &dyn Fn(Tick) -> Option<String>,
    hidden: &dyn Fn(Tick) -> bool,
) -> String {
    let bar = piece.meter.bar();
    let mut out = String::new();
    let mut at = 0;
    let mut column = 0;
    let mut push = |out: &mut String, token: String, from: Tick| {
        // What the bar that begins here has to say comes first.
        let mut token = token;
        if (from - piece.origin).rem_euclid(bar) == 0
            && let Some(command) = at_bar(from)
        {
            token = format!("{command} {token}");
        }
        if column + token.len() > 96 {
            out.push_str("\n    ");
            column = 0;
        }
        column += token.len() + 1;
        out.push_str(&token);
        out.push(' ');
    };
    let rests = |out: &mut String,
                 from: Tick,
                 until: Tick,
                 push: &mut dyn FnMut(&mut String, String, Tick)| {
        let mut on = from;
        for (ticks, value) in written(piece, from, until) {
            // A whole bar of silence beside a sounding voice shows nothing.
            let whole = ticks == bar && (on - piece.origin).rem_euclid(bar) == 0;
            let letter = if whole && hidden(on) { "s" } else { "r" };
            push(out, format!("{letter}{value}"), on);
            on += ticks;
        }
    };
    for note in notes {
        if note.on > at {
            rests(&mut out, at, note.on, &mut push);
        }
        let parts = written(piece, note.on, note.off);
        let count = parts.len();
        let mut on = note.on;
        for (i, (ticks, value)) in parts.into_iter().enumerate() {
            let colour = if grey(note) {
                "\\tweak color #(x11-color 'grey45) "
            } else {
                ""
            };
            let tie = if i + 1 < count { "~" } else { "" };
            push(
                &mut out,
                format!("{colour}{}{value}{tie}", note.pitch.lilypond()),
                on,
            );
            on += ticks;
        }
        at = note.off;
    }
    if at < end {
        rests(&mut out, at, end, &mut push);
    }
    out
}

/// The piece as a LilyPond file on a piano staff. Tones for which `grey` holds are printed grey
/// (the fixed tones of a re-setting). `marks` are words over the upper staff, each from its
/// tick on (the sections of a fugue).
pub fn lilypond(
    piece: &Piece,
    title: &str,
    subtitle: &str,
    marks: &[(Tick, String)],
    grey: &dyn Fn(usize, &Note) -> bool,
) -> String {
    let bar = piece.meter.bar();
    let end = piece.origin + (piece.end - piece.origin + bar - 1) / bar * bar;
    let count = piece.voices.len();
    let partial = if piece.origin > 0 {
        match VALUES.into_iter().find(|(v, _)| *v == piece.origin) {
            Some((_, name)) => format!("\\partial {name} "),
            None => format!("\\partial 32*{}/3 ", piece.origin),
        }
    } else {
        String::new()
    };
    let global = format!(
        "{} \\time {}/{} {partial}",
        piece.key.lilypond(),
        piece.meter.num,
        piece.meter.den
    );
    // The bars, and for every voice the staff it stands on in each of them.
    let mut starts: Vec<Tick> = Vec::new();
    if piece.origin > 0 {
        starts.push(0);
    }
    let mut t = piece.origin;
    while t < end {
        starts.push(t);
        t += bar;
    }
    let hands = crate::hands::staves(piece, &starts);
    let bar_of = |t: Tick| starts.partition_point(|s| *s <= t).saturating_sub(1);
    let sounding = |v: usize, k: usize| {
        let until = starts.get(k + 1).copied().unwrap_or(end);
        piece.voices[v]
            .notes
            .iter()
            .any(|n| n.on < until && starts[k] < n.off)
    };
    // The stems of a voice in a bar: up for the first sounding voice of its staff, down for the
    // last, free where it is alone.
    let stems = |v: usize, k: usize| -> &'static str {
        let mates: Vec<usize> = (0..count)
            .filter(|w| hands[*w][k] == hands[v][k] && (sounding(*w, k) || *w == v))
            .collect();
        match (mates.len(), mates.iter().position(|w| *w == v)) {
            (1, _) => "\\oneVoice",
            (_, Some(0)) => "\\voiceOne",
            (n, Some(i)) if i + 1 == n => "\\voiceTwo",
            _ => "\\voiceFour",
        }
    };
    let name = |staff: u8| if staff == 0 { "oben" } else { "unten" };
    let voice = |v: usize| -> String {
        let at_bar = |t: Tick| -> Option<String> {
            let k = bar_of(t);
            if k == 0 {
                return None;
            }
            let mut says = Vec::new();
            if hands[v][k] != hands[v][k - 1] {
                says.push(format!("\\change Staff = \"{}\"", name(hands[v][k])));
            }
            if stems(v, k) != stems(v, k - 1) || hands[v][k] != hands[v][k - 1] {
                says.push(stems(v, k).to_string());
            }
            (!says.is_empty()).then(|| says.join(" "))
        };
        // A silent bar shows its rest only if the whole staff is silent and this is its first
        // voice.
        let hidden = |t: Tick| -> bool {
            let k = bar_of(t);
            let staff = hands[v][k];
            let any = (0..count).any(|w| hands[w][k] == staff && sounding(w, k));
            let first = (0..count).find(|w| hands[*w][k] == staff) == Some(v);
            any || !first
        };
        let music = voice_music(
            piece,
            &piece.voices[v].notes,
            &|n| grey(v, n),
            end,
            &at_bar,
            &hidden,
        );
        format!(
            "    \\new Voice {{ {} {global}\n    {music}\\bar \"|.\" }}",
            stems(v, 0)
        )
    };
    let quote = |text: &str| text.replace('\\', "").replace('"', "'");
    // The words over the upper staff: a voice of spacers, each carrying the word that begins
    // with it.
    let words = || -> Option<String> {
        let first = starts[0];
        let mut places: Vec<(Tick, &str)> = marks
            .iter()
            .filter(|(t, _)| *t < end)
            .map(|(t, text)| ((*t).max(first), text.as_str()))
            .collect();
        places.sort_by_key(|(t, _)| *t);
        places.dedup_by_key(|(t, _)| *t);
        if places.is_empty() {
            return None;
        }
        let spacer = |ticks: Tick| -> String {
            let gcd = |mut a: Tick, mut b: Tick| {
                while b != 0 {
                    (a, b) = (b, a % b);
                }
                a
            };
            let d = gcd(ticks, crate::time::TPW);
            match (ticks / d, crate::time::TPW / d) {
                (n, 1) => format!("s1*{n}"),
                (n, m) => format!("s1*{n}/{m}"),
            }
        };
        let mut music = String::new();
        if places[0].0 > first {
            music.push_str(&format!("{} ", spacer(places[0].0 - first)));
        }
        for (k, (at, text)) in places.iter().enumerate() {
            let until = places.get(k + 1).map_or(end, |next| next.0);
            music.push_str(&format!(
                "{}^\\markup \\italic \"{}\" ",
                spacer(until - at),
                quote(text)
            ));
        }
        Some(format!(
            "    \\new Voice {{ {global}\n    \\override TextScript.font-size = #-1 {music}}}"
        ))
    };
    let staff = |which: u8, clef: &str| -> String {
        let mut lines: Vec<String> = (0..count)
            .filter(|v| hands[*v][0] == which)
            .map(voice)
            .collect();
        if which == 0
            && let Some(words) = words()
        {
            lines.push(words);
        }
        format!(
            "  \\new Staff = \"{}\" {{ \\clef {clef} <<\n{}\n  >> }}",
            name(which),
            if lines.is_empty() {
                format!("    {{ {global} s1 }}")
            } else {
                lines.join("\n")
            }
        )
    };
    format!(
        "\\version \"2.24.0\"\n\\header {{\n  title = \"{}\"\n  subtitle = \"{}\"\n  tagline = ##f\n}}\n\\paper {{ indent = 0 }}\n\\score {{\n \\new PianoStaff <<\n{}\n{}\n >>\n \\layout {{ \\context {{ \\Score \\override SpacingSpanner.common-shortest-duration = #(ly:make-moment 1/16) }} }}\n}}\n",
        quote(title),
        quote(subtitle),
        staff(0, "treble"),
        staff(1, "bass"),
    )
}

/// The piece as a standard MIDI file: one track per voice, harpsichord, the given quarters per
/// minute.
pub fn midi(piece: &Piece, quarters_per_minute: u32) -> Vec<u8> {
    use midly::num::{u4, u7, u15, u24, u28};
    use midly::{
        Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
    };
    const PER_QUARTER: u32 = 480;
    let scale = PER_QUARTER / QUARTER as u32;
    let mut smf = Smf::new(Header::new(
        Format::Parallel,
        Timing::Metrical(u15::new(PER_QUARTER as u16)),
    ));
    let tempo = TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(
            60_000_000 / quarters_per_minute.max(1),
        ))),
    };
    let end = TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    };
    smf.tracks.push(vec![tempo, end]);
    for (v, voice) in piece.voices.iter().enumerate() {
        let channel = u4::new(v as u8 % 16);
        let mut events: Vec<(Tick, bool, Pitch)> = Vec::new();
        for note in &voice.notes {
            events.push((note.on, true, note.pitch));
            events.push((note.off, false, note.pitch));
        }
        // A tone ends before the next begins at the same tick.
        events.sort_by_key(|(tick, on, _)| (*tick, *on));
        let mut track = vec![TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Midi {
                channel,
                message: MidiMessage::ProgramChange {
                    program: u7::new(6),
                },
            },
        }];
        let mut at = 0;
        for (tick, on, pitch) in events {
            let key = u7::new(pitch.midi.clamp(0, 127) as u8);
            let message = if on {
                MidiMessage::NoteOn {
                    key,
                    vel: u7::new(80),
                }
            } else {
                MidiMessage::NoteOff {
                    key,
                    vel: u7::new(0),
                }
            };
            track.push(TrackEvent {
                delta: u28::new((tick - at).max(0) as u32 * scale),
                kind: TrackEventKind::Midi { channel, message },
            });
            at = tick;
        }
        track.push(end);
        smf.tracks.push(track);
    }
    let mut bytes = Vec::new();
    smf.write(&mut bytes).expect("writing to memory");
    bytes
}

/// A usual tempo for a meter, in quarters per minute.
pub fn tempo(piece: &Piece) -> u32 {
    match (piece.meter.num, piece.meter.den) {
        (_, 2) => 132,
        (_, 8) | (_, 16) => 96,
        (3, 4) => 84,
        _ => 72,
    }
}

/// Bars as text for a quick look: per bar and voice the tones that begin in it.
pub fn bars(piece: &Piece, from: i32, to: i32) -> String {
    let mut out = String::new();
    for bar in from..=to {
        let start = piece.tick_of(bar, 0);
        if start >= piece.end {
            break;
        }
        out.push_str(&format!("bar {bar:3}\n"));
        for (v, voice) in piece.voices.iter().enumerate() {
            let tones: Vec<String> = voice
                .range(start, start + piece.meter.bar())
                .iter()
                .map(|n| format!("{}:{}", n.pitch, n.dur() / 6))
                .collect();
            out.push_str(&format!("   v{v} {}\n", tones.join(" ")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
    use crate::score::Voice;
    use crate::time::Meter;

    fn piece() -> Piece {
        let mut upper = Voice::default();
        upper.push(6, 30, Pitch::new(0, 1, 5));
        upper.push(30, 96, Pitch::new(1, 0, 5));
        upper.push(96, 104, Pitch::new(2, -1, 5));
        let mut lower = Voice::default();
        lower.push(0, 192, Pitch::new(0, 0, 3));
        Piece::new(
            "two",
            Key::parse("c").unwrap(),
            Meter::COMMON,
            vec![upper, lower],
        )
    }

    #[test]
    fn values_stay_inside_beats_and_bars() {
        let piece = piece();
        let names = |from, until| -> Vec<String> {
            written(&piece, from, until)
                .into_iter()
                .map(|w| w.1)
                .collect()
        };
        // From the second sixteenth: fill the beat, then a sixteenth of the next.
        assert_eq!(names(6, 30), ["16", "8", "16"]);
        assert_eq!(names(30, 96), ["16", "8", "2"]);
        assert_eq!(names(0, 192), ["1", "1"]);
        assert_eq!(names(96, 104), ["8*2/3"]);
    }

    #[test]
    fn files_are_written() {
        let piece = piece();
        let marks = [(0, "Thema".to_string()), (144, "Antwort \"a\"".to_string())];
        let ly = lilypond(&piece, "Probe", "zwei Stimmen", &marks, &|v, _| v == 1);
        assert!(ly.contains("\\key c \\minor"));
        // The words of the sections stand over the upper staff, each at its tick.
        assert!(
            ly.contains(
                "s1*3/2^\\markup \\italic \"Thema\" s1*1/2^\\markup \\italic \"Antwort 'a'\""
            ),
            "{ly}"
        );
        assert!(!lilypond(&piece, "Probe", "", &[], &|_, _| false).contains("TextScript"));
        assert!(ly.contains("cis''16~ cis''8~ cis''16"));
        assert!(ly.contains("grey45) c1~"));
        let bytes = midi(&piece, 72);
        let smf = midly::Smf::parse(&bytes).unwrap();
        assert_eq!(smf.tracks.len(), 3);
        assert!(bars(&piece, 1, 1).contains("v0 C#5:4 D5:11"));
    }
}
