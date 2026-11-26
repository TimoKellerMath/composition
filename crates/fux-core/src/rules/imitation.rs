//! The rules of imitation (IMI-*, docs/regeln.md § 18): how a setting carries out the points of
//! imitation it was planned with (phase 13), after Zarlino (1558), Vicentino (1555), Pontio
//! (1595), Fux (1725), Morley, Jeppesen and the openings of Palestrina's masses.
//!
//! A point sets some words of the text, and its voices take them up one after the other with the
//! head of one subject; the plan in the score ([`Score::imitation`]) says for every entry where
//! it stands and what its head sings. IMI-ENTRY and IMI-REST compare the voices with the plan:
//! whether each head sounds as planned, after a rest and on a minim beat. IMI-SILENCE prices the
//! silences of the voices around the plan: a voice that falls silent before every voice has
//! entered, for longer than four breves, or half of a choir of four or six at once after the
//! exposition; IMI-HELD the dissonances against notes tied over the barline that the rules of
//! free counterpoint excuse, which long head notes make frequent. The others judge the plan as
//! the treatises and Palestrina's practice do: the interval and the time of each entry against
//! the leader, the first entry of its point (IMI-INTERVAL, IMI-TIME), the answer (IMI-ANSWER),
//! the shape of each point's subject and the range of each head (IMI-SUBJECT), the degree a
//! later point begins on (IMI-DEGREE) and how it follows the cadence before it (IMI-OVERLAP).
//!
//! Each case is decided at a tick of the plan where the voices show it: an entry's attack, the
//! attack of a head note, the end of a note (the voice's next event). It is reported there once
//! the voices it concerns are written up to that tick, on a note that sounds there, so that the
//! windows and the prefixes of the search find each case once. Without a plan the rules stay
//! silent, as on every score of the corpora; IMI-ENTRY compares syllables only where both voices
//! sing the text, which the voices of the search do not.
//!
//! The parts of a mass that begin together have no plan (phase 14), yet their held notes and
//! their silences are those of imitative parts. IMI-HELD and IMI-SILENCE `thin` judge them with
//! parameters of their own, `unplanned_struck`, `unplanned_passing` and `unplanned_thin`, for
//! scores without a plan or with a plan without points; the standard profile leaves them off,
//! so that the calibrations stay as they were, and the profile of the mass sets them to the
//! levels the cases have with a plan.
//!
//! [`Score::imitation`]: crate::score::Score::imitation

use super::analysis::species;
use super::dissonance::{Clash, clash_with, repeated};
use super::melodic::level;
use super::species::{judged_suspension, passing_case, steps_down};
use super::{Ctx, Detail, Level, Rule, RuleInfo, Violation};
use crate::mode::Mode;
use crate::pitch::Pitch;
use crate::score::{Lyric, ScoreEntry, ScoreImitation, ScorePoint, Sounding, Species};
use crate::time::Duration;

/// A minim: entries begin on its beats, and a re-entry rests at least as long.
const MINIM: u32 = Duration::HALF.ticks();

/// A semibreve: the first note of a subject is at least as long, and a new point begins at most
/// this long after the cadence before it.
const SEMIBREVE: u32 = Duration::WHOLE.ticks();

/// The latest second entry of a point: two breves, Vicentino's four rests.
const SECOND_ENTRY: u32 = 2 * Duration::BREVE.ticks();

/// The longest wait of a later entry after the one before it: four breves.
const LATER_ENTRY: u32 = 4 * Duration::BREVE.ticks();

/// The longest silence of a voice that has entered, without a cost: four breves.
const LONG_SILENCE: u32 = 4 * Duration::BREVE.ticks();

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(Entry),
        Box::new(Rest),
        Box::new(Silence),
        Box::new(Held),
        Box::new(Interval),
        Box::new(Time),
        Box::new(Answer),
        Box::new(Subject),
        Box::new(Degree),
        Box::new(Overlap),
    ]
}

/// The plan of the score, if it has one.
fn plan<'a>(ctx: &Ctx<'a>) -> Option<&'a ScoreImitation> {
    ctx.score.imitation.as_deref()
}

/// A note of a head as planned: its place in the head, its attack, its pitch and its value.
#[derive(Clone, Copy, Debug)]
struct Planned {
    index: usize,
    tick: u32,
    pitch: Pitch,
    ticks: u32,
}

/// The notes of an entry's head as planned, one after the other from its start.
fn planned(entry: &ScoreEntry) -> impl Iterator<Item = Planned> + '_ {
    entry
        .notes
        .iter()
        .enumerate()
        .scan(entry.start, |tick, (index, &(pitch, ticks))| {
            let at = *tick;
            *tick = tick.saturating_add(ticks);
            Some(Planned {
                index,
                tick: at,
                pitch,
                ticks,
            })
        })
}

/// Where an entry's head ends as planned.
fn head_end(entry: &ScoreEntry) -> u32 {
    entry
        .notes
        .iter()
        .fold(entry.start, |tick, &(_, ticks)| tick.saturating_add(ticks))
}

/// The diatonic steps from one pitch to the next.
fn steps(from: Pitch, to: Pitch) -> i32 {
    to.diatonic() - from.diatonic()
}

/// Whether the score shows what voice `v` sings at `tick`: its notes are written beyond it, or
/// the score is complete. The search writes the voices up to the current tick and sets the voices
/// of a tick one after the other; a voice whose notes end at the tick is not yet set there.
fn known(ctx: &Ctx, v: usize, tick: u32) -> bool {
    ctx.analysis
        .ends
        .get(v)
        .is_some_and(|&end| end > tick || ctx.complete)
}

/// What voice `v` sounds at `tick`, ties merged; `None` during a rest or after its end.
fn sounding(ctx: &Ctx, v: usize, tick: u32) -> Option<Sounding> {
    let line = &ctx.analysis.lines[v];
    let i = line.partition_point(|s| s.end <= tick);
    line.get(i).filter(|s| s.start <= tick).copied()
}

/// The note of voice `v` sounding at `tick` with its attack there, if there is one.
fn attack(ctx: &Ctx, v: usize, tick: u32) -> Option<Sounding> {
    sounding(ctx, v, tick).filter(|s| s.start == tick)
}

/// The note to mark in voice `v` at `tick`: the first note of what sounds there, otherwise the
/// rest written there, otherwise the voice's last note.
fn mark(ctx: &Ctx, v: usize, tick: u32) -> usize {
    if let Some(s) = sounding(ctx, v, tick) {
        return s.note;
    }
    let voice = &ctx.score.voices[v];
    voice
        .timed()
        .position(|(start, note)| start <= tick && tick < start + note.duration.ticks())
        .unwrap_or(voice.notes.len().saturating_sub(1))
}

/// Records a case of a rule, its parameter the detail.
fn report(
    ctx: &Ctx,
    out: &mut Vec<Violation>,
    info: &'static RuleInfo,
    param: &'static str,
    (v, note): (usize, usize),
    tick: u32,
) {
    ctx.report(
        out,
        info.id,
        ctx.settings.level(param),
        v,
        note,
        tick,
        Detail::Param(param),
    );
}

/// The entries of a point whose voices the score has and whose heads have notes, with their
/// places in the point.
fn entries<'p>(
    ctx: &Ctx,
    point: &'p ScorePoint,
) -> impl Iterator<Item = (usize, &'p ScoreEntry)> + use<'p> {
    let voices = ctx.analysis.voice_count();
    point
        .entries
        .iter()
        .enumerate()
        .filter(move |(_, e)| e.voice < voices && !e.notes.is_empty())
}

/// The entries of a point after its first, as [`entries`] gives them.
fn followers<'p>(
    ctx: &Ctx,
    point: &'p ScorePoint,
) -> impl Iterator<Item = (usize, &'p ScoreEntry)> + use<'p> {
    entries(ctx, point).filter(|&(j, _)| j > 0)
}

/// The leader of a point (its first entry), if the score has its voice and its head notes.
fn leader<'a>(ctx: &Ctx, point: &'a ScorePoint) -> Option<&'a ScoreEntry> {
    point
        .entries
        .first()
        .filter(|e| e.voice < ctx.analysis.voice_count() && !e.notes.is_empty())
}

struct Entry;

static ENTRY: RuleInfo = RuleInfo {
    id: "IMI-ENTRY",
    title_de: "Einsatz weicht vom geplanten Kopf ab",
    title_en: "An entry departs from its planned head",
    source: "Zarlino 1558, III.51 (die Fuge folgt mit denselben Intervallen); Jeppesen, Counterpoint (1939), S. 160, Regel 8 (die Textunterlegung des ersten Einsatzes bleibt in allen folgenden); Vicentino 1555, IV.29 („la medesima pronuntia“); PAL-REP-3 (Phase 12); Korpus Palestrina: die Werte der Töne 2–4 weichen in 22 % der imitierenden Einsätze vom ersten ab; Silben vorläufig",
    params: &[
        level(
            "level",
            "ein Kopfton erklingt nicht wie geplant: andere Tonhöhe, Pause, Überbindung oder kein Anschlag zu seiner Zeit",
            "a head note does not sound as planned: another pitch, a rest, a tie or no attack at its tick",
        ),
        level(
            "rhythm",
            "ein Kopfton außer dem ersten ist kürzer als geplant",
            "a head note other than the first is shorter than planned",
        ),
        level(
            "text",
            "ein Anschlag des Kopfes in einer Stimme mit Text trägt eine andere Silbe als der erste Einsatz auf diesem Ton, eine, wo dieser die Silbe hält, oder keine, wo er eine beginnt",
            "a head attack of a singing voice carries another syllable than the leader's on that note, one where the leader holds, or none where the leader begins one",
        ),
    ],
};

/// Whether two attacks sing differently: one begins a syllable where the other holds one, or
/// they begin different syllables of the text. A syllable that is not in the text
/// ([`Lyric::UNKNOWN`]) may be any.
fn sung_apart(a: Option<Lyric>, b: Option<Lyric>) -> bool {
    match (a, b) {
        (None, None) => false,
        (Some(_), None) | (None, Some(_)) => true,
        (Some(a), Some(b)) => match (a.index(), b.index()) {
            (Some(a), Some(b)) => a != b,
            _ => false,
        },
    }
}

impl Rule for Entry {
    fn info(&self) -> &'static RuleInfo {
        &ENTRY
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some(plan) = plan(ctx) else {
            return;
        };
        let sings = |v: usize| super::text::sings(ctx.score, v);
        for point in &plan.points {
            let Some(leader) = leader(ctx, point) else {
                continue;
            };
            // The syllables of followers are compared with the leader's where both sing.
            let leader_notes: Vec<Planned> = if sings(leader.voice) {
                planned(leader).collect()
            } else {
                Vec::new()
            };
            for (j, entry) in entries(ctx, point) {
                let v = entry.voice;
                let mut last = head_end(entry);
                if j > 0 {
                    last = last.max(head_end(leader));
                }
                if last < ctx.window.from || entry.start > ctx.window.until {
                    continue;
                }
                let compare = j > 0 && sings(v) && !leader_notes.is_empty();
                for note in planned(entry) {
                    // The cases of a note are decided from its attack on: before its planned end,
                    // or where the leader's note begins.
                    if note.tick > ctx.window.until {
                        break;
                    }
                    let theirs = leader_notes.get(note.index).filter(|_| compare);
                    let latest = note
                        .tick
                        .saturating_add(note.ticks)
                        .max(theirs.map_or(0, |theirs| theirs.tick));
                    if latest < ctx.window.from {
                        continue;
                    }
                    // The voices are written up to a tick, so a later note is not known either.
                    if !known(ctx, v, note.tick) {
                        break;
                    }
                    let attacked = attack(ctx, v, note.tick);
                    let sounds = attacked.filter(|s| s.pitch == note.pitch);
                    if ctx.window.contains(note.tick) && sounds.is_none() {
                        let marked = mark(ctx, v, note.tick);
                        report(ctx, out, &ENTRY, "level", (v, marked), note.tick);
                    }
                    // The value is known where the note ends, when the voice's next event
                    // begins. A note held longer silences the next head note, which `level`
                    // reports; the head's last note may sound on into the free continuation.
                    if let Some(s) = sounds.filter(|_| note.index > 0)
                        && s.end < note.tick.saturating_add(note.ticks)
                        && (s.end < ctx.analysis.ends[v] || ctx.complete)
                        && ctx.window.contains(s.end)
                    {
                        report(ctx, out, &ENTRY, "rhythm", (v, s.note), s.end);
                    }
                    let Some((sung, theirs)) = attacked.zip(theirs) else {
                        continue;
                    };
                    // Decided once both attacks are known; a follower enters after its leader.
                    let at = note.tick.max(theirs.tick);
                    if !ctx.window.contains(at) || !known(ctx, leader.voice, theirs.tick) {
                        continue;
                    }
                    let Some(model) = attack(ctx, leader.voice, theirs.tick) else {
                        continue;
                    };
                    let lyric = |v: usize, s: Sounding| ctx.score.voices[v].notes[s.note].lyric;
                    if sung_apart(lyric(v, sung), lyric(leader.voice, model)) {
                        report(ctx, out, &ENTRY, "text", (v, sung.note), at);
                    }
                }
            }
        }
    }
}

struct Rest;

static REST: RuleInfo = RuleInfo {
    id: "IMI-REST",
    title_de: "Einsatz ohne Pause oder zwischen den halben Schlägen",
    title_en: "Entry without a rest, or off the minim beats",
    source: "Pontio 1595, S. 62–65 („si fermi … per spatio d'vn tempo di breue, ò di semibreue, ò almeno d'vna pausa di Minima“); Vicentino 1555, IV.15, fol. 79v (keine Minima vor den Pausen in lateinischer Kirchenmusik); Fux 1725, S. 247; Jeppesen, Counterpoint (1939), S. 269 (im Anhang über die Vokalfuge); Korpus Palestrina: Wiedereinsätze nach weniger als einer Halben 24 von 35 683 (0,07 %), erste Einsätze zwischen den halben Schlägen 0 von 5 377, vor 4,1 % der Pausen eine Halbe",
    params: &[
        level(
            "level",
            "Wiedereinsatz ohne Pause oder nach einer Pause kürzer als eine Halbe",
            "a re-entry without a rest, or after a rest shorter than a minim",
        ),
        level(
            "offbeat",
            "Einsatz zwischen den halben Schlägen",
            "an entry off the minim beats",
        ),
        level(
            "minim_before",
            "der Ton vor der Pause vor einem Wiedereinsatz ist eine Halbe oder kürzer",
            "the note before the rest before a re-entry is a minim or shorter",
        ),
    ],
};

impl Rule for Rest {
    fn info(&self) -> &'static RuleInfo {
        &REST
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some(plan) = plan(ctx) else {
            return;
        };
        for point in &plan.points {
            for (_, entry) in entries(ctx, point) {
                let (v, start) = (entry.voice, entry.start);
                if !ctx.window.contains(start) || !known(ctx, v, start) {
                    continue;
                }
                if !start.is_multiple_of(MINIM) {
                    report(ctx, out, &REST, "offbeat", (v, mark(ctx, v, start)), start);
                }
                // The silence before the entry's attack, if the voice has sung before it.
                let line = &ctx.analysis.lines[v];
                let i = line.partition_point(|s| s.end <= start);
                let Some(first) = line.get(i).filter(|s| s.start == start) else {
                    continue;
                };
                let Some(before) = i.checked_sub(1).map(|k| line[k]) else {
                    continue;
                };
                if start - before.end < MINIM {
                    report(ctx, out, &REST, "level", (v, first.note), start);
                } else if before.end - before.start <= MINIM {
                    report(ctx, out, &REST, "minim_before", (v, before.note), start);
                }
            }
        }
    }
}

struct Silence;

static SILENCE: RuleInfo = RuleInfo {
    id: "IMI-SILENCE",
    title_de: "Eingesetzte Stimmen schweigen zu lange oder zu viele zugleich",
    title_en: "Voices that have entered are silent too long, or too many at once",
    source: "Fux 1725, S. 161 („nunquam pausam ponendam esse, nisi immediate subjectum sequatur“); Vicentino 1555, IV.29, fol. 86v (die zuerst endende Stimme beginnt zuerst wieder); Korpus Palestrina, drei- bis sechsstimmige Abschnitte: Stimmen, die eingesetzt haben, schweigen vor dem letzten ersten Einsatz an 1,6–3,7 % der Taktanfänge (Mittel etwa 2,4 %), Pausen von vier Breven und mehr sind 0–3,2 % der Pausen ab einer Halben (vierstimmig 1,1 %) und 0–2,5 % der Zeit der Stimmen; nach dem letzten ersten Einsatz eines Abschnitts klingt vierstimmig an 10,7 %, sechsstimmig an 9,9 % der Taktanfänge genau die Hälfte der Stimmen; ohne Plan (`unplanned_thin`, Phase 14) nach dem ersten Ton jeder Stimme eines Satzes vierstimmig an 11,4 %, sechsstimmig an 10,1 % der Taktanfänge",
    params: &[
        level(
            "exposition",
            "eine Stimme, die eingesetzt hat, schweigt auf einem Taktanfang, bevor die letzte Stimme eingesetzt hat",
            "a voice that has entered is silent on a downbeat before the last voice has entered",
        ),
        level(
            "long",
            "eine Stimme schweigt länger als vier Breven, je Taktanfang darüber hinaus",
            "a voice is silent for more than four breves, per downbeat beyond them",
        ),
        level(
            "thin",
            "nach dem letzten ersten Einsatz klingt auf einem Taktanfang genau die Hälfte der Stimmen (vier- und sechsstimmig)",
            "after the last first entry exactly half of the voices sound on a downbeat (four and six voices)",
        ),
        level(
            "unplanned_thin",
            "dasselbe in einem Satz ohne Imitationsplan, nach dem ersten Ton jeder Stimme (Teile einer Messe ohne Einsätze)",
            "the same in a setting without a plan of imitation, after every voice's first note (parts of a mass without entries)",
        ),
    ],
};

impl Rule for Silence {
    fn info(&self) -> &'static RuleInfo {
        &SILENCE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some(plan) = plan(ctx).filter(|plan| !plan.points.is_empty()) else {
            // Without entries only half of the choir counts, once every voice has sung.
            if ctx.settings.level("unplanned_thin") != Level::Off
                && let Some(entered) = first_notes(ctx)
            {
                half_choir(ctx, out, "unplanned_thin", entered);
            }
            return;
        };
        let voices = ctx.analysis.voice_count();
        let per = ctx.analysis.per_measure;
        // Where each voice enters first, and the last of these entries.
        let first = |v: usize| {
            plan.points
                .iter()
                .flat_map(|point| &point.entries)
                .filter(|e| e.voice == v)
                .map(|e| e.start)
                .min()
        };
        let firsts: Vec<Option<u32>> = (0..voices).map(first).collect();
        let last_first = firsts.iter().flatten().max().copied();
        for (v, entered) in firsts.iter().enumerate() {
            let Some(entered) = *entered else {
                continue;
            };
            let line = &ctx.analysis.lines[v];
            // Each downbeat of the window after the entry, decided once the voice's note there
            // is known.
            let end = ctx.analysis.ends.get(v).copied().unwrap_or(0);
            let mut tick = ctx.window.from.max(entered + 1).div_ceil(per) * per;
            while tick <= ctx.window.until && tick < end {
                if sounding(ctx, v, tick).is_none() {
                    let marked = (v, mark(ctx, v, tick));
                    if last_first.is_some_and(|last| tick < last) {
                        report(ctx, out, &SILENCE, "exposition", marked, tick);
                    }
                    // How long the voice has been silent: since its last note ended.
                    let i = line.partition_point(|s| s.end <= tick);
                    let since = i.checked_sub(1).map(|k| line[k].end);
                    if since.is_some_and(|end| tick - end >= LONG_SILENCE) {
                        report(ctx, out, &SILENCE, "long", marked, tick);
                    }
                }
                tick += per;
            }
        }
        if let Some(last_first) = last_first {
            half_choir(ctx, out, "thin", last_first);
        }
    }
}

/// The latest of the voices' first notes: from there on every voice has sung, which ends the
/// exposition of a score without a plan. `None` while a voice has not sung, in a prefix of the
/// search too, whose voices show their first notes once they are written up to them. A tacet
/// voice ([`Ctx::tacet`]) does not count.
fn first_notes(ctx: &Ctx) -> Option<u32> {
    ctx.analysis
        .lines
        .iter()
        .enumerate()
        .filter(|&(v, _)| !ctx.tacet(v))
        .map(|(_, line)| line.first().map(|s| s.start))
        .collect::<Option<Vec<u32>>>()?
        .into_iter()
        .max()
}

/// IMI-SILENCE `thin` and `unplanned_thin`: half of a choir of four or six resting on a downbeat
/// from `entered` on, the end of the exposition, before the last measure; fewer than half is
/// FREE-TEXTURE's. The choir is the voices that are not tacet ([`Ctx::tacet`]): a section of a
/// mass in three of its movement's four voices has no half choir. Decided once every voice is
/// set on the downbeat, attributed to the highest resting voice.
fn half_choir(ctx: &Ctx, out: &mut Vec<Violation>, param: &'static str, entered: u32) {
    let choir: Vec<usize> = (0..ctx.analysis.voice_count())
        .filter(|&v| !ctx.tacet(v))
        .collect();
    let voices = choir.len();
    if voices < 4 || !voices.is_multiple_of(2) {
        return;
    }
    for i in ctx.downbeats() {
        let slice = &ctx.analysis.slices[i];
        let tick = slice.tick;
        if tick < entered || ctx.in_last_measure(tick) || !ctx.analysis.complete_at(tick) {
            continue;
        }
        let sounding = choir
            .iter()
            .filter(|&&v| slice.sounding[v].is_some())
            .count();
        if 2 * sounding != voices {
            continue;
        }
        if let Some(&v) = choir.iter().find(|&&v| slice.sounding[v].is_none()) {
            report(ctx, out, &SILENCE, param, (v, mark(ctx, v, tick)), tick);
        }
    }
}

struct Held;

static HELD: RuleInfo = RuleInfo {
    id: "IMI-HELD",
    title_de: "Dissonanz gegen einen übergebundenen Ton, der kein Vorhalt ist",
    title_en: "Dissonance against a note tied over the barline that is no suspension",
    source: "Wie FREE-PASSING und FREE-RESOLUTION (docs/regeln.md § 16), die diese Fälle nicht beurteilen; Korpus Palestrina, mit einem Plan ohne Einsätze gezählt: auf dem Taktanfang gegen einen übergebundenen Ton angeschlagen, der nicht stufenweise abwärts geht, 12 von 40 381 dissonanten Anschlägen gegen einen übergebundenen Ton (0,03 %), danach weder Durchgang noch Figur 63 von 14 650 (0,43 %); ohne Plan (`unplanned_struck`, `unplanned_passing`, Phase 14) dieselben Fälle",
    params: &[
        level(
            "struck",
            "auf dem Taktanfang gegen einen übergebundenen Ton angeschlagen, der nicht wie ein Vorhalt stufenweise abwärts geht",
            "struck on the downbeat against a note tied over it that does not go down by step as a suspension does",
        ),
        level(
            "passing",
            "nach dem Taktanfang gegen einen übergebundenen Ton, der kein Vorhalt ist, weder Durchgang noch eine Figur des freien Satzes",
            "after the downbeat against a note tied over it that is no suspension, neither a passing note nor a figure of free counterpoint",
        ),
        level(
            "unplanned_struck",
            "wie `struck` in einem Satz ohne Imitationsplan (Teile einer Messe ohne Einsätze)",
            "as `struck` in a setting without a plan of imitation (parts of a mass without entries)",
        ),
        level(
            "unplanned_passing",
            "wie `passing` in einem Satz ohne Imitationsplan (Teile einer Messe ohne Einsätze)",
            "as `passing` in a setting without a plan of imitation (parts of a mass without entries)",
        ),
    ],
};

/// Whether the note `held` of voice `u`, tied over the downbeat `tick`, is dissonant there with
/// a note struck on it: then it is the suspension, if it goes down by step.
fn suspended_at(ctx: &Ctx, u: usize, held: Sounding, tick: u32) -> bool {
    (0..ctx.analysis.voice_count())
        .filter(|&w| w != u)
        .any(|w| clash_with(ctx, u, held, w, tick).is_some_and(|(other, _)| other.start == tick))
}

impl Rule for Held {
    fn info(&self) -> &'static RuleInfo {
        &HELD
    }

    /// The general rules excuse a note from its clash with one tied over the downbeat of its
    /// measure, as if that note were the suspension, and find a suspension only in the first
    /// clash of a tied note, never behind a fourth above the bass. Long head notes tied over the
    /// barline make those cases frequent in imitation, so a plan has them judged here: a note
    /// struck on the downbeat, dissonant with a tied note, needs that note to go down by step as
    /// a suspension; a note struck later against a tied note that is no suspension (consonant on
    /// the downbeat, or not going down) answers for its clash as FREE-PASSING has every other
    /// note do. Each case is decided where both are known: the tied note's way on, the figure.
    /// A score without points, such as a part of a mass that begins together, has the same cases
    /// judged at the levels of `unplanned_struck` and `unplanned_passing`.
    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let (struck_param, passing_param) = if plan(ctx).is_some_and(|plan| !plan.points.is_empty())
        {
            ("struck", "passing")
        } else {
            ("unplanned_struck", "unplanned_passing")
        };
        if ctx.settings.level(struck_param) == Level::Off
            && ctx.settings.level(passing_param) == Level::Off
        {
            return;
        }
        let per = ctx.analysis.per_measure;
        let voices = ctx.analysis.voice_count();
        // A case is decided within two measures of its note's attack: the tied note's way on by
        // the next downbeat, the figure of a note by the notes after it.
        let back = ctx.window.from.saturating_sub(2 * per);
        for v in 0..voices {
            if species(ctx.score, v) != Some(Species::Free) {
                continue;
            }
            let line = &ctx.analysis.lines[v];
            let first = line.partition_point(|s| s.start < back);
            for (i, &note) in line.iter().enumerate().skip(first) {
                if note.start > ctx.window.until {
                    break;
                }
                // Eighths are FREE-EIGHTHS'; a note struck again on the downbeat is prepared.
                let downbeat = note.start / per * per;
                let struck = note.start == downbeat;
                if note.end - note.start <= per / 8 || (struck && repeated(ctx, v, note)) {
                    continue;
                }
                // Only a note of another voice begun before the downbeat can be held: the test
                // `held.start >= downbeat` below, made before the clash and the bass are computed
                // (clash_with takes `held` from this slice).
                let Some(slice) = ctx.analysis.slice_at(note.start) else {
                    continue;
                };
                for u in (0..voices).filter(|&u| u != v) {
                    if slice
                        .sounding
                        .get(u)
                        .copied()
                        .flatten()
                        .is_none_or(|other| other.start >= downbeat)
                    {
                        continue;
                    }
                    let Some((held, clash)) = clash_with(ctx, v, note, u, note.start) else {
                        continue;
                    };
                    if held.start >= downbeat {
                        continue;
                    }
                    let case = if struck {
                        // FREE-RESOLUTION judges the suspensions it finds.
                        if judged_suspension(ctx, u, held, downbeat) {
                            continue;
                        }
                        match steps_down(ctx, u, held, downbeat) {
                            Some((false, at)) => Some((struck_param, at)),
                            _ => None,
                        }
                    } else {
                        // A suspension answers for what moves against it before it resolves.
                        let resolution = if suspended_at(ctx, u, held, downbeat) {
                            match steps_down(ctx, u, held, downbeat) {
                                Some((true, _)) | None => continue,
                                Some((false, at)) => at,
                            }
                        } else {
                            0
                        };
                        let held_bass = clash == Clash::FourthAboveBass;
                        match passing_case(ctx, v, line, i, true, held_bass) {
                            Some(("level", tick)) => Some((passing_param, tick.max(resolution))),
                            _ => None,
                        }
                    };
                    if let Some((name, at)) = case
                        && ctx.window.contains(at)
                    {
                        report(ctx, out, &HELD, name, (v, note.note), at);
                    }
                }
            }
        }
    }
}

struct Interval;

static INTERVAL: RuleInfo = RuleInfo {
    id: "IMI-INTERVAL",
    title_de: "Einsatz nicht im Einklang, in der Oktave, Quarte oder Quinte zum ersten Einsatz",
    title_en: "Entry not at the unison, octave, fourth or fifth of the leader",
    source: "Zarlino 1558, III.51, S. 212 (die Fuge im Einklang, in der Quarte, Quinte oder Oktave) und III.52, S. 217 (die Imitation in den übrigen Intervallen); Morley 1597, S. 155 (in Terz und Sexte „seldome vsed“); Jeppesen, Counterpoint (1939), S. 167; Korpus Palestrina: zweite Einsätze in der Terz oder Sexte etwa 5 % (meist Terzen und Dezimen, echte Sexten 0,8 %), in der Sekunde oder Septime 0,1 % der imitierenden",
    params: &[
        level(
            "third_sixth",
            "in der Terz oder Sexte (auch Dezime)",
            "at a third or a sixth (a tenth too)",
        ),
        level(
            "second_seventh",
            "in der Sekunde oder Septime",
            "at a second or a seventh",
        ),
    ],
};

impl Rule for Interval {
    fn info(&self) -> &'static RuleInfo {
        &INTERVAL
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some(plan) = plan(ctx) else {
            return;
        };
        for point in &plan.points {
            let Some(leader) = leader(ctx, point) else {
                continue;
            };
            let first = leader.notes[0].0;
            for (_, entry) in followers(ctx, point) {
                let (v, start) = (entry.voice, entry.start);
                if !ctx.window.contains(start) || !known(ctx, v, start) {
                    continue;
                }
                let name = match steps(first, entry.notes[0].0).rem_euclid(7) {
                    0 | 3 | 4 => continue,
                    2 | 5 => "third_sixth",
                    _ => "second_seventh",
                };
                report(ctx, out, &INTERVAL, name, (v, mark(ctx, v, start)), start);
            }
        }
    }
}

struct Time;

static TIME: RuleInfo = RuleInfo {
    id: "IMI-TIME",
    title_de: "Einsatz zu spät oder ohne Engführung",
    title_en: "Entry too late or without stretto",
    source: "Vicentino 1555, IV.14, fol. 78v („quattro pause sono quasi troppo“) und IV.32; Pontio 1595, S. 48 (nicht mehr als zwei Breven); Morris 1922, S. 47 (die Antwort setzt vor dem Ende des Themas ein); Tigrini 1588, IV.2, S. 104 (im Kanon mit mehr Stimmen auch acht oder zehn Pausen); Korpus Palestrina: der zweite Einsatz in 735 von 737 Eröffnungen höchstens 32 Ticks nach dem ersten (zweimal 40 Ticks: 0,27 %), erst nach dem Ende des Kopfes in 3,9 %; spätere Abstände über 64 Ticks 2,8 %",
    params: &[
        level(
            "second",
            "zweiter Einsatz mehr als zwei Breven (32 Ticks) nach dem ersten",
            "the second entry more than two breves (32 ticks) after the first",
        ),
        level(
            "apart",
            "zweiter Einsatz erst nach dem Ende des Kopfes des ersten",
            "the second entry only after the leader's head has ended",
        ),
        level(
            "later",
            "ein weiterer Einsatz mehr als vier Breven (64 Ticks) nach dem vorigen",
            "a further entry more than four breves (64 ticks) after the one before",
        ),
    ],
};

impl Rule for Time {
    fn info(&self) -> &'static RuleInfo {
        &TIME
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some(plan) = plan(ctx) else {
            return;
        };
        for point in &plan.points {
            let Some(leader) = leader(ctx, point) else {
                continue;
            };
            let leader_end = head_end(leader);
            for (j, entry) in followers(ctx, point) {
                let (v, start) = (entry.voice, entry.start);
                if !ctx.window.contains(start) || !known(ctx, v, start) {
                    continue;
                }
                let gap = start.saturating_sub(point.entries[j - 1].start);
                let marked = (v, mark(ctx, v, start));
                if j == 1 {
                    if gap > SECOND_ENTRY {
                        report(ctx, out, &TIME, "second", marked, start);
                    }
                    if start >= leader_end {
                        report(ctx, out, &TIME, "apart", marked, start);
                    }
                } else if gap > LATER_ENTRY {
                    report(ctx, out, &TIME, "later", marked, start);
                }
            }
        }
    }
}

struct Answer;

static ANSWER: RuleInfo = RuleInfo {
    id: "IMI-ANSWER",
    title_de: "Antwort ändert ein Intervall des Soggetto",
    title_en: "The answer changes an interval of the subject",
    source: "Fux 1725, S. 144 (die modale Antwort: die Quinte wird mit der Quarte beantwortet und umgekehrt); Vicentino 1555, IV.32, fol. 88v–89 (in Fugen in Gegenbewegung; Jeppesen, Counterpoint, S. 22, liest es als erste Regel der tonalen Antwort); Morris 1922, S. 47; Jeppesen, Counterpoint (1939), S. 164 (bei Palestrina ist die reale Antwort die häufigste); Korpus Palestrina: Quarte und Quinte in den ersten beiden Intervallen getauscht in 12,6–20,7 % der Köpfe, wo es möglich ist (zwischen Finalis und Quinte 32 %), eine andere Änderung um eine Stufe in den ersten drei Intervallen etwa 2–5 % (eine obere Grenze: stärker veränderte Antworten erkennt die Zählung nicht)",
    params: &[
        level(
            "modal",
            "Quarte und Quinte in den Intervallen 1–2 getauscht (modale Antwort)",
            "fourth and fifth exchanged in intervals 1–2 (a modal answer)",
        ),
        level(
            "adjusted",
            "eine andere Änderung um eine Stufe in den Intervallen 1–3",
            "another change by one step in intervals 1–3",
        ),
    ],
};

/// The intervals of a head in diatonic steps, from each note to the next.
fn intervals(entry: &ScoreEntry) -> impl Iterator<Item = i32> + '_ {
    entry.notes.windows(2).map(|w| steps(w[0].0, w[1].0))
}

impl Rule for Answer {
    fn info(&self) -> &'static RuleInfo {
        &ANSWER
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some(plan) = plan(ctx) else {
            return;
        };
        for point in &plan.points {
            let Some(leader) = leader(ctx, point) else {
                continue;
            };
            for (_, entry) in followers(ctx, point) {
                let v = entry.voice;
                if head_end(entry) < ctx.window.from || entry.start > ctx.window.until {
                    continue;
                }
                // Intervals 1–3, each decided at the attack of the note it leads to.
                let changed = intervals(leader)
                    .zip(intervals(entry))
                    .zip(planned(entry).skip(1))
                    .take(3)
                    .enumerate();
                for (k, ((theirs, ours), to)) in changed {
                    if theirs == ours {
                        continue;
                    }
                    let exchanged = theirs.signum() == ours.signum()
                        && theirs.abs().min(ours.abs()) == 3
                        && theirs.abs().max(ours.abs()) == 4;
                    let name = if k < 2 && exchanged {
                        "modal"
                    } else if (theirs - ours).abs() == 1 {
                        "adjusted"
                    } else {
                        continue;
                    };
                    if ctx.window.contains(to.tick) && known(ctx, v, to.tick) {
                        let marked = (v, mark(ctx, v, to.tick));
                        report(ctx, out, &ANSWER, name, marked, to.tick);
                    }
                }
            }
        }
    }
}

struct Subject;

static SUBJECT: RuleInfo = RuleInfo {
    id: "IMI-SUBJECT",
    title_de: "Gestalt des Soggetto",
    title_en: "Shape of the subject",
    source: "Vicentino 1555, IV.29, fol. 86v (die Anfänge „col moto tardo“; IV.14 rät dasselbe für den Beginn über einem Cantus firmus); Jeppesen, Counterpoint (1939), S. 136–138 (vom Langsamen zum Schnellen) und S. 167–168 (nicht zu glatt); Fux 1725, S. 244 („subjecta … naturalia, facilia, non tamen sterilia“); Morris 1922, S. 46 (im Umfang des Modus); Korpus Palestrina, Köpfe der ersten Einsätze: erster Ton kürzer als eine Ganze 3,6 %, kürzer als einer der Töne 2–4 11,6 %; Umfang eine Septime oder Oktave 14 %, mehr nie; Breve zu Halbe oder Viertel 1,4 %, Ganze zu Viertel 3,6 % der Übergänge; letzter Ton schwarz oder nicht auf dem Taktanfang 12–21 %; ohne Sprung und Wertwechsel, zwei große Sprünge vorläufig",
    params: &[
        level(
            "short_first",
            "erster Einsatz des ersten Punkts: erster Ton kürzer als eine Ganze",
            "the first point's leader: its first note shorter than a whole",
        ),
        level(
            "not_longest",
            "erster Einsatz des ersten Punkts: erster Ton kürzer als einer der Töne 2–4 (nicht nach einer unbetonten ersten Silbe)",
            "the first point's leader: its first note shorter than one of notes 2–4 (not after an unaccented first syllable)",
        ),
        level(
            "drop",
            "ein Wert fällt auf ein Viertel des vorigen oder weniger (nicht nach einem punktierten)",
            "a value falls to a quarter of the one before or less (not after a dotted one)",
        ),
        level(
            "range",
            "Umfang des Kopfes eine Septime oder Oktave",
            "the head spans a seventh or an octave",
        ),
        level(
            "beyond_octave",
            "Umfang eines Kopfes über eine Oktave, auch einer Antwort",
            "a head spans more than an octave, an answer's too",
        ),
        level(
            "plain",
            "kein Sprung ab der Terz und kein Wertwechsel in den Intervallen 1–5",
            "no leap of a third or more and no change of value in intervals 1–5",
        ),
        level(
            "leaps",
            "ein zweiter Sprung um eine Quarte oder mehr",
            "a second leap of a fourth or more",
        ),
        level(
            "end",
            "letzter Ton des Kopfes schwarz oder nicht auf dem Taktanfang",
            "the head's last note black or off the downbeat",
        ),
    ],
};

/// Whether a value is dotted: three halves of a plain value (6, 12, 24 ticks).
fn dotted(ticks: u32) -> bool {
    ticks.is_multiple_of(3) && (ticks / 3).is_power_of_two()
}

/// Whether a point's first syllable is unaccented, as in Ho-sán-na and Et in ter-ra.
fn opens_unaccented(ctx: &Ctx, point: &ScorePoint) -> bool {
    ctx.score.text.as_ref().is_some_and(|sung| {
        sung.text
            .syllables
            .get(usize::from(point.words.start))
            .is_some_and(|syllable| !syllable.flags.accented())
    })
}

/// The note of a head at which its range widens beyond an octave, if it does.
fn beyond_octave(head: &[Planned]) -> Option<Planned> {
    let first = head.first()?.pitch.diatonic();
    let (mut low, mut high) = (first, first);
    head.iter().copied().find(|note| {
        low = low.min(note.pitch.diatonic());
        high = high.max(note.pitch.diatonic());
        high - low > 7
    })
}

/// The cases of a leader's head as (parameter, the tick decided at, the tick of the note to
/// mark), in the leader's plan; `first` for the leader of the first point.
fn shape(
    ctx: &Ctx,
    point: &ScorePoint,
    head: &[Planned],
    first: bool,
) -> Vec<(&'static str, u32, u32)> {
    let mut cases = Vec::new();
    let opening = head[0];
    if first {
        if opening.ticks < SEMIBREVE {
            let next = opening.tick.saturating_add(opening.ticks);
            cases.push(("short_first", next, opening.tick));
        }
        if let Some(longer) = head[1..].iter().take(3).find(|n| n.ticks > opening.ticks)
            && !opens_unaccented(ctx, point)
        {
            cases.push(("not_longest", longer.tick, longer.tick));
        }
    }
    for w in head.windows(2) {
        if 4 * w[1].ticks <= w[0].ticks && !dotted(w[0].ticks) {
            cases.push(("drop", w[1].tick, w[1].tick));
        }
    }
    // The range at each note, decided where it widens to a seventh or beyond an octave.
    let (mut low, mut high) = (opening.pitch.diatonic(), opening.pitch.diatonic());
    for note in &head[1..] {
        low = low.min(note.pitch.diatonic());
        high = high.max(note.pitch.diatonic());
        if (6..=7).contains(&(high - low)) {
            cases.push(("range", note.tick, note.tick));
            break;
        }
    }
    if let Some(note) = beyond_octave(head) {
        cases.push(("beyond_octave", note.tick, note.tick));
    }
    if let Some(six) = head.get(..6)
        && six
            .windows(2)
            .all(|w| steps(w[0].pitch, w[1].pitch).abs() <= 1)
        && six.iter().all(|n| n.ticks == opening.ticks)
    {
        cases.push(("plain", six[5].tick, opening.tick));
    }
    if let Some(second) = head
        .windows(2)
        .filter(|w| steps(w[0].pitch, w[1].pitch).abs() >= 3)
        .nth(1)
    {
        cases.push(("leaps", second[1].tick, second[1].tick));
    }
    // Off the downbeat at its attack, black where it ends.
    let last = head[head.len() - 1];
    if !last.tick.is_multiple_of(ctx.analysis.per_measure) {
        cases.push(("end", last.tick, last.tick));
    } else if last.ticks < MINIM {
        cases.push(("end", last.tick.saturating_add(last.ticks), last.tick));
    }
    cases
}

impl Rule for Subject {
    fn info(&self) -> &'static RuleInfo {
        &SUBJECT
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some(plan) = plan(ctx) else {
            return;
        };
        for (p, point) in plan.points.iter().enumerate() {
            let Some(leader) = leader(ctx, point) else {
                continue;
            };
            // An answer may widen the head too: a modal one shifts the notes after its changed
            // interval by a step.
            for (_, entry) in followers(ctx, point) {
                let v = entry.voice;
                if head_end(entry) < ctx.window.from || entry.start > ctx.window.until {
                    continue;
                }
                let head: Vec<Planned> = planned(entry).collect();
                if let Some(note) = beyond_octave(&head)
                    && ctx.window.contains(note.tick)
                    && known(ctx, v, note.tick)
                {
                    let marked = (v, mark(ctx, v, note.tick));
                    report(ctx, out, &SUBJECT, "beyond_octave", marked, note.tick);
                }
            }
            let v = leader.voice;
            if head_end(leader) < ctx.window.from || leader.start > ctx.window.until {
                continue;
            }
            let head: Vec<Planned> = planned(leader).collect();
            for (name, at, marked) in shape(ctx, point, &head, p == 0) {
                if ctx.window.contains(at) && known(ctx, v, marked) {
                    report(ctx, out, &SUBJECT, name, (v, mark(ctx, v, marked)), at);
                }
            }
        }
    }
}

struct Degree;

static DEGREE: RuleInfo = RuleInfo {
    id: "IMI-DEGREE",
    title_de: "Späterer Punkt beginnt nicht auf Finalis oder Quinte",
    title_en: "A later point begins off the final and the fifth",
    source: "Zarlino 1558, IV.18, S. 320–321 (die Anfänge auf den Grenztönen von Quinte und Quarte des Modus); Pontio 1595, S. 49; Morris 1922, S. 46–47; Korpus Palestrina: erste Einsätze auf der Quarte 12,2 %, auf jeder anderen Stufe 3,7–8,9 %; im Phrygischen gilt A als Quinte, B wie die Quarte (Jeppesen, Counterpoint, S. 170, Anm. 36, nennt B die Dominante, im Korpus ist A häufiger). Den ersten Punkt beurteilt FREE-ENTRY",
    params: &[
        level(
            "fourth",
            "auf der Quarte (im Phrygischen auf der Quinte B)",
            "on the fourth (in the Phrygian mode on the fifth, B)",
        ),
        level("level", "auf einer anderen Stufe", "on another degree"),
    ],
};

impl Rule for Degree {
    fn info(&self) -> &'static RuleInfo {
        &DEGREE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some(plan) = plan(ctx) else {
            return;
        };
        let mode = ctx.score.mode;
        let phrygian = mode.mode == Mode::Phrygian;
        for point in plan.points.iter().skip(1) {
            let Some(leader) = leader(ctx, point) else {
                continue;
            };
            let (v, start) = (leader.voice, leader.start);
            if !ctx.window.contains(start) || !known(ctx, v, start) {
                continue;
            }
            let name = match (mode.degree(leader.notes[0].0), phrygian) {
                (1, _) | (5, false) | (4, true) => continue,
                (4, false) | (5, true) => "fourth",
                _ => "level",
            };
            report(ctx, out, &DEGREE, name, (v, mark(ctx, v, start)), start);
        }
    }
}

struct Overlap;

static OVERLAP: RuleInfo = RuleInfo {
    id: "IMI-OVERLAP",
    title_de: "Neuer Punkt setzt spät nach dem Binnenschluss oder allein ein",
    title_en: "A new point enters late after the cadence, or alone",
    source: "Fux 1725, S. 247 („aut sub ipsam modulationem, aut in Clausula, rariús post Clausulam factam“); Vicentino 1555, IV.15, fol. 79 (nicht warten, bis eine Stimme geendet hat); Morley 1597, S. 158 (ein Einsatz, während alle anderen pausieren, klingt „bare and lame“); Korpus Palestrina: neue Punkte beginnen auf dem Ziel einer Klausel oder eine Ganze danach 1,7-mal so oft wie zufällig, an 0,1 % der Taktanfänge klingt keine Stimme (FREE-TEXTURE); spät vorläufig",
    params: &[
        level(
            "late",
            "erster Einsatz mehr als eine Ganze nach dem Ziel des Binnenschlusses davor",
            "the first entry more than a whole note after the goal of the cadence before it",
        ),
        level(
            "bare",
            "keine andere Stimme klingt beim ersten Einsatz",
            "no other voice sounds at the first entry",
        ),
    ],
};

impl Rule for Overlap {
    fn info(&self) -> &'static RuleInfo {
        &OVERLAP
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some(plan) = plan(ctx) else {
            return;
        };
        let voices = ctx.analysis.voice_count();
        for pair in plan.points.windows(2) {
            let Some(leader) = leader(ctx, &pair[1]) else {
                continue;
            };
            let (v, start) = (leader.voice, leader.start);
            if !ctx.window.contains(start) || !known(ctx, v, start) {
                continue;
            }
            let marked = (v, mark(ctx, v, start));
            let goal = pair[0].goal.map(|measure| {
                u32::try_from(measure)
                    .unwrap_or(u32::MAX)
                    .saturating_mul(ctx.analysis.per_measure)
            });
            if goal.is_some_and(|goal| start > goal.saturating_add(SEMIBREVE)) {
                report(ctx, out, &OVERLAP, "late", marked, start);
            }
            // Once every voice is set at the entry.
            if (0..voices).all(|u| known(ctx, u, start))
                && (0..voices).all(|u| u == v || sounding(ctx, u, start).is_none())
            {
                report(ctx, out, &OVERLAP, "bare", marked, start);
            }
        }
    }
}
