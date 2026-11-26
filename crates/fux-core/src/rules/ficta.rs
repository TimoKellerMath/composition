//! Musica ficta (FIC-*): which alterations a counterpoint may use, and where.

use super::analysis::harmonic;
use super::melodic::level;
use super::{Ctx, Detail, Rule, RuleInfo, Violation};
use crate::interval::Interval;
use crate::mode::{Clausula, Mode};
use crate::score::{Role, Sounding};

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(Subsemitonium),
        Box::new(Scope),
        Box::new(InnerCadence),
        Box::new(Bflat),
        Box::new(CrossRelation),
    ]
}

fn counterpoints<'a>(ctx: &'a Ctx<'a>) -> impl Iterator<Item = usize> + 'a {
    (0..ctx.analysis.voice_count()).filter(|&v| ctx.score.voices[v].role != Role::CantusFirmus)
}

/// Whether the pitch is raised against the scale.
fn raised(ctx: &Ctx, note: Sounding) -> bool {
    note.pitch.alter > ctx.score.mode.scale_alter(note.pitch.step)
}

/// Whether the pitch is the lowered "mi" (B♭, transposed E♭).
fn flattened_mi(ctx: &Ctx, note: Sounding) -> bool {
    let mode = ctx.score.mode;
    note.pitch.step == mode.flattened_mi()
        && note.pitch.alter == mode.scale_alter(note.pitch.step) - 1
}

/// Whether a raised note is the third of a major sonority where it begins, as Palestrina raises
/// it at cadences and before them: a major third (or tenth) above the bass, or in the bass with
/// the root a minor sixth above it.
fn third_of_major_sonority(ctx: &Ctx, voice: usize, note: Sounding) -> bool {
    let Some((bass, b)) = ctx.analysis.bass_at(note.start) else {
        return false;
    };
    let major_third = |low: Sounding, high: Sounding| {
        let s = harmonic(low, high).simple();
        s.steps == 2 && s.semitones == 4
    };
    if bass != voice {
        return major_third(b, note);
    }
    ctx.analysis.slice_at(note.start).is_some_and(|slice| {
        slice.sounding.iter().flatten().any(|other| {
            let s = harmonic(note, *other).simple();
            s.steps == 5 && s.semitones == 8
        })
    })
}

/// Whether a note ending at `tick` ends a part of a mass section (phase 14): the next part begins
/// there, at a mark of the score's sections, after every voice has held the part's last chord.
/// Only a joined section has marks.
fn ends_part(ctx: &Ctx, tick: u32) -> bool {
    let per = ctx.analysis.per_measure;
    ctx.score
        .sections
        .iter()
        .any(|mark| u32::try_from(mark.measure).is_ok_and(|m| m.saturating_mul(per) == tick))
}

/// The downbeat by which a raised note has to resolve: the first one after the note began,
/// or for a note held over the barline the first one after it ends.
fn resolve_by(note: Sounding, per_measure: u32) -> u32 {
    note.end.div_ceil(per_measure) * per_measure
}

/// Whether a raised note resolves upwards by step: directly, or by the next downbeat after an
/// ornament (Fux: F♯–D–G, C♯–A–H–C♯–D).
fn resolves_up(line: &[Sounding], i: usize, per_measure: u32) -> bool {
    let note = line[i];
    let by = resolve_by(note, per_measure);
    line[i + 1..].iter().take_while(|n| n.start <= by).any(|n| {
        let step = Interval::between(note.pitch, n.pitch);
        step.is_step() && step.direction() > 0
    })
}

struct Subsemitonium;

static SUBSEMITONIUM: RuleInfo = RuleInfo {
    id: "FIC-SUBSEMITONIUM",
    title_de: "Schlussklausel ohne richtigen Leitton (Subsemitonium)",
    title_en: "Final cadence without the proper leading tone (subsemitonium)",
    source: "Ch. 54; W23 S. 28; Korpus",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for Subsemitonium {
    fn info(&self) -> &'static RuleInfo {
        &SUBSEMITONIUM
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        // In free counterpoint TEX-CLAUSULA finds the cadence with its semitone; the last notes
        // may belong to a coda over the final.
        if !ctx.complete || ctx.score.is_free() {
            return;
        }
        let mode = ctx.score.mode;
        let target = |octave: i8| mode.final_pitch(octave);
        for v in counterpoints(ctx) {
            let line = &ctx.analysis.lines[v];
            let [.., before, last] = line.as_slice() else {
                continue;
            };
            if before.end != last.start || !last.pitch.same_class(target(last.pitch.octave)) {
                continue;
            }
            let step = Interval::between(before.pitch, last.pitch);
            if !(step.is_step() && step.direction() > 0) {
                continue;
            }
            let goal = target(last.pitch.octave);
            let correct = match mode.clausula(goal.step) {
                Some(Clausula::Leading) => step.semitones == 1,
                // Phrygian: the rising voice comes from a whole step below.
                Some(Clausula::Phrygian) => step.semitones == 2,
                None => true,
            };
            if !correct {
                ctx.report(
                    out,
                    SUBSEMITONIUM.id,
                    ctx.settings.level("level"),
                    v,
                    before.note,
                    last.start,
                    Detail::Pitch(before.pitch),
                );
            }
        }
    }
}

struct Scope;

static SCOPE: RuleInfo = RuleInfo {
    id: "FIC-SCOPE",
    title_de: "Unzulässige Alteration",
    title_en: "Alteration not allowed",
    source: "Korpus; Impl.; freier Satz: Korpus Palestrina (das erniedrigte Mi in allen Modi, die große Terz über dem Bass; von 18 093 erhöhten Noten gehen 1 933 abwärts oder springen)",
    params: &[
        level("level", "Stufe", "level"),
        level(
            "free_unresolved",
            "freier Satz: erhöhte Terz eines Durklangs, die nicht aufwärts weitergeht",
            "free counterpoint: raised third of a major sonority that does not go on upwards",
        ),
    ],
};

impl Rule for Scope {
    fn info(&self) -> &'static RuleInfo {
        &SCOPE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let mode = ctx.score.mode;
        let many = ctx.analysis.voice_count() >= 3;
        let per_measure = ctx.analysis.per_measure;
        let free = ctx.score.is_free();
        for v in counterpoints(ctx) {
            let line = &ctx.analysis.lines[v];
            for (i, note) in line.iter().enumerate() {
                if mode.is_scale_pitch(note.pitch) {
                    continue;
                }
                // The tick at which the case is decided, and whether it is allowed.
                let (decided, allowed) =
                    if free && raised(ctx, *note) && third_of_major_sonority(ctx, v, *note) {
                        // It may go on upwards (or stay); otherwise it costs.
                        let Some(next) = line.get(i + 1).filter(|n| n.start == note.end) else {
                            continue;
                        };
                        if Interval::between(note.pitch, next.pitch).steps < 0
                            || Interval::between(note.pitch, next.pitch).steps > 1
                        {
                            ctx.report(
                                out,
                                SCOPE.id,
                                ctx.settings.level("free_unresolved"),
                                v,
                                note.note,
                                next.start,
                                Detail::Pitch(note.pitch),
                            );
                        }
                        continue;
                    } else if raised(ctx, *note) {
                        // From three voices on, the major third of the final may stand in the first
                        // or the last chord (Fig. 184, 185), and so in the last chord of each part
                        // of a mass section, which every voice holds before the next part begins.
                        let outer_chord =
                            note.start == 0 || i + 1 == line.len() || ends_part(ctx, note.end);
                        let picardy = many
                            && outer_chord
                            && note.pitch.same_class(mode.major_third(note.pitch.octave));
                        // A leading tone resolves by the next downbeat, so that note decides.
                        let by = resolve_by(*note, per_measure);
                        match line[i + 1..].iter().find(|n| n.start >= by) {
                            Some(n) => (n.start, picardy || resolves_up(line, i, per_measure)),
                            None if ctx.complete => {
                                (note.start, picardy || resolves_up(line, i, per_measure))
                            }
                            None => continue, // the resolution may still come
                        }
                    } else {
                        // Fux lowers mi in the Dorian and Lydian modes, Palestrina in all.
                        let allowed = flattened_mi(ctx, *note)
                            && (free || matches!(mode.mode, Mode::Dorian | Mode::Lydian));
                        (note.start, allowed)
                    };
                if !allowed {
                    ctx.report(
                        out,
                        SCOPE.id,
                        ctx.settings.level("level"),
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

struct InnerCadence;

static INNER_CADENCE: RuleInfo = RuleInfo {
    id: "FIC-INNER-CADENCE",
    title_de: "Leitton außerhalb der Schlussklausel und der Binnenschlüsse",
    title_en: "Leading tone outside the final cadence and the internal cadences",
    source: "Korpus; freier Satz: Korpus Palestrina",
    params: &[
        level("level", "je Note", "per note"),
        level(
            "free",
            "freier Satz, je Note",
            "free counterpoint, per note",
        ),
    ],
};

impl Rule for InnerCadence {
    fn info(&self) -> &'static RuleInfo {
        &INNER_CADENCE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let per = ctx.analysis.per_measure;
        // A free score has no length given in advance: its end is known in the final check.
        let cadence_from = if ctx.score.is_free() && !ctx.complete {
            u32::MAX
        } else {
            ctx.analysis.total.saturating_sub(2 * per)
        };
        // The leading tone of a marked internal cadence, in its penultimate measure.
        let inner_leading_tone = |note: &Sounding| {
            ctx.analysis.inner_cadences().any(|c| {
                c.clausula == Clausula::Leading
                    && c.goal - per <= note.start
                    && note.start < c.goal
                    && ctx
                        .score
                        .mode
                        .clausula_semitone(c.target_pitch(ctx.score.mode, note.pitch.octave))
                        .is_some_and(|p| p.same_class(note.pitch))
            })
        };
        let free = ctx.score.is_free();
        let name = if free { "free" } else { "level" };
        for v in counterpoints(ctx) {
            let line = &ctx.analysis.lines[v];
            // In free counterpoint a note is decided when it ends; it may have begun before.
            let first = if free {
                line.partition_point(|s| s.end < ctx.window.from)
            } else {
                ctx.first_in_window(line)
            };
            for (i, note) in line.iter().enumerate().skip(first) {
                if !raised(ctx, *note) || note.start >= cadence_from || inner_leading_tone(note) {
                    continue;
                }
                let decided = if free {
                    match own_clausula(ctx, v, line, i) {
                        Some(None) => note.end,
                        Some(Some(tick)) => tick,
                        None => continue,
                    }
                } else {
                    note.start
                };
                ctx.report(
                    out,
                    INNER_CADENCE.id,
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

/// Whether the raised note `line[i]` of voice `v` is the leading tone of a clausula of its own,
/// as at most of Palestrina's cadences, which no one marks: it rises by a semitone into a
/// downbeat, on which another voice reaches the same step by step from above (tenorizans) or by
/// a fourth or fifth (basizans). `None` if so or not known yet; otherwise the tick at which it
/// fails (`Some(None)`: where the note ends, before a rest).
fn own_clausula(ctx: &Ctx, v: usize, line: &[Sounding], i: usize) -> Option<Option<u32>> {
    let note = line[i];
    let next = match line.get(i + 1) {
        Some(next) if next.start == note.end => *next,
        Some(_) => return Some(None),
        None if ctx.complete || ctx.analysis.ends[v] > note.end => return Some(None),
        None => return None,
    };
    let step = Interval::between(note.pitch, next.pitch);
    if (step.steps, step.semitones) != (1, 1) || next.start % ctx.analysis.per_measure != 0 {
        return Some(Some(next.start));
    }
    if !ctx.analysis.complete_at(next.start) {
        return None;
    }
    let partner = ctx.analysis.lines.iter().enumerate().any(|(w, other)| {
        let j = other.partition_point(|s| s.start < next.start);
        w != v
            && j > 0
            && other.get(j).is_some_and(|arrival| {
                let before = other[j - 1];
                let came = Interval::between(before.pitch, arrival.pitch).steps;
                arrival.start == next.start
                    && before.end == next.start
                    && arrival.pitch.same_class(next.pitch)
                    && (came == -1 || matches!(came.abs(), 3 | 4))
            })
    });
    (!partner).then_some(Some(next.start))
}

struct Bflat;

static BFLAT: RuleInfo = RuleInfo {
    id: "FIC-BFLAT",
    title_de: "B statt H gegen den Tritonus",
    title_en: "B flat instead of B against the tritone",
    source: "Korpus (dorisch und lydisch); freier Satz: Korpus Palestrina (in allen Modi)",
    params: &[
        level("level", "je Note", "per note"),
        level(
            "free",
            "freier Satz, je Note",
            "free counterpoint, per note",
        ),
    ],
};

impl Rule for Bflat {
    fn info(&self) -> &'static RuleInfo {
        &BFLAT
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let name = if ctx.score.is_free() { "free" } else { "level" };
        for v in counterpoints(ctx) {
            let line = &ctx.analysis.lines[v];
            for note in &line[ctx.first_in_window(line)..] {
                if flattened_mi(ctx, *note) {
                    ctx.report(
                        out,
                        BFLAT.id,
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

struct CrossRelation;

static CROSS_RELATION: RuleInfo = RuleInfo {
    id: "FIC-CROSS-RELATION",
    title_de: "Querstand",
    title_en: "Cross relation",
    source: "Trad.",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for CrossRelation {
    fn info(&self) -> &'static RuleInfo {
        &CROSS_RELATION
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let level = ctx.settings.level("level");
        let slices = &ctx.analysis.slices;
        for i in ctx.slice_range().filter(|&i| i > 0) {
            let (before, now) = (&slices[i - 1], &slices[i]);
            for (b, y) in now.sounding.iter().enumerate() {
                let Some(y) = *y else { continue };
                if y.start != now.tick {
                    continue;
                }
                for (a, x) in before.sounding.iter().enumerate() {
                    let Some(x) = *x else { continue };
                    if a != b && x.pitch.step == y.pitch.step && x.pitch.alter != y.pitch.alter {
                        ctx.report_pair(
                            out,
                            CROSS_RELATION.id,
                            level,
                            (b, y.note),
                            (a, x.note),
                            now.tick,
                            Detail::Pitches(x.pitch, y.pitch),
                        );
                    }
                }
            }
        }
    }
}
