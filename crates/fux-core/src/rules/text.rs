//! The rules of the text underlay (TXT-*, docs/regeln.md § 17): where the syllables of the text
//! stand under the notes of a singing voice, after Zarlino (1558), Lanfranco, Vicentino,
//! Stoquerus and Palestrina's practice as Jeppesen describes it. The corpus has no text, so
//! every level is provisional.
//!
//! The underlay model ([`crate::text::underlay::Model`]) decides every case, one voice at a time;
//! the search and the dynamic program after it price the same cases with the same
//! [`Levels`]. A check runs the model once along each voice's own underlay (a syllable begins
//! where a note has a lyric) and each rule reports the cases that belong to it, at the tick the
//! model decides them and on the note it marks. No case depends on another voice, and the model
//! decides each case as soon as the notes up to its tick show it, so windows and prefixes of a
//! score find each case once.
//!
//! The rules stay silent without a text in the score ([`Score::text`]), on voices that do not
//! sing it, on a cantus firmus and on voices the check does not count. A lyric that matches no
//! syllable of the text ([`crate::score::Lyric::UNKNOWN`], an imported syllable the aligner could
//! not place) is a syllable of unknown text: no TXT-ORDER case for it, and nothing that depends on
//! which syllable it is (see [`Model::cases`]).

use super::melodic::{int, level};
use super::{Ctx, Detail, Level, Rule, RuleInfo, Violation};
use crate::lang::Lang;
use crate::score::Score;
use crate::text::underlay::{Found, Goal, Levels, Model};

/// Per voice of a score, the cases of the underlay model if the voice sings the text and counts in
/// the check; `None` for the other voices.
pub(super) type Cases = Vec<Option<Vec<Found>>>;

/// Whether voice `v` of a score sings the score's text.
pub(super) fn sings(score: &Score, v: usize) -> bool {
    score
        .text
        .as_ref()
        .is_some_and(|text| text.voices.get(v).copied().unwrap_or(false))
}

/// The cases of the underlay model in the voices the TXT rules look at, found once per check for
/// all of them: voices that sing the text, are no cantus firmus and count in the check. The model
/// walks all written notes of such a voice, and its end once the score is complete.
fn cases<'a>(ctx: &Ctx<'a>) -> &'a Cases {
    ctx.shared.text.get_or_init(|| {
        let Some(sung) = &ctx.score.text else {
            return Vec::new();
        };
        // A parsed text always has a syllable; an empty one has nothing to sing.
        if sung.text.syllables.is_empty() {
            return Vec::new();
        }
        let score = ctx.score;
        let goals = Goal::of(&score.cadences, score.meter);
        let restarts = score
            .imitation
            .as_deref()
            .map(|plan| plan.restarts(&sung.text))
            .unwrap_or_default();
        let model = Model::new(&sung.text, score.mode, score.meter, &goals)
            .with_restarts(&sung.text, &restarts);
        score
            .voices
            .iter()
            .enumerate()
            .map(|(v, voice)| {
                let looked_at = sings(score, v) && !voice.is_cantus_firmus() && ctx.is_target(v);
                looked_at.then(|| model.cases(&voice.notes, ctx.complete))
            })
            .collect()
    })
}

pub(super) fn rules() -> Vec<Box<dyn Rule>> {
    [
        &ONSET,
        &AFTER_BLACK,
        &REPEATED_PITCH,
        &ENTRY,
        &ORDER,
        &REPEAT,
        &REST,
        &COMPLETE,
        &MELISMA,
        &ACCENT,
        &LEAP,
        &CADENCE,
    ]
    .into_iter()
    .map(|info| Box::new(TextRule(info)) as Box<dyn Rule>)
    .collect()
}

/// A rule of the text underlay: reports the cases of the underlay model that belong to it.
struct TextRule(&'static RuleInfo);

impl Rule for TextRule {
    fn info(&self) -> &'static RuleInfo {
        self.0
    }

    fn check(&self, ctx: &Ctx, out: &mut Vec<Violation>) {
        // Without a text this is all a check pays, in the search too.
        if ctx.score.text.is_none() {
            return;
        }
        let id = self.0.id;
        // The levels of this rule's cases; TXT-MELISMA `long` counts only beyond its limit, which
        // the case names by the syllable and the kind of the text (`max_short`, `max_long`,
        // `max_weak`, `max_opening`, `max_last`).
        let levels = Levels::new(
            |rule, param| {
                if rule == id {
                    ctx.settings.level(param)
                } else {
                    Level::Off
                }
            },
            |rule, param| {
                if rule == id {
                    ctx.settings.int(param)
                } else {
                    0
                }
            },
        );
        for (v, found) in cases(ctx).iter().enumerate() {
            for f in found.iter().flatten().filter(|f| f.rule() == id) {
                // The attack's number for a melisma that is too long, otherwise the case.
                let detail = match f.case.limit() {
                    Some(_) => Detail::Count(u32::from(f.value)),
                    None => Detail::Param(f.param()),
                };
                ctx.report(out, id, levels.level(f), v, f.note, f.tick, detail);
            }
        }
    }

    /// The rule's title, followed by the case unless it is the rule's main one (`level`), or by
    /// the number of the attack for a melisma that is too long.
    fn message(&self, violation: &Violation, lang: Lang) -> String {
        let title = self.0.title(lang);
        match violation.detail {
            Detail::Count(n) => match lang {
                Lang::De => format!("{title}: {n}. Ton auf einer Silbe"),
                Lang::En => format!("{title}: note {n} on one syllable"),
            },
            Detail::Param(name) if name != "level" => match self.0.param(name) {
                Some(param) => format!("{title}: {}", lang.pick(param.de, param.en)),
                None => title.to_string(),
            },
            _ => title.to_string(),
        }
    }
}

static ONSET: RuleInfo = RuleInfo {
    id: "TXT-ONSET",
    title_de: "Silbe auf einem Wert oder an einer Stelle, die keine Silbe trägt",
    title_en: "Syllable on a value or position that cannot carry one",
    source: "Zarlino 1558, IV.33, Regeln 1b–7; Lanfranco 4–6; Stoquerus N2, N3, M2, O5, XXIV.1; Jeppesen, Regel 2; Zacconi (Dominus); Einsätze frei (Zarlino 7: „qualunque figura“); Abschnitte beginnen auf einer Halben oder länger (Jeppesen, S. 160, 269); Korpus ohne Text, vorläufig",
    params: &[
        level(
            "level",
            "neue Silbe auf einer Achtel, auf einer weißen Note zwischen den halben Schlägen, auf einem Viertel zwischen ihnen (außer dem Daktylus-Viertel), auf einer Pause oder Überbindung",
            "a new syllable on an eighth, on a white note off the minim beats, on a quarter off them (except the dactyl quarter), on a rest or a tie continuation",
        ),
        level(
            "run_upbeat",
            "Silbe auf einem Viertel auf Zählzeit 2, mit dem dort ein Lauf beginnt",
            "a syllable on a quarter on beat 2 that starts a run there",
        ),
        level(
            "single",
            "textiertes Viertel auf einem halben Schlag ohne folgenden Lauf; textiertes Daktylus-Viertel ohne folgende Halbe oder längere Note",
            "a texted quarter on a minim beat without a run after it; a texted dactyl quarter without a half or longer after it",
        ),
        level(
            "figure",
            "unbetonte Silbe im Sinnabschnitt auf dem Daktylus-Viertel (nach einer punktierten Halben auf einem halben Schlag): Dó-mi-ne",
            "an unaccented syllable inside a unit on the dactyl quarter (after a dotted half on a minim beat): Dó-mi-ne",
        ),
        level(
            "figure_accent",
            "jede andere Silbe auf dem Daktylus-Viertel: betont, Anfangssilbe (auch einer Wiederholung) oder letzte Silbe eines Sinnabschnitts",
            "any other syllable on the dactyl quarter: an accented one, an opening one (a restatement too) or the last of a unit",
        ),
        level(
            "opening_black",
            "Anfangssilbe (eines Sinnabschnitts oder Schlussteils, nach einer Zäsur, auch einer Wiederholung) auf einer schwarzen Note auf einem halben Schlag oder nach einer Pause, außer beim ersten Einsatz",
            "an opening syllable (of a unit or tail, after a marked break, a restatement too) on a black note on a minim beat or after a rest, except at the voice's first entry",
        ),
    ],
};

static AFTER_BLACK: RuleInfo = RuleInfo {
    id: "TXT-AFTER-BLACK",
    title_de: "Neue Silbe direkt nach Vierteln oder Achteln",
    title_en: "New syllable right after quarters or eighths",
    source: "Stoquerus M3; Vicentino 1, 2; Lanfranco 5, 6; Zarlino 1558, IV.33, Regeln 4–6; Jeppesen, Regel 7; Palestrina, Missa Dies sanctificatus, Christe (Jeppesen, S. 254); Korpus ohne Text, vorläufig",
    params: &[
        level(
            "level",
            "neue Silbe direkt nach Vierteln oder Achteln",
            "a new syllable right after quarters or eighths",
        ),
        level(
            "figure",
            "nach dem textierten Daktylus-Viertel",
            "after the texted dactyl quarter",
        ),
        level(
            "unit_end",
            "letzte Silbe eines Sinnabschnitts auf einer Halben oder länger",
            "the last syllable of a sense unit on a half or longer",
        ),
        level(
            "dotted",
            "auf einer Halben oder länger nach dem untextierten Daktylus-Viertel (nach einer punktierten Halben auf einem halben Schlag)",
            "on a half or longer after the untexted dactyl quarter (after a dotted half on a minim beat)",
        ),
    ],
};

static REPEATED_PITCH: RuleInfo = RuleInfo {
    id: "TXT-REPEATED-PITCH",
    title_de: "Tonwiederholung ohne neue Silbe",
    title_en: "Repeated pitch without a new syllable",
    source: "Stoquerus N4 (nach Vierteln frei: Antizipation); Jeppesen, Regel 9; Quadflieg IV; Korpus ohne Text, vorläufig",
    params: &[level(
        "level",
        "Tonwiederholung nach einer Halben oder länger ohne neue Silbe",
        "a repeated pitch after a half or longer without a new syllable",
    )],
};

static ENTRY: RuleInfo = RuleInfo {
    id: "TXT-ENTRY",
    title_de: "Einsatz ohne Silbe",
    title_en: "Entry without a syllable",
    source: "Zarlino 1558, IV.33, Regel 7 („di necessità“); Stoquerus N5; Korpus ohne Text, vorläufig",
    params: &[level(
        "level",
        "Einsatz (erster Ton oder Ton nach einer Pause) ohne neue Silbe",
        "an entry (the first note or a note after a rest) without a new syllable",
    )],
};

static ORDER: RuleInfo = RuleInfo {
    id: "TXT-ORDER",
    title_de: "Text nicht in der Reihenfolge",
    title_en: "Text out of order",
    source: "Zarlino 1558, IV.33, Regel 8c; Vicentino 4; Zacconi 4; Stoquerus M4; Palestrina (Jeppesen); Korpus ohne Text, vorläufig",
    params: &[
        level(
            "level",
            "Silbe weder die nächste noch nach dem Ende eines Sinnabschnitts ein Wiederholungsbeginn in dessen Satz",
            "a syllable that is neither the next one nor, after the end of a unit, a restart in its sentence",
        ),
        level(
            "first",
            "erste Silbe der Stimme nicht die erste des Textes",
            "the voice's first syllable is not the first of the text",
        ),
    ],
};

static REPEAT: RuleInfo = RuleInfo {
    id: "TXT-REPEAT",
    title_de: "Textwiederholung",
    title_en: "Repetition of text",
    source: "Zarlino 1558, IV.33, Regeln 8b–8d („non dico gia di vna sillaba, ne di vna parola“); Vicentino 4; Stoquerus M4 („perraro“); Lanfranco 7; Palestrina: in Kyrie, Sanctus und Agnus üblich, in Gloria und Credo selten (Jeppesen); Korpus ohne Text, vorläufig",
    params: &[
        level(
            "short",
            "Wiederholung in einem kurzen Text (Kyrie, Sanctus, Agnus)",
            "a restatement in a short text (Kyrie, Sanctus, Agnus)",
        ),
        level(
            "long",
            "Wiederholung in einem langen Text (Gloria, Credo), außer seinem letzten Abschnitt, wo der Text endet",
            "a restatement in a long text (Gloria, Credo), except of its last unit where the text ends",
        ),
        level(
            "again",
            "jede weitere Wiederholung im selben Satz",
            "each further restatement within one sentence",
        ),
        level(
            "word",
            "Wiederholung einzelner Wörter: Sinnabschnitte aus je einem Wort, die zusammen keinen Satz bilden (San-ctus, San-ctus)",
            "a restatement of lone words: units of one word each that make no sentence together (San-ctus, San-ctus)",
        ),
    ],
};

static REST: RuleInfo = RuleInfo {
    id: "TXT-REST",
    title_de: "Pause im Wort, im Sinnabschnitt oder nach zu kurzer Note",
    title_en: "Rest inside a word or unit, or after too short a note",
    source: "Zarlino 1558, IV.32 („alcuna sua parte“, „alcuni poco intelligenti“; an den Zwischenpunkten kurze Pausen, am Satzende „quanta quantità di pause“); Morley; Zacconi 4; Jeppesen (S. 269); Korpus ohne Text, vorläufig",
    params: &[
        level("word", "Pause im Wort", "a rest inside a word"),
        level(
            "short",
            "Pause oder Stimmende direkt nach Vierteln oder Achteln",
            "a rest or the voice's end right after quarters or eighths",
        ),
        level(
            "clause",
            "Pause nach einem Wort ohne Satzzeichen und ohne Zäsur (| oder /)",
            "a rest after a word without punctuation or a marked break (| or /)",
        ),
        level(
            "last_word",
            "dasselbe, wenn danach nur noch ein Wort bis zum Satzzeichen oder zur Zäsur bleibt (Laudámus | te.)",
            "the same where only one word is left up to the punctuation or break (Laudámus | te.)",
        ),
        level(
            "comma",
            "Pause nach einem Sinnabschnitt, der keinen Satz beendet (, ; :), oder an einer Zäsur",
            "a rest after a unit that ends no sentence (, ; :) or at a marked break",
        ),
        level(
            "long",
            "Stille länger als eine Halbe, obwohl die letzte Silbe keinen Satz beendet",
            "silence longer than a half although the last syllable did not end a sentence",
        ),
    ],
};

static COMPLETE: RuleInfo = RuleInfo {
    id: "TXT-COMPLETE",
    title_de: "Text nicht vollständig",
    title_en: "Text incomplete",
    source: "Zarlino 1558, IV.33, Regel 10; Stoquerus N5; Palestrina (Jeppesen); Korpus ohne Text, vorläufig",
    params: &[level(
        "level",
        "letzter Ton ohne die letzte Silbe des Textes",
        "the last note does not carry the last syllable of the text",
    )],
};

static MELISMA: RuleInfo = RuleInfo {
    id: "TXT-MELISMA",
    title_de: "Melisma an falscher Stelle oder zu lang",
    title_en: "Melisma misplaced or too long",
    source: "Palestrina (Jeppesen, Style S. 38); Zarlino 1589 (die Zahl der Töne folgt dem Akzent); Stoquerus XX.3, XXV, XXIX.4; Andrews, Quadflieg VIII; Höchstzahlen nach Jeppesens Unterlegung von Dies sanctificatus; Korpus ohne Text, vorläufig",
    params: &[
        level(
            "unstressed",
            "Melisma auf einer unbetonten Silbe (nicht am Beginn, nicht am Ende eines Abschnitts)",
            "a melisma on an unaccented syllable (not opening, not the last of a unit)",
        ),
        level(
            "function_word",
            "Melisma auf einem Funktionswort (et, in, qui …)",
            "a melisma on a function word (et, in, qui …)",
        ),
        level(
            "last_syllable",
            "Melisma auf der letzten Silbe eines Sinnabschnitts (außer einem Auftakt in Vierteln im Takt vor dem letzten Ton)",
            "a melisma on the last syllable of a unit (except a lead-in in quarters within the measure before its last note)",
        ),
        level(
            "long",
            "je Ton einer Silbe über ihre Höchstzahl hinaus",
            "per note of a syllable beyond its maximum",
        ),
        int(
            "max_short",
            "Höchstzahl der Töne einer betonten Silbe in kurzen Texten",
            "most notes on an accented syllable in short texts",
        ),
        int(
            "max_long",
            "Höchstzahl der Töne einer betonten Silbe in langen Texten",
            "most notes on an accented syllable in long texts",
        ),
        int(
            "max_weak",
            "Höchstzahl der Töne einer unbetonten Silbe, die weder eröffnet noch einen Sinnabschnitt beendet",
            "most notes on an unaccented syllable that neither opens a part of the text nor ends a unit",
        ),
        int(
            "max_opening",
            "Höchstzahl der Töne einer unbetonten eröffnenden Silbe (e- von eleison)",
            "most notes on an unaccented opening syllable (the e- of eleison)",
        ),
        int(
            "max_last",
            "Höchstzahl der Töne auf der letzten Silbe eines Sinnabschnitts, außer einem Auftakt",
            "most notes on the last syllable of a unit, except a lead-in",
        ),
    ],
};

static ACCENT: RuleInfo = RuleInfo {
    id: "TXT-ACCENT",
    title_de: "Barbarismus: Wortakzent gegen Notenwerte",
    title_en: "Barbarism: word accent against note values",
    source: "Zarlino 1558, IV.32 und IV.33, Regeln 1a und 9b; Zarlino 1588/89; Stoquerus M5; Vicentino 5; Zacconi; Morley; Quadflieg; Palestrina (Jeppesen, Andrews); Korpus ohne Text, vorläufig",
    params: &[
        level(
            "short_penult",
            "Melisma auf der kurzen vorletzten Silbe (Dó-mi-nus)",
            "a melisma on the short penult (Dó-mi-nus)",
        ),
        level(
            "longer",
            "unbetonte Silbe länger als die betonte daneben im selben Wort",
            "an unaccented syllable longer than the stressed one next to it in the same word",
        ),
        level(
            "opening",
            "dasselbe für eine Anfangssilbe (e- vor lé)",
            "the same for an opening syllable (e- before lé)",
        ),
        level(
            "weak",
            "betonte Silbe kürzer als die unbetonte daneben und nicht auf dem Taktanfang, die unbetonte aber darauf",
            "a stressed syllable shorter than the unaccented one next to it and off the downbeat, the unaccented one on it",
        ),
    ],
};

static LEAP: RuleInfo = RuleInfo {
    id: "TXT-LEAP",
    title_de: "Silbenwechsel im Wort über einen großen Sprung",
    title_en: "Syllable change inside a word across a large leap",
    source: "Vicentino 7 (nie über eine Oktave; über eine Quinte „weniger schlecht“, auch Quinten und Quarten besser meiden); Quadflieg I; Palestrina (Jeppesen); Korpus ohne Text, vorläufig",
    params: &[
        level(
            "octave",
            "Silbenwechsel im Wort über eine Septime, eine Oktave oder mehr",
            "a syllable change inside a word across a seventh, an octave or more",
        ),
        level("sixth", "über eine Sexte", "across a sixth"),
        level("fifth", "über eine Quinte", "across a fifth"),
        level("fourth", "über eine Quarte", "across a fourth"),
    ],
};

static CADENCE: RuleInfo = RuleInfo {
    id: "TXT-CADENCE",
    title_de: "Kadenz vor dem Ende des Sinnabschnitts oder Satzes",
    title_en: "Cadence before the unit or sentence ends",
    source: "Zarlino 1558, IV.32 (der Punkt); Lanfranco 1; Morley; Korpus ohne Text, vorläufig",
    params: &[
        level(
            "level",
            "Binnenkadenz weder auf der letzten Silbe eines Sinnabschnitts noch vor einer Zäsur (| oder /)",
            "an internal cadence neither on the last syllable of a unit nor before a marked break (| or /)",
        ),
        level(
            "principal",
            "nur Ende eines Sinnabschnitts mit Zwischenschluss (, ; :) oder Zäsur, auf der Finalis oder der ersten Kadenzstufe",
            "only an intermediate unit ends or a marked break comes, on the final or the first cadence degree",
        ),
        level(
            "comma",
            "nur Ende eines Sinnabschnitts mit Zwischenschluss oder Zäsur, auf einer anderen Stufe",
            "only an intermediate unit ends or a marked break comes, on another degree",
        ),
    ],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ParamKind;
    use crate::text::underlay::Case;

    /// Every case of the underlay model carries a level parameter of its rule, and every level
    /// parameter of the TXT rules has a case.
    #[test]
    fn cases_match_the_rules() {
        let infos: Vec<&RuleInfo> = rules().iter().map(|r| r.info()).collect();
        for case in Case::ALL {
            let info = infos
                .iter()
                .find(|info| info.id == case.rule())
                .unwrap_or_else(|| panic!("{case:?}: no rule {}", case.rule()));
            let param = info.param(case.param()).unwrap();
            assert_eq!(param.kind, ParamKind::Level, "{case:?}");
            if let Some(limit) = case.limit() {
                assert_eq!(info.param(limit).unwrap().kind, ParamKind::Int, "{case:?}");
            }
        }
        for info in infos {
            for param in info.params.iter().filter(|p| p.kind == ParamKind::Level) {
                assert!(
                    Case::ALL
                        .iter()
                        .any(|c| c.rule() == info.id && c.param() == param.name),
                    "{} {}",
                    info.id,
                    param.name
                );
            }
        }
    }
}
