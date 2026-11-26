//! Clausulae at the end and at internal cadences (TEX-PENULT, TEX-CLAUSULA, CAD-INNER-*).
//!
//! A cadence arrives on its target at the start of a measure, the goal. The last sonority before
//! the goal holds the clausula: one voice leads into the target by a semitone (up from the
//! leading tone, or in a Phrygian clausula down from the semitone above), and from three voices
//! on the bass comes from a degree of the cadence. The final cadence follows Fux's examples
//! (TEX-PENULT, TEX-CLAUSULA, from three voices on; two voices: CAD-*). Internal cadences are
//! marked in the score and follow the same pattern with rules of their own (CAD-INNER-*), also
//! in two voices.

use super::analysis::{CadencePoint, free_final_cadence, harmonic};
use super::dissonance::{cadence_tritone, delayed_leading_tone, free_delayed_leading_tone};
use super::melodic::level;
use super::species::held_part;
use super::texture::{report_sonority, sounding};
use super::{Ctx, Detail, Rule, RuleInfo, Violation};
use crate::interval::Interval;
use crate::mode::Clausula;
use crate::score::{Slice, Sounding};

pub(super) fn inner_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(InnerPenultimate),
        Box::new(InnerClausula),
        Box::new(InnerGoal),
    ]
}

/// Whether a bass note lies on a degree from which the bass leads into the target: the fifth, or
/// the second with the step of the tenor's clausula; before a Phrygian clausula the seventh or
/// the second.
fn cadence_degree(ctx: &Ctx, cadence: &CadencePoint, note: Sounding) -> bool {
    let target = cadence.target_pitch(ctx.score.mode, note.pitch.octave);
    let above_target = (note.pitch.midi() - target.midi()).rem_euclid(12);
    match cadence.clausula {
        Clausula::Leading => matches!(above_target, 2 | 7),
        Clausula::Phrygian => matches!(above_target, 1 | 10),
    }
}

/// Whether `from` leads into the target by the semitone of the clausula: up from the leading
/// tone, or in a Phrygian clausula down from the semitone above.
fn clausula_step(ctx: &Ctx, cadence: &CadencePoint, from: Sounding) -> bool {
    (from.pitch.octave - 1..=from.pitch.octave + 1).any(|octave| {
        let to_target = Interval::between(from.pitch, cadence.target_pitch(ctx.score.mode, octave));
        match cadence.clausula {
            Clausula::Leading => to_target.steps == 1 && to_target.semitones == 1,
            Clausula::Phrygian => to_target.steps == -1 && to_target.semitones == -1,
        }
    })
}

/// The last sonority before the goal, once it is known: every voice has its note there, and the
/// notes end at the goal (the voices move into it), or the piece is complete.
fn penultimate_sonority<'a>(ctx: &'a Ctx<'a>, cadence: &CadencePoint) -> Option<&'a Slice> {
    let slices = &ctx.analysis.slices;
    let slice = &slices[slices
        .partition_point(|s| s.tick < cadence.goal)
        .checked_sub(1)?];
    let moved = slice
        .sounding
        .iter()
        .all(|s| s.is_some_and(|s| s.end == cadence.goal));
    (ctx.analysis.complete_at(slice.tick) && (moved || ctx.complete)).then_some(slice)
}

/// Names of the parameters for the penultimate sonority of a cadence.
struct PenultimateParams {
    three: &'static str,
    four: &'static str,
    bass: &'static str,
}

/// The penultimate sonority of a cadence, from three voices on: each voice consonant with the
/// bass (three voices: unison/octave, third, fifth or major sixth), and the bass on a degree of
/// the cadence.
fn check_penultimate(
    ctx: &Ctx,
    out: &mut Vec<Violation>,
    id: &'static str,
    cadence: &CadencePoint,
    params: &PenultimateParams,
) {
    let voices = ctx.analysis.voice_count();
    if voices < 3 {
        return;
    }
    // On the penultimate downbeat the bass stands on a degree of the cadence, or it holds the
    // target over the barline and steps down later (Fig. 123, 126). Known as soon as every voice
    // has its note there, which lets the search drop hopeless cadences early.
    let penultimate = cadence.goal.saturating_sub(ctx.analysis.per_measure);
    let mut downbeat = true;
    // In free counterpoint the bass may still move in the penultimate measure (IV–V–I).
    if let Some((bass, b)) = ctx
        .analysis
        .bass_at(penultimate)
        .filter(|_| !ctx.score.is_free())
    {
        let held_target = cadence.is_target(ctx.score.mode, b.pitch) && b.start < penultimate;
        if !cadence_degree(ctx, cadence, b) && !held_target {
            downbeat = false;
            let note = held_part(ctx, bass, b, penultimate);
            ctx.report(
                out,
                id,
                ctx.settings.level(params.bass),
                bass,
                note,
                penultimate,
                Detail::Pitch(b.pitch),
            );
        }
    }
    let Some(slice) = penultimate_sonority(ctx, cadence) else {
        return;
    };
    let Some((bass, b)) = slice.lowest() else {
        return;
    };
    // At the end of the measure the bass is on a degree of the cadence, or it has brought the
    // delayed leading tone.
    if downbeat
        && !cadence_degree(ctx, cadence, b)
        && delayed_leading_tone(ctx, bass, b).is_none()
        && !free_delayed_leading_tone(ctx, bass, b)
    {
        ctx.report(
            out,
            id,
            ctx.settings.level(params.bass),
            bass,
            b.note,
            slice.tick,
            Detail::Pitch(b.pitch),
        );
    }
    let three = voices == 3;
    let level = ctx
        .settings
        .level(if three { params.three } else { params.four });
    for (v, note) in sounding(slice) {
        if v == bass {
            continue;
        }
        let interval = harmonic(b, note);
        let s = interval.simple();
        let allowed = if three {
            matches!(
                (s.steps, s.semitones),
                (0, 0) | (2, 3) | (2, 4) | (4, 7) | (5, 9)
            )
        } else {
            interval.is_consonant()
        };
        if !allowed && !cadence_tritone(ctx, bass, b, interval, slice.tick) {
            // Known once the last voice of the sonority has entered.
            ctx.report_pair(
                out,
                id,
                level,
                (v, note.note),
                (bass, b.note),
                slice.tick,
                Detail::Interval(interval),
            );
        }
    }
}

/// The clausula of a cadence: exactly one voice of the penultimate sonority leads into the target
/// by the semitone, and it arrives there.
fn check_clausula(ctx: &Ctx, out: &mut Vec<Violation>, id: &'static str, cadence: &CadencePoint) {
    let plain = plain_subtonium(ctx, out, id, cadence);
    let Some(slice) = penultimate_sonority(ctx, cadence) else {
        return;
    };
    let notes = sounding(slice);
    let candidates: Vec<(usize, Sounding)> = notes
        .iter()
        .copied()
        .filter(|(_, s)| clausula_step(ctx, cadence, *s))
        .collect();
    let decided = notes
        .iter()
        .map(|(_, s)| s.start)
        .max()
        .unwrap_or(slice.tick);
    if candidates.is_empty() {
        if !plain {
            report_sonority(ctx, out, id, ctx.settings.level("level"), &notes, decided);
        }
        return;
    }
    // A doubled leading tone: the voices that are not the cantus and took it last answer.
    let mut doubled = candidates.clone();
    doubled.sort_by_key(|&(v, s)| (ctx.is_target(v), s.start));
    for &(v, s) in &doubled[1..] {
        ctx.report(
            out,
            id,
            ctx.settings.level("doubled"),
            v,
            s.note,
            decided,
            Detail::Pitch(s.pitch),
        );
    }
    if !ctx.analysis.complete_at(cadence.goal) {
        return;
    }
    // One of them has to arrive on the target.
    let arrives = candidates.iter().any(|(v, s)| {
        ctx.analysis.lines[*v]
            .iter()
            .find(|n| n.start == cadence.goal)
            .is_some_and(|n| {
                cadence.is_target(ctx.score.mode, n.pitch)
                    && Interval::between(s.pitch, n.pitch).abs().steps == 1
            })
    });
    if !arrives {
        report_sonority(
            ctx,
            out,
            id,
            ctx.settings.level("level"),
            &notes,
            cadence.goal,
        );
    }
}

/// Where the leading tone needs ficta (C♯ before D), a counterpoint that sounds the plain step
/// (C) into the goal leaves no room for it: the two would clash. Reported as soon as that note is
/// set, which lets the search drop the state early; returns whether it reported.
fn plain_subtonium(
    ctx: &Ctx,
    out: &mut Vec<Violation>,
    id: &'static str,
    cadence: &CadencePoint,
) -> bool {
    let mode = ctx.score.mode;
    let target = cadence.target_pitch(mode, 4);
    let Some(leading) = mode.clausula_semitone(target) else {
        return false;
    };
    if cadence.clausula != Clausula::Leading || mode.is_scale_pitch(leading) {
        return false;
    }
    let penultimate = cadence.goal.saturating_sub(ctx.analysis.per_measure);
    let mut reported = false;
    for (v, line) in ctx.analysis.lines.iter().enumerate() {
        if !ctx.is_target(v) {
            continue;
        }
        let first = line.partition_point(|s| s.end <= penultimate);
        for note in line[first..].iter().take_while(|s| s.start < cadence.goal) {
            if note.end == cadence.goal
                && note.pitch.step == leading.step
                && note.pitch.alter < leading.alter
            {
                let tick = note.start.max(penultimate);
                ctx.report(
                    out,
                    id,
                    ctx.settings.level("level"),
                    v,
                    held_part(ctx, v, *note, tick),
                    tick,
                    Detail::Pitch(note.pitch),
                );
                reported = true;
            }
        }
    }
    reported
}

/// The final cadence. In free counterpoint it is found in the notes of the complete score.
fn final_cadence(ctx: &Ctx) -> Option<CadencePoint> {
    if ctx.score.is_free() {
        return ctx
            .complete
            .then(|| free_final_cadence(ctx.score, ctx.analysis.per_measure, ctx.analysis.total))
            .flatten();
    }
    ctx.analysis
        .cadences
        .last()
        .filter(|c| c.last && near_window(ctx, c))
        .copied()
}

/// The internal cadences whose decisions may fall into the window: all of them lie between the
/// penultimate downbeat and the goal.
fn inner_cadences<'a>(ctx: &'a Ctx<'a>) -> impl Iterator<Item = &'a CadencePoint> + 'a {
    ctx.analysis
        .inner_cadences()
        .filter(move |c| near_window(ctx, c))
}

fn near_window(ctx: &Ctx, cadence: &CadencePoint) -> bool {
    let penultimate = cadence.goal.saturating_sub(ctx.analysis.per_measure);
    ctx.window.contains_range(penultimate, cadence.goal)
}

pub(super) struct Penultimate;

static PENULT: RuleInfo = RuleInfo {
    id: "TEX-PENULT",
    title_de: "Vorletzter Klang: Stimme in ungeeignetem Intervall zum Bass, oder Bass ohne Kadenzschritt",
    title_en: "Penultimate sonority: a voice at an unsuitable interval above the bass, or the bass without a cadential step",
    source: "L25 1.H7 (aus Fux' Beispielen abgeleitet, große Terz eingeschlossen); CdP24 S. 30; verminderte Quinte des verzögerten Leittons im Bass frei (Gallon/Bitsch Regel 54); Korpus (Bass in allen 77 Beispielen auf der 5. oder 2. Stufe oder dem verzögerten Leitton, phrygisch auf D oder F; vierstimmig eine Dissonanz zum Bass in 32 Beispielen)",
    params: &[
        level(
            "level",
            "dreistimmig: nicht Einklang/Oktave, Terz, Quinte oder große Sexte zum Bass",
            "three voices: not a unison/octave, third, fifth or major sixth above the bass",
        ),
        level(
            "four",
            "ab vier Stimmen: dissonant zum Bass",
            "four or more voices: dissonant against the bass",
        ),
        level(
            "bass",
            "Bass nicht auf der 5. oder 2. Stufe oder dem verzögerten Leitton (phrygisch: 7. oder 2. Stufe)",
            "bass not on the fifth or second degree or the delayed leading tone (Phrygian: seventh or second degree)",
        ),
    ],
};

const PENULT_PARAMS: PenultimateParams = PenultimateParams {
    three: "level",
    four: "four",
    bass: "bass",
};

impl Rule for Penultimate {
    fn info(&self) -> &'static RuleInfo {
        &PENULT
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if let Some(cadence) = final_cadence(ctx) {
            check_penultimate(ctx, out, PENULT.id, &cadence, &PENULT_PARAMS);
        }
    }
}

pub(super) struct FinalClausula;

static CLAUSULA: RuleInfo = RuleInfo {
    id: "TEX-CLAUSULA",
    title_de: "Klausel: keine Stimme erreicht die Finalis mit dem Leitton (phrygisch: mit dem Halbton von oben), oder der Leitton ist verdoppelt",
    title_en: "Clausula: no voice reaches the final with the leading tone (Phrygian: with the semitone from above), or the leading tone is doubled",
    source: "Fux; Korpus (in allen 77 drei- und vierstimmigen Beispielen genau ein Leitton im vorletzten Klang, immer in die Finalis aufgelöst)",
    params: &[
        level(
            "level",
            "keine Stimme mit dem Leitton",
            "no voice with the leading tone",
        ),
        level(
            "doubled",
            "Leitton in mehreren Stimmen",
            "the leading tone in several voices",
        ),
    ],
};

impl Rule for FinalClausula {
    fn info(&self) -> &'static RuleInfo {
        &CLAUSULA
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() < 3 {
            return;
        }
        if let Some(cadence) = final_cadence(ctx) {
            check_clausula(ctx, out, CLAUSULA.id, &cadence);
        }
    }
}

struct InnerPenultimate;

static INNER_PENULT: RuleInfo = RuleInfo {
    id: "CAD-INNER-PENULT",
    title_de: "Binnenschluss, vorletzter Klang: Stimme in ungeeignetem Intervall zum Bass, oder Bass ohne Kadenzschritt",
    title_en: "Internal cadence, penultimate sonority: a voice at an unsuitable interval above the bass, or the bass without a cadential step",
    source: "Wie TEX-PENULT, bezogen auf das Ziel des Binnenschlusses",
    params: &[
        level(
            "level",
            "dreistimmig: nicht Einklang/Oktave, Terz, Quinte oder große Sexte zum Bass",
            "three voices: not a unison/octave, third, fifth or major sixth above the bass",
        ),
        level(
            "four",
            "ab vier Stimmen: dissonant zum Bass",
            "four or more voices: dissonant against the bass",
        ),
        level(
            "bass",
            "Bass nicht auf der 5. oder 2. Stufe des Ziels oder dem verzögerten Leitton (phrygische Klausel: 7. oder 2. Stufe)",
            "bass not on the fifth or second degree of the target or the delayed leading tone (Phrygian clausula: seventh or second degree)",
        ),
    ],
};

impl Rule for InnerPenultimate {
    fn info(&self) -> &'static RuleInfo {
        &INNER_PENULT
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for cadence in inner_cadences(ctx) {
            check_penultimate(ctx, out, INNER_PENULT.id, cadence, &PENULT_PARAMS);
        }
    }
}

struct InnerClausula;

static INNER_CLAUSULA: RuleInfo = RuleInfo {
    id: "CAD-INNER-CLAUSULA",
    title_de: "Binnenschluss: keine Stimme erreicht das Ziel mit dem Leitton (phrygische Klausel: mit dem Halbton von oben), oder der Leitton ist verdoppelt",
    title_en: "Internal cadence: no voice reaches the target with the leading tone (Phrygian clausula: with the semitone from above), or the leading tone is doubled",
    source: "Wie TEX-CLAUSULA, bezogen auf das Ziel des Binnenschlusses, auch zweistimmig",
    params: &[
        level(
            "level",
            "keine Stimme mit dem Leitton",
            "no voice with the leading tone",
        ),
        level(
            "doubled",
            "Leitton in mehreren Stimmen",
            "the leading tone in several voices",
        ),
    ],
};

impl Rule for InnerClausula {
    fn info(&self) -> &'static RuleInfo {
        &INNER_CLAUSULA
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for cadence in inner_cadences(ctx) {
            check_clausula(ctx, out, INNER_CLAUSULA.id, cadence);
        }
    }
}

struct InnerGoal;

static INNER_GOAL: RuleInfo = RuleInfo {
    id: "CAD-INNER-GOAL",
    title_de: "Binnenschluss, Zielklang: zweistimmig nicht Einklang oder Oktave auf dem Ziel; ab drei Stimmen Bass nicht auf dem Ziel oder fremder Ton",
    title_en: "Internal cadence, goal sonority: in two voices not a unison or octave on the target; from three voices on the bass not on the target, or a foreign tone",
    source: "Wie CAD-END und TEX-FINAL-CHORD, bezogen auf das Ziel; die Terz darf klein sein",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for InnerGoal {
    fn info(&self) -> &'static RuleInfo {
        &INNER_GOAL
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let level = ctx.settings.level("level");
        let mode = ctx.score.mode;
        for cadence in inner_cadences(ctx) {
            let Some(slice) = ctx
                .analysis
                .slice_at(cadence.goal)
                .filter(|s| s.tick == cadence.goal)
            else {
                continue;
            };
            // The tones of the goal are known voice by voice once the bass is set.
            let Some((bass, b)) = ctx.analysis.bass_at(cadence.goal) else {
                continue;
            };
            if !cadence.is_target(mode, b.pitch) {
                ctx.report(
                    out,
                    INNER_GOAL.id,
                    level,
                    bass,
                    b.note,
                    cadence.goal,
                    Detail::Pitch(b.pitch),
                );
            }
            let two = ctx.analysis.voice_count() == 2;
            for (v, note) in sounding(slice) {
                if v == bass {
                    continue;
                }
                let interval = harmonic(b, note);
                let s = interval.simple();
                let allowed = if two {
                    (s.steps, s.semitones) == (0, 0)
                } else {
                    matches!((s.steps, s.semitones), (0, 0) | (2, 3) | (2, 4) | (4, 7))
                };
                if !allowed {
                    ctx.report_pair(
                        out,
                        INNER_GOAL.id,
                        level,
                        (v, note.note),
                        (bass, b.note),
                        cadence.goal,
                        Detail::Interval(interval),
                    );
                }
            }
        }
    }
}
