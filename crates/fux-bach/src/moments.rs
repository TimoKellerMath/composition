//! Counts at the exposed moments of a fugue: the arrivals of its cadences, the entries of its
//! subject and the ends of its episodes (protocol of phase B4). The same definitions read
//! Bach's fugues, whose moments come from labels, and the program's, whose moments come from
//! the plan.

use crate::key::Key;
use crate::pitch::Pitch;
use crate::score::{Note, Piece};
use crate::time::Tick;

macro_rules! counts {
    ($($name:ident, $id:literal, $german:literal;)*) => {
        /// A statement about a moment; its count is the share of the moments where it holds.
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum Count { $($name,)* }

        impl Count {
            pub const ALL: &'static [Count] = &[$(Count::$name,)*];

            pub const fn id(self) -> &'static str {
                match self { $(Count::$name => $id,)* }
            }

            pub const fn german(self) -> &'static str {
                match self { $(Count::$name => $german,)* }
            }
        }
    };
}

counts! {
    KadBass, "KAD-BASS", "Bass auf dem Grundton";
    KadBass5, "KAD-BASS-5", "Bass kommt von der Quinte";
    KadOben, "KAD-OBEN", "Oberstimme auf dem Grundton";
    KadObenSchritt, "KAD-OBEN-SCHRITT", "Oberstimme kommt mit einem Schritt";
    KadObenSprung, "KAD-OBEN-SPRUNG", "Oberstimme springt oder kommt aus der Pause";
    KadTerz, "KAD-TERZ", "Terz im Klang";
    KadLeitton, "KAD-LEITTON", "Leitton steigt in den Grundton";
    KadDominante, "KAD-DOMINANTE", "davor nur Töne der Dominante, mit Leitton";
    KadBassDavor, "KAD-BASS-DAVOR", "Bass davor auf der Quinte";
    KadAlle, "KAD-ALLE", "alle Stimmen klingen";
    KadEinsatz, "KAD-EINSATZ", "ein Einsatz beginnt in der Kadenz";
    EinPause, "EIN-PAUSE", "die Stimme pausiert im Schlag davor";
    EinErster, "EIN-ERSTER", "erster Einsatz der Stimme";
    EinAlle, "EIN-ALLE", "alle anderen Stimmen klingen";
    EinAussen, "EIN-AUSSEN", "erster Ton ist der höchste oder tiefste";
    EinKonsonant, "EIN-KONSONANT", "erster Ton konsonant zum Bass";
    EinVerdoppelt, "EIN-VERDOPPELT", "eine andere Stimme setzt mit demselben Ton an";
    EinKadenz, "EIN-KADENZ", "eine Kadenz kommt am Einsatz an";
    EinAbloesung, "EIN-ABLOESUNG", "eine andere Stimme verstummt am Einsatz";
    ZwLeitton, "ZW-LEITTON", "Leitton der Einsatztonart im Schlag davor";
    ZwDominante, "ZW-DOMINANTE", "davor nur Töne der Dominante, mit Leitton";
    ZwBass, "ZW-BASS", "Bass von der Quinte in den Grundton";
    ZwKadenz, "ZW-KADENZ", "eine Kadenz kommt am Einsatz an";
    ZwPause, "ZW-PAUSE", "die Stimme pausiert im Schlag davor";
    ZwBewegung, "ZW-BEWEGUNG", "die Bewegung hält im Takt davor an";
}

pub const COUNT: usize = Count::ALL.len();

/// The moments of a piece.
#[derive(Clone, Debug, Default)]
pub struct Marks {
    /// The cadences in root position: where they arrive and in which key.
    pub cadences: Vec<(Tick, Key)>,
    /// The arrivals of all cadences, of any kind.
    pub arrivals: Vec<Tick>,
    /// The complete entries of the subject: voice, first tone, end.
    pub entries: Vec<(usize, Tick, Tick)>,
    /// Where material of the subject sounds, in any form.
    pub subject: Vec<(Tick, Tick)>,
}

/// How often each statement held, and at how many moments it could.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tally {
    pub hits: [u32; COUNT],
    pub chances: [u32; COUNT],
    /// The moments read: cadences, entries, ends of episodes.
    pub moments: [u32; 3],
}

impl Default for Tally {
    fn default() -> Tally {
        Tally {
            hits: [0; COUNT],
            chances: [0; COUNT],
            moments: [0; 3],
        }
    }
}

impl Tally {
    fn note(&mut self, count: Count, holds: bool) {
        self.chances[count as usize] += 1;
        self.hits[count as usize] += u32::from(holds);
    }

    pub fn add(&mut self, other: &Tally) {
        for k in 0..COUNT {
            self.hits[k] += other.hits[k];
            self.chances[k] += other.chances[k];
        }
        for k in 0..3 {
            self.moments[k] += other.moments[k];
        }
    }

    pub fn share(&self, count: Count) -> Option<f64> {
        let n = self.chances[count as usize];
        (n > 0).then(|| self.hits[count as usize] as f64 / n as f64)
    }
}

/// The 95 % interval of a share after Wilson.
pub fn wilson(hits: u32, chances: u32) -> (f64, f64) {
    if chances == 0 {
        return (0.0, 1.0);
    }
    let (n, z) = (chances as f64, 1.96f64);
    let p = hits as f64 / n;
    let centre = (p + z * z / (2.0 * n)) / (1.0 + z * z / n);
    let half = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt() / (1.0 + z * z / n);
    ((centre - half).max(0.0), (centre + half).min(1.0))
}

/// Whether two shares differ as the protocol asks of a finding: by `points` or more of 100,
/// with intervals that do not touch, and at least twenty moments on either side.
pub fn differs(a: (u32, u32), b: (u32, u32), points: f64) -> bool {
    if a.1 < 20 || b.1 < 20 {
        return false;
    }
    let (pa, pb) = (a.0 as f64 / a.1 as f64, b.0 as f64 / b.1 as f64);
    let (ia, ib) = (wilson(a.0, a.1), wilson(b.0, b.1));
    (pa - pb).abs() * 100.0 >= points - 1e-9 && (ia.1 < ib.0 || ib.1 < ia.0)
}

/// Semitones above the tonic of `key`.
fn degree(pitch: Pitch, key: Key) -> u8 {
    (pitch.pc() + 12 - key.tonic.pc()) % 12
}

/// The tones of the dominant seventh chord, as degrees.
const DOMINANT: [u8; 4] = [7, 11, 2, 5];

/// The voices that sound at `t`, each with its tone.
fn sounding(piece: &Piece, t: Tick) -> Vec<(usize, Note)> {
    piece
        .voices
        .iter()
        .enumerate()
        .filter_map(|(v, voice)| voice.at(t).map(|n| (v, *n)))
        .collect()
}

fn lowest(tones: &[(usize, Note)]) -> Option<(usize, Note)> {
    tones
        .iter()
        .copied()
        .min_by_key(|(v, n)| (n.pitch.midi, usize::MAX - v))
}

fn highest(tones: &[(usize, Note)]) -> Option<(usize, Note)> {
    tones
        .iter()
        .copied()
        .max_by_key(|(v, n)| (n.pitch.midi, usize::MAX - v))
}

/// How a voice comes to the tone it has at `t`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Way {
    /// The tone began before.
    Holds,
    /// It begins at `t` after the tone given, which ended at most a beat before.
    From(Note),
    /// It begins at `t` after a longer silence.
    Rest,
}

fn way(piece: &Piece, v: usize, t: Tick) -> Option<Way> {
    let voice = &piece.voices[v];
    let i = voice.index_at(t)?;
    if voice.notes[i].on < t {
        return Some(Way::Holds);
    }
    Some(match i.checked_sub(1).map(|k| voice.notes[k]) {
        Some(before) if before.off >= t - piece.meter.beat() => Way::From(before),
        _ => Way::Rest,
    })
}

/// Whether what sounds at `t` is a dominant of `key`: two voices or more, tones of its seventh
/// chord only, the leading tone among them.
fn dominant(piece: &Piece, t: Tick, key: Key) -> bool {
    let tones = sounding(piece, t);
    tones.len() >= 2
        && tones
            .iter()
            .all(|(_, n)| DOMINANT.contains(&degree(n.pitch, key)))
        && tones.iter().any(|(_, n)| degree(n.pitch, key) == 11)
}

/// Whether voice `v` is silent in the beat before `t`.
fn rests_before(piece: &Piece, v: usize, t: Tick) -> bool {
    let from = t - piece.meter.beat();
    !piece.voices[v]
        .notes
        .iter()
        .any(|n| n.on < t && n.off > from)
}

/// Where a cadence arrives, seen from the entries of the subject.
pub const PLACES: [&str; 4] = [
    "am Anfang eines Einsatzes",
    "am Ende eines Einsatzes",
    "in einem Einsatz",
    "außerhalb der Einsätze",
];

/// The cadences in root position by their place (`PLACES`): within a beat of the first tone of
/// an entry; else from a beat before the onset of its last tone to a beat after its end; else
/// while material of the subject sounds; else outside. For those outside, the bars to the next
/// entry are collected in `ahead` (rounded down).
pub fn places(piece: &Piece, marks: &Marks, ahead: &mut Vec<i32>) -> [u32; 4] {
    let beat = piece.meter.beat();
    let mut out = [0u32; 4];
    for (at, _) in &marks.cadences {
        let at = *at;
        if sounding(piece, at).len() < 2 {
            continue;
        }
        let begins = marks.entries.iter().any(|(_, t, _)| (t - at).abs() <= beat);
        let ends = marks.entries.iter().any(|(v, _, end)| {
            let last = piece
                .voices
                .get(*v)
                .and_then(|voice| voice.at(end - 1))
                .map_or(*end, |n| n.on);
            at >= last - beat && at <= end + beat
        });
        let inside = marks
            .subject
            .iter()
            .any(|(from, until)| *from <= at && at < *until);
        let place = if begins {
            0
        } else if ends {
            1
        } else if inside {
            2
        } else {
            3
        };
        out[place] += 1;
        if place == 3
            && let Some(next) = marks.entries.iter().map(|e| e.1).filter(|t| *t > at).min()
        {
            ahead.push((next - at).div_euclid(piece.meter.bar()));
        }
    }
    out
}

/// What the voices do that double the first tone of an entry (EIN-VERDOPPELT): the entries
/// with such a voice, those where it is the lowest that sounds, those where it falls silent
/// within a bar, and those that are the first of their voice.
pub fn doublings(piece: &Piece, marks: &Marks) -> [u32; 4] {
    let mut out = [0u32; 4];
    let first = marks.entries.iter().map(|e| e.1).min();
    for (v, at, _) in &marks.entries {
        let (v, at) = (*v, *at);
        if Some(at) == first || v >= piece.voices.len() {
            continue;
        }
        let Some(tone) = piece.voices[v].at(at).copied().filter(|n| n.on == at) else {
            continue;
        };
        let tones = sounding(piece, at);
        let doubling: Vec<&(usize, Note)> = tones
            .iter()
            .filter(|(w, n)| *w != v && n.on == at && n.pitch.pc() == tone.pitch.pc())
            .collect();
        if doubling.is_empty() {
            continue;
        }
        out[0] += 1;
        let low = lowest(&tones).map(|(w, _)| w);
        out[1] += u32::from(doubling.iter().any(|(w, _)| Some(*w) == low));
        out[2] += u32::from(doubling.iter().any(|(w, _)| {
            (1..=4).any(|k| {
                piece.voices[*w]
                    .at(at + k * piece.meter.bar() / 4)
                    .is_none()
            })
        }));
        out[3] += u32::from(piece.voices[v].notes.first().is_some_and(|n| n.on == at));
    }
    out
}

/// Reads the moments of a piece.
pub fn count(piece: &Piece, marks: &Marks) -> Tally {
    let mut tally = Tally::default();
    let beat = piece.meter.beat();
    let bar = piece.meter.bar();
    let near = |a: Tick, b: Tick| (a - b).abs() <= beat;

    for (at, key) in &marks.cadences {
        let (at, key) = (*at, *key);
        let tones = sounding(piece, at);
        if tones.len() < 2 {
            continue;
        }
        tally.moments[0] += 1;
        let (Some((bass_voice, bass)), Some((top_voice, top))) = (lowest(&tones), highest(&tones))
        else {
            continue;
        };
        tally.note(Count::KadBass, degree(bass.pitch, key) == 0);
        tally.note(
            Count::KadBass5,
            matches!(way(piece, bass_voice, at), Some(Way::From(n)) if degree(n.pitch, key) == 7),
        );
        tally.note(Count::KadOben, degree(top.pitch, key) == 0);
        let top_way = way(piece, top_voice, at);
        let step = |n: Note| (1..=2).contains(&(n.pitch.midi - top.pitch.midi).abs());
        tally.note(
            Count::KadObenSchritt,
            matches!(top_way, Some(Way::From(n)) if step(n)),
        );
        tally.note(
            Count::KadObenSprung,
            match top_way {
                Some(Way::Rest) => true,
                Some(Way::From(n)) => (n.pitch.midi - top.pitch.midi).abs() >= 3,
                _ => false,
            },
        );
        tally.note(
            Count::KadTerz,
            tones
                .iter()
                .any(|(_, n)| matches!(degree(n.pitch, key), 3 | 4)),
        );
        tally.note(
            Count::KadLeitton,
            tones.iter().any(|(v, n)| {
                degree(n.pitch, key) == 0
                    && matches!(way(piece, *v, at), Some(Way::From(b)) if b.pitch.midi + 1 == n.pitch.midi)
            }),
        );
        tally.note(Count::KadDominante, dominant(piece, at - 1, key));
        tally.note(
            Count::KadBassDavor,
            lowest(&sounding(piece, at - 1)).is_some_and(|(_, n)| degree(n.pitch, key) == 7),
        );
        tally.note(Count::KadAlle, tones.len() == piece.voices.len());
        tally.note(
            Count::KadEinsatz,
            marks.entries.iter().any(|(_, t, _)| near(*t, at)),
        );
    }

    let first = marks.entries.iter().map(|e| e.1).min();
    let onsets = |from: Tick, until: Tick| -> usize {
        piece
            .voices
            .iter()
            .map(|v| v.range(from, until).len())
            .sum()
    };
    let bars = ((piece.end - piece.origin) as f64 / bar as f64).max(1.0);
    let mean = piece.note_count() as f64 / bars;
    for (v, at, end) in &marks.entries {
        let (v, at, end) = (*v, *at, *end);
        if Some(at) == first || v >= piece.voices.len() {
            continue;
        }
        let Some(tone) = piece.voices[v].at(at).copied().filter(|n| n.on == at) else {
            continue;
        };
        tally.moments[1] += 1;
        let tones = sounding(piece, at);
        let pause = rests_before(piece, v, at);
        let cadence = marks.arrivals.iter().any(|t| near(*t, at));
        tally.note(Count::EinPause, pause);
        tally.note(
            Count::EinErster,
            piece.voices[v].notes.first().is_some_and(|n| n.on == at),
        );
        tally.note(Count::EinAlle, tones.len() == piece.voices.len());
        let (low, high) = (lowest(&tones), highest(&tones));
        tally.note(
            Count::EinAussen,
            low.is_some_and(|(_, n)| n.pitch.midi == tone.pitch.midi)
                || high.is_some_and(|(_, n)| n.pitch.midi == tone.pitch.midi),
        );
        tally.note(
            Count::EinKonsonant,
            low.is_some_and(|(_, n)| {
                matches!(
                    (tone.pitch.midi - n.pitch.midi).rem_euclid(12),
                    0 | 3 | 4 | 7 | 8 | 9
                )
            }),
        );
        tally.note(
            Count::EinVerdoppelt,
            tones
                .iter()
                .any(|(w, n)| *w != v && n.on == at && n.pitch.pc() == tone.pitch.pc()),
        );
        tally.note(Count::EinKadenz, cadence);
        tally.note(
            Count::EinAbloesung,
            sounding(piece, at - 1)
                .iter()
                .any(|(w, _)| *w != v && piece.voices[*w].at(at).is_none()),
        );
        // The end of an episode: no subject in the bar before.
        if marks
            .subject
            .iter()
            .any(|(from, until)| *from < at && *until > at - bar)
        {
            continue;
        }
        tally.moments[2] += 1;
        let key = crate::scaffold::key_of(piece.voices[v].range(at, end), piece.key);
        let from = at - beat;
        tally.note(
            Count::ZwLeitton,
            piece.voices.iter().any(|voice| {
                voice
                    .notes
                    .iter()
                    .any(|n| n.on < at && n.off > from && degree(n.pitch, key) == 11)
            }),
        );
        tally.note(Count::ZwDominante, dominant(piece, at - 1, key));
        tally.note(
            Count::ZwBass,
            lowest(&sounding(piece, at - 1)).is_some_and(|(_, n)| degree(n.pitch, key) == 7)
                && low.is_some_and(|(_, n)| degree(n.pitch, key) == 0),
        );
        tally.note(Count::ZwKadenz, cadence);
        tally.note(Count::ZwPause, pause);
        tally.note(
            Count::ZwBewegung,
            onsets(at - bar, at) as f64 >= 0.75 * mean,
        );
    }
    tally
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::Voice;
    use crate::themes::parse;
    use crate::time::Meter;

    fn piece(key: &str, lines: &[&str]) -> Piece {
        let voices = lines
            .iter()
            .map(|line| Voice {
                notes: parse(line).unwrap(),
            })
            .collect();
        Piece::new("moments", Key::parse(key).unwrap(), Meter::COMMON, voices)
    }

    #[test]
    fn a_cadence_as_the_book_has_it() {
        // V7 to I in C: the bass falls a fifth, the top steps from the leading tone, the seventh
        // falls into the third.
        let piece = piece("C", &["d''2 b'2 | c''1", "g'2 f'2 | e'1", "g2 g2 | c1"]);
        let marks = Marks {
            cadences: vec![(96, Key::parse("C").unwrap())],
            arrivals: vec![96],
            ..Marks::default()
        };
        let tally = count(&piece, &marks);
        assert_eq!(tally.moments, [1, 0, 0]);
        for count in [
            Count::KadBass,
            Count::KadBass5,
            Count::KadOben,
            Count::KadObenSchritt,
            Count::KadTerz,
            Count::KadLeitton,
            Count::KadDominante,
            Count::KadBassDavor,
            Count::KadAlle,
        ] {
            assert_eq!(tally.share(count), Some(1.0), "{}", count.id());
        }
        assert_eq!(tally.share(Count::KadObenSprung), Some(0.0));
        assert_eq!(tally.share(Count::KadEinsatz), Some(0.0));
        // The same tones read as a cadence in G are none: the bass has the fourth.
        let marks = Marks {
            cadences: vec![(96, Key::parse("G").unwrap())],
            ..Marks::default()
        };
        let tally = count(&piece, &marks);
        assert_eq!(tally.share(Count::KadBass), Some(0.0));
        assert_eq!(tally.share(Count::KadDominante), Some(0.0));
    }

    #[test]
    fn an_entry_after_an_episode() {
        // The subject c' d' e' c' first in the upper voice, then, after two bars without it, in
        // the lower one, which rested: over a dominant with the bass on g, and the upper voice
        // falls silent.
        let piece = piece(
            "C",
            &[
                "c''4 d'' e'' c'' | g'1 | a'1 | g'2 b'2 | r1",
                "r1 | e'1 | f'1 | d'2 r2 | c'4 d' e' c'",
                "r1 | c1 | f1 | g1 | c1",
            ],
        );
        let marks = Marks {
            cadences: vec![(384, Key::parse("C").unwrap())],
            arrivals: vec![384],
            entries: vec![(0, 0, 96), (1, 384, 480)],
            subject: vec![(0, 96), (384, 480)],
        };
        let tally = count(&piece, &marks);
        // The first entry of the piece does not count.
        assert_eq!(tally.moments, [1, 1, 1]);
        assert_eq!(tally.share(Count::EinPause), Some(1.0));
        assert_eq!(tally.share(Count::EinErster), Some(0.0));
        assert_eq!(tally.share(Count::EinAlle), Some(0.0));
        assert_eq!(tally.share(Count::EinAussen), Some(1.0));
        assert_eq!(tally.share(Count::EinKonsonant), Some(1.0));
        assert_eq!(tally.share(Count::EinKadenz), Some(1.0));
        assert_eq!(tally.share(Count::EinAbloesung), Some(1.0));
        assert_eq!(tally.share(Count::ZwLeitton), Some(1.0));
        assert_eq!(tally.share(Count::ZwDominante), Some(1.0));
        assert_eq!(tally.share(Count::ZwBass), Some(1.0));
        assert_eq!(tally.share(Count::ZwPause), Some(1.0));
        // Four tones begin in the bar before, the piece has 3.8 to a bar.
        assert_eq!(tally.share(Count::ZwBewegung), Some(1.0));
        assert_eq!(tally.share(Count::KadEinsatz), Some(1.0));
    }

    #[test]
    fn findings() {
        // 35 of 40 against 0 of 47 is one; 12 of 40 against 11 of 47 is none; few moments none.
        assert!(differs((35, 40), (0, 47), 20.0));
        assert!(!differs((12, 40), (11, 47), 20.0));
        assert!(!differs((10, 10), (0, 47), 20.0));
        let (low, high) = wilson(0, 47);
        assert!(low < 1e-9 && high > 0.05 && high < 0.09, "{low} {high}");
    }
}
