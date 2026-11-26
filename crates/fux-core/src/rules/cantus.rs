//! Rules for the cantus firmus (CF-*). They apply to the voice with the role `CantusFirmus`.

use super::melodic::{flag, int, level};
use super::{Ctx, Detail, ParamInfo, Rule, RuleInfo, Violation};
use crate::interval::Interval;
use crate::lang::Lang;
use crate::mode::Ambitus;
use crate::pitch::Pitch;
use crate::score::{Role, Sounding};

const LEVEL: ParamInfo = level("level", "Stufe", "level");

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(Start),
        Box::new(End),
        Box::new(Penultimate),
        Box::new(Length),
        Box::new(Diatonic),
        Box::new(Range),
        Box::new(AmbitusRule),
        Box::new(Repeat),
        Box::new(Steps),
        Box::new(LeapCost),
        Box::new(Climax),
        Box::new(Variety),
        Box::new(Motif),
        Box::new(Solvable),
    ]
}

/// Index and sounding notes of the cantus firmus, if there is one and its violations count.
fn cantus<'a>(ctx: &'a Ctx) -> Option<(usize, &'a [Sounding])> {
    let v = ctx
        .score
        .voices
        .iter()
        .position(|voice| voice.role == Role::CantusFirmus)?;
    ctx.is_target(v)
        .then(|| (v, ctx.analysis.lines[v].as_slice()))
}

/// The final nearest to a pitch (the cantus starts on the final, so this finds its octave).
fn final_near(ctx: &Ctx, pitch: Pitch) -> Pitch {
    let candidates = (pitch.octave - 1..=pitch.octave + 1).map(|o| ctx.score.mode.final_pitch(o));
    candidates
        .min_by_key(|f| (f.diatonic() - pitch.diatonic()).abs())
        .expect("three candidates")
}

struct Start;

static START: RuleInfo = RuleInfo {
    id: "CF-START",
    title_de: "Cantus beginnt nicht auf der Finalis",
    title_en: "Cantus does not start on the final",
    source: "Korpus",
    params: &[LEVEL],
};

impl Rule for Start {
    fn info(&self) -> &'static RuleInfo {
        &START
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, line)) = cantus(ctx) else { return };
        if let Some(first) = line.first()
            && !first
                .pitch
                .same_class(ctx.score.mode.final_pitch(first.pitch.octave))
        {
            ctx.report(
                out,
                START.id,
                ctx.settings.level("level"),
                v,
                first.note,
                first.start,
                Detail::Pitch(first.pitch),
            );
        }
    }
}

struct End;

static END: RuleInfo = RuleInfo {
    id: "CF-END",
    title_de: "Cantus endet nicht auf der Finalis",
    title_en: "Cantus does not end on the final",
    source: "Korpus",
    params: &[LEVEL],
};

impl Rule for End {
    fn info(&self) -> &'static RuleInfo {
        &END
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, line)) = cantus(ctx) else { return };
        if !ctx.complete {
            return;
        }
        if let Some(last) = line.last()
            && !last
                .pitch
                .same_class(ctx.score.mode.final_pitch(last.pitch.octave))
        {
            ctx.report(
                out,
                END.id,
                ctx.settings.level("level"),
                v,
                last.note,
                last.start,
                Detail::Pitch(last.pitch),
            );
        }
    }
}

struct Penultimate;

static PENULT: RuleInfo = RuleInfo {
    id: "CF-PENULT",
    title_de: "Vorletzter Ton nicht die Sekunde über der Finalis",
    title_en: "Penultimate note is not the second above the final",
    source: "Korpus; W23 S. 28 f.",
    params: &[LEVEL],
};

impl Rule for Penultimate {
    fn info(&self) -> &'static RuleInfo {
        &PENULT
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, line)) = cantus(ctx) else { return };
        if !ctx.complete || line.len() < 2 {
            return;
        }
        let (penultimate, last) = (line[line.len() - 2], line[line.len() - 1]);
        let falls_by_step = Interval::between(penultimate.pitch, last.pitch).steps == -1;
        if ctx.score.mode.degree(penultimate.pitch) != 2 || !falls_by_step {
            ctx.report(
                out,
                PENULT.id,
                ctx.settings.level("level"),
                v,
                penultimate.note,
                last.start,
                Detail::Pitch(penultimate.pitch),
            );
        }
    }
}

struct Length;

static LENGTH: RuleInfo = RuleInfo {
    id: "CF-LENGTH",
    title_de: "Länge des Cantus",
    title_en: "Length of the cantus",
    source: "Impl.; Korpus: 9–14 Töne",
    params: &[
        level("level", "außerhalb von min–max", "outside min–max"),
        level(
            "warning",
            "außerhalb von warn_min–warn_max",
            "outside warn_min–warn_max",
        ),
        int("min", "kleinste Länge", "minimum length"),
        int("max", "größte Länge", "maximum length"),
        int("warn_min", "kleinste übliche Länge", "minimum usual length"),
        int("warn_max", "größte übliche Länge", "maximum usual length"),
    ],
};

impl Rule for Length {
    fn info(&self) -> &'static RuleInfo {
        &LENGTH
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, line)) = cantus(ctx) else { return };
        let n = line.len() as i64;
        let s = ctx.settings;
        let Some(last) = line.last() else { return };
        let over = n > s.int("max");
        if !(ctx.complete || over) {
            return;
        }
        let name = if over || n < s.int("min") {
            "level"
        } else if n < s.int("warn_min") || n > s.int("warn_max") {
            "warning"
        } else {
            return;
        };
        ctx.report(
            out,
            LENGTH.id,
            s.level(name),
            v,
            last.note,
            last.start,
            Detail::Count(n as u32),
        );
    }
}

struct Diatonic;

static DIATONIC: RuleInfo = RuleInfo {
    id: "CF-DIATONIC",
    title_de: "Leiterfremder Ton im Cantus",
    title_en: "Note outside the scale in the cantus",
    source: "Korpus",
    params: &[LEVEL],
};

impl Rule for Diatonic {
    fn info(&self) -> &'static RuleInfo {
        &DIATONIC
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, line)) = cantus(ctx) else { return };
        for s in line
            .iter()
            .filter(|s| !ctx.score.mode.is_scale_pitch(s.pitch))
        {
            ctx.report(
                out,
                DIATONIC.id,
                ctx.settings.level("level"),
                v,
                s.note,
                s.start,
                Detail::Pitch(s.pitch),
            );
        }
    }
}

struct Range;

static RANGE: RuleInfo = RuleInfo {
    id: "CF-RANGE",
    title_de: "Umfang des Cantus zu groß",
    title_en: "Range of the cantus too wide",
    source: "Korpus (alle ≤ Oktave); Impl.",
    params: &[
        level("level", "über max_steps", "beyond max_steps"),
        level("soft", "über soft_max_steps", "beyond soft_max_steps"),
        level("narrow", "unter min_steps", "below min_steps"),
        int(
            "min_steps",
            "kleinster üblicher Umfang in Stufen (4 = Quinte)",
            "smallest usual range in steps (4 = fifth)",
        ),
        int(
            "max_steps",
            "größter Umfang in Stufen (9 = Dezime)",
            "largest range in steps (9 = tenth)",
        ),
        int(
            "soft_max_steps",
            "üblicher Umfang in Stufen (7 = Oktave)",
            "usual range in steps (7 = octave)",
        ),
    ],
};

impl Rule for Range {
    fn info(&self) -> &'static RuleInfo {
        &RANGE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, line)) = cantus(ctx) else { return };
        if ctx.complete
            && let Some(last) = line.last()
        {
            let low = line.iter().map(|s| s.pitch.diatonic()).min().unwrap_or(0);
            let high = line.iter().map(|s| s.pitch.diatonic()).max().unwrap_or(0);
            if i64::from(high - low) < ctx.settings.int("min_steps") {
                ctx.report(
                    out,
                    RANGE.id,
                    ctx.settings.level("narrow"),
                    v,
                    last.note,
                    last.start,
                    Detail::Count((high - low) as u32),
                );
            }
        }
        let limits = [
            ("level", ctx.settings.int("max_steps") as i32),
            ("soft", ctx.settings.int("soft_max_steps") as i32),
        ];
        for (name, limit) in limits {
            let mut range: Option<(i32, i32)> = None;
            for s in line {
                let d = s.pitch.diatonic();
                let before = range.map_or(0, |(low, high)| high - low);
                let (low, high) = range.map_or((d, d), |(low, high)| (low.min(d), high.max(d)));
                range = Some((low, high));
                // Report where the range first exceeds the limit.
                if high - low > limit && before <= limit {
                    ctx.report(
                        out,
                        RANGE.id,
                        ctx.settings.level(name),
                        v,
                        s.note,
                        s.start,
                        Detail::Count((high - low) as u32),
                    );
                    break;
                }
            }
        }
    }
}

struct AmbitusRule;

static AMBITUS: RuleInfo = RuleInfo {
    id: "CF-AMBITUS",
    title_de: "Ton außerhalb des Ambitus des Modus",
    title_en: "Note outside the ambitus of the mode",
    source: "Trad.; § 13 (phrygischer Cantus)",
    params: &[
        LEVEL,
        int(
            "tolerance",
            "erlaubte Stufen über den Ambitus hinaus",
            "steps allowed beyond the ambitus",
        ),
        flag(
            "either",
            "authentischer und plagaler Ambitus zählen beide",
            "authentic and plagal ambitus both count",
        ),
    ],
};

impl Rule for AmbitusRule {
    fn info(&self) -> &'static RuleInfo {
        &AMBITUS
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, line)) = cantus(ctx) else { return };
        let Some(first) = line.first() else { return };
        let final_pitch = final_near(ctx, first.pitch);
        let tolerance = ctx.settings.int("tolerance") as i32;
        let mode = ctx.score.mode;
        let ranges: Vec<(i32, i32)> = if ctx.settings.flag("either") {
            vec![Ambitus::Authentic, Ambitus::Plagal]
        } else {
            vec![mode.ambitus]
        }
        .into_iter()
        .map(|a| {
            let (low, high) = mode.with_ambitus(a).range(final_pitch.octave);
            (low.diatonic() - tolerance, high.diatonic() + tolerance)
        })
        .collect();
        let low = ranges.iter().map(|r| r.0).min().unwrap_or(i32::MIN);
        let high = ranges.iter().map(|r| r.1).max().unwrap_or(i32::MAX);
        for s in line {
            let d = s.pitch.diatonic();
            if d < low || d > high {
                ctx.report(
                    out,
                    AMBITUS.id,
                    ctx.settings.level("level"),
                    v,
                    s.note,
                    s.start,
                    Detail::Pitch(s.pitch),
                );
            }
        }
    }
}

struct Repeat;

static REPEAT: RuleInfo = RuleInfo {
    id: "CF-REPEAT",
    title_de: "Tonwiederholung im Cantus",
    title_en: "Repeated note in the cantus",
    source: "Korpus",
    params: &[LEVEL],
};

impl Rule for Repeat {
    fn info(&self) -> &'static RuleInfo {
        &REPEAT
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, _)) = cantus(ctx) else { return };
        for (_, a, b) in ctx.analysis.melodic_pairs(v) {
            if a.pitch == b.pitch {
                ctx.report(
                    out,
                    REPEAT.id,
                    ctx.settings.level("level"),
                    v,
                    b.note,
                    b.start,
                    Detail::Pitch(b.pitch),
                );
            }
        }
    }
}

struct Steps;

static STEPS: RuleInfo = RuleInfo {
    id: "CF-STEPS",
    title_de: "Zu viele Sprünge im Cantus",
    title_en: "Too many leaps in the cantus",
    source: "Trad.; § 13 (mixolydischer Cantus)",
    params: &[
        LEVEL,
        int(
            "max_leap_percent",
            "höchster Anteil der Sprünge an allen Intervallen in Prozent",
            "highest share of leaps among all intervals, in percent",
        ),
    ],
};

impl Rule for Steps {
    fn info(&self) -> &'static RuleInfo {
        &STEPS
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, line)) = cantus(ctx) else { return };
        if !ctx.complete {
            return;
        }
        let pairs: Vec<_> = ctx.analysis.melodic_pairs(v).collect();
        let leaps = pairs
            .iter()
            .filter(|(_, a, b)| Interval::between(a.pitch, b.pitch).is_leap())
            .count() as i64;
        let Some(last) = line.last() else { return };
        if !pairs.is_empty()
            && leaps * 100 > ctx.settings.int("max_leap_percent") * pairs.len() as i64
        {
            ctx.report(
                out,
                STEPS.id,
                ctx.settings.level("level"),
                v,
                last.note,
                last.start,
                Detail::Count(leaps as u32),
            );
        }
    }
}

struct Climax;

static CLIMAX: RuleInfo = RuleInfo {
    id: "CF-CLIMAX",
    title_de: "Höhepunkt des Cantus nicht einmalig",
    title_en: "Highest note of the cantus not unique",
    source: "Trad.; Korpus (alle Cantus)",
    params: &[
        level("level", "Höhepunkt mehrfach", "highest note more than once"),
        level(
            "at_ends",
            "Höhepunkt am Anfang oder Schluss",
            "highest note at the start or end",
        ),
        level(
            "outside_middle",
            "Höhepunkt außerhalb von middle_from–middle_to",
            "highest note outside middle_from–middle_to",
        ),
        int(
            "middle_from",
            "frühester üblicher Ort des Höhepunkts in Prozent der Länge",
            "earliest usual place of the highest note, in percent of the length",
        ),
        int(
            "middle_to",
            "spätester üblicher Ort des Höhepunkts in Prozent der Länge",
            "latest usual place of the highest note, in percent of the length",
        ),
    ],
};

impl Rule for Climax {
    fn info(&self) -> &'static RuleInfo {
        &CLIMAX
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, line)) = cantus(ctx) else { return };
        if !ctx.complete {
            return;
        }
        let Some(top) = line.iter().map(|s| s.pitch.midi()).max() else {
            return;
        };
        let peaks: Vec<(usize, &Sounding)> = line
            .iter()
            .enumerate()
            .filter(|(_, s)| s.pitch.midi() == top)
            .collect();
        let last = line.len() - 1;
        if peaks.len() > 1 {
            let (_, second) = peaks[1];
            ctx.report(
                out,
                CLIMAX.id,
                ctx.settings.level("level"),
                v,
                second.note,
                second.start,
                Detail::Pitch(second.pitch),
            );
        }
        if let Some((_, s)) = peaks.iter().find(|(i, _)| *i == 0 || *i == last) {
            ctx.report(
                out,
                CLIMAX.id,
                ctx.settings.level("at_ends"),
                v,
                s.note,
                s.start,
                Detail::Pitch(s.pitch),
            );
        }
        let (first, s) = peaks[0];
        let percent = (first * 100 / line.len().max(1)) as i64;
        let middle = ctx.settings.int("middle_from")..=ctx.settings.int("middle_to");
        if first != 0 && first != last && !middle.contains(&percent) {
            ctx.report(
                out,
                CLIMAX.id,
                ctx.settings.level("outside_middle"),
                v,
                s.note,
                s.start,
                Detail::Pitch(s.pitch),
            );
        }
    }
}

struct LeapCost;

static LEAP_COST: RuleInfo = RuleInfo {
    id: "CF-LEAP-COST",
    title_de: "Sprung im Cantus",
    title_en: "Leap in the cantus",
    source: "Korpus (8 Cantus: 20 Terzen, 5 Quarten, 1 Quinte, 1 Oktave, keine Sexte)",
    params: &[
        level("third", "Terz", "third"),
        level("fourth", "Quarte", "fourth"),
        level("fifth", "Quinte", "fifth"),
        level("sixth", "Sexte", "sixth"),
        level("octave", "Oktave", "octave"),
    ],
};

impl Rule for LeapCost {
    fn info(&self) -> &'static RuleInfo {
        &LEAP_COST
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, _)) = cantus(ctx) else { return };
        for (_, a, b) in ctx.analysis.melodic_pairs(v) {
            let interval = Interval::between(a.pitch, b.pitch);
            let name = match interval.abs().steps {
                2 => "third",
                3 => "fourth",
                4 => "fifth",
                5 => "sixth",
                7 => "octave",
                _ => continue,
            };
            ctx.report(
                out,
                LEAP_COST.id,
                ctx.settings.level(name),
                v,
                b.note,
                b.start,
                Detail::Interval(interval),
            );
        }
    }
}

struct Variety;

static VARIETY: RuleInfo = RuleInfo {
    id: "CF-VARIETY",
    title_de: "Zu wenig Abwechslung im Cantus",
    title_en: "Too little variety in the cantus",
    source: "Korpus (Pendelfigur x–y–x einmal in 8 Cantus); Mann 73",
    params: &[
        level(
            "oscillation",
            "Pendelfigur: Ton kehrt nach einem Zwischenton zurück",
            "oscillation: a pitch returns after one other note",
        ),
        level(
            "frequent",
            "ein Ton öfter als max_count-mal (außer der Finalis), je weiteres Vorkommen",
            "a pitch more than max_count times (except the final), per further occurrence",
        ),
        int(
            "max_count",
            "übliche Höchstzahl gleicher Töne",
            "usual highest count of one pitch",
        ),
    ],
};

impl Rule for Variety {
    fn info(&self) -> &'static RuleInfo {
        &VARIETY
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, line)) = cantus(ctx) else { return };
        for w in line.windows(3) {
            if w[0].pitch == w[2].pitch && w[1].pitch != w[0].pitch {
                ctx.report(
                    out,
                    VARIETY.id,
                    ctx.settings.level("oscillation"),
                    v,
                    w[2].note,
                    w[2].start,
                    Detail::Pitches(w[0].pitch, w[1].pitch),
                );
            }
        }
        let max_count = ctx.settings.int("max_count").max(1) as usize;
        let Some(first) = line.first() else { return };
        let final_pitch = final_near(ctx, first.pitch);
        for (i, s) in line.iter().enumerate() {
            if s.pitch == final_pitch {
                continue;
            }
            let count = line[..=i].iter().filter(|o| o.pitch == s.pitch).count();
            if count > max_count {
                ctx.report(
                    out,
                    VARIETY.id,
                    ctx.settings.level("frequent"),
                    v,
                    s.note,
                    s.start,
                    Detail::Count(count as u32),
                );
            }
        }
    }
}

struct Solvable;

static SOLVABLE: RuleInfo = RuleInfo {
    id: "CF-SOLVABLE",
    title_de: "Erzeugter Cantus nicht kontrapunktierbar (1. Gattung darüber und darunter)",
    title_en: "Generated cantus cannot take a first-species counterpoint above and below",
    source: "Impl.",
    params: &[LEVEL],
};

/// Checked by the cantus generator, which runs the counterpoint search; the checker itself has
/// nothing to do.
impl Rule for Solvable {
    fn info(&self) -> &'static RuleInfo {
        &SOLVABLE
    }

    fn check(&self, _: &Ctx, _: &mut Vec<Violation>) {}
}

struct Motif;

static MOTIF: RuleInfo = RuleInfo {
    id: "CF-MOTIF",
    title_de: "Wiederholte Tonfolge im Cantus (monotonia)",
    title_en: "Repeated figure in the cantus (monotonia)",
    source: "Mann 54",
    params: &[
        LEVEL,
        int(
            "literal",
            "kürzeste wörtliche Wiederholung (Töne)",
            "shortest literal repetition (notes)",
        ),
        int(
            "transposed",
            "kürzeste transponierte Wiederholung, also Sequenz (Töne)",
            "shortest transposed repetition, i.e. sequence (notes)",
        ),
    ],
};

impl Rule for Motif {
    fn info(&self) -> &'static RuleInfo {
        &MOTIF
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let Some((v, line)) = cantus(ctx) else { return };
        let literal = ctx.settings.int("literal").max(2) as usize;
        let transposed = ctx.settings.int("transposed").max(2) as usize;
        let pitches: Vec<Pitch> = line.iter().map(|s| s.pitch).collect();
        let steps: Vec<i32> = pitches
            .windows(2)
            .map(|w| w[1].diatonic() - w[0].diatonic())
            .collect();
        for (end, s) in line.iter().enumerate() {
            let found_literal =
                end + 1 >= literal && repeats_earlier(&pitches, end + 1 - literal, literal);
            let found_sequence = end + 1 >= transposed
                && end >= 1
                && repeats_earlier(&steps, end + 1 - transposed, transposed - 1);
            if found_literal || found_sequence {
                ctx.report(
                    out,
                    MOTIF.id,
                    ctx.settings.level("level"),
                    v,
                    s.note,
                    s.start,
                    Detail::Pitch(s.pitch),
                );
            }
        }
    }

    fn message(&self, violation: &Violation, lang: Lang) -> String {
        let title = self.info().title(lang);
        match violation.detail {
            Detail::Pitch(p) => match lang {
                Lang::De => format!("{title}, bis {}", p.name(lang)),
                Lang::En => format!("{title}, up to {}", p.name(lang)),
            },
            _ => title.to_string(),
        }
    }
}

/// Whether `items[start..start + len]` also starts at least two positions earlier. Overlapping
/// counts, so a figure circling with a short period is found; a shift by one position would only
/// find scale runs (or repeated notes), which are no motifs.
pub(super) fn repeats_earlier<T: PartialEq>(items: &[T], start: usize, len: usize) -> bool {
    if len == 0 || start < 2 || start + len > items.len() {
        return false;
    }
    let target = &items[start..start + len];
    (0..=start - 2).any(|i| &items[i..i + len] == target)
}
