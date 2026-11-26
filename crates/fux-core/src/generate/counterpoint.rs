//! Counterpoint generation: new voices against a cantus firmus and other given voices, found by
//! a beam search over the notes and checked by the rule engine.
//!
//! With a text, every new voice carries through the search the underlays its notes allow (the
//! automaton of `text::underlay`), which cuts notes that leave it none and steers the voices
//! through the text; after the search the cheapest underlay is laid under each result.

use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::{Arc, OnceLock};

use super::free::{SearchControl, cancelled};
use super::rng::Rng;
use super::{GenerateError, Generated};
use crate::clef::VoiceType;
use crate::interval::Interval;
use crate::lang::Lang;
use crate::mode::{Clausula, Mode, ModeSpec};
use crate::pitch::{Pitch, Step};
use crate::rhythm::{self, Place, Token};
use crate::rules::{Analysis, Checker, Level, Profile, Window};
use crate::score::{Note, Role, Score, ScoreText, Species, Voice};
use crate::text::underlay::{
    Automaton, Capacity, Choice as Underlaid, Gates, Goal, LIMIT, Levels, Model, Pins, Set,
};
use crate::text::{Text, TextKind};
use crate::time::{Duration, Meter};

/// A voice to generate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewVoice {
    pub voice_type: VoiceType,
    pub species: Species,
}

/// A note given in a new voice, which the search takes as it is: a cadence formula, an entry of
/// an imitation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fixed {
    /// Index into the request's voices.
    pub voice: usize,
    /// Where the note begins.
    pub start: u32,
    /// Pitch (or rest) and duration; the tie into the next note comes with the next fixed note.
    pub note: Note,
    /// Continues the note before, tied over to it.
    pub tied_in: bool,
    /// The pitch in any octave of the voice's pitches: the search chooses where it lies.
    pub any_octave: bool,
    /// What the note is for.
    pub role: FixedRole,
    /// The underlay pinned at the note's attack: `Start` where it begins a syllable, `Hold` in
    /// the syllable's melisma. `None` leaves it to the underlay, as for the formulas and the
    /// continuations of tied notes.
    pub sung: Option<Underlaid>,
}

/// What a fixed note is for: a cadence formula, or the head of an imitative entry (phase 13),
/// whose pitches are exact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FixedRole {
    /// A note of a cadence formula.
    #[default]
    Formula,
    /// The first note of an entry, after at least `rest` ticks of silence; 0 for the voice's
    /// first note, before which it is silent from the start.
    Entry { rest: u32 },
    /// A further note of an entry's head, the continuations of tied notes included.
    Subject,
}

impl Fixed {
    /// Whether `pitch` is the fixed pitch, in any octave if that is free.
    fn matches(&self, pitch: Option<Pitch>) -> bool {
        match (self.note.pitch, pitch) {
            (Some(fixed), Some(pitch)) if self.any_octave => {
                fixed.step == pitch.step && fixed.alter == pitch.alter
            }
            (fixed, pitch) => fixed == pitch,
        }
    }

    /// Whether the note belongs to the head of an entry, whose pitch is exact.
    fn in_head(&self) -> bool {
        self.role != FixedRole::Formula
    }

    /// The rest window before the note, if it is an entry after a rest: (start, ticks).
    fn rest_window(&self) -> Option<(u32, u32)> {
        match self.role {
            FixedRole::Entry { rest } if rest > 0 => {
                let rest = rest.min(self.start);
                Some((self.start - rest, rest))
            }
            _ => None,
        }
    }
}

/// Where a voice with these fixed notes (in order) enters: its first note of a head, if that is
/// an entry. The voice is silent before it.
fn first_entry(fixed: &[&Fixed]) -> Option<u32> {
    fixed
        .iter()
        .find(|f| f.in_head())
        .filter(|f| matches!(f.role, FixedRole::Entry { .. }))
        .map(|f| f.start)
}

/// The silences that a voice's entries demand, as (start, ticks): from the start to its first
/// entry, and before every entry after a rest its rest window, the least rest it plans. None
/// for a voice without entries.
fn silences(fixed: &[&Fixed]) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = first_entry(fixed)
        .map(|start| (0, start))
        .into_iter()
        .collect();
    out.extend(fixed.iter().filter_map(|f| f.rest_window()));
    out
}

/// A text the new voices sing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SungText {
    pub text: Arc<Text>,
    /// The planned pace: syllable starts per 100 minim beats, restatements included. In a long
    /// text the search keeps the voices from racing ahead of it.
    pub rate: u16,
}

/// What to generate.
#[derive(Clone, Debug)]
pub struct CounterpointRequest {
    /// The given voices, complete, with the cantus firmus among them; for free counterpoint
    /// without a cantus firmus no voices, with the mode, the meter and the cadences.
    pub base: Score,
    pub voices: Vec<NewVoice>,
    /// The number of measures when the base has no voices.
    pub measures: Option<usize>,
    /// Notes given in the new voices.
    pub fixed: Vec<Fixed>,
    pub seed: u64,
    /// Number of distinct solutions to return, best first.
    pub count: usize,
    /// Partial solutions kept per measure.
    pub beam: usize,
    /// Random noise added to the costs when ranking, for variety (0 = always the cheapest).
    pub noise: f64,
    /// The text every new voice sings; `None` for untexted voices. The search keeps, per voice,
    /// the underlays its notes allow (`text::underlay`) and lays the cheapest one under the
    /// notes of each result.
    pub text: Option<SungText>,
    /// How wide the beam may grow when the search finds nothing, as a multiple of `beam`: 4
    /// tries twice and then four times the beam, 2 only twice, 1 not at all. WebAssembly stops
    /// at twice ([`MAX_WIDENING`]).
    pub widen: usize,
    /// The target of the final cadence of free counterpoint; `None` for the final of the mode
    /// (phase 14: a part of a mass section may end on the fifth or the fourth).
    pub ending: Option<Step>,
    /// Cancels the search, which it reads once per step, and learns the measure it has reached;
    /// `None` for a search that runs to its end.
    pub control: Option<Arc<SearchControl>>,
}

impl CounterpointRequest {
    pub fn new(base: Score, voices: Vec<NewVoice>) -> CounterpointRequest {
        CounterpointRequest {
            base,
            voices,
            measures: None,
            fixed: Vec::new(),
            seed: 1,
            count: 1,
            beam: 200,
            noise: 2.0,
            text: None,
            widen: 4,
            ending: None,
            control: None,
        }
    }
}

/// How wide the beam grows at most, as a multiple of the request's: in WebAssembly twice, since
/// five and six voices take several gigabytes with four times the beam.
pub const MAX_WIDENING: usize = if cfg!(target_arch = "wasm32") { 2 } else { 4 };

/// Pitches a new voice may use: scale pitches of the mode in its range (with two steps beyond
/// the staff, which VOI-RANGE charges for), the subsemitonium of the final, in the Aeolian mode
/// the raised sixth before it, the major third of the final for the last chord, and in the
/// Dorian and Lydian modes the flattened mi.
pub fn pitch_pool(mode: ModeSpec, voice_type: VoiceType) -> Vec<Pitch> {
    pitch_pool_with(mode, voice_type, &[])
}

/// Like [`pitch_pool`], with the leading tones of internal cadences on `targets` (G♯ for a
/// cadence on A in the Dorian mode).
pub fn pitch_pool_with(mode: ModeSpec, voice_type: VoiceType, targets: &[Step]) -> Vec<Pitch> {
    pitch_pool_ending(mode, voice_type, targets, mode.final_step())
}

/// Like [`pitch_pool_with`] for a setting that ends on `ending` (a part of a mass section, phase
/// 14): the ending's leading tone and the major third of its last chord join the pool, C♯ for a
/// part that ends on A in the Phrygian mode. On the final it is [`pitch_pool_with`].
pub fn pitch_pool_ending(
    mode: ModeSpec,
    voice_type: VoiceType,
    targets: &[Step],
    ending: Step,
) -> Vec<Pitch> {
    let clef = voice_type.clef();
    let low = clef.bottom_line().diatonic() - 3;
    let high = clef.top_line().diatonic() + 3;
    let mut pool: Vec<Pitch> = (low..=high).map(|d| mode.scale_pitch(d)).collect();
    let mut leading: Vec<Step> = vec![mode.final_step()];
    leading.extend(targets.iter().copied());
    if ending != mode.final_step() {
        leading.push(ending);
    }
    leading.retain(|&t| mode.clausula(t) == Some(Clausula::Leading));
    // The major third of the final for the last chord (F♯ in the Dorian mode), and of the
    // ending's where the setting ends elsewhere.
    let mut thirds = vec![mode.major_third(4)];
    if ending != mode.final_step() {
        let e = Pitch::new(ending, mode.scale_alter(ending), 4);
        thirds.push(e.transpose(Interval::MAJOR_THIRD));
    }
    for d in low..=high {
        let p = mode.scale_pitch(d);
        for third in &thirds {
            if p.step == third.step && p.alter != third.alter {
                pool.push(p.with_alter(third.alter));
            }
        }
        let above = mode.scale_pitch(d + 1);
        if leading.contains(&above.step) {
            let sub = above.transpose(-Interval::MINOR_SECOND);
            if sub != p {
                pool.push(sub);
                // Avoid the augmented second before the leading tone (Aeolian F♯–G♯–A).
                let sixth = mode.scale_pitch(d - 1);
                if Interval::between(sixth, sub).semitones == 3 {
                    pool.push(sixth.transpose(Interval::new(0, 1)));
                }
            }
        }
        if matches!(mode.mode, Mode::Dorian | Mode::Lydian) && p.step == mode.flattened_mi() {
            pool.push(p.with_alter(p.alter - 1));
        }
    }
    pool.sort_by_key(|p| (p.diatonic(), p.midi()));
    pool.dedup();
    pool
}

/// A usual choir of `voices` voices with a voice of the cantus' type: three voices as in Fux
/// (A T B, S A T or S A B), four S A T B, five S A T T B, six S S A T T B. A cantus of another
/// type takes the place of the voice nearest to it. Two voices: the cantus and its neighbour
/// above (or below, for the highest voice).
pub fn default_choir(voices: usize, cantus: VoiceType) -> Vec<VoiceType> {
    choir_with(voices, cantus, None)
}

/// Like [`default_choir`], with a voice of type `also` besides the cantus' (for the voice in the
/// species): a usual choir with both if there is one, otherwise `also` takes the place of the
/// voice nearest to it.
pub fn choir_with(voices: usize, cantus: VoiceType, also: Option<VoiceType>) -> Vec<VoiceType> {
    use VoiceType::*;
    let sets: &[&[VoiceType]] = match voices {
        0..=2 => {
            let other = cantus
                .neighbor(true)
                .or_else(|| cantus.neighbor(false))
                .expect("a neighbour");
            let mut pair = vec![cantus, other];
            pair.sort();
            return pair;
        }
        3 => &[
            &[Alto, Tenor, Bass],
            &[Soprano, Alto, Tenor],
            &[Soprano, Alto, Bass],
        ],
        4 => &[&[Soprano, Alto, Tenor, Bass]],
        5 => &[&[Soprano, Alto, Tenor, Tenor, Bass]],
        _ => &[&[Soprano, Soprano, Alto, Tenor, Tenor, Bass]],
    };
    let has_both = |set: &[VoiceType]| {
        let mut rest = set.to_vec();
        rest.iter()
            .position(|v| *v == cantus)
            .map(|i| rest.remove(i))
            .is_some()
            && also.is_none_or(|a| rest.contains(&a))
    };
    if let Some(set) = sets.iter().find(|set| has_both(set)) {
        return set.to_vec();
    }
    let mut set = sets
        .iter()
        .find(|set| set.contains(&cantus))
        .unwrap_or(&sets[0])
        .to_vec();
    // Put a voice in at the place of the voice nearest to it, keeping the cantus' place.
    let put = |set: &mut Vec<VoiceType>, voice: VoiceType, keep: Option<usize>| {
        let nearest = (0..set.len())
            .filter(|&i| Some(i) != keep)
            .min_by_key(|&i| (set[i] as i32 - voice as i32).abs())
            .expect("a voice");
        set[nearest] = voice;
        set.sort();
    };
    if !set.contains(&cantus) {
        put(&mut set, cantus, None);
    }
    if let Some(also) = also {
        let keep = set.iter().position(|v| *v == cantus);
        if !set
            .iter()
            .enumerate()
            .any(|(i, v)| *v == also && Some(i) != keep)
        {
            put(&mut set, also, keep);
        }
    }
    set
}

/// How many times the beam the search keeps between the voices of the final sonority.
const FINAL_WIDTH: usize = 20;

/// What becomes of a state with one more choice.
enum Outcome {
    /// A new state.
    Keep {
        notes: Vec<Vec<Note>>,
        /// The cost so far.
        cost: u32,
        /// Whether the cost covers the step's window.
        checked: bool,
        /// The voice's new underlay states, where it has a new note.
        set: Option<Arc<Set>>,
        /// The voice's new steering, where it has a new note.
        term: Option<Steer>,
        /// The steering of the text ([`Underlay::steer`]); 0 without a text.
        steer: f64,
        /// The ranking's look ahead, computed in the parallel job: `distance_to_fixed + Σ
        /// prefix_cost` and `clash_with_fixed` (0 without heads ahead).
        ahead: (f64, f64),
    },
    /// Cut off by these rules, when looking ahead at the given tick.
    Blocked(Vec<&'static str>, Option<u32>),
    /// Left out before any check.
    Skip,
}

/// How the ranking steers sung voices through their text: its tuning constants. A voice that
/// falls behind its text, or in a long text races ahead of the planned pace, a voice whose
/// formula note at an internal goal comes before its words can stop there (at the end of a
/// sense unit or before a marked break), and a voice that lags behind the syllable its next
/// entry begins, rank as if they cost more.
#[derive(Clone, Copy, Debug)]
struct Steering {
    /// A voice falls behind when it still needs more syllables than this share of the minim
    /// beats on which it can begin one; before an entry, more than this share of those left
    /// before it.
    behind: f64,
    /// In a long text a voice races ahead when it needs fewer syllables than this share of
    /// those the planned pace begins on the beats left.
    ahead: f64,
    /// The weight of the steering against the costs.
    weight: f64,
    /// How many measures ahead a formula note at an internal goal draws its voice to the end of
    /// a sense unit or a marked break, and an entry to the syllable it begins.
    horizon: u32,
}

/// Tuned on texted settings of two to six voices (phase 12). A horizon of four measures with a
/// weight of 3 lowered the cost of TXT-CADENCE in two to four voices by a third while repeated
/// notes within a melisma cost nothing; since MEL-REPEAT `free_black` prices them, it no longer
/// does (96 settings of Kyrie I, Christe, Agnus III and `gloria:1-5`: less in two and three
/// voices, more in four), lowers it by an eighth in five voices (16 settings), and six voices
/// singing the Gloria then need a longer plan. Counting only the beats on which a formula voice
/// can begin a syllable lowers the cost of the text by about 5 %, but the search takes about a
/// third longer. No other value tried was better everywhere.
const STEERING: Steering = Steering {
    behind: 0.8,
    ahead: 0.5,
    weight: 2.0,
    horizon: 2,
};

/// The steering of one new voice ([`Underlay::term`]): how far it falls behind its text, how
/// many syllables it lacks before an internal goal, if one is near, and how far it lags behind
/// the syllable of its next entry, if that is near. It changes only with the voice's notes and
/// underlay states, so each state of the search keeps it per voice.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Steer {
    pace: f64,
    goal: Option<f64>,
    entry: Option<f64>,
}

/// The text in the search: the underlay model with the profile's levels, and per new voice the
/// beats left for its syllables, its formula notes at internal goals and the syllables pinned
/// at the attacks of its entries' heads.
struct Underlay {
    model: Model,
    levels: Levels,
    /// Where the voices end.
    total: u32,
    /// Per new voice.
    caps: Vec<Capacity>,
    /// Per new voice, the ticks of the internal goals at which a formula note of it begins.
    goals: Vec<Vec<u32>>,
    /// Per new voice, the underlay pinned at the attacks of its heads, by tick.
    pins: Vec<Vec<(u32, Underlaid)>>,
    /// Per new voice, its entries: where each begins and the syllable it begins there.
    entries: Vec<Vec<(u32, u16)>>,
    /// Per new voice, the syllables it does not begin before the point of an imitation that sets
    /// them takes it in ([`Gates`]); none without a plan.
    gates: Vec<Vec<(u16, u32)>>,
    /// The planned syllable starts per minim beat.
    rate: f64,
    steering: Steering,
    /// The steering's horizon in ticks.
    horizon: u32,
}

impl Underlay {
    /// The underlay of `sung` for the new voices with these fixed notes, in voices of `total`
    /// ticks. TXT-CADENCE judges the base's internal cadences; every voice holds its last
    /// measure, which the capacity leaves out with the fixed notes, as it leaves out the silence
    /// before a voice's first entry and the rest window before each later one. The heads of the
    /// entries sing the syllables pinned at their attacks.
    fn new(
        sung: &SungText,
        base: &Score,
        fixed: &[Vec<&Fixed>],
        total: u32,
        checker: &Checker,
    ) -> Underlay {
        let per_measure = base.meter.ticks_per_measure();
        // The goals as the TXT rules take them from the finished score, which keeps the base's
        // cadences.
        let goals = Goal::of(&base.cadences, base.meter);
        let restarts = base
            .imitation
            .as_deref()
            .map(|plan| plan.restarts(&sung.text))
            .unwrap_or_default();
        let model = Model::new(&sung.text, base.mode, base.meter, &goals)
            .with_restarts(&sung.text, &restarts);
        let levels = Levels::from(checker);
        let caps = fixed
            .iter()
            .map(|list| {
                // A fixed rest (the breath before a part's start) is held from its first tick: no
                // syllable begins there.
                let mut held: Vec<(u32, u32, bool)> = list
                    .iter()
                    .map(|f| {
                        (
                            f.start,
                            f.note.duration.ticks(),
                            f.tied_in || f.note.is_rest(),
                        )
                    })
                    .collect();
                // No syllable begins where the voice is silent: before its first entry and in
                // the rest window before every later one.
                held.extend(
                    silences(list)
                        .into_iter()
                        .map(|(start, ticks)| (start, ticks, true)),
                );
                held.push((total.saturating_sub(per_measure), per_measure, false));
                Capacity::new(total, &held, &levels)
            })
            .collect();
        let formula_goals = fixed
            .iter()
            .map(|list| {
                let mut ticks: Vec<u32> = list
                    .iter()
                    .filter(|f| !f.in_head())
                    .map(|f| f.start)
                    .filter(|&start| goals.iter().any(|g| g.tick == start))
                    .collect();
                ticks.dedup();
                ticks
            })
            .collect();
        let pins: Vec<Vec<(u32, Underlaid)>> = fixed
            .iter()
            .map(|list| {
                list.iter()
                    .filter_map(|f| f.sung.map(|sung| (f.start, sung)))
                    .collect()
            })
            .collect();
        debug_assert!(
            pins.iter()
                .all(|pins| pins.windows(2).all(|w| w[0].0 < w[1].0)),
            "one pinned choice per attack"
        );
        let entries = fixed
            .iter()
            .map(|list| {
                list.iter()
                    .filter_map(|f| match (f.role, f.sung) {
                        (FixedRole::Entry { .. }, Some(Underlaid::Start(y))) => Some((f.start, y)),
                        _ => None,
                    })
                    .collect()
            })
            .collect();
        let gates = (0..fixed.len())
            .map(|k| {
                base.imitation
                    .as_deref()
                    .map(|plan| plan.gates(k))
                    .unwrap_or_default()
            })
            .collect();
        let steering = STEERING;
        Underlay {
            model,
            levels,
            total,
            caps,
            goals: formula_goals,
            pins,
            entries,
            gates,
            rate: f64::from(sung.rate) / 100.0,
            steering,
            horizon: steering.horizon * per_measure,
        }
    }

    /// The choices pinned in new voice k, if it has any.
    fn pins(&self, k: usize) -> Option<&Pins> {
        let pins = &self.pins[k];
        (!pins.is_empty()).then_some(&pins[..])
    }

    /// The gates of new voice k, if it has any.
    fn gates(&self, k: usize) -> Option<&Gates> {
        let gates = &self.gates[k];
        (!gates.is_empty()).then_some(&gates[..])
    }

    /// The automaton of new voice k, as the search runs it. With pins, the next pinned syllable
    /// start is also a milestone: states that can no longer begin it in time are cut; no syllable
    /// begins before its gate.
    fn automaton(&self, k: usize) -> Automaton<'_> {
        Automaton {
            model: &self.model,
            levels: &self.levels,
            total: self.total,
            cap: Some(&self.caps[k]),
            limit: Some(LIMIT),
            pins: self.pins(k),
            gates: self.gates(k),
        }
    }

    /// The underlay states of voice k after its notes, from those before its last note: every
    /// note steps them as the TXT rules check it, a tie continuation too
    /// ([`Event::Held`](crate::text::underlay::Event::Held)), which may already decide
    /// TXT-ACCENT; a pinned attack takes only its pinned choice. Fails with the rule that cut
    /// the most states if none is left.
    fn advance(&self, k: usize, set: &Set, notes: &[Note]) -> Result<Arc<Set>, &'static str> {
        self.automaton(k).advance(set, notes).map(Arc::new)
    }

    /// The steering of new voice k with these notes and underlay states: at the end of its
    /// notes, the least over its states of how far it falls behind its text (or in a long text
    /// races ahead of the planned pace), where a voice with entries that has ended its text and
    /// rested still has to restate the last words before the final measure (without that, the
    /// wide beams filled up with voices that rested after their last heads and could not end);
    /// before an internal goal at which it has a formula note, how many syllables it lacks to
    /// stop its words there, at the end of a sense unit or before a marked break (TXT-CADENCE);
    /// and before an entry, how many syllables more than the share `behind` of the beats left
    /// before it (on which a syllable can begin) it still has to begin before the entry's
    /// syllable ([`Model::to_reach`]).
    fn term(&self, k: usize, line: &[Note], states: &Set) -> Steer {
        let s = &self.steering;
        let long = self.model.kind() == TextKind::Long;
        let end: u32 = line.iter().map(|n| n.duration.ticks()).sum();
        let cap = f64::from(self.caps[k].at(end));
        // A voice with entries sounds to the end ([`Model::need_sounding`]).
        let pinned = self.pins(k).is_some();
        let pace = states
            .iter()
            .map(|(state, _, _)| {
                let need = if pinned {
                    self.model.need_sounding(state)
                } else {
                    self.model.need(state)
                };
                let need = f64::from(need);
                let behind = (need - s.behind * cap).max(0.0);
                if long {
                    behind + (s.ahead * self.rate * cap - need).max(0.0)
                } else {
                    behind
                }
            })
            .fold(f64::INFINITY, f64::min);
        let goal = self.goals[k]
            .iter()
            .find(|&&g| g >= end && g - end < self.horizon)
            .map(|&goal| {
                let beats = f64::from((goal - end) / Duration::HALF.ticks());
                states
                    .iter()
                    .map(|(state, _, _)| {
                        (f64::from(self.model.to_stop(state)) - beats - 1.0).max(0.0)
                    })
                    .fold(f64::INFINITY, f64::min)
            });
        let entry = self.entries[k]
            .iter()
            .find(|&&(e, _)| e >= end && e - end < self.horizon)
            .map(|&(e, y)| {
                let caps = &self.caps[k];
                let beats = f64::from(caps.at(end).saturating_sub(caps.at(e)));
                // A state that can no longer reach the syllable lags behind by the whole text;
                // the milestone cuts it before it gets here.
                let never = self.model.syllables() as u32;
                states
                    .iter()
                    .map(|(state, _, _)| {
                        let lacking = self.model.to_reach(state, y).unwrap_or(never);
                        (f64::from(lacking) - s.behind * beats).max(0.0)
                    })
                    .fold(f64::INFINITY, f64::min)
            });
        Steer { pace, goal, entry }
    }

    /// The steering of all new voices from their terms, to add to the estimate of the cost still
    /// to come. The term of a voice that `changed` names replaces its own.
    fn steer(&self, terms: &[Steer], changed: Option<(usize, Steer)>) -> f64 {
        let mut sum = 0.0;
        for (k, &term) in terms.iter().enumerate() {
            let term = match changed {
                Some((v, new)) if v == k => new,
                _ => term,
            };
            sum += term.pace;
            if let Some(goal) = term.goal {
                sum += goal;
            }
            if let Some(entry) = term.entry {
                sum += entry;
            }
        }
        self.steering.weight * sum
    }

    /// Lays the text under the notes of every new voice: the cheapest underlay that keeps the
    /// voice's pins and the gates, which the dynamic program finds. Returns what the underlays
    /// cost together; `None` if a voice has none, which the search rules out.
    fn lay(&self, notes: &mut [Vec<Note>]) -> Option<u32> {
        let mut cost = 0;
        for (k, line) in notes.iter_mut().enumerate() {
            let path = self
                .model
                .best_pinned(&self.levels, line, self.pins(k), self.gates(k))?;
            path.apply(line);
            // The checker finds the underlay's cases as the dynamic program priced them.
            debug_assert_eq!(
                self.levels.price(&self.model.cases(line, true)),
                (path.cost, None),
                "the underlay's cases differ from its path"
            );
            cost += path.cost;
        }
        Some(cost)
    }
}

/// What the violations of the TXT rules in a report cost.
fn text_cost(report: &crate::rules::Report) -> u32 {
    report
        .violations
        .iter()
        .filter(|v| v.rule.starts_with("TXT-"))
        .map(|v| v.level.cost())
        .sum()
}

/// The rules that forbid a candidate, each once.
fn forbidden_rules(violations: &[crate::rules::Violation]) -> Vec<&'static str> {
    let mut rules: Vec<&'static str> = violations
        .iter()
        .filter(|v| v.is_forbidden())
        .map(|v| v.rule)
        .collect();
    rules.sort_unstable();
    rules.dedup();
    rules
}

/// What to set at a tick in a new voice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Choice {
    Rest(Duration),
    Pitch(Pitch, Duration),
    /// The note before, held over the barline by a tie.
    Tie(Duration),
}

/// The rhythm of a new voice: the measure patterns its species allows (`crate::rhythm`), without
/// those the profile forbids. The voice follows them note by note; whether a note is tied into the
/// next measure is decided at the next downbeat.
struct Rhythm {
    measures: usize,
    /// The measures in which internal cadences arrive.
    goals: Vec<usize>,
    /// Per place, the allowed patterns with their tokens.
    allowed: Vec<(Place, Vec<Pattern>)>,
}

/// A measure pattern with its tokens and what it costs under the profile.
type Pattern = (&'static str, Vec<Token>, u32);

impl Rhythm {
    fn new(
        species: Species,
        many: bool,
        measures: usize,
        goals: Vec<usize>,
        checker: &Checker,
    ) -> Rhythm {
        let rule = rhythm::rule_id(species);
        let places = [
            Place::Entry,
            Place::Inner,
            Place::Antepenultimate,
            Place::Penultimate,
            Place::Last,
        ];
        let allowed = places
            .into_iter()
            .map(|place| {
                let level = |param: &Option<&str>| match (rule, param) {
                    (Some(rule), Some(param)) => checker.level(rule, param),
                    _ => Level::Off,
                };
                let patterns = rhythm::patterns(species, place, many)
                    .iter()
                    .filter(|(_, param)| level(param) != Level::Forbidden)
                    .map(|(pattern, param)| {
                        (*pattern, rhythm::tokens(pattern), level(param).cost())
                    })
                    .collect();
                (place, patterns)
            })
            .collect();
        Rhythm {
            measures,
            goals,
            allowed,
        }
    }

    fn patterns(&self, measure: usize) -> &[Pattern] {
        let place = Place::of(measure, self.measures, Some(0), &self.goals);
        self.allowed
            .iter()
            .find(|(p, _)| *p == place)
            .map_or(&[], |(_, patterns)| patterns)
    }

    /// The ticks at which the voice may begin a note.
    fn onsets(&self, per_measure: u32) -> Vec<u32> {
        let mut ticks = Vec::new();
        for m in 0..self.measures {
            for (_, tokens, _) in self.patterns(m) {
                let mut at = m as u32 * per_measure;
                for token in tokens {
                    ticks.push(at);
                    at += token.ticks;
                }
            }
        }
        ticks
    }

    /// The least the rhythm of the measure the voice's notes end in can still cost, as far as
    /// they go: the rule charges it only once the measure is complete.
    fn prefix_cost(&self, notes: &[Note], per_measure: u32) -> u32 {
        let end: u32 = notes.iter().map(|n| n.duration.ticks()).sum();
        if end.is_multiple_of(per_measure) {
            return 0;
        }
        let measure_start = end / per_measure * per_measure;
        let mut prefix = Vec::new();
        let (mut start, mut tied) = (0, false);
        for note in notes {
            if start >= measure_start {
                prefix.push(Token::of(note, start == measure_start && tied));
            }
            tied = note.tie;
            start += note.duration.ticks();
        }
        self.patterns((end / per_measure) as usize)
            .iter()
            .filter(|(_, tokens, _)| tokens.starts_with(&prefix))
            .map(|(_, _, cost)| *cost)
            .min()
            .unwrap_or(0)
    }

    /// What may begin at `tick` in a voice whose notes end there. With `end_tie` the measure has
    /// to end with a note tied into the next one (`Some(true)`) or without (`Some(false)`), as
    /// a fixed note at the next downbeat demands.
    fn choices(
        &self,
        notes: &[Note],
        tick: u32,
        per_measure: u32,
        pool: &[Pitch],
        end_tie: Option<bool>,
    ) -> Vec<Choice> {
        let measure = (tick / per_measure) as usize;
        let measure_start = measure as u32 * per_measure;
        // The tokens of the measure so far, and the notes of the previous measure.
        let mut prefix = Vec::new();
        let mut before: Vec<&Note> = Vec::new();
        let (mut start, mut tied, mut tied_into_before) = (0, false, false);
        for note in notes {
            if start >= measure_start {
                prefix.push(Token::of(note, start == measure_start && tied));
            } else if start + per_measure >= measure_start {
                if before.is_empty() {
                    tied_into_before = tied;
                }
                before.push(note);
            }
            tied = note.tie;
            start += note.duration.ticks();
        }
        let mut tokens: Vec<Token> = Vec::new();
        for (name, pattern, _) in self.patterns(measure) {
            if end_tie.is_some_and(|tie| name.ends_with('⁀') != tie) {
                continue;
            }
            if pattern.len() > prefix.len() && pattern[..prefix.len()] == prefix[..] {
                let token = pattern[prefix.len()];
                if !tokens.contains(&token) {
                    tokens.push(token);
                }
            }
        }
        // At a downbeat the choice also settles whether the previous measure ends tied.
        let closes = |tie: bool| {
            if measure == 0 || tick != measure_start {
                return true;
            }
            let Some((last, rest)) = before.split_last() else {
                return false;
            };
            let last = Note {
                tie,
                ..(*last).clone()
            };
            let mut all = rest.to_vec();
            all.push(&last);
            let found = rhythm::measure_rhythm(&all, tied_into_before);
            self.patterns(measure - 1)
                .iter()
                .any(|(p, _, _)| *p == found)
        };
        let mut choices = Vec::new();
        for token in tokens {
            let duration = Duration::from_ticks(token.ticks);
            if token.tied_in {
                if notes.last().is_some_and(|n| !n.is_rest()) && closes(true) {
                    choices.push(Choice::Tie(duration));
                }
            } else if !closes(false) {
                continue;
            } else if token.rest {
                choices.push(Choice::Rest(duration));
            } else {
                choices.extend(pool.iter().map(|p| Choice::Pitch(*p, duration)));
            }
        }
        choices
    }
}

/// Whether `pitch` belongs to the sonority where a cadence on `target` (spelled with `alter`)
/// arrives: the target in any octave, its fifth or its major third, at an internal goal (`last`
/// unset) also its minor third; the lowest voice ends (`last`) on the target itself. The
/// interval counts up from the target at or below the pitch: E3 lies a fifth above A2, not a
/// fourth below A3.
fn goal_tone(target: Step, alter: i8, pitch: Pitch, lowest: bool, last: bool) -> bool {
    let mut from = Pitch::new(target, alter, pitch.octave);
    if from.diatonic() > pitch.diatonic() {
        from = Pitch::new(target, alter, pitch.octave - 1);
    }
    let above = Interval::between(from, pitch).simple();
    match (above.steps, above.semitones) {
        (0, 0) => true,
        _ if last && lowest => false,
        (4, 7) | (2, 4) => true,
        (2, 3) => !last,
        _ => false,
    }
}

/// Whether a voice whose note `last` leads into the final measure can still end: on a tone of
/// the triad of the setting's ending `ending` from its pool (the bass on the ending itself), by
/// an interval it may sing, and without parallel unisons, fifths or octaves against a given
/// voice. The search looks one note ahead here, because the final sonority binds every voice at
/// once.
fn can_close(
    base: &Score,
    pool: &[Pitch],
    last: Pitch,
    bass: bool,
    final_start: u32,
    ending: Step,
) -> bool {
    let mode = base.mode;
    let given: Vec<(Pitch, Pitch)> = base
        .voices
        .iter()
        .filter_map(|v| {
            let line = v.soundings();
            let end = line.iter().find(|s| s.start == final_start)?;
            let before = line.iter().find(|s| s.end == final_start)?;
            Some((before.pitch, end.pitch))
        })
        .collect();
    pool.iter().any(|&end| {
        if !goal_tone(ending, mode.scale_alter(ending), end, bass, true) {
            return false;
        }
        let leap = Interval::between(last, end);
        let up = leap.abs();
        let singable = leap.is_regular()
            && !leap.is_tritone()
            && (up.steps <= 4
                || (up.steps == 5 && up.semitones == 8)
                || (up.steps == 7 && up.semitones == 12));
        singable
            && given.iter().all(|&(before, after)| {
                let (x, y) = (
                    Interval::between(before, last),
                    Interval::between(after, end),
                );
                let perfect = |i: Interval| i.is_perfect_consonance();
                let parallel = perfect(x) && perfect(y) && x.simple().steps == y.simple().steps;
                let moved = (end.midi() - last.midi()).signum()
                    == (after.midi() - before.midi()).signum()
                    && end != last;
                !(parallel && moved)
            })
    })
}

/// The pitches of the given voices sounding just before `tick` and at `tick`.
fn given_moves(base: &Score, tick: u32) -> Vec<(Pitch, Pitch)> {
    base.voices
        .iter()
        .filter_map(|v| {
            let line = v.soundings();
            let at = line.iter().find(|s| s.start <= tick && tick < s.end)?;
            let before = line.iter().find(|s| s.start < tick && tick <= s.end)?;
            Some((before.pitch, at.pitch))
        })
        .collect()
}

/// Whether a voice may move from `a` to `b`: an interval it may sing.
fn singable(a: Pitch, b: Pitch) -> bool {
    let leap = Interval::between(a, b);
    let up = leap.abs();
    leap.is_regular()
        && !leap.is_tritone()
        && (up.steps <= 4
            || (up.steps == 5 && up.semitones == 8)
            || (up.steps == 7 && up.semitones == 12))
}

/// Whether moving from `a` to `b` makes parallel unisons, fifths or octaves with a given voice
/// that moves as in `moves`.
fn parallel_to_given(a: Pitch, b: Pitch, moves: &[(Pitch, Pitch)]) -> bool {
    moves.iter().any(|&(before, after)| {
        let (x, y) = (Interval::between(before, a), Interval::between(after, b));
        let perfect = |i: Interval| i.is_perfect_consonance();
        let parallel = perfect(x) && perfect(y) && x.simple().steps == y.simple().steps;
        let moved =
            (b.midi() - a.midi()).signum() == (after.midi() - before.midi()).signum() && b != a;
        parallel && moved
    })
}

/// Whether the bass, whose note `last` ends at the penultimate downbeat of a cadence arriving at
/// `goal` on `target`, can still bring the cadence: a note on a degree of the cadence (its fifth
/// or second; before a Phrygian clausula its seventh or second), then the target, singable and
/// without parallels against the given voices. Holding the target over the barline for a delayed
/// leading tone is left to the rules.
fn bass_reaches_cadence(
    base: &Score,
    pool: &[Pitch],
    last: Pitch,
    goal: u32,
    target: Step,
    per_measure: u32,
) -> bool {
    let mode = base.mode;
    let Some(clausula) = mode.clausula(target) else {
        return true;
    };
    let target_in = |octave: i8| Pitch::new(target, mode.scale_alter(target), octave);
    if last.same_class(target_in(last.octave)) {
        return true;
    }
    let penultimate = goal - per_measure;
    let into_penultimate = given_moves(base, penultimate);
    let into_goal = given_moves(base, goal);
    pool.iter().any(|&x| {
        let above = (x.midi() - target_in(x.octave).midi()).rem_euclid(12);
        let degree = match clausula {
            Clausula::Leading => matches!(above, 2 | 7),
            Clausula::Phrygian => matches!(above, 1 | 10),
        };
        degree
            && singable(last, x)
            && !parallel_to_given(last, x, &into_penultimate)
            && pool.iter().any(|&y| {
                y.same_class(target_in(y.octave))
                    && singable(x, y)
                    && !parallel_to_given(x, y, &into_goal)
            })
    })
}

/// The melodic moves the profile forbids outright (MEL-MAX-LEAP, MEL-TRITONE, MEL-AUG-DIM): the
/// search leaves them out before any check, which would only find them forbidden.
#[derive(Clone, Copy, Debug)]
struct ForbiddenMoves {
    max_leap: bool,
    tritone: bool,
    aug_dim: bool,
    chromatic: bool,
}

impl ForbiddenMoves {
    fn new(checker: &Checker, many: bool) -> ForbiddenMoves {
        let forbidden = |rule, param| checker.level(rule, param) == Level::Forbidden;
        ForbiddenMoves {
            max_leap: forbidden("MEL-MAX-LEAP", "level"),
            tritone: forbidden("MEL-TRITONE", "level"),
            aug_dim: forbidden("MEL-AUG-DIM", "level"),
            chromatic: forbidden(
                "MEL-AUG-DIM",
                if many {
                    "chromatic"
                } else {
                    "chromatic_two_voices"
                },
            ),
        }
    }

    /// Whether the profile forbids moving from `from` to `to`.
    fn forbid(self, from: Pitch, to: Pitch) -> bool {
        let interval = Interval::between(from, to);
        let up = interval.abs();
        if interval.is_tritone() {
            self.tritone
        } else if !interval.is_regular() {
            if up.steps == 0 {
                self.chromatic
            } else {
                self.aug_dim
            }
        } else {
            self.max_leap
                && ((up.steps == 5 && up.semitones == 9) || up.steps == 6 || up.steps >= 8)
        }
    }
}

/// A note heard in a voice at some tick, ties merged: its pitch, where it began, and whether it
/// strikes the pitch of the note before again.
#[derive(Clone, Copy, Debug)]
struct Heard {
    pitch: Pitch,
    start: u32,
    repeated: bool,
}

/// What a voice with these written notes sounds at `tick`, if its notes reach that far.
fn heard_at(notes: &[Note], tick: u32) -> Option<Heard> {
    let mut at = 0;
    // The sounding the last note belongs to, and the one before it if they touch.
    let mut current: Option<(Pitch, u32)> = None;
    let mut before: Option<(Pitch, u32)> = None;
    let mut tied = false;
    for note in notes {
        let end = at + note.duration.ticks();
        match note.pitch {
            None => {
                current = None;
                before = None;
                tied = false;
            }
            Some(pitch) => {
                if !(tied && current.is_some_and(|(p, _)| p == pitch)) {
                    before = current.map(|(p, _)| (p, at));
                    current = Some((pitch, at));
                }
                tied = note.tie;
            }
        }
        if at <= tick && tick < end {
            return current.map(|(pitch, start)| Heard {
                pitch,
                start,
                repeated: before.is_some_and(|(p, end)| p == pitch && end == start),
            });
        }
        at = end;
    }
    None
}

/// The interval between two pitches, upwards from the lower one, as the rules measure it.
fn between(a: Pitch, b: Pitch) -> Interval {
    if (a.diatonic(), a.midi()) <= (b.diatonic(), b.midi()) {
        Interval::between(a, b)
    } else {
        Interval::between(b, a)
    }
}

/// Two prohibitions the search can see in a candidate before the full check, from the notes
/// alone: a dissonance struck against the bass on the downbeat (HAR-CONSONANCE) and parallel
/// unisons, fifths or octaves (TEX-PARALLEL). Each is judged exactly as the rule judges it while
/// the voices above the candidate are not yet set, so the search finds the same; it only saves
/// the check. For free counterpoint, whose voices are all new and in score order.
#[derive(Clone, Copy, Debug)]
struct QuickChecks {
    consonance: bool,
    parallels: bool,
}

impl QuickChecks {
    fn new(checker: &Checker, voices: usize) -> QuickChecks {
        QuickChecks {
            consonance: checker.level("HAR-CONSONANCE", "level") == Level::Forbidden,
            parallels: voices >= 3 && checker.level("TEX-PARALLEL", "level") == Level::Forbidden,
        }
    }

    /// The rule that forbids pitch `pitch` struck in voice `k` at `tick`, if one of the two does.
    fn rejects(
        self,
        notes: &[Vec<Note>],
        k: usize,
        pitch: Pitch,
        tick: u32,
        per_measure: u32,
    ) -> Option<&'static str> {
        let n = notes.len();
        let before_k = heard_at(&notes[k], tick.saturating_sub(1)).filter(|_| tick > 0);
        let candidate = Heard {
            pitch,
            start: tick,
            repeated: before_k.is_some_and(|h| h.pitch == pitch && h.start < tick),
        };
        // The other voices that sound at `tick` already: set before this one, or held.
        let known: Vec<(usize, Heard)> = (0..n)
            .filter(|&j| j != k)
            .filter_map(|j| heard_at(&notes[j], tick).map(|h| (j, h)))
            .collect();
        if self.consonance && tick.is_multiple_of(per_measure) {
            let struck = |h: &Heard| h.start == tick && !h.repeated;
            if n == 2 {
                if let Some((_, other)) = known.first()
                    && struck(&candidate)
                    && struck(other)
                    && !between(pitch, other.pitch).is_consonant()
                {
                    return Some("HAR-CONSONANCE");
                }
            } else {
                // The lowest of them is the bass; on equal pitch the lower voice in the score.
                let mut all = known.clone();
                all.push((k, candidate));
                let key = |&(j, h): &(usize, Heard)| (h.pitch.midi(), h.pitch.diatonic(), n - j);
                if let Some(&(bass, b)) = all.iter().min_by_key(|x| key(x)) {
                    let clash = |h: &Heard| {
                        struck(h) && struck(&b) && !between(h.pitch, b.pitch).is_consonant()
                    };
                    let forbidden = if bass == k {
                        known.iter().any(|(_, h)| clash(h))
                    } else {
                        clash(&candidate)
                    };
                    if forbidden {
                        return Some("HAR-CONSONANCE");
                    }
                }
            }
        }
        if self.parallels {
            // The slice before: the last onset of any voice before `tick`.
            let previous = notes
                .iter()
                .filter_map(|line| {
                    let mut at = 0;
                    let mut last = None;
                    for note in line {
                        if at >= tick {
                            break;
                        }
                        last = Some(at);
                        at += note.duration.ticks();
                    }
                    last
                })
                .max()?;
            let a1 = heard_at(&notes[k], previous)?;
            for &(j, b2) in &known {
                let Some(b1) = heard_at(&notes[j], previous) else {
                    continue;
                };
                let (da, db) = (
                    pitch.midi() - a1.pitch.midi(),
                    b2.pitch.midi() - b1.pitch.midi(),
                );
                let similar = da != 0 && db != 0 && (da > 0) == (db > 0);
                let (x, y) = (between(a1.pitch, b1.pitch), between(pitch, b2.pitch));
                if similar
                    && x.is_perfect_consonance()
                    && y.is_perfect_consonance()
                    && x.simple().steps == y.simple().steps
                {
                    return Some("TEX-PARALLEL");
                }
            }
        }
        None
    }
}

/// The value of a choice.
fn choice_duration(choice: Choice) -> Duration {
    match choice {
        Choice::Rest(d) | Choice::Pitch(_, d) | Choice::Tie(d) => d,
    }
}

/// The choices at `tick` that agree with the fixed notes of a voice: where a fixed note begins
/// only that note (in the octaves the voice may sing, if they are free), if the rhythm allows its
/// value there; before one nothing that would run into it, and a note that ends at a fixed tie
/// only with its pitch. Before an entry after a rest, notes end before its rest window and rests
/// at the entry; in the window the only choice is the rest up to the entry or the next barline,
/// so that no shorter rest leads into a measure that cannot reach the entry.
fn agree(
    choices: Vec<Choice>,
    fixed: &[&Fixed],
    notes: &[Note],
    tick: u32,
    per_measure: u32,
) -> Vec<Choice> {
    let last_pitch = notes.last().and_then(|n| n.pitch);
    if let Some(f) = fixed.iter().find(|f| f.start == tick) {
        let duration = f.note.duration;
        if f.tied_in {
            let tie = Choice::Tie(duration);
            return if choices.contains(&tie) && f.matches(last_pitch) {
                vec![tie]
            } else {
                Vec::new()
            };
        }
        return choices
            .into_iter()
            .filter(|c| match *c {
                Choice::Pitch(p, d) => d == duration && f.matches(Some(p)),
                Choice::Rest(d) => d == duration && f.note.pitch.is_none(),
                Choice::Tie(_) => false,
            })
            .collect();
    }
    let Some(next) = fixed.iter().find(|f| f.start > tick) else {
        return choices;
    };
    if let Some((window, _)) = next.rest_window() {
        if tick >= window {
            let barline = (tick / per_measure + 1) * per_measure;
            let rest = Choice::Rest(Duration::from_ticks(next.start.min(barline) - tick));
            return if choices.contains(&rest) {
                vec![rest]
            } else {
                Vec::new()
            };
        }
        return choices
            .into_iter()
            .filter(|c| match *c {
                Choice::Rest(d) => tick + d.ticks() <= next.start,
                Choice::Pitch(_, d) | Choice::Tie(d) => tick + d.ticks() <= window,
            })
            .collect();
    }
    let room = next.start - tick;
    choices
        .into_iter()
        .filter(|c| {
            let ticks = choice_duration(*c).ticks();
            if ticks > room {
                return false;
            }
            if ticks < room || !next.tied_in {
                return true;
            }
            match c {
                Choice::Pitch(p, _) => next.matches(Some(*p)),
                Choice::Tie(_) => next.matches(last_pitch),
                Choice::Rest(_) => false,
            }
        })
        .collect()
}

/// Appends the chosen note to a voice.
fn apply(notes: &mut Vec<Note>, choice: Choice, mode: ModeSpec) {
    let note = match choice {
        Choice::Rest(duration) => Note::rest(duration),
        Choice::Pitch(pitch, duration) => {
            Note::new(pitch, duration).with_ficta(!mode.is_scale_pitch(pitch))
        }
        Choice::Tie(duration) => {
            let held = notes.last_mut().expect("a note to hold");
            held.tie = true;
            Note {
                duration,
                tie: false,
                syllable: None,
                lyric: None,
                ..held.clone()
            }
        }
    };
    notes.push(note);
}

#[derive(Clone)]
struct State {
    /// Notes per new voice.
    notes: Vec<Vec<Note>>,
    /// Cost of the violations decided before the current tick.
    base: u32,
    cost: u32,
    /// Cost plus accumulated noise; the beam keeps the lowest keys (plus `ahead`).
    key: f64,
    /// Estimated cost still to come: the least the measures begun can cost by their rhythm
    /// ([`Rhythm::prefix_cost`]) and the way to the next fixed notes ([`distance_to_fixed`]).
    ahead: f64,
    /// Whether `cost` includes the violations decided at the current tick so far.
    checked: bool,
    /// Whether fixed notes begin at the current tick, so that its window needs a check even if
    /// no voice chooses a note there.
    forced: bool,
    /// The state of the beam at the last downbeat this one descends from.
    lineage: usize,
    /// With a text, per new voice the underlay states its notes allow, shared with the states
    /// that descend from this one; empty without a text.
    text: Vec<Arc<Set>>,
    /// The least cost of the text in each new voice, summed.
    text_cost: u32,
    /// With a text, per new voice its steering by its notes and underlay states; empty without.
    steer: Vec<Steer>,
}

/// A step of a voice's underlay states over its new note: the new states, or the rule that left
/// none.
type Advanced = Result<Arc<Set>, &'static str>;

/// The steps of a voice's underlay states at one tick, each taken once for all candidates that
/// share it: the same choice after the same set of states. Every new note of a voice steps its
/// set, so states share a set only where they took the same steps from the voice's first note
/// on: the voice has sung the same notes, and the new note makes the same event for each of
/// them. The first candidate that needs a step takes it; the others wait for it.
struct Steps {
    /// The slot of each set, by its address (which stays the set's while the beam holds it),
    /// and choice.
    index: HashMap<(usize, Choice), usize, BuildHasherDefault<StepHasher>>,
    slots: Vec<OnceLock<Advanced>>,
}

/// A quick hash for the keys of [`Steps`], addresses and choices, which nobody chooses to make
/// collide (after the multiplicative hash of rustc's `FxHasher`).
#[derive(Default)]
struct StepHasher(u64);

impl Hasher for StepHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_u64(u64::from(byte));
        }
    }

    fn write_u8(&mut self, n: u8) {
        self.write_u64(u64::from(n));
    }

    fn write_u32(&mut self, n: u32) {
        self.write_u64(u64::from(n));
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }

    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }

    fn write_isize(&mut self, n: isize) {
        self.write_u64(n as u64);
    }
}

impl Steps {
    fn new<'a>(keys: impl Iterator<Item = (&'a Arc<Set>, Choice)>) -> Steps {
        let mut index = HashMap::default();
        for (set, choice) in keys {
            let next = index.len();
            index
                .entry((Arc::as_ptr(set).addr(), choice))
                .or_insert(next);
        }
        let slots = (0..index.len()).map(|_| OnceLock::new()).collect();
        Steps { index, slots }
    }

    /// The step of `set` with `choice`, which `step` takes if no candidate has taken it yet.
    fn take(&self, set: &Arc<Set>, choice: Choice, step: impl FnOnce() -> Advanced) -> &Advanced {
        self.slots[self.index[&(Arc::as_ptr(set).addr(), choice)]].get_or_init(step)
    }
}

/// The least text cost of each voice's underlay states, summed: each set has its cheapest state
/// first.
fn text_cost_of(sets: &[Arc<Set>]) -> u32 {
    sets.iter().map(|set| set[0].1).sum()
}

/// The rules that cut off the most candidates, at most three, most frequent first.
fn most_frequent(blocked: BTreeMap<&'static str, usize>) -> Vec<&'static str> {
    let mut rules: Vec<(&'static str, usize)> = blocked.into_iter().collect();
    rules.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    rules.into_iter().take(3).map(|(rule, _)| rule).collect()
}

/// In free counterpoint the beam keeps at most this share of its states from one state of the
/// beam at the last downbeat, so that it does not narrow to one history that may lead nowhere.
const LINEAGE_SHARE: usize = 8;

/// How often a search for free counterpoint goes back to an earlier downbeat after a dead end.
const MAX_RESTARTS: u32 = 12;

/// How many downbeats back the search for free counterpoint can go.
const CHECKPOINTS: usize = 4;

/// The rule the diagnosis names where a voice cannot take a note of an entry's head.
const ENTRY_RULE: &str = "IMI-ENTRY";

/// The rule the diagnosis names where a voice cannot rest in the window before an entry.
const REST_RULE: &str = "IMI-REST";

/// The weight of [`clash_with_fixed`] in the ranking.
const CLASH: f64 = 1.0;

/// How far the voices are from their next formula notes, in steps beyond what they can cover by
/// thirds in the time left: an estimate of the leaps still to come. The beam adds it to the
/// ranking so that the voices head for their formulas in time. An entry's head needs no way
/// there: the voice reaches it after a rest.
fn distance_to_fixed(notes: &[Vec<Note>], fixed: &[Vec<&Fixed>]) -> f64 {
    let mut steps = 0;
    for (line, fixed) in notes.iter().zip(fixed) {
        let end: u32 = line.iter().map(|n| n.duration.ticks()).sum();
        let Some(last) = line.iter().rev().find_map(|n| n.pitch) else {
            continue;
        };
        let Some((start, target, any_octave)) = fixed
            .iter()
            .filter(|f| f.start >= end && !f.in_head())
            .find_map(|f| Some((f.start, f.note.pitch?, f.any_octave)))
        else {
            continue;
        };
        let mut distance = (target.diatonic() - last.diatonic()).abs();
        if any_octave {
            distance = distance.rem_euclid(7).min(7 - distance.rem_euclid(7));
        }
        let notes_left = ((start - end) / Duration::QUARTER.ticks() + 1) as i32;
        steps += (distance - 2 * notes_left).max(0);
    }
    f64::from(steps)
}

/// The attack of a note of an entry's head that lies ahead, as [`clash_with_fixed`] weighs it.
#[derive(Clone, Copy, Debug)]
struct HeadAttack {
    voice: usize,
    tick: u32,
    pitch: Pitch,
    /// The lowest voice that may sound there, if it is a new one: a voice is silent before its
    /// first entry.
    lowest: Option<usize>,
}

/// How many attacks of entries' heads in other voices that lie ahead (`upcoming`, within the
/// next measure) the current notes of the voices will still sound against with a dissonance: a
/// second, a seventh, a tritone or another altered interval, or a fourth whose lower note lies
/// in the lowest voice that may sound there. The beam adds it to the ranking ([`CLASH`]), so
/// that a held note that would clash with an entry ranks lower. It is a cost, not a cut: most of
/// Palestrina's entries are struck against a held note, nearly all of them consonantly.
fn clash_with_fixed(notes: &[Vec<Note>], upcoming: &[HeadAttack]) -> f64 {
    let mut clashes = 0;
    for (j, line) in notes.iter().enumerate() {
        let Some(pitch) = line.last().and_then(|n| n.pitch) else {
            continue;
        };
        let end: u32 = line.iter().map(|n| n.duration.ticks()).sum();
        for attack in upcoming.iter().filter(|a| a.voice != j && a.tick < end) {
            let interval = between(pitch, attack.pitch);
            if interval.is_consonant() {
                continue;
            }
            let lower = if (pitch.diatonic(), pitch.midi())
                <= (attack.pitch.diatonic(), attack.pitch.midi())
            {
                j
            } else {
                attack.voice
            };
            if !interval.is_perfect_fourth() || attack.lowest == Some(lower) {
                clashes += 1;
            }
        }
    }
    f64::from(clashes)
}

/// Where the voices of the result come from.
#[derive(Clone, Copy)]
enum Part {
    Given(usize),
    New(usize),
}

/// Generates counterpoint. Every result breaks no prohibition of the profile in the new voices;
/// violations of the given voices do not count. If the search finds nothing, it tries again with
/// twice and four times the beam (as far as [`CounterpointRequest::widen`] allows) before it
/// gives up. A search cancelled through the request's control, at any step or before a wider
/// beam, fails with [`GenerateError::Cancelled`].
pub fn generate_counterpoint(
    request: &CounterpointRequest,
    profile: &Profile,
) -> Result<Vec<Generated>, GenerateError> {
    // A cantus that rises into a cadence by a whole step gets the leading tone.
    let mut base = request.base.clone();
    base.raise_cantus_leading_tones();
    let request = &CounterpointRequest {
        base,
        ..request.clone()
    };
    let sung;
    let profile = if request.text.is_some() {
        sung = sung_profile(profile);
        &sung
    } else {
        profile
    };
    let mut result = search(request, profile);
    for factor in [2, 4] {
        if factor > request.widen.min(MAX_WIDENING)
            || !matches!(result, Err(GenerateError::NoCounterpoint { .. }))
        {
            break;
        }
        if cancelled(request.control.as_deref()) {
            return Err(GenerateError::Cancelled);
        }
        let wider = CounterpointRequest {
            beam: request.beam.max(1) * factor,
            ..request.clone()
        };
        result = search(&wider, profile);
    }
    result
}

/// The profile for the search of a texted request. The scores of the search carry no text yet:
/// their free voices repeat a white note under MEL-REPEAT `free` and rest under FREE-REST
/// `level`, the voices of the results, which sing, under `free_texted` and `texted`. With the
/// level of the one for the other the search pays what the results cost; `free_black` and
/// `free_run` hold for both. Applied twice, it gives the same profile.
pub(crate) fn sung_profile(profile: &Profile) -> Profile {
    let mut sung = profile.clone();
    for (rule, plain, texted) in [
        ("MEL-REPEAT", "free", "free_texted"),
        ("FREE-REST", "level", "texted"),
    ] {
        let level = profile.settings(rule).level(texted);
        sung.apply_override(&format!("{rule}.{plain}={}", level.label(Lang::De)))
            .expect("a level of the profile");
    }
    sung
}

/// One search with the request's beam.
///
/// The search goes through the ticks at which a new voice may begin a note and keeps the `beam`
/// cheapest partial solutions after each. At a tick every new voice whose last note ends there
/// gets a note that fits the rhythm patterns of its species. A step counts the violations
/// decided since the previous step; the last step checks the whole score.
fn search(
    request: &CounterpointRequest,
    profile: &Profile,
) -> Result<Vec<Generated>, GenerateError> {
    let base = &request.base;
    // Free counterpoint has no given voice: the request names the length.
    let free = base.voices.is_empty();
    if free {
        if request.measures.is_none_or(|m| m < 2) {
            return Err(GenerateError::Invalid(
                "free counterpoint needs two measures or more".into(),
            ));
        }
        if request.voices.iter().any(|v| v.species != Species::Free) {
            return Err(GenerateError::Invalid(
                "without a cantus firmus every voice is free".into(),
            ));
        }
    } else {
        base.validate()
            .map_err(|e| GenerateError::Invalid(e.to_string()))?;
        if base.cantus_firmus().is_none() {
            return Err(GenerateError::Invalid("no cantus firmus".into()));
        }
    }
    if base.meter != Meter::ALLA_BREVE {
        return Err(GenerateError::Invalid(format!(
            "counterpoint is generated in 2/2, not {}",
            base.meter
        )));
    }
    if let Some(f) = request
        .fixed
        .iter()
        .find(|f| f.voice >= request.voices.len())
    {
        return Err(GenerateError::Invalid(format!(
            "a fixed note for voice {}, but there are {} new voices",
            f.voice,
            request.voices.len()
        )));
    }
    if let Some(sung) = &request.text {
        if sung.text.syllables.is_empty() {
            return Err(GenerateError::EmptyText);
        }
        if base
            .text
            .as_ref()
            .is_some_and(|given| given.text != sung.text)
        {
            return Err(GenerateError::Invalid(
                "the given voices sing another text".into(),
            ));
        }
    }
    let measures = if free {
        request.measures.unwrap_or(0)
    } else {
        base.measure_count()
    };
    let per_measure = base.meter.ticks_per_measure();
    let total = measures as u32 * per_measure;
    let fixed = fixed_by_voice(request);
    let parts = order_voices(base, &request.voices);
    let targets: Vec<bool> = parts.iter().map(|s| matches!(s, Part::New(_))).collect();
    let targets_of_cadences: Vec<Step> = base.cadences.iter().map(|c| c.target).collect();
    // The target of the final cadence: the final of the mode, or the ending of a part.
    let ending = request.ending.unwrap_or_else(|| base.mode.final_step());
    let pools: Vec<Vec<Pitch>> = request
        .voices
        .iter()
        .map(|v| pitch_pool_ending(base.mode, v.voice_type, &targets_of_cadences, ending))
        .collect();
    // The heads of the entries are exact: a voice may sing a head's pitch at its attack also
    // where its pool lacks it, and only there, so that the free counterpoint keeps the pool.
    let head_pitches: Vec<Vec<(u32, Pitch)>> = fixed
        .iter()
        .zip(&pools)
        .map(|(list, pool)| {
            list.iter()
                .filter(|f| f.in_head() && !f.tied_in)
                .filter_map(|f| Some((f.start, f.note.pitch?)))
                .filter(|(_, pitch)| !pool.contains(pitch))
                .collect()
        })
        .collect();
    // Where each new voice enters, if its heads begin with an entry: it is silent before. And
    // where the rest windows before its later entries begin.
    let first_entries: Vec<Option<u32>> = fixed.iter().map(|list| first_entry(list)).collect();
    // A fixed note before a voice's first entry would never sound: the voice is silent there.
    for (k, list) in fixed.iter().enumerate() {
        if let Some(entry) = first_entries[k]
            && let Some(f) = list.iter().find(|f| f.start < entry)
        {
            return Err(GenerateError::Invalid(format!(
                "a fixed note of voice {k} at tick {} lies before its first entry at tick {entry}",
                f.start
            )));
        }
    }
    let windows: Vec<Vec<u32>> = fixed
        .iter()
        .map(|list| {
            list.iter()
                .filter_map(|f| f.rest_window().map(|(start, _)| start))
                .collect()
        })
        .collect();
    let checker = Checker::new(profile);
    // With a text, every new voice carries its underlay states through the search.
    let underlay = request
        .text
        .as_ref()
        .map(|sung| Underlay::new(sung, base, &fixed, total, &checker));
    let many = parts.len() >= 3;
    let goals = base.cadence_goals();
    let rhythms: Vec<Rhythm> = request
        .voices
        .iter()
        .map(|v| Rhythm::new(v.species, many, measures, goals.clone(), &checker))
        .collect();
    let moves = ForbiddenMoves::new(&checker, many);
    let quick = QuickChecks::new(&checker, parts.len());
    let any_free = request.voices.iter().any(|v| v.species == Species::Free);
    let mut ticks: Vec<u32> = rhythms.iter().flat_map(|r| r.onsets(per_measure)).collect();
    ticks.extend(request.fixed.iter().map(|f| f.start));
    ticks.retain(|&t| t < total);
    ticks.sort_unstable();
    ticks.dedup();
    // The lowest voice of all, if it is a new one: it has to end on the final (or the part's
    // ending).
    let bottom = match parts.last() {
        Some(Part::New(k)) => Some(k),
        _ => None,
    };
    let final_start = total.saturating_sub(per_measure);
    // The ticks at which cadences arrive, with their targets: internal goals and the final
    // cadence on the ending.
    let mut cadence_ticks: Vec<u32> = goals.iter().map(|&m| m as u32 * per_measure).collect();
    cadence_ticks.push(final_start);
    let target_at = |tick: u32| -> Step {
        base.cadences
            .iter()
            .find(|c| c.measure as u32 * per_measure == tick)
            .map_or(ending, |c| c.target)
    };
    // In free counterpoint the semitone of each clausula belongs to the voice that sings it in
    // the formula: another voice singing its step in the second half of the penultimate measure
    // would double it (TEX-CLAUSULA, CAD-INNER-CLAUSULA) or clash with it (C against C♯,
    // HAR-CHROMATIC-CLASH).
    let semitones: Vec<(u32, u32, Pitch, usize)> = cadence_ticks
        .iter()
        .filter(|_| free)
        .filter_map(|&goal| {
            let target = target_at(goal);
            let semitone = base.mode.clausula_semitone(Pitch::new(
                target,
                base.mode.scale_alter(target),
                4,
            ))?;
            let owner = fixed.iter().position(|list| {
                list.iter().any(|f| {
                    !f.in_head()
                        && goal.saturating_sub(per_measure) <= f.start
                        && f.start < goal
                        && f.note.pitch.is_some_and(|p| p.same_class(semitone))
                })
            })?;
            Some((goal - per_measure / 2, goal, semitone, owner))
        })
        .collect();
    // A tone of the final sonority: octave, fifth or major third of the ending, in the lowest
    // voice the ending itself.
    let final_tone = |k: usize, pitch: Pitch| {
        let alter = base.mode.scale_alter(ending);
        goal_tone(ending, alter, pitch, Some(&k) == bottom, true)
    };
    // What voice k may set at a tick: its rhythm and its fixed notes. A free voice rests before it
    // enters in the first measure, and from three voices on also later (FREE-REST and
    // FREE-TEXTURE weigh it): before a cadence a voice without a part in it may fall silent, as
    // in many of Palestrina's, but every voice sings the last measure.
    let rests_until = if many {
        total.saturating_sub(per_measure)
    } else {
        0
    };
    let choices_for = |k: usize, notes: &[Note], tick: u32| -> Vec<Choice> {
        // A fixed note at the next downbeat decides whether this measure ends tied.
        let next_downbeat = (tick / per_measure + 1) * per_measure;
        let end_tie = fixed[k]
            .iter()
            .find(|f| f.start > tick)
            .filter(|f| f.start == next_downbeat)
            .map(|f| f.tied_in);
        let widened: Vec<Pitch>;
        let pool = match head_pitches[k].iter().find(|(at, _)| *at == tick) {
            Some(&(_, pitch)) => {
                let mut pool = pools[k].clone();
                pool.push(pitch);
                pool.sort_by_key(|p| (p.diatonic(), p.midi()));
                widened = pool;
                &widened
            }
            None => &pools[k],
        };
        let mut choices = rhythms[k].choices(notes, tick, per_measure, pool, end_tie);
        // Until its first entry the voice is silent: it rests up to the next barline or to the
        // entry.
        if let Some(entry) = first_entries[k]
            && tick < entry
        {
            let rest = Choice::Rest(Duration::from_ticks(entry.min(next_downbeat) - tick));
            choices.retain(|c| *c == rest);
            return choices;
        }
        if let Some(last) = notes.last().and_then(|n| n.pitch) {
            choices.retain(|c| !matches!(c, Choice::Pitch(p, _) if moves.forbid(last, *p)));
        }
        // The rules learn where a free score ends only when it is complete; the search knows
        // it: at the final downbeat every voice takes a tone of the final sonority and holds it.
        if free {
            let held = notes.last().and_then(|n| n.pitch);
            choices.retain(|c| {
                let (pitch, duration) = match *c {
                    Choice::Pitch(p, d) => (Some(p), d),
                    Choice::Tie(d) => (held, d),
                    Choice::Rest(d) => (None, d),
                };
                // The last measure is the final sonority, held (`1`, `⁀1`): a note moving
                // within it would be judged only in the complete score.
                if tick == final_start
                    && (!pitch.is_some_and(|p| final_tone(k, p)) || duration.ticks() != per_measure)
                {
                    return false;
                }
                let end = tick + duration.ticks();
                !semitones.iter().any(|&(from, goal, semitone, owner)| {
                    owner != k
                        && tick < goal
                        && end > from
                        && pitch.is_some_and(|p| p.step == semitone.step)
                })
            });
        }
        let fixed_here = fixed[k].iter().any(|f| f.start == tick);
        if request.voices[k].species == Species::Free && !fixed_here {
            let entered = notes.iter().any(|n| !n.is_rest());
            // A voice with a formula sings through the three measures before its goal: after a
            // rest it would have to enter on the formula's note just where that may be
            // dissonant.
            let formula_ahead = fixed[k]
                .iter()
                .any(|f| !f.in_head() && f.start > tick && f.start <= tick + 3 * per_measure);
            // Before an entry after a rest the voice may rest, in two voices too: it enters on
            // the entry's note, and its rest window demands a rest.
            let entry_ahead = !windows[k].is_empty()
                && fixed[k]
                    .iter()
                    .find(|f| f.start > tick)
                    .is_some_and(|f| f.rest_window().is_some());
            let may_rest = if entered {
                entry_ahead || (tick < rests_until && !formula_ahead)
            } else {
                tick < per_measure
            };
            if !may_rest {
                choices.retain(|c| !matches!(c, Choice::Rest(_)));
            }
        }
        agree(choices, &fixed[k], notes, tick, per_measure)
    };
    // Whether a note of an entry's head of voice k ends at `tick`: where the voice then has no
    // choice, the plan's head runs into the fixed note there, and the diagnosis names the entry.
    let head_ends_at = |k: usize, tick: u32| {
        fixed[k]
            .iter()
            .any(|f| f.in_head() && f.start + f.note.duration.ticks() == tick)
    };
    // Where the given voices begin notes: a step without these and without new notes adds nothing.
    let mut given: Vec<u32> = base
        .voices
        .iter()
        .flat_map(|v| v.timed().map(|(start, _)| start))
        .collect();
    given.sort_unstable();
    let mut rng = Rng::new(request.seed);
    // One shared set: every voice starts before its first syllable.
    let start = Arc::new(Automaton::start());
    let mut beam = vec![State {
        notes: vec![Vec::new(); request.voices.len()],
        base: 0,
        cost: 0,
        key: 0.0,
        ahead: 0.0,
        checked: false,
        forced: false,
        lineage: 0,
        text: match &underlay {
            Some(_) => vec![start.clone(); request.voices.len()],
            None => Vec::new(),
        },
        text_cost: 0,
        steer: match &underlay {
            Some(underlay) => (0..request.voices.len())
                .map(|k| underlay.term(k, &[], &start))
                .collect(),
            None => Vec::new(),
        },
    }];
    // The new voices of a tick are set one after the other from the bottom up, so that the bass
    // of a sonority is known before the voices above it: only then can their dissonances be
    // judged, and a voice in the species does not choose a note that no bass can carry.
    let order: Vec<usize> = parts
        .iter()
        .rev()
        .filter_map(|part| match part {
            Part::New(k) => Some(*k),
            Part::Given(_) => None,
        })
        .collect();
    // The attacks of entries' heads in the measure after `tick`, which the ranking weighs
    // against the notes held there (`clash_with_fixed`), each with the lowest voice that may
    // sound there: the lowest new voice that has entered, unless a given voice lies below.
    let heads_ahead = |tick: u32| -> Vec<HeadAttack> {
        let lowest = |at: u32| {
            parts
                .iter()
                .rev()
                .find_map(|part| match *part {
                    Part::Given(_) => Some(None),
                    Part::New(k) => first_entries[k]
                        .is_none_or(|entry| entry <= at)
                        .then_some(Some(k)),
                })
                .flatten()
        };
        request
            .fixed
            .iter()
            .filter(|f| {
                f.in_head() && !f.tied_in && tick < f.start && f.start <= tick + per_measure
            })
            .filter_map(|f| {
                Some(HeadAttack {
                    voice: f.voice,
                    tick: f.start,
                    pitch: f.note.pitch?,
                    lowest: lowest(f.start),
                })
            })
            .collect()
    };
    // Whether voice k, whose notes end at `tick` in the penultimate measure of a cadence or where
    // it arrives, has a next note there that no rule decided at that tick forbids; otherwise the
    // forbidding rules. Where the cadence arrives, the note must also belong to the goal's
    // sonority (at the end: octave, fifth or major third, the bass on the final). With a text the
    // note must also leave the voice an underlay: `set` holds the voice's underlay states. A
    // note's check ends at the first rule that forbids it, so the rules name only that one;
    // `exact` checks every rule, for the diagnosis of a dead end.
    let continues = |notes: &[Vec<Note>],
                     k: usize,
                     tick: u32,
                     set: Option<&Set>,
                     exact: bool|
     -> Result<(), Vec<&'static str>> {
        let goal = cadence_ticks.contains(&tick);
        let target = target_at(tick);
        let last = tick == final_start;
        let fits = |pitch: Pitch| {
            !goal
                || goal_tone(
                    target,
                    base.mode.scale_alter(target),
                    pitch,
                    Some(&k) == bottom,
                    last,
                )
        };
        let mut rules = Vec::new();
        let window = Window::partial(tick, tick);
        let choices = choices_for(k, &notes[k], tick);
        // A note of an entry's head, or the rest of its window, that the voice cannot take here,
        // or a fixed note that a head runs into: the diagnosis names the entry.
        if choices.is_empty() {
            if fixed[k].iter().any(|f| f.start == tick && f.in_head()) || head_ends_at(k, tick) {
                rules.push(ENTRY_RULE);
            } else if windows[k].contains(&tick) {
                rules.push(REST_RULE);
            }
        }
        for choice in choices {
            let pitch = match choice {
                Choice::Pitch(pitch, _) => Some(pitch),
                Choice::Tie(_) => notes[k].last().and_then(|n| n.pitch),
                Choice::Rest(_) => None,
            };
            if pitch.is_some_and(|p| !fits(p)) {
                continue;
            }
            let mut next = notes.to_vec();
            apply(&mut next[k], choice, base.mode);
            let score = assemble_owned(base, &parts, &request.voices, next);
            let analysis = Analysis::new(&score);
            let mut violations = Vec::new();
            // Only whether a rule forbids the note counts here, not what it costs.
            let forbidden = if exact {
                checker.check_window(&score, &analysis, window, &targets, &mut violations);
                violations.iter().any(|v| v.is_forbidden())
            } else {
                checker.check_window_until_forbidden(
                    &score,
                    &analysis,
                    window,
                    &targets,
                    &mut violations,
                    false,
                )
            };
            if forbidden {
                rules.extend(forbidden_rules(&violations));
                continue;
            }
            drop(analysis);
            let next = notes_of(score, &parts, request.voices.len());
            // The text after the rules: the loop ends at the first note both allow, and the
            // rules turn away far more notes than the text.
            match (&underlay, set) {
                (Some(underlay), Some(set)) => match underlay.advance(k, set, &next[k]) {
                    Ok(_) => return Ok(()),
                    Err(rule) => rules.push(rule),
                },
                _ => return Ok(()),
            }
        }
        rules.sort_unstable();
        rules.dedup();
        Err(rules)
    };
    // Free counterpoint keeps the beam of every downbeat: where the search runs into a dead
    // end, it goes back a measure (and further if that end returns) and tries again with other
    // noise, before the caller widens the beam and starts afresh.
    let mut checkpoints: Vec<(usize, Vec<State>)> = Vec::new();
    let mut failures: BTreeMap<u32, u32> = BTreeMap::new();
    let mut restarts = 0;
    let mut step = 0;
    let control = request.control.as_deref();
    while step < ticks.len() {
        let tick = ticks[step];
        // Once per step, also after going back to an earlier downbeat: stop if asked to, and
        // tell how far the search has come.
        if let Some(control) = control {
            if control.is_cancelled() {
                return Err(GenerateError::Cancelled);
            }
            control.reach(tick / per_measure + 1);
        }
        if any_free
            && tick.is_multiple_of(per_measure)
            && checkpoints.last().is_none_or(|(at, _)| *at < step)
        {
            if checkpoints.len() == CHECKPOINTS {
                checkpoints.remove(0);
            }
            checkpoints.push((step, beam.clone()));
        }
        let dead_end: Option<GenerateError> = 'tick: {
            let complete = step + 1 == ticks.len();
            let from = if step == 0 { 0 } else { ticks[step - 1] + 1 };
            let window = Window::partial(from, tick);
            let given_onset = given
                .get(given.partition_point(|&t| t < from))
                .is_some_and(|&t| t <= tick);
            // Noise in proportion to the time a step covers, so it weighs the same in every species.
            let span = ticks.get(step + 1).map_or(per_measure, |next| next - tick);
            let noise =
                request.noise * f64::from(span) / f64::from(per_measure) / order.len() as f64;
            let upcoming = heads_ahead(tick);
            for (i, state) in beam.iter_mut().enumerate() {
                state.base = state.cost;
                state.checked = false;
                state.forced = false;
                if tick.is_multiple_of(per_measure) {
                    state.lineage = i;
                }
            }
            // Fixed notes that leave no choice here are set first, so that the voices below them,
            // which choose first, are judged against them: a note held over, the note of an
            // entry's head, or the note a step away in a free octave. Where the octave is a real
            // choice the voice makes it in its turn. A state whose voice cannot take its fixed note
            // (the look ahead below should have spared it), or whose text then has no underlay
            // left, drops out; one that cannot take a head's note counts for IMI-ENTRY.
            if request.fixed.iter().any(|f| f.start == tick) {
                let mut blocked: BTreeMap<&'static str, usize> = BTreeMap::new();
                beam.retain_mut(|state| {
                    for (k, fixed) in fixed.iter().enumerate() {
                        let end: u32 = state.notes[k].iter().map(|n| n.duration.ticks()).sum();
                        if end != tick || !fixed.iter().any(|f| f.start == tick) {
                            continue;
                        }
                        let choices = choices_for(k, &state.notes[k], tick);
                        let last = state.notes[k].iter().rev().find_map(|n| n.pitch);
                        let exact = fixed.iter().any(|f| f.start == tick && f.in_head());
                        let forced = match choices.as_slice() {
                            [] => {
                                if exact || head_ends_at(k, tick) {
                                    *blocked.entry(ENTRY_RULE).or_default() += 1;
                                }
                                return false;
                            }
                            [Choice::Tie(_)] => choices.first().copied(),
                            [only] if exact => Some(*only),
                            _ => choices.iter().copied().find(|c| match (c, last) {
                                (Choice::Pitch(p, _), Some(l)) => {
                                    (p.diatonic() - l.diatonic()).abs() <= 1
                                }
                                _ => false,
                            }),
                        };
                        if let Some(choice) = forced {
                            apply(&mut state.notes[k], choice, base.mode);
                            state.forced = true;
                            if let Some(underlay) = &underlay {
                                match underlay.advance(k, &state.text[k], &state.notes[k]) {
                                    Ok(set) => {
                                        let before = state.text_cost;
                                        state.text[k] = set;
                                        state.text_cost = text_cost_of(&state.text);
                                        state.key += f64::from(state.text_cost) - f64::from(before);
                                    }
                                    Err(rule) => {
                                        *blocked.entry(rule).or_default() += 1;
                                        return false;
                                    }
                                }
                                state.steer[k] = underlay.term(k, &state.notes[k], &state.text[k]);
                            }
                        }
                    }
                    true
                });
                if beam.is_empty() {
                    break 'tick Some(GenerateError::NoCounterpoint {
                        measure: (tick / per_measure) as usize + 1,
                        rules: most_frequent(blocked),
                    });
                }
            }
            for (sub, &k) in order.iter().enumerate() {
                let last = sub + 1 == order.len();
                // Per rule, how many candidates it cut off here (for the diagnosis), and the latest
                // tick a look ahead failed at.
                let mut blocked: BTreeMap<&'static str, usize> = BTreeMap::new();
                let mut ahead: Option<u32> = None;
                // Every state with every choice for voice k: `None` while a note sounds on.
                let mut jobs: Vec<(usize, Option<Choice>)> = Vec::new();
                for (index, state) in beam.iter().enumerate() {
                    let end: u32 = state.notes[k].iter().map(|n| n.duration.ticks()).sum();
                    match end.cmp(&tick) {
                        std::cmp::Ordering::Greater => jobs.push((index, None)),
                        std::cmp::Ordering::Equal => {
                            let choices = choices_for(k, &state.notes[k], tick);
                            // A voice that cannot rest in the window before an entry: the
                            // diagnosis names the entry's rest.
                            if choices.is_empty() && windows[k].contains(&tick) {
                                *blocked.entry(REST_RULE).or_default() += 1;
                            }
                            for choice in choices {
                                if free
                                    && let Choice::Pitch(pitch, _) = choice
                                    && let Some(rule) =
                                        quick.rejects(&state.notes, k, pitch, tick, per_measure)
                                {
                                    *blocked.entry(rule).or_default() += 1;
                                    continue;
                                }
                                jobs.push((index, Some(choice)));
                            }
                        }
                        std::cmp::Ordering::Less => {}
                    }
                }
                let steps = underlay.as_ref().map(|_| {
                    Steps::new(jobs.iter().filter_map(|&(index, option)| {
                        option.map(|choice| (&beam[index].text[k], choice))
                    }))
                });
                // A candidate's check ends at the first rule that forbids it; `exact` checks every
                // rule, so that a dead end names the rules a full check would.
                let evaluate =
                    |&(index, option): &(usize, Option<Choice>), exact: bool| -> Outcome {
                        let state = &beam[index];
                        let mut notes = state.notes.clone();
                        // The voice's underlay states after its new note, if it has one.
                        let mut set: Option<Arc<Set>> = None;
                        if let Some(choice) = option {
                            apply(&mut notes[k], choice, base.mode);
                            // The text first: a note that leaves the voice no underlay costs no check.
                            if let (Some(underlay), Some(steps)) = (&underlay, &steps) {
                                let step = steps.take(&state.text[k], choice, || {
                                    underlay.advance(k, &state.text[k], &notes[k])
                                });
                                match step {
                                    Ok(next) => set = Some(next.clone()),
                                    Err(rule) => return Outcome::Blocked(vec![rule], None),
                                }
                            }
                            // The last note before the final measure must leave a way to end.
                            let end: u32 = notes[k].iter().map(|n| n.duration.ticks()).sum();
                            if end == final_start
                                && let Some(last) = notes[k].last().and_then(|n| n.pitch)
                                && !can_close(
                                    base,
                                    &pools[k],
                                    last,
                                    Some(&k) == bottom,
                                    final_start,
                                    ending,
                                )
                            {
                                return Outcome::Skip;
                            }
                            // Before and where a cadence arrives, before a fixed note and before the rest
                            // window of an entry, the voice must have a next note the rules allow: in
                            // the window its rest, which the text must allow there too.
                            let approaching = cadence_ticks.iter().any(|&goal| {
                                goal.saturating_sub(per_measure) <= end && end <= goal
                            }) || fixed[k].iter().any(|f| f.start == end)
                                || windows[k].contains(&end);
                            let states = set.as_deref().or(state.text.get(k).map(|s| &**s));
                            if approaching
                                && end < total
                                && let Err(rules) = continues(&notes, k, end, states, exact)
                            {
                                return Outcome::Blocked(rules, Some(end));
                            }
                            // A measure before, the bass must still be able to reach the cadence.
                            if Some(&k) == bottom
                                && cadence_ticks.contains(&(end + per_measure))
                                && let Some(last) = notes[k].last().and_then(|n| n.pitch)
                                && !bass_reaches_cadence(
                                    base,
                                    &pools[k],
                                    last,
                                    end + per_measure,
                                    target_at(end + per_measure),
                                    per_measure,
                                )
                            {
                                return Outcome::Skip;
                            }
                        }
                        let keep = |notes: Vec<Vec<Note>>, cost: u32, checked: bool| {
                            // Only the voice with a new note steers differently.
                            let (term, steer) = match &underlay {
                                Some(underlay) if option.is_some() => {
                                    let states = set.as_deref().unwrap_or(&state.text[k]);
                                    let term = underlay.term(k, &notes[k], states);
                                    (Some(term), underlay.steer(&state.steer, Some((k, term))))
                                }
                                Some(underlay) => (None, underlay.steer(&state.steer, None)),
                                None => (None, 0.0),
                            };
                            let estimate = distance_to_fixed(&notes, &fixed)
                                + f64::from(
                                    rhythms
                                        .iter()
                                        .zip(&notes)
                                        .map(|(r, n)| r.prefix_cost(n, per_measure))
                                        .sum::<u32>(),
                                );
                            let clash = if upcoming.is_empty() {
                                0.0
                            } else {
                                clash_with_fixed(&notes, &upcoming)
                            };
                            Outcome::Keep {
                                notes,
                                cost,
                                checked,
                                set,
                                term,
                                steer,
                                ahead: (estimate, clash),
                            }
                        };
                        if complete && last {
                            let score = assemble(base, &parts, &request.voices, &notes);
                            let report = checker.check_targets(&score, &targets);
                            if report.forbidden() > 0 {
                                return Outcome::Blocked(forbidden_rules(&report.violations), None);
                            }
                            keep(notes, report.cost(), true)
                        } else if option.is_none()
                            && (state.checked || !(given_onset || state.forced))
                        {
                            keep(notes, state.cost, state.checked)
                        } else {
                            // The whole window again: what earlier voices of this tick added may
                            // change now that this voice sounds.
                            let score = assemble_owned(base, &parts, &request.voices, notes);
                            let analysis = Analysis::new(&score);
                            let mut violations = Vec::new();
                            let forbidden = if exact {
                                checker.check_window(
                                    &score,
                                    &analysis,
                                    window,
                                    &targets,
                                    &mut violations,
                                );
                                violations.iter().any(|v| v.is_forbidden())
                            } else {
                                checker.check_window_until_forbidden(
                                    &score,
                                    &analysis,
                                    window,
                                    &targets,
                                    &mut violations,
                                    true,
                                )
                            };
                            if forbidden {
                                return Outcome::Blocked(forbidden_rules(&violations), None);
                            }
                            drop(analysis);
                            let notes = notes_of(score, &parts, request.voices.len());
                            let added: u32 = violations.iter().map(|v| v.level.cost()).sum();
                            keep(notes, state.base + added, true)
                        }
                    };
                let evaluate_all =
                    |jobs: &[&(usize, Option<Choice>)], exact: bool| -> Vec<Outcome> {
                        #[cfg(feature = "parallel")]
                        {
                            use rayon::prelude::*;
                            jobs.par_iter().map(|job| evaluate(job, exact)).collect()
                        }
                        #[cfg(not(feature = "parallel"))]
                        {
                            jobs.iter().map(|job| evaluate(job, exact)).collect()
                        }
                    };
                let outcomes = evaluate_all(&jobs.iter().collect::<Vec<_>>(), false);
                // The jobs a rule cut off, for the diagnosis.
                let mut cut: Vec<&(usize, Option<Choice>)> = Vec::new();
                // In the order of the jobs, so that the noise is the same on every machine.
                let mut next = Vec::new();
                for (job, outcome) in jobs.iter().zip(outcomes) {
                    let index = job.0;
                    let state = &beam[index];
                    match outcome {
                        Outcome::Keep {
                            notes,
                            cost,
                            checked,
                            set,
                            term,
                            steer,
                            ahead: (estimate, clash),
                        } => {
                            let (text, text_cost) = match set {
                                Some(set) => {
                                    let mut text = state.text.clone();
                                    text[k] = set;
                                    let sum = text_cost_of(&text);
                                    (text, sum)
                                }
                                None => (state.text.clone(), state.text_cost),
                            };
                            let mut terms = state.steer.clone();
                            if let Some(term) = term {
                                terms[k] = term;
                            }
                            let mut ahead = estimate;
                            let mut key = state.key + f64::from(cost) - f64::from(state.cost);
                            if underlay.is_some() {
                                ahead += steer;
                                key += f64::from(text_cost) - f64::from(state.text_cost);
                            }
                            if !upcoming.is_empty() {
                                ahead += CLASH * clash;
                            }
                            next.push(State {
                                ahead,
                                notes,
                                base: state.base,
                                cost,
                                key: key + rng.next_f64() * noise,
                                checked,
                                forced: state.forced,
                                lineage: state.lineage,
                                text,
                                text_cost,
                                steer: terms,
                            });
                        }
                        Outcome::Blocked(_, at) => {
                            cut.push(job);
                            ahead = ahead.max(at);
                        }
                        Outcome::Skip => {}
                    }
                }
                if next.is_empty() {
                    // The checks above ended at the first rule that forbade a candidate; the
                    // diagnosis counts every rule that forbids it.
                    for outcome in evaluate_all(&cut, true) {
                        if let Outcome::Blocked(rules, _) = outcome {
                            for rule in rules {
                                *blocked.entry(rule).or_default() += 1;
                            }
                        }
                    }
                    break 'tick Some(GenerateError::NoCounterpoint {
                        measure: (ahead.unwrap_or(tick).max(tick) / per_measure) as usize + 1,
                        rules: most_frequent(blocked),
                    });
                }
                next.sort_by(|a, b| (a.key + a.ahead).total_cmp(&(b.key + b.ahead)));
                // In the final sonority and at the goal of an internal cadence every voice is bound
                // by the others; until the last voice is set, keep more states so that one that fits
                // is still there.
                let goal = tick.is_multiple_of(per_measure)
                    && goals.contains(&((tick / per_measure) as usize));
                let width = if (complete || goal) && !last {
                    request.beam.max(1) * FINAL_WIDTH
                } else {
                    request.beam.max(1)
                };
                if free {
                    let share = width.div_ceil(LINEAGE_SHARE);
                    let mut taken: BTreeMap<usize, usize> = BTreeMap::new();
                    next.retain(|state| {
                        let count = taken.entry(state.lineage).or_default();
                        *count += 1;
                        *count <= share
                    });
                }
                if any_free {
                    // A voice still to be set here whose note is held over a barline may be a
                    // suspension: it needs a next note the rules allow, which the voices set
                    // first must not take away (a bass that turns its resolution into a
                    // dissonance).
                    let pending: Vec<usize> = order[sub + 1..]
                        .iter()
                        .copied()
                        .filter(|&j| request.voices[j].species == Species::Free)
                        .collect();
                    if !pending.is_empty() {
                        let mut kept = Vec::with_capacity(width);
                        // The states reached with an open suspension, for the diagnosis.
                        let mut open = Vec::new();
                        // The open suspensions of a chunk of states are found in parallel (the
                        // first pending voice without a next note, with the rules that took it
                        // away); the states are then taken in order.
                        let open_of = |state: &State, exact: bool| -> Option<Vec<&'static str>> {
                            pending.iter().copied().find_map(|j| {
                                let notes = &state.notes[j];
                                let end: u32 = notes.iter().map(|n| n.duration.ticks()).sum();
                                if end != tick || notes.len() < 2 || !notes[notes.len() - 2].tie {
                                    return None;
                                }
                                let set = state.text.get(j).map(|s| &**s);
                                continues(&state.notes, j, tick, set, exact).err()
                            })
                        };
                        let mut rest = next.into_iter();
                        'chunks: loop {
                            let chunk: Vec<State> =
                                rest.by_ref().take(width - kept.len()).collect();
                            if chunk.is_empty() {
                                break;
                            }
                            #[cfg(feature = "parallel")]
                            let opens: Vec<bool> = {
                                use rayon::prelude::*;
                                chunk
                                    .par_iter()
                                    .map(|state| open_of(state, false).is_some())
                                    .collect()
                            };
                            #[cfg(not(feature = "parallel"))]
                            let opens: Vec<bool> = chunk
                                .iter()
                                .map(|state| open_of(state, false).is_some())
                                .collect();
                            for (state, is_open) in chunk.into_iter().zip(opens) {
                                if kept.len() == width {
                                    break 'chunks;
                                }
                                if is_open {
                                    open.push(state);
                                } else {
                                    kept.push(state);
                                }
                            }
                            if kept.len() == width {
                                break;
                            }
                        }
                        next = kept;
                        if next.is_empty() {
                            // The rules of the text that took a suspension's resolution away,
                            // from a check of every rule.
                            let mut text_rules: BTreeMap<&'static str, usize> = BTreeMap::new();
                            for rules in open.iter().filter_map(|state| open_of(state, true)) {
                                for rule in rules.into_iter().filter(|r| r.starts_with("TXT-")) {
                                    *text_rules.entry(rule).or_default() += 1;
                                }
                            }
                            let mut rules = vec!["FREE-RESOLUTION"];
                            rules.extend(most_frequent(text_rules).into_iter().take(2));
                            break 'tick Some(GenerateError::NoCounterpoint {
                                measure: (tick / per_measure) as usize + 1,
                                rules,
                            });
                        }
                    }
                }
                next.truncate(width);
                beam = next;
            }
            None
        };
        if let Some(error) = dead_end {
            let measure = tick / per_measure;
            let back = failures.entry(measure).or_default();
            *back += 1;
            let until = tick.saturating_sub(*back * per_measure);
            let checkpoint = checkpoints
                .iter()
                .rposition(|(at, _)| ticks[*at] <= until)
                .or((!checkpoints.is_empty()).then_some(0));
            match checkpoint {
                Some(c) if restarts < MAX_RESTARTS => {
                    restarts += 1;
                    checkpoints.truncate(c + 1);
                    step = checkpoints[c].0;
                    beam = checkpoints[c].1.clone();
                    rng = Rng::new(
                        request
                            .seed
                            .wrapping_add(u64::from(restarts).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
                    );
                    continue;
                }
                _ => return Err(error),
            }
        }
        step += 1;
    }
    // Every new voice has reached its end, and with a text its underlay states were stepped over
    // the end: each state left in the beam has an underlay of the whole text in every voice.
    beam.sort_by(|a, b| {
        (a.cost + a.text_cost)
            .cmp(&(b.cost + b.text_cost))
            .then(a.key.total_cmp(&b.key))
    });
    let mut results: Vec<Generated> = Vec::new();
    // The notes of the results so far, before the text was laid under them.
    let mut taken: Vec<Vec<Vec<Note>>> = Vec::new();
    // What kept a texted result out, which only a disagreement between the search and the rules
    // can do.
    let mut failed: Vec<&'static str> = Vec::new();
    for state in beam {
        if results.len() >= request.count.max(1) {
            break;
        }
        if taken.contains(&state.notes) {
            continue;
        }
        taken.push(state.notes.clone());
        let mut notes = state.notes;
        // What the dynamic program says the underlays cost.
        let laid = match &underlay {
            None => None,
            Some(underlay) => {
                let Some(cost) = underlay.lay(&mut notes) else {
                    debug_assert!(false, "a voice the search accepted has no underlay");
                    failed.push("TXT-COMPLETE");
                    continue;
                };
                Some(cost)
            }
        };
        let mut score = assemble(base, &parts, &request.voices, &notes);
        if let Some(sung) = &request.text {
            // The new voices sing the text; the given ones keep what the base says of them.
            let given = score.text.take().map(|t| t.voices);
            score.text = Some(ScoreText {
                text: sung.text.clone(),
                voices: parts
                    .iter()
                    .enumerate()
                    .map(|(i, part)| match part {
                        Part::Given(_) => given.as_ref().is_some_and(|voices| voices[i]),
                        Part::New(_) => true,
                    })
                    .collect(),
            });
        }
        let report = checker.check_targets(&score, &targets);
        if underlay.is_some() && report.forbidden() > 0 {
            debug_assert!(
                false,
                "a texted result breaks a prohibition: {:?}",
                report.violations
            );
            failed.extend(forbidden_rules(&report.violations));
            continue;
        }
        // The TXT rules find the cases of the underlays at the prices the dynamic program paid.
        debug_assert!(
            laid.is_none_or(|cost| cost == text_cost(&report)),
            "the TXT rules cost {} where the underlays cost {laid:?}: {:?}",
            text_cost(&report),
            report.violations
        );
        results.push(Generated {
            cost: report.cost(),
            report,
            score,
            warnings: Vec::new(),
            subjects: Vec::new(),
        });
    }
    if results.is_empty() {
        failed.sort_unstable();
        failed.dedup();
        return Err(GenerateError::NoCounterpoint {
            measure: measures,
            rules: failed,
        });
    }
    Ok(results)
}

/// The fixed notes per new voice, in order.
fn fixed_by_voice(request: &CounterpointRequest) -> Vec<Vec<&Fixed>> {
    let mut fixed: Vec<Vec<&Fixed>> = vec![Vec::new(); request.voices.len()];
    for f in &request.fixed {
        fixed[f.voice].push(f);
    }
    for list in &mut fixed {
        list.sort_by_key(|f| f.start);
    }
    fixed
}

/// The order of the voices from high to low: given and new voices sorted by voice type.
fn order_voices(base: &Score, voices: &[NewVoice]) -> Vec<Part> {
    let mut parts: Vec<(VoiceType, usize, Part)> = base
        .voices
        .iter()
        .enumerate()
        .map(|(i, v)| (v.voice_type, 0, Part::Given(i)))
        .chain(
            voices
                .iter()
                .enumerate()
                .map(|(k, v)| (v.voice_type, 1, Part::New(k))),
        )
        .collect();
    parts.sort_by_key(|(voice_type, new, _)| (*voice_type, *new));
    parts.into_iter().map(|(_, _, part)| part).collect()
}

fn assemble(base: &Score, parts: &[Part], voices: &[NewVoice], notes: &[Vec<Note>]) -> Score {
    assemble_owned(base, parts, voices, notes.to_vec())
}

/// [`assemble`] that takes the new voices' notes instead of cloning them; [`notes_of`] gives
/// them back.
fn assemble_owned(
    base: &Score,
    parts: &[Part],
    voices: &[NewVoice],
    mut notes: Vec<Vec<Note>>,
) -> Score {
    let all = parts
        .iter()
        .map(|part| match *part {
            Part::Given(i) => base.voices[i].clone(),
            Part::New(k) => Voice::new(
                voices[k].voice_type,
                Role::Counterpoint(voices[k].species),
                std::mem::take(&mut notes[k]),
            ),
        })
        .collect();
    let mut score = Score::new(base.mode, all);
    score.title = base.title.clone();
    score.meter = base.meter;
    score.cadences = base.cadences.clone();
    score.imitation = base.imitation.clone();
    // The given voices keep whether they sing the text; the new ones carry no lyrics yet.
    score.text = base.text.as_ref().map(|sung| ScoreText {
        text: sung.text.clone(),
        voices: parts
            .iter()
            .map(|part| match *part {
                Part::Given(i) => sung.voices.get(i).copied().unwrap_or(false),
                Part::New(_) => false,
            })
            .collect(),
    });
    score
}

/// The notes of the new voices of a score made by [`assemble_owned`].
fn notes_of(score: Score, parts: &[Part], count: usize) -> Vec<Vec<Note>> {
    let mut notes = vec![Vec::new(); count];
    for (voice, part) in score.voices.into_iter().zip(parts) {
        if let Part::New(k) = *part {
            notes[k] = voice.notes;
        }
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::fux_cantus;
    use crate::generate::{cadence_formula, cadences_at};
    use crate::imitation;
    use crate::score::Lyric;
    use crate::text::TextOptions;
    use crate::text::ordinary::section;

    fn cantus_score(id: &str, voice_type: VoiceType) -> Score {
        let cantus = fux_cantus().into_iter().find(|c| c.id == id).unwrap();
        let shift = voice_type.octave_shift_for(&cantus.notes) as i8;
        let notes = cantus
            .notes
            .iter()
            .map(|p| {
                Note::new(
                    Pitch::new(p.step, p.alter, p.octave + shift),
                    Duration::WHOLE,
                )
            })
            .collect();
        Score::new(
            cantus.mode,
            vec![Voice::new(voice_type, Role::CantusFirmus, notes)],
        )
    }

    #[test]
    fn pools_contain_the_ficta() {
        let names = |pool: Vec<Pitch>| -> Vec<String> {
            pool.iter()
                .filter(|p| p.alter != 0)
                .map(ToString::to_string)
                .collect()
        };
        let soprano = |mode: &str| pitch_pool(mode.parse().unwrap(), VoiceType::Soprano);
        // The leading tone, the major third of the final and the flattened mi.
        let dorian = names(soprano("dorian"));
        for name in ["C#5", "F#4", "Bb4"] {
            assert!(dorian.contains(&name.to_string()), "{dorian:?}");
        }
        assert_eq!(
            names(soprano("aeolian")),
            ["F#3", "G#3", "C#4", "F#4", "G#4", "C#5", "F#5", "G#5"]
        );
        assert_eq!(names(soprano("phrygian")), ["G#3", "G#4", "G#5"]);
        assert!(names(soprano("ionian")).is_empty());
        // An internal cadence on A in the Dorian mode adds G♯.
        let with_a = pitch_pool_with("dorian".parse().unwrap(), VoiceType::Soprano, &[Step::A]);
        assert!(names(with_a).contains(&"G#4".to_string()));
    }

    /// A part that ends elsewhere than on the final takes the leading tone of its ending and the
    /// major third of its last chord into the pool: G♯ and C♯ for A in the Phrygian mode (with
    /// F♯ against the augmented second before G♯, as for any cadence on A), G♯ and C♯ for A in
    /// the Dorian mode, nothing for E in the Aeolian mode (a Phrygian clausula, its third G♯
    /// already the leading tone of the final). On the final the pool is the one of
    /// [`pitch_pool_with`], in every mode and voice.
    #[test]
    fn pools_take_the_ending() {
        let names = |pool: Vec<Pitch>| -> Vec<String> {
            pool.iter()
                .filter(|p| p.alter != 0)
                .map(ToString::to_string)
                .collect()
        };
        let soprano = |mode: &str, ending: Step| {
            pitch_pool_ending(mode.parse().unwrap(), VoiceType::Soprano, &[], ending)
        };
        let phrygian_on_a = names(soprano("phrygian", Step::A));
        for name in ["C#4", "C#5", "G#4", "F#4"] {
            assert!(
                phrygian_on_a.contains(&name.to_string()),
                "{phrygian_on_a:?}"
            );
        }
        assert!(!names(soprano("phrygian", Step::E)).contains(&"C#4".to_string()));
        let dorian_on_a = names(soprano("dorian", Step::A));
        for name in ["G#4", "C#5", "C#4", "F#4", "Bb4"] {
            assert!(dorian_on_a.contains(&name.to_string()), "{dorian_on_a:?}");
        }
        assert_eq!(
            soprano("aeolian", Step::E),
            pitch_pool("aeolian".parse().unwrap(), VoiceType::Soprano)
        );
        for mode in Mode::ALL {
            for transposed in [false, true] {
                let mode = ModeSpec::new(mode).with_transposition(transposed);
                for voice in [VoiceType::Soprano, VoiceType::Alto, VoiceType::Bass] {
                    let targets = crate::generate::regular_cadences(mode);
                    assert_eq!(
                        pitch_pool_ending(mode, voice, &targets, mode.final_step()),
                        pitch_pool_with(mode, voice, &targets)
                    );
                }
            }
        }
    }

    /// The tones of the sonority where a cadence arrives, in every octave: on the final A, E is
    /// the fifth and C♯ the major third also where they lie below A in their octave, while F
    /// and D belong to no sonority on A; the lowest voice ends on A itself, and an internal
    /// goal also takes the minor third. The same on a final with a flat, B♭.
    #[test]
    fn goal_tones_in_every_octave() {
        for octave in 1..=5 {
            let tone = |target: Step, alter: i8, name: &str, lowest, last| {
                let pitch: Pitch = format!("{name}{octave}").parse().unwrap();
                goal_tone(target, alter, pitch, lowest, last)
            };
            let on_a = |name: &str, lowest, last| tone(Step::A, 0, name, lowest, last);
            for name in ["A", "E", "C#"] {
                assert!(on_a(name, false, true), "{name}{octave}");
            }
            for name in ["F", "D", "C", "G", "B", "G#", "Bb"] {
                assert!(!on_a(name, false, true), "{name}{octave}");
            }
            assert!(on_a("A", true, true));
            assert!(!on_a("E", true, true) && !on_a("C#", true, true));
            assert!(on_a("C", false, false) && on_a("E", true, false));
            assert!(!on_a("F", false, false) && !on_a("D", true, false));
            let on_b_flat = |name: &str| tone(Step::B, -1, name, false, true);
            for name in ["Bb", "F", "D"] {
                assert!(on_b_flat(name), "{name}{octave}");
            }
            for name in ["B", "Eb", "G", "C"] {
                assert!(!on_b_flat(name), "{name}{octave}");
            }
        }
    }

    #[test]
    fn rhythms_follow_the_patterns() {
        let checker = Checker::new(&Profile::fux());
        let onsets = |species, measures| {
            let mut ticks = Rhythm::new(species, false, measures, Vec::new(), &checker).onsets(8);
            ticks.sort_unstable();
            ticks.dedup();
            ticks
        };
        assert_eq!(onsets(Species::Second, 3), [0, 4, 8, 12, 16]);
        assert_eq!(onsets(Species::Third, 2), [0, 2, 4, 6, 8]);
        assert_eq!(
            onsets(Species::Fifth, 4),
            [0, 4, 8, 10, 11, 12, 14, 15, 16, 20, 24]
        );
        let pool = ["D5".parse().unwrap()];
        let fifth = Rhythm::new(Species::Fifth, false, 4, Vec::new(), &checker);
        let half = |p: &str, tie| Note {
            tie,
            ..Note::new(p.parse().unwrap(), Duration::HALF)
        };
        // After r2 A4 the next measure begins tied (⁀4, ⁀2) or with a new quarter or half.
        let notes = [Note::rest(Duration::HALF), half("A4", false)];
        let choices = fifth.choices(&notes, 8, 8, &pool, None);
        assert!(choices.contains(&Choice::Tie(Duration::QUARTER)));
        assert!(choices.contains(&Choice::Tie(Duration::HALF)));
        assert!(choices.contains(&Choice::Pitch(pool[0], Duration::QUARTER)));
        // The penultimate measure only continues the syncopation (⁀2 2).
        let notes = [
            Note::rest(Duration::HALF),
            half("A4", true),
            half("A4", false),
            half("D5", false),
        ];
        assert_eq!(
            fifth.choices(&notes, 16, 8, &pool, None),
            [Choice::Tie(Duration::HALF)]
        );
        // The second species may start with a rest or, at a cost, with a note.
        let second = Rhythm::new(Species::Second, false, 3, Vec::new(), &checker);
        let choices = second.choices(&[], 0, 8, &pool, None);
        assert_eq!(choices[0], Choice::Rest(Duration::HALF));
        assert_eq!(choices.len(), 2);
    }

    #[test]
    fn syncopations_are_tied() {
        let base = cantus_score("dorian", VoiceType::Alto);
        let request = CounterpointRequest::new(
            base,
            vec![NewVoice {
                voice_type: VoiceType::Soprano,
                species: Species::Fourth,
            }],
        );
        let results = generate_counterpoint(&request, &Profile::fux()).unwrap();
        let voice = &results[0].score.voices[0];
        assert!(voice.notes[0].is_rest());
        let ties = voice.notes.iter().filter(|n| n.tie).count();
        assert!(ties >= 6, "{ties} ties: {:?}", voice.notes);
        results[0].score.validate().unwrap();
    }

    #[test]
    fn impossible_requests_name_the_measure() {
        let notes = (0..5)
            .map(|_| Note::new("D4".parse().unwrap(), Duration::WHOLE))
            .collect();
        let base = Score::new(
            "dorian".parse().unwrap(),
            vec![Voice::new(VoiceType::Alto, Role::CantusFirmus, notes)],
        );
        let mut profile = Profile::fux();
        // A cantus that never moves leaves only oblique motion, which this profile forbids: the
        // counterpoint can only repeat its note until the cadence has to move.
        profile.apply_override("MOT-PREFERENCE=V").unwrap();
        let request = CounterpointRequest::new(
            base,
            vec![NewVoice {
                voice_type: VoiceType::Soprano,
                species: Species::First,
            }],
        );
        let error = generate_counterpoint(&request, &profile).unwrap_err();
        assert!(
            matches!(&error, GenerateError::NoCounterpoint { measure: 4, rules } if rules[0] == "MOT-PREFERENCE"),
            "{error:?}"
        );
        assert!(error.message(crate::lang::Lang::De).starts_with(
            "Keine Lösung: Die Regeln verbieten jede Fortsetzung in Takt 4 (vor allem MOT-PREFERENCE"
        ));
    }

    #[test]
    fn first_species_above_and_below() {
        let profile = Profile::fux();
        let base = cantus_score("dorian", VoiceType::Alto);
        for voice_type in [VoiceType::Soprano, VoiceType::Tenor] {
            let request = CounterpointRequest::new(
                base.clone(),
                vec![NewVoice {
                    voice_type,
                    species: Species::First,
                }],
            );
            let results = generate_counterpoint(&request, &profile).unwrap();
            let score = &results[0].score;
            assert_eq!(results[0].report.forbidden(), 0);
            assert_eq!(score.voices.len(), 2);
            score.validate().unwrap();
        }
    }

    #[test]
    fn ties_carry_no_text() {
        let mode: ModeSpec = "dorian".parse().unwrap();
        let held = Note {
            lyric: Some(Lyric::new(0)),
            syllable: Some("Ký-".into()),
            ..Note::new("D4".parse().unwrap(), Duration::HALF)
        };
        let mut notes = vec![held];
        apply(&mut notes, Choice::Tie(Duration::HALF), mode);
        assert!(notes[0].tie);
        assert_eq!(notes[0].lyric, Some(Lyric::new(0)));
        assert_eq!(notes[1].pitch, notes[0].pitch);
        assert_eq!(
            (notes[1].lyric, notes[1].syllable.as_deref(), notes[1].tie),
            (None, None, false)
        );
    }

    /// A free setting as `generate_free` asks for it: the voices in score order, internal
    /// cadences arriving in the measures `goals` (from 0) and the final cadence as formulas, with
    /// a section of the Ordinary sung at a syllable start per measure.
    fn free_request(
        mode: &str,
        voices: &[VoiceType],
        measures: usize,
        goals: &[usize],
        text: Option<&str>,
        seed: u64,
    ) -> CounterpointRequest {
        let mode: ModeSpec = mode.parse().unwrap();
        let mut voices = voices.to_vec();
        voices.sort();
        let mut base = Score::new(mode, Vec::new());
        base.cadences = cadences_at(mode, goals);
        let meter = base.meter;
        let mut fixed = cadence_formula(mode, mode.final_step(), measures - 1, voices.len(), meter);
        for cadence in &base.cadences {
            fixed.extend(cadence_formula(
                mode,
                cadence.target,
                cadence.measure,
                voices.len(),
                meter,
            ));
        }
        CounterpointRequest {
            measures: Some(measures),
            fixed,
            seed,
            text: text.map(|id| SungText {
                text: Arc::new(section(id, &TextOptions::default()).unwrap().clone()),
                rate: 50,
            }),
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

    /// The underlay choices that `request` pins in each voice of its results, in score order.
    fn pins_by_voice(request: &CounterpointRequest) -> Vec<Vec<(u32, Underlaid)>> {
        let fixed = fixed_by_voice(request);
        order_voices(&request.base, &request.voices)
            .iter()
            .map(|part| match *part {
                Part::Given(_) => Vec::new(),
                Part::New(k) => fixed[k]
                    .iter()
                    .filter_map(|f| f.sung.map(|sung| (f.start, sung)))
                    .collect(),
            })
            .collect()
    }

    /// Every singing voice of a texted result of `request` that is no cantus firmus carries the
    /// cheapest underlay of its notes that keeps the request's pins, as the dynamic program lays
    /// it, and the TXT rules of the result's report find its cases at the price the dynamic
    /// program paid, none of them forbidden. The model is built as the TXT rules build it, from
    /// the score.
    fn assert_priced_as_laid(
        generated: &Generated,
        request: &CounterpointRequest,
        profile: &Profile,
        label: &str,
    ) {
        let score = &generated.score;
        let sung = score.text.as_ref().expect("a text");
        let goals = Goal::of(&score.cadences, score.meter);
        let restarts = score
            .imitation
            .as_deref()
            .map(|plan| plan.restarts(&sung.text))
            .unwrap_or_default();
        let model = Model::new(&sung.text, score.mode, score.meter, &goals)
            .with_restarts(&sung.text, &restarts);
        let levels = Levels::from(&Checker::new(profile));
        let pins = pins_by_voice(request);
        let gates = |v: usize| {
            score
                .imitation
                .as_deref()
                .map(|plan| plan.gates(v))
                .filter(|gates| !gates.is_empty())
        };
        let lyrics = |notes: &[Note]| notes.iter().map(|n| n.lyric).collect::<Vec<_>>();
        let mut singing = 0;
        for (v, voice) in score.voices.iter().enumerate() {
            if !sung.voices[v] || voice.is_cantus_firmus() {
                continue;
            }
            singing += 1;
            let mut bare = voice.notes.clone();
            for note in &mut bare {
                note.lyric = None;
            }
            let pinned = (!pins[v].is_empty()).then_some(&pins[v][..]);
            let path = model
                .best_pinned(&levels, &bare, pinned, gates(v).as_deref())
                .unwrap_or_else(|| panic!("{label}, voice {v}: no underlay"));
            path.apply(&mut bare);
            assert_eq!(lyrics(&bare), lyrics(&voice.notes), "{label}, voice {v}");
            let found: Vec<&crate::rules::Violation> = generated
                .report
                .violations
                .iter()
                .filter(|x| x.voice == v && x.rule.starts_with("TXT-"))
                .collect();
            assert!(
                found.iter().all(|x| !x.is_forbidden()),
                "{label}, voice {v}: {found:?}"
            );
            let cost: u32 = found.iter().map(|x| x.level.cost()).sum();
            assert_eq!(cost, path.cost, "{label}, voice {v}: {found:?}");
        }
        assert!(singing > 0, "{label}: no voice sings");
    }

    /// Without a text the search finds what it found before there were texts (phase 11b), and
    /// its results carry no text: the two-voice Dorian setting of tests/free.rs.
    #[test]
    fn untexted_results_are_unchanged() {
        use VoiceType::*;
        let request = CounterpointRequest {
            beam: 100,
            ..free_request("dorian", &[Soprano, Tenor], 10, &[5], None, 1)
        };
        let results = generate_counterpoint(&request, &Profile::fux()).unwrap();
        let score = &results[0].score;
        assert_eq!(score.text, None);
        assert!(
            score
                .voices
                .iter()
                .flat_map(|v| &v.notes)
                .all(|n| n.lyric.is_none() && n.syllable.is_none())
        );
        let voices: Vec<String> = score
            .voices
            .iter()
            .map(|voice| {
                let notes: Vec<String> = voice
                    .notes
                    .iter()
                    .map(|n| {
                        let pitch = n.pitch.map_or("r".to_string(), |p| p.to_string());
                        let tie = if n.tie { "~" } else { "" };
                        format!("{pitch}:{}{tie}", n.duration.ticks())
                    })
                    .collect();
                notes.join(" ")
            })
            .collect();
        assert_eq!(
            voices,
            [
                "D5:8 E5:8 C5:2 B4:2 A4:2 G4:2 A4:8~ A4:4 G#4:4 A4:8 Bb4:6 G4:2 F4:4 D4:4~ D4:4 \
                 C#4:4 D4:8",
                "r:4 D4:4~ D4:4 C4:4 E4:6 D4:2 C4:6 A3:2 B3:8 A3:8 G3:2 F3:2 D3:2 E3:2 A3:2 G3:2 \
                 F3:4 E3:8 D3:8",
            ]
        );
        assert_eq!(results[0].cost, 3);
    }

    /// A free setting whose final formula and ending are on the fifth ends there: every voice on
    /// a tone of its triad, the bass on A itself, nothing forbidden (TEX-FINAL-CHORD leaves the
    /// fifth free, and the rules read the final cadence from the bass).
    #[test]
    fn searches_end_on_their_ending() {
        use VoiceType::*;
        let mode: ModeSpec = "dorian".parse().unwrap();
        let mut request = CounterpointRequest {
            beam: 40,
            ending: Some(Step::A),
            ..free_request("dorian", &[Alto, Tenor, Bass], 10, &[5], None, 1)
        };
        request.fixed = cadence_formula(mode, Step::A, 9, 3, Meter::ALLA_BREVE);
        request
            .fixed
            .extend(cadence_formula(mode, Step::F, 5, 3, Meter::ALLA_BREVE));
        request.base.cadences = vec![crate::score::Cadence {
            measure: 5,
            target: Step::F,
        }];
        let results = generate_counterpoint(&request, &Profile::fux()).unwrap();
        assert_eq!(results[0].report.forbidden(), 0, "{:?}", results[0].report);
        let score = &results[0].score;
        let last: Vec<Pitch> = score
            .voices
            .iter()
            .map(|v| v.notes.last().unwrap().pitch.unwrap())
            .collect();
        assert_eq!(last[2].step, Step::A, "{last:?}");
        for pitch in &last {
            assert!(goal_tone(Step::A, 0, *pitch, false, true), "{last:?}");
        }
    }

    /// A search with a control finds what it finds without one and tells the measure it has
    /// reached, the last at its end; once cancelled, it fails at its first step.
    #[test]
    fn controlled_searches_find_the_same() {
        use VoiceType::*;
        let request = CounterpointRequest {
            beam: 40,
            ..free_request("dorian", &[Soprano, Tenor], 10, &[5], None, 1)
        };
        let plain = generate_counterpoint(&request, &Profile::fux()).unwrap();
        let control = Arc::new(SearchControl::new());
        let controlled = CounterpointRequest {
            control: Some(Arc::clone(&control)),
            ..request
        };
        let found = generate_counterpoint(&controlled, &Profile::fux()).unwrap();
        assert_eq!(found[0].score, plain[0].score);
        assert_eq!(found[0].cost, plain[0].cost);
        assert_eq!(control.reached(), 10);
        control.cancel();
        assert_eq!(
            generate_counterpoint(&controlled, &Profile::fux()).unwrap_err(),
            GenerateError::Cancelled
        );
    }

    /// The plan of an imitation travels from the search's base into every score it assembles,
    /// those of the results included; a plan without points changes nothing.
    #[test]
    fn plans_reach_the_results() {
        use crate::score::ScoreImitation;
        use VoiceType::*;
        let request = CounterpointRequest {
            beam: 20,
            ..free_request("dorian", &[Soprano, Tenor], 8, &[], None, 1)
        };
        let plain = generate_counterpoint(&request, &Profile::fux()).unwrap();
        assert_eq!(plain[0].score.imitation, None);
        let plan = Arc::new(ScoreImitation { points: Vec::new() });
        let mut planned = request.clone();
        planned.base.imitation = Some(plan.clone());
        let results = generate_counterpoint(&planned, &Profile::fux()).unwrap();
        let kept = results[0].score.imitation.as_ref().expect("the plan");
        assert!(Arc::ptr_eq(kept, &plan));
        assert_eq!(results[0].score.voices, plain[0].score.voices);
        assert_eq!(results[0].cost, plain[0].cost);
    }

    /// A text too long for the setting leaves no voice an underlay: the diagnosis names the
    /// text's rule, at the first note (a rest before it begins no syllable).
    #[test]
    fn texts_that_do_not_fit_name_their_rule() {
        use VoiceType::*;
        let request = free_request("dorian", &[Soprano, Tenor], 4, &[], Some("agnus"), 1);
        let error = generate_counterpoint(&request, &Profile::fux()).unwrap_err();
        assert!(
            matches!(&error, GenerateError::NoCounterpoint { measure: 1 | 2, rules } if rules[0] == "TXT-COMPLETE"),
            "{error:?}"
        );
        let mut empty = request.clone();
        empty.text = Some(SungText {
            text: Arc::new(Text {
                syllables: Vec::new(),
                ..(*request.text.as_ref().unwrap().text).clone()
            }),
            rate: 50,
        });
        assert_eq!(
            generate_counterpoint(&empty, &Profile::fux()).unwrap_err(),
            GenerateError::EmptyText
        );
    }

    /// The search of a texted request prices a repeated white note at the level of
    /// `free_texted`, as the results that sing are priced; the other parameters keep theirs.
    #[test]
    fn sung_voices_repeat_notes_at_their_level_in_the_search() {
        let level = |profile: &Profile, param| profile.settings("MEL-REPEAT").level(param);
        let fux = Profile::fux();
        assert_eq!(level(&fux, "free"), Level::P1);
        assert_eq!(
            level(&sung_profile(&fux), "free"),
            level(&fux, "free_texted")
        );
        for texted in ["V", "P2", "P(3)", "aus"] {
            let mut profile = Profile::fux();
            profile
                .apply_override(&format!("MEL-REPEAT.free_texted={texted}"))
                .unwrap();
            profile.apply_override("MEL-REPEAT.free_black=P4").unwrap();
            profile.apply_override("MEL-REPEAT.free_run=P3").unwrap();
            let sung = sung_profile(&profile);
            assert_eq!(
                level(&sung, "free"),
                level(&profile, "free_texted"),
                "{texted}"
            );
            for param in ["anticipation", "free_black", "free_run", "free_texted"] {
                assert_eq!(level(&sung, param), level(&profile, param), "{param}");
            }
            // Applied twice, the same.
            let twice = sung_profile(&sung);
            assert_eq!(twice.to_toml(None), sung.to_toml(None));
        }
    }

    /// The search of a texted request prices a rest at the level of FREE-REST `texted`, as the
    /// results that sing are priced.
    #[test]
    fn sung_voices_rest_at_their_level_in_the_search() {
        let level = |profile: &Profile, param| profile.settings("FREE-REST").level(param);
        let fux = Profile::fux();
        assert_eq!(level(&fux, "level"), Level::P1);
        assert_eq!(level(&sung_profile(&fux), "level"), level(&fux, "texted"));
        for texted in ["V", "P2", "aus"] {
            let mut profile = Profile::fux();
            profile
                .apply_override(&format!("FREE-REST.texted={texted}"))
                .unwrap();
            let sung = sung_profile(&profile);
            assert_eq!(level(&sung, "level"), level(&profile, "texted"), "{texted}");
            assert_eq!(
                level(&sung, "texted"),
                level(&profile, "texted"),
                "{texted}"
            );
        }
    }

    /// A direct texted request prices repeated notes as its results: with repeated notes free
    /// in untexted voices but forbidden in voices that sing, the search finds settings whose
    /// singing voices repeat no white note.
    #[test]
    fn direct_texted_requests_repeat_notes_as_their_results() {
        use VoiceType::*;
        let mut profile = Profile::fux();
        profile.apply_override("MEL-REPEAT.free=aus").unwrap();
        profile.apply_override("MEL-REPEAT.free_texted=V").unwrap();
        let request = CounterpointRequest {
            beam: 60,
            ..free_request("dorian", &[Soprano, Tenor], 16, &[6], Some("kyrie"), 1)
        };
        let results = generate_counterpoint(&request, &profile).unwrap();
        assert_eq!(results[0].report.forbidden(), 0);
        assert_eq!(
            results[0].cost,
            Checker::new(&profile)
                .check_targets(&results[0].score, &[true, true])
                .cost()
        );
        assert_priced_as_laid(&results[0], &request, &profile, "strict repeats");
    }

    /// A search that finds nothing tries again with twice and four times its beam, as far as
    /// the request's `widen` allows: here only four times the beam finds a setting, the same as
    /// a search with that beam from the start.
    #[test]
    fn searches_widen_their_beam_as_far_as_allowed() {
        use VoiceType::*;
        assert_eq!(MAX_WIDENING, 4);
        let request = |beam, widen| CounterpointRequest {
            beam,
            widen,
            ..free_request("dorian", &[Soprano, Tenor], 8, &[], None, 3)
        };
        assert_eq!(
            request(3, 4).widen,
            CounterpointRequest::new(
                Score::new(ModeSpec::new(Mode::Dorian), Vec::new()),
                Vec::new()
            )
            .widen
        );
        for widen in [1, 2] {
            assert!(matches!(
                generate_counterpoint(&request(3, widen), &Profile::fux()),
                Err(GenerateError::NoCounterpoint { .. })
            ));
        }
        let widened = generate_counterpoint(&request(3, 4), &Profile::fux()).unwrap();
        let widest = generate_counterpoint(&request(12, 1), &Profile::fux()).unwrap();
        assert_eq!(widened[0].score, widest[0].score);
    }

    /// The given voices of a request that sing another text than the new ones are refused.
    #[test]
    fn given_voices_sing_the_same_text() {
        let mut base = cantus_score("dorian", VoiceType::Alto);
        let text = |id: &str| Arc::new(section(id, &TextOptions::default()).unwrap().clone());
        base.text = Some(ScoreText {
            text: text("christe"),
            voices: vec![false],
        });
        let request = CounterpointRequest {
            text: Some(SungText {
                text: text("kyrie"),
                rate: 50,
            }),
            ..CounterpointRequest::new(
                base,
                vec![NewVoice {
                    voice_type: VoiceType::Soprano,
                    species: Species::First,
                }],
            )
        };
        assert_eq!(
            generate_counterpoint(&request, &Profile::fux()).unwrap_err(),
            GenerateError::Invalid("the given voices sing another text".into())
        );
    }

    /// Replayed note by note as the search advances it ([`Underlay::advance`], tie continuations
    /// included, and in an imitation with the pins and the milestones of its entries), every
    /// voice of a texted result keeps underlay states, and they hold the state of the underlay
    /// laid under it: always where no limit cuts the sets, and with the search's limit until it
    /// first cuts one.
    #[test]
    fn search_sets_are_never_empty_on_their_own_result() {
        use crate::text::underlay::{Event, UState};
        use VoiceType::*;
        let profile = Profile::fux();
        let checker = Checker::new(&profile);
        let (mut events, mut held, mut limited, mut pinned) = (0, 0, 0, 0);
        let requests = [
            (
                "2 voices",
                free_request("dorian", &[Soprano, Tenor], 16, &[6], Some("kyrie"), 2),
            ),
            (
                "3 voices",
                free_request("dorian", &[Alto, Tenor, Bass], 16, &[6], Some("kyrie"), 3),
            ),
            (
                "imitation",
                imitative_request(
                    &kyrie_plan(true),
                    &[Alto, Tenor, Bass],
                    Some("kyrie"),
                    1,
                    40,
                ),
            ),
        ];
        for (name, request) in &requests {
            let results = generate_counterpoint(request, &profile).unwrap();
            assert_priced_as_laid(&results[0], request, &profile, name);
            let score = &results[0].score;
            let sung = request.text.as_ref().unwrap();
            let fixed = fixed_by_voice(request);
            let total = request.measures.unwrap() as u32 * 8;
            let underlay = Underlay::new(sung, &request.base, &fixed, total, &checker);
            let (model, levels) = (&underlay.model, &underlay.levels);
            let parts = order_voices(&request.base, &request.voices);
            for (i, part) in parts.iter().enumerate() {
                let Part::New(k) = *part else { continue };
                let label = format!("{name}, voice {i}");
                let notes = &score.voices[i].notes;
                let automaton = underlay.automaton(k);
                let unlimited = Automaton {
                    limit: None,
                    ..automaton
                };
                let (mut set, mut all) = (Arc::new(Automaton::start()), Automaton::start());
                let mut state = UState::START;
                let mut cut = false;
                let mut found = Vec::new();
                for n in 0..notes.len() {
                    let prefix = &notes[..=n];
                    set = underlay
                        .advance(k, &set, prefix)
                        .unwrap_or_else(|rule| panic!("{label}: empty at note {n} ({rule})"));
                    all = unlimited.advance(&all, prefix).unwrap();
                    if let Some(event) = Event::of_last(prefix) {
                        let choice = match (event, notes[n].lyric) {
                            (Event::Attack(_), Some(lyric)) => {
                                Underlaid::Start(lyric.index().unwrap() as u16)
                            }
                            _ => Underlaid::Hold,
                        };
                        state = model
                            .canonical(&model.step(&state, &event, choice, &mut found), levels);
                        events += 1;
                        held += usize::from(matches!(event, Event::Held(_)));
                    }
                    if n + 1 == notes.len() {
                        let end = Event::end(prefix).unwrap();
                        state = model.canonical(
                            &model.step(&state, &end, Underlaid::Hold, &mut found),
                            levels,
                        );
                    }
                    let holds = |states: &Set| states.iter().any(|(s, _, _)| *s == state);
                    assert!(holds(&all), "{label}: note {n}");
                    cut |= set.len() >= LIMIT;
                    assert!(cut || holds(&set), "{label}: note {n}");
                    limited += usize::from(cut && !holds(&set));
                }
                // The underlay laid is the cheapest that keeps the pins, and so is the best state
                // at the end.
                assert_eq!(levels.price(&found).1, None, "{label}");
                let path = model
                    .best_pinned(levels, notes, underlay.pins(k), underlay.gates(k))
                    .unwrap();
                assert_eq!((all[0].1, all[0].2), (path.cost, path.lateness), "{label}");
                pinned += underlay.pins[k].len();
            }
        }
        assert!(
            events >= 30 && held > 0 && pinned >= 30,
            "{events} events, {held} tie continuations, {pinned} pinned attacks"
        );
        println!(
            "{events} events, {held} tie continuations, {pinned} pinned attacks; {limited} times \
             the limit had cut the underlay's state"
        );
    }

    /// Times free settings of sixteen measures without a text and with Kyrie I, in two to four
    /// voices, seeds 1 to 3: the least of three rounds, which alternate the two, against the
    /// noise of a busy machine. Run with
    /// `cargo test --release -p fux-core texted_search_timing -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn texted_search_timing() {
        use VoiceType::*;
        use std::time::Instant;
        const SEEDS: u64 = 3;
        const ROUNDS: usize = 3;
        let profile = Profile::fux();
        let choirs = [
            &[Soprano, Tenor][..],
            &[Alto, Tenor, Bass],
            &[Soprano, Alto, Tenor, Bass],
        ];
        for voices in choirs {
            // Per text and seed: the least time and the result's cost.
            let mut least = [[f64::INFINITY; SEEDS as usize]; 2];
            let mut costs: [[String; SEEDS as usize]; 2] = Default::default();
            for round in 0..ROUNDS {
                for seed in 1..=SEEDS {
                    for (t, text) in [None, Some("kyrie")].into_iter().enumerate() {
                        let request = free_request("dorian", voices, 16, &[6], text, seed);
                        let clock = Instant::now();
                        let results = generate_counterpoint(&request, &profile);
                        let seconds = clock.elapsed().as_secs_f64();
                        let s = seed as usize - 1;
                        least[t][s] = least[t][s].min(seconds);
                        costs[t][s] = match results {
                            Ok(results) => {
                                let mut score = results[0].score.clone();
                                if text.is_some() && round == 0 {
                                    score.fill_syllables(true);
                                    for v in 0..score.voices.len() {
                                        println!("  {}", score.voice_lyrics(v).unwrap());
                                    }
                                }
                                results[0].cost.to_string()
                            }
                            Err(e) => format!("({e})"),
                        };
                    }
                }
            }
            let per_measure = |t: usize| least[t].iter().sum::<f64>() / SEEDS as f64 / 16.0;
            println!(
                "{} voices: {:.3} s per measure untexted (costs {}), {:.3} s with Kyrie I (costs \
                 {}): {:.2} times",
                voices.len(),
                per_measure(0),
                costs[0].join(", "),
                per_measure(1),
                costs[1].join(", "),
                per_measure(1) / per_measure(0)
            );
        }
    }

    /// An entry of a hand-made plan: `voice` enters at `start` after a rest of at least `rest`
    /// (0: its first note), singing the head `head` as `pitch.ticks`, each note with the syllable
    /// it begins after `=`; a note without one holds its syllable.
    fn planned_entry(voice: usize, start: u32, rest: u32, head: &str) -> imitation::Entry {
        let notes = head
            .split_whitespace()
            .map(|note| {
                let (note, sung) = match note.split_once('=') {
                    Some((note, y)) => (note, Underlaid::Start(y.parse().unwrap())),
                    None => (note, Underlaid::Hold),
                };
                let (pitch, ticks) = note.split_once('.').unwrap();
                (pitch.parse().unwrap(), ticks.parse().unwrap(), sung)
            })
            .collect();
        imitation::Entry {
            voice,
            start,
            rest,
            notes,
            relation: imitation::Relation::Same,
            answer: imitation::Answer::Real,
            first: imitation::FirstNote::AsLeader,
        }
    }

    /// A point of a hand-made plan on the syllables `words`, its subject the leader's head, which
    /// declaims the syllables `head`; the cadence in measure `goal` ends it.
    fn planned_point(
        words: std::ops::Range<u16>,
        head: std::ops::Range<u16>,
        kind: imitation::PointKind,
        entries: Vec<imitation::Entry>,
        goal: Option<usize>,
    ) -> imitation::Point {
        let first = entries[0].notes[0].0;
        let notes = entries[0]
            .notes
            .iter()
            .map(|&(pitch, ticks, sung)| imitation::SubjectNote {
                step: (pitch.diatonic() - first.diatonic()) as i8,
                ticks: ticks as u8,
                sung,
                alter: 0,
            })
            .collect();
        let subject = imitation::Soggetto {
            degree: 0,
            notes,
            words: head,
            origin: imitation::Origin::Given,
            answers: Vec::new(),
            stretto: Vec::new(),
            cost: 0,
        };
        imitation::Point {
            words,
            kind,
            subject: Arc::new(subject),
            entries,
            goal,
        }
    }

    /// Kyrie I in three voices (alto, tenor, bass), Dorian: each voice sings the head Ký-ri-e
    /// e-lé in whole notes, D A G F E, the tenor a measure after the alto at the fifth below, the
    /// bass two measures later an octave below the alto, before a cadence on A in measure 10 (from
    /// 0). With `restate`, the three take the head up again after the cadence, its first note a
    /// half on the minim beat after a rest: the tenor, which has no part in the cadence, at G + 4,
    /// then the alto and the bass, which hold its goal. The search finds settings of it with
    /// and without the restatement.
    fn kyrie_plan(restate: bool) -> imitation::ImitationPlan {
        use imitation::PointKind;
        let mode: ModeSpec = "dorian".parse().unwrap();
        let mut points = vec![planned_point(
            0..7,
            0..5,
            PointKind::New,
            vec![
                planned_entry(0, 0, 0, "D4.8=0 A4.8=1 G4.8=2 F4.8=3 E4.8=4"),
                planned_entry(1, 8, 0, "A3.8=0 E4.8=1 D4.8=2 C4.8=3 B3.8=4"),
                planned_entry(2, 24, 0, "D3.8=0 A3.8=1 G3.8=2 F3.8=3 E3.8=4"),
            ],
            Some(10),
        )];
        if restate {
            points.push(planned_point(
                0..7,
                0..5,
                PointKind::Restatement { of: 0 },
                vec![
                    planned_entry(1, 84, 4, "A3.4=0 E4.8=1 D4.8=2 C4.8=3 B3.8=4"),
                    planned_entry(0, 100, 4, "D4.4=0 A4.8=1 G4.8=2 F4.8=3 E4.8=4"),
                    planned_entry(2, 108, 4, "A2.4=0 E3.8=1 D3.8=2 C3.8=3 B2.8=4"),
                ],
                None,
            ));
        }
        let measures = if restate { 23 } else { 20 };
        let cadences = cadences_at(mode, &[10]);
        let text = section("kyrie", &TextOptions::default()).unwrap();
        let planned =
            crate::text::plan::plan(text, mode, 3, Some(measures), Some(cadences.clone()), true);
        imitation::ImitationPlan {
            points,
            measures,
            cadences,
            rate: planned.unwrap().rate,
            cost: 0,
            ending: mode.final_step(),
        }
    }

    /// The search of a plan, as the driver of an imitative setting asks for it: in the Dorian
    /// mode, with the entries and every formula as fixed notes, the plan in the base and the text
    /// `text` at the plan's pace.
    fn imitative_request(
        plan: &imitation::ImitationPlan,
        voices: &[VoiceType],
        text: Option<&str>,
        seed: u64,
        beam: usize,
    ) -> CounterpointRequest {
        let mode: ModeSpec = "dorian".parse().unwrap();
        let mut voices = voices.to_vec();
        voices.sort();
        let mut base = Score::new(mode, Vec::new());
        base.cadences = plan.cadences.clone();
        base.imitation = Some(Arc::new(plan.score_imitation()));
        CounterpointRequest {
            measures: Some(plan.measures),
            fixed: plan.fixed(mode, voices.len()),
            seed,
            beam,
            text: text.map(|id| SungText {
                text: Arc::new(section(id, &TextOptions::default()).unwrap().clone()),
                rate: plan.rate,
            }),
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

    /// Ten untexted measures in two voices: the tenor leads with the head `tenor` from the start,
    /// the soprano enters with the heads `soprano` as (start, rest, head); a beam of `beam`
    /// without noise or widening.
    fn duo(tenor: &str, soprano: &[(u32, u32, &str)], beam: usize) -> CounterpointRequest {
        use VoiceType::*;
        let mut entries = vec![planned_entry(1, 0, 0, tenor)];
        entries.extend(
            soprano
                .iter()
                .map(|&(start, rest, head)| planned_entry(0, start, rest, head)),
        );
        let plan = imitation::ImitationPlan {
            points: vec![planned_point(
                0..7,
                0..5,
                imitation::PointKind::New,
                entries,
                None,
            )],
            measures: 10,
            cadences: Vec::new(),
            rate: 50,
            cost: 0,
            ending: Step::D,
        };
        CounterpointRequest {
            noise: 0.0,
            widen: 1,
            ..imitative_request(&plan, &[Soprano, Tenor], None, 1, beam)
        }
    }

    /// Every note of an entry's head in `request` sounds in its result as planned: at its tick,
    /// with its pitch and value, a continuation tied over from the note before; in a texted
    /// result every pinned attack carries its syllable, and a melisma none. The report has no case
    /// of IMI-ENTRY and nothing forbidden.
    fn assert_sung_as_planned(generated: &Generated, request: &CounterpointRequest, label: &str) {
        let score = &generated.score;
        let parts = order_voices(&request.base, &request.voices);
        for f in request.fixed.iter().filter(|f| f.in_head()) {
            let v = parts
                .iter()
                .position(|part| matches!(*part, Part::New(k) if k == f.voice))
                .unwrap();
            let notes = &score.voices[v].notes;
            let Some((n, note)) = score.voices[v]
                .timed()
                .enumerate()
                .find_map(|(n, (start, note))| (start == f.start).then_some((n, note)))
            else {
                panic!("{label}: voice {v} has no note at {}", f.start);
            };
            let at = format!("{label}: voice {v} at {}", f.start);
            assert_eq!(
                (note.pitch, note.duration),
                (f.note.pitch, f.note.duration),
                "{at}"
            );
            if f.tied_in {
                assert!(n > 0 && notes[n - 1].tie, "{at}: not tied over");
            }
            if score.text.is_some()
                && let Some(sung) = f.sung
            {
                let syllable = match sung {
                    Underlaid::Start(y) => Some(usize::from(y)),
                    Underlaid::Hold => None,
                };
                assert_eq!(note.lyric.and_then(|l| l.index()), syllable, "{at}");
            }
        }
        let report = &generated.report;
        assert_eq!(report.forbidden(), 0, "{label}: {:?}", report.violations);
        assert!(
            report.violations.iter().all(|v| v.rule != ENTRY_RULE),
            "{label}: {:?}",
            report.violations
        );
    }

    /// Until its first entry a voice rests, a measure at a time and then up to the entry, also
    /// where it enters on the minim beat; from there on it sings its head as planned.
    #[test]
    fn voices_are_silent_until_their_first_entry() {
        use VoiceType::*;
        let mut plan = kyrie_plan(false);
        // The tenor enters on the minim beat of the second measure, with a half.
        plan.points[0].entries[1] = planned_entry(1, 12, 0, "A3.4=0 E4.8=1 D4.8=2 C4.8=3 B3.8=4");
        for text in [None, Some("kyrie")] {
            let label = format!("text {text:?}");
            let request = imitative_request(&plan, &[Alto, Tenor, Bass], text, 1, 40);
            let results = generate_counterpoint(&request, &Profile::fux()).unwrap();
            assert_sung_as_planned(&results[0], &request, &label);
            for (v, entry) in [(0, 0), (1, 12), (2, 24)] {
                let silence: Vec<(u32, u32)> = results[0].score.voices[v]
                    .timed()
                    .take_while(|&(start, _)| start < entry)
                    .map(|(start, note)| {
                        assert!(note.is_rest(), "{label}, voice {v}: {note:?} at {start}");
                        (start, note.duration.ticks())
                    })
                    .collect();
                let measures: Vec<(u32, u32)> = (0..entry)
                    .step_by(8)
                    .map(|start| (start, (entry - start).min(8)))
                    .collect();
                assert_eq!(silence, measures, "{label}, voice {v}");
            }
        }
    }

    /// Before an entry after a rest the voice is silent for at least the planned rest: its notes
    /// end before the rest window, its rests at the entry, and in the window the only choice is
    /// the rest up to the entry or the next barline. A quarter rest where the window begins would
    /// lead into `r4 2 4`, whose half runs into the entry, a dead end that only the next step
    /// would find. A search with three re-entries keeps their windows so.
    #[test]
    fn re_entries_keep_their_rest_window() {
        use Choice::{Pitch as Sung, Rest, Tie};
        use Duration as D;
        use VoiceType::*;
        let d5: Pitch = "D5".parse().unwrap();
        let entry = |start, rest| Fixed {
            voice: 0,
            start,
            note: Note::new(d5, D::WHOLE),
            tied_in: false,
            any_octave: false,
            role: FixedRole::Entry { rest },
            sung: None,
        };
        let all = vec![
            Rest(D::QUARTER),
            Rest(D::HALF),
            Rest(D::WHOLE),
            Sung(d5, D::QUARTER),
            Sung(d5, D::HALF),
            Sung(d5, D::DOTTED_HALF),
            Tie(D::HALF),
        ];
        let before = [Note::new(d5, D::WHOLE)];
        // An entry on the minim beat at 12 after a half rest: its window is [8, 12).
        let minim = entry(12, 4);
        let agreed =
            |tick: u32, choices: &[Choice]| agree(choices.to_vec(), &[&minim], &before, tick, 8);
        assert_eq!(agreed(8, &all), [Rest(D::HALF)]);
        assert_eq!(agreed(10, &all), [Rest(D::QUARTER)]);
        assert!(agreed(8, &[Rest(D::QUARTER), Sung(d5, D::HALF)]).is_empty());
        assert_eq!(
            agreed(4, &all),
            [
                Rest(D::QUARTER),
                Rest(D::HALF),
                Rest(D::WHOLE),
                Sung(d5, D::QUARTER),
                Sung(d5, D::HALF),
                Tie(D::HALF)
            ]
        );
        // A window over a barline: rests up to the barline, then up to the entry.
        let long = entry(20, 8);
        for tick in [12, 16] {
            assert_eq!(
                agree(all.clone(), &[&long], &before, tick, 8),
                [Rest(D::HALF)]
            );
        }
        // An entry without a rest, as before.
        let close = entry(12, 0);
        assert_eq!(
            agree(all.clone(), &[&close], &before, 8, 8),
            [
                Rest(D::QUARTER),
                Rest(D::HALF),
                Sung(d5, D::QUARTER),
                Sung(d5, D::HALF),
                Tie(D::HALF)
            ]
        );

        let plan = kyrie_plan(true);
        for text in [None, Some("kyrie")] {
            let label = format!("text {text:?}");
            let request = imitative_request(&plan, &[Alto, Tenor, Bass], text, 1, 40);
            let results = generate_counterpoint(&request, &Profile::fux()).unwrap();
            assert_sung_as_planned(&results[0], &request, &label);
            let mut windows = 0;
            for e in plan
                .points
                .iter()
                .flat_map(|p| &p.entries)
                .filter(|e| e.rest > 0)
            {
                let (from, until) = (e.start - e.rest, e.start);
                for (start, note) in results[0].score.voices[e.voice].timed() {
                    let end = start + note.duration.ticks();
                    if end <= from || start >= until {
                        continue;
                    }
                    let at = format!("{label}, voice {} at {start}", e.voice);
                    assert!(note.is_rest(), "{at}: {note:?}");
                    if start >= from {
                        assert_eq!(end, until.min((start / 8 + 1) * 8), "{at}");
                    }
                }
                windows += 1;
            }
            assert_eq!(windows, 3);
        }
    }

    /// The notes of an entry's head are set before the voices below them choose theirs, so that
    /// those are judged against them. With a single state in the beam and no noise, the tenor,
    /// which chooses first, must take the final D under the soprano's entry (two voices begin on a
    /// perfect consonance over the final, CAD-START): on D5 a step up from its C4, under A4 a
    /// third down from its F4. Judged against nothing, its cheapest note leaves the entry no such
    /// consonance, and the search ends there.
    #[test]
    fn entries_are_set_before_the_lower_voices() {
        for (tenor, soprano, under) in [
            ("D4.8 C4.8", "D5.8 C5.8 B4.8 A4.8", "D4"),
            ("D4.8 F4.8", "A4.8 G4.8 F4.8 E4.8", "D4"),
        ] {
            let request = duo(tenor, &[(16, 0, soprano)], 1);
            let results = generate_counterpoint(&request, &Profile::fux()).unwrap();
            assert_sung_as_planned(&results[0], &request, soprano);
            let at_entry = results[0].score.voices[1]
                .timed()
                .find_map(|(start, note)| (start == 16).then_some(note.pitch))
                .flatten();
            assert_eq!(at_entry, under.parse().ok(), "{soprano}");
        }
    }

    /// Where a voice cannot take an entry's note (an eighth on the minim beat, which no rhythm of
    /// free counterpoint begins) or cannot rest in the window before one (after a head that ends
    /// in quarters), the search fails at the entry and names IMI-ENTRY or IMI-REST.
    #[test]
    fn impossible_entries_name_their_rule() {
        let fails = |soprano: &[(u32, u32, &str)]| {
            generate_counterpoint(&duo("D4.8 C4.8", soprano, 20), &Profile::fux()).unwrap_err()
        };
        assert_eq!(
            fails(&[(12, 0, "D5.1 C5.1 B4.2 A4.4")]),
            GenerateError::NoCounterpoint {
                measure: 2,
                rules: vec![ENTRY_RULE]
            }
        );
        assert_eq!(
            fails(&[(16, 0, "D5.8 C5.6 B4.2 A4.2 G4.2"), (40, 4, "D5.8 C5.8")]),
            GenerateError::NoCounterpoint {
                measure: 5,
                rules: vec![REST_RULE]
            }
        );
    }

    /// A head may sing a pitch that its voice's pool lacks, B♭ in the Mixolydian mode; the free
    /// counterpoint keeps to the pool: no B♭ sounds outside the heads.
    #[test]
    fn head_pitches_stay_in_their_heads() {
        use VoiceType::*;
        let mode: ModeSpec = "mixolydian".parse().unwrap();
        let plan = imitation::ImitationPlan {
            points: vec![planned_point(
                0..7,
                0..5,
                imitation::PointKind::New,
                vec![
                    planned_entry(0, 0, 0, "G4.8=0 F4.8=1 Bb4.8=2 A4.8=3 G4.8=4"),
                    planned_entry(1, 16, 0, "G3.8=0 F3.8=1 Bb3.8=2 A3.8=3 G3.8=4"),
                    planned_entry(2, 48, 0, "G2.8=0 F2.8=1 Bb2.8=2 A2.8=3 G2.8=4"),
                ],
                Some(13),
            )],
            measures: 22,
            cadences: cadences_at(mode, &[13]),
            rate: 50,
            cost: 0,
            ending: mode.final_step(),
        };
        for seed in 1..=2 {
            let mut request = imitative_request(&plan, &[Alto, Tenor, Bass], None, seed, 40);
            request.base.mode = mode;
            request.fixed = plan.fixed(mode, 3);
            let results = generate_counterpoint(&request, &Profile::fux()).unwrap();
            let heads: Vec<(usize, u32)> = request
                .fixed
                .iter()
                .filter(|f| f.in_head())
                .map(|f| (f.voice, f.start))
                .collect();
            for (v, voice) in results[0].score.voices.iter().enumerate() {
                for (start, note) in voice.timed() {
                    let flat = note.pitch.is_some_and(|p| p.step == Step::B && p.alter < 0);
                    assert!(
                        !flat || heads.contains(&(v, start)),
                        "seed {seed}: voice {v} sings B♭ at {start}"
                    );
                }
            }
        }
    }

    /// A head that runs into the tied preparation of a cadence formula leaves the voice no note
    /// where the formula begins: the search fails there and names the entry, not nothing. A
    /// fixed note before a voice's first entry, which it could never sing, is an invalid request.
    #[test]
    fn heads_into_a_formula_name_their_entry() {
        use VoiceType::*;
        let mode: ModeSpec = "dorian".parse().unwrap();
        let plan = |goal: usize| imitation::ImitationPlan {
            points: vec![planned_point(
                0..7,
                0..5,
                imitation::PointKind::New,
                vec![
                    planned_entry(1, 0, 0, "D4.16=0 A4.8=1 G4.8=2 F4.8=3 E4.8=4"),
                    planned_entry(0, 8, 0, "A4.16=0 E5.8=1 D5.8=2 C5.8=3 B4.8=4"),
                ],
                Some(goal),
            )],
            measures: 14,
            cadences: cadences_at(mode, &[goal]),
            rate: 50,
            cost: 0,
            ending: mode.final_step(),
        };
        let request = imitative_request(&plan(8), &[Soprano, Tenor], None, 1, 40);
        assert_eq!(
            generate_counterpoint(&request, &Profile::fux()).unwrap_err(),
            GenerateError::NoCounterpoint {
                measure: 8,
                rules: vec![ENTRY_RULE]
            }
        );
        // A measure later the formula leaves room for the head.
        let request = imitative_request(&plan(9), &[Soprano, Tenor], None, 1, 40);
        assert!(generate_counterpoint(&request, &Profile::fux()).is_ok());
        // The soprano's first entry after its cantizans of a cadence in measure 4.
        let late = imitation::ImitationPlan {
            points: vec![planned_point(
                0..7,
                0..5,
                imitation::PointKind::New,
                vec![
                    planned_entry(1, 0, 0, "D4.8=0 A4.8=1 G4.8=2 F4.8=3 E4.8=4"),
                    planned_entry(0, 48, 0, "A4.8=0 E5.8=1 D5.8=2 C5.8=3 B4.8=4"),
                ],
                Some(4),
            )],
            measures: 12,
            cadences: cadences_at(mode, &[4]),
            rate: 50,
            cost: 0,
            ending: mode.final_step(),
        };
        let request = imitative_request(&late, &[Soprano, Tenor], None, 1, 40);
        assert!(matches!(
            generate_counterpoint(&request, &Profile::fux()),
            Err(GenerateError::Invalid(_))
        ));
    }

    /// A smoke search of hand-made plans, as the driver of an imitative setting runs one: Kyrie I
    /// in three voices with a beam of 40, every voice with the head of Ký-ri-e e-lé, once and
    /// again after the internal cadence. Every entry sounds as planned with its syllables, the
    /// underlays keep the pins at their least cost, nothing is forbidden, and the plan travels
    /// in the result.
    #[test]
    fn hand_made_plans_are_sung_as_planned() {
        use VoiceType::*;
        let profile = Profile::fux();
        for restate in [false, true] {
            let label = if restate { "restated" } else { "one point" };
            let plan = kyrie_plan(restate);
            let request = imitative_request(&plan, &[Alto, Tenor, Bass], Some("kyrie"), 1, 40);
            let results = generate_counterpoint(&request, &profile).unwrap();
            assert_sung_as_planned(&results[0], &request, label);
            assert_priced_as_laid(&results[0], &request, &profile, label);
            let score = &results[0].score;
            assert_eq!(score.measure_count(), plan.measures, "{label}");
            assert_eq!(
                score.imitation.as_deref(),
                Some(&plan.score_imitation()),
                "{label}"
            );
        }
    }

    /// Times the searches of the hand-made plans of Kyrie I in three voices against texted
    /// searches without imitation of the same length and cadences, seeds 1 to 3, beam 40: the
    /// least of three rounds, which alternate the two. Run with
    /// `cargo test --release -p fux-core imitative_search_timing -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn imitative_search_timing() {
        use VoiceType::*;
        use std::time::Instant;
        const SEEDS: u64 = 3;
        const ROUNDS: usize = 3;
        let profile = Profile::fux();
        let voices = [Alto, Tenor, Bass];
        for restate in [false, true] {
            let plan = kyrie_plan(restate);
            // Per kind of search and seed: the least time and the result's cost.
            let mut least = [[f64::INFINITY; SEEDS as usize]; 2];
            let mut costs: [[String; SEEDS as usize]; 2] = Default::default();
            for _ in 0..ROUNDS {
                for seed in 1..=SEEDS {
                    let imitative = imitative_request(&plan, &voices, Some("kyrie"), seed, 40);
                    let mut plain =
                        free_request("dorian", &voices, plan.measures, &[10], Some("kyrie"), seed);
                    plain.beam = 40;
                    plain.text.as_mut().unwrap().rate = plan.rate;
                    for (i, request) in [imitative, plain].iter().enumerate() {
                        let clock = Instant::now();
                        let results = generate_counterpoint(request, &profile);
                        let seconds = clock.elapsed().as_secs_f64();
                        let s = seed as usize - 1;
                        least[i][s] = least[i][s].min(seconds);
                        costs[i][s] = match results {
                            Ok(results) => results[0].cost.to_string(),
                            Err(e) => format!("({e})"),
                        };
                    }
                }
            }
            let per_measure =
                |i: usize| least[i].iter().sum::<f64>() / SEEDS as f64 / plan.measures as f64;
            println!(
                "{} measures, {}: {:.4} s per measure imitative (costs {}), {:.4} s without \
                 imitation (costs {}): {:.2} times",
                plan.measures,
                if restate { "two points" } else { "one point" },
                per_measure(0),
                costs[0].join(", "),
                per_measure(1),
                costs[1].join(", "),
                per_measure(0) / per_measure(1)
            );
        }
    }
}
