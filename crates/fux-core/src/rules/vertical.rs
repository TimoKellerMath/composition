//! Rules about sonorities (HAR-*).

use super::analysis::harmonic;
use super::dissonance::{cadence_tritone, free_between_upper_voices, repeated};
use super::melodic::{int, level};
use super::species::held_part;
use super::{Ctx, Detail, Rule, RuleInfo, Violation};
use crate::score::Sounding;

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(Consonance),
        Box::new(Dissonance),
        Box::new(ChromaticClash),
        Box::new(Unison),
        Box::new(PerfectCost),
        Box::new(ImperfectRun),
    ]
}

/// The voice and note to compare with: the other voice in two parts, the bass in more (once it is
/// certain).
fn reference(ctx: &Ctx, slice: usize, voice: usize) -> Option<(usize, Sounding)> {
    let s = &ctx.analysis.slices[slice];
    if ctx.analysis.voice_count() == 2 {
        let other = 1 - voice;
        return s.sounding[other].map(|n| (other, n));
    }
    let (bass, note) = ctx.analysis.bass_at(s.tick)?;
    (bass != voice).then_some((bass, note))
}

/// Whether a note was tied over from before `tick` (a syncopation, judged by the fourth species
/// rules).
fn held(note: Sounding, tick: u32) -> bool {
    note.start < tick
}

struct Consonance;

static CONSONANCE: RuleInfo = RuleInfo {
    id: "HAR-CONSONANCE",
    title_de: "Dissonanz auf der Thesis",
    title_en: "Dissonance on the downbeat",
    source: "Mann 27, 71; Ch. 53, 64; W23 1.H1, 2.H1",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for Consonance {
    fn info(&self) -> &'static RuleInfo {
        &CONSONANCE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let level = ctx.settings.level("level");
        for i in ctx.downbeats() {
            let slice = &ctx.analysis.slices[i];
            for (v, note) in slice.sounding.iter().enumerate() {
                let Some(note) = *note else { continue };
                let Some((r, other)) = reference(ctx, i, v) else {
                    continue;
                };
                if ctx.analysis.voice_count() == 2 && v > r {
                    continue; // each pair once
                }
                // Suspensions, also struck again in free counterpoint, are judged by the rules of
                // the species.
                if held(note, slice.tick)
                    || held(other, slice.tick)
                    || repeated(ctx, v, note)
                    || repeated(ctx, r, other)
                {
                    continue;
                }
                let interval = harmonic(note, other);
                if !interval.is_consonant() {
                    ctx.report_pair(
                        out,
                        CONSONANCE.id,
                        level,
                        (v, note.note),
                        (r, other.note),
                        slice.tick,
                        Detail::Interval(interval),
                    );
                }
            }
        }
    }
}

struct Dissonance;

static DISSONANCE: RuleInfo = RuleInfo {
    id: "HAR-DISSONANCE",
    title_de: "Dissonanz zwischen Oberstimmen auf der Thesis angeschlagen",
    title_en: "Dissonance struck between upper voices on the downbeat",
    source: "Fux; B26; Korpus (Sekunden und Septimen zwischen Oberstimmen auf der Thesis nur als Vorhalt oder Schlussbindung); Gallon/Bitsch Regel 54",
    params: &[
        level(
            "level",
            "Sekunde, Septime oder anderes dissonantes Intervall",
            "second, seventh or other dissonant interval",
        ),
        level(
            "upper_fourths",
            "Quarte oder verminderte Quinte zwischen Oberstimmen",
            "fourth or diminished fifth between upper voices",
        ),
        level(
            "cadence_tritone",
            "verminderte Quinte zum verzögerten Leitton im Bass vor dem Schluss",
            "diminished fifth against the delayed leading tone in the bass before the end",
        ),
    ],
};

impl Rule for Dissonance {
    fn info(&self) -> &'static RuleInfo {
        &DISSONANCE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() < 3 {
            return;
        }
        for i in ctx.slice_range() {
            let slice = &ctx.analysis.slices[i];
            let Some((bass, b)) = ctx.analysis.bass_at(slice.tick) else {
                continue;
            };
            let downbeat = ctx.analysis.is_downbeat(slice.tick);
            let notes: Vec<(usize, Sounding)> = slice
                .sounding
                .iter()
                .enumerate()
                .filter_map(|(v, s)| s.map(|s| (v, s)))
                .collect();
            for &(v, note) in &notes {
                if v == bass {
                    continue;
                }
                // The tolerated tritone of the delayed leading tone in the bass.
                let interval = harmonic(b, note);
                let attacked = note.start == slice.tick || b.start == slice.tick;
                if attacked && cadence_tritone(ctx, bass, b, interval, slice.tick) {
                    ctx.report_pair(
                        out,
                        DISSONANCE.id,
                        ctx.settings.level("cadence_tritone"),
                        (v, note.note),
                        (bass, b.note),
                        slice.tick,
                        Detail::Interval(interval),
                    );
                }
            }
            if !downbeat {
                continue;
            }
            // Upper voices struck together on the downbeat.
            for (k, &(v, x)) in notes.iter().enumerate() {
                for &(u, y) in &notes[k + 1..] {
                    if v == bass
                        || u == bass
                        || x.start != slice.tick
                        || y.start != slice.tick
                        || repeated(ctx, v, x)
                        || repeated(ctx, u, y)
                    {
                        continue;
                    }
                    let interval = harmonic(x, y);
                    if interval.is_consonant() {
                        continue;
                    }
                    let name = if free_between_upper_voices(interval) {
                        "upper_fourths"
                    } else {
                        "level"
                    };
                    ctx.report_pair(
                        out,
                        DISSONANCE.id,
                        ctx.settings.level(name),
                        (v, x.note),
                        (u, y.note),
                        slice.tick,
                        Detail::Interval(interval),
                    );
                }
            }
        }
    }
}

struct ChromaticClash;

static CHROMATIC_CLASH: RuleInfo = RuleInfo {
    id: "HAR-CHROMATIC-CLASH",
    title_de: "Ton und seine Alteration gleichzeitig (z. B. F gegen F♯)",
    title_en: "A tone and its alteration at the same time (e.g. F against F♯)",
    source: "L25 1.H8; Trad.",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for ChromaticClash {
    fn info(&self) -> &'static RuleInfo {
        &CHROMATIC_CLASH
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for i in ctx.slice_range() {
            let slice = &ctx.analysis.slices[i];
            let n = slice.sounding.len();
            for a in 0..n {
                for b in a + 1..n {
                    let (Some(x), Some(y)) = (slice.sounding[a], slice.sounding[b]) else {
                        continue;
                    };
                    let attacked = x.start == slice.tick || y.start == slice.tick;
                    if attacked && x.pitch.step == y.pitch.step && x.pitch.alter != y.pitch.alter {
                        ctx.report_pair(
                            out,
                            CHROMATIC_CLASH.id,
                            ctx.settings.level("level"),
                            (a, x.note),
                            (b, y.note),
                            slice.tick,
                            Detail::Pitches(x.pitch, y.pitch),
                        );
                    }
                }
            }
        }
    }
}

struct Unison;

static UNISON: RuleInfo = RuleInfo {
    id: "HAR-UNISON",
    title_de: "Einklang auf der Thesis im Satzinneren",
    title_en: "Unison on an inner downbeat",
    source: "Ch. 62; Mann 79, 121; W23 1.H5; L25 1.H5; Gallon/Bitsch Regel 53 (ab fünf Stimmen frei); Korpus; freier Satz: Korpus Palestrina",
    params: &[
        level("level", "zwei- und dreistimmig", "two and three voices"),
        level("four_voices", "vierstimmig", "four voices"),
        level("more_voices", "ab fünf Stimmen", "five or more voices"),
        level(
            "free",
            "freier Satz, zwei- und dreistimmig",
            "free counterpoint, two and three voices",
        ),
    ],
};

impl Rule for Unison {
    fn info(&self) -> &'static RuleInfo {
        &UNISON
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let name = match ctx.analysis.voice_count() {
            0..=3 if ctx.score.is_free() => "free",
            0..=3 => "level",
            4 => "four_voices",
            _ => "more_voices",
        };
        let level = ctx.settings.level(name);
        for i in ctx.downbeats() {
            let slice = &ctx.analysis.slices[i];
            if ctx.analysis.in_first_measure(slice.tick) || ctx.in_last_measure(slice.tick) {
                continue;
            }
            for a in 0..slice.sounding.len() {
                for b in a + 1..slice.sounding.len() {
                    let (Some(x), Some(y)) = (slice.sounding[a], slice.sounding[b]) else {
                        continue;
                    };
                    if x.pitch == y.pitch && !held(x, slice.tick) && !held(y, slice.tick) {
                        ctx.report_pair(
                            out,
                            UNISON.id,
                            level,
                            (a, x.note),
                            (b, y.note),
                            slice.tick,
                            Detail::Pitch(x.pitch),
                        );
                    }
                }
            }
        }
    }
}

struct PerfectCost;

static PERFECT_COST: RuleInfo = RuleInfo {
    id: "HAR-PERFECT-COST",
    title_de: "Vollkommene Konsonanz auf der Thesis",
    title_en: "Perfect consonance on the downbeat",
    source: "Ch. 54; W23 1.H6",
    params: &[level(
        "level",
        "Stufe (nur zweistimmig)",
        "level (two voices only)",
    )],
};

impl Rule for PerfectCost {
    fn info(&self) -> &'static RuleInfo {
        &PERFECT_COST
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() != 2 {
            return;
        }
        let level = ctx.settings.level("level");
        for i in ctx.downbeats() {
            let slice = &ctx.analysis.slices[i];
            if ctx.analysis.in_first_measure(slice.tick) || ctx.in_last_measure(slice.tick) {
                continue;
            }
            let (Some(x), Some(y)) = (slice.sounding[0], slice.sounding[1]) else {
                continue;
            };
            let interval = harmonic(x, y);
            if interval.is_perfect_consonance() {
                ctx.report_pair(
                    out,
                    PERFECT_COST.id,
                    level,
                    (0, held_part(ctx, 0, x, slice.tick)),
                    (1, held_part(ctx, 1, y, slice.tick)),
                    slice.tick,
                    Detail::Interval(interval),
                );
            }
        }
    }
}

struct ImperfectRun;

static IMPERFECT_RUN: RuleInfo = RuleInfo {
    id: "HAR-IMPERFECT-RUN",
    title_de: "Zu viele Terzen oder Sexten in Parallelbewegung",
    title_en: "Too many thirds or sixths in parallel motion",
    source: "Trad.; Korpus (zweistimmig höchstens 3, drei- und vierstimmig bis 6)",
    params: &[
        level(
            "two_voices",
            "zweistimmig, über max_two_voices",
            "two voices, beyond max_two_voices",
        ),
        int(
            "max_two_voices",
            "längste Folge zweistimmig",
            "longest run with two voices",
        ),
        level(
            "more",
            "ab drei Stimmen, über max_more",
            "three or more voices, beyond max_more",
        ),
        int(
            "max_more",
            "längste Folge ab drei Stimmen",
            "longest run with three or more voices",
        ),
    ],
};

impl Rule for ImperfectRun {
    fn info(&self) -> &'static RuleInfo {
        &IMPERFECT_RUN
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let two = ctx.analysis.voice_count() == 2;
        let (level, max) = if two {
            (
                ctx.settings.level("two_voices"),
                ctx.settings.int("max_two_voices"),
            )
        } else {
            (ctx.settings.level("more"), ctx.settings.int("max_more"))
        };
        let n = ctx.analysis.voice_count();
        let range = ctx.slice_range();
        let continues = |t: &super::analysis::Transition| {
            let (before, after) = (t.before().simple().steps, t.after().simple().steps);
            t.motion() == super::analysis::Motion::Similar
                && before == after
                && (after == 2 || after == 5)
        };
        for a in 0..n {
            for b in a + 1..n {
                // The run that reaches into the window, counted backwards from its start.
                let first = range.start.max(1);
                let mut earlier = 0;
                for i in (1..first).rev() {
                    match ctx.analysis.transition(i, a, b) {
                        Some(t) if continues(&t) => earlier += 1,
                        Some(t) if t.motion() == super::analysis::Motion::Static => {}
                        _ => break,
                    }
                }
                let mut run: i64 = if earlier == 0 { 0 } else { earlier + 1 };
                for i in first..range.end {
                    let Some(t) = ctx.analysis.transition(i, a, b) else {
                        run = 0;
                        continue;
                    };
                    if continues(&t) {
                        run = if run == 0 { 2 } else { run + 1 };
                        if run > max {
                            ctx.report_pair(
                                out,
                                IMPERFECT_RUN.id,
                                level,
                                (a, t.a2.note),
                                (b, t.b2.note),
                                t.tick,
                                Detail::Count(run as u32),
                            );
                        }
                    } else if t.motion() != super::analysis::Motion::Static {
                        run = 0;
                    }
                }
            }
        }
    }
}
