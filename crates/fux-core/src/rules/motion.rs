//! Rules about the motion of two voices (MOT-*).

use super::analysis::{Motion, Transition, harmonic};
use super::melodic::level;
use super::species::g2_reference;
use super::{Ctx, Detail, Rule, RuleInfo, Violation};
use crate::interval::Interval;

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(DirectPerfect),
        Box::new(Preference),
        Box::new(Battuta),
        Box::new(Antiparallel),
    ]
}

/// Both intervals perfect and of the same kind (fifth or octave/unison).
fn same_perfect(a: Interval, b: Interval) -> bool {
    a.is_perfect_consonance() && b.is_perfect_consonance() && a.simple().steps == b.simple().steps
}

struct DirectPerfect;

static DIRECT_PERFECT: RuleInfo = RuleInfo {
    id: "MOT-DIRECT-PERFECT",
    title_de: "Vollkommene Konsonanz in gerader Bewegung erreicht",
    title_en: "Perfect consonance reached in similar motion",
    source: "Mann 22; Ch. 51, 57; W23 1.P1; Korpus",
    params: &[level("level", "Stufe (zweistimmig)", "level (two voices)")],
};

impl Rule for DirectPerfect {
    fn info(&self) -> &'static RuleInfo {
        &DIRECT_PERFECT
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() != 2 {
            return;
        }
        let level = ctx.settings.level("level");
        for i in ctx.slice_range().filter(|&i| i > 0) {
            let Some(t) = ctx.analysis.transition(i, 0, 1) else {
                continue;
            };
            let after = t.after();
            if t.motion() == Motion::Similar && after.is_perfect_consonance() {
                ctx.report_pair(
                    out,
                    DIRECT_PERFECT.id,
                    level,
                    (0, t.a2.note),
                    (1, t.b2.note),
                    t.tick,
                    Detail::Interval(after),
                );
            }
        }
    }

    fn message(&self, violation: &Violation, lang: crate::lang::Lang) -> String {
        let title = self.info().title(lang);
        match violation.detail {
            Detail::Interval(i) => format!("{title}: {}", i.name(lang)),
            _ => title.to_string(),
        }
    }
}

struct Preference;

static PREFERENCE: RuleInfo = RuleInfo {
    id: "MOT-PREFERENCE",
    title_de: "Bewegungsart in den Taktanfang (Gegenbewegung bevorzugt, ab drei Stimmen gegen den Bass)",
    title_en: "Kind of motion into the downbeat (contrary motion preferred, with three or more voices against the bass)",
    source: "Ch. 53; W23 1.P2, 2.P1, 3.P1; L25 1.P2 (ab drei Stimmen jede Stimme gegen den Bass)",
    params: &[
        level("oblique", "Seitenbewegung", "oblique motion"),
        level("similar", "gerade Bewegung", "similar motion"),
    ],
};

impl Rule for Preference {
    fn info(&self) -> &'static RuleInfo {
        &PREFERENCE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let n = ctx.analysis.voice_count();
        for i in ctx.downbeats() {
            let slice = &ctx.analysis.slices[i];
            // With three or more voices each voice counts against the bass (L25 1.P2).
            let bass = if n > 2 {
                if !ctx.analysis.complete_at(slice.tick) {
                    continue;
                }
                slice.lowest().map(|(b, _)| b)
            } else {
                None
            };
            for a in 0..n {
                for b in a + 1..n {
                    if bass.is_some_and(|bass| a != bass && b != bass) {
                        continue;
                    }
                    // A note held over the barline (fourth species) does not move.
                    if !(slice.attacks(a) && slice.attacks(b)) {
                        continue;
                    }
                    let Some(t) = ctx.analysis.transition(i, a, b) else {
                        continue;
                    };
                    // The motion comes from the last note before the downbeat; in the second
                    // species from the downbeat before if the voice skips a third at most.
                    let t = Transition {
                        a1: g2_reference(ctx, a, t.a1).unwrap_or(t.a1),
                        b1: g2_reference(ctx, b, t.b1).unwrap_or(t.b1),
                        ..t
                    };
                    let name = match t.motion() {
                        Motion::Oblique => "oblique",
                        Motion::Similar => "similar",
                        Motion::Contrary | Motion::Static => continue,
                    };
                    ctx.report_pair(
                        out,
                        PREFERENCE.id,
                        ctx.settings.level(name),
                        (a, t.a2.note),
                        (b, t.b2.note),
                        slice.tick,
                        Detail::None,
                    );
                }
            }
        }
    }
}

struct Battuta;

static BATTUTA: RuleInfo = RuleInfo {
    id: "MOT-BATTUTA",
    title_de: "Battuta: Oktave mit fallender, springender Oberstimme erreicht",
    title_en: "Battuta: octave reached with the upper voice leaping down",
    source: "Mann 39; Ch. 61 f.; W23 1.P3, 2.P2*",
    params: &[level("level", "Stufe (zweistimmig)", "level (two voices)")],
};

impl Rule for Battuta {
    fn info(&self) -> &'static RuleInfo {
        &BATTUTA
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() != 2 {
            return;
        }
        let level = ctx.settings.level("level");
        for i in ctx.slice_range().filter(|&i| i > 0) {
            let Some(t) = ctx.analysis.transition(i, 0, 1) else {
                continue;
            };
            if !ctx.analysis.is_downbeat(t.tick) || t.motion() != Motion::Contrary {
                continue;
            }
            let after = t.after();
            if !(after.is_perfect_consonance() && after.simple().steps == 0) {
                continue;
            }
            // The upper voice is the one that comes from above.
            let (upper1, upper2, lower1, lower2) = if t.a1.pitch.midi() >= t.b1.pitch.midi() {
                (t.a1, t.a2, t.b1, t.b2)
            } else {
                (t.b1, t.b2, t.a1, t.a2)
            };
            let upper_step = Interval::between(upper1.pitch, upper2.pitch);
            let lower_step = Interval::between(lower1.pitch, lower2.pitch);
            if upper_step.direction() < 0 && upper_step.is_leap() && lower_step.direction() > 0 {
                ctx.report_pair(
                    out,
                    BATTUTA.id,
                    level,
                    (0, t.a2.note),
                    (1, t.b2.note),
                    t.tick,
                    Detail::Interval(upper_step),
                );
            }
        }
    }
}

struct Antiparallel;

static ANTIPARALLEL: RuleInfo = RuleInfo {
    id: "MOT-ANTIPARALLEL",
    title_de: "Gleiche vollkommene Konsonanz in Gegenbewegung",
    title_en: "Same perfect consonance in contrary motion",
    source: "Korpus (zwei- und vierstimmig nie, dreistimmig 1×)",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for Antiparallel {
    fn info(&self) -> &'static RuleInfo {
        &ANTIPARALLEL
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let level = ctx.settings.level("level");
        let n = ctx.analysis.voice_count();
        for i in ctx.slice_range().filter(|&i| i > 0) {
            for a in 0..n {
                for b in a + 1..n {
                    let Some(t) = ctx.analysis.transition(i, a, b) else {
                        continue;
                    };
                    if t.motion() == Motion::Contrary && same_perfect(t.before(), t.after()) {
                        let after = harmonic(t.a2, t.b2);
                        ctx.report_pair(
                            out,
                            ANTIPARALLEL.id,
                            level,
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
}
