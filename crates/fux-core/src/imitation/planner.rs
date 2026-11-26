//! The planner (docs/regeln.md § 18): the entries of every point, the time line with the goals
//! of the internal cadences, the length and the pace, the plan's cost and the alternatives of
//! each point for re-planning.
//!
//! **Entries.** For each point the planner draws 64 entry plans by Palestrina's shares (the study
//! of the corpus, confirmed by its verifier), over the subjects of the point in turn:
//!
//! - *Voices.* The first point has every voice; a later one as many as the request asks for,
//!   else as many as in Palestrina's later points of a choir of its size: mostly two (61 % in
//!   four-voice sections, 54–67 % with three to six voices), now and then more (four voices in
//!   15 % of the later points of four-voice sections); of those the voices that stopped singing
//!   first enter first (Vicentino, IV.29), ties drawn by the shares of the leaders below.
//! - *Order.* Four voices S A T B open in the orders ASTB, SATB, ASBT, TASB, BTAS, ATSB and TBSA
//!   (28, 27, 21, 15, 14, 12 and 10 of 254), one plan in five in another one. Other choirs draw
//!   the leader by voice type (alto .37, soprano .26, tenor .23, bass .14), a later point's
//!   leader by bass and tenor .29, soprano .21 and alto .19, the others in a random order: which
//!   voice follows which is at chance level (IMI-ORDER).
//! - *Pairs.* From four voices, one plan in ten lets the third and fourth entries repeat the
//!   first two an octave lower (SA TB, AS BT) or higher (BT AS, TB SA), after a gap at least a
//!   measure longer than the one between them (IMI-DUO); with the duos that the degrees and gaps
//!   give by themselves, a fifth of the openings, as in Palestrina's.
//! - *Degrees.* The second entry takes the other main degree between voices of different types
//!   (.81), the same between voices of one type (.74). Later a tenor takes the soprano's degree
//!   (.79) and a bass the alto's (.63), where that voice has entered, other voices the leader's
//!   (.59) or the second entry's (IMI-INTERVAL); where the first two share a degree, a later
//!   entry goes back to the other main degree with .5 first. A head off the main degrees (from
//!   chant or given notes) answers on the other main degree at the fifth above or the fourth
//!   below. A third entry at the pitch of two before it costs 1, a point of three entries on one
//!   degree 1, of four or more 2.
//! - *Delays.* The second entry follows the leader after 8, 16, 24 or 32 ticks by the number of
//!   voices (with three .27 .15 .20 .33, four .38 .18 .21 .16, five .41 .24 .08 .06, six .44 .18
//!   .08 .05), after a minim more now and then (.05 for all such delays), in an opening where the
//!   subject passes its stretto test; always before the leader's head ends. Later entries follow
//!   after 8 to 64 ticks (IMI-GAP), a restated point's after 8 or 16.
//! - *First note.* An entry on the minim beat shortens its first note by a minim (.63) or ties
//!   it over the barline (IMI-METER).
//! - *Answer.* Real; modal now and then (.25) where the head joins final and fifth in its first
//!   intervals, and where the real answer cannot be sung in any octave (IMI-TONAL).
//!
//! **Octaves and heads.** An entry takes an octave of its voice in which its head lies within the
//! voice's octave of the mode and its staff, as far as FREE-AMBITUS and VOI-RANGE leave every note
//! free; the opening's second entry by preference at a shift that passes the stretto test; then
//! at the least cost of the head checked alone and of crossing the heads of other voices that
//! sound with it (VOI-CROSSING), its first note nearest the middle of the voice. Re-planning moves
//! an entry only to another such octave. Where a voice has no such octave for the head, it sings
//! it within its staff beyond the octave of the mode, else a step below the staff, and only where
//! none of these is further beyond, as far as its pitches reach: in the Mixolydian and the Aeolian
//! modes the octaves of the mode of the lower voices lie partly below their staves (the bass's
//! from D2 to D3 against a staff from G2), and a strict range would leave a fifth of their plans
//! without a valid one. The plan's cost counts what these heads cost, so plans whose heads lie
//! within the ranges win. No head spans more than an octave.
//! A head keeps the subject's notes but where a pair check fails: there it ends before the
//! failing note (fuga sciolta, Vicentino IV.32), never below four notes, five with a modal
//! answer, which needs a fifth agreeing note to be recognised as one. Its octave is judged by the
//! notes it keeps: where such a cut leaves it beyond its range and the octave above or below holds
//! it further within, it moves there if the heads still keep clear of each other. A layout in
//! which a head stays more than a step beyond its staff only because that octave clashes is
//! rejected.
//!
//! **Time line.** The first point begins at tick 0, a part of a mass section at its start (after
//! a breath or a measure's rest, phase 14); each entry follows the one before by its delay. A
//! point's goal is the first measure two measures after the heads of the voices of its cadence
//! formula, one after the others' and three after the goal before it and the point's leader,
//! where no head sounds a second or a seventh against the target that the cantizans holds into
//! the penultimate measure, and where the formula voices can begin the rest of the point's words
//! within four fifths of their minim beats before it. The next point's leader enters on the
//! minim beat after the goal, a voice of its formula (and now and then another one, .15) a
//! measure later. The last point ends in the final cadence, on the final or a part's ending, two
//! measures after the last head and three after the last goal, where every voice can begin the
//! rest of the text within four fifths of its minim beats. No voice begins the words of a point
//! before its own entry in the point, or a voice without one before the point's second entry
//! ([`crate::text::underlay::Gates`]), so these words count only on the beats from there on, here
//! and between the heads of a voice (C7). The targets follow the endings of the points' words as
//! without imitation ([`crate::text::plan`]), by the part's chain in a mass.
//!
//! **Choice.** Every plan drawn goes through the checks of [`super::check`] and costs what its
//! subject costs and what the checks price, ranked with noise. Closer strettos cost more under the
//! profile and fail the checks more often: the cheapest of plans drawn one by one would let the
//! second entry come late and the later gaps run long (with a semibreve in half of Palestrina's
//! share). So the shape of a point is drawn once by his shares (the second entry's delay, the
//! later gaps, the pairs, a later leader's wait, a later point's voices), and its 64 plans vary
//! the subject, the order, the degrees and the first notes within it; 32 more plans are drawn
//! freely. Each point keeps four valid plans as alternatives: the cheapest in the shape, or where
//! none of those passes the cheapest other one with the nearest second entry, then the cheapest
//! others. Where no plan is valid, the planner draws once more with heads of at most five notes,
//! every gap a measure longer and later points with a voice fewer; failing that, the request
//! fails with [`GenerateError::NoImitation`], unless the point is a restatement that the planner
//! chose itself: then the text's last tail takes its place. Options of the request that the
//! choir or the text cannot have fail at once ([`GenerateError::ImitationOption`]).
//!
//! **Afterwards.** [`Planner::replan`] gives up the plan of a point that the search failed on and
//! lays out the points after it again; [`Planner::lengthen`] stretches a plan by a quarter. A
//! length the request gives is met by compressing every point (gaps of a measure at most, heads
//! of four notes, later points of two voices) where the plan is longer, and by stretching it;
//! every plan anew keeps it, the points after a re-planned one compressed where they must be,
//! and a plan that does not fit is given up.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

use super::chant::ChantFallback;
use super::check::{self, Checks, FormulaNote, Frame, Goal, Head, Tone};
use super::points::{self, Part};
use super::subject::{self, Builder};
use super::{
    Answer, Entry, FirstNote, Imitation, ImitationPlan, OptionError, Point, PointKind, Relation,
    Soggetto, SubjectSource, Toggle,
};
use crate::clef::VoiceType;
use crate::generate::{
    CADENCE_SPACING, FreeRequest, GenerateError, MIN_MEASURES, Rng, cadence_formula, fingerprint,
    sung_profile,
};
use crate::mode::ModeSpec;
use crate::pitch::{Pitch, Step};
use crate::rules::{Level, Profile, Violation};
use crate::score::{Cadence, ScoreEntry, ScoreImitation, ScorePoint};
use crate::text::plan::{CadenceChain, Ending, plan, targets_with};
use crate::text::underlay::{Choice, gate_restarts};
use crate::text::{Stop, SylFlags, Text, TextKind};
use crate::time::Meter;

/// Ticks per 2/2 measure.
const MEASURE: u32 = 8;

/// A minim: entries begin on its beats, and a voice rests at least this long before it enters
/// again.
const MINIM: u32 = 4;

/// The entry plans drawn per point.
const SAMPLES: usize = 64;

/// The valid plans each point keeps, the first in use, the others for re-planning.
const KEPT: usize = 4;

/// How often a point's plans are drawn anew once its alternatives are spent.
const REDRAWS: u64 = 2;

/// How often a point's plans are drawn anew, with another shape, where none of the first draws
/// is valid, before the request fails: a few points have only one valid plan or two among the
/// draws of a seed.
const FRESH: u64 = 2;

/// The noise added to the costs when ranking plans, as in the search.
const NOISE: f64 = 2.0;

/// The fewest notes of a head.
const LEAST_NOTES: usize = 4;

/// The fewest notes of a head with a modal answer: four notes with a changed interval keep only
/// two of their three intervals, and without its plan a score shows such an answer as one only
/// from five agreeing notes on (`detect`).
const MODAL_NOTES: usize = 5;

/// The most notes of a head in a relaxed plan.
const RELAXED_NOTES: usize = 5;

/// The latest second entry after the leader: two breves (IMI-TIME).
const SECOND_ENTRY: u32 = 32;

/// The share of second entries a minim off the downbeat, which shorten or syncopate their first
/// note.
const MINIM_DELAYS: f64 = 0.05;

/// The gaps between later entries in ticks, with their shares (IMI-GAP; up to four breves).
const LATER_GAPS: [(u32, f64); 13] = [
    (8, 0.18),
    (16, 0.125),
    (24, 0.125),
    (32, 0.144),
    (40, 0.086),
    (48, 0.089),
    (56, 0.043),
    (64, 0.041),
    (12, 0.024),
    (20, 0.024),
    (28, 0.024),
    (36, 0.024),
    (44, 0.024),
];

/// The gaps of a restated point: closer, in stretto.
const RESTATED_GAPS: [(u32, f64); 2] = [(8, 0.5), (16, 0.5)];

/// How often a later point's leader that sings no formula of the goal before it waits a measure
/// longer (IMI-CADENCE).
const LATE: f64 = 0.15;

/// How often four voices or more enter in pairs (IMI-DUO). The degrees and the gaps drawn anyway
/// repeat a duo now and then by themselves: one plan in ten in pairs makes 20 % of the complete
/// four-voice openings repeated duos, as the detector finds in 20 % of Palestrina's (22 % in the
/// study of the corpus); one in five made 34 %.
const PAIRS: f64 = 0.1;

/// How often an answer is modal where the subject allows both (IMI-TONAL).
const MODAL: f64 = 0.25;

/// How often an entry on the minim beat shortens its first note instead of tying it over.
const SHORTENED: f64 = 0.63;

/// How often four voices open in an order outside Palestrina's usual ones.
const OTHER_ORDERS: f64 = 0.2;

/// How often an entry after the second goes to the other main degree where the first two
/// entries of its point share one degree: Palestrina goes back to it, keeping both later entries
/// of such a complete four-voice opening on the shared degree in 14 % of them, all three of a
/// five-voice one in 21 %.
const BACK: f64 = 0.5;

/// The voices of a later point unless the request gives their number, by their shares among
/// Palestrina's later points (the detector with entries after rests of a semibreve: 96, 1370,
/// 1292 and 870 later points of sections of three to six voices): mostly duos, larger points in
/// larger choirs. With every later point of two or three voices and no price on silence
/// (IMI-SILENCE), a voice of Kyrie I with four and five voices rested longer than four breves in
/// a quarter and three eighths of the settings, against 4 and 17 % of Palestrina's Kyrie
/// sections, which are longer, and four in five re-entries or more brought no subject.
fn later_counts(voices: usize) -> &'static [(usize, f64)] {
    match voices {
        ..=2 => &[(2, 1.0)],
        3 => &[(2, 0.667), (3, 0.333)],
        4 => &[(2, 0.605), (3, 0.25), (4, 0.145)],
        5 => &[(2, 0.541), (3, 0.275), (4, 0.133), (5, 0.051)],
        _ => &[(2, 0.54), (3, 0.255), (4, 0.108), (5, 0.059), (6, 0.038)],
    }
}

/// How many measures beyond its least a goal may move for the text to fit.
const ROOM: usize = 48;

/// The salt of the planner's draws, apart from those of the subjects.
const SALT: u64 = 0x504c_414e_5345_4544;

/// The four voice types the shares of the corpus count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Register {
    Soprano,
    Alto,
    Tenor,
    Bass,
}

impl Register {
    fn of(voice: VoiceType) -> Register {
        match voice {
            VoiceType::HighSoprano | VoiceType::Soprano => Register::Soprano,
            VoiceType::MezzoSoprano | VoiceType::Alto => Register::Alto,
            VoiceType::Tenor | VoiceType::Baritone => Register::Tenor,
            VoiceType::Bass => Register::Bass,
        }
    }

    /// The share of the leaders of opening points (`opening`) or of later ones of this type
    /// (IMI-LEAD).
    fn leads(self, opening: bool) -> f64 {
        match (opening, self) {
            (true, Register::Soprano) => 0.26,
            (true, Register::Alto) => 0.37,
            (true, Register::Tenor) => 0.23,
            (true, Register::Bass) => 0.14,
            (false, Register::Soprano) => 0.21,
            (false, Register::Alto) => 0.19,
            (false, Register::Tenor | Register::Bass) => 0.29,
        }
    }
}

/// The orders of four-voice openings S A T B with their counts (IMI-ORDER).
const SATB_ORDERS: [([Register; 4], f64); 7] = {
    use Register::*;
    [
        ([Alto, Soprano, Tenor, Bass], 28.0),
        ([Soprano, Alto, Tenor, Bass], 27.0),
        ([Alto, Soprano, Bass, Tenor], 21.0),
        ([Tenor, Alto, Soprano, Bass], 15.0),
        ([Bass, Tenor, Alto, Soprano], 14.0),
        ([Alto, Tenor, Soprano, Bass], 12.0),
        ([Tenor, Bass, Soprano, Alto], 10.0),
    ]
};

/// The orders of entries in pairs: the second pair an octave below the first, or above.
const PAIR_ORDERS: [[Register; 4]; 4] = {
    use Register::*;
    [
        [Soprano, Alto, Tenor, Bass],
        [Alto, Soprano, Bass, Tenor],
        [Bass, Tenor, Alto, Soprano],
        [Tenor, Bass, Soprano, Alto],
    ]
};

/// The delays of a second entry in ticks by the number of voices, with their shares.
fn second_delays(voices: usize) -> Vec<(u32, f64)> {
    let whole = match voices {
        ..=3 => [0.27, 0.15, 0.20, 0.33],
        4 => [0.38, 0.18, 0.21, 0.16],
        5 => [0.41, 0.24, 0.08, 0.06],
        _ => [0.44, 0.18, 0.08, 0.05],
    };
    let sum: f64 = whole.iter().sum();
    let mut out: Vec<(u32, f64)> = [8, 16, 24, 32]
        .into_iter()
        .zip(whole)
        .map(|(delay, share)| (delay, share / sum * (1.0 - MINIM_DELAYS)))
        .collect();
    out.extend([4, 12, 20, 28].map(|delay| (delay, MINIM_DELAYS / 4.0)));
    out
}

/// One of `items` drawn by its weight.
fn pick<T: Copy>(rng: &mut Rng, items: &[(T, f64)]) -> Option<T> {
    let total: f64 = items.iter().map(|&(_, w)| w).sum();
    let mut u = rng.next_f64() * total;
    for &(item, weight) in items {
        if u < weight {
            return Some(item);
        }
        u -= weight;
    }
    items.last().map(|&(item, _)| item)
}

/// `items` in an order drawn by their weights, the heavier earlier by preference.
fn weighted_order<T: Copy + PartialEq>(rng: &mut Rng, items: &[(T, f64)]) -> Vec<T> {
    let mut left = items.to_vec();
    let mut out = Vec::with_capacity(items.len());
    while let Some(item) = pick(rng, &left) {
        out.push(item);
        left.retain(|&(other, _)| other != item);
    }
    out
}

/// The generator of the planner's draws for point `p`, its `attempt`-th sampling.
fn generator(seed: u64, p: usize, attempt: u64) -> Rng {
    Rng::new(Rng::new(subject::sub_seed(seed, p) ^ SALT.wrapping_add(attempt)).next_u64())
}

/// Maps `f` over `items`, on all cores with the parallel search, in their order.
fn each<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync + Send) -> Vec<R> {
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        items.par_iter().map(f).collect()
    }
    #[cfg(not(feature = "parallel"))]
    {
        items.iter().map(f).collect()
    }
}

/// How a point's plans are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Fit {
    /// By the shares.
    Planned,
    /// After no planned plan was valid: heads of at most five notes, every gap a measure
    /// longer, a later point with a voice fewer.
    Relaxed,
    /// Into a length the request gives: gaps of a measure at most, heads of four notes, later
    /// points of two voices.
    Compressed,
}

/// What a plan draws for one entry.
#[derive(Clone, Debug, PartialEq)]
struct Drawn {
    voice: usize,
    /// The ticks after the entry before; 0 for the leader.
    delay: u32,
    relation: Relation,
    answer: Answer,
    /// On the minim beat its first note is shortened by a minim rather than tied over.
    shortened: bool,
    /// Which of the voice's octaves for the head, the best first.
    octave: usize,
}

/// What a plan draws for one point.
#[derive(Clone, Debug, PartialEq)]
struct Draft {
    /// The subject, by its place among the point's.
    subject: usize,
    /// A later point's leader that sings no formula waits a measure longer.
    late: bool,
    entries: Vec<Drawn>,
    fit: Fit,
}

/// What the plans of a point share where they follow Palestrina's shares, drawn once for the
/// point: the delay of the second entry, the gaps between the later ones (from the third entry
/// on), whether four voices enter in pairs, whether a later point's leader waits a measure longer
/// and how many voices a later point has. The cheapest of plans drawn one by one would come with
/// the longest gaps, which clash least: half the share of later gaps of a semibreve, a mean gap
/// of 32 ticks instead of 28.5.
#[derive(Clone, Debug, PartialEq)]
struct Shape {
    second: u32,
    gaps: Vec<u32>,
    pairs: bool,
    late: bool,
    /// The voices of a later point.
    voices: Option<usize>,
}

/// What the points laid out before a point fix: their heads, the notes of their formulas and
/// their goals with the targets.
struct Earlier<'a> {
    heads: Vec<&'a Head>,
    formulas: Vec<FormulaNote>,
    goals: Vec<(usize, Step)>,
}

impl<'a> Earlier<'a> {
    fn of(before: &[&'a Laid]) -> Earlier<'a> {
        Earlier {
            heads: before.iter().flat_map(|l| &l.heads).collect(),
            formulas: before
                .iter()
                .flat_map(|l| l.formulas.iter().copied())
                .collect(),
            goals: before.iter().filter_map(|l| l.goal).collect(),
        }
    }
}

/// How an entry sings the subject: in which relation to the leader, with which answer and
/// first note.
type Manner = (Relation, Answer, FirstNote);

/// The fewest notes of a head with `answer`.
fn least_notes(answer: Answer) -> usize {
    match answer {
        Answer::Modal { .. } => MODAL_NOTES,
        Answer::Real => LEAST_NOTES,
    }
}

/// An entry to place in an octave of its voice: the head of `subject` on `degree` with `answer`
/// and `values`, from `start` after `rest`, singing `sung`, the voice's `first` entry or a later
/// one; by preference on one of the first notes `stretto` (diatonic indices), where the second
/// entry of an opening passes the stretto test of its subject.
struct Placing<'a> {
    voice: usize,
    subject: &'a Soggetto,
    degree: i8,
    answer: Answer,
    values: &'a [u32],
    start: u32,
    rest: u32,
    sung: &'a [Choice],
    first: bool,
    stretto: &'a [i32],
    /// The notes the head keeps at most, as the request or the fit cuts it.
    keep: usize,
}

/// A point as laid out: what its plan drew, its entries with their heads, its goal or the length
/// of the setting, and what it costs.
#[derive(Clone, Debug)]
struct Laid {
    draft: Draft,
    heads: Vec<Head>,
    entries: Vec<Entry>,
    /// The internal cadence that ends the point: its measure and its target.
    goal: Option<(usize, Step)>,
    /// The length of the setting, after the last point.
    measures: Option<usize>,
    /// The notes of the formula of the point's cadence, the final one after the last point.
    formulas: Vec<FormulaNote>,
    cost: u32,
}

/// The voices that sing the formula of a cadence on `target`, by index from the top.
fn formula_voices(mode: ModeSpec, target: Step, voices: usize) -> Vec<usize> {
    let mut out: Vec<usize> = cadence_formula(mode, target, 2, voices, Meter::ALLA_BREVE)
        .iter()
        .map(|f| f.voice)
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The notes of the formula of a cadence on `target` arriving in measure `goal`.
fn formula_notes(mode: ModeSpec, target: Step, goal: usize, voices: usize) -> Vec<FormulaNote> {
    cadence_formula(mode, target, goal, voices, Meter::ALLA_BREVE)
        .iter()
        .filter_map(|f| {
            Some(FormulaNote {
                voice: f.voice,
                pitch: f.note.pitch?,
                start: f.start,
                end: f.start + f.note.duration.ticks(),
                tied_in: f.tied_in,
            })
        })
        .collect()
}

/// How the words of a point end, which decides the target of its cadence.
fn ending(text: &Text, words: &Range<u16>) -> Ending {
    let last = usize::from(words.end.saturating_sub(1));
    if text.syllables[last].flags.contains(SylFlags::UNIT_END) {
        match text.unit_of(last).stop {
            Stop::Full => Ending::Full,
            Stop::Intermediate => Ending::Intermediate,
        }
    } else {
        Ending::Break
    }
}

/// The planned pace: syllable starts per 100 minim beats, of `syllables` in `measures`.
fn pace(syllables: usize, measures: usize) -> u16 {
    let per_measure = syllables as f64 / measures.saturating_sub(2).max(1) as f64;
    (per_measure * 50.0).round().clamp(1.0, f64::from(u16::MAX)) as u16
}

/// Plans an imitative free setting of `request` (which has a text) with the choices of
/// `imitation` and the rules of `profile`: its points, their subjects and entries, the cadences
/// and the length. The planner keeps alternatives for re-planning a point. Where the planner
/// restated the first point of its own accord and that point finds no valid entries, the text's
/// last tail takes its place.
///
/// Fails with [`GenerateError::ImitationOption`] where `imitation` asks for something the choir or
/// the text cannot give, with [`GenerateError::PointCount`] where the request asks for no point
/// or for more than the text has room for, with the errors of the subjects
/// ([`GenerateError::SubjectInvalid`], [`GenerateError::NoChantPhrase`]), with
/// [`GenerateError::NoImitation`] where a point finds no valid entries, and with
/// [`GenerateError::ImitationTooLong`] where a given length leaves too little room.
pub fn plan_imitation(
    request: &FreeRequest,
    imitation: &Imitation,
    profile: &Profile,
) -> Result<Planner, GenerateError> {
    let sung = request
        .text
        .as_ref()
        .ok_or(GenerateError::ImitationNeedsText)?;
    if request.voices.len() < 2 {
        return Err(GenerateError::TooFewVoices);
    }
    let mut choir = request.voices.clone();
    choir.sort();
    let text = Arc::clone(&sung.text);
    let kind = match text.kind {
        TextKind::Short => "short",
        TextKind::Long => "long",
    };
    let restatements = profile.settings("TXT-REPEAT").level(kind) != Level::Forbidden;
    let mode = request.mode;
    let statements = plan(&text, mode, choir.len(), None, None, restatements)?.statements;
    if imitation.restate == Toggle::On && (statements < 2 || !restatements) {
        return Err(GenerateError::ImitationOption(OptionError::Restate));
    }
    // The planner's draws and the subjects' mix the request's text, mode and voices into the
    // seed: requests that differ draw differently with one seed. With the seed alone, every mode
    // would take the same drawn head shapes, and 48 settings of four texts in four modes with
    // three seeds would begin with a repeated note in 45 % of their openings, against 29 % in
    // Palestrina's.
    let seed =
        request.seed ^ fingerprint(&format!("{}|{}|{}", text.spec(), mode.id(), choir.len()));
    let points_with = |imitation: &Imitation| {
        points::points_of(
            &text,
            statements,
            restatements,
            imitation,
            &mut generator(seed, 0, u64::MAX),
        )
    };
    let lay = |parts: Vec<Part>| -> Result<Planner, GenerateError> {
        check_options(imitation, choir.len(), parts.len())?;
        let builder = Builder::new(
            Arc::clone(&text),
            mode,
            &choir,
            profile,
            imitation,
            parts.clone(),
            seed,
        )?;
        let count = parts.len();
        let restarts = gate_restarts(&text, parts.iter().map(|part| &part.words));
        let mut planner = Planner {
            ending: request.part.ending_in(mode),
            start: request.part.start,
            chain: request.part.chain.clone(),
            checks: Checks::new(
                Arc::clone(&text),
                mode,
                &choir,
                &sung_profile(profile),
                &restarts,
            ),
            builder,
            parts,
            imitation: imitation.clone(),
            seed,
            subjects: vec![Vec::new(); count],
            fallbacks: vec![None; count],
            layout: Vec::new(),
            tried: vec![Vec::new(); count],
            redrawn: vec![0; count],
            fit: Fit::Planned,
            length: None,
            too_long: None,
            samples: 0,
            valid: 0,
            rejected: BTreeMap::new(),
        };
        planner.lay_from(0)?;
        Ok(planner)
    };
    let mut parts = points_with(imitation)?;
    // The subjects given, heads or a chant's phrases, go to the points that take a subject of
    // their own; a restatement takes its point's again. More than such points: the tail instead
    // of a restatement the planner drew, else none is dropped without a word.
    let given = match &imitation.subject {
        SubjectSource::Given(heads) => heads.len(),
        SubjectSource::Chant {
            phrases: Some(phrases),
            ..
        } => phrases.len(),
        _ => 0,
    };
    let own = |parts: &[Part]| {
        parts
            .iter()
            .filter(|part| !matches!(part.kind, PointKind::Restatement { .. }))
            .count()
    };
    let drawn = |parts: &[Part]| {
        parts
            .iter()
            .any(|part| matches!(part.kind, PointKind::Restatement { .. }))
    };
    if given > own(&parts)
        && imitation.restate == Toggle::Auto
        && imitation.points.is_none()
        && drawn(&parts)
    {
        parts = points_with(&Imitation {
            restate: Toggle::Off,
            ..imitation.clone()
        })?;
    }
    if given > own(&parts) {
        return Err(GenerateError::ImitationOption(OptionError::Subjects {
            given,
            points: own(&parts),
        }));
    }
    // Whether point `point` (from 1) is the restatement the planner chose itself.
    let chosen = |point: usize| {
        imitation.restate == Toggle::Auto
            && imitation.points.is_none()
            && matches!(
                parts.get(point.wrapping_sub(1)).map(|part| part.kind),
                Some(PointKind::Restatement { .. })
            )
    };
    let mut planner = match lay(parts.clone()) {
        Err(GenerateError::NoImitation { point, .. }) if chosen(point) => {
            let tail = Imitation {
                restate: Toggle::Off,
                ..imitation.clone()
            };
            lay(points_with(&tail)?)?
        }
        laid => laid?,
    };
    if !sung.auto_length {
        planner.fit_into(request.measures)?;
    }
    Ok(planner)
}

/// The options of `imitation` that a choir of `voices` voices and `points` points cannot have:
/// a head of fewer than four notes or more than a subject has, later points of fewer than two
/// voices or more than the choir has, an option for a point the text does not have, an order with a voice the choir
/// does not have or twice, a later point's order of fewer than two voices, delays off the minim
/// beats, and more delays or relations than a point has entries after its first.
fn check_options(imitation: &Imitation, voices: usize, points: usize) -> Result<(), GenerateError> {
    let fail = |error: OptionError| Err(GenerateError::ImitationOption(error));
    // A built subject has ten notes at most, a given head twelve.
    let most = match imitation.subject {
        SubjectSource::Given(_) => *subject::GIVEN_NOTES.end(),
        _ => subject::MOST_NOTES,
    };
    if let Some(notes) = imitation
        .head
        .filter(|&notes| !(LEAST_NOTES..=most).contains(&notes))
    {
        return fail(OptionError::Head { notes, most });
    }
    if let Some(count) = imitation
        .later_voices
        .filter(|&k| !(2..=voices).contains(&k))
    {
        return fail(OptionError::LaterVoices {
            voices: count,
            most: voices,
        });
    }
    // The last point an option is given for.
    let given = |lists: &[bool]| lists.iter().rposition(|&given| given);
    let orders: Vec<bool> = imitation.orders.iter().map(Option::is_some).collect();
    let delays: Vec<bool> = imitation.delays.iter().map(Option::is_some).collect();
    let relations: Vec<bool> = imitation.relations.iter().map(Option::is_some).collect();
    let last = [given(&orders), given(&delays), given(&relations)]
        .into_iter()
        .flatten()
        .max();
    if let Some(p) = last.filter(|&p| p >= points) {
        return fail(OptionError::NoPoint {
            point: p + 1,
            points,
        });
    }
    for p in 0..points {
        let point = p + 1;
        let order = imitation.orders.get(p).and_then(Option::as_ref);
        if let Some(order) = order {
            for (i, &v) in order.iter().enumerate() {
                if v >= voices {
                    return fail(OptionError::NoVoice {
                        point,
                        voice: v + 1,
                        voices,
                    });
                }
                if order[..i].contains(&v) {
                    return fail(OptionError::VoiceTwice {
                        point,
                        voice: v + 1,
                    });
                }
            }
            if p > 0 && order.len() == 1 {
                return fail(OptionError::FewVoices { point });
            }
        }
        // The entries of the point after its first: every voice in the first point; in a later
        // one as many as its order or the request names.
        let most = match (p, order) {
            (0, _) => voices - 1,
            (_, Some(order)) if !order.is_empty() => order.len() - 1,
            _ => imitation.later_voices.unwrap_or(voices) - 1,
        };
        let delays = imitation.delays.get(p).and_then(Option::as_ref);
        if let Some(&ticks) = delays
            .into_iter()
            .flatten()
            .find(|&&ticks| ticks == 0 || !ticks.is_multiple_of(MINIM))
        {
            return fail(OptionError::Delay { point, ticks });
        }
        let relations = imitation.relations.get(p).and_then(Option::as_ref);
        let counts = [delays.map(Vec::len), relations.map(Vec::len)];
        if let Some(given) = counts.into_iter().flatten().find(|&given| given > most) {
            return fail(OptionError::TooMany { point, given, most });
        }
    }
    Ok(())
}

/// The planner of an imitative setting: the points as laid out, each with its alternatives, and
/// what it needs to lay them out again.
pub struct Planner {
    /// The target of the final cadence: the final, or the ending of a part of a mass section
    /// (phase 14).
    ending: Step,
    /// Where the leader of the first point enters: 0, or after a breath or a measure's rest at
    /// the start of a part.
    start: u32,
    /// How the goals of the points take their targets.
    chain: CadenceChain,
    checks: Checks,
    builder: Builder,
    parts: Vec<Part>,
    imitation: Imitation,
    seed: u64,
    /// Per point, its subjects, best first.
    subjects: Vec<Vec<Arc<Soggetto>>>,
    /// Per point, the warning that its chant gave no subject.
    fallbacks: Vec<Option<ChantFallback>>,
    /// Per point laid out, its valid plans: the first in use, the others cheapest first.
    layout: Vec<Vec<Laid>>,
    /// Per point, the plans given up after a failure.
    tried: Vec<Vec<Draft>>,
    /// Per point, how often its plans were drawn anew once its alternatives were spent.
    redrawn: Vec<u64>,
    fit: Fit,
    /// The length the request gives: every plan has it, stretched where it is shorter.
    length: Option<usize>,
    /// The least length of the plans that re-planning laid out but gave up for being longer than
    /// the given length.
    too_long: Option<usize>,
    /// The plans drawn and those that passed every check, for the statistics, and the rules
    /// that rejected the others.
    samples: usize,
    valid: usize,
    rejected: BTreeMap<&'static str, usize>,
}

/// What a planner has laid out, to return to: the points' plans, subjects and fallbacks
/// ([`Planner::snapshot`]).
#[derive(Clone)]
pub(crate) struct Snapshot {
    layout: Vec<Vec<Laid>>,
    subjects: Vec<Vec<Arc<Soggetto>>>,
    fallbacks: Vec<Option<ChantFallback>>,
}

impl Planner {
    /// The plan as laid out now; with a given length, stretched to it.
    pub fn plan(&self) -> ImitationPlan {
        let plan = self.assemble(&self.current(self.layout.len()));
        match self.length {
            Some(measures) => {
                debug_assert!(
                    plan.measures <= measures,
                    "a plan of {} measures for a given length of {measures}",
                    plan.measures
                );
                retime(&plan, measures)
            }
            None => plan,
        }
    }

    /// The least length of the plans that re-planning laid out but gave up because they did not
    /// fit into the length the request gives, if it gave any up so: a length the imitation
    /// could take.
    pub fn too_long(&self) -> Option<usize> {
        self.too_long
    }

    /// What the planner has laid out, to return to with [`Planner::restore`]. What it has tried
    /// since stays tried.
    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot {
            layout: self.layout.clone(),
            subjects: self.subjects.clone(),
            fallbacks: self.fallbacks.clone(),
        }
    }

    /// Returns to what the planner had laid out at `snapshot`.
    pub(crate) fn restore(&mut self, snapshot: Snapshot) {
        (self.layout, self.subjects, self.fallbacks) =
            (snapshot.layout, snapshot.subjects, snapshot.fallbacks);
    }

    /// Whether the plan as laid out fits into the given length, if there is one.
    fn fits(&self) -> bool {
        let natural = || self.assemble(&self.current(self.layout.len())).measures;
        self.length.is_none_or(|measures| natural() <= measures)
    }

    /// The number of points.
    pub fn points(&self) -> usize {
        self.parts.len()
    }

    /// The words of point `p` as written, for messages: `Ký-ri-e e-lé-i-son.`
    pub fn words(&self, p: usize) -> String {
        points::spelled(&self.checks.text, &self.parts[p].words)
    }

    /// The points whose chant gave no usable subject, which took one from their words.
    pub fn fallbacks(&self) -> Vec<ChantFallback> {
        self.fallbacks.iter().flatten().cloned().collect()
    }

    /// The plans drawn so far and how many of them passed every check.
    pub fn samples(&self) -> (usize, usize) {
        (self.samples, self.valid)
    }

    /// The rules that rejected the plans drawn so far, with how many each.
    pub fn rejected(&self) -> &BTreeMap<&'static str, usize> {
        &self.rejected
    }

    /// Lays out point `p` anew after a failure that [`ImitationPlan::owner`] put down to it: its
    /// next alternative, first its leader in another octave, then its other plans with the same
    /// subject (other orders and delays), then those with other subjects, and once these are
    /// spent plans drawn anew, twice at most. The points after it keep their plans where these
    /// still pass the checks and are drawn anew otherwise. With a length the request gives, an
    /// alternative counts only where the plan still fits into it, if need be with the points
    /// after it compressed; it is then stretched to the length. `None` once `p` has no plan left.
    pub fn replan(&mut self, p: usize) -> Option<ImitationPlan> {
        self.replan_near(p, None)
    }

    /// As [`Planner::replan`] for a failure in `measure` (from 1, as
    /// [`GenerateError::NoCounterpoint`] names it): the entry nearest to it takes another octave
    /// first.
    pub fn replan_near(&mut self, p: usize, measure: Option<usize>) -> Option<ImitationPlan> {
        if p >= self.layout.len() {
            return None;
        }
        let current = self.layout[p][0].clone();
        self.tried[p].push(current.draft.clone());
        let mut queue: Vec<Laid> = Vec::new();
        // An octave variant of the entry nearest the failure.
        let tick = measure.map_or(0, |m| m.saturating_sub(1) as u32 * MEASURE);
        let nearest = (0..current.heads.len()).min_by_key(|&i| {
            (
                measure.map_or(0, |_| current.heads[i].start().abs_diff(tick)),
                i,
            )
        });
        if let Some(i) = nearest {
            let mut variant = current.draft.clone();
            variant.entries[i].octave += 1;
            if !self.tried[p].contains(&variant) {
                let before = self.current(p);
                if let Ok(laid) = self.evaluate(p, &variant, &before) {
                    queue.push(laid);
                }
            }
        }
        // Then other orders and delays, then other subjects.
        let mut rest: Vec<Laid> = self.layout[p].drain(1..).collect();
        rest.sort_by_key(|laid| laid.draft.subject != current.draft.subject);
        queue.extend(
            rest.into_iter()
                .filter(|laid| !self.tried[p].contains(&laid.draft)),
        );
        // Once they are spent, plans drawn anew, twice at most.
        if queue.is_empty() && self.redrawn[p] < REDRAWS {
            self.redrawn[p] += 1;
            let round = self.redrawn[p];
            if let Ok(fresh) = self.sample_round(p, round) {
                queue.extend(fresh);
            }
        }
        // The first of them after which the later points find plans, within the given length.
        let kept = self.snapshot();
        while !queue.is_empty() {
            let changed = queue[0].draft.subject != current.draft.subject;
            self.layout[p] = queue.clone();
            if self.relay(p + 1, changed).is_ok() {
                if self.fits() {
                    return Some(self.plan());
                }
                // Too long for the given length: the points after it compressed.
                let natural = self.assemble(&self.current(self.layout.len())).measures;
                if self.fit != Fit::Compressed && p + 1 < self.parts.len() {
                    self.fit = Fit::Compressed;
                    let compressed = self.lay_from(p + 1);
                    self.fit = Fit::Planned;
                    if compressed.is_ok() && self.fits() {
                        return Some(self.plan());
                    }
                }
                self.too_long = Some(self.too_long.map_or(natural, |m| m.min(natural)));
            }
            self.restore(kept.clone());
            self.tried[p].push(queue.remove(0).draft);
        }
        self.layout[p] = vec![current];
        None
    }

    /// The plan a quarter longer: every point's goal and the final cadence later, the entries
    /// where they were against the goal before them, so that the voices have more room for their
    /// text after their heads.
    pub fn lengthen(&self, plan: &ImitationPlan) -> ImitationPlan {
        retime(plan, (plan.measures * 5).div_ceil(4))
    }

    /// The points laid out before point `p`, as they are in use.
    fn current(&self, p: usize) -> Vec<&Laid> {
        self.layout[..p]
            .iter()
            .map(|alternatives| &alternatives[0])
            .collect()
    }

    /// The plan of the points laid out as `laid`.
    fn assemble(&self, laid: &[&Laid]) -> ImitationPlan {
        let points: Vec<Point> = laid
            .iter()
            .enumerate()
            .map(|(p, l)| Point {
                words: self.parts[p].words.clone(),
                kind: self.parts[p].kind,
                subject: Arc::clone(&self.subjects[p][l.draft.subject]),
                entries: l.entries.clone(),
                goal: l.goal.map(|(measure, _)| measure),
            })
            .collect();
        let measures = laid.last().and_then(|l| l.measures).unwrap_or(MIN_MEASURES);
        let cadences = laid
            .iter()
            .filter_map(|l| l.goal)
            .map(|(measure, target)| Cadence { measure, target })
            .collect();
        let syllables = self.parts.iter().map(|part| part.words.len()).sum();
        ImitationPlan {
            points,
            measures,
            cadences,
            rate: pace(syllables, measures),
            cost: laid.iter().map(|l| l.cost).sum(),
            ending: self.ending,
        }
    }

    /// Lays out the points from `from` on, each with its subjects and alternatives.
    fn lay_from(&mut self, from: usize) -> Result<(), GenerateError> {
        self.layout.truncate(from);
        for p in from..self.parts.len() {
            self.find_subjects(p)?;
            let alternatives = self.sample(p)?;
            self.layout.push(alternatives);
        }
        Ok(())
    }

    /// Lays out the points from `from` on again after an earlier one changed: each keeps the
    /// plans that still pass the checks, or is drawn anew. With `subjects`, an earlier subject
    /// changed, and the later points find theirs again.
    fn relay(&mut self, from: usize, subjects: bool) -> Result<(), GenerateError> {
        for q in from..self.parts.len() {
            let old: Vec<(Arc<Soggetto>, Draft)> = self.layout[q]
                .iter()
                .map(|laid| {
                    (
                        Arc::clone(&self.subjects[q][laid.draft.subject]),
                        laid.draft.clone(),
                    )
                })
                .collect();
            if subjects {
                self.find_subjects(q)?;
            }
            let before = self.current(q);
            let kept: Vec<Laid> = old
                .into_iter()
                .filter_map(|(subject, mut draft)| {
                    draft.subject = self.subjects[q].iter().position(|s| *s == subject)?;
                    self.evaluate(q, &draft, &before).ok()
                })
                .collect();
            let alternatives = if kept.is_empty() {
                self.sample(q)?
            } else {
                kept
            };
            self.layout[q] = alternatives;
        }
        Ok(())
    }

    /// Lays out every point again compressed if the plan is longer than `measures`, the length
    /// the request gives, and fails if it still is, naming the shortest plan found; a shorter
    /// plan is stretched to it. From then on every plan has this length.
    fn fit_into(&mut self, measures: usize) -> Result<(), GenerateError> {
        let natural = self.plan().measures;
        if natural > measures {
            let laid = self.snapshot();
            self.fit = Fit::Compressed;
            let compressed = self.lay_from(0).map(|()| self.plan().measures);
            if let Ok(shorter) = compressed
                && shorter <= measures
            {
                self.length = Some(measures);
                return Ok(());
            }
            // No compressed plan that fits: the request fails with the shortest plan found.
            self.restore(laid);
            self.fit = Fit::Planned;
            let least = compressed.map_or(natural, |shorter| shorter.min(natural));
            return Err(GenerateError::ImitationTooLong { measures: least });
        }
        self.length = Some(measures);
        Ok(())
    }

    /// The subjects of point `p` after those the points before it use.
    fn find_subjects(&mut self, p: usize) -> Result<(), GenerateError> {
        let earlier: Vec<Arc<Soggetto>> = (0..p)
            .map(|q| Arc::clone(&self.subjects[q][self.layout[q][0].draft.subject]))
            .collect();
        let found = self.builder.subjects(p, &earlier)?;
        if found.subjects.is_empty() {
            return Err(self.no_imitation(p, &BTreeMap::from([("IMI-SUBJECT", 1)])));
        }
        let given = self.imitation.relations.get(p).and_then(Option::as_deref);
        if !found.subjects.iter().any(|s| offers(s, given)) {
            return Err(GenerateError::ImitationOption(OptionError::Relation {
                point: p + 1,
            }));
        }
        self.subjects[p] = found.subjects.into_iter().map(Arc::new).collect();
        self.fallbacks[p] = found.fallback;
        Ok(())
    }

    /// Point `p` cannot be planned: the error with its words, where it would begin and the
    /// rules that rejected the most plans.
    fn no_imitation(&self, p: usize, rejected: &BTreeMap<&'static str, usize>) -> GenerateError {
        let mut rules: Vec<(&'static str, usize)> = rejected
            .iter()
            .map(|(&rule, &count)| (rule, count))
            .collect();
        rules.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let measure = self
            .current(p.min(self.layout.len()))
            .last()
            .and_then(|l| l.goal)
            .map_or(1, |(goal, _)| goal + 1);
        GenerateError::NoImitation {
            point: p + 1,
            words: self.words(p),
            measure,
            rules: rules.into_iter().take(3).map(|(rule, _)| rule).collect(),
        }
    }

    /// The valid plans of point `p`, cheapest first with noise, at most [`KEPT`]: drawn by the
    /// shares, and if none is valid relaxed; where neither gives one, drawn anew up to [`FRESH`]
    /// times. The plan in use is the cheapest of those drawn in the point's shape ([`Shape`]),
    /// else the cheapest of the others whose second entry comes nearest the shape's.
    fn sample(&mut self, p: usize) -> Result<Vec<Laid>, GenerateError> {
        let first = self.sample_round(p, 0);
        if !matches!(first, Err(GenerateError::NoImitation { .. })) {
            return first;
        }
        // Rounds of their own, after those of re-planning.
        for round in REDRAWS + 1..=REDRAWS + FRESH {
            if let Ok(laid) = self.sample_round(p, round) {
                return Ok(laid);
            }
        }
        first
    }

    /// As [`Planner::sample`] with the draws of round `round`: the plans drawn anew once a
    /// point's alternatives are spent take other draws.
    fn sample_round(&mut self, p: usize, round: u64) -> Result<Vec<Laid>, GenerateError> {
        let mut rejected: BTreeMap<&'static str, usize> = BTreeMap::new();
        let fits = match self.fit {
            Fit::Compressed => vec![Fit::Compressed],
            _ => vec![Fit::Planned, Fit::Relaxed],
        };
        for (attempt, fit) in fits.into_iter().enumerate() {
            let mut rng = generator(self.seed, p, attempt as u64 + 2 * round);
            let before = self.current(p);
            let shape = self.shape(p, fit, &mut rng);
            let second = |laid: &Laid| {
                laid.entries
                    .get(1)
                    .map_or(0, |e| e.start - laid.entries[0].start)
            };
            // Each valid plan with its rank and whether it was drawn in the shape.
            let mut valid: Vec<(f64, bool, Laid)> = Vec::new();
            let mut drawn = 0;
            for shaped in [true, false] {
                let count = if shaped { SAMPLES } else { SAMPLES / 2 };
                let mut drafts: Vec<Draft> = Vec::new();
                for sample in 0..count {
                    let shape = shaped.then_some(&shape);
                    let draft = self.draw(p, sample, fit, shape, &before, &mut rng);
                    if !drafts.contains(&draft) && !self.tried[p].contains(&draft) {
                        drafts.push(draft);
                    }
                }
                let laid = each(&drafts, |draft| self.evaluate(p, draft, &before));
                for result in laid {
                    let noise = NOISE * rng.next_f64();
                    match result {
                        Ok(laid) if !valid.iter().any(|(_, _, v)| v.draft == laid.draft) => {
                            valid.push((f64::from(laid.cost) + noise, shaped, laid));
                        }
                        Ok(_) => {}
                        Err(rule) => *rejected.entry(rule).or_default() += 1,
                    }
                }
                drawn += drafts.len();
            }
            self.samples += drawn;
            self.valid += valid.len();
            for (&rule, &count) in &rejected {
                *self.rejected.entry(rule).or_default() += count;
            }
            if !valid.is_empty() {
                valid.sort_by(|a, b| a.0.total_cmp(&b.0));
                let chosen = valid
                    .iter()
                    .position(|(_, shaped, _)| *shaped)
                    .unwrap_or_else(|| {
                        (0..valid.len())
                            .min_by_key(|&i| (second(&valid[i].2).abs_diff(shape.second), i))
                            .unwrap_or(0)
                    });
                let first = valid.remove(chosen);
                return Ok(std::iter::once(first)
                    .chain(valid)
                    .take(KEPT)
                    .map(|(_, _, laid)| laid)
                    .collect());
            }
        }
        Err(self.no_imitation(p, &rejected))
    }

    /// The shape of point `p`'s plans, drawn by Palestrina's shares once for the point: the delay
    /// of the second entry, the gaps between the later ones, whether four voices enter in pairs,
    /// whether a later point's leader waits a measure longer and how many voices it has.
    fn shape(&self, p: usize, fit: Fit, rng: &mut Rng) -> Shape {
        let second = pick(rng, &self.second_delays(p, fit)).unwrap_or(MEASURE);
        let pairs = match self.imitation.pairs {
            Toggle::On => true,
            Toggle::Off => false,
            Toggle::Auto => rng.next_f64() < PAIRS,
        };
        let restated = matches!(self.parts[p].kind, PointKind::Restatement { .. });
        let gaps = (2..self.checks.choir.len())
            .map(|i| {
                if restated {
                    pick(rng, &RESTATED_GAPS).unwrap_or(MEASURE)
                } else if pairs && i == 3 {
                    second
                } else if pairs && i == 2 {
                    let least = second + MEASURE;
                    let gaps: Vec<(u32, f64)> = LATER_GAPS
                        .iter()
                        .copied()
                        .filter(|&(gap, _)| gap >= least)
                        .collect();
                    pick(rng, &gaps).unwrap_or(least)
                } else {
                    pick(rng, &LATER_GAPS).unwrap_or(MEASURE)
                }
            })
            .collect();
        Shape {
            second,
            gaps,
            pairs,
            late: p > 0 && rng.next_f64() < LATE,
            voices: (p > 0).then(|| self.later_count(rng)),
        }
    }

    /// The number of voices of a later point: as the request gives it, else drawn by
    /// Palestrina's shares.
    fn later_count(&self, rng: &mut Rng) -> usize {
        match self.imitation.later_voices {
            Some(count) => count,
            None => pick(rng, later_counts(self.checks.choir.len())).unwrap_or(2),
        }
    }

    /// The delays of point `p`'s second entry with their shares: by the number of voices,
    /// closer in a restated point, at most a measure in a compressed plan.
    fn second_delays(&self, p: usize, fit: Fit) -> Vec<(u32, f64)> {
        let mut delays = match self.parts[p].kind {
            PointKind::Restatement { .. } => RESTATED_GAPS.to_vec(),
            _ => second_delays(self.checks.choir.len()),
        };
        if fit == Fit::Compressed {
            delays.retain(|&(delay, _)| delay <= MEASURE);
        }
        delays
    }

    /// Draws plan `sample` of point `p` after the points `before`, in `shape` if that is given:
    /// its delays, its pairs and its leader's wait.
    fn draw(
        &self,
        p: usize,
        sample: usize,
        fit: Fit,
        shape: Option<&Shape>,
        before: &[&Laid],
        rng: &mut Rng,
    ) -> Draft {
        let subjects = &self.subjects[p];
        let given_relations = self.imitation.relations.get(p).cloned().flatten();
        let given_delays = self.imitation.delays.get(p).cloned().flatten();
        // The subjects that can be answered as the request asks.
        let eligible: Vec<usize> = (0..subjects.len())
            .filter(|&i| offers(&subjects[i], given_relations.as_deref()))
            .collect();
        let index = eligible
            .get(sample % eligible.len().max(1))
            .copied()
            .unwrap_or(sample % subjects.len());
        let subject = &subjects[index];
        let choir = &self.checks.choir;
        let second = shape.map(|s| s.second);
        let (order, pairs) = self.order(p, fit, shape, before, rng);
        let restated = matches!(self.parts[p].kind, PointKind::Restatement { .. });
        let mut entries: Vec<Drawn> = Vec::with_capacity(order.len());
        for (i, &voice) in order.iter().enumerate() {
            let given = i
                .checked_sub(1)
                .and_then(|k| given_relations.as_ref()?.get(k).copied())
                .filter(|r| subject.answers.iter().any(|(relation, _)| relation == r));
            let (relation, answer) = if i == 0 {
                (Relation::Same, Answer::Real)
            } else if let Some(relation) = given {
                (relation, draw_answer(subject, relation, rng))
            } else if pairs && (i == 2 || i == 3) {
                (entries[i - 2].relation, entries[i - 2].answer)
            } else if i == 1 {
                let equal = choir[voice] == choir[order[0]];
                let dominant = if equal { 0.26 } else { 0.81 };
                let relation = if rng.next_f64() < dominant {
                    Relation::Dominant
                } else {
                    Relation::Same
                };
                answered(subject, relation, rng)
            } else {
                self.later_degree(subject, &entries, voice, rng)
            };
            let given_delay = i
                .checked_sub(1)
                .and_then(|k| given_delays.as_ref()?.get(k).copied());
            let delay = match (i, given_delay, second) {
                (0, _, _) => 0,
                (_, Some(delay), _) => delay,
                (1, None, Some(delay)) => delay,
                _ => {
                    let shaped = i
                        .checked_sub(2)
                        .and_then(|k| shape.and_then(|s| s.gaps.get(k).copied()));
                    let drawn = if let Some(gap) = shaped {
                        gap
                    } else if pairs && i == 3 {
                        entries[1].delay
                    } else if pairs && i == 2 {
                        let least = entries[1].delay + MEASURE;
                        let gaps: Vec<(u32, f64)> = LATER_GAPS
                            .iter()
                            .copied()
                            .filter(|&(gap, _)| gap >= least)
                            .collect();
                        pick(rng, &gaps).unwrap_or(least)
                    } else if restated {
                        pick(rng, &RESTATED_GAPS).unwrap_or(MEASURE)
                    } else if i == 1 {
                        // In the opening, among the delays at which the subject passes the
                        // stretto test in this relation, if any.
                        let shifts = subject::shifts(self.checks.mode, subject.degree, relation);
                        let delays = self.second_delays(p, fit);
                        let passing: Vec<(u32, f64)> = delays
                            .iter()
                            .copied()
                            .filter(|&(delay, _)| {
                                subject
                                    .stretto
                                    .iter()
                                    .any(|&(shift, d, _)| d == delay && shifts.contains(&shift))
                            })
                            .collect();
                        let delays = if p == 0 && !passing.is_empty() {
                            passing
                        } else {
                            delays
                        };
                        pick(rng, &delays).unwrap_or(MEASURE)
                    } else {
                        pick(rng, &LATER_GAPS).unwrap_or(MEASURE)
                    };
                    match fit {
                        Fit::Planned => drawn,
                        Fit::Relaxed if i == 1 => (drawn + MEASURE).min(SECOND_ENTRY),
                        Fit::Relaxed => drawn + MEASURE,
                        Fit::Compressed => drawn.min(MEASURE),
                    }
                }
            };
            entries.push(Drawn {
                voice,
                delay,
                relation,
                answer,
                shortened: rng.next_f64() < SHORTENED,
                octave: 0,
            });
        }
        let late = match shape {
            Some(shape) => shape.late,
            None => p > 0 && rng.next_f64() < LATE,
        };
        Draft {
            subject: index,
            late,
            entries,
            fit,
        }
    }

    /// The degree of an entry after the second: where the first two entries share a degree, now
    /// and then the other main degree ([`BACK`]); else a tenor takes the soprano's, a bass the
    /// alto's, where that voice has entered; any other voice the leader's or the second entry's.
    fn later_degree(
        &self,
        subject: &Soggetto,
        entries: &[Drawn],
        voice: usize,
        rng: &mut Rng,
    ) -> (Relation, Answer) {
        // One draw decides both: where the first two entries share a degree, its lowest share
        // [`BACK`] goes back to the other main degree, and the rest, rescaled, makes the choice
        // below. The rule takes no draw of its own, so the draws after it (the next entries,
        // delays and orders) shift only where an entry it sends back draws its answer.
        let mut draw = rng.next_f64();
        if entries.get(1).is_some_and(|e| e.relation == Relation::Same) {
            if draw < BACK {
                return answered(subject, Relation::Dominant, rng);
            }
            draw = (draw - BACK) / (1.0 - BACK);
        }
        let choir = &self.checks.choir;
        let model = match Register::of(choir[voice]) {
            Register::Tenor => Some((Register::Soprano, 0.79)),
            Register::Bass => Some((Register::Alto, 0.63)),
            _ => None,
        }
        .and_then(|(register, share)| {
            let entered = entries
                .iter()
                .find(|e| Register::of(choir[e.voice]) == register)?;
            Some((entered, share))
        });
        match model {
            Some((entered, share)) if draw < share => (entered.relation, entered.answer),
            Some((entered, _)) => {
                // The other main degree: of a head off the main degrees, at the fifth or the
                // fourth ([`answered`]).
                let other = match entered.relation {
                    Relation::Same => Relation::Dominant,
                    _ => Relation::Same,
                };
                answered(subject, other, rng)
            }
            None if draw < 0.59 => (Relation::Same, Answer::Real),
            None => (entries[1].relation, entries[1].answer),
        }
    }

    /// The voices of point `p` in the order of their entries, and whether they enter in pairs.
    fn order(
        &self,
        p: usize,
        fit: Fit,
        shape: Option<&Shape>,
        before: &[&Laid],
        rng: &mut Rng,
    ) -> (Vec<usize>, bool) {
        let choir = &self.checks.choir;
        let n = choir.len();
        let given = self
            .imitation
            .orders
            .get(p)
            .cloned()
            .flatten()
            .filter(|order| !order.is_empty());
        if let Some(given) = given {
            let mut order: Vec<usize> = Vec::new();
            for v in given {
                if v < n && !order.contains(&v) {
                    order.push(v);
                }
            }
            if p == 0 {
                // The first point has every voice: the others follow in a random order.
                let others: Vec<(usize, f64)> = (0..n)
                    .filter(|v| !order.contains(v))
                    .map(|v| (v, 1.0))
                    .collect();
                order.extend(weighted_order(rng, &others));
            }
            return (order, false);
        }
        let voices: Vec<usize> = if p == 0 {
            (0..n).collect()
        } else {
            self.later_voices(p, fit, shape, before, rng)
        };
        let pairs = match (self.imitation.pairs, shape) {
            (Toggle::On, _) => true,
            (Toggle::Off, _) => false,
            (Toggle::Auto, Some(shape)) => shape.pairs,
            (Toggle::Auto, None) => rng.next_f64() < PAIRS,
        };
        if pairs
            && voices.len() >= 4
            && let Some(order) = self.pair_order(&voices, rng)
        {
            return (order, true);
        }
        let registers: Vec<Register> = voices.iter().map(|&v| Register::of(choir[v])).collect();
        if p == 0
            && registers
                == [
                    Register::Soprano,
                    Register::Alto,
                    Register::Tenor,
                    Register::Bass,
                ]
        {
            let order: Option<Vec<usize>> = if rng.next_f64() < OTHER_ORDERS {
                let others: Vec<Vec<usize>> = permutations(&voices)
                    .into_iter()
                    .filter(|order| {
                        let regs: Vec<Register> =
                            order.iter().map(|&v| Register::of(choir[v])).collect();
                        SATB_ORDERS.iter().all(|(usual, _)| usual[..] != regs[..])
                    })
                    .collect();
                let k = (rng.next_f64() * others.len() as f64) as usize;
                others.get(k).cloned()
            } else {
                let counts: Vec<(usize, f64)> = SATB_ORDERS
                    .iter()
                    .enumerate()
                    .map(|(k, &(_, count))| (k, count))
                    .collect();
                pick(rng, &counts).map(|k| {
                    SATB_ORDERS[k]
                        .0
                        .iter()
                        .filter_map(|&register| {
                            let at = registers.iter().position(|&r| r == register)?;
                            Some(voices[at])
                        })
                        .collect()
                })
            };
            if let Some(order) = order {
                return (order, false);
            }
        }
        // The leader by voice type, the others in a random order.
        let opening = p == 0;
        let shares: Vec<(usize, f64)> = voices
            .iter()
            .map(|&v| {
                let register = Register::of(choir[v]);
                let alike = voices
                    .iter()
                    .filter(|&&w| Register::of(choir[w]) == register)
                    .count();
                (v, register.leads(opening) / alike as f64)
            })
            .collect();
        let leader = pick(rng, &shares).unwrap_or(voices[0]);
        let others: Vec<(usize, f64)> = voices
            .iter()
            .filter(|&&v| v != leader)
            .map(|&v| (v, 1.0))
            .collect();
        let mut order = vec![leader];
        order.extend(weighted_order(rng, &others));
        (order, false)
    }

    /// The voices of a later point `p`: those that stopped singing first, the voices of the
    /// formula before it at the end of its goal's measure, the others where their last head
    /// ended; ties drawn by the shares of later leaders. As many as the delays and relations the
    /// request gives for the point need, at least.
    fn later_voices(
        &self,
        p: usize,
        fit: Fit,
        shape: Option<&Shape>,
        before: &[&Laid],
        rng: &mut Rng,
    ) -> Vec<usize> {
        let choir = &self.checks.choir;
        let n = choir.len();
        let wanted = match shape.and_then(|s| s.voices) {
            Some(count) => count,
            None => self.later_count(rng),
        };
        let given = |lists: &[Option<Vec<u32>>]| {
            lists
                .get(p)
                .and_then(Option::as_ref)
                .map_or(0, |l| l.len() + 1)
        };
        let relations = self
            .imitation
            .relations
            .get(p)
            .and_then(Option::as_ref)
            .map_or(0, |l| l.len() + 1);
        let count = match fit {
            Fit::Planned => wanted,
            Fit::Relaxed => wanted.saturating_sub(1),
            Fit::Compressed => 2,
        }
        .max(given(&self.imitation.delays))
        .max(relations)
        .clamp(2, n);
        let previous = before.last().and_then(|l| l.goal);
        let formula = previous.map_or(Vec::new(), |(_, target)| {
            formula_voices(self.checks.mode, target, n)
        });
        let out_at = |v: usize| -> u32 {
            match previous {
                Some((goal, _)) if formula.contains(&v) => goal as u32 * MEASURE + MEASURE,
                _ => before
                    .iter()
                    .flat_map(|l| &l.heads)
                    .filter(|h| h.voice == v)
                    .map(Head::end)
                    .max()
                    .unwrap_or(0),
            }
        };
        let mut times: Vec<u32> = (0..n).map(out_at).collect();
        times.sort_unstable();
        times.dedup();
        let mut out = Vec::new();
        for time in times {
            let tied: Vec<(usize, f64)> = (0..n)
                .filter(|&v| out_at(v) == time)
                .map(|v| (v, Register::of(choir[v]).leads(false)))
                .collect();
            out.extend(weighted_order(rng, &tied));
        }
        out.truncate(count);
        out
    }

    /// Four voices of `voices` that enter in pairs, with the others after them, if the voices
    /// have every type.
    fn pair_order(&self, voices: &[usize], rng: &mut Rng) -> Option<Vec<usize>> {
        let choir = &self.checks.choir;
        let k = (rng.next_f64() * PAIR_ORDERS.len() as f64) as usize;
        let mut order = Vec::new();
        for register in PAIR_ORDERS[k.min(PAIR_ORDERS.len() - 1)] {
            let v = voices
                .iter()
                .copied()
                .find(|&v| Register::of(choir[v]) == register && !order.contains(&v))?;
            order.push(v);
        }
        let others: Vec<(usize, f64)> = voices
            .iter()
            .filter(|v| !order.contains(v))
            .map(|&v| (v, 1.0))
            .collect();
        order.extend(weighted_order(rng, &others));
        Some(order)
    }

    /// Point `p` laid out by `draft` after the points `before`, with every check passed; the
    /// rule that rejects it otherwise.
    fn evaluate(&self, p: usize, draft: &Draft, before: &[&Laid]) -> Result<Laid, &'static str> {
        let mode = self.checks.mode;
        let n = self.checks.choir.len();
        let earlier = Earlier::of(before);
        let start = self.leader_enters(draft, before)?;
        let (mut heads, entries) = self.heads(p, draft, start, &earlier)?;
        self.resolve(&mut heads, &entries, &earlier)?;
        self.settle(&mut heads, &entries, &earlier)?;
        let given_second = self
            .imitation
            .delays
            .get(p)
            .and_then(Option::as_ref)
            .is_some_and(|delays| !delays.is_empty());
        self.keep_limits(&heads, &earlier, given_second)?;
        let all: Vec<&Head> = earlier.heads.iter().copied().chain(&heads).collect();
        let gates = self.gates(p, before, &heads);
        // The goal, or the end of the setting.
        let (goal, measures, formulas) = if p + 1 == self.parts.len() {
            let measure = self.final_measure(start, &all, &earlier, &gates)?;
            let formulas = formula_notes(mode, self.ending, measure, n);
            (None, Some(measure + 1), formulas)
        } else {
            let (measure, target) = self.goal(p, start, &heads, &all, &earlier, before, &gates)?;
            let formulas = formula_notes(mode, target, measure, n);
            (Some((measure, target)), None, formulas)
        };
        let laid = Laid {
            draft: draft.clone(),
            entries: entries
                .iter()
                .zip(&heads)
                .map(|(&(relation, answer, first), head)| Entry {
                    voice: head.voice,
                    start: head.start(),
                    rest: head.rest,
                    notes: head
                        .tones
                        .iter()
                        .zip(&head.sung)
                        .map(|(t, &sung)| (t.pitch, t.end - t.start, sung))
                        .collect(),
                    relation,
                    answer,
                    first,
                })
                .collect(),
            heads,
            goal,
            measures,
            formulas,
            cost: 0,
        };
        let cost = self.price(p, &laid, start, &earlier, before, &gates)?;
        Ok(Laid {
            cost: self.subjects[p][draft.subject].cost + cost,
            ..laid
        })
    }

    /// The gates of each voice for the points up to `p` ([`ScoreImitation::gates`]): the points
    /// before it laid out as `before`, its own entries with `heads`.
    fn gates(&self, p: usize, before: &[&Laid], heads: &[Head]) -> Vec<Vec<(u16, u32)>> {
        let voices = self.checks.choir.len();
        let mut out = vec![Vec::new(); voices];
        let mut set = 0;
        for (q, part) in self.parts.iter().enumerate().take(p + 1) {
            let own = part.words.start.max(set);
            set = set.max(part.words.end);
            if q == 0 || own >= part.words.end {
                continue;
            }
            // The entries of point q in time order: voice and tick.
            let entries: Vec<(usize, u32)> = match before.get(q) {
                Some(laid) if q < p => laid.entries.iter().map(|e| (e.voice, e.start)).collect(),
                _ => heads.iter().map(|h| (h.voice, h.start())).collect(),
            };
            let joined = entries.get(1).or(entries.first()).map(|&(_, tick)| tick);
            for (v, gates) in out.iter_mut().enumerate() {
                let entered = entries.iter().find(|e| e.0 == v).map(|&(_, tick)| tick);
                if let Some(tick) = entered.or(joined) {
                    gates.push((own, tick));
                }
            }
        }
        out
    }

    /// Where the leader of a point after the points `before` enters: on the minim beat after the
    /// goal before it, a measure later if it sings that goal's formula or the plan waits; the
    /// leader of the first point at the start of the setting, after a part's breath or rest.
    fn leader_enters(&self, draft: &Draft, before: &[&Laid]) -> Result<u32, &'static str> {
        let Some(previous) = before.last() else {
            return Ok(self.start);
        };
        let (goal, target) = previous.goal.ok_or("CAD-INNER-GOAL")?;
        let leader = draft.entries.first().ok_or("IMI-ENTRY")?.voice;
        let formula = formula_voices(self.checks.mode, target, self.checks.choir.len());
        let late = draft.late || formula.contains(&leader);
        Ok(goal as u32 * MEASURE + if late { MEASURE + MINIM } else { MINIM })
    }

    /// The heads of the entries `draft` draws for point `p`, its leader entering at `start`, each
    /// in the best octave of its voice, cut to the notes the request or the fit allows; with the
    /// relation, the answer (modal where the real one cannot be sung at all) and the first note
    /// of each.
    fn heads(
        &self,
        p: usize,
        draft: &Draft,
        start: u32,
        earlier: &Earlier,
    ) -> Result<(Vec<Head>, Vec<Manner>), &'static str> {
        let mode = self.checks.mode;
        let subject = &self.subjects[p][draft.subject];
        let most = match draft.fit {
            Fit::Planned => self
                .imitation
                .head
                .map_or(usize::MAX, |n| n.max(LEAST_NOTES)),
            Fit::Relaxed => RELAXED_NOTES,
            Fit::Compressed => LEAST_NOTES,
        };
        let values = subject.values();
        let sung: Vec<Choice> = subject.notes.iter().map(|note| note.sung).collect();
        let mut heads: Vec<Head> = Vec::with_capacity(draft.entries.len());
        let mut entries = Vec::with_capacity(draft.entries.len());
        let mut tick = start;
        for drawn in &draft.entries {
            tick += drawn.delay;
            if !tick.is_multiple_of(MINIM) {
                return Err("IMI-REST");
            }
            let first = if tick.is_multiple_of(MEASURE) {
                FirstNote::AsLeader
            } else if drawn.shortened && values[0] >= MEASURE {
                FirstNote::Shortened
            } else {
                FirstNote::Syncopated
            };
            let degree = match drawn.relation {
                Relation::Same => subject.degree,
                Relation::Dominant => {
                    subject::dominant(mode, subject.degree).ok_or("IMI-INTERVAL")?
                }
                Relation::Steps(k) => subject.degree + k,
            };
            let mut ticks = values.clone();
            if first == FirstNote::Shortened {
                ticks[0] -= MINIM;
            }
            let entered = earlier.heads.iter().any(|h| h.voice == drawn.voice)
                || heads.iter().any(|h| h.voice == drawn.voice);
            let others: Vec<&Head> = earlier.heads.iter().copied().chain(&heads).collect();
            // The second entry of the opening by preference at a shift that passes the
            // subject's stretto test.
            let stretto: Vec<i32> = match heads.first() {
                Some(leader) if p == 0 && heads.len() == 1 => subject
                    .stretto
                    .iter()
                    .filter(|&&(_, delay, variant)| {
                        delay == tick - leader.start()
                            && (variant == first || first == FirstNote::AsLeader)
                    })
                    .map(|&(shift, _, _)| leader.tones[0].pitch.diatonic() + i32::from(shift))
                    .collect(),
                _ => Vec::new(),
            };
            // The drawn answer, or the other one of the relation where it cannot be sung.
            let mut answers = vec![drawn.answer];
            answers.extend(
                subject
                    .answers
                    .iter()
                    .filter(|&&(r, a)| r == drawn.relation && a != drawn.answer)
                    .map(|&(_, a)| a),
            );
            let placed = answers.into_iter().find_map(|answer| {
                let entry = Placing {
                    voice: drawn.voice,
                    subject,
                    degree,
                    answer,
                    values: &ticks,
                    start: tick,
                    rest: if entered { MINIM } else { 0 },
                    sung: &sung,
                    first: !entered,
                    stretto: &stretto,
                    keep: most.max(least_notes(answer)),
                };
                Some((self.place(&entry, &others, drawn.octave)?, answer))
            });
            let (head, answer) = placed.ok_or("VOI-RANGE")?;
            let least = least_notes(answer);
            if head.tones.len() < least {
                return Err("IMI-ANSWER");
            }
            heads.push(head.cut(most.max(least)));
            entries.push((drawn.relation, answer, first));
        }
        Ok((heads, entries))
    }

    /// Ends the heads of a point before the notes at which a pair check fails, never below
    /// their least notes; the rule of a conflict no cut resolves.
    fn resolve(
        &self,
        heads: &mut [Head],
        entries: &[Manner],
        earlier: &Earlier,
    ) -> Result<(), &'static str> {
        loop {
            let found = {
                let all: Vec<&Head> = earlier.heads.iter().copied().chain(heads.iter()).collect();
                let frame = Frame {
                    choir: &self.checks.choir,
                    heads: &all,
                };
                first_conflict(&frame, heads, &earlier.heads, &earlier.formulas)
            };
            let Some(Conflicting { conflict, cuts, .. }) = found else {
                return Ok(());
            };
            let cut = cuts
                .into_iter()
                .flatten()
                .filter(|&(i, k)| k >= least_notes(entries[i].1) && k < heads[i].tones.len())
                .min_by_key(|&(i, _)| (heads[i].start(), i));
            let Some((i, k)) = cut else {
                return Err(conflict.rule());
            };
            heads[i] = heads[i].cut(k);
        }
    }

    /// The limits of a point's entries: heads within an octave, the second entry within the
    /// leader's head unless the request gives its delay (IMI-TIME prices it then), voices
    /// entering together a third or a sixth apart, and each voice after its last head and its
    /// formula notes with the rest before its entry.
    fn keep_limits(
        &self,
        heads: &[Head],
        earlier: &Earlier,
        given_second: bool,
    ) -> Result<(), &'static str> {
        if heads.iter().any(|h| h.span() > subject::OCTAVE) {
            return Err("IMI-SUBJECT");
        }
        if heads.len() >= 2 && heads[1].start() >= heads[0].end() && !given_second {
            return Err("IMI-TIME");
        }
        for (i, a) in heads.iter().enumerate() {
            for b in &heads[i + 1..] {
                let apart =
                    (a.tones[0].pitch.diatonic() - b.tones[0].pitch.diatonic()).rem_euclid(7);
                if a.start() == b.start() && !matches!(apart, 2 | 5) {
                    return Err("TEX-PARALLEL");
                }
            }
        }
        for head in heads {
            let previous = earlier
                .heads
                .iter()
                .filter(|h| h.voice == head.voice)
                .map(|h| h.end())
                .chain(
                    earlier
                        .formulas
                        .iter()
                        .filter(|f| f.voice == head.voice && f.start < head.start())
                        .map(|f| f.end),
                )
                .max();
            if previous.is_some_and(|end| end + head.rest > head.start()) {
                return Err("IMI-REST");
            }
        }
        Ok(())
    }

    /// What the checks of point `p` laid out as `laid` charge (C1–C4, C6–C8), its leader entering
    /// at `start`; the rule that rejects it.
    fn price(
        &self,
        p: usize,
        laid: &Laid,
        start: u32,
        earlier: &Earlier,
        before: &[&Laid],
        gates: &[Vec<(u16, u32)>],
    ) -> Result<u32, &'static str> {
        let checks = &self.checks;
        let mode = checks.mode;
        let n = checks.choir.len();
        let heads = &laid.heads;
        let all: Vec<&Head> = earlier.heads.iter().copied().chain(heads).collect();
        let end = match (laid.goal, laid.measures) {
            (Some((measure, _)), _) => measure as u32 * MEASURE,
            (None, Some(measures)) => (measures as u32 - 1) * MEASURE,
            (None, None) => return Err("CAD-INNER-GOAL"),
        };
        // C1.
        if !heads.iter().all(|h| checks.rhythms.fit(h)) {
            return Err("FREE-RHYTHM");
        }
        // C2.
        let mut alone: Vec<Violation> = Vec::new();
        let mut melody = 0;
        for head in heads {
            for v in checks.melody(head, head.rest == 0) {
                match v.level {
                    Level::Forbidden => return Err(v.rule),
                    level => melody += level.cost(),
                }
                alone.push(v);
            }
        }
        // C3.
        let from = points::state_before(&checks.model, &self.parts, p);
        let mut text = 0;
        for head in heads {
            text += checks.declaim(head, from)?;
        }
        // C4.
        let goals: Vec<Goal> = earlier
            .goals
            .iter()
            .copied()
            .chain(laid.goal)
            .map(|(measure, target)| Goal {
                tick: measure as u32 * MEASURE,
                voices: formula_voices(mode, target, n),
                semitone: mode
                    .clausula_semitone(Pitch::new(target, mode.scale_alter(target), 4))
                    .map(|p| p.step),
            })
            .collect();
        let last = laid.measures.map(|_| Goal {
            tick: end,
            voices: formula_voices(mode, self.ending, n),
            semitone: None,
        });
        let own: Vec<&Head> = heads.iter().collect();
        if !check::clear_of_formulas(&own, &goals, last.as_ref()) {
            return Err("CAD-INNER-CLAUSULA");
        }
        // C7, from each voice's head before to its entry here, the words of the points between
        // them not before these begin; after the last heads the final measure saw to it.
        let mut formulas = earlier.formulas.clone();
        formulas.extend(laid.formulas.iter().copied());
        let total = end + 4 * MEASURE;
        let mut crowding = 0;
        for head in heads {
            let previous = earlier
                .heads
                .iter()
                .filter(|h| h.voice == head.voice)
                .max_by_key(|h| h.start());
            if let Some(previous) = previous {
                let gates = &gates[head.voice];
                crowding += checks.reach(previous, head, &all, &formulas, total, gates)?;
            }
        }
        // C8, from the last voice's first entry on.
        let entered = (0..n)
            .filter_map(|v| all.iter().filter(|h| h.voice == v).map(|h| h.start()).min())
            .max()
            .unwrap_or(0);
        let (fewer, half) = check::thin_downbeats(&all, n, start.max(entered), end);
        let texture = fewer * checks.checker.level("FREE-TEXTURE", "level").cost()
            + half * checks.checker.level("IMI-SILENCE", "thin").cost();
        // C6.
        let mut plan = self.score_plan(before);
        plan.points.push(ScorePoint {
            words: self.parts[p].words.clone(),
            entries: heads.iter().map(score_entry).collect(),
            goal: laid.goal.map(|(measure, _)| measure),
        });
        let heads_end = heads.iter().map(Head::end).max().unwrap_or(start);
        let total = (heads_end.div_ceil(MEASURE) + 1) * MEASURE;
        let skeleton = checks.skeleton(&all, &plan, start, total, &alone)?;
        Ok(melody + text + crowding + texture + skeleton + degree_cost(heads))
    }

    /// The plan of the points `before` as the score keeps it.
    fn score_plan(&self, before: &[&Laid]) -> ScoreImitation {
        ScoreImitation {
            points: before
                .iter()
                .enumerate()
                .map(|(q, l)| ScorePoint {
                    words: self.parts[q].words.clone(),
                    entries: l.heads.iter().map(score_entry).collect(),
                    goal: l.goal.map(|(measure, _)| measure),
                })
                .collect(),
        }
    }

    /// The head of an `entry` in the octave of rank `rank` among those its voice can take, judged
    /// by the notes it keeps. First come the octaves where these lie within the voice's range as
    /// far as FREE-AMBITUS and VOI-RANGE leave it free: from a step below its octave of the mode
    /// to a step above it, and on its staff or a step above. Only where no octave is, those within
    /// the staff but beyond the octave of the mode, then those a step beyond the staff, and only
    /// where none of these is, as far as the voice's pitches reach (three steps beyond the staff),
    /// at what VOI-RANGE charges ([`Planner::range_tier`]). Among equals, the second entry of the
    /// opening by preference at a shift that passes the stretto test, then at the least cost of
    /// the head checked alone and of crossing the heads of other voices (`others`) that sound
    /// with it (VOI-CROSSING), its first note nearest the middle of the voice. An octave other
    /// than the best (`rank` from 1, for re-planning) only within the range. The head sings its
    /// subject's pitches as the entry's shift from the leader and its answer give them
    /// ([`Soggetto::pitches`]): a given head keeps its accidentals.
    fn place(&self, entry: &Placing, others: &[&Head], rank: usize) -> Option<Head> {
        let checks = &self.checks;
        let mode = checks.mode;
        let bottom = checks.choir[entry.voice].clef().bottom_line().diatonic();
        let crossing = checks
            .checker
            .level(
                "VOI-CROSSING",
                if checks.choir.len() >= 4 {
                    "four_voices"
                } else {
                    "level"
                },
            )
            .cost();
        // The leader's first note and this entry's, in the lowest octave.
        let leader = mode.final_pitch(0).diatonic() + i32::from(entry.subject.degree);
        let base = mode.final_pitch(0).diatonic() + i32::from(entry.degree);
        let mut candidates: Vec<((u8, bool, u32, i32), Head)> = Vec::new();
        for octave in 0..10 {
            let pitches = i8::try_from(base + 7 * octave - leader)
                .ok()
                .and_then(|shift| entry.subject.pitches(mode, leader, shift, entry.answer));
            let Some(pitches) = pitches else {
                continue;
            };
            let mut tick = entry.start;
            let tones: Vec<Tone> = pitches
                .iter()
                .zip(entry.values)
                .map(|(&pitch, &value)| {
                    let tone = Tone {
                        pitch,
                        start: tick,
                        end: tick + value,
                    };
                    tick += value;
                    tone
                })
                .collect();
            let head = Head {
                voice: entry.voice,
                rest: entry.rest,
                tones,
                sung: entry.sung.to_vec(),
            };
            // Judged by the notes it keeps.
            let kept = head.cut(entry.keep);
            let Some((tier, cost)) = self.range_tier(&kept, entry.first) else {
                continue;
            };
            let crossed = kept
                .tones
                .iter()
                .map(|t| {
                    others
                        .iter()
                        .filter(|h| h.voice != entry.voice)
                        .flat_map(|h| h.tones.iter().map(move |u| (h.voice, u)))
                        .filter(|(v, u)| {
                            u.start < t.end
                                && t.start < u.end
                                && ((*v < entry.voice && u.pitch.midi() < t.pitch.midi())
                                    || (*v > entry.voice && u.pitch.midi() > t.pitch.midi()))
                        })
                        .count() as u32
                })
                .sum::<u32>();
            let cost = cost + crossed * crossing;
            let d = pitches[0].diatonic();
            let nearness = (bottom + 3 - d).max(d - bottom - 5).max(0);
            let stretto = entry.stretto.is_empty() || entry.stretto.contains(&d);
            candidates.push(((tier, !stretto, cost, nearness), head));
        }
        candidates.sort_by_key(|(key, _)| *key);
        // Another octave than the best only within the range.
        candidates
            .into_iter()
            .nth(rank)
            .filter(|((tier, ..), _)| rank == 0 || *tier == 0)
            .map(|(_, head)| head)
    }

    /// How far a head lies beyond its voice's range, checked alone in its voice as a voice's
    /// `first` entry or a later one, with what the check costs: 0 within it (no case of
    /// FREE-AMBITUS or VOI-RANGE), 1 within the staff beyond the octave of the mode, 2 within a
    /// step of the staff, 3 further, as far as the voice's pitches reach (three steps beyond the
    /// staff). `None` beyond them, or where the check forbids the head.
    fn range_tier(&self, head: &Head, first: bool) -> Option<(u8, u32)> {
        let clef = self.checks.choir[head.voice].clef();
        let (bottom, top) = (clef.bottom_line().diatonic(), clef.top_line().diatonic());
        let steps = || head.tones.iter().map(|t| t.pitch.diatonic());
        let (low, high) = (steps().min()?, steps().max()?);
        if low < bottom - 3 || high > top + 3 {
            return None;
        }
        let cases = self.checks.melody(head, first);
        if cases.iter().any(Violation::is_forbidden) {
            return None;
        }
        let ambitus = cases.iter().any(|v| v.rule == "FREE-AMBITUS");
        let staff = cases.iter().any(|v| v.rule == "VOI-RANGE");
        let tier = match (staff, ambitus) {
            (false, false) => 0,
            (false, true) => 1,
            (true, _) if low >= bottom - 1 && high <= top + 1 => 2,
            (true, _) => 3,
        };
        Some((tier, cases.iter().map(|v| v.level.cost()).sum()))
    }

    /// Moves the heads that the cuts of [`Planner::resolve`] left beyond their voices' ranges,
    /// each an octave up or down where it then lies further within the range and the heads still
    /// keep clear of each other: a head is placed by its notes before the cuts. Rejects the
    /// layout (VOI-RANGE) where a head stays more than a step beyond its staff because the octave
    /// within a step of it clashes with the other heads.
    fn settle(
        &self,
        heads: &mut Vec<Head>,
        entries: &[Manner],
        earlier: &Earlier,
    ) -> Result<(), &'static str> {
        for i in 0..heads.len() {
            let first = heads[i].rest == 0;
            let Some((tier, _)) = self.range_tier(&heads[i], first).filter(|t| t.0 > 0) else {
                continue;
            };
            let mut blocked = false;
            for octave in [7, -7] {
                let moved = heads[i].octave(octave);
                let Some((better, _)) = self.range_tier(&moved, first).filter(|t| t.0 < tier)
                else {
                    continue;
                };
                let mut trial = heads.clone();
                trial[i] = moved;
                if self.resolve(&mut trial, entries, earlier).is_ok() {
                    *heads = trial;
                    blocked = false;
                    break;
                }
                blocked |= tier == 3 && better <= 2;
            }
            if blocked {
                return Err("VOI-RANGE");
            }
        }
        Ok(())
    }

    /// The goal of point `p` with the `heads` of its entries, its leader entering at `start`
    /// (see the module's time line), and its target. A formula voice that begins the point's own
    /// words after its last head begins them only from where the point begins (`gates`).
    #[allow(clippy::too_many_arguments)]
    fn goal(
        &self,
        p: usize,
        start: u32,
        heads: &[Head],
        all: &[&Head],
        earlier: &Earlier,
        before: &[&Laid],
        gates: &[Vec<(u16, u32)>],
    ) -> Result<(usize, Step), &'static str> {
        let checks = &self.checks;
        let mode = checks.mode;
        let n = checks.choir.len();
        let mut endings: Vec<(usize, Ending)> = before
            .iter()
            .enumerate()
            .filter_map(|(q, l)| Some((l.goal?.0, ending(&checks.text, &self.parts[q].words))))
            .collect();
        endings.push((0, ending(&checks.text, &self.parts[p].words)));
        let count = endings.len();
        // The targets of the goals before it and its own, as the chain gives them: by index, so
        // the points before keep theirs.
        let target = targets_with(mode, endings, &self.chain)
            .get(count - 1)
            .map(|c| c.target)
            .ok_or("CAD-INNER-GOAL")?;
        let formula = formula_voices(mode, target, n);
        let mut least = (start / MEASURE) as usize + CADENCE_SPACING;
        if let Some((previous, _)) = before.last().and_then(|l| l.goal) {
            least = least.max(previous + CADENCE_SPACING);
        }
        for head in heads {
            let room = if formula.contains(&head.voice) {
                2 * MEASURE
            } else {
                MEASURE
            };
            least = least.max((head.end() + room).div_ceil(MEASURE) as usize);
        }
        let words_end = self.parts[p].words.end - 1;
        let target_pitch = Pitch::new(target, mode.scale_alter(target), 4);
        for measure in least..least + ROOM {
            let tick = measure as u32 * MEASURE;
            // The cantizans holds the target into the penultimate measure: no head may sound
            // a second or a seventh against it just before.
            let prepared = heads.iter().all(|head| {
                head.tones.iter().all(|t| {
                    let before = tick - MEASURE - 1;
                    !(t.start <= before && before < t.end && check::harsh(t.pitch, target_pitch))
                })
            });
            if !prepared {
                continue;
            }
            let mut formulas = earlier.formulas.clone();
            formulas.extend(formula_notes(mode, target, measure, n));
            let total = tick + 4 * MEASURE;
            let fits = formula.iter().all(|&f| {
                let Some(last) = all
                    .iter()
                    .filter(|h| h.voice == f)
                    .max_by_key(|h| h.start())
                else {
                    return true;
                };
                let Some(z) = last.last_syllable() else {
                    return true;
                };
                let needed = if z == words_end {
                    Some(0)
                } else {
                    checks.needed(z, Some(words_end)).map(|k| k + 1)
                };
                let Some(needed) = needed else {
                    return false;
                };
                let capacity = checks.capacity(f, all, &formulas, total);
                let places =
                    |from: u32| capacity.at(from).saturating_sub(capacity.at(tick + MINIM));
                // The words of the point after the voice's head, gated where the point begins.
                let way: Vec<u16> = (z + 1..=words_end).collect();
                5 * needed <= 4 * places(last.end())
                    && check::gated(&way, &gates[f], last.end())
                        .into_iter()
                        .all(|(gated, from)| 5 * gated <= 4 * places(from))
            });
            if fits {
                return Ok((measure, target));
            }
        }
        Err("TXT-COMPLETE")
    }

    /// The measure of the final cadence (from 0): two measures after the last head, three after
    /// the last goal, where every voice can begin the rest of the text within four fifths of its
    /// minim beats, the words of each later point from where that point begins (`gates`).
    fn final_measure(
        &self,
        start: u32,
        all: &[&Head],
        earlier: &Earlier,
        gates: &[Vec<(u16, u32)>],
    ) -> Result<usize, &'static str> {
        let checks = &self.checks;
        let mode = checks.mode;
        let n = checks.choir.len();
        let mut least = ((start / MEASURE) as usize + CADENCE_SPACING).max(MIN_MEASURES - 1);
        if let Some(&(goal, _)) = earlier.goals.last() {
            least = least.max(goal + CADENCE_SPACING);
        }
        let end = all.iter().map(|h| h.end()).max().unwrap_or(0);
        least = least.max((end + 2 * MEASURE).div_ceil(MEASURE) as usize);
        for measure in least..least + ROOM {
            let total = (measure as u32 + 1) * MEASURE;
            let mut formulas = earlier.formulas.clone();
            formulas.extend(formula_notes(mode, self.ending, measure, n));
            let fits = (0..n).all(|v| {
                let Some(last) = all
                    .iter()
                    .filter(|h| h.voice == v)
                    .max_by_key(|h| h.start())
                else {
                    return true;
                };
                let Some(z) = last.last_syllable() else {
                    return true;
                };
                let Some(needed) = checks.needed(z, None) else {
                    return false;
                };
                let capacity = checks.capacity(v, all, &formulas, total);
                let state = checks.model.after_rest(Some(usize::from(z)));
                let n = checks.text.syllables.len() as u16;
                let way = checks.model.way(&state, n).unwrap_or_default();
                5 * needed <= 4 * capacity.at(last.end())
                    && check::gated(&way, &gates[v], last.end())
                        .into_iter()
                        .all(|(gated, from)| 5 * gated <= 4 * capacity.at(from))
            });
            if fits {
                return Ok(measure);
            }
        }
        Err("TXT-COMPLETE")
    }
}

/// The plan's entry as the score keeps it.
fn score_entry(head: &Head) -> ScoreEntry {
    ScoreEntry {
        voice: head.voice,
        start: head.start(),
        rest: head.rest,
        notes: head
            .tones
            .iter()
            .map(|t| (t.pitch, t.end - t.start))
            .collect(),
    }
}

/// A certain conflict of a point's heads: where it is decided, what it is and, for each head of
/// the point involved, its place among the point's heads and the note before which it could end.
struct Conflicting {
    tick: u32,
    conflict: check::Conflict,
    cuts: [Option<(usize, usize)>; 2],
}

/// The first certain conflict of a point's `heads` among themselves, with the `earlier` heads
/// and with the formulas before them.
fn first_conflict(
    frame: &Frame,
    heads: &[Head],
    earlier: &[&Head],
    formulas: &[FormulaNote],
) -> Option<Conflicting> {
    let lowest = frame.choir.len().saturating_sub(1);
    let mut found: Option<Conflicting> = None;
    let mut note = |next: Conflicting| {
        if found
            .as_ref()
            .is_none_or(|f| (next.tick, next.conflict) < (f.tick, f.conflict))
        {
            found = Some(next);
        }
    };
    for (i, head) in heads.iter().enumerate() {
        if let Some(tick) = check::against_formulas(head, formulas, lowest) {
            note(Conflicting {
                tick,
                conflict: check::Conflict::Formula,
                cuts: [None, None],
            });
        }
        for (j, other) in heads.iter().enumerate().skip(i + 1) {
            if let Some(clash) = frame.clash(head, other) {
                note(Conflicting {
                    tick: clash.tick,
                    conflict: clash.conflict,
                    cuts: [Some((i, clash.before[0])), Some((j, clash.before[1]))],
                });
            }
        }
        for other in earlier
            .iter()
            .filter(|e| e.start() < head.end() && head.start() < e.end())
        {
            if let Some(clash) = frame.clash(head, other) {
                note(Conflicting {
                    tick: clash.tick,
                    conflict: clash.conflict,
                    cuts: [Some((i, clash.before[0])), None],
                });
            }
        }
    }
    found
}

/// The orders of `items`, every one.
fn permutations(items: &[usize]) -> Vec<Vec<usize>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut out = Vec::new();
    for (i, &first) in items.iter().enumerate() {
        let mut rest = items.to_vec();
        rest.remove(i);
        for mut tail in permutations(&rest) {
            tail.insert(0, first);
            out.push(tail);
        }
    }
    out
}

/// Whether `subject` can be answered in every relation the request gives for a point, if it
/// gives any.
fn offers(subject: &Soggetto, given: Option<&[Relation]>) -> bool {
    given.is_none_or(|given| {
        given
            .iter()
            .all(|r| subject.answers.iter().any(|(relation, _)| relation == r))
    })
}

/// The answer of an entry in `relation` among the subject's: modal now and then where it
/// allows a real and a modal one.
fn draw_answer(subject: &Soggetto, relation: Relation, rng: &mut Rng) -> Answer {
    let real = subject.answers.contains(&(relation, Answer::Real));
    let modal = subject
        .answers
        .iter()
        .find(|(r, a)| *r == relation && matches!(a, Answer::Modal { .. }))
        .map(|&(_, a)| a);
    match (real, modal) {
        (true, Some(modal)) if rng.next_f64() < MODAL => modal,
        (true, _) => Answer::Real,
        (false, Some(modal)) => modal,
        (false, None) => Answer::Real,
    }
}

/// What the first notes of a point's entries cost (`heads` in the order of the entries), after
/// Palestrina's complete openings by the program's detector: 1 for each entry at the pitch of two
/// entries before it (three or more entries at one pitch: 11–12 % of complete five-voice
/// openings, häufig by the Leitlinie), 1 for three entries all on one degree (12.1 % of
/// three-voice openings) and 2 for four or more (2.7 % of complete four-voice openings, 3.5 % of
/// five-voice ones, gelegentlich). The draws make such points often enough; the cost keeps the
/// cheapest plans from preferring them.
fn degree_cost(heads: &[Head]) -> u32 {
    let firsts: Vec<Pitch> = heads.iter().map(|h| h.tones[0].pitch).collect();
    let stacked = (2..firsts.len())
        .filter(|&i| firsts[..i].iter().filter(|&&p| p == firsts[i]).count() >= 2)
        .count() as u32;
    let one_degree = firsts.len() >= 3 && firsts.iter().all(|p| p.step == firsts[0].step);
    let degree = match (one_degree, firsts.len()) {
        (false, _) => 0,
        (true, 3) => 1,
        (true, _) => 2,
    };
    stacked + degree
}

/// An entry in `relation` with an answer the subject has, at the unison or octave where it has
/// none in that relation. A head off the main degrees (from chant or given notes) has no other
/// main degree to answer on: it answers at the fifth above or the fourth below instead, as
/// Palestrina answers chant heads at the fifth and the fourth (docs/regeln.md § 18, Choral), so
/// that its entries do not all sing one degree.
fn answered(subject: &Soggetto, relation: Relation, rng: &mut Rng) -> (Relation, Answer) {
    if subject.answers.iter().any(|(r, _)| *r == relation) {
        return (relation, draw_answer(subject, relation, rng));
    }
    let steps: Vec<Relation> = [Relation::Steps(4), Relation::Steps(-3)]
        .into_iter()
        .filter(|r| subject.answers.iter().any(|(s, _)| s == r))
        .collect();
    match (relation, steps.as_slice()) {
        (Relation::Dominant, [only]) => (*only, draw_answer(subject, *only, rng)),
        (Relation::Dominant, [up, down]) => {
            let other = if rng.next_f64() < 0.5 { *up } else { *down };
            (other, draw_answer(subject, other, rng))
        }
        _ => (Relation::Same, Answer::Real),
    }
}

/// The plan stretched to `measures`: the measures added are shared out among the points, each
/// goal and the final cadence moving later, and each point's entries with the goal before it.
fn retime(plan: &ImitationPlan, measures: usize) -> ImitationPlan {
    let count = plan.points.len().max(1);
    let extra = measures.saturating_sub(plan.measures);
    let share = |p: usize| extra / count + usize::from(p >= count - extra % count);
    let mut out = plan.clone();
    let mut shift = 0;
    for (p, point) in out.points.iter_mut().enumerate() {
        for entry in &mut point.entries {
            entry.start += shift as u32 * MEASURE;
        }
        shift += share(p);
        if let Some(goal) = &mut point.goal {
            *goal += shift;
        }
    }
    let goals: Vec<usize> = out.points.iter().filter_map(|point| point.goal).collect();
    for (cadence, goal) in out.cadences.iter_mut().zip(goals) {
        cadence.measure = goal;
    }
    out.measures = plan.measures + extra;
    let syllables = plan.points.iter().map(|point| point.words.len()).sum();
    out.rate = pace(syllables, out.measures);
    out
}

impl ImitationPlan {
    /// The point a failure of the search in `measure` (from 1, as
    /// [`GenerateError::NoCounterpoint`] names it) is put down to: the point of the planned entry
    /// nearest the measure's start, from there up to two measures on; otherwise the point that
    /// spans the measure, from two measures before its first entry to two after its last head;
    /// otherwise the last point.
    pub fn owner(&self, measure: usize) -> usize {
        let tick = measure.saturating_sub(1) as u32 * MEASURE;
        let entries = self
            .points
            .iter()
            .enumerate()
            .flat_map(|(p, point)| point.entries.iter().map(move |e| (p, e.start)));
        if let Some((p, _)) = entries
            .filter(|&(_, start)| tick <= start && start <= tick + 2 * MEASURE)
            .min_by_key(|&(p, start)| (start - tick, p))
        {
            return p;
        }
        let spans = self.points.iter().enumerate().rev().find(|(_, point)| {
            let start = point.entries.iter().map(|e| e.start).min().unwrap_or(0);
            let end = point
                .entries
                .iter()
                .map(|e| e.start + e.notes.iter().map(|&(_, ticks, _)| ticks).sum::<u32>())
                .max()
                .unwrap_or(start);
            start.saturating_sub(2 * MEASURE) <= tick && tick <= end + 2 * MEASURE
        });
        spans.map_or(self.points.len().saturating_sub(1), |(p, _)| p)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::generate::{Generated, cadences_at, free_search, generate_counterpoint};
    use crate::imitation::test_support::{choir, head, request, text};
    use crate::imitation::{AnswerChoice, Origin, SubjectNote};
    use crate::score::{Score, Species};
    use crate::text::plan::targets;

    fn dorian() -> ModeSpec {
        "dorian".parse().unwrap()
    }

    fn planner(spec: &str, mode: &str, voices: usize, seed: u64) -> Planner {
        plan_imitation(
            &request(spec, mode, voices, seed),
            &Imitation::default(),
            &Profile::fux(),
        )
        .unwrap()
    }

    /// The search of a plan without its text, as the driver's probe runs it: beam `beam`, no
    /// widening, one result.
    fn probe(
        plan: &ImitationPlan,
        mode: ModeSpec,
        voices: &[VoiceType],
        beam: usize,
    ) -> Result<Vec<Generated>, GenerateError> {
        let mut base = Score::new(mode, Vec::new());
        base.cadences = plan.cadences.clone();
        base.imitation = Some(Arc::new(plan.score_imitation()));
        let request = FreeRequest::new(mode, voices.to_vec(), plan.measures);
        let fixed = plan.fixed(mode, voices.len());
        let search = free_search(&request, base, plan.measures, fixed, None, (beam, 1, 1));
        generate_counterpoint(&search, &Profile::fux())
    }

    /// The ticks a head lasts.
    fn ticks(entry: &Entry) -> u32 {
        entry.notes.iter().map(|&(_, ticks, _)| ticks).sum()
    }

    /// The limits that every plan keeps (docs/regeln.md § 18): every voice in the first point;
    /// entries on minim beats, one at a time, after the silence before them and a rest of a
    /// minim after the voice's last head or formula note; a second entry within two breves and
    /// before the leader's head ends; heads of four notes or more (five with a modal answer)
    /// within an octave, all with the leader's syllables; formula windows and the step of the
    /// clausula's semitone free of heads; goals three measures apart; the plan fits a score of
    /// its length.
    fn assert_limits(plan: &ImitationPlan, mode: ModeSpec, voices: usize, label: &str) {
        let first: BTreeSet<usize> = plan.points[0].entries.iter().map(|e| e.voice).collect();
        assert_eq!(
            first,
            (0..voices).collect(),
            "{label}: every voice in point 1"
        );
        let mut sung: Vec<Vec<(u32, u32)>> = vec![Vec::new(); voices];
        let mut formulas: Vec<FormulaNote> =
            formula_notes(mode, plan.ending, plan.measures - 1, voices);
        for cadence in &plan.cadences {
            formulas.extend(formula_notes(mode, cadence.target, cadence.measure, voices));
        }
        for (p, point) in plan.points.iter().enumerate() {
            let label = format!("{label} point {}", p + 1);
            let leader = &point.entries[0];
            for (i, entry) in point.entries.iter().enumerate() {
                let at = format!("{label} entry {}", i + 1);
                assert_eq!(entry.start % MINIM, 0, "{at}: off the minim beats");
                let least = least_notes(entry.answer);
                assert!(
                    entry.notes.len() >= least,
                    "{at}: {} notes",
                    entry.notes.len()
                );
                let range: Vec<i32> = entry.notes.iter().map(|n| n.0.diatonic()).collect();
                let span = range.iter().max().unwrap() - range.iter().min().unwrap();
                assert!(span <= 7, "{at}: beyond an octave");
                assert!(
                    matches!(entry.notes[0].2, Choice::Start(y) if y == point.words.start),
                    "{at}: the entry begins the point's words"
                );
                for (k, note) in entry.notes.iter().enumerate() {
                    if let Some(theirs) = leader.notes.get(k) {
                        assert_eq!(note.2, theirs.2, "{at}: note {k} sings as the leader's");
                    }
                }
                let before = sung[entry.voice]
                    .iter()
                    .map(|&(_, end)| end)
                    .chain(
                        formulas
                            .iter()
                            .filter(|f| f.voice == entry.voice && f.start < entry.start)
                            .map(|f| f.end),
                    )
                    .max();
                match before {
                    None => assert_eq!(entry.rest, 0, "{at}: a first entry"),
                    Some(end) => {
                        assert!(entry.rest >= MINIM, "{at}: rest {}", entry.rest);
                        assert!(end + entry.rest <= entry.start, "{at}: no room to rest");
                    }
                }
                sung[entry.voice].push((entry.start, entry.start + ticks(entry)));
            }
            let starts: BTreeSet<u32> = point.entries.iter().map(|e| e.start).collect();
            assert_eq!(
                starts.len(),
                point.entries.len(),
                "{label}: entries together"
            );
            assert!(point.entries.windows(2).all(|w| w[0].start < w[1].start));
            if let Some(second) = point.entries.get(1) {
                let delay = second.start - leader.start;
                assert!(
                    delay <= SECOND_ENTRY,
                    "{label}: the second entry after {delay}"
                );
                assert!(
                    delay < ticks(leader),
                    "{label}: the second entry after the head"
                );
            }
        }
        // Formulas and the end clear of heads.
        let goals: Vec<(u32, Step)> = plan
            .cadences
            .iter()
            .map(|c| (c.measure as u32 * MEASURE, c.target))
            .collect();
        let last = (plan.measures as u32 - 1) * MEASURE;
        for entry in plan.points.iter().flat_map(|p| &p.entries) {
            let mut tick = entry.start;
            for &(pitch, value, _) in &entry.notes {
                let (from, to) = (tick, tick + value);
                tick = to;
                let touches = |a: u32, b: u32| from < b && a < to;
                for &(goal, target) in &goals {
                    if formula_voices(mode, target, voices).contains(&entry.voice) {
                        assert!(
                            !touches(goal - 2 * MEASURE, goal + MEASURE),
                            "formula window"
                        );
                    } else if let Some(semitone) =
                        mode.clausula_semitone(Pitch::new(target, mode.scale_alter(target), 4))
                    {
                        let on = pitch.step == semitone.step;
                        assert!(
                            !(on && touches(goal - MINIM, goal)),
                            "the clausula's semitone"
                        );
                    }
                }
                let formula = formula_voices(mode, plan.ending, voices).contains(&entry.voice);
                assert!(
                    !(formula && touches(last - 2 * MEASURE, last)),
                    "final formula"
                );
                assert!(!touches(last, last + MEASURE), "the last measure");
            }
        }
        let mut goal = 0;
        for cadence in &plan.cadences {
            assert!(
                cadence.measure >= goal + CADENCE_SPACING,
                "goals {:?}",
                plan.cadences
            );
            goal = cadence.measure;
        }
        assert!(plan.measures > goal + CADENCE_SPACING, "the final cadence");
    }

    /// Every plan and every alternative the planner keeps keeps the limits, over the sections of
    /// the Ordinary and an own text, three to five voices and the modes.
    #[test]
    fn plans_keep_the_limits() {
        let texts = [
            "kyrie",
            "christe",
            "agnus",
            "sanctus",
            "benedictus",
            "Ad te le-vá-vi á-ni-mam me-am.",
        ];
        let modes = ["dorian", "phrygian", "mixolydian-transposed"];
        let mut count = 0;
        for (k, spec) in texts.iter().enumerate() {
            for voices in 3..=5 {
                let mode = modes[(k + voices) % modes.len()];
                let seed = (k * 3 + voices) as u64;
                let label = format!("{spec} {mode} {voices} voices seed {seed}");
                let planner = planner(spec, mode, voices, seed);
                let plan = planner.plan();
                assert_limits(&plan, mode.parse().unwrap(), voices, &label);
                // The alternatives too, after the points as laid out.
                for (p, alternatives) in planner.layout.iter().enumerate() {
                    for laid in alternatives {
                        let mut layout: Vec<&Laid> = planner.current(p);
                        layout.push(laid);
                        let points = layout.len();
                        let partial = planner.assemble(&layout);
                        if points == planner.points() {
                            assert_limits(&partial, mode.parse().unwrap(), voices, &label);
                        }
                        count += 1;
                    }
                }
                // The score keeps the plan of a setting of its length.
                let score_voices = choir(voices)
                    .into_iter()
                    .map(|voice_type| {
                        let rests = vec![
                            crate::score::Note::rest(crate::time::Duration::WHOLE);
                            plan.measures
                        ];
                        crate::score::Voice::new(
                            voice_type,
                            crate::score::Role::Counterpoint(Species::Free),
                            rests,
                        )
                    })
                    .collect();
                let mut score = Score::new(mode.parse().unwrap(), score_voices);
                score.cadences = plan.cadences.clone();
                score.imitation = Some(Arc::new(plan.score_imitation()));
                assert_eq!(score.validate(), Ok(()), "{label}");
            }
        }
        assert!(count >= texts.len() * 3 * 2, "{count} plans");
        // Modal answers wherever the heads allow them keep the heads within an octave too.
        let modal = Imitation {
            answer: AnswerChoice::Modal,
            ..Imitation::default()
        };
        let mut answered = 0;
        for (spec, mode, voices, seed) in [
            ("kyrie", "hypomixolydian", 4, 5),
            ("agnus", "hypoaeolian-transposed", 3, 19),
            ("kyrie", "dorian", 4, 1),
            ("christe", "mixolydian", 4, 2),
            ("sanctus", "ionian-transposed", 3, 28),
        ] {
            let mut request = request(spec, mode, voices, seed);
            request.imitation = Some(modal.clone());
            let mut planner = plan_imitation(&request, &modal, &Profile::fux()).unwrap();
            let label = format!("{spec} {mode} {voices} voices seed {seed}, modal");
            let mut plan = planner.plan();
            for _ in 0..4 {
                assert_limits(&plan, mode.parse().unwrap(), voices, &label);
                answered += plan
                    .points
                    .iter()
                    .flat_map(|p| &p.entries)
                    .filter(|e| matches!(e.answer, Answer::Modal { .. }))
                    .count();
                let Some(next) = planner.replan(0) else { break };
                plan = next;
            }
        }
        assert!(answered > 0, "no modal answer planned");
    }

    /// Every head of the plans and of the plans anew lies within its voice's range, as it is
    /// sung, where an octave of the voice holds it: no head is placed beyond the range (a case of
    /// FREE-AMBITUS or VOI-RANGE) where the octave above or below lies within it, or beyond the
    /// staff where the other lies within it, but where that octave clashes with the other heads
    /// of its point; and never more than a step beyond the staff where the other lies within a
    /// step of it.
    /// Over the sections of the Ordinary in the modes whose octaves of the mode fit the staves
    /// worst (Mixolydian, Aeolian), and others, with three to five voices; most heads lie within
    /// the range, almost none more than a step beyond the staff, and hardly one in another octave
    /// than the best.
    #[test]
    fn heads_lie_within_their_voices_ranges() {
        let mut counts = [0; 4];
        let mut worse = Vec::new();
        for (spec, mode, voices, seed) in [
            ("kyrie", "phrygian", 3, 2),
            ("kyrie", "mixolydian", 4, 2),
            ("agnus", "mixolydian", 4, 2),
            ("sanctus", "aeolian", 4, 1),
            ("kyrie", "aeolian", 5, 2),
            ("sanctus", "dorian", 5, 1),
            ("Ad te le-vá-vi á-ni-mam me-am.", "dorian", 4, 1),
        ] {
            let mut planner = planner(spec, mode, voices, seed);
            let label = format!("{spec} {mode} {voices} voices seed {seed}");
            let mut plan = planner.plan();
            for round in 0..6 {
                assert_in_range(&planner, &plan, &label, &mut counts, &mut worse);
                let point = round % plan.points.len();
                let measure = (plan.points[point].entries[0].start / MEASURE) as usize + 1;
                match planner.replan_near(point, Some(measure)) {
                    Some(next) => plan = next,
                    None => break,
                }
            }
        }
        let entries: usize = counts.iter().sum();
        assert!(entries >= 150, "{entries} entries");
        // In these modes and choirs, chosen for heads that fit them badly: three in four heads
        // lie within the range, one in twenty more than a step beyond the staff (in the Sanctus
        // with five voices and Ad te levavi, whose subjects span an octave).
        let [within, _, _, far] = counts;
        assert!(3 * within >= 2 * entries, "{counts:?}: beyond the range");
        assert!(
            12 * far <= entries,
            "{counts:?}: beyond a step outside the staff"
        );
        assert!(50 * worse.len() <= entries, "{counts:?}: {worse:#?}");
    }

    /// Asserts that the heads of `plan` as they are sung lie within their voices' ranges as far
    /// as their places and an octave of the voice allow ([`heads_lie_within_their_voices_ranges`]);
    /// counts them in `counts` by how far: within the range, beyond the octave of the mode only,
    /// a step beyond the staff, more. Collects in `worse` the heads where another octave would
    /// lie further within the range.
    fn assert_in_range(
        planner: &Planner,
        plan: &ImitationPlan,
        label: &str,
        counts: &mut [usize; 4],
        worse: &mut Vec<String>,
    ) {
        let checks = &planner.checks;
        // How far a head of `pitches` sung as `entry` lies beyond the voice's range.
        let tier = |pitches: &[Pitch], entry: &Entry| -> usize {
            let clef = checks.choir[entry.voice].clef();
            let outside: Vec<i32> = pitches.iter().map(|&p| clef.steps_outside(p)).collect();
            if outside.iter().any(|o| !(-3..=3).contains(o)) {
                return 4;
            }
            let mut tick = entry.start;
            let head = Head {
                voice: entry.voice,
                rest: entry.rest,
                tones: pitches
                    .iter()
                    .zip(&entry.notes)
                    .map(|(&pitch, &(_, ticks, _))| {
                        tick += ticks;
                        Tone {
                            pitch,
                            start: tick - ticks,
                            end: tick,
                        }
                    })
                    .collect(),
                sung: entry.notes.iter().map(|n| n.2).collect(),
            };
            let cases = checks.melody(&head, entry.rest == 0);
            if cases.iter().any(Violation::is_forbidden) {
                return 4;
            }
            let staff = cases.iter().any(|v| v.rule == "VOI-RANGE");
            let ambitus = cases.iter().any(|v| v.rule == "FREE-AMBITUS");
            match (staff, ambitus) {
                (false, false) => 0,
                (false, true) => 1,
                (true, _) if outside.iter().all(|o| (-1..=1).contains(o)) => 2,
                (true, _) => 3,
            }
        };
        let mode = checks.mode;
        for (p, point) in plan.points.iter().enumerate() {
            let subject = &point.subject;
            for entry in &point.entries {
                let at = format!(
                    "{label}: point {}, voice {} at {}",
                    p + 1,
                    entry.voice,
                    entry.start
                );
                // The head as it is sung: the subject's in the entry's octave, perhaps cut.
                let leader = mode.final_pitch(0).diatonic() + i32::from(subject.degree);
                let shift = entry.notes[0].0.diatonic() - leader;
                let sung = |shift: i32| {
                    let shift = i8::try_from(shift).ok()?;
                    let mut pitches = subject.pitches(mode, leader, shift, entry.answer)?;
                    pitches.truncate(entry.notes.len());
                    Some(pitches)
                };
                let pitches = sung(shift).expect("the head as planned");
                assert_eq!(
                    pitches,
                    entry.notes.iter().map(|n| n.0).collect::<Vec<_>>(),
                    "{at}"
                );
                let own = tier(&pitches, entry);
                counts[own.min(3)] += 1;
                for octave in [-7, 7] {
                    let Some(other) = sung(shift + octave).map(|other| tier(&other, entry)) else {
                        continue;
                    };
                    // Never more than a step beyond the staff where the octave lies within it.
                    assert!(
                        own < 3 || other > 2,
                        "{at}: the octave {octave:+} lies on the staff"
                    );
                    if other < own {
                        worse.push(format!("{at}: the octave {octave:+} lies better"));
                    }
                }
            }
        }
    }

    /// A head that the cuts leave more than a step beyond its staff moves an octave to where it
    /// lies on it, and a layout where that octave clashes with another head fails (VOI-RANGE):
    /// the tenor's E4 G4 F4 E4 of a Phrygian Sanctus reaches two steps above its staff; an octave
    /// lower it lies on it, but there the alto entering on A3 at its last note sounds a fourth
    /// above it while the bass is still silent.
    #[test]
    fn heads_beyond_the_staff_settle_an_octave_away() {
        let planner = planner("sanctus", "phrygian", 3, 5);
        let earlier = Earlier::of(&[]);
        let tenor = || head(1, 0, 0, "E4.8 G4.6 F4.2 E4.4");
        let leader = (Relation::Same, Answer::Real, FirstNote::AsLeader);
        let mut heads = vec![tenor()];
        assert_eq!(planner.settle(&mut heads, &[leader], &earlier), Ok(()));
        assert_eq!(heads, [tenor().octave(-7)]);
        let others = [
            head(0, 16, 0, "A3.8 C4.6 B3.2 A3.4 E3.4"),
            head(
                2,
                36,
                0,
                "E3.4 G3.6 F3.2 E3.4 B2.4 C3.6 D3.2 E3.8 D3.8 F3.8",
            ),
        ];
        let entries = [
            leader,
            (Relation::Dominant, Answer::Real, FirstNote::AsLeader),
            (Relation::Same, Answer::Real, FirstNote::Shortened),
        ];
        let laid = |tenor: Head| {
            let mut heads = vec![tenor];
            heads.extend(others.iter().cloned());
            heads
        };
        let mut heads = laid(tenor());
        assert_eq!(planner.resolve(&mut heads, &entries, &earlier), Ok(()));
        assert_eq!(heads, laid(tenor()));
        assert!(
            planner
                .resolve(&mut laid(tenor().octave(-7)), &entries, &earlier)
                .is_err()
        );
        assert_eq!(
            planner.settle(&mut heads, &entries, &earlier),
            Err("VOI-RANGE")
        );
    }

    /// The subject of a constructed point: the leader's head as it sings it.
    fn subject_of(leader: &Entry) -> Arc<Soggetto> {
        let first = leader.notes[0].0;
        Arc::new(Soggetto {
            degree: dorian().degree(first) as i8 - 1,
            notes: leader
                .notes
                .iter()
                .map(|&(pitch, ticks, sung)| SubjectNote {
                    step: (pitch.diatonic() - first.diatonic()) as i8,
                    ticks: ticks as u8,
                    sung,
                    alter: 0,
                })
                .collect(),
            words: 0..4,
            origin: Origin::Text,
            answers: vec![(Relation::Same, Answer::Real)],
            stretto: Vec::new(),
            cost: 0,
        })
    }

    /// An entry of `voice` at `start` after `rest`, its head as `pitch.ticks` notes, the first
    /// beginning the text, the others in its melisma.
    fn entry(voice: usize, start: u32, rest: u32, head: &str) -> Entry {
        let notes = head
            .split_whitespace()
            .enumerate()
            .map(|(i, note)| {
                let (pitch, ticks) = note.split_once('.').unwrap();
                let sung = if i == 0 {
                    Choice::Start(0)
                } else {
                    Choice::Hold
                };
                (pitch.parse().unwrap(), ticks.parse().unwrap(), sung)
            })
            .collect();
        Entry {
            voice,
            start,
            rest,
            notes,
            relation: Relation::Same,
            answer: Answer::Real,
            first: FirstNote::AsLeader,
        }
    }

    /// A plan of Kyrie I in Dorian with the points `points` (their entries and goals) and
    /// `measures` measures.
    fn constructed(points: Vec<(Vec<Entry>, Option<usize>)>, measures: usize) -> ImitationPlan {
        let goals: Vec<usize> = points.iter().filter_map(|(_, goal)| *goal).collect();
        ImitationPlan {
            points: points
                .into_iter()
                .map(|(entries, goal)| Point {
                    words: 0..7,
                    kind: PointKind::New,
                    subject: subject_of(&entries[0]),
                    entries,
                    goal,
                })
                .collect(),
            measures,
            cadences: cadences_at(dorian(), &goals),
            rate: 50,
            cost: 0,
            ending: dorian().final_step(),
        }
    }

    /// The first certain conflict that the pair checks and the skeleton find in a whole plan
    /// for `voices` in Dorian: the rule it breaks.
    fn certain_conflict(plan: &ImitationPlan, voices: &[VoiceType]) -> Option<&'static str> {
        let mode = dorian();
        let n = voices.len();
        let checks = Checks::new(
            text("kyrie"),
            mode,
            voices,
            &sung_profile(&Profile::fux()),
            &[],
        );
        let heads: Vec<Head> = plan
            .points
            .iter()
            .flat_map(|p| &p.entries)
            .map(|e| {
                let mut tick = e.start;
                Head {
                    voice: e.voice,
                    rest: e.rest,
                    tones: e
                        .notes
                        .iter()
                        .map(|&(pitch, ticks, _)| {
                            tick += ticks;
                            Tone {
                                pitch,
                                start: tick - ticks,
                                end: tick,
                            }
                        })
                        .collect(),
                    sung: e.notes.iter().map(|n| n.2).collect(),
                }
            })
            .collect();
        let mut formulas = formula_notes(mode, plan.ending, plan.measures - 1, n);
        for cadence in &plan.cadences {
            formulas.extend(formula_notes(mode, cadence.target, cadence.measure, n));
        }
        let all: Vec<&Head> = heads.iter().collect();
        let frame = Frame {
            choir: voices,
            heads: &all,
        };
        if let Some(found) = first_conflict(&frame, &heads, &[], &formulas) {
            return Some(found.conflict.rule());
        }
        let mut alone = Vec::new();
        for head in &heads {
            for v in checks.melody(head, head.rest == 0) {
                if v.is_forbidden() {
                    return Some(v.rule);
                }
                alone.push(v);
            }
        }
        let total = plan.measures as u32 * MEASURE;
        checks
            .skeleton(&all, &plan.score_imitation(), 0, total, &alone)
            .err()
    }

    /// Plans that the pair checks or the skeleton reject on constructed cases (one for each pair
    /// check and two that only the skeleton sees) are certain to fail: a probe with a beam of
    /// 200 finds nothing either. The plan they vary passes both.
    #[test]
    fn skeleton_rejects_only_certain_conflicts() {
        use VoiceType::*;
        let atb = [Alto, Tenor, Bass];
        let alto = "D4.8 E4.4 F4.4 G4.8 F4.8";
        let tenor = "A3.8 B3.4 C4.4 D4.8 C4.8";
        let bass = "D3.8 E3.4 F3.4 G3.8 F3.8";
        let base = |a: &str, t: (u32, &str), b: (u32, &str)| {
            constructed(
                vec![(
                    vec![
                        entry(0, 0, 0, a),
                        entry(1, t.0, 0, t.1),
                        entry(2, b.0, 0, b.1),
                    ],
                    None,
                )],
                10,
            )
        };
        let control = base(alto, (8, tenor), (24, bass));
        assert_eq!(certain_conflict(&control, &atb), None);
        assert!(probe(&control, dorian(), &atb, 40).is_ok());
        let cases: Vec<(&str, ImitationPlan, &[VoiceType])> = vec![
            // a: B♭ against B.
            (
                "a",
                base("D4.8 E4.4 F4.4 Bb4.8 F4.8", (8, tenor), (24, bass)),
                &atb,
            ),
            // b: a second struck together.
            (
                "b",
                base(alto, (8, "A3.8 B3.4 C4.4 E4.8 C4.8"), (24, bass)),
                &atb,
            ),
            // c: a fourth over the entering bass on the downbeat.
            (
                "c",
                base(
                    "D4.8 E4.4 F4.4 G4.8 G4.8",
                    (8, "A3.8 B3.4 C4.4 G3.8 C4.8"),
                    (24, bass),
                ),
                &atb,
            ),
            // d: a seventh under the held tenor, which leaps on.
            (
                "d",
                base(alto, (8, "A3.8 B3.4 C4.8 D4.4 C4.8"), (24, bass)),
                &atb,
            ),
            // e: the bass enters on the minim beat a seventh under the tenor.
            (
                "e",
                base(alto, (8, tenor), (28, "E3.8 F3.4 G3.4 A3.8 G3.8")),
                &atb,
            ),
            // g: the tenor enters a fourth above the held bass.
            (
                "g",
                base(
                    alto,
                    (20, "G3.8 A3.4 B3.4 C4.8"),
                    (16, "D3.16 E3.4 F3.4 G3.8"),
                ),
                &atb,
            ),
            // h: parallel fifths of alto and tenor.
            (
                "h",
                base(
                    "D4.8 E4.8 D4.8 C4.8",
                    (8, "A3.8 G3.8 F3.8 E3.8"),
                    (32, bass),
                ),
                &atb,
            ),
            // The skeleton: a two-voice opening not on a perfect consonance over the final
            // (CAD-START).
            (
                "start",
                constructed(
                    vec![(
                        vec![
                            entry(0, 0, 0, "D4.16 E4.8 F4.8"),
                            entry(1, 8, 0, "G3.8 A3.8 B3.8 C4.8"),
                        ],
                        None,
                    )],
                    8,
                ),
                &[Soprano, Tenor],
            ),
            // The skeleton: the alto's last head note, a second against the held tenor, can only
            // pass on into a second against the tenor's next note.
            (
                "end",
                constructed(
                    vec![(
                        vec![
                            entry(0, 0, 0, "D4.8 E4.8 D4.8 D4.8 C4.4 D4.8 C4.4"),
                            entry(1, 32, 0, "A3.8 B3.8 A3.8 A3.8"),
                            entry(2, 80, 0, "D3.8 E3.8 F3.8 D3.8"),
                        ],
                        None,
                    )],
                    15,
                ),
                &atb,
            ),
        ];
        for (name, plan, voices) in cases {
            let found = certain_conflict(&plan, voices);
            assert!(found.is_some(), "{name}: no conflict found");
            let result = probe(&plan, dorian(), voices, 200);
            assert!(
                matches!(result, Err(GenerateError::NoCounterpoint { .. })),
                "{name} ({found:?}): the probe found a setting"
            );
        }
        // f: the tenor enters on the minim beat after a goal on A with a B, against the
        // formula's A; on C it is fine.
        let two = |next: &str| {
            constructed(
                vec![
                    (
                        vec![
                            entry(0, 0, 0, alto),
                            entry(1, 8, 0, tenor),
                            entry(2, 24, 0, "D3.8 E3.4 F3.4 G3.4"),
                        ],
                        Some(8),
                    ),
                    (vec![entry(1, 68, 4, next)], None),
                ],
                16,
            )
        };
        assert_eq!(certain_conflict(&two("C4.8 D4.4 E4.4 F4.8"), &atb), None);
        let formula = two("B3.8 C4.4 D4.4 E4.8");
        assert_eq!(certain_conflict(&formula, &atb), Some("FREE-PASSING"));
        assert!(probe(&formula, dorian(), &atb, 200).is_err());
    }

    /// C7: a voice that has to begin more syllables between the end of a head and its next
    /// entry than it has minim beats there rejects the plan; four fifths of them cost nothing,
    /// each syllable beyond one.
    #[test]
    fn text_capacity_rejects_crowded_plans() {
        let voices = choir(3);
        let checks = Checks::new(
            text("kyrie"),
            dorian(),
            &voices,
            &sung_profile(&Profile::fux()),
            &[],
        );
        let head = |start: u32, rest: u32, notes: &[(&str, u32, Choice)]| {
            let mut tick = start;
            Head {
                voice: 0,
                rest,
                tones: notes
                    .iter()
                    .map(|&(pitch, ticks, _)| {
                        tick += ticks;
                        Tone {
                            pitch: pitch.parse().unwrap(),
                            start: tick - ticks,
                            end: tick,
                        }
                    })
                    .collect(),
                sung: notes.iter().map(|n| n.2).collect(),
            }
        };
        // Ký- in a melisma to tick 24; the tail e-lé-i-son entering on e-: ri-e still to come.
        let first = head(
            0,
            0,
            &[
                ("D4", 8, Choice::Start(0)),
                ("E4", 4, Choice::Hold),
                ("F4", 4, Choice::Hold),
                ("G4", 8, Choice::Hold),
            ],
        );
        let tail = |start: u32| {
            head(
                start,
                MINIM,
                &[
                    ("A4", 8, Choice::Start(3)),
                    ("G4", 8, Choice::Start(4)),
                    ("F4", 8, Choice::Start(5)),
                    ("E4", 8, Choice::Start(6)),
                ],
            )
        };
        let gated = |next: &Head, gates: &[(u16, u32)]| {
            let heads = [&first, next];
            checks.reach(&first, next, &heads, &[], 30 * MEASURE, gates)
        };
        let reach = |next: &Head| gated(next, &[]);
        // Right after the rest: no minim beat is left for ri-e.
        assert_eq!(reach(&tail(28)), Err("TXT-COMPLETE"));
        // Two beats for two syllables: they fit, one beyond four fifths of them.
        assert_eq!(reach(&tail(36)), Ok(1));
        // Room enough.
        assert_eq!(reach(&tail(64)), Ok(0));
        // Where a point begins ri- only at tick 32, a beat is left for ri-e before the entry at
        // 36, and with an entry at 64, eight.
        assert_eq!(gated(&tail(36), &[(1, 32)]), Err("TXT-COMPLETE"));
        assert_eq!(gated(&tail(64), &[(1, 32)]), Ok(0));
        assert_eq!(gated(&tail(64), &[(1, 60)]), Err("TXT-COMPLETE"));
        // A syllable of a later sentence the voice has gone past: never.
        let late = head(
            0,
            0,
            &[("D4", 8, Choice::Start(6)), ("E4", 8, Choice::Hold)],
        );
        let again = head(
            64,
            MINIM,
            &[("D4", 8, Choice::Start(3)), ("E4", 8, Choice::Hold)],
        );
        let heads = [&late, &again];
        assert_eq!(
            checks.reach(&late, &again, &heads, &[], 30 * MEASURE, &[]),
            Ok(0)
        );
        // A planner keeps the text's room: no plan of the Ordinary crowds a voice.
        for spec in ["kyrie", "agnus"] {
            let planner = planner(spec, "dorian", 4, 3);
            for laid in planner.layout.iter().map(|a| &a[0]) {
                assert!(laid.cost < u32::MAX);
            }
        }
    }

    /// The measures, goals and targets of the plans of the Ordinary, Dorian, seed 1, three to
    /// five voices (they follow the subjects of the points, from `subject`).
    #[test]
    fn layouts_of_the_ordinary() {
        let mut found = Vec::new();
        for spec in [
            "kyrie",
            "christe",
            "agnus",
            "sanctus",
            "benedictus",
            "gloria:1-5",
        ] {
            for voices in 3..=5 {
                let planner = planner(spec, "dorian", voices, 1);
                let plan = planner.plan();
                let goals: Vec<String> = plan
                    .cadences
                    .iter()
                    .map(|c| format!("{}{}", c.measure, c.target.letter()))
                    .collect();
                let line = format!(
                    "{spec} {voices}: {} measures, goals [{}], rate {}",
                    plan.measures,
                    goals.join(" "),
                    plan.rate
                );
                // The goals end the points that have one, on the targets of their endings.
                assert_eq!(plan.cadences.len(), plan.points.len() - 1, "{line}");
                let endings: Vec<(usize, Ending)> = plan
                    .points
                    .iter()
                    .filter_map(|p| Some((p.goal?, ending(&planner.checks.text, &p.words))))
                    .collect();
                assert_eq!(targets(dorian(), endings), plan.cadences, "{line}");
                found.push(line);
            }
        }
        let golden = GOLDEN_LAYOUTS
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty());
        assert_eq!(
            found,
            golden.collect::<Vec<_>>(),
            "the layouts now:\n{}",
            found.join("\n")
        );
    }

    /// See [`layouts_of_the_ordinary`].
    const GOLDEN_LAYOUTS: &str = "
        kyrie 3: 23 measures, goals [13A], rate 26
        kyrie 4: 38 measures, goals [18A], rate 15
        kyrie 5: 33 measures, goals [20A], rate 18
        christe 3: 24 measures, goals [13A], rate 23
        christe 4: 35 measures, goals [18A], rate 15
        christe 5: 24 measures, goals [15A], rate 27
        agnus 3: 37 measures, goals [14F 24G], rate 26
        agnus 4: 39 measures, goals [17F 28G], rate 24
        agnus 5: 45 measures, goals [21F 33G], rate 21
        sanctus 3: 25 measures, goals [10F], rate 30
        sanctus 4: 26 measures, goals [17F], rate 29
        sanctus 5: 36 measures, goals [21F], rate 21
        benedictus 3: 34 measures, goals [20F], rate 22
        benedictus 4: 36 measures, goals [23F], rate 21
        benedictus 5: 41 measures, goals [24F], rate 18
        gloria:1-5 3: 39 measures, goals [15A 25F], rate 49
        gloria:1-5 4: 44 measures, goals [20A 31F], rate 43
        gloria:1-5 5: 65 measures, goals [28A 47F], rate 29
    ";

    /// The plan of a part of a mass section (phase 14): the leader of the first point enters at
    /// the part's start, after a breath or a measure's rest, and no entry before it; the final
    /// cadence goes to the part's ending, in the plan and in its fixed notes; and the goals take
    /// their targets from the part's chain, by index as the endings of the points' words give
    /// them. The plans keep every limit.
    #[test]
    fn parts_start_and_end_where_asked() {
        use crate::generate::PartContext;
        for (spec, start, ending) in [
            ("christe", 4, Step::A),
            ("christe", 8, Step::G),
            ("kyrie", 0, Step::A),
            ("agnus", 4, Step::D),
        ] {
            let label = format!("{spec} start {start} on {}", ending.letter());
            let mut request = request(spec, "dorian", 4, 1);
            request.part = PartContext {
                start,
                ending: Some(ending),
                ..PartContext::default()
            };
            let planner = plan_imitation(&request, &Imitation::default(), &Profile::fux()).unwrap();
            let plan = planner.plan();
            assert_eq!(plan.ending, ending, "{label}");
            assert_eq!(plan.points[0].entries[0].start, start, "{label}");
            assert!(
                plan.points
                    .iter()
                    .flat_map(|p| &p.entries)
                    .all(|e| e.start >= start),
                "{label}"
            );
            assert_limits(&plan, dorian(), 4, &label);
            let last = (plan.measures as u32 - 1) * MEASURE;
            let closing: Vec<Step> = plan
                .fixed(dorian(), 4)
                .iter()
                .filter(|f| f.start == last)
                .filter_map(|f| Some(f.note.pitch?.step))
                .collect();
            assert!(closing.len() >= 2, "{label}: {closing:?}");
            assert!(closing.iter().all(|&step| step == ending), "{label}");
        }
        // Without a part the planner plans as before: on the final, from tick 0.
        let plain = planner("christe", "dorian", 4, 1).plan();
        assert_eq!(
            (plain.ending, plain.points[0].entries[0].start),
            (Step::D, 0)
        );
        // A weighted chain gives the goals their targets.
        let chain = CadenceChain::Weighted {
            seed: 5,
            weights: vec![
                (Step::A, 300),
                (Step::F, 300),
                (Step::G, 200),
                (Step::C, 200),
            ],
            before: Some(Step::D),
            christe: false,
        };
        let mut request = request("gloria:1-5", "dorian", 4, 1);
        request.part.chain = chain.clone();
        let planner = plan_imitation(&request, &Imitation::default(), &Profile::fux()).unwrap();
        let plan = planner.plan();
        let endings: Vec<(usize, Ending)> = plan
            .points
            .iter()
            .filter_map(|p| Some((p.goal?, ending(&planner.checks.text, &p.words))))
            .collect();
        assert!(!endings.is_empty());
        assert_eq!(targets_with(dorian(), endings, &chain), plan.cadences);
        assert_limits(&plan, dorian(), 4, "gloria:1-5 chained");
    }

    /// A failure is put down to the point of the entry nearest after the start of its measure,
    /// up to two measures on; otherwise to the point that spans it; otherwise to the last.
    #[test]
    fn failures_are_put_down_to_points() {
        let plan = planner("agnus", "dorian", 4, 1).plan();
        let measure = |tick: u32| (tick / MEASURE) as usize + 1;
        for (p, point) in plan.points.iter().enumerate() {
            for entry in &point.entries {
                assert_eq!(
                    plan.owner(measure(entry.start)),
                    p,
                    "entry at {}",
                    entry.start
                );
            }
        }
        let second = plan.points[1].entries[0].start;
        // In the measure before the second point's leader it is due; two measures before, the
        // first point still spans it.
        assert_eq!(plan.owner(measure(second) - 1), 1);
        assert_eq!(plan.owner(measure(second) - 2), 0);
        // The final measure goes to the last point.
        assert_eq!(plan.owner(plan.measures), plan.points.len() - 1);
        assert_eq!(plan.owner(plan.measures + 10), plan.points.len() - 1);
    }

    /// Re-planning a point takes its next alternative and keeps the points before it; the
    /// points after it are laid out anew where they must. Once its alternatives and the plans
    /// drawn anew are spent it fails.
    #[test]
    fn replanning_takes_the_next_alternative() {
        let mut planner = planner("kyrie", "dorian", 4, 2);
        let plan = planner.plan();
        let mut seen = vec![plan.points[0].clone()];
        let mut replans = 0;
        while let Some(next) = planner.replan(0) {
            assert!(!seen.contains(&next.points[0]), "the same point again");
            seen.push(next.points[0].clone());
            assert_limits(&next, dorian(), 4, "replanned");
            replans += 1;
            let most = (KEPT + 4) * (1 + REDRAWS as usize);
            assert!(replans <= most, "{replans} replans");
        }
        assert!(replans >= 1, "no alternative");
        // The last point re-planned: the first stays.
        let mut planner = self::planner("kyrie", "dorian", 4, 2);
        let before = planner.plan();
        let after = planner.replan_near(1, Some(before.measures - 4)).unwrap();
        assert_eq!(after.points[0], before.points[0]);
        assert_ne!(after.points[1], before.points[1]);
    }

    /// A plan a quarter longer keeps its points: the goals and the end later, each point's
    /// entries with the goal before them; and a length the request gives is met, or the request
    /// fails where even compressed plans do not fit.
    #[test]
    fn plans_grow_and_meet_given_lengths() {
        let planner = planner("agnus", "dorian", 4, 1);
        let plan = planner.plan();
        let longer = planner.lengthen(&plan);
        assert_eq!(longer.measures, (plan.measures * 5).div_ceil(4));
        assert_limits(&longer, dorian(), 4, "longer");
        for (a, b) in plan.points.iter().zip(&longer.points) {
            let shift = b.entries[0].start - a.entries[0].start;
            assert_eq!(shift % MEASURE, 0);
            for (x, y) in a.entries.iter().zip(&b.entries) {
                assert_eq!((y.start - x.start, &y.notes), (shift, &x.notes));
            }
        }
        assert!(longer.rate < plan.rate);
        // A given length.
        let given = |measures: usize| {
            let mut request = request("kyrie", "dorian", 3, 1);
            request.measures = measures;
            request.text.as_mut().unwrap().auto_length = false;
            plan_imitation(&request, &Imitation::default(), &Profile::fux())
        };
        let natural = planner_measures("kyrie", 3, 1);
        let fitted = given(natural + 5).unwrap().plan();
        assert_eq!(fitted.measures, natural + 5);
        assert_limits(&fitted, dorian(), 3, "given");
        let tight = given(natural - 2).map(|p| p.plan().measures);
        assert!(
            matches!(tight, Ok(m) if m == natural - 2)
                || matches!(tight, Err(GenerateError::ImitationTooLong { .. })),
            "{natural}: {tight:?}"
        );
        assert!(matches!(
            given(8),
            Err(GenerateError::ImitationTooLong { measures }) if measures > 8
        ));
        // Every plan anew keeps the given length: each point re-planned until it has none left,
        // the points after it compressed where they must be; none comes out longer.
        for (spec, voices, seed, extra) in
            [("kyrie", 3, 1, 0), ("kyrie", 4, 1, 1), ("agnus", 4, 1, 0)]
        {
            let natural = planner_measures(spec, voices, seed);
            let mut request = request(spec, "dorian", voices, seed);
            request.measures = natural + extra;
            request.text.as_mut().unwrap().auto_length = false;
            let mut planner =
                plan_imitation(&request, &Imitation::default(), &Profile::fux()).unwrap();
            let first = planner.plan();
            assert_eq!(first.measures, natural + extra);
            let mut replans = 0;
            for p in 0..first.points.len() {
                while let Some(next) = planner.replan(p) {
                    let label = format!("{spec} {voices} voices seed {seed}, point {}", p + 1);
                    assert_eq!(next.measures, natural + extra, "{label}");
                    assert_limits(&next, dorian(), voices, &label);
                    replans += 1;
                }
            }
            assert!(replans >= 2, "{spec}: {replans} plans anew");
        }
    }

    /// Options that the choir or the text cannot have fail with a message that names them, in
    /// German and English, instead of being left aside; delays and relations given for a later
    /// point give it the voices they need.
    #[test]
    fn options_the_request_cannot_have() {
        use OptionError::*;
        let fails = |spec: &str, imitation: Imitation| -> OptionError {
            let request = request(spec, "dorian", 4, 1);
            match plan_imitation(&request, &imitation, &Profile::fux()) {
                Err(GenerateError::ImitationOption(option)) => option,
                other => panic!("{spec}: {:?}", other.map(|p| p.plan())),
            }
        };
        let with = |f: &dyn Fn(&mut Imitation)| {
            let mut imitation = Imitation::default();
            f(&mut imitation);
            imitation
        };
        let cases = [
            (with(&|i| i.head = Some(3)), Head { notes: 3, most: 10 }),
            (
                with(&|i| i.head = Some(11)),
                Head {
                    notes: 11,
                    most: 10,
                },
            ),
            (
                with(&|i| i.later_voices = Some(1)),
                LaterVoices { voices: 1, most: 4 },
            ),
            (
                with(&|i| i.later_voices = Some(5)),
                LaterVoices { voices: 5, most: 4 },
            ),
            (
                with(&|i| i.orders = vec![None, Some(vec![7])]),
                NoVoice {
                    point: 2,
                    voice: 8,
                    voices: 4,
                },
            ),
            (
                with(&|i| i.orders = vec![Some(vec![1, 1])]),
                VoiceTwice { point: 1, voice: 2 },
            ),
            (
                with(&|i| i.orders = vec![None, Some(vec![2])]),
                FewVoices { point: 2 },
            ),
            (
                with(&|i| i.delays = vec![Some(vec![8, 6])]),
                Delay { point: 1, ticks: 6 },
            ),
            (
                with(&|i| i.delays = vec![Some(vec![0])]),
                Delay { point: 1, ticks: 0 },
            ),
            (
                with(&|i| i.delays = vec![Some(vec![8; 4])]),
                TooMany {
                    point: 1,
                    given: 4,
                    most: 3,
                },
            ),
            (
                with(&|i| {
                    i.relations = vec![None, Some(vec![super::Relation::Same; 3])];
                    i.later_voices = Some(2);
                }),
                TooMany {
                    point: 2,
                    given: 3,
                    most: 1,
                },
            ),
            (
                with(&|i| i.delays = vec![None, None, Some(vec![8])]),
                NoPoint {
                    point: 3,
                    points: 2,
                },
            ),
        ];
        for (imitation, error) in cases {
            assert_eq!(fails("kyrie", imitation), error);
        }
        let restated = with(&|i| i.restate = Toggle::On);
        assert_eq!(fails("Ad te le-vá-vi á-ni-mam me-am.", restated), Restate);
        // Given heads go to the points with subjects of their own: three for Kyrie I's two
        // points, or two where the request asks for the restatement, which takes none.
        let heads = |n: usize| {
            ["d'1 f'2 e'2 d'1", "a'1 g'2 f'2 e'1", "d'1 a'1 g'2 f'2 e'1"][..n]
                .iter()
                .map(|head| {
                    crate::parse::parse_lilypond(head, crate::time::Duration::WHOLE).unwrap()
                })
                .collect::<Vec<_>>()
        };
        let three = with(&|i| i.subject = SubjectSource::Given(heads(3)));
        assert_eq!(
            fails("kyrie", three),
            Subjects {
                given: 3,
                points: 2
            }
        );
        let two_restated = with(&|i| {
            i.subject = SubjectSource::Given(heads(2));
            i.restate = Toggle::On;
        });
        assert_eq!(
            fails("kyrie", two_restated),
            Subjects {
                given: 2,
                points: 1
            }
        );
        let error = GenerateError::ImitationOption(NoVoice {
            point: 2,
            voice: 8,
            voices: 4,
        });
        assert_eq!(
            error.message(crate::lang::Lang::De),
            "Die Reihenfolge von Punkt 2 nennt Stimme 8, der Satz hat 4 Stimmen."
        );
        assert_eq!(
            error.message(crate::lang::Lang::En),
            "The order of point 2 names voice 8, but the setting has 4 voices."
        );
        let delay = GenerateError::ImitationOption(Delay { point: 1, ticks: 6 });
        assert_eq!(
            delay.message(crate::lang::Lang::De),
            "Punkt 1: Ein Abstand von 0,75 Takten ist kein Vielfaches einer Halben."
        );
        // Delays of a later point give it as many voices: three entries a measure apart.
        let delays = with(&|i| i.delays = vec![None, Some(vec![8, 8])]);
        let plan = plan_imitation(&request("kyrie", "dorian", 4, 1), &delays, &Profile::fux())
            .unwrap()
            .plan();
        let starts: Vec<u32> = plan.points[1].entries.iter().map(|e| e.start).collect();
        assert!(starts.len() >= 3, "{starts:?}");
        assert!(
            starts[..3].windows(2).all(|w| w[1] - w[0] == 8),
            "{starts:?}"
        );
    }

    /// A second entry whose delay the request gives may come after the leader's head has ended,
    /// even after two breves: IMI-TIME prices it (`apart`, `second`), the plan keeps it.
    #[test]
    fn given_delays_may_come_after_the_head() {
        let imitation = Imitation {
            delays: vec![Some(vec![40])],
            ..Imitation::default()
        };
        let request = request("kyrie", "dorian", 4, 1);
        let planner = plan_imitation(&request, &imitation, &Profile::fux()).unwrap();
        let plan = planner.plan();
        let point = &plan.points[0];
        assert_eq!(point.entries[1].start - point.entries[0].start, 40);
        assert!(point.entries[1].start >= point.entries[0].start + ticks(&point.entries[0]));
        // It costs more than the plan with the delays drawn.
        let drawn = planner_cost("kyrie", 4, 1);
        assert!(plan.cost > drawn, "{} against {drawn}", plan.cost);
    }

    fn planner_cost(spec: &str, voices: usize, seed: u64) -> u32 {
        planner(spec, "dorian", voices, seed).plan().cost
    }

    /// The openings of 600 first plans against Palestrina's complete openings by the program's
    /// detector: the entries spread over the main degrees. Three entries or more at one pitch in
    /// five voices, about 12 % in Palestrina's (the plans before had 34 %); one degree for every
    /// entry in four voices, 2.7 % (14 % before), in three voices 12.1 % (19.5 %). It also prints
    /// how often the leader's head is cut. Run with
    /// `cargo test --release -p fux-core --lib openings_spread -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn openings_spread_their_degrees() {
        let texts = ["kyrie", "christe", "agnus", "sanctus", "benedictus"];
        let modes = ["dorian", "phrygian", "mixolydian", "aeolian"];
        let mut plans = [0usize; 6];
        let (mut stacked, mut one_degree) = ([0usize; 6], [0usize; 6]);
        let (mut leaders, mut cut, mut four) = (0, 0, 0);
        let (mut followers, mut short) = (0, 0);
        for spec in texts {
            for mode in modes {
                for voices in 3..=5 {
                    for seed in 1..=10 {
                        let request = request(spec, mode, voices, seed);
                        let Ok(planner) =
                            plan_imitation(&request, &Imitation::default(), &Profile::fux())
                        else {
                            continue;
                        };
                        let plan = planner.plan();
                        let opening = &plan.points[0];
                        plans[voices] += 1;
                        let firsts: Vec<Pitch> =
                            opening.entries.iter().map(|e| e.notes[0].0).collect();
                        let most = firsts
                            .iter()
                            .map(|p| firsts.iter().filter(|q| *q == p).count())
                            .max()
                            .unwrap_or(0);
                        stacked[voices] += usize::from(most >= 3);
                        one_degree[voices] +=
                            usize::from(firsts.iter().all(|p| p.step == firsts[0].step));
                        let leader = &opening.entries[0];
                        leaders += 1;
                        cut += usize::from(leader.notes.len() < opening.subject.notes.len());
                        four += usize::from(leader.notes.len() == 4);
                        for entry in &opening.entries[1..] {
                            followers += 1;
                            short += usize::from(entry.notes.len() <= 4);
                        }
                    }
                }
            }
        }
        let share = |part: usize, whole: usize| 100.0 * part as f64 / whole.max(1) as f64;
        for voices in 3..=5 {
            println!(
                "{voices} voices: {} openings, three entries or more at one pitch {:.1} %, all on one degree {:.1} %",
                plans[voices],
                share(stacked[voices], plans[voices]),
                share(one_degree[voices], plans[voices])
            );
        }
        println!(
            "leaders: {leaders}, cut {:.1} %, four notes {:.1} %; followers of four notes or fewer {:.1} % of {followers}",
            share(cut, leaders),
            share(four, leaders),
            share(short, followers)
        );
        assert!(share(stacked[5], plans[5]) < 25.0);
        assert!(share(one_degree[4], plans[4]) < 6.0);
        assert!(share(one_degree[3], plans[3]) < 15.0);
    }

    /// Where the request gives a head for every point a tail would give, a restatement the
    /// planner draws gives way to the tail, which sings the second head: three voices of Kyrie I,
    /// seed 2, draw the restatement.
    #[test]
    fn given_heads_take_the_tail_from_a_drawn_restatement() {
        let request = request("kyrie", "dorian", 3, 2);
        let text = text("kyrie");
        let mode: ModeSpec = "dorian".parse().unwrap();
        let statements = plan(&text, mode, 3, None, None, true).unwrap().statements;
        let seed = 2 ^ fingerprint(&format!("{}|{}|{}", text.spec(), mode.id(), 3));
        let drawn = points::points_of(
            &text,
            statements,
            true,
            &Imitation::default(),
            &mut generator(seed, 0, u64::MAX),
        )
        .unwrap();
        assert_eq!(drawn[1].kind, PointKind::Restatement { of: 0 });
        let heads = ["d'1 f'2 e'2 d'1", "a'1 g'2 f'2 e'1"]
            .iter()
            .map(|head| crate::parse::parse_lilypond(head, crate::time::Duration::WHOLE).unwrap())
            .collect();
        let given = Imitation {
            subject: SubjectSource::Given(heads),
            ..Imitation::default()
        };
        let plan = plan_imitation(&request, &given, &Profile::fux())
            .unwrap()
            .plan();
        assert_eq!(plan.points[1].kind, PointKind::Tail);
        assert_eq!(plan.points[1].subject.origin, Origin::Given);
    }

    /// Where the restatement of the first point, which the planner drew itself, finds no valid
    /// entries, the text's last tail takes its place; a restatement the request asks for fails.
    #[test]
    fn drawn_restatements_give_way_to_the_tail() {
        let request = request("kyrie2", "aeolian", 4, 71);
        // The planner draws the restatement for this request.
        let text = text("kyrie2");
        let mode: ModeSpec = "aeolian".parse().unwrap();
        let statements = plan(&text, mode, 4, None, None, true).unwrap().statements;
        let seed = 71 ^ fingerprint(&format!("{}|{}|{}", text.spec(), mode.id(), 4));
        let drawn = points::points_of(
            &text,
            statements,
            true,
            &Imitation::default(),
            &mut generator(seed, 0, u64::MAX),
        )
        .unwrap();
        assert_eq!(drawn[1].kind, PointKind::Restatement { of: 0 });
        // Delays of a measure, a measure and a minim in the second point: the restatement finds
        // no entries, the tail does.
        let delayed = Imitation {
            delays: vec![None, Some(vec![8, 8, 4])],
            ..Imitation::default()
        };
        let asked = Imitation {
            restate: Toggle::On,
            ..delayed.clone()
        };
        assert!(matches!(
            plan_imitation(&request, &asked, &Profile::fux()),
            Err(GenerateError::NoImitation { point: 2, .. })
        ));
        let plan = plan_imitation(&request, &delayed, &Profile::fux())
            .unwrap()
            .plan();
        assert_eq!(plan.points[1].kind, PointKind::Tail);
    }

    fn planner_measures(spec: &str, voices: usize, seed: u64) -> usize {
        planner(spec, "dorian", voices, seed).plan().measures
    }

    /// A given head keeps its accidentals in every entry of its point: the entries at the unison
    /// and the octave sing them as given, C♯ below D, those on the other main degree a fifth or a
    /// fourth away, G♯ below A; and a written natural stays natural, B between two A's in the
    /// Dorian mode, which a head from the words sings B♭ (fa super la). The search sings them so
    /// (a probe of three voices, re-planned as the driver does after a failure).
    #[test]
    fn given_heads_keep_their_accidentals() {
        for (head, at, same, dominant) in [
            (
                "d'1 cis'2 d'2 e'1 f'1 e'1 d'1",
                1,
                (Step::C, 1),
                (Step::G, 1),
            ),
            ("d'1 e'2 f'2 g'1 a'1 b'1 a'1", 5, (Step::B, 0), (Step::F, 0)),
        ] {
            let notes = crate::parse::parse_lilypond(head, crate::time::Duration::HALF).unwrap();
            let imitation = Imitation {
                subject: SubjectSource::Given(vec![notes]),
                ..Imitation::default()
            };
            // Entries at the unison that reach note `at`, which a head cut short does not.
            let mut sung = 0;
            for (voices, seed) in [(3, 1), (4, 1), (4, 2), (4, 3)] {
                let request = request("kyrie", "dorian", voices, seed);
                let mut planner = plan_imitation(&request, &imitation, &Profile::fux()).unwrap();
                let mut plan = planner.plan();
                if voices == 3 {
                    let choir = choir(voices);
                    let found = loop {
                        match probe(&plan, dorian(), &choir, 20) {
                            Ok(found) => break found,
                            Err(GenerateError::NoCounterpoint { measure, .. }) => {
                                plan = planner
                                    .replan_near(plan.owner(measure), Some(measure))
                                    .expect("a plan the probe finds");
                            }
                            Err(error) => panic!("{head}: {error:?}"),
                        }
                    };
                    let score = &found[0].score;
                    for entry in plan.points.iter().flat_map(|point| &point.entries) {
                        let mut tick = entry.start;
                        for &(pitch, ticks, _) in &entry.notes {
                            assert!(
                                score.voices[entry.voice].timed().any(|(start, note)| {
                                    start == tick && note.pitch == Some(pitch)
                                }),
                                "{head}: voice {} does not sing {pitch} at {tick}",
                                entry.voice
                            );
                            tick += ticks;
                        }
                    }
                }
                assert_limits(&plan, dorian(), voices, head);
                let point = &plan.points[0];
                assert_eq!(point.subject.origin, Origin::Given, "{head}");
                for entry in &point.entries {
                    let Some(&(pitch, _, _)) = entry.notes.get(at) else {
                        continue;
                    };
                    let wanted = match entry.relation {
                        Relation::Same => same,
                        Relation::Dominant => dominant,
                        Relation::Steps(_) => panic!("{head}: an entry off the main degrees"),
                    };
                    assert_eq!((pitch.step, pitch.alter), wanted, "{head}: {entry:?}");
                    sung += usize::from(entry.relation == Relation::Same);
                }
            }
            assert!(sung > 0, "{head}: no entry at the unison sings note {at}");
        }
    }

    /// Plans the sections of the Ordinary in every mode with three to five voices and prints
    /// the planner's time, the share of valid plans drawn, the lengths, and with `PROBES` set
    /// how many plans a probe (beam 20) finds after re-planning up to that many times. Run with
    /// `cargo test --release -p fux-core planner_statistics -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn planner_statistics() {
        let texts = [
            "kyrie",
            "christe",
            "agnus",
            "sanctus",
            "benedictus",
            "gloria:1-5",
        ];
        let modes = ["dorian", "phrygian", "mixolydian", "aeolian"];
        let probes: usize = std::env::var("PROBES").map_or(0, |v| v.parse().unwrap());
        let seeds: u64 = std::env::var("SEEDS").map_or(2, |v| v.parse().unwrap());
        for spec in texts {
            let (mut count, mut seconds, mut drawn, mut valid, mut found) = (0, 0.0, 0, 0, 0);
            let mut lengths = Vec::new();
            for mode in modes {
                for voices in 3..=5 {
                    for seed in 1..=seeds {
                        let request = request(spec, mode, voices, seed);
                        let clock = std::time::Instant::now();
                        let mut planner =
                            plan_imitation(&request, &Imitation::default(), &Profile::fux())
                                .unwrap();
                        seconds += clock.elapsed().as_secs_f64();
                        let (d, v) = planner.samples();
                        (count, drawn, valid) = (count + 1, drawn + d, valid + v);
                        let mut plan = planner.plan();
                        lengths.push(plan.measures);
                        let voices = {
                            let mut voices = request.voices.clone();
                            voices.sort();
                            voices
                        };
                        for _ in 0..probes {
                            match probe(&plan, request.mode, &voices, 20) {
                                Ok(_) => {
                                    found += 1;
                                    break;
                                }
                                Err(GenerateError::NoCounterpoint { measure, .. }) => {
                                    let owner = plan.owner(measure);
                                    match planner.replan_near(owner, Some(measure)) {
                                        Some(next) => plan = next,
                                        None => break,
                                    }
                                }
                                Err(e) => panic!("{e:?}"),
                            }
                        }
                    }
                }
            }
            lengths.sort_unstable();
            println!(
                "{spec:<11} {count} plans: {:.1} ms each, valid {valid} of {drawn} drawn ({:.0} %), measures {}–{} (median {}){}",
                1000.0 * seconds / count as f64,
                100.0 * valid as f64 / drawn as f64,
                lengths[0],
                lengths[lengths.len() - 1],
                lengths[lengths.len() / 2],
                if probes > 0 {
                    format!(", probes find {found} of {count}")
                } else {
                    String::new()
                }
            );
        }
    }
}
