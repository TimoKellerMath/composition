//! Rules of free counterpoint that the species have no counterpart for: how a voice enters
//! (FREE-ENTRY), how long it holds a note (FREE-LONG), how often it rests (FREE-REST), how many
//! voices sound at once (FREE-TEXTURE) and in which octave of the mode it moves (FREE-AMBITUS).

use super::melodic::level;
use super::{Ctx, Detail, Rule, RuleInfo, Violation};
use crate::clef::VoiceType;
use crate::mode::{Ambitus as ModeAmbitus, ModeSpec};
use crate::score::{Role, Species};

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(Entry),
        Box::new(Long),
        Box::new(Rest),
        Box::new(Texture),
        Box::new(Ambitus),
    ]
}

fn free_voices<'a>(ctx: &'a Ctx<'a>) -> impl Iterator<Item = usize> + 'a {
    (0..ctx.analysis.voice_count())
        .filter(|&v| ctx.score.voices[v].role == Role::Counterpoint(Species::Free))
}

struct Entry;

static ENTRY: RuleInfo = RuleInfo {
    id: "FREE-ENTRY",
    title_de: "Einsatz einer Stimme nicht auf Finalis oder Quinte, oder mit kurzem Wert",
    title_en: "A voice enters off the final and the fifth, or with a short value",
    source: "Korpus Palestrina (erste Töne von 5 400 Stimmen in Sätzen, die auf der Finalis enden: Finalis 44 %, Quinte 33 %, Quarte 12 %, andere Stufen je 2–7 %; 92 % mindestens eine Ganze; nach einer Pause 0,7 % kürzer als eine Halbe)",
    params: &[
        level("level", "auf einer anderen Stufe", "on another degree"),
        level("fourth", "auf der Quarte", "on the fourth"),
        level(
            "short",
            "kürzer als eine Ganze",
            "shorter than a whole note",
        ),
        level(
            "reentry_short",
            "nach einer Pause kürzer als eine Halbe",
            "after a rest, shorter than a half note",
        ),
    ],
};

impl Rule for Entry {
    fn info(&self) -> &'static RuleInfo {
        &ENTRY
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let mode = ctx.score.mode;
        let per = ctx.analysis.per_measure;
        for v in free_voices(ctx) {
            let Some(&first) = ctx.analysis.lines[v].first() else {
                continue;
            };
            let degree = match mode.degree(first.pitch) {
                1 | 5 => None,
                4 => Some("fourth"),
                _ => Some("level"),
            };
            if let Some(name) = degree {
                ctx.report(
                    out,
                    ENTRY.id,
                    ctx.settings.level(name),
                    v,
                    first.note,
                    first.start,
                    Detail::Pitch(first.pitch),
                );
            }
            // The value is known when the next note (or rest) begins.
            let ended = first.end < ctx.analysis.ends[v] || ctx.complete;
            if first.end - first.start < per && ended {
                ctx.report(
                    out,
                    ENTRY.id,
                    ctx.settings.level("short"),
                    v,
                    first.note,
                    first.end,
                    Detail::Pitch(first.pitch),
                );
            }
            // Entries after a rest: at least a half note.
            let line = &ctx.analysis.lines[v];
            let from = line.partition_point(|s| s.end < ctx.window.from).max(1);
            for (i, note) in line.iter().enumerate().skip(from) {
                let after_rest = line[i - 1].end < note.start;
                let ended = note.end < ctx.analysis.ends[v] || ctx.complete;
                if after_rest && ended && note.end - note.start < per / 2 {
                    ctx.report(
                        out,
                        ENTRY.id,
                        ctx.settings.level("reentry_short"),
                        v,
                        note.note,
                        note.end,
                        Detail::Pitch(note.pitch),
                    );
                }
            }
        }
    }
}

struct Rest;

static REST: RuleInfo = RuleInfo {
    id: "FREE-REST",
    title_de: "Pause innerhalb einer Stimme",
    title_en: "Rest within a voice",
    source: "Korpus Palestrina (zwischen erstem und letztem Ton pausieren die Stimmen dreistimmig 8 %, vierstimmig 14 %, fünfstimmig 15 %, sechsstimmig 19 % der Zeit; die Hälfte der Pausen dauert einen Takt)",
    params: &[
        level(
            "level",
            "je Pause nach dem Einsatz (höchstens ein Takt)",
            "per rest after the entry (a measure at most)",
        ),
        level(
            "texted",
            "dasselbe in einer Stimme mit Text (statt level)",
            "the same in a voice that sings a text (instead of level)",
        ),
    ],
};

impl Rule for Rest {
    fn info(&self) -> &'static RuleInfo {
        &REST
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for v in free_voices(ctx) {
            // A voice that sings rests where its text lets it, which TXT-REST judges.
            let name = if super::text::sings(ctx.score, v) {
                "texted"
            } else {
                "level"
            };
            let mut entered = false;
            for (n, (start, note)) in ctx.score.voices[v].timed().enumerate() {
                if !note.is_rest() {
                    entered = true;
                } else if entered {
                    ctx.report(
                        out,
                        REST.id,
                        ctx.settings.level(name),
                        v,
                        n,
                        start,
                        Detail::None,
                    );
                }
            }
        }
    }
}

struct Texture;

static TEXTURE: RuleInfo = RuleInfo {
    id: "FREE-TEXTURE",
    title_de: "Zu wenige Stimmen klingen (nach dem Einsatz aller Stimmen, vor dem letzten Takt)",
    title_en: "Too few voices sound (after every voice has entered, before the last measure)",
    source: "Korpus Palestrina (Taktanfänge mit weniger als der Hälfte der Stimmen 0,4–2,3 %, ohne jede Stimme 0,1 %)",
    params: &[
        level(
            "level",
            "weniger als die Hälfte der Stimmen, je Taktanfang",
            "fewer than half of the voices, per downbeat",
        ),
        level(
            "silence",
            "keine Stimme, je Taktanfang",
            "no voice, per downbeat",
        ),
    ],
};

impl Rule for Texture {
    fn info(&self) -> &'static RuleInfo {
        &TEXTURE
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        // The choir: every voice but a tacet one, which rests through a section of a mass that
        // it does not sing (phase 14).
        let choir: Vec<usize> = (0..ctx.analysis.voice_count())
            .filter(|&v| !ctx.tacet(v))
            .collect();
        let n = choir.len();
        if n < 3 || !ctx.score.is_free() {
            return;
        }
        // Every voice must have entered; until then the entries are still being set.
        let Some(entered) = choir
            .iter()
            .map(|&v| ctx.analysis.lines[v].first().map(|s| s.start))
            .collect::<Option<Vec<u32>>>()
            .and_then(|starts| starts.into_iter().max())
        else {
            return;
        };
        for i in ctx.downbeats() {
            let slice = &ctx.analysis.slices[i];
            if slice.tick < entered
                || ctx.in_last_measure(slice.tick)
                || !ctx.analysis.complete_at(slice.tick)
            {
                continue;
            }
            let sounding: Vec<usize> = choir
                .iter()
                .copied()
                .filter(|&v| slice.sounding[v].is_some())
                .collect();
            let name = match sounding.len() {
                0 => "silence",
                k if 2 * k < n => "level",
                _ => continue,
            };
            // Attributed to the highest resting voice, which may sing.
            let Some(v) = choir.iter().copied().find(|v| !sounding.contains(v)) else {
                continue;
            };
            let note = ctx.score.voices[v]
                .timed()
                .position(|(start, n)| {
                    start <= slice.tick && slice.tick < start + n.duration.ticks()
                })
                .unwrap_or(0);
            ctx.report(
                out,
                TEXTURE.id,
                ctx.settings.level(name),
                v,
                note,
                slice.tick,
                Detail::Count(sounding.len() as u32),
            );
        }
    }
}

struct Long;

static LONG: RuleInfo = RuleInfo {
    id: "FREE-LONG",
    title_de: "Note länger als eine Brevis gehalten",
    title_en: "Note held longer than a breve",
    source: "Korpus Palestrina (0,47 % der Noten außer der letzten jeder Stimme; punktierte Ganze 1,9 %, Brevis 3,1 %)",
    params: &[
        level(
            "level",
            "je Takt über die Brevis hinaus",
            "per measure beyond the breve",
        ),
        level(
            "over_whole",
            "länger als eine Ganze, je Note",
            "longer than a whole note, per note",
        ),
    ],
};

impl Rule for Long {
    fn info(&self) -> &'static RuleInfo {
        &LONG
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        let per = ctx.analysis.per_measure;
        let breve = 2 * per;
        for v in free_voices(ctx) {
            let line = &ctx.analysis.lines[v];
            let first = line.partition_point(|s| s.end < ctx.window.from);
            for (i, s) in line.iter().enumerate().skip(first) {
                // The last note of a voice may be held: Palestrina's movements end on long notes.
                if ctx.complete && i + 1 == line.len() {
                    continue;
                }
                // Longer than a whole note: decided where it reaches beyond it.
                let whole = per;
                if s.end > s.start + whole {
                    ctx.report(
                        out,
                        LONG.id,
                        ctx.settings.level("over_whole"),
                        v,
                        s.note,
                        s.start + whole,
                        Detail::Pitch(s.pitch),
                    );
                }
                // Once for each measure the note is held into beyond the breve, decided where the
                // tie reaches it.
                let mut tick = s.start + breve;
                while tick < s.end {
                    ctx.report(
                        out,
                        LONG.id,
                        ctx.settings.level("level"),
                        v,
                        s.note,
                        tick,
                        Detail::Pitch(s.pitch),
                    );
                    tick = (tick / per + 1) * per;
                }
            }
        }
    }
}

struct Ambitus;

static AMBITUS: RuleInfo = RuleInfo {
    id: "FREE-AMBITUS",
    title_de: "Ton mehr als eine Stufe außerhalb der Oktave der Stimme im Modus (Sopran und Tenor im Ambitus des Modus, Alt und Bass im anderen)",
    title_en: "Note more than a step outside the voice's octave of the mode (soprano and tenor in the mode's ambitus, alto and bass in the other)",
    source: "Zarlino, Glarean; Korpus Palestrina (in Sätzen, die auf der Finalis enden, nach dem Tenor authentisch oder plagal: außerhalb Tenor 0,3 %, Cantus 1,4 %, Altus 3,3 %, Bassus 1,9 %, Bariton 0,7 % der Töne)",
    params: &[
        level(
            "principal",
            "Sopran und Tenor, je Ton",
            "soprano and tenor, per note",
        ),
        level(
            "collateral",
            "Alt, Bariton und Bass, je Ton",
            "alto, baritone and bass, per note",
        ),
    ],
};

/// Whether a voice type takes the mode's own ambitus, as the tenor and the soprano an octave
/// above it do; the alto, the baritone and the bass take the other one.
pub(crate) fn principal(voice: VoiceType) -> bool {
    matches!(
        voice,
        VoiceType::HighSoprano | VoiceType::Soprano | VoiceType::MezzoSoprano | VoiceType::Tenor
    )
}

/// The octave of the mode a voice moves in, as the diatonic index of its lowest step: from the
/// final (authentic) or the fourth below it (plagal), where its middle lies nearest the middle
/// of the voice's range (on a tie the lower one).
pub fn modal_octave(mode: ModeSpec, voice: VoiceType) -> i32 {
    let authentic = (mode.ambitus == ModeAmbitus::Authentic) == principal(voice);
    let offset = if authentic { 0 } else { -3 };
    let (low, high) = voice.free_range();
    let middle = low.diatonic() + high.diatonic();
    (0..8)
        .map(|octave| mode.final_pitch(octave).diatonic() + offset)
        .min_by_key(|bottom| ((2 * bottom + 7 - middle).abs(), *bottom))
        .unwrap_or(0)
}

impl Rule for Ambitus {
    fn info(&self) -> &'static RuleInfo {
        &AMBITUS
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        for v in free_voices(ctx) {
            let voice = ctx.score.voices[v].voice_type;
            let bottom = modal_octave(ctx.score.mode, voice);
            let name = if principal(voice) {
                "principal"
            } else {
                "collateral"
            };
            let line = &ctx.analysis.lines[v];
            for note in &line[ctx.first_in_window(line)..] {
                let d = note.pitch.diatonic();
                if d < bottom - 1 || d > bottom + 8 {
                    ctx.report(
                        out,
                        AMBITUS.id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::Pitch;

    #[test]
    fn modal_octaves_follow_the_ambitus() {
        let octave = |mode: &str, voice| {
            let bottom = modal_octave(mode.parse().unwrap(), voice);
            Pitch::from_diatonic(bottom).to_string()
        };
        // Dorian: tenor and soprano from D, alto and bass from A below.
        assert_eq!(octave("dorian", VoiceType::Soprano), "D4");
        assert_eq!(octave("dorian", VoiceType::Alto), "A3");
        assert_eq!(octave("dorian", VoiceType::Tenor), "D3");
        assert_eq!(octave("dorian", VoiceType::Bass), "A2");
        // Hypodorian: the other way round, a fourth lower.
        assert_eq!(octave("hypodorian", VoiceType::Soprano), "A3");
        assert_eq!(octave("hypodorian", VoiceType::Alto), "D3");
        assert_eq!(octave("hypodorian", VoiceType::Tenor), "A2");
        assert_eq!(octave("hypodorian", VoiceType::Bass), "D2");
    }
}
