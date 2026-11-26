//! A small seeded random number generator (SplitMix64), so results are reproducible from a seed
//! on every platform without extra dependencies.

/// SplitMix64 by Sebastiano Vigna: fast, statistically good enough for tie-breaking noise.
#[derive(Clone, Debug)]
pub struct Rng(u64);

/// The step by which SplitMix64 advances its state, and the two odd factors of its mix.
const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
const MIX_1: u64 = 0xBF58_476D_1CE4_E5B9;
const MIX_2: u64 = 0x94D0_49BB_1331_11EB;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(GAMMA);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(MIX_1);
        z = (z ^ (z >> 27)).wrapping_mul(MIX_2);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// One step of SplitMix64 from `x`: a seed of its own for each of many draws that must not depend
/// on one another, such as the cadences of a chain by their index or the parts of a mass.
pub(crate) fn splitmix64(x: u64) -> u64 {
    Rng::new(x).next_u64()
}

/// The `x` whose step of SplitMix64 is `z` ([`splitmix64`]): every step of the mix is a bijection
/// of the 64-bit numbers and is undone in turn, the xor with a shift, the product with an odd
/// factor (by its inverse modulo 2⁶⁴) and the addition. So the seed of a part of a mass comes back
/// from the seed of its search and its attempt (`--rewrite`).
pub(crate) fn splitmix64_inverse(z: u64) -> u64 {
    let x = unshifted(z, 31).wrapping_mul(inverse(MIX_2));
    let x = unshifted(x, 27).wrapping_mul(inverse(MIX_1));
    unshifted(x, 30).wrapping_sub(GAMMA)
}

/// The `x` with `x ^ (x >> k) == y`: `y ^ (y >> k) ^ (y >> 2k) ^ …`, whose terms after the first
/// cancel those of the shift.
fn unshifted(y: u64, k: u32) -> u64 {
    let mut x = y;
    let mut shift = k;
    while shift < u64::BITS {
        x ^= y >> shift;
        shift += k;
    }
    x
}

/// The inverse of the odd `a` modulo 2⁶⁴, by Newton's step `x ↦ x (2 - a x)`: where `a x ≡ 1`
/// modulo 2ᵏ, the step gives it modulo 2²ᵏ, as `1 - a x (2 - a x) = (1 - a x)²`. From `x = a`,
/// with `a a ≡ 1` modulo 8, five steps give it modulo 2⁹⁶.
const fn inverse(a: u64) -> u64 {
    let mut x = a;
    let mut step = 0;
    while step < 5 {
        x = x.wrapping_mul(2u64.wrapping_sub(a.wrapping_mul(x)));
        step += 1;
    }
    x
}

/// A number for a text (FNV-1a, 64 bits): requests that differ in their text, mode or voices mix
/// different numbers into one seed (the planner of an imitation, the draws of a mass), a profile
/// is known again by its number (the parts of a mass), and a corrected text is noticed.
pub(crate) fn fingerprint(key: &str) -> u64 {
    key.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seed of a draw by its index is the first number of the generator from it, and nearby
    /// indices give unrelated seeds.
    #[test]
    fn splitmix64_is_one_step() {
        assert_eq!(splitmix64(7), Rng::new(7).next_u64());
        let seeds: Vec<u64> = (0..4).map(|i| splitmix64(42 ^ i)).collect();
        for (i, a) in seeds.iter().enumerate() {
            for b in &seeds[i + 1..] {
                assert!((a ^ b).count_ones() > 16, "{a:x} {b:x}");
            }
        }
    }

    /// A step of SplitMix64 is undone, for any number: its factors have their inverses, and the
    /// seed of a part comes back from the seed of an attempt at it.
    #[test]
    fn splitmix64_is_undone() {
        assert_eq!(MIX_1.wrapping_mul(inverse(MIX_1)), 1);
        assert_eq!(MIX_2.wrapping_mul(inverse(MIX_2)), 1);
        let mut rng = Rng::new(3);
        let numbers = [0, 1, 7, u64::MAX, GAMMA, 1 << 63].into_iter();
        for x in numbers.chain((0..1000).map(|_| rng.next_u64())) {
            assert_eq!(splitmix64_inverse(splitmix64(x)), x, "{x:x}");
        }
        let part = 8_233_720_775_215_435_644_u64;
        let attempt = splitmix64(part.wrapping_add(4));
        assert_eq!(splitmix64_inverse(attempt).wrapping_sub(4), part);
    }

    /// The fingerprint is FNV-1a: the offset basis for nothing, and the published value for
    /// `a`.
    #[test]
    fn fingerprint_is_fnv1a() {
        assert_eq!(fingerprint(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fingerprint("a"), 0xaf63_dc4c_8601_ec8c);
        assert_ne!(fingerprint("kyrie|dorian|4"), fingerprint("kyrie|dorian|3"));
    }

    #[test]
    fn reproducible_and_spread() {
        let a: Vec<u64> = (0..5)
            .scan(Rng::new(7), |r, _| Some(r.next_u64()))
            .collect();
        let b: Vec<u64> = (0..5)
            .scan(Rng::new(7), |r, _| Some(r.next_u64()))
            .collect();
        assert_eq!(a, b);
        let mut r = Rng::new(1);
        let mean = (0..10_000).map(|_| r.next_f64()).sum::<f64>() / 10_000.0;
        assert!((mean - 0.5).abs() < 0.02, "{mean}");
    }
}
