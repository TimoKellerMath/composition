//! The driver of imitative free settings (phase 13, docs/regeln.md § 18): the plan of the points,
//! probes of it, the search with the entries as fixed notes, re-planning the point that failed
//! before a wider beam, and the results with the plan in their scores.
//!
//! **Probes.** The planner's checks find the conflicts that are certain from the planned notes
//! alone. What only the free voices around them show, a probe finds: the texted search of the
//! plan with a beam of 20, no wider beam and one result, which takes a tenth of the search with
//! the request's beam. With the text, and so under the search's own profile, a probe takes about a
//! sixth longer than without it, but it misjudges far fewer plans: of the 48 first plans of Kyrie
//! I, the Christe and Agnus Dei I in four modes, three and four voices, a probe without the text
//! rejected 9 that the search finds and passed 3 that it does not, a texted one 2 and 4. Where a
//! probe fails, the measure it fails in is put down to a point ([`ImitationPlan::owner`]) and that
//! point is planned anew, its entry nearest the failure first in another octave. After six probes
//! (two in WebAssembly, where a search takes about ten times as long) the plan whose probe came
//! furthest is searched, and the planner goes back to it, so that a point planned anew after a
//! failed search keeps that plan's other points. A request's budget ([`Budget`]) may set the
//! probes and the widest beam otherwise: the parts of a mass take six probes and twice the beam
//! on every platform, so that a seed sets the same part natively and in WebAssembly.
//!
//! **Attempts.** The texted search tries the request's beam first. Where it fails, the point it
//! failed in is planned anew (and probed twice at most) before a wider beam, twice at most; only
//! then the beam doubles. A length planned from the text then grows by a quarter, and the
//! request's beam searches the longer plan first, as without imitation; natively four times the
//! beam comes last, which costs the most. Each search keeps its beam: the driver widens it, not
//! the search. Once a probe has found a setting, a wider beam is no longer tried: a plan whose
//! search failed with the request's beam rarely passes with a wider one, which takes twice or
//! four times as long; nor is the longer plan, unless the text found no room in the last measure
//! (TXT-COMPLETE). Where every search fails, the cheapest setting a probe found is the result: it
//! keeps every guarantee below, at the cost of the narrower beam. A length the request gives
//! holds for every plan: the planner gives up plans anew that do not fit into it.
//!
//! **Results.** Every result breaks no prohibition, every voice sings the whole text, and every
//! planned entry sounds as planned: at its tick, after at least its rest (from the start of the
//! setting for a voice's first entry), with the planned pitches and values and the leader's
//! syllables on the same head notes; no voice begins the words of a point before its entry in
//! it, or a voice without one before the point's second entry. The search keeps this by its fixed
//! notes, pinned syllables and gates, IMI-ENTRY confirms it, and a debug build asserts it. The
//! plan travels in the scores, and the results carry the warnings of their plan (a point whose
//! chant gave no subject). Where no attempt and no probe finds a setting, the request fails with
//! [`GenerateError::ImitationTooLong`] where the planner gave up plans anew for a given length,
//! else with [`GenerateError::NoImitation`]: the point the last search failed in, its words, the
//! measure and the rules. It never falls back on a setting without imitation.

use std::sync::Arc;

use super::counterpoint::{MAX_WIDENING, SungText, generate_counterpoint};
use super::free::{Budget, FreeRequest, FreeText, cancelled, free_search};
use super::{GenerateError, Generated, Warning};
use crate::imitation::planner::{Planner, Snapshot, plan_imitation};
use crate::imitation::{Imitation, ImitationPlan};
use crate::rules::Profile;
use crate::score::Score;

/// The probes of a plan made after a failed search: re-planned or longer.
const REPROBES: usize = 2;

/// The beam of a probe.
const PROBE_BEAM: usize = 20;

/// Ticks per 2/2 measure.
const MEASURE: u32 = 8;

/// The searches that the driver tries one after the other, until one finds a setting.
const ATTEMPTS: [Attempt; 6] = [
    Attempt::Beam(1),
    Attempt::Beam(1),
    Attempt::Beam(1),
    Attempt::Beam(2),
    Attempt::Longer,
    Attempt::Beam(4),
];

/// A search of the driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Attempt {
    /// The texted search of the plan with a multiple of the request's beam. With the request's
    /// beam again, the plan first takes another plan for the point that failed; without one, the
    /// attempt drops out.
    Beam(usize),
    /// The texted search of the plan a quarter longer, with the request's beam; only where the
    /// text plans the length. Where the text did not fit (TXT-COMPLETE in the last measure), the
    /// longer plan finds a setting where a wider beam runs into the same lack of room.
    Longer,
}

/// What the driver asks a search for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Search {
    /// The texted search of the plan with a beam of [`PROBE_BEAM`]: whether it finds a setting
    /// at all.
    Probe,
    /// The texted search of the plan with this multiple of the request's beam.
    Beam(usize),
}

/// How far the driver may go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Limits {
    /// The probes of the first plan.
    probes: usize,
    /// The widest beam, as a multiple of the request's.
    widening: usize,
    /// Whether the plan may grow longer: the text plans the length.
    longer: bool,
}

impl Limits {
    /// The limits of a request's `budget`, the widest beam within what the platform allows
    /// ([`MAX_WIDENING`]); with `longer` where the text plans the length.
    fn of(budget: Budget, longer: bool) -> Limits {
        Limits {
            probes: budget.probes,
            widening: budget.widening.min(MAX_WIDENING),
            longer,
        }
    }
}

/// Generates an imitative free setting of `request`, which sings `sung` with the imitation
/// `imitation`: plans its points, probes the plan and searches it as the module documentation
/// says, within the request's budget. The request has two voices or more and lets the text plan
/// its cadences. A length the request gives holds for every plan; where re-planning finds no plan
/// that fits into it and no setting is found, the request fails with
/// [`GenerateError::ImitationTooLong`], naming the shortest plan it laid out. Before every probe
/// and every search the driver reads the request's control, and a cancelled request fails with
/// [`GenerateError::Cancelled`]. The results carry the subject of each point of the plan they
/// were set by ([`Generated::subjects`]).
pub(super) fn generate_imitative(
    request: &FreeRequest,
    imitation: &Imitation,
    sung: &FreeText,
    profile: &Profile,
) -> Result<Vec<Generated>, GenerateError> {
    let mut planner = plan_imitation(request, imitation, profile)?;
    let limits = Limits::of(request.budget, sung.auto_length);
    let mut results = drive(&mut planner, limits, |plan, search| {
        if cancelled(request.control.as_deref()) {
            return Err(GenerateError::Cancelled);
        }
        let text = SungText {
            text: Arc::clone(&sung.text),
            rate: plan.rate,
        };
        let settings = match search {
            Search::Probe => setting(request, plan, Some(text), PROBE_BEAM, 1, profile),
            Search::Beam(width) => {
                let beam = request.beam.max(1) * width;
                setting(request, plan, Some(text), beam, request.count, profile)
            }
        };
        settings.map(|settings| with_subjects(plan, settings))
    })?;
    for result in &mut results {
        result.score.fill_syllables(sung.accents);
        debug_assert!(
            result
                .report
                .violations
                .iter()
                .all(|v| v.rule != "IMI-ENTRY"),
            "an entry departs from its plan: {:?}",
            result.report.violations
        );
        debug_assert_eq!(departure(&result.score), None);
        debug_assert!(
            sung.auto_length || result.score.measure_count() == request.measures,
            "{} measures for a given length of {}",
            result.score.measure_count(),
            request.measures
        );
    }
    Ok(results)
}

/// The settings of a search of `plan` with the subject of each of its points, as a mass reads the
/// head motif from them.
fn with_subjects(plan: &ImitationPlan, mut settings: Vec<Generated>) -> Vec<Generated> {
    for setting in &mut settings {
        setting.subjects = plan
            .points
            .iter()
            .map(|point| Arc::clone(&point.subject))
            .collect();
    }
    settings
}

/// The settings of a search with the warnings of the plan the planner has laid out: the points
/// whose chant gave no usable subject.
fn warned(planner: &Planner, mut settings: Vec<Generated>) -> Vec<Generated> {
    let warnings: Vec<Warning> = planner
        .fallbacks()
        .into_iter()
        .map(Warning::ChantFallback)
        .collect();
    for setting in &mut settings {
        setting.warnings.clone_from(&warnings);
    }
    settings
}

/// Runs the attempts on the plans of `planner` within `limits`, with `search` for the probes
/// and the searches, until one finds a setting; after the last, the cheapest setting a probe
/// found, or if none did [`GenerateError::ImitationTooLong`] where the failing point had no plan
/// left and re-planning gave up plans for a given length, else [`GenerateError::NoImitation`]; at
/// once any error but [`GenerateError::NoCounterpoint`]. The settings carry the warnings of
/// their plans.
fn drive(
    planner: &mut Planner,
    limits: Limits,
    mut search: impl FnMut(&ImitationPlan, Search) -> Result<Vec<Generated>, GenerateError>,
) -> Result<Vec<Generated>, GenerateError> {
    // The cheapest setting a probe found.
    let mut found: Option<Vec<Generated>> = None;
    let first = planner.plan();
    let mut plan = probed(
        planner,
        first,
        limits.probes,
        false,
        &mut search,
        &mut found,
    )?;
    // The measure and the rules of the last failed search, of `plan`.
    let mut failure: Option<(usize, Vec<&'static str>)> = None;
    // Whether the point that failed has no other plan left.
    let mut spent = false;
    for attempt in ATTEMPTS {
        // Whether the last search failed for lack of room for the text at the end.
        let crowded = failure.as_ref().is_some_and(|(measure, rules)| {
            *measure >= plan.measures && rules.contains(&"TXT-COMPLETE")
        });
        let width = match attempt {
            Attempt::Beam(width) if width > limits.widening => continue,
            Attempt::Beam(width) if width > 1 && found.is_some() => continue,
            Attempt::Beam(1) => {
                if let Some((measure, _)) = &failure {
                    if spent {
                        continue;
                    }
                    let measure = *measure;
                    match planner.replan_near(plan.owner(measure), Some(measure)) {
                        Some(next) => {
                            plan = probed(planner, next, REPROBES, false, &mut search, &mut found)?;
                        }
                        None => {
                            spent = true;
                            continue;
                        }
                    }
                }
                1
            }
            Attempt::Beam(width) => width,
            Attempt::Longer if !limits.longer || (found.is_some() && !crowded) => continue,
            Attempt::Longer => {
                let longer = planner.lengthen(&plan);
                plan = probed(planner, longer, REPROBES, true, &mut search, &mut found)?;
                1
            }
        };
        match search(&plan, Search::Beam(width)) {
            Err(GenerateError::NoCounterpoint { measure, rules }) => {
                failure = Some((measure, rules));
            }
            Ok(settings) => return Ok(warned(planner, settings)),
            Err(error) => return Err(error),
        }
    }
    if let Some(found) = found {
        return Ok(found);
    }
    // The plans left for the failing point did not fit into the given length.
    if !limits.longer
        && spent
        && let Some(measures) = planner.too_long()
    {
        return Err(GenerateError::ImitationTooLong { measures });
    }
    let (measure, rules) = failure.unwrap_or_default();
    let point = plan.owner(measure);
    Err(GenerateError::NoImitation {
        point: point + 1,
        words: planner.words(point),
        measure,
        rules,
    })
}

/// Keeps in `found` the cheaper of the settings it holds and those a probe found, if any.
fn keep(found: &mut Option<Vec<Generated>>, probe: Vec<Generated>) {
    let cost_of = |settings: &[Generated]| settings.first().map(|g| g.cost);
    if let Some(cost) = cost_of(&probe)
        && found
            .as_deref()
            .and_then(cost_of)
            .is_none_or(|kept| cost < kept)
    {
        *found = Some(probe);
    }
}

/// Probes `plan` and, while a probe fails and `budget` allows another, the plan with the point
/// it failed in planned anew (and a quarter longer if `longer`); the first plan a probe finds,
/// or once the budget or the alternatives are spent the one whose probe came furthest, the
/// earlier of two that came equally far, with the planner back at that plan, so that it plans
/// anew from there. Keeps in `found` the cheapest setting a probe found.
fn probed(
    planner: &mut Planner,
    mut plan: ImitationPlan,
    budget: usize,
    longer: bool,
    search: &mut impl FnMut(&ImitationPlan, Search) -> Result<Vec<Generated>, GenerateError>,
    found: &mut Option<Vec<Generated>>,
) -> Result<ImitationPlan, GenerateError> {
    // The measure the probe came to, the plan and what the planner had laid out for it.
    let mut furthest: Option<(usize, ImitationPlan, Snapshot)> = None;
    for probe in 1..=budget {
        let measure = match search(&plan, Search::Probe) {
            Ok(settings) => {
                keep(found, warned(planner, settings));
                return Ok(plan);
            }
            Err(GenerateError::NoCounterpoint { measure, .. }) => measure,
            Err(error) => return Err(error),
        };
        let owner = plan.owner(measure);
        if furthest.as_ref().is_none_or(|(m, _, _)| measure > *m) {
            furthest = Some((measure, plan.clone(), planner.snapshot()));
        }
        if probe == budget {
            break;
        }
        // A longer plan has the points a few measures later than the planner lays them out.
        let shift = if longer {
            let laid = planner.plan();
            let start = |plan: &ImitationPlan| plan.points[owner].entries[0].start;
            (start(&plan).saturating_sub(start(&laid)) / MEASURE) as usize
        } else {
            0
        };
        match planner.replan_near(owner, Some(measure.saturating_sub(shift))) {
            Some(next) if longer => plan = planner.lengthen(&next),
            Some(next) => plan = next,
            None => break,
        }
    }
    match furthest {
        Some((_, furthest, laid)) => {
            planner.restore(laid);
            Ok(furthest)
        }
        None => Ok(plan),
    }
}

/// The search of `plan` for `request`: its entries and formulas as fixed notes, the plan and its
/// cadences in the base score, the text `text` at the plan's pace, with the beam `beam` and no
/// wider one, for `count` results.
fn setting(
    request: &FreeRequest,
    plan: &ImitationPlan,
    text: Option<SungText>,
    beam: usize,
    count: usize,
    profile: &Profile,
) -> Result<Vec<Generated>, GenerateError> {
    let mut base = Score::new(request.mode, Vec::new());
    base.cadences = plan.cadences.clone();
    base.imitation = Some(Arc::new(plan.score_imitation()));
    let fixed = plan.fixed(request.mode, request.voices.len());
    let search = free_search(request, base, plan.measures, fixed, text, (beam, 1, count));
    generate_counterpoint(&search, profile)
}

/// The first planned entry of `score` that does not sound as its plan says, found without the
/// code of the rules: a head within an octave, at its tick, silent before it for at least its
/// rest (from the start for a rest of 0), each head note at its tick with its pitch and value
/// (the last one at least as long), and where the voices sing, the leader's syllables on the same
/// head notes, the first one beginning the point's words. `None` where every entry does, or the
/// score has no plan.
fn departure(score: &Score) -> Option<String> {
    let plan = score.imitation.as_deref()?;
    let sings = |v: usize| score.text.as_ref().is_some_and(|t| t.voices[v]);
    for (p, point) in plan.points.iter().enumerate() {
        // The syllables of the leader's head notes, by note.
        let mut syllables: Option<Vec<Option<usize>>> = None;
        for entry in &point.entries {
            let at = format!("point {}, voice {} at {}", p + 1, entry.voice, entry.start);
            let Some(voice) = score.voices.get(entry.voice) else {
                return Some(format!("{at}: no such voice"));
            };
            let steps = || entry.notes.iter().map(|&(pitch, _)| pitch.diatonic());
            if steps()
                .max()
                .zip(steps().min())
                .is_some_and(|(hi, lo)| hi - lo > 7)
            {
                return Some(format!("{at}: a head beyond an octave"));
            }
            let soundings = voice.soundings();
            let silent_from = entry.start.saturating_sub(entry.rest);
            let silent_from = if entry.rest == 0 { 0 } else { silent_from };
            if let Some(s) = soundings
                .iter()
                .find(|s| s.start < entry.start && s.end > silent_from)
            {
                return Some(format!("{at}: sounds {} at {}", s.pitch, s.start));
            }
            let mut tick = entry.start;
            let mut sung = Vec::with_capacity(entry.notes.len());
            for (i, &(pitch, ticks)) in entry.notes.iter().enumerate() {
                let last = i + 1 == entry.notes.len();
                let Some(s) = soundings.iter().find(|s| s.start == tick) else {
                    return Some(format!("{at}: no attack of note {} at {tick}", i + 1));
                };
                let value = s.end - s.start;
                if s.pitch != pitch || value < ticks || (value > ticks && !last) {
                    return Some(format!(
                        "{at}: note {} is {}.{value}, planned {pitch}.{ticks}",
                        i + 1,
                        s.pitch
                    ));
                }
                sung.push(voice.notes[s.note].lyric.and_then(|l| l.index()));
                tick += ticks;
            }
            if !sings(entry.voice) {
                continue;
            }
            let leader = syllables.get_or_insert_with(|| sung.clone());
            if sung[0] != Some(usize::from(point.words.start))
                || sung.iter().zip(leader.iter()).any(|(a, b)| a != b)
            {
                return Some(format!("{at}: sings {sung:?}, the leader {leader:?}"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::generate_free;
    use crate::imitation::test_support;
    use crate::mode::ModeSpec;

    fn dorian() -> ModeSpec {
        "dorian".parse().unwrap()
    }

    /// An imitative request on the text `spec` in the Dorian mode for the usual choir of
    /// `voices`, everything else automatic.
    fn request(spec: &str, voices: usize, seed: u64) -> FreeRequest {
        test_support::request(spec, "dorian", voices, seed)
    }

    fn planner(spec: &str, voices: usize, seed: u64) -> Planner {
        plan_imitation(
            &request(spec, voices, seed),
            &Imitation::default(),
            &Profile::fux(),
        )
        .unwrap()
    }

    /// What a fake search was asked: probe or beam, and the plan.
    type Asked = Vec<(Search, ImitationPlan)>;

    /// Runs [`drive`] on `planner` within `limits` with a fake search: `fails` says for each
    /// question (counted from 0) whether it fails, and in which measure, with the rule
    /// FREE-PASSING. Returns what the search was asked and the result.
    fn driven(
        planner: &mut Planner,
        limits: Limits,
        mut fails: impl FnMut(usize, Search, &ImitationPlan) -> Option<usize>,
    ) -> (Asked, Result<Vec<Generated>, GenerateError>) {
        let mut asked: Asked = Vec::new();
        let result = drive(planner, limits, |plan, search| {
            let failed = fails(asked.len(), search, plan);
            asked.push((search, plan.clone()));
            match failed {
                Some(measure) => Err(GenerateError::NoCounterpoint {
                    measure,
                    rules: vec!["FREE-PASSING"],
                }),
                None => Ok(Vec::new()),
            }
        });
        (asked, result)
    }

    const NATIVE: Limits = Limits {
        probes: 6,
        widening: 4,
        longer: true,
    };

    /// The searches asked for, without their plans.
    fn searches(asked: &Asked) -> Vec<Search> {
        asked.iter().map(|(search, _)| *search).collect()
    }

    /// Where a search fails, the point it failed in is planned anew and probed, and the
    /// request's beam searches that plan, twice at most; only then twice the beam, the plan a
    /// quarter longer with the request's beam, and four times the beam. Without another plan for
    /// the point, or in WebAssembly or at a given length, the attempts that need one drop out.
    #[test]
    fn replanning_comes_before_widening() {
        use Search::*;
        // Every search fails, every probe passes.
        let mut kyrie = planner("kyrie", 4, 2);
        let first = kyrie.plan();
        let fail_searches =
            |_: usize, search: Search, _: &ImitationPlan| (search != Probe).then_some(3);
        let (asked, result) = driven(&mut kyrie, NATIVE, fail_searches);
        assert_eq!(
            searches(&asked),
            [
                Probe,
                Beam(1),
                Probe,
                Beam(1),
                Probe,
                Beam(1),
                Beam(2),
                Probe,
                Beam(1),
                Beam(4)
            ]
        );
        let plans: Vec<&ImitationPlan> = asked.iter().map(|(_, plan)| plan).collect();
        // The first plan, then two plans anew, each searched as it was probed.
        assert_eq!(*plans[0], first);
        assert_eq!(plans[0], plans[1]);
        for k in [2, 4] {
            assert_eq!(plans[k], plans[k + 1]);
            assert!(!plans[..k].contains(&plans[k]), "plan {k} again");
        }
        // Twice the beam searches the last of them; the longer plan is that one a quarter
        // longer, searched with the request's beam and four times the beam.
        assert_eq!(plans[6], plans[5]);
        let longer = kyrie.lengthen(plans[6]);
        assert_eq!(longer.measures, (plans[6].measures * 5).div_ceil(4));
        assert!(plans[7..].iter().all(|&plan| *plan == longer));
        assert!(matches!(
            result,
            Err(GenerateError::NoImitation { measure: 3, .. })
        ));

        // In WebAssembly, no four times the beam; at a given length, no longer plan.
        let wasm = Limits {
            probes: 2,
            widening: 2,
            ..NATIVE
        };
        let (asked, _) = driven(&mut planner("kyrie", 4, 2), wasm, fail_searches);
        let up_to_twice = [Probe, Beam(1), Probe, Beam(1), Probe, Beam(1), Beam(2)];
        assert_eq!(
            searches(&asked),
            [&up_to_twice[..], &[Probe, Beam(1)]].concat()
        );
        let given = Limits {
            longer: false,
            ..NATIVE
        };
        let (asked, _) = driven(&mut planner("kyrie", 4, 2), given, fail_searches);
        assert_eq!(searches(&asked), [&up_to_twice[..], &[Beam(4)]].concat());

        // Where the first point has no other plan left, the searches with the request's beam
        // after the first drop out.
        let mut spent = planner("kyrie", 4, 2);
        while spent.replan_near(0, Some(1)).is_some() {}
        let (asked, _) = driven(&mut spent, NATIVE, |_, search, _| {
            (search != Probe).then_some(1)
        });
        assert_eq!(
            searches(&asked),
            [Probe, Beam(1), Beam(2), Probe, Beam(1), Beam(4)]
        );

        // Found with the first search: nothing more.
        let (asked, result) = driven(&mut planner("kyrie", 4, 2), NATIVE, |_, _, _| None);
        assert_eq!(searches(&asked), [Probe, Beam(1)]);
        assert!(result.is_ok());
        // Found after one plan anew.
        let (asked, result) = driven(&mut planner("kyrie", 4, 2), NATIVE, |i, search, _| {
            (i == 1 && search == Beam(1)).then_some(5)
        });
        assert_eq!(searches(&asked), [Probe, Beam(1), Probe, Beam(1)]);
        assert!(result.is_ok());

        // Another error ends at once.
        let mut asked = 0;
        let result = drive(&mut planner("kyrie", 4, 2), NATIVE, |_, _| {
            asked += 1;
            Err(GenerateError::Invalid("no".into()))
        });
        assert_eq!(
            (asked, result.unwrap_err()),
            (1, GenerateError::Invalid("no".into()))
        );
    }

    /// The parts of a mass take a budget that is the same on every platform, six probes and
    /// twice the beam, so that a seed sets the same part natively and in WebAssembly; with it the
    /// driver never tries four times the beam. The platform's budget gives the limits the driver
    /// always had: natively six probes and four times the beam, in WebAssembly two and twice.
    #[test]
    fn portable_budget_is_platform_free() {
        use Search::*;
        let portable = Limits::of(Budget::PORTABLE, true);
        assert_eq!(
            portable,
            Limits {
                probes: 6,
                widening: 2,
                longer: true
            }
        );
        let native = !cfg!(target_arch = "wasm32");
        assert_eq!(
            Limits::of(Budget::PLATFORM, false),
            Limits {
                probes: if native { 6 } else { 2 },
                widening: if native { 4 } else { 2 },
                longer: false
            }
        );
        assert_eq!(Limits::of(Budget::PLATFORM, true), NATIVE);
        // A budget beyond the platform's widest beam is held to it.
        let wide = Budget {
            probes: 3,
            widening: 8,
        };
        assert_eq!(Limits::of(wide, true).widening, MAX_WIDENING);
        // Every search fails, every probe passes: twice the beam, but not four times.
        let (asked, _) = driven(&mut planner("kyrie", 4, 2), portable, |_, search, _| {
            (search != Probe).then_some(3)
        });
        let kinds = searches(&asked);
        assert!(
            kinds.contains(&Beam(2)) && !kinds.contains(&Beam(4)),
            "{kinds:?}"
        );
        assert_eq!(kinds.last(), Some(&Beam(1)), "the longer plan last");
    }

    /// The results of an imitative setting carry the subject of each point of the plan they were
    /// set by, its leader's first note on the subject's degree; a setting without imitation has
    /// none.
    #[test]
    fn subjects_reach_the_results() {
        let request = FreeRequest {
            beam: 20,
            ..request("kyrie", 3, 1)
        };
        let results = generate_free(&request, &Profile::fux()).unwrap();
        let generated = &results[0];
        let plan = generated.score.imitation.as_deref().expect("a plan");
        assert_eq!(generated.subjects.len(), plan.points.len());
        let mode = request.mode;
        for (point, subject) in plan.points.iter().zip(&generated.subjects) {
            let leader = point.entries[0].notes[0].0;
            let degree = (leader.diatonic() - mode.final_pitch(0).diatonic()).rem_euclid(7);
            assert_eq!(i32::from(subject.degree).rem_euclid(7), degree);
            assert!(subject.notes.len() >= point.entries[0].notes.len());
        }
        let plain = FreeRequest {
            imitation: None,
            ..request
        };
        assert!(
            generate_free(&plain, &Profile::fux()).unwrap()[0]
                .subjects
                .is_empty()
        );
    }

    /// A setting as a fake search finds it, at `cost`.
    fn setting_at(cost: u32) -> Vec<Generated> {
        vec![Generated {
            score: Score::new(dorian(), Vec::new()),
            cost,
            report: crate::rules::Report {
                violations: Vec::new(),
            },
            warnings: Vec::new(),
            subjects: Vec::new(),
        }]
    }

    /// Once a probe has found a setting, a failed search is followed by plans anew with the
    /// request's beam only, not by wider beams, nor by the longer plan unless the text lacked
    /// room in the last measure; where they fail too, the cheapest setting a probe found is the
    /// result.
    #[test]
    fn probes_settle_where_searches_fail() {
        use Search::*;
        // Every probe finds a setting, each cheaper than the one before; every search fails.
        let run = |measure: Option<usize>, rules: Vec<&'static str>| {
            let mut kyrie = planner("kyrie", 4, 2);
            let mut asked: Vec<Search> = Vec::new();
            let result = drive(&mut kyrie, NATIVE, |plan, search| {
                asked.push(search);
                match search {
                    Probe => Ok(setting_at(100 - asked.len() as u32)),
                    Beam(_) => Err(GenerateError::NoCounterpoint {
                        measure: measure.unwrap_or(plan.measures),
                        rules: rules.clone(),
                    }),
                }
            });
            (asked, result)
        };
        let (asked, result) = run(Some(3), vec!["FREE-PASSING"]);
        assert_eq!(
            asked,
            [Probe, Beam(1), Probe, Beam(1), Probe, Beam(1)],
            "no wider beam, no longer plan"
        );
        assert_eq!(result.unwrap()[0].cost, 100 - 5, "the cheapest probe's");
        // Where the text found no room at the end, the longer plan comes before the settings
        // of the probes.
        let (asked, result) = run(None, vec!["TXT-COMPLETE"]);
        assert_eq!(
            asked,
            [
                Probe,
                Beam(1),
                Probe,
                Beam(1),
                Probe,
                Beam(1),
                Probe,
                Beam(1)
            ]
        );
        assert_eq!(result.unwrap()[0].cost, 100 - 7);
    }

    /// Probes plan the failing point anew until one finds a setting or six have failed, two
    /// after a failed search; then the plan whose probe came furthest is searched, the earlier
    /// of two that came equally far.
    #[test]
    fn probes_keep_the_plan_that_came_furthest() {
        use Search::*;
        // Every probe fails, each in another measure of the first point or the second; every
        // search fails.
        let measures = [4, 6, 9, 2, 9, 7, 3, 8, 1, 5];
        let mut probes = 0;
        let (asked, _) = driven(&mut planner("kyrie", 4, 2), NATIVE, |_, search, _| {
            Some(match search {
                Probe => {
                    probes += 1;
                    measures[(probes - 1) % measures.len()]
                }
                Beam(_) => 2,
            })
        });
        let kinds = searches(&asked);
        assert_eq!(
            &kinds[..7],
            [Probe, Probe, Probe, Probe, Probe, Probe, Beam(1)]
        );
        assert_eq!(&kinds[7..10], [Probe, Probe, Beam(1)]);
        // Six plans, each a new one.
        for k in 1..6 {
            assert!(
                !asked[..k].iter().any(|(_, p)| *p == asked[k].1),
                "plan {k} again"
            );
        }
        // Each search after probes takes the plan of the furthest of them.
        let mut since: Vec<(usize, &ImitationPlan)> = Vec::new();
        let mut probe = 0;
        let mut checked = 0;
        for (search, plan) in &asked {
            if *search == Probe {
                since.push((measures[probe % measures.len()], plan));
                probe += 1;
            } else if !since.is_empty() {
                let furthest = since.iter().map(|&(m, _)| m).max().unwrap();
                let first = since.iter().find(|&&(m, _)| m == furthest).unwrap();
                let failed: Vec<usize> = since.iter().map(|&(m, _)| m).collect();
                assert!(
                    plan == first.1,
                    "the search after probes failing in {failed:?}"
                );
                since.clear();
                checked += 1;
            }
        }
        assert!(checked >= 3, "{kinds:?}");
        assert_eq!(asked[6].1, asked[2].1);

        // The planner goes back to the plan that came furthest: where its search fails in the
        // second point, the plan anew keeps that plan's first point, although the probes after
        // it planned the first point anew.
        let measure = |tick: u32| (tick / 8) as usize + 1;
        let mut probes = 0;
        let (asked, _) = driven(&mut planner("agnus", 4, 1), NATIVE, |_, search, plan| {
            let second = measure(plan.points[1].entries[0].start);
            Some(match search {
                Probe => {
                    probes += 1;
                    if probes == 1 { second + 1 } else { 1 }
                }
                Beam(_) => second,
            })
        });
        assert_eq!(
            searches(&asked)[..8],
            [Probe, Probe, Probe, Probe, Probe, Probe, Beam(1), Probe]
        );
        let (furthest, last, next) = (&asked[0].1, &asked[5].1, &asked[7].1);
        assert_eq!(asked[6].1, *furthest, "the search takes the furthest plan");
        assert_ne!(last.points[0], furthest.points[0]);
        assert_eq!(
            next.points[0], furthest.points[0],
            "the first point as searched"
        );
        assert_ne!(next.points[1], furthest.points[1]);
    }

    /// The warnings of the plan reach the results: a point whose chant phrase gave no usable
    /// head took a subject from its words, from a search and from the settings of probes.
    #[test]
    fn chant_fallbacks_reach_the_results() {
        use crate::imitation::SubjectSource;
        use crate::imitation::chant::{ChantFallback, Kyriale};
        // The last invocation of Kyrie IX for Kyrie I: the tail has no phrase left.
        let imitation = Imitation {
            subject: SubjectSource::Chant {
                chant: Kyriale::KyrieIX.chant(),
                phrases: Some(vec![8]),
            },
            restate: crate::imitation::Toggle::Off,
            ..Imitation::default()
        };
        let request = FreeRequest {
            mode: "dorian-transposed".parse().unwrap(),
            imitation: Some(imitation.clone()),
            ..request("kyrie", 4, 1)
        };
        let warned = |planner: &mut Planner, fails: bool| {
            drive(planner, NATIVE, |_, search| match search {
                Search::Beam(_) if fails => Err(GenerateError::NoCounterpoint {
                    measure: 3,
                    rules: vec!["FREE-PASSING"],
                }),
                _ => Ok(setting_at(1)),
            })
            .unwrap()
        };
        let expected = [Warning::ChantFallback(ChantFallback { point: 2 })];
        let mut planner = plan_imitation(&request, &imitation, &Profile::fux()).unwrap();
        assert_eq!(planner.fallbacks(), [ChantFallback { point: 2 }]);
        assert_eq!(warned(&mut planner, false)[0].warnings, expected);
        let mut planner = plan_imitation(&request, &imitation, &Profile::fux()).unwrap();
        let found = warned(&mut planner, true);
        assert_eq!(found[0].warnings, expected, "from a probe");
        assert_eq!(
            expected[0].message(crate::lang::Lang::De),
            "Punkt 2: kein brauchbarer Kopf aus dem Choral; Soggetto aus dem Text"
        );
        // A setting without imitation has none.
        let plain = FreeRequest {
            imitation: None,
            beam: 10,
            ..request.clone()
        };
        assert!(
            generate_free(&plain, &Profile::fux()).unwrap()[0]
                .warnings
                .is_empty()
        );
    }

    /// A length the request gives holds for every plan the driver probes and searches, however
    /// often it plans a point anew; where no plan of that length remains, the request fails with
    /// the length of the shortest plan given up, or names the point that failed.
    #[test]
    fn given_lengths_hold_through_replanning() {
        let natural = planner("kyrie", 4, 2).plan().measures;
        let mut request = request("kyrie", 4, 2);
        request.measures = natural;
        request.text.as_mut().unwrap().auto_length = false;
        let mut planner = plan_imitation(&request, &Imitation::default(), &Profile::fux()).unwrap();
        let given = Limits {
            longer: false,
            ..NATIVE
        };
        let mut i = 0;
        let (asked, result) = driven(&mut planner, given, |_, _, plan| {
            i += 1;
            Some([1, plan.measures - 2, 9, 3][i % 4])
        });
        assert!(asked.len() >= 8, "{} searches", asked.len());
        for (search, plan) in &asked {
            assert_eq!(plan.measures, natural, "{search:?}");
        }
        // Every plan given up for the length was longer than it.
        assert!(planner.too_long().is_none_or(|measures| measures > natural));
        match result {
            Err(GenerateError::ImitationTooLong { measures }) => assert!(measures > natural),
            Err(GenerateError::NoImitation { .. }) => {}
            other => panic!("{other:?}"),
        }
    }

    /// A failure is put down to the point of the entry it is nearest: the probe that fails in
    /// the measure of the second point's leader plans that point anew and keeps the first; one
    /// that fails at the first entry plans the first anew. Where every attempt fails, the error
    /// names the point of the last failure with its words, the measure and the rules.
    #[test]
    fn failures_are_attributed_to_points() {
        use Search::*;
        let mut agnus = planner("agnus", 4, 1);
        let first = agnus.plan();
        assert!(first.points.len() >= 2);
        let measure = |tick: u32| (tick / 8) as usize + 1;
        let second = measure(first.points[1].entries[0].start);
        // The first probe fails at the second point's leader, the second one passes.
        let (asked, result) = driven(&mut agnus, NATIVE, |i, _, _| (i == 0).then_some(second));
        assert!(result.is_ok());
        assert_eq!(searches(&asked), [Probe, Probe, Beam(1)]);
        let (before, after) = (&asked[0].1, &asked[1].1);
        assert_eq!(before.points[0], after.points[0]);
        assert_ne!(before.points[1], after.points[1]);
        // At the first entry: the first point anew.
        let mut agnus = planner("agnus", 4, 1);
        let (asked, _) = driven(&mut agnus, NATIVE, |i, _, _| (i == 0).then_some(1));
        assert_ne!(asked[0].1.points[0], asked[1].1.points[0]);
        // A search that fails at the second point's leader: that point anew before the next
        // search.
        let mut agnus = planner("agnus", 4, 1);
        let (asked, result) = driven(&mut agnus, NATIVE, |i, _, _| (i == 1).then_some(second));
        assert!(result.is_ok());
        assert_eq!(searches(&asked), [Probe, Beam(1), Probe, Beam(1)]);
        let (before, after) = (&asked[1].1, &asked[2].1);
        assert_eq!(before.points[0], after.points[0]);
        assert_ne!(before.points[1], after.points[1]);

        // Everything fails in the second point's leader's measure of each plan.
        let mut agnus = planner("agnus", 4, 1);
        let (asked, result) = driven(&mut agnus, NATIVE, |_, _, plan| {
            Some(measure(plan.points[1].entries[0].start))
        });
        let last = &asked.last().unwrap().1;
        let failed = measure(last.points[1].entries[0].start);
        assert_eq!(last.owner(failed), 1);
        assert_eq!(
            result.unwrap_err(),
            GenerateError::NoImitation {
                point: 2,
                words: agnus.words(1),
                measure: failed,
                rules: vec!["FREE-PASSING"],
            }
        );
        assert!(agnus.words(1).starts_with("qui"), "{}", agnus.words(1));
    }

    /// Imitation needs a text: a request without one fails before any plan, with a German and an
    /// English message. Too few voices are named first, as without imitation.
    #[test]
    fn imitation_needs_a_text() {
        let mut untexted = request("kyrie", 3, 1);
        untexted.text = None;
        let error = generate_free(&untexted, &Profile::fux()).unwrap_err();
        assert_eq!(error, GenerateError::ImitationNeedsText);
        assert_eq!(
            error.message(crate::lang::Lang::De),
            "Imitation braucht einen Text, denn ihre Punkte folgen den Worten."
        );
        assert_eq!(
            error.message(crate::lang::Lang::En),
            "Imitation needs a text: its points follow the words."
        );
        // Too few voices are named first, as without imitation.
        let mut alone = request("kyrie", 3, 1);
        alone.voices.truncate(1);
        assert_eq!(
            generate_free(&alone, &Profile::fux()).unwrap_err(),
            GenerateError::TooFewVoices
        );
    }

    /// Given cadences are refused: imitation places them at the ends of its points. Cadences in
    /// a request whose text plans them anyway are left aside, as without imitation.
    #[test]
    fn given_cadences_are_refused() {
        let mut given = request("kyrie", 3, 1);
        given.cadences = crate::generate::cadences_at(dorian(), &[6]);
        given.text.as_mut().unwrap().auto_cadences = false;
        let error = generate_free(&given, &Profile::fux()).unwrap_err();
        assert_eq!(error, GenerateError::ImitationCadences);
        assert_eq!(
            error.message(crate::lang::Lang::En),
            "Imitation places the internal cadences itself, at the ends of its points; they \
             cannot be given."
        );
        // Without its cadences the same request goes on to the planner, which plans them.
        given.text.as_mut().unwrap().auto_cadences = true;
        let planned = plan_imitation(&given, &Imitation::default(), &Profile::fux()).unwrap();
        assert_ne!(planned.plan().cadences, given.cadences);
    }

    /// Runs the driver on imitative requests and prints, per request, the planner's time, each
    /// probe and search with its plan's length, its outcome and time, and the time of the texted
    /// search without imitation of the same request; at the end the totals. `TEXTS`, `MODES`,
    /// `VOICES` and `SEEDS` (comma-separated) choose the requests, Kyrie I in the Dorian mode
    /// with three and four voices and seeds 1 and 2 by default; `BEAM`, `PROBE_BEAM` and
    /// `PROBES` change the beam (200), the probe's beam and the probes of the first plan, and
    /// `PLAIN=0` leaves out the search without imitation. Run with `cargo test --release -p
    /// fux-core driver_statistics -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn driver_statistics() {
        use std::time::Instant;
        let list = |name: &str, default: &str| -> Vec<String> {
            std::env::var(name)
                .unwrap_or_else(|_| default.to_string())
                .split(',')
                .map(str::to_string)
                .collect()
        };
        let number = |name: &str, default: usize| -> usize {
            std::env::var(name).map_or(default, |v| v.parse().unwrap())
        };
        let (beam, probe_beam) = (number("BEAM", 200), number("PROBE_BEAM", PROBE_BEAM));
        let limits = Limits {
            probes: number("PROBES", Budget::PLATFORM.probes),
            ..NATIVE
        };
        let plain_too = number("PLAIN", 1) != 0;
        let profile = Profile::fux();
        // Requests and those found, seconds with and without imitation, in probes and in
        // searches; probes and searches.
        let (mut count, mut found) = (0, 0);
        let (mut imitative, mut plain, mut probing, mut searching) = (0.0, 0.0, 0.0, 0.0);
        let (mut probes, mut searches) = (0, 0);
        for spec in list("TEXTS", "kyrie") {
            for mode in list("MODES", "dorian") {
                for voices in list("VOICES", "3,4") {
                    for seed in list("SEEDS", "1,2") {
                        let voices: usize = voices.parse().unwrap();
                        let request = FreeRequest {
                            mode: mode.parse().unwrap(),
                            beam,
                            ..request(&spec, voices, seed.parse().unwrap())
                        };
                        let label = format!("{spec} {mode} {voices} voices seed {seed}");
                        let sung = request.text.clone().unwrap();
                        let clock = Instant::now();
                        let mut planner =
                            plan_imitation(&request, &Imitation::default(), &profile).unwrap();
                        let planning = clock.elapsed().as_secs_f64();
                        let mut log = Vec::new();
                        let result = drive(&mut planner, limits, |plan, search| {
                            let clock = Instant::now();
                            let text = SungText {
                                text: Arc::clone(&sung.text),
                                rate: plan.rate,
                            };
                            let beam = match search {
                                Search::Probe => probe_beam,
                                Search::Beam(width) => request.beam * width,
                            };
                            let result = setting(&request, plan, Some(text), beam, 1, &profile);
                            let seconds = clock.elapsed().as_secs_f64();
                            if search == Search::Probe {
                                (probes, probing) = (probes + 1, probing + seconds);
                            } else {
                                (searches, searching) = (searches + 1, searching + seconds);
                            }
                            let outcome = match &result {
                                Ok(results) => format!("cost {}", results[0].cost),
                                Err(GenerateError::NoCounterpoint { measure, rules }) => format!(
                                    "fails in {measure} ({}), point {}",
                                    rules.join(" "),
                                    plan.owner(*measure) + 1
                                ),
                                Err(e) => format!("{e}"),
                            };
                            log.push(format!(
                                "    {search:?} {} measures: {outcome}, {seconds:.2} s",
                                plan.measures
                            ));
                            result
                        });
                        let seconds = clock.elapsed().as_secs_f64();
                        let (plain_seconds, other) = if plain_too {
                            let without = FreeRequest {
                                imitation: None,
                                ..request.clone()
                            };
                            let clock = Instant::now();
                            let other = match generate_free(&without, &profile) {
                                Ok(r) => format!("{} measures", r[0].score.measure_count()),
                                Err(e) => format!("{e}"),
                            };
                            (clock.elapsed().as_secs_f64(), other)
                        } else {
                            (0.0, String::new())
                        };
                        count += 1;
                        imitative += seconds;
                        plain += plain_seconds;
                        let outcome = match &result {
                            Ok(results) => {
                                found += 1;
                                let score = &results[0].score;
                                let measures = score.measure_count();
                                format!("found, {measures} measures, cost {}", results[0].cost)
                            }
                            Err(e) => format!("{e}"),
                        };
                        println!(
                            "{label}: {outcome}; {seconds:.2} s (planner {planning:.2} s), \
                             without imitation {plain_seconds:.2} s ({other})"
                        );
                        for line in log {
                            println!("{line}");
                        }
                    }
                }
            }
        }
        println!(
            "found {found} of {count}; {imitative:.1} s imitative ({probes} probes {probing:.1} s, \
             {searches} searches {searching:.1} s), {plain:.1} s without imitation ({:.2} times)",
            imitative / plain
        );
    }
}
