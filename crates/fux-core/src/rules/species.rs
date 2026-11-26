//! Rules of the second to fifth species (G2-* to G5-*).
//!
//! A rule that needs a later note to decide reports at that note's start, so the search counts
//! each violation once, when it becomes certain; the violation still marks the offending note.

use super::analysis::{Motion, Transition, harmonic, species};
use super::dissonance::{Clash, clash_at, dissonance_at, is_dissonant, repeated};
use super::melodic::{int, level};
use super::{Ctx, Detail, Rule, RuleInfo, Violation};
use crate::interval::Interval;
use crate::lang::Lang;
use crate::rhythm::{Place, measure_rhythm, patterns};
use crate::score::{Note, Role, Sounding, Species};
use crate::time::TICKS_PER_WHOLE;

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(Rhythm {
            info: &G2_RHYTHM,
            species: Species::Second,
        }),
        Box::new(Passing2),
        Box::new(DownbeatMotion),
        Box::new(Rhythm {
            info: &G3_RHYTHM,
            species: Species::Third,
        }),
        Box::new(Run),
        Box::new(ThirdQuarter),
        Box::new(Passing3),
        Box::new(Cambiata),
        Box::new(AccentParallels),
        Box::new(AccentedPassing),
        Box::new(AccentLeap),
        Box::new(Triad3),
        Box::new(Rhythm {
            info: &G4_RHYTHM,
            species: Species::Fourth,
        }),
        Box::new(Arsis4),
        Box::new(Resolution),
        Box::new(Suspensions),
        Box::new(Octaves4),
        Box::new(Break),
        Box::new(Rhythm {
            info: &G5_RHYTHM,
            species: Species::Fifth,
        }),
        Box::new(Passing5 {
            info: &PASSING5,
            species: Species::Fifth,
        }),
        Box::new(Eighths {
            info: &EIGHTHS,
            species: Species::Fifth,
        }),
        Box::new(Resolution5 {
            info: &RESOLUTION5,
            species: Species::Fifth,
        }),
        Box::new(Variety5),
        Box::new(Rhythm {
            info: &FREE_RHYTHM,
            species: Species::Free,
        }),
        Box::new(Passing5 {
            info: &FREE_PASSING,
            species: Species::Free,
        }),
        Box::new(Eighths {
            info: &FREE_EIGHTHS,
            species: Species::Free,
        }),
        Box::new(Resolution5 {
            info: &FREE_RESOLUTION,
            species: Species::Free,
        }),
    ]
}

/// The parameter for the voice count: `level` up to three voices, `four_voices` from four on.
fn many_name(ctx: &Ctx) -> &'static str {
    if ctx.analysis.voice_count() >= 4 {
        "four_voices"
    } else {
        "level"
    }
}

/// The voices in one of the given species.
fn voices_of(ctx: &Ctx, wanted: &[Species]) -> Vec<usize> {
    (0..ctx.analysis.voice_count())
        .filter(|&v| species(ctx.score, v).is_some_and(|s| wanted.contains(&s)))
        .collect()
}

/// The note after `i` in the line if it follows without a rest.
fn next(line: &[Sounding], i: usize) -> Option<Sounding> {
    line.get(i + 1).copied().filter(|n| n.start == line[i].end)
}

/// The note before `i` in the line if it leads into it without a rest.
fn previous(line: &[Sounding], i: usize) -> Option<Sounding> {
    i.checked_sub(1)
        .map(|j| line[j])
        .filter(|p| p.end == line[i].start)
}

/// Whether nothing more can follow note `i`: the score is complete or a later note exists.
fn settled(ctx: &Ctx, line: &[Sounding], i: usize) -> bool {
    ctx.complete || i + 1 < line.len()
}

fn step(a: Sounding, b: Sounding) -> Interval {
    Interval::between(a.pitch, b.pitch)
}

/// Reached and left by step in the same direction.
fn is_passing(previous: Sounding, note: Sounding, next: Sounding) -> bool {
    let (into, out) = (step(previous, note), step(note, next));
    into.is_step() && out.is_step() && into.direction() == out.direction()
}

/// The parameter `free` for a voice in free counterpoint, `level` otherwise.
fn free_or_level(ctx: &Ctx, voice: usize) -> &'static str {
    if species(ctx.score, voice) == Some(Species::Free) {
        "free"
    } else {
        "level"
    }
}

/// Whether a note begins on a beat of the half-note tactus: on the first or third quarter in 2/2.
fn on_beat(note: Sounding) -> bool {
    note.start.is_multiple_of(TICKS_PER_WHOLE / 2)
}

/// Whether a note is a quarter on a beat: an accented quarter.
fn accented_quarter(note: Sounding) -> bool {
    note.end - note.start == TICKS_PER_WHOLE / 4 && on_beat(note)
}

struct Rhythm {
    info: &'static RuleInfo,
    species: Species,
}

static G2_RHYTHM: RuleInfo = RuleInfo {
    id: "G2-RHYTHM",
    title_de: "Rhythmus der 2. Gattung: zwei Halbe je Takt, am Anfang eine halbe Pause, am Schluss eine Ganze",
    title_en: "Second species rhythm: two half notes per measure, a half rest at the start, a whole note at the end",
    source: "Fux; Korpus (11 von 12 zweistimmigen Beispielen mit halber Pause); Mann 87 f.; L24 2.M2",
    params: &[
        level("level", "anderer Rhythmus", "other rhythm"),
        level(
            "no_rest",
            "erster Takt ohne halbe Pause",
            "first measure without a half rest",
        ),
        level(
            "cadence_tie",
            "ab drei Stimmen: Bindung oder Ganze im Schluss",
            "three or more voices: tie or whole note in the cadence",
        ),
        level(
            "late_entry",
            "Einsatz erst nach dem ersten Takt",
            "entry after the first measure",
        ),
    ],
};

static G3_RHYTHM: RuleInfo = RuleInfo {
    id: "G3-RHYTHM",
    title_de: "Rhythmus der 3. Gattung: vier Viertel je Takt, am Schluss eine Ganze",
    title_en: "Third species rhythm: four quarter notes per measure, a whole note at the end",
    source: "Fux; Korpus (Viertelpause am Anfang nur Fig. 132, Ganze im vorletzten Takt nur Fig. 181)",
    params: &[
        level("level", "anderer Rhythmus", "other rhythm"),
        level(
            "rest_two_voices",
            "zweistimmig: Viertelpause am Anfang",
            "two voices: quarter rest at the start",
        ),
        level(
            "rest",
            "ab drei Stimmen: Viertelpause am Anfang",
            "three or more voices: quarter rest at the start",
        ),
        level(
            "cadence_whole",
            "ab drei Stimmen: Ganze im vorletzten Takt",
            "three or more voices: whole note in the penultimate measure",
        ),
        level(
            "late_entry",
            "Einsatz erst nach dem ersten Takt",
            "entry after the first measure",
        ),
    ],
};

static G4_RHYTHM: RuleInfo = RuleInfo {
    id: "G4-RHYTHM",
    title_de: "Rhythmus der 4. Gattung: halbe Pause, dann über den Taktstrich gebundene Halbe, am Schluss eine Ganze",
    title_en: "Fourth species rhythm: a half rest, then half notes tied over the barline, a whole note at the end",
    source: "Fux; Korpus (später Einsatz nur Fig. 144; Pause statt Synkope Fig. 151); Mann 98; L24 4.P6; W23",
    params: &[
        level("level", "anderer Rhythmus", "other rhythm"),
        level(
            "no_rest",
            "erster Takt ohne halbe Pause",
            "first measure without a half rest",
        ),
        level(
            "rest",
            "ab drei Stimmen: halbe Pause statt Synkope",
            "three or more voices: half rest instead of a syncopation",
        ),
        level(
            "late_entry",
            "Einsatz erst nach dem ersten Takt",
            "entry after the first measure",
        ),
    ],
};

static G5_RHYTHM: RuleInfo = RuleInfo {
    id: "G5-RHYTHM",
    title_de: "Rhythmus der 5. Gattung: Taktmuster wie bei Fux, Beginn mit halber Pause und Synkope, vorletzter Takt Synkope und Auflösung",
    title_en: "Fifth species rhythm: measure patterns as in Fux, a half rest and a syncopation at the start, syncopation and resolution in the penultimate measure",
    source: "Fux; Korpus (Taktmuster nach Häufigkeit, docs/regeln.md § 11); W23 5.R6",
    params: &[
        level(
            "level",
            "bei Fux nicht belegter Rhythmus",
            "rhythm Fux does not use",
        ),
        level(
            "occasional",
            "bei Fux 3- bis 9-mal belegt",
            "used 3 to 9 times by Fux",
        ),
        level(
            "rare",
            "bei Fux 1- oder 2-mal belegt",
            "used once or twice by Fux",
        ),
        level(
            "entry_untied",
            "erster Takt ohne Bindung",
            "first measure without a tie",
        ),
        level(
            "late_entry",
            "Einsatz erst nach dem ersten Takt",
            "entry after the first measure",
        ),
    ],
};

static FREE_RHYTHM: RuleInfo = RuleInfo {
    id: "FREE-RHYTHM",
    title_de: "Rhythmus des freien Satzes: Taktmuster wie bei Palestrina",
    title_en: "Free counterpoint rhythm: measure patterns as in Palestrina",
    source: "Korpus Palestrina (Taktmuster nach Häufigkeit in 519 764 Takten, docs/regeln.md § 16)",
    params: &[
        level(
            "level",
            "bei Palestrina nicht belegter Rhythmus",
            "rhythm Palestrina does not use",
        ),
        level(
            "frequent",
            "bei Palestrina in 1–5 % der Takte",
            "used by Palestrina in 1–5 % of the measures",
        ),
        level(
            "uncommon",
            "bei Palestrina in 0,1–1 % der Takte",
            "used by Palestrina in 0.1–1 % of the measures",
        ),
        level(
            "occasional",
            "bei Palestrina in 0,01–0,1 % der Takte",
            "used by Palestrina in 0.01–0.1 % of the measures",
        ),
        level(
            "rare",
            "bei Palestrina in 0,001–0,01 % der Takte",
            "used by Palestrina in 0.001–0.01 % of the measures",
        ),
        level(
            "very_rare",
            "bei Palestrina in weniger als 0,001 % der Takte",
            "used by Palestrina in less than 0.001 % of the measures",
        ),
    ],
};

impl Rule for Rhythm {
    fn info(&self) -> &'static RuleInfo {
        self.info
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let per = ctx.analysis.per_measure;
        let measures = ctx.analysis.total.div_ceil(per) as usize;
        let many = ctx.analysis.voice_count() >= 3;
        // A measure is decided at the start of the next one (the last at its last note).
        let from = (ctx.window.from / per).saturating_sub(1) as usize;
        let goals: Vec<usize> = ctx
            .analysis
            .inner_cadences()
            .map(|c| (c.goal / per) as usize)
            .collect();
        for v in voices_of(ctx, &[self.species]) {
            let voice = &ctx.score.voices[v];
            // Per measure from `from` on: index of its first note, start of its last note, its
            // notes. Notes do not cross barlines, so every measure begins with a note.
            let mut by_measure: Vec<(usize, u32, Vec<&Note>)> =
                vec![(0, 0, Vec::new()); measures.saturating_sub(from)];
            // The notes before measure `from` count for nothing: begin at the last sounding note
            // that starts before it (its index and start from the analysis).
            let line = &ctx.analysis.lines[v];
            let before = line.partition_point(|s| s.start < from as u32 * per);
            let (skip, mut tick) = match before.checked_sub(1) {
                Some(q) => (line[q].note, line[q].start),
                None => (0, 0),
            };
            for (n, note) in voice.notes.iter().enumerate().skip(skip) {
                let start = tick;
                tick += note.duration.ticks();
                let m = (start / per) as usize;
                if let Some(m) = m.checked_sub(from).and_then(|i| by_measure.get_mut(i)) {
                    if m.2.is_empty() {
                        m.0 = n;
                    }
                    m.1 = start;
                    m.2.push(note);
                }
            }
            // Before the first note every measure is silent, also while the note is not yet set.
            let entry = voice
                .timed()
                .find(|(_, n)| !n.is_rest())
                .map_or(Some(usize::MAX), |(start, _)| Some((start / per) as usize));
            for (i, (first, last_start, notes)) in by_measure.iter().enumerate() {
                let m = from + i;
                if notes.is_empty() {
                    continue;
                }
                // The rhythm is known once the next measure has begun: that decides a tie. A free
                // score has no length given in advance: its last measure is known at the end.
                let decided = match by_measure.get(i + 1) {
                    Some((_, _, next)) if !next.is_empty() => (m as u32 + 1) * per,
                    None if self.species != Species::Free || ctx.complete => *last_start,
                    _ => continue,
                };
                let place = Place::of(m, measures, entry, &goals);
                let tied_in = *first > 0 && voice.notes[first - 1].tie;
                let found = measure_rhythm(notes, tied_in);
                let (level, detail) = match patterns(self.species, place, many)
                    .iter()
                    .find(|(pattern, _)| *pattern == found)
                {
                    Some((_, None)) => continue,
                    Some((pattern, Some(name))) => {
                        (ctx.settings.level(name), Detail::Rhythm(pattern, name))
                    }
                    None => (ctx.settings.level("level"), Detail::None),
                };
                ctx.report(out, self.info.id, level, v, *first, decided, detail);
            }
        }
    }

    fn message(&self, violation: &Violation, lang: Lang) -> String {
        match violation.detail {
            Detail::Rhythm(pattern, name) => {
                let why = self
                    .info
                    .param(name)
                    .map_or(name, |p| lang.pick(p.de, p.en));
                match (lang, self.species) {
                    (Lang::En, Species::Free) => {
                        format!("Free counterpoint, measure rhythm {pattern}: {why}")
                    }
                    (Lang::De, species) => {
                        format!("{}, Taktrhythmus {pattern}: {why}", species.name(lang))
                    }
                    (Lang::En, species) => format!(
                        "Species {}, measure rhythm {pattern}: {why}",
                        species.number()
                    ),
                }
            }
            _ => self.info.title(lang).to_string(),
        }
    }
}

struct Passing2;

static PASSING2: RuleInfo = RuleInfo {
    id: "G2-PASSING",
    title_de: "2. Gattung: Dissonanz auf der Arsis ohne Durchgang",
    title_en: "Second species: dissonance on the upbeat that is not a passing tone",
    source: "Ch. 64; W23 2.H2",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for Passing2 {
    fn info(&self) -> &'static RuleInfo {
        &PASSING2
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let level = ctx.settings.level("level");
        for v in voices_of(ctx, &[Species::Second]) {
            let line = &ctx.analysis.lines[v];
            // Decided by the next note at the latest.
            let first = ctx.first_in_window(line).saturating_sub(1);
            for (i, note) in line.iter().enumerate().skip(first) {
                if ctx.analysis.is_downbeat(note.start) || !is_dissonant(ctx, v, *note) {
                    continue;
                }
                let decided = match next(line, i) {
                    Some(n) => n.start,
                    None if settled(ctx, line, i) => note.start,
                    None => continue,
                };
                let passing = matches!(
                    (previous(line, i), next(line, i)),
                    (Some(p), Some(n)) if is_passing(p, *note, n)
                );
                if !passing {
                    ctx.report(
                        out,
                        PASSING2.id,
                        level,
                        v,
                        note.note,
                        decided,
                        Detail::Pitch(note.pitch),
                    );
                }
            }
        }
    }
}

/// For a second-species voice: the downbeat note whose motion counts at the next downbeat
/// instead of the upbeat note `arsis`, if the voice moves from it to the upbeat by a third at
/// most (Ch. 65–67; W23 2.P1).
pub(super) fn g2_reference(ctx: &Ctx, voice: usize, arsis: Sounding) -> Option<Sounding> {
    if species(ctx.score, voice) != Some(Species::Second) || ctx.analysis.is_downbeat(arsis.start) {
        return None;
    }
    let line = &ctx.analysis.lines[voice];
    let i = line.partition_point(|s| s.start < arsis.start);
    if line.get(i) != Some(&arsis) {
        return None;
    }
    let thesis = previous(line, i)?;
    (ctx.analysis.is_downbeat(thesis.start) && step(thesis, arsis).abs().steps <= 2)
        .then_some(thesis)
}

struct DownbeatMotion;

static DOWNBEAT_MOTION: RuleInfo = RuleInfo {
    id: "G2-DOWNBEAT-MOTION",
    title_de: "2. Gattung: vollkommene Konsonanz in gerader Bewegung von Thesis zu Thesis erreicht (ein Terzsprung verdeckt sie nicht)",
    title_en: "Second species: perfect consonance reached in similar motion from downbeat to downbeat (a skip of a third does not hide it)",
    source: "Ch. 65–67; W23 2.P1; Mann 86 (ab drei Stimmen darf der Terzsprung Quinten verdecken); Korpus",
    params: &[
        level("level", "zweistimmig", "two voices"),
        level(
            "parallel_many",
            "ab drei Stimmen: parallele Quinten oder Oktaven, Arsis im Schritt",
            "three or more voices: parallel fifths or octaves, upbeat by step",
        ),
        level(
            "third_skip_many",
            "ab drei Stimmen: parallele Quinten oder Oktaven, Arsis im Terzsprung",
            "three or more voices: parallel fifths or octaves, upbeat by a skip of a third",
        ),
    ],
};

impl Rule for DownbeatMotion {
    fn info(&self) -> &'static RuleInfo {
        &DOWNBEAT_MOTION
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let n = ctx.analysis.voice_count();
        if voices_of(ctx, &[Species::Second]).is_empty() {
            return;
        }
        for i in ctx.downbeats() {
            for a in 0..n {
                for b in a + 1..n {
                    let Some(t) = ctx.analysis.transition(i, a, b) else {
                        continue;
                    };
                    // Replace an upbeat note by the downbeat before it where that one counts. The
                    // plain motion from the upbeat is judged by MOT-DIRECT-PERFECT.
                    let a1 = g2_reference(ctx, a, t.a1).unwrap_or(t.a1);
                    let b1 = g2_reference(ctx, b, t.b1).unwrap_or(t.b1);
                    if a1 == t.a1 && b1 == t.b1 {
                        continue;
                    }
                    let judged = Transition { a1, b1, ..t };
                    if judged.motion() != Motion::Similar || !judged.after().is_perfect_consonance()
                    {
                        continue;
                    }
                    let name = if n == 2 {
                        "level"
                    } else {
                        let (before, after) = (judged.before(), judged.after());
                        if !(before.is_perfect_consonance()
                            && before.simple().steps == after.simple().steps)
                        {
                            continue; // hidden: TEX-DIRECT-PERFECT judges the real motion
                        }
                        let skip = [(a1, t.a1), (b1, t.b1)]
                            .iter()
                            .any(|(thesis, arsis)| step(*thesis, *arsis).abs().steps == 2);
                        if skip {
                            "third_skip_many"
                        } else {
                            "parallel_many"
                        }
                    };
                    ctx.report_pair(
                        out,
                        DOWNBEAT_MOTION.id,
                        ctx.settings.level(name),
                        (a, t.a2.note),
                        (b, t.b2.note),
                        t.tick,
                        Detail::Interval(judged.after()),
                    );
                }
            }
        }
    }
}

struct Run;

static RUN: RuleInfo = RuleInfo {
    id: "G3-RUN",
    title_de: "3. Gattung: Laufen fünf Viertel stufenweise in eine Richtung, ist das 3. Viertel konsonant",
    title_en: "Third species: in five quarters running stepwise in one direction the third is consonant",
    source: "Mann 50; Ch. 73; lat. 63; W23 3.H1; Korpus (vierstimmig im Bass verletzt: Fig. 186)",
    params: &[
        level("level", "zwei- und dreistimmig", "two and three voices"),
        level("four_voices", "ab vier Stimmen", "four or more voices"),
    ],
};

impl Rule for Run {
    fn info(&self) -> &'static RuleInfo {
        &RUN
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let per = ctx.analysis.per_measure;
        for v in voices_of(ctx, &[Species::Third]) {
            let line = &ctx.analysis.lines[v];
            let first = ctx.first_in_window(line).saturating_sub(4);
            for group in line.windows(5).skip(first) {
                if group[0].start % per != 0 || group.windows(2).any(|w| w[0].end != w[1].start) {
                    continue;
                }
                let steps: Vec<Interval> = group.windows(2).map(|w| step(w[0], w[1])).collect();
                let one_way = steps
                    .iter()
                    .all(|s| s.is_step() && s.direction() == steps[0].direction());
                let third = group[2];
                if one_way && is_dissonant(ctx, v, third) {
                    ctx.report(
                        out,
                        RUN.id,
                        ctx.settings.level(many_name(ctx)),
                        v,
                        third.note,
                        group[4].start,
                        Detail::Pitch(third.pitch),
                    );
                }
            }
        }
    }
}

struct ThirdQuarter;

static THIRD_QUARTER: RuleInfo = RuleInfo {
    id: "G3-THIRD-QUARTER",
    title_de: "3. Gattung: dissonantes 3. Viertel ohne konsonantes 2. und 4. Viertel",
    title_en: "Third species: dissonant third quarter without consonant second and fourth quarters",
    source: "Ch. 73 f.; W23 3.H2; Korpus (vierstimmig verletzt: Fig. 186, 204)",
    params: &[
        level("level", "zwei- und dreistimmig", "two and three voices"),
        level("four_voices", "ab vier Stimmen", "four or more voices"),
    ],
};

impl Rule for ThirdQuarter {
    fn info(&self) -> &'static RuleInfo {
        &THIRD_QUARTER
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let per = ctx.analysis.per_measure;
        for v in voices_of(ctx, &[Species::Third]) {
            let line = &ctx.analysis.lines[v];
            let first = ctx.first_in_window(line).saturating_sub(1);
            for (i, note) in line.iter().enumerate().skip(first) {
                if note.start % per != per / 2 || !is_dissonant(ctx, v, *note) {
                    continue;
                }
                // Without both neighbours G3-PASSING judges the note.
                let (Some(second), Some(fourth)) = (previous(line, i), next(line, i)) else {
                    continue;
                };
                if is_dissonant(ctx, v, second) || is_dissonant(ctx, v, fourth) {
                    ctx.report(
                        out,
                        THIRD_QUARTER.id,
                        ctx.settings.level(many_name(ctx)),
                        v,
                        note.note,
                        fourth.start,
                        Detail::Pitch(note.pitch),
                    );
                }
            }
        }
    }
}

/// How a dissonance off the downbeat in the third species is treated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Figure {
    Passing,
    /// Nota cambiata: a dissonant second quarter reached by step down, a third down to a
    /// consonance, then a step up.
    Cambiata,
    /// Reached and left by step in opposite directions: below the notes around it, or above.
    Neighbor {
        lower: bool,
    },
    Other,
}

/// The figure of the dissonant note `line[i]` with the tick at which it is decided; `None` while
/// a note that decides it is still missing.
fn figure(ctx: &Ctx, voice: usize, line: &[Sounding], i: usize) -> Option<(Figure, u32)> {
    let note = line[i];
    let Some(before) = previous(line, i) else {
        return Some((Figure::Other, note.start));
    };
    // Without a next note the figure is no figure: known where the rest after it begins, or at
    // the end.
    let Some(after) = next(line, i) else {
        let rest_follows = ctx.analysis.ends[voice] > note.end;
        return (rest_follows || ctx.complete).then_some((Figure::Other, note.end));
    };
    if is_passing(before, note, after) {
        return Some((Figure::Passing, after.start));
    }
    let (into, out) = (step(before, note), step(note, after));
    let per = ctx.analysis.per_measure;
    // Free counterpoint also has the cambiata on the fourth quarter, over the barline.
    let free = species(ctx.score, voice) == Some(Species::Free);
    let weak_quarter = note.start % per == per / 4 || (free && note.start % per == 3 * per / 4);
    let cambiata_start = weak_quarter && into.steps == -1 && out.steps == -2;
    // A figure that begins like a cambiata is decided by its fourth note.
    let mut decided = after.start;
    if cambiata_start && !is_dissonant(ctx, voice, after) {
        match next(line, i + 1) {
            Some(last) if step(after, last).steps == 1 => {
                return Some((Figure::Cambiata, last.start));
            }
            Some(last) => decided = last.start,
            // A rest after the third note: known where it begins.
            None if ctx.analysis.ends[voice] > after.end || ctx.complete => decided = after.end,
            None => return None,
        }
    }
    if into.is_step() && out.is_step() {
        return Some((
            Figure::Neighbor {
                lower: into.steps < 0,
            },
            after.start,
        ));
    }
    Some((Figure::Other, decided))
}

struct Passing3;

static PASSING3: RuleInfo = RuleInfo {
    id: "G3-PASSING",
    title_de: "3. Gattung: Dissonanz ohne Durchgang",
    title_en: "Third species: dissonance that is not a passing tone",
    source: "Mann 50; Korpus (zweistimmig 59 Durchgänge, 2 Cambiaten, keine Wechselnote; ab drei Stimmen je 1 untere Wechselnote im Bass, Fig. 132, 182, nie eine obere); Trad. (Jeppesen: untere Wechselnote)",
    params: &[
        level("level", "andere Dissonanz", "other dissonance"),
        level(
            "neighbor_two_voices",
            "dissonante untere Wechselnote, zweistimmig",
            "dissonant lower neighbour note, two voices",
        ),
        level(
            "neighbor",
            "dissonante untere Wechselnote, ab drei Stimmen",
            "dissonant lower neighbour note, three or more voices",
        ),
        level(
            "upper_neighbor_two_voices",
            "dissonante obere Wechselnote, zweistimmig",
            "dissonant upper neighbour note, two voices",
        ),
        level(
            "upper_neighbor",
            "dissonante obere Wechselnote, ab drei Stimmen",
            "dissonant upper neighbour note, three or more voices",
        ),
    ],
};

impl Rule for Passing3 {
    fn info(&self) -> &'static RuleInfo {
        &PASSING3
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let two = ctx.analysis.voice_count() == 2;
        for v in voices_of(ctx, &[Species::Third]) {
            let line = &ctx.analysis.lines[v];
            // A figure is decided two notes later at the latest (the cambiata).
            let first = ctx.first_in_window(line).saturating_sub(2);
            for (i, note) in line.iter().enumerate().skip(first) {
                if ctx.analysis.is_downbeat(note.start) || !is_dissonant(ctx, v, *note) {
                    continue;
                }
                let (name, tick) = match figure(ctx, v, line, i) {
                    // A neighbour note on the third quarter is an accented dissonance.
                    Some((Figure::Neighbor { .. }, tick)) if on_beat(*note) => ("level", tick),
                    Some((Figure::Neighbor { lower }, tick)) => match (lower, two) {
                        (true, true) => ("neighbor_two_voices", tick),
                        (true, false) => ("neighbor", tick),
                        (false, true) => ("upper_neighbor_two_voices", tick),
                        (false, false) => ("upper_neighbor", tick),
                    },
                    Some((Figure::Other, tick)) => ("level", tick),
                    _ => continue,
                };
                ctx.report(
                    out,
                    PASSING3.id,
                    ctx.settings.level(name),
                    v,
                    note.note,
                    tick,
                    Detail::Pitch(note.pitch),
                );
            }
        }
    }
}

struct Cambiata;

static CAMBIATA: RuleInfo = RuleInfo {
    id: "G3-CAMBIATA",
    title_de: "3. Gattung: Nota cambiata (dissonantes 2. Viertel, Terz abwärts, dann Schritt aufwärts)",
    title_en: "Third species: nota cambiata (dissonant second quarter, a third down, then a step up)",
    source: "Mann 51; Ch. 74 f.; W23 3.H3; Korpus (Fig. 55, 56)",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for Cambiata {
    fn info(&self) -> &'static RuleInfo {
        &CAMBIATA
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for v in voices_of(ctx, &[Species::Third]) {
            let line = &ctx.analysis.lines[v];
            let first = ctx.first_in_window(line).saturating_sub(2);
            for (i, note) in line.iter().enumerate().skip(first) {
                if ctx.analysis.is_downbeat(note.start) || !is_dissonant(ctx, v, *note) {
                    continue;
                }
                if let Some((Figure::Cambiata, tick)) = figure(ctx, v, line, i) {
                    ctx.report(
                        out,
                        CAMBIATA.id,
                        ctx.settings.level("level"),
                        v,
                        note.note,
                        tick,
                        Detail::Pitch(note.pitch),
                    );
                }
            }
        }
    }
}

struct AccentParallels;

static ACCENT_PARALLELS: RuleInfo = RuleInfo {
    id: "G3-ACCENT-PARALLELS",
    title_de: "Quinten oder Oktaven auf aufeinanderfolgenden Taktanfängen",
    title_en: "Fifths or octaves on consecutive downbeats",
    source: "Trad.; Korpus (zweistimmig: 3. Gattung Fig. 55, 57; 5. Gattung Fig. 85b, 86a)",
    params: &[level(
        "level",
        "Stufe (zweistimmig, 3. und 5. Gattung)",
        "level (two voices, third and fifth species)",
    )],
};

impl Rule for AccentParallels {
    fn info(&self) -> &'static RuleInfo {
        &ACCENT_PARALLELS
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() != 2
            || voices_of(ctx, &[Species::Third, Species::Fifth]).is_empty()
        {
            return;
        }
        let per = ctx.analysis.per_measure;
        for i in ctx.downbeats() {
            let s2 = &ctx.analysis.slices[i];
            let Some(s1) = s2
                .tick
                .checked_sub(per)
                .and_then(|tick| ctx.analysis.slice_at(tick))
                .filter(|s1| s1.tick + per == s2.tick)
            else {
                continue;
            };
            let (Some(a1), Some(a2), Some(b1), Some(b2)) = (
                s1.sounding[0],
                s2.sounding[0],
                s1.sounding[1],
                s2.sounding[1],
            ) else {
                continue;
            };
            let t = Transition {
                a1,
                a2,
                b1,
                b2,
                tick: s2.tick,
            };
            let (before, after) = (t.before(), t.after());
            let same = before.is_perfect_consonance()
                && after.is_perfect_consonance()
                && before.simple().steps == after.simple().steps;
            if same && t.motion() == Motion::Similar {
                ctx.report_pair(
                    out,
                    ACCENT_PARALLELS.id,
                    ctx.settings.level("level"),
                    (0, a2.note),
                    (1, b2.note),
                    s2.tick,
                    Detail::Interval(after),
                );
            }
        }
    }
}

struct AccentedPassing;

static ACCENTED_PASSING: RuleInfo = RuleInfo {
    id: "G3-ACCENTED-PASSING",
    title_de: "Betonter Durchgang aufwärts (dissonantes Viertel auf dem 3. Viertel)",
    title_en: "Accented passing tone upwards (dissonant quarter on the third quarter)",
    source: "Trad. (Jeppesen: betonte Durchgänge nur abwärts); Korpus; Korpus Palestrina (7×)",
    params: &[
        level(
            "level",
            "Stufe (3. und 5. Gattung)",
            "level (third and fifth species)",
        ),
        level("free", "freier Satz", "free counterpoint"),
    ],
};

impl Rule for AccentedPassing {
    fn info(&self) -> &'static RuleInfo {
        &ACCENTED_PASSING
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for v in voices_of(ctx, &[Species::Third, Species::Fifth, Species::Free]) {
            let line = &ctx.analysis.lines[v];
            let first = ctx.first_in_window(line).saturating_sub(1);
            for (i, note) in line.iter().enumerate().skip(first) {
                if !accented_quarter(*note)
                    || ctx.analysis.is_downbeat(note.start)
                    || !is_dissonant(ctx, v, *note)
                {
                    continue;
                }
                let (Some(before), Some(after)) = (previous(line, i), next(line, i)) else {
                    continue;
                };
                if is_passing(before, *note, after) && step(before, *note).steps > 0 {
                    ctx.report(
                        out,
                        ACCENTED_PASSING.id,
                        ctx.settings.level(free_or_level(ctx, v)),
                        v,
                        note.note,
                        after.start,
                        Detail::Pitch(note.pitch),
                    );
                }
            }
        }
    }
}

struct AccentLeap;

static ACCENT_LEAP: RuleInfo = RuleInfo {
    id: "G3-ACCENT-LEAP",
    title_de: "Sprung aufwärts von einem betonten Viertel",
    title_en: "Leap upwards from an accented quarter",
    source: "Trad. (Jeppesen); Korpus; Korpus Palestrina (29×)",
    params: &[
        level(
            "level",
            "Stufe (3. und 5. Gattung)",
            "level (third and fifth species)",
        ),
        level("free", "freier Satz", "free counterpoint"),
    ],
};

impl Rule for AccentLeap {
    fn info(&self) -> &'static RuleInfo {
        &ACCENT_LEAP
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for v in voices_of(ctx, &[Species::Third, Species::Fifth, Species::Free]) {
            let line = &ctx.analysis.lines[v];
            let first = ctx.first_in_window(line).saturating_sub(1);
            for (i, note) in line.iter().enumerate().skip(first) {
                if !accented_quarter(*note) {
                    continue;
                }
                let Some(after) = next(line, i) else {
                    continue;
                };
                let leap = step(*note, after);
                if leap.is_leap() && leap.direction() > 0 {
                    ctx.report(
                        out,
                        ACCENT_LEAP.id,
                        ctx.settings.level(free_or_level(ctx, v)),
                        v,
                        note.note,
                        after.start,
                        Detail::Interval(leap),
                    );
                }
            }
        }
    }
}

struct Triad3;

static TRIAD3: RuleInfo = RuleInfo {
    id: "G3-TRIAD",
    title_de: "3. Gattung ab drei Stimmen: vollständiger Dreiklang weder auf der Thesis noch auf dem 2. oder 3. Viertel",
    title_en: "Third species with three or more voices: no complete triad on the downbeat nor on the second or third quarter",
    source: "Mann 91; L24 3.H6",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for Triad3 {
    fn info(&self) -> &'static RuleInfo {
        &TRIAD3
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        use super::texture::{Kind, kind, sounding, steps_above};
        if ctx.analysis.voice_count() < 3 {
            return;
        }
        let per = ctx.analysis.per_measure;
        let complete_triad = |tick: u32| {
            let Some(slice) = ctx.analysis.slice_at(tick) else {
                return false;
            };
            let Some(bass) = slice.lowest() else {
                return false;
            };
            kind(&steps_above(&sounding(slice), bass)) == Some(Kind::Complete)
        };
        for v in voices_of(ctx, &[Species::Third]) {
            let line = &ctx.analysis.lines[v];
            for note in &line[ctx.first_in_window(line)..] {
                // Decided on the third quarter.
                if note.start % per != per / 2 {
                    continue;
                }
                let downbeat = note.start - per / 2;
                if ctx.analysis.in_first_measure(downbeat)
                    || ctx.analysis.in_last_measure(downbeat)
                    || !ctx.analysis.complete_at(note.start)
                {
                    continue;
                }
                if [downbeat, downbeat + per / 4, note.start]
                    .into_iter()
                    .any(complete_triad)
                {
                    continue;
                }
                ctx.report(
                    out,
                    TRIAD3.id,
                    ctx.settings.level("level"),
                    v,
                    note.note,
                    note.start,
                    Detail::None,
                );
            }
        }
    }
}

struct Arsis4;

static ARSIS4: RuleInfo = RuleInfo {
    id: "G4-ARSIS",
    title_de: "4. Gattung: dissonante Arsis",
    title_en: "Fourth species: dissonant upbeat",
    source: "Ch. 78; W23 4.H1; Korpus (Quarte über dem Bass dreistimmig 1×, Fig. 141)",
    params: &[
        level("level", "Stufe", "level"),
        level(
            "fourth_above_bass",
            "ab drei Stimmen: Quarte über dem Bass (Quartsextklang)",
            "three or more voices: fourth above the bass (6/4 sonority)",
        ),
    ],
};

impl Rule for Arsis4 {
    fn info(&self) -> &'static RuleInfo {
        &ARSIS4
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for v in voices_of(ctx, &[Species::Fourth]) {
            let line = &ctx.analysis.lines[v];
            for note in &line[ctx.first_in_window(line)..] {
                if ctx.analysis.is_downbeat(note.start) {
                    continue;
                }
                if let Some((_, _, clash)) = clash_at(ctx, v, *note, note.start) {
                    let name = match clash {
                        Clash::FourthAboveBass => "fourth_above_bass",
                        Clash::Dissonance => "level",
                    };
                    ctx.report(
                        out,
                        ARSIS4.id,
                        ctx.settings.level(name),
                        v,
                        note.note,
                        note.start,
                        Detail::Pitch(note.pitch),
                    );
                }
            }
        }
    }
}

/// A note held from an upbeat over the next downbeat, where it is dissonant.
#[derive(Clone, Copy, Debug)]
pub(super) struct Suspension {
    voice: usize,
    /// Index in the voice's line.
    index: usize,
    note: Sounding,
    /// The voice and note it is dissonant with.
    against: (usize, Sounding),
    /// The downbeat.
    tick: u32,
}

impl Suspension {
    /// The written note to mark: the tied part on the downbeat, where the dissonance sounds.
    fn marked(&self, ctx: &Ctx) -> usize {
        held_part(ctx, self.voice, self.note, self.tick)
    }
}

/// Index of the written note of a tied sounding that begins at `tick`, or its first note.
pub(super) fn held_part(ctx: &Ctx, voice: usize, note: Sounding, tick: u32) -> usize {
    ctx.score.voices[voice]
        .timed()
        .enumerate()
        .skip(note.note)
        .take_while(|(_, (start, _))| *start <= tick)
        .find(|(_, (start, _))| *start == tick)
        .map_or(note.note, |(index, _)| index)
}

/// The dissonant suspensions of the fourth and fifth species, and the cadence ties of the second
/// species with three or more voices; found once per check, for all the rules that need them.
fn suspensions<'a>(ctx: &'a Ctx<'a>) -> impl Iterator<Item = Suspension> + 'a {
    ctx.shared
        .suspensions
        .get_or_init(|| find_suspensions(ctx))
        .iter()
        .copied()
}

/// Whether the rules of the species judge `note` of voice `voice` as a suspension over the
/// downbeat `tick` (G4-RESOLUTION, G5-ORNAMENTED-RES, FREE-RESOLUTION).
pub(super) fn judged_suspension(ctx: &Ctx, voice: usize, note: Sounding, tick: u32) -> bool {
    suspensions(ctx).any(|s| s.voice == voice && s.note.start == note.start && s.tick == tick)
}

fn find_suspensions(ctx: &Ctx) -> Vec<Suspension> {
    let mut found = Vec::new();
    for v in voices_of(
        ctx,
        &[
            Species::Second,
            Species::Fourth,
            Species::Fifth,
            Species::Free,
        ],
    ) {
        let line = &ctx.analysis.lines[v];
        // Decided within a measure of the note's end (an ornamented resolution on the third
        // quarter after the downbeat); a suspended note may be long.
        let back = ctx.window.from.saturating_sub(ctx.analysis.per_measure);
        let first = line.partition_point(|n| n.end < back);
        for (index, note) in line.iter().enumerate().skip(first) {
            // In free counterpoint a suspension may be struck again on the downbeat.
            if ctx.analysis.is_downbeat(note.start) && repeated(ctx, v, *note) {
                if let Some(against) = dissonance_at(ctx, v, *note, note.start) {
                    found.push(Suspension {
                        voice: v,
                        index,
                        note: *note,
                        against,
                        tick: note.start,
                    });
                }
                continue;
            }
            // In free counterpoint a long note may also become dissonant on a later downbeat
            // it is held over, where the other voices move against it.
            let free = species(ctx.score, v) == Some(Species::Free);
            let mut tick = ctx.analysis.next_downbeat(note.start);
            if !free && ctx.analysis.is_downbeat(note.start) {
                continue;
            }
            while tick < note.end {
                // In those new cases a held bass is suspended only when a voice above strikes a
                // second, seventh or tritone against it (2–3); the fourth over it is the upper
                // voice's own suspension (4–3) or the consonant fourth.
                let extended = ctx.analysis.is_downbeat(note.start)
                    || tick > ctx.analysis.next_downbeat(note.start);
                let bass = extended
                    && ctx
                        .analysis
                        .bass_at(tick)
                        .is_some_and(|(b, s)| b == v || s.pitch == note.pitch);
                let against = dissonance_at(ctx, v, *note, tick).filter(|(_, other)| {
                    !bass
                        || other.start == tick
                            && !Interval::between(note.pitch, other.pitch)
                                .simple()
                                .is_perfect_fourth()
                });
                if let Some(against) = against {
                    found.push(Suspension {
                        voice: v,
                        index,
                        note: *note,
                        against,
                        tick,
                    });
                    break;
                }
                if !free {
                    break;
                }
                tick += ctx.analysis.per_measure;
            }
        }
    }
    found
}

struct Resolution;

static RESOLUTION: RuleInfo = RuleInfo {
    id: "G4-RESOLUTION",
    title_de: "Vorhalt nicht stufenweise abwärts in eine Konsonanz aufgelöst (4. Gattung, Schlussbindung der 2. Gattung)",
    title_en: "Suspension not resolved down by step into a consonance (fourth species, cadence tie of the second)",
    source: "Ch. 78–81; W23 4.P1; Korpus (Auflösung in die Quarte über dem Bass dreistimmig 1×, Fig. 141)",
    params: &[
        level("level", "Stufe", "level"),
        level(
            "fourth_above_bass",
            "ab drei Stimmen: Auflösung in die Quarte über dem Bass",
            "three or more voices: resolution into the fourth above the bass",
        ),
    ],
};

impl Rule for Resolution {
    fn info(&self) -> &'static RuleInfo {
        &RESOLUTION
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for s in suspensions(ctx) {
            if !matches!(
                species(ctx.score, s.voice),
                Some(Species::Fourth | Species::Second)
            ) {
                continue;
            }
            let line = &ctx.analysis.lines[s.voice];
            let (decided, name) = match next(line, s.index) {
                Some(resolution) => {
                    let down_by_step = step(s.note, resolution).steps == -1;
                    let clash = clash_at(ctx, s.voice, resolution, resolution.start);
                    match clash {
                        None if down_by_step => continue,
                        Some((_, _, Clash::FourthAboveBass)) if down_by_step => {
                            (resolution.start, "fourth_above_bass")
                        }
                        _ => (resolution.start, "level"),
                    }
                }
                None if settled(ctx, line, s.index) => (s.tick, "level"),
                None => continue,
            };
            let level = ctx.settings.level(name);
            ctx.report(
                out,
                RESOLUTION.id,
                level,
                s.voice,
                s.marked(ctx),
                decided,
                Detail::Pitch(s.note.pitch),
            );
        }
    }
}

struct Suspensions;

static SUSPENSIONS: RuleInfo = RuleInfo {
    id: "G4-SUSPENSIONS",
    title_de: "Unzulässiger Vorhalt: Septime in der Unterstimme (7–8)",
    title_en: "Suspension not allowed: seventh in the lower voice (7–8)",
    source: "Ch. 78–81; W23 4.H2; freier Satz: Korpus Palestrina",
    params: &[
        level("level", "Stufe", "level"),
        level("free", "freier Satz", "free counterpoint"),
    ],
};

impl Rule for Suspensions {
    fn info(&self) -> &'static RuleInfo {
        &SUSPENSIONS
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for s in suspensions(ctx) {
            let (_, other) = s.against;
            let interval = harmonic(s.note, other);
            if s.note.pitch.midi() < other.pitch.midi() && interval.simple().steps == 6 {
                let name = if species(ctx.score, s.voice) == Some(Species::Free) {
                    "free"
                } else {
                    "level"
                };
                ctx.report(
                    out,
                    SUSPENSIONS.id,
                    ctx.settings.level(name),
                    s.voice,
                    s.marked(ctx),
                    s.tick,
                    Detail::Interval(interval),
                );
            }
        }
    }
}

struct Octaves4;

static OCTAVES4: RuleInfo = RuleInfo {
    id: "G4-OCTAVES",
    title_de: "Sekunde oder None nach Einklang oder Oktave auf der Arsis (versteckte Oktavparallelen)",
    title_en: "Second or ninth after a unison or octave on the upbeat (hidden consecutive octaves)",
    source: "Ch. 79 f.; W23 4.P2",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for Octaves4 {
    fn info(&self) -> &'static RuleInfo {
        &OCTAVES4
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some(cf) = ctx
            .score
            .voices
            .iter()
            .position(|v| v.role == Role::CantusFirmus)
        else {
            return;
        };
        for s in suspensions(ctx) {
            let (u, other) = s.against;
            let above = s.note.pitch.midi() > other.pitch.midi();
            if u != cf || !above || harmonic(s.note, other).simple().steps != 1 {
                continue;
            }
            // The interval with which the suspended note entered on the upbeat.
            let Some(entered) = ctx
                .analysis
                .slice_at(s.note.start)
                .and_then(|slice| slice.sounding[cf])
            else {
                continue;
            };
            let entry = harmonic(s.note, entered);
            if entry.is_perfect_consonance() && entry.simple().steps == 0 {
                ctx.report(
                    out,
                    OCTAVES4.id,
                    ctx.settings.level("level"),
                    s.voice,
                    s.marked(ctx),
                    s.tick,
                    Detail::Interval(entry),
                );
            }
        }
    }
}

struct Break;

static BREAK: RuleInfo = RuleInfo {
    id: "G4-BREAK",
    title_de: "4. Gattung: Bindung gelöst (Takt ohne Synkope)",
    title_en: "Fourth species: syncopation broken (measure without a tie)",
    source: "Fux; W23 4.M1",
    params: &[level("level", "je Takt", "per measure")],
};

impl Rule for Break {
    fn info(&self) -> &'static RuleInfo {
        &BREAK
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let per = ctx.analysis.per_measure;
        for v in voices_of(ctx, &[Species::Fourth]) {
            let line = &ctx.analysis.lines[v];
            for note in &line[ctx.first_in_window(line)..] {
                let inner = note.start >= per && !ctx.analysis.in_last_measure(note.start);
                if inner && ctx.analysis.is_downbeat(note.start) {
                    ctx.report(
                        out,
                        BREAK.id,
                        ctx.settings.level("level"),
                        v,
                        note.note,
                        note.start,
                        Detail::None,
                    );
                }
            }
        }
    }
}

/// Whether a sounding note is an eighth.
fn is_eighth(note: Sounding) -> bool {
    note.end - note.start == TICKS_PER_WHOLE / 8
}

/// The dissonances of a florid voice off the downbeat: the fifth species, or free counterpoint
/// with Palestrina's figures besides.
struct Passing5 {
    info: &'static RuleInfo,
    species: Species,
}

static PASSING5: RuleInfo = RuleInfo {
    id: "G5-PASSING",
    title_de: "5. Gattung: Dissonanz ohne Durchgang",
    title_en: "Fifth species: dissonance that is not a passing tone",
    source: "Fux (jede Note nach den Regeln ihrer Gattung); Korpus (zweistimmig 48 Durchgänge in Vierteln, 1 Cambiata Fig. 87b, 1 untere Wechselnote Fig. 86b, nie eine obere; Halbe und Synkopen nie dissonant angeschlagen)",
    params: &[
        level("level", "andere Dissonanz", "other dissonance"),
        level(
            "cambiata",
            "Nota cambiata in Vierteln",
            "nota cambiata in quarter notes",
        ),
        level(
            "neighbor",
            "dissonante untere Wechselnote in Vierteln",
            "dissonant lower neighbour note in quarter notes",
        ),
        level(
            "upper_neighbor",
            "dissonante obere Wechselnote in Vierteln",
            "dissonant upper neighbour note in quarter notes",
        ),
    ],
};

static FREE_PASSING: RuleInfo = RuleInfo {
    id: "FREE-PASSING",
    title_de: "Freier Satz: Dissonanz ohne Durchgang oder eine von Palestrinas Figuren",
    title_en: "Free counterpoint: dissonance that is neither a passing tone nor one of Palestrina's figures",
    source: "Korpus Palestrina (docs/regeln.md § 16); Jeppesen (konsonante Quarte, Portament)",
    params: &[
        level("level", "andere Dissonanz", "other dissonance"),
        level(
            "cambiata",
            "Nota cambiata in Vierteln (auf dem 2. oder 4. Viertel)",
            "nota cambiata in quarter notes (on the second or fourth quarter)",
        ),
        level(
            "neighbor",
            "dissonante untere Wechselnote in Vierteln",
            "dissonant lower neighbour note in quarter notes",
        ),
        level(
            "upper_neighbor",
            "dissonante obere Wechselnote in Vierteln",
            "dissonant upper neighbour note in quarter notes",
        ),
        level(
            "consonant_fourth",
            "Quarte über dem liegenden Bass, schrittweise erreicht und verlassen (auch als Vorhalt 4–3)",
            "fourth above the held bass, reached and left by step (also as a 4–3 suspension)",
        ),
        level(
            "anticipation",
            "Vorausnahme: der nächste Ton ist derselbe",
            "anticipation: the next note is the same",
        ),
        level(
            "repetition",
            "wiederholter Ton, der dissonant wird und schrittweise abwärts geht",
            "a repeated note that becomes dissonant and goes down by step",
        ),
    ],
};

/// The figures free counterpoint adds to the fifth species, after Palestrina: the consonant
/// fourth (a fourth above a held bass, reached by step or repetition and left by step or held
/// on; `held_bass` says whether the dissonance is one), the anticipation (a short note that the
/// next note repeats) and a repeated note that turns dissonant and goes down by step. With the
/// parameter and the tick it is decided at; `None` for none of them.
fn free_figure(line: &[Sounding], i: usize, held_bass: bool) -> Option<(&'static str, u32)> {
    let note = line[i];
    let before = previous(line, i)?;
    let after = next(line, i)?;
    let (into, out) = (step(before, note), step(note, after));
    if held_bass && into.abs().steps <= 1 && out.abs().steps <= 1 {
        return Some(("consonant_fourth", after.start));
    }
    let short = note.end - note.start <= TICKS_PER_WHOLE / 4;
    if short && out.steps == 0 && out.semitones == 0 && into.abs().steps <= 1 {
        return Some(("anticipation", after.start));
    }
    if into.steps == 0 && into.semitones == 0 && out.steps == -1 {
        return Some(("repetition", after.start));
    }
    None
}

/// How the dissonant note `line[i]` of `voice` off the downbeat stands as a figure of a florid
/// voice (the fifth species, or free counterpoint with Palestrina's figures besides, `free`):
/// the parameter of G5-PASSING or FREE-PASSING that prices it (`level` for none) and the tick it
/// is decided at; `None` for a passing note, or while a note that decides it is missing.
/// `held_bass` says whether the dissonance is a fourth above a bass that sounds from before the
/// note, which free counterpoint may treat as the consonant fourth.
pub(super) fn passing_case(
    ctx: &Ctx,
    voice: usize,
    line: &[Sounding],
    i: usize,
    free: bool,
    held_bass: bool,
) -> Option<(&'static str, u32)> {
    let quarter = TICKS_PER_WHOLE / 4;
    let note = line[i];
    // Reached by a leap or after a rest, the dissonance can be no figure, whatever follows: that
    // is known where it begins.
    if previous(line, i).is_none_or(|b| step(b, note).abs().steps >= 2) {
        return Some(("level", note.start));
    }
    let syncopation = note.end > ctx.analysis.next_downbeat(note.start);
    let is_quarter = note.end - note.start == quarter;
    let (name, tick) = if syncopation {
        // Whether the note is tied over is settled on the downbeat. In free counterpoint a
        // fourth above the held bass may be held over (4–3).
        let downbeat = ctx.analysis.next_downbeat(note.start);
        let fourth =
            free && previous(line, i).is_some_and(|b| step(b, note).abs().steps <= 1) && held_bass;
        (if fourth { "consonant_fourth" } else { "level" }, downbeat)
    } else if let Some(found) = free.then(|| free_figure(line, i, held_bass)).flatten() {
        found
    } else {
        match figure(ctx, voice, line, i) {
            None | Some((Figure::Passing, _)) => return None,
            Some((Figure::Cambiata, tick)) if is_quarter => ("cambiata", tick),
            Some((Figure::Neighbor { lower }, tick)) if is_quarter && !on_beat(note) => {
                (if lower { "neighbor" } else { "upper_neighbor" }, tick)
            }
            Some((_, tick)) => ("level", tick),
        }
    };
    // A note that reaches the barline may still be held over it: that is known on the downbeat.
    let downbeat = ctx.analysis.next_downbeat(note.start);
    let tick = if note.end == downbeat {
        tick.max(downbeat)
    } else {
        tick
    };
    Some((name, tick))
}

/// Whether the note `held` of voice `u`, sounding over the downbeat `tick`, goes down a step as
/// a suspension there does (FREE-RESOLUTION): directly by its next note, or on the third quarter
/// after an ornament that begins downwards, and not held over the next downbeat. With the tick it
/// is decided at; `None` while a note that decides it is missing.
pub(super) fn steps_down(ctx: &Ctx, u: usize, held: Sounding, tick: u32) -> Option<(bool, u32)> {
    let per = ctx.analysis.per_measure;
    if held.end > tick + per {
        return Some((false, tick + per));
    }
    let line = &ctx.analysis.lines[u];
    let index = line.partition_point(|s| s.start < held.start);
    let target = (tick + per / 2).max(held.end);
    let mut end = held.end;
    for n in line.iter().skip(index + 1) {
        if n.start != end {
            // A rest before it, known where the rest begins.
            return Some((false, end));
        }
        if (n.start == held.end || n.start == target) && step(held, *n).steps == -1 {
            return Some((true, n.start));
        }
        // An ornament begins downwards, as all of Fux's do.
        if n.start >= target || n.start == held.end && step(held, *n).steps >= 0 {
            return Some((false, n.start));
        }
        end = n.end;
    }
    let rest_follows = ctx.score.voices[u]
        .timed()
        .any(|(start, n)| start == end && n.is_rest());
    (rest_follows || ctx.complete).then_some((false, end))
}

impl Rule for Passing5 {
    fn info(&self) -> &'static RuleInfo {
        self.info
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let free = self.species == Species::Free;
        for v in voices_of(ctx, &[self.species]) {
            let line = &ctx.analysis.lines[v];
            let first = ctx.first_in_window(line).saturating_sub(2);
            for (i, note) in line.iter().enumerate().skip(first) {
                // Eighths are judged by G5-EIGHTHS, downbeats by HAR-CONSONANCE.
                if is_eighth(*note) || ctx.analysis.is_downbeat(note.start) {
                    continue;
                }
                let Some((_, other, clash)) = clash_at(ctx, v, *note, note.start) else {
                    continue;
                };
                let held_bass = clash == Clash::FourthAboveBass && other.start < note.start;
                if let Some((name, tick)) = passing_case(ctx, v, line, i, free, held_bass) {
                    ctx.report(
                        out,
                        self.info.id,
                        ctx.settings.level(name),
                        v,
                        note.note,
                        tick,
                        Detail::Pitch(note.pitch),
                    );
                }
            }
        }
    }
}

/// Eighth notes of a florid voice: the fifth species or free counterpoint.
struct Eighths {
    info: &'static RuleInfo,
    species: Species,
}

static EIGHTHS: RuleInfo = RuleInfo {
    id: "G5-EIGHTHS",
    title_de: "5. Gattung: Achtel nicht stufenweise erreicht und verlassen",
    title_en: "Fifth species: eighth notes not reached and left by step",
    source: "Mann 63; Korpus (zweistimmig alle Achtel stufenweise; dreistimmig 4× Terzsprung in das Achtelpaar, Fig. 155, 158; dissonant betont nur als Durchgang, unbetont als Durchgang oder untere Wechselnote)",
    params: &[
        level("level", "nicht stufenweise", "not by step"),
        level(
            "dissonance",
            "zweistimmig: dissonant, aber weder Durchgang noch (unbetont) untere Wechselnote",
            "two voices: dissonant, but neither a passing tone nor (off the beat) a lower neighbour",
        ),
        level(
            "turn_two_voices",
            "zweistimmig: Terz abwärts in das Achtelpaar, dann schrittweise zurück (A–F–G–A)",
            "two voices: a third down into the pair of eighths, then back by step (A–F–G–A)",
        ),
        level(
            "turn",
            "ab drei Stimmen: Terz abwärts in das Achtelpaar, dann schrittweise zurück",
            "three or more voices: a third down into the pair of eighths, then back by step",
        ),
    ],
};

static FREE_EIGHTHS: RuleInfo = RuleInfo {
    id: "FREE-EIGHTHS",
    title_de: "Freier Satz: Achtel nicht stufenweise erreicht und verlassen",
    title_en: "Free counterpoint: eighth notes not reached and left by step",
    source: "Korpus Palestrina (docs/regeln.md § 16)",
    params: &[
        level("level", "nicht stufenweise", "not by step"),
        level(
            "dissonance",
            "dissonant, aber weder Durchgang noch (unbetont) untere Wechselnote",
            "dissonant, but neither a passing tone nor (off the beat) a lower neighbour",
        ),
        level(
            "turn_two_voices",
            "zweistimmig: Terz abwärts in das Achtelpaar, dann schrittweise zurück",
            "two voices: a third down into the pair of eighths, then back by step",
        ),
        level(
            "turn",
            "ab drei Stimmen: Terz abwärts in das Achtelpaar, dann schrittweise zurück",
            "three or more voices: a third down into the pair of eighths, then back by step",
        ),
    ],
};

impl Rule for Eighths {
    fn info(&self) -> &'static RuleInfo {
        self.info
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let turn = if ctx.analysis.voice_count() == 2 {
            "turn_two_voices"
        } else {
            "turn"
        };
        for v in voices_of(ctx, &[self.species]) {
            let line = &ctx.analysis.lines[v];
            // Decided two notes later at the latest (the turn).
            let first = ctx.first_in_window(line).saturating_sub(2);
            for (i, note) in line.iter().enumerate().skip(first) {
                if !is_eighth(*note) {
                    continue;
                }
                let before = previous(line, i);
                let Some(after) = next(line, i) else {
                    // Left into a rest (known where the rest begins) or the end.
                    if ctx.analysis.ends[v] > note.end || ctx.complete {
                        ctx.report(
                            out,
                            self.info.id,
                            ctx.settings.level("level"),
                            v,
                            note.note,
                            note.end,
                            Detail::Pitch(note.pitch),
                        );
                    }
                    continue;
                };
                let into = before.map(|b| step(b, *note));
                let out_step = step(*note, after);
                if let Some(into) = into.filter(|s| s.is_step())
                    && out_step.is_step()
                {
                    // On the beat only a passing tone may be dissonant, off the beat also the
                    // lower neighbour.
                    let passing = into.direction() == out_step.direction();
                    let lower = into.steps == -1 && out_step.steps == 1;
                    // In free counterpoint the lower neighbour may also fall on a weak quarter.
                    let beat = if self.species == Species::Free {
                        TICKS_PER_WHOLE / 2
                    } else {
                        TICKS_PER_WHOLE / 4
                    };
                    let on_beat = note.start.is_multiple_of(beat);
                    let allowed = passing || (lower && !on_beat);
                    if !allowed && is_dissonant(ctx, v, *note) {
                        ctx.report(
                            out,
                            self.info.id,
                            ctx.settings.level("dissonance"),
                            v,
                            note.note,
                            after.start,
                            Detail::Pitch(note.pitch),
                        );
                    }
                    continue;
                }
                // A third down into the first eighth, then back up by step to the note before:
                // decided by the note after the pair.
                let turn_begins =
                    into.is_some_and(|s| s.steps == -2) && out_step.steps == 1 && is_eighth(after);
                let (name, decided) = if turn_begins {
                    match next(line, i + 1) {
                        Some(back) if step(after, back).steps == 1 => (turn, back.start),
                        Some(back) => ("level", back.start),
                        // A rest after the pair: known where it begins.
                        None if ctx.analysis.ends[v] > after.end || ctx.complete => {
                            ("level", after.end)
                        }
                        None => continue,
                    }
                } else {
                    ("level", after.start)
                };
                ctx.report(
                    out,
                    self.info.id,
                    ctx.settings.level(name),
                    v,
                    note.note,
                    decided,
                    Detail::Pitch(note.pitch),
                );
            }
        }
    }
}

/// The resolution of a suspension in a florid voice: the fifth species or free counterpoint.
struct Resolution5 {
    info: &'static RuleInfo,
    species: Species,
}

static RESOLUTION5: RuleInfo = RuleInfo {
    id: "G5-ORNAMENTED-RES",
    title_de: "5. Gattung: Vorhalt nicht stufenweise abwärts in eine Konsonanz aufgelöst (auch nicht verziert auf dem 3. Viertel)",
    title_en: "Fifth species: suspension not resolved down by step into a consonance (nor ornamented, on the third quarter)",
    source: "Ch. 85; W23 Gl. 7.13; Korpus (zweistimmig: gehaltene Halbe 20× direkt aufgelöst; gehaltene Viertel 12× direkt, davon 9× als erstes Achtel eines Paars, 7× nach Quintsprung abwärts auf dem 3. Viertel)",
    params: &[level("level", "Stufe", "level")],
};

static FREE_RESOLUTION: RuleInfo = RuleInfo {
    id: "FREE-RESOLUTION",
    title_de: "Freier Satz: Vorhalt nicht stufenweise abwärts in eine Konsonanz aufgelöst (auch nicht verziert auf dem 3. Viertel)",
    title_en: "Free counterpoint: suspension not resolved down by step into a consonance (nor ornamented, on the third quarter)",
    source: "Korpus Palestrina (docs/regeln.md § 16)",
    params: &[
        level("level", "Stufe", "level"),
        level(
            "fourth_above_bass",
            "Auflösung in die Quarte über dem liegenden Bass (die dann 4–3 weitergeht)",
            "resolution into the fourth above the held bass (which goes on 4–3)",
        ),
    ],
};

impl Rule for Resolution5 {
    fn info(&self) -> &'static RuleInfo {
        self.info
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let per = ctx.analysis.per_measure;
        for s in suspensions(ctx) {
            if species(ctx.score, s.voice) != Some(self.species) {
                continue;
            }
            let line = &ctx.analysis.lines[s.voice];
            // Free counterpoint: a dissonance held over the next downbeat is no suspension.
            if self.species == Species::Free && s.note.end > s.tick + per {
                ctx.report(
                    out,
                    self.info.id,
                    ctx.settings.level("level"),
                    s.voice,
                    s.marked(ctx),
                    s.tick + per,
                    Detail::Pitch(s.note.pitch),
                );
                continue;
            }
            let resolves =
                |n: &Sounding| step(s.note, *n).steps == -1 && !is_dissonant(ctx, s.voice, *n);
            // Free counterpoint: into a fourth above the held bass, the consonant fourth.
            let into_fourth = |n: &Sounding| {
                self.species == Species::Free
                    && step(s.note, *n).steps == -1
                    && matches!(
                        clash_at(ctx, s.voice, *n, n.start),
                        Some((_, bass, Clash::FourthAboveBass)) if bass.start < n.start
                    )
            };
            let mut fourth = None;
            // Directly by the next note, or on the third quarter with an ornament between (a
            // quarter or two eighths) after a quarter held over.
            let target = (s.tick + per / 2).max(s.note.end);
            let mut end = s.note.end;
            let mut decided = None;
            let mut resolved = false;
            for n in &line[s.index + 1..] {
                if n.start != end {
                    decided = Some(end); // a rest before the resolution, known where it begins
                    break;
                }
                end = n.end;
                if (n.start == s.note.end || n.start == target) && resolves(n) {
                    resolved = true;
                    break;
                }
                if (n.start == s.note.end || n.start == target) && into_fourth(n) {
                    fourth = Some(n.start);
                    break;
                }
                // An ornament begins downwards, as all of Fux's do.
                if n.start >= target || n.start == s.note.end && step(s.note, *n).steps >= 0 {
                    decided = Some(n.start);
                    break;
                }
            }
            if resolved {
                continue;
            }
            // A rest after the last note, already written.
            let rest_follows = ctx.score.voices[s.voice]
                .timed()
                .any(|(start, n)| start == end && n.is_rest());
            if decided.is_none() && rest_follows {
                decided = Some(end);
            }
            let (name, decided) = match (fourth, decided) {
                (Some(tick), _) => ("fourth_above_bass", tick),
                (None, Some(tick)) => ("level", tick),
                (None, None) if ctx.complete => ("level", s.tick),
                (None, None) => continue,
            };
            ctx.report(
                out,
                self.info.id,
                ctx.settings.level(name),
                s.voice,
                s.marked(ctx),
                decided,
                Detail::Pitch(s.note.pitch),
            );
        }
    }
}

struct Variety5;

static VARIETY5: RuleInfo = RuleInfo {
    id: "G5-VARIETY",
    title_de: "5. Gattung: zu wenig rhythmische Abwechslung",
    title_en: "Fifth species: too little rhythmic variety",
    source: "W23 (Programmcode); Korpus (zweistimmig: Takte ohne Bindung meist 0 oder 1, höchstens 3 in Folge, Fig. 88a; gleicher Rhythmus wie im Vortakt 4 von 88)",
    params: &[
        level(
            "level",
            "je Takt ohne Synkope über max",
            "per measure without a syncopation beyond max",
        ),
        int(
            "max",
            "längste Folge von Takten ohne Bindung",
            "longest run of measures without a tie",
        ),
        level(
            "same_rhythm",
            "gleicher Rhythmus wie im Takt davor",
            "same rhythm as the measure before",
        ),
    ],
};

impl Rule for Variety5 {
    fn info(&self) -> &'static RuleInfo {
        &VARIETY5
    }

    fn message(&self, violation: &Violation, lang: Lang) -> String {
        match violation.detail {
            Detail::Count(n) => match lang {
                Lang::De => format!("5. Gattung: {n} Takte ohne Synkope in Folge"),
                Lang::En => format!("Fifth species: {n} measures without a syncopation in a row"),
            },
            _ => lang
                .pick(
                    "5. Gattung: gleicher Rhythmus wie im Takt davor",
                    "Fifth species: same rhythm as the measure before",
                )
                .to_string(),
        }
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let per = ctx.analysis.per_measure;
        let measures = ctx.analysis.total.div_ceil(per) as usize;
        let max = ctx.settings.int("max");
        for v in voices_of(ctx, &[Species::Fifth]) {
            let voice = &ctx.score.voices[v];
            // Per measure: its first note and its notes; and where the voice ends so far.
            let mut first: Vec<Option<usize>> = vec![None; measures];
            let mut notes: Vec<Vec<&Note>> = vec![Vec::new(); measures];
            let mut end = 0;
            for (n, (start, note)) in voice.timed().enumerate() {
                let m = (start / per) as usize;
                if m >= measures {
                    break;
                }
                first[m].get_or_insert(n);
                notes[m].push(note);
                end = start + note.duration.ticks();
            }
            let tied_in = |m: usize| first[m].is_some_and(|n| n > 0 && voice.notes[n - 1].tie);
            let mut run = 0;
            let mut before: Option<String> = None;
            // Inner measures only; a measure is decided when the next one begins.
            for m in 1..measures.saturating_sub(1) {
                let Some(note) = first[m] else { break };
                let decided = (m as u32 + 1) * per;
                if decided >= end && !ctx.complete {
                    break;
                }
                let tied = tied_in(m) || notes[m].iter().any(|n| n.tie);
                run = if tied { 0 } else { run + 1 };
                if run > max {
                    ctx.report(
                        out,
                        VARIETY5.id,
                        ctx.settings.level("level"),
                        v,
                        note,
                        decided,
                        Detail::Count(run as u32),
                    );
                }
                let rhythm = measure_rhythm(&notes[m], tied_in(m));
                if m + 2 < measures && before.as_ref() == Some(&rhythm) {
                    ctx.report(
                        out,
                        VARIETY5.id,
                        ctx.settings.level("same_rhythm"),
                        v,
                        note,
                        decided,
                        Detail::None,
                    );
                }
                before = Some(rhythm);
            }
        }
    }
}
