//! Recognises imitation in a score: the points whose entries take up the same head after rests,
//! as information for the check and as the measure of the corpus.
//!
//! The detector measures as the study of Palestrina's masses did (docs/regeln.md § 18), so that
//! what it finds in generated settings compares with his figures:
//!
//! - **Sections.** A score is cut where a held chord closes a section: at a tick at least four
//!   breves from both ends that no note and no rest crosses, where at least half of the voices,
//!   and two at least, end a note, each held for a dotted semibreve or longer and one for a breve.
//!   Of breaks less than three breves apart only the last counts ([`sections`]).
//! - **Entries.** The first note of a voice in its section, and every note after a rest of at
//!   least `min_rest` ticks. The head of an entry runs up to the voice's next such rest, at most
//!   40 notes; shorter rests stay in it.
//! - **Imitation** (the study's IMI-MATCH). Two entries of different voices, the second 1 to 96
//!   ticks (six breves) after the first, imitate each other where at least four notes agree from
//!   their first notes on: the same diatonic intervals, the same values except for the first note
//!   and the last that agrees, and rests inside the heads in the same places with the same
//!   values. One of the first two intervals may differ by a step, as a fourth for a fifth in a
//!   modal answer, a third for a second or a step for a repeated note; then five notes must agree,
//!   or four where the score's plan has them in one point and cuts either head to four notes.
//! - **Points.** The entries are taken in time order. Each joins the point, among those whose
//!   last entry lies at most 96 ticks before it, that has the member it imitates best: with the
//!   most notes agreeing (a changed interval counts one note less), on equal terms the latest
//!   member. An entry that imitates none opens a point of its own. Points with entries of two
//!   voices at least are points of imitation.
//! - **Openings.** A point opens its section if its leader is a voice's first note at most a
//!   breve after the section's first note; of several, the earliest, then the one with the most
//!   voices.
//!
//! Like the study, it compares diatonic steps only: an answer at the fifth that turns a whole
//! step into a half step is still real.

use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::ops::Range;

use super::{Answer, Relation};
use crate::mode::{Mode, ModeSpec};
use crate::pitch::Pitch;
use crate::score::{Score, Sounding, Voice};

/// The most ticks from an entry to a later one that imitates it, and from a point's last entry to
/// one that joins it: six breves.
const WINDOW: u32 = 96;
/// The notes that must agree for an imitation; one more with a changed interval.
const AGREEING: usize = 4;
/// The most notes of a head.
const HEAD_NOTES: usize = 40;
/// How long after its section's first note a point may begin and still open the section: a breve.
const OPENING: u32 = 16;
/// How far a section break lies from both ends of the score, at least: four breves.
const BREAK_MARGIN: u32 = 64;
/// How long every voice that sounds into a break has held its note, at least: a dotted semibreve.
const BREAK_HOLD: u32 = 12;
/// How long one of them has held it, at least: a breve.
const BREAK_LONGEST: u32 = 16;
/// Breaks less than three breves apart are one, the last.
const BREAK_MERGE: u32 = 48;

/// A point of imitation found in a score.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundPoint {
    /// The section that holds the point, by its index in [`sections`].
    pub section: usize,
    /// Whether the point opens its section: its leader is a voice's first note at most a breve
    /// after the section's first note, and no other such point begins earlier, or at the same
    /// tick with more voices. A section has one opening point at most.
    pub opens: bool,
    /// The entries in time order; the first is the leader's. A voice may enter more than once.
    pub entries: Vec<FoundEntry>,
}

impl FoundPoint {
    /// The voices of the point, in the order of their first entries.
    pub fn voices(&self) -> Vec<usize> {
        let mut voices = Vec::new();
        for entry in &self.entries {
            if !voices.contains(&entry.voice) {
                voices.push(entry.voice);
            }
        }
        voices
    }
}

/// An entry of a found point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundEntry {
    /// The voice, by its index in the score.
    pub voice: usize,
    /// The tick of its first note.
    pub start: u32,
    /// The silence before it within its section, in ticks; 0 where the voice goes on without a
    /// rest at the start of a section.
    pub rest: u32,
    /// Whether it is the voice's first note in its section.
    pub first: bool,
    /// The notes of its head as they sound, tied notes merged: up to the voice's next rest as long
    /// as those before entries ([`detect`]), at most 40 notes. Shorter rests lie between them.
    pub notes: Vec<Sounding>,
    /// The entry of the point that it imitates, by its index in the point; `None` for the
    /// leader.
    pub follows: Option<usize>,
    /// The notes that agree with that entry, four or more; 0 for the leader.
    pub matched: usize,
    /// The degree it begins on, against the leader's; [`Relation::Same`] for the leader itself.
    pub relation: Relation,
    /// How it takes up the intervals of the entry it imitates; [`Answered::Real`] for the leader.
    pub answer: Answered,
}

/// How a found entry takes up the intervals of the entry it imitates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Answered {
    /// With the same diatonic intervals: a real answer.
    Real,
    /// With interval `at` (0 or 1, from the first note) a step wider or narrower: `from` diatonic
    /// steps in the entry it imitates, `to` in this one, both in the same direction or one of
    /// them a repeated note. A fourth for a fifth, or a fifth for a fourth, makes a modal answer.
    Changed { at: u8, from: i8, to: i8 },
}

impl Answered {
    /// Whether the answer exchanges a fourth and a fifth: a modal answer.
    pub fn is_modal(self) -> bool {
        match self {
            Answered::Real => false,
            Answered::Changed { from, to, .. } => {
                from.signum() == to.signum() && matches!((from.abs(), to.abs()), (3, 4) | (4, 3))
            }
        }
    }

    /// The answer as a plan describes it: real, or modal where a fourth and a fifth are
    /// exchanged; `None` for another change.
    pub fn answer(self) -> Option<Answer> {
        match self {
            Answered::Real => Some(Answer::Real),
            Answered::Changed { at, .. } if self.is_modal() => Some(Answer::Modal { at }),
            Answered::Changed { .. } => None,
        }
    }
}

/// The sections of a score, as the detector cuts it: from the start to the first break, from
/// break to break, and from the last to the end. A section break is a tick at least four breves
/// from both ends that no note and no rest crosses, where at least half of the voices, and two at
/// least, end a note that they have held for a dotted semibreve or longer, one of them for a
/// breve. Of breaks less than three breves apart only the last counts: a held chord struck again.
pub fn sections(score: &Score) -> Vec<Range<u32>> {
    let lines: Vec<Vec<Event>> = score.voices.iter().map(events).collect();
    spans(&lines, score.total_ticks())
}

/// The points of imitation in a score, section by section and in the order of their leaders'
/// entries. An entry is a voice's first note in its section or a note after a rest of at least
/// `min_rest` ticks: 8, a semibreve, as the study of Palestrina's masses counted them; 4 also
/// takes the re-entries after a minim rest.
pub fn detect(score: &Score, min_rest: u32) -> Vec<FoundPoint> {
    let lines: Vec<Vec<Event>> = score.voices.iter().map(events).collect();
    // The planned entries, if the score has a plan: each with its point and its head's notes.
    let planned: Vec<((usize, u32), (usize, usize))> = score
        .imitation
        .iter()
        .flat_map(|plan| plan.points.iter().enumerate())
        .flat_map(|(p, point)| {
            point
                .entries
                .iter()
                .map(move |e| ((e.voice, e.start), (p, e.notes.len())))
        })
        .collect();
    let mut found = Vec::new();
    for (section, span) in spans(&lines, score.total_ticks()).iter().enumerate() {
        let mut entries = section_entries(&lines, span, min_rest);
        for entry in &mut entries {
            entry.planned = planned
                .iter()
                .find(|(at, _)| *at == (entry.voice, entry.start))
                .map(|&(_, plan)| plan);
        }
        let Some(first) = entries.first().map(|e| e.start) else {
            continue;
        };
        let mut points: Vec<FoundPoint> = build(&entries)
            .into_iter()
            .map(|members| found_point(score.mode, section, &entries, &members))
            .filter(|point| point.voices().len() >= 2)
            .collect();
        let opening = points
            .iter()
            .enumerate()
            .filter(|(_, point)| {
                let leader = &point.entries[0];
                leader.first && leader.start - first <= OPENING
            })
            .min_by_key(|(_, point)| (point.entries[0].start, Reverse(point.voices().len())))
            .map(|(i, _)| i);
        if let Some(i) = opening {
            points[i].opens = true;
        }
        found.extend(points);
    }
    found
}

/// A note or a rest as the detector reads a voice: a note as it sounds, tied notes merged, and a
/// rest as the whole silence between two notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Event {
    start: u32,
    end: u32,
    /// `None` for a rest.
    note: Option<Sounding>,
}

impl Event {
    fn ticks(self) -> u32 {
        self.end - self.start
    }

    fn pitch(self) -> Option<Pitch> {
        self.note.map(|s| s.pitch)
    }
}

/// The notes and rests of a voice.
fn events(voice: &Voice) -> Vec<Event> {
    let mut out = Vec::new();
    let mut at = 0;
    let rest = |start, end| Event {
        start,
        end,
        note: None,
    };
    for sounding in voice.soundings() {
        if sounding.start > at {
            out.push(rest(at, sounding.start));
        }
        out.push(Event {
            start: sounding.start,
            end: sounding.end,
            note: Some(sounding),
        });
        at = sounding.end;
    }
    if voice.total_ticks() > at {
        out.push(rest(at, voice.total_ticks()));
    }
    out
}

/// The sections of voices that last `total` ticks; see [`sections`].
fn spans(lines: &[Vec<Event>], total: u32) -> Vec<Range<u32>> {
    let candidates: BTreeSet<u32> = lines
        .iter()
        .flatten()
        .map(|e| e.end)
        .filter(|&tick| tick >= BREAK_MARGIN && tick + BREAK_MARGIN <= total)
        .collect();
    let mut breaks: Vec<u32> = Vec::new();
    for tick in candidates.into_iter().filter(|&tick| is_break(lines, tick)) {
        match breaks.last_mut() {
            Some(last) if tick - *last < BREAK_MERGE => *last = tick,
            _ => breaks.push(tick),
        }
    }
    let mut bounds = vec![0];
    bounds.extend(breaks);
    bounds.push(total);
    bounds.windows(2).map(|w| w[0]..w[1]).collect()
}

/// Whether a held chord closes a section at `tick`; see [`sections`].
fn is_break(lines: &[Vec<Event>], tick: u32) -> bool {
    let (mut sounding, mut longest) = (0, 0);
    for line in lines {
        let before = &line[..line.partition_point(|e| e.start < tick)];
        if before.last().is_some_and(|e| e.end > tick) {
            return false;
        }
        if let Some(last) = before.iter().rev().find(|e| e.note.is_some())
            && last.end == tick
        {
            let held = tick - last.start;
            if held < BREAK_HOLD {
                return false;
            }
            sounding += 1;
            longest = longest.max(held);
        }
    }
    sounding >= 2 && 2 * sounding >= lines.len() && longest >= BREAK_LONGEST
}

/// An entry of a section with its head, while the points are built.
struct Candidate {
    voice: usize,
    start: u32,
    rest: u32,
    first: bool,
    /// The head: its notes, and the rests between them that are shorter than an entry's.
    head: Vec<Event>,
    /// The point of the score's plan that has this entry, and how many notes its head has there.
    planned: Option<(usize, usize)>,
}

/// The entries of the voices in a section, in time order and from the top on equal ticks.
fn section_entries(lines: &[Vec<Event>], span: &Range<u32>, min_rest: u32) -> Vec<Candidate> {
    let mut out = Vec::new();
    for (voice, line) in lines.iter().enumerate() {
        let events: Vec<Event> = line
            .iter()
            .filter(|e| span.contains(&e.start))
            .map(|&e| Event {
                end: e.end.min(span.end),
                note: e.note.map(|s| Sounding {
                    end: s.end.min(span.end),
                    ..s
                }),
                ..e
            })
            .collect();
        let mut first = true;
        for (i, &event) in events.iter().enumerate() {
            if event.note.is_none() {
                continue;
            }
            let rest = match i.checked_sub(1).map(|b| events[b]) {
                Some(before) if before.note.is_none() => before.ticks(),
                _ => 0,
            };
            if first || (rest > 0 && rest >= min_rest) {
                out.push(Candidate {
                    voice,
                    start: event.start,
                    rest,
                    first,
                    head: head(&events[i..], min_rest),
                    planned: None,
                });
            }
            first = false;
        }
    }
    out.sort_by_key(|c| (c.start, c.voice));
    out
}

/// The head of an entry, from its first note on: up to the next rest of at least `min_rest`
/// ticks, at most [`HEAD_NOTES`] notes, never ending with a rest.
fn head(events: &[Event], min_rest: u32) -> Vec<Event> {
    let mut head = Vec::new();
    let mut notes = 0;
    for &event in events {
        if event.note.is_none() && event.ticks() >= min_rest {
            break;
        }
        head.push(event);
        if event.note.is_some() {
            notes += 1;
            if notes == HEAD_NOTES {
                break;
            }
        }
    }
    while head.last().is_some_and(|e| e.note.is_none()) {
        head.pop();
    }
    head
}

/// A changed interval: interval `at` (from 0) of `from` diatonic steps in one head and `to` in
/// the other.
type Change = (u8, i8, i8);

/// How far the heads `a` and `b` agree from their first notes on: the notes that agree, and the
/// interval changed by a step on the way, if any.
fn agree(a: &[Event], b: &[Event]) -> (usize, Option<Change>) {
    let (Some(mut pa), Some(mut pb)) = (
        a.first().and_then(|e| e.pitch()),
        b.first().and_then(|e| e.pitch()),
    ) else {
        return (0, None);
    };
    let (mut matched, mut change) = (1, None);
    for j in 1..a.len().min(b.len()) {
        if a[j].note.is_none() != b[j].note.is_none() {
            break;
        }
        // The event before becomes an inner one, whose value must agree; the first's need not.
        if j >= 2 && a[j - 1].ticks() != b[j - 1].ticks() {
            break;
        }
        let (Some(qa), Some(qb)) = (a[j].pitch(), b[j].pitch()) else {
            continue;
        };
        let (ia, ib) = (qa.diatonic() - pa.diatonic(), qb.diatonic() - pb.diatonic());
        if ia != ib {
            // One of the first two intervals, by a step: in the same direction, or one of them a
            // repeated note.
            let at = matched - 1;
            if change.is_some() || at >= 2 || (ia - ib).abs() != 1 {
                break;
            }
            change = Some((at as u8, steps(ia), steps(ib)));
        }
        matched += 1;
        (pa, pb) = (qa, qb);
    }
    (matched, change)
}

/// Diatonic steps as a small number, saturating.
fn steps(n: i32) -> i8 {
    n.clamp(i8::MIN.into(), i8::MAX.into()) as i8
}

/// What a member of a point imitates: another member, by its index in the point, with the notes
/// that agree and the changed interval.
#[derive(Clone, Copy)]
struct Follows {
    member: usize,
    matched: usize,
    change: Option<Change>,
}

/// A member of a point while the points are built: the entry, by its index among the section's,
/// and what it imitates.
struct Member {
    entry: usize,
    follows: Option<Follows>,
}

/// The points of a section's entries, each as its members in time order; a point of one voice
/// included.
fn build(entries: &[Candidate]) -> Vec<Vec<Member>> {
    // The members of each point and the tick of its last entry.
    let mut points: Vec<(Vec<Member>, u32)> = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        // The best match so far: its rank, the point and what the entry imitates there.
        let mut best: Option<((bool, usize, u32), usize, Follows)> = None;
        for (p, (members, last)) in points.iter().enumerate() {
            if e.start - last > WINDOW {
                continue;
            }
            for (k, member) in members.iter().enumerate() {
                let x = &entries[member.entry];
                if x.voice == e.voice || x.start == e.start || e.start - x.start > WINDOW {
                    continue;
                }
                let (matched, change) = agree(&x.head, &e.head);
                let changed = usize::from(change.is_some());
                // A changed interval needs a note more, unless the plan cuts either head of the
                // pair to four notes.
                let needed = match (x.planned, e.planned) {
                    (Some((px, lx)), Some((pe, le))) if px == pe => {
                        (AGREEING + changed).min(lx.min(le).max(AGREEING))
                    }
                    _ => AGREEING + changed,
                };
                if matched < needed {
                    continue;
                }
                // A member of the same planned point first: a restatement's follower may agree
                // in more notes with an entry of the first statement than with its own leader,
                // whose head the plan cut.
                let planned = matches!(
                    (x.planned, e.planned),
                    (Some((px, _)), Some((pe, _))) if px == pe
                );
                let rank = (planned, matched - changed, x.start);
                if best.as_ref().is_none_or(|b| rank > b.0) {
                    let follows = Follows {
                        member: k,
                        matched,
                        change,
                    };
                    best = Some((rank, p, follows));
                }
            }
        }
        match best {
            Some((_, p, follows)) => {
                let (members, last) = &mut points[p];
                members.push(Member {
                    entry: i,
                    follows: Some(follows),
                });
                *last = e.start;
            }
            None => points.push((
                vec![Member {
                    entry: i,
                    follows: None,
                }],
                e.start,
            )),
        }
    }
    points.into_iter().map(|(members, _)| members).collect()
}

/// A built point as the detector reports it.
fn found_point(
    mode: ModeSpec,
    section: usize,
    entries: &[Candidate],
    members: &[Member],
) -> FoundPoint {
    let first_pitch = |c: &Candidate| c.head[0].pitch().expect("a head begins with a note");
    let leader = first_pitch(&entries[members[0].entry]);
    let entries = members
        .iter()
        .map(|member| {
            let c = &entries[member.entry];
            let (follows, matched, answer) = match member.follows {
                None => (None, 0, Answered::Real),
                Some(f) => {
                    let answer = match f.change {
                        None => Answered::Real,
                        Some((at, from, to)) => Answered::Changed { at, from, to },
                    };
                    (Some(f.member), f.matched, answer)
                }
            };
            FoundEntry {
                voice: c.voice,
                start: c.start,
                rest: c.rest,
                first: c.first,
                notes: c.head.iter().filter_map(|e| e.note).collect(),
                follows,
                matched,
                relation: relation(mode, leader, first_pitch(c)),
                answer,
            }
        })
        .collect();
    FoundPoint {
        section,
        opens: false,
        entries,
    }
}

/// The degree an entry on `entry` begins on against a leader on `leader`: the same at the unison
/// or an octave; the other main degree for the fifth against the final and back and for the final
/// against the fourth, in the Phrygian mode for A against E and back; otherwise the diatonic
/// steps from the leader's note.
fn relation(mode: ModeSpec, leader: Pitch, entry: Pitch) -> Relation {
    let distance = entry.diatonic() - leader.diatonic();
    if distance.rem_euclid(7) == 0 {
        return Relation::Same;
    }
    let degrees = (mode.degree(leader), mode.degree(entry));
    let main = if mode.mode == Mode::Phrygian {
        matches!(degrees, (1, 4) | (4, 1))
    } else {
        matches!(degrees, (1, 5) | (5, 1) | (4, 1))
    };
    if main {
        Relation::Dominant
    } else {
        Relation::Steps(steps(distance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clef::VoiceType;
    use crate::parse::parse_lilypond;
    use crate::score::{Note, Role, ScoreEntry, ScoreImitation, Species};
    use crate::time::Duration;

    /// A free score in `mode` with the given voices, from the top, in measures of 2/2: notes
    /// across a barline become tied notes, and rests fill every voice up to the same whole number
    /// of measures.
    fn free_score(mode: &str, voices: Vec<Vec<Note>>) -> Score {
        let per = 8;
        let ticks = |notes: &[Note]| notes.iter().map(|n| n.duration.ticks()).sum::<u32>();
        let total = voices.iter().map(|v| ticks(v)).max().unwrap_or(0);
        let total = total.div_ceil(per) * per;
        let voices = voices
            .into_iter()
            .map(|mut written| {
                let rests = total - ticks(&written);
                written.push(Note::rest(Duration::from_ticks(rests)));
                let mut notes = Vec::new();
                let mut at = 0;
                for note in written {
                    let mut left = note.duration.ticks();
                    while left > 0 {
                        let ticks = left.min(per - at % per);
                        (left, at) = (left - ticks, at + ticks);
                        notes.push(Note {
                            duration: Duration::from_ticks(ticks),
                            tie: note.tie || (left > 0 && !note.is_rest()),
                            ..note.clone()
                        });
                    }
                }
                let pitches: Vec<Pitch> = notes.iter().filter_map(|n| n.pitch).collect();
                let voice_type = VoiceType::best_for(&pitches);
                Voice::new(voice_type, Role::Counterpoint(Species::Free), notes)
            })
            .collect();
        let score = Score::new(mode.parse().unwrap(), voices);
        score.validate().unwrap();
        score
    }

    /// A free score in `mode` with voices in LilyPond notation (whole notes unless marked), from
    /// the top; see [`free_score`].
    fn score(mode: &str, voices: &[&str]) -> Score {
        let voices = voices
            .iter()
            .map(|voice| parse_lilypond(voice, Duration::WHOLE).unwrap())
            .collect();
        free_score(mode, voices)
    }

    /// The heads of a plan's entries in a Dorian score of `voices` voices, with rests everywhere
    /// else.
    fn placed(plan: &ScoreImitation, voices: usize) -> Score {
        let mut lines = vec![Vec::new(); voices];
        let mut ends = vec![0; voices];
        let mut entries: Vec<&ScoreEntry> = plan.points.iter().flat_map(|p| &p.entries).collect();
        entries.sort_by_key(|e| e.start);
        for entry in entries {
            let rest = entry.start - ends[entry.voice];
            if rest > 0 {
                lines[entry.voice].push(Note::rest(Duration::from_ticks(rest)));
            }
            for &(pitch, ticks) in &entry.notes {
                lines[entry.voice].push(Note::new(pitch, Duration::from_ticks(ticks)));
            }
            ends[entry.voice] = entry.start + entry.notes.iter().map(|&(_, t)| t).sum::<u32>();
        }
        free_score("dorian", lines)
    }

    /// The entries of a point as `voice@start`, each follower with the entry it imitates and the
    /// notes that agree, such as `0@16<1:7`.
    fn entries_of(point: &FoundPoint) -> Vec<String> {
        point
            .entries
            .iter()
            .map(|e| match e.follows {
                None => format!("{}@{}", e.voice, e.start),
                Some(k) => format!("{}@{}<{k}:{}", e.voice, e.start, e.matched),
            })
            .collect()
    }

    fn pitch(name: &str) -> Pitch {
        name.parse().unwrap()
    }

    /// The first and the last tick of each section of a score.
    fn section_ticks(score: &Score) -> Vec<(u32, u32)> {
        sections(score).iter().map(|s| (s.start, s.end)).collect()
    }

    /// The alto leads, the soprano answers at the upper fifth two measures later and the bass at
    /// the lower octave two measures after it: one point that opens the piece. The bass imitates
    /// both entries before it in all seven notes and follows the later one.
    #[test]
    fn entries_at_the_fifth_and_the_octave_form_a_point() {
        let s = score(
            "dorian",
            &[
                r"r\breve a'1 b'2 c'' d''1 c''2 b' a'1",
                "d'1 e'2 f' g'1 f'2 e' d'1",
                r"r\breve r\breve d1 e2 f g1 f2 e d1",
            ],
        );
        let points = detect(&s, 8);
        assert_eq!(points.len(), 1);
        let point = &points[0];
        assert_eq!(entries_of(point), ["1@0", "0@16<0:7", "2@32<1:7"]);
        assert_eq!((point.section, point.opens), (0, true));
        assert_eq!(point.voices(), [1, 0, 2]);
        let relations: Vec<Relation> = point.entries.iter().map(|e| e.relation).collect();
        assert_eq!(
            relations,
            [Relation::Same, Relation::Dominant, Relation::Same]
        );
        assert!(point.entries.iter().all(|e| e.answer == Answered::Real));
        assert!(point.entries.iter().all(|e| e.first));
        let rests: Vec<u32> = point.entries.iter().map(|e| e.rest).collect();
        assert_eq!(rests, [0, 16, 32]);
        // The leader's head as it sounds; the soprano's begins with its third written note,
        // after the breve rest written as two.
        let leader: Vec<(u32, Pitch, u32)> = point.entries[0]
            .notes
            .iter()
            .map(|s| (s.start, s.pitch, s.end))
            .collect();
        let expected: Vec<(u32, Pitch, u32)> = [
            (0, "D4", 8),
            (8, "E4", 12),
            (12, "F4", 16),
            (16, "G4", 24),
            (24, "F4", 28),
            (28, "E4", 32),
            (32, "D4", 40),
        ]
        .into_iter()
        .map(|(start, p, end)| (start, pitch(p), end))
        .collect();
        assert_eq!(leader, expected);
        assert_eq!(point.entries[1].notes[0].note, 2);
        assert_eq!(section_ticks(&s), [(0, 72)]);
    }

    /// One of the first two intervals may differ by a step, as the fifth D–A answered by the
    /// fourth A–D; then five notes must agree, not four. A third for a step is a change too, but
    /// no modal answer; a change in the third interval ends the imitation there.
    #[test]
    fn a_changed_interval_needs_five_notes() {
        let modal = score(
            "dorian",
            &[r"r\breve a'1 d'' c''2 b' a' g'1", "d'1 a' g'2 f' e' d'1"],
        );
        let points = detect(&modal, 8);
        assert_eq!(entries_of(&points[0]), ["1@0", "0@16<0:6"]);
        let answer = points[0].entries[1].answer;
        assert_eq!(
            answer,
            Answered::Changed {
                at: 0,
                from: 4,
                to: 3
            }
        );
        assert!(answer.is_modal());
        assert_eq!(answer.answer(), Some(Answer::Modal { at: 0 }));
        assert_eq!(points[0].entries[1].relation, Relation::Dominant);
        // Four notes are not enough with the change.
        let short = score("dorian", &[r"r\breve a'1 d'' c''2 b'", "d'1 a' g'2 f'"]);
        assert_eq!(detect(&short, 8), []);
        // A third for a step in the first interval, a repeated note for a step in the second.
        let third = score(
            "dorian",
            &[r"r\breve a'1 c'' d''2 e'' f''1", "d'1 e' f'2 g' a'1"],
        );
        let answer = detect(&third, 8)[0].entries[1].answer;
        assert_eq!(
            answer,
            Answered::Changed {
                at: 0,
                from: 1,
                to: 2
            }
        );
        assert!(!answer.is_modal());
        assert_eq!(answer.answer(), None);
        let repeat = score(
            "dorian",
            &[r"r\breve a'1 b' b'2 c'' d''1", "d'1 e' f'2 g' a'1"],
        );
        assert_eq!(
            detect(&repeat, 8)[0].entries[1].answer,
            Answered::Changed {
                at: 1,
                from: 1,
                to: 0
            }
        );
        // In the third interval it comes too late: three notes agree.
        let late = score(
            "dorian",
            &[r"r\breve a'1 b' c''2 e'' f''1", "d'1 e' f'2 g' a'1"],
        );
        assert_eq!(detect(&late, 8), []);
    }

    /// The first note may be longer or shorter, and so may the last that agrees; another value
    /// between them ends the imitation.
    #[test]
    fn values_agree_but_for_the_first_and_the_last() {
        let alto = r"d'\breve e'2 f' g'1 a'";
        let ends = score("dorian", &[r"r1 r1 r1 a'1 b'2 c'' d''1 e''\breve", alto]);
        assert_eq!(entries_of(&detect(&ends, 8)[0]), ["1@0", "0@24<0:5"]);
        let inner = score("dorian", &["r1 r1 r1 a'1 b' c''2 d''1 e''", alto]);
        assert_eq!(detect(&inner, 8), []);
    }

    /// An entry follows a rest of at least `min_rest` ticks, and its head runs up to the next such
    /// rest. With semibreve rests the soprano's subject after a minim rest is no entry, and its
    /// first note, at the same tick as the alto's, imitates nothing; with minim rests it is one.
    #[test]
    fn entries_follow_rests_of_the_given_length() {
        let s = score(
            "dorian",
            &[
                "a'1 g'2 r a'1 b'2 c'' d''1 c''2 b' a'1",
                "d'1 e'2 f' g'1 f'2 e' d'1",
            ],
        );
        assert_eq!(detect(&s, 8), []);
        let points = detect(&s, 4);
        assert_eq!(entries_of(&points[0]), ["1@0", "0@16<0:7"]);
        let entry = &points[0].entries[1];
        assert_eq!((entry.rest, entry.first), (4, false));
    }

    /// Rests shorter than an entry's stay in the head and must stand in the same places, with the
    /// same values, in both heads.
    #[test]
    fn rests_inside_heads_must_agree() {
        let alto = "d'1 r2 e' f'1 g'";
        let same = score("dorian", &["r1 r1 r1 a'1 r2 b' c''1 d''", alto]);
        assert_eq!(entries_of(&detect(&same, 8)[0]), ["1@0", "0@24<0:4"]);
        let elsewhere = score("dorian", &["r1 r1 r1 a'1 b'2 r c''1 d''", alto]);
        assert_eq!(detect(&elsewhere, 8), []);
    }

    /// Entries imitate each other from 1 to 96 ticks apart, six breves; not at the same tick.
    #[test]
    fn entries_imitate_within_six_breves() {
        let alto = "d'1 e'2 f' g'1 a'";
        let after = |rests: &str| {
            let soprano = format!("{rests} a'1 b'2 c'' d''1 e''");
            detect(&score("dorian", &[&soprano, alto]), 8)
        };
        let breves = r"r\breve r r r r r";
        assert_eq!(entries_of(&after(breves)[0]), ["1@0", "0@96<0:5"]);
        assert_eq!(after(&format!("{breves} r8")), []);
        assert_eq!(after(""), []);
    }

    /// The bass imitates the alto in all seven notes and the soprano, which changes the sixth
    /// note, in five: it follows the alto.
    #[test]
    fn entries_follow_the_entry_they_imitate_best() {
        let s = score(
            "dorian",
            &[
                r"r\breve a'1 b'2 c'' d''1 c''2 a' b'1",
                "d'1 e'2 f' g'1 f'2 e' d'1",
                r"r\breve r\breve d1 e2 f g1 f2 e d1",
            ],
        );
        assert_eq!(
            entries_of(&detect(&s, 8)[0]),
            ["1@0", "0@16<0:5", "2@32<0:7"]
        );
    }

    /// A voice that takes up its own head again is no point of imitation.
    #[test]
    fn a_voice_alone_is_no_point() {
        let s = score(
            "dorian",
            &[
                "d'1 e'2 f' g'1 a' r d'1 e'2 f' g'1 a'",
                r"a\breve f\breve g1",
            ],
        );
        assert_eq!(detect(&s, 8), []);
    }

    /// A chord held for a breve closes the first section. The soprano goes on into the second
    /// without a rest, and its first note there leads the second section's point; where it holds
    /// the chord for only a semibreve, the section goes on and that note is no entry.
    #[test]
    fn held_chords_cut_sections() {
        let piece = |chord: &str| {
            let soprano =
                format!(r"r\breve a'1 b'2 c'' d''1 c''2 b' a'1 r r r {chord} d''1 c''2 b' a'1 g'");
            score(
                "dorian",
                &[
                    &soprano,
                    r"d'1 e'2 f' g'1 f'2 e' d'1 r r r r r f'\breve r\breve g'1 f'2 e' d'1 c'",
                    r"r\breve r\breve d1 e2 f g1 f2 e d1 d\breve. d\breve d d g",
                ],
            )
        };
        let held = piece(r"a'\breve");
        assert_eq!(section_ticks(&held), [(0, 96), (96, 160)]);
        let points = detect(&held, 8);
        assert_eq!(points.len(), 2);
        assert_eq!(entries_of(&points[0]), ["1@0", "0@16<0:7", "2@32<1:7"]);
        assert_eq!(entries_of(&points[1]), ["0@96", "1@112<0:5"]);
        assert_eq!((points[1].section, points[1].opens), (1, true));
        let leader = &points[1].entries[0];
        assert_eq!((leader.rest, leader.first), (0, true));
        let short = piece("a'1 g'");
        assert_eq!(section_ticks(&short), [(0, 160)]);
        assert_eq!(detect(&short, 8).len(), 1);
    }

    /// A point opens its section if its leader is a voice's first note at most a breve after the
    /// section's first note.
    #[test]
    fn points_open_their_section_from_its_start() {
        let opens = |rests: &str| {
            let soprano = format!(r"{rests} r\breve a'1 b'2 c'' d''1 e''");
            let alto = format!("{rests} d'1 e'2 f' g'1 a'");
            let bass = r"d\breve f\breve e\breve d\breve";
            let points = detect(&score("dorian", &[&soprano, &alto, bass]), 8);
            assert_eq!(points.len(), 1);
            points[0].opens
        };
        assert!(opens(r"r\breve"));
        assert!(!opens(r"r\breve r1"));
    }

    /// The heads of a plan are found as its points, entry by entry: Kyrie I in three voices, the
    /// tenor on the minim beat with its first note tied over the barline, then the tail in two
    /// voices after minim rests at least.
    #[test]
    fn planned_points_are_found() {
        let plan = ScoreImitation::parse(
            "p0-7g8|0@0/0:A4.8,A4.4,B4.4,C5.12,B4.4;1@12/0:D4.8,D4.4,E4.4,F4.12;\
             2@24/0:A2.8,A2.4,B2.4,C3.8|p3-7|1@68/4:A3.8,C4.4,Bb3.4,A3.8;\
             0@76/4:D4.8,F4.4,E4.4,D4.8",
        )
        .unwrap();
        let found = detect(&placed(&plan, 3), 4);
        let planned: Vec<Vec<(usize, u32)>> = plan
            .points
            .iter()
            .map(|p| p.entries.iter().map(|e| (e.voice, e.start)).collect())
            .collect();
        let detected: Vec<Vec<(usize, u32)>> = found
            .iter()
            .map(|p| p.entries.iter().map(|e| (e.voice, e.start)).collect())
            .collect();
        assert_eq!(detected, planned);
        assert_eq!(entries_of(&found[0]), ["0@0", "1@12<0:4", "2@24<1:4"]);
        let relations: Vec<Relation> = found[0].entries.iter().map(|e| e.relation).collect();
        assert_eq!(
            relations,
            [Relation::Same, Relation::Dominant, Relation::Same]
        );
        assert_eq!(found[1].entries[1].relation, Relation::Dominant);
        assert_eq!((found[0].opens, found[1].opens), (true, false));
    }

    /// A restatement's follower joins its own point, where the score's plan puts it, though it
    /// agrees in more notes with an entry of the first statement within six breves than with
    /// the restatement's leader, whose head the plan cut to five notes; without the plan it
    /// follows the longer agreement, and the leader, alone, is no point.
    #[test]
    fn planned_restatements_keep_their_followers() {
        let plan = ScoreImitation::parse(
            "p0-7|0@0/0:A4.8,B4.4,C5.4,D5.8,C5.4,B4.4,A4.8,G4.8;\
             1@16/0:D4.8,E4.4,F4.4,G4.8,F4.4,E4.4,D4.8,C4.8;\
             2@40/0:D3.8,E3.4,F3.4,G3.8,F3.4,E3.4,D3.8,C3.8|\
             p0-7|2@120/32:D3.8,E3.4,F3.4,G3.8,F3.4;\
             1@136/72:D4.8,E4.4,F4.4,G4.8,F4.4,E4.4,D4.8,C4.8",
        )
        .unwrap();
        let mut score = placed(&plan, 3);
        let voices = |found: &[FoundPoint]| -> Vec<Vec<(usize, u32)>> {
            found
                .iter()
                .map(|p| p.entries.iter().map(|e| (e.voice, e.start)).collect())
                .collect()
        };
        assert_eq!(
            voices(&detect(&score, 4)),
            [vec![(0, 0), (1, 16), (2, 40), (1, 136)]]
        );
        score.imitation = Some(std::sync::Arc::new(plan));
        assert_eq!(
            voices(&detect(&score, 4)),
            [vec![(0, 0), (1, 16), (2, 40)], vec![(2, 120), (1, 136)]]
        );
    }

    /// A modal answer needs five agreeing notes, but where the score's plan cuts either head of
    /// the pair to four notes, four will do: both heads of four notes, or a real leader of four
    /// and a modal follower of five, as the planner keeps them.
    #[test]
    fn planned_modal_answers_of_four_notes_are_found() {
        for spec in [
            "p0-4|1@0/0:D4.8,A4.8,G4.8,F4.8;0@16/0:A4.8,D5.8,C5.8,B4.8",
            "p0-4|1@0/0:D4.8,A4.8,G4.8,F4.8;0@16/0:A4.8,D5.8,C5.8,B4.8,A4.8",
        ] {
            let plan = ScoreImitation::parse(spec).unwrap();
            let mut score = placed(&plan, 2);
            assert_eq!(detect(&score, 4), [], "{spec}");
            score.imitation = Some(std::sync::Arc::new(plan));
            let found = detect(&score, 4);
            assert_eq!(found.len(), 1, "{spec}");
            assert_eq!(entries_of(&found[0]), ["1@0", "0@16<0:4"], "{spec}");
            assert!(found[0].entries[1].answer.is_modal(), "{spec}");
        }
    }

    /// The other main degree: the fifth against the final and back, the final against the
    /// fourth; in the Phrygian mode A against E and back.
    #[test]
    fn relations_name_the_main_degrees() {
        let relation_in = |mode: &str, leader: &str, entry: &str| {
            relation(mode.parse().unwrap(), pitch(leader), pitch(entry))
        };
        let dorian = |leader, entry| relation_in("dorian", leader, entry);
        assert_eq!(dorian("D4", "D3"), Relation::Same);
        assert_eq!(dorian("D4", "D4"), Relation::Same);
        assert_eq!(dorian("D4", "A4"), Relation::Dominant);
        assert_eq!(dorian("D4", "A3"), Relation::Dominant);
        assert_eq!(dorian("A4", "D4"), Relation::Dominant);
        assert_eq!(dorian("G4", "D5"), Relation::Dominant);
        assert_eq!(dorian("D4", "G4"), Relation::Steps(3));
        assert_eq!(dorian("D4", "F4"), Relation::Steps(2));
        assert_eq!(dorian("E4", "B3"), Relation::Steps(-3));
        let phrygian = |leader, entry| relation_in("phrygian", leader, entry);
        assert_eq!(phrygian("E4", "A4"), Relation::Dominant);
        assert_eq!(phrygian("A4", "E4"), Relation::Dominant);
        assert_eq!(phrygian("E4", "B4"), Relation::Steps(4));
        // Transposed, the Dorian final lies on G.
        let transposed = |leader, entry| relation_in("dorian-transposed", leader, entry);
        assert_eq!(transposed("G4", "D5"), Relation::Dominant);
        assert_eq!(transposed("C5", "G4"), Relation::Dominant);
        assert_eq!(transposed("D4", "A4"), Relation::Steps(4));
    }
}
