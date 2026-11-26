//! Clefs and the voice types that go with them.
//!
//! Fux writes each voice in its own clef and keeps it within the five lines: "One should not
//! exceed the limits of the five lines without grave necessity" (Mann, p. 79). The ranges of the
//! voice types are therefore derived from their clefs.

use std::fmt;
use std::str::FromStr;

use crate::error::{ParseError, ParseErrorKind};
use crate::lang::Lang;
use crate::pitch::{Pitch, Step};

/// Clefs named by sign and staff line: C1 is the soprano clef, F4 the bass clef.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Clef {
    G2,
    C1,
    C2,
    C3,
    C4,
    F3,
    F4,
}

impl Clef {
    pub const ALL: [Clef; 7] = [
        Clef::G2,
        Clef::C1,
        Clef::C2,
        Clef::C3,
        Clef::C4,
        Clef::F3,
        Clef::F4,
    ];

    /// The pitch on the bottom line of the staff.
    pub const fn bottom_line(self) -> Pitch {
        match self {
            Clef::G2 => Pitch::natural(Step::E, 4),
            Clef::C1 => Pitch::natural(Step::C, 4),
            Clef::C2 => Pitch::natural(Step::A, 3),
            Clef::C3 => Pitch::natural(Step::F, 3),
            Clef::C4 => Pitch::natural(Step::D, 3),
            Clef::F3 => Pitch::natural(Step::B, 2),
            Clef::F4 => Pitch::natural(Step::G, 2),
        }
    }

    /// The pitch on the top line of the staff, eight steps above the bottom line.
    pub fn top_line(self) -> Pitch {
        Pitch::from_diatonic(self.bottom_line().diatonic() + 8)
    }

    /// How many steps a pitch lies outside the five lines: 0 on or between them, positive above,
    /// negative below. The space directly above or below the staff counts as one step outside.
    pub fn steps_outside(self, pitch: Pitch) -> i32 {
        let d = pitch.diatonic();
        let bottom = self.bottom_line().diatonic();
        if d < bottom {
            d - bottom
        } else {
            (d - (bottom + 8)).max(0)
        }
    }

    /// LilyPond name of the original clef.
    pub const fn lilypond(self) -> &'static str {
        match self {
            Clef::G2 => "treble",
            Clef::C1 => "soprano",
            Clef::C2 => "mezzosoprano",
            Clef::C3 => "alto",
            Clef::C4 => "tenor",
            Clef::F3 => "varbaritone",
            Clef::F4 => "bass",
        }
    }

    /// LilyPond name of the modern clef for the same voice: treble, octave treble or bass.
    pub const fn modern_lilypond(self) -> &'static str {
        match self {
            Clef::G2 | Clef::C1 | Clef::C2 | Clef::C3 => "treble",
            Clef::C4 => "treble_8",
            Clef::F3 | Clef::F4 => "bass",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Clef::G2 => "G2",
            Clef::C1 => "C1",
            Clef::C2 => "C2",
            Clef::C3 => "C3",
            Clef::C4 => "C4",
            Clef::F3 => "F3",
            Clef::F4 => "F4",
        }
    }
}

impl FromStr for Clef {
    type Err = ParseError;

    /// Accepts `G2`, `C1` … `F4` and the LilyPond names (`treble`, `soprano`, `alto`, …).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim();
        Clef::ALL
            .into_iter()
            .find(|c| c.name().eq_ignore_ascii_case(t) || c.lilypond().eq_ignore_ascii_case(t))
            .ok_or_else(|| ParseError::new(ParseErrorKind::UnknownClef, s, 0))
    }
}

impl fmt::Display for Clef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Voice types, each with its clef and the range that follows from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VoiceType {
    HighSoprano,
    Soprano,
    MezzoSoprano,
    Alto,
    Tenor,
    Baritone,
    Bass,
}

impl VoiceType {
    /// From high to low.
    pub const ALL: [VoiceType; 7] = [
        VoiceType::HighSoprano,
        VoiceType::Soprano,
        VoiceType::MezzoSoprano,
        VoiceType::Alto,
        VoiceType::Tenor,
        VoiceType::Baritone,
        VoiceType::Bass,
    ];

    pub const fn clef(self) -> Clef {
        match self {
            VoiceType::HighSoprano => Clef::G2,
            VoiceType::Soprano => Clef::C1,
            VoiceType::MezzoSoprano => Clef::C2,
            VoiceType::Alto => Clef::C3,
            VoiceType::Tenor => Clef::C4,
            VoiceType::Baritone => Clef::F3,
            VoiceType::Bass => Clef::F4,
        }
    }

    pub const fn from_clef(clef: Clef) -> VoiceType {
        match clef {
            Clef::G2 => VoiceType::HighSoprano,
            Clef::C1 => VoiceType::Soprano,
            Clef::C2 => VoiceType::MezzoSoprano,
            Clef::C3 => VoiceType::Alto,
            Clef::C4 => VoiceType::Tenor,
            Clef::F3 => VoiceType::Baritone,
            Clef::F4 => VoiceType::Bass,
        }
    }

    pub fn name(self, lang: Lang) -> &'static str {
        match self {
            VoiceType::HighSoprano => lang.pick("Hoher Sopran", "High soprano"),
            VoiceType::Soprano => lang.pick("Sopran", "Soprano"),
            VoiceType::MezzoSoprano => lang.pick("Mezzosopran", "Mezzo-soprano"),
            VoiceType::Alto => lang.pick("Alt", "Alto"),
            VoiceType::Tenor => "Tenor",
            VoiceType::Baritone => lang.pick("Bariton", "Baritone"),
            VoiceType::Bass => "Bass",
        }
    }

    /// Short label: `S+`, `S`, `Mz`, `A`, `T`, `Bar`, `B`.
    pub const fn abbreviation(self) -> &'static str {
        match self {
            VoiceType::HighSoprano => "S+",
            VoiceType::Soprano => "S",
            VoiceType::MezzoSoprano => "Mz",
            VoiceType::Alto => "A",
            VoiceType::Tenor => "T",
            VoiceType::Baritone => "Bar",
            VoiceType::Bass => "B",
        }
    }

    /// The range without cost: the staff of the clef plus one step above and below.
    pub fn free_range(self) -> (Pitch, Pitch) {
        let clef = self.clef();
        (
            Pitch::from_diatonic(clef.bottom_line().diatonic() - 1),
            Pitch::from_diatonic(clef.top_line().diatonic() + 1),
        )
    }

    /// The usual voice type directly above or below, for a counterpoint against a cantus in this
    /// voice: as in Fux, alto against soprano above and tenor below, and so on.
    pub const fn neighbor(self, above: bool) -> Option<VoiceType> {
        use VoiceType::*;
        match (self, above) {
            (HighSoprano, true) => None,
            (HighSoprano, false) => Some(Soprano),
            (Soprano, true) => Some(HighSoprano),
            (Soprano | MezzoSoprano, false) => Some(Alto),
            (MezzoSoprano | Alto, true) => Some(Soprano),
            (Alto, false) => Some(Tenor),
            (Tenor | Baritone, true) => Some(Alto),
            (Tenor | Baritone, false) => Some(Bass),
            (Bass, true) => Some(Tenor),
            (Bass, false) => None,
        }
    }

    /// Total steps by which the pitches lie outside the free range.
    pub fn misfit(self, pitches: &[Pitch]) -> i32 {
        let (low, high) = self.free_range();
        pitches
            .iter()
            .map(|p| {
                let d = p.diatonic();
                (low.diatonic() - d).max(0) + (d - high.diatonic()).max(0)
            })
            .sum()
    }

    /// Soprano, alto, tenor or bass; the other types are for five and six voices.
    pub const fn is_standard(self) -> bool {
        matches!(
            self,
            VoiceType::Soprano | VoiceType::Alto | VoiceType::Tenor | VoiceType::Bass
        )
    }

    /// The voice type whose range fits the pitches best. On a tie a standard type wins, then the
    /// lower voice, since a cantus firmus usually lies in a middle or lower voice.
    pub fn best_for(pitches: &[Pitch]) -> VoiceType {
        Self::ALL
            .into_iter()
            .rev()
            .min_by_key(|v| (v.misfit(pitches), !v.is_standard()))
            .unwrap_or(VoiceType::Alto)
    }

    /// Whole octaves (from −3 to +3) by which to move the pitches so they fit this voice best;
    /// on a tie the smallest move.
    pub fn octave_shift_for(self, pitches: &[Pitch]) -> i32 {
        (-3..=3)
            .min_by_key(|&k: &i32| {
                let moved: Vec<Pitch> = pitches
                    .iter()
                    .map(|p| Pitch::new(p.step, p.alter, p.octave + k as i8))
                    .collect();
                (self.misfit(&moved), k.abs())
            })
            .unwrap_or(0)
    }
}

impl FromStr for VoiceType {
    type Err = ParseError;

    /// Accepts abbreviations (`S`, `A`, `T`, `B`, `Mz`, `Bar`, `S+`), German and English names,
    /// and clef names (`C1` is a soprano).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim();
        if let Ok(clef) = t.parse::<Clef>() {
            return Ok(VoiceType::from_clef(clef));
        }
        let lower = t.to_lowercase();
        VoiceType::ALL
            .into_iter()
            .find(|v| {
                v.abbreviation().eq_ignore_ascii_case(t)
                    || v.name(Lang::De).to_lowercase() == lower
                    || v.name(Lang::En).to_lowercase() == lower
                    || (*v == VoiceType::MezzoSoprano && lower == "ms")
            })
            .ok_or_else(|| ParseError::new(ParseErrorKind::UnknownVoiceType, s, 0))
    }
}

impl fmt::Display for VoiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.abbreviation())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Pitch {
        s.parse().unwrap()
    }

    #[test]
    fn staff_lines() {
        assert_eq!(Clef::C1.top_line(), p("D5"));
        assert_eq!(Clef::C3.top_line(), p("G4"));
        assert_eq!(Clef::F4.top_line(), p("A3"));
        assert_eq!(Clef::G2.top_line(), p("F5"));
        assert_eq!(Clef::F3.top_line(), p("C4"));
    }

    #[test]
    fn steps_outside_the_staff() {
        assert_eq!(Clef::C1.steps_outside(p("C4")), 0);
        assert_eq!(Clef::C1.steps_outside(p("D5")), 0);
        assert_eq!(Clef::C1.steps_outside(p("E5")), 1);
        assert_eq!(Clef::C1.steps_outside(p("F5")), 2);
        assert_eq!(Clef::C1.steps_outside(p("B3")), -1);
        assert_eq!(Clef::C4.steps_outside(p("A2")), -3);
        assert_eq!(Clef::C4.steps_outside(p("C#4")), 0);
    }

    #[test]
    fn free_ranges() {
        let range = |v: VoiceType| {
            let (lo, hi) = v.free_range();
            format!("{lo}-{hi}")
        };
        assert_eq!(range(VoiceType::Soprano), "B3-E5");
        assert_eq!(range(VoiceType::Alto), "E3-A4");
        assert_eq!(range(VoiceType::Tenor), "C3-F4");
        assert_eq!(range(VoiceType::Bass), "F2-B3");
    }

    #[test]
    fn fitting_voices() {
        let cf = |s: &str| crate::parse::parse_pitches(s).unwrap();
        // Fux's Dorian cantus fits the alto, the Lydian one (F3-C4) the tenor.
        assert_eq!(
            VoiceType::best_for(&cf("D4 F4 E4 D4 G4 F4 A4 G4 F4 E4 D4")),
            VoiceType::Alto
        );
        assert_eq!(
            VoiceType::best_for(&cf("F3 G3 A3 F3 D3 E3 F3 C4 A3 F3 G3 F3")),
            VoiceType::Tenor
        );
        let dorian = cf("D4 F4 E4 D4 G4 F4 A4 G4 F4 E4 D4");
        assert_eq!(VoiceType::Bass.octave_shift_for(&dorian), -1);
        assert_eq!(VoiceType::Alto.octave_shift_for(&dorian), 0);
        assert_eq!(VoiceType::Soprano.octave_shift_for(&dorian), 0);
    }

    #[test]
    fn parsing() {
        assert_eq!("C3".parse::<Clef>().unwrap(), Clef::C3);
        assert_eq!("varbaritone".parse::<Clef>().unwrap(), Clef::F3);
        assert_eq!("S".parse::<VoiceType>().unwrap(), VoiceType::Soprano);
        assert_eq!("Alt".parse::<VoiceType>().unwrap(), VoiceType::Alto);
        assert_eq!("tenor".parse::<VoiceType>().unwrap(), VoiceType::Tenor);
        assert_eq!("Bar".parse::<VoiceType>().unwrap(), VoiceType::Baritone);
        assert_eq!("F4".parse::<VoiceType>().unwrap(), VoiceType::Bass);
        assert_eq!("ms".parse::<VoiceType>().unwrap(), VoiceType::MezzoSoprano);
        assert!("X".parse::<VoiceType>().is_err());
        for v in VoiceType::ALL {
            assert_eq!(VoiceType::from_clef(v.clef()), v);
            assert_eq!(v.abbreviation().parse::<VoiceType>().unwrap(), v);
        }
    }
}
