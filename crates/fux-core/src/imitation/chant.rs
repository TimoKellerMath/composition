//! Subjects from chant: the notes of a chant phrase, found by the words, as the pitches of a
//! head, with a rhythm from the words.
//!
//! The chants of the Kyriale that Palestrina paraphrased are part of the library ([`Kyriale`]),
//! so that they serve without files, in WebAssembly too: Mass IX *Cum jubilo* (his Missa de
//! beata Virgine), Credo I and Kyrie IV *Cunctipotens genitor Deus* (Missa in minoribus
//! duplicibus). `kyriale/README.md` names their sources.
//!
//! **Which notes.** A point's notes begin where its words begin in the chant: at the first
//! syllables after the point before's that spell its head words (or its first word, or its first
//! syllable), compared as plain letters without accents and h, j and y as i, oe as ae. So Kyrie I
//! takes the first invocation, the Christe the first Christe and its tail the eleison of that
//! invocation; Kyrie II takes the last invocation (Wagner 1913, p. 446). An own text, or words the
//! chant does not sing, take the phrases in order; the request may name them. The notes run to
//! the end of their phrase and on into the next while they have fewer than six distinct pitches
//! (a fifth of Palestrina's heads cross the bar), at most to the tenth.
//!
//! **Mode.** The notes count as degrees above the chant's final, the final of its numbered mode
//! ([`Chant::numbered_mode`]: Kyrie IV ends on A but is in the first mode), and the setting sings
//! them in its own mode, with its accidentals: Kyrie IX's D F G A G F E D becomes G B♭ C D C B♭ A
//! G in the transposed Dorian of the Missa de beata Virgine.
//!
//! **Parts of a mass** (phase 14). A part that begins within a long section takes the chant from
//! where it sings the part's first words ([`section_note`], [`from_note`]), so that the chant runs
//! on through the parts of a Gloria or a Credo. The head motif of a mass takes the path of the
//! chant too: its tones are the notes of a source of their own ([`Motif`]).

use std::fmt;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use super::points::Part;
use super::{Motif, PointKind};
use crate::gabc::{Chant, ChantNote, Phrase, parse_gabc};
use crate::generate::GenerateError;
use crate::lang::Lang;
use crate::pitch::Step;
use crate::text::{Text, latin};

/// A source runs on into the next phrase while it has fewer distinct notes.
const DISTINCT: usize = 6;

/// The most distinct notes a source keeps: a head paraphrases ten at most.
const MOST_DISTINCT: usize = 10;

/// The notes of a chant that a point paraphrases: from the syllable where its words begin
/// in the chant to the end of that phrase, and on into the next ones while it has fewer than six
/// distinct pitches, ten at most. Or the tones of the head motif of a mass (phase 14), which the
/// first point of a later movement takes up ([`motif_source`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Source {
    /// The phrase it begins in, counted from 0.
    pub phrase: usize,
    /// The index of its first note in [`Chant::notes`].
    pub start: usize,
    /// Its notes as diatonic steps above the chant's final, the first within the octave above it.
    pub degrees: Vec<i8>,
    /// The alteration of each note against the scale of the setting's mode: none for a chant,
    /// whose notes the setting sings on its own scale with its flattened mi; a head motif's as
    /// Kyrie I sang them.
    pub alters: Vec<i8>,
    /// The notes are a head motif (MASS-MOTIF): a head sings each of them in order, its pitch
    /// kept, none passing between them, and strikes at most
    /// [`MOTIF_REPEATS`](super::MOTIF_REPEATS) of them again for syllables.
    pub motif: bool,
}

/// The source of a head on the head motif of a mass: its tones with their alterations.
pub(crate) fn motif_source(motif: &Motif) -> Source {
    Source {
        phrase: 0,
        start: 0,
        degrees: motif.tones.iter().map(|&(degree, _)| degree).collect(),
        alters: motif.tones.iter().map(|&(_, alter)| alter).collect(),
        motif: true,
    }
}

/// A point whose chant phrase gave no usable head: it takes a subject from its words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChantFallback {
    /// The point, counted from 1.
    pub point: usize,
}

impl ChantFallback {
    /// Message for users in their language.
    pub fn message(&self, lang: Lang) -> String {
        let point = self.point;
        match lang {
            Lang::De => {
                format!(
                    "Punkt {point}: kein brauchbarer Kopf aus dem Choral; Soggetto aus dem Text"
                )
            }
            Lang::En => {
                format!("Point {point}: no usable head from the chant; subject from the text")
            }
        }
    }
}

impl fmt::Display for ChantFallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message(Lang::En))
    }
}

/// The plain form in which the syllables of a chant and of a text are compared: lowercase letters
/// only, without accents, punctuation or hyphens, j and y as i, oe as ae, without h.
fn plain(syllable: &str) -> String {
    let mut out = String::with_capacity(syllable.len());
    for c in latin::normalize(syllable).chars() {
        let letter = match c {
            'a'..='z' => c,
            'à' | 'â' | 'ä' => 'a',
            'è' | 'ê' | 'ë' => 'e',
            'ì' | 'î' | 'ï' => 'i',
            'ò' | 'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ÿ' => 'y',
            _ => continue,
        };
        match letter {
            'h' => {}
            'y' => out.push('i'),
            letter => out.push(letter),
        }
    }
    out.replace("oe", "ae")
}

/// The final of a chant as its subjects take it: that of its numbered mode, else its last note's.
pub(crate) fn final_step(chant: &Chant) -> Option<Step> {
    match chant.numbered_mode() {
        Ok(mode) => Some(mode.final_step()),
        Err(_) => chant.notes.last().map(|n| n.pitch.step),
    }
}

/// The notes of `chant` that each of the points `parts` of `text` paraphrases; `None` for a
/// restatement, which sings the subject of the point it restates, and for a point the chant has
/// no more phrases for. With `phrases` (counted from 0) the points that are not restatements take
/// these phrases in order, and the points after them the phrases that follow while the chant has
/// any; a named phrase the chant does not have fails with [`GenerateError::NoChantPhrase`].
pub(crate) fn sources(
    chant: &Chant,
    text: &Text,
    parts: &[Part],
    phrases: Option<&[usize]>,
) -> Result<Vec<Option<Source>>, GenerateError> {
    let all = chant.phrases();
    let count = all.len();
    let Some(final_step) = final_step(chant) else {
        return Ok(vec![None; parts.len()]);
    };
    let phrase_of = |note: usize| all.iter().position(|p| p.range.contains(&note));
    // The syllables the chant begins, with their notes, in the plain form.
    let sung: Vec<(usize, String)> = chant
        .notes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| Some((i, plain(n.syllable.as_deref()?))))
        .filter(|(_, s)| !s.is_empty())
        .collect();
    let mut out = Vec::with_capacity(parts.len());
    let mut before: Option<(usize, usize)> = None;
    let mut ordinal = 0;
    for (p, part) in parts.iter().enumerate() {
        if let PointKind::Restatement { .. } = part.kind {
            out.push(None);
            continue;
        }
        // The phrase the request names for the point, or the one after the last it names.
        let named = phrases.map(|named| match named.get(ordinal) {
            Some(&phrase) => (phrase, true),
            None => {
                let after = named
                    .last()
                    .map_or(ordinal, |&last| last + 1 + ordinal - named.len());
                (after, false)
            }
        });
        ordinal += 1;
        let start = match named {
            Some((phrase, true)) if phrase >= count => {
                return Err(GenerateError::NoChantPhrase {
                    phrase: phrase + 1,
                    phrases: count,
                });
            }
            Some((phrase, _)) if phrase >= count => None,
            Some((phrase, _)) => Some(all[phrase].range.start),
            None => {
                let after = before.map(|(note, _)| note);
                let last = text.id == "kyrie2" && p == 0 && chant.is_kyrie();
                let by_words = (text.id != crate::text::OWN_ID)
                    .then(|| found(text, part, &sung, after, last))
                    .flatten();
                by_words.or_else(|| {
                    let next = before.map_or(0, |(_, phrase)| phrase + 1);
                    (next < count).then(|| all[next].range.start)
                })
            }
        };
        let Some(start) = start else {
            out.push(None);
            continue;
        };
        let phrase = phrase_of(start).unwrap_or(0);
        before = Some((start, phrase));
        out.push(Some(source(chant, &all, phrase, start, final_step)));
    }
    Ok(out)
}

/// Where the words of a point begin in the chant: the note of the first syllable after `after`
/// (or of the last one with `last`) from which the chant spells the point's head words, else its
/// first word, else its first syllable.
fn found(
    text: &Text,
    part: &Part,
    sung: &[(usize, String)],
    after: Option<usize>,
    last: bool,
) -> Option<usize> {
    let start = usize::from(part.words.start);
    let word_end = text.word_of(start).range().end;
    let head_end = usize::from(part.head.end).max(start + 1);
    let candidates = sung
        .iter()
        .enumerate()
        .filter(|(_, (note, _))| after.is_none_or(|after| *note > after));
    let positions: Vec<usize> = if last {
        candidates.map(|(k, _)| k).rev().collect()
    } else {
        candidates.map(|(k, _)| k).collect()
    };
    for end in [head_end, word_end, start + 1] {
        let wanted: Vec<String> = (start..end)
            .map(|s| plain(&text.syllables[s].text))
            .collect();
        let spells = |k: usize| {
            sung.get(k..k + wanted.len())
                .is_some_and(|run| run.iter().map(|(_, s)| s).eq(wanted.iter()))
        };
        if let Some(&k) = positions.iter().find(|&&k| spells(k)) {
            return Some(sung[k].0);
        }
    }
    None
}

/// The source that begins at note `start` of phrase `phrase`.
fn source(
    chant: &Chant,
    phrases: &[Phrase],
    phrase: usize,
    start: usize,
    final_step: Step,
) -> Source {
    let mut end = phrases[phrase].range.end;
    let distinct = |notes: &[ChantNote]| {
        1 + notes
            .windows(2)
            .filter(|w| w[0].pitch.diatonic() != w[1].pitch.diatonic())
            .count()
    };
    let mut next = phrase + 1;
    while distinct(&chant.notes[start..end]) < DISTINCT && next < phrases.len() {
        end = phrases[next].range.end;
        next += 1;
    }
    // At most ten distinct notes, the repetitions of the tenth kept.
    let mut kept = start + 1;
    let mut seen = 1;
    while kept < end {
        let changes = chant.notes[kept].pitch.diatonic() != chant.notes[kept - 1].pitch.diatonic();
        if changes && seen == MOST_DISTINCT {
            break;
        }
        seen += usize::from(changes);
        kept += 1;
    }
    let notes = &chant.notes[start..kept];
    let base = notes[0].pitch.diatonic() - final_step.index();
    let octave = base.div_euclid(7) * 7 + final_step.index();
    let degrees: Vec<i8> = notes
        .iter()
        .map(|n| (n.pitch.diatonic() - octave) as i8)
        .collect();
    Source {
        phrase,
        start,
        alters: vec![0; degrees.len()],
        degrees,
        motif: false,
    }
}

/// The syllables a chant begins, each with its note, in the plain form in which they are
/// compared with a text's.
fn sung_syllables(chant: &Chant) -> Vec<(usize, String)> {
    chant
        .notes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| Some((i, plain(n.syllable.as_deref()?))))
        .filter(|(_, s)| !s.is_empty())
        .collect()
}

/// Where `sung` (the syllables of a chant, [`sung_syllables`]) spells the syllables `range` of
/// `text`, from the sung syllable `from` on: the first place (with `last` the last) where it
/// spells all of them, else their first word, else their first syllable. The index of the sung
/// syllable it begins at and of the one after those it spelled.
fn spelled(
    sung: &[(usize, String)],
    text: &Text,
    range: Range<usize>,
    from: usize,
    last: bool,
) -> Option<(usize, usize)> {
    let word_end = text.word_of(range.start).range().end.min(range.end);
    for end in [range.end, word_end, range.start + 1] {
        let wanted: Vec<String> = (range.start..end)
            .map(|s| plain(&text.syllables[s].text))
            .collect();
        let spells = |k: &usize| {
            sung.get(*k..*k + wanted.len())
                .is_some_and(|run| run.iter().map(|(_, s)| s).eq(wanted.iter()))
        };
        let mut places = from..sung.len();
        let found = if last {
            places.rev().find(spells)
        } else {
            places.find(spells)
        };
        if let Some(k) = found {
            return Some((k, k + wanted.len()));
        }
    }
    None
}

/// The note of `chant` at which it begins to sing unit `unit` (from 0) of the section `id` of the
/// Ordinary: the section's units found by their words one after the other, its first where the
/// chant first sings it (with `last` where it last sings it), each later one after the one
/// before. So a part of a mass that begins within a long section (phase 14, MASS-CHANT) takes up
/// the chant where its words stand, not at an earlier place that sings the same words (the
/// Gloria's second Dó-mi-ne De-us, Gé-ni-tum after u-ni-gé-ni-tum, the second Qui tollis); with
/// `last` a section sung more than once in the chant takes its last place, as Kyrie II takes the
/// last invocation (the third Agnus Dei, the Osanna after the Benedictus). `None` where the
/// section is no section of the Ordinary, has no such unit, or the chant does not sing its units
/// in their order.
pub fn section_note(chant: &Chant, id: &str, unit: usize, last: bool) -> Option<usize> {
    let text = crate::text::ordinary::section(id, &crate::text::TextOptions::default())?;
    if unit >= text.units.len() {
        return None;
    }
    let sung = sung_syllables(chant);
    let mut from = 0;
    let mut start = None;
    for (u, found) in text.units.iter().enumerate().take(unit + 1) {
        let (k, after) = spelled(&sung, text, found.range(), from, last && u == 0)?;
        start = Some(sung[k].0);
        from = after;
    }
    start
}

/// The chant from note `from` on, as a part of a mass that begins there takes it up: its points
/// find their notes by their words from there ([`section_note`]).
pub fn from_note(chant: &Chant, from: usize) -> Chant {
    Chant {
        name: chant.name.clone(),
        mode_number: chant.mode_number,
        notes: chant.notes[from.min(chant.notes.len())..].to_vec(),
    }
}

/// A chant of the Kyriale in the library, after the Graduale Romanum of 1961. It sings the texts
/// of the Ordinary ([`crate::text::ordinary`]), so the points find their notes by the words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kyriale {
    /// Kyrie IX *Cum jubilo*, mode 1.
    KyrieIX,
    /// Gloria IX, mode 7.
    GloriaIX,
    /// Credo I, mode 4.
    CredoI,
    /// Sanctus IX with the Benedictus, mode 5.
    SanctusIX,
    /// Agnus Dei IX, mode 5.
    AgnusIX,
    /// Kyrie IV *Cunctipotens genitor Deus*, mode 1.
    KyrieIV,
}

impl Kyriale {
    /// All of them: Mass IX and Credo I in the order of the Mass, then Kyrie IV.
    pub const ALL: [Kyriale; 6] = [
        Kyriale::KyrieIX,
        Kyriale::GloriaIX,
        Kyriale::CredoI,
        Kyriale::SanctusIX,
        Kyriale::AgnusIX,
        Kyriale::KyrieIV,
    ];

    /// The name of its file without `.gabc`, by which users choose it: `kyrie-ix`.
    pub fn key(self) -> &'static str {
        match self {
            Kyriale::KyrieIX => "kyrie-ix",
            Kyriale::GloriaIX => "gloria-ix",
            Kyriale::CredoI => "credo-i",
            Kyriale::SanctusIX => "sanctus-ix",
            Kyriale::AgnusIX => "agnus-ix",
            Kyriale::KyrieIV => "kyrie-iv",
        }
    }

    /// The chant with this key.
    pub fn from_key(key: &str) -> Option<Kyriale> {
        Kyriale::ALL.into_iter().find(|k| k.key() == key)
    }

    /// Its GABC, as GregoBase gives it.
    pub fn gabc(self) -> &'static str {
        match self {
            Kyriale::KyrieIX => include_str!("kyriale/kyrie-ix.gabc"),
            Kyriale::GloriaIX => include_str!("kyriale/gloria-ix.gabc"),
            Kyriale::CredoI => include_str!("kyriale/credo-i.gabc"),
            Kyriale::SanctusIX => include_str!("kyriale/sanctus-ix.gabc"),
            Kyriale::AgnusIX => include_str!("kyriale/agnus-ix.gabc"),
            Kyriale::KyrieIV => include_str!("kyriale/kyrie-iv.gabc"),
        }
    }

    /// The chant, read once.
    pub fn chant(self) -> Arc<Chant> {
        static CHANTS: OnceLock<Vec<Arc<Chant>>> = OnceLock::new();
        let chants = CHANTS.get_or_init(|| {
            Kyriale::ALL
                .iter()
                .map(|k| match parse_gabc(k.gabc()) {
                    Ok(chant) => Arc::new(chant),
                    Err(error) => panic!("kyriale/{}.gabc: {error}", k.key()),
                })
                .collect()
        });
        Arc::clone(&chants[self as usize])
    }

    /// The sections of the Ordinary that it sings, by their ids in [`crate::text::ordinary`].
    pub fn sections(self) -> &'static [&'static str] {
        match self {
            Kyriale::KyrieIX | Kyriale::KyrieIV => &["kyrie", "christe", "kyrie2"],
            Kyriale::GloriaIX => &["gloria", "qui-tollis"],
            Kyriale::CredoI => &[
                "credo",
                "incarnatus",
                "crucifixus",
                "resurrexit",
                "spiritum",
            ],
            Kyriale::SanctusIX => &["sanctus", "pleni", "osanna", "benedictus", "osanna2"],
            Kyriale::AgnusIX => &["agnus", "agnus3"],
        }
    }

    /// The chant for a section of the Ordinary, also for some of its units (`gloria:1-5`): Mass
    /// IX, for the Credo Credo I. Sections sung one after the other (`sanctus+pleni+osanna`,
    /// phase 14) take the chant of the first, which sings the others too. `None` for an own
    /// text.
    pub fn for_section(id: &str) -> Option<Kyriale> {
        let section = id.split([':', '+']).next().unwrap_or(id);
        Kyriale::ALL
            .into_iter()
            .find(|k| k.sections().contains(&section))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::{Ambitus, Mode, ModeSpec};
    use crate::text::{TextOptions, latin, ordinary};

    fn names(chant: &Chant, phrase: usize) -> String {
        chant.phrases()[phrase]
            .notes
            .iter()
            .map(|n| n.pitch.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn kyriale_chants_parse_with_their_modes() {
        let dorian = ModeSpec::new(Mode::Dorian);
        let ionian_flat = ModeSpec::new(Mode::Ionian).with_transposition(true);
        // The number in the Graduale, the mode it names, the phrases, the words of the first.
        let table = [
            (Kyriale::KyrieIX, 1, dorian, 9, "Ky-ri-e e-lé-i-son."),
            (
                Kyriale::GloriaIX,
                7,
                ModeSpec::new(Mode::Mixolydian),
                30,
                "Gló-ri-a in ex-cél-sis De-o.",
            ),
            (
                Kyriale::CredoI,
                4,
                ModeSpec::new(Mode::Phrygian).with_ambitus(Ambitus::Plagal),
                46,
                "Cre-do in u-num De-um,",
            ),
            (Kyriale::SanctusIX, 5, ionian_flat, 11, "San-ctus,"),
            (Kyriale::AgnusIX, 5, ionian_flat, 11, "A-gnus De-i,"),
            (Kyriale::KyrieIV, 1, dorian, 4, "Ky-ri-e e-lé-i-son."),
        ];
        for (kyriale, number, mode, phrases, first) in table {
            let chant = kyriale.chant();
            let key = kyriale.key();
            assert_eq!(chant.mode_number, Some(number), "{key}");
            assert_eq!(chant.numbered_mode(), Ok(mode), "{key}");
            assert_eq!(chant.phrases().len(), phrases, "{key}");
            assert_eq!(chant.phrases()[0].words(), first, "{key}");
            assert_eq!(chant.is_kyrie(), key.starts_with("kyrie"), "{key}");
            assert_eq!(Kyriale::from_key(key), Some(kyriale));
            assert!(Arc::ptr_eq(&chant, &kyriale.chant()), "read once");
            // As a cantus firmus each chant ends on the final of its mode, except Kyrie IV, whose
            // last invocations end on A, the fifth: its last note makes it Aeolian.
            if kyriale == Kyriale::KyrieIV {
                assert_eq!(chant.mode(), Ok(ModeSpec::new(Mode::Aeolian)));
                assert_eq!(names(&chant, 3).rsplit(' ').next(), Some("A3"));
            } else {
                assert_eq!(chant.mode(), Ok(mode), "{key}");
            }
        }
        // Kyrie IX's first invocation and the beginning of Gloria IX, as Palestrina took them up
        // in his Missa de beata Virgine.
        let kyrie = Kyriale::KyrieIX.chant();
        assert_eq!(
            names(&kyrie, 0),
            "D3 F3 G3 A3 G3 F3 E3 D3 F3 D3 C3 D3 C3 D3 F3 G3 F3 E3 D3"
        );
        assert_eq!(kyrie.phrases()[3].words(), "Chri-ste e-lé-i-son.");
        let gloria = Kyriale::GloriaIX.chant();
        assert_eq!(gloria.phrases()[1].words(), "Et in ter-ra pax ho-mí-ni-bus");
        assert_eq!(
            names(&gloria, 1),
            "D4 E4 B3 C4 D4 D4 G3 A3 B3 C4 B3 A3 G3 G3"
        );
        assert_eq!(Kyriale::from_key("kyrie-x"), None);
    }

    #[test]
    fn kyriale_chants_sing_the_ordinary() {
        let letters = |s: &str| -> String {
            latin::normalize(s)
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect()
        };
        for kyriale in Kyriale::ALL {
            let sung: String = kyriale
                .chant()
                .notes
                .iter()
                .filter_map(|n| n.syllable.as_deref())
                .map(letters)
                .collect();
            for &id in kyriale.sections() {
                let text = ordinary::section(id, &TextOptions::default()).unwrap();
                let words: String = text.syllables.iter().map(|s| letters(&s.text)).collect();
                assert!(sung.contains(&words), "{} sings no {id}", kyriale.key());
            }
        }
        assert_eq!(Kyriale::for_section("christe"), Some(Kyriale::KyrieIX));
        assert_eq!(Kyriale::for_section("gloria:1-5"), Some(Kyriale::GloriaIX));
        assert_eq!(Kyriale::for_section("resurrexit"), Some(Kyriale::CredoI));
        assert_eq!(Kyriale::for_section("osanna2"), Some(Kyriale::SanctusIX));
        assert_eq!(Kyriale::for_section("agnus3"), Some(Kyriale::AgnusIX));
        assert_eq!(Kyriale::for_section("eigen"), None);
    }

    /// Sections sung one after the other take the chant of the first, which sings them all: the
    /// Sanctus of a Missa brevis, a part of the Credo across a section of the Ordinary.
    #[test]
    fn for_section_of_a_concatenation() {
        let options = TextOptions::default();
        for (spec, kyriale) in [
            ("sanctus+pleni+osanna", Kyriale::SanctusIX),
            ("crucifixus+resurrexit:1-2", Kyriale::CredoI),
            ("gloria:10-13+qui-tollis:1-4", Kyriale::GloriaIX),
            ("benedictus+osanna2", Kyriale::SanctusIX),
            ("agnus:3+agnus3", Kyriale::AgnusIX),
        ] {
            let (text, _) = crate::text::resolve(spec, &options).unwrap();
            assert_eq!(Kyriale::for_section(&text.id), Some(kyriale), "{spec}");
            for piece in spec.split('+') {
                assert_eq!(Kyriale::for_section(piece), Some(kyriale), "{piece}");
            }
        }
        assert_eq!(Kyriale::for_section("eigen+kyrie"), None);
    }

    /// The intonations of the Gloria and the Credo are the words of the first phrase of their
    /// chants in the Graduale of 1961, which the choir does not sing.
    #[test]
    fn intonations_open_the_chants() {
        for (kyriale, id) in [(Kyriale::GloriaIX, "gloria"), (Kyriale::CredoI, "credo")] {
            let words = kyriale.chant().phrases()[0].words();
            assert_eq!(
                Some(words.replace('-', "").as_str()),
                ordinary::intonation(id)
            );
        }
    }

    /// The points of a text, a short text's second statement as its tail or restated.
    fn parts_of(spec: &str, restate: super::super::Toggle) -> (Text, Vec<Part>) {
        use crate::generate::Rng;
        use crate::imitation::Imitation;
        use crate::imitation::points::points_of;
        let text = crate::text::resolve(spec, &TextOptions::default())
            .unwrap()
            .0;
        let dorian = ModeSpec::new(Mode::Dorian);
        let plan = crate::text::plan::plan(&text, dorian, 4, None, None, true).unwrap();
        let imitation = Imitation {
            restate,
            ..Imitation::default()
        };
        let parts = points_of(&text, plan.statements, true, &imitation, &mut Rng::new(1)).unwrap();
        (text, parts)
    }

    /// Where each point's source begins: its phrase and its first note.
    fn starts(chant: &Chant, spec: &str, phrases: Option<&[usize]>) -> Vec<Option<(usize, usize)>> {
        let (text, parts) = parts_of(spec, super::super::Toggle::Off);
        sources(chant, &text, &parts, phrases)
            .unwrap()
            .iter()
            .map(|s| s.as_ref().map(|s| (s.phrase, s.start)))
            .collect()
    }

    /// The points find their notes by the words: Kyrie I the first invocation and its
    /// tail the eleison of it, the Christe the first Christe, Kyrie II the last invocation, the
    /// Gloria's first point the phrase after the intonation; an own text takes the phrases in
    /// order, and named phrases come first.
    #[test]
    fn points_find_their_chant_by_the_words() {
        let kyrie = Kyriale::KyrieIX.chant();
        let invocation = |i: usize| kyrie.phrases()[i].range.start;
        assert_eq!(starts(&kyrie, "kyrie", None), [Some((0, 0)), Some((0, 12))]);
        // Chri- has one note, -ste six (A G B♭ A G G), then the eleison.
        assert_eq!(
            starts(&kyrie, "christe", None),
            [Some((3, invocation(3))), Some((3, invocation(3) + 7))]
        );
        assert_eq!(starts(&kyrie, "kyrie2", None)[0], Some((8, invocation(8))));
        let (text, parts) = parts_of("kyrie", super::super::Toggle::Off);
        let found = sources(&kyrie, &text, &parts, None).unwrap();
        // Ten distinct notes at most, as degrees above the final D.
        assert_eq!(
            found[0].as_ref().unwrap().degrees,
            [0, 2, 3, 4, 3, 2, 1, 0, 2, 0]
        );
        // The eleison: C D F G F E D, its phrase long enough.
        assert_eq!(found[1].as_ref().unwrap().degrees, [6, 7, 9, 10, 9, 8, 7]);
        // A restatement sings the subject of its point.
        let (text, parts) = parts_of("kyrie", super::super::Toggle::On);
        assert_eq!(sources(&kyrie, &text, &parts, None).unwrap()[1], None);
        // Gloria IX begins with the intonation; Et in terra is its second phrase.
        let gloria = Kyriale::GloriaIX.chant();
        assert_eq!(starts(&gloria, "gloria:1-5", None)[0].map(|s| s.0), Some(1));
        // Sanctus IX: San-ctus at its beginning, Dó-mi-nus after it.
        let sanctus = Kyriale::SanctusIX.chant();
        let found = starts(&sanctus, "sanctus", None);
        assert_eq!(found[0], Some((0, 0)));
        let dominus = sanctus
            .notes
            .iter()
            .position(|n| n.syllable.as_deref() == Some("Dó-"))
            .unwrap();
        assert_eq!(found[1].map(|s| s.1), Some(dominus));
        // An own text over a hymn takes its phrases in order.
        let iste = parse_gabc(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/chant/iste-confessor.gabc"
        )))
        .unwrap();
        let own = "Lau-dá-te Dó-mi-num, / om-nes gen-tes.";
        assert_eq!(
            starts(&iste, own, None),
            [Some((0, 0)), Some((1, iste.phrases()[1].range.start))]
        );
        // Named phrases, and those after them.
        assert_eq!(
            starts(&kyrie, "kyrie", Some(&[2])),
            [Some((2, invocation(2))), Some((3, invocation(3)))]
        );
        let (text, parts) = parts_of("kyrie", super::super::Toggle::Off);
        assert_eq!(
            sources(&kyrie, &text, &parts, Some(&[9])),
            Err(GenerateError::NoChantPhrase {
                phrase: 10,
                phrases: 9
            })
        );
        // The last phrase named, the tail after it has none left and takes its words.
        assert_eq!(
            starts(&kyrie, "kyrie", Some(&[8])),
            [Some((8, invocation(8))), None]
        );
    }

    #[test]
    fn fallbacks_speak_both_languages() {
        let fallback = ChantFallback { point: 2 };
        assert_eq!(
            fallback.message(Lang::De),
            "Punkt 2: kein brauchbarer Kopf aus dem Choral; Soggetto aus dem Text"
        );
        assert_eq!(
            fallback.to_string(),
            "Point 2: no usable head from the chant; subject from the text"
        );
    }

    /// The words of a note and the ones after it, as plain syllables.
    fn words_at(chant: &Chant, note: usize, syllables: usize) -> Vec<String> {
        sung_syllables(chant)
            .into_iter()
            .filter(|&(i, _)| i >= note)
            .take(syllables)
            .map(|(_, s)| s)
            .collect()
    }

    /// A unit of a section is found where the chant sings it after the units before it, not at
    /// an earlier place with the same words (phase 14, MASS-CHANT); a section that the chant
    /// sings more than once takes its last place with `last`. The chant from a note on keeps the
    /// notes after it, its name and its mode.
    #[test]
    fn units_are_found_in_their_order() {
        let credo = Kyriale::CredoI.chant();
        // Gé-ni-tum, unit 11 of the Credo, after u-ni-gé-ni-tum of unit 6.
        let genitum = section_note(&credo, "credo", 10, false).unwrap();
        assert_eq!(words_at(&credo, genitum, 3), ["ge", "ni", "tum"]);
        let unigenitum = credo
            .notes
            .iter()
            .position(|n| n.syllable.as_deref() == Some("gé-"))
            .unwrap();
        assert!(genitum > unigenitum + 20, "{genitum} {unigenitum}");
        assert_eq!(section_note(&credo, "credo", 0, false), Some(7));
        let gloria = Kyriale::GloriaIX.chant();
        // The second Dó-mi-ne De-us of the Gloria, its unit 11.
        let first = section_note(&gloria, "gloria", 6, false).unwrap();
        let second = section_note(&gloria, "gloria", 10, false).unwrap();
        assert!(second > first);
        assert_eq!(words_at(&gloria, second, 3), ["do", "mi", "ne"]);
        // The second Qui tollis of the Gloria, its unit 3.
        let again = section_note(&gloria, "qui-tollis", 2, false).unwrap();
        assert!(again > section_note(&gloria, "qui-tollis", 0, false).unwrap());
        // Agnus Dei III at the third invocation, the Osanna after the Benedictus.
        let agnus = Kyriale::AgnusIX.chant();
        let third = section_note(&agnus, "agnus3", 0, true).unwrap();
        assert_eq!(agnus.phrases().len(), 11);
        assert!(third > section_note(&agnus, "agnus", 0, false).unwrap());
        assert_eq!(
            words_at(&agnus, third, 20).last().map(String::as_str),
            Some("cem")
        );
        let sanctus = Kyriale::SanctusIX.chant();
        let osanna = section_note(&sanctus, "osanna2", 0, true).unwrap();
        assert!(osanna > section_note(&sanctus, "benedictus", 0, false).unwrap());
        assert_eq!(section_note(&sanctus, "eigen", 0, false), None);
        assert_eq!(section_note(&sanctus, "osanna", 3, false), None);
        // Every unit of the long sections, one after the other through their chants.
        for kyriale in [Kyriale::GloriaIX, Kyriale::CredoI] {
            let chant = kyriale.chant();
            let mut last = 0;
            for &id in kyriale.sections() {
                let text = ordinary::section(id, &TextOptions::default()).unwrap();
                for unit in 0..text.units.len() {
                    let note = section_note(&chant, id, unit, false).unwrap();
                    assert!(note > last, "{id}:{}: {note} after {last}", unit + 1);
                    last = note;
                }
            }
        }
        let from = from_note(&credo, genitum);
        assert_eq!(from.notes[..], credo.notes[genitum..]);
        assert_eq!(
            (from.name.as_deref(), from.numbered_mode()),
            (Some("Credo I"), credo.numbered_mode())
        );
    }
}
