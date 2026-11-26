//! Spelled pitches and pitch classes.
//!
//! A pitch is the pair (MIDI key, diatonic index); spelled pitches form the group Z², so an
//! interval is a pair as well and transposition is addition. A spelled pitch class is a point on
//! the line of fifths (F = −1, C = 0, G = 1, …, F♯ = 6, B♭ = −2), which makes chords and scales
//! intervals of that line.

use std::fmt;

/// Semitones of the natural steps C D E F G A B above C.
const NATURAL: [i8; 7] = [0, 2, 4, 5, 7, 9, 11];
/// Line-of-fifths positions of the natural steps C D E F G A B.
const STEP_LOF: [i8; 7] = [0, 2, 4, -1, 1, 3, 5];

/// A pitch as written. `dia` counts diatonic steps (seven per octave, C0 = 0), `midi` semitones
/// (C4 = 60). The order is by sound first, so the minimum of a chord is its bass.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Pitch {
    pub midi: i16,
    pub dia: i16,
}

/// A directed interval: semitones and diatonic steps.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Iv {
    pub semis: i16,
    pub steps: i16,
}

/// A spelled pitch class on the line of fifths.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Lof(pub i8);

impl Pitch {
    /// `step` 0 = C … 6 = B, `alter` in semitones, `octave` scientific (C4 is middle C).
    pub const fn new(step: i8, alter: i8, octave: i8) -> Pitch {
        Pitch {
            midi: 12 * (octave as i16 + 1) + NATURAL[step as usize] as i16 + alter as i16,
            dia: 7 * octave as i16 + step as i16,
        }
    }

    pub const fn step(self) -> i8 {
        self.dia.rem_euclid(7) as i8
    }

    pub const fn octave(self) -> i8 {
        self.dia.div_euclid(7) as i8
    }

    pub const fn alter(self) -> i8 {
        (self.midi - (12 * (self.octave() as i16 + 1) + NATURAL[self.step() as usize] as i16)) as i8
    }

    /// Pitch class 0–11.
    pub const fn pc(self) -> u8 {
        self.midi.rem_euclid(12) as u8
    }

    pub const fn lof(self) -> Lof {
        Lof(STEP_LOF[self.step() as usize] + 7 * self.alter())
    }

    pub const fn to(self, other: Pitch) -> Iv {
        Iv {
            semis: other.midi - self.midi,
            steps: other.dia - self.dia,
        }
    }

    pub const fn up(self, iv: Iv) -> Pitch {
        Pitch {
            midi: self.midi + iv.semis,
            dia: self.dia + iv.steps,
        }
    }

    pub const fn down(self, iv: Iv) -> Pitch {
        Pitch {
            midi: self.midi - iv.semis,
            dia: self.dia - iv.steps,
        }
    }

    /// The pitch with this spelled class nearest to `near` (at most a tritone away by sound).
    pub fn nearest(class: Lof, near: Pitch) -> Pitch {
        let base = Pitch::new(class.step(), class.alter(), near.octave());
        [-1i16, 0, 1]
            .into_iter()
            .map(|o| Pitch {
                midi: base.midi + 12 * o,
                dia: base.dia + 7 * o,
            })
            .min_by_key(|p| ((p.midi - near.midi).abs(), p.midi))
            .unwrap_or(base)
    }

    /// LilyPond absolute notation with Dutch names: `cis''`, `bes`, `es'`.
    pub fn lilypond(self) -> String {
        let step = self.step();
        let mut name = String::from(b"cdefgab"[step as usize] as char);
        let alter = self.alter();
        for _ in 0..alter.abs() {
            if alter > 0 {
                name.push_str("is");
            } else if name.len() == 1 && (step == 2 || step == 5) {
                name.push('s');
            } else {
                name.push_str("es");
            }
        }
        let marks = self.octave() - 3;
        for _ in 0..marks.abs() {
            name.push(if marks > 0 { '\'' } else { ',' });
        }
        name
    }

    /// `**kern` pitch: `c` is C4, `cc` C5, `C` C3, `CC` C2; `#` and `-` for the alteration.
    pub fn kern(self) -> String {
        let step = self.step();
        let octave = self.octave();
        let (letter, count) = if octave >= 4 {
            (b"cdefgab"[step as usize] as char, octave - 3)
        } else {
            (b"CDEFGAB"[step as usize] as char, 4 - octave)
        };
        let mut out: String = std::iter::repeat_n(letter, count as usize).collect();
        let alter = self.alter();
        for _ in 0..alter.abs() {
            out.push(if alter > 0 { '#' } else { '-' });
        }
        out
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.lof(), self.octave())
    }
}

impl Iv {
    pub const UNISON: Iv = Iv::new(0, 0);
    pub const FIFTH: Iv = Iv::new(7, 4);
    pub const FOURTH: Iv = Iv::new(5, 3);
    pub const OCTAVE: Iv = Iv::new(12, 7);

    pub const fn new(semis: i16, steps: i16) -> Iv {
        Iv { semis, steps }
    }

    pub const fn neg(self) -> Iv {
        Iv::new(-self.semis, -self.steps)
    }

    /// −1, 0, 1 by sound.
    pub const fn dir(self) -> i8 {
        self.semis.signum() as i8
    }

    /// A second, also a chromatic semitone; not an augmented second.
    pub const fn is_step(self) -> bool {
        let s = self.semis.abs();
        (s == 1 || s == 2)
            && self.steps.abs() <= 1
            && (self.steps == 0 || self.steps.signum() == self.semis.signum())
    }

    /// Unison with the same spelling.
    pub const fn is_unison(self) -> bool {
        self.semis == 0 && self.steps == 0
    }

    /// Upwards and reduced to less than an octave: (steps mod 7, semitones mod 12).
    pub const fn simple(self) -> (i16, i16) {
        let up = if self.semis < 0 || (self.semis == 0 && self.steps < 0) {
            self.neg()
        } else {
            self
        };
        (up.steps.rem_euclid(7), up.semis.rem_euclid(12))
    }

    /// Unison, octave, perfect fifth, thirds and sixths. A perfect fourth counts as a consonance
    /// only if `fourth` is set (between upper voices).
    pub const fn is_consonant(self, fourth: bool) -> bool {
        matches!(
            self.simple(),
            (0, 0) | (4, 7) | (2, 3) | (2, 4) | (5, 8) | (5, 9)
        ) || (fourth && matches!(self.simple(), (3, 5)))
    }

    /// Unison, octave or perfect fifth, also compound.
    pub const fn is_perfect(self) -> bool {
        matches!(self.simple(), (0, 0) | (4, 7))
    }

    /// Augmented or diminished by spelling.
    pub const fn is_altered(self) -> bool {
        let (steps, semis) = self.simple();
        let base = NATURAL[steps as usize] as i16;
        let diff = semis - base;
        match steps {
            0 | 3 | 4 => diff != 0,
            _ => diff != 0 && diff != -1,
        }
    }
}

impl Lof {
    pub const fn step(self) -> i8 {
        (4 * self.0 as i32).rem_euclid(7) as i8
    }

    pub const fn alter(self) -> i8 {
        (self.0 + 1).div_euclid(7)
    }

    pub const fn pc(self) -> u8 {
        (7 * self.0 as i32).rem_euclid(12) as u8
    }

    pub const fn up(self, fifths: i8) -> Lof {
        Lof(self.0 + fifths)
    }

    /// From a name like `C`, `f#`, `Bb`, `e-`, `F##` (case is ignored).
    pub fn parse(name: &str) -> Option<Lof> {
        let mut chars = name.chars();
        let letter = chars.next()?.to_ascii_lowercase();
        let step = "cdefgab".find(letter)?;
        let mut alter = 0i8;
        for c in chars {
            match c {
                '#' => alter += 1,
                'b' | '-' => alter -= 1,
                _ => return None,
            }
        }
        Some(Lof(STEP_LOF[step] + 7 * alter))
    }

    /// The spelling of pitch class `pc` inside the window of twelve fifths starting at `low`.
    pub fn spell(pc: u8, low: i8) -> Lof {
        let base = (7 * pc as i32).rem_euclid(12) as i8;
        Lof(low + (base - low).rem_euclid(12))
    }
}

impl fmt::Display for Lof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let letter = b"CDEFGAB"[self.step() as usize] as char;
        let alter = self.alter();
        let sign = if alter > 0 { "#" } else { "b" };
        write!(f, "{letter}{}", sign.repeat(alter.unsigned_abs() as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spelling_round_trips() {
        let cis5 = Pitch::new(0, 1, 5);
        assert_eq!((cis5.midi, cis5.dia), (73, 35));
        assert_eq!((cis5.step(), cis5.alter(), cis5.octave()), (0, 1, 5));
        assert_eq!(cis5.lilypond(), "cis''");
        assert_eq!(cis5.kern(), "cc#");
        assert_eq!(Pitch::new(6, -1, 3).lilypond(), "bes");
        assert_eq!(Pitch::new(2, -1, 2).lilypond(), "es,");
        assert_eq!(Pitch::new(2, -1, 2).kern(), "EE-");
        assert_eq!(cis5.lof(), Lof(7));
        assert_eq!(Pitch::new(6, -1, 3).lof(), Lof(-2));
        for lof in -9..=12 {
            let l = Lof(lof);
            let p = Pitch::new(l.step(), l.alter(), 4);
            assert_eq!(p.lof(), l);
            assert_eq!(p.pc(), l.pc());
        }
    }

    #[test]
    fn intervals() {
        let c = Pitch::new(0, 0, 4);
        let fis = Pitch::new(3, 1, 4);
        let ges = Pitch::new(4, -1, 4);
        assert_eq!(c.to(fis).simple(), (3, 6));
        assert!(c.to(fis).is_altered() && c.to(ges).is_altered());
        assert!(!c.to(Pitch::new(4, 0, 4)).is_altered());
        assert!(c.to(Pitch::new(2, 0, 5)).is_consonant(false));
        assert!(!c.to(Pitch::new(3, 0, 4)).is_consonant(false));
        assert!(c.to(Pitch::new(3, 0, 4)).is_consonant(true));
        assert!(c.to(Pitch::new(0, 1, 4)).is_step());
        assert!(!Pitch::new(5, -1, 4).to(Pitch::new(6, 0, 4)).is_step());
        assert_eq!(c.up(Iv::FIFTH), Pitch::new(4, 0, 4));
        assert_eq!(Pitch::nearest(Lof(-2), c), Pitch::new(6, -1, 3));
    }

    #[test]
    fn names() {
        assert_eq!(Lof::parse("f#"), Some(Lof(6)));
        assert_eq!(Lof::parse("Eb"), Some(Lof(-3)));
        assert_eq!(Lof::parse("b-"), Some(Lof(-2)));
        assert_eq!(Lof(8).to_string(), "G#");
        assert_eq!(Lof::spell(1, -5), Lof(-5));
        assert_eq!(Lof::spell(1, 0), Lof(7));
    }
}
