//! Pitches with their spelling: step, alteration and octave.

use std::fmt;
use std::str::FromStr;

use crate::error::{ParseError, ParseErrorKind};
use crate::interval::Interval;
use crate::lang::Lang;

/// One of the seven diatonic steps (letter names).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Step {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Step {
    pub const ALL: [Step; 7] = [
        Step::C,
        Step::D,
        Step::E,
        Step::F,
        Step::G,
        Step::A,
        Step::B,
    ];

    /// Position in the octave, C = 0 … B = 6.
    pub const fn index(self) -> i32 {
        self as i32
    }

    /// The step `index` positions above C, wrapping around the octave.
    pub fn from_index(index: i32) -> Step {
        Self::ALL[index.rem_euclid(7) as usize]
    }

    /// Semitones of the natural step above C.
    pub const fn semitones(self) -> i32 {
        match self {
            Step::C => 0,
            Step::D => 2,
            Step::E => 4,
            Step::F => 5,
            Step::G => 7,
            Step::A => 9,
            Step::B => 11,
        }
    }

    /// Upper-case English letter.
    pub const fn letter(self) -> char {
        b"CDEFGAB"[self as usize] as char
    }

    /// Name of the natural step: German calls B natural H.
    pub fn name(self, lang: Lang) -> String {
        match (self, lang) {
            (Step::B, Lang::De) => "H".into(),
            _ => self.letter().to_string(),
        }
    }

    /// Step for an English letter (either case).
    pub fn from_letter(letter: char) -> Option<Step> {
        Self::ALL
            .into_iter()
            .find(|s| s.letter() == letter.to_ascii_uppercase())
    }
}

/// A pitch as written: step, alteration in semitones (−1 = ♭, +1 = ♯) and octave in scientific
/// pitch notation (C4 is middle C, the octave changes between B and C).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pitch {
    pub step: Step,
    pub alter: i8,
    pub octave: i8,
}

impl Pitch {
    pub const fn new(step: Step, alter: i8, octave: i8) -> Pitch {
        Pitch {
            step,
            alter,
            octave,
        }
    }

    pub const fn natural(step: Step, octave: i8) -> Pitch {
        Pitch::new(step, 0, octave)
    }

    /// Diatonic index: seven per octave, C0 = 0. Ignores the alteration.
    pub const fn diatonic(self) -> i32 {
        7 * self.octave as i32 + self.step.index()
    }

    /// MIDI key number, C4 = 60.
    pub const fn midi(self) -> i32 {
        12 * (self.octave as i32 + 1) + self.step.semitones() + self.alter as i32
    }

    /// The natural pitch with the given diatonic index.
    pub fn from_diatonic(index: i32) -> Pitch {
        Pitch::natural(Step::from_index(index), index.div_euclid(7) as i8)
    }

    pub const fn with_alter(self, alter: i8) -> Pitch {
        Pitch::new(self.step, alter, self.octave)
    }

    /// Interval from this pitch to `other` (negative if `other` is lower).
    pub fn interval_to(self, other: Pitch) -> Interval {
        Interval::between(self, other)
    }

    /// The pitch the given interval away, spelled by the interval's steps.
    pub fn transpose(self, interval: Interval) -> Pitch {
        let base = Pitch::from_diatonic(self.diatonic() + interval.steps);
        let alter = self.midi() + interval.semitones - base.midi();
        base.with_alter(alter as i8)
    }

    /// Same step and alteration, in any octave.
    pub const fn same_class(self, other: Pitch) -> bool {
        self.step as u8 == other.step as u8 && self.alter == other.alter
    }

    /// LilyPond absolute notation with Dutch note names, e.g. `cis''` for C♯5 and `bes` for B♭3.
    pub fn to_lilypond(self) -> String {
        let mut name = String::from(self.step.letter().to_ascii_lowercase());
        let flats_short = matches!(self.step, Step::E | Step::A);
        match self.alter {
            0 => {}
            a if a > 0 => name.push_str(&"is".repeat(a as usize)),
            a => {
                let suffix = "es".repeat(-a as usize);
                // Dutch spells E♭ and A♭ as "es" and "as".
                name.push_str(if flats_short { &suffix[1..] } else { &suffix });
            }
        }
        let octave = i32::from(self.octave);
        let marks = if octave >= 3 {
            "'".repeat((octave - 3) as usize)
        } else {
            ",".repeat((3 - octave) as usize)
        };
        name + &marks
    }

    /// Name without octave in the given language: English `C#`, `Bb`; German `Cis`, `B` (= B♭),
    /// `H`.
    pub fn class_name(self, lang: Lang) -> String {
        match lang {
            Lang::En => {
                let accidental = match self.alter {
                    a if a > 0 => "#".repeat(a as usize),
                    a => "b".repeat((-a) as usize),
                };
                format!("{}{accidental}", self.step.letter())
            }
            Lang::De => german_class(self.step, self.alter),
        }
    }

    /// Name with octave in the given language: English `C#4`, `Bb3`; German `Cis4`, `B3` (= B♭3),
    /// `H3`, `Es4`, `As4`.
    pub fn name(self, lang: Lang) -> String {
        match lang {
            Lang::En => self.to_string(),
            Lang::De => format!("{}{}", german_class(self.step, self.alter), self.octave),
        }
    }
}

fn german_class(step: Step, alter: i8) -> String {
    let flats = (-alter).max(0) as usize;
    let sharps = alter.max(0) as usize;
    match (step, flats) {
        (Step::B, 0) => format!("H{}", "is".repeat(sharps)),
        (Step::B, 1) => "B".into(),
        (Step::B, n) => format!("H{}", "es".repeat(n)),
        (Step::E | Step::A, n) if n > 0 => {
            format!("{}s{}", step.letter(), "es".repeat(n - 1))
        }
        (_, n) if n > 0 => format!("{}{}", step.letter(), "es".repeat(n)),
        _ => format!("{}{}", step.letter(), "is".repeat(sharps)),
    }
}

impl fmt::Display for Pitch {
    /// Scientific notation with ASCII accidentals: `C#4`, `Bb3`, `D4`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let accidental = match self.alter {
            a if a > 0 => "#".repeat(a as usize),
            a => "b".repeat(-a as usize),
        };
        write!(f, "{}{}{}", self.step.letter(), accidental, self.octave)
    }
}

impl FromStr for Pitch {
    type Err = ParseError;

    /// Parses scientific notation: a letter (`H` is accepted for B), accidentals `#`, `♯`, `x`
    /// (double sharp), `b`, `♭`, and an octave number. `Bb3` is B♭3, `C#5` is C♯5.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_scientific(s, 0)
    }
}

pub(crate) fn parse_scientific(token: &str, position: usize) -> Result<Pitch, ParseError> {
    let error = |kind| ParseError::new(kind, token, position);
    let mut chars = token.chars().peekable();
    let letter = chars.next().ok_or(error(ParseErrorKind::UnknownToken))?;
    let step = match letter {
        'H' | 'h' => Step::B,
        c => Step::from_letter(c).ok_or(error(ParseErrorKind::UnknownToken))?,
    };
    let mut alter: i8 = 0;
    while let Some(&c) = chars.peek() {
        match c {
            '#' | '♯' => alter += 1,
            'x' => alter += 2,
            'b' | '♭' => alter -= 1,
            _ => break,
        }
        chars.next();
    }
    let rest: String = chars.collect();
    if rest.is_empty() {
        return Err(error(ParseErrorKind::MissingOctave));
    }
    let octave: i8 = rest
        .parse()
        .map_err(|_| error(ParseErrorKind::UnknownToken))?;
    Ok(Pitch::new(step, alter, octave))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Pitch {
        s.parse().unwrap()
    }

    #[test]
    fn scientific_round_trip() {
        for s in ["C4", "C#5", "Bb3", "D4", "F##4", "Ebb2", "A0", "G9"] {
            assert_eq!(p(s).to_string(), s);
        }
        assert_eq!(p("H3"), Pitch::natural(Step::B, 3));
        assert_eq!(p("b♭3"), Pitch::new(Step::B, -1, 3));
        assert_eq!(p("Fx4"), Pitch::new(Step::F, 2, 4));
    }

    #[test]
    fn scientific_errors() {
        assert_eq!(
            "D".parse::<Pitch>().unwrap_err().kind,
            ParseErrorKind::MissingOctave
        );
        assert_eq!(
            "Q4".parse::<Pitch>().unwrap_err().kind,
            ParseErrorKind::UnknownToken
        );
    }

    #[test]
    fn midi_and_diatonic() {
        assert_eq!(p("C4").midi(), 60);
        assert_eq!(p("A4").midi(), 69);
        assert_eq!(p("B#3").midi(), 60);
        assert_eq!(p("Cb4").midi(), 59);
        assert_eq!(p("C4").diatonic() - p("B3").diatonic(), 1);
        assert_eq!(Pitch::from_diatonic(p("E2").diatonic()), p("E2"));
        assert_eq!(Pitch::from_diatonic(-1), p("B-1"));
    }

    #[test]
    fn lilypond_names() {
        assert_eq!(p("C4").to_lilypond(), "c'");
        assert_eq!(p("C#5").to_lilypond(), "cis''");
        assert_eq!(p("Bb3").to_lilypond(), "bes");
        assert_eq!(p("Eb4").to_lilypond(), "es'");
        assert_eq!(p("Ab2").to_lilypond(), "as,");
        assert_eq!(p("Ebb4").to_lilypond(), "eses'");
        assert_eq!(p("G#3").to_lilypond(), "gis");
        assert_eq!(p("E2").to_lilypond(), "e,");
    }

    #[test]
    fn german_names() {
        assert_eq!(p("B3").name(Lang::De), "H3");
        assert_eq!(p("Bb3").name(Lang::De), "B3");
        assert_eq!(p("C#5").name(Lang::De), "Cis5");
        assert_eq!(p("Eb4").name(Lang::De), "Es4");
        assert_eq!(p("Ab4").name(Lang::De), "As4");
        assert_eq!(p("Gb4").name(Lang::De), "Ges4");
        assert_eq!(p("Bbb3").name(Lang::De), "Heses3");
        assert_eq!(p("F#4").name(Lang::En), "F#4");
    }

    #[test]
    fn transpose_keeps_spelling() {
        let up_minor_third = Interval::new(2, 3);
        assert_eq!(p("E4").transpose(up_minor_third), p("G4"));
        assert_eq!(p("A4").transpose(up_minor_third), p("C5"));
        assert_eq!(p("B3").transpose(Interval::new(4, 6)), p("F4"));
        assert_eq!(p("F4").transpose(Interval::new(3, 6)), p("B4"));
        assert_eq!(p("D4").transpose(Interval::new(-1, -1)), p("C#4"));
    }
}
