//! Rules about the voices relative to each other (VOI-*; VOI-RANGE is in `melodic`).

use super::melodic::level;
use super::species::held_part;
use super::{Ctx, Detail, Rule, RuleInfo, Violation};

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    vec![Box::new(Crossing), Box::new(Spacing), Box::new(Overlap)]
}

struct Crossing;

static CROSSING: RuleInfo = RuleInfo {
    id: "VOI-CROSSING",
    title_de: "Stimmkreuzung",
    title_en: "Voice crossing",
    source: "Mann 100; Korpus; W23 S. 64",
    params: &[
        level("level", "zwei- und dreistimmig", "two and three voices"),
        level("four_voices", "ab vier Stimmen", "four or more voices"),
    ],
};

impl Rule for Crossing {
    fn info(&self) -> &'static RuleInfo {
        &CROSSING
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let n = ctx.analysis.voice_count();
        let level = ctx
            .settings
            .level(if n >= 4 { "four_voices" } else { "level" });
        for slice in &ctx.analysis.slices[ctx.slice_range()] {
            for a in 0..n {
                for b in a + 1..n {
                    let (Some(x), Some(y)) = (slice.sounding[a], slice.sounding[b]) else {
                        continue;
                    };
                    let attacked = x.start == slice.tick || y.start == slice.tick;
                    if attacked && x.pitch.midi() < y.pitch.midi() {
                        ctx.report_pair(
                            out,
                            CROSSING.id,
                            level,
                            (a, held_part(ctx, a, x, slice.tick)),
                            (b, held_part(ctx, b, y, slice.tick)),
                            slice.tick,
                            Detail::Pitches(x.pitch, y.pitch),
                        );
                    }
                }
            }
        }
    }
}

struct Spacing;

static SPACING: RuleInfo = RuleInfo {
    id: "VOI-SPACING",
    title_de: "Zu großer Abstand zwischen Stimmen",
    title_en: "Voices too far apart",
    source: "Trad.; Korpus",
    params: &[
        level(
            "wide",
            "zweistimmig: Undezime oder Duodezime",
            "two voices: eleventh or twelfth",
        ),
        level(
            "too_wide",
            "zweistimmig: über die Duodezime",
            "two voices: beyond the twelfth",
        ),
        level(
            "upper",
            "benachbarte Oberstimmen über eine Oktave",
            "adjacent upper voices beyond an octave",
        ),
        level(
            "bass",
            "Bass zur nächsten Stimme über die Duodezime",
            "bass to the next voice beyond a twelfth",
        ),
    ],
};

impl Rule for Spacing {
    fn info(&self) -> &'static RuleInfo {
        &SPACING
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let two = ctx.analysis.voice_count() == 2;
        for i in ctx.downbeats() {
            let slice = &ctx.analysis.slices[i];
            if !ctx.analysis.complete_at(slice.tick) {
                continue; // neighbours may still come
            }
            let mut sounding: Vec<(usize, crate::score::Sounding)> = slice
                .sounding
                .iter()
                .enumerate()
                .filter_map(|(v, s)| s.map(|s| (v, s)))
                .collect();
            if sounding.len() < 2 {
                continue;
            }
            sounding.sort_by_key(|(_, s)| (s.pitch.diatonic(), s.pitch.midi()));
            for (k, pair) in sounding.windows(2).enumerate() {
                let ((a, x), (b, y)) = (pair[0], pair[1]);
                if x.start != slice.tick && y.start != slice.tick {
                    continue;
                }
                let steps = y.pitch.diatonic() - x.pitch.diatonic();
                let name = if two {
                    match steps {
                        s if s > 11 => "too_wide",
                        s if s > 9 => "wide",
                        _ => continue,
                    }
                } else if k == 0 {
                    if steps > 11 { "bass" } else { continue }
                } else if steps > 7 {
                    "upper"
                } else {
                    continue;
                };
                ctx.report_pair(
                    out,
                    SPACING.id,
                    ctx.settings.level(name),
                    (b, held_part(ctx, b, y, slice.tick)),
                    (a, held_part(ctx, a, x, slice.tick)),
                    slice.tick,
                    Detail::Pitches(x.pitch, y.pitch),
                );
            }
        }
    }
}

struct Overlap;

static OVERLAP: RuleInfo = RuleInfo {
    id: "VOI-OVERLAP",
    title_de: "Stimmüberschreitung",
    title_en: "Voice overlap",
    source: "Trad.",
    params: &[level("level", "Stufe", "level")],
};

impl Rule for Overlap {
    fn info(&self) -> &'static RuleInfo {
        &OVERLAP
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let level = ctx.settings.level("level");
        let n = ctx.analysis.voice_count();
        for i in ctx.slice_range().filter(|&i| i > 0) {
            for upper in 0..n.saturating_sub(1) {
                let lower = upper + 1;
                let Some(t) = ctx.analysis.transition(i, upper, lower) else {
                    continue;
                };
                let upper_moves_below =
                    t.a2.start == t.tick && t.a2.pitch.midi() < t.b1.pitch.midi();
                let lower_moves_above =
                    t.b2.start == t.tick && t.b2.pitch.midi() > t.a1.pitch.midi();
                if upper_moves_below || lower_moves_above {
                    ctx.report_pair(
                        out,
                        OVERLAP.id,
                        level,
                        (upper, t.a2.note),
                        (lower, t.b2.note),
                        t.tick,
                        Detail::Pitches(t.a2.pitch, t.b2.pitch),
                    );
                }
            }
        }
    }
}
