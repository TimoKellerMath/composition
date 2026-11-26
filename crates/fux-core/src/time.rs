//! Time in integer ticks: an eighth note is one tick, a whole note eight.

use std::fmt;

/// Ticks in a whole note. The eighth note is the shortest value of the species.
pub const TICKS_PER_WHOLE: u32 = 8;

/// A note value in ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Duration(u32);

impl Duration {
    pub const EIGHTH: Duration = Duration(1);
    pub const QUARTER: Duration = Duration(2);
    pub const DOTTED_QUARTER: Duration = Duration(3);
    pub const HALF: Duration = Duration(4);
    pub const DOTTED_HALF: Duration = Duration(6);
    pub const WHOLE: Duration = Duration(8);
    pub const DOTTED_WHOLE: Duration = Duration(12);
    pub const BREVE: Duration = Duration(16);

    pub const fn from_ticks(ticks: u32) -> Duration {
        Duration(ticks)
    }

    pub const fn ticks(self) -> u32 {
        self.0
    }

    /// The written value for LilyPond (`8`, `4.`, `2`, `1`, `\breve`), if there is a single one.
    pub const fn lilypond(self) -> Option<&'static str> {
        match self.0 {
            1 => Some("8"),
            2 => Some("4"),
            3 => Some("4."),
            4 => Some("2"),
            6 => Some("2."),
            8 => Some("1"),
            12 => Some("1."),
            16 => Some("\\breve"),
            24 => Some("\\breve."),
            _ => None,
        }
    }

    /// Splits a length into writable values from long to short, e.g. 5 ticks into 4 + 1.
    pub fn split(ticks: u32) -> Vec<Duration> {
        const VALUES: [u32; 9] = [24, 16, 12, 8, 6, 4, 3, 2, 1];
        let mut rest = ticks;
        let mut parts = Vec::new();
        while rest > 0 {
            let value = VALUES.into_iter().find(|&v| v <= rest).unwrap_or(1);
            parts.push(Duration(value));
            rest -= value;
        }
        parts
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.lilypond() {
            Some(name) => f.write_str(name),
            None => write!(f, "{} ticks", self.0),
        }
    }
}

/// Time signature. Fux's examples are in 2/2 with one cantus note per measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Meter {
    pub numerator: u32,
    pub denominator: u32,
}

impl Meter {
    pub const ALLA_BREVE: Meter = Meter {
        numerator: 2,
        denominator: 2,
    };

    pub const fn ticks_per_measure(self) -> u32 {
        self.numerator * TICKS_PER_WHOLE / self.denominator
    }

    /// Measure index (from 0) and tick within the measure.
    pub const fn locate(self, tick: u32) -> (usize, u32) {
        let per = self.ticks_per_measure();
        ((tick / per) as usize, tick % per)
    }
}

impl Default for Meter {
    fn default() -> Self {
        Meter::ALLA_BREVE
    }
}

impl fmt::Display for Meter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lilypond_values() {
        assert_eq!(Duration::DOTTED_HALF.lilypond(), Some("2."));
        assert_eq!(Duration::from_ticks(5).lilypond(), None);
        assert_eq!(Duration::split(5), [Duration::HALF, Duration::EIGHTH]);
        assert_eq!(Duration::split(8), [Duration::WHOLE]);
    }

    #[test]
    fn meters() {
        assert_eq!(Meter::ALLA_BREVE.ticks_per_measure(), 8);
        assert_eq!(
            Meter {
                numerator: 4,
                denominator: 2
            }
            .ticks_per_measure(),
            16
        );
        assert_eq!(Meter::ALLA_BREVE.locate(13), (1, 5));
    }
}
