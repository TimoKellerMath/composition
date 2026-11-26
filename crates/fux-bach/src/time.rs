//! Time in ticks, 96 to the whole note: a thirty-second note is 3 ticks, a triplet sixteenth 4.

pub type Tick = i32;

/// Ticks in a whole note. The 48 fugues use no value shorter than a thirty-second and no tuplet
/// finer than triplet sixteenths.
pub const TPW: Tick = 96;
pub const SIXTEENTH: Tick = TPW / 16;
pub const EIGHTH: Tick = TPW / 8;
pub const QUARTER: Tick = TPW / 4;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Meter {
    pub num: u8,
    pub den: u8,
}

impl Meter {
    pub const COMMON: Meter = Meter { num: 4, den: 4 };

    pub const fn bar(self) -> Tick {
        self.num as Tick * TPW / self.den as Tick
    }

    /// 6, 9 or 12 in the numerator: the beat is the dotted value.
    pub const fn compound(self) -> bool {
        self.num.is_multiple_of(3) && self.num > 3
    }

    /// The notated beat.
    pub const fn beat(self) -> Tick {
        if self.compound() {
            3 * TPW / self.den as Tick
        } else {
            TPW / self.den as Tick
        }
    }

    /// The unit of the rhythm patterns: the quarter in simple meters, the dotted quarter in
    /// compound ones and in 3/8.
    pub const fn pulse(self) -> Tick {
        if self.compound() || (self.num == 3 && self.den == 8) {
            3 * TPW / 8
        } else {
            QUARTER
        }
    }

    /// The unit of the rhythm cells of the machine: as `pulse`, but the quarter where the beat
    /// is a dotted half (6/4), whose thirds are the quarters.
    pub const fn unit(self) -> Tick {
        if self.compound() && self.den <= 4 {
            QUARTER
        } else {
            self.pulse()
        }
    }

    /// Metric weight of a position inside the bar: 4 the bar start, 3 the middle of a bar with an
    /// even number of beats, 2 another beat, 1 the first subdivision of a beat (the half beat, in
    /// compound meters the thirds), 0 anything finer.
    pub const fn weight(self, pos: Tick) -> u8 {
        let beat = self.beat();
        let beats = self.bar() / beat;
        let sub = if self.compound() { beat / 3 } else { beat / 2 };
        if pos == 0 {
            4
        } else if beats % 2 == 0 && pos == self.bar() / 2 {
            3
        } else if pos % beat == 0 {
            2
        } else if pos % sub == 0 {
            1
        } else {
            0
        }
    }

    pub fn parse(text: &str) -> Option<Meter> {
        let (num, den) = text.split_once('/')?;
        let meter = Meter {
            num: num.trim().parse().ok()?,
            den: den.trim().parse().ok()?,
        };
        (meter.num > 0 && meter.den > 0 && TPW % meter.den as Tick == 0).then_some(meter)
    }
}

impl std::fmt::Display for Meter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beats_and_weights() {
        let common = Meter::COMMON;
        assert_eq!((common.bar(), common.beat(), common.pulse()), (96, 24, 24));
        assert_eq!(
            [0, 24, 48, 72, 12, 6].map(|p| common.weight(p)),
            [4, 2, 3, 2, 1, 0]
        );
        let nine = Meter { num: 9, den: 8 };
        assert_eq!((nine.bar(), nine.beat(), nine.pulse()), (108, 36, 36));
        assert_eq!([0, 36, 12, 6].map(|p| nine.weight(p)), [4, 2, 1, 0]);
        let alla = Meter { num: 2, den: 2 };
        assert_eq!((alla.beat(), alla.pulse()), (48, 24));
        assert_eq!(Meter::parse("12/16").map(Meter::beat), Some(18));
        assert_eq!(Meter { num: 3, den: 8 }.beat(), 12);
    }
}
