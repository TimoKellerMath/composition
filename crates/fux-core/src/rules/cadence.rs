//! Beginning and end of a two-part piece (CAD-*). From three voices on, the TEX-* rules apply.

use super::analysis::{Motion, harmonic, species};
use super::melodic::level;
use super::species::held_part;
use super::{Ctx, Detail, Rule, RuleInfo, Violation};
use crate::interval::Interval;
use crate::score::{Role, Sounding, Species};

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(Start),
        Box::new(End),
        Box::new(Penultimate),
        Box::new(Penultimate2),
        Box::new(Penultimate3),
        Box::new(Penultimate4 {
            info: &PENULT_G4,
            species: Species::Fourth,
        }),
        Box::new(Penultimate4 {
            info: &PENULT_G5,
            species: Species::Fifth,
        }),
    ]
}

/// In two parts: the cantus firmus and the counterpoint voice.
fn two_parts(ctx: &Ctx) -> Option<(usize, usize)> {
    if ctx.analysis.voice_count() != 2 {
        return None;
    }
    let cf = ctx
        .score
        .voices
        .iter()
        .position(|v| v.role == Role::CantusFirmus)?;
    Some((cf, 1 - cf))
}

/// How the cantus firmus enters its last note: by step from above (the tenor's clausula, as in
/// all of Fux's cantus), by step from below, or by leap. A repeated note counts as the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CantusInto {
    Falls,
    Rises,
    Leaps,
}

fn cantus_into_final(ctx: &Ctx, cf: usize) -> Option<CantusInto> {
    let [.., before, last] = ctx.analysis.lines[cf].as_slice() else {
        return None;
    };
    let step = Interval::between(before.pitch, last.pitch);
    Some(match (step.is_step(), step.direction()) {
        (true, d) if d > 0 => CantusInto::Rises,
        (false, d) if d != 0 => CantusInto::Leaps,
        _ => CantusInto::Falls,
    })
}

/// Whether the cantus falls into its last note, the case the formulas of the species describe.
fn cantus_falls(ctx: &Ctx, cf: usize) -> bool {
    cantus_into_final(ctx, cf).is_none_or(|into| into == CantusInto::Falls)
}

/// Start of the penultimate measure.
fn penultimate_measure(ctx: &Ctx) -> u32 {
    ctx.analysis
        .total
        .saturating_sub(2 * ctx.analysis.per_measure)
}

/// The notes of the cantus firmus and the counterpoint sounding at `tick`.
fn pair_at(ctx: &Ctx, cf: usize, cp: usize, tick: u32) -> Option<(Sounding, Sounding)> {
    let slice = ctx.analysis.slice_at(tick)?;
    Some((slice.sounding[cf]?, slice.sounding[cp]?))
}

/// The lower and the upper of two sounding notes.
fn ordered(x: Sounding, y: Sounding) -> (Sounding, Sounding) {
    if (x.pitch.diatonic(), x.pitch.midi()) <= (y.pitch.diatonic(), y.pitch.midi()) {
        (x, y)
    } else {
        (y, x)
    }
}

/// The first slice in which both voices sound.
fn first_full(ctx: &Ctx) -> Option<usize> {
    ctx.analysis
        .slices
        .iter()
        .position(|s| s.sounding.iter().all(Option::is_some))
}

struct Start;

static START: RuleInfo = RuleInfo {
    id: "CAD-START",
    title_de: "Erster Klang keine vollkommene Konsonanz über der Finalis",
    title_en: "First sonority is not a perfect consonance over the final",
    source: "Ch. 54, 56; W23 1.H2, 1.H4",
    params: &[level("level", "Stufe (zweistimmig)", "level (two voices)")],
};

impl Rule for Start {
    fn info(&self) -> &'static RuleInfo {
        &START
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() != 2 {
            return;
        }
        let Some(i) = first_full(ctx) else { return };
        let slice = &ctx.analysis.slices[i];
        let (Some(x), Some(y)) = (slice.sounding[0], slice.sounding[1]) else {
            return;
        };
        let (lower, _) = ordered(x, y);
        let interval = harmonic(x, y);
        let on_final = lower
            .pitch
            .same_class(ctx.score.mode.final_pitch(lower.pitch.octave));
        if !interval.is_perfect_consonance() || !on_final {
            ctx.report_pair(
                out,
                START.id,
                ctx.settings.level("level"),
                (0, x.note),
                (1, y.note),
                slice.tick,
                Detail::Interval(interval),
            );
        }
    }
}

struct End;

static END: RuleInfo = RuleInfo {
    id: "CAD-END",
    title_de: "Schlussklang nicht Einklang oder Oktave über der Finalis",
    title_en: "Final sonority is not a unison or octave over the final",
    source: "Ch. 54; W23 1.H3, 1.H4",
    params: &[level("level", "Stufe (zweistimmig)", "level (two voices)")],
};

impl Rule for End {
    fn info(&self) -> &'static RuleInfo {
        &END
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if ctx.analysis.voice_count() != 2 || !ctx.complete {
            return;
        }
        let Some(slice) = ctx.analysis.slices.last() else {
            return;
        };
        let (Some(x), Some(y)) = (slice.sounding[0], slice.sounding[1]) else {
            return;
        };
        let (lower, _) = ordered(x, y);
        let interval = harmonic(x, y);
        let octave = interval.is_perfect_consonance() && interval.simple().steps == 0;
        let on_final = lower
            .pitch
            .same_class(ctx.score.mode.final_pitch(lower.pitch.octave));
        if !octave || !on_final {
            ctx.report_pair(
                out,
                END.id,
                ctx.settings.level("level"),
                (0, x.note),
                (1, y.note),
                slice.tick,
                Detail::Interval(interval),
            );
        }
    }
}

struct Penultimate;

static PENULT: RuleInfo = RuleInfo {
    id: "CAD-PENULT",
    title_de: "Klausel: vorletzter Klang nicht große Sexte (Cantus unten) bzw. kleine Terz (Cantus oben), oder Schluss nicht schrittweise in Gegenbewegung",
    title_en: "Cadence: penultimate sonority not a major sixth (cantus below) or minor third (cantus above), or no stepwise contrary motion into the end",
    source: "Ch. 54; W23 1.H7, 1.H8",
    params: &[level("level", "Stufe (zweistimmig)", "level (two voices)")],
};

impl Rule for Penultimate {
    fn info(&self) -> &'static RuleInfo {
        &PENULT
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((cf, cp)) = two_parts(ctx) else {
            return;
        };
        let last_measure = ctx.analysis.total.saturating_sub(ctx.analysis.per_measure);
        let slices = &ctx.analysis.slices;
        // The last sonority before the final measure is known before the end.
        let Some(i) = slices.iter().rposition(|s| s.tick < last_measure) else {
            return;
        };
        let (Some(cf_note), Some(other)) = (slices[i].sounding[cf], slices[i].sounding[cp]) else {
            return;
        };
        if other.end != last_measure && !ctx.complete {
            return; // the counterpoint's penultimate note is still missing
        }
        let interval = harmonic(cf_note, other).simple();
        // The counterpoint takes the clausula the cantus leaves: the discant's (major sixth above,
        // minor third below) when the cantus falls, the tenor's when it rises. Over a leaping
        // cantus it brings the leading tone.
        let cf_below = cf_note.pitch.midi() < other.pitch.midi();
        let into = cantus_into_final(ctx, cf).unwrap_or(CantusInto::Falls);
        let fits = match into {
            CantusInto::Falls if cf_below => interval == Interval::MAJOR_SIXTH,
            CantusInto::Falls => interval == Interval::MINOR_THIRD,
            CantusInto::Rises if cf_below => interval == Interval::MINOR_THIRD,
            CantusInto::Rises => interval == Interval::MAJOR_SIXTH,
            CantusInto::Leaps => ctx
                .score
                .mode
                .clausula_semitone(ctx.score.mode.final_pitch(other.pitch.octave))
                .is_some_and(|p| p.same_class(other.pitch) && interval.is_imperfect_consonance()),
        };
        let level = ctx.settings.level("level");
        if !fits {
            ctx.report_pair(
                out,
                PENULT.id,
                level,
                (cp, other.note),
                (cf, cf_note.note),
                cf_note.start.max(other.start),
                Detail::Interval(interval),
            );
            return;
        }
        if !ctx.complete {
            return;
        }
        let Some(t) = ctx.analysis.transition(i + 1, 0, 1) else {
            return;
        };
        let step = |a: Sounding, b: Sounding| Interval::between(a.pitch, b.pitch).is_step();
        let (cf_moves, cp_moves) = if cf == 0 {
            (step(t.a1, t.a2), step(t.b1, t.b2))
        } else {
            (step(t.b1, t.b2), step(t.a1, t.a2))
        };
        let steps = cp_moves && (cf_moves || into == CantusInto::Leaps);
        if t.motion() != Motion::Contrary || !steps {
            ctx.report_pair(
                out,
                PENULT.id,
                level,
                (0, t.a1.note),
                (1, t.b1.note),
                t.tick,
                Detail::Interval(interval),
            );
        }
    }
}

struct Penultimate2;

static PENULT_G2: RuleInfo = RuleInfo {
    id: "CAD-PENULT-G2",
    title_de: "2. Gattung, vorletzter Takt: Thesis keine Quinte",
    title_en: "Second species, penultimate measure: downbeat is not a fifth",
    source: "Ch. 64 f., 69; W23 2.H3, 2.H4; Korpus (Quinte 11×, Sexte 1× phrygisch unten)",
    params: &[
        level(
            "level",
            "weder Quinte noch Sexte",
            "neither fifth nor sixth",
        ),
        level("sixth", "Sexte", "sixth"),
    ],
};

impl Rule for Penultimate2 {
    fn info(&self) -> &'static RuleInfo {
        &PENULT_G2
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((cf, cp)) = two_parts(ctx) else {
            return;
        };
        if species(ctx.score, cp) != Some(Species::Second) || !cantus_falls(ctx, cf) {
            return;
        }
        let tick = penultimate_measure(ctx);
        let Some((cf_note, note)) = pair_at(ctx, cf, cp, tick) else {
            return;
        };
        let interval = harmonic(cf_note, note).simple();
        let name = match interval.steps {
            4 if interval.is_perfect_consonance() => return,
            5 if interval.is_imperfect_consonance() => "sixth",
            _ => "level",
        };
        ctx.report(
            out,
            PENULT_G2.id,
            ctx.settings.level(name),
            cp,
            note.note,
            note.start.max(tick),
            Detail::Interval(interval),
        );
    }
}

struct Penultimate3;

static PENULT_G3: RuleInfo = RuleInfo {
    id: "CAD-PENULT-G3",
    title_de: "3. Gattung, vorletzter Takt: Liegt der Cantus oben, ist das 1. Viertel keine kleine Terz",
    title_en: "Third species, penultimate measure: with the cantus above, the first quarter is not a minor third",
    source: "Ch. 75; Mann 54; W23 3.H4; Korpus (Fux weicht ab: Fig. 58)",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for Penultimate3 {
    fn info(&self) -> &'static RuleInfo {
        &PENULT_G3
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((cf, cp)) = two_parts(ctx) else {
            return;
        };
        if species(ctx.score, cp) != Some(Species::Third) || !cantus_falls(ctx, cf) {
            return;
        }
        let tick = penultimate_measure(ctx);
        let Some((cf_note, note)) = pair_at(ctx, cf, cp, tick) else {
            return;
        };
        let interval = harmonic(cf_note, note).simple();
        if cf_note.pitch.midi() > note.pitch.midi() && interval != Interval::MINOR_THIRD {
            ctx.report(
                out,
                PENULT_G3.id,
                ctx.settings.level("level"),
                cp,
                note.note,
                note.start.max(tick),
                Detail::Interval(interval),
            );
        }
    }
}

/// The penultimate measure of the fourth and fifth species: a syncopation that resolves.
struct Penultimate4 {
    info: &'static RuleInfo,
    species: Species,
}

static PENULT_G4: RuleInfo = RuleInfo {
    id: "CAD-PENULT-G4",
    title_de: "4. Gattung, vorletzter Takt: keine Septime (Cantus unten) bzw. Sekunde (Cantus oben) als Synkope",
    title_en: "Fourth species, penultimate measure: no seventh (cantus below) or second (cantus above) as the syncopation",
    source: "W23 4.H3, 4.H4; Korpus (alle zweistimmigen Beispiele)",
    params: &[level("level", "Stufe", "level")],
};

static PENULT_G5: RuleInfo = RuleInfo {
    id: "CAD-PENULT-G5",
    title_de: "5. Gattung, vorletzter Takt: keine Septime (Cantus unten) bzw. Sekunde (Cantus oben) als Synkope",
    title_en: "Fifth species, penultimate measure: no seventh (cantus below) or second (cantus above) as the syncopation",
    source: "Korpus (alle zweistimmigen Beispiele: Takt ⁀2 2); W23 5.R6",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for Penultimate4 {
    fn info(&self) -> &'static RuleInfo {
        self.info
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((cf, cp)) = two_parts(ctx) else {
            return;
        };
        if species(ctx.score, cp) != Some(self.species) || !cantus_falls(ctx, cf) {
            return;
        }
        let tick = penultimate_measure(ctx);
        let Some((cf_note, note)) = pair_at(ctx, cf, cp, tick) else {
            return;
        };
        let interval = harmonic(cf_note, note).simple();
        let wanted = if cf_note.pitch.midi() < note.pitch.midi() {
            6
        } else {
            1
        };
        if note.start >= tick || interval.steps != wanted {
            ctx.report(
                out,
                self.info.id,
                ctx.settings.level("level"),
                cp,
                held_part(ctx, cp, note, tick),
                tick,
                Detail::Interval(interval),
            );
        }
    }
}
