//! The rule engine: rules with ids, levels and parameters, profiles that set them, and the checker
//! that evaluates a score.
//!
//! Every rule reports [`Violation`]s with a [`Level`]: a cost or a prohibition. The same code
//! serves the checker (the whole score) and the search (only what the newest notes add): a rule
//! reports a violation at the tick at which it is decided, the start of the last note involved,
//! and only violations inside the [`Window`] count. Rules about the whole piece report only when
//! the window says the score is complete.

mod analysis;
mod cadence;
mod cantus;
mod clausula;
#[cfg(test)]
mod clausula_tests;
mod dissonance;
mod ficta;
mod free;
mod imitation;
#[cfg(test)]
mod imitation_tests;
mod melodic;
mod motion;
mod profile;
#[cfg(test)]
mod rule_tests;
mod species;
#[cfg(test)]
mod species_tests;
mod text;
#[cfg(test)]
mod text_tests;
mod texture;
#[cfg(test)]
mod texture_tests;
mod vertical;
mod voices;

use std::fmt;

pub use analysis::{Analysis, CadencePoint, Motion, Transition, harmonic};
pub(crate) use free::{modal_octave, principal};
pub use profile::{BUILTIN_PROFILES, Profile, ProfileError};

use crate::interval::Interval;
use crate::lang::Lang;
use crate::pitch::Pitch;
use crate::score::{Score, Sounding};

/// How strictly a rule, or one case of it, is enforced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Level {
    /// Not checked.
    Off,
    /// Allowed at this cost.
    Cost(u32),
    /// Forbidden.
    Forbidden,
}

impl Level {
    pub const P1: Level = Level::Cost(1);
    pub const P2: Level = Level::Cost(2);
    pub const P3: Level = Level::Cost(4);
    pub const P4: Level = Level::Cost(8);

    /// Parses `aus`/`off`/`frei`, `V`/`verboten`/`forbidden`, `P1`–`P4`, `P(n)` or a number `n`.
    pub fn parse(text: &str) -> Option<Level> {
        let t = text.trim();
        match t.to_lowercase().as_str() {
            "aus" | "off" | "frei" | "free" => return Some(Level::Off),
            "v" | "verboten" | "forbidden" => return Some(Level::Forbidden),
            "p1" => return Some(Level::P1),
            "p2" => return Some(Level::P2),
            "p3" => return Some(Level::P3),
            "p4" => return Some(Level::P4),
            _ => {}
        }
        let number = t
            .strip_prefix("P(")
            .or_else(|| t.strip_prefix("p("))
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(t);
        number.parse().ok().map(Level::Cost)
    }

    pub const fn cost(self) -> u32 {
        match self {
            Level::Cost(c) => c,
            Level::Off | Level::Forbidden => 0,
        }
    }

    /// Short label: `aus`/`off`, `V`, `P1`–`P4`, `P(n)`.
    pub fn label(self, lang: Lang) -> String {
        match self {
            Level::Off => lang.pick("aus", "off").to_string(),
            Level::Forbidden => "V".to_string(),
            Level::Cost(1) => "P1".to_string(),
            Level::Cost(2) => "P2".to_string(),
            Level::Cost(4) => "P3".to_string(),
            Level::Cost(8) => "P4".to_string(),
            Level::Cost(n) => format!("P({n})"),
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label(Lang::De))
    }
}

/// Kind of a rule parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamKind {
    Level,
    Int,
    Bool,
}

/// A rule parameter: its name in profiles and on the command line, and a description.
#[derive(Clone, Copy, Debug)]
pub struct ParamInfo {
    pub name: &'static str,
    pub kind: ParamKind,
    pub de: &'static str,
    pub en: &'static str,
}

/// The beginning of the names of the parameters that extend a rule of imitation to settings
/// without a plan (IMI-HELD `unplanned_struck` and `unplanned_passing`, IMI-SILENCE
/// `unplanned_thin`; phase 14).
const UNPLANNED: &str = "unplanned_";

impl ParamInfo {
    /// Whether a level for the whole rule (`ID=LEVEL`, [`Profile::apply_override`]) sets this
    /// parameter: every level parameter except those that extend a rule of imitation to settings
    /// without a plan. They change only with the profile of a mass ([`crate::mass::profile`]),
    /// where an override names them (`IMI-HELD.unplanned_struck=V`) or where the whole rule is
    /// switched off (`IMI-HELD=aus`, which also takes back the profile of a mass), so that
    /// `IMI-HELD=V` or `IMI-SILENCE=P1` judges the scores it judged before they came, the corpora
    /// and every setting without imitation (decision D15 of phase 14).
    pub fn follows_rule(&self) -> bool {
        self.kind == ParamKind::Level && !self.name.starts_with(UNPLANNED)
    }
}

/// Value of a rule parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamValue {
    Level(Level),
    Int(i64),
    Bool(bool),
}

impl fmt::Display for ParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamValue::Level(level) => write!(f, "{level}"),
            ParamValue::Int(n) => write!(f, "{n}"),
            ParamValue::Bool(b) => write!(f, "{b}"),
        }
    }
}

/// Fixed description of a rule.
#[derive(Debug)]
pub struct RuleInfo {
    /// Stable id, as in `docs/regeln.md`.
    pub id: &'static str,
    pub title_de: &'static str,
    pub title_en: &'static str,
    /// Where the rule comes from (Fux page, FuxCP rule, corpus, tradition).
    pub source: &'static str,
    pub params: &'static [ParamInfo],
}

impl RuleInfo {
    pub fn title(&self, lang: Lang) -> &'static str {
        lang.pick(self.title_de, self.title_en)
    }

    pub fn param(&self, name: &str) -> Option<&'static ParamInfo> {
        self.params.iter().find(|p| p.name == name)
    }
}

/// The values of one rule's parameters in a profile.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleSettings {
    values: Vec<(&'static str, ParamValue)>,
}

impl RuleSettings {
    /// The value of a parameter, if set.
    pub fn get(&self, name: &str) -> Option<ParamValue> {
        self.values
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
    }

    pub(crate) fn set(&mut self, name: &'static str, value: ParamValue) {
        match self.values.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 = value,
            None => self.values.push((name, value)),
        }
    }

    /// A level parameter; `Off` if missing.
    pub fn level(&self, name: &str) -> Level {
        match self.get(name) {
            Some(ParamValue::Level(level)) => level,
            _ => Level::Off,
        }
    }

    /// An integer parameter; 0 if missing.
    pub fn int(&self, name: &str) -> i64 {
        match self.get(name) {
            Some(ParamValue::Int(n)) => n,
            _ => 0,
        }
    }

    pub fn flag(&self, name: &str) -> bool {
        matches!(self.get(name), Some(ParamValue::Bool(true)))
    }

    /// All parameters with their values, in the order of the rule's description.
    pub fn values(&self) -> &[(&'static str, ParamValue)] {
        &self.values
    }

    /// Whether every level parameter is `Off`.
    pub fn is_off(&self) -> bool {
        self.values
            .iter()
            .filter_map(|(_, v)| match v {
                ParamValue::Level(level) => Some(level),
                _ => None,
            })
            .all(|level| *level == Level::Off)
    }
}

/// Extra information about a violation, used to build its message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detail {
    None,
    Interval(Interval),
    Pitch(Pitch),
    Pitches(Pitch, Pitch),
    Count(u32),
    /// A measure rhythm and the parameter of the rule that sets its level.
    Rhythm(&'static str, &'static str),
    /// The parameter of the rule that sets the level of the case found, for rules with several
    /// cases.
    Param(&'static str),
}

/// A rule violation in a score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Violation {
    pub rule: &'static str,
    /// `Cost` or `Forbidden`, never `Off`.
    pub level: Level,
    pub voice: usize,
    /// Index into the voice's notes of the note to mark.
    pub note: usize,
    /// The other voice for rules about two voices.
    pub other: Option<usize>,
    /// Start of the last note involved; the violation is decided at this tick.
    pub tick: u32,
    pub detail: Detail,
}

impl Violation {
    pub fn is_forbidden(&self) -> bool {
        self.level == Level::Forbidden
    }
}

/// Which violations a check reports: those decided between `from` and `until` (inclusive).
///
/// The search adds the notes starting at one tick per step and checks the window from just after
/// the previous step to that tick, so every violation counts exactly once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub from: u32,
    pub until: u32,
    /// The score is complete, so rules about the whole piece may report.
    pub complete: bool,
}

impl Window {
    /// The whole, complete score.
    pub const ALL: Window = Window {
        from: 0,
        until: u32::MAX,
        complete: true,
    };

    /// Part of an incomplete score.
    pub const fn partial(from: u32, until: u32) -> Window {
        Window {
            from,
            until,
            complete: false,
        }
    }

    /// Whether a violation decided at `tick` lies in the window.
    pub const fn contains(&self, tick: u32) -> bool {
        self.from <= tick && tick <= self.until
    }

    /// Whether violations decided between `from` and `until` may lie in the window.
    pub const fn contains_range(&self, from: u32, until: u32) -> bool {
        from <= self.until && self.from <= until
    }
}

/// What a rule sees while checking.
pub struct Ctx<'a> {
    pub score: &'a Score,
    pub analysis: &'a Analysis,
    pub settings: &'a RuleSettings,
    /// Only violations decided in this window are reported.
    pub window: Window,
    /// The score is complete, so rules about the whole piece may report (`window.complete`).
    pub complete: bool,
    /// Voices whose violations count; the others are given (e.g. a fixed cantus firmus).
    pub targets: &'a [bool],
    /// What several rules of one check need, found once.
    pub shared: &'a Shared,
}

/// Facts that several rules of one check need and that do not depend on their settings, found
/// once per check of a window.
#[derive(Default)]
pub struct Shared {
    suspensions: std::cell::OnceCell<Vec<species::Suspension>>,
    /// The cases of the text underlay in the voices that sing (see `text::cases`).
    text: std::cell::OnceCell<text::Cases>,
}

impl Ctx<'_> {
    /// Whether violations of voice `v` count.
    pub fn is_target(&self, v: usize) -> bool {
        self.targets.get(v).copied().unwrap_or(true)
    }

    /// Whether voice `v` is tacet: it sings none of the score's text and has no note, as a
    /// voice of a movement of a mass rests through a section that it does not sing (phase 14,
    /// `mass::section_score`). The rules that count the voices of a choir leave it out. In a
    /// setting of its own every voice sings its text or has notes from its start (a cantus
    /// firmus), and a score without a text has no tacet voice.
    pub fn tacet(&self, v: usize) -> bool {
        self.score
            .text
            .as_ref()
            .is_some_and(|sung| sung.voices.get(v) == Some(&false))
            && self.analysis.lines.get(v).is_some_and(Vec::is_empty)
    }

    /// Whether a violation decided at `tick` or earlier would lie before the window. Rules use
    /// it to skip work the search does not need.
    pub fn before_window(&self, tick: u32) -> bool {
        tick < self.window.from
    }

    /// Indices of the slices whose tick lies in the window.
    pub fn slice_range(&self) -> std::ops::Range<usize> {
        let slices = &self.analysis.slices;
        let start = slices.partition_point(|s| s.tick < self.window.from);
        let end = slices.partition_point(|s| s.tick <= self.window.until);
        start..end.max(start)
    }

    /// Whether the tick lies in the last measure. A free score has no length given in advance:
    /// until the check of the complete score nothing lies in its last measure.
    pub fn in_last_measure(&self, tick: u32) -> bool {
        (self.complete || !self.score.is_free()) && self.analysis.in_last_measure(tick)
    }

    /// Indices of the downbeat slices in the window.
    pub fn downbeats(&self) -> impl Iterator<Item = usize> + '_ {
        self.slice_range()
            .filter(|&i| self.analysis.is_downbeat(self.analysis.slices[i].tick))
    }

    /// Consecutive notes of a voice without a rest between them, whose second note starts in the
    /// window: (index of the second in the line, first, second).
    pub fn melodic_pairs(
        &self,
        voice: usize,
    ) -> impl Iterator<Item = (usize, Sounding, Sounding)> + '_ {
        let line = &self.analysis.lines[voice];
        let start = line.partition_point(|s| s.start < self.window.from).max(1);
        let until = self.window.until;
        (start..line.len())
            .take_while(move |&i| line[i].start <= until)
            .filter(move |&i| line[i - 1].end == line[i].start)
            .map(move |i| (i, line[i - 1], line[i]))
    }

    /// Index of the first note of a line that starts in the window or later.
    pub fn first_in_window(&self, line: &[Sounding]) -> usize {
        line.partition_point(|s| s.start < self.window.from)
    }

    /// Records a violation between two voices, attributed to `a` if it is a target voice,
    /// otherwise to `b`; nothing if neither is.
    #[allow(clippy::too_many_arguments)]
    pub fn report_pair(
        &self,
        out: &mut Vec<Violation>,
        rule: &'static str,
        level: Level,
        (a, note_a): (usize, usize),
        (b, note_b): (usize, usize),
        tick: u32,
        detail: Detail,
    ) {
        let (voice, note, other) = if self.is_target(a) {
            (a, note_a, b)
        } else if self.is_target(b) {
            (b, note_b, a)
        } else {
            return;
        };
        if level != Level::Off && self.window.contains(tick) {
            out.push(Violation {
                rule,
                level,
                voice,
                note,
                other: Some(other),
                tick,
                detail,
            });
        }
    }

    /// Records a violation unless its level is `Off`, it lies before the window or concerns no
    /// target voice.
    #[allow(clippy::too_many_arguments)]
    pub fn report(
        &self,
        out: &mut Vec<Violation>,
        rule: &'static str,
        level: Level,
        voice: usize,
        note: usize,
        tick: u32,
        detail: Detail,
    ) {
        if level != Level::Off && self.window.contains(tick) && self.is_target(voice) {
            out.push(Violation {
                rule,
                level,
                voice,
                note,
                other: None,
                tick,
                detail,
            });
        }
    }
}

/// A rule of counterpoint.
pub trait Rule: Send + Sync {
    fn info(&self) -> &'static RuleInfo;

    /// Reports the violations in `ctx.score` that lie in the window of `ctx`.
    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>);

    /// Message for a violation; by default the rule's title with the detail.
    fn message(&self, violation: &Violation, lang: Lang) -> String {
        let title = self.info().title(lang);
        match violation.detail {
            Detail::None => title.to_string(),
            Detail::Interval(i) => format!("{title}: {}", i.abs().name(lang)),
            Detail::Pitch(p) => format!("{title}: {}", p.name(lang)),
            Detail::Pitches(a, b) => format!("{title}: {}–{}", a.name(lang), b.name(lang)),
            Detail::Count(n) => format!("{title} ({n})"),
            Detail::Rhythm(pattern, _) => format!("{title}: {pattern}"),
            Detail::Param(name) => match self.info().param(name) {
                Some(param) => format!("{title}: {}", lang.pick(param.de, param.en)),
                None => title.to_string(),
            },
        }
    }
}

/// All rules, in the order of `docs/regeln.md`.
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = Vec::new();
    rules.extend(cantus::rules());
    rules.extend(melodic::rules());
    rules.extend(vertical::rules());
    rules.extend(motion::rules());
    rules.extend(cadence::rules());
    rules.extend(clausula::inner_rules());
    rules.extend(ficta::rules());
    rules.extend(voices::rules());
    rules.extend(species::rules());
    rules.extend(free::rules());
    rules.extend(text::rules());
    rules.extend(imitation::rules());
    rules.extend(texture::rules());
    rules
}

/// The rule with the given id.
pub fn rule(id: &str) -> Option<Box<dyn Rule>> {
    all_rules().into_iter().find(|r| r.info().id == id)
}

/// Result of checking a score.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub violations: Vec<Violation>,
}

impl Report {
    pub fn forbidden(&self) -> usize {
        self.violations.iter().filter(|v| v.is_forbidden()).count()
    }

    pub fn cost(&self) -> u32 {
        self.violations.iter().map(|v| v.level.cost()).sum()
    }
}

/// Checks rules on a score. Built once per profile, used for whole scores and by the search.
pub struct Checker {
    rules: Vec<(Box<dyn Rule>, RuleSettings)>,
    /// The order in which [`Checker::check_window_until_forbidden`] takes the rules (indices
    /// into `rules`): first those with a level `Forbidden` in the profile, the ones that forbid
    /// most often in front, then the rules that only cost.
    search: Vec<usize>,
    /// How many rules of `search` may forbid.
    forbidding: usize,
}

/// The rules that forbid the most candidates of the search, cheap melodic ones first (counted
/// on a Gloria and a Kyrie, 28 Oct 2026); the search checks them first.
const OFTEN_FORBIDDING: [&str; 7] = [
    "MEL-MIN6-DOWN",
    "G3-ACCENT-LEAP",
    "HAR-CHROMATIC-CLASH",
    "HAR-DISSONANCE",
    "FREE-PASSING",
    "FREE-RESOLUTION",
    "FIC-SCOPE",
];

impl Checker {
    pub fn new(profile: &Profile) -> Checker {
        let rules: Vec<_> = all_rules()
            .into_iter()
            .map(|rule| {
                let settings = profile.settings(rule.info().id).clone();
                (rule, settings)
            })
            .filter(|(_, settings)| !settings.is_off())
            .collect();
        let may_forbid = |i: usize| {
            rules[i]
                .1
                .values()
                .iter()
                .any(|(_, v)| *v == ParamValue::Level(Level::Forbidden))
        };
        let mut search: Vec<usize> = (0..rules.len()).collect();
        search.sort_by_key(|&i| {
            let often = OFTEN_FORBIDDING
                .iter()
                .position(|id| *id == rules[i].0.info().id)
                .unwrap_or(OFTEN_FORBIDDING.len());
            (!may_forbid(i), often)
        });
        let forbidding = search.iter().filter(|&&i| may_forbid(i)).count();
        Checker {
            rules,
            search,
            forbidding,
        }
    }

    /// As [`Checker::check_window`], for the search, which needs to know whether a candidate is
    /// forbidden and only otherwise what it costs: the check ends at the first rule that
    /// forbids, and the rules that only cost run only when none did (with `costs`; without,
    /// never). Returns whether the candidate is forbidden. Its violations then name only the
    /// first rule that forbids it; for the diagnosis of a dead end the search checks such a
    /// candidate again with [`Checker::check_window`].
    pub fn check_window_until_forbidden(
        &self,
        score: &Score,
        analysis: &Analysis,
        window: Window,
        targets: &[bool],
        out: &mut Vec<Violation>,
        costs: bool,
    ) -> bool {
        let shared = Shared::default();
        for (n, &i) in self.search.iter().enumerate() {
            if n == self.forbidding && !costs {
                break;
            }
            let (rule, settings) = &self.rules[i];
            let ctx = Ctx {
                score,
                analysis,
                settings,
                window,
                complete: window.complete,
                targets,
                shared: &shared,
            };
            let before = out.len();
            rule.check(&ctx, out);
            if out[before..].iter().any(Violation::is_forbidden) {
                return true;
            }
        }
        false
    }

    /// Checks the whole score; all voices count.
    pub fn check(&self, score: &Score) -> Report {
        self.check_targets(score, &vec![true; score.voices.len()])
    }

    /// Checks the whole score; only violations of the target voices count.
    pub fn check_targets(&self, score: &Score, targets: &[bool]) -> Report {
        let analysis = Analysis::new(score);
        let mut violations = Vec::new();
        self.check_window(score, &analysis, Window::ALL, targets, &mut violations);
        // In the order of the marked notes, which is how they are shown.
        violations.sort_by_key(|v| {
            let at = score.note_start(v.voice, v.note).unwrap_or(v.tick);
            (at, v.voice, v.tick, v.rule)
        });
        Report { violations }
    }

    /// The setting of a rule in the profile this checker was built with.
    pub fn level(&self, rule: &str, param: &str) -> Level {
        self.rules
            .iter()
            .find(|(r, _)| r.info().id == rule)
            .map_or(Level::Off, |(_, settings)| settings.level(param))
    }

    /// An integer parameter of a rule in the profile this checker was built with; 0 if the rule
    /// is switched off or has no such parameter.
    pub fn int(&self, rule: &str, param: &str) -> i64 {
        self.rules
            .iter()
            .find(|(r, _)| r.info().id == rule)
            .map_or(0, |(_, settings)| settings.int(param))
    }

    /// Checks only violations decided in the window, for the target voices.
    pub fn check_window(
        &self,
        score: &Score,
        analysis: &Analysis,
        window: Window,
        targets: &[bool],
        out: &mut Vec<Violation>,
    ) {
        let shared = Shared::default();
        for (rule, settings) in &self.rules {
            let ctx = Ctx {
                score,
                analysis,
                settings,
                window,
                complete: window.complete,
                targets,
                shared: &shared,
            };
            rule.check(&ctx, out);
        }
    }

    /// Message for a violation.
    pub fn message(&self, violation: &Violation, lang: Lang) -> String {
        self.rules
            .iter()
            .find(|(rule, _)| rule.info().id == violation.rule)
            .map(|(rule, _)| rule.message(violation, lang))
            .unwrap_or_else(|| violation.rule.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        assert_eq!(Level::parse("V"), Some(Level::Forbidden));
        assert_eq!(Level::parse("aus"), Some(Level::Off));
        assert_eq!(Level::parse("P3"), Some(Level::Cost(4)));
        assert_eq!(Level::parse("P(6)"), Some(Level::Cost(6)));
        assert_eq!(Level::parse("16"), Some(Level::Cost(16)));
        assert_eq!(Level::parse("x"), None);
        assert_eq!(Level::Cost(8).label(Lang::De), "P4");
        assert_eq!(Level::Cost(6).label(Lang::En), "P(6)");
    }

    /// A level for the whole rule leaves the three parameters for settings without a plan of
    /// imitation as they are, off in the standard profile and at the levels of the mass in its
    /// profile; an override that names them sets them, and so does switching the whole rule off,
    /// after the profile of a mass too. Every other level parameter follows the rule; the other
    /// parameters keep their values when it is switched off.
    #[test]
    fn whole_rule_levels_leave_the_unplanned_parameters() {
        let mut profile = Profile::fux();
        profile.apply_override("IMI-HELD=V").unwrap();
        profile.apply_override("IMI-SILENCE=P1").unwrap();
        let held = profile.settings("IMI-HELD");
        assert_eq!(
            [held.level("struck"), held.level("passing")],
            [Level::Forbidden; 2]
        );
        assert_eq!(
            [
                held.level("unplanned_struck"),
                held.level("unplanned_passing")
            ],
            [Level::Off; 2]
        );
        let silence = profile.settings("IMI-SILENCE");
        for param in ["exposition", "long", "thin"] {
            assert_eq!(silence.level(param), Level::P1, "{param}");
        }
        assert_eq!(silence.level("unplanned_thin"), Level::Off);
        profile
            .apply_override("IMI-HELD.unplanned_struck=P2")
            .unwrap();
        assert_eq!(
            profile.settings("IMI-HELD").level("unplanned_struck"),
            Level::P2
        );
        // After the profile of a mass, a level for the whole rule keeps the mass's levels; the
        // rule switched off is off for the parts without a plan too.
        let mass = crate::mass::profile(&Profile::fux());
        let mut held_v = mass.clone();
        held_v.apply_override("IMI-HELD=P2").unwrap();
        held_v.apply_override("IMI-SILENCE=V").unwrap();
        let held = held_v.settings("IMI-HELD");
        assert_eq!(
            [held.level("struck"), held.level("passing")],
            [Level::P2; 2]
        );
        assert_eq!(
            [
                held.level("unplanned_struck"),
                held.level("unplanned_passing")
            ],
            [Level::Forbidden, Level::P3]
        );
        assert_eq!(
            held_v.settings("IMI-SILENCE").level("unplanned_thin"),
            Level::P1
        );
        let mut off = mass.clone();
        off.apply_override("IMI-HELD=aus").unwrap();
        off.apply_override("IMI-SILENCE=aus").unwrap();
        for (rule, param) in [
            ("IMI-HELD", "struck"),
            ("IMI-HELD", "passing"),
            ("IMI-HELD", "unplanned_struck"),
            ("IMI-HELD", "unplanned_passing"),
            ("IMI-SILENCE", "exposition"),
            ("IMI-SILENCE", "long"),
            ("IMI-SILENCE", "thin"),
            ("IMI-SILENCE", "unplanned_thin"),
        ] {
            assert_eq!(
                off.settings(rule).level(param),
                Level::Off,
                "{rule}.{param}"
            );
        }
        let mut plain = Profile::fux();
        plain.apply_override("IMI-HELD=aus").unwrap();
        plain.apply_override("IMI-SILENCE=aus").unwrap();
        assert_eq!(off.to_toml(None), plain.to_toml(None));
        // An integer or flag parameter keeps its value when its rule is switched off.
        let mut length = Profile::fux();
        let before = length.settings("CF-LENGTH").clone();
        length.apply_override("CF-LENGTH=aus").unwrap();
        let after = length.settings("CF-LENGTH");
        assert!(after.is_off());
        assert!(before.values().contains(&("max", ParamValue::Int(20))));
        for &(name, value) in before.values() {
            match value {
                ParamValue::Level(_) => assert_eq!(after.level(name), Level::Off, "{name}"),
                other => assert_eq!(after.get(name), Some(other), "{name}"),
            }
        }
        let apart: Vec<(&str, &str)> = all_rules()
            .iter()
            .flat_map(|rule| {
                let id = rule.info().id;
                rule.info()
                    .params
                    .iter()
                    .filter(|p| p.kind == ParamKind::Level && !p.follows_rule())
                    .map(move |p| (id, p.name))
            })
            .collect();
        assert_eq!(
            apart,
            [
                ("IMI-SILENCE", "unplanned_thin"),
                ("IMI-HELD", "unplanned_struck"),
                ("IMI-HELD", "unplanned_passing"),
            ]
        );
    }

    #[test]
    fn rule_ids_are_unique() {
        let mut ids: Vec<&str> = all_rules().iter().map(|r| r.info().id).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count);
    }
}
