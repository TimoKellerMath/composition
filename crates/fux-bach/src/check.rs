//! The check of a finished piece against given chords: the machine of `engine` walks the piece
//! with every tone known and reports what it finds, rule by rule.

use serde_json::{Value, json};

use crate::engine::{Event, Kind, Machine, Part, Sink, Tally, Task, Tones};
use crate::key::Harmony;
use crate::pitch::Pitch;
use crate::rules::{Bearer, Level, Profile, Rule};
use crate::score::{Note, Piece};
use crate::time::Tick;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Finding {
    pub rule: Rule,
    pub level: Level,
    pub at: Tick,
    pub voice: usize,
    pub other: Option<usize>,
    pub bearer: Bearer,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub tally: Tally,
}

impl Report {
    pub fn forbidden(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.level == Level::Forbidden)
            .count()
    }

    pub fn preferences(&self) -> usize {
        self.findings.len() - self.forbidden()
    }

    /// The prices of the rules, without the plan.
    pub const fn cost(&self) -> u32 {
        self.tally.cost
    }

    /// How often each forbidden rule was broken.
    pub fn forbidden_by_rule(&self) -> Vec<(Rule, usize)> {
        Rule::ALL
            .iter()
            .map(|rule| {
                let count = self
                    .findings
                    .iter()
                    .filter(|f| f.rule == *rule && f.level == Level::Forbidden)
                    .count();
                (*rule, count)
            })
            .filter(|r| r.1 > 0)
            .collect()
    }
}

struct Collector(Vec<Finding>);

impl Sink for Collector {
    fn found(
        &mut self,
        rule: Rule,
        level: Level,
        at: Tick,
        voice: usize,
        other: Option<usize>,
        bearer: Bearer,
    ) {
        if level != Level::Off {
            self.0.push(Finding {
                rule,
                level,
                at,
                voice,
                other,
                bearer,
            });
        }
    }
}

/// How often each rule could have been broken and how often it was, whatever its level.
#[derive(Clone, Debug)]
pub struct Census {
    pub chances: Vec<u64>,
    pub found: Vec<u64>,
}

impl Default for Census {
    fn default() -> Census {
        Census {
            chances: vec![0; Rule::COUNT],
            found: vec![0; Rule::COUNT],
        }
    }
}

impl Census {
    pub fn add(&mut self, other: &Census) {
        for k in 0..Rule::COUNT {
            self.chances[k] += other.chances[k];
            self.found[k] += other.found[k];
        }
    }

    pub fn share(&self, rule: Rule) -> Option<f64> {
        let chances = self.chances[rule as usize];
        (chances > 0).then(|| self.found[rule as usize] as f64 / chances as f64)
    }
}

impl Sink for Census {
    fn chance(&mut self, rule: Rule) {
        self.chances[rule as usize] += 1;
    }

    fn found(
        &mut self,
        rule: Rule,
        _level: Level,
        _at: Tick,
        _voice: usize,
        _other: Option<usize>,
        _bearer: Bearer,
    ) {
        self.found[rule as usize] += 1;
    }
}

/// The tones of a piece that belong to a pedal point, as `figures` reads them: the pitch lies or
/// returns in its voice over at least four beats and two changes of chord.
pub fn pedals(piece: &Piece, harmony: &Harmony) -> Vec<Vec<bool>> {
    let long = 4 * piece.meter.beat();
    piece
        .voices
        .iter()
        .map(|voice| {
            let notes = &voice.notes;
            let mut out = vec![false; notes.len()];
            let mut first = 0;
            while first < notes.len() {
                let mut last = first;
                while last + 1 < notes.len()
                    && notes[last + 1].pitch == notes[first].pitch
                    && notes[last + 1].on == notes[last].off
                {
                    last += 1;
                }
                let (from, until) = (notes[first].on, notes[last].off);
                if until - from >= long && harmony.changes(from, until - 1) >= 2 {
                    for mark in &mut out[first..=last] {
                        *mark = true;
                    }
                }
                first = last + 1;
            }
            out
        })
        .collect()
}

/// The events of a piece: a tone at every onset, a rest where a voice falls silent. `given`
/// says which tones are given (the others count as searched), `pedal` which belong to a pedal
/// point.
pub fn parts_of(
    piece: &Piece,
    given: &dyn Fn(usize, &Note) -> bool,
    pedal: &dyn Fn(usize, usize) -> bool,
) -> (Vec<Part>, Tones) {
    let mut parts = Vec::new();
    let mut tones: Tones = Vec::new();
    for (v, voice) in piece.voices.iter().enumerate() {
        let mut events: Vec<Event> = Vec::new();
        let mut pitches: Vec<Option<Pitch>> = Vec::new();
        for (i, note) in voice.notes.iter().enumerate() {
            if i > 0 && voice.notes[i - 1].off < note.on {
                events.push(Event::rest(voice.notes[i - 1].off));
                pitches.push(None);
            }
            let mut event = if given(v, note) {
                Event::given(note.on, note.pitch)
            } else {
                Event::free(note.on, None)
            };
            event.pedal = pedal(v, i);
            events.push(event);
            pitches.push(Some(note.pitch));
        }
        if let Some(last) = voice.notes.last()
            && last.off < piece.end
        {
            events.push(Event::rest(last.off));
            pitches.push(None);
        }
        parts.push(Part {
            low: Pitch::new(0, 0, 0),
            high: Pitch::new(0, 0, 9),
            events,
        });
        tones.push(pitches);
    }
    (parts, tones)
}

/// The task that reads a piece in which every tone is given.
pub fn task_of<'a>(piece: &Piece, harmony: &'a Harmony, profile: &'a Profile) -> (Task<'a>, Tones) {
    let marks = pedals(piece, harmony);
    let (parts, tones) = parts_of(piece, &|_, _| true, &|v, i| marks[v][i]);
    (
        Task {
            meter: piece.meter,
            origin: piece.origin,
            end: piece.end,
            harmony,
            parts,
            profile,
            beam: 1,
            seed: 0,
            invertible: Vec::new(),
            root_at: None,
            resolve_given: false,
        },
        tones,
    )
}

/// Reads the tones of a task and collects what the rules find.
pub fn report(task: &Task, tones: &Tones) -> Report {
    let mut sink = Collector(Vec::new());
    let tally = Machine::new(task).replay(tones, false, &mut sink);
    let mut findings = sink.0;
    findings.sort_by_key(|f| (f.at, f.voice, f.other, f.rule));
    Report { findings, tally }
}

/// Checks a finished piece against its chords.
pub fn check(piece: &Piece, harmony: &Harmony, profile: &Profile) -> Report {
    let (task, tones) = task_of(piece, harmony, profile);
    report(&task, &tones)
}

/// Counts the rules on a finished piece: opportunities and findings.
pub fn census(piece: &Piece, harmony: &Harmony, profile: &Profile) -> Census {
    let (task, tones) = task_of(piece, harmony, profile);
    let mut sink = Census::default();
    Machine::new(&task).replay(&tones, true, &mut sink);
    sink
}

/// Whether the task has a free tone.
pub fn has_free(task: &Task) -> bool {
    task.parts
        .iter()
        .any(|p| p.events.iter().any(|e| e.kind == Kind::Free))
}

fn gcd(a: Tick, b: Tick) -> Tick {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// `Takt 12, Schlag 3` or `Takt 12, Schlag 3 + 1/2`.
pub fn place(piece: &Piece, at: Tick, english: bool) -> String {
    let (bar, pos) = piece.bar_of(at);
    let beat = piece.meter.beat();
    let (whole, rest) = (pos / beat + 1, pos % beat);
    let fraction = if rest == 0 {
        String::new()
    } else {
        let d = gcd(rest, beat);
        format!(" + {}/{}", rest / d, beat / d)
    };
    if english {
        format!("bar {bar}, beat {whole}{fraction}")
    } else {
        format!("Takt {bar}, Schlag {whole}{fraction}")
    }
}

fn voices(finding: &Finding, english: bool) -> String {
    let word = if english { "voice" } else { "Stimme" };
    match finding.other {
        Some(other) => format!(
            "{word} {} {} {}",
            finding.voice + 1,
            if english { "and" } else { "und" },
            other + 1
        ),
        None => format!("{word} {}", finding.voice + 1),
    }
}

/// One line per finding: place, voices, rule, level, message.
pub fn lines(report: &Report, piece: &Piece, english: bool) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|f| {
            format!(
                "{}, {}: {} ({}) {}",
                place(piece, f.at, english),
                voices(f, english),
                f.rule.id(),
                f.level,
                f.rule.message(english)
            )
        })
        .collect()
}

/// „0 Verbot(e), Kosten 45 aus 7 Präferenz(en).“
pub fn summary(report: &Report, english: bool) -> String {
    if english {
        format!(
            "{} prohibition(s), cost {} from {} preference(s).",
            report.forbidden(),
            report.cost(),
            report.preferences()
        )
    } else {
        format!(
            "{} Verbot(e), Kosten {} aus {} Präferenz(en).",
            report.forbidden(),
            report.cost(),
            report.preferences()
        )
    }
}

pub fn json(report: &Report, piece: &Piece, english: bool) -> Value {
    let findings: Vec<Value> = report
        .findings
        .iter()
        .map(|f| {
            let (bar, _) = piece.bar_of(f.at);
            json!({
                "rule": f.rule.id(),
                "level": f.level.to_string(),
                "forbidden": f.level == Level::Forbidden,
                "tick": f.at,
                "bar": bar,
                "place": place(piece, f.at, english),
                "voice": f.voice,
                "other": f.other,
                "message": f.rule.message(english),
            })
        })
        .collect();
    json!({
        "forbidden": report.forbidden(),
        "preferences": report.preferences(),
        "cost": report.cost(),
        "summary": summary(report, english),
        "findings": findings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::{Chord, Key, Kind as ChordKind};
    use crate::pitch::Lof;
    use crate::score::Voice;
    use crate::time::Meter;

    fn line(tones: &[(i8, i8, Tick)]) -> Voice {
        let mut voice = Voice::default();
        let mut on = 0;
        for (step, octave, ticks) in tones {
            voice.push(on, on + ticks, Pitch::new(*step, 0, *octave));
            on += ticks;
        }
        voice
    }

    #[test]
    fn a_report_names_rule_and_place() {
        let key = Key::parse("C").unwrap();
        // Parallel fifths on the second beat, then a passing tone above a held bass.
        let upper = line(&[(4, 4, 24), (5, 4, 24), (4, 4, 12), (3, 4, 12), (2, 4, 24)]);
        let lower = line(&[(0, 4, 24), (1, 4, 24), (0, 4, 48)]);
        let piece = Piece::new("fifths", key, Meter::COMMON, vec![upper, lower]);
        let mut harmony = Harmony::default();
        harmony.push(0, Chord::new(Lof(0), ChordKind::Maj), key);
        let profile = Profile::start();
        let report = check(&piece, &harmony, &profile);
        let text = lines(&report, &piece, false);
        assert!(
            text.iter()
                .any(|l| l.starts_with("Takt 1, Schlag 2, Stimme 1 und 2: MOT-PARALLEL (V)")),
            "{text:?}"
        );
        // The fifths move up and back down again.
        assert_eq!(report.forbidden(), 2, "{text:?}");
        assert!(summary(&report, false).starts_with("2 Verbot(e), Kosten "));
        assert_eq!(place(&piece, 60, true), "bar 1, beat 3 + 1/2");
        // The count hears of the opportunities too.
        let counted = census(&piece, &harmony, &profile);
        assert_eq!(counted.found[Rule::MotParallel as usize], 2);
        assert!(counted.chances[Rule::MotParallel as usize] >= 1);
        assert_eq!(json(&report, &piece, true)["forbidden"], 2);
    }

    #[test]
    fn a_pedal_owes_nothing() {
        let key = Key::parse("C").unwrap();
        // A bass G under G, C, D7 and G: foreign to the chord of C, but a pedal.
        let upper = line(&[(6, 4, 96), (0, 5, 96), (0, 5, 96), (6, 4, 96)]);
        let lower = line(&[(4, 3, 384)]);
        let piece = Piece::new("pedal", key, Meter::COMMON, vec![upper, lower]);
        let mut harmony = Harmony::default();
        harmony.push(0, Chord::new(Lof(1), ChordKind::Maj), key);
        harmony.push(96, Chord::new(Lof(-1), ChordKind::Maj), key);
        harmony.push(192, Chord::new(Lof(2), ChordKind::Dom7), key);
        harmony.push(288, Chord::new(Lof(1), ChordKind::Maj), key);
        assert_eq!(pedals(&piece, &harmony)[1], [true]);
        let report = check(&piece, &harmony, &Profile::start());
        assert!(
            report.findings.iter().all(|f| f.voice != 1),
            "{:?}",
            report.findings
        );
    }
}
