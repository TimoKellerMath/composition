//! A fugue on a given subject, for two to four voices, on the machine of the style.
//!
//! The way of a fugue: the answer comes from the rule of `answer` (or from the user); the
//! countersubject is searched once against the answer in double counterpoint at the octave and
//! then moves with every entry, its head following the answer's change between fifth and fourth;
//! a plan sets entries, episodes, cadences and pedal points; the episodes carry the head or the
//! tail of the subject from voice to voice, stepping down; the cadences get their suspension;
//! the scaffold of `scaffold` gives the chords for all of it; everything else is searched by the
//! machine of `engine`.

use anyhow::{Result, anyhow, bail};

use crate::answer::{self, Answer};
use crate::check;
use crate::draft::{self, Draft, VoiceDraft};
use crate::engine::{Invertible, Tally};
use crate::fugue::in_key;
use crate::key::{Harmony, Key};
use crate::pitch::{Iv, Lof, Pitch};
use crate::rhythm;
use crate::rules::Profile;
use crate::scaffold::{self, Cadence, CadenceKind, Pedal, Plan};
use crate::score::{Note, Piece, Voice};
use crate::tables;
use crate::themes::Subject;
use crate::time::{Meter, Tick};

/// How a fugue ends.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ending {
    /// A stretto if the subject gives one, a pedal point otherwise.
    Auto,
    /// A dominant pedal before the last entry.
    Pedal,
    /// A stretto (fails if the subject gives none).
    Stretto,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub voices: usize,
    pub seed: u64,
    pub beam: usize,
    /// How often a new rhythm is drawn after a dead end.
    pub attempts: usize,
    pub profile: Profile,
    /// The user's own answer instead of the rule's, in the rhythm of the subject.
    pub answer: Option<Vec<Note>>,
    /// The plan of entries after a fugue of WTC I (its number) instead of the planner's.
    pub template: Option<Template>,
    pub ending: Ending,
    /// The episodes are sequences in all voices: what the voices have beside the first
    /// statement of the motif comes again with every further one, a step lower. Takes a second
    /// search.
    pub sequences: bool,
    /// The voices that are free beside an entry get figures of the subject and of the
    /// countersubject: their rhythm for sure, their steps as wishes.
    pub figures: bool,
    /// The scaffold names a bass for every chord (see `bass`), and the search is asked to put
    /// it lowest where the chord begins (HAR-BASS).
    pub bass: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            voices: 3,
            seed: 1,
            beam: 400,
            attempts: 12,
            profile: Profile::fuge(),
            answer: None,
            template: None,
            ending: Ending::Auto,
            sequences: true,
            figures: true,
            bass: false,
        }
    }
}

/// The ranges of the voices from the top down.
pub fn ranges(voices: usize) -> Vec<(Pitch, Pitch)> {
    let p = |step: i8, octave: i8| Pitch::new(step, 0, octave);
    match voices {
        2 => vec![(p(5, 3), p(0, 6)), (p(0, 2), p(4, 4))],
        3 => vec![(p(5, 3), p(0, 6)), (p(1, 3), p(2, 5)), (p(0, 2), p(2, 4))],
        _ => vec![
            (p(0, 4), p(0, 6)),
            (p(4, 3), p(3, 5)),
            (p(0, 3), p(5, 4)),
            (p(0, 2), p(2, 4)),
        ],
    }
}

/// The voices in the order of their first entries.
pub fn order(voices: usize) -> Vec<usize> {
    match voices {
        2 => vec![0, 1],
        3 => vec![1, 0, 2],
        _ => vec![1, 0, 2, 3],
    }
}

/// The tones later by `at` ticks.
fn moved(notes: &[Note], at: Tick) -> Vec<Note> {
    notes
        .iter()
        .map(|n| Note {
            on: n.on + at,
            off: n.off + at,
            pitch: n.pitch,
        })
        .collect()
}

fn octaves(notes: &[Note], k: i16) -> Vec<Note> {
    notes
        .iter()
        .map(|n| Note {
            on: n.on,
            off: n.off,
            pitch: n.pitch.up(Iv::new(12 * k, 7 * k)),
        })
        .collect()
}

fn mean(notes: &[Note]) -> i32 {
    notes.iter().map(|n| n.pitch.midi as i32).sum::<i32>() / notes.len().max(1) as i32
}

/// The tones in the octave in which they fit the range best.
pub fn fitted(notes: &[Note], range: (Pitch, Pitch)) -> Vec<Note> {
    let (low, high) = (range.0.midi as i32, range.1.midi as i32);
    let centre = (low + high) / 2;
    let best = (-4i16..=4)
        .min_by_key(|k| {
            let mut over = 0i32;
            for n in notes {
                let midi = n.pitch.midi as i32 + 12 * *k as i32;
                over += (low - midi).max(0) + (midi - high).max(0);
            }
            (over, (mean(notes) + 12 * *k as i32 - centre).abs())
        })
        .unwrap_or(0);
    octaves(notes, best)
}

/// What a given group of tones is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum What {
    Subject,
    Answer,
    Countersubject,
    Motif,
    Pedal,
    /// The suspension of a cadence: the tonic held over the dominant, and the leading tone it
    /// resolves to.
    Suspension,
    /// The bass of a cadence: the fifth of the key under the dominant, the root at the arrival.
    Bass,
    /// A link of a sequence: what a voice had beside the first statement of the motif of an
    /// episode, and its repetitions a step lower each.
    Sequence,
}

#[derive(Clone, Debug)]
pub struct Given {
    pub voice: usize,
    pub what: What,
    pub key: Key,
    pub notes: Vec<Note>,
    /// Stretches inside the group that were given back to the search.
    pub holes: Vec<(Tick, Tick)>,
}

impl Given {
    pub fn new(voice: usize, what: What, key: Key, notes: Vec<Note>) -> Given {
        Given {
            voice,
            what,
            key,
            notes,
            holes: Vec::new(),
        }
    }

    /// Where the voice is given: from the first tone to the end of the last, without the holes.
    fn spans(&self) -> Vec<(Tick, Tick)> {
        let (Some(first), Some(last)) = (self.notes.first(), self.notes.last()) else {
            return Vec::new();
        };
        let mut spans = vec![(first.on, last.off)];
        for (from, until) in &self.holes {
            spans = spans
                .into_iter()
                .flat_map(|(a, b)| {
                    if *until <= a || *from >= b {
                        vec![(a, b)]
                    } else {
                        vec![(a, *from), (*until, b)]
                    }
                })
                .filter(|(a, b)| a < b)
                .collect();
        }
        spans
    }
}

/// A section of the plan with its first tick.
#[derive(Clone, Debug)]
pub struct Section {
    pub name: &'static str,
    pub from: Tick,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    Subject,
    Answer,
}

/// An entry of the plan.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub voice: usize,
    pub at: Tick,
    pub key: Key,
    pub form: Form,
    /// The voice that has the countersubject against it.
    pub counter: Option<usize>,
    /// Part of the exposition, of a group of middle entries, or of the end.
    pub group: usize,
}

/// The plan of a fugue: where everything stands, before any tone is searched.
#[derive(Clone, Debug)]
pub struct Design {
    pub end: Tick,
    pub entries: Vec<Entry>,
    /// Episodes as (from, until).
    pub episodes: Vec<(Tick, Tick)>,
    pub keys: Vec<(Tick, Key)>,
    pub cadences: Vec<Cadence>,
    pub pedals: Vec<Pedal>,
    /// Voices that rest before they enter again: (voice, from, until).
    pub rests: Vec<(usize, Tick, Tick)>,
    pub sections: Vec<Section>,
    pub final_hold: Tick,
    /// The delay of the stretto the end is built on.
    pub stretto: Option<Tick>,
    /// The length of the motif of the episodes: a statement stands on every such unit.
    pub motif: Tick,
}

/// A plan after one of Bach's fugues: the order of its entries with their voices and keys, and
/// the bars between them.
#[derive(Clone, Debug)]
pub struct Template {
    pub name: String,
    /// The voices of Bach's fugue.
    pub voices: usize,
    /// Entries after the exposition: voice from the top, degree of the key in the home key
    /// (0 the tonic), whether that key is minor, and the bars since the entry before ended.
    pub entries: Vec<(usize, usize, bool, i32)>,
}

impl Template {
    /// The names of the plans that come with the program (`wtk1-02`).
    pub fn names() -> Vec<String> {
        let value: serde_json::Value =
            serde_json::from_str(include_str!("../data/plans.json")).unwrap_or_default();
        value["fugues"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|f| f["name"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The plan of a fugue of WTC I by its name.
    pub fn named(name: &str) -> Option<Template> {
        let value: serde_json::Value =
            serde_json::from_str(include_str!("../data/plans.json")).ok()?;
        let fugue = value["fugues"]
            .as_array()?
            .iter()
            .find(|f| f["name"].as_str() == Some(name))?;
        let entries = fugue["entries"]
            .as_array()?
            .iter()
            .filter_map(|row| {
                Some((
                    row[0].as_u64()? as usize,
                    row[1].as_u64()? as usize,
                    row[2].as_bool()?,
                    row[3].as_i64()? as i32,
                ))
            })
            .collect();
        Some(Template {
            name: name.to_string(),
            voices: fugue["voices"].as_u64()? as usize,
            entries,
        })
    }
}

fn up_to_bar(t: Tick, bar: Tick) -> Tick {
    (t + bar - 1).div_euclid(bar) * bar
}

/// The keys of the middle entries, by degree of the home key: the relative key and its
/// neighbours first.
fn middle_keys(home: Key) -> Vec<Key> {
    let key = |degree: usize, minor: bool| Key {
        tonic: home.degree(degree),
        minor,
    };
    if home.minor {
        vec![
            key(2, false),
            key(6, false),
            key(3, true),
            key(5, false),
            key(4, true),
        ]
    } else {
        vec![
            key(5, true),
            key(2, true),
            key(3, false),
            key(1, true),
            key(4, false),
        ]
    }
}

impl Design {
    /// The planner: an exposition with every voice, groups of middle entries in related keys
    /// with episodes between them, and an end with a stretto or a pedal point.
    pub fn plan(
        subject: &Subject,
        voices: usize,
        stretto: Option<Tick>,
        template: Option<&Template>,
        motif: Tick,
    ) -> Design {
        let meter = subject.meter;
        let bar = meter.bar();
        let period = subject.period;
        let span = subject.notes.last().map_or(period, |n| n.off);
        let home = subject.key;
        let order = order(voices);
        let modulating = answer::modulates(&subject.notes, home);
        let answer_key = if modulating { home } else { home.dominant() };
        let lowest = voices - 1;
        let mut design = Design {
            end: 0,
            entries: Vec::new(),
            episodes: Vec::new(),
            keys: vec![(0, home)],
            cadences: Vec::new(),
            pedals: Vec::new(),
            rests: Vec::new(),
            sections: Vec::new(),
            final_hold: 0,
            stretto,
            motif,
        };
        // The exposition: subject and answer in turn, a bar of codetta after each answer that
        // another entry follows.
        let mut t = 0;
        for k in 0..voices {
            let form = if k % 2 == 0 {
                Form::Subject
            } else {
                Form::Answer
            };
            design.entries.push(Entry {
                voice: order[k],
                at: t,
                key: home,
                form,
                counter: (k > 0).then(|| order[k - 1]),
                group: 0,
            });
            design.sections.push(Section {
                name: if form == Form::Subject {
                    "Thema"
                } else {
                    "Antwort"
                },
                from: t,
            });
            design.keys.push((
                t,
                if form == Form::Answer {
                    answer_key
                } else {
                    home
                },
            ));
            t += period;
            if k % 2 == 1 && k + 1 < voices {
                design.sections.push(Section {
                    name: "Codetta",
                    from: t,
                });
                design.keys.push((t, home));
                t += bar;
            }
        }
        // How long an episode is: at least two bars, about eight quarters, and room for two
        // statements of the motif.
        let episode = (((192 + bar - 1) / bar).clamp(2, 4) * bar).max(up_to_bar(2 * motif, bar));
        // The groups of middle entries.
        let related = middle_keys(home);
        let double = voices >= 3 && period <= 2 * bar + bar / 2;
        let mut groups: Vec<Vec<(usize, Key)>> = match template {
            Some(template) => {
                // Bach's entries in his order: one that follows within two bars joins the
                // group before it. His voices are spread over ours.
                let mut groups: Vec<Vec<(usize, Key)>> = Vec::new();
                for (voice, degree, minor, gap) in &template.entries {
                    let voice = if template.voices > 1 {
                        (voice * lowest + (template.voices - 1) / 2) / (template.voices - 1)
                    } else {
                        0
                    };
                    let entry = (
                        voice.min(lowest),
                        Key {
                            tonic: home.degree(*degree),
                            minor: *minor,
                        },
                    );
                    match groups.last_mut() {
                        Some(group)
                            if *gap < 2
                                && group.len() < 3
                                && group.last().is_some_and(|e| e.0 != entry.0) =>
                        {
                            group.push(entry);
                        }
                        _ => groups.push(vec![entry]),
                    }
                }
                groups
            }
            None => {
                // The voices take turns: the top and the bass first, then the middle.
                let turns: Vec<Vec<usize>> = match voices {
                    2 => vec![vec![1], vec![0], vec![1], vec![0]],
                    3 => vec![vec![0, 2], vec![1, 0], vec![2, 1], vec![0, 2]],
                    _ => vec![vec![0, 3], vec![2, 1], vec![3, 0], vec![1, 2]],
                };
                let mut groups = Vec::new();
                for (g, turn) in turns.iter().enumerate().take(2) {
                    let count = if double { 2 } else { 1 };
                    groups.push(
                        turn.iter()
                            .take(count)
                            .enumerate()
                            .map(|(k, v)| (*v, related[(2 * g + k) % related.len()]))
                            .collect(),
                    );
                }
                groups
            }
        };
        // The end: after the last entry a bar of cadence, the arrival, and a bar of final chord.
        let tail = |entries: Tick| -> Tick { entries + 3 * bar };
        let final_span = match stretto {
            Some(delay) => delay + span,
            None => span,
        };
        let length = |groups: &Vec<Vec<(usize, Key)>>, episode: Tick| -> Tick {
            let middle: Tick = groups
                .iter()
                .map(|g| episode + g.len() as Tick * period)
                .sum();
            tail(up_to_bar(t, bar) + middle + episode + final_span)
        };
        if template.is_none() {
            // Not shorter than about thirty bars: more groups, then longer episodes.
            let mut g = groups.len();
            while length(&groups, episode) < 32 * bar && g < 6 {
                let turns: [usize; 2] = match voices {
                    2 => [g % 2, (g + 1) % 2],
                    3 => [[2, 1], [0, 2], [1, 0]][g % 3],
                    _ => [[3, 0], [1, 2], [0, 3], [2, 1]][g % 4],
                };
                let count = if double { 2 } else { 1 };
                groups.push(
                    (0..count)
                        .map(|k| (turns[k], related[(2 * g + k) % related.len()]))
                        .collect(),
                );
                g += 1;
            }
        }
        let mut episode = episode;
        while length(&groups, episode) < 30 * bar && episode < 6 * bar {
            episode += bar;
        }
        // Not longer than eighty bars: one entry to a group, then fewer groups.
        while length(&groups, episode) > 80 * bar && groups.iter().any(|g| g.len() > 1) {
            if let Some(group) = groups.iter_mut().rev().find(|g| g.len() > 1) {
                group.pop();
            }
        }
        while length(&groups, episode) > 80 * bar && groups.len() > 2 {
            groups.pop();
        }
        // How long a voice rests before an entry it is staged for: half a bar in whole beats.
        let beat = meter.beat();
        let rest = (((bar / 2) + beat - 1) / beat * beat).max(beat);
        // Where a cadence can close an entry: on the beat of the subject's last tone, if that
        // is the root or the third of the key the subject ends in.
        let ends_in = |key: Key| if modulating { key.dominant() } else { key };
        let closing: Option<Tick> = subject
            .notes
            .last()
            .filter(|n| {
                matches!(
                    (n.pitch.pc() + 12 - ends_in(home).tonic.pc()) % 12,
                    0 | 3 | 4
                )
            })
            .map(|n| up_to_bar(n.on, beat));
        // Episodes and groups in turn.
        t = up_to_bar(t, bar);
        for (g, group) in groups.iter().enumerate() {
            let wait = episode;
            design.sections.push(Section {
                name: "Zwischenspiel",
                from: t,
            });
            design.episodes.push((t, t + wait));
            design.keys.push((t, group[0].1));
            t += wait;
            for (k, (voice, key)) in group.iter().enumerate() {
                let counter = if k > 0 {
                    Some(group[k - 1].0)
                } else {
                    // A neighbour that does not enter next.
                    let next = group.get(1).map(|e| e.0);
                    [voice + 1, voice.wrapping_sub(1)]
                        .into_iter()
                        .find(|c| *c < voices && Some(*c) != next)
                };
                design.entries.push(Entry {
                    voice: *voice,
                    at: t,
                    key: *key,
                    form: Form::Subject,
                    counter,
                    group: g + 1,
                });
                design.sections.push(Section {
                    name: "Einsatz",
                    from: t,
                });
                design.keys.push((t, *key));
                if k == 0 {
                    if g % 2 == 1 {
                        // Every second group is staged: its voice rests before it enters.
                        design.rests.push((*voice, t - rest.min(wait / 2), t));
                    } else if let Some(other) = group
                        .get(1)
                        .map(|e| e.0)
                        .or_else(|| (0..voices).find(|v| v != voice && Some(*v) != counter))
                    {
                        // Into the others the episode flows, and another voice falls silent
                        // with the first tone of the entry: the one that enters next, or one
                        // that has neither the subject nor the countersubject.
                        let first = t + subject.notes.first().map_or(0, |n| n.on);
                        design.rests.push((other, first, t + bar.min(period)));
                    }
                }
                if k + 1 == group.len()
                    && let Some(close) = closing
                {
                    // A cadence may close the group, with the last tone of its last entry. The
                    // composer keeps it where the bass can carry it, and not after every
                    // group.
                    design.cadences.push(Cadence {
                        at: t + close,
                        key: ends_in(*key),
                        kind: CadenceKind::RootImperfect,
                    });
                }
                t += period;
            }
            t = up_to_bar(t - period + span, bar).max(t);
        }
        // The last episode leads home; without a stretto it ends on a dominant pedal.
        design.sections.push(Section {
            name: "Zwischenspiel",
            from: t,
        });
        design.episodes.push((t, t + episode));
        design.keys.push((t, home));
        let last = groups.len() + 1;
        let first_voice = if stretto.is_some() { lowest } else { 0 };
        let at = t + episode;
        if stretto.is_none() {
            design.pedals.push(Pedal {
                from: at - (episode / 2).max(bar),
                until: at,
                key: home,
                degree: 4,
            });
        }
        design.cadences.push(Cadence {
            at,
            key: home,
            kind: CadenceKind::RootImperfect,
        });
        if stretto.is_some() {
            // The stretto is staged: its first voice rests before it. Over the dominant pedal
            // the top voice runs into the last entry without a rest.
            design
                .rests
                .push((first_voice, at - rest.min(episode / 2), at));
        }
        design.entries.push(Entry {
            voice: first_voice,
            at,
            key: home,
            form: Form::Subject,
            counter: if stretto.is_some() || voices < 2 {
                None
            } else {
                Some(1)
            },
            group: last,
        });
        design.sections.push(Section {
            name: if stretto.is_some() {
                "Engführung"
            } else {
                "Schlusseinsatz"
            },
            from: at,
        });
        let mut finish = at + span;
        if let Some(delay) = stretto {
            design.entries.push(Entry {
                voice: 0,
                at: at + delay,
                key: home,
                form: Form::Subject,
                counter: None,
                group: last,
            });
            finish = at + delay + span;
        }
        // A bar of cadence, the arrival on the tonic over its pedal, the final chord.
        let cadence = up_to_bar(finish, bar);
        let arrival = cadence + bar;
        design.sections.push(Section {
            name: "Kadenz",
            from: cadence,
        });
        design.cadences.push(Cadence {
            at: arrival,
            key: home,
            kind: CadenceKind::Perfect,
        });
        design.end = arrival + 2 * bar;
        design.final_hold = design.end - bar;
        design.pedals.push(Pedal {
            from: arrival,
            until: design.end,
            key: home,
            degree: 0,
        });
        design.keys.sort_by_key(|(t, _)| *t);
        design
    }

    pub fn bars(&self, meter: Meter) -> i32 {
        self.end / meter.bar()
    }
}

/// The head of a subject as a motif for the episodes: whole units from its first bar line that
/// hold at least three tones (at most four units), and its length.
pub fn motif(subject: &[Note], unit: Tick) -> (Vec<Note>, Tick) {
    let count = (1..=4)
        .find(|n| subject.iter().filter(|x| x.on < *n as Tick * unit).count() >= 3)
        .unwrap_or(4) as Tick;
    let length = count * unit;
    let notes = subject
        .iter()
        .filter(|n| n.on < length)
        .map(|n| Note {
            on: n.on,
            off: n.off.min(length),
            pitch: n.pitch,
        })
        .collect();
    (notes, length)
}

/// The tail of a subject as a motif of the same length as its head: the tones of its last
/// units, from a unit's start. `None` if these are fewer than three or the head itself.
pub fn tail(subject: &[Note], unit: Tick, length: Tick) -> Option<Vec<Note>> {
    let end = subject.last()?.off;
    let from = ((end - length).max(0) / unit) * unit;
    if from == 0 {
        return None;
    }
    let notes: Vec<Note> = subject
        .iter()
        .filter(|n| n.on >= from && n.on < from + length)
        .map(|n| Note {
            on: n.on - from,
            off: n.off.min(from + length) - from,
            pitch: n.pitch,
        })
        .collect();
    (notes.len() >= 3).then_some(notes)
}

/// The tones of a line of key `key` moved by `steps` degrees of its natural scale.
fn shifted(notes: &[Note], steps: i16, key: Key) -> Vec<Note> {
    notes
        .iter()
        .map(|n| {
            let dia = n.pitch.dia + steps;
            let step = dia.rem_euclid(7) as i8;
            let degree = (step - key.tonic.step()).rem_euclid(7) as usize;
            let class: Lof = key.degree(degree);
            Note {
                on: n.on,
                off: n.off,
                pitch: Pitch::new(step, class.alter(), dia.div_euclid(7) as i8),
            }
        })
        .collect()
}

/// The countersubject for the subject, from the one found against the answer: every tone moves
/// down by what the tone of the answer above it went up, so that its head follows the answer's
/// change between fifth and fourth.
pub fn adapted(countersubject: &[Note], answer: &Answer) -> Vec<Note> {
    countersubject
        .iter()
        .map(|n| {
            let index = answer.notes.iter().rposition(|a| a.on <= n.on).unwrap_or(0);
            let fourth = answer.fourth.get(index).copied().unwrap_or(false);
            Note {
                on: n.on,
                off: n.off,
                pitch: n.pitch.down(if fourth { Iv::FOURTH } else { Iv::FIFTH }),
            }
        })
        .collect()
}

/// A finished fugue with what it was made from.
pub struct Fugue {
    pub piece: Piece,
    pub draft: Draft,
    pub harmony: Harmony,
    pub design: Design,
    pub given: Vec<Given>,
    pub answer: Answer,
    /// The countersubject as found against the answer, from the answer's first tick.
    pub countersubject: Vec<Note>,
    pub tally: Tally,
    /// Searches run for the countersubject and for the fugue.
    pub attempts: (usize, usize),
    pub seconds: f64,
    pub learned_from: Vec<String>,
}

impl Fugue {
    /// Whether a tone of the piece was given.
    pub fn is_given(&self, voice: usize, note: &Note) -> bool {
        self.draft.is_given(voice, note)
    }

    /// Whether a tone was given by the plan: not searched, and not a link of a sequence either
    /// (those are tones the search found once and the second setting repeats).
    pub fn is_planned(&self, voice: usize, note: &Note) -> bool {
        self.given.iter().any(|g| {
            g.voice == voice
                && g.what != What::Sequence
                && g.notes
                    .iter()
                    .any(|n| n.on == note.on && n.pitch == note.pitch)
        })
    }

    /// The tones the plan gave.
    pub fn planned(&self) -> usize {
        self.given
            .iter()
            .filter(|g| g.what != What::Sequence)
            .map(|g| g.notes.len())
            .sum()
    }

    /// The check of the finished piece against its scaffold, with the roles of its tones.
    pub fn report(&self, profile: &Profile) -> check::Report {
        let (parts, tones) = check::parts_of(
            &self.piece,
            &|v, note| self.draft.is_given(v, note),
            &|v, i| {
                let on = self.piece.voices[v].notes[i].on;
                self.draft.voices[v]
                    .pedals
                    .iter()
                    .any(|(from, until)| *from <= on && on < *until)
            },
        );
        let task = self.draft.task(parts, &self.harmony, profile, 1, 0);
        check::report(&task, &tones)
    }
}

/// A voice of a draft from its groups of given tones; it is silent until its first tone and in
/// its rests.
fn voice_draft(range: (Pitch, Pitch), groups: &[&Given], rests: Vec<(Tick, Tick)>) -> VoiceDraft {
    let mut given: Vec<Note> = Vec::new();
    let mut spans: Vec<(Tick, Tick)> = Vec::new();
    let mut pedals: Vec<(Tick, Tick)> = Vec::new();
    for group in groups {
        spans.extend(group.spans());
        if group.what == What::Pedal {
            pedals.extend(group.spans());
        }
        given.extend(group.notes.iter().copied());
    }
    given.sort_by_key(|n| n.on);
    let first = given.first().map_or(Tick::MAX / 2, |n| n.on);
    let mut silent = vec![(Tick::MIN / 2, first)];
    silent.extend(rests);
    VoiceDraft {
        low: range.0,
        high: range.1,
        given,
        spans,
        silent,
        pedals,
        figures: Vec::new(),
    }
}

/// A cell of the material: the tones that begin in one unit of the subject or of the
/// countersubject, as their onsets from the start of the unit and the steps between them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cell {
    pub onsets: Vec<Tick>,
    pub steps: Vec<i8>,
}

/// The cells of some lines for units of `unit` ticks: the units in which at least three tones
/// begin, one after the other without a rest, the last of them up to the end of the unit or to
/// a rest. A cell may begin after a rest, not under a tone held into its unit. Each cell once,
/// in the order of the lines.
pub fn cells_of(lines: &[&[Note]], unit: Tick) -> Vec<Cell> {
    let mut out: Vec<Cell> = Vec::new();
    for line in lines {
        let Some(last) = line.last() else {
            continue;
        };
        let mut from = 0;
        while from < last.off {
            let notes: Vec<&Note> = line
                .iter()
                .filter(|n| n.on >= from && n.on < from + unit)
                .collect();
            let held = line.iter().any(|n| n.on < from && n.off > from);
            if notes.len() >= 3 && !held && notes.windows(2).all(|w| w[0].off == w[1].on) {
                let cell = Cell {
                    onsets: notes.iter().map(|n| n.on - from).collect(),
                    steps: notes
                        .windows(2)
                        .map(|w| (w[1].pitch.dia - w[0].pitch.dia) as i8)
                        .collect(),
                };
                if !out.contains(&cell) {
                    out.push(cell);
                }
            }
            from += unit;
        }
    }
    out
}

/// The rhythm for a figure of a voice's own, from a cell: the cell's, if it has four tones
/// from the start of its unit; else its most frequent value running through the unit (halved
/// until there are four), so that the figure is one a sequence can show.
fn running(cell: &Cell, unit: Tick) -> Vec<Tick> {
    if cell.onsets.len() >= 4 && cell.onsets.first() == Some(&0) {
        return cell.onsets.clone();
    }
    let mut values: Vec<Tick> = cell.onsets.windows(2).map(|w| w[1] - w[0]).collect();
    values.sort_unstable();
    let mut value = values
        .iter()
        .copied()
        .max_by_key(|v| (values.iter().filter(|x| *x == v).count(), -*v))
        .unwrap_or(unit);
    while value > 6 && value % 2 == 0 && unit / value < 4 {
        value /= 2;
    }
    if value <= 0 || unit % value != 0 {
        return cell.onsets.clone();
    }
    (0..unit / value).map(|k| k * value).collect()
}

/// The figures of the free voices, unit by unit in a round, the voices at different places of
/// it: two units with a cell of the material, the second time wished a step lower; then units in
/// pairs with a figure of the voice's own, the first in the rhythm of a cell with steps of the
/// search (in a first setting), the second its repetition a step lower (in the setting after
/// it). Where a unit is free only in part, the part gets what a cell of the material has there.
/// From where a voice has entered to `until`, wherever it has nothing given and does not rest.
#[allow(clippy::too_many_arguments)]
fn accompaniment(
    design: &Design,
    given: &[Given],
    cells: &[Cell],
    unit: Tick,
    voices: usize,
    until: Tick,
    first: Option<&Piece>,
    free_units: usize,
    run: bool,
) -> Vec<Vec<draft::Figure>> {
    let mut figures: Vec<Vec<draft::Figure>> = vec![Vec::new(); voices];
    if unit <= 0 || cells.is_empty() {
        return figures;
    }
    // The round: two units of the material, four of the voice's own (two pairs), of which the
    // last `free_units` are left to chance.
    const ROUND: usize = 6;
    const MATERIAL: usize = 2;
    for (w, own) in figures.iter_mut().enumerate() {
        let of_voice = || given.iter().filter(|g| g.voice == w);
        let Some(entered) = of_voice()
            .filter_map(|g| g.notes.first())
            .map(|n| n.on)
            .min()
        else {
            continue;
        };
        // Where the voice is taken: its given groups from their first tone to their last, and
        // its planned rests.
        let mut taken: Vec<(Tick, Tick)> = of_voice().flat_map(|g| g.spans()).collect();
        taken.extend(
            design
                .rests
                .iter()
                .filter(|(v, _, _)| *v == w)
                .map(|(_, a, b)| (*a, *b)),
        );
        // The longest free stretch of a unit.
        let gap = |at: Tick| -> Option<(Tick, Tick)> {
            let end = at + unit;
            if at < entered || end > until {
                return None;
            }
            let mut cuts: Vec<(Tick, Tick)> = taken
                .iter()
                .filter(|(a, b)| *a < end && at < *b)
                .map(|(a, b)| ((*a).max(at), (*b).min(end)))
                .collect();
            cuts.sort_unstable();
            let mut best: Option<(Tick, Tick)> = None;
            let mut t = at;
            for (a, b) in cuts.into_iter().chain([(end, end)]) {
                if a > t && best.is_none_or(|(x, y)| y - x < a - t) {
                    best = Some((t, a));
                }
                t = t.max(b);
            }
            best
        };
        let free = |at: Tick| gap(at) == Some((at, at + unit));
        // What the voice had in a unit of the first setting, as a cell: at least three tones
        // one after the other.
        let found = |at: Tick| -> Option<Cell> {
            let voice = first?.voices.get(w)?;
            let notes = voice.range(at, at + unit);
            (notes.len() >= 3
                && notes.windows(2).all(|n| n[0].off == n[1].on)
                && voice.at(at).is_none_or(|n| n.on == at))
            .then(|| Cell {
                onsets: notes.iter().map(|n| n.on - at).collect(),
                steps: notes
                    .windows(2)
                    .map(|n| (n[1].pitch.dia - n[0].pitch.dia) as i8)
                    .collect(),
            })
        };
        let figure = |at: Tick, cell: &Cell, lower: bool| -> draft::Figure {
            let mut steps: Vec<Option<i8>> = vec![None];
            steps.extend(cell.steps.iter().map(|s| Some(*s)));
            if lower {
                // Its first tone a step under that of the statement before.
                steps[0] = Some(-1 - cell.steps.iter().sum::<i8>());
            }
            draft::Figure {
                at,
                until: at + unit,
                onsets: cell.onsets.clone(),
                steps,
            }
        };
        let mut at = up_to_bar(entered, unit) - unit;
        while at + unit <= until {
            at += unit;
            let index = (at / unit) as usize;
            let place = (index + 2 * w) % ROUND;
            let pair = at - unit * (place % 2) as Tick;
            let cell = &cells[(index / ROUND + w) % cells.len()];
            match gap(at) {
                Some(stretch) if stretch == (at, at + unit) => {
                    if place < MATERIAL {
                        own.push(figure(at, cell, place % 2 == 1 && free(pair)));
                    } else if place < ROUND - free_units && free(pair) && free(pair + unit) {
                        if first.is_none() {
                            // The first setting: the unit that will be taken up again gets
                            // the rhythm of a cell and steps of the search's own.
                            if place.is_multiple_of(2) {
                                let rhythm = &cells[(index / ROUND + w + 1) % cells.len()];
                                let onsets = if run {
                                    running(rhythm, unit)
                                } else {
                                    rhythm.onsets.clone()
                                };
                                own.push(draft::Figure {
                                    at,
                                    until: at + unit,
                                    steps: vec![None; onsets.len()],
                                    onsets,
                                });
                            }
                        } else if let Some(cell) = found(pair) {
                            own.push(figure(at, &cell, place % 2 == 1));
                        }
                    }
                }
                Some((from, to)) if place < ROUND - free_units && 2 * (to - from) >= unit => {
                    // A part of the unit: the tones a cell has there, in their places.
                    let kept: Vec<usize> = (0..cell.onsets.len())
                        .filter(|k| at + cell.onsets[*k] >= from && at + cell.onsets[*k] < to)
                        .collect();
                    if kept.len() >= 3 {
                        let mut steps: Vec<Option<i8>> = vec![None];
                        steps.extend(kept[1..].iter().map(|k| Some(cell.steps[k - 1])));
                        own.push(draft::Figure {
                            at: from,
                            until: to,
                            onsets: kept.iter().map(|k| at + cell.onsets[*k] - from).collect(),
                            steps,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    figures
}

/// Adds a group of tones to the given material; tones that overlap what the voice already has
/// are left out. Returns whether anything was added.
fn give(given: &mut Vec<Given>, mut group: Given) -> bool {
    group.notes.retain(|n| {
        !given
            .iter()
            .filter(|g| g.voice == group.voice)
            .flat_map(|g| &g.notes)
            .any(|m| n.on < m.off && m.on < n.off)
    });
    if group.notes.is_empty() {
        return false;
    }
    given.push(group);
    true
}

/// The countersubject: the voice that had the subject goes on against the answer, in double
/// counterpoint at the octave. Returns its tones from the answer's first tick, and the searches
/// it took.
fn countersubject(
    subject: &Subject,
    answer: &Answer,
    options: &Options,
    table: &crate::cells::Table,
    intervals: &crate::melody::Table,
) -> Result<(Vec<Note>, usize)> {
    let voices = options.voices;
    let meter = subject.meter;
    let bar = meter.bar();
    let period = subject.period;
    let home = subject.key;
    let order = order(voices);
    let ranges = ranges(voices);
    let (a, b) = (order[0], order[1]);
    let compass = subject
        .notes
        .iter()
        .map(|n| n.pitch.midi)
        .max()
        .unwrap_or(0)
        - subject
            .notes
            .iter()
            .map(|n| n.pitch.midi)
            .min()
            .unwrap_or(0);
    let theme = fitted(&subject.notes, ranges[a]);
    // The answer near the subject's voice: double counterpoint at the octave leaves the two no
    // more than an octave apart.
    let reply = {
        let reply = moved(&answer.notes, period);
        let toward = mean(&theme) + if b < a { 7 } else { -7 };
        let (low, high) = (ranges[b].0.midi as i32, ranges[b].1.midi as i32);
        let best = (-4i16..=4)
            .min_by_key(|k| {
                let over: i32 = reply
                    .iter()
                    .map(|n| {
                        let midi = n.pitch.midi as i32 + 12 * *k as i32;
                        (low - midi).max(0) + (midi - high).max(0)
                    })
                    .sum();
                (over, (mean(&reply) + 12 * *k as i32 - toward).abs())
            })
            .unwrap_or(0);
        octaves(&reply, best)
    };
    let span = subject.notes.last().map_or(period, |n| n.off);
    let end = up_to_bar(period + span, bar);
    let modulating = answer::modulates(&subject.notes, home);
    let answer_key = if modulating { home } else { home.dominant() };
    let plan = Plan {
        key: home,
        meter,
        origin: 0,
        end,
        lines: vec![theme.clone(), reply.clone()],
        keys: vec![(0, home), (period, answer_key)],
        cadences: Vec::new(),
        pedals: Vec::new(),
        must: Vec::new(),
        picardy: false,
        close: false,
        hold: None,
        lowest: Vec::new(),
        seed: options.seed,
    };
    let harmony = scaffold::generate(&plan, scaffold::embedded());
    let lines = [
        Given::new(a, What::Subject, home, theme.clone()),
        Given::new(b, What::Answer, home, reply.clone()),
    ];
    let draft = Draft {
        name: subject.name.clone(),
        key: home,
        meter,
        origin: 0,
        end,
        voices: (0..voices)
            .map(|v| {
                let groups: Vec<&Given> = lines.iter().filter(|g| g.voice == v).collect();
                voice_draft(ranges[v], &groups, Vec::new())
            })
            .collect(),
        final_hold: None,
        invertible: vec![Invertible {
            upper: a.min(b),
            lower: a.max(b),
            from: period,
            until: period + span,
            // A subject wider than a ninth is inverted at the fifteenth.
            span: if compass > 14 { 24 } else { 12 },
        }],
        root_at_hold: false,
        longest: Some(bar),
        goes_on: true,
    };
    let theme_end = theme.last().map_or(0, |n| n.off);
    let from = reply.first().map_or(period, |n| n.on);
    // A countersubject needs tones: a rhythm in which the voice rests through most of the answer
    // is drawn again.
    let least = (answer.notes.len() / 3).max(2);
    let mut searches = 0;
    for round in 0..options.attempts as u64 {
        let search = draft::Options {
            seed: options.seed.wrapping_add(7919 * round),
            beam: options.beam,
            attempts: options.attempts,
            profile: options.profile.clone(),
        };
        let found = draft::set(&draft, &harmony, table, intervals, &search)
            .map_err(|dead| anyhow!("{}: no countersubject; dead ends {dead:?}", subject.name))?;
        searches += found.attempts;
        let tones: Vec<Note> = found.piece.voices[a]
            .notes
            .iter()
            .filter(|n| n.on >= theme_end.max(from) && n.on < period + span)
            .map(|n| Note {
                on: n.on - period,
                off: n.off.min(period + span) - period,
                pitch: n.pitch,
            })
            .collect();
        if tones.len() >= least && found.tally.count.junction == 0 {
            return Ok((tones, searches));
        }
    }
    bail!(
        "{}: no countersubject with at least {least} tones",
        subject.name
    )
}

/// The statements of the motif in an episode: from voice to voice, each a step lower than the
/// one before, so that the line of the sequence leads down to the entry that follows. The
/// chords come afterwards, from the statements.
#[allow(clippy::too_many_arguments)]
fn chain(
    motif: &[Note],
    length: Tick,
    home: Key,
    target: Key,
    episode: (Tick, Tick),
    voices: &[usize],
    ranges: &[(Pitch, Pitch)],
    busy: &dyn Fn(usize, Tick, Tick) -> bool,
) -> Vec<Given> {
    let mut out: Vec<Given> = Vec::new();
    if voices.is_empty() || motif.is_empty() || length <= 0 {
        return out;
    }
    let count = ((episode.1 - episode.0) / length).max(0) as usize;
    let base = in_key(motif, home, target);
    let mut turn = 0;
    for j in 0..count {
        let at = episode.0 + j as Tick * length;
        // The next voice in turn that is free.
        let Some(skip) =
            (0..voices.len()).find(|k| !busy(voices[(turn + k) % voices.len()], at, at + length))
        else {
            continue;
        };
        let voice = voices[(turn + skip) % voices.len()];
        // Two statements stay in one voice: only so does a voice come back to its own figure,
        // and the episode is a sequence. From three on the motif goes from voice to voice.
        turn += skip + usize::from(count > 2);
        let level = (count - j) as i16;
        let tones = fitted(&shifted(&moved(&base, at), level, target), ranges[voice]);
        let (low, high) = ranges[voice];
        if tones
            .iter()
            .any(|n| n.pitch.midi < low.midi - 2 || n.pitch.midi > high.midi + 2)
        {
            continue;
        }
        out.push(Given::new(voice, What::Motif, target, tones));
    }
    out
}

/// The links of the sequences of a fugue, from a first setting of it: in every episode the free
/// tones of each voice in the unit of the first statement of the motif (the model), and their
/// repetitions in the units after it, a step lower each. Where the motif goes from one voice to
/// another and back, the two change places with it: the one that gets the motif takes on what
/// the other had beside it. The beat before the entry that follows stays to the search.
fn sequences(first: &Fugue, ranges: &[(Pitch, Pitch)]) -> Vec<Given> {
    let design = &first.design;
    let piece = &first.piece;
    let length = design.motif;
    let beat = piece.meter.beat();
    let mut out = Vec::new();
    if length <= 0 {
        return out;
    }
    for (from, until) in &design.episodes {
        let (from, until) = (*from, *until);
        let units = ((until - from) / length) as usize;
        if units < 2 {
            continue;
        }
        let target = design
            .keys
            .iter()
            .rev()
            .find(|(t, _)| *t <= until)
            .map_or(piece.key, |(_, key)| *key);
        let motif_voice = |j: Tick| -> Option<usize> {
            first
                .given
                .iter()
                .filter(|g| g.what == What::Motif)
                .find(|g| {
                    g.notes.first().is_some_and(|n| {
                        n.on >= from + j * length && n.on < from + (j + 1) * length
                    })
                })
                .map(|g| g.voice)
        };
        let Some(a) = motif_voice(0) else {
            continue;
        };
        let b = motif_voice(1).filter(|v| *v != a);
        let model: Vec<Vec<Note>> = piece
            .voices
            .iter()
            .enumerate()
            .map(|(v, voice)| {
                voice
                    .range(from, from + length)
                    .iter()
                    .filter(|n| !first.is_given(v, n))
                    .map(|n| Note {
                        on: n.on,
                        off: n.off.min(from + length),
                        pitch: n.pitch,
                    })
                    .collect()
            })
            .collect();
        let end = until - beat;
        for j in 0..units {
            for (v, range) in ranges.iter().enumerate().take(piece.voices.len()) {
                let source = match b {
                    Some(b) if j % 2 == 1 && v == a => b,
                    Some(b) if j % 2 == 1 && v == b => a,
                    _ => v,
                };
                let at = j as Tick * length;
                let mut tones = shifted(&moved(&model[source], at), -(j as i16), target);
                let (low, high) = *range;
                if source != v
                    || tones
                        .iter()
                        .any(|n| n.pitch.midi < low.midi || n.pitch.midi > high.midi)
                {
                    tones = fitted(&tones, *range);
                }
                let tones: Vec<Note> = tones
                    .into_iter()
                    .filter(|n| n.on < end)
                    .map(|n| Note {
                        on: n.on,
                        off: n.off.min(end),
                        pitch: n.pitch,
                    })
                    .collect();
                if !tones.is_empty() {
                    out.push(Given::new(v, What::Sequence, target, tones));
                }
            }
        }
    }
    out
}

/// Bach's band of recurrence (protocol of phase B5): the share of the free tones of a fugue that
/// lie in a figure of four tones the free tones have again, from the 10th to the 90th percentile
/// of his 23 labelled fugues of the Well-Tempered Clavier I.
pub const RECURRENCE: (f64, f64) = (0.711, 0.859);

/// The share of the free tones of a fugue that lie in a figure which comes again.
pub fn recurrence(subject: &Subject, fugue: &Fugue) -> f64 {
    let bound: Vec<(usize, Tick)> = fugue
        .given
        .iter()
        .filter(|g| matches!(g.what, What::Subject | What::Answer | What::Countersubject))
        .flat_map(|g| g.notes.iter().map(move |n| (g.voice, n.on)))
        .collect();
    let free = |v: usize, note: &Note| !bound.contains(&(v, note.on));
    let material = [subject.notes.clone(), fugue.countersubject.clone()];
    crate::motifs::measure(&fugue.piece, &free, &material, 4).shares()[1]
}

/// A fugue on the subject. With sequences it is set twice: the first setting gives the models
/// of the episodes and the figures of the voices' own, the second repeats them. If its free
/// tones then recur more than in nine of ten fugues of Bach's, it is set again with a part of
/// the units left to chance, and the setting nearest to his band stands.
pub fn compose(subject: &Subject, options: &Options) -> Result<Fugue> {
    // The time of the whole way, with the settings that fell.
    let begun = crate::clock::Stopwatch::start();
    // Figures fix the rhythm of the free voices, and a rhythm that leads into a dead end does so
    // in every attempt. Then the figures are taken back: first the running rhythm of the
    // voices' own figures, then all of them.
    let mut level = if options.figures { 2u8 } else { 0 };
    let mut first = loop {
        match compose_pass(subject, options, &[], None, 0, level) {
            Ok(first) => break first,
            Err(error) if level == 0 => return Err(error),
            Err(_) => level -= 1,
        }
    };
    if !options.sequences {
        first.seconds = begun.seconds();
        return Ok(first);
    }
    let links = sequences(&first, &ranges(options.voices));
    let clean = |fugue: &Fugue| {
        fugue.tally.count == crate::engine::Count::default()
            && fugue.report(&options.profile).forbidden() == 0
    };
    // The episodes of a setting that hold two statements of the motif.
    let statements = |fugue: &Fugue| -> usize {
        fugue
            .design
            .episodes
            .iter()
            .filter(|(from, until)| {
                fugue
                    .given
                    .iter()
                    .filter(|g| g.what == What::Motif)
                    .filter(|g| {
                        g.notes
                            .first()
                            .is_some_and(|n| n.on >= *from && n.on < *until)
                    })
                    .count()
                    >= 2
            })
            .count()
    };
    let trace = std::env::var_os("FUX_BACH_TRACE").is_some();
    // How far the recurrence of a setting lies from Bach's band.
    let off = |share: f64| (RECURRENCE.0 - share).max(share - RECURRENCE.1).max(0.0);
    let mut best: Option<(f64, Fugue)> = None;
    let mut attempts = first.attempts;
    for free_units in [0usize, 2, 4] {
        let Ok(next) = compose_pass(subject, options, &links, Some(&first), free_units, level)
        else {
            continue;
        };
        attempts = (attempts.0 + next.attempts.0, attempts.1 + next.attempts.1);
        let share = recurrence(subject, &next);
        if trace {
            let kept = next
                .given
                .iter()
                .filter(|g| g.what == What::Sequence)
                .count();
            eprintln!(
                "sequences {} {}: {} of {} links kept, {} units of the round free, recurrence {:.1} %, {:?}, {} prohibitions",
                subject.name,
                options.voices,
                kept,
                links.len(),
                free_units,
                100.0 * share,
                next.tally.count,
                next.report(&options.profile).forbidden(),
            );
        }
        // A setting stands only if it breaks nothing the first kept, and keeps the statements
        // of the motif in every episode that had two.
        if !(clean(&next) || !clean(&first)) || statements(&next) < statements(&first) {
            continue;
        }
        let above = share > RECURRENCE.1;
        if best.as_ref().is_none_or(|(b, _)| off(share) < off(*b)) {
            best = Some((share, next));
        }
        if !above || level == 0 {
            break;
        }
    }
    match best {
        Some((_, mut fugue)) => {
            fugue.attempts = attempts;
            fugue.seconds = begun.seconds();
            Ok(fugue)
        }
        None => {
            first.seconds = begun.seconds();
            Ok(first)
        }
    }
}

/// One setting of a fugue; `links` are tones given beside the plan's own (the links of the
/// sequences), `first` is the setting before this one, whose figures the free voices take up,
/// `free_units` says how many units of the round of six are left without a figure, and `level`
/// how far the figures go: 0 none, 1 with the rhythms of the cells, 2 with a running rhythm for
/// the voices' own where a cell is thin.
fn compose_pass(
    subject: &Subject,
    options: &Options,
    links: &[Given],
    first: Option<&Fugue>,
    free_units: usize,
    level: u8,
) -> Result<Fugue> {
    let begun = crate::clock::Stopwatch::start();
    let voices = options.voices;
    if !(2..=4).contains(&voices) {
        bail!("a fugue has two to four voices here, not {voices}");
    }
    if subject.notes.len() < 3 {
        bail!("a subject needs at least three tones");
    }
    let meter = subject.meter;
    let bar = meter.bar();
    let home = subject.key;
    let ranges = ranges(voices);
    let embedded = tables::embedded();
    let profile = rhythm::value_profile(subject.notes.iter());
    let except = subject.from.clone().unwrap_or_default();
    let (table, intervals, learned_from) =
        embedded.nearest_cells(meter.unit(), profile, &except, 8);
    if table.size() == 0 {
        bail!("no rhythm tables for the meter {meter}");
    }
    let answer = match &options.answer {
        Some(notes) => {
            if notes.len() != subject.notes.len() {
                bail!(
                    "the answer has {} tones, the subject {}",
                    notes.len(),
                    subject.notes.len()
                );
            }
            let fourth = subject
                .notes
                .iter()
                .zip(notes)
                .map(|(s, a)| s.pitch.to(a.pitch).steps.rem_euclid(7) == 3)
                .collect::<Vec<bool>>();
            Answer {
                kind: if fourth.iter().any(|f| *f) {
                    answer::AnswerKind::Tonal
                } else {
                    answer::AnswerKind::Real
                },
                notes: subject
                    .notes
                    .iter()
                    .zip(notes)
                    .map(|(s, a)| Note {
                        on: s.on,
                        off: s.off,
                        pitch: a.pitch,
                    })
                    .collect(),
                fourth,
            }
        }
        None => answer::answer(&subject.notes, home),
    };
    // The end: a stretto if the subject gives one.
    let stretto = match options.ending {
        Ending::Pedal => None,
        Ending::Auto | Ending::Stretto => {
            crate::stretto::find(subject, voices, &options.profile).map(|s| s.delay)
        }
    };
    if options.ending == Ending::Stretto && stretto.is_none() {
        bail!("{}: the subject gives no stretto", subject.name);
    }
    let (counter_answer, first_attempts) =
        countersubject(subject, &answer, options, &table, &intervals)?;
    let counter_subject = adapted(&counter_answer, &answer);
    let unit = scaffold::sequence_unit(meter, 2 * bar);
    let (head, length) = motif(&subject.notes, unit);
    let ending = tail(&subject.notes, unit, length);
    let design = Design::plan(subject, voices, stretto, options.template.as_ref(), length);

    // The given material: entries and countersubjects.
    let mut given: Vec<Given> = Vec::new();
    for entry in &design.entries {
        let tones = match entry.form {
            Form::Subject => in_key(&subject.notes, home, entry.key),
            Form::Answer => answer.notes.clone(),
        };
        let placed = fitted(&moved(&tones, entry.at), ranges[entry.voice]);
        give(
            &mut given,
            Given::new(
                entry.voice,
                if entry.form == Form::Subject {
                    What::Subject
                } else {
                    What::Answer
                },
                entry.key,
                placed.clone(),
            ),
        );
        if let Some(c) = entry.counter {
            let line = match entry.form {
                Form::Subject => in_key(&counter_subject, home, entry.key),
                Form::Answer => counter_answer.clone(),
            };
            let mut line = fitted(&moved(&line, entry.at), ranges[c]);
            // On the side of the entry where its voice is written.
            let (m, e) = (mean(&line), mean(&placed));
            let fits = |notes: &[Note]| {
                notes.iter().all(|n| {
                    n.pitch.midi >= ranges[c].0.midi - 2 && n.pitch.midi <= ranges[c].1.midi + 2
                })
            };
            if c < entry.voice && m < e && fits(&octaves(&line, 1)) {
                line = octaves(&line, 1);
            } else if c > entry.voice && m > e && fits(&octaves(&line, -1)) {
                line = octaves(&line, -1);
            }
            give(
                &mut given,
                Given::new(c, What::Countersubject, entry.key, line),
            );
        }
    }
    // Pedal points in the lowest voice.
    let lowest = voices - 1;
    for pedal in &design.pedals {
        let tone = pedal.tone(ranges[lowest].0);
        give(
            &mut given,
            Given::new(
                lowest,
                What::Pedal,
                pedal.key,
                vec![Note {
                    on: pedal.from,
                    off: pedal.until,
                    pitch: tone,
                }],
            ),
        );
    }
    // The cadences of the plan need their bass: under the dominant the fifth of the key, at
    // the arrival its root (Bach's bass has them in 97 of 100 cadences in root position). Where
    // the lowest voice is free it gets them as given tones; where its given tones say
    // otherwise, or the voice is not there, the place has no cadence. A cadence that closes a
    // group does not follow one that closed the group before.
    let mut design = design;
    let beat = meter.beat();
    let mut kept: Vec<Cadence> = Vec::new();
    let mut closed_before = false;
    let closes = |cadence: &Cadence| {
        design
            .entries
            .iter()
            .any(|e| e.group > 0 && e.at < cadence.at && cadence.at < e.at + subject.period)
            && cadence.at < design.final_hold - 2 * bar
    };
    for cadence in design.cadences.clone() {
        let (at, key) = (cadence.at, cadence.key);
        let closing = closes(&cadence);
        if closing && closed_before {
            closed_before = false;
            continue;
        }
        let of_lowest = |from: Tick, until: Tick| -> Vec<Note> {
            let mut notes: Vec<Note> = given
                .iter()
                .filter(|g| g.voice == lowest)
                .flat_map(|g| g.notes.iter().copied())
                .filter(|n| n.on < until && from < n.off)
                .collect();
            notes.sort_by_key(|n| n.on);
            notes
        };
        let resting = |from: Tick, until: Tick| {
            design
                .rests
                .iter()
                .any(|(v, a, b)| *v == lowest && *a < until && from < *b)
        };
        let entered = given
            .iter()
            .any(|g| g.voice == lowest && g.notes.first().is_some_and(|n| n.on < at - beat));
        let (low, high) = ranges[lowest];
        let inside = |p: Pitch| p.midi >= low.midi && p.midi <= high.midi;
        let middle = Pitch {
            midi: ((low.midi as i32 + high.midi as i32) / 2) as i16,
            dia: ((low.dia as i32 + high.dia as i32) / 2) as i16,
        };
        let fifth_of = |root: Pitch| -> Option<Pitch> {
            let below = Pitch::nearest(key.degree(4), root);
            let above = Pitch {
                midi: below.midi + 12,
                dia: below.dia + 7,
            };
            let below = if below.midi < root.midi {
                below
            } else {
                Pitch {
                    midi: below.midi - 12,
                    dia: below.dia - 7,
                }
            };
            [above, below].into_iter().find(|p| inside(*p))
        };
        let before = of_lowest(at - beat, at);
        let mut bass: Vec<Note> = Vec::new();
        let carried = match of_lowest(at, at + 1).first() {
            // A given tone at the arrival: the root, or the place is no cadence.
            Some(tone) if tone.pitch.pc() == key.tonic.pc() => {
                if before.is_empty()
                    && !resting(at - beat, at)
                    && entered
                    && at - beat >= 0
                    && let Some(fifth) = fifth_of(tone.pitch)
                {
                    bass.push(Note {
                        on: at - beat,
                        off: at,
                        pitch: fifth,
                    });
                }
                true
            }
            Some(_) => false,
            None if resting(at - beat, at + beat) || !entered => false,
            None if !of_lowest(at, at + beat).is_empty() => false,
            None => match before.last() {
                // The fifth is there already, a pedal point that ends here: the root follows.
                Some(tone) if tone.pitch.pc() == key.degree(4).pc() && tone.off == at => {
                    let root = Pitch::nearest(key.tonic, tone.pitch);
                    let root = if inside(root) {
                        root
                    } else {
                        Pitch::nearest(key.tonic, middle)
                    };
                    bass.push(Note {
                        on: at,
                        off: at + beat,
                        pitch: root,
                    });
                    true
                }
                Some(_) => false,
                None => {
                    let root = Pitch::nearest(key.tonic, middle);
                    match fifth_of(root) {
                        Some(fifth) if at - beat >= 0 => {
                            bass.push(Note {
                                on: at - beat,
                                off: at,
                                pitch: fifth,
                            });
                            bass.push(Note {
                                on: at,
                                off: at + beat,
                                pitch: root,
                            });
                            true
                        }
                        _ => false,
                    }
                }
            },
        };
        if !carried {
            continue;
        }
        if !bass.is_empty() {
            give(&mut given, Given::new(lowest, What::Bass, key, bass));
        }
        closed_before = closing;
        kept.push(cadence);
    }
    // The cadences that close a group can still go: where the entry they close does not bear
    // the dominant under its last tones.
    let closing_at: Vec<Tick> = kept.iter().filter(|c| closes(c)).map(|c| c.at).collect();
    let mut cadences = kept;
    // The tones a cadence was given: its bass and its suspension begin or end at its arrival.
    let of_cadence =
        |group: &Given, at: Tick| group.notes.iter().any(|n| n.on == at || n.off == at);
    // Episodes and scaffold. An episode on the tail of the subject that keeps fewer than two
    // statements is made again on the head.
    let before_episodes = given.clone();
    let mut on_head = vec![false; design.episodes.len()];
    let mut harmony = Harmony::default();
    for _ in 0..2 {
        given = before_episodes.clone();
        given.retain(|g| g.what != What::Bass || cadences.iter().any(|c| of_cadence(g, c.at)));
        // The episodes: the head of the subject from voice to voice, stepping down to the entry.
        for (k, episode) in design.episodes.iter().enumerate() {
            let busy = |voice: usize, from: Tick, until: Tick| -> bool {
                given
                    .iter()
                    .filter(|g| g.voice == voice)
                    .flat_map(|g| &g.notes)
                    .any(|n| n.on < until && from < n.off)
                    || design
                        .rests
                        .iter()
                        .any(|(v, a, b)| *v == voice && *a < until && from < *b)
            };
            // Two voices carry it, the others step in where one of them is taken; which two
            // changes from episode to episode.
            let mut pair: Vec<usize> = match voices {
                2 => vec![0, 1],
                3 => [[0, 1], [1, 2], [0, 2]][k % 3].to_vec(),
                _ => [[0, 1], [2, 3], [1, 2], [0, 3]][k % 4].to_vec(),
            };
            pair.extend(
                (0..voices)
                    .filter(|v| !pair.contains(v))
                    .collect::<Vec<_>>(),
            );
            let target = design
                .keys
                .iter()
                .rev()
                .find(|(t, _)| *t <= episode.1)
                .map_or(home, |(_, key)| *key);
            // The head of the subject, and in every second episode its tail.
            let material = match (&ending, k % 2) {
                (Some(tail), 1) if !on_head[k] => tail,
                _ => &head,
            };
            let statements = chain(
                material, length, home, target, *episode, &pair, &ranges, &busy,
            );
            for statement in statements {
                give(&mut given, statement);
            }
        }
        // The cadences of the plan get their suspension, where a voice is free for it: an upper
        // voice holds the tonic from the chord before the dominant into it and steps down to the
        // leading tone (the fourth that resolves to the third). The chord before has to hold
        // the tonic.
        let mut prepared: Vec<(Tick, u8)> = Vec::new();
        for cadence in &cadences {
            if !matches!(
                cadence.kind,
                CadenceKind::Perfect | CadenceKind::RootImperfect
            ) {
                continue;
            }
            let beat = meter.beat();
            let (from, until) = (cadence.at - 2 * beat, cadence.at);
            let turn = until - if meter.compound() { beat / 3 } else { beat / 2 };
            let free = |voice: usize| -> bool {
                !given
                    .iter()
                    .filter(|g| g.voice == voice)
                    .flat_map(|g| &g.notes)
                    .any(|n| n.on < until && from < n.off)
                    && !design
                        .rests
                        .iter()
                        .any(|(v, a, b)| *v == voice && *a < until && from < *b)
                    && given
                        .iter()
                        .any(|g| g.voice == voice && g.notes.first().is_some_and(|n| n.on < from))
                    // The leading tone needs its root: a given tone of the voice at the
                    // arrival has to be it.
                    && given
                        .iter()
                        .filter(|g| g.voice == voice)
                        .flat_map(|g| &g.notes)
                        .filter(|n| n.on <= until && until < n.off)
                        .all(|n| n.pitch.pc() == cadence.key.tonic.pc())
            };
            // The top voice for choice, so that the cadence closes on its root there (Bach's
            // top voice has it in 72 of 100 cadences in root position); never the lowest.
            let mut candidates: Vec<usize> = vec![0];
            candidates.extend(1..lowest);
            let Some(voice) = candidates.into_iter().find(|v| free(*v)) else {
                continue;
            };
            let (low, high) = ranges[voice];
            let middle = Pitch {
                midi: ((low.midi as i32 + high.midi as i32) / 2) as i16,
                dia: ((low.dia as i32 + high.dia as i32) / 2) as i16,
            };
            let tonic = Pitch::nearest(cadence.key.tonic, middle);
            let leading = Pitch::nearest(cadence.key.raised(6), tonic);
            if leading.midi >= tonic.midi || from < 0 {
                continue;
            }
            prepared.push((from, tonic.pc()));
            let mut tones = vec![
                Note {
                    on: from,
                    off: turn,
                    pitch: tonic,
                },
                Note {
                    on: turn,
                    off: until,
                    pitch: leading,
                },
            ];
            // The leading tone goes on to the root, where the voice has room for it.
            let arrival = (until, until + beat);
            let room = !given
                .iter()
                .filter(|g| g.voice == voice)
                .flat_map(|g| &g.notes)
                .any(|n| n.on < arrival.1 && arrival.0 < n.off)
                && !design
                    .rests
                    .iter()
                    .any(|(v, a, b)| *v == voice && *a < arrival.1 && arrival.0 < *b);
            if room && arrival.1 <= design.end {
                tones.push(Note {
                    on: arrival.0,
                    off: arrival.1,
                    pitch: tonic,
                });
            }
            give(
                &mut given,
                Given::new(voice, What::Suspension, cadence.key, tones),
            );
        }
        // The links of the sequences, where nothing else stands.
        for link in links {
            give(&mut given, link.clone());
        }
        // The scaffold over everything given but the pedal tones. Where a tone of a countersubject
        // is not explained against it (the head, moved tone by tone with the answer's change, does
        // not always fit the subject), the tone is given back to the search; a statement of the
        // motif with such a tone goes as a whole. Then the scaffold is made again.
        let mut must: Vec<(Tick, u8)> = prepared.clone();
        let scaffold_of = |given: &[Given], cadences: &[Cadence], must: &[(Tick, u8)]| -> Harmony {
            // With a planned bass the chords heed the given tone that lies lowest where they
            // begin.
            let lowest: Vec<(Tick, u8)> = if options.bass {
                let drafts: Vec<VoiceDraft> = (0..voices)
                    .map(|v| {
                        let groups: Vec<&Given> = given.iter().filter(|g| g.voice == v).collect();
                        let rests = design
                            .rests
                            .iter()
                            .filter(|(voice, _, _)| *voice == v)
                            .map(|(_, a, b)| (*a, *b))
                            .collect();
                        voice_draft(ranges[v], &groups, rests)
                    })
                    .collect();
                let step = scaffold::step_of(meter);
                (0..design.end / step)
                    .map(|k| k * step)
                    .filter_map(|t| {
                        crate::bass::lowest_given(&drafts, t).map(|pitch| (t, pitch.pc()))
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let plan = Plan {
                key: home,
                meter,
                origin: 0,
                end: design.end,
                lines: given
                    .iter()
                    .filter(|g| g.what != What::Pedal)
                    .map(|g| g.notes.clone())
                    .collect(),
                keys: design.keys.clone(),
                // The final chord is reached from a dominant or the chord on the leading tone
                // over the pedal, and holds for its bar.
                cadences: cadences
                    .iter()
                    .copied()
                    .chain([Cadence {
                        at: design.final_hold,
                        key: home,
                        kind: CadenceKind::Imperfect,
                    }])
                    .collect(),
                pedals: design.pedals.clone(),
                must: must.to_vec(),
                picardy: true,
                close: true,
                hold: Some(design.final_hold),
                lowest,
                seed: options.seed,
            };
            scaffold::generate(&plan, scaffold::embedded())
        };
        // With links there is more to take back, and one that goes can uncover another.
        let rounds = if links.is_empty() { 5 } else { 16 };
        let mut stale = false;
        for _ in 0..rounds {
            harmony = scaffold_of(&given, &cadences, &must);
            stale = false;
            let mut lines: Vec<Vec<Note>> = vec![Vec::new(); voices];
            for group in &given {
                lines[group.voice].extend(group.notes.iter().copied());
            }
            for line in &mut lines {
                line.sort_by_key(|n| n.on);
            }
            let mut alone = Piece::new(
                &subject.name,
                home,
                meter,
                lines.into_iter().map(|notes| Voice { notes }).collect(),
            );
            alone.end = design.end;
            let report = check::check(&alone, &harmony, &options.profile);
            if std::env::var_os("FUX_BACH_TRACE").is_some() {
                for line in check::lines(&report, &alone, false) {
                    if line.contains("(V)") {
                        eprintln!("given material: {line}");
                    }
                }
            }
            let faults: Vec<(usize, Tick)> = report
                .findings
                .iter()
                .filter(|f| f.level == crate::rules::Level::Forbidden)
                // Alone, every group seems to end before a rest; in the piece a searched tone
                // follows and takes the foreign tone on.
                .filter(|f| f.rule != crate::rules::Rule::FigBeforeRest)
                .flat_map(|f| {
                    let mut at = vec![(f.voice, f.at)];
                    at.extend(f.other.map(|o| (o, f.at)));
                    at
                })
                .collect();
            let mut dropped = false;
            let mut gone: Vec<Tick> = Vec::new();
            // A link goes with a fault inside it, and also with one on the tone before it or
            // after it: there the link leads in or out badly, and it is what can be spared.
            let beside = |group: &Given| -> bool {
                let (Some(first), Some(last)) = (group.notes.first(), group.notes.last()) else {
                    return false;
                };
                faults.iter().any(|(v, at)| {
                    *v == group.voice
                        && ((*at >= first.on && *at < last.off)
                            || alone.voices[*v]
                                .at(*at)
                                .is_some_and(|n| n.on == last.off || n.off == first.on))
                })
            };
            // Links go first and alone: a fault they share with a motif or a countersubject is
            // theirs to take away.
            let links_only = given.iter().any(|g| g.what == What::Sequence && beside(g));
            for group in given.iter_mut() {
                let hit = |n: &Note| {
                    faults
                        .iter()
                        .any(|(v, at)| *v == group.voice && n.on <= *at && *at < n.off)
                };
                if links_only && group.what != What::Sequence {
                    continue;
                }
                match group.what {
                    What::Countersubject => {
                        let (out, kept): (Vec<Note>, Vec<Note>) =
                            group.notes.iter().partition(|n| hit(n));
                        dropped |= !out.is_empty();
                        let extent = group.spans();
                        group.holes.extend(out.iter().map(|n| (n.on, n.off)));
                        // The group keeps its extent while it has tones at both ends.
                        if let (Some(first), Some(last)) = (kept.first(), kept.last())
                            && let (Some(a), Some(b)) = (extent.first(), extent.last())
                        {
                            if first.on > a.0 {
                                group.holes.push((a.0, first.on));
                            }
                            if last.off < b.1 {
                                group.holes.push((last.off, b.1));
                            }
                        }
                        group.notes = kept;
                    }
                    What::Subject | What::Answer => {
                        // The entry stays. A cadence that closes it over the tone goes, with
                        // the dominant it put there; else the chord has to hold the tone.
                        for note in group.notes.iter().filter(|n| hit(n)) {
                            let closed = closing_at.iter().copied().find(|at| {
                                cadences.iter().any(|c| c.at == *at)
                                    && note.off > *at - 2 * beat
                                    && note.on <= *at
                            });
                            let pc = note.pitch.pc();
                            if let Some(at) = closed {
                                if !gone.contains(&at) {
                                    gone.push(at);
                                }
                                dropped = true;
                            } else if !must.contains(&(note.on, pc)) {
                                must.push((note.on, pc));
                                dropped = true;
                            } else {
                                // Asked for and not given: the chords on either side have to
                                // hold the tone as well.
                                let step = scaffold::step_of(meter);
                                for at in [note.on - step, note.on + step] {
                                    if at >= 0 && !must.contains(&(at, pc)) {
                                        must.push((at, pc));
                                        dropped = true;
                                    }
                                }
                            }
                        }
                    }
                    What::Sequence if beside(group) => {
                        group.notes.clear();
                        dropped = true;
                    }
                    What::Motif | What::Suspension | What::Bass if group.notes.iter().any(hit) => {
                        group.notes.clear();
                        dropped = true;
                    }
                    _ => {}
                }
            }
            for at in &gone {
                cadences.retain(|c| c.at != *at);
                for group in given.iter_mut().filter(|g| {
                    matches!(g.what, What::Bass | What::Suspension) && of_cadence(g, *at)
                }) {
                    group.notes.clear();
                }
                must.retain(|(t, _)| *t != *at - 2 * beat);
            }
            given.retain(|g| !g.notes.is_empty());
            if !dropped {
                break;
            }
            stale = true;
        }
        if stale {
            // The rounds ran out on a change: the scaffold follows it.
            harmony = scaffold_of(&given, &cadences, &must);
        }
        let mut again = false;
        for (k, (from, until)) in design.episodes.iter().enumerate() {
            let statements = given
                .iter()
                .filter(|g| g.what == What::Motif)
                .filter(|g| {
                    g.notes
                        .first()
                        .is_some_and(|n| n.on >= *from && n.on < *until)
                })
                .count();
            if statements < 2 && !on_head[k] && k % 2 == 1 && ending.is_some() {
                on_head[k] = true;
                again = true;
            }
        }
        if !again {
            break;
        }
    }

    design.cadences = cadences;
    let design = design;

    // A countersubject or a statement of the motif ends on a tone of its chord: what follows
    // is searched, and a foreign last tone would hand it a duty it cannot always pay.
    for group in given
        .iter_mut()
        .filter(|g| matches!(g.what, What::Countersubject | What::Motif))
    {
        while group.notes.len() > 2
            && group.notes.last().is_some_and(|n| {
                let foreign = |t: Tick| harmony.chord_at(t).is_some_and(|c| !c.has(n.pitch));
                foreign(n.on) || foreign(n.off - 1)
            })
        {
            group.notes.pop();
        }
    }

    // A link stops on a tone of its chord, wherever nothing given goes on after one of its
    // tones: there a rest may follow, or a searched tone that cannot always take a foreign one
    // on.
    loop {
        let starts: Vec<(usize, Tick)> = given
            .iter()
            .flat_map(|g| g.notes.iter().map(move |n| (g.voice, n.on)))
            .collect();
        let mut changed = false;
        for group in given.iter_mut().filter(|g| g.what == What::Sequence) {
            let voice = group.voice;
            let before = group.notes.len();
            group.notes.retain(|n| {
                let foreign = |t: Tick| harmony.chord_at(t).is_some_and(|c| !c.has(n.pitch));
                starts.contains(&(voice, n.off)) || !(foreign(n.on) || foreign(n.off - 1))
            });
            changed |= group.notes.len() != before;
        }
        given.retain(|g| !g.notes.is_empty());
        if !changed {
            break;
        }
    }

    // The free voices work with cells of the subject and of the countersubject: the smallest of
    // unit, bar and two bars that cuts a cell out of them. (A larger unit with richer cells was
    // tried after the test of phase B5 and gave less: fewer units are free as a whole.)
    let (cell_unit, cells) = [unit, bar, 2 * bar]
        .into_iter()
        .map(|u| (u, cells_of(&[&subject.notes[..], &counter_answer[..]], u)))
        .find(|(_, cells)| !cells.is_empty())
        .unwrap_or((unit, Vec::new()));
    // Up to the final chord. (Stopping a unit or a bar before it was tried: the top voice then
    // leaps into its last tone in some fugues, and the triad of the close is no fuller.)
    let figures = if level > 0 {
        accompaniment(
            &design,
            &given,
            &cells,
            cell_unit,
            voices,
            design.final_hold,
            first.map(|f| &f.piece),
            free_units,
            level > 1,
        )
    } else {
        vec![Vec::new(); voices]
    };
    let draft = Draft {
        name: subject.name.clone(),
        key: home,
        meter,
        origin: 0,
        end: design.end,
        voices: (0..voices)
            .map(|v| {
                let groups: Vec<&Given> = given.iter().filter(|g| g.voice == v).collect();
                let rests = design
                    .rests
                    .iter()
                    .filter(|(voice, _, _)| *voice == v)
                    .map(|(_, a, b)| (*a, *b))
                    .collect();
                let mut voice = voice_draft(ranges[v], &groups, rests);
                voice.figures = figures[v].clone();
                voice
            })
            .collect(),
        final_hold: Some(design.final_hold),
        invertible: Vec::new(),
        root_at_hold: true,
        longest: Some(bar),
        goes_on: true,
    };
    // The bass of the chords, once everything else of the scaffold stands.
    if options.bass {
        crate::bass::plan(
            &draft,
            &mut harmony,
            &scaffold::embedded().bass,
            options.seed,
        );
    }
    let search = draft::Options {
        seed: options.seed,
        beam: options.beam,
        attempts: options.attempts,
        profile: options.profile.clone(),
    };
    let setting = draft::set(&draft, &harmony, &table, &intervals, &search).map_err(|dead| {
        let bars: Vec<String> = dead
            .iter()
            .map(|(at, why)| {
                format!(
                    "{} ({})",
                    1 + at / bar,
                    why.map_or("nothing to choose from", |r| r.id())
                )
            })
            .collect();
        anyhow!(
            "{}: no fugue in {} attempts; dead ends in bars {}",
            subject.name,
            dead.len(),
            bars.join(", ")
        )
    })?;
    Ok(Fugue {
        piece: setting.piece,
        draft,
        harmony,
        design,
        given,
        answer,
        countersubject: counter_answer,
        tally: setting.tally,
        attempts: (first_attempts, setting.attempts),
        seconds: begun.seconds(),
        learned_from,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::{Chord, Kind};
    use crate::themes::{BUILTIN, parse};

    #[test]
    fn a_plan_has_its_parts() {
        let subject = BUILTIN[0].subject().unwrap();
        for voices in 2..=4 {
            let design = Design::plan(&subject, voices, None, None, 48);
            let bars = design.bars(subject.meter);
            assert!((30..=80).contains(&bars), "{voices} voices: {bars} bars");
            // Every voice enters in the exposition.
            let mut first: Vec<usize> = design
                .entries
                .iter()
                .filter(|e| e.group == 0)
                .map(|e| e.voice)
                .collect();
            first.sort_unstable();
            assert_eq!(first, (0..voices).collect::<Vec<_>>());
            // Two groups in keys other than the tonic, and an end on two pedal points.
            let keys: std::collections::BTreeSet<String> = design
                .entries
                .iter()
                .filter(|e| e.group > 0 && e.key != subject.key)
                .map(|e| e.key.to_string())
                .collect();
            assert!(keys.len() >= 2, "{keys:?}");
            assert_eq!(design.pedals.len(), 2);
            assert_eq!(design.final_hold, design.end - subject.meter.bar());
        }
        // With a stretto the end has no dominant pedal.
        let design = Design::plan(&subject, 3, Some(96), None, 48);
        assert_eq!(design.pedals.len(), 1);
        assert!(design.sections.iter().any(|s| s.name == "Engführung"));
    }

    /// A whole fugue in a compound meter: complete, no prohibition in the finished piece, every
    /// planned entry there to be found, episodes on material of the subject.
    #[test]
    fn a_fugue_in_six_sixteen() {
        let subject = BUILTIN
            .iter()
            .find(|b| b.name == "wtk2-11")
            .unwrap()
            .subject()
            .unwrap();
        let options = Options {
            beam: 60,
            ..Options::default()
        };
        let fugue = compose(&subject, &options).unwrap();
        let bars = fugue.design.bars(subject.meter);
        assert!((30..=80).contains(&bars), "{bars} bars");
        assert_eq!(fugue.piece.voices.len(), 3);
        let report = fugue.report(&options.profile);
        assert_eq!(report.forbidden(), 0, "{:?}", report.forbidden_by_rule());
        assert_eq!(fugue.tally.count, crate::engine::Count::default());
        let hits = crate::finder::find(&fugue.piece, &subject.notes);
        for entry in &fugue.design.entries {
            assert!(
                hits.iter()
                    .any(|h| h.voice == entry.voice && h.at == entry.at),
                "entry at {} in voice {}",
                entry.at,
                entry.voice
            );
        }
        let motifs = fugue.given.iter().filter(|g| g.what == What::Motif).count();
        assert!(
            motifs >= 2 * fugue.design.episodes.len(),
            "{motifs} statements"
        );
        // The same subject and options give the same fugue.
        let again = compose(&subject, &options).unwrap();
        assert_eq!(again.piece.note_count(), fugue.piece.note_count());
    }

    /// The close: the scaffold holds the final chord through the last bar (the major tonic,
    /// also in minor), every voice sounds in it, the outer voices have the root, the third is
    /// there, four voices have the whole triad, and the top voice does not leap into it. Before
    /// this was planned, the dominant stood on the first half of the last bar and left the
    /// voices only the fifth.
    #[test]
    fn the_close_is_planned() {
        for (name, voices) in [("bwv1080", 3), ("bwv1080", 4), ("wtk2-11", 3)] {
            let subject = BUILTIN
                .iter()
                .find(|b| b.name == name)
                .unwrap()
                .subject()
                .unwrap();
            let options = Options {
                voices,
                beam: 60,
                ..Options::default()
            };
            let fugue = compose(&subject, &options).unwrap();
            let hold = fugue.design.final_hold;
            let tonic = Chord::new(subject.key.tonic, Kind::Maj);
            let mut at = hold;
            while at < fugue.design.end {
                let span = fugue.harmony.at(at).unwrap();
                assert_eq!(
                    (span.chord, span.key),
                    (tonic, subject.key),
                    "{name} at {at}"
                );
                at += scaffold::step_of(subject.meter);
            }
            // A dominant or the chord on the leading tone leads into it.
            let before = fugue.harmony.at(hold - 1).unwrap().chord;
            let fifth = subject.key.degree(4);
            let leading = Lof(subject.key.tonic.0 + 5);
            assert!(
                before.root == fifth || before.root == leading,
                "{name}: {before} before the final chord"
            );
            let mut classes: Vec<u8> = Vec::new();
            for voice in &fugue.piece.voices {
                let last = voice.notes.last().unwrap();
                assert!(last.on <= hold && last.off == fugue.design.end, "{name}");
                assert!(
                    tonic.member(last.pitch.pc()).is_some(),
                    "{name}: {}",
                    last.pitch
                );
                classes.push((last.pitch.pc() + 12 - subject.key.tonic.pc()) % 12);
            }
            // The root in the outer voices, the third between them, and with four voices the
            // fifth as well.
            assert_eq!(
                (classes[0], classes[voices - 1]),
                (0, 0),
                "{name}: {classes:?}"
            );
            assert!(classes.contains(&4), "{name}: {classes:?}");
            assert!(voices < 4 || classes.contains(&7), "{name}: {classes:?}");
            let top = &fugue.piece.voices[0].notes;
            let (last, before) = (top[top.len() - 1], top[top.len() - 2]);
            assert_eq!(before.off, last.on, "{name}: a rest before the close");
            let arrives = before.pitch.to(last.pitch);
            assert!(
                arrives.is_step() || arrives.is_unison(),
                "{name}: {arrives:?}"
            );
        }
    }

    /// The cadences of the plan are cadences in the voices: at each the lowest tone is the
    /// root of its key, and where the lowest voice was free it comes from the fifth. Not every
    /// group of entries begins with one any more.
    #[test]
    fn the_cadences_have_their_bass() {
        for (name, voices) in [("bwv1080", 3), ("wtk1-02", 4)] {
            let subject = BUILTIN
                .iter()
                .find(|b| b.name == name)
                .unwrap()
                .subject()
                .unwrap();
            let options = Options {
                voices,
                beam: 60,
                ..Options::default()
            };
            let fugue = compose(&subject, &options).unwrap();
            let design = &fugue.design;
            assert!(design.cadences.len() >= 2, "{name}: {:?}", design.cadences);
            for cadence in &design.cadences {
                let lowest = fugue
                    .piece
                    .voices
                    .iter()
                    .filter_map(|v| v.at(cadence.at))
                    .min_by_key(|n| n.pitch.midi)
                    .unwrap();
                assert_eq!(
                    lowest.pitch.pc(),
                    cadence.key.tonic.pc(),
                    "{name}: the cadence at {}",
                    cadence.at
                );
            }
            let basses: Vec<&Given> = fugue
                .given
                .iter()
                .filter(|g| g.what == What::Bass)
                .collect();
            assert!(!basses.is_empty(), "{name}");
            // The fifth and the root, the fifth alone before a root that was given (a pedal
            // point), or the root alone after a fifth that was.
            for bass in basses {
                let (fifth, root) = (bass.key.degree(4).pc(), bass.key.tonic.pc());
                match bass.notes[..] {
                    [a, b] => {
                        assert_eq!((a.pitch.pc(), b.pitch.pc()), (fifth, root), "{name}");
                        assert_eq!(a.off, b.on, "{name}");
                    }
                    [a] => assert!(a.pitch.pc() == fifth || a.pitch.pc() == root, "{name}"),
                    _ => panic!("{name}: {:?}", bass.notes),
                }
            }
            // Fewer cadences than groups of entries with their episodes.
            let groups = design.entries.iter().map(|e| e.group).max().unwrap();
            let at_entries = design
                .cadences
                .iter()
                .filter(|c| design.entries.iter().any(|e| e.at == c.at))
                .count();
            assert!(
                at_entries <= 1 && at_entries < groups,
                "{name}: {at_entries} of {groups}"
            );
        }
    }

    #[test]
    fn the_cells_of_a_subject() {
        // An eighth rest and three tones; four tones; a tone held over the next unit, under
        // which no cell begins; two tones are no cell.
        let subject = parse("r8 c''8 d'' e'' f''8 g''16 f'' e''8 d'' | c''2. d''8 e''").unwrap();
        let cells = cells_of(&[&subject[..]], 48);
        assert_eq!(
            cells,
            [
                Cell {
                    onsets: vec![12, 24, 36],
                    steps: vec![1, 1],
                },
                Cell {
                    onsets: vec![0, 12, 18, 24, 36],
                    steps: vec![1, -1, -1, -1],
                },
            ]
        );
        // With a whole bar as the unit the first bar is one cell.
        assert_eq!(cells_of(&[&subject[..]], 96)[0].onsets.len(), 8);
    }

    /// With a planned bass the scaffold names a bass for its chords, the search puts it lowest
    /// where they begin about as often as Bach does under the chords of the analyses (86 %),
    /// and the chord file carries it.
    #[test]
    fn the_bass_is_planned() {
        let subject = BUILTIN
            .iter()
            .find(|b| b.name == "bwv1080")
            .unwrap()
            .subject()
            .unwrap();
        let options = Options {
            beam: 60,
            bass: true,
            ..Options::default()
        };
        let fugue = compose(&subject, &options).unwrap();
        assert_eq!(fugue.report(&options.profile).forbidden(), 0);
        let low = crate::bass::lowest(&fugue.piece, &fugue.harmony);
        assert!(low.named * 10 >= low.onsets * 8, "{low:?}");
        assert!(low.met * 10 >= low.named * 7, "{low:?}");
        // No six-four chord is planned: the fifth lies lowest only where a given tone has it.
        assert!(low.members[2] * 10 <= low.onsets, "{low:?}");
        let text = crate::chordline::write(
            &fugue.harmony,
            &fugue.piece,
            scaffold::step_of(fugue.piece.meter),
        );
        assert!(text.contains('/'), "{text}");
        // Without it no chord names a bass.
        let free = compose(
            &subject,
            &Options {
                bass: false,
                ..options.clone()
            },
        )
        .unwrap();
        assert!(free.harmony.spans.iter().all(|s| s.chord.bass.is_none()));
    }

    #[test]
    fn a_running_rhythm_for_thin_cells() {
        // Four tones from the start of the unit: the cell's own rhythm.
        let full = Cell {
            onsets: vec![0, 12, 18, 24, 36],
            steps: vec![1, -1, -1, -1],
        };
        assert_eq!(running(&full, 48), [0, 12, 18, 24, 36]);
        // Three eighths after a rest: the eighths run through the unit.
        let late = Cell {
            onsets: vec![12, 24, 36],
            steps: vec![1, 1],
        };
        assert_eq!(running(&late, 48), [0, 12, 24, 36]);
        // Three tones in a bar of 3/2 would be three: their value is halved.
        let slow = Cell {
            onsets: vec![0, 48, 96],
            steps: vec![1, 1],
        };
        assert_eq!(running(&slow, 144), [0, 24, 48, 72, 96, 120]);
        // A value that does not divide the unit: the cell stays as it is.
        let dotted = Cell {
            onsets: vec![0, 36, 72],
            steps: vec![1, 1],
        };
        assert_eq!(running(&dotted, 96), [0, 36, 72]);
    }

    /// The episodes are sequences and the free voices work with figures: the second setting
    /// keeps links of the first, stays without a prohibition, and what the protocol of phase
    /// B5 counts lies far above what chance gave before (recurrence about 62 %, sequence about
    /// 11 % of the free tones).
    #[test]
    fn the_free_voices_work_with_figures() {
        let subject = BUILTIN
            .iter()
            .find(|b| b.name == "wtk1-02")
            .unwrap()
            .subject()
            .unwrap();
        let options = Options {
            beam: 60,
            ..Options::default()
        };
        let fugue = compose(&subject, &options).unwrap();
        assert_eq!(fugue.report(&options.profile).forbidden(), 0);
        assert_eq!(fugue.tally.count, crate::engine::Count::default());
        let links = fugue
            .given
            .iter()
            .filter(|g| g.what == What::Sequence)
            .count();
        assert!(links >= 6, "{links} links");
        assert!(fugue.draft.voices.iter().any(|v| v.figures.len() >= 4));
        // A link is not a tone of the plan.
        let link = fugue
            .given
            .iter()
            .find(|g| g.what == What::Sequence)
            .unwrap();
        assert!(fugue.is_given(link.voice, &link.notes[0]));
        assert!(!fugue.is_planned(link.voice, &link.notes[0]));
        let bound: Vec<(usize, Tick)> = fugue
            .given
            .iter()
            .filter(|g| matches!(g.what, What::Subject | What::Answer | What::Countersubject))
            .flat_map(|g| g.notes.iter().map(move |n| (g.voice, n.on)))
            .collect();
        let free = |v: usize, note: &Note| !bound.contains(&(v, note.on));
        let material = vec![subject.notes.clone(), fugue.countersubject.clone()];
        let shares = crate::motifs::measure(&fugue.piece, &free, &material, 4).shares();
        assert!(shares[1] > 0.68 && shares[2] > 0.2, "{shares:?}");
        // Without the two, a fugue as before.
        let plain = compose(
            &subject,
            &Options {
                sequences: false,
                figures: false,
                ..options.clone()
            },
        )
        .unwrap();
        assert!(!plain.given.iter().any(|g| g.what == What::Sequence));
        assert!(plain.draft.voices.iter().all(|v| v.figures.is_empty()));
    }

    #[test]
    fn motifs_and_adapted_countersubjects() {
        let theme = parse("r8 c''16 b' c''8 g' as' c''16 b' c''8 d''").unwrap();
        let (head, length) = motif(&theme, 48);
        assert_eq!((head.len(), length), (4, 48));
        assert_eq!(tail(&theme, 48, 48).map(|t| t.len()), Some(5));
        // Slow subjects need more units for three tones.
        let slow = parse("d'2 a' | f' d'").unwrap();
        assert_eq!(motif(&slow, 48).1, 144);
        // A tone under a tone answered at the fourth moves down a fourth.
        let key = Key::parse("c").unwrap();
        let subject = parse("g'4 c'' es'' d''").unwrap();
        let answer = answer::answer(&subject, key);
        assert!(answer.fourth[0] && !answer.fourth[1]);
        let counter = parse("d'4 d' d' d'").unwrap();
        let moved: Vec<String> = adapted(&counter, &answer)
            .iter()
            .map(|n| n.pitch.lof().to_string())
            .collect();
        assert_eq!(moved, ["A", "G", "G", "G"]);
        // Degrees of the natural scale: c d es up two steps in C minor is es f g.
        let up: Vec<String> = shifted(&parse("c'4 d' es'").unwrap(), 2, key)
            .iter()
            .map(|n| n.pitch.lof().to_string())
            .collect();
        assert_eq!(up, ["Eb", "F", "G"]);
    }
}
