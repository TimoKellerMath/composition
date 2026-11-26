//! The ten subjects of protocol T7. Those from the Well-Tempered Clavier are read from the
//! fugues themselves; the three others are typed in a small notation.

use anyhow::{Context, Result, bail};

use crate::key::Key;
use crate::pitch::Pitch;
use crate::score::{Note, Piece};
use crate::subject;
use crate::time::{Meter, TPW, Tick};

/// A subject with what a fugue on it needs to know.
#[derive(Clone, Debug)]
pub struct Subject {
    pub name: String,
    pub title: String,
    pub key: Key,
    pub meter: Meter,
    /// The tones; tick 0 is the barline before the first tone.
    pub notes: Vec<Note>,
    /// Ticks from one entry of the exposition to the next.
    pub period: Tick,
    /// The fugue of the corpus this subject comes from, if any (left out of every table).
    pub from: Option<String>,
    /// Whether the theme served the development of the prototype.
    pub development: bool,
}

/// Where a subject of the list comes from.
pub enum Source {
    Wtc(usize, usize),
    /// Key, meter, entry period in ticks, tones.
    Typed(&'static str, (u8, u8), Tick, &'static str),
}

pub struct Entry {
    pub name: &'static str,
    pub title: &'static str,
    pub source: Source,
    pub development: bool,
}

/// The list of the protocol, in its order.
pub const LIST: [Entry; 10] = [
    Entry {
        name: "wtk1-02",
        title: "WTK I/2, BWV 847, c-Moll",
        source: Source::Wtc(1, 2),
        development: true,
    },
    Entry {
        name: "wtk1-16",
        title: "WTK I/16, BWV 861, g-Moll",
        source: Source::Wtc(1, 16),
        development: true,
    },
    // Typed from memory for the study; checked on 2026-11-24 against the edition of Tobis
    // Notenarchiv (BWV_0578.xml): every tone agrees. The last tone is a sixteenth there, where
    // the line runs on; the subject ends on the third beat of its fifth bar.
    Entry {
        name: "bwv578",
        title: "Orgelfuge BWV 578, g-Moll",
        source: Source::Typed(
            "g",
            (4, 4),
            5 * 96,
            "g'4 d'' bes'4. a'8 | g' bes' a' g' fis' a' d'4 | g'8 d' a' d' bes' a'16 g' a'8 d' | \
             g' d'16 g' a'8 d'16 a' bes'8 a'16 g' a' d' d'' c'' | bes' a' g' bes' a' g' fis' a' g'8",
        ),
        development: false,
    },
    Entry {
        name: "bwv1080",
        title: "Kunst der Fuge, BWV 1080/1, d-Moll",
        source: Source::Typed(
            "d",
            (2, 2),
            4 * 96,
            "d'2 a' | f' d' | cis' d'4 e' | f'2~ f'8 g' f' e' | d'4",
        ),
        development: false,
    },
    Entry {
        name: "wtk2-02",
        title: "WTK II/2, BWV 871, c-Moll",
        source: Source::Wtc(2, 2),
        development: false,
    },
    Entry {
        name: "wtk1-21",
        title: "WTK I/21, BWV 866, B-Dur",
        source: Source::Wtc(1, 21),
        development: false,
    },
    Entry {
        name: "wtk1-01",
        title: "WTK I/1, BWV 846, C-Dur",
        source: Source::Wtc(1, 1),
        development: false,
    },
    Entry {
        name: "wtk2-20",
        title: "WTK II/20, BWV 889, a-Moll",
        source: Source::Wtc(2, 20),
        development: false,
    },
    Entry {
        name: "wtk1-07",
        title: "WTK I/7, BWV 852, Es-Dur",
        source: Source::Wtc(1, 7),
        development: false,
    },
    // After the LilyPond edition of N. Sceaux (Mutopia, CC BY 3.0, from Chrysander's edition):
    // the soprano of the chorus. Handel's answer enters after four bars, before the subject
    // ends; here the entries follow each other after six.
    Entry {
        name: "haendel",
        title: "Händel, „And with His stripes“ (Messiah), f-Moll",
        source: Source::Typed(
            "f",
            (2, 2),
            6 * 96,
            "r2 c''2 | as' des'' | e'1 | f'2. g'4 | as'2. bes'4 | c''1",
        ),
        development: false,
    },
];

/// Tones in a small LilyPond: Dutch names, absolute octaves, durations that carry on, `r` for
/// rests, `~` for ties, `|` ignored.
pub fn parse(text: &str) -> Result<Vec<Note>> {
    let mut notes: Vec<Note> = Vec::new();
    let mut at: Tick = 0;
    let mut ticks: Tick = TPW / 4;
    let mut tie = false;
    for token in text.split_whitespace() {
        if token == "|" {
            continue;
        }
        let body = token.trim_end_matches('~');
        let name: String = body
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let rest = &body[name.len()..];
        let marks: String = rest
            .chars()
            .take_while(|c| *c == '\'' || *c == ',')
            .collect();
        let value = &rest[marks.len()..];
        if !value.is_empty() {
            let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
            let dots = value.len() - digits.len();
            let number: Tick = digits
                .parse()
                .with_context(|| format!("duration of {token:?}"))?;
            if number == 0 || TPW % number != 0 || !value[digits.len()..].chars().all(|c| c == '.')
            {
                bail!("duration of {token:?}");
            }
            ticks = TPW / number;
            let mut added = ticks;
            for _ in 0..dots {
                added /= 2;
                ticks += added;
            }
        }
        if name == "r" {
            at += ticks;
            tie = false;
            continue;
        }
        let mut chars = name.chars();
        let letter = chars.next().with_context(|| format!("tone of {token:?}"))?;
        let step = "cdefgab"
            .find(letter)
            .with_context(|| format!("tone of {token:?}"))? as i8;
        let tail: String = chars.collect();
        let alter = match tail.as_str() {
            "" => 0,
            "is" => 1,
            "isis" => 2,
            "es" | "s" => -1,
            "eses" | "ses" => -2,
            _ => bail!("tone of {token:?}"),
        };
        let octave = 3 + marks
            .chars()
            .map(|c| if c == '\'' { 1 } else { -1 })
            .sum::<i8>();
        let pitch = Pitch::new(step, alter, octave);
        match notes.last_mut() {
            Some(last) if tie && last.pitch == pitch && last.off == at => last.off = at + ticks,
            _ => notes.push(Note {
                on: at,
                off: at + ticks,
                pitch,
            }),
        }
        at += ticks;
        tie = token.ends_with('~');
    }
    Ok(notes)
}

/// Tones in the notation that `parse` reads, with every duration written out.
pub fn text(notes: &[Note]) -> String {
    let value = |ticks: Tick| -> Option<String> {
        for dots in 0..3 {
            // A value with `dots` dots lasts (2 − 1/2^dots) times its plain value.
            let scaled = ticks * (1 << dots);
            let factor = (2 << dots) - 1;
            if scaled % factor == 0 {
                let plain = scaled / factor;
                if plain > 0 && TPW % plain == 0 {
                    return Some(format!("{}{}", TPW / plain, ".".repeat(dots)));
                }
            }
        }
        None
    };
    let mut out: Vec<String> = Vec::new();
    let mut at = 0;
    let gap = |from: Tick, until: Tick, out: &mut Vec<String>| {
        // Rests in values that exist, the longest first.
        let mut left = until - from;
        for size in [96, 48, 24, 12, 6, 3, 2, 1] {
            while left >= size {
                if let Some(v) = value(size) {
                    out.push(format!("r{v}"));
                }
                left -= size;
            }
        }
    };
    for note in notes {
        if note.on > at {
            gap(at, note.on, &mut out);
        }
        let name = note.pitch.lilypond();
        match value(note.dur()) {
            Some(v) => out.push(format!("{name}{v}")),
            None => {
                // A value that needs a tie: the longest writable part first.
                let mut left = note.dur();
                let mut parts = Vec::new();
                for size in [144, 96, 72, 48, 36, 24, 18, 12, 9, 6, 3, 2, 1] {
                    while left >= size {
                        if let Some(v) = value(size) {
                            parts.push(format!("{name}{v}"));
                            left -= size;
                        } else {
                            break;
                        }
                    }
                }
                out.push(parts.join("~ "));
            }
        }
        at = note.off;
    }
    out.join(" ")
}

/// The subject of an entry of the list; `pieces` are the 48 fugues, `data` their directory.
pub fn subject(entry: &Entry, pieces: &[Piece], data: &std::path::Path) -> Result<Subject> {
    match entry.source {
        Source::Typed(key, (num, den), period, text) => Ok(Subject {
            name: entry.name.to_string(),
            title: entry.title.to_string(),
            key: Key::parse(key).context("key")?,
            meter: Meter { num, den },
            notes: parse(text)?,
            period,
            from: None,
            development: entry.development,
        }),
        Source::Wtc(book, number) => {
            let name = format!("wtc{book}f{number:02}");
            let piece = pieces
                .iter()
                .find(|p| p.name == name)
                .with_context(|| format!("no piece {name}"))?;
            let labels = (book == 1)
                .then(|| crate::algomus::read_for(data, number).ok())
                .flatten();
            let exposition = subject::exposition(piece, labels.as_deref())
                .with_context(|| format!("no subject in {name}"))?;
            let first = exposition.subject.first().context("empty subject")?.on;
            let answer = exposition.answer.first().context("empty answer")?.on;
            // Tick 0 is the barline before the first tone.
            let bar = first - piece.pos(first);
            Ok(Subject {
                name: entry.name.to_string(),
                title: entry.title.to_string(),
                key: piece.key,
                meter: piece.meter,
                notes: exposition
                    .subject
                    .iter()
                    .map(|n| Note {
                        on: n.on - bar,
                        off: n.off - bar,
                        pitch: n.pitch,
                    })
                    .collect(),
                period: answer - first,
                from: Some(name),
                development: entry.development,
            })
        }
    }
}

/// A subject that comes with the program, typed out.
pub struct Builtin {
    pub name: &'static str,
    pub title: &'static str,
    pub key: &'static str,
    pub meter: (u8, u8),
    /// Ticks from one entry of the exposition to the next.
    pub period: Tick,
    pub text: &'static str,
}

/// The subjects of the study, as tones; the fugue of the corpus a subject comes from is named
/// after the pattern of the corpus, so that its tables are left out.
pub const BUILTIN: [Builtin; 20] = [
    Builtin {
        name: "wtk1-02",
        title: "WTK I/2, BWV 847, c-Moll",
        key: "c",
        meter: (4, 4),
        period: 192,
        text: "r8 c''16 b' c''8 g' as' c''16 b' c''8 d'' | g' c''16 b' c''8 d'' f'16 g' as'4 g'16 f' | es'16",
    },
    Builtin {
        name: "wtk1-16",
        title: "WTK I/16, BWV 861, g-Moll",
        key: "g",
        meter: (4, 4),
        period: 144,
        text: "r8 d'8 es' g fis4 g | r8 a16 bes c'8 bes16 a bes4",
    },
    Builtin {
        name: "bwv578",
        title: "Orgelfuge BWV 578, g-Moll",
        key: "g",
        meter: (4, 4),
        period: 480,
        text: "g'4 d'' bes'4. a'8 | g' bes' a' g' fis' a' d'4 | g'8 d' a' d' bes' a'16 g' a'8 d' | \
               g' d'16 g' a'8 d'16 a' bes'8 a'16 g' a' d' d'' c'' | bes' a' g' bes' a' g' fis' a' g'8",
    },
    Builtin {
        name: "bwv1080",
        title: "Kunst der Fuge, BWV 1080/1, d-Moll",
        key: "d",
        meter: (2, 2),
        period: 384,
        text: "d'2 a' | f' d' | cis' d'4 e' | f'2~ f'8 g' f' e' | d'4",
    },
    Builtin {
        name: "wtk2-02",
        title: "WTK II/2, BWV 871, c-Moll",
        key: "c",
        meter: (4, 4),
        period: 96,
        text: "r8 g'8 es' f' g' c' f' es'16 d' | es'4",
    },
    Builtin {
        name: "wtk1-21",
        title: "WTK I/21, BWV 866, B-Dur",
        key: "Bb",
        meter: (3, 4),
        period: 288,
        text: "r8 f'8 g' f' bes' d' | c' a'16 g' bes' a' g' f' c''8 es' | d' bes'16 c'' a' bes' c'' d'' es'' d'' c'' es'' | \
               d'' c'' bes' c'' a' bes' c'' d'' es'' d'' c'' es'' | d''8.",
    },
    Builtin {
        name: "wtk1-01",
        title: "WTK I/1, BWV 846, C-Dur",
        key: "C",
        meter: (4, 4),
        period: 144,
        text: "r8 c'8 d' e' f'8. g'32 f' e'8 a' | d' g'8. a'16 g' f' e'",
    },
    Builtin {
        name: "wtk2-20",
        title: "WTK II/20, BWV 889, a-Moll",
        key: "a",
        meter: (4, 4),
        period: 192,
        text: "r4 e'4 c' f' | gis r r8 d'8 b e' | c'",
    },
    Builtin {
        name: "wtk1-07",
        title: "WTK I/7, BWV 852, Es-Dur",
        key: "Eb",
        meter: (4, 4),
        period: 192,
        text: "bes'16 g' f' g' es' as' g' as' c''8 bes' r8 a'16 f' | es''8 d'' c''4 bes'16",
    },
    Builtin {
        name: "haendel",
        title: "Händel, „And with His stripes“ (Messiah), f-Moll",
        key: "f",
        meter: (2, 2),
        period: 576,
        text: "r2 c''2 | as' des'' | e'1 | f'2. g'4 | as'2. bes'4 | c''1",
    },
    // Ten more for phase B3, among them compound meters; as the first voice of the corpus
    // states them (`bach-probe b3-extract`).
    Builtin {
        name: "wtk2-01",
        title: "WTK II/1, BWV 870, C-Dur",
        key: "C",
        meter: (2, 4),
        period: 192,
        text: "r8 g'16 f' g'8 c' | a'4 g' | r8 f'16 e' f' g' e' f' | d' f' e' f' g' a' f' g' | e'16",
    },
    Builtin {
        name: "wtk2-05",
        title: "WTK II/5, BWV 874, D-Dur",
        key: "D",
        meter: (2, 2),
        period: 144,
        text: "r8 d'8 d' d' g4 b4. e8 a g fis4",
    },
    Builtin {
        name: "wtk2-12",
        title: "WTK II/12, BWV 881, f-Moll",
        key: "f",
        meter: (2, 4),
        period: 192,
        text: "r4 r8 c''8 | f' f' f' des''16 bes' | e'8 e' e'16 f' g' as' | bes' as' g' as' bes' des'' c'' bes' | as' g' f' g' as'16",
    },
    Builtin {
        name: "wtk2-16",
        title: "WTK II/16, BWV 885, g-Moll",
        key: "g",
        meter: (3, 4),
        period: 288,
        text: "r4 d'4 r8 bes8 | es'4 c' r8 a8 | d'4 bes r8 g8 | c'8 c' c' c' c' c' | c'8",
    },
    Builtin {
        name: "wtk2-18",
        title: "WTK II/18, BWV 887, gis-Moll",
        key: "g#",
        meter: (6, 8),
        period: 288,
        text: "gis'8 ais' b' ais' dis'' dis' | gis' b' ais' b' ais' gis' | ais' b' cis'' b' e'' gis' | ais' cis'' b' cis'' b' ais' | b'4",
    },
    Builtin {
        name: "wtk2-11",
        title: "WTK II/11, BWV 880, F-Dur",
        key: "F",
        meter: (6, 16),
        period: 144,
        text: "r8 r16 f'16 e' f' | c''8 r16 a'16 g' a' | d''8 r16 c''16 d'' e'' | f'' e'' d'' c'' d'' bes' | a' g' f'16",
    },
    Builtin {
        name: "wtk2-21",
        title: "WTK II/21, BWV 890, B-Dur",
        key: "Bb",
        meter: (3, 4),
        period: 288,
        text: "r8 c''8 bes' a' bes' f' | d' g' f' es' f' d' | bes es' es' d' d' c' | c' f' f' es' es' d' | d'4",
    },
    Builtin {
        name: "bwv1079",
        title: "Musikalisches Opfer, BWV 1079, Thema regium, c-Moll",
        key: "c",
        meter: (4, 4),
        period: 864,
        text: "c''2 es'' | g'' as'' | b' r4 g''4 | fis''2 f'' | e'' es''~ | es''4 d'' des'' c'' | b' a'8 g' c''4 f'' | es''2 d'' | c''4",
    },
    Builtin {
        name: "wtk1-15",
        title: "WTK I/15, BWV 860, G-Dur",
        key: "G",
        meter: (6, 8),
        period: 288,
        text: "g'8 a'16 g' fis' g' a'8 b'16 a' g' a' | b'8 a' g' d' c''4 | b'8 a' g' fis' e''4 | d''8 e''16 d'' c'' b' a'8 c''16 b' a' g' | fis'8",
    },
    Builtin {
        name: "wtk2-09",
        title: "WTK II/9, BWV 878, E-Dur",
        key: "E",
        meter: (4, 2),
        period: 288,
        text: "e1 fis2 a | gis fis e4",
    },
];

impl Builtin {
    pub fn subject(&self) -> Result<Subject> {
        // The fugues of the corpus are named wtc1f02; a subject from one of them leaves it out.
        let from = self.name.strip_prefix("wtk").and_then(|rest| {
            let (book, number) = rest.split_once('-')?;
            Some(format!("wtc{book}f{number}"))
        });
        Ok(Subject {
            name: self.name.to_string(),
            title: self.title.to_string(),
            key: Key::parse(self.key).context("key")?,
            meter: Meter {
                num: self.meter.0,
                den: self.meter.1,
            },
            notes: parse(self.text)?,
            period: self.period,
            from,
            development: false,
        })
    }
}

/// The period of the entries for a subject that names none: up to the bar in which its last tone
/// begins if that tone begins on a barline, else to the end of the bar in which it ends.
pub fn default_period(notes: &[Note], meter: Meter) -> Tick {
    let bar = meter.bar();
    let Some(last) = notes.last() else {
        return bar;
    };
    if last.on > 0 && last.on % bar == 0 {
        last.on
    } else {
        ((last.off + bar - 1) / bar).max(1) * bar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_subjects_parse() {
        let notes = parse("r2 c''2 | as' des'' | e'1 | f'2. g'4 | as'2. bes'4 | c''1").unwrap();
        assert_eq!(notes.len(), 9);
        assert_eq!(
            (notes[0].on, notes[0].off, notes[0].pitch.to_string()),
            (48, 96, "C5".into())
        );
        assert_eq!(notes[3].pitch.to_string(), "E4");
        assert_eq!(notes.last().unwrap().off, 576);
        let art = parse("d'2 a' | f' d' | cis' d'4 e' | f'2~ f'8 g' f' e' | d'4").unwrap();
        assert_eq!(art.len(), 12);
        assert_eq!((art[7].on, art[7].off), (288, 348));
        assert_eq!(art[4].pitch.to_string(), "C#4");
        for entry in &LIST {
            if let Source::Typed(_, _, _, text) = entry.source {
                assert!(!parse(text).unwrap().is_empty(), "{}", entry.name);
            }
        }
        // Writing and reading again gives the same tones.
        for source in [
            "d'2 a' | f' d' | cis' d'4 e' | f'2~ f'8 g' f' e' | d'4",
            "r8 c''16 b' c''8 g' as'4. bes'16 c''32 d''",
        ] {
            let notes = parse(source).unwrap();
            assert_eq!(parse(&text(&notes)).unwrap(), notes, "{}", text(&notes));
        }
        // The subject of BWV 578 fills four bars and two beats.
        if let Source::Typed(_, _, _, text) = LIST[2].source {
            assert_eq!(parse(text).unwrap().last().unwrap().off, 4 * 96 + 60);
        }
    }
}
