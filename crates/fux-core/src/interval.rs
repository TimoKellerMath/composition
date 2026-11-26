//! Intervals with their quality, computed from spelled pitches.

use std::fmt;
use std::ops::{Add, Neg, Sub};

use crate::lang::Lang;
use crate::pitch::Pitch;

/// A directed interval: diatonic steps and semitones, both negative for a descending interval.
///
/// A major third up is `Interval { steps: 2, semitones: 4 }`, a diminished fifth down is
/// `Interval { steps: -4, semitones: -6 }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Interval {
    pub steps: i32,
    pub semitones: i32,
}

/// Quality of an interval. Augmented and diminished carry their degree (1 = simply augmented).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Quality {
    Perfect,
    Major,
    Minor,
    Augmented(u8),
    Diminished(u8),
}

/// Semitones of the perfect or major interval for each simple step count.
const BASE_SEMITONES: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

impl Interval {
    pub const UNISON: Interval = Interval::new(0, 0);
    pub const MINOR_SECOND: Interval = Interval::new(1, 1);
    pub const MAJOR_SECOND: Interval = Interval::new(1, 2);
    pub const MINOR_THIRD: Interval = Interval::new(2, 3);
    pub const MAJOR_THIRD: Interval = Interval::new(2, 4);
    pub const PERFECT_FOURTH: Interval = Interval::new(3, 5);
    pub const AUGMENTED_FOURTH: Interval = Interval::new(3, 6);
    pub const DIMINISHED_FIFTH: Interval = Interval::new(4, 6);
    pub const PERFECT_FIFTH: Interval = Interval::new(4, 7);
    pub const MINOR_SIXTH: Interval = Interval::new(5, 8);
    pub const MAJOR_SIXTH: Interval = Interval::new(5, 9);
    pub const MINOR_SEVENTH: Interval = Interval::new(6, 10);
    pub const MAJOR_SEVENTH: Interval = Interval::new(6, 11);
    pub const OCTAVE: Interval = Interval::new(7, 12);

    pub const fn new(steps: i32, semitones: i32) -> Interval {
        Interval { steps, semitones }
    }

    /// Interval from `from` to `to`.
    pub const fn between(from: Pitch, to: Pitch) -> Interval {
        Interval::new(to.diatonic() - from.diatonic(), to.midi() - from.midi())
    }

    /// −1, 0 or +1: direction by steps, by semitones for unisons.
    pub const fn direction(self) -> i32 {
        if self.steps != 0 {
            self.steps.signum()
        } else {
            self.semitones.signum()
        }
    }

    /// The same interval measured upwards.
    pub const fn abs(self) -> Interval {
        if self.direction() < 0 {
            Interval::new(-self.steps, -self.semitones)
        } else {
            self
        }
    }

    /// Generic number of the upward interval: 1 = unison, 8 = octave, 10 = tenth.
    pub const fn number(self) -> u32 {
        self.abs().steps as u32 + 1
    }

    /// Reduced by whole octaves to less than an octave, measured upwards; an octave reduces to a
    /// unison and a tenth to a third.
    pub const fn simple(self) -> Interval {
        let a = self.abs();
        let octaves = a.steps / 7;
        Interval::new(a.steps - 7 * octaves, a.semitones - 12 * octaves)
    }

    /// Larger than an octave.
    pub const fn is_compound(self) -> bool {
        self.abs().steps > 7
    }

    pub fn quality(self) -> Quality {
        let s = self.simple();
        let diff = s.semitones - BASE_SEMITONES[s.steps as usize];
        let perfect_kind = matches!(s.steps, 0 | 3 | 4);
        match (perfect_kind, diff) {
            (true, 0) => Quality::Perfect,
            (false, 0) => Quality::Major,
            (false, -1) => Quality::Minor,
            (_, d) if d > 0 => Quality::Augmented(d as u8),
            (true, d) => Quality::Diminished((-d) as u8),
            (false, d) => Quality::Diminished((-d - 1) as u8),
        }
    }

    /// Perfect, major or minor; not augmented or diminished.
    pub fn is_regular(self) -> bool {
        matches!(
            self.quality(),
            Quality::Perfect | Quality::Major | Quality::Minor
        )
    }

    /// Unison, octave or perfect fifth (also compound).
    pub const fn is_perfect_consonance(self) -> bool {
        let s = self.simple();
        matches!((s.steps, s.semitones), (0, 0) | (4, 7))
    }

    /// Third or sixth, major or minor (also compound).
    pub const fn is_imperfect_consonance(self) -> bool {
        let s = self.simple();
        matches!((s.steps, s.semitones), (2, 3) | (2, 4) | (5, 8) | (5, 9))
    }

    /// Consonant in two-part writing and against the bass, where the fourth is a dissonance.
    pub const fn is_consonant(self) -> bool {
        self.is_perfect_consonance() || self.is_imperfect_consonance()
    }

    pub const fn is_perfect_fourth(self) -> bool {
        let s = self.simple();
        s.steps == 3 && s.semitones == 5
    }

    /// Augmented fourth or diminished fifth (also compound).
    pub const fn is_tritone(self) -> bool {
        let s = self.simple();
        matches!((s.steps, s.semitones), (3, 6) | (4, 6))
    }

    /// A melodic second of any quality.
    pub const fn is_step(self) -> bool {
        self.abs().steps == 1
    }

    /// A melodic interval of a third or more.
    pub const fn is_leap(self) -> bool {
        self.abs().steps >= 2
    }

    /// Short name of the upward interval: `P5`, `m3`, `M6`, `A4`, `d5`, `P8`, `M10`.
    pub fn short_name(self) -> String {
        let prefix = match self.quality() {
            Quality::Perfect => "P".to_string(),
            Quality::Major => "M".to_string(),
            Quality::Minor => "m".to_string(),
            Quality::Augmented(n) => "A".repeat(n as usize),
            Quality::Diminished(n) => "d".repeat(n as usize),
        };
        format!("{prefix}{}", self.number())
    }

    /// Full name of the upward interval, e.g. „reine Quinte“, „kleine Dezime“, "minor tenth".
    pub fn name(self, lang: Lang) -> String {
        let quality = match (self.quality(), lang) {
            (Quality::Perfect, Lang::De) => "reine".to_string(),
            (Quality::Major, Lang::De) => "große".to_string(),
            (Quality::Minor, Lang::De) => "kleine".to_string(),
            (Quality::Augmented(1), Lang::De) => "übermäßige".to_string(),
            (Quality::Diminished(1), Lang::De) => "verminderte".to_string(),
            (Quality::Augmented(_), Lang::De) => "mehrfach übermäßige".to_string(),
            (Quality::Diminished(_), Lang::De) => "mehrfach verminderte".to_string(),
            (Quality::Perfect, Lang::En) => "perfect".to_string(),
            (Quality::Major, Lang::En) => "major".to_string(),
            (Quality::Minor, Lang::En) => "minor".to_string(),
            (Quality::Augmented(1), Lang::En) => "augmented".to_string(),
            (Quality::Diminished(1), Lang::En) => "diminished".to_string(),
            (Quality::Augmented(_), Lang::En) => "multiply augmented".to_string(),
            (Quality::Diminished(_), Lang::En) => "multiply diminished".to_string(),
        };
        format!("{quality} {}", number_name(self.number(), lang))
    }
}

fn number_name(number: u32, lang: Lang) -> String {
    const DE: [&str; 15] = [
        "Prime",
        "Sekunde",
        "Terz",
        "Quarte",
        "Quinte",
        "Sexte",
        "Septime",
        "Oktave",
        "None",
        "Dezime",
        "Undezime",
        "Duodezime",
        "Tredezime",
        "Quartdezime",
        "Doppeloktave",
    ];
    const EN: [&str; 15] = [
        "unison",
        "second",
        "third",
        "fourth",
        "fifth",
        "sixth",
        "seventh",
        "octave",
        "ninth",
        "tenth",
        "eleventh",
        "twelfth",
        "thirteenth",
        "fourteenth",
        "double octave",
    ];
    let names = match lang {
        Lang::De => &DE,
        Lang::En => &EN,
    };
    match names.get(number as usize - 1) {
        Some(name) => (*name).to_string(),
        None => match lang {
            Lang::De => format!("{number}. Stufe"),
            Lang::En => format!("{number}th"),
        },
    }
}

impl Neg for Interval {
    type Output = Interval;

    fn neg(self) -> Interval {
        Interval::new(-self.steps, -self.semitones)
    }
}

impl Add for Interval {
    type Output = Interval;

    fn add(self, other: Interval) -> Interval {
        Interval::new(self.steps + other.steps, self.semitones + other.semitones)
    }
}

impl Sub for Interval {
    type Output = Interval;

    fn sub(self, other: Interval) -> Interval {
        self + -other
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.direction() < 0 { "-" } else { "" };
        write!(f, "{sign}{}", self.short_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iv(from: &str, to: &str) -> Interval {
        Interval::between(from.parse().unwrap(), to.parse().unwrap())
    }

    #[test]
    fn qualities() {
        assert_eq!(iv("D4", "A4").short_name(), "P5");
        assert_eq!(iv("B3", "F4").short_name(), "d5");
        assert_eq!(iv("F4", "B4").short_name(), "A4");
        assert_eq!(iv("E4", "G4").short_name(), "m3");
        assert_eq!(iv("F4", "D5").short_name(), "M6");
        assert_eq!(iv("E4", "C#5").short_name(), "M6");
        assert_eq!(iv("F4", "G#4").short_name(), "A2");
        assert_eq!(iv("C4", "C#4").short_name(), "A1");
        assert_eq!(iv("C#4", "C5").short_name(), "d8");
        assert_eq!(iv("D3", "F4").short_name(), "m10");
        assert_eq!(iv("D4", "D5").short_name(), "P8");
        assert_eq!(iv("C4", "E##4").short_name(), "AA3");
    }

    #[test]
    fn direction_and_numbers() {
        let down = iv("A4", "D4");
        assert_eq!(down.direction(), -1);
        assert_eq!(down.abs(), Interval::PERFECT_FIFTH);
        assert_eq!(down.number(), 5);
        assert_eq!(down.to_string(), "-P5");
        assert_eq!(iv("C4", "E5").number(), 10);
        assert_eq!(iv("C4", "E5").simple(), Interval::MAJOR_THIRD);
        assert!(iv("C4", "E5").is_compound());
        assert!(!iv("C4", "C5").is_compound());
        assert_eq!(iv("C4", "C5").simple(), Interval::UNISON);
    }

    #[test]
    fn consonance_classes() {
        assert!(iv("D4", "D4").is_perfect_consonance());
        assert!(iv("D4", "A5").is_perfect_consonance());
        assert!(iv("D4", "F5").is_imperfect_consonance());
        assert!(!iv("D4", "G4").is_consonant());
        assert!(iv("D4", "G4").is_perfect_fourth());
        assert!(!iv("B3", "F4").is_consonant());
        assert!(iv("B3", "F4").is_tritone());
        assert!(iv("F3", "B4").is_tritone());
        assert!(!iv("C4", "G#4").is_consonant());
        assert!(!iv("C4", "D4").is_consonant());
    }

    #[test]
    fn steps_and_leaps() {
        assert!(iv("E4", "F4").is_step());
        assert!(iv("F4", "E4").is_step());
        assert!(iv("C4", "C#4").abs().steps == 0);
        assert!(iv("C4", "E4").is_leap());
        assert!(!iv("C4", "D4").is_leap());
    }

    #[test]
    fn names() {
        assert_eq!(Interval::PERFECT_FIFTH.name(Lang::De), "reine Quinte");
        assert_eq!(iv("D3", "F4").name(Lang::De), "kleine Dezime");
        assert_eq!(
            Interval::AUGMENTED_FOURTH.name(Lang::En),
            "augmented fourth"
        );
        assert_eq!(Interval::UNISON.name(Lang::De), "reine Prime");
        assert_eq!(iv("C3", "C5").name(Lang::En), "perfect double octave");
    }

    #[test]
    fn arithmetic() {
        assert_eq!(
            Interval::MAJOR_THIRD + Interval::MINOR_THIRD,
            Interval::PERFECT_FIFTH
        );
        assert_eq!(
            Interval::OCTAVE - Interval::PERFECT_FIFTH,
            Interval::PERFECT_FOURTH
        );
    }
}
