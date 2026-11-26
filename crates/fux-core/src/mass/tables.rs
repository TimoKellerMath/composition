//! The data of a mass plan: the scoring of the sections, the principal degree beside the final,
//! the shares of the part endings and of the internal cadences, the restarts, the lengths and
//! the estimates (docs/design/phase-14/spec.md §§ 3.2–3.10). The shares come from the counts in
//! Palestrina's masses (`research/research_structure.json` and `research_cadences.json`, with
//! the corrections of their verifications); the plagal modes take the tables of their authentic
//! counterparts. The measurements of phase 14a (WP15) decided the size of the parts, the lengths,
//! the weights of the chains and the estimates; values marked provisional still wait for counts.

use std::ops::RangeInclusive;

use super::Preset;
use super::preset::Scoring;
use crate::clef::VoiceType;
use crate::generate::part_endings;
use crate::mode::{Mode, ModeSpec};
use crate::pitch::Step;

/// The most syllables of a part of a long section (D3): the measured size. Patrem set whole
/// took 756 s, in four parts 61 s. At 55 (Gloria and Credo 11 instead of 16 joins) two parts of
/// the Dorian Gloria and Credo would take more than four minutes in WebAssembly (about 55 s
/// natively, × 5.3), none at 45, and the two movements took 13 % longer (the first measurements
/// of phase 14a), so WP15 kept 45.
pub const CHUNK: usize = 45;

/// The sizes of parts a request may ask for.
pub const CHUNK_RANGE: RangeInclusive<usize> = 20..=200;

/// How many points of imitation the Benedictus and Agnus Dei I get beyond those the imitation
/// plans by itself, restating their short texts (D10, MASS-LENGTH): a generated Benedictus a 3
/// has 25–35 measures against Palestrina's median of 67 in four-voice masses, an Agnus Dei I
/// 35–42 against 78. With them the Benedictus came out with 38–56 measures, Agnus Dei I with
/// 54–71, every one on its first seed, at about 6 % of a mass's time (the first measurements of
/// phase 14a; WP15 kept `--lengths palestrina` as the default).
pub const EXTRA_POINTS: usize = 2;

/// The sections that get [`EXTRA_POINTS`] with `--lengths palestrina`.
pub const LENGTHENED: [&str; 2] = ["benedictus", "agnus"];

/// How often a seam of a normal mass restarts in imitation rather than together (MASS-RESTART):
/// the imitation share of the openings in Palestrina's Gloria and Credo, 32–35 %, which suits
/// the slower declamation of the four-voice medians (0.75 and 0.9 syllables per measure).
/// Provisional: the openings at internal breaks were not verified (§ 1.2).
pub const RESTART_SEAM: f64 = 0.35;

/// How often a seam of a Missa brevis restarts in imitation (MASS-RESTART, WP15). The Missa
/// brevis declaims its Gloria and Credo nearly syllabically, 0.82 measures per syllable, with
/// imitative points in 15 % and 35 % of their measures. A seam in imitation takes 1.54 measures
/// per syllable and twice the time of one together, 0.93 (measured in phase 14a on the Dorian
/// mass with every seam set both ways). One seam in five in imitation, and Et in Spiritum in one
/// of three ([`RESTART_SPIRITUM`]), bring its Gloria and Credo from 1.17 to 1.10 measures per
/// syllable and still open about two of its sentences with a point, besides those of Et in terra
/// and Patrem. Provisional, as [`RESTART_SEAM`].
pub const RESTART_SEAM_BREVIS: f64 = 0.2;

/// How often the seam at Et in Spiritum of a Missa brevis restarts in imitation: openings there
/// are imitative in 52 %, homophonic in 61 % (GC-SECTION-OPENING, both counted), lowered with
/// the other seams of the Missa brevis ([`RESTART_SEAM_BREVIS`]). In a normal mass Et in
/// Spiritum begins a section. Provisional.
pub const RESTART_SPIRITUM: f64 = 0.35;

/// How often a seam of a mass of the form `preset` restarts in imitation: [`RESTART_SEAM_BREVIS`]
/// or [`RESTART_SEAM`].
pub fn restart_seam(preset: Preset) -> f64 {
    match preset {
        Preset::Brevis => RESTART_SEAM_BREVIS,
        Preset::Normal => RESTART_SEAM,
    }
}

/// The voices of a section scored `scoring` in a mass for `choir` (from the top), by the
/// majority of Palestrina's masses of that size (MASS-SCORING, § 3.2):
/// - the Christe keeps every voice up to five (four-voice masses 37 of 37, five-voice 19 of 27);
///   six voices sing it in four, without the lowest two (11 of 21);
/// - a reduced section drops the bass: four voices sing it a 3 (Benedictus 27 of 38, mostly
///   S A T; Crucifixus 19 of 23; Pleni 12 of 14), five a 4 (Benedictus 25 of 27, Crucifixus 25
///   of 26) and six a 4 too (Benedictus 18 of 21, Crucifixus 21 of 21); three voices keep them
///   all, as Palestrina wrote no mass for three;
/// - Agnus Dei II gains a voice: a tenor in four voices (every four-voice Agnus II of 34 is
///   enlarged, by a tenor in 15, an alto in 11), a soprano in five (enlarged in 17 of 24); six
///   voices keep six (15 of 19).
pub fn scoring(choir: &[VoiceType], scoring: Scoring) -> Vec<VoiceType> {
    let top = |k: usize| choir[..k.min(choir.len())].to_vec();
    let with = |added: VoiceType| {
        let mut voices = choir.to_vec();
        voices.push(added);
        voices.sort();
        voices
    };
    match (scoring, choir.len()) {
        (Scoring::Christe, 6) => top(4),
        (Scoring::Reduced, 4) => top(3),
        (Scoring::Reduced, 5 | 6) => top(4),
        (Scoring::Enlarged, 4) => with(VoiceType::Tenor),
        (Scoring::Enlarged, 5) => with(VoiceType::Soprano),
        _ => choir.to_vec(),
    }
}

/// The step `steps` diatonic steps above the final of `mode`.
fn above_final(mode: ModeSpec, steps: i32) -> Step {
    Step::from_index(mode.final_step().index() + steps)
}

/// The principal degree beside the final (P, § 3.4), drawn once for a mass with `draw` in
/// [0, 1): the fifth in the Dorian, Ionian and Lydian modes, the fourth in the Phrygian (A) and
/// the Aeolian on A (D); the Aeolian on D the fifth or the fourth and the Mixolydian the fourth
/// (C) or the fifth (D), half each. In the Mixolydian the choice is a per-mass preference, not
/// exclusive: masses with C or D in their „Cadence finals“ cadence on the other as well
/// (verify_cadences, CAD-PRINCIPAL-SECONDARY), so it only shifts the chain's weights
/// ([`chain_weights`]).
pub fn principal(mode: ModeSpec, draw: f64) -> Step {
    let steps = match (mode.mode, mode.transposed) {
        (Mode::Mixolydian, _) if draw < 0.5 => 3,
        (Mode::Mixolydian, _) => 4,
        (Mode::Aeolian, true) if draw < 0.5 => 4,
        (Mode::Aeolian, true) => 3,
        (Mode::Aeolian | Mode::Phrygian, _) => 3,
        _ => 4,
    };
    above_final(mode, steps)
}

/// What decides the ending of a part (§ 3.5): its place in the movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EndingKind {
    /// The last part of a movement: on the final (97 % of movement ends).
    Last,
    KyrieI,
    Christe,
    /// The part before Qui tollis: the end of Et in terra.
    BeforeQuiTollis,
    AgnusI,
    /// The Benedictus before a new Osanna II.
    BenedictusBeforeOsanna,
    /// The Benedictus before „Osanna ut supra“, which then ends the movement on the final.
    BenedictusUtSupra,
    /// Every other part: before a Break, a seam or another section.
    Other,
}

/// A degree of a distribution: diatonic steps above the final, or P.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Degree {
    Above(i32),
    Principal,
}

const FINAL: Degree = Degree::Above(0);
const FIFTH: Degree = Degree::Above(4);
const FOURTH: Degree = Degree::Above(3);
const P: Degree = Degree::Principal;

/// The shares of the endings of a part of `kind`, before they are restricted to the allowed
/// degrees (§ 3.5, after verify_cadences and verify_structure):
/// - Kyrie I ends on the final (95–100 %); in the Phrygian also on A (4 of 8).
/// - The Christe moves away: Dorian and Ionian to the fifth (about 0.8), Mixolydian to the
///   final, C or D, Aeolian mostly to D, Phrygian to E or A, Lydian as Ionian on F.
/// - Et in terra before Qui tollis, in two-part Glorias: Dorian on the fifth 0.71 (n = 17),
///   Ionian 0.67 (n = 12), Mixolydian on the final 0.73 (n = 11), Aeolian on the final 0.62 and
///   D 0.38 (n = 8). The rest goes to P, or to the final where the named degree is P itself.
/// - Agnus Dei I mostly on the final (0.86–0.88); in the Phrygian on A in 3 of 7.
/// - The Benedictus before Osanna II as every other part, but Ionian on the fifth 0.64 (n = 14),
///   Mixolydian on the final 0.77; before „ut supra“ on the final 0.70, the fifth 0.30, the
///   Ionian on the fifth in 7 of 8.
/// - Every other part: inner sections end on the final by mode, Dorian 0.53, Ionian 0.46,
///   Mixolydian 0.70, Aeolian 0.56, Phrygian 0.50 (the Lydian, absent from the masses, as the
///   Ionian on F), the rest on P.
fn shares(kind: EndingKind, mode: Mode) -> &'static [(Degree, f64)] {
    use EndingKind::*;
    use Mode::*;
    match (kind, mode) {
        (Last, _) => &[(FINAL, 1.0)],
        (KyrieI, Phrygian) => &[(FINAL, 0.5), (FOURTH, 0.5)],
        (KyrieI, _) => &[(FINAL, 1.0)],
        (Christe, Dorian) => &[(FIFTH, 0.87), (FINAL, 0.13)],
        (Christe, Ionian) => &[(FIFTH, 0.81), (FINAL, 0.19)],
        (Christe, Mixolydian) => &[(FINAL, 0.38), (FOURTH, 0.38), (FIFTH, 0.24)],
        (Christe, Aeolian) => &[(FOURTH, 0.50), (FIFTH, 0.30), (FINAL, 0.20)],
        (Christe, Phrygian) => &[(FINAL, 0.5), (FOURTH, 0.5)],
        (Christe, Lydian) => &[(FIFTH, 0.8), (FINAL, 0.2)],
        (BeforeQuiTollis, Dorian) => &[(FIFTH, 0.71), (FINAL, 0.29)],
        (BeforeQuiTollis, Ionian | Lydian) => &[(FIFTH, 0.67), (FINAL, 0.33)],
        (BeforeQuiTollis, Mixolydian) => &[(FINAL, 0.73), (P, 0.27)],
        (BeforeQuiTollis, Aeolian) => &[(FINAL, 0.62), (FOURTH, 0.38)],
        (BeforeQuiTollis, Phrygian) => &[(FINAL, 0.5), (FOURTH, 0.5)],
        (AgnusI, Phrygian) => &[(FINAL, 0.57), (FOURTH, 0.43)],
        (AgnusI, _) => &[(FINAL, 0.88), (P, 0.12)],
        (BenedictusBeforeOsanna, Ionian) => &[(FIFTH, 0.64), (FINAL, 0.36)],
        (BenedictusBeforeOsanna, Mixolydian) => &[(FINAL, 0.77), (P, 0.23)],
        (BenedictusUtSupra, Ionian) => &[(FIFTH, 0.875), (FINAL, 0.125)],
        (BenedictusUtSupra, _) => &[(FINAL, 0.70), (FIFTH, 0.30)],
        (Other | BenedictusBeforeOsanna, Dorian) => &[(FINAL, 0.53), (P, 0.47)],
        (Other | BenedictusBeforeOsanna, Ionian | Lydian) => &[(FINAL, 0.46), (P, 0.54)],
        (Other, Mixolydian) => &[(FINAL, 0.70), (P, 0.30)],
        (Other | BenedictusBeforeOsanna, Aeolian) => &[(FINAL, 0.56), (P, 0.44)],
        (Other | BenedictusBeforeOsanna, Phrygian) => &[(FINAL, 0.50), (P, 0.50)],
    }
}

/// The distribution of the ending of a part of `kind` in `mode` with the principal degree
/// `principal`: its degrees with their shares, restricted to the degrees a part may end on
/// ([`part_endings`]) and renormalised, each degree once. Never empty: where nothing is left,
/// the final.
pub fn ending_distribution(kind: EndingKind, mode: ModeSpec, principal: Step) -> Vec<(Step, f64)> {
    let allowed = part_endings(mode);
    let mut out: Vec<(Step, f64)> = Vec::new();
    for &(degree, share) in shares(kind, mode.mode) {
        let step = match degree {
            Degree::Above(steps) => above_final(mode, steps),
            Degree::Principal => principal,
        };
        if !allowed.contains(&step) {
            continue;
        }
        match out.iter_mut().find(|(s, _)| *s == step) {
            Some((_, sum)) => *sum += share,
            None => out.push((step, share)),
        }
    }
    let total: f64 = out.iter().map(|&(_, share)| share).sum();
    if total <= 0.0 {
        return vec![(mode.final_step(), 1.0)];
    }
    for (_, share) in &mut out {
        *share /= total;
    }
    out
}

/// The shares to draw the ending of a part from where an ending equal to the one before it is
/// drawn once more (within a section, § 3.5), so that the part still ends on each degree as
/// often as its distribution `shares` says, with the ending before drawn from `before` (one
/// degree with 1 where it was certain). The redraw takes a degree away where the part before
/// ended on it, so these shares raise each degree for it: drawn from d′ after an ending drawn
/// from e, a degree x comes out d′(x)·(1 − e(x) + S) of the time, S = Σ e(y)·d′(y) being the
/// chance of drawing the ending before again; with d′(x) = d(x) / (1 − e(x) + S), where S makes
/// them sum to one, that is d(x). The redraw alone (d′ = d) would move the shares towards
/// evenness, the Mixolydian seams on the final from 0.70 to 0.645, its part before Qui tollis from
/// 0.73 to 0.685 (10 000 plans, phase 14a).
pub fn redrawn(shares: &[(Step, f64)], before: &[(Step, f64)]) -> Vec<(Step, f64)> {
    let before = |step: Step| {
        before
            .iter()
            .find(|&&(s, _)| s == step)
            .map_or(0.0, |&(_, share)| share)
    };
    let raised = |again: f64| -> Vec<(Step, f64)> {
        shares
            .iter()
            .map(|&(step, share)| (step, share / (1.0 - before(step) + again)))
            .collect()
    };
    let sum = |raised: &[(Step, f64)]| raised.iter().map(|&(_, share)| share).sum::<f64>();
    // The sum falls as S grows, from at least one at S = 0 to at most one at S = 1.
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..50 {
        let middle = (low + high) / 2.0;
        if sum(&raised(middle)) > 1.0 {
            low = middle;
        } else {
            high = middle;
        }
    }
    let raised = raised((low + high) / 2.0);
    let total = sum(&raised);
    raised
        .into_iter()
        .map(|(step, share)| (step, share / total))
        .collect()
}

/// One of the degrees of a distribution, by `draw` in [0, 1).
pub fn pick(distribution: &[(Step, f64)], draw: f64) -> Step {
    let mut left = draw;
    for &(step, share) in distribution {
        if left < share {
            return step;
        }
        left -= share;
    }
    distribution.last().map_or(Step::C, |&(step, _)| step)
}

/// How far the Mixolydian's principal degree shifts the weights of C and D, in ‰: masses with
/// one of them in their „Cadence finals“ cadence on it about 0.05–0.1 more often.
pub const CHAIN_MIXOLYDIAN_SHIFT: u16 = 50;

/// The shares of the internal cadences of a mass part in ‰ by degree (MASS-CADENCES, § 3.6):
/// the shares of Palestrina's internal full cadences per mode family with a clausula and at
/// least 2 % (CAD-INTERNAL-DEGREES, all eight families within 2 points in the verification),
/// as diatonic steps above the final so that transposed modes without a row of their own take
/// them along. The Lydian, absent from the masses, takes the Ionian on F without B♭. These are
/// the shares the cadences of the plans come out with; the chain draws with weights compensated
/// for its repeats and its factors ([`chain_weights`]).
pub fn corpus_weights(mode: ModeSpec, principal: Step) -> Vec<(Step, u16)> {
    let weights: &[(i32, u16)] = match (mode.mode, mode.transposed) {
        // D 448, A 349, G 67, C 63, F 59.
        (Mode::Dorian, false) => &[(0, 448), (4, 349), (3, 67), (6, 63), (2, 59)],
        // On G: D 360, G 346, B♭ 95, C 92, F 67, A 40.
        (Mode::Dorian, true) => &[(4, 360), (0, 346), (2, 95), (3, 92), (6, 67), (1, 40)],
        // A 406, E 180, C 159, G 128, D 114.
        (Mode::Phrygian, _) => &[(3, 406), (0, 180), (5, 159), (2, 128), (6, 114)],
        // G 396, C 292, D 198, A 70.
        (Mode::Mixolydian, _) => &[(0, 396), (3, 292), (4, 198), (1, 70)],
        // A 413, D 290, C 119, E 70, F 57, G 51.
        (Mode::Aeolian, false) => &[(0, 413), (3, 290), (2, 119), (4, 70), (5, 57), (6, 51)],
        // On D: D 478, G 172, A 167, F 119, C 52.
        (Mode::Aeolian, true) => &[(0, 478), (3, 172), (4, 167), (2, 119), (6, 52)],
        // C 464, G 314, A 76, D 74, E 46, F 25.
        (Mode::Ionian, false) => &[(0, 464), (4, 314), (5, 76), (1, 74), (2, 46), (3, 25)],
        // On F: F 436, C 361, B♭ 77, A 50, D 41, G 35; the Lydian without its fourth.
        (Mode::Ionian, true) | (Mode::Lydian, _) => {
            &[(0, 436), (4, 361), (3, 77), (2, 50), (5, 41), (1, 35)]
        }
    };
    let shifted = |steps: i32, weight: u16| -> u16 {
        if mode.mode != Mode::Mixolydian || !matches!(steps, 3 | 4) {
            return weight;
        }
        if above_final(mode, steps) == principal {
            weight + CHAIN_MIXOLYDIAN_SHIFT
        } else {
            weight.saturating_sub(CHAIN_MIXOLYDIAN_SHIFT)
        }
    };
    let shifted: Vec<(i32, u16)> = weights
        .iter()
        .map(|&(steps, weight)| (steps, shifted(steps, weight)))
        .collect();
    degrees_of(mode, &shifted)
}

/// The weights of `row`, by diatonic steps above the final, as degrees of `mode`, without the
/// degrees that have no clausula in it.
pub(crate) fn degrees_of(mode: ModeSpec, row: &[(i32, u16)]) -> Vec<(Step, u16)> {
    row.iter()
        .map(|&(steps, weight)| (above_final(mode, steps), weight))
        .filter(|&(step, _)| mode.clausula(step).is_some())
        .collect()
}

/// The weights of the chain of a mass part in ‰ by degree (MASS-CADENCES, § 3.6;
/// [`CadenceChain::Weighted`]), compensated so that the internal cadences of the plans come out
/// with the shares of the corpus ([`corpus_weights`]). The chain repeats the degree before with
/// p .30 and otherwise draws another; at an intermediate stop, a caesura or before a tail, where
/// most goals of the parts lie, it weighs the final and the mode's first regular degree ×0.3, and
/// the final of the Christe's first cadence ×0.4. With the corpus shares as its weights the final
/// would come out 9 to 18 points below its share (the Dorian D 0.32 against 0.45, the Aeolian on
/// D 0.30 against 0.48; in the Phrygian the first regular degree A, its P, 0.26 against 0.41) and
/// the colour degrees twice as often as in the corpus (measured in phase 14a). Each row is therefore calibrated (`calibrate_chain_weights` in the tests of
/// `mass::plan`): the cadences that the chains of 300 plans per form, choir of three to five
/// voices and mode draw at the goals that their parts' texts and points plan, which are the goals
/// of the parts' searches (`goals_are_those_of_the_settings`), counted with the weights, each
/// weight multiplied by its degree's share in the corpus over its share drawn, 60 rounds. With
/// the rows in ‰, every degree comes out within 0.3 points of its share in the corpus, which is
/// named beside each row. The Mixolydian has a row for each principal degree; transposed and
/// plagal modes take the rows of their families, the Lydian its own without its fourth.
///
/// [`CadenceChain::Weighted`]: crate::text::plan::CadenceChain::Weighted
pub fn chain_weights(mode: ModeSpec, principal: Step) -> Vec<(Step, u16)> {
    let row: &[(i32, u16)] = match (mode.mode, mode.transposed) {
        // Corpus D 448, A 349, G 67, C 63, F 59.
        (Mode::Dorian, false) => &[(0, 692), (4, 254), (3, 19), (6, 19), (2, 15)],
        // On G: D 360, G 346, B♭ 95, C 92, F 67, A 40.
        (Mode::Dorian, true) => &[(4, 450), (0, 433), (2, 37), (3, 37), (6, 29), (1, 14)],
        // A 406, E 180, C 159, G 128, D 114.
        (Mode::Phrygian, _) => &[(3, 726), (0, 120), (5, 60), (2, 50), (6, 43)],
        // P C: G 396, C 342, D 148, A 70.
        (Mode::Mixolydian, _) if above_final(mode, 3) == principal => {
            &[(0, 676), (3, 162), (4, 136), (1, 25)]
        }
        // P D: G 396, C 242, D 248, A 70.
        (Mode::Mixolydian, _) => &[(0, 691), (3, 89), (4, 198), (1, 22)],
        // A 413, D 290, C 119, E 70, F 57, G 51.
        (Mode::Aeolian, false) => &[(0, 731), (3, 121), (2, 47), (4, 60), (5, 22), (6, 18)],
        // On D: D 478, G 172, A 167, F 119, C 52.
        (Mode::Aeolian, true) => &[(0, 889), (3, 27), (4, 56), (2, 21), (6, 8)],
        // C 464, G 314, A 76, D 74, E 46, F 25.
        (Mode::Ionian, false) => &[(0, 754), (4, 188), (5, 21), (1, 20), (2, 11), (3, 6)],
        // On F: F 436, C 361, B♭ 77, A 50, D 41, G 35.
        (Mode::Ionian, true) => &[(0, 620), (4, 316), (3, 25), (2, 17), (5, 12), (1, 10)],
        // The Ionian on F without B♭: F 436, C 361, A 50, D 41, G 35.
        (Mode::Lydian, _) => &[(0, 695), (4, 271), (2, 14), (5, 12), (1, 9)],
    };
    degrees_of(mode, row)
}

/// Seconds per estimate (§ 3.10): `t = v · (a · N + b · N²) · (plain ? c : 1)` for N measures
/// natively on eight cores, refitted in WP15 to the 151 parts set first in phase 14a under the
/// profile of the mass with the portable budget (seconds on an idle machine by their CPU time; on
/// the measures each part came out with, the error is ×1.36 rms) and checked on the 104 parts of
/// `mass_timings` (×1.25 rms, the best fit there ×1.24), as a = 0.067 and b = 0.006. The faster
/// search after the plan (docs/naechste-schritte.md, Rang 1) set the Missa brevis of example 11
/// in 0.551 of the time (165 s against 299 s, paired, each part 0.47 to 0.70 of it), and both
/// terms took that factor. The prototype's fit, `v · (0.06 · N + 0.007 · N²) · (plain ? 0.8 :
/// 1)` with imitative parts 1.35 times their automatic length, estimated these parts 1.47 times
/// too short.
pub const ESTIMATE_LINEAR: f64 = 0.037;

/// The quadratic term: each step of the search checks the whole score so far (§ 1.2).
pub const ESTIMATE_SQUARE: f64 = 0.0033;

/// A part without imitation takes this many times as long per measure as one with: its search
/// has no planned heads that fix notes (measured 1.20 in phase 14a; the prototype assumed 0.8).
pub const ESTIMATE_PLAIN: f64 = 1.2;

/// An imitative part on a long text (the parts of Gloria and Credo) is this much longer than the
/// automatic length of its text: 1.67 in the first masses of phase 14a (38 parts) and in WP15's
/// (19).
pub const ESTIMATE_IMITATIVE: f64 = 1.7;

/// An imitative part on a short text at the syllabic pace (Kyrie, Christe, Agnus Dei) is this
/// much longer than the automatic length of its text: Kyrie I 1.60 and 1.73 (medians of the first
/// masses of phase 14a and of WP15's), Christe 2.06 and 2.09, Kyrie II 1.90 and 1.83, Agnus Dei II
/// in five voices 2.30 and 2.33, Agnus Dei I with its natural points 1.95 (the first masses).
pub const ESTIMATE_SHORT: f64 = 1.9;

/// An imitative part on a melismatic short text (Sanctus, Pleni, Osanna, Benedictus) is about as
/// long as the automatic length of its text, whose slow pace already leaves room for the points:
/// the Sanctus of the Missa brevis 0.91 to 1.14 times it (median 1.06 in the first five masses
/// of phase 14a, 0.95 in WP15's four), the Benedictus with its natural points 0.91 to 1.34 (the
/// first masses).
pub const ESTIMATE_MELISMATIC: f64 = 1.05;

/// The measures that each point beyond those the imitation plans by itself adds to a lengthened
/// part (MASS-LENGTH): two more points gave the Benedictus 15 to 22 more measures and Agnus Dei I
/// 21 to 26 in the first masses of phase 14a.
pub const ESTIMATE_POINT: f64 = 10.0;

/// WebAssembly takes about this many times as long (Deno 4.7–5.3); measured in 14c. The faster
/// search after the plan gained about as much there as natively (−42 to −44 % in Deno, −41 to
/// −47 % natively on the same Gloria, research of 28 Oct 2026).
pub const ESTIMATE_WASM: f32 = 5.0;

/// The parts per worker at the end of a run that the pool of the web version, and `fux mass`
/// with several parts at once, start longest first, after filling the workers movement by
/// movement ([`MassPlan::pool_order_of`](super::MassPlan::pool_order_of), web/pool.js), so
/// that the workers finish about together. Simulated with the estimates on 180 plans (six
/// modes, five seeds, with the head motif, in three and five voices, the form normal and on
/// chant IX), the mass is done at most 4.7 s later than with the longest parts first on four
/// workers, 6.3 s on two and 6.7 s on three, and on average sooner; filled movement by movement
/// to the end, up to 41 s later on four workers (K12 (a)).
pub const POOL_TAIL: usize = 3;

/// Natively `fux mass` sets several parts at once (K6), their searches sharing the threads of
/// rayon's pool, so that each has fewer of them than alone: a part takes this much longer per part
/// set beside it ([`estimate_jobs`]). Fitted once on example 11 (`--mode ionisch-transponiert
/// --seed 2`, 26 parts), set with four parts at once and with one, back to back, on a machine of
/// eight cores with sixteen threads that other work shared (never idle): at a load of 8 to 16 four
/// took 136 s against 165 and 198 s for one after the other (0.82 and 0.69), at a load of 30 to
/// 80, 230 against 310 s and 385 against 680 s (0.74 and 0.57); each part took 2.2 to 3.3 times
/// as long as alone, and the pool's estimate with that factor gave the ratio within 0.03, as the
/// four-job estimate is 41.8 s at the factor 1 against 161.3 s for one part after the other. The
/// factor 2.8 for four parts (0.73) lies in the middle. Two and three parts at once are
/// interpolated: a run with two at a load of 50 to 70 (611 s) gave no clear measure. Confirmed on
/// the idle machine (5 Nov 2026, with the program after L6, L3, the parts in the pool's order):
/// three pairs, set with 1, 4, 4, 1, 1 and 4 parts at once, each run from a load below 1, took
/// 151.2, 155.1 and 152.3 s one part after the other and 118.3, 119.4 and 116.5 s four at once.
/// With S = 161.29 s, the sum of the parts' estimates, and P = 41.75 s, the pool of four workers at
/// the factor 1, each pair gives the factor ((T4 / T1) · S / P − 1) / 3: 0.675, 0.658 and 0.652,
/// the median 0.658, 9.6 % above 0.6, which stays. With it the plan estimates 116.9 s for four at
/// once, close to the 116.5 to 119.4 s measured, as S lies 4 to 7 % above the time of one part
/// after the other. The first runs began the parts longest first (example 11 has no head motif), as
/// the pool did before K12 (a); its order now estimates example 11 alike (41.8 s at the factor 1,
/// 117 s at 2.8).
pub const ESTIMATE_JOBS: f32 = 0.6;

/// How many times as long as alone a part takes natively while `jobs` parts are set at once
/// ([`ESTIMATE_JOBS`]): 1 for one part at a time. With [`MassPlan::pool_seconds`] it gives the
/// native estimate of a mass (at most the sum of its parts), as [`ESTIMATE_WASM`] gives the
/// browser's.
///
/// [`MassPlan::pool_seconds`]: super::MassPlan::pool_seconds
pub fn estimate_jobs(jobs: usize) -> f32 {
    1.0 + ESTIMATE_JOBS * jobs.saturating_sub(1) as f32
}

/// The factor v of the estimate by the number of voices: three .55 (phase 14a, nine parts), four
/// 1, five 2.2 (measured 2.22, five parts), six 4; two and six extrapolated.
pub fn estimate_voices(voices: usize) -> f64 {
    match voices {
        0..=2 => 0.2,
        3 => 0.55,
        4 => 1.0,
        5 => 2.2,
        _ => 4.0,
    }
}

/// The seconds of a part of `measures` measures in `voices` voices, `plain` without imitation
/// ([`ESTIMATE_LINEAR`]).
pub fn estimate_seconds(measures: f64, voices: usize, plain: bool) -> f64 {
    let factor = if plain { ESTIMATE_PLAIN } else { 1.0 };
    estimate_voices(voices)
        * (ESTIMATE_LINEAR * measures + ESTIMATE_SQUARE * measures * measures)
        * factor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::Ambitus;

    fn mode(name: &str) -> ModeSpec {
        name.parse().unwrap()
    }

    #[test]
    fn principal_degrees_by_mode() {
        let p = |name: &str, draw: f64| principal(mode(name), draw).letter();
        assert_eq!(p("dorian", 0.1), 'A');
        assert_eq!(p("ionian", 0.9), 'G');
        assert_eq!(p("phrygian", 0.1), 'A');
        assert_eq!(p("lydian", 0.1), 'C');
        assert_eq!(p("aeolian", 0.9), 'D');
        assert_eq!(
            (p("aeolian-transposed", 0.1), p("aeolian-transposed", 0.9)),
            ('A', 'G')
        );
        assert_eq!((p("mixolydian", 0.1), p("mixolydian", 0.9)), ('C', 'D'));
        assert_eq!(p("hypodorian", 0.5), 'A');
        assert_eq!(p("dorian-transposed", 0.5), 'D');
    }

    /// Every distribution is restricted to the allowed degrees, sums to one and holds each
    /// degree once; the last part ends on the final.
    #[test]
    fn distributions_are_allowed() {
        let kinds = [
            EndingKind::Last,
            EndingKind::KyrieI,
            EndingKind::Christe,
            EndingKind::BeforeQuiTollis,
            EndingKind::AgnusI,
            EndingKind::BenedictusBeforeOsanna,
            EndingKind::BenedictusUtSupra,
            EndingKind::Other,
        ];
        for m in Mode::ALL {
            for transposed in [false, true] {
                for ambitus in [Ambitus::Authentic, Ambitus::Plagal] {
                    let mode = ModeSpec::new(m)
                        .with_transposition(transposed)
                        .with_ambitus(ambitus);
                    for draw in [0.1, 0.9] {
                        let p = principal(mode, draw);
                        assert!(part_endings(mode).contains(&p), "{mode}");
                        for kind in kinds {
                            let d = ending_distribution(kind, mode, p);
                            let total: f64 = d.iter().map(|&(_, s)| s).sum();
                            assert!((total - 1.0).abs() < 1e-9, "{kind:?} {mode}");
                            for (i, &(step, _)) in d.iter().enumerate() {
                                assert!(part_endings(mode).contains(&step), "{kind:?} {mode}");
                                assert!(d[..i].iter().all(|&(s, _)| s != step));
                            }
                        }
                        let last = ending_distribution(EndingKind::Last, mode, p);
                        assert_eq!(last, [(mode.final_step(), 1.0)]);
                    }
                }
            }
        }
        // The rest of the part before Qui tollis goes to the final where the fifth is P.
        let dorian = mode("dorian");
        assert_eq!(
            ending_distribution(EndingKind::BeforeQuiTollis, dorian, Step::A),
            [(Step::A, 0.71), (Step::D, 0.29)]
        );
        // In the Phrygian the Benedictus before ut supra cannot end on its fifth, B.
        let phrygian = mode("phrygian");
        assert_eq!(
            ending_distribution(EndingKind::BenedictusUtSupra, phrygian, Step::A),
            [(Step::E, 1.0)]
        );
        assert_eq!(pick(&[(Step::A, 0.7), (Step::D, 0.3)], 0.69), Step::A);
        assert_eq!(pick(&[(Step::A, 0.7), (Step::D, 0.3)], 0.71), Step::D);
    }

    /// Drawn from the raised shares, with an equal ending drawn once more, a part ends on each
    /// degree as often as its distribution says, whatever the ending before was drawn from: the
    /// same distribution, another one or a certain degree. The redraw alone would not keep them.
    #[test]
    fn redrawn_shares_keep_the_tables() {
        // The distribution of an ending drawn from `drawn` with an equal one drawn again, after
        // an ending drawn from `before`.
        let outcome = |drawn: &[(Step, f64)], before: &[(Step, f64)]| -> Vec<(Step, f64)> {
            let share = |list: &[(Step, f64)], x: Step| {
                list.iter().find(|&&(s, _)| s == x).map_or(0.0, |s| s.1)
            };
            drawn
                .iter()
                .map(|&(x, d)| {
                    let p: f64 = before
                        .iter()
                        .map(|&(y, e)| e * (if x == y { 0.0 } else { d } + share(drawn, y) * d))
                        .sum();
                    (x, p)
                })
                .collect()
        };
        let mixolydian = [(Step::G, 0.70), (Step::C, 0.30)];
        let before_qui_tollis = [(Step::G, 0.73), (Step::C, 0.27)];
        let dorian = [(Step::D, 0.53), (Step::A, 0.47)];
        let christe = [(Step::G, 0.38), (Step::C, 0.38), (Step::D, 0.24)];
        for (shares, before) in [
            (&mixolydian[..], &mixolydian[..]),
            (&before_qui_tollis, &mixolydian),
            (&[(Step::A, 0.71), (Step::D, 0.29)], &dorian),
            (&dorian, &[(Step::D, 1.0)]),
            (&christe, &[(Step::G, 0.5), (Step::D, 0.5)]),
            (&[(Step::D, 1.0)], &dorian),
        ] {
            let raised = redrawn(shares, before);
            let total: f64 = raised.iter().map(|r| r.1).sum();
            assert!((total - 1.0).abs() < 1e-9);
            for (&(x, wanted), (y, got)) in shares.iter().zip(outcome(&raised, before)) {
                assert_eq!(x, y);
                assert!(
                    (got - wanted).abs() < 1e-9,
                    "{shares:?} after {before:?}: {got}"
                );
            }
        }
        // Without the raise, the Mixolydian seams end on the final 0.641 of the time.
        let seams = outcome(&mixolydian, &[(Step::G, 0.641), (Step::C, 0.359)]);
        assert!((seams[0].1 - 0.641).abs() < 0.001, "{seams:?}");
        // After a certain degree it is drawn with the square root of its share.
        let raised = redrawn(&dorian, &[(Step::D, 1.0)]);
        assert!((raised[0].1 - 0.53_f64.sqrt()).abs() < 1e-9, "{raised:?}");
    }

    /// The corpus shares by mode and the chain's weights compensated for them: on the same
    /// degrees in every mode, the Mixolydian's by its principal degree, each row about a
    /// thousand.
    #[test]
    fn chain_weights_by_mode() {
        let letters = |weights: Vec<(Step, u16)>| {
            weights
                .iter()
                .map(|&(s, w)| format!("{}{w}", s.letter()))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let corpus = |name: &str, p: Step| letters(corpus_weights(mode(name), p));
        let chain = |name: &str, p: Step| letters(chain_weights(mode(name), p));
        assert_eq!(corpus("dorian", Step::A), "D448 A349 G67 C63 F59");
        assert_eq!(chain("dorian", Step::A), "D692 A254 G19 C19 F15");
        assert_eq!(
            corpus("dorian-transposed", Step::D),
            "D360 G346 B95 C92 F67 A40"
        );
        assert_eq!(corpus("mixolydian", Step::C), "G396 C342 D148 A70");
        assert_eq!(corpus("mixolydian", Step::D), "G396 C242 D248 A70");
        assert_eq!(chain("mixolydian", Step::C), "G676 C162 D136 A25");
        assert_eq!(chain("mixolydian", Step::D), "G691 C89 D198 A22");
        assert_eq!(chain("mixolydian-transposed", Step::G), "C691 F89 G198 D22");
        assert_eq!(corpus("lydian", Step::C), "F436 C361 A50 D41 G35");
        assert_eq!(chain("lydian", Step::C), "F695 C271 A14 D12 G9");
        assert_eq!(
            corpus("ionian-transposed", Step::C),
            "F436 C361 B77 A50 D41 G35"
        );
        assert_eq!(corpus("phrygian", Step::A), "A406 E180 C159 G128 D114");
        assert_eq!(chain("hypophrygian", Step::A), "A726 E120 C60 G50 D43");
        // The Lydian on B♭ has no cadence on E either.
        for weights in [corpus_weights, chain_weights] {
            assert!(
                weights(mode("lydian-transposed"), Step::F)
                    .iter()
                    .all(|&(s, _)| s != Step::E)
            );
        }
        for m in Mode::ALL {
            for transposed in [false, true] {
                let mode = ModeSpec::new(m).with_transposition(transposed);
                for draw in [0.1, 0.9] {
                    let p = principal(mode, draw);
                    let corpus = corpus_weights(mode, p);
                    let chain = chain_weights(mode, p);
                    let degrees = |w: &[(Step, u16)]| {
                        let mut steps: Vec<Step> = w.iter().map(|&(s, _)| s).collect();
                        steps.sort();
                        steps
                    };
                    assert_eq!(degrees(&corpus), degrees(&chain), "{mode}");
                    let total: u16 = chain.iter().map(|&(_, w)| w).sum();
                    assert!((997..=1003).contains(&total), "{mode}: {total}");
                }
            }
        }
    }

    #[test]
    fn scorings_by_choir() {
        use VoiceType::*;
        let choir = |n: usize| super::super::choir(n);
        let names = |voices: Vec<VoiceType>| -> String {
            voices
                .iter()
                .map(|v| v.abbreviation())
                .collect::<Vec<_>>()
                .join(" ")
        };
        assert_eq!(names(scoring(&choir(4), Scoring::Reduced)), "S A T");
        assert_eq!(names(scoring(&choir(4), Scoring::Enlarged)), "S A T T B");
        assert_eq!(names(scoring(&choir(4), Scoring::Christe)), "S A T B");
        assert_eq!(names(scoring(&choir(5), Scoring::Reduced)), "S A T T");
        assert_eq!(names(scoring(&choir(5), Scoring::Enlarged)), "S S A T T B");
        assert_eq!(names(scoring(&choir(6), Scoring::Christe)), "S S A T");
        assert_eq!(names(scoring(&choir(6), Scoring::Reduced)), "S S A T");
        assert_eq!(names(scoring(&choir(6), Scoring::Enlarged)), "S S A T T B");
        assert_eq!(names(scoring(&choir(3), Scoring::Reduced)), "A T B");
        assert_eq!(names(scoring(&choir(3), Scoring::Enlarged)), "A T B");
        assert_eq!(names(scoring(&choir(2), Scoring::Reduced)), "S T");
        assert_eq!(
            scoring(&[Soprano, Alto, Baritone, Bass], Scoring::Enlarged).len(),
            5
        );
    }

    #[test]
    fn estimates_grow_with_voices_and_length() {
        assert!((estimate_seconds(30.0, 4, false) - (0.037 * 30.0 + 0.0033 * 900.0)).abs() < 1e-9);
        // A part without imitation takes longer per measure.
        assert!(
            (estimate_seconds(30.0, 4, true) - 1.2 * estimate_seconds(30.0, 4, false)).abs() < 1e-9
        );
        assert!(estimate_seconds(30.0, 3, false) < estimate_seconds(30.0, 4, false));
        assert!(estimate_seconds(30.0, 5, false) > estimate_seconds(30.0, 4, false));
        assert!(estimate_seconds(60.0, 4, false) > 3.0 * estimate_seconds(30.0, 4, false));
    }
}
