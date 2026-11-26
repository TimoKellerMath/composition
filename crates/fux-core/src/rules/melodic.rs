//! Melodic rules (MEL-*) and the voice range (VOI-RANGE).

use super::analysis::species;
use super::{Ctx, Detail, Level, ParamInfo, ParamKind, Rule, RuleInfo, Violation};
use crate::interval::Interval;
use crate::lang::Lang;
use crate::score::{Role, Sounding, Species};
use crate::time::Duration;

pub(super) const fn level(name: &'static str, de: &'static str, en: &'static str) -> ParamInfo {
    ParamInfo {
        name,
        kind: ParamKind::Level,
        de,
        en,
    }
}

pub(super) const fn int(name: &'static str, de: &'static str, en: &'static str) -> ParamInfo {
    ParamInfo {
        name,
        kind: ParamKind::Int,
        de,
        en,
    }
}

pub(super) const fn flag(name: &'static str, de: &'static str, en: &'static str) -> ParamInfo {
    ParamInfo {
        name,
        kind: ParamKind::Bool,
        de,
        en,
    }
}

const LEVEL: ParamInfo = level("level", "Stufe", "level");

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(Tritone),
        Box::new(AugDim),
        Box::new(MaxLeap),
        Box::new(Octave),
        Box::new(MinorSixthDown),
        Box::new(LeapCost),
        Box::new(LeapRecovery),
        Box::new(ConsecutiveLeaps),
        Box::new(TritoneOutline),
        Box::new(Repeat),
        Box::new(Variety),
        Box::new(Motif),
        Box::new(Climax),
        Box::new(VoiceRange),
    ]
}

fn is_counterpoint(ctx: &Ctx, voice: usize) -> bool {
    !matches!(ctx.score.voices[voice].role, Role::CantusFirmus)
}

/// The stricter of two levels: off below any cost, a cost below a higher one, forbidden above
/// all.
fn stricter(a: Level, b: Level) -> Level {
    let rank = |level: Level| match level {
        Level::Off => (0, 0),
        Level::Cost(cost) => (1, cost),
        Level::Forbidden => (2, 0),
    };
    if rank(b) > rank(a) { b } else { a }
}

/// Reports a violation at the second note of a melodic pair.
fn report_at(
    ctx: &Ctx,
    out: &mut Vec<Violation>,
    id: &'static str,
    level: Level,
    voice: usize,
    note: &Sounding,
    detail: Detail,
) {
    ctx.report(out, id, level, voice, note.note, note.start, detail);
}

struct Tritone;

static TRITONE: RuleInfo = RuleInfo {
    id: "MEL-TRITONE",
    title_de: "Tritonus als melodisches Intervall",
    title_en: "Melodic tritone",
    source: "Ch. 59; Mann 35; W23 1.M1",
    params: &[LEVEL],
};

impl Rule for Tritone {
    fn info(&self) -> &'static RuleInfo {
        &TRITONE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let level = ctx.settings.level("level");
        for v in 0..ctx.analysis.voice_count() {
            for (_, a, b) in ctx.melodic_pairs(v) {
                if Interval::between(a.pitch, b.pitch).is_tritone() {
                    report_at(
                        ctx,
                        out,
                        TRITONE.id,
                        level,
                        v,
                        &b,
                        Detail::Pitches(a.pitch, b.pitch),
                    );
                }
            }
        }
    }
}

struct AugDim;

static AUG_DIM: RuleInfo = RuleInfo {
    id: "MEL-AUG-DIM",
    title_de: "Übermäßiges oder vermindertes melodisches Intervall",
    title_en: "Augmented or diminished melodic interval",
    source: "W23 G6*; L25; Korpus",
    params: &[
        LEVEL,
        level(
            "chromatic_two_voices",
            "chromatischer Halbton, zweistimmig",
            "chromatic semitone, two voices",
        ),
        level(
            "chromatic",
            "chromatischer Halbton, ab drei Stimmen",
            "chromatic semitone, three or more voices",
        ),
    ],
};

impl Rule for AugDim {
    fn info(&self) -> &'static RuleInfo {
        &AUG_DIM
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let two_voices = ctx.analysis.voice_count() <= 2;
        for v in 0..ctx.analysis.voice_count() {
            for (_, a, b) in ctx.melodic_pairs(v) {
                let interval = Interval::between(a.pitch, b.pitch);
                if interval.is_regular() || interval.is_tritone() {
                    continue;
                }
                let level = if interval.abs().steps == 0 {
                    ctx.settings.level(if two_voices {
                        "chromatic_two_voices"
                    } else {
                        "chromatic"
                    })
                } else {
                    ctx.settings.level("level")
                };
                report_at(
                    ctx,
                    out,
                    AUG_DIM.id,
                    level,
                    v,
                    &b,
                    Detail::Interval(interval),
                );
            }
        }
    }
}

struct MaxLeap;

static MAX_LEAP: RuleInfo = RuleInfo {
    id: "MEL-MAX-LEAP",
    title_de: "Zu großer Sprung (große Sexte, Septime, über die Oktave)",
    title_en: "Leap too large (major sixth, seventh, beyond the octave)",
    source: "Mann 37, 79; Ch. 61; W23 1.M2; Korpus",
    params: &[LEVEL],
};

impl Rule for MaxLeap {
    fn info(&self) -> &'static RuleInfo {
        &MAX_LEAP
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let level = ctx.settings.level("level");
        for v in 0..ctx.analysis.voice_count() {
            for (_, a, b) in ctx.melodic_pairs(v) {
                let interval = Interval::between(a.pitch, b.pitch);
                let up = interval.abs();
                let too_large = interval.is_regular()
                    && ((up.steps == 5 && up.semitones == 9) || up.steps == 6 || up.steps >= 8);
                if too_large {
                    report_at(
                        ctx,
                        out,
                        MAX_LEAP.id,
                        level,
                        v,
                        &b,
                        Detail::Interval(interval),
                    );
                }
            }
        }
    }
}

struct Octave;

static OCTAVE: RuleInfo = RuleInfo {
    id: "MEL-OCTAVE",
    title_de: "Oktavsprung",
    title_en: "Octave leap",
    source: "Mann 45, 115; Ch. 67 f.; W23 1.M2, 2.M1; Korpus",
    params: &[
        level("cantus", "im Cantus firmus", "in the cantus firmus"),
        level(
            "g1_two_voices",
            "1. Gattung, zweistimmig",
            "first species, two voices",
        ),
        level(
            "g2_wide",
            "2. Gattung, zweistimmig, Thesis→Arsis bei mehr als einer Terz Abstand",
            "second species, two voices, downbeat to upbeat with the voices more than a third apart",
        ),
        level("default", "alle übrigen Fälle", "all other cases"),
    ],
};

impl Rule for Octave {
    fn info(&self) -> &'static RuleInfo {
        &OCTAVE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let two_voices = ctx.analysis.voice_count() == 2;
        let per_measure = ctx.analysis.per_measure;
        for v in 0..ctx.analysis.voice_count() {
            for (_, a, b) in ctx.melodic_pairs(v) {
                let up = Interval::between(a.pitch, b.pitch).abs();
                if !(up.steps == 7 && up.semitones == 12) {
                    continue;
                }
                let name = match species(ctx.score, v) {
                    None => "cantus",
                    Some(Species::First) if two_voices => "g1_two_voices",
                    Some(Species::Second) if two_voices && a.start % per_measure == 0 => {
                        let other = 1 - v;
                        let close = ctx.analysis.lines[other]
                            .iter()
                            .find(|s| s.start <= a.start && a.start < s.end)
                            .is_some_and(|s| (s.pitch.diatonic() - a.pitch.diatonic()).abs() <= 2);
                        if close { "default" } else { "g2_wide" }
                    }
                    Some(_) => "default",
                };
                report_at(
                    ctx,
                    out,
                    OCTAVE.id,
                    ctx.settings.level(name),
                    v,
                    &b,
                    Detail::Pitches(a.pitch, b.pitch),
                );
            }
        }
    }
}

struct MinorSixthDown;

static MIN6_DOWN: RuleInfo = RuleInfo {
    id: "MEL-MIN6-DOWN",
    title_de: "Kleine Sexte abwärts",
    title_en: "Minor sixth downwards",
    source: "Korpus; Trad. (Jeppesen); Korpus Palestrina (7×)",
    params: &[LEVEL, level("free", "freier Satz", "free counterpoint")],
};

impl Rule for MinorSixthDown {
    fn info(&self) -> &'static RuleInfo {
        &MIN6_DOWN
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for v in 0..ctx.analysis.voice_count() {
            let level = ctx
                .settings
                .level(if species(ctx.score, v) == Some(Species::Free) {
                    "free"
                } else {
                    "level"
                });
            for (_, a, b) in ctx.melodic_pairs(v) {
                if Interval::between(a.pitch, b.pitch) == -Interval::MINOR_SIXTH {
                    report_at(
                        ctx,
                        out,
                        MIN6_DOWN.id,
                        level,
                        v,
                        &b,
                        Detail::Pitches(a.pitch, b.pitch),
                    );
                }
            }
        }
    }
}

struct LeapCost;

static LEAP_COST: RuleInfo = RuleInfo {
    id: "MEL-LEAP-COST",
    title_de: "Sprung statt Schritt",
    title_en: "Leap instead of a step",
    source: "Mann 73; W23 G7*; freier Satz: Korpus Palestrina (von allen Tonschritten Oberstimme: Terz 11,4 %, Quarte 4,0 %, Quinte 1,8 %, Sexte 0,2 %, Oktave 0,4 %; Mittelstimmen 12,6 / 5,3 / 3,5 / 0,2 / 1,0 %; Bass 11,5 / 9,7 / 10,0 / 0,1 / 1,3 %)",
    params: &[
        level("third", "Terz", "third"),
        level("fourth", "Quarte", "fourth"),
        level("fifth", "Quinte", "fifth"),
        level("sixth", "Sexte", "sixth"),
        level("octave", "Oktave", "octave"),
        level(
            "free_third",
            "freier Satz: Terz",
            "free counterpoint: third",
        ),
        level(
            "free_fourth",
            "freier Satz: Quarte in einer Oberstimme",
            "free counterpoint: fourth in an upper voice",
        ),
        level(
            "free_fifth",
            "freier Satz: Quinte in einer Oberstimme",
            "free counterpoint: fifth in an upper voice",
        ),
        level(
            "free_sixth",
            "freier Satz: Sexte",
            "free counterpoint: sixth",
        ),
        level(
            "free_octave",
            "freier Satz: Oktave in einer Oberstimme",
            "free counterpoint: octave in an upper voice",
        ),
        level(
            "free_bass_fourth",
            "freier Satz: Quarte im Bass",
            "free counterpoint: fourth in the bass",
        ),
        level(
            "free_bass_fifth",
            "freier Satz: Quinte im Bass",
            "free counterpoint: fifth in the bass",
        ),
        level(
            "free_bass_octave",
            "freier Satz: Oktave im Bass",
            "free counterpoint: octave in the bass",
        ),
    ],
};

impl Rule for LeapCost {
    fn info(&self) -> &'static RuleInfo {
        &LEAP_COST
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let lowest = ctx.analysis.voice_count().saturating_sub(1);
        for v in (0..ctx.analysis.voice_count()).filter(|&v| is_counterpoint(ctx, v)) {
            // Palestrina leaps far more than Fux allows, the bass most of all.
            let free = species(ctx.score, v) == Some(Species::Free);
            let bass = free && v == lowest && v > 0;
            for (_, a, b) in ctx.melodic_pairs(v) {
                let name = match (Interval::between(a.pitch, b.pitch).abs().steps, free, bass) {
                    (2, false, _) => "third",
                    (3, false, _) => "fourth",
                    (4, false, _) => "fifth",
                    (5, false, _) => "sixth",
                    (7, false, _) => "octave",
                    (2, true, _) => "free_third",
                    (3, true, false) => "free_fourth",
                    (4, true, false) => "free_fifth",
                    (7, true, false) => "free_octave",
                    (3, true, true) => "free_bass_fourth",
                    (4, true, true) => "free_bass_fifth",
                    (7, true, true) => "free_bass_octave",
                    (5, true, _) => "free_sixth",
                    _ => continue,
                };
                report_at(
                    ctx,
                    out,
                    LEAP_COST.id,
                    ctx.settings.level(name),
                    v,
                    &b,
                    Detail::Interval(Interval::between(a.pitch, b.pitch)),
                );
            }
        }
    }
}

struct LeapRecovery;

static LEAP_RECOVERY: RuleInfo = RuleInfo {
    id: "MEL-LEAP-RECOVERY",
    title_de: "Kein Richtungswechsel nach großem Sprung",
    title_en: "No change of direction after a large leap",
    source: "Trad. (Jeppesen)",
    params: &[
        LEVEL,
        int(
            "min_steps",
            "kleinster Sprung, der ausgeglichen werden muss (Stufen: 4 = Quinte)",
            "smallest leap that needs recovery (steps: 4 = fifth)",
        ),
    ],
};

impl Rule for LeapRecovery {
    fn info(&self) -> &'static RuleInfo {
        &LEAP_RECOVERY
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let level = ctx.settings.level("level");
        let min_steps = ctx.settings.int("min_steps") as i32;
        for v in 0..ctx.analysis.voice_count() {
            let line = &ctx.analysis.lines[v];
            for i in ctx.first_in_window(line).max(2)..line.len() {
                let (a, b, c) = (line[i - 2], line[i - 1], line[i]);
                if a.end != b.start || b.end != c.start {
                    continue;
                }
                let leap = Interval::between(a.pitch, b.pitch);
                let next = Interval::between(b.pitch, c.pitch);
                if leap.abs().steps >= min_steps && next.direction() != -leap.direction() {
                    report_at(
                        ctx,
                        out,
                        LEAP_RECOVERY.id,
                        level,
                        v,
                        &c,
                        Detail::Interval(leap),
                    );
                }
            }
        }
    }
}

struct ConsecutiveLeaps;

static CONSEC_LEAPS: RuleInfo = RuleInfo {
    id: "MEL-CONSEC-LEAPS",
    title_de: "Sprünge in Folge (in dieselbe Richtung; im freien Satz auch mit Wende)",
    title_en: "Consecutive leaps (in the same direction; in free counterpoint also turning)",
    source: "Trad. (Jeppesen); Korpus; freier Satz: Korpus Palestrina (zwei Sprünge in dieselbe Richtung 0,5 %, mit Wende 4,3 % der Dreitonfolgen)",
    params: &[
        level(
            "level",
            "zwei Sprünge ohne Dreiklangsbrechung, oder drei Sprünge",
            "two leaps not outlining a triad, or three leaps",
        ),
        level(
            "beyond_octave",
            "zusammen mehr als eine Oktave",
            "together more than an octave",
        ),
        level(
            "larger_above",
            "zwei Sprünge als Dreiklang, der größere oben (Jeppesen: der größere unten)",
            "two leaps outlining a triad, the larger above (Jeppesen: the larger below)",
        ),
        level(
            "free",
            "freier Satz: zwei Sprünge als Dreiklang in dieselbe Richtung",
            "free counterpoint: two leaps outlining a triad in one direction",
        ),
        level(
            "free_turn",
            "freier Satz: zwei Sprünge mit Wende",
            "free counterpoint: two leaps turning",
        ),
    ],
};

/// Whether three notes moving in one direction outline a major or minor triad (in any
/// inversion) or the frame fifth + fourth.
fn outlines_triad(a: Sounding, b: Sounding, c: Sounding) -> bool {
    let mut notes = [a.pitch, b.pitch, c.pitch];
    notes.sort_by_key(|p| p.diatonic());
    let lower = Interval::between(notes[0], notes[1]);
    let upper = Interval::between(notes[1], notes[2]);
    matches!(
        (
            (lower.steps, lower.semitones),
            (upper.steps, upper.semitones)
        ),
        ((2, 4), (2, 3))
            | ((2, 3), (2, 4))
            | ((2, 3), (3, 5))
            | ((2, 4), (3, 5))
            | ((3, 5), (2, 4))
            | ((3, 5), (2, 3))
            | ((4, 7), (3, 5))
            | ((3, 5), (4, 7))
    )
}

impl Rule for ConsecutiveLeaps {
    fn info(&self) -> &'static RuleInfo {
        &CONSEC_LEAPS
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for v in 0..ctx.analysis.voice_count() {
            let line = &ctx.analysis.lines[v];
            for i in ctx.first_in_window(line).max(2)..line.len() {
                let (a, b, c) = (line[i - 2], line[i - 1], line[i]);
                if a.end != b.start || b.end != c.start {
                    continue;
                }
                let first = Interval::between(a.pitch, b.pitch);
                let second = Interval::between(b.pitch, c.pitch);
                if !(first.is_leap() && second.is_leap()) {
                    continue;
                }
                let free = species(ctx.score, v) == Some(Species::Free);
                if first.direction() != second.direction() {
                    if free {
                        report_at(
                            ctx,
                            out,
                            CONSEC_LEAPS.id,
                            ctx.settings.level("free_turn"),
                            v,
                            &c,
                            Detail::Pitches(a.pitch, c.pitch),
                        );
                    }
                    continue;
                }
                let span = Interval::between(a.pitch, c.pitch).abs();
                let mut name = None;
                if span.steps > 7 {
                    name = Some("beyond_octave");
                } else if !outlines_triad(a, b, c) {
                    name = Some("level");
                } else {
                    if i >= 3 {
                        let before = line[i - 3];
                        let third = Interval::between(before.pitch, a.pitch);
                        if before.end == a.start
                            && third.is_leap()
                            && third.direction() == first.direction()
                        {
                            name = Some("level");
                        }
                    }
                    // Upwards the second leap lies above the first, downwards below it.
                    let (lower, upper) = if first.direction() > 0 {
                        (first, second)
                    } else {
                        (second, first)
                    };
                    if name.is_none() && upper.abs().steps > lower.abs().steps {
                        name = Some("larger_above");
                    }
                    if name.is_none() && free {
                        name = Some("free");
                    }
                }
                if let Some(name) = name {
                    report_at(
                        ctx,
                        out,
                        CONSEC_LEAPS.id,
                        ctx.settings.level(name),
                        v,
                        &c,
                        Detail::Pitches(a.pitch, c.pitch),
                    );
                }
            }
        }
    }
}

struct TritoneOutline;

static TRITONE_OUTLINE: RuleInfo = RuleInfo {
    id: "MEL-TRITONE-OUTLINE",
    title_de: "Tritonus zwischen zwei Wendepunkten",
    title_en: "Tritone between two turning points",
    source: "Trad.",
    params: &[LEVEL],
};

impl Rule for TritoneOutline {
    fn info(&self) -> &'static RuleInfo {
        &TRITONE_OUTLINE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let level = ctx.settings.level("level");
        for v in 0..ctx.analysis.voice_count() {
            let line = &ctx.analysis.lines[v];
            // Turning points with the tick at which they are known to be one.
            let mut points: Vec<(Sounding, u32)> = Vec::new();
            let mut direction = 0;
            // A violation is reported at the start of a note in the window, so the scan begins at
            // the first such note, with the direction and the last turning point that a scan from
            // the first note would have found there (looked up backwards to the last change of
            // direction or the start of the phrase).
            let first = ctx.first_in_window(line);
            if first >= line.len() {
                continue;
            }
            if first > 0 {
                let mut j = first - 1;
                let mut newest: Option<(usize, i32)> = None;
                loop {
                    let joined = j > 0 && line[j - 1].end == line[j].start;
                    if !joined {
                        points.push((line[j], line[j].start));
                        break;
                    }
                    let step = Interval::between(line[j - 1].pitch, line[j].pitch).direction();
                    if step != 0 {
                        if direction == 0 {
                            direction = step;
                        }
                        match newest {
                            Some((later, d)) if d != step => {
                                points.push((line[later - 1], line[later].start));
                                break;
                            }
                            _ => newest = Some((j, step)),
                        }
                    }
                    j -= 1;
                }
            }
            for (i, s) in line.iter().enumerate().skip(first) {
                let joined = i > 0 && line[i - 1].end == s.start;
                if !joined {
                    points.clear();
                    points.push((*s, s.start));
                    direction = 0;
                    continue;
                }
                let step = Interval::between(line[i - 1].pitch, s.pitch).direction();
                if step != 0 && direction != 0 && step != direction {
                    points.push((line[i - 1], s.start));
                    check_outline(ctx, out, level, v, &points);
                }
                if step != 0 {
                    direction = step;
                }
                if ctx.complete && i + 1 == line.len() {
                    points.push((*s, s.start));
                    check_outline(ctx, out, level, v, &points);
                }
            }
        }
    }
}

fn check_outline(
    ctx: &Ctx,
    out: &mut Vec<Violation>,
    level: Level,
    voice: usize,
    points: &[(Sounding, u32)],
) {
    if let [.., (a, _), (b, known)] = points
        && Interval::between(a.pitch, b.pitch).is_tritone()
    {
        ctx.report(
            out,
            TRITONE_OUTLINE.id,
            level,
            voice,
            b.note,
            *known,
            Detail::Pitches(a.pitch, b.pitch),
        );
    }
}

struct Repeat;

static REPEAT: RuleInfo = RuleInfo {
    id: "MEL-REPEAT",
    title_de: "Tonwiederholung",
    title_en: "Repeated note",
    source: "W23 1.P2, 2.M2*; Korpus",
    params: &[
        level("g1", "1. Gattung", "first species"),
        level(
            "g2_g4",
            "2.–4. Gattung, zwei- und dreistimmig",
            "species 2–4, two and three voices",
        ),
        level(
            "g5",
            "5. Gattung, zwei- und dreistimmig, außer Vorausnahme",
            "fifth species, two and three voices, except anticipation",
        ),
        level(
            "four_voices",
            "2.–5. Gattung, ab vier Stimmen",
            "species 2–5, four or more voices",
        ),
        level(
            "anticipation",
            "5. Gattung und freier Satz: Vorausnahme (Viertel, dann derselbe Ton als Synkope)",
            "fifth species and free counterpoint: anticipation (a quarter, then the same pitch as a syncopation)",
        ),
        level("free", "freier Satz", "free counterpoint"),
        level(
            "free_black",
            "freier Satz, nach einer schwarzen Note außer der Vorausnahme (statt free und free_texted)",
            "free counterpoint, after a black note except the anticipation (instead of free and free_texted)",
        ),
        level(
            "free_run",
            "freier Satz, dritter und weiterer gleicher Ton in Folge: mindestens",
            "free counterpoint, the third and any later note of one pitch in a row: at least",
        ),
        level(
            "free_texted",
            "freier Satz, Stimme mit Text, nach einer weißen Note (statt free)",
            "free counterpoint, a voice that sings a text, after a white note (instead of free)",
        ),
    ],
};

impl Rule for Repeat {
    fn info(&self) -> &'static RuleInfo {
        &REPEAT
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let many = ctx.analysis.voice_count() >= 4;
        for v in 0..ctx.analysis.voice_count() {
            let free = species(ctx.score, v) == Some(Species::Free);
            let name = match species(ctx.score, v) {
                None => continue,
                Some(Species::First) => "g1",
                // A voice that sings repeats a note for a new syllable (Palestrina, Jeppesen).
                Some(Species::Free) if super::text::sings(ctx.score, v) => "free_texted",
                Some(Species::Free) => "free",
                Some(_) if many => "four_voices",
                Some(Species::Fifth) => "g5",
                Some(_) => "g2_g4",
            };
            let fifth = species(ctx.score, v).is_some_and(Species::is_florid);
            let line = &ctx.analysis.lines[v];
            let notes = &ctx.score.voices[v].notes;
            for (i, a, b) in ctx.melodic_pairs(v) {
                if a.pitch == b.pitch {
                    // A quarter repeated as a note written as a half or longer: the anticipation.
                    // The written value is known where the case is decided, at the repeat; a tie
                    // after it is not yet, so a quarter tied over stays a repeated quarter.
                    let anticipation = fifth
                        && a.end - a.start == 2
                        && notes[b.note].duration.ticks() >= Duration::HALF.ticks();
                    // After a black note a new syllable begins only by licence (TXT-AFTER-BLACK,
                    // TXT-ONSET): such a repeat stays within its syllable, and the text does not
                    // excuse it.
                    let name = if anticipation {
                        "anticipation"
                    } else if free && a.end - a.start < 4 {
                        "free_black"
                    } else {
                        name
                    };
                    let mut level = ctx.settings.level(name);
                    // The third note of one pitch in a row, or a later one: a text excuses a
                    // repeat, not a recitation.
                    if free && i >= 2 && line[i - 2].end == a.start && line[i - 2].pitch == a.pitch
                    {
                        level = stricter(level, ctx.settings.level("free_run"));
                    }
                    report_at(ctx, out, REPEAT.id, level, v, &b, Detail::Pitch(b.pitch));
                }
            }
        }
    }
}

struct Variety;

static VARIETY: RuleInfo = RuleInfo {
    id: "MEL-VARIETY",
    title_de: "Zu wenig Abwechslung (Ton kehrt rasch wieder)",
    title_en: "Too little variety (pitch returns soon)",
    source: "Mann 73; W23 3.M1*, 4.M2; L24 1.M4; Korpus (x–y–x in Vierteln: 3. Gattung 31 von 228, 5. Gattung 28 von 153)",
    params: &[
        level(
            "g1_within_three_measures",
            "1. Gattung: Ton innerhalb von drei Takten wieder",
            "first species: pitch again within three measures",
        ),
        level(
            "g3_two_beats",
            "3. Gattung: Ton zwei Viertel später wieder",
            "third species: pitch again two beats later",
        ),
        level(
            "g4_arsis",
            "4. Gattung: gleicher Arsis-Ton zwei Takte später",
            "fourth species: same upbeat pitch two measures later",
        ),
        level(
            "g5_two_beats",
            "5. Gattung: Viertel kehrt zum Ton zwei Noten davor zurück",
            "fifth species: a quarter returns to the pitch two notes before",
        ),
        level(
            "free_two_beats",
            "freier Satz: Viertel kehrt zum Ton zwei Noten davor zurück",
            "free counterpoint: a quarter returns to the pitch two notes before",
        ),
    ],
};

impl Rule for Variety {
    fn info(&self) -> &'static RuleInfo {
        &VARIETY
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let half = ctx.analysis.per_measure / 2;
        for v in 0..ctx.analysis.voice_count() {
            let line = &ctx.analysis.lines[v];
            match species(ctx.score, v) {
                Some(Species::First) => {
                    for i in ctx.first_in_window(line).max(2)..line.len() {
                        let back = if i >= 3 { 3 } else { 2 };
                        if line[i - back..i - 1]
                            .iter()
                            .any(|s| s.pitch == line[i].pitch)
                        {
                            report_at(
                                ctx,
                                out,
                                VARIETY.id,
                                ctx.settings.level("g1_within_three_measures"),
                                v,
                                &line[i],
                                Detail::Pitch(line[i].pitch),
                            );
                        }
                    }
                }
                Some(Species::Third) => {
                    for i in ctx.first_in_window(line).max(2)..line.len() {
                        let quarters = line[i].end - line[i].start == 2
                            && line[i - 2].end - line[i - 2].start == 2;
                        if quarters
                            && line[i - 2].pitch == line[i].pitch
                            && line[i - 1].pitch != line[i].pitch
                        {
                            report_at(
                                ctx,
                                out,
                                VARIETY.id,
                                ctx.settings.level("g3_two_beats"),
                                v,
                                &line[i],
                                Detail::Pitch(line[i].pitch),
                            );
                        }
                    }
                }
                Some(species @ (Species::Fifth | Species::Free)) => {
                    let name = if species == Species::Free {
                        "free_two_beats"
                    } else {
                        "g5_two_beats"
                    };
                    // A quarter returning to the pitch two notes before (x–y–x). Whether the note
                    // is a quarter is certain at its end: a tie may still lengthen it.
                    let first = line.partition_point(|s| s.end < ctx.window.from).max(2);
                    for i in first..line.len() {
                        let joined = line[i - 2].end == line[i - 1].start
                            && line[i - 1].end == line[i].start;
                        if joined
                            && line[i].end - line[i].start == 2
                            && line[i - 2].pitch == line[i].pitch
                            && line[i - 1].pitch != line[i].pitch
                        {
                            ctx.report(
                                out,
                                VARIETY.id,
                                ctx.settings.level(name),
                                v,
                                line[i].note,
                                line[i].end,
                                Detail::Pitch(line[i].pitch),
                            );
                        }
                    }
                }
                Some(Species::Fourth) => {
                    let arses: Vec<&Sounding> = line
                        .iter()
                        .filter(|s| s.start % ctx.analysis.per_measure == half)
                        .collect();
                    for i in 2..arses.len() {
                        if arses[i].pitch == arses[i - 2].pitch {
                            report_at(
                                ctx,
                                out,
                                VARIETY.id,
                                ctx.settings.level("g4_arsis"),
                                v,
                                arses[i],
                                Detail::Pitch(arses[i].pitch),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

struct Motif;

static MOTIF: RuleInfo = RuleInfo {
    id: "MEL-MOTIF",
    title_de: "Wörtlich wiederholte Tonfolge in der Gegenstimme (monotonia)",
    title_en: "Literally repeated figure in the counterpoint (monotonia)",
    source: "Mann 54; Korpus (zweistimmig: 1. und 4. Gattung nie, 2. Gattung 3×, 3. und 5. Gattung häufig)",
    params: &[
        level("g1", "1. Gattung", "first species"),
        level("g2", "2. Gattung", "second species"),
        level("g3", "3. Gattung", "third species"),
        level("g4", "4. Gattung", "fourth species"),
        level("g5", "5. Gattung", "fifth species"),
        level("free", "freier Satz", "free counterpoint"),
        int(
            "notes",
            "kürzeste Wiederholung (Töne)",
            "shortest repetition (notes)",
        ),
    ],
};

impl Rule for Motif {
    fn info(&self) -> &'static RuleInfo {
        &MOTIF
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let length = ctx.settings.int("notes").max(2) as usize;
        for v in 0..ctx.analysis.voice_count() {
            let name = match species(ctx.score, v) {
                None => continue,
                Some(Species::First) => "g1",
                Some(Species::Second) => "g2",
                Some(Species::Third) => "g3",
                Some(Species::Fourth) => "g4",
                Some(Species::Fifth) => "g5",
                Some(Species::Free) => "free",
            };
            let level = ctx.settings.level(name);
            if level == Level::Off {
                continue;
            }
            let line = &ctx.analysis.lines[v];
            let pitches: Vec<_> = line.iter().map(|s| s.pitch).collect();
            let first = ctx.first_in_window(line).max(length - 1);
            for (end, note) in line.iter().enumerate().skip(first) {
                if super::cantus::repeats_earlier(&pitches, end + 1 - length, length) {
                    report_at(
                        ctx,
                        out,
                        MOTIF.id,
                        level,
                        v,
                        note,
                        Detail::Pitch(note.pitch),
                    );
                }
            }
        }
    }
}

struct Climax;

static CLIMAX: RuleInfo = RuleInfo {
    id: "MEL-CLIMAX",
    title_de: "Höhepunkt der Gegenstimme mehrfach",
    title_en: "Highest note of the counterpoint repeated",
    source: "Trad.",
    params: &[LEVEL],
};

impl Rule for Climax {
    fn info(&self) -> &'static RuleInfo {
        &CLIMAX
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        if !ctx.complete {
            return;
        }
        for v in (0..ctx.analysis.voice_count()).filter(|&v| is_counterpoint(ctx, v)) {
            let line = &ctx.analysis.lines[v];
            let Some(top) = line.iter().map(|s| s.pitch.midi()).max() else {
                continue;
            };
            let peaks: Vec<&Sounding> = line.iter().filter(|s| s.pitch.midi() == top).collect();
            if peaks.len() > 1 {
                report_at(
                    ctx,
                    out,
                    CLIMAX.id,
                    ctx.settings.level("level"),
                    v,
                    peaks[1],
                    Detail::Count(peaks.len() as u32),
                );
            }
        }
    }
}

struct VoiceRange;

static VOICE_RANGE: RuleInfo = RuleInfo {
    id: "VOI-RANGE",
    title_de: "Außerhalb des Liniensystems",
    title_en: "Outside the staff",
    source: "Mann 79; Korpus",
    params: &[
        level("one_step", "eine Stufe darüber", "one step above"),
        level("one_step_below", "eine Stufe darunter", "one step below"),
        level("two_steps", "zwei Stufen außerhalb", "two steps outside"),
        level(
            "three_steps",
            "drei Stufen außerhalb",
            "three steps outside",
        ),
        level("more", "weiter außerhalb", "further outside"),
    ],
};

impl Rule for VoiceRange {
    fn info(&self) -> &'static RuleInfo {
        &VOICE_RANGE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for v in 0..ctx.analysis.voice_count() {
            let clef = ctx.score.voices[v].voice_type.clef();
            let line = &ctx.analysis.lines[v];
            for s in &line[ctx.first_in_window(line)..] {
                let signed = clef.steps_outside(s.pitch);
                let outside = signed.abs();
                let name = match outside {
                    0 => continue,
                    1 if signed < 0 => "one_step_below",
                    1 => "one_step",
                    2 => "two_steps",
                    3 => "three_steps",
                    _ => "more",
                };
                ctx.report(
                    out,
                    VOICE_RANGE.id,
                    ctx.settings.level(name),
                    v,
                    s.note,
                    s.start,
                    Detail::Count(outside as u32),
                );
            }
        }
    }

    fn message(&self, violation: &Violation, lang: Lang) -> String {
        let steps = match violation.detail {
            Detail::Count(n) => n,
            _ => 0,
        };
        match lang {
            Lang::De => format!("{steps} Stufe(n) außerhalb des Liniensystems"),
            Lang::En => format!("{steps} step(s) outside the staff"),
        }
    }
}
