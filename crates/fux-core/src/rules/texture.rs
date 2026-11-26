//! Rules of the texture with three or more voices (TEX-*): parallels and hidden parallels between
//! any two voices, the sonority on each downbeat, and the first, penultimate and final chord.
//!
//! A sonority is measured against the bass, the lowest sounding voice. Rules about a whole
//! sonority wait until the search has set every voice there.

use super::analysis::{Motion, Transition, harmonic};
use super::melodic::level;
use super::{Ctx, Detail, Rule, RuleInfo, Violation};
use crate::interval::Interval;
use crate::score::{Slice, Sounding};

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(Triad),
        Box::new(Doubling),
        Box::new(Parallel),
        Box::new(DirectPerfect),
        Box::new(SuccessivePerfect),
        Box::new(SameDirection),
        Box::new(SixthsUp),
        Box::new(FirstChord),
        Box::new(super::clausula::Penultimate),
        Box::new(super::clausula::FinalClausula),
        Box::new(FinalChord),
    ]
}

/// The parameter for the number of voices: `<name>` for three, `<name>_four` from four on.
fn by_voices(ctx: &Ctx, three: &'static str, four: &'static str) -> &'static str {
    if ctx.analysis.voice_count() >= 4 {
        four
    } else {
        three
    }
}

/// The sounding notes of a slice with their voices.
pub(super) fn sounding(slice: &Slice) -> Vec<(usize, Sounding)> {
    slice
        .sounding
        .iter()
        .enumerate()
        .filter_map(|(v, s)| s.map(|s| (v, s)))
        .collect()
}

/// The notes of a sonority with their voices, and its bass.
type Sonority = (Vec<(usize, Sounding)>, (usize, Sounding));

/// A downbeat sonority of three or more voices that is complete and struck by every voice (no
/// note held over), with its bass.
fn struck_sonority(ctx: &Ctx, i: usize) -> Option<Sonority> {
    let slice = &ctx.analysis.slices[i];
    if ctx.analysis.voice_count() < 3 || !ctx.analysis.complete_at(slice.tick) {
        return None;
    }
    let notes = sounding(slice);
    if notes.len() < 3 || notes.iter().any(|(_, s)| s.start != slice.tick) {
        return None;
    }
    let bass = slice.lowest()?;
    Some((notes, bass))
}

/// Simple diatonic steps of each upper note above the bass (0 = unison or octave, 2 = third, …).
pub(super) fn steps_above(notes: &[(usize, Sounding)], bass: (usize, Sounding)) -> Vec<i32> {
    notes
        .iter()
        .filter(|(v, _)| *v != bass.0)
        .map(|(_, s)| harmonic(bass.1, *s).simple().steps)
        .collect()
}

/// The kind of a consonant sonority over its bass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// Third and fifth: Fux's harmonic triad.
    Complete,
    /// Third and sixth.
    Sixth,
    /// Third with octaves or unisons only.
    NoFifth,
    /// Fifths and octaves only.
    NoThird,
    /// Sixths with octaves or unisons only: the bare sixth.
    BareSixth,
}

/// The kind of the sonority whose upper notes lie `steps` above the bass, or `None` where it is
/// dissonant (a second, fourth or seventh above the bass, or a fifth with a sixth).
pub(super) fn kind(steps: &[i32]) -> Option<Kind> {
    let has = |s: i32| steps.contains(&s);
    if steps.iter().any(|s| ![0, 2, 4, 5].contains(s)) || (has(4) && has(5)) {
        return None; // dissonant: judged by other rules
    }
    Some(match (has(2), has(4), has(5)) {
        (true, true, _) => Kind::Complete,
        (true, false, true) => Kind::Sixth,
        (true, false, false) => Kind::NoFifth,
        (false, _, false) => Kind::NoThird,
        (false, _, true) => Kind::BareSixth,
    })
}

struct Triad;

static TRIAD: RuleInfo = RuleInfo {
    id: "TEX-TRIAD",
    title_de: "Kein vollständiger Dreiklang auf der Thesis",
    title_en: "No complete triad on the downbeat",
    source: "Mann 71 f., 109; L24 1.H8, 1.H9; L25 1.H8; Korpus (1. Gattung, innere Klänge: dreistimmig 5/3 75, Terz und Oktave 51, 6/3 26, Quinte und Oktave ohne Terz 0, leere Sexte 3; vierstimmig 82, 1, 17, 0, 0); freier Satz: Korpus Palestrina (angeschlagene Klänge innerer Takte, dreistimmig und ab vier Stimmen: Terz und Oktave 37,9 und 16,3 %, ohne Terz 7,4 und 3,2 %, leere Sexte 9,0 und 2,6 %)",
    params: &[
        level("sixth", "Sextklang (6/3)", "sixth chord (6/3)"),
        level(
            "no_fifth",
            "dreistimmig: Terz und Oktave, ohne Quinte",
            "three voices: third and octave, no fifth",
        ),
        level(
            "no_fifth_four",
            "ab vier Stimmen: Terz und Oktave, ohne Quinte",
            "four or more voices: third and octave, no fifth",
        ),
        level(
            "no_third",
            "dreistimmig: ohne Terz",
            "three voices: no third",
        ),
        level(
            "no_third_four",
            "ab vier Stimmen: ohne Terz",
            "four or more voices: no third",
        ),
        level(
            "free_no_fifth",
            "freier Satz: Terz und Oktave, ohne Quinte",
            "free counterpoint: third and octave, no fifth",
        ),
        level(
            "free_no_third",
            "freier Satz: ohne Terz",
            "free counterpoint: no third",
        ),
        level(
            "free_bare_sixth",
            "freier Satz: leere Sexte (Sexte und Oktave, ohne Terz)",
            "free counterpoint: bare sixth (sixth and octave, no third)",
        ),
    ],
};

impl Rule for Triad {
    fn info(&self) -> &'static RuleInfo {
        &TRIAD
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for i in ctx.downbeats() {
            let tick = ctx.analysis.slices[i].tick;
            if ctx.analysis.in_first_measure(tick) || ctx.in_last_measure(tick) {
                continue;
            }
            let Some((notes, bass)) = struck_sonority(ctx, i) else {
                continue;
            };
            let free = ctx.score.is_free();
            let name = match kind(&steps_above(&notes, bass)) {
                Some(Kind::Sixth) => "sixth",
                Some(Kind::NoFifth) if free => "free_no_fifth",
                Some(Kind::NoFifth) => by_voices(ctx, "no_fifth", "no_fifth_four"),
                Some(Kind::NoThird) if free => "free_no_third",
                Some(Kind::NoThird) => by_voices(ctx, "no_third", "no_third_four"),
                Some(Kind::BareSixth) if free => "free_bare_sixth",
                // The species leave the bare sixth free (docs/regeln.md § 12).
                Some(Kind::Complete | Kind::BareSixth) | None => continue,
            };
            report_sonority(ctx, out, TRIAD.id, ctx.settings.level(name), &notes, tick);
        }
    }
}

/// Reports a violation about a sonority on the first target voice that sounds in it.
pub(super) fn report_sonority(
    ctx: &Ctx,
    out: &mut Vec<Violation>,
    id: &'static str,
    level: super::Level,
    notes: &[(usize, Sounding)],
    tick: u32,
) {
    if let Some((v, s)) = notes.iter().find(|(v, _)| ctx.is_target(*v)) {
        let note = super::species::held_part(ctx, *v, *s, tick);
        ctx.report(out, id, level, *v, note, tick, Detail::None);
    }
}

struct Doubling;

static DOUBLING: RuleInfo = RuleInfo {
    id: "TEX-DOUBLING",
    title_de: "Verdopplung im vollständigen Dreiklang: Terz oder Quinte statt Grundton",
    title_en: "Doubling in the complete triad: third or fifth instead of the root",
    source: "Mann 109, 111; L25; Korpus (vierstimmig: Grundton 55×, Terz 25×, Quinte 2×)",
    params: &[
        level("third", "verdoppelte Terz", "doubled third"),
        level(
            "fifth",
            "vierstimmig: verdoppelte Quinte",
            "four voices: doubled fifth",
        ),
        level(
            "fifth_more",
            "ab fünf Stimmen: verdoppelte Quinte",
            "five or more voices: doubled fifth",
        ),
    ],
};

impl Rule for Doubling {
    fn info(&self) -> &'static RuleInfo {
        &DOUBLING
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() < 4 {
            return;
        }
        for i in ctx.downbeats() {
            let tick = ctx.analysis.slices[i].tick;
            if ctx.analysis.in_first_measure(tick) || ctx.in_last_measure(tick) {
                continue;
            }
            let Some((notes, bass)) = struck_sonority(ctx, i) else {
                continue;
            };
            let steps = steps_above(&notes, bass);
            if kind(&steps) != Some(Kind::Complete) {
                continue;
            }
            let count = |s: i32| steps.iter().filter(|&&x| x == s).count();
            if count(2) >= 2 {
                report_sonority(
                    ctx,
                    out,
                    DOUBLING.id,
                    ctx.settings.level("third"),
                    &notes,
                    tick,
                );
            }
            if count(4) >= 2 {
                let name = if ctx.analysis.voice_count() >= 5 {
                    "fifth_more"
                } else {
                    "fifth"
                };
                report_sonority(
                    ctx,
                    out,
                    DOUBLING.id,
                    ctx.settings.level(name),
                    &notes,
                    tick,
                );
            }
        }
    }
}

/// Whether two intervals are the same perfect consonance (unison or octave, or fifth).
fn same_perfect(a: Interval, b: Interval) -> bool {
    a.is_perfect_consonance() && b.is_perfect_consonance() && a.simple().steps == b.simple().steps
}

struct Parallel;

static PARALLEL: RuleInfo = RuleInfo {
    id: "TEX-PARALLEL",
    title_de: "Parallele Einklänge, Quinten oder Oktaven",
    title_en: "Parallel unisons, fifths or octaves",
    source: "Fux; Korpus (in allen 77 drei- und vierstimmigen Beispielen nie); B26 P8",
    params: &[level("level", "ab drei Stimmen", "three or more voices")],
};

impl Rule for Parallel {
    fn info(&self) -> &'static RuleInfo {
        &PARALLEL
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let n = ctx.analysis.voice_count();
        if n < 3 {
            return;
        }
        for i in ctx.slice_range().filter(|&i| i > 0) {
            for a in 0..n {
                for b in a + 1..n {
                    let Some(t) = ctx.analysis.transition(i, a, b) else {
                        continue;
                    };
                    if t.motion() == Motion::Similar && same_perfect(t.before(), t.after()) {
                        ctx.report_pair(
                            out,
                            PARALLEL.id,
                            ctx.settings.level("level"),
                            (a, t.a2.note),
                            (b, t.b2.note),
                            t.tick,
                            Detail::Interval(t.after()),
                        );
                    }
                }
            }
        }
    }
}

struct DirectPerfect;

static DIRECT_PERFECT: RuleInfo = RuleInfo {
    id: "TEX-DIRECT-PERFECT",
    title_de: "Verdeckte Quinte oder Oktave (vollkommene Konsonanz in gerader Bewegung erreicht)",
    title_en: "Hidden fifth or octave (perfect consonance reached in similar motion)",
    source: "Mann 77 („if there is no other possibility“), 109, 123; L24 1.P1; L25; Korpus (dreistimmig: Außenstimmen 6×, andere Paare 13×, in den Schlussklang 22×; vierstimmig 20×, 44×, 21×)",
    params: &[
        level(
            "outer",
            "dreistimmig: Außenstimmen",
            "three voices: outer voices",
        ),
        level(
            "inner",
            "dreistimmig: andere Stimmpaare",
            "three voices: other pairs",
        ),
        level(
            "outer_four",
            "vierstimmig: Außenstimmen",
            "four voices: outer voices",
        ),
        level(
            "inner_four",
            "vierstimmig: andere Stimmpaare",
            "four voices: other pairs",
        ),
        level(
            "outer_more",
            "ab fünf Stimmen: Außenstimmen",
            "five or more voices: outer voices",
        ),
        level(
            "inner_more",
            "ab fünf Stimmen: andere Stimmpaare",
            "five or more voices: other pairs",
        ),
        level("final_chord", "in den Schlussklang", "into the final chord"),
    ],
};

impl Rule for DirectPerfect {
    fn info(&self) -> &'static RuleInfo {
        &DIRECT_PERFECT
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let n = ctx.analysis.voice_count();
        if n < 3 {
            return;
        }
        // In free counterpoint the end is known only in the complete score.
        let final_tick = if ctx.score.is_free() && !ctx.complete {
            u32::MAX
        } else {
            ctx.analysis.total.saturating_sub(ctx.analysis.per_measure)
        };
        for i in ctx.slice_range().filter(|&i| i > 0) {
            let slice = &ctx.analysis.slices[i];
            if !ctx.analysis.complete_at(slice.tick) {
                continue;
            }
            let notes = sounding(slice);
            let lowest = slice.lowest().map(|(v, _)| v);
            let highest = notes
                .iter()
                .max_by_key(|(v, s)| (s.pitch.midi(), s.pitch.diatonic(), usize::MAX - v))
                .map(|(v, _)| *v);
            for a in 0..n {
                for b in a + 1..n {
                    let Some(t) = ctx.analysis.transition(i, a, b) else {
                        continue;
                    };
                    let after = t.after();
                    if t.motion() != Motion::Similar
                        || !after.is_perfect_consonance()
                        || same_perfect(t.before(), after)
                    {
                        continue;
                    }
                    let outer = [Some(a), Some(b)].contains(&lowest)
                        && [Some(a), Some(b)].contains(&highest);
                    let name = match (slice.tick >= final_tick, n, outer) {
                        (true, _, _) => "final_chord",
                        (false, 3, true) => "outer",
                        (false, 3, false) => "inner",
                        (false, 4, true) => "outer_four",
                        (false, 4, false) => "inner_four",
                        (false, _, true) => "outer_more",
                        (false, _, false) => "inner_more",
                    };
                    ctx.report_pair(
                        out,
                        DIRECT_PERFECT.id,
                        ctx.settings.level(name),
                        (a, t.a2.note),
                        (b, t.b2.note),
                        t.tick,
                        Detail::Interval(after),
                    );
                }
            }
        }
    }
}

/// Consecutive downbeat slices where both voices of every pair counted strike the second one.
fn downbeat_pairs(ctx: &Ctx) -> Vec<(usize, usize)> {
    let per = ctx.analysis.per_measure;
    ctx.downbeats()
        .filter_map(|i| {
            let tick = ctx.analysis.slices[i].tick;
            let before = tick.checked_sub(per)?;
            let j = ctx
                .analysis
                .slices
                .binary_search_by_key(&before, |s| s.tick)
                .ok()?;
            (ctx.analysis.complete_at(tick) && ctx.analysis.complete_at(before)).then_some((j, i))
        })
        .collect()
}

struct SuccessivePerfect;

static SUCCESSIVE_PERFECT: RuleInfo = RuleInfo {
    id: "TEX-SUCCESSIVE-PERFECT",
    title_de: "Zwei vollkommene Konsonanzen nacheinander im selben Stimmpaar",
    title_en: "Two perfect consonances in a row between the same two voices",
    source: "Mann 72, 86; L24 1.P4, 2.P3 (Fux hält sich oft nicht daran: Fig. 108–110)",
    params: &[level(
        "level",
        "von Thesis zu Thesis",
        "from downbeat to downbeat",
    )],
};

impl Rule for SuccessivePerfect {
    fn info(&self) -> &'static RuleInfo {
        &SUCCESSIVE_PERFECT
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let n = ctx.analysis.voice_count();
        if n < 3 {
            return;
        }
        for (j, i) in downbeat_pairs(ctx) {
            let (s1, s2) = (&ctx.analysis.slices[j], &ctx.analysis.slices[i]);
            for a in 0..n {
                for b in a + 1..n {
                    let (Some(a1), Some(a2), Some(b1), Some(b2)) = (
                        s1.sounding[a],
                        s2.sounding[a],
                        s1.sounding[b],
                        s2.sounding[b],
                    ) else {
                        continue;
                    };
                    if a2.start != s2.tick && b2.start != s2.tick {
                        continue;
                    }
                    let t = Transition {
                        a1,
                        a2,
                        b1,
                        b2,
                        tick: s2.tick,
                    };
                    let (before, after) = (t.before(), t.after());
                    if !(before.is_perfect_consonance() && after.is_perfect_consonance()) {
                        continue;
                    }
                    // Similar motion is judged by TEX-PARALLEL and TEX-DIRECT-PERFECT, the same
                    // consonance in contrary motion by MOT-ANTIPARALLEL.
                    let motion = t.motion();
                    if motion == Motion::Similar
                        || motion == Motion::Static
                        || (motion == Motion::Contrary && same_perfect(before, after))
                    {
                        continue;
                    }
                    ctx.report_pair(
                        out,
                        SUCCESSIVE_PERFECT.id,
                        ctx.settings.level("level"),
                        (a, super::species::held_part(ctx, a, a2, s2.tick)),
                        (b, super::species::held_part(ctx, b, b2, s2.tick)),
                        s2.tick,
                        Detail::Interval(after),
                    );
                }
            }
        }
    }
}

struct SameDirection;

static SAME_DIRECTION: RuleInfo = RuleInfo {
    id: "TEX-SAME-DIRECTION",
    title_de: "Alle Stimmen in dieselbe Richtung",
    title_en: "All voices move in the same direction",
    source: "Mann 76 („can hardly be managed without awkwardness“); L24 1.P6; Korpus (1. Gattung: dreistimmig 10 von 171 Klangwechseln, vierstimmig 0 von 111)",
    params: &[
        level("level", "dreistimmig", "three voices"),
        level("four_voices", "ab vier Stimmen", "four or more voices"),
        level("free", "freier Satz", "free counterpoint"),
    ],
};

impl Rule for SameDirection {
    fn info(&self) -> &'static RuleInfo {
        &SAME_DIRECTION
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() < 3 {
            return;
        }
        for i in ctx.downbeats() {
            let Some((after, _)) = struck_sonority(ctx, i) else {
                continue;
            };
            // Each voice moves from its last note before the downbeat.
            let directions: Option<Vec<i32>> = after
                .iter()
                .map(|(v, y)| {
                    let line = &ctx.analysis.lines[*v];
                    let k = line.partition_point(|s| s.start < y.start);
                    let x = line[..k].last().filter(|x| x.end == y.start)?;
                    Some((y.pitch.midi() - x.pitch.midi()).signum())
                })
                .collect();
            let Some(directions) = directions else {
                continue;
            };
            if directions[0] != 0 && directions.iter().all(|d| *d == directions[0]) {
                let name = if ctx.score.is_free() {
                    "free"
                } else {
                    by_voices(ctx, "level", "four_voices")
                };
                let tick = ctx.analysis.slices[i].tick;
                report_sonority(
                    ctx,
                    out,
                    SAME_DIRECTION.id,
                    ctx.settings.level(name),
                    &after,
                    tick,
                );
            }
        }
    }
}

struct SixthsUp;

static SIXTHS_UP: RuleInfo = RuleInfo {
    id: "TEX-SIXTHS-UP",
    title_de: "Aufeinanderfolgende Sexten zum Bass in steigender Bewegung",
    title_en: "Consecutive sixths above the bass in rising motion",
    source: "Mann 77 („Ascending sixths on the downbeat sound harsh“); L24 1.P7; Korpus (dreistimmig 1×, vierstimmig nie)",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for SixthsUp {
    fn info(&self) -> &'static RuleInfo {
        &SIXTHS_UP
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() < 3 {
            return;
        }
        for (j, i) in downbeat_pairs(ctx) {
            let (s1, s2) = (&ctx.analysis.slices[j], &ctx.analysis.slices[i]);
            let Some((bass, b2)) = s2.lowest() else {
                continue;
            };
            let Some(b1) = s1.sounding[bass] else {
                continue;
            };
            if b2.pitch.midi() <= b1.pitch.midi() {
                continue;
            }
            for (v, (x1, x2)) in s1.sounding.iter().zip(&s2.sounding).enumerate() {
                let (Some(x1), Some(x2)) = (*x1, *x2) else {
                    continue;
                };
                let sixth = |a: Sounding, b: Sounding| harmonic(a, b).simple().steps == 5;
                if v != bass && x2.pitch.midi() > x1.pitch.midi() && sixth(b1, x1) && sixth(b2, x2)
                {
                    ctx.report_pair(
                        out,
                        SIXTHS_UP.id,
                        ctx.settings.level("level"),
                        (v, x2.note),
                        (bass, b2.note),
                        s2.tick,
                        Detail::Interval(harmonic(b2, x2)),
                    );
                }
            }
        }
    }
}

/// The first slice in which every voice sounds, with its bass.
fn first_full<'a>(ctx: &'a Ctx<'a>) -> Option<&'a Slice> {
    ctx.analysis
        .slices
        .iter()
        .find(|s| s.sounding.iter().all(Option::is_some))
}

fn on_final(ctx: &Ctx, note: Sounding) -> bool {
    note.pitch
        .same_class(ctx.score.mode.final_pitch(note.pitch.octave))
}

struct FirstChord;

static FIRST_CHORD: RuleInfo = RuleInfo {
    id: "TEX-FIRST-CHORD",
    title_de: "Erster Klang nicht der Dreiklang der Finalis (Bass auf der Finalis, darüber Oktave, Terz oder Quinte)",
    title_en: "First sonority not the triad of the final (the final in the bass, octave, third or fifth above it)",
    source: "L24 S. 26; Korpus (Bass immer auf der Finalis, darüber nur Einklang/Oktave, Terz und Quinte; Terz in 23 von 45 bzw. 24 von 32 ersten Klängen)",
    params: &[level("level", "ab drei Stimmen", "three or more voices")],
};

impl Rule for FirstChord {
    fn info(&self) -> &'static RuleInfo {
        &FIRST_CHORD
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        // In free counterpoint the voices enter one after the other: no first sonority.
        if ctx.analysis.voice_count() < 3 || ctx.score.is_free() {
            return;
        }
        let Some(slice) = first_full(ctx) else { return };
        let Some((bass, b)) = ctx.analysis.bass_at(slice.tick) else {
            return;
        };
        let level = ctx.settings.level("level");
        if !on_final(ctx, b) {
            ctx.report(
                out,
                FIRST_CHORD.id,
                level,
                bass,
                b.note,
                slice.tick,
                Detail::Pitch(b.pitch),
            );
        }
        for (v, note) in sounding(slice) {
            let interval = harmonic(b, note);
            if v != bass && ![0, 2, 4].contains(&interval.simple().steps) {
                // Known once the last voice has entered.
                ctx.report_pair(
                    out,
                    FIRST_CHORD.id,
                    level,
                    (v, note.note),
                    (bass, b.note),
                    slice.tick,
                    Detail::Interval(interval),
                );
            }
        }
    }
}

struct FinalChord;

static FINAL_CHORD: RuleInfo = RuleInfo {
    id: "TEX-FINAL-CHORD",
    title_de: "Schlussklang: Bass nicht auf der Finalis, fremder Ton oder kleine Terz",
    title_en: "Final sonority: the bass not on the final, a foreign tone or a minor third",
    source: "Mann 77, 80 („The minor third is not capable of giving a sense of conclusion“), 82; L25 1.H3, 1.H12; Korpus (Terz dreistimmig in 19 von 45, vierstimmig in 18 von 32 Schlussklängen, immer groß); freier Satz: Korpus Palestrina (ohne große Terz 110 von 1156 Schlussklängen: dreistimmig 19 von 74, vierstimmig 86 von 448, fünf- und sechsstimmig 5 von 634)",
    params: &[
        level("level", "Stufe", "level"),
        level(
            "third",
            "große Terz im Schlussklang (Einstellung „vermeiden“: P2)",
            "major third in the final chord (setting “avoid”: P2)",
        ),
        level(
            "missing_third",
            "Schlussklang ohne Terz (Einstellung „bevorzugt“: P2)",
            "final chord without a third (setting “prefer”: P2)",
        ),
        level(
            "free_missing_third",
            "freier Satz: Schlussklang ohne große Terz",
            "free counterpoint: final chord without a major third",
        ),
        level(
            "free_degree",
            "freier Satz: Abschnitt endet nicht auf Finalis, Quarte oder Quinte des Modus",
            "free counterpoint: a section ends not on the final, the fourth or the fifth of the mode",
        ),
    ],
};

impl Rule for FinalChord {
    fn info(&self) -> &'static RuleInfo {
        &FINAL_CHORD
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() < 3 {
            return;
        }
        let free = ctx.score.is_free();
        // The tones of the final chord are known voice by voice once the bass is set. In free
        // counterpoint the final chord is the last sonority: the final is held long, and a coda
        // may move over it.
        let final_tick = if free {
            match ctx.analysis.slices.last() {
                Some(last) if ctx.complete => last.tick,
                _ => return,
            }
        } else {
            ctx.analysis.total.saturating_sub(ctx.analysis.per_measure)
        };
        let Some(slice) = ctx
            .analysis
            .slice_at(final_tick)
            .filter(|s| s.tick == final_tick)
        else {
            return;
        };
        let Some((bass, b)) = ctx.analysis.bass_at(final_tick) else {
            return;
        };
        let level = ctx.settings.level("level");
        if free {
            // A section may end on the final, the fourth or the fifth of the mode.
            let above_final =
                Interval::between(ctx.score.mode.final_pitch(b.pitch.octave), b.pitch).simple();
            if !matches!(above_final.steps, 0 | 3 | 4) {
                ctx.report(
                    out,
                    FINAL_CHORD.id,
                    ctx.settings.level("free_degree"),
                    bass,
                    b.note,
                    slice.tick,
                    Detail::Pitch(b.pitch),
                );
            }
        } else if !on_final(ctx, b) {
            ctx.report(
                out,
                FINAL_CHORD.id,
                level,
                bass,
                b.note,
                slice.tick,
                Detail::Pitch(b.pitch),
            );
        }
        let mut third = None;
        for (v, note) in sounding(slice) {
            if v == bass {
                continue;
            }
            let interval = harmonic(b, note);
            let s = interval.simple();
            match (s.steps, s.semitones) {
                (0, 0) | (4, 7) => {}
                (2, 4) => third = Some((v, note)),
                _ => ctx.report_pair(
                    out,
                    FINAL_CHORD.id,
                    level,
                    (v, note.note),
                    (bass, b.note),
                    slice.tick,
                    Detail::Interval(interval),
                ),
            }
        }
        if !ctx.complete {
            return;
        }
        match third {
            Some((v, note)) => ctx.report(
                out,
                FINAL_CHORD.id,
                ctx.settings.level("third"),
                v,
                note.note,
                slice.tick,
                Detail::Pitch(note.pitch),
            ),
            None => {
                // The species keep their earlier message (phase 14, D15); free counterpoint names
                // its case.
                let (name, detail) = if free {
                    ("free_missing_third", Detail::Param("free_missing_third"))
                } else {
                    ("missing_third", Detail::None)
                };
                ctx.report(
                    out,
                    FINAL_CHORD.id,
                    ctx.settings.level(name),
                    bass,
                    b.note,
                    slice.tick,
                    detail,
                )
            }
        }
    }
}
