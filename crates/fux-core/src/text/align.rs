//! Aligning the lyrics of a score with a text: which syllable of the text each printed syllable
//! sings.
//!
//! A score read from a file has its lyrics as printed strings (`Ky-`, `ri-`, `e`). [`align`]
//! finds the position in the text that each of them stands for and stores it in the note's
//! [`lyric`](crate::score::Note::lyric), so that the text rules can check the underlay.
//! [`find_text`] finds the section of the Ordinary when the user names no text.
//!
//! **Comparing syllables.** Syllables are compared in a plain form: lowercase letters without
//! accents, punctuation or hyphens, with j and y read as i, æ, œ and oe as ae, and without h. So
//! `Ký-`, `Ky-` and `KY` are the same syllable, and so are the spellings of older prints `coe-`
//! and `cae`, `ley-` and `léi`, `O-` and `Ho`.
//!
//! **Aligning a voice.** Each voice is aligned on its own by a small dynamic program over the
//! positions of the text. Going on to the next position is free, and so is a restart at a
//! restart target right after the end of a unit ([`Text::restart_targets`]), so Ký-ri-e
//! e-lé-i-son e-lé-i-son costs nothing. Any other jump costs 1. So does a syllable that is
//! taken for a text syllable written differently, or for none at all. Among the cheapest
//! alignments, the one with the fewest syllables not found as written wins. Further ties go,
//! in this order, to the next position, to the earliest restart target, to leaving a syllable
//! unknown, and to a jump to the earliest position. So a syllable spelled differently is taken
//! for the syllable at its place (Jeppesen's o-mni-po-ten-tem for o-mni-pot-én-tem), and only a
//! syllable that fits nowhere stays [`Lyric::UNKNOWN`].

use std::fmt;
use std::ops::Add;
use std::sync::Arc;

use super::{Eleison, Text, TextOptions, latin, ordinary};
use crate::lang::Lang;
use crate::score::{Lyric, Score, ScoreText, Voice};

/// A text counts as recognised if at most one syllable in this many is unmatched: 5 %.
const RECOGNISED: usize = 20;

/// The most syllables a warning lists.
const LISTED: usize = 6;

/// A syllable as written in the score, and where it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Written {
    /// Index of its note in the voice.
    pub note: usize,
    /// The measure, counted from 1.
    pub measure: usize,
    /// The syllable as written under the note.
    pub syllable: String,
}

/// What [`align`] reports about the lyrics of a score.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlignWarning {
    /// No voice carries lyrics, apart from a cantus firmus: the score keeps no text, and the
    /// text rules stay silent.
    NoLyrics,
    /// Syllables of a voice that fit no syllable of the text; their notes get
    /// [`Lyric::UNKNOWN`].
    Unknown {
        /// Index of the voice.
        voice: usize,
        /// Name of the voice.
        name: String,
        syllables: Vec<Written>,
    },
    /// Syllables of a voice taken, by their place, for a syllable of the text that is written
    /// differently; that syllable is given in its book form: `po-` for `pot` in Jeppesen's
    /// o-mni-po-ten-tem.
    Respelled {
        /// Index of the voice.
        voice: usize,
        /// Name of the voice.
        name: String,
        syllables: Vec<(Written, String)>,
    },
}

impl AlignWarning {
    /// Message for users in their language.
    pub fn message(&self, lang: Lang) -> String {
        match self {
            AlignWarning::NoLyrics => lang
                .pick(
                    "Keine Stimme trägt Text; die Textregeln bleiben ungeprüft.",
                    "No voice carries lyrics; the text rules are not checked.",
                )
                .to_string(),
            AlignWarning::Unknown {
                name, syllables, ..
            } => {
                let list = listing(
                    syllables.iter().map(|w| match lang {
                        Lang::De => format!("„{}“ (Takt {})", w.syllable, w.measure),
                        Lang::En => format!("\"{}\" (measure {})", w.syllable, w.measure),
                    }),
                    syllables.len(),
                    lang,
                );
                let count = count(syllables.len(), lang);
                match lang {
                    Lang::De => format!("{name}: {count} nicht im Text: {list}."),
                    Lang::En => format!("{name}: {count} not in the text: {list}."),
                }
            }
            AlignWarning::Respelled {
                name, syllables, ..
            } => {
                let list = listing(
                    syllables.iter().map(|(w, book)| match lang {
                        Lang::De => format!("„{}“ als „{book}“ (Takt {})", w.syllable, w.measure),
                        Lang::En => {
                            format!("\"{}\" as \"{book}\" (measure {})", w.syllable, w.measure)
                        }
                    }),
                    syllables.len(),
                    lang,
                );
                let count = count(syllables.len(), lang);
                match lang {
                    Lang::De => format!(
                        "{name}: {count} anders geschrieben als im Text, nach der Stelle zugeordnet: {list}."
                    ),
                    Lang::En => format!(
                        "{name}: {count} written differently from the text, matched by position: {list}."
                    ),
                }
            }
        }
    }
}

impl fmt::Display for AlignWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message(Lang::En))
    }
}

/// „1 Silbe“, „3 Silben“.
fn count(n: usize, lang: Lang) -> String {
    match (lang, n) {
        (Lang::De, 1) => "1 Silbe".into(),
        (Lang::De, _) => format!("{n} Silben"),
        (Lang::En, 1) => "1 syllable".into(),
        (Lang::En, _) => format!("{n} syllables"),
    }
}

/// The first [`LISTED`] of `len` items, and how many more there are.
fn listing(items: impl Iterator<Item = String>, len: usize, lang: Lang) -> String {
    let mut list = items.take(LISTED).collect::<Vec<_>>().join(", ");
    if len > LISTED {
        let more = len - LISTED;
        list.push_str(&match lang {
            Lang::De => format!(" und {more} weitere"),
            Lang::En => format!(" and {more} more"),
        });
    }
    list
}

/// How well a text fits the lyrics of a score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fit {
    /// Syllables that do not match where [`align`] puts them: written differently, in no place
    /// of the text, or out of order (reached by a jump).
    pub unmatched: usize,
    /// All syllables of the voices that sing.
    pub syllables: usize,
}

impl Fit {
    /// Whether the text counts as recognised: the voices carry syllables, and at most 5 % of
    /// them are unmatched.
    pub fn recognised(self) -> bool {
        self.syllables > 0 && self.unmatched * RECOGNISED <= self.syllables
    }
}

/// Aligns the lyrics of a score with a text. Every note that carries a syllable gets the
/// position it sings as its [`lyric`](crate::score::Note::lyric), or [`Lyric::UNKNOWN`] if the
/// syllable fits nowhere; all other notes get none. [`Score::text`] becomes the text, with the
/// voices that carry lyrics marked as singing it. A cantus firmus never sings: its syllables,
/// such as a chant's own text, are not aligned. The printed syllables stay as they are.
///
/// If no voice carries lyrics, the score keeps no text and the only warning is
/// [`AlignWarning::NoLyrics`]. Otherwise the warnings name, voice by voice, the syllables that fit
/// nowhere and those taken by their place for a syllable written differently.
pub fn align(score: &mut Score, text: Arc<Text>) -> Vec<AlignWarning> {
    let target = Target::new(&text);
    let meter = score.meter;
    let mut warnings = Vec::new();
    let mut sings = Vec::with_capacity(score.voices.len());
    for (v, voice) in score.voices.iter_mut().enumerate() {
        let (notes, syllables) = sung(voice);
        sings.push(!notes.is_empty());
        let starts: Vec<u32> = voice.timed().map(|(start, _)| start).collect();
        for note in &mut voice.notes {
            note.lyric = None;
        }
        if notes.is_empty() {
            continue;
        }
        let (moves, _) = place(&target, &syllables);
        let (mut unknown, mut respelled) = (Vec::new(), Vec::new());
        for ((&note, syllable), step) in notes.iter().zip(&syllables).zip(moves) {
            let written = Written {
                note,
                measure: meter.locate(starts[note]).0 + 1,
                syllable: voice.notes[note]
                    .syllable
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            };
            voice.notes[note].lyric = Some(match step {
                Move::To { position, .. } => {
                    if target.plain[position] != *syllable {
                        respelled.push((written, text.syllables[position].text.clone()));
                    }
                    Lyric::new(position)
                }
                Move::Unknown => {
                    unknown.push(written);
                    Lyric::UNKNOWN
                }
            });
        }
        if !unknown.is_empty() {
            warnings.push(AlignWarning::Unknown {
                voice: v,
                name: voice.name.clone(),
                syllables: unknown,
            });
        }
        if !respelled.is_empty() {
            warnings.push(AlignWarning::Respelled {
                voice: v,
                name: voice.name.clone(),
                syllables: respelled,
            });
        }
    }
    if sings.contains(&true) {
        score.text = Some(ScoreText {
            text,
            voices: sings,
        });
    } else {
        score.text = None;
        warnings.push(AlignWarning::NoLyrics);
    }
    warnings
}

/// How well a text fits the lyrics of a score, aligned as [`align`] would align them, without
/// changing the score. [`find_text`] compares the sections of the Ordinary by it; a frontend can
/// choose the division of eleison for a section the user names.
pub fn fit(score: &Score, text: &Text) -> Fit {
    let voices = sung_voices(score);
    let (unmatched, _) = assess(&Target::new(text), &voices);
    Fit {
        unmatched,
        syllables: voices.iter().map(Vec::len).sum(),
    }
}

/// Finds the section of the Ordinary, or the units of one, that the lyrics of a score set. Every
/// section is tried, with eleison in four syllables and in three, and the one with the fewest
/// unmatched syllables is taken (see [`Fit`]). Among equals the cheaper alignment wins, then the
/// earlier section in the Mass, then four syllables. If the voices sing only some of its units,
/// as a setting of `gloria:1-5` does, the section is narrowed to them: from the first to the last
/// unit in which a voice sings a syllable in order, unless that leaves more syllables unmatched.
/// A section of which every voice stops at the same unit therefore counts as a range too. Gives
/// the text, its division of eleison and how well it fits.
///
/// `None` if no voice carries lyrics, or if even the best section leaves more than 5 % of the
/// syllables unmatched; then the user has to name the text.
pub fn find_text(score: &Score) -> Option<(Arc<Text>, Eleison, Fit)> {
    let voices = sung_voices(score);
    let syllables = voices.iter().map(Vec::len).sum();
    if syllables == 0 {
        return None;
    }
    let four = ordinary::sections(&TextOptions {
        eleison: Eleison::Four,
    });
    let three = ordinary::sections(&TextOptions {
        eleison: Eleison::Three,
    });
    let mut best: Option<((usize, u32), &Text, Eleison)> = None;
    for (in_four, in_three) in four.iter().zip(three) {
        let variants = [(in_four, Eleison::Four), (in_three, Eleison::Three)];
        // Most sections have no eleison and come out the same twice.
        let distinct = usize::from(in_three.syllables != in_four.syllables);
        for &(text, eleison) in &variants[..1 + distinct] {
            let key = assess(&Target::new(text), &voices);
            if best.is_none_or(|(least, ..)| key < least) {
                best = Some((key, text, eleison));
            }
        }
    }
    let ((unmatched, _), text, eleison) = best?;
    let (text, unmatched) = match narrowed(text, eleison, &voices) {
        Some((range, fewer)) if fewer <= unmatched => (range, fewer),
        _ => (text.clone(), unmatched),
    };
    let fit = Fit {
        unmatched,
        syllables,
    };
    fit.recognised().then(|| (Arc::new(text), eleison, fit))
}

/// The units of a section that the voices sing: from the first to the last unit in which a voice
/// sings a syllable in order and as written, as a range of the section with its unmatched
/// syllables. `None` if that is the whole section, or no syllable matches.
fn narrowed(text: &Text, eleison: Eleison, voices: &[Vec<String>]) -> Option<(Text, usize)> {
    let target = Target::new(text);
    let mut range: Option<(usize, usize)> = None;
    for syllables in voices {
        let (moves, _) = place(&target, syllables);
        for (&step, syllable) in moves.iter().zip(syllables) {
            if let Move::To { position, .. } = step
                && target.matches(step, syllable)
            {
                let unit = usize::from(text.syllables[position].unit);
                range = Some(range.map_or((unit, unit), |(a, b)| (a.min(unit), b.max(unit))));
            }
        }
    }
    let (first, last) = range?;
    if first == 0 && last + 1 == text.units.len() {
        return None;
    }
    let spec = format!("{}:{}-{}", text.id, first + 1, last + 1);
    let (units, _) = super::resolve(&spec, &TextOptions { eleison }).ok()?;
    let (unmatched, _) = assess(&Target::new(&units), voices);
    Some((units, unmatched))
}

/// The plain form in which syllables are compared: lowercase letters only, without accents,
/// punctuation or hyphens, with j and y as i, æ, œ and oe as ae, and without h.
fn plain(syllable: &str) -> String {
    let mut out = String::with_capacity(syllable.len());
    for c in latin::normalize(syllable).chars() {
        match base_letter(c) {
            Some('y') => out.push('i'),
            Some('h') | None => {}
            Some(letter) => out.push(letter),
        }
    }
    out.replace("oe", "ae")
}

/// A lowercase letter without its accent; `None` for anything else.
fn base_letter(c: char) -> Option<char> {
    Some(match c {
        'a'..='z' => c,
        'à' | 'â' | 'ä' => 'a',
        'è' | 'ê' | 'ë' => 'e',
        'ì' | 'î' | 'ï' => 'i',
        'ò' | 'ô' | 'ö' => 'o',
        'ù' | 'û' | 'ü' => 'u',
        'ÿ' => 'y',
        _ => return None,
    })
}

/// The notes of a voice that carry a syllable, and the plain forms of their syllables. A
/// syllable without letters counts as none. A cantus firmus never sings the text, so it has
/// none.
fn sung(voice: &Voice) -> (Vec<usize>, Vec<String>) {
    if voice.is_cantus_firmus() {
        return (Vec::new(), Vec::new());
    }
    voice
        .notes
        .iter()
        .enumerate()
        .filter_map(|(i, note)| {
            let plain = plain(note.syllable.as_deref()?);
            (!plain.is_empty()).then_some((i, plain))
        })
        .unzip()
}

/// The plain syllables of each voice that sings.
fn sung_voices(score: &Score) -> Vec<Vec<String>> {
    score
        .voices
        .iter()
        .map(|voice| sung(voice).1)
        .filter(|syllables| !syllables.is_empty())
        .collect()
}

/// The unmatched syllables of the voices aligned with a text, and the cost of the alignments.
fn assess(target: &Target, voices: &[Vec<String>]) -> (usize, u32) {
    voices.iter().fold((0, 0), |(unmatched, cost), syllables| {
        let (moves, total) = place(target, syllables);
        let missed = moves
            .iter()
            .zip(syllables)
            .filter(|&(&step, syllable)| !target.matches(step, syllable))
            .count();
        (unmatched + missed, cost + total.cost)
    })
}

/// The cost of an alignment: jumps and syllables not found as written, then these syllables
/// alone, which decides between equally cheap alignments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Cost {
    cost: u32,
    mismatches: u32,
}

impl Add for Cost {
    type Output = Cost;

    fn add(self, other: Cost) -> Cost {
        Cost {
            cost: self.cost + other.cost,
            mismatches: self.mismatches + other.mismatches,
        }
    }
}

/// A syllable found as written.
const FOUND: Cost = Cost {
    cost: 0,
    mismatches: 0,
};
/// A jump that is neither to the next position nor a restart after a unit end.
const JUMP: Cost = Cost {
    cost: 1,
    mismatches: 0,
};
/// A syllable taken for one written differently, or for none.
const MISMATCH: Cost = Cost {
    cost: 1,
    mismatches: 1,
};

/// Where the alignment puts a syllable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Move {
    /// At a position of the text; `free` if it is the next position, or a restart target right
    /// after the end of a unit.
    To { position: usize, free: bool },
    /// Nowhere: the syllable is unknown.
    Unknown,
}

/// A text as the alignment sees it.
struct Target {
    /// The plain form of each syllable.
    plain: Vec<String>,
    /// The restart targets after each position; empty unless it ends a unit.
    restarts: Vec<Vec<usize>>,
}

impl Target {
    fn new(text: &Text) -> Target {
        Target {
            plain: text.syllables.iter().map(|s| plain(&s.text)).collect(),
            restarts: (0..text.syllables.len())
                .map(|p| text.restart_targets(p).collect())
                .collect(),
        }
    }

    /// Whether a syllable placed by `step` matches: in order, and written as in the text.
    fn matches(&self, step: Move, syllable: &str) -> bool {
        matches!(step, Move::To { position, free: true } if self.plain[position] == syllable)
    }

    /// Fills `via` with the cost of taking `syllable` for each position, the syllables after
    /// it included (`after` holds their least costs by state). Returns the least of these with
    /// the earliest position among equals, the target of a jump; `None` for an empty text.
    fn via(&self, syllable: &str, after: &[Cost], via: &mut [Cost]) -> Option<(Cost, usize)> {
        let mut least: Option<(Cost, usize)> = None;
        for (position, (plain, cost)) in self.plain.iter().zip(via.iter_mut()).enumerate() {
            let here = if plain == syllable { FOUND } else { MISMATCH };
            *cost = here + after[position + 1];
            if least.is_none_or(|(c, _)| *cost < c) {
                least = Some((*cost, position));
            }
        }
        least
    }

    /// The best move from `state`, which is 0 before the first position and p + 1 after
    /// position p, with its cost. Among equal costs the next position comes first, then the
    /// restart targets in order, then an unknown syllable, then a jump.
    fn best(
        &self,
        state: usize,
        via: &[Cost],
        after: &[Cost],
        jump: Option<(Cost, usize)>,
    ) -> (Cost, Move) {
        let mut best = None;
        let mut offer = |cost: Cost, step: Move| {
            if best.is_none_or(|(least, _)| cost < least) {
                best = Some((cost, step));
            }
        };
        if let Some(&cost) = via.get(state) {
            offer(
                cost,
                Move::To {
                    position: state,
                    free: true,
                },
            );
        }
        if let Some(p) = state.checked_sub(1) {
            for &position in &self.restarts[p] {
                offer(
                    via[position],
                    Move::To {
                        position,
                        free: true,
                    },
                );
            }
        }
        offer(MISMATCH + after[state], Move::Unknown);
        // A jump to the next position or to a restart target is never the best: going there
        // freely costs less.
        if let Some((cost, position)) = jump {
            offer(
                JUMP + cost,
                Move::To {
                    position,
                    free: false,
                },
            );
        }
        best.expect("an unknown syllable is always possible")
    }
}

/// Aligns the plain syllables of one voice with a text: the move for each syllable, and the
/// cost of the whole alignment.
fn place(target: &Target, syllables: &[String]) -> (Vec<Move>, Cost) {
    // States: 0 before the first position, p + 1 after position p.
    let width = target.plain.len() + 1;
    // Row i holds the least cost of the syllables from i on, by state; the last row is zero.
    let mut least = vec![Cost::default(); (syllables.len() + 1) * width];
    let mut via = vec![Cost::default(); target.plain.len()];
    for (i, syllable) in syllables.iter().enumerate().rev() {
        let (row, after) = least[i * width..].split_at_mut(width);
        let after = &after[..width];
        let jump = target.via(syllable, after, &mut via);
        for (state, cost) in row.iter_mut().enumerate() {
            *cost = target.best(state, &via, after, jump).0;
        }
    }
    // Forward along the least costs, taking the first best move at each syllable.
    let mut moves = Vec::with_capacity(syllables.len());
    let mut state = 0;
    for (i, syllable) in syllables.iter().enumerate() {
        let after = &least[(i + 1) * width..(i + 2) * width];
        let jump = target.via(syllable, after, &mut via);
        let (_, step) = target.best(state, &via, after, jump);
        if let Move::To { position, .. } = step {
            state = position + 1;
        }
        moves.push(step);
    }
    (moves, least[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clef::VoiceType;
    use crate::generate::Rng;
    use crate::mode::{Mode, ModeSpec};
    use crate::parse::parse_lilypond;
    use crate::score::{Note, Role, Species};
    use crate::text::resolve;
    use crate::time::Duration;

    fn half() -> Note {
        parse_lilypond("d'2", Duration::WHOLE).unwrap().remove(0)
    }

    /// A free voice of half notes, one per token: a syllable as written, or `_` for a note
    /// without one.
    fn voice(voice_type: VoiceType, syllables: &str) -> Voice {
        let notes = syllables
            .split_whitespace()
            .map(|token| Note {
                syllable: (token != "_").then(|| token.to_string()),
                ..half()
            })
            .collect();
        Voice::new(voice_type, Role::Counterpoint(Species::Free), notes)
    }

    /// A score of the voices, each filled up with notes to the same whole number of measures.
    fn score(mut voices: Vec<Voice>) -> Score {
        let len = voices
            .iter()
            .map(|v| v.notes.len())
            .max()
            .unwrap_or(0)
            .next_multiple_of(2);
        for voice in &mut voices {
            voice.notes.resize(len, half());
        }
        Score::new(ModeSpec::new(Mode::Dorian), voices)
    }

    fn text(spec: &str, eleison: Eleison) -> Arc<Text> {
        Arc::new(resolve(spec, &TextOptions { eleison }).unwrap().0)
    }

    /// The lyrics of a voice up to its last one, a token per note: the position, `?` for an
    /// unknown syllable, `_` for none.
    fn lyrics(score: &Score, v: usize) -> String {
        let tokens: Vec<String> = score.voices[v]
            .notes
            .iter()
            .map(|note| {
                note.lyric.map_or("_".into(), |lyric| {
                    lyric.index().map_or("?".into(), |i| i.to_string())
                })
            })
            .collect();
        tokens.join(" ").trim_end_matches([' ', '_']).to_string()
    }

    fn written(note: usize, measure: usize, syllable: &str) -> Written {
        Written {
            note,
            measure,
            syllable: syllable.into(),
        }
    }

    #[test]
    fn plain_forms() {
        let forms = [
            ("Ký-", "ki"),
            ("KY", "ki"),
            ("Ky-", "ki"),
            ("son.", "son"),
            ("e,", "e"),
            ("Chri-", "cri"),
            ("coe-", "cae"),
            ("cœ-", "cae"),
            ("cæ-", "cae"),
            ("ley-", "lei"),
            ("léi-", "lei"),
            ("Ho-", "o"),
            ("Je-", "ie"),
            ("Iè-", "ie"),
            ("„Pax“", "pax"),
            ("-", ""),
            ("_", ""),
        ];
        for (written, expected) in forms {
            assert_eq!(plain(written), expected, "{written}");
        }
    }

    /// Kyrie I as a MusicXML file gives it: hyphens, book forms or plain ones, punctuation, any
    /// case. The voices restate the text and its tail; the cantus firmus does not sing.
    #[test]
    fn kyrie_with_restatements() {
        let mut cantus = voice(VoiceType::Bass, "A- men. _");
        cantus.role = Role::CantusFirmus;
        cantus.notes[2].lyric = Some(Lyric::new(3));
        let mut score = score(vec![
            voice(
                VoiceType::Soprano,
                "Ky- ri- e _ e- lé- i- son. _ Ky- ri- e e- _ lé- i- son, e- le- i- son.",
            ),
            voice(
                VoiceType::Alto,
                "Ký- _ ri- e e- lé- _ i- son. e- lé- i- son.",
            ),
            voice(VoiceType::Tenor, "_ KY- RI- E, E- LE- I- SON!"),
            cantus,
        ]);
        let kyrie = text("kyrie", Eleison::Four);
        assert_eq!(align(&mut score, kyrie.clone()), []);
        assert_eq!(
            lyrics(&score, 0),
            "0 1 2 _ 3 4 5 6 _ 0 1 2 3 _ 4 5 6 3 4 5 6"
        );
        assert_eq!(lyrics(&score, 1), "0 _ 1 2 3 4 _ 5 6 3 4 5 6");
        assert_eq!(lyrics(&score, 2), "_ 0 1 2 3 4 5 6");
        // The cantus keeps its own syllables and loses its lyric.
        assert_eq!(lyrics(&score, 3), "");
        assert_eq!(score.voices[3].notes[0].syllable.as_deref(), Some("A-"));
        let sung = score.text.as_ref().unwrap();
        assert_eq!(sung.text, kyrie);
        assert_eq!(sung.voices, [true, true, true, false]);
        assert_eq!(score.validate(), Ok(()));
        assert_eq!(
            fit(&score, &kyrie),
            Fit {
                unmatched: 0,
                syllables: 36
            }
        );
        // The section is found, as Kyrie I rather than the same text of Kyrie II.
        let (found, eleison, how) = find_text(&score).unwrap();
        assert_eq!((found.id.as_str(), eleison), ("kyrie", Eleison::Four));
        assert_eq!(how.unmatched, 0);

        // A voice that enters with the tail jumps, but is aligned as before.
        let mut tail = self::score(vec![voice(VoiceType::Alto, "e- lé- i- son.")]);
        assert_eq!(align(&mut tail, kyrie.clone()), []);
        assert_eq!(lyrics(&tail, 0), "3 4 5 6");
        assert_eq!(fit(&tail, &kyrie).unmatched, 1);
    }

    /// Jeppesen's e-lei-son matches the division in three syllables. Against four, lei- stands
    /// for lé, and son jumps over i.
    #[test]
    fn three_syllable_eleison() {
        let mut score = score(vec![voice(
            VoiceType::Tenor,
            "Ky- ri- e e- lei- _ son. e- lei- son.",
        )]);
        let three = text("kyrie", Eleison::Three);
        let four = text("kyrie", Eleison::Four);
        let mut by_four = score.clone();
        assert_eq!(align(&mut score, three.clone()), []);
        assert_eq!(lyrics(&score, 0), "0 1 2 3 4 _ 5 3 4 5");

        let warnings = align(&mut by_four, four.clone());
        assert_eq!(lyrics(&by_four, 0), "0 1 2 3 4 _ 6 3 4 6");
        assert_eq!(
            warnings,
            [AlignWarning::Respelled {
                voice: 0,
                name: "Tenor".into(),
                syllables: vec![
                    (written(4, 3, "lei-"), "lé".into()),
                    (written(8, 5, "lei-"), "lé".into()),
                ],
            }]
        );
        assert_eq!(
            warnings[0].message(Lang::De),
            "Tenor: 2 Silben anders geschrieben als im Text, nach der Stelle zugeordnet: \
             „lei-“ als „lé“ (Takt 3), „lei-“ als „lé“ (Takt 5)."
        );
        assert_eq!(
            warnings[0].message(Lang::En),
            "Tenor: 2 syllables written differently from the text, matched by position: \
             \"lei-\" as \"lé\" (measure 3), \"lei-\" as \"lé\" (measure 5)."
        );
        // Both lei- are written differently, both son jump.
        assert_eq!(fit(&score, &four).unmatched, 4);
        assert_eq!(fit(&score, &three).unmatched, 0);
        let (found, eleison, _) = find_text(&score).unwrap();
        assert_eq!((found.as_ref(), eleison), (three.as_ref(), Eleison::Three));
    }

    /// A range of the Gloria is a text of its own, counted from 0. Restating the previous
    /// sentence jumps; so does entering with a later unit.
    #[test]
    fn gloria_range() {
        let range = text("gloria:2-4", Eleison::Four);
        let mut score = score(vec![
            voice(
                VoiceType::Soprano,
                "Lau- da- mus te. Lau- da- mus te. Be- ne- di- ci- mus te. Ad- o- ra- mus te.",
            ),
            voice(
                VoiceType::Bass,
                "_ Be- ne- _ di- ci- mus te. Lau- da- mus te. Ad- o- ra- mus te.",
            ),
        ]);
        assert_eq!(align(&mut score, range.clone()), []);
        assert_eq!(
            lyrics(&score, 0),
            "0 1 2 3 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14"
        );
        assert_eq!(lyrics(&score, 1), "_ 4 5 _ 6 7 8 9 0 1 2 3 10 11 12 13 14");
        assert_eq!(score.text.as_ref().unwrap().text.id, "gloria:2-4");
        // The bass enters with a jump, goes back to Laudamus te and on to Adoramus te.
        assert_eq!(fit(&score, &range).unmatched, 3);

        // In the whole Gloria the same voices sing from position 15 on: Laudamus te.
        let gloria = text("gloria", Eleison::Four);
        let mut whole = score.clone();
        assert_eq!(align(&mut whole, gloria.clone()), []);
        assert_eq!(
            lyrics(&whole, 0),
            "15 16 17 18 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29"
        );
        assert_eq!(fit(&whole, &gloria).unmatched, 4);
    }

    /// Without a text named, voices that sing only some units of a section are recognised as
    /// that range: a setting of `gloria:1-2` does not lack the rest of the Gloria, and one of
    /// `gloria:2-4` does not enter with a jump.
    #[test]
    fn ranges_are_recognised() {
        let first = "Et in ter- ra pax ho- mi- ni- bus bo- nae vo- lun- ta- tis. Lau- da- mus te.";
        let score = score(vec![
            voice(VoiceType::Soprano, first),
            voice(VoiceType::Tenor, &format!("_ {first}")),
        ]);
        let (found, eleison, how) = find_text(&score).unwrap();
        assert_eq!((found.id.as_str(), eleison), ("gloria:1-2", Eleison::Four));
        assert_eq!(how.unmatched, 0);
        assert_eq!(found.name(Lang::De), "Gloria: Et in terra 1–2");
        let middle = "Lau- da- mus te. Be- ne- di- ci- mus te. Ad- o- ra- mus te.";
        let score = self::score(vec![
            voice(VoiceType::Alto, middle),
            voice(VoiceType::Bass, &format!("{middle} Ad- o- ra- mus te.")),
        ]);
        let (found, _, how) = find_text(&score).unwrap();
        assert_eq!((found.id.as_str(), how.unmatched), ("gloria:2-4", 0));
        // The whole Gloria would leave both entries unmatched, more than 5 % of 30 syllables.
        let gloria = text("gloria", Eleison::Four);
        assert_eq!(fit(&score, &gloria).unmatched, 2);
        // A section the voices sing to its end stays whole: Kyrie I has a single unit.
        let kyrie = self::score(vec![voice(VoiceType::Tenor, "Ky- ri- e e- lé- i- son.")]);
        assert_eq!(find_text(&kyrie).unwrap().0.id, "kyrie");
    }

    /// Jeppesen's tenor of the Credo of Missa Dies sanctificatus: o-mni-po-ten-tem has
    /// different syllables from o-mni-pot-én-tem and is matched by position; coe-li is the old
    /// spelling of cae-li.
    #[test]
    fn jeppesen_spellings() {
        let mut score = score(vec![
            voice(
                VoiceType::Tenor,
                "Pa- trem o- mni- po- ten- tem, fa- cto- rem coe- li et ter- _ _ _ rae, vi- si- \
                 bi- li-",
            ),
            voice(
                VoiceType::Alto,
                "_ _ _ _ _ _ _ Fa- cto- rem coe- _ _ li et ter- rae, vi- si-",
            ),
        ]);
        let credo = text("credo", Eleison::Four);
        let warnings = align(&mut score, credo.clone());
        assert_eq!(
            lyrics(&score, 0),
            "0 1 2 3 4 5 6 7 8 9 10 11 12 13 _ _ _ 14 15 16 17 18"
        );
        // The alto enters with fa-cto-rem, a jump.
        assert_eq!(
            lyrics(&score, 1),
            "_ _ _ _ _ _ _ 7 8 9 10 _ _ 11 12 13 14 15 16"
        );
        assert_eq!(
            warnings,
            [AlignWarning::Respelled {
                voice: 0,
                name: "Tenor".into(),
                syllables: vec![
                    (written(4, 3, "po-"), "pot".into()),
                    (written(5, 3, "ten-"), "én".into()),
                ],
            }]
        );
        assert_eq!(fit(&score, &credo).unmatched, 3);
    }

    /// A syllable that fits nowhere stays unknown; one in the place of another is taken by its
    /// place.
    #[test]
    fn unknown_syllables() {
        let kyrie = text("kyrie", Eleison::Four);
        let mut score = score(vec![
            voice(VoiceType::Soprano, "xx Ky- ri- e ij e- lé- i- son."),
            voice(VoiceType::Alto, "Ky- ri- a e- lé- i- son."),
        ]);
        let warnings = align(&mut score, kyrie.clone());
        assert_eq!(lyrics(&score, 0), "? 0 1 2 ? 3 4 5 6");
        assert_eq!(lyrics(&score, 1), "0 1 2 3 4 5 6");
        assert_eq!(
            warnings,
            [
                AlignWarning::Unknown {
                    voice: 0,
                    name: "Sopran".into(),
                    syllables: vec![written(0, 1, "xx"), written(4, 3, "ij")],
                },
                AlignWarning::Respelled {
                    voice: 1,
                    name: "Alt".into(),
                    syllables: vec![(written(2, 2, "a"), "e".into())],
                },
            ]
        );
        assert_eq!(
            warnings[0].message(Lang::De),
            "Sopran: 2 Silben nicht im Text: „xx“ (Takt 1), „ij“ (Takt 3)."
        );
        assert_eq!(
            warnings[0].to_string(),
            "Sopran: 2 syllables not in the text: \"xx\" (measure 1), \"ij\" (measure 3)."
        );
        assert_eq!(
            warnings[1].message(Lang::De),
            "Alt: 1 Silbe anders geschrieben als im Text, nach der Stelle zugeordnet: „a“ als „e“ \
             (Takt 2)."
        );
        // Unknown lyrics are valid.
        assert_eq!(score.validate(), Ok(()));
        assert_eq!(
            fit(&score, &kyrie),
            Fit {
                unmatched: 3,
                syllables: 16
            }
        );

        // A voice in another text is taken position by position; the warning lists six of its
        // syllables.
        let mut motet = self::score(vec![voice(
            VoiceType::Tenor,
            "Si- cut cer- vus de- si- de- rat",
        )]);
        let warnings = align(&mut motet, kyrie);
        assert_eq!(lyrics(&motet, 0), "0 1 2 3 4 5 6 0");
        assert_eq!(
            warnings[0].message(Lang::En),
            "Tenor: 8 syllables written differently from the text, matched by position: \
             \"Si-\" as \"Ký\" (measure 1), \"cut\" as \"ri\" (measure 1), \"cer-\" as \"e\" \
             (measure 2), \"vus\" as \"e\" (measure 2), \"de-\" as \"lé\" (measure 3), \
             \"si-\" as \"i\" (measure 3) and 2 more."
        );
        assert_eq!(find_text(&motet), None);
    }

    /// A score without lyrics, or with lyrics only in its cantus firmus, keeps no text, and any
    /// stale lyrics go.
    #[test]
    fn untexted_score() {
        let mut cantus = voice(VoiceType::Bass, "A- men. _ _");
        cantus.role = Role::CantusFirmus;
        let mut score = score(vec![voice(VoiceType::Soprano, "_ - _ _"), cantus]);
        let kyrie = text("kyrie", Eleison::Four);
        score.text = Some(ScoreText {
            text: kyrie.clone(),
            voices: vec![true, false],
        });
        score.voices[0].notes[1].lyric = Some(Lyric::new(3));
        let before = score.voices.clone();
        assert_eq!(align(&mut score, kyrie.clone()), [AlignWarning::NoLyrics]);
        assert_eq!(score.text, None);
        assert_eq!(lyrics(&score, 0), "");
        assert_eq!(score.voices[1], before[1]);
        assert_eq!(
            AlignWarning::NoLyrics.message(Lang::De),
            "Keine Stimme trägt Text; die Textregeln bleiben ungeprüft."
        );
        assert_eq!(find_text(&score), None);
        let none = fit(&score, &kyrie);
        assert_eq!(
            none,
            Fit {
                unmatched: 0,
                syllables: 0
            }
        );
        assert!(!none.recognised());
    }

    /// A Christe is recognised from its lyrics, also in older spellings, with one misprint in
    /// fewer than twenty syllables, and with Jeppesen's e-lei-son in three syllables.
    #[test]
    fn find_text_on_a_christe() {
        let christe = |eleison: &str| {
            score(vec![
                voice(
                    VoiceType::Soprano,
                    &format!("Chri- _ ste {eleison} Chri- ste {eleison}"),
                ),
                voice(VoiceType::Alto, &format!("Chri- ste {eleison} {eleison}")),
                voice(VoiceType::Bass, &format!("_ Chri- _ _ ste {eleison}")),
            ])
        };
        let mut books = christe("e- lé- _ i- son.");
        let (found, eleison, how) = find_text(&books).unwrap();
        assert_eq!((found.id.as_str(), eleison), ("christe", Eleison::Four));
        assert_eq!(
            how,
            Fit {
                unmatched: 0,
                syllables: 28
            }
        );
        assert!(how.recognised());
        // Aligning with the text found raises no warning.
        assert_eq!(align(&mut books, found), []);

        let (found, eleison, _) = find_text(&christe("e- lei- son,")).unwrap();
        assert_eq!((found.id.as_str(), eleison), ("christe", Eleison::Three));

        // Older spellings are no misprints.
        let mut misprint = christe("e- lé- _ i- son.");
        misprint.voices[0].notes[0].syllable = Some("Chry-".into());
        misprint.voices[1].notes[0].syllable = Some("Cri-".into());
        misprint.voices[2].notes[8].syllable = Some("y-".into());
        assert_eq!(find_text(&misprint).unwrap().2.unmatched, 0);
        // One misprint in 28 syllables is less than 5 %, two are more.
        misprint.voices[1].notes[1].syllable = Some("sto".into());
        let (found, _, how) = find_text(&misprint).unwrap();
        assert_eq!((found.id.as_str(), how.unmatched), ("christe", 1));
        misprint.voices[0].notes[2].syllable = Some("sti".into());
        assert_eq!(find_text(&misprint), None);
    }

    /// Among equally good moves the next position comes first, then the earliest restart
    /// target.
    #[test]
    fn ties_prefer_the_next_position_then_the_earliest_restart() {
        let sanctus = text("sanctus", Eleison::Four);
        let mut score = score(vec![
            voice(VoiceType::Soprano, "San- ctus, San- ctus,"),
            voice(
                VoiceType::Alto,
                "San- ctus, San- ctus, San- ctus, San- ctus,",
            ),
        ]);
        assert_eq!(align(&mut score, sanctus), []);
        // San- after the first unit: the next position, not the restart.
        assert_eq!(lyrics(&score, 0), "0 1 2 3");
        // A fourth San-ctus restarts after the second unit, at the first of its targets.
        assert_eq!(lyrics(&score, 1), "0 1 2 3 0 1 2 3");

        // After the end of the Agnus Dei an unknown syllable is taken for the first restart
        // target.
        let agnus = text("agnus", Eleison::Four);
        let mut score = self::score(vec![voice(
            VoiceType::Tenor,
            "A- gnus De- i, qui tol- lis pec- ca- ta mun- di: mi- se- re- re no- bis. xx",
        )]);
        let warnings = align(&mut score, agnus);
        assert_eq!(
            lyrics(&score, 0),
            "0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 0"
        );
        assert_eq!(
            warnings,
            [AlignWarning::Respelled {
                voice: 0,
                name: "Tenor".into(),
                syllables: vec![(written(18, 10, "xx"), "A".into())],
            }]
        );

        // A syllable found as written wins over an equally cheap one taken by its place: the i
        // after the end of Kyrie I is a jump back to its i, not a restart at Ký.
        let mut score = self::score(vec![voice(VoiceType::Tenor, "Ky- ri- e e- lé- i- son. i-")]);
        assert_eq!(align(&mut score, text("kyrie", Eleison::Four)), []);
        assert_eq!(lyrics(&score, 0), "0 1 2 3 4 5 6 5");
    }

    /// Syllables on rests and tie continuations are aligned like any other, so the text rules
    /// can report them.
    #[test]
    fn syllables_on_rests_and_ties() {
        let mut notes = parse_lilypond("d'2 r e'~ e' f' g'", Duration::WHOLE).unwrap();
        for (note, syllable) in notes.iter_mut().zip(["Ky-", "ri-", "e", "e-", "lé-", "i-"]) {
            note.syllable = Some(syllable.into());
        }
        let voice = Voice::new(VoiceType::Tenor, Role::Counterpoint(Species::Free), notes);
        let mut score = score(vec![voice]);
        assert_eq!(align(&mut score, text("kyrie", Eleison::Four)), []);
        assert_eq!(lyrics(&score, 0), "0 1 2 3 4 5");
    }

    /// Jeppesen's underlay of Palestrina's Missa Dies sanctificatus (pp. 254–258), one note per
    /// attack: every syllable is found, his spellings by their place, and only the entries he
    /// remarks on jump. The section, or the units of it that an excerpt reaches, is recognised
    /// wherever his text agrees with the books.
    #[test]
    fn jeppesen_underlay_of_dies_sanctificatus() {
        #[derive(serde::Deserialize)]
        struct Fixture {
            excerpt: Vec<Excerpt>,
        }
        #[derive(serde::Deserialize)]
        struct Excerpt {
            id: String,
            text: String,
            eleison: String,
            voice: Vec<Part>,
        }
        #[derive(serde::Deserialize)]
        struct Part {
            name: String,
            syllables: Vec<String>,
        }
        // Jeppesen's underlay is transcribed from his book and not in every checkout; with
        // FUX_REQUIRE_CORPUS set its absence fails the test.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/text/dies-sanctificatus.toml");
        let Ok(fixture) = std::fs::read_to_string(&path) else {
            assert!(
                std::env::var_os("FUX_REQUIRE_CORPUS").is_none_or(|v| v.is_empty() || v == "0"),
                "{} is missing, and FUX_REQUIRE_CORPUS is set",
                path.display()
            );
            println!("skipped: {} is missing", path.display());
            return;
        };
        let fixture: Fixture = toml::from_str(&fixture).unwrap();
        let ids: Vec<&str> = fixture.excerpt.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "christe",
                "gloria",
                "credo",
                "sanctus",
                "sanctus-cantus",
                "incarnatus"
            ]
        );
        for excerpt in &fixture.excerpt {
            let eleison = match excerpt.eleison.as_str() {
                "3" => Eleison::Three,
                _ => Eleison::Four,
            };
            let voices = excerpt
                .voice
                .iter()
                .map(|part| {
                    let mut voice = voice(VoiceType::Tenor, &part.syllables.join(" "));
                    voice.name = part.name.clone();
                    voice
                })
                .collect();
            let mut score = score(voices);
            let sung = text(&excerpt.text, eleison);
            let warnings = align(&mut score, sung.clone());
            let how = fit(&score, &sung);
            let found = find_text(&score);
            if excerpt.id == "credo" {
                // Every attack is a half note here: two to a measure.
                let at = |note: usize, syllable: &str| written(note, note / 2 + 1, syllable);
                let respelled = |voice: usize, name: &str, po: usize| AlignWarning::Respelled {
                    voice,
                    name: name.into(),
                    syllables: vec![
                        (at(po, "po-"), "pot".into()),
                        (at(po + 1, "ten-"), "én".into()),
                    ],
                };
                assert_eq!(
                    warnings,
                    [respelled(0, "Bassus", 8), respelled(1, "Tenor", 4)]
                );
                // po- and ten- twice, the entries of altus and cantus with fa-ctó-rem and the
                // bassus, which leaves out fa-ctó-rem cae-li et ter-rae: 7 of 62 syllables.
                assert_eq!(
                    how,
                    Fit {
                        unmatched: 7,
                        syllables: 62
                    }
                );
                assert_eq!(found, None);
                // The bassus comes back with vi-si-bí-li-um.
                assert_eq!(lyrics(&score, 0).split(' ').nth(15), Some("15"));
            } else {
                assert_eq!(warnings, [], "{}", excerpt.id);
                assert_eq!(how.unmatched, 0, "{}", excerpt.id);
                // Each excerpt breaks off: the section is narrowed to the units it reaches.
                let (found, found_eleison, _) = found.expect(&excerpt.id);
                let units = match excerpt.id.as_str() {
                    "gloria" => "gloria:1-3",
                    "sanctus" => "sanctus:1-2",
                    "incarnatus" => "incarnatus:1",
                    _ => excerpt.text.as_str(),
                };
                assert_eq!(found.id, units, "{}", excerpt.id);
                assert_eq!(found_eleison, eleison, "{}", excerpt.id);
            }
        }
    }

    /// The alignment is the cheapest one, and among equally cheap ones the first in the order
    /// of preference, as checked against every possible alignment of short random voices.
    #[test]
    fn alignment_is_the_best_of_all() {
        const WORDS: [&str; 6] = ["la", "le", "li", "la-le", "le-la", "li-la-le"];
        const BETWEEN: [&str; 6] = [" ", ", ", ". ", " / ", " | ", ": / "];
        const SUNG: [&str; 4] = ["la", "le", "li", "lo"];
        let mut rng = Rng::new(12);
        let mut pick = |n: usize| (rng.next_u64() % n as u64) as usize;
        let mut cases = 0;
        while cases < 300 {
            let words = 1 + pick(3);
            let mut src = String::new();
            for w in 0..words {
                if w > 0 {
                    src.push_str(BETWEEN[pick(BETWEEN.len())]);
                }
                src.push_str(WORDS[pick(WORDS.len())]);
            }
            let (text, _) = Text::own(&src, &TextOptions::default()).unwrap();
            if text.syllables.len() > 5 {
                continue;
            }
            cases += 1;
            let target = Target::new(&text);
            let syllables: Vec<String> = (0..1 + pick(5))
                .map(|_| SUNG[pick(SUNG.len())].to_string())
                .collect();
            let (moves, cost) = place(&target, &syllables);
            let (best_cost, best_moves) = brute_force(&target, &syllables);
            assert_eq!(cost, best_cost, "{src}: {syllables:?}");
            assert_eq!(moves, best_moves, "{src}: {syllables:?}");
        }
    }

    /// Every alignment, with its cost and the ranks of its moves in the order of preference;
    /// the least cost, then the least ranks.
    fn brute_force(target: &Target, syllables: &[String]) -> (Cost, Vec<Move>) {
        let len = target.plain.len();
        let mut best: Option<(Cost, Vec<usize>, Vec<Move>)> = None;
        // Each syllable: a position, or `len` for unknown.
        let mut choice = vec![0; syllables.len()];
        loop {
            let mut cost = Cost::default();
            let mut ranks = Vec::new();
            let mut moves = Vec::new();
            let mut previous: Option<usize> = None;
            for (syllable, &c) in syllables.iter().zip(&choice) {
                let restarts: &[usize] = previous.map_or(&[], |p| &target.restarts[p]);
                let next = previous.map_or(0, |p| p + 1);
                if c == len {
                    cost = cost + MISMATCH;
                    ranks.push(1 + restarts.len());
                    moves.push(Move::Unknown);
                    continue;
                }
                let (step, rank) = if c == next {
                    (FOUND, 0)
                } else if let Some(k) = restarts.iter().position(|&r| r == c) {
                    (FOUND, 1 + k)
                } else {
                    (JUMP, 2 + restarts.len() + c)
                };
                let here = if target.plain[c] == *syllable {
                    FOUND
                } else {
                    MISMATCH
                };
                cost = cost + step + here;
                ranks.push(rank);
                moves.push(Move::To {
                    position: c,
                    free: step == FOUND,
                });
                previous = Some(c);
            }
            if best
                .as_ref()
                .is_none_or(|(c, r, _)| (cost, &ranks) < (*c, r))
            {
                best = Some((cost, ranks, moves));
            }
            // The next choice, counting in base len + 1.
            let Some(k) = choice.iter().position(|&c| c < len) else {
                break;
            };
            choice[k] += 1;
            choice[..k].fill(0);
        }
        let (cost, _, moves) = best.unwrap();
        (cost, moves)
    }
}
