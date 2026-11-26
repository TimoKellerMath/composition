//! Free counterpoint without a cantus firmus (phase 11): every voice free, in Palestrina's
//! rhythm, with the cadences written as fixed formulas and the rest found by the search.
//!
//! A cadence arrives on its target at the start of its goal measure. The top voice sings the
//! cantizans: it holds the target over the penultimate downbeat as a suspension and resolves it
//! to the leading tone (in a Phrygian clausula to the whole step below) before it returns to the
//! target. In two voices the lower one sings the
//! tenorizans, from the step above. From three voices on the bass comes from the fifth or the
//! fourth below (basizans), from four voices on with the tenorizans in the voice above it; a
//! Phrygian clausula has no basizans, so there the bass sings the tenorizans. The other voices
//! are free.
//!
//! With a text (phase 12) every voice sings it. The text plans the setting
//! ([`crate::text::plan`]): its length, its pace and internal cadences at the ends of sense
//! units. The search carries the underlays of each voice along and lays the cheapest one under
//! its notes ([`crate::text::underlay`]).
//!
//! A setting may be a part of a mass section (phase 14, [`PartContext`]): it ends on its own
//! degree, the final or the fifth or the fourth where the mode has a clausula; its voices may
//! rest a minim or a measure before they begin, after the part before; and it may open with a
//! chord that every voice strikes together.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::counterpoint::{
    CounterpointRequest, Fixed, FixedRole, MAX_WIDENING, NewVoice, SungText, generate_counterpoint,
};
use super::imitation::generate_imitative;
use super::{GenerateError, Generated};
use crate::clef::VoiceType;
use crate::imitation::Imitation;
use crate::interval::Interval;
use crate::mode::{Clausula, Mode, ModeSpec};
use crate::pitch::{Pitch, Step};
use crate::rules::{Level, Profile};
use crate::score::{Cadence, Note, Score, Species};
#[cfg(test)]
use crate::text::plan::plan;
use crate::text::plan::{CadenceChain, Plan, plan_part};
use crate::text::underlay::Choice as Underlaid;
use crate::text::{Text, TextKind};
use crate::time::{Duration, Meter};

/// What to generate: a free setting for the given voices.
#[derive(Clone, Debug)]
pub struct FreeRequest {
    pub mode: ModeSpec,
    /// The voices; the setting lists them from high to low.
    pub voices: Vec<VoiceType>,
    /// The length, unless a text plans it.
    pub measures: usize,
    /// Internal cadences: the measure (from 0) their target arrives in, and the target; unless a
    /// text plans them.
    pub cadences: Vec<Cadence>,
    pub seed: u64,
    /// Number of distinct solutions to return, best first.
    pub count: usize,
    /// Partial solutions kept per step of the search.
    pub beam: usize,
    /// Random noise added to the costs when ranking, for variety.
    pub noise: f64,
    /// The text every voice sings; `None` for untexted voices.
    pub text: Option<FreeText>,
    /// Imitation (phase 13): the voices take up the words point by point, each with the head of
    /// a subject, planned before the search. It needs a text whose cadences it plans itself.
    /// `None` for a free setting without planned imitation.
    pub imitation: Option<Imitation>,
    /// Where the setting stands in a larger work (phase 14): its ending, its start, its opening
    /// chord and the chain of its cadences. The default is a setting of its own.
    pub part: PartContext,
    /// How far the drivers go: probes of an imitation's plan and the widest beam.
    pub budget: Budget,
    /// Cancels the search and tells how far it has come; `None` for a search that runs to its
    /// end.
    pub control: Option<Arc<SearchControl>>,
}

/// How a free setting stands in a larger work, such as a part of a mass section (phase 14). The
/// default is a setting of its own: it ends on the final, its voices may enter at once, and its
/// internal cadences take the regular degrees of the mode in turn.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartContext {
    /// The target of the final cadence; `None` for the final of the mode. Besides the final, the
    /// fifth and the fourth where the mode has a clausula on them ([`part_endings`]).
    pub ending: Option<Step>,
    /// No voice sounds before this tick: 0, 4 (a minim's breath after the held chord of the part
    /// before) or 8 (a whole measure's rest after a break). With imitation the leader of the
    /// first point enters here.
    pub start: u32,
    /// The opening chord, one pitch per voice from the top: every voice strikes it at `start` as
    /// a semibreve on the first syllable. Only without imitation.
    pub chord: Option<Vec<Pitch>>,
    /// How the internal cadences choose their degrees.
    pub chain: CadenceChain,
}

impl PartContext {
    /// The target of the final cadence in `mode`.
    pub fn ending_in(&self, mode: ModeSpec) -> Step {
        self.ending.unwrap_or_else(|| mode.final_step())
    }

    /// The measure (from 0) in which the first internal cadence of the part may arrive at the
    /// earliest: two measures after the start, the measure before the penultimate one preparing
    /// the cantizans' suspension (the third measure where the voices begin at once). After an
    /// opening chord the cantizans prepares its suspension with a note of its own, a minim at
    /// least after the chord's semibreve, so that the first cadence comes a measure later where
    /// the chord falls on a downbeat.
    pub(crate) fn first_goal(&self) -> usize {
        let per = Meter::ALLA_BREVE.ticks_per_measure();
        // Where the penultimate measure of the first cadence begins at the earliest.
        let penultimate = match self.chord {
            None => self.start + per,
            Some(_) => self.start + per + per / 2,
        };
        (penultimate + per).div_ceil(per) as usize
    }

    /// The fixed notes of the part's opening for `voices` voices in `mode`: where it has an
    /// opening chord, each voice's pitch as a semibreve at the start, an [`FixedRole::Entry`]
    /// after silence from the beginning with the first syllable pinned, split and tied at the
    /// barline; otherwise, after a start, a rest up to it in every voice. Nothing for a part that
    /// begins at once without a chord.
    pub(crate) fn opening(&self, mode: ModeSpec, voices: usize, meter: Meter) -> Vec<Fixed> {
        let per = meter.ticks_per_measure();
        let mut out = Vec::new();
        let Some(chord) = &self.chord else {
            if self.start > 0 {
                for voice in 0..voices {
                    out.push(Fixed {
                        voice,
                        start: 0,
                        note: Note::rest(Duration::from_ticks(self.start)),
                        tied_in: false,
                        any_octave: false,
                        role: FixedRole::Formula,
                        sung: None,
                    });
                }
            }
            return out;
        };
        for (voice, &pitch) in chord.iter().enumerate().take(voices) {
            let (attack, end) = (self.start, self.start + per);
            let mut tick = attack;
            while tick < end {
                let until = end.min((tick / per + 1) * per);
                out.push(Fixed {
                    voice,
                    start: tick,
                    note: Note::new(pitch, Duration::from_ticks(until - tick))
                        .with_ficta(!mode.is_scale_pitch(pitch)),
                    tied_in: tick > attack,
                    any_octave: false,
                    role: if tick == attack {
                        FixedRole::Entry { rest: 0 }
                    } else {
                        FixedRole::Subject
                    },
                    sung: (tick == attack).then_some(Underlaid::Start(0)),
                });
                tick = until;
            }
        }
        out
    }
}

/// How far the drivers of a free setting go before they give up: the probes of an imitation's
/// first plan ([`super::imitation`]) and the widest beam as a multiple of the request's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    pub probes: usize,
    pub widening: usize,
}

impl Budget {
    /// As far as the platform allows: natively six probes and four times the beam; in
    /// WebAssembly, where a search takes about five to ten times as long and four times the beam
    /// of five or six voices takes gigabytes, two probes and twice the beam.
    pub const PLATFORM: Budget = Budget {
        probes: if cfg!(target_arch = "wasm32") { 2 } else { 6 },
        widening: MAX_WIDENING,
    };

    /// The same on every platform, so that a seed gives the same setting natively and in
    /// WebAssembly: six probes and twice the beam (the parts of a mass, decision D13).
    pub const PORTABLE: Budget = Budget {
        probes: 6,
        widening: 2,
    };
}

impl Default for Budget {
    fn default() -> Budget {
        Budget::PLATFORM
    }
}

/// Cancels a running search and tells how far it has come, from another thread: the search reads
/// it once per step and the drivers before each probe and each search, so a cancel takes effect
/// within a step.
#[derive(Debug, Default)]
pub struct SearchControl {
    cancel: AtomicBool,
    measure: AtomicU32,
}

impl SearchControl {
    pub fn new() -> SearchControl {
        SearchControl::default()
    }

    /// Asks the search to stop: it returns [`GenerateError::Cancelled`].
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// The measure (from 1) the search has reached; 0 before it begins. A new search, a probe or
    /// a wider beam, begins again from the first measure.
    pub fn reached(&self) -> u32 {
        self.measure.load(Ordering::Relaxed)
    }

    /// The search has reached `measure` (from 1).
    pub(crate) fn reach(&self, measure: u32) {
        self.measure.store(measure, Ordering::Relaxed);
    }
}

/// Whether `control`, if there is one, asks the search to stop.
pub(crate) fn cancelled(control: Option<&SearchControl>) -> bool {
    control.is_some_and(SearchControl::is_cancelled)
}

impl FreeRequest {
    pub fn new(mode: ModeSpec, voices: Vec<VoiceType>, measures: usize) -> FreeRequest {
        FreeRequest {
            mode,
            voices,
            measures,
            cadences: Vec::new(),
            seed: 1,
            count: 1,
            beam: 200,
            noise: 2.0,
            text: None,
            imitation: None,
            part: PartContext::default(),
            budget: Budget::PLATFORM,
            control: None,
        }
    }
}

/// The text of a free setting, and what it plans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreeText {
    pub text: Arc<Text>,
    /// The length comes from the text ([`crate::text::plan::plan`]) instead of
    /// [`FreeRequest::measures`].
    pub auto_length: bool,
    /// The internal cadences fall at the planned ends of sense units instead of
    /// [`FreeRequest::cadences`].
    pub auto_cadences: bool,
    /// The printed syllables keep the acute of the liturgical books.
    pub accents: bool,
}

impl FreeText {
    /// A text with its length and cadences planned from it, printed with its accents.
    pub fn new(text: Arc<Text>) -> FreeText {
        FreeText {
            text,
            auto_length: true,
            auto_cadences: true,
            accents: true,
        }
    }
}

/// The shortest free setting: an entry, the measure before the penultimate (which prepares the
/// suspension), the penultimate and the last.
pub const MIN_MEASURES: usize = 4;

/// How many measures an internal cadence needs before the next one: its own penultimate and goal
/// measures, and the preparation of the next suspension.
pub(crate) const CADENCE_SPACING: usize = 3;

/// The degrees besides the final on which a mode makes its internal cadences, in the order a
/// setting uses them: the fifth, the third and the fourth; in the Phrygian mode, whose fifth has
/// no cadence, the fourth, the sixth and the third. A degree with a diminished fifth above (B,
/// transposed E) drops out.
pub fn regular_cadences(mode: ModeSpec) -> Vec<Step> {
    let final_step = mode.final_step();
    let up = |steps: i32| Step::from_index(final_step.index() + steps);
    let degrees = match mode.mode {
        Mode::Phrygian => [up(3), up(5), up(2)],
        _ => [up(4), up(2), up(3)],
    };
    degrees
        .into_iter()
        .filter(|&step| mode.clausula(step).is_some())
        .collect()
}

/// The degrees on which a part of a larger work may end (MASS-ENDING): the final, the fifth and
/// the fourth, the last two where the mode has a clausula on them (Dorian D A G, Phrygian E A,
/// Lydian F C, Mixolydian G D C, Aeolian A E D, Ionian C G F; transposed and plagal modes
/// alike). TEX-FINAL-CHORD (`free_degree`) leaves the final, the fourth and the fifth free for
/// the last chord; of these a part takes only the degrees with a clausula, so the Phrygian
/// fifth and the Lydian fourth are left out. Pontio forbids ending a part on the sixth of the
/// Phrygian mode.
pub fn part_endings(mode: ModeSpec) -> Vec<Step> {
    let final_step = mode.final_step();
    let up = |steps: i32| Step::from_index(final_step.index() + steps);
    [up(0), up(4), up(3)]
        .into_iter()
        .filter(|&step| mode.clausula(step).is_some())
        .collect()
}

/// The usual internal cadences of a setting of `measures` measures: one per six measures or so,
/// evenly spaced, none in the last four, on the regular degrees of the mode in turn.
pub fn default_cadences(mode: ModeSpec, measures: usize) -> Vec<Cadence> {
    let count = measures.saturating_sub(4) / 6;
    let goals: Vec<usize> = (1..=count)
        .map(|i| ((i * (measures - 1)) as f64 / (count + 1) as f64).round() as usize)
        .collect();
    cadences_at(mode, &goals)
}

/// Internal cadences arriving in the given measures (from 0), on the regular degrees of the mode
/// in turn.
pub fn cadences_at(mode: ModeSpec, goals: &[usize]) -> Vec<Cadence> {
    let degrees = regular_cadences(mode);
    goals
        .iter()
        .zip(degrees.iter().cycle())
        .map(|(&measure, &target)| Cadence { measure, target })
        .collect()
}

/// What a voice sings in a clausula.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    /// Holds the target over the penultimate downbeat and resolves it to the step below.
    Cantizans,
    /// Comes from the step above.
    Tenorizans,
    /// Comes from the fifth above or the fourth below.
    Basizans,
}

/// Which voices sing the clausula, by index from the top (see the module documentation).
fn clausula_parts(clausula: Clausula, voices: usize) -> Vec<(usize, Part)> {
    match (clausula, voices) {
        (_, 0 | 1) => Vec::new(),
        (_, 2) => vec![(0, Part::Cantizans), (1, Part::Tenorizans)],
        (Clausula::Phrygian, n) => vec![(0, Part::Cantizans), (n - 1, Part::Tenorizans)],
        (Clausula::Leading, 3) => vec![(0, Part::Cantizans), (2, Part::Basizans)],
        (Clausula::Leading, n) => vec![
            (0, Part::Cantizans),
            (n - 2, Part::Tenorizans),
            (n - 1, Part::Basizans),
        ],
    }
}

/// The fixed notes of a cadence whose target arrives at the start of measure `goal` (from 0),
/// for `voices` voices from the top down. The notes are fixed in pitch class only: the search
/// chooses their octaves, where the voices are. Empty where the target has no clausula (B,
/// transposed E), with fewer than two voices or before the third measure.
pub fn cadence_formula(
    mode: ModeSpec,
    target: Step,
    goal: usize,
    voices: usize,
    meter: Meter,
) -> Vec<Fixed> {
    let Some(clausula) = mode.clausula(target) else {
        return Vec::new();
    };
    if voices < 2 || goal < 2 {
        return Vec::new();
    }
    let per = meter.ticks_per_measure();
    let goal_tick = goal as u32 * per;
    let penultimate = goal_tick - per;
    let half = Duration::from_ticks(per / 2);
    let whole = Duration::from_ticks(per);
    let t = Pitch::new(target, mode.scale_alter(target), 4);
    let mut out = Vec::new();
    let mut push = |voice: usize, start: u32, pitch: Pitch, duration: Duration, tied_in: bool| {
        out.push(Fixed {
            voice,
            start,
            note: Note::new(pitch, duration).with_ficta(!mode.is_scale_pitch(pitch)),
            tied_in,
            any_octave: true,
            role: FixedRole::Formula,
            sung: None,
        });
    };
    for (voice, part) in clausula_parts(clausula, voices) {
        match (part, clausula) {
            // The preparation before the suspension is left to the search: any note on the
            // target tied over.
            (Part::Cantizans, _) => {
                let below = match clausula {
                    Clausula::Leading => -Interval::MINOR_SECOND,
                    Clausula::Phrygian => -Interval::MAJOR_SECOND,
                };
                push(voice, penultimate, t, half, true);
                push(
                    voice,
                    penultimate + per / 2,
                    t.transpose(below),
                    half,
                    false,
                );
            }
            (Part::Tenorizans, Clausula::Leading) => {
                push(
                    voice,
                    penultimate,
                    t.transpose(Interval::MAJOR_SECOND),
                    whole,
                    false,
                );
            }
            (Part::Tenorizans, Clausula::Phrygian) => {
                push(
                    voice,
                    penultimate,
                    t.transpose(Interval::MINOR_SECOND),
                    whole,
                    false,
                );
            }
            (Part::Basizans, _) => {
                push(
                    voice,
                    penultimate,
                    t.transpose(Interval::PERFECT_FIFTH),
                    whole,
                    false,
                );
            }
        }
        push(voice, goal_tick, t, whole, false);
    }
    out
}

/// Checks that the request has two voices or more.
fn enough_voices(request: &FreeRequest) -> Result<(), GenerateError> {
    if request.voices.len() < 2 {
        return Err(GenerateError::TooFewVoices);
    }
    Ok(())
}

/// The ticks at which a part may begin: at once, after a minim's breath (after a seam) or after a
/// whole measure's rest (after a break).
const STARTS: [u32; 3] = [0, 4, 8];

/// Checks the part of the request (phase 14): an ending that the mode allows ([`part_endings`]),
/// a start of 0, 4 or 8 ticks, and an opening chord only without imitation, whose voices enter
/// one by one, with a pitch for each voice.
fn validate_part(request: &FreeRequest) -> Result<(), GenerateError> {
    let part = &request.part;
    let mode = request.mode;
    if let Some(target) = part.ending
        && !part_endings(mode).contains(&target)
    {
        return Err(GenerateError::Ending { mode, target });
    }
    if !STARTS.contains(&part.start) {
        return Err(GenerateError::Start { start: part.start });
    }
    if let Some(chord) = &part.chord {
        if request.imitation.is_some() {
            return Err(GenerateError::ChordWithImitation);
        }
        if chord.len() != request.voices.len() {
            return Err(GenerateError::ChordVoices {
                pitches: chord.len(),
                voices: request.voices.len(),
            });
        }
    }
    Ok(())
}

/// Checks the request: enough voices and measures, its part ([`validate_part`]), and internal
/// cadences that leave room for their formulas, the first after the part's start
/// ([`PartContext::first_goal`]). A cadence too close to the final one is named, not the final
/// cadence.
fn validate(request: &FreeRequest) -> Result<(), GenerateError> {
    enough_voices(request)?;
    validate_part(request)?;
    if request.measures < MIN_MEASURES {
        return Err(GenerateError::TooShort {
            least: MIN_MEASURES,
        });
    }
    let mut goals: Vec<usize> = request.cadences.iter().map(|c| c.measure).collect();
    goals.sort_unstable();
    let last = goals.len();
    goals.push(request.measures - 1);
    let first = request.part.first_goal();
    let mut before = 0;
    for (i, &goal) in goals.iter().enumerate() {
        let least = if i == 0 {
            first
        } else {
            before + CADENCE_SPACING
        };
        if goal < least {
            let measure = if i == last { before } else { goal };
            return Err(GenerateError::CadenceTooClose {
                measure: measure + 1,
            });
        }
        before = goal;
    }
    for cadence in &request.cadences {
        if request.mode.clausula(cadence.target).is_none() {
            return Err(GenerateError::NoClausula {
                target: cadence.target,
            });
        }
    }
    Ok(())
}

/// Generates free counterpoint: the cadences as formulas, the rest by the search. Every result
/// breaks no prohibition of the profile.
///
/// With a text, the text plans the length and the cadences where the request leaves them to it,
/// and every voice of every result sings the whole text, with its syllables written out. If the
/// search finds nothing with the beam and twice the beam, a planned length gets a quarter more
/// measures once, before the widest beam.
///
/// With imitation, its plan sets the length, the cadences and the entries of the points before
/// the search, and every result sings each entry as planned (the driver in
/// `generate::imitation`). It needs a text ([`GenerateError::ImitationNeedsText`]) whose
/// cadences it plans itself ([`GenerateError::ImitationCadences`]).
///
/// As a part of a larger work ([`FreeRequest::part`]), the final cadence arrives on the part's
/// ending, with the bass on it; no voice sounds before the part's start; an opening chord sounds
/// in every voice at the start, on the first syllable; and the text plans the internal cadences
/// by the part's chain. The request's budget bounds the drivers, and its control can cancel them
/// ([`GenerateError::Cancelled`]).
pub fn generate_free(
    request: &FreeRequest,
    profile: &Profile,
) -> Result<Vec<Generated>, GenerateError> {
    if let Some(imitation) = &request.imitation {
        enough_voices(request)?;
        validate_part(request)?;
        let sung = request
            .text
            .as_ref()
            .ok_or(GenerateError::ImitationNeedsText)?;
        if !sung.auto_cadences {
            return Err(GenerateError::ImitationCadences);
        }
        return generate_imitative(request, imitation, sung, profile);
    }
    let Some(sung) = &request.text else {
        validate(request)?;
        return setting(request, None, profile, Beams::All);
    };
    enough_voices(request)?;
    validate_part(request)?;
    let kind = match sung.text.kind {
        TextKind::Short => "short",
        TextKind::Long => "long",
    };
    let restatements = profile.settings("TXT-REPEAT").level(kind) != Level::Forbidden;
    let plan_with = |measures: Option<usize>| {
        plan_part(
            &sung.text,
            request.mode,
            request.voices.len(),
            measures,
            (!sung.auto_cadences).then(|| request.cadences.clone()),
            restatements,
            &request.part,
        )
    };
    let planned = plan_with((!sung.auto_length).then_some(request.measures))?;
    let replan = |measures| plan_with(Some(measures));
    let mut results = with_plans(planned, sung.auto_length, replan, |planned, beams| {
        let planned_request = FreeRequest {
            measures: planned.measures,
            cadences: planned.cadences.clone(),
            ..request.clone()
        };
        validate(&planned_request)?;
        let text = SungText {
            text: sung.text.clone(),
            rate: planned.rate,
        };
        setting(&planned_request, Some(text), profile, beams)
    })?;
    for result in &mut results {
        result.score.fill_syllables(sung.accents);
    }
    Ok(results)
}

/// The beams a search for free counterpoint tries one after the other, in multiples of the
/// request's beam.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Beams {
    /// The beam, then twice and four times as wide, as far as the request's budget and
    /// [`MAX_WIDENING`](super::MAX_WIDENING) allow.
    All,
    /// The beam, then twice as wide.
    UpToTwice,
}

/// The settings of a texted request by `search`, with `planned` and, if its length is automatic
/// and neither the beam nor twice the beam finds anything, with a quarter more measures, its
/// cadences planned anew by `replan`, and every beam. The longer plan comes before the widest
/// beam, which costs the most: where the text did not fit (TXT-COMPLETE) that beam only runs
/// into the same lack of room. Where other rules cut the search off, the widest beam may still
/// find a setting of the planned length, which this order passes over for a longer one at about
/// the same effort (docs/plan.md, open points of phase 12).
fn with_plans(
    planned: Plan,
    auto_length: bool,
    replan: impl Fn(usize) -> Result<Plan, GenerateError>,
    mut search: impl FnMut(&Plan, Beams) -> Result<Vec<Generated>, GenerateError>,
) -> Result<Vec<Generated>, GenerateError> {
    if !auto_length {
        return search(&planned, Beams::All);
    }
    match search(&planned, Beams::UpToTwice) {
        Err(GenerateError::NoCounterpoint { .. }) => {}
        done => return done,
    }
    let longer = replan((planned.measures * 5).div_ceil(4))?;
    search(&longer, Beams::All)
}

/// The search for a valid request: the cadences as formulas, the final one on the part's ending,
/// with the part's opening (its chord, or a rest up to its start in every voice) and the text the
/// voices sing, with the beams `beams` as far as the request's budget allows. A cancelled request
/// fails at once.
fn setting(
    request: &FreeRequest,
    text: Option<SungText>,
    profile: &Profile,
    beams: Beams,
) -> Result<Vec<Generated>, GenerateError> {
    if cancelled(request.control.as_deref()) {
        return Err(GenerateError::Cancelled);
    }
    let voices = request.voices.len();
    let mut base = Score::new(request.mode, Vec::new());
    base.cadences = request.cadences.clone();
    base.cadences.sort_by_key(|c| c.measure);
    let meter = base.meter;
    let mut fixed = cadence_formula(
        request.mode,
        request.part.ending_in(request.mode),
        request.measures - 1,
        voices,
        meter,
    );
    fixed.extend(request.part.opening(request.mode, voices, meter));
    for cadence in &base.cadences {
        fixed.extend(cadence_formula(
            request.mode,
            cadence.target,
            cadence.measure,
            voices,
            meter,
        ));
    }
    let widen = match beams {
        Beams::All => 4,
        Beams::UpToTwice => 2,
    }
    .min(request.budget.widening);
    let beams = (request.beam, widen, request.count);
    let search = free_search(request, base, request.measures, fixed, text, beams);
    generate_counterpoint(&search, profile)
}

/// The search of a free setting of `request`'s voices, in score order, on `base` (its mode, its
/// cadences and, for an imitation, its plan), `measures` long with the `fixed` notes, singing
/// `text`, with the request's seed, noise, ending and control: `(beam, widen, count)` give the
/// beam, how far it may widen and the results wanted.
pub(crate) fn free_search(
    request: &FreeRequest,
    base: Score,
    measures: usize,
    fixed: Vec<Fixed>,
    text: Option<SungText>,
    (beam, widen, count): (usize, usize, usize),
) -> CounterpointRequest {
    let mut voices = request.voices.clone();
    voices.sort();
    CounterpointRequest {
        measures: Some(measures),
        fixed,
        seed: request.seed,
        count,
        beam,
        noise: request.noise,
        text,
        widen,
        ending: request.part.ending,
        control: request.control.clone(),
        ..CounterpointRequest::new(
            base,
            voices
                .iter()
                .map(|&voice_type| NewVoice {
                    voice_type,
                    species: Species::Free,
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Meter;

    fn mode(name: &str) -> ModeSpec {
        name.parse().unwrap()
    }

    #[test]
    fn regular_cadences_follow_the_mode() {
        let names = |m: &str| -> String {
            regular_cadences(mode(m))
                .iter()
                .map(|s| s.letter())
                .collect()
        };
        assert_eq!(names("dorian"), "AFG");
        assert_eq!(names("phrygian"), "ACG");
        // Lydian: no cadence on B.
        assert_eq!(names("lydian"), "CA");
        assert_eq!(names("mixolydian"), "DC");
        assert_eq!(names("aeolian"), "ECD");
        assert_eq!(names("ionian"), "GEF");
        // Transposed Dorian on G: the fifth D, the third B♭, the fourth C.
        assert_eq!(names("dorian-transposed"), "DBC");
    }

    #[test]
    fn default_cadences_are_spaced() {
        let goals = |measures| -> Vec<usize> {
            default_cadences(mode("dorian"), measures)
                .iter()
                .map(|c| c.measure)
                .collect()
        };
        assert!(goals(8).is_empty());
        assert_eq!(goals(12), [6]);
        assert_eq!(goals(16), [5, 10]);
        for measures in 4..40 {
            let request = FreeRequest {
                cadences: default_cadences(mode("dorian"), measures),
                ..FreeRequest::new(mode("dorian"), vec![VoiceType::Soprano; 2], measures)
            };
            validate(&request).unwrap();
        }
    }

    #[test]
    fn formulas_name_the_clausula() {
        let notes = |m: &str, target, voices, voice: usize| -> Vec<(u32, String, bool)> {
            cadence_formula(mode(m), target, 7, voices, Meter::ALLA_BREVE)
                .iter()
                .filter(|f| f.voice == voice)
                .inspect(|f| assert!(f.any_octave))
                .map(|f| {
                    let pitch = f.note.pitch.unwrap().to_string();
                    (f.start, pitch.trim_end_matches('4').to_string(), f.tied_in)
                })
                .collect()
        };
        // Four voices: the soprano holds D over the barline, then C♯ and D; the tenor comes from
        // E, the bass from A; the alto is free.
        assert_eq!(
            notes("dorian", Step::D, 4, 0),
            [
                (48, "D".into(), true),
                (52, "C#".into(), false),
                (56, "D".into(), false)
            ]
        );
        assert!(notes("dorian", Step::D, 4, 1).is_empty());
        assert_eq!(
            notes("dorian", Step::D, 4, 2),
            [(48, "E".into(), false), (56, "D".into(), false)]
        );
        assert_eq!(
            notes("dorian", Step::D, 4, 3),
            [(48, "A".into(), false), (56, "D".into(), false)]
        );
        // Three voices: no tenorizans; Phrygian: the bass sings it, the cantizans comes from D.
        assert!(notes("dorian", Step::D, 3, 1).is_empty());
        assert_eq!(
            notes("phrygian", Step::E, 3, 2),
            [(48, "F".into(), false), (56, "E".into(), false)]
        );
        assert_eq!(notes("phrygian", Step::E, 3, 0)[1].1, "D");
        // No cadence on B.
        assert!(cadence_formula(mode("dorian"), Step::B, 7, 4, Meter::ALLA_BREVE).is_empty());
    }

    /// Fixed notes are formula notes unless they say otherwise, and the formulas say so: in every
    /// mode, transposed or not, with two to six voices and on every degree of a cadence, their
    /// notes leave the octave to the search and the underlay to the text.
    #[test]
    fn fixed_roles_default_to_formula() {
        assert_eq!(FixedRole::default(), FixedRole::Formula);
        let mut count = 0;
        for transposed in [false, true] {
            for m in Mode::ALL {
                let mode = ModeSpec::new(m).with_transposition(transposed);
                let targets = std::iter::once(mode.final_step()).chain(regular_cadences(mode));
                for target in targets {
                    for voices in 2..=6 {
                        for f in cadence_formula(mode, target, 7, voices, Meter::ALLA_BREVE) {
                            let what = (f.role, f.sung, f.any_octave);
                            assert_eq!(what, (FixedRole::Formula, None, true), "{f:?}");
                            count += 1;
                        }
                    }
                }
            }
        }
        assert!(count > 500, "{count} formula notes");
    }

    /// What a fake search was asked: the plan's length and cadences, and the beams.
    type Asked = Vec<(usize, Vec<Cadence>, Beams)>;

    /// Runs [`with_plans`] on Kyrie I (16 measures, Dorian, four voices) with a fake search that
    /// fails with `rules` as often as `failures` says, and records what it was asked.
    fn plans_tried(
        auto_length: bool,
        failures: usize,
        rules: &[&'static str],
    ) -> (Asked, Result<Vec<Generated>, GenerateError>) {
        let kyrie = crate::text::ordinary::section("kyrie", &Default::default()).unwrap();
        let replan = |measures: usize| plan(kyrie, mode("dorian"), 4, Some(measures), None, true);
        let planned = plan(kyrie, mode("dorian"), 4, None, None, true).unwrap();
        let mut asked = Vec::new();
        let result = with_plans(planned, auto_length, replan, |planned, beams| {
            asked.push((planned.measures, planned.cadences.clone(), beams));
            if asked.len() <= failures {
                Err(GenerateError::NoCounterpoint {
                    measure: 9,
                    rules: rules.to_vec(),
                })
            } else {
                Ok(Vec::new())
            }
        });
        (asked, result)
    }

    /// A planned length gets a quarter more measures once, with its cadences planned anew, right
    /// after twice the beam and before four times the beam, whatever cut the search off.
    #[test]
    fn a_planned_length_grows_once() {
        let kyrie = crate::text::ordinary::section("kyrie", &Default::default()).unwrap();
        let cadences = |measures| {
            plan(kyrie, mode("dorian"), 4, Some(measures), None, true)
                .unwrap()
                .cadences
        };
        let (planned, longer) = (cadences(16), cadences(20));
        assert_eq!(planned, cadences_at(mode("dorian"), &[6]));
        assert_ne!(longer, planned);
        for rules in [
            &["TXT-COMPLETE", "FREE-RESOLUTION"][..],
            &["FREE-RESOLUTION"],
        ] {
            let (asked, result) = plans_tried(true, 1, rules);
            assert_eq!(
                asked,
                [
                    (16, planned.clone(), Beams::UpToTwice),
                    (20, longer.clone(), Beams::All)
                ],
                "{rules:?}"
            );
            assert!(result.is_ok());
        }
        // Found with the beam or twice the beam: no longer plan.
        let (asked, result) = plans_tried(true, 0, &["FREE-RESOLUTION"]);
        assert_eq!(asked, [(16, planned.clone(), Beams::UpToTwice)]);
        assert!(result.is_ok());
        // Only once longer: the error of the longer plan.
        let (asked, result) = plans_tried(true, usize::MAX, &["TXT-COMPLETE"]);
        assert_eq!(asked.len(), 2);
        assert!(matches!(result, Err(GenerateError::NoCounterpoint { .. })));
        // A given length is searched with every beam, and never longer.
        let (asked, result) = plans_tried(false, usize::MAX, &["TXT-COMPLETE"]);
        assert_eq!(asked, [(16, planned, Beams::All)]);
        assert!(result.is_err());
        // Another error ends at once.
        let mut asked = 0;
        let planned = plan(kyrie, mode("dorian"), 4, None, None, true).unwrap();
        let result = with_plans(
            planned,
            true,
            |_| unreachable!(),
            |_, _| {
                asked += 1;
                Err(GenerateError::Invalid("no".into()))
            },
        );
        assert_eq!(
            (asked, result.unwrap_err()),
            (1, GenerateError::Invalid("no".into()))
        );
    }

    #[test]
    fn texted_requests_need_voices() {
        let kyrie = crate::text::ordinary::section("kyrie", &Default::default()).unwrap();
        let request = FreeRequest {
            text: Some(FreeText::new(Arc::new(kyrie.clone()))),
            ..FreeRequest::new(mode("dorian"), vec![VoiceType::Soprano], 0)
        };
        assert_eq!(
            generate_free(&request, &Profile::fux()).unwrap_err(),
            GenerateError::TooFewVoices
        );
    }

    #[test]
    fn requests_need_room() {
        let request = |measures, goals: &[usize]| FreeRequest {
            cadences: cadences_at(mode("dorian"), goals),
            ..FreeRequest::new(
                mode("dorian"),
                vec![VoiceType::Soprano, VoiceType::Alto],
                measures,
            )
        };
        assert_eq!(
            validate(&request(3, &[])),
            Err(GenerateError::TooShort { least: 4 })
        );
        assert!(validate(&request(4, &[])).is_ok());
        assert!(validate(&request(12, &[6])).is_ok());
        // Too close to the final cadence: the internal one is named, in measure 10 (from 1).
        let close = validate(&request(12, &[9])).unwrap_err();
        assert_eq!(close, GenerateError::CadenceTooClose { measure: 10 });
        assert_eq!(
            close.message(crate::lang::Lang::De),
            "Der Binnenschluss in Takt 10 lässt seiner Klausel keinen Platz: Der erste kommt \
             frühestens in Takt 3, und zwischen zwei Binnenschlüssen und vor dem Schlusstakt \
             liegen mindestens drei Takte."
        );
        assert_eq!(
            validate(&request(12, &[1])),
            Err(GenerateError::CadenceTooClose { measure: 2 })
        );
        assert_eq!(
            validate(&request(12, &[4, 6])),
            Err(GenerateError::CadenceTooClose { measure: 7 })
        );
        let mut on_b = request(12, &[6]);
        on_b.cadences[0].target = Step::B;
        assert_eq!(
            validate(&on_b).unwrap_err().message(crate::lang::Lang::De),
            "Kein Binnenschluss auf B in diesem Modus (verminderte Quinte)."
        );
    }

    /// A part ends on the final, the fifth or the fourth where the mode has a clausula there, in
    /// that order: the plagal modes as their authentic ones, the transposed a fourth higher with
    /// B♭ (the fourth of the Ionian on F, the fifth of the Lydian on B♭).
    #[test]
    fn part_endings_have_a_clausula() {
        let letters =
            |m: &str| -> String { part_endings(mode(m)).iter().map(|s| s.letter()).collect() };
        for (name, endings) in [
            ("dorian", "DAG"),
            ("phrygian", "EA"),
            ("lydian", "FC"),
            ("mixolydian", "GDC"),
            ("aeolian", "AED"),
            ("ionian", "CGF"),
            ("dorian-transposed", "GDC"),
            ("phrygian-transposed", "AD"),
            ("lydian-transposed", "BF"),
            ("mixolydian-transposed", "CGF"),
            ("aeolian-transposed", "DAG"),
            ("ionian-transposed", "FCB"),
        ] {
            assert_eq!(letters(name), endings, "{name}");
            assert_eq!(letters(&format!("hypo{name}")), endings, "hypo{name}");
            let spec = mode(name);
            assert_eq!(part_endings(spec)[0], spec.final_step());
            assert!(
                part_endings(spec)
                    .iter()
                    .all(|&s| spec.clausula(s).is_some())
            );
        }
    }

    /// A free request for the soprano and the tenor, twelve measures, as the part `part`.
    fn part_request(part: PartContext) -> FreeRequest {
        FreeRequest {
            part,
            ..FreeRequest::new(
                mode("dorian"),
                vec![VoiceType::Soprano, VoiceType::Tenor],
                12,
            )
        }
    }

    /// A part asks for an ending the mode allows, a start of none, a minim or a measure, and an
    /// opening chord only without imitation and with a pitch for each voice; otherwise it fails
    /// before any search, naming what it asked for in German and English.
    #[test]
    fn parts_are_checked_before_the_search() {
        use crate::lang::Lang;
        let part = |ending: Option<Step>, start: u32, chord: Option<&str>| PartContext {
            ending,
            start,
            chord: chord.map(|c| c.split(' ').map(|p| p.parse().unwrap()).collect()),
            ..PartContext::default()
        };
        for ending in [None, Some(Step::D), Some(Step::A), Some(Step::G)] {
            for start in STARTS {
                assert_eq!(validate(&part_request(part(ending, start, None))), Ok(()));
            }
        }
        let on_e = validate(&part_request(part(Some(Step::E), 0, None))).unwrap_err();
        assert_eq!(
            on_e,
            GenerateError::Ending {
                mode: mode("dorian"),
                target: Step::E
            }
        );
        assert_eq!(
            on_e.message(Lang::De),
            "Ein Satz in diesem Modus schließt auf D, A oder G (Finalis, Quinte oder Quarte mit \
             einer Klausel), nicht auf E."
        );
        assert_eq!(
            on_e.message(Lang::En),
            "A setting in this mode ends on D, A or G (the final, or the fifth or the fourth with \
             a clausula), not on E."
        );
        let late = validate(&part_request(part(None, 12, None))).unwrap_err();
        assert_eq!(late, GenerateError::Start { start: 12 });
        assert_eq!(
            late.message(Lang::De),
            "Ein Satz beginnt sofort, nach einer Halben oder nach einem ganzen Takt Pause, nicht \
             nach 1,5 Takten."
        );
        assert_eq!(
            late.message(Lang::En),
            "A setting begins at once, after a minim or after a whole measure's rest, not after \
             1.5 measures."
        );
        assert!(validate(&part_request(part(None, 8, Some("A4 D3")))).is_ok());
        assert_eq!(
            validate(&part_request(part(None, 0, Some("A4 F4 D3")))),
            Err(GenerateError::ChordVoices {
                pitches: 3,
                voices: 2
            })
        );
        // With imitation the chord is refused before anything else of the imitation.
        let imitative = FreeRequest {
            imitation: Some(Imitation::default()),
            ..part_request(part(None, 0, Some("A4 D3")))
        };
        let error = generate_free(&imitative, &Profile::fux()).unwrap_err();
        assert_eq!(error, GenerateError::ChordWithImitation);
        assert_eq!(
            error.message(Lang::De),
            "Ein Anfangsklang lässt alle Stimmen zugleich einsetzen, eine Imitation eine nach der \
             anderen; beides zusammen geht nicht."
        );
        let imitative = FreeRequest {
            imitation: Some(Imitation::default()),
            ..part_request(part(Some(Step::C), 0, None))
        };
        assert!(matches!(
            generate_free(&imitative, &Profile::fux()),
            Err(GenerateError::Ending { .. })
        ));
    }

    /// The first internal cadence of a part comes two measures after its start, the third
    /// measure where the voices begin at once; after an opening chord on a downbeat a measure
    /// later, so that the cantizans prepares its suspension with a note of its own. Given
    /// cadences that come earlier are refused.
    #[test]
    fn first_cadences_leave_room_after_the_start() {
        let first = |start: u32, chord: bool| {
            PartContext {
                start,
                chord: chord.then(|| vec!["A4".parse().unwrap(), "D3".parse().unwrap()]),
                ..PartContext::default()
            }
            .first_goal()
        };
        assert_eq!(
            [first(0, false), first(4, false), first(8, false)],
            [2, 3, 3]
        );
        assert_eq!([first(0, true), first(4, true), first(8, true)], [3, 3, 4]);
        let with = |start: u32, chord: bool, goal: usize| FreeRequest {
            cadences: cadences_at(mode("dorian"), &[goal]),
            ..part_request(PartContext {
                start,
                chord: chord.then(|| vec!["A4".parse().unwrap(), "D3".parse().unwrap()]),
                ..PartContext::default()
            })
        };
        for (start, chord) in [(0, false), (4, false), (8, false), (0, true), (8, true)] {
            let least = first(start, chord);
            assert!(validate(&with(start, chord, least)).is_ok());
            assert_eq!(
                validate(&with(start, chord, least - 1)),
                Err(GenerateError::CadenceTooClose { measure: least })
            );
        }
    }

    /// The fixed notes of a part's opening, voice by voice as `start:pitch.ticks`, `~` before a
    /// continuation, the role and the pinned syllable.
    fn opening_of(part: &PartContext, voices: usize) -> Vec<Vec<String>> {
        let fixed = part.opening(mode("dorian"), voices, Meter::ALLA_BREVE);
        (0..voices)
            .map(|v| {
                fixed
                    .iter()
                    .filter(|f| f.voice == v)
                    .map(|f| {
                        let pitch = f.note.pitch.map_or("r".to_string(), |p| p.to_string());
                        let role = match f.role {
                            FixedRole::Formula => "F".to_string(),
                            FixedRole::Entry { rest } => format!("E{rest}"),
                            FixedRole::Subject => "S".to_string(),
                        };
                        let sung = match f.sung {
                            Some(Underlaid::Start(y)) => format!("={y}"),
                            Some(Underlaid::Hold) => "=_".to_string(),
                            None => String::new(),
                        };
                        let tie = if f.tied_in { "~" } else { "" };
                        let ticks = f.note.duration.ticks();
                        format!("{tie}{}:{pitch}.{ticks} {role}{sung}", f.start)
                    })
                    .collect()
            })
            .collect()
    }

    /// An opening chord gives every voice its pitch as a semibreve at the start, an entry after
    /// silence from the beginning on the first syllable, split and tied at the barline after a
    /// minim's breath; without a chord the voices rest up to the start; a part that begins at
    /// once has no opening.
    #[test]
    fn openings_of_parts() {
        let chord = |start: u32| PartContext {
            start,
            chord: Some(vec!["A4".parse().unwrap(), "D3".parse().unwrap()]),
            ..PartContext::default()
        };
        assert_eq!(opening_of(&chord(0), 2), [["0:A4.8 E0=0"], ["0:D3.8 E0=0"]]);
        assert_eq!(
            opening_of(&chord(4), 2),
            [["4:A4.4 E0=0", "~8:A4.4 S"], ["4:D3.4 E0=0", "~8:D3.4 S"]]
        );
        assert_eq!(opening_of(&chord(8), 2), [["8:A4.8 E0=0"], ["8:D3.8 E0=0"]]);
        let breath = |start: u32| PartContext {
            start,
            ..PartContext::default()
        };
        assert_eq!(opening_of(&breath(4), 2), [["0:r.4 F"], ["0:r.4 F"]]);
        assert_eq!(opening_of(&breath(8), 3)[2], ["0:r.8 F"]);
        assert!(
            PartContext::default()
                .opening(mode("dorian"), 4, Meter::ALLA_BREVE)
                .is_empty()
        );
    }

    /// A request is a setting of its own unless it says otherwise: on the final, at once, without
    /// a chord, its cadences in turn, with the platform's budget and no control. The portable
    /// budget is the same everywhere: six probes and twice the beam.
    #[test]
    fn requests_are_settings_of_their_own() {
        let request = FreeRequest::new(mode("phrygian"), vec![VoiceType::Alto; 2], 10);
        assert_eq!(request.part, PartContext::default());
        assert_eq!(request.part.ending_in(request.mode), Step::E);
        assert_eq!(
            (request.part.start, &request.part.chord, &request.part.chain),
            (0, &None, &CadenceChain::Rotation)
        );
        assert_eq!(request.budget, Budget::PLATFORM);
        assert_eq!(Budget::default(), Budget::PLATFORM);
        assert!(request.control.is_none());
        let native = !cfg!(target_arch = "wasm32");
        assert_eq!(
            Budget::PLATFORM,
            Budget {
                probes: if native { 6 } else { 2 },
                widening: MAX_WIDENING
            }
        );
        assert_eq!(
            Budget::PORTABLE,
            Budget {
                probes: 6,
                widening: 2
            }
        );
        // The control starts uncancelled at no measure.
        let control = SearchControl::new();
        assert!(!control.is_cancelled() && !cancelled(Some(&control)));
        assert_eq!(control.reached(), 0);
        control.reach(7);
        control.cancel();
        assert!(control.is_cancelled() && cancelled(Some(&control)));
        assert_eq!(control.reached(), 7);
        assert!(!cancelled(None));
    }

    /// A cancelled request fails before its search, without imitation as with it, and a wider
    /// beam is not tried.
    #[test]
    fn cancelled_requests_fail_at_once() {
        let control = Arc::new(SearchControl::new());
        control.cancel();
        let untexted = FreeRequest {
            control: Some(Arc::clone(&control)),
            ..part_request(PartContext::default())
        };
        assert_eq!(
            generate_free(&untexted, &Profile::fux()).unwrap_err(),
            GenerateError::Cancelled
        );
        let kyrie = crate::text::ordinary::section("kyrie", &Default::default()).unwrap();
        let texted = FreeRequest {
            text: Some(FreeText::new(Arc::new(kyrie.clone()))),
            ..untexted.clone()
        };
        assert_eq!(
            generate_free(&texted, &Profile::fux()).unwrap_err(),
            GenerateError::Cancelled
        );
        let imitative = FreeRequest {
            imitation: Some(Imitation::default()),
            ..texted
        };
        assert_eq!(
            generate_free(&imitative, &Profile::fux()).unwrap_err(),
            GenerateError::Cancelled
        );
        assert_eq!(control.reached(), 0, "no search began");
    }
}
