//! Latin text for singing: the Ordinary of the Mass and own texts, divided into syllables,
//! words and sense units, with the word accents.
//!
//! A sense unit runs up to a punctuation mark: `,` `;` `:` end it at an intermediate stop,
//! `.` `?` `!` and the end of the text at a full stop, which also ends a sentence. Two markers
//! add what the books do not print. `/` before a word starts a tail, an end of a unit that can
//! be restated on its own (Ký-ri-e / e-lé-i-son), and marks a break before it; `|` between two
//! words marks a break, where a voice may rest.

pub mod align;
pub mod latin;
pub mod ordinary;
pub mod plan;
#[cfg(test)]
pub(crate) mod test_support;
pub mod underlay;

use std::fmt;
use std::ops::{BitOr, BitOrAssign, Range, RangeInclusive};

use thiserror::Error;

use crate::lang::Lang;

/// The id of an own text.
pub const OWN_ID: &str = "eigen";

/// Own texts count as short up to this many syllables, as long beyond.
pub const SHORT_MAX: usize = 20;

/// The most syllables a text may have: a position plus one fits into 16 bits, with one value to
/// spare.
pub const MAX_SYLLABLES: usize = u16::MAX as usize - 1;

/// Monosyllables that count as unstressed.
const FUNCTION_WORDS: [&str; 19] = [
    "et", "in", "de", "ex", "e", "a", "ab", "ad", "cum", "pro", "per", "sub", "qui", "quem",
    "quod", "sed", "ac", "nec", "ut",
];

/// The punctuation that ends a unit.
const PUNCTUATION: [char; 6] = [',', ';', ':', '.', '?', '!'];

/// Characters that texts copied from web pages and PDFs contain but that mean nothing in Latin:
/// the byte order mark, the soft hyphen, the zero-width space and joiners, and the word joiner.
/// They are dropped.
const IGNORED: [char; 6] = [
    '\u{feff}', '\u{ad}', '\u{200b}', '\u{200c}', '\u{200d}', '\u{2060}',
];

/// A text to be sung.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    /// `kyrie` for a section of the Ordinary, `gloria:1-5` for some of its units,
    /// `sanctus+pleni+osanna` for sections sung one after the other ([`Text::concat`]), `eigen`
    /// for an own text.
    pub id: String,
    pub name_de: String,
    pub name_en: String,
    pub kind: TextKind,
    pub syllables: Vec<Syllable>,
    pub words: Vec<Word>,
    pub units: Vec<Unit>,
    /// The words go on after this text, in the next part of a longer setting (phase 14): in a
    /// mass, every part of a section but its last. Its last unit is then no Amen that a long
    /// text may restate freely; a restatement of it costs as one of any other unit (TXT-REPEAT
    /// `long`). Every text that [`resolve`] and [`Text::parse`] give ends where it stands.
    pub continued: bool,
}

/// Short texts (Kyrie, Sanctus, Agnus) are usually restated, long ones (Gloria, Credo) sung
/// once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextKind {
    Short,
    Long,
}

impl TextKind {
    /// The kind of an own text: short up to [`SHORT_MAX`] syllables, long beyond.
    pub fn of_length(syllables: usize) -> TextKind {
        if syllables <= SHORT_MAX {
            TextKind::Short
        } else {
            TextKind::Long
        }
    }

    /// The name for users: „kurz“ or „lang“.
    pub fn name(self, lang: Lang) -> &'static str {
        match self {
            TextKind::Short => lang.pick("kurz", "short"),
            TextKind::Long => lang.pick("lang", "long"),
        }
    }
}

/// One syllable of a text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Syllable {
    /// The book form with its acute, without hyphen or punctuation: `lé`.
    pub text: String,
    /// Index of its word.
    pub word: u16,
    /// Index of its unit.
    pub unit: u16,
    pub flags: SylFlags,
}

/// The properties of a syllable, as a set of flags.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SylFlags(u16);

impl SylFlags {
    /// The first syllable of a word.
    pub const WORD_START: SylFlags = SylFlags(1);
    /// The last syllable of a word.
    pub const WORD_END: SylFlags = SylFlags(1 << 1);
    /// The first syllable of a unit.
    pub const UNIT_START: SylFlags = SylFlags(1 << 2);
    /// The last syllable of a unit.
    pub const UNIT_END: SylFlags = SylFlags(1 << 3);
    /// The first syllable of a tail: its word follows a `/`.
    pub const TAIL_START: SylFlags = SylFlags(1 << 4);
    /// A marked break follows: a `|`, or a `/` before the next word.
    pub const BREAK_AFTER: SylFlags = SylFlags(1 << 5);
    /// The last syllable of a sentence.
    pub const FULL_STOP: SylFlags = SylFlags(1 << 6);
    /// The stressed syllable of its word. A monosyllable has it unless it is a function word.
    pub const STRESS: SylFlags = SylFlags(1 << 7);
    /// Two or four syllables before the stress, in words of four or more syllables:
    /// VÒ-lun-tá-tis, o-MNÌ-pot-én-tem.
    pub const SECONDARY: SylFlags = SylFlags(1 << 8);
    /// The unstressed penult of a word stressed on the antepenult: Dó-MI-nus.
    pub const SHORT_PENULT: SylFlags = SylFlags(1 << 9);
    /// A monosyllabic function word such as et, in or qui.
    pub const FUNCTION_WORD: SylFlags = SylFlags(1 << 10);
    /// The stress of its word is a guess.
    pub const UNCERTAIN: SylFlags = SylFlags(1 << 11);

    const NAMES: [&'static str; 12] = [
        "WORD_START",
        "WORD_END",
        "UNIT_START",
        "UNIT_END",
        "TAIL_START",
        "BREAK_AFTER",
        "FULL_STOP",
        "STRESS",
        "SECONDARY",
        "SHORT_PENULT",
        "FUNCTION_WORD",
        "UNCERTAIN",
    ];

    /// No flag.
    pub const fn empty() -> SylFlags {
        SylFlags(0)
    }

    /// The flags as bits, the first constant lowest.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Whether every flag of `other` is set.
    pub const fn contains(self, other: SylFlags) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether any flag of `other` is set.
    pub const fn intersects(self, other: SylFlags) -> bool {
        self.0 & other.0 != 0
    }

    /// Accented: STRESS or SECONDARY.
    pub const fn accented(self) -> bool {
        self.intersects(SylFlags(SylFlags::STRESS.0 | SylFlags::SECONDARY.0))
    }

    /// Sets the flags of `other`.
    pub fn insert(&mut self, other: SylFlags) {
        self.0 |= other.0;
    }

    /// Clears the flags of `other`.
    pub fn remove(&mut self, other: SylFlags) {
        self.0 &= !other.0;
    }
}

impl BitOr for SylFlags {
    type Output = SylFlags;

    fn bitor(self, other: SylFlags) -> SylFlags {
        SylFlags(self.0 | other.0)
    }
}

impl BitOrAssign for SylFlags {
    fn bitor_assign(&mut self, other: SylFlags) {
        self.0 |= other.0;
    }
}

impl fmt::Debug for SylFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = SylFlags::NAMES
            .iter()
            .enumerate()
            .filter(|&(bit, _)| self.0 & (1 << bit) != 0)
            .map(|(_, &name)| name)
            .collect();
        write!(f, "SylFlags({})", names.join(" | "))
    }
}

/// A word of a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Word {
    /// Index of its first syllable.
    pub first: u16,
    /// Number of syllables.
    pub len: u8,
    /// The syllable with STRESS, counted within the word; `None` for a function word.
    pub stress: Option<u8>,
    /// The punctuation after it.
    pub punct: Option<char>,
}

impl Word {
    /// The indices of its syllables.
    pub fn range(&self) -> Range<usize> {
        let first = usize::from(self.first);
        first..first + usize::from(self.len)
    }
}

/// How a unit ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Stop {
    /// `,` `;` `:`.
    Intermediate,
    /// `.` `?` `!` or the end of the text; it ends a sentence.
    Full,
}

/// A sense unit: the syllables up to a punctuation mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Unit {
    /// Index of its first syllable.
    pub first: u16,
    /// Index of its last syllable.
    pub last: u16,
    pub stop: Stop,
    /// Index of the first unit of its sentence.
    pub sentence: u16,
}

impl Unit {
    /// The indices of its syllables.
    pub fn range(&self) -> Range<usize> {
        usize::from(self.first)..usize::from(self.last) + 1
    }
}

/// How eleison is divided.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Eleison {
    /// e-lé-i-son, as in the liturgical books.
    #[default]
    Four,
    /// e-léi-son, as Jeppesen underlays Palestrina.
    Three,
}

/// Choices that change how a text is divided.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextOptions {
    pub eleison: Eleison,
}

/// Something in a text that may not be what the user meant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextWarning {
    /// The stress of a word without an acute is a guess.
    UncertainAccent {
        /// The word as written, without hyphens.
        word: String,
        /// Its division with the acute the parser added.
        result: String,
    },
}

impl TextWarning {
    /// Message for users in their language.
    pub fn message(&self, lang: Lang) -> String {
        match self {
            TextWarning::UncertainAccent { word, result } => match lang {
                Lang::De => format!("Akzent unsicher: {word} → {result}; mit Akut angeben"),
                Lang::En => format!("Uncertain accent: {word} → {result}; mark it with an acute"),
            },
        }
    }
}

impl fmt::Display for TextWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message(Lang::En))
    }
}

/// Why a text cannot be used.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum TextError {
    #[error("the text is empty")]
    Empty,
    #[error("character {} not allowed in \"{word}\"", quoted(*ch, ("\"", "\"")))]
    Character { word: String, ch: char },
    #[error("empty syllable in \"{word}\"")]
    EmptySyllable { word: String },
    #[error("the syllable \"{syllable}\" of \"{word}\" has no vowel")]
    NoVowel { word: String, syllable: String },
    #[error("\"{word}\" has more than one acute")]
    SeveralAcutes { word: String },
    #[error("the marker {marker} must stand between two words")]
    Marker { marker: char },
    #[error(
        "the text is too long: at most {} syllables and {} in a word",
        MAX_SYLLABLES,
        u8::MAX
    )]
    TooLong,
    #[error(
        "unknown section \"{id}\"; the sections are {}; an own Latin text needs a space, a capital letter or a punctuation mark",
        valid.join(", ")
    )]
    UnknownSection {
        id: String,
        /// The ids of all sections.
        valid: Vec<String>,
    },
    /// A piece of a spec joined by `+` that names no section (`Kyrie + pax`): `+` joins only
    /// sections of the Ordinary, and an own text cannot have it.
    #[error(
        "unknown section \"{id}\"; \"+\" joins only the sections of the Ordinary, which are {}; an own Latin text cannot have a \"+\"",
        valid.join(", ")
    )]
    UnknownPiece {
        id: String,
        /// The ids of all sections.
        valid: Vec<String>,
    },
    #[error("invalid range \"{spec}\": the section has the units 1 to {units}")]
    BadRange { spec: String, units: usize },
}

impl TextError {
    /// Message for users in their language.
    pub fn message(&self, lang: Lang) -> String {
        if lang == Lang::En {
            let text = self.to_string();
            let mut chars = text.chars();
            return chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).chain(['.']).collect()
            });
        }
        match self {
            TextError::Empty => "Der Text ist leer.".into(),
            TextError::Character { word, ch } => {
                format!(
                    "Unerlaubtes Zeichen {} in „{word}“.",
                    quoted(*ch, ("„", "“"))
                )
            }
            TextError::EmptySyllable { word } => format!("Leere Silbe in „{word}“."),
            TextError::NoVowel { word, syllable } => {
                format!("Die Silbe „{syllable}“ in „{word}“ hat keinen Vokal.")
            }
            TextError::SeveralAcutes { word } => format!("„{word}“ hat mehr als einen Akut."),
            TextError::Marker { marker } => {
                format!("Die Marke {marker} muss zwischen zwei Wörtern stehen.")
            }
            TextError::TooLong => format!(
                "Der Text ist zu lang: höchstens {MAX_SYLLABLES} Silben und {} in einem Wort.",
                u8::MAX
            ),
            TextError::UnknownSection { id, valid } => format!(
                "Unbekannter Abschnitt „{id}“; die Abschnitte sind {}; ein eigener lateinischer Text braucht ein Leerzeichen, einen Großbuchstaben oder ein Satzzeichen.",
                valid.join(", ")
            ),
            TextError::UnknownPiece { id, valid } => format!(
                "Unbekannter Abschnitt „{id}“; „+“ verbindet nur die Abschnitte des Ordinariums, und die sind {}; ein eigener lateinischer Text kann kein „+“ enthalten.",
                valid.join(", ")
            ),
            TextError::BadRange { spec, units } => format!(
                "Ungültiger Bereich „{spec}“: Der Abschnitt hat die Sinnabschnitte 1 bis {units}."
            ),
        }
    }
}

/// A character for messages, between the quotes `open` and `close`: one that cannot be seen by
/// its code point alone, another one outside ASCII with its code point after it, so that
/// look-alikes show (`„#“`, `U+200E`, `„е“ (U+0435)`).
fn quoted(ch: char, (open, close): (&str, &str)) -> String {
    let code = format!("U+{:04X}", u32::from(ch));
    if invisible(ch) {
        code
    } else if ch.is_ascii() {
        format!("{open}{ch}{close}")
    } else {
        format!("{open}{ch}{close} ({code})")
    }
}

/// Whether a character cannot be seen on its own: control and format characters, whitespace,
/// combining marks and variation selectors.
fn invisible(ch: char) -> bool {
    ch.is_control()
        || ch.is_whitespace()
        || matches!(
            ch,
            '\u{ad}'
                | '\u{300}'..='\u{36f}'
                | '\u{61c}'
                | '\u{180e}'
                | '\u{1ab0}'..='\u{1aff}'
                | '\u{1dc0}'..='\u{1dff}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{20d0}'..='\u{20ff}'
                | '\u{fe00}'..='\u{fe0f}'
                | '\u{fe20}'..='\u{fe2f}'
                | '\u{feff}'
                | '\u{fff9}'..='\u{fffb}'
                | '\u{e0000}'..='\u{e0fff}'
        )
}

/// The text without the characters of [`IGNORED`].
fn without_ignored(src: &str) -> String {
    src.chars().filter(|c| !IGNORED.contains(c)).collect()
}

impl Text {
    /// Parses a text: words separated by whitespace, with hyphens between syllables, acutes,
    /// punctuation and the markers `/` and `|`.
    ///
    /// A word with hyphens keeps its division; a word without goes to [`latin::syllabify`]. An
    /// acute marks the stress: á é í ó ú ý, ǽ, or a vowel followed by U+0301 (æ and œ are read
    /// as ae and oe). Without one, a disyllable is stressed on its first syllable and longer
    /// words as [`latin::syllabify`] finds; these get the acute, as in the books, and a warning
    /// if their stress is a guess. A diaeresis (ë ï ü, or U+0308) keeps a vowel apart from the
    /// one before it: po-ë-ta. Byte order marks, soft hyphens and zero-width characters are
    /// dropped.
    pub fn parse(
        id: &str,
        names: (&str, &str),
        kind: TextKind,
        src: &str,
        options: &TextOptions,
    ) -> Result<(Text, Vec<TextWarning>), TextError> {
        let mut words: Vec<ReadWord> = Vec::new();
        let mut warnings = Vec::new();
        let mut marker: Option<char> = None;
        let src = without_ignored(src);
        for token in tokens(&src) {
            match token {
                Token::Marker(m) => {
                    if words.is_empty() {
                        return Err(TextError::Marker { marker: m });
                    }
                    marker = Some(if marker == Some('/') { '/' } else { m });
                }
                Token::Word(raw) => {
                    let body = raw.trim_end_matches(PUNCTUATION);
                    let trail = &raw[body.len()..];
                    if body.is_empty() {
                        // Punctuation standing alone belongs to the word before it.
                        match words.last_mut() {
                            Some(last) if marker.is_none() => last.punctuate(trail),
                            _ => {
                                return Err(TextError::Character {
                                    word: raw.into(),
                                    ch: trail.chars().next().unwrap_or(' '),
                                });
                            }
                        }
                        continue;
                    }
                    let mut word = ReadWord::read(body, options, &mut warnings)?;
                    word.punctuate(trail);
                    if let Some(m) = marker.take() {
                        word.break_before = true;
                        word.tail = m == '/';
                    }
                    words.push(word);
                }
            }
        }
        if let Some(m) = marker {
            return Err(TextError::Marker { marker: m });
        }
        if words.is_empty() {
            return Err(TextError::Empty);
        }
        if words.iter().map(|w| w.syllables.len()).sum::<usize>() > MAX_SYLLABLES {
            return Err(TextError::TooLong);
        }
        Ok((assemble(id, names, kind, words), warnings))
    }

    /// Parses an own Latin text: its id is [`OWN_ID`], and it is short up to [`SHORT_MAX`]
    /// syllables.
    pub fn own(src: &str, options: &TextOptions) -> Result<(Text, Vec<TextWarning>), TextError> {
        let names = ("Eigener Text", "Own text");
        let (mut text, warnings) = Text::parse(OWN_ID, names, TextKind::Short, src, options)?;
        text.kind = TextKind::of_length(text.syllables.len());
        Ok((text, warnings))
    }

    /// The name in the user's language, e.g. „Gloria: Et in terra 1–5“.
    pub fn name(&self, lang: Lang) -> &str {
        lang.pick(&self.name_de, &self.name_en)
    }

    /// The word of a syllable.
    pub fn word_of(&self, syl: usize) -> &Word {
        &self.words[usize::from(self.syllables[syl].word)]
    }

    /// The unit of a syllable.
    pub fn unit_of(&self, syl: usize) -> &Unit {
        &self.units[usize::from(self.syllables[syl].unit)]
    }

    /// The sentences, as ranges of unit indices.
    pub fn sentences(&self) -> impl Iterator<Item = RangeInclusive<usize>> {
        self.units
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.stop == Stop::Full)
            .map(|(i, unit)| usize::from(unit.sentence)..=i)
    }

    /// Whether a syllable opens a part of the text: it starts a unit or a tail, or follows a
    /// marked break.
    pub fn is_opening(&self, syl: usize) -> bool {
        self.syllables[syl]
            .flags
            .intersects(SylFlags::UNIT_START | SylFlags::TAIL_START)
            || (syl > 0
                && self.syllables[syl - 1]
                    .flags
                    .contains(SylFlags::BREAK_AFTER))
    }

    /// Where a voice may restart after a syllable with UNIT_END: every start of a unit or a
    /// tail from the first unit of its sentence up to its own unit. Empty for other syllables.
    pub fn restart_targets(&self, syl: usize) -> impl Iterator<Item = usize> {
        let range = if self.syllables[syl].flags.contains(SylFlags::UNIT_END) {
            let unit = self.unit_of(syl);
            usize::from(self.units[usize::from(unit.sentence)].first)..usize::from(unit.last) + 1
        } else {
            0..0
        };
        range.filter(|&i| {
            self.syllables[i]
                .flags
                .intersects(SylFlags::UNIT_START | SylFlags::TAIL_START)
        })
    }

    /// A syllable as written under its note: the book form, without the acute unless `accents`
    /// is set, with a `-` if the word goes on and with the word's punctuation after its last
    /// syllable.
    pub fn syllable_label(&self, syl: usize, accents: bool) -> String {
        let syllable = &self.syllables[syl];
        let mut label = if accents {
            syllable.text.clone()
        } else {
            latin::strip_acute(&syllable.text)
        };
        if !syllable.flags.contains(SylFlags::WORD_END) {
            label.push('-');
        } else if let Some(punct) = self.word_of(syl).punct {
            label.push(punct);
        }
        label
    }

    /// The whole text with hyphens and punctuation, the acutes only if `accents` is set and the
    /// markers `/` and `|` only if `markers` is set. With both set, parsing the result again
    /// gives the same text, except that no stress is uncertain any more.
    pub fn render(&self, accents: bool, markers: bool) -> String {
        let mut out = String::new();
        for (w, word) in self.words.iter().enumerate() {
            let first = usize::from(word.first);
            if w > 0 {
                let broken = self.syllables[first - 1]
                    .flags
                    .contains(SylFlags::BREAK_AFTER);
                let tail = self.syllables[first].flags.contains(SylFlags::TAIL_START);
                out.push_str(match (markers && broken, tail) {
                    (true, true) => " / ",
                    (true, false) => " | ",
                    (false, _) => " ",
                });
            }
            for i in word.range() {
                if i > first {
                    out.push('-');
                }
                if accents {
                    out.push_str(&self.syllables[i].text);
                } else {
                    out.push_str(&latin::strip_acute(&self.syllables[i].text));
                }
            }
            out.extend(word.punct);
        }
        out
    }

    /// The text as [`resolve`] takes it back: the id of a section or of some of its units
    /// (`kyrie`, `gloria:1-5`) or of several sung one after the other (`sanctus+pleni+osanna`),
    /// an own text in full with its acutes and markers. The division of eleison is not part of
    /// it, nor whether the words go on after it ([`Text::continued`]), which belongs to the part
    /// that sings it.
    pub fn spec(&self) -> String {
        if self.id == OWN_ID {
            self.render(true, true)
        } else {
            self.id.clone()
        }
    }

    /// The units in `units` as a text of their own, which ends at a full stop.
    fn select_units(
        &self,
        units: RangeInclusive<usize>,
        id: String,
        names: (String, String),
    ) -> Text {
        let first_unit = *units.start();
        let s0 = usize::from(self.units[first_unit].first);
        let s1 = usize::from(self.units[*units.end()].last);
        let w0 = self.syllables[s0].word;
        let w1 = usize::from(self.syllables[s1].word);
        // Every index fits into 16 bits, so the shifts do too.
        let (s_shift, u_shift) = (s0 as u16, first_unit as u16);
        let mut syllables: Vec<Syllable> = self.syllables[s0..=s1]
            .iter()
            .map(|s| Syllable {
                text: s.text.clone(),
                word: s.word - w0,
                unit: s.unit - u_shift,
                flags: s.flags,
            })
            .collect();
        let words = self.words[usize::from(w0)..=w1]
            .iter()
            .map(|w| Word {
                first: w.first - s_shift,
                ..*w
            })
            .collect();
        let mut units: Vec<Unit> = self.units[units]
            .iter()
            .map(|u| Unit {
                first: u.first - s_shift,
                last: u.last - s_shift,
                stop: u.stop,
                sentence: u.sentence.saturating_sub(u_shift),
            })
            .collect();
        if let (Some(unit), Some(last)) = (units.last_mut(), syllables.last_mut()) {
            unit.stop = Stop::Full;
            last.flags.insert(SylFlags::FULL_STOP);
            last.flags.remove(SylFlags::BREAK_AFTER);
        }
        Text {
            id,
            name_de: names.0,
            name_en: names.1,
            kind: self.kind,
            syllables,
            words,
            units,
            continued: false,
        }
    }

    /// Texts sung one after the other as one text (phase 14), such as the Sanctus, the Pleni and
    /// the Osanna in one section of a Missa brevis. The syllables, words and units of each follow
    /// those of the text before it, and so do its sentences, for every text ends at a full stop:
    /// a voice restates only within the sentence it sings. The id joins the ids with `+`
    /// (`sanctus+pleni+osanna`), which [`resolve`] reads back for sections of the Ordinary; an own
    /// text has no id that names it. The names are joined too, each without the movement the name
    /// before it begins with (Credo: Crucifixus · Et resurrexit 1–2). The text is long if any of
    /// them is, and it goes on if the last one does. One text gives itself.
    ///
    /// # Panics
    ///
    /// If the texts have more than [`MAX_SYLLABLES`] syllables together.
    pub fn concat(texts: Vec<Text>) -> Text {
        let total: usize = texts.iter().map(|text| text.syllables.len()).sum();
        assert!(
            total <= MAX_SYLLABLES,
            "texts with more syllables than the longest possible text"
        );
        let ids: Vec<&str> = texts.iter().map(|text| text.id.as_str()).collect();
        let mut out = Text {
            id: ids.join("+"),
            name_de: joined_names(texts.iter().map(|text| text.name_de.as_str())),
            name_en: joined_names(texts.iter().map(|text| text.name_en.as_str())),
            kind: if texts.iter().any(|text| text.kind == TextKind::Long) {
                TextKind::Long
            } else {
                TextKind::Short
            },
            syllables: Vec::with_capacity(total),
            words: Vec::new(),
            units: Vec::new(),
            continued: texts.last().is_some_and(|text| text.continued),
        };
        for text in texts {
            // Within the limit every index fits into 16 bits, and so does every shift.
            let (s0, w0, u0) = (
                out.syllables.len() as u16,
                out.words.len() as u16,
                out.units.len() as u16,
            );
            out.syllables
                .extend(text.syllables.into_iter().map(|s| Syllable {
                    word: s.word + w0,
                    unit: s.unit + u0,
                    ..s
                }));
            out.words.extend(text.words.into_iter().map(|w| Word {
                first: w.first + s0,
                ..w
            }));
            out.units.extend(text.units.into_iter().map(|u| Unit {
                first: u.first + s0,
                last: u.last + s0,
                sentence: u.sentence + u0,
                ..u
            }));
        }
        out
    }
}

/// The names of texts sung one after the other, joined by ` · `, each without the movement that
/// the name before it begins with: `Sanctus`, `Sanctus: Pleni` and `Sanctus: Osanna` give
/// `Sanctus · Pleni · Osanna`.
fn joined_names<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let mut out = String::new();
    let mut movement: Option<&str> = None;
    for name in names {
        let split = name.split_once(": ");
        let shown = match split {
            Some((own, rest)) if movement == Some(own) => rest,
            _ => name,
        };
        if !out.is_empty() {
            out.push_str(" · ");
        }
        out.push_str(shown);
        movement = Some(split.map_or(name, |(own, _)| own));
    }
    out
}

/// Finds the text a user asks for: a section of the Ordinary (`kyrie`), some of its units,
/// counted from 1 (`gloria:1-5`, `gloria:3`), several of these sung one after the other, joined
/// by `+` (`sanctus+pleni+osanna`, `crucifixus+resurrexit:1-2`; [`Text::concat`]), or an own
/// Latin text, recognised by whitespace or a character outside `a`–`z`, `0`–`9`, `:`, `-` and
/// `+`. A section is found in any case and with spaces around the colon (`Credo`,
/// `Gloria: 1-5`) and around a `+`, as long as the rest has no whitespace.
pub fn resolve(spec: &str, options: &TextOptions) -> Result<(Text, Vec<TextWarning>), TextError> {
    let cleaned = without_ignored(spec);
    let spec = cleaned.trim();
    if let Some(pieces) = joined_spec(spec, options) {
        let texts = pieces
            .iter()
            .map(|piece| {
                section_text(piece, options).map_err(|error| match error {
                    TextError::UnknownSection { id, valid } => {
                        TextError::UnknownPiece { id, valid }
                    }
                    error => error,
                })
            })
            .collect::<Result<Vec<Text>, TextError>>()?;
        if texts.iter().map(|text| text.syllables.len()).sum::<usize>() > MAX_SYLLABLES {
            return Err(TextError::TooLong);
        }
        return Ok((Text::concat(texts), Vec::new()));
    }
    let named = section_spec(spec, options);
    let spec = match &named {
        Some(named) => named.as_str(),
        None if spec.is_empty() || !plain_spec(spec) => return Text::own(spec, options),
        None => spec,
    };
    Ok((section_text(spec, options)?, Vec::new()))
}

/// A section of the Ordinary or some of its units, by a spec in the form [`resolve`] reads
/// (`kyrie`, `gloria:1-5`: lowercase, without spaces).
fn section_text(spec: &str, options: &TextOptions) -> Result<Text, TextError> {
    let (id, range) = match spec.split_once(':') {
        Some((id, range)) => (id, Some(range)),
        None => (spec, None),
    };
    let Some(section) = ordinary::section(id, options) else {
        return Err(TextError::UnknownSection {
            id: id.to_string(),
            valid: ordinary::sections(options)
                .iter()
                .map(|text| text.id.clone())
                .collect(),
        });
    };
    let Some(range) = range else {
        return Ok(section.clone());
    };
    let units = section.units.len();
    let (a, b) = parse_range(range)
        .filter(|&(a, b)| 1 <= a && a <= b && b <= units)
        .ok_or_else(|| TextError::BadRange {
            spec: spec.to_string(),
            units,
        })?;
    let (id, label) = if a == b {
        (format!("{id}:{a}"), a.to_string())
    } else {
        (format!("{id}:{a}-{b}"), format!("{a}–{b}"))
    };
    let names = (
        format!("{} {label}", section.name_de),
        format!("{} {label}", section.name_en),
    );
    Ok(section.select_units(a - 1..=b - 1, id, names))
}

/// Whether a spec has only the characters of a section's id and range: `a`–`z`, `0`–`9`, `:` and
/// `-`.
fn plain_spec(spec: &str) -> bool {
    spec.chars()
        .all(|c| matches!(c, 'a'..='z' | '0'..='9' | ':' | '-'))
}

/// A spec without the spaces around its colon: `Gloria : 1-5` gives `Gloria:1-5`.
fn compact(spec: &str) -> String {
    match spec.split_once(':') {
        Some((id, range)) => format!("{}:{}", id.trim_end(), range.trim_start()),
        None => spec.to_string(),
    }
}

/// A spec that names a section in any case, perhaps with spaces around the colon of a range, in
/// the form [`resolve`] reads: `Gloria: 1-5` gives `gloria:1-5`. `None` if what is left has
/// whitespace or its id is no section.
fn section_spec(spec: &str, options: &TextOptions) -> Option<String> {
    let compact = compact(spec);
    if compact.contains(char::is_whitespace) {
        return None;
    }
    let lower = compact.to_lowercase();
    let id = lower.split_once(':').map_or(lower.as_str(), |(id, _)| id);
    ordinary::section(id, options).is_some().then_some(lower)
}

/// The pieces of a spec that joins sections or their units with `+`, each in the form
/// [`section_text`] reads: `Crucifixus + Resurrexit: 1-2` gives `crucifixus` and
/// `resurrexit:1-2`. Each piece is read as [`resolve`] reads a spec alone: a section in any case,
/// otherwise the characters of an id and a range. `None` without a `+`, or if a piece is empty
/// or would be an own text: then the whole spec is one, which cannot have a `+`.
fn joined_spec(spec: &str, options: &TextOptions) -> Option<Vec<String>> {
    if !spec.contains('+') {
        return None;
    }
    spec.split('+')
        .map(|piece| {
            let piece = piece.trim();
            section_spec(piece, options)
                .or_else(|| (!piece.is_empty() && plain_spec(piece)).then(|| piece.to_string()))
        })
        .collect()
}

/// `A-B` or `A`.
fn parse_range(range: &str) -> Option<(usize, usize)> {
    match range.split_once('-') {
        Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
        None => range.parse().ok().map(|a| (a, a)),
    }
}

enum Token<'a> {
    Word(&'a str),
    /// `/` or `|`.
    Marker(char),
}

/// The words and markers of a text; a marker need not be separated by spaces.
fn tokens(src: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    for piece in src.split_whitespace() {
        let mut start = 0;
        for (i, marker) in piece.match_indices(['/', '|']) {
            if i > start {
                out.push(Token::Word(&piece[start..i]));
            }
            out.push(Token::Marker(if marker == "/" { '/' } else { '|' }));
            start = i + 1;
        }
        if start < piece.len() {
            out.push(Token::Word(&piece[start..]));
        }
    }
    out
}

/// A word as read, before the text is assembled.
struct ReadWord {
    syllables: Vec<String>,
    stress: Option<usize>,
    uncertain: bool,
    function: bool,
    punct: Option<char>,
    stop: Option<Stop>,
    /// A marker stands before it.
    break_before: bool,
    /// The marker before it is `/`.
    tail: bool,
}

impl ReadWord {
    /// Reads a word without punctuation.
    fn read(
        body: &str,
        options: &TextOptions,
        warnings: &mut Vec<TextWarning>,
    ) -> Result<ReadWord, TextError> {
        let word = compose_acutes(body)?;
        let pieces: Vec<&str> = word.split('-').collect();
        if pieces.iter().any(|piece| piece.is_empty()) {
            return Err(TextError::EmptySyllable { word: body.into() });
        }
        if let Some(piece) = pieces.iter().find(|p| !p.chars().any(latin::is_vowel)) {
            return Err(TextError::NoVowel {
                word: body.into(),
                syllable: piece.to_string(),
            });
        }
        let acutes = word.chars().filter(|&c| latin::is_acute(c)).count();
        if acutes > 1 {
            return Err(TextError::SeveralAcutes { word: body.into() });
        }
        let joined = pieces.concat();
        let found = (pieces.len() == 1).then(|| latin::syllabify(&joined));
        let mut syllables: Vec<String> = match &found {
            Some(found) => found.syllables.clone(),
            None => pieces.iter().map(|piece| piece.to_string()).collect(),
        };
        debug_assert_eq!(syllables.concat(), joined, "syllabify keeps the letters");
        if syllables.len() > usize::from(u8::MAX) {
            return Err(TextError::TooLong);
        }
        let acute = syllables.iter().position(|s| latin::has_acute(s));
        let plain = latin::normalize(&joined);
        let function = syllables.len() == 1 && acute.is_none() && FUNCTION_WORDS.contains(&&*plain);
        let (stress, mut uncertain) = match (acute, syllables.len()) {
            (Some(k), _) => (Some(k), false),
            (None, 1) => ((!function).then_some(0), false),
            (None, 2) => (Some(0), false),
            (None, _) => {
                let found = found.unwrap_or_else(|| latin::syllabify(&joined));
                (Some(stress_in(&syllables, &found)), found.uncertain)
            }
        };
        // A diaeresis on its i keeps the four syllables (e-le-ï-son).
        if options.eleison == Eleison::Three
            && plain == "eleison"
            && syllables.len() == 4
            && stress == Some(1)
            && !syllables[2].chars().any(latin::is_diaeresis)
        {
            let i = syllables.remove(2);
            syllables[1].push_str(&i);
            uncertain = false;
        }
        if syllables.len() >= 3
            && acute.is_none()
            && let Some(k) = stress
        {
            syllables[k] = latin::add_acute(&syllables[k]);
            if uncertain {
                let warning = TextWarning::UncertainAccent {
                    word: joined,
                    result: syllables.join("-"),
                };
                if !warnings.contains(&warning) {
                    warnings.push(warning);
                }
            }
        }
        Ok(ReadWord {
            syllables,
            stress,
            uncertain,
            function,
            punct: None,
            stop: None,
            break_before: false,
            tail: false,
        })
    }

    /// Takes the punctuation after the word. The strongest stop wins, and of several marks with
    /// that stop the last is kept, so that the printed mark shows the stop: `?,` keeps `?`.
    fn punctuate(&mut self, trail: &str) {
        for mark in trail.chars() {
            let stop = if matches!(mark, '.' | '?' | '!') {
                Stop::Full
            } else {
                Stop::Intermediate
            };
            if self.stop.is_none_or(|current| stop >= current) {
                self.punct = Some(mark);
                self.stop = Some(stop);
            }
        }
    }
}

/// Checks the characters of a word and composes a vowel and a combining acute or diaeresis
/// where Unicode has a precomposed letter.
fn compose_acutes(word: &str) -> Result<String, TextError> {
    let invalid = |ch: char| TextError::Character {
        word: word.into(),
        ch,
    };
    let mut out = String::with_capacity(word.len());
    let mut prev: Option<char> = None;
    for c in word.chars() {
        if c == latin::COMBINING_ACUTE {
            match prev.filter(|&p| latin::is_vowel(p)) {
                Some(p) => match latin::precomposed_acute(p) {
                    Some(accented) => {
                        out.pop();
                        out.push(accented);
                    }
                    None => out.push(c),
                },
                None => return Err(invalid(c)),
            }
        } else if c == latin::COMBINING_DIAERESIS {
            let Some(marked) = prev.and_then(latin::precomposed_diaeresis) else {
                return Err(invalid(c));
            };
            out.pop();
            out.push(marked);
            prev = Some(marked);
            continue;
        } else if c.is_ascii_alphabetic() || c == '-' || latin::is_vowel(c) {
            out.push(c);
        } else {
            return Err(invalid(c));
        }
        prev = Some(c);
    }
    Ok(out)
}

/// The stressed syllable of a given division: the one that holds the nucleus of the syllable
/// the algorithm stresses in its own division of the word.
fn stress_in(given: &[String], found: &latin::Syllabified) -> usize {
    let fallback = given.len() - 3;
    let Some(k) = found.stress else {
        return fallback;
    };
    if found.syllables.len() == given.len() {
        return k;
    }
    let before: usize = found.syllables[..k].iter().map(|s| s.chars().count()).sum();
    let at = before + latin::nucleus(&found.syllables[k]).unwrap_or(0);
    let mut end = 0;
    for (i, syllable) in given.iter().enumerate() {
        end += syllable.chars().count();
        if at < end {
            return i;
        }
    }
    fallback
}

/// Builds the text from its words: indices, units, sentences and flags.
fn assemble(id: &str, names: (&str, &str), kind: TextKind, read: Vec<ReadWord>) -> Text {
    let mut syllables: Vec<Syllable> = Vec::new();
    let mut words = Vec::with_capacity(read.len());
    let mut units: Vec<Unit> = Vec::new();
    let (mut unit_first, mut sentence) = (0, 0);
    let count = read.len();
    // The parser limits the number of syllables, so every index fits into 16 bits.
    let index = |i: usize| i as u16;
    for (w, word) in read.into_iter().enumerate() {
        let first = syllables.len();
        let len = word.syllables.len();
        if word.break_before {
            syllables[first - 1].flags.insert(SylFlags::BREAK_AFTER);
        }
        for (k, text) in word.syllables.into_iter().enumerate() {
            let mut flags = SylFlags::empty();
            if k == 0 {
                flags |= SylFlags::WORD_START;
                if word.tail {
                    flags |= SylFlags::TAIL_START;
                }
            }
            if k + 1 == len {
                flags |= SylFlags::WORD_END;
            }
            if let Some(s) = word.stress {
                if k == s {
                    flags |= SylFlags::STRESS;
                }
                if len >= 4 && (k + 2 == s || k + 4 == s) {
                    flags |= SylFlags::SECONDARY;
                }
                if len >= 3 && s + 3 == len && k + 2 == len {
                    flags |= SylFlags::SHORT_PENULT;
                }
            }
            if word.function {
                flags |= SylFlags::FUNCTION_WORD;
            }
            if word.uncertain {
                flags |= SylFlags::UNCERTAIN;
            }
            syllables.push(Syllable {
                text,
                word: index(w),
                unit: index(units.len()),
                flags,
            });
        }
        words.push(Word {
            first: index(first),
            len: len as u8,
            stress: word.stress.map(|s| s as u8),
            punct: word.punct,
        });
        let stop = if w + 1 == count {
            Some(Stop::Full)
        } else {
            word.stop
        };
        if let Some(stop) = stop {
            let last = syllables.len() - 1;
            syllables[unit_first].flags.insert(SylFlags::UNIT_START);
            syllables[last].flags.insert(SylFlags::UNIT_END);
            if stop == Stop::Full {
                syllables[last].flags.insert(SylFlags::FULL_STOP);
            }
            units.push(Unit {
                first: index(unit_first),
                last: index(last),
                stop,
                sentence: index(sentence),
            });
            if stop == Stop::Full {
                sentence = units.len();
            }
            unit_first = syllables.len();
        }
    }
    Text {
        id: id.into(),
        name_de: names.0.into(),
        name_en: names.1.into(),
        kind,
        syllables,
        words,
        units,
        continued: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn own(src: &str) -> (Text, Vec<TextWarning>) {
        Text::own(src, &TextOptions::default()).unwrap()
    }

    fn texts(text: &Text) -> Vec<&str> {
        text.syllables.iter().map(|s| s.text.as_str()).collect()
    }

    fn flagged(text: &Text, flag: SylFlags) -> Vec<usize> {
        (0..text.syllables.len())
            .filter(|&i| text.syllables[i].flags.contains(flag))
            .collect()
    }

    /// The flags about stress of the syllables of the first word written as `word`.
    fn stress_flags(text: &Text, word: &str) -> Vec<SylFlags> {
        let kinds = [
            SylFlags::STRESS,
            SylFlags::SECONDARY,
            SylFlags::SHORT_PENULT,
            SylFlags::FUNCTION_WORD,
        ];
        let found = text
            .words
            .iter()
            .find(|w| {
                let written: Vec<&str> =
                    w.range().map(|i| text.syllables[i].text.as_str()).collect();
                written.join("-") == word
            })
            .unwrap_or_else(|| panic!("{word} not in {}", text.id));
        found
            .range()
            .map(|i| {
                kinds
                    .into_iter()
                    .filter(|&k| text.syllables[i].flags.contains(k))
                    .fold(SylFlags::empty(), |a, b| a | b)
            })
            .collect()
    }

    /// Checks that indices, flags, units and sentences agree, and that the rendered text parses
    /// back to the same text.
    fn check(text: &Text, options: &TextOptions) {
        consistent(text);
        let names = (text.name_de.as_str(), text.name_en.as_str());
        let src = text.render(true, true);
        let (again, _) = Text::parse(&text.id, names, text.kind, &src, options).unwrap();
        assert_eq!(&again, text, "{src}");
    }

    /// Checks that indices, flags, units and sentences agree.
    fn consistent(text: &Text) {
        let id = &text.id;
        for (w, word) in text.words.iter().enumerate() {
            assert!(word.len > 0, "{id}");
            for i in word.range() {
                let flags = text.syllables[i].flags;
                assert_eq!(usize::from(text.syllables[i].word), w, "{id}");
                assert_eq!(
                    flags.contains(SylFlags::WORD_START),
                    i == word.range().start
                );
                assert_eq!(
                    flags.contains(SylFlags::WORD_END),
                    i + 1 == word.range().end
                );
                let stressed = word
                    .stress
                    .map(|k| usize::from(word.first) + usize::from(k));
                assert_eq!(
                    flags.contains(SylFlags::STRESS),
                    stressed == Some(i),
                    "{id}"
                );
            }
        }
        let mut next = 0;
        for (u, unit) in text.units.iter().enumerate() {
            assert_eq!(usize::from(unit.first), next, "{id}: units are contiguous");
            next = usize::from(unit.last) + 1;
            for i in unit.range() {
                let flags = text.syllables[i].flags;
                assert_eq!(usize::from(text.syllables[i].unit), u, "{id}");
                assert_eq!(
                    flags.contains(SylFlags::UNIT_START),
                    i == unit.range().start
                );
                assert_eq!(
                    flags.contains(SylFlags::UNIT_END),
                    i + 1 == unit.range().end
                );
                let full = unit.stop == Stop::Full && i + 1 == unit.range().end;
                assert_eq!(flags.contains(SylFlags::FULL_STOP), full, "{id}");
            }
            let sentence = usize::from(unit.sentence);
            assert!(sentence <= u);
            assert!(sentence == 0 || text.units[sentence - 1].stop == Stop::Full);
            assert!(
                text.units[sentence..u]
                    .iter()
                    .all(|v| v.stop == Stop::Intermediate)
            );
        }
        assert_eq!(next, text.syllables.len(), "{id}");
        assert_eq!(text.units.last().map(|u| u.stop), Some(Stop::Full), "{id}");
    }

    #[test]
    fn texts_are_consistent() {
        for eleison in [Eleison::Four, Eleison::Three] {
            let options = TextOptions { eleison };
            for text in ordinary::sections(&options) {
                check(text, &options);
            }
            for spec in [
                "gloria:1-5",
                "gloria:7",
                "qui-tollis:2-3",
                "spiritum:4-12",
                "agnus:3",
            ] {
                check(&resolve(spec, &options).unwrap().0, &options);
            }
            let (text, _) = Text::own("Ad te le-vá-vi á-ni-mam me-am.", &options).unwrap();
            check(&text, &options);
        }
    }

    #[test]
    fn own_text_parses() {
        // Given hyphens and acutes win.
        let (text, warnings) = own("Ad te le-vá-vi á-ni-mam me-am.");
        assert!(warnings.is_empty());
        assert_eq!(
            texts(&text),
            ["Ad", "te", "le", "vá", "vi", "á", "ni", "mam", "me", "am"]
        );
        assert_eq!((text.id.as_str(), text.kind), (OWN_ID, TextKind::Short));
        assert_eq!(text.name(Lang::De), "Eigener Text");
        let stresses: Vec<Option<u8>> = text.words.iter().map(|w| w.stress).collect();
        assert_eq!(stresses, [None, Some(0), Some(1), Some(0), Some(0)]);
        assert_eq!(flagged(&text, SylFlags::STRESS), [1, 3, 5, 8]);
        assert_eq!(flagged(&text, SylFlags::FUNCTION_WORD), [0]);
        assert_eq!(flagged(&text, SylFlags::SHORT_PENULT), [6]);
        let (text, _) = own("e-léi-son Dé-us ét-i-am De-ús");
        assert_eq!(
            texts(&text),
            ["e", "léi", "son", "Dé", "us", "ét", "i", "am", "De", "ús"]
        );
        let stresses: Vec<Option<u8>> = text.words.iter().map(|w| w.stress).collect();
        assert_eq!(stresses, [Some(1), Some(0), Some(0), Some(1)]);
        // An acute on a monosyllable makes it stressed, even a function word.
        let (text, _) = own("ét in");
        assert_eq!(flagged(&text, SylFlags::STRESS), [0]);
        assert_eq!(flagged(&text, SylFlags::FUNCTION_WORD), [1]);

        // Without an acute, a disyllable is stressed on its first syllable and a longer word
        // gets the acute; words without hyphens are divided.
        let (text, warnings) = own("Ky-ri-e Deus pax");
        assert_eq!(texts(&text), ["Ký", "ri", "e", "De", "us", "pax"]);
        assert!(warnings.is_empty());
        // An uncertain accent: a warning, the flag and the acute.
        let (text, warnings) = own("Ad te a-ni-mam");
        assert_eq!(texts(&text)[2..], ["á", "ni", "mam"]);
        assert_eq!(
            warnings,
            [TextWarning::UncertainAccent {
                word: "animam".into(),
                result: "á-ni-mam".into()
            }]
        );
        assert_eq!(
            warnings[0].message(Lang::De),
            "Akzent unsicher: animam → á-ni-mam; mit Akut angeben"
        );
        assert_eq!(
            warnings[0].to_string(),
            "Uncertain accent: animam → á-ni-mam; mark it with an acute"
        );
        assert_eq!(flagged(&text, SylFlags::UNCERTAIN), [2, 3, 4]);
        assert_eq!(own("a-ni-mam, a-ni-mam.").1.len(), 1);
        // A known ending decides: le-vá-vi without a warning.
        let (text, warnings) = own("Ad te le-va-vi");
        assert_eq!(texts(&text)[2..], ["le", "vá", "vi"]);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(flagged(&text, SylFlags::UNCERTAIN).is_empty());

        // A diaeresis keeps a hiatus, given or composed, and stays in the syllable.
        let (text, warnings) = own("po\u{eb}ta ae\u{308}r Isai\u{308}as");
        assert_eq!(
            texts(&text),
            ["pó", "ë", "ta", "a", "ër", "I", "sá", "ï", "as"]
        );
        assert_eq!(text.render(false, false), "po-ë-ta a-ër I-sa-ï-as");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        let (text, _) = own("po-ë-ta Mi-cha-ë\u{301}-lem");
        assert_eq!(
            texts(&text),
            ["pó", "ë", "ta", "Mi", "cha", "ë\u{301}", "lem"]
        );
        assert_eq!(flagged(&text, SylFlags::STRESS), [0, 5]);

        // Combining acutes are composed, ligatures kept.
        let (text, _) = own("Le\u{301}-va-vi cæ-li sǽ-cu-la cœ\u{301}-li-tus");
        assert_eq!(
            texts(&text),
            [
                "Lé",
                "va",
                "vi",
                "cæ",
                "li",
                "sǽ",
                "cu",
                "la",
                "cœ\u{301}",
                "li",
                "tus"
            ]
        );
        // Punctuation and markers need no spaces; lone punctuation belongs to the word before.
        let (text, _) = own("Ky-ri-e , e-lé-i-son|Chri-ste/e-lé-i-son");
        assert_eq!(
            text.render(true, true),
            "Ký-ri-e, e-lé-i-son | Chri-ste / e-lé-i-son"
        );
        assert_eq!(text.units.len(), 2);
        let (text, _) = own("A | / men");
        assert_eq!(text.render(true, true), "A / men");
        // Own texts count as short up to 20 syllables.
        assert_eq!(own(&"pax ".repeat(20)).0.kind, TextKind::Short);
        assert_eq!(own(&"pax ".repeat(21)).0.kind, TextKind::Long);
    }

    #[test]
    fn units_and_punctuation() {
        let (text, _) = own("Ky-ri-e, e-lé-i-son; Chri-ste: e-lé-i-son? A-men! Pax");
        let units: Vec<(u16, u16, Stop, u16)> = text
            .units
            .iter()
            .map(|u| (u.first, u.last, u.stop, u.sentence))
            .collect();
        use Stop::{Full, Intermediate};
        assert_eq!(
            units,
            [
                (0, 2, Intermediate, 0),
                (3, 6, Intermediate, 0),
                (7, 8, Intermediate, 0),
                (9, 12, Full, 0),
                (13, 14, Full, 4),
                (15, 15, Full, 5)
            ]
        );
        let puncts: Vec<Option<char>> = text.words.iter().map(|w| w.punct).collect();
        assert_eq!(
            puncts,
            [Some(','), Some(';'), Some(':'), Some('?'), Some('!'), None]
        );
        let sentences: Vec<RangeInclusive<usize>> = text.sentences().collect();
        assert_eq!(sentences, [0..=3, 4..=4, 5..=5]);
        assert_eq!(flagged(&text, SylFlags::FULL_STOP), [12, 14, 15]);
        assert_eq!(text.restart_targets(6).collect::<Vec<_>>(), [0, 3]);
        assert_eq!(text.restart_targets(12).collect::<Vec<_>>(), [0, 3, 7, 9]);
        assert_eq!(text.restart_targets(14).collect::<Vec<_>>(), [13]);
        assert_eq!(text.restart_targets(5).count(), 0);
        // The strongest stop of several marks wins, and its mark is kept, so that labels and
        // the rendered text show the stop; of marks with the same stop the last is kept.
        let options = TextOptions::default();
        for (src, stop, mark) in [
            ("Ky-ri-e?, e-lé-i-son", Full, '?'),
            ("Ky-ri-e,? e-lé-i-son", Full, '?'),
            ("Ky-ri-e. , e-lé-i-son", Full, '.'),
            ("Ky-ri-e , . e-lé-i-son", Full, '.'),
            ("Ky-ri-e?! e-lé-i-son", Full, '!'),
            ("Ky-ri-e,; e-lé-i-son", Intermediate, ';'),
        ] {
            let (text, _) = own(src);
            assert_eq!(
                (text.units[0].stop, text.words[0].punct),
                (stop, Some(mark)),
                "{src}"
            );
            assert_eq!(text.syllable_label(2, true), format!("e{mark}"), "{src}");
            assert_eq!(
                text.render(true, true),
                format!("Ký-ri-e{mark} e-lé-i-son"),
                "{src}"
            );
            // Parsing the rendered text again gives the same text.
            check(&text, &options);
        }
    }

    #[test]
    fn stress_flags_of_the_ordinary() {
        let options = TextOptions::default();
        let gloria = ordinary::section("gloria", &options).unwrap();
        let credo = ordinary::section("credo", &options).unwrap();
        let kyrie = ordinary::section("kyrie", &options).unwrap();
        let (none, stress, secondary, short, function) = (
            SylFlags::empty(),
            SylFlags::STRESS,
            SylFlags::SECONDARY,
            SylFlags::SHORT_PENULT,
            SylFlags::FUNCTION_WORD,
        );
        assert_eq!(stress_flags(kyrie, "Ký-ri-e"), [stress, short, none]);
        assert_eq!(
            stress_flags(kyrie, "e-lé-i-son"),
            [none, stress, short, none]
        );
        assert_eq!(
            stress_flags(gloria, "vo-lun-tá-tis"),
            [secondary, none, stress, none]
        );
        assert_eq!(
            stress_flags(gloria, "Be-ne-dí-ci-mus"),
            [secondary, none, stress, short, none]
        );
        assert_eq!(
            stress_flags(gloria, "o-mní-pot-ens"),
            [none, stress, short, none]
        );
        assert_eq!(
            stress_flags(credo, "o-mni-pot-én-tem"),
            [none, secondary, none, stress, none]
        );
        assert_eq!(
            stress_flags(credo, "con-sub-stan-ti-á-lem"),
            [secondary, none, secondary, none, stress, none]
        );
        assert_eq!(stress_flags(gloria, "Lau-dá-mus"), [none, stress, none]);
        assert_eq!(stress_flags(gloria, "De-us"), [stress, none]);
        for word in ["pax", "te", "Rex"] {
            assert_eq!(stress_flags(gloria, word), [stress], "{word}");
        }
        for word in ["non", "sunt", "nos"] {
            assert_eq!(stress_flags(credo, word), [stress], "{word}");
        }
        let crucifixus = ordinary::section("crucifixus", &options).unwrap();
        assert_eq!(stress_flags(crucifixus, "est"), [stress]);
        for word in ["Et", "in", "de", "ex", "per", "quem", "Qui"] {
            assert_eq!(stress_flags(credo, word), [function], "{word}");
        }
        assert!(gloria.syllables[0].flags.contains(SylFlags::FUNCTION_WORD));
        assert!(!gloria.syllables[0].flags.accented());
        assert!(gloria.syllables[4].flags.accented());
    }

    #[test]
    fn restarts_and_openings() {
        let options = TextOptions::default();
        let kyrie = ordinary::section("kyrie", &options).unwrap();
        assert_eq!(kyrie.restart_targets(6).collect::<Vec<_>>(), [0, 3]);
        assert_eq!(kyrie.restart_targets(5).count(), 0);
        let openings: Vec<usize> = (0..7).filter(|&i| kyrie.is_opening(i)).collect();
        assert_eq!(openings, [0, 3]);

        // San-ctus, San-ctus, San-ctus / Dó-mi-nus De-us Sá-ba-oth.
        let sanctus = ordinary::section("sanctus", &options).unwrap();
        assert_eq!(sanctus.restart_targets(1).collect::<Vec<_>>(), [0]);
        assert_eq!(sanctus.restart_targets(3).collect::<Vec<_>>(), [0, 2]);
        assert_eq!(
            sanctus.restart_targets(13).collect::<Vec<_>>(),
            [0, 2, 4, 6]
        );

        let gloria = ordinary::section("gloria", &options).unwrap();
        let sentences: Vec<RangeInclusive<usize>> = gloria.sentences().collect();
        assert_eq!(
            sentences,
            [
                0..=0,
                1..=1,
                2..=2,
                3..=3,
                4..=4,
                5..=5,
                6..=8,
                9..=9,
                10..=12
            ]
        );
        // After "Rex caelestis," a voice may go back to "Domine Deus" or to "Rex".
        let caelestis = usize::from(gloria.units[7].last);
        assert_eq!(gloria.syllable_label(caelestis, true), "stis,");
        let targets: Vec<usize> = gloria.restart_targets(caelestis).collect();
        let starts = [gloria.units[6].first, gloria.units[7].first].map(usize::from);
        assert_eq!(targets, starts);
        // After a full stop, only within the sentence just ended.
        let tuam = usize::from(gloria.units[5].last);
        let targets: Vec<usize> = gloria.restart_targets(tuam).collect();
        assert_eq!(targets, [usize::from(gloria.units[5].first)]);
        // Et in ter-ra pax | ho-mí-ni-bus: the syllable after the break opens.
        assert_eq!(gloria.syllables[5].text, "ho");
        assert!(gloria.is_opening(5));
        assert!(!gloria.is_opening(4) && !gloria.is_opening(6));
        assert_eq!(gloria.word_of(6).first, 5);
        assert_eq!(gloria.unit_of(6).last, 14);
    }

    #[test]
    fn labels() {
        let (kyrie, _) = resolve("kyrie", &TextOptions::default()).unwrap();
        let labels = |accents| -> Vec<String> {
            (0..kyrie.syllables.len())
                .map(|i| kyrie.syllable_label(i, accents))
                .collect()
        };
        assert_eq!(labels(true), ["Ký-", "ri-", "e", "e-", "lé-", "i-", "son."]);
        assert_eq!(
            labels(false),
            ["Ky-", "ri-", "e", "e-", "le-", "i-", "son."]
        );
        assert_eq!(kyrie.render(false, true), "Ky-ri-e / e-le-i-son.");
    }

    #[test]
    fn eleison_three() {
        let three = TextOptions {
            eleison: Eleison::Three,
        };
        let (kyrie, _) = resolve("kyrie", &three).unwrap();
        assert_eq!(texts(&kyrie), ["Ký", "ri", "e", "e", "léi", "son"]);
        assert_eq!((kyrie.words[1].len, kyrie.words[1].stress), (3, Some(1)));
        assert_eq!(flagged(&kyrie, SylFlags::STRESS), [0, 4]);
        assert_eq!(flagged(&kyrie, SylFlags::SHORT_PENULT), [1]);
        assert_eq!(flagged(&kyrie, SylFlags::TAIL_START), [3]);
        assert_eq!(kyrie.render(true, true), "Ký-ri-e / e-léi-son.");
        assert_eq!(kyrie.restart_targets(5).collect::<Vec<_>>(), [0, 3]);
        let (christe, _) = resolve("christe", &three).unwrap();
        assert_eq!(christe.render(false, false), "Chri-ste e-lei-son.");
        // Own texts too, with or without hyphens and acutes.
        for src in [
            "Kyrie eleison.",
            "Ky-ri-e e-le-i-son.",
            "Ký-ri-e e-lé-i-son.",
        ] {
            let (text, warnings) = Text::own(src, &three).unwrap();
            assert_eq!(text.render(true, false), "Ký-ri-e e-léi-son.", "{src}");
            assert!(warnings.is_empty(), "{src}: {warnings:?}");
        }
        // The default keeps the four syllables of the books, and other words stay.
        let (kyrie, _) = resolve("kyrie", &TextOptions::default()).unwrap();
        assert_eq!(kyrie.render(true, false), "Ký-ri-e e-lé-i-son.");
        let (text, _) = Text::own("De-i e-lé-i-son-que", &three).unwrap();
        assert_eq!(text.syllables.len(), 7);
        // A diaeresis keeps the four syllables.
        for src in ["eleïson", "e-le-ï-son"] {
            let (text, _) = Text::own(src, &three).unwrap();
            assert_eq!(text.render(true, false), "e-lé-ï-son", "{src}");
        }
    }

    #[test]
    fn resolve_sections_ranges_and_own_texts() {
        let options = TextOptions::default();
        let (kyrie, warnings) = resolve(" kyrie ", &options).unwrap();
        assert_eq!(
            (kyrie.id.as_str(), kyrie.name(Lang::De)),
            ("kyrie", "Kyrie I")
        );
        assert!(warnings.is_empty());

        let (gloria, _) = resolve("gloria:1-5", &options).unwrap();
        assert_eq!(gloria.id, "gloria:1-5");
        assert_eq!(gloria.name(Lang::En), "Gloria: Et in terra 1–5");
        assert_eq!(gloria.kind, TextKind::Long);
        let counts = (
            gloria.syllables.len(),
            gloria.words.len(),
            gloria.units.len(),
        );
        assert_eq!(counts, (36, 15, 5));
        assert_eq!(
            gloria.render(true, true),
            "Et in ter-ra pax | ho-mí-ni-bus bo-nae vo-lun-tá-tis. Lau-dá-mus te. \
             Be-ne-dí-ci-mus te. Ad-o-rá-mus te. Glo-ri-fi-cá-mus te."
        );

        // Indices start from 0 again, and the end of the selection is a full stop.
        let (domine, _) = resolve("gloria:7", &options).unwrap();
        assert_eq!(
            (domine.id.as_str(), domine.name_de.as_str()),
            ("gloria:7", "Gloria: Et in terra 7")
        );
        assert_eq!(domine.render(true, false), "Dó-mi-ne De-us,");
        let unit = Unit {
            first: 0,
            last: 4,
            stop: Stop::Full,
            sentence: 0,
        };
        assert_eq!(domine.units, [unit]);
        assert_eq!(flagged(&domine, SylFlags::FULL_STOP), [4]);
        let (miserere, _) = resolve("qui-tollis:2-3", &options).unwrap();
        assert_eq!(
            miserere.render(true, false),
            "mi-se-ré-re no-bis. Qui tol-lis pec-cá-ta mun-di,"
        );
        let sentences: Vec<u16> = miserere.units.iter().map(|u| u.sentence).collect();
        assert_eq!(sentences, [0, 1]);
        assert_eq!(miserere.syllables[6].word, 2);

        // Own texts: whitespace, capitals or punctuation.
        for spec in ["Amen", "a-men.", "a men", "A-men"] {
            let (text, _) = resolve(spec, &options).unwrap();
            assert_eq!(
                (text.id.as_str(), text.syllables.len()),
                (OWN_ID, 2),
                "{spec}"
            );
        }

        // A section id in any case, also with a range and spaces around its colon, is the
        // section, not an own text of one word.
        for (spec, id) in [
            ("Credo", "credo"),
            ("KYRIE", "kyrie"),
            ("Qui-Tollis", "qui-tollis"),
            ("Gloria:1-5", "gloria:1-5"),
            ("gloria: 1-5", "gloria:1-5"),
            ("Gloria : 1-5", "gloria:1-5"),
            ("Agnus:3", "agnus:3"),
            ("\u{feff}kyrie", "kyrie"),
        ] {
            let (text, warnings) = resolve(spec, &options).unwrap();
            assert_eq!(text.id, id, "{spec}");
            assert!(warnings.is_empty(), "{spec}");
        }
        let (credo, _) = resolve("Credo", &options).unwrap();
        assert_eq!(credo.syllables.len(), 128);
        assert_eq!(resolve("Gloria: 1-5", &options).unwrap().0, gloria);
        assert!(matches!(
            resolve("Gloria:1-14", &options),
            Err(TextError::BadRange { spec, .. }) if spec == "gloria:1-14"
        ));
        // With other words or punctuation it stays an own text.
        for spec in ["Credo in unum Deum", "Credo.", "Gloria: 1 - 5 ."] {
            let result = resolve(spec, &options);
            assert!(
                !matches!(&result, Ok((text, _)) if text.id != OWN_ID),
                "{spec}: {result:?}"
            );
        }
    }

    #[test]
    fn resolve_errors() {
        let options = TextOptions::default();
        let error = resolve("glria", &options).unwrap_err();
        let TextError::UnknownSection { id, valid } = &error else {
            panic!("{error:?}");
        };
        assert_eq!(id, "glria");
        let ids: Vec<&str> = ordinary::sections(&options)
            .iter()
            .map(|t| t.id.as_str())
            .collect();
        assert_eq!(valid, &ids);
        assert_eq!(valid.len(), 17);
        for lang in [Lang::De, Lang::En] {
            let message = error.message(lang);
            assert!(
                message.contains("„glria“") || message.contains("\"glria\""),
                "{message}"
            );
            assert!(
                message.contains("kyrie, christe, kyrie2, gloria, qui-tollis"),
                "{message}"
            );
            assert!(message.contains("agnus, agnus3"), "{message}");
        }
        assert_eq!(
            error.message(Lang::En),
            "Unknown section \"glria\"; the sections are kyrie, christe, kyrie2, gloria, \
             qui-tollis, credo, incarnatus, crucifixus, resurrexit, spiritum, sanctus, pleni, \
             osanna, benedictus, osanna2, agnus, agnus3; an own Latin text needs a space, a \
             capital letter or a punctuation mark."
        );
        assert!(matches!(
            resolve("glria:1-2", &options),
            Err(TextError::UnknownSection { id, .. }) if id == "glria"
        ));
        assert!(matches!(
            resolve("amen", &options),
            Err(TextError::UnknownSection { .. })
        ));
        for spec in [
            "gloria:0-3",
            "gloria:5-2",
            "gloria:1-14",
            "gloria:x",
            "gloria:",
            "gloria:1-2-3",
            "gloria:-2",
            "kyrie:2",
        ] {
            assert!(
                matches!(resolve(spec, &options), Err(TextError::BadRange { .. })),
                "{spec}"
            );
        }
        let error = resolve("gloria:1-14", &options).unwrap_err();
        assert_eq!(
            error.message(Lang::De),
            "Ungültiger Bereich „gloria:1-14“: Der Abschnitt hat die Sinnabschnitte 1 bis 13."
        );
        assert_eq!(
            error.message(Lang::En),
            "Invalid range \"gloria:1-14\": the section has the units 1 to 13."
        );
        assert_eq!(resolve("  ", &options).unwrap_err(), TextError::Empty);
    }

    /// Sections and ranges joined by `+` are read back from their spec, in any case and with
    /// spaces around a `+` and a colon; the parts keep their syllables, words and units in
    /// order, shifted, and every index agrees.
    #[test]
    fn concat_round_trips() {
        for eleison in [Eleison::Four, Eleison::Three] {
            let options = TextOptions { eleison };
            for spec in [
                "sanctus+pleni+osanna",
                "crucifixus+resurrexit:1-2",
                "kyrie+christe+kyrie2",
                "gloria:1-5+gloria:7",
                "agnus:3+agnus3",
                "benedictus+osanna2",
            ] {
                let (text, warnings) = resolve(spec, &options).unwrap();
                assert!(warnings.is_empty(), "{spec}");
                assert_eq!(text.spec(), spec);
                assert_eq!(resolve(&text.spec(), &options).unwrap().0, text, "{spec}");
                consistent(&text);
                assert!(!text.continued, "{spec}");
                let parts: Vec<Text> = spec
                    .split('+')
                    .map(|piece| resolve(piece, &options).unwrap().0)
                    .collect();
                let syllables: Vec<&Syllable> = parts.iter().flat_map(|p| &p.syllables).collect();
                assert_eq!(syllables.len(), text.syllables.len(), "{spec}");
                for (joined, part) in text.syllables.iter().zip(syllables) {
                    assert_eq!((&joined.text, joined.flags), (&part.text, part.flags));
                }
                let units: usize = parts.iter().map(|p| p.units.len()).sum();
                let sentences: usize = parts.iter().map(|p| p.sentences().count()).sum();
                assert_eq!(text.units.len(), units, "{spec}");
                assert_eq!(text.sentences().count(), sentences, "{spec}");
                let words: Vec<String> = parts.iter().map(|p| p.render(true, true)).collect();
                assert_eq!(text.render(true, true), words.join(" "), "{spec}");
            }
            for (spec, canonical) in [
                ("Sanctus + Pleni + Osanna", "sanctus+pleni+osanna"),
                ("Crucifixus+Resurrexit : 1-2", "crucifixus+resurrexit:1-2"),
                ("gloria:7-7+gloria:8", "gloria:7+gloria:8"),
                ("\u{feff}kyrie+christe ", "kyrie+christe"),
            ] {
                let (text, _) = resolve(spec, &options).unwrap();
                assert_eq!(text.spec(), canonical, "{spec}");
            }
        }
    }

    /// The Sanctus of a Missa brevis in one section: Sanctus, Pleni and Osanna as one short text
    /// of three sentences, whose voices restate only within the sentence they sing.
    #[test]
    fn sanctus_pleni_osanna_is_one_short_text() {
        let options = TextOptions::default();
        let (text, _) = resolve("sanctus+pleni+osanna", &options).unwrap();
        assert_eq!(text.syllables.len(), 34);
        assert_eq!(text.units.len(), 5);
        assert_eq!(text.kind, TextKind::Short);
        assert_eq!(text.sentences().collect::<Vec<_>>(), [0..=2, 3..=3, 4..=4]);
        assert_eq!(text.name(Lang::De), "Sanctus · Pleni · Osanna");
        assert_eq!(text.name(Lang::En), "Sanctus · Pleni · Osanna");
        // The plan sets the three at the melismatic pace of the Sanctus that opens them, 0.6
        // syllable starts per measure (30 per 100 minim beats), not at the pace of other short
        // texts: the 34 syllables, sung once, take round(34 / 0.6) + 2 = 59 measures.
        let dorian: crate::ModeSpec = "dorian".parse().unwrap();
        assert_eq!(super::plan::automatic_length(&text, true), 59);
        let planned = super::plan::plan(&text, dorian, 4, None, None, true).unwrap();
        assert_eq!(
            (planned.measures, planned.rate, planned.statements),
            (59, 30, 1)
        );
        let (alone, _) = resolve("sanctus", &options).unwrap();
        let alone = super::plan::plan(&alone, dorian, 4, None, None, true).unwrap();
        assert_eq!(alone.rate, planned.rate);
        // After Sá-ba-oth. back to a San-ctus or to the tail Dó-mi-nus, after gló-ri-a tu-a. to
        // Ple-ni or to its tail, after in ex-cél-sis. to Ho-sán-na or to its tail: never into
        // another part.
        let sabaoth = usize::from(text.units[2].last);
        assert_eq!(
            text.restart_targets(sabaoth).collect::<Vec<_>>(),
            [0, 2, 4, 6]
        );
        let pleni = usize::from(text.units[3].last);
        assert_eq!(text.restart_targets(pleni).collect::<Vec<_>>(), [14, 22]);
        assert_eq!(text.restart_targets(33).collect::<Vec<_>>(), [27, 30]);
        assert!(text.is_opening(14) && text.is_opening(27));
        assert_eq!(text.syllable_label(27, false), "Ho-");
        assert_eq!(text.word_of(30).first, 30);
        // A long part makes a long text, and the names drop the movement they share.
        let (credo, _) = resolve("crucifixus+resurrexit:1-2", &options).unwrap();
        assert_eq!(credo.kind, TextKind::Long);
        assert_eq!(
            credo.name(Lang::De),
            "Credo: Crucifixus · Et resurrexit 1–2"
        );
        let (kyrie, _) = resolve("kyrie+christe", &options).unwrap();
        assert_eq!(kyrie.name(Lang::En), "Kyrie I · Christe");
        assert_eq!(
            resolve("kyrie+gloria:1-5", &options).unwrap().0.kind,
            TextKind::Long
        );
        // One text gives itself, and the words go on after the last text if they go on there.
        let (sanctus, _) = resolve("sanctus", &options).unwrap();
        assert_eq!(Text::concat(vec![sanctus.clone()]), sanctus);
        let mut continued = sanctus.clone();
        continued.continued = true;
        assert!(Text::concat(vec![sanctus.clone(), continued.clone()]).continued);
        assert!(!Text::concat(vec![continued, sanctus]).continued);
    }

    /// A piece that names no section fails as one, saying that `+` joins sections only and an
    /// own text cannot have it (`Kyrie + pax` is no own text either); a bad range fails by its
    /// piece; an empty piece or one that would be an own text alone makes the spec an own text,
    /// which has no `+`; too many syllables fail.
    #[test]
    fn concat_errors() {
        let options = TextOptions::default();
        assert!(matches!(
            resolve("sanctus+plen", &options),
            Err(TextError::UnknownPiece { id, .. }) if id == "plen"
        ));
        assert!(matches!(
            resolve("Sanctus + amen", &options),
            Err(TextError::UnknownPiece { id, .. }) if id == "amen"
        ));
        let error = resolve("Kyrie + pax", &options).unwrap_err();
        let valid = ordinary::sections(&options)
            .iter()
            .map(|text| text.id.clone())
            .collect::<Vec<_>>()
            .join(", ");
        assert_eq!(
            error.message(Lang::De),
            format!(
                "Unbekannter Abschnitt „pax“; „+“ verbindet nur die Abschnitte des Ordinariums, \
                 und die sind {valid}; ein eigener lateinischer Text kann kein „+“ enthalten."
            )
        );
        assert_eq!(
            error.message(Lang::En),
            format!(
                "Unknown section \"pax\"; \"+\" joins only the sections of the Ordinary, which \
                 are {valid}; an own Latin text cannot have a \"+\"."
            )
        );
        assert_eq!(
            resolve("sanctus+pleni:2", &options).unwrap_err(),
            TextError::BadRange {
                spec: "pleni:2".into(),
                units: 1
            }
        );
        for spec in ["sanctus+", "+kyrie", "kyrie++christe", "Ky-ri-e+Chri-ste"] {
            assert!(
                matches!(
                    resolve(spec, &options),
                    Err(TextError::Character { ch: '+', .. })
                ),
                "{spec}"
            );
        }
        let many = vec!["credo"; MAX_SYLLABLES / 128 + 1].join("+");
        assert_eq!(resolve(&many, &options).unwrap_err(), TextError::TooLong);
    }

    #[test]
    fn parse_errors() {
        let error = |src: &str| Text::own(src, &TextOptions::default()).unwrap_err();
        let character = |word: &str, ch: char| TextError::Character {
            word: word.into(),
            ch,
        };
        assert_eq!(error(""), TextError::Empty);
        assert_eq!(error(" \n "), TextError::Empty);
        assert_eq!(error("/ Ky-ri-e"), TextError::Marker { marker: '/' });
        assert_eq!(error("|Ky-ri-e"), TextError::Marker { marker: '|' });
        assert_eq!(error("Ky-ri-e |"), TextError::Marker { marker: '|' });
        assert_eq!(error("Ky-ri-e/"), TextError::Marker { marker: '/' });
        assert_eq!(error("Ky#rie"), character("Ky#rie", '#'));
        assert_eq!(error("Ky,rie"), character("Ky,rie", ','));
        assert_eq!(error("Ky-ri-è"), character("Ky-ri-è", 'è'));
        assert_eq!(error("pä-ter"), character("pä-ter", 'ä'));
        assert_eq!(error("a\u{308}"), character("a\u{308}", '\u{308}'));
        assert_eq!(error("\u{308}e"), character("\u{308}e", '\u{308}'));
        assert_eq!(error("\u{301}a"), character("\u{301}a", '\u{301}'));
        assert_eq!(
            error("ve-n\u{301}it"),
            character("ve-n\u{301}it", '\u{301}')
        );
        assert_eq!(error(", Ky-ri-e"), character(",", ','));
        assert_eq!(error("Ky-ri-e | ."), character(".", '.'));
        let empty = |word: &str| TextError::EmptySyllable { word: word.into() };
        assert_eq!(error("Ky--ri-e"), empty("Ky--ri-e"));
        assert_eq!(error("-que"), empty("-que"));
        assert_eq!(error("Ky-"), empty("Ky-"));
        assert_eq!(
            error("Chr-i-ste"),
            TextError::NoVowel {
                word: "Chr-i-ste".into(),
                syllable: "Chr".into()
            }
        );
        assert_eq!(
            error("Ký-ri-é"),
            TextError::SeveralAcutes {
                word: "Ký-ri-é".into()
            }
        );
        assert_eq!(
            error("ve-ní\u{301}t"),
            TextError::SeveralAcutes {
                word: "ve-ní\u{301}t".into()
            }
        );
        let long = format!("{}a", "a-".repeat(255));
        assert_eq!(error(&long), TextError::TooLong);

        // Messages in both languages.
        assert_eq!(error("").message(Lang::De), "Der Text ist leer.");
        assert_eq!(error("").message(Lang::En), "The text is empty.");
        assert_eq!(
            error("Ky#rie").message(Lang::De),
            "Unerlaubtes Zeichen „#“ in „Ky#rie“."
        );
        assert_eq!(
            error("Ky#rie").message(Lang::En),
            "Character \"#\" not allowed in \"Ky#rie\"."
        );
        assert_eq!(
            error("\u{301}a").message(Lang::De),
            "Unerlaubtes Zeichen U+0301 in „\u{301}a“."
        );
        assert_eq!(
            error("Ky-ri-e /").message(Lang::De),
            "Die Marke / muss zwischen zwei Wörtern stehen."
        );
        assert_eq!(
            error("Chr-i-ste").message(Lang::En),
            "The syllable \"Chr\" of \"Chr-i-ste\" has no vowel."
        );
    }

    #[test]
    fn invisible_and_foreign_characters() {
        let options = TextOptions::default();
        let (kyrie, _) = Text::own("Ky-ri-e e-lé-i-son", &options).unwrap();
        // A byte order mark, soft hyphens and zero-width characters are dropped.
        for src in [
            "\u{feff}Ky-ri-e e-lé-i-son",
            "Ky\u{ad}ri\u{ad}e e\u{ad}lé\u{ad}i\u{ad}son",
            "Ky-ri-e\u{200b} e-lé\u{200d}-i-son\u{2060}",
            "Ky\u{200c}rie e-lé-i-son",
        ] {
            let (text, _) = Text::own(src, &options).unwrap();
            assert_eq!(text, kyrie, "{src:?}");
        }
        assert_eq!(
            Text::own("\u{feff}\u{ad} \u{200b}", &options).unwrap_err(),
            TextError::Empty
        );

        // Other characters that cannot be seen are named by their code point, characters
        // outside ASCII also show it, so that look-alikes stand out.
        let error = |src: &str| Text::own(src, &options).unwrap_err();
        let mark = error("Ky\u{200e}rie");
        assert_eq!(
            mark,
            TextError::Character {
                word: "Ky\u{200e}rie".into(),
                ch: '\u{200e}'
            }
        );
        assert_eq!(
            mark.message(Lang::De),
            "Unerlaubtes Zeichen U+200E in „Ky\u{200e}rie“."
        );
        assert_eq!(
            mark.message(Lang::En),
            "Character U+200E not allowed in \"Ky\u{200e}rie\"."
        );
        // A Cyrillic е in Kyrie.
        let cyrillic = error("Kyri\u{435}");
        assert_eq!(
            cyrillic.message(Lang::De),
            "Unerlaubtes Zeichen „\u{435}“ (U+0435) in „Kyri\u{435}“."
        );
        assert_eq!(
            cyrillic.message(Lang::En),
            "Character \"\u{435}\" (U+0435) not allowed in \"Kyri\u{435}\"."
        );
        assert_eq!(
            error("Ky\u{301}\u{301}rie").message(Lang::De),
            "Unerlaubtes Zeichen U+0301 in „Ky\u{301}\u{301}rie“."
        );
        assert_eq!(
            error("Ky-ri-è").message(Lang::De),
            "Unerlaubtes Zeichen „è“ (U+00E8) in „Ky-ri-è“."
        );
        for ch in ['\u{7}', '\u{300}', '\u{feff}', '\u{e0041}', '\u{fe0f}'] {
            assert!(invisible(ch), "{ch:?}");
        }
        for ch in ['a', 'é', '\u{435}', '#', '„'] {
            assert!(!invisible(ch), "{ch:?}");
        }
    }
}
