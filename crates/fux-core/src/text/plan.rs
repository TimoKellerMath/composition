//! The structure of a texted free setting from its text: how long it is, how fast its voices
//! declaim and where its internal cadences fall.
//!
//! The pace follows Palestrina's densities (PAL-DENS-1): about one syllable per measure in the
//! Kyrie, Christe and Agnus, fewer in the melismatic Sanctus, Osanna and Benedictus, more in the
//! Gloria and Credo, which he sets almost a note to a syllable. A short text is planned to be
//! sung more than once, as in his Kyries, a long one once. Internal cadences fall where a planned
//! statement ends a sense unit, by preference at a full stop. The plan only sets the length and
//! the cadences: what each voice restates and where its syllables fall, the search decides.
//!
//! A part of a mass section (phase 14, [`plan_part`]) may end on another degree than the final,
//! begin after a breath, and take the degrees of its cadences from a chain over the whole
//! movement ([`CadenceChain`]) instead of the mode's regular degrees in turn.

use crate::generate::{
    CADENCE_SPACING, GenerateError, MIN_MEASURES, PartContext, Rng, cadence_formula,
    regular_cadences, splitmix64,
};
use crate::mode::ModeSpec;
use crate::pitch::Step;
use crate::rules::Level;
use crate::score::Cadence;
use crate::text::underlay::{Capacity, Case, Levels};
use crate::text::{Stop, SylFlags, Text, TextKind};
use crate::time::Meter;

/// Syllable starts per measure, restatements included, in the Kyrie, Christe and Agnus and in
/// own short texts. Provisional.
const SHORT_RATE: f64 = 1.0;

/// Syllable starts per measure in the melismatic short texts ([`MELISMATIC`]). Provisional.
const MELISMATIC_RATE: f64 = 0.6;

/// Syllable starts per measure in the sections of the Gloria and the Credo and in own long
/// texts. Provisional.
const LONG_RATE: f64 = 1.2;

/// The short sections that Palestrina sets with long melismas.
const MELISMATIC: [&str; 5] = ["sanctus", "pleni", "osanna", "osanna2", "benedictus"];

/// How many syllables a short text is planned to have in all its statements: Kyrie I twice.
const PLANNED_SYLLABLES: f64 = 14.0;

/// The measures a setting has beyond its syllables at the planned pace: the held last measure
/// and room for the cadential melisma.
const SLACK: usize = 2;

/// The shortest automatic length.
const SHORTEST: usize = 8;

/// How many measures an internal cadence may move from its even place to the end of a unit.
const REACH: usize = 3;

/// A setting planned from its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub measures: usize,
    /// Internal cadences: the measure (from 0) their target arrives in, and the target.
    pub cadences: Vec<Cadence>,
    /// The planned pace: syllable starts per 100 minim beats, restatements included, as
    /// [`crate::generate::SungText::rate`] takes it. It fills the setting with the planned
    /// statements, up to the slack of its last measures.
    pub rate: u16,
    /// How many times the text is planned to be sung: the length and the cadences count with
    /// as many statements, while each voice restates what its notes allow.
    pub statements: usize,
}

/// Plans a free setting of `voices` voices in `mode` on `text`: its length, unless `measures`
/// gives it, its internal cadences, unless `cadences` gives them, and its pace. `restatements`
/// says whether the text may be restated; without, a short text is planned to be sung once.
///
/// The automatic length is `max(8, round(m·S / rate) + 2)` for a text of S syllables planned
/// to be sung m times at `rate` syllables per measure. A given length must leave every voice a
/// minim beat for each syllable, besides the beats its cadence formulas hold; otherwise the plan
/// fails with [`GenerateError::TextTooLong`] and the least length that does.
pub fn plan(
    text: &Text,
    mode: ModeSpec,
    voices: usize,
    measures: Option<usize>,
    cadences: Option<Vec<Cadence>>,
    restatements: bool,
) -> Result<Plan, GenerateError> {
    plan_part(
        text,
        mode,
        voices,
        measures,
        cadences,
        restatements,
        &PartContext::default(),
    )
}

/// Like [`plan`], for a part of a larger work such as a mass section (phase 14, `part`): its final
/// cadence goes to the part's ending, its internal cadences take their degrees by the part's
/// chain ([`targets_with`]), and where its voices are silent before its start (a breath after the
/// held chord of the part before, or a whole measure after a break), the automatic length has
/// ⌈start / 8⌉ measures more, the syllables and the cadences spread over the measures from the
/// start on, and the first cadence comes two measures after the start at the earliest, a measure
/// later after an opening chord ([`PartContext::first_goal`]). The capacity leaves out the silence
/// and the beat that the opening chord holds, as the search counts them. With the default part
/// the plan is that of [`plan`].
pub fn plan_part(
    text: &Text,
    mode: ModeSpec,
    voices: usize,
    measures: Option<usize>,
    cadences: Option<Vec<Cadence>>,
    restatements: bool,
    part: &PartContext,
) -> Result<Plan, GenerateError> {
    let syllables = text.syllables.len();
    if syllables == 0 {
        return Err(GenerateError::EmptyText);
    }
    let statements = statements(text, restatements);
    let cadences_for = |measures: usize| match &cadences {
        Some(given) => given.clone(),
        None => internal_cadences(text, mode, measures, statements, part),
    };
    let fits = |measures: usize| {
        let room = capacity(mode, voices, measures, &cadences_for(measures), part);
        room >= syllables as u32
    };
    // A length beyond which every text fits.
    let bound = |from: usize| from + 2 * syllables + SHORTEST;
    let measures = match measures {
        Some(given) if given < MIN_MEASURES => {
            return Err(GenerateError::TooShort {
                least: MIN_MEASURES,
            });
        }
        Some(given) if !fits(given) => {
            let least = (given + 1..bound(given))
                .find(|&m| fits(m))
                .unwrap_or(bound(given));
            return Err(GenerateError::TextTooLong { measures: least });
        }
        Some(given) => given,
        None => {
            let automatic = automatic_length(text, restatements) + silent_measures(part.start);
            (automatic..bound(automatic))
                .find(|&m| fits(m))
                .unwrap_or(bound(automatic))
        }
    };
    let starts = (statements * syllables) as f64;
    // The measures in which the voices sing, up to the slack of the last ones.
    let singing = (measures.saturating_sub(SLACK).max(1)) as f64 - start_in_measures(part.start);
    let per_measure = starts / singing.max(1.0);
    Ok(Plan {
        measures,
        cadences: cadences_for(measures),
        rate: (per_measure * 50.0).round().clamp(1.0, f64::from(u16::MAX)) as u16,
        statements,
    })
}

/// The length of a setting of `text` planned from the text alone: `max(8, round(m·S / rate) +
/// 2)`, with m statements of its S syllables at its rate (syllable starts per measure).
pub fn automatic_length(text: &Text, restatements: bool) -> usize {
    let starts = (statements(text, restatements) * text.syllables.len()) as f64;
    let measures = (starts / rate(text)).round() as usize + SLACK;
    measures.max(SHORTEST)
}

/// The planned syllable starts per measure: after PAL-DENS-1, by the kind of text and, for the
/// short sections of the Ordinary, by how melismatic Palestrina sets them. Sections sung one after
/// the other as one text (`sanctus+pleni+osanna`, phase 14) go at the pace of the first.
fn rate(text: &Text) -> f64 {
    let section = text.id.split([':', '+']).next().unwrap_or_default();
    match text.kind {
        TextKind::Long => LONG_RATE,
        TextKind::Short if MELISMATIC.contains(&section) => MELISMATIC_RATE,
        TextKind::Short => SHORT_RATE,
    }
}

/// Whether `text` is planned at the pace of the short texts that Palestrina sets with long
/// melismas ([`MELISMATIC`]): its automatic length leaves room for them.
pub(crate) fn melismatic(text: &Text) -> bool {
    rate(text) == MELISMATIC_RATE
}

/// How many times a text is planned to be sung: a short text about as often as makes
/// [`PLANNED_SYLLABLES`], a long one once, and without restatements every text once.
fn statements(text: &Text, restatements: bool) -> usize {
    if !restatements || text.kind == TextKind::Long {
        return 1;
    }
    let times = (PLANNED_SYLLABLES / text.syllables.len() as f64).round() as usize;
    times.max(1)
}

/// How a planned place for an internal cadence ends the words before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Ending {
    /// A sentence ends (`.` `?` `!`).
    Full,
    /// A unit ends at an intermediate stop (`,` `;` `:`).
    Intermediate,
    /// A marked break inside a unit: a `|`, or the words before a tail `/`.
    Break,
}

/// The internal cadences of a setting of `measures` measures in which the text is sung
/// `statements` times, as a part `part`: at the goals of [`internal_goals`], with targets after
/// how the words end there and the part's chain ([`targets_with`]).
fn internal_cadences(
    text: &Text,
    mode: ModeSpec,
    measures: usize,
    statements: usize,
    part: &PartContext,
) -> Vec<Cadence> {
    let goals = internal_goals(text, measures, statements, part.start, part.first_goal());
    targets_with(mode, goals, &part.chain)
}

/// The measures a part's start leaves silent before its voices sing, counted whole: one for a
/// breath of a minim as for a whole measure's rest.
fn silent_measures(start: u32) -> usize {
    start.div_ceil(Meter::ALLA_BREVE.ticks_per_measure()) as usize
}

/// A part's start in measures: ½ for a breath of a minim, 1 after a whole measure's rest.
fn start_in_measures(start: u32) -> f64 {
    f64::from(start) / f64::from(Meter::ALLA_BREVE.ticks_per_measure())
}

/// Where the internal cadences of a setting of `measures` measures in which the text is sung
/// `statements` times arrive, and how the words end there: about one per six measures, spread as
/// evenly as [`crate::generate::default_cadences`] spreads them, each moved to the nearest
/// planned end of a unit or marked break within [`REACH`] measures. An intermediate stop or a
/// break counts a measure farther, and of two places equally near the end of a unit wins over a
/// break. A short text cadences at most once per planned end of a unit but the last, and once at
/// a break if it has no other place (Pleni, Benedictus). Those that leave no room for their
/// clausulas drop out: the first comes in measure `first` (from 0) or later. Where the voices
/// begin only at tick `start` (a part after a breath or a break), the syllables and the cadences
/// spread over the measures from there, and the measures before it count for no cadence.
pub(crate) fn internal_goals(
    text: &Text,
    measures: usize,
    statements: usize,
    start: u32,
    first: usize,
) -> Vec<(usize, Ending)> {
    let syllables = text.syllables.len();
    let sequence = statements * syllables;
    // Where the i-th of `of` evenly spread places falls, in measures from 0: over the whole
    // setting, or from the start on.
    let from = start_in_measures(start);
    let spread = |i: usize, of: usize| -> f64 {
        let even = ((measures - 1) * i) as f64 / of as f64;
        if start == 0 {
            even
        } else {
            let last = (measures - 1) as f64;
            from + even * (last - from) / last
        }
    };
    // Every planned end of a unit or break but the last syllable, at the measure its syllable
    // falls in when the syllables are spread evenly over the setting, with how it ends.
    let candidates: Vec<(usize, Ending)> = (0..statements)
        .flat_map(|j| (0..syllables).map(move |s| (j, s)))
        .filter_map(|(j, s)| {
            let flags = text.syllables[s].flags;
            let ending = if flags.contains(SylFlags::UNIT_END) {
                match text.unit_of(s).stop {
                    Stop::Full => Ending::Full,
                    Stop::Intermediate => Ending::Intermediate,
                }
            } else if flags.contains(SylFlags::BREAK_AFTER) {
                Ending::Break
            } else {
                return None;
            };
            let i = j * syllables + s;
            (i + 1 < sequence).then(|| (spread(i, sequence).round() as usize, ending))
        })
        .collect();
    let mut count = measures
        .saturating_sub(silent_measures(start))
        .saturating_sub(4)
        / 6;
    if text.kind == TextKind::Short {
        let ends = statements * text.units.len() - 1;
        let breaks = candidates
            .iter()
            .any(|&(_, ending)| ending == Ending::Break);
        count = count.min(ends.max(usize::from(breaks)));
    }
    let mut taken = vec![false; candidates.len()];
    let mut goals = Vec::new();
    for p in 1..=count {
        let even = spread(p, count + 1).round() as usize;
        let nearest = candidates
            .iter()
            .enumerate()
            .filter(|&(c, _)| !taken[c])
            .map(|(c, &(goal, ending))| {
                let distance = goal.abs_diff(even) + usize::from(ending != Ending::Full);
                (distance, ending == Ending::Break, c)
            })
            .filter(|&(distance, _, _)| distance <= REACH)
            .min();
        if let Some((_, _, c)) = nearest {
            taken[c] = true;
            goals.push(candidates[c]);
        }
    }
    goals.sort_unstable();
    // The spacing that free counterpoint checks: the first goal in the third measure or later
    // (after a start, as `first` says), the others three measures apart, the last three before
    // the final.
    let mut kept: Vec<(usize, Ending)> = Vec::new();
    for (goal, ending) in goals {
        let after = kept
            .last()
            .map_or(first, |&(before, _)| before + CADENCE_SPACING);
        if goal >= after && goal + CADENCE_SPACING < measures {
            kept.push((goal, ending));
        }
    }
    kept
}

/// The targets of internal cadences arriving in the given measures, after the endings there: the
/// mode's regular degrees in turn, at intermediate stops and breaks only the degrees after the
/// first. A principal cadence belongs at the end of a period (Zarlino, TXT-CADENCE
/// `principal`). Two cadences in a row do not share a degree, the second takes the next degree
/// of its turn; but where that turn has a single degree, as at the intermediate stops of the
/// Lydian and the Mixolydian mode, it comes again rather than the principal one.
pub(crate) fn targets(mode: ModeSpec, goals: Vec<(usize, Ending)>) -> Vec<Cadence> {
    let degrees = regular_cadences(mode);
    let lesser = match degrees.len() {
        0 | 1 => &degrees[..],
        _ => &degrees[1..],
    };
    let mut full_turn = degrees.iter().copied().cycle();
    let mut lesser_turn = lesser.iter().copied().cycle();
    let mut before: Option<Step> = None;
    goals
        .into_iter()
        .filter_map(|(measure, ending)| {
            let turn = match ending {
                Ending::Full => &mut full_turn,
                Ending::Intermediate | Ending::Break => &mut lesser_turn,
            };
            let mut target = turn.next()?;
            if Some(target) == before {
                target = turn.next()?;
            }
            before = Some(target);
            Some(Cadence { measure, target })
        })
        .collect()
}

/// How the targets of a setting's internal cadences follow one another.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CadenceChain {
    /// The mode's regular degrees in turn ([`targets`]), as in every setting of its own (phases
    /// 11 to 13).
    #[default]
    Rotation,
    /// The chain over a movement of a mass (phase 14, MASS-CADENCES): each cadence repeats the
    /// degree before it with p .30 ([`CHAIN_REPEAT`]), as 28–44 % of Palestrina's consecutive
    /// cadences do, and otherwise takes another degree by `weights`. There the final and the
    /// mode's first regular degree weigh ×0.3 at an intermediate stop, a caesura or before a tail
    /// ([`CHAIN_FINAL_AND_FIRST_AT_STOPS`]): a principal cadence belongs at the end of a period,
    /// and TXT-CADENCE `principal` still prices them there. In the Christe the first cadence takes
    /// the final ×0.4 ([`CHAIN_CHRISTE_FINAL`]): it avoids the final early (where Kyrie I ended on
    /// the final, the draw of another degree leaves the final out anyway). Cadence i draws from a
    /// generator of its own (`seed ^ i`) and the degree of cadence i − 1, so the chain is
    /// evaluated by index: planning a later point anew never changes an earlier cadence.
    Weighted {
        /// The seed of the chain, the part's.
        seed: u64,
        /// The degrees and their weights in per mille, such that the cadences come out with the
        /// shares of Palestrina's internal full cadences in the mode (`mass::tables`). Degrees
        /// without a clausula in the mode drop out.
        weights: Vec<(Step, u16)>,
        /// The degree before the first cadence: the planned ending of the part before.
        before: Option<Step>,
        /// The part begins the Christe.
        christe: bool,
    },
}

/// How often a cadence of a weighted chain repeats the degree of the cadence before it: the
/// corpus has the same degree twice in a row in 28–44 % of its consecutive internal cadences.
pub(crate) const CHAIN_REPEAT: f64 = 0.30;

/// The factor on the weights of the final and of the mode's first regular degree
/// ([`regular_cadences`]`(mode)[0]`, the degrees of a principal cadence in TXT-CADENCE's sense) at
/// an intermediate stop, a caesura or before a tail, in a weighted chain: such cadences belong at
/// the end of a period (Zarlino III.53), so there they stay possible, only rarer. It applies to
/// these two degrees in every mode, not to the principal degree P that a mass draws beside the
/// final, which may be another one (the fourth in the Phrygian and the Aeolian, C or D in the
/// Mixolydian).
pub(crate) const CHAIN_FINAL_AND_FIRST_AT_STOPS: f64 = 0.3;

/// The factor on the weight of the final at the first cadence of the Christe, which avoids the
/// final early: 30 % of its articulated cadences in its first fifth lie on the final against 80 %
/// in Kyrie I, 24 % of all its internal cadences there against 61 % (verified corpus counts).
/// In a mass the Christe's chain starts from the ending of Kyrie I, which is the final but in the
/// Phrygian (or after `--endings`); a degree other than the one before is drawn without it, so
/// the factor acts only where Kyrie I ended elsewhere, and after a Kyrie I on the final the first
/// cadence of the Christe lies on the final only where it repeats that ending ([`CHAIN_REPEAT`]).
pub(crate) const CHAIN_CHRISTE_FINAL: f64 = 0.4;

/// The targets of internal cadences arriving in the given measures, after the endings there, as
/// `chain` gives them: the rotation of [`targets`], or a weighted chain evaluated by index (see
/// [`CadenceChain::Weighted`]). Where a weighted cadence finds no degree to draw (no weights
/// besides the degree before), it repeats that degree, and without one it takes the mode's first
/// regular degree.
pub(crate) fn targets_with(
    mode: ModeSpec,
    goals: Vec<(usize, Ending)>,
    chain: &CadenceChain,
) -> Vec<Cadence> {
    let CadenceChain::Weighted {
        seed,
        weights,
        before,
        christe,
    } = chain
    else {
        return targets(mode, goals);
    };
    let final_step = mode.final_step();
    // The mode's first regular degree: with the final the degree of a principal cadence.
    let first = regular_cadences(mode).first().copied();
    let mut previous = before.filter(|&step| mode.clausula(step).is_some());
    goals
        .into_iter()
        .enumerate()
        .filter_map(|(i, (measure, ending))| {
            let mut rng = Rng::new(splitmix64(seed ^ i as u64));
            let repeat = rng.next_f64() < CHAIN_REPEAT;
            let target = match previous {
                Some(step) if repeat => step,
                _ => {
                    let candidates: Vec<(Step, f64)> = weights
                        .iter()
                        .filter(|&&(step, weight)| {
                            weight > 0 && Some(step) != previous && mode.clausula(step).is_some()
                        })
                        .map(|&(step, weight)| {
                            let mut weight = f64::from(weight);
                            let final_or_first = step == final_step || Some(step) == first;
                            if ending != Ending::Full && final_or_first {
                                weight *= CHAIN_FINAL_AND_FIRST_AT_STOPS;
                            }
                            if *christe && i == 0 && step == final_step {
                                weight *= CHAIN_CHRISTE_FINAL;
                            }
                            (step, weight)
                        })
                        .collect();
                    drawn(&mut rng, &candidates).or(previous).or(first)?
                }
            };
            previous = Some(target);
            Some(Cadence { measure, target })
        })
        .collect()
}

/// One of `items` drawn by its weight, in their order; `None` without any.
fn drawn(rng: &mut Rng, items: &[(Step, f64)]) -> Option<Step> {
    let total: f64 = items.iter().map(|&(_, weight)| weight).sum();
    let mut u = rng.next_f64() * total;
    for &(item, weight) in items {
        if u < weight {
            return Some(item);
        }
        u -= weight;
    }
    items.last().map(|&(item, _)| item)
}

/// The least number of minim beats on which a voice of a free setting can begin a syllable: the
/// beats of the setting, without those its cadence formulas hold (the tied half of the
/// cantizans, the second half of a whole note), without the second beat of the held last
/// measure, and in a part (`part`) without the silence before its start and the second beat of
/// its opening chord. As the search counts it ([`Capacity`]) where syllables begin on the minim
/// beats.
fn capacity(
    mode: ModeSpec,
    voices: usize,
    measures: usize,
    cadences: &[Cadence],
    part: &PartContext,
) -> u32 {
    let meter = Meter::ALLA_BREVE;
    let per_measure = meter.ticks_per_measure();
    let total = measures as u32 * per_measure;
    let ending = part.ending_in(mode);
    let mut fixed = cadence_formula(mode, ending, measures - 1, voices, meter);
    for cadence in cadences {
        fixed.extend(cadence_formula(
            mode,
            cadence.target,
            cadence.measure,
            voices,
            meter,
        ));
    }
    fixed.extend(part.opening(mode, voices, meter));
    // Syllables on the minim beats only, as TXT-ONSET wants them.
    let onset = (Case::Onset.rule(), Case::Onset.param());
    let levels = Levels::new(
        |rule, param| {
            if (rule, param) == onset {
                Level::Forbidden
            } else {
                Level::Off
            }
        },
        |_, _| 0,
    );
    (0..voices.max(1))
        .map(|k| {
            // A rest is held from its first tick: no syllable begins there.
            let mut held: Vec<(u32, u32, bool)> = fixed
                .iter()
                .filter(|f| f.voice == k)
                .map(|f| {
                    (
                        f.start,
                        f.note.duration.ticks(),
                        f.tied_in || f.note.is_rest(),
                    )
                })
                .collect();
            held.push((total - per_measure, per_measure, false));
            // Every voice is silent before the part's start.
            if part.start > 0 {
                held.push((0, part.start, true));
            }
            Capacity::new(total, &held, &levels).at(0)
        })
        .min()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::part_endings;
    use crate::pitch::Step;
    use crate::text::{SylFlags, TextOptions, ordinary, resolve};

    fn text(spec: &str) -> Text {
        resolve(spec, &TextOptions::default()).unwrap().0
    }

    fn dorian() -> ModeSpec {
        "dorian".parse().unwrap()
    }

    /// The measures of a plan's internal cadences.
    fn goals(plan: &Plan) -> Vec<usize> {
        plan.cadences.iter().map(|c| c.measure).collect()
    }

    #[test]
    fn automatic_lengths_follow_the_rates() {
        for (id, measures) in [
            ("kyrie", 16),
            ("christe", 14),
            ("agnus", 20),
            ("crucifixus", 22),
            ("sanctus", 25),
            ("osanna", 25),
            ("benedictus", 25),
            ("gloria:1-5", 32),
            ("gloria", 84),
        ] {
            assert_eq!(automatic_length(&text(id), true), measures, "{id}");
            let plan = plan(&text(id), dorian(), 4, None, None, true).unwrap();
            assert_eq!(plan.measures, measures, "{id}");
        }
        // Without restatements Kyrie I is sung once.
        assert_eq!(automatic_length(&text("kyrie"), false), 9);
        // An own text: short up to 20 syllables, sung about 14 syllables' worth.
        let own = text("Ad te le-vá-vi á-ni-mam me-am.");
        assert_eq!((own.syllables.len(), own.kind), (10, TextKind::Short));
        assert_eq!(automatic_length(&own, true), 12);
    }

    #[test]
    fn kyrie_cadences_after_its_first_statement() {
        let plan = plan(&text("kyrie"), dorian(), 4, None, None, true).unwrap();
        assert_eq!(
            plan,
            Plan {
                measures: 16,
                cadences: vec![Cadence {
                    measure: 6,
                    target: Step::A
                }],
                rate: 50,
                statements: 2,
            }
        );
        // Sung once, eight measures and no internal cadence.
        let once = super::plan(&text("kyrie"), dorian(), 4, None, None, false).unwrap();
        assert_eq!((once.measures, once.statements), (9, 1));
        assert!(once.cadences.is_empty());
    }

    #[test]
    fn gloria_cadences_after_its_sentences() {
        let gloria = text("gloria:1-5");
        let plan = plan(&gloria, dorian(), 4, None, None, true).unwrap();
        assert_eq!((plan.measures, plan.statements, plan.rate), (32, 1, 60));
        assert_eq!(goals(&plan), [12, 21, 25]);
        // There the syllables, spread evenly over the setting, end the units of voluntatis,
        // Benedicimus te and Adoramus te, all at full stops.
        let ended: Vec<String> = gloria
            .units
            .iter()
            .filter(|unit| {
                let at = (31 * usize::from(unit.last)) as f64 / 36.0;
                goals(&plan).contains(&(at.round() as usize))
            })
            .inspect(|unit| {
                let last = &gloria.syllables[usize::from(unit.last)];
                assert!(last.flags.contains(SylFlags::FULL_STOP));
            })
            .map(|unit| {
                let syllables: Vec<&str> = unit
                    .range()
                    .map(|s| gloria.syllables[s].text.as_str())
                    .collect();
                syllables.join("-")
            })
            .collect();
        assert_eq!(ended.len(), 3);
        assert!(ended[0].ends_with("vo-lun-tá-tis"), "{ended:?}");
        assert!(ended[1].starts_with("Be-ne-dí-ci-mus"), "{ended:?}");
        assert!(ended[2].starts_with("Ad-o-rá-mus"), "{ended:?}");
    }

    #[test]
    fn short_texts_cadence_at_most_between_their_units() {
        // Agnus III: three units sung once, of which only the second ends near an even place,
        // at the colon after mundi. An intermediate stop takes a cadence on a degree that is not
        // principal: F, the Dorian third.
        let plan = plan(&text("agnus3"), dorian(), 4, None, None, true).unwrap();
        assert_eq!(
            (plan.measures, plan.cadences),
            (
                20,
                vec![Cadence {
                    measure: 12,
                    target: Step::F
                }]
            )
        );
        // An own text of one unit sung once has nowhere to cadence.
        let own = text("Ad te le-vá-vi á-ni-mam me-am.");
        let plan = super::plan(&own, dorian(), 3, None, None, true).unwrap();
        assert_eq!((plan.measures, plan.statements), (12, 1));
        assert!(plan.cadences.is_empty());
    }

    #[test]
    fn given_lengths_need_room_for_the_text() {
        let kyrie = text("kyrie");
        let error = plan(&kyrie, dorian(), 4, Some(4), None, true).unwrap_err();
        let GenerateError::TextTooLong { measures } = error else {
            panic!("{error:?}");
        };
        assert!(plan(&kyrie, dorian(), 4, Some(measures), None, true).is_ok());
        assert!(plan(&kyrie, dorian(), 4, Some(measures - 1), None, true).is_err());
        assert_eq!(
            error.message(crate::lang::Lang::De),
            format!("Der Text braucht mindestens {measures} Takte.")
        );
        assert_eq!(
            error.message(crate::lang::Lang::En),
            format!("The text needs at least {measures} measures.")
        );
        // The 98 syllables of the Gloria do not fit into 40 measures, Kyrie I fits into twelve.
        assert!(matches!(
            plan(&text("gloria"), dorian(), 4, Some(40), None, true),
            Err(GenerateError::TextTooLong { measures: 41.. })
        ));
        let twelve = plan(&kyrie, dorian(), 4, Some(12), None, true).unwrap();
        assert_eq!(twelve.measures, 12);
        // A longer setting declaims more slowly.
        assert!(twelve.rate > 50);
        assert!(plan(&kyrie, dorian(), 4, Some(3), None, true).is_err());
    }

    /// The Sanctus, the Pleni and the Benedictus have no end of a unit near an even place, the
    /// last two only their final one: each cadences once where a marked break ends its words, a
    /// measure farther counted, and on a degree that is not principal.
    #[test]
    fn melismatic_texts_cadence_at_a_break() {
        for (id, measures, goal, before, after) in [
            ("sanctus", 25, 9, "ctus", "Dó"),
            ("pleni", 24, 12, "ra", "gló"),
            ("benedictus", 25, 10, "nit", "in"),
        ] {
            let text = text(id);
            let plan = plan(&text, dorian(), 4, None, None, true).unwrap();
            assert_eq!(plan.measures, measures, "{id}");
            assert_eq!(
                plan.cadences,
                [Cadence {
                    measure: goal,
                    target: Step::F
                }],
                "{id}"
            );
            // The syllable whose place, the syllables spread evenly over the setting, is the
            // goal: the last before the break.
            let s = (0..text.syllables.len())
                .find(|&s| {
                    let at = ((measures - 1) * s) as f64 / text.syllables.len() as f64;
                    text.syllables[s].flags.contains(SylFlags::BREAK_AFTER)
                        && at.round() as usize == goal
                })
                .unwrap_or_else(|| panic!("{id}: no break at the goal"));
            assert_eq!(text.syllables[s].text, before, "{id}");
            assert_eq!(text.syllables[s + 1].text, after, "{id}");
            assert!(
                !text.syllables[s].flags.contains(SylFlags::UNIT_END),
                "{id}"
            );
        }
        // Their texts sung without a break have no place to cadence: an own text of one unit.
        let own = text("Be-ne-dí-ctus qui ve-nit in nó-mi-ne Dó-mi-ni.");
        let plan = super::plan(&own, dorian(), 4, Some(25), None, true).unwrap();
        assert!(plan.cadences.is_empty());
    }

    /// Two internal cadences in a row do not share a degree, though the full stops and the
    /// intermediate ones of a long text take their degrees in turns of their own; only where the
    /// mode has a single degree for the intermediate stops (Lydian, Mixolydian) does it come
    /// again.
    #[test]
    fn cadences_in_a_row_change_their_degree() {
        let gloria = plan(&text("gloria"), dorian(), 4, None, None, true).unwrap();
        let targets: String = gloria.cadences.iter().map(|c| c.target.letter()).collect();
        assert_eq!(targets, "AFGAFGFGAFGF");
        let mut again = 0;
        for section in ordinary::sections(&TextOptions::default()) {
            for mode in crate::mode::Mode::ALL {
                let mode = ModeSpec::new(mode);
                let degrees = regular_cadences(mode);
                let plan = plan(section, mode, 4, None, None, true).unwrap();
                for pair in plan.cadences.windows(2) {
                    let label = format!("{} {}", section.id, mode.id());
                    if pair[0].target == pair[1].target {
                        assert_eq!(degrees.len(), 2, "{label}");
                        assert_eq!(pair[1].target, degrees[1], "{label}");
                        again += 1;
                    }
                }
            }
        }
        assert!(again > 0);
    }

    /// The principal degree, the mode's first regular one, takes no intermediate stop or break:
    /// in the Mixolydian mode (D, then C) the intermediate stops take C, also twice in a row,
    /// and the full stops D and C in turn; in the Dorian mode (A, then F and G) the intermediate
    /// stops take F and G in turn.
    #[test]
    fn principal_cadences_end_periods() {
        use Ending::*;
        let letters = |mode: &str, endings: &[Ending]| -> String {
            let goals = endings
                .iter()
                .enumerate()
                .map(|(i, &ending)| (2 + 3 * i, ending))
                .collect();
            targets(mode.parse().unwrap(), goals)
                .iter()
                .map(|c| c.target.letter())
                .collect()
        };
        let endings = [Intermediate, Break, Full, Intermediate, Full, Full, Break];
        assert_eq!(letters("mixolydian", &endings), "CCDCDCC");
        assert_eq!(letters("lydian", &endings), "AACACAA");
        assert_eq!(letters("dorian", &endings), "FGAFGAG");
        // So in every section of the Ordinary in every mode.
        let mut lesser = 0;
        for section in ordinary::sections(&TextOptions::default()) {
            for mode in crate::mode::Mode::ALL {
                let mode = ModeSpec::new(mode);
                let principal = regular_cadences(mode)[0];
                let measures = automatic_length(section, true);
                let statements = statements(section, true);
                let goals = internal_goals(section, measures, statements, 0, 2);
                let part = PartContext::default();
                let cadences = internal_cadences(section, mode, measures, statements, &part);
                assert_eq!(goals.len(), cadences.len());
                for (&(_, ending), cadence) in goals.iter().zip(&cadences) {
                    if ending != Full {
                        assert_ne!(cadence.target, principal, "{} {}", section.id, mode.id());
                        lesser += 1;
                    }
                }
            }
        }
        assert!(lesser > 0);
    }

    #[test]
    fn given_cadences_are_kept() {
        let given = vec![Cadence {
            measure: 9,
            target: Step::F,
        }];
        let plan = plan(&text("kyrie"), dorian(), 3, None, Some(given.clone()), true).unwrap();
        assert_eq!((plan.measures, plan.cadences), (16, given));
        let none = super::plan(&text("kyrie"), dorian(), 3, None, Some(Vec::new()), true).unwrap();
        assert!(none.cadences.is_empty());
    }

    /// Every section of the Ordinary fits at its automatic length, in any mode and choir, with
    /// cadences spaced as free counterpoint needs them.
    #[test]
    fn every_section_has_a_plan() {
        for section in ordinary::sections(&TextOptions::default()) {
            for mode in crate::mode::Mode::ALL {
                for voices in 2..=6 {
                    let mode = ModeSpec::new(mode);
                    let plan = plan(section, mode, voices, None, None, true).unwrap();
                    assert_eq!(
                        plan.measures,
                        automatic_length(section, true),
                        "{}",
                        section.id
                    );
                    let mut before = 0;
                    for (i, cadence) in plan.cadences.iter().enumerate() {
                        let room = if i == 0 { 2 } else { CADENCE_SPACING };
                        assert!(cadence.measure >= before + room, "{}", section.id);
                        assert!(mode.clausula(cadence.target).is_some());
                        before = cadence.measure;
                    }
                    assert!(before + CADENCE_SPACING < plan.measures, "{}", section.id);
                }
            }
        }
    }

    /// The modes in every variant: authentic and plagal, untransposed and transposed.
    fn every_mode() -> Vec<ModeSpec> {
        use crate::mode::{Ambitus, Mode};
        let mut out = Vec::new();
        for mode in Mode::ALL {
            for ambitus in [Ambitus::Authentic, Ambitus::Plagal] {
                for transposed in [false, true] {
                    out.push(
                        ModeSpec::new(mode)
                            .with_ambitus(ambitus)
                            .with_transposition(transposed),
                    );
                }
            }
        }
        out
    }

    /// A list of goals drawn by `rng`: up to twelve, three or more measures apart, each ending a
    /// sentence, a unit or a marked break.
    fn random_goals(rng: &mut Rng) -> Vec<(usize, Ending)> {
        let count = (rng.next_f64() * 13.0) as usize;
        let mut measure = 2;
        (0..count)
            .map(|_| {
                let ending = match (rng.next_f64() * 3.0) as usize {
                    0 => Ending::Full,
                    1 => Ending::Intermediate,
                    _ => Ending::Break,
                };
                let goal = (measure, ending);
                measure += 3 + (rng.next_f64() * 5.0) as usize;
                goal
            })
            .collect()
    }

    /// The weights of the Dorian mode on D (internal full cadences in Palestrina's masses, ‰).
    fn dorian_weights() -> Vec<(Step, u16)> {
        use Step::*;
        vec![(D, 448), (A, 349), (G, 67), (C, 63), (F, 59)]
    }

    /// The default chain is the rotation of the phases before: the targets of every goal list in
    /// every mode are those of [`targets`].
    #[test]
    fn rotation_is_the_default() {
        assert_eq!(CadenceChain::default(), CadenceChain::Rotation);
        let mut rng = Rng::new(14);
        let mut cadences = 0;
        for mode in every_mode() {
            for _ in 0..40 {
                let goals = random_goals(&mut rng);
                let rotated = targets(mode, goals.clone());
                cadences += rotated.len();
                assert_eq!(
                    targets_with(mode, goals, &CadenceChain::default()),
                    rotated,
                    "{}",
                    mode.id()
                );
            }
        }
        assert!(cadences > 1000, "{cadences} cadences");
    }

    /// A chain gives the same cadences to the goals that two lists share at their start, however
    /// the lists go on: a goal's target depends only on the goals up to it, so planning a later
    /// point anew never changes an earlier cadence. So for the rotation and for weighted chains
    /// of any seed, degree before and Christe.
    #[test]
    fn chain_is_stable_by_index() {
        let mut rng = Rng::new(3);
        let mut compared = 0;
        for round in 0..300u64 {
            let mode = every_mode()[round as usize % 24];
            let chain = if round % 5 == 0 {
                CadenceChain::Rotation
            } else {
                let before = part_endings(mode)[(rng.next_f64() * 2.0) as usize];
                CadenceChain::Weighted {
                    seed: rng.next_u64(),
                    weights: regular_cadences(mode)
                        .into_iter()
                        .chain([mode.final_step()])
                        .zip([300, 200, 100, 400])
                        .collect(),
                    before: (round % 3 != 0).then_some(before),
                    christe: round % 4 == 1,
                }
            };
            let goals = random_goals(&mut rng);
            let whole = targets_with(mode, goals.clone(), &chain);
            assert_eq!(whole.len(), goals.len());
            for k in 0..=goals.len() {
                let mut other = goals[..k].to_vec();
                // Another continuation: other endings and places after the shared goals.
                other.extend(
                    random_goals(&mut rng)
                        .into_iter()
                        .map(|(m, e)| (m + 100, e)),
                );
                let prefix = targets_with(mode, goals[..k].to_vec(), &chain);
                assert_eq!(prefix, whole[..k], "{} {chain:?}", mode.id());
                assert_eq!(targets_with(mode, other, &chain)[..k], whole[..k]);
                compared += k;
            }
        }
        assert!(compared > 1000, "{compared}");
    }

    /// The shares of the degrees a weighted chain draws, over 20 000 seeds: the weights at a full
    /// stop; the final and the first regular degree (A in the Dorian mode) ×0.3 at an
    /// intermediate stop or a break; the final ×0.4 at the first cadence of the Christe; and
    /// after a degree before, that degree again with p .30, the others by their weights.
    #[test]
    fn weighted_chains_follow_their_weights() {
        use Step::*;
        let dorian = dorian();
        let shares = |goals: &[(usize, Ending)], before: Option<Step>, christe: bool, at: usize| {
            let mut counts: std::collections::BTreeMap<Step, f64> = Default::default();
            const SEEDS: u64 = 20_000;
            for seed in 0..SEEDS {
                let chain = CadenceChain::Weighted {
                    seed,
                    weights: dorian_weights(),
                    before,
                    christe,
                };
                let target = targets_with(dorian, goals.to_vec(), &chain)[at].target;
                *counts.entry(target).or_default() += 1.0 / SEEDS as f64;
            }
            counts
        };
        let near = |found: f64, wanted: f64, what: &str| {
            assert!(
                (found - wanted).abs() < 0.015,
                "{what}: {found:.3} against {wanted:.3}"
            );
        };
        let full = shares(&[(4, Ending::Full)], None, false, 0);
        for (step, weight) in dorian_weights() {
            near(full[&step], f64::from(weight) / 986.0, "full stop");
        }
        // At a comma the final and A weigh ×0.3: D 134.4, A 104.7, G 67, C 63, F 59.
        let comma = shares(&[(4, Ending::Intermediate)], None, false, 0);
        near(comma[&D], 134.4 / 428.1, "comma D");
        near(comma[&A], 104.7 / 428.1, "comma A");
        near(comma[&G], 67.0 / 428.1, "comma G");
        let tail = shares(&[(4, Ending::Break)], None, false, 0);
        near(tail[&D], 134.4 / 428.1, "break D");
        // The Christe's first cadence: the final ×0.4; its second as any other.
        let christe = shares(&[(4, Ending::Full), (8, Ending::Full)], None, true, 0);
        near(christe[&D], 179.2 / 717.2, "Christe");
        // As a mass plans it, after Kyrie I on the final, which the draw of another degree
        // leaves out: the final only by the repeat, the factor without effect.
        let christe = shares(&[(4, Ending::Full), (8, Ending::Full)], Some(D), true, 0);
        near(christe[&D], CHAIN_REPEAT, "Christe after D");
        near(christe[&A], 0.70 * 349.0 / 538.0, "Christe after D, A");
        // After Kyrie I on A (the Phrygian may end it there) the factor acts.
        let christe = shares(&[(4, Ending::Full), (8, Ending::Full)], Some(A), true, 0);
        near(christe[&D], 0.70 * 179.2 / 368.2, "Christe after A");
        // After A, A again with p .30; the others share the rest by their weights.
        let after = shares(&[(4, Ending::Full)], Some(A), false, 0);
        near(after[&A], 0.30, "repeat");
        near(after[&D], 0.70 * 448.0 / 637.0, "after A, D");
        near(after[&F], 0.70 * 59.0 / 637.0, "after A, F");
        // Along a chain a cadence repeats the one before it three times in ten.
        let mut repeats = 0;
        let mut pairs = 0;
        for seed in 0..2_000 {
            let chain = CadenceChain::Weighted {
                seed,
                weights: dorian_weights(),
                before: Some(D),
                christe: false,
            };
            let goals: Vec<(usize, Ending)> = (0..10).map(|i| (3 + 3 * i, Ending::Full)).collect();
            let cadences = targets_with(dorian, goals, &chain);
            for pair in cadences.windows(2) {
                pairs += 1;
                repeats += usize::from(pair[0].target == pair[1].target);
            }
        }
        near(repeats as f64 / pairs as f64, 0.30, "repeats along a chain");
    }

    /// A weighted chain draws only degrees with a clausula in the mode, ignores a degree before
    /// that has none, and without weights to draw repeats the degree before or, without one,
    /// takes the mode's first regular degree.
    #[test]
    fn weighted_chains_keep_to_the_clausulas() {
        use Step::*;
        let goals = vec![
            (3, Ending::Full),
            (6, Ending::Intermediate),
            (9, Ending::Full),
        ];
        for seed in 0..200 {
            // B has a diminished fifth in the Dorian mode.
            let chain = CadenceChain::Weighted {
                seed,
                weights: vec![(B, 900), (D, 50), (A, 50)],
                before: Some(B),
                christe: false,
            };
            for cadence in targets_with(dorian(), goals.clone(), &chain) {
                assert!([D, A].contains(&cadence.target), "{cadence:?}");
            }
        }
        let empty = |before| CadenceChain::Weighted {
            seed: 1,
            weights: Vec::new(),
            before,
            christe: false,
        };
        let letters = |chain: &CadenceChain| -> String {
            targets_with(dorian(), goals.clone(), chain)
                .iter()
                .map(|c| c.target.letter())
                .collect()
        };
        assert_eq!(letters(&empty(Some(G))), "GGG");
        assert_eq!(letters(&empty(None)), "AAA");
    }

    /// A part that begins after a breath or a measure's rest plans a measure more; after a
    /// measure's rest it declaims at the pace of a part that begins at once, after a breath a
    /// little slower (the Kyrie at 48 against 50). Its first cadence comes two measures after the
    /// start, a measure later after an opening chord. Every section of the Ordinary has such
    /// plans, in every mode.
    #[test]
    fn parts_plan_after_their_start() {
        let kyrie = text("kyrie");
        let at = |start: u32, chord: bool| {
            let part = PartContext {
                start,
                chord: chord.then(|| vec!["A4".parse().unwrap(); 4]),
                ..PartContext::default()
            };
            plan_part(&kyrie, dorian(), 4, None, None, true, &part).unwrap()
        };
        let plain = plan(&kyrie, dorian(), 4, None, None, true).unwrap();
        assert_eq!(at(0, false), plain);
        assert_eq!((plain.measures, plain.rate), (16, 50));
        assert_eq!((at(8, false).measures, at(8, false).rate), (17, 50));
        assert_eq!((at(4, false).measures, at(4, false).rate), (17, 48));
        for section in ordinary::sections(&TextOptions::default()) {
            for mode in crate::mode::Mode::ALL {
                let mode = ModeSpec::new(mode);
                for start in [0, 4, 8] {
                    for chord in [false, true] {
                        let part = PartContext {
                            start,
                            chord: chord.then(|| vec!["A4".parse().unwrap(); 4]),
                            ..PartContext::default()
                        };
                        let label =
                            format!("{} {} start {start} chord {chord}", section.id, mode.id());
                        let planned = plan_part(section, mode, 4, None, None, true, &part).unwrap();
                        let least = automatic_length(section, true) + silent_measures(start);
                        assert!(planned.measures >= least, "{label}");
                        let mut before = 0;
                        for (i, cadence) in planned.cadences.iter().enumerate() {
                            let room = if i == 0 {
                                part.first_goal()
                            } else {
                                before + CADENCE_SPACING
                            };
                            assert!(cadence.measure >= room, "{label}: {:?}", planned.cadences);
                            before = cadence.measure;
                        }
                        assert!(before + CADENCE_SPACING < planned.measures, "{label}");
                    }
                }
            }
        }
    }

    /// The capacity of a part leaves out its silence before the start (a beat for a minim's
    /// breath, two for a measure's rest) and the second beat of its opening chord. Its ending's
    /// formula holds a beat in each of its voices, as the final's does.
    #[test]
    fn capacity_of_a_part() {
        let at = |mode: ModeSpec, start: u32, chord: bool, ending: Option<Step>| {
            let part = PartContext {
                start,
                chord: chord.then(|| vec!["A4".parse().unwrap(); 4]),
                ending,
                ..PartContext::default()
            };
            capacity(mode, 4, 16, &[], &part)
        };
        let whole = at(dorian(), 0, false, None);
        assert_eq!(
            whole,
            2 * 16 - 1 - 1,
            "the last beat and a beat of the formula"
        );
        assert_eq!(at(dorian(), 4, false, None), whole - 1);
        assert_eq!(at(dorian(), 8, false, None), whole - 2);
        assert_eq!(at(dorian(), 0, true, None), whole - 1);
        assert_eq!(at(dorian(), 4, true, None), whole - 2);
        assert_eq!(at(dorian(), 8, true, None), whole - 3);
        for ending in [Step::D, Step::A, Step::G] {
            assert_eq!(at(dorian(), 0, false, Some(ending)), whole);
        }
        let phrygian: ModeSpec = "phrygian".parse().unwrap();
        assert_eq!(at(phrygian, 0, false, Some(Step::A)), whole);
    }

    /// Sections sung one after the other as one text go at the pace of the first: the
    /// melismatic Sanctus with its Pleni and Osanna, an own Kyrie with a Sanctus as a short text.
    #[test]
    fn joined_texts_take_the_pace_of_their_first() {
        let mut joined = text("sanctus");
        joined.id = "sanctus+pleni+osanna".into();
        assert_eq!(rate(&joined), MELISMATIC_RATE);
        joined.id = "benedictus:1-2+osanna2".into();
        assert_eq!(rate(&joined), MELISMATIC_RATE);
        joined.id = "kyrie+sanctus".into();
        assert_eq!(rate(&joined), SHORT_RATE);
    }
}
