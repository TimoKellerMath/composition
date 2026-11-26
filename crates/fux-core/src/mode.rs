//! The six church modes after Fux, authentic or plagal, and their transposition by one flat.

use std::fmt;
use std::str::FromStr;

use crate::error::{ParseError, ParseErrorKind};
use crate::interval::Interval;
use crate::lang::Lang;
use crate::pitch::{Pitch, Step};

/// How a clausula arrives on a step: the discant and the tenor meet there in contrary motion by
/// step, one of them by a semitone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Clausula {
    /// The discant rises by a semitone, raised by ficta where the scale has a whole step (C♯–D);
    /// the tenor falls by a step.
    Leading,
    /// On the step with a semitone above it (E, transposed A): the tenor falls by that semitone,
    /// the discant rises by a whole step.
    Phrygian,
}

/// The six modes Fux teaches. He rejects the mode on B (Locrian) for its diminished fifth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Aeolian,
    Ionian,
}

impl Mode {
    pub const ALL: [Mode; 6] = [
        Mode::Dorian,
        Mode::Phrygian,
        Mode::Lydian,
        Mode::Mixolydian,
        Mode::Aeolian,
        Mode::Ionian,
    ];

    /// Final of the untransposed mode.
    pub const fn final_step(self) -> Step {
        match self {
            Mode::Dorian => Step::D,
            Mode::Phrygian => Step::E,
            Mode::Lydian => Step::F,
            Mode::Mixolydian => Step::G,
            Mode::Aeolian => Step::A,
            Mode::Ionian => Step::C,
        }
    }

    /// The untransposed mode with the given final.
    pub fn from_final(step: Step) -> Option<Mode> {
        Self::ALL.into_iter().find(|m| m.final_step() == step)
    }

    pub fn name(self, lang: Lang) -> &'static str {
        match self {
            Mode::Dorian => lang.pick("dorisch", "Dorian"),
            Mode::Phrygian => lang.pick("phrygisch", "Phrygian"),
            Mode::Lydian => lang.pick("lydisch", "Lydian"),
            Mode::Mixolydian => lang.pick("mixolydisch", "Mixolydian"),
            Mode::Aeolian => lang.pick("äolisch", "Aeolian"),
            Mode::Ionian => lang.pick("ionisch", "Ionian"),
        }
    }

    /// Identifier used in files and on the command line: `dorian`, `phrygian`, …
    pub fn id(self) -> &'static str {
        match self {
            Mode::Dorian => "dorian",
            Mode::Phrygian => "phrygian",
            Mode::Lydian => "lydian",
            Mode::Mixolydian => "mixolydian",
            Mode::Aeolian => "aeolian",
            Mode::Ionian => "ionian",
        }
    }
}

/// Authentic modes span the octave above the final, plagal ones the octave from the fourth below
/// to the fifth above.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Ambitus {
    #[default]
    Authentic,
    Plagal,
}

/// A mode as used in a piece: authentic or plagal, untransposed or transposed up a fourth with one
/// flat in the signature (cantus mollis, e.g. Dorian on G).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModeSpec {
    pub mode: Mode,
    pub ambitus: Ambitus,
    pub transposed: bool,
}

impl ModeSpec {
    /// The authentic, untransposed mode.
    pub const fn new(mode: Mode) -> ModeSpec {
        ModeSpec {
            mode,
            ambitus: Ambitus::Authentic,
            transposed: false,
        }
    }

    pub const fn with_ambitus(self, ambitus: Ambitus) -> ModeSpec {
        ModeSpec { ambitus, ..self }
    }

    pub const fn with_transposition(self, transposed: bool) -> ModeSpec {
        ModeSpec { transposed, ..self }
    }

    /// Number of flats in the key signature: 0, or 1 when transposed.
    pub const fn flats(self) -> u8 {
        self.transposed as u8
    }

    /// Alteration of a step in the scale of this mode: B♭ when transposed, otherwise natural.
    pub const fn scale_alter(self, step: Step) -> i8 {
        if self.transposed && matches!(step, Step::B) {
            -1
        } else {
            0
        }
    }

    /// The scale pitch with the given diatonic index.
    pub fn scale_pitch(self, diatonic: i32) -> Pitch {
        let p = Pitch::from_diatonic(diatonic);
        p.with_alter(self.scale_alter(p.step))
    }

    /// Whether the pitch belongs to the scale (no ficta).
    pub const fn is_scale_pitch(self, pitch: Pitch) -> bool {
        pitch.alter == self.scale_alter(pitch.step)
    }

    /// Step of the final: transposition moves it up a fourth (D → G, F → B♭, …).
    pub fn final_step(self) -> Step {
        let step = self.mode.final_step();
        if self.transposed {
            Step::from_index(step.index() + 3)
        } else {
            step
        }
    }

    /// The final in the given octave, spelled in the scale.
    pub fn final_pitch(self, octave: i8) -> Pitch {
        let step = self.final_step();
        Pitch::new(step, self.scale_alter(step), octave)
    }

    /// Scale degree of a pitch: 1 = final, 2 = the step above, … 7.
    pub fn degree(self, pitch: Pitch) -> u8 {
        ((pitch.step.index() - self.final_step().index()).rem_euclid(7) + 1) as u8
    }

    /// Scale degree of the repercussa (reciting tone). Authentic modes recite on the fifth, the
    /// Phrygian on the sixth; the plagal modes a third or fourth above their final.
    pub const fn repercussa_degree(self) -> u8 {
        match (self.ambitus, self.mode) {
            (Ambitus::Authentic, Mode::Phrygian) => 6,
            (Ambitus::Authentic, _) => 5,
            (Ambitus::Plagal, Mode::Phrygian | Mode::Mixolydian) => 4,
            (Ambitus::Plagal, _) => 3,
        }
    }

    /// The repercussa nearest above the given final.
    pub fn repercussa(self, final_octave: i8) -> Pitch {
        let final_pitch = self.final_pitch(final_octave);
        self.scale_pitch(final_pitch.diatonic() + i32::from(self.repercussa_degree()) - 1)
    }

    /// Lowest and highest pitch of the ambitus for a final in the given octave: final to octave
    /// (authentic) or fourth below to fifth above the final (plagal).
    pub fn range(self, final_octave: i8) -> (Pitch, Pitch) {
        let f = self.final_pitch(final_octave).diatonic();
        let (low, high) = match self.ambitus {
            Ambitus::Authentic => (f, f + 7),
            Ambitus::Plagal => (f - 3, f + 4),
        };
        (self.scale_pitch(low), self.scale_pitch(high))
    }

    /// The subsemitonium modi: the pitch a semitone below the final, raised if necessary
    /// (C♯ in Dorian, F♯ in Mixolydian, G♯ in Aeolian). The Phrygian has none: its cadence
    /// approaches the final by a whole step from below.
    pub fn subsemitonium(self, final_octave: i8) -> Option<Pitch> {
        if self.mode == Mode::Phrygian {
            return None;
        }
        Some(
            self.final_pitch(final_octave)
                .transpose(-Interval::MINOR_SECOND),
        )
    }

    /// The clausula that can arrive on `target` in this scale, or `None` on the step whose fifth
    /// is diminished (B, transposed E): no cadence is possible there.
    pub fn clausula(self, target: Step) -> Option<Clausula> {
        let pitch = Pitch::new(target, self.scale_alter(target), 4);
        let fifth = self.scale_pitch(pitch.diatonic() + 4);
        if Interval::between(pitch, fifth).semitones != 7 {
            return None;
        }
        let above = self.scale_pitch(pitch.diatonic() + 1);
        Some(if Interval::between(pitch, above).semitones == 1 {
            Clausula::Phrygian
        } else {
            Clausula::Leading
        })
    }

    /// The note that leads into `target` by the semitone of its clausula: the leading tone below,
    /// raised if necessary (C♯ before D), or in a Phrygian clausula the semitone above (F before
    /// E).
    pub fn clausula_semitone(self, target: Pitch) -> Option<Pitch> {
        Some(match self.clausula(target.step)? {
            Clausula::Leading => target.transpose(-Interval::MINOR_SECOND),
            Clausula::Phrygian => target.transpose(Interval::MINOR_SECOND),
        })
    }

    /// The major third above the final, raised if necessary (F♯ in Dorian, G♯ in Phrygian).
    pub fn major_third(self, final_octave: i8) -> Pitch {
        self.final_pitch(final_octave)
            .transpose(Interval::MAJOR_THIRD)
    }

    /// The lowered step that avoids the tritone against F (B♭ in the untransposed scale,
    /// E♭ in the transposed one).
    pub fn flattened_mi(self) -> Step {
        if self.transposed { Step::E } else { Step::B }
    }

    /// Identifier for files and the command line: `dorian`, `hypodorian`, with `-transposed`
    /// appended for cantus mollis.
    pub fn id(self) -> String {
        let hypo = if self.ambitus == Ambitus::Plagal {
            "hypo"
        } else {
            ""
        };
        let transposed = if self.transposed { "-transposed" } else { "" };
        format!("{hypo}{}{transposed}", self.mode.id())
    }

    /// Readable name, e.g. „hypodorisch“ or "Dorian, transposed to G".
    pub fn name(self, lang: Lang) -> String {
        let base = self.mode.name(lang);
        let mut name = match (self.ambitus, lang) {
            (Ambitus::Authentic, _) => base.to_string(),
            (Ambitus::Plagal, Lang::De) => format!("hypo{base}"),
            (Ambitus::Plagal, Lang::En) => format!("Hypo{}", base.to_lowercase()),
        };
        if self.transposed {
            let final_name = self.final_pitch(4).name(lang);
            let final_name = final_name.trim_end_matches('4');
            name += &match lang {
                Lang::De => format!(", transponiert nach {final_name}"),
                Lang::En => format!(", transposed to {final_name}"),
            };
        }
        name
    }
}

impl FromStr for ModeSpec {
    type Err = ParseError;

    /// Accepts German and English names, with `hypo` for plagal modes and a suffix `-transposed`,
    /// `-transponiert` or `-mollis` for the transposition by one flat: `dorisch`, `Hypodorian`,
    /// `dorian-transposed`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let error = || ParseError::new(ParseErrorKind::UnknownMode, s, 0);
        let mut name = s.trim().to_lowercase();
        let mut transposed = false;
        for suffix in [
            "-transposed",
            "-transponiert",
            "-mollis",
            " transposed",
            " transponiert",
        ] {
            if let Some(stripped) = name.strip_suffix(suffix) {
                name = stripped.to_string();
                transposed = true;
            }
        }
        let (ambitus, base) = match name.strip_prefix("hypo") {
            Some(rest) => (Ambitus::Plagal, rest.trim_start_matches('-').to_string()),
            None => (Ambitus::Authentic, name),
        };
        let mode = Mode::ALL
            .into_iter()
            .find(|m| {
                base == m.id()
                    || base == m.name(Lang::De)
                    || (m == &Mode::Aeolian && base == "aeolisch")
            })
            .ok_or_else(error)?;
        Ok(ModeSpec {
            mode,
            ambitus,
            transposed,
        })
    }
}

impl fmt::Display for ModeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Pitch {
        s.parse().unwrap()
    }

    #[test]
    fn clausulae_on_every_step() {
        let plain = ModeSpec::new(Mode::Dorian);
        let semitone =
            |mode: ModeSpec, target: &str| mode.clausula_semitone(p(target)).map(|p| p.to_string());
        assert_eq!(semitone(plain, "D4").as_deref(), Some("C#4"));
        assert_eq!(semitone(plain, "A4").as_deref(), Some("G#4"));
        assert_eq!(semitone(plain, "F4").as_deref(), Some("E4"));
        assert_eq!(semitone(plain, "C4").as_deref(), Some("B3"));
        assert_eq!(plain.clausula(Step::E), Some(Clausula::Phrygian));
        assert_eq!(semitone(plain, "E4").as_deref(), Some("F4"));
        assert_eq!(plain.clausula(Step::B), None);
        // With one flat: A is the Phrygian step, E has no cadence, B♭ leads from A.
        let flat = plain.with_transposition(true);
        assert_eq!(flat.clausula(Step::A), Some(Clausula::Phrygian));
        assert_eq!(flat.clausula(Step::E), None);
        assert_eq!(semitone(flat, "Bb4").as_deref(), Some("A4"));
        assert_eq!(semitone(flat, "C5").as_deref(), Some("B4"));
        // The final's clausula agrees with the subsemitonium.
        for mode in Mode::ALL {
            let spec = ModeSpec::new(mode);
            let expected = spec.subsemitonium(4).map(|p| p.to_string());
            let found = match spec.clausula(spec.final_step()) {
                Some(Clausula::Leading) => semitone(spec, &spec.final_pitch(4).to_string()),
                _ => None,
            };
            assert_eq!(found, expected, "{mode:?}");
        }
    }

    #[test]
    fn finals() {
        let finals: Vec<String> = Mode::ALL
            .into_iter()
            .map(|m| ModeSpec::new(m).final_pitch(4).to_string())
            .collect();
        assert_eq!(finals, ["D4", "E4", "F4", "G4", "A4", "C4"]);
        let transposed: Vec<String> = Mode::ALL
            .into_iter()
            .map(|m| {
                ModeSpec::new(m)
                    .with_transposition(true)
                    .final_pitch(4)
                    .to_string()
            })
            .collect();
        assert_eq!(transposed, ["G4", "A4", "Bb4", "C4", "D4", "F4"]);
    }

    #[test]
    fn subsemitonium() {
        let sub = |m: Mode, t: bool| {
            ModeSpec::new(m)
                .with_transposition(t)
                .subsemitonium(4)
                .map(|p| p.to_string())
        };
        assert_eq!(sub(Mode::Dorian, false).as_deref(), Some("C#4"));
        assert_eq!(sub(Mode::Phrygian, false), None);
        assert_eq!(sub(Mode::Lydian, false).as_deref(), Some("E4"));
        assert_eq!(sub(Mode::Mixolydian, false).as_deref(), Some("F#4"));
        assert_eq!(sub(Mode::Aeolian, false).as_deref(), Some("G#4"));
        assert_eq!(sub(Mode::Ionian, false).as_deref(), Some("B3"));
        assert_eq!(sub(Mode::Dorian, true).as_deref(), Some("F#4"));
        assert_eq!(sub(Mode::Lydian, true).as_deref(), Some("A4"));
        assert_eq!(sub(Mode::Mixolydian, true).as_deref(), Some("B3"));
    }

    #[test]
    fn scale_and_degrees() {
        let dorian = ModeSpec::new(Mode::Dorian);
        assert!(dorian.is_scale_pitch(p("B3")));
        assert!(!dorian.is_scale_pitch(p("Bb3")));
        assert_eq!(dorian.degree(p("A4")), 5);
        assert_eq!(dorian.degree(p("C#4")), 7);
        let g_dorian = dorian.with_transposition(true);
        assert!(g_dorian.is_scale_pitch(p("Bb3")));
        assert_eq!(g_dorian.flats(), 1);
        assert_eq!(g_dorian.flattened_mi(), Step::E);
        assert_eq!(dorian.major_third(4), p("F#4"));
        assert_eq!(ModeSpec::new(Mode::Phrygian).major_third(4), p("G#4"));
    }

    #[test]
    fn ranges_and_repercussa() {
        let dorian = ModeSpec::new(Mode::Dorian);
        assert_eq!(dorian.range(4), (p("D4"), p("D5")));
        assert_eq!(
            dorian.with_ambitus(Ambitus::Plagal).range(4),
            (p("A3"), p("A4"))
        );
        assert_eq!(dorian.repercussa(4), p("A4"));
        assert_eq!(ModeSpec::new(Mode::Phrygian).repercussa(4), p("C5"));
        let hypodorian = dorian.with_ambitus(Ambitus::Plagal);
        assert_eq!(hypodorian.repercussa(4), p("F4"));
        let hypophrygian = ModeSpec::new(Mode::Phrygian).with_ambitus(Ambitus::Plagal);
        assert_eq!(hypophrygian.repercussa(4), p("A4"));
    }

    #[test]
    fn parse_and_names() {
        let parse = |s: &str| s.parse::<ModeSpec>().unwrap();
        assert_eq!(parse("dorisch"), ModeSpec::new(Mode::Dorian));
        assert_eq!(parse("Aeolian"), ModeSpec::new(Mode::Aeolian));
        assert_eq!(parse("äolisch"), ModeSpec::new(Mode::Aeolian));
        assert_eq!(
            parse("hypodorisch"),
            ModeSpec::new(Mode::Dorian).with_ambitus(Ambitus::Plagal)
        );
        assert_eq!(
            parse("dorian-transposed"),
            ModeSpec::new(Mode::Dorian).with_transposition(true)
        );
        assert!("locrian".parse::<ModeSpec>().is_err());
        assert_eq!(parse("hypolydian").name(Lang::En), "Hypolydian");
        assert_eq!(
            parse("dorisch-transponiert").name(Lang::De),
            "dorisch, transponiert nach G"
        );
        assert_eq!(
            parse("lydisch-mollis").name(Lang::De),
            "lydisch, transponiert nach B"
        );
        for spec in [parse("hypomixolydian-transposed"), parse("ionisch")] {
            assert_eq!(spec.id().parse::<ModeSpec>().unwrap(), spec);
        }
    }
}
