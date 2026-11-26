//! A small seeded generator (SplitMix64), so that every run can be repeated.

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }

    pub fn bits(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number below `n` (`n` > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.bits() % n
    }

    /// An index drawn with the given weights; `None` if all are zero.
    pub fn weighted(&mut self, weights: &[f64]) -> Option<usize> {
        let total: f64 = weights.iter().sum();
        if total <= 0.0 {
            return None;
        }
        let mut at = (self.bits() >> 11) as f64 / (1u64 << 53) as f64 * total;
        for (i, w) in weights.iter().enumerate() {
            if at < *w {
                return Some(i);
            }
            at -= w;
        }
        weights.iter().rposition(|w| *w > 0.0)
    }
}
