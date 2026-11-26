//! Gregorian chant in GABC, the notation of Gregorio (<https://gregorio-project.github.io/gabc/>):
//! the melody of a chant as a cantus firmus or as the source of subjects, with its text, its
//! phrases and its mode.
//!
//! A GABC file has a header of `key: value;` lines, then `%%` and the body: syllables, each
//! followed by its neumes in parentheses, e.g. `Ky(f)ri(gf)e(h) (,)`. The letters `a`–`m` are the
//! positions on the staff from below, relative to the clef (`c4`: C on the fourth line). Each
//! letter is one note; the shapes of the neumes (virga, quilisma, liquescence, …) do not matter
//! for a cantus firmus. A flat (`ix`) holds to the end of the word or the next bar.
//!
//! What the books add to a chant is left out: the ending of the psalm tone after an antiphon
//! (`<eu>E u o u a e</eu>`) and the alleluia for Paschaltide after „T. P.“ (*tempore paschali*).

use std::ops::Range;

use thiserror::Error;

use crate::interval::Interval;
use crate::mode::{Ambitus, Clausula, Mode, ModeSpec};
use crate::pitch::{Pitch, Step};
use crate::score::{Cadence, Note};
use crate::text::latin;
use crate::time::Duration;

/// A chant as read from GABC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chant {
    pub name: Option<String>,
    /// The mode number from the header (1–8), if any.
    pub mode_number: Option<u8>,
    pub notes: Vec<ChantNote>,
}

/// One note of a chant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChantNote {
    pub pitch: Pitch,
    /// The syllable that begins on this note, ending in `-` where the word goes on.
    pub syllable: Option<String>,
    /// The bar after the note.
    pub bar: Option<Bar>,
}

/// A phrase of a chant ([`Chant::phrases`]): its notes from one bar to the next, with their
/// syllables, the last with the bar that ends it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Phrase {
    /// Where its notes lie in [`Chant::notes`].
    pub range: Range<usize>,
    pub notes: Vec<ChantNote>,
}

impl Phrase {
    /// Its words as its syllables spell them, e.g. `Ky-ri-e e-lé-i-son.`
    pub fn words(&self) -> String {
        let mut words = String::new();
        for syllable in self.notes.iter().filter_map(|n| n.syllable.as_deref()) {
            if !words.is_empty() && !words.ends_with('-') {
                words.push(' ');
            }
            words.push_str(syllable);
        }
        words
    }
}

/// The bars of chant, from the weakest: they divide the melody into phrases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bar {
    /// `` ` ``: a breath.
    Virgula,
    /// `,`: divisio minima.
    Quarter,
    /// `;`: divisio minor.
    Half,
    /// `:`: divisio maior.
    Full,
    /// `::`: divisio finalis.
    Double,
}

/// At which bars internal cadences go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CadenceBars {
    None,
    /// Full bars only.
    Full,
    /// Half and full bars.
    Half,
    /// Every bar from the quarter bar on.
    Quarter,
}

impl CadenceBars {
    fn takes(self, bar: Bar) -> bool {
        match self {
            CadenceBars::None => false,
            CadenceBars::Full => bar >= Bar::Full,
            CadenceBars::Half => bar >= Bar::Half,
            CadenceBars::Quarter => bar >= Bar::Quarter,
        }
    }
}

/// Errors in a GABC text.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum GabcError {
    #[error("the chant has no notes")]
    Empty,
    #[error("a note before the first clef")]
    NoClef,
    #[error("a parenthesis is not closed")]
    Unclosed,
    #[error("the chant ends on {0:?}, where no mode has its final")]
    NoMode(Step),
}

/// The clef: C or F on a line from 1 (lowest) to 4, maybe with a flat on B.
#[derive(Clone, Copy, Debug)]
struct Clef {
    /// Diatonic index of the pitch on position `a`.
    bottom: i32,
    flat: bool,
}

impl Clef {
    fn new(letter: char, line: u32, flat: bool) -> Clef {
        // Lines lie on the positions d, f, h, j (3, 5, 7, 9); the C clef's line is middle C,
        // the F clef's the F below it.
        let anchor = if letter == 'c' {
            Pitch::new(Step::C, 0, 4)
        } else {
            Pitch::new(Step::F, 0, 3)
        };
        Clef {
            bottom: anchor.diatonic() - (2 * line as i32 + 1),
            flat,
        }
    }

    fn pitch(self, position: i32, accidental: Option<i8>) -> Pitch {
        let p = Pitch::from_diatonic(self.bottom + position);
        let alter = accidental.unwrap_or(if self.flat && p.step == Step::B {
            -1
        } else {
            0
        });
        p.with_alter(alter)
    }
}

/// Reads a chant from GABC.
pub fn parse_gabc(text: &str) -> Result<Chant, GabcError> {
    let (header, body) = match text.split_once("\n%%") {
        Some((header, body)) => (header, body.split_once('\n').map_or("", |(_, b)| b)),
        None if text.starts_with("%%") => ("", text[2..].trim_start_matches(['\r', '\n'])),
        None => ("", text),
    };
    let mut name = None;
    let mut mode_number = None;
    for field in header.split(';') {
        let Some((key, value)) = field.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "name" if !value.is_empty() => name = Some(value.to_string()),
            "mode" => {
                let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
                mode_number = digits.parse().ok().filter(|n| (1..=8).contains(n));
            }
            _ => {}
        }
    }
    let notes = parse_body(body)?;
    if notes.is_empty() {
        return Err(GabcError::Empty);
    }
    Ok(Chant {
        name,
        mode_number,
        notes,
    })
}

/// The notes of the body, with syllables and bars. The syllables of the euouae, from the one
/// whose text opens `<eu>` to the one that closes it, and every syllable from „T. P.“ on are
/// read for their clefs and left out with their notes and bars.
fn parse_body(body: &str) -> Result<Vec<ChantNote>, GabcError> {
    let mut notes: Vec<ChantNote> = Vec::new();
    // The notes left out, read only for the clefs.
    let mut left_out: Vec<ChantNote> = Vec::new();
    let mut euouae = false;
    let mut paschal = false;
    let mut clef: Option<Clef> = None;
    // Accidentals by staff position, until the end of the word or a bar.
    let mut accidentals: Vec<(i32, i8)> = Vec::new();
    let mut text = String::new();
    // The note with the last syllable: it gets a hyphen if the next syllable goes on the word.
    let mut last_syllable: Option<usize> = None;
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '%' => {
                // A comment to the end of the line.
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '(' => {
                let mut neumes = String::new();
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == ')' {
                        closed = true;
                        break;
                    }
                    neumes.push(c);
                }
                if !closed {
                    return Err(GabcError::Unclosed);
                }
                // White space before the syllable begins a new word.
                let new_word = text.starts_with(char::is_whitespace) || last_syllable.is_none();
                if new_word {
                    accidentals.clear();
                }
                euouae |= text.contains("<eu>");
                paschal |= paschaltide(&text);
                if euouae || paschal {
                    euouae &= !text.contains("</eu>");
                    text.clear();
                    read_neumes(
                        &neumes,
                        &mut clef,
                        &mut accidentals,
                        &mut None,
                        &mut left_out,
                    )?;
                    continue;
                }
                let mut syllable = clean_syllable(&text).map(|s| normal_case(&s, new_word));
                text.clear();
                if let (Some(_), false, Some(i)) = (&syllable, new_word, last_syllable)
                    && let Some(previous) = &mut notes[i].syllable
                {
                    previous.push('-');
                }
                let first = notes.len();
                let has_syllable = syllable.is_some();
                read_neumes(
                    &neumes,
                    &mut clef,
                    &mut accidentals,
                    &mut syllable,
                    &mut notes,
                )?;
                if has_syllable && notes.len() > first {
                    last_syllable = Some(first);
                }
            }
            _ => text.push(c),
        }
    }
    Ok(notes)
}

/// Reads the neumes of one syllable; the syllable goes to its first note.
fn read_neumes(
    neumes: &str,
    clef: &mut Option<Clef>,
    accidentals: &mut Vec<(i32, i8)>,
    syllable: &mut Option<String>,
    notes: &mut Vec<ChantNote>,
) -> Result<(), GabcError> {
    let chars: Vec<char> = neumes.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            '[' => {
                // Attributes such as [ll:1] or [alt:…]: their colons are no bars.
                while i < chars.len() && chars[i] != ']' {
                    i += 1;
                }
            }
            'c' | 'f' if next.is_some_and(|n| n.is_ascii_digit() || n == 'b') => {
                let flat = next == Some('b');
                let digit = chars.get(i + 1 + usize::from(flat)).copied();
                if let Some(line) = digit.and_then(|d| d.to_digit(10)) {
                    *clef = Some(Clef::new(c, line.clamp(1, 4), flat));
                    accidentals.clear();
                    i += 1 + usize::from(flat);
                } else {
                    i += read_note(c, next, *clef, accidentals, syllable, notes)?;
                }
            }
            'a'..='m' | 'A'..='M' => {
                i += read_note(c, next, *clef, accidentals, syllable, notes)?;
            }
            ',' | ';' | ':' | '`' => {
                let bar = match c {
                    ':' if next == Some(':') => {
                        i += 1;
                        Bar::Double
                    }
                    ':' => Bar::Full,
                    ';' => Bar::Half,
                    ',' => Bar::Quarter,
                    _ => Bar::Virgula,
                };
                if let Some(last) = notes.last_mut() {
                    last.bar = Some(last.bar.map_or(bar, |b| b.max(bar)));
                }
                accidentals.clear();
            }
            _ => {} // shapes, rhythmic signs, spaces, custodes
        }
        i += 1;
    }
    Ok(())
}

/// Reads a note or an accidental at the letter `c`; returns how many further characters it used.
fn read_note(
    c: char,
    next: Option<char>,
    clef: Option<Clef>,
    accidentals: &mut Vec<(i32, i8)>,
    syllable: &mut Option<String>,
    notes: &mut Vec<ChantNote>,
) -> Result<usize, GabcError> {
    let position = c.to_ascii_lowercase() as i32 - 'a' as i32;
    let accidental = match next {
        Some('x') => Some(-1),
        Some('y') => Some(0),
        Some('#') => Some(1),
        _ => None,
    };
    if let Some(alter) = accidental {
        accidentals.retain(|(p, _)| *p != position);
        accidentals.push((position, alter));
        return Ok(1);
    }
    if next == Some('+') {
        return Ok(1); // a custos shows the next note; it is not sung
    }
    let clef = clef.ok_or(GabcError::NoClef)?;
    let alter = accidentals
        .iter()
        .find(|(p, _)| *p == position)
        .map(|(_, a)| *a);
    notes.push(ChantNote {
        pitch: clef.pitch(position, alter),
        syllable: syllable.take(),
        bar: None,
    });
    Ok(0)
}

/// The text of a syllable without markup: tags, braces and the asterisks of the intonation.
fn clean_syllable(raw: &str) -> Option<String> {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let tag_end = rest[start..]
            .find('>')
            .map_or(rest.len(), |e| start + e + 1);
        let tag = &rest[start..tag_end];
        rest = &rest[tag_end..];
        // Verbatim code, special signs and text above the staff are left out with their content.
        for name in ["v", "sp", "alt", "nlba"] {
            if tag == format!("<{name}>") {
                let close = format!("</{name}>");
                rest = rest.find(&close).map_or("", |e| &rest[e + close.len()..]);
            }
        }
    }
    out.push_str(rest);
    // Without spaces: a lyric syllable is one word.
    let cleaned: String = out
        .chars()
        .filter(|c| !matches!(c, '{' | '}' | '*' | '†' | '+') && !c.is_whitespace())
        .collect();
    let cleaned = cleaned.trim();
    (!cleaned.is_empty()).then(|| cleaned.to_string())
}

/// Whether the text before a syllable's notes marks the alleluia for Paschaltide: „T. P.“, with
/// or without spaces and markup (`<i>T. P.</i>`, `<i>Extra T.P.</i>`).
fn paschaltide(raw: &str) -> bool {
    let mut plain = String::with_capacity(raw.len());
    let mut in_tag = false;
    for c in raw.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag && !c.is_whitespace() => plain.push(c),
            _ => {}
        }
    }
    plain.contains("T.P.")
}

/// A syllable in capitals, as chant books print the first word, in normal case: capitalized at the
/// beginning of a word. The capitals are all its letters (`KY-ri-e`) or its first two
/// (`SAn-ctus`).
fn normal_case(syllable: &str, word_start: bool) -> String {
    let letters: Vec<char> = syllable.chars().filter(|c| c.is_alphabetic()).collect();
    let capitals = letters.iter().take_while(|c| c.is_uppercase()).count();
    if capitals < letters.len().min(2) || letters.len() < 2 && word_start {
        return syllable.to_string();
    }
    let lower = syllable.to_lowercase();
    if !word_start {
        return lower;
    }
    let mut chars = lower.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// The mode that a header's number names with its final on `last`: at pitch or a fourth higher
/// with one flat; modes 5 and 6 with mostly B♭ are the transposed Ionian.
fn numbered(number: u8, last: Step, mostly_flat: bool) -> Option<ModeSpec> {
    let modes = [Mode::Dorian, Mode::Phrygian, Mode::Lydian, Mode::Mixolydian];
    let mode = *modes.get(usize::from(number.checked_sub(1)? / 2))?;
    if mode == Mode::Lydian && mostly_flat && last == Step::F {
        return Some(ModeSpec::new(Mode::Ionian).with_transposition(true));
    }
    [false, true]
        .into_iter()
        .map(|t| ModeSpec::new(mode).with_transposition(t))
        .find(|spec| spec.final_step() == last)
}

/// How many of the notes on B are flat and how many natural.
fn b_flats(notes: &[ChantNote]) -> (usize, usize) {
    notes
        .iter()
        .filter(|n| n.pitch.step == Step::B)
        .fold((0, 0), |(f, n), note| match note.pitch.alter {
            -1 => (f + 1, n),
            _ => (f, n + 1),
        })
}

impl Chant {
    /// The mode in which the chant serves as a cantus firmus, whose last note is the final. The
    /// header's number (1–8) names the mode and whether it is plagal (even numbers); the last
    /// note shows whether it is transposed by a fourth, with one flat. Modes 5 and 6 with mostly
    /// B♭ are the transposed Ionian, as in chant books. Without a number, or where the last note
    /// is no final of the numbered mode, the last note gives the mode, transposed when B♭
    /// prevails; without a number the range shows whether it is plagal.
    pub fn mode(&self) -> Result<ModeSpec, GabcError> {
        let notes = self.melody();
        let last = notes.last().ok_or(GabcError::Empty)?.pitch;
        let (flat, natural) = b_flats(&notes);
        let mostly_flat = flat > natural;
        let by_number = self
            .mode_number
            .and_then(|number| numbered(number, last.step, mostly_flat));
        let spec = match by_number {
            Some(spec) => spec,
            None => {
                let transposed = mostly_flat && flat >= 2;
                let untransposed = if transposed {
                    Step::from_index(last.step.index() - 3)
                } else {
                    last.step
                };
                let mode = Mode::from_final(untransposed).ok_or(GabcError::NoMode(last.step))?;
                ModeSpec::new(mode).with_transposition(transposed)
            }
        };
        let ambitus = match self.mode_number {
            Some(n) if n % 2 == 0 => Ambitus::Plagal,
            Some(_) => Ambitus::Authentic,
            None => {
                let lowest = notes.iter().map(|n| n.pitch.diatonic()).min();
                if lowest.is_some_and(|low| low <= last.diatonic() - 3) {
                    Ambitus::Plagal
                } else {
                    Ambitus::Authentic
                }
            }
        };
        Ok(spec.with_ambitus(ambitus))
    }

    /// The mode that the header's number names, for subjects from the chant's phrases. A chant
    /// may end off the final of its mode: Kyrie IV, in mode 1, ends its last invocations on A,
    /// the fifth, and [`Chant::mode`] makes it Aeolian, since a cantus firmus ends on the final.
    /// Here the final is the last note of the last phrase that ends on a final of the number, at
    /// pitch or a fourth higher: D for Kyrie IV, where its Christe ends. Without a number, or
    /// where no phrase ends on such a final, the mode of [`Chant::mode`].
    pub fn numbered_mode(&self) -> Result<ModeSpec, GabcError> {
        let Some(number) = self.mode_number else {
            return self.mode();
        };
        let (flat, natural) = b_flats(&self.melody());
        let ambitus = if number % 2 == 0 {
            Ambitus::Plagal
        } else {
            Ambitus::Authentic
        };
        self.phrases()
            .iter()
            .rev()
            .filter_map(|phrase| phrase.notes.last())
            .find_map(|last| numbered(number, last.pitch.step, flat > natural))
            .map_or_else(|| self.mode(), |spec| Ok(spec.with_ambitus(ambitus)))
    }

    /// The melody of the cantus: the notes, with a final note repeated at the end (on several
    /// syllables, as in "por-ta") sung once; the syllables are joined.
    pub fn melody(&self) -> Vec<ChantNote> {
        let mut notes = self.notes.clone();
        while notes.len() >= 2 && notes[notes.len() - 1].pitch == notes[notes.len() - 2].pitch {
            let last = notes.pop().expect("two notes");
            let before = notes.last_mut().expect("one note");
            before.syllable = match (before.syllable.take(), last.syllable) {
                (Some(a), Some(b)) => Some(a.trim_end_matches('-').to_string() + &b),
                (a, b) => a.or(b),
            };
            before.bar = before.bar.max(last.bar);
        }
        notes
    }

    /// The phrases, divided at the bars from the quarter bar on; a virgula is a breath within a
    /// phrase. In a Kyrie each invocation is a phrase, up to its double bar: the bars within an
    /// invocation only divide its melisma, at the marks where the choir comes in (`*`, `**`).
    pub fn phrases(&self) -> Vec<Phrase> {
        let end = if self.is_kyrie() {
            Bar::Double
        } else {
            Bar::Quarter
        };
        let mut phrases = Vec::new();
        let mut start = 0;
        for (i, note) in self.notes.iter().enumerate() {
            if note.bar.is_some_and(|bar| bar >= end) || i + 1 == self.notes.len() {
                phrases.push(Phrase {
                    range: start..i + 1,
                    notes: self.notes[start..=i].to_vec(),
                });
                start = i + 1;
            }
        }
        phrases
    }

    /// Whether the chant is a Kyrie: its first word is Kyrie, in any spelling (`KY-ri-e`,
    /// `Ký-ri-e`).
    pub fn is_kyrie(&self) -> bool {
        let mut word = String::new();
        for syllable in self.notes.iter().filter_map(|n| n.syllable.as_deref()) {
            word.push_str(syllable);
            if !syllable.ends_with('-') {
                break;
            }
        }
        let letters: String = latin::normalize(&word)
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect();
        letters == "kyrie"
    }

    /// The melody as a cantus firmus in whole notes with its syllables.
    pub fn cantus_notes(&self) -> Vec<Note> {
        self.melody()
            .into_iter()
            .map(|n| Note {
                syllable: n.syllable,
                ..Note::new(n.pitch, Duration::WHOLE)
            })
            .collect()
    }

    /// Internal cadences at the bars of the chosen kinds, where a clausula can arrive on the
    /// phrase's last note (on the first of repeated ones): the cantus reaches it by step, which
    /// gives it the tenor's or the discant's part, or from its fifth like a bass. A cadence keeps
    /// two measures from the beginning, three from the previous one and from the end.
    pub fn cadences(&self, bars: CadenceBars, mode: ModeSpec) -> Vec<Cadence> {
        let notes = self.melody();
        let n = notes.len();
        let mut cadences: Vec<Cadence> = Vec::new();
        for (i, note) in notes.iter().enumerate() {
            if !note.bar.is_some_and(|b| bars.takes(b)) || i + 1 == n {
                continue;
            }
            let mut goal = i;
            while goal > 0 && notes[goal - 1].pitch == note.pitch {
                goal -= 1;
            }
            if goal < 2 || goal + 3 > n {
                continue;
            }
            if cadences.last().is_some_and(|c| goal < c.measure + 3) {
                continue;
            }
            let target = note.pitch;
            let Some(clausula) = mode.clausula(target.step) else {
                continue;
            };
            if !mode.is_scale_pitch(target) {
                continue;
            }
            let step = Interval::between(notes[goal - 1].pitch, target);
            let from_fifth = clausula == Clausula::Leading
                && matches!(step.steps, 3 | -4)
                && step.semitones.abs() % 12 != 6;
            if step.is_step() || from_fifth {
                cadences.push(Cadence {
                    measure: goal,
                    target: target.step,
                });
            }
        }
        cadences
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/chant/ave-maris-stella.gabc"
    ));
    const VENI: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/chant/veni-creator.gabc"
    ));
    const PANGE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/chant/pange-lingua.gabc"
    ));
    const ISTE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/chant/iste-confessor.gabc"
    ));

    fn names(notes: &[ChantNote]) -> String {
        notes
            .iter()
            .map(|n| n.pitch.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn ave_maris_stella() {
        let chant = parse_gabc(AVE).unwrap();
        assert_eq!(chant.name.as_deref(), Some("Ave Maris Stella"));
        assert_eq!(chant.mode_number, Some(1));
        assert_eq!(chant.notes.len(), 27);
        // The flat on "ve" makes B♭, and only there.
        assert_eq!(names(&chant.notes[..7]), "D3 A3 Bb3 A3 G3 A3 G3");
        assert_eq!(chant.notes[0].syllable.as_deref(), Some("A-"));
        assert_eq!(chant.notes[1].syllable.as_deref(), Some("ve"));
        assert_eq!(chant.notes[2].syllable, None);
        assert_eq!(chant.notes[6].bar, Some(Bar::Quarter));
        assert_eq!(chant.notes[12].bar, Some(Bar::Half));
        // "cae-li por-ta": the repeated final is sung once, on "porta".
        let melody = chant.melody();
        assert_eq!(melody.len(), 25);
        assert_eq!(names(&melody[22..]), "D3 C3 D3");
        assert_eq!(melody[24].syllable.as_deref(), Some("porta."));
        assert_eq!(melody[24].bar, Some(Bar::Double));
        let mode = chant.mode().unwrap();
        assert_eq!(mode, ModeSpec::new(Mode::Dorian));
        // The half bar after "alma" ends on A, reached from G: a cadence on A.
        assert_eq!(
            chant.cadences(CadenceBars::Half, mode),
            [Cadence {
                measure: 12,
                target: Step::A
            }]
        );
        // With the quarter bars also on G after "stella" and on D after "Virgo".
        let all: Vec<(usize, Step)> = chant
            .cadences(CadenceBars::Quarter, mode)
            .iter()
            .map(|c| (c.measure, c.target))
            .collect();
        assert_eq!(all, [(6, Step::G), (12, Step::A), (19, Step::D)]);
    }

    #[test]
    fn veni_creator_and_pange_lingua() {
        let veni = parse_gabc(VENI).unwrap();
        assert_eq!(veni.notes.len(), 46);
        assert_eq!(veni.melody().len(), 45);
        let mode = veni.mode().unwrap();
        assert_eq!(
            mode,
            ModeSpec::new(Mode::Mixolydian).with_ambitus(Ambitus::Plagal)
        );
        // Line 1 ends D–C–C (cadence on the first C), line 2 E–D–D; line 3 leaps into C.
        let cadences: Vec<(usize, Step)> = veni
            .cadences(CadenceBars::Half, mode)
            .iter()
            .map(|c| (c.measure, c.target))
            .collect();
        assert_eq!(cadences, [(9, Step::C), (19, Step::D)]);
        let pange = parse_gabc(PANGE).unwrap();
        assert_eq!(pange.notes.len(), 53);
        let mode = pange.mode().unwrap();
        assert_eq!(mode, ModeSpec::new(Mode::Phrygian));
        // Full bars: on G (from A) and on D (from A, a fifth above).
        let cadences: Vec<(usize, Step)> = pange
            .cadences(CadenceBars::Full, mode)
            .iter()
            .map(|c| (c.measure, c.target))
            .collect();
        assert_eq!(cadences, [(19, Step::G), (35, Step::D)]);
    }

    #[test]
    fn clefs_accidentals_and_errors() {
        // C clef on the third line: h is C4. A flat holds to the end of the word.
        let chant = parse_gabc("%%\n(c3) A(hgx)ve(g) ma(g)").unwrap();
        assert_eq!(names(&chant.notes), "C4 Bb3 B3");
        // The F clef on the third line: h is F3; clefs may change.
        let chant = parse_gabc("%%\n(f3) Fa(h) (c3) do(h)").unwrap();
        assert_eq!(names(&chant.notes), "F3 C4");
        // A clef with a flat; a custos is not sung; a comment is skipped.
        let chant = parse_gabc("%%\n(cb3) Al(g g+)le % comment (h)\n(h)").unwrap();
        assert_eq!(names(&chant.notes), "Bb3 C4");
        assert_eq!(parse_gabc("%%\n(c4) A(d"), Err(GabcError::Unclosed));
        assert_eq!(parse_gabc("%%\nA(d)"), Err(GabcError::NoClef));
        assert_eq!(parse_gabc("name: x;\n%%\n(c4) (::)"), Err(GabcError::Empty));
        let on_b = parse_gabc("%%\n(c4) A(i)").unwrap();
        assert_eq!(on_b.mode(), Err(GabcError::NoMode(Step::B)));
        // Mode 6 with B♭ throughout is the transposed Ionian; without a number the flats decide.
        let sixth = parse_gabc("mode:6;\n%%\n(cb3) A(e)ve(g)ma(f)ri(e)a(d)").unwrap();
        assert_eq!(
            sixth.mode(),
            Ok(ModeSpec::new(Mode::Ionian)
                .with_transposition(true)
                .with_ambitus(Ambitus::Plagal))
        );
        let flats = parse_gabc("%%\n(cb3) Do(d)mi(g)nus(f)de(g)us(d)").unwrap();
        assert_eq!(
            flats.mode(),
            Ok(ModeSpec::new(Mode::Ionian).with_transposition(true))
        );
    }

    fn words(chant: &Chant) -> Vec<String> {
        chant.phrases().iter().map(Phrase::words).collect()
    }

    #[test]
    fn eu_and_tp_are_dropped() {
        // GregoBase 1820, shortened: the antiphon Dum esset summus pontifex (mode 1) ends on D.
        // The alleluia for Paschaltide and the euouae of the psalm tone after it, which would end
        // it on F, are no part of its melody.
        let dum = parse_gabc(concat!(
            "name:Dum esset summus pontifex;\nmode:1;\n%%\n",
            "(c4) DUM(d) es(d)set(dc) sum(f)mus(g') Pón(f)ti(gh)fex,(h.) *(,) ",
            "glo(f)ri(f)ó(e_[uh:l]f)sus(g) mi(fe)grá(d.)vit.(d.) (::) ",
            "<i>T. P.</i> Al(e)le(fe)lú(d.){ia}.(d.) (::) ",
            "<eu>E(h) u(h) o(g) u(f) a(gh) e.</eu>(gf..) (::)\n",
        ))
        .unwrap();
        assert_eq!(dum.notes.len(), 19);
        assert_eq!(names(&dum.notes[15..]), "F3 E3 D3 D3");
        let last = dum.notes.last().unwrap();
        assert_eq!(last.syllable.as_deref(), Some("vit."));
        assert_eq!(last.bar, Some(Bar::Double));
        assert_eq!(
            words(&dum),
            ["Dum es-set sum-mus Pón-ti-fex,", "glo-ri-ó-sus mi-grá-vit."]
        );
        assert_eq!(dum.mode(), Ok(ModeSpec::new(Mode::Dorian)));
        // The euouae may close after its notes, and the chant may go on after it; a clef it sets
        // holds.
        let psalm = parse_gabc(concat!(
            "%%\n(c4) Al(d)le(f)lú(g)ia.(g) (::) ",
            "<eu>E(j) u(j) o(i) (c3) u(j) a(h) e.(g.) </eu>(::) Lau(h)da(g)te.(h) (::)",
        ))
        .unwrap();
        assert_eq!(names(&psalm.notes), "D3 F3 G3 G3 C4 B3 C4");
        assert_eq!(words(&psalm), ["Al-le-lú-ia.", "Lau-da-te."]);
        // „T. P.“ in any spacing and markup ends the melody, also „Extra T. P.“, which gives the
        // ending outside Paschaltide after the paschal one.
        for marked in [
            "<i>T.P.</i>(::) Al(e)le(f)lú(gh)ia.(g) (::)",
            "(::) <i>Extra T. P.</i> †() no(ghF'Ef)stri.(f.) (::)",
            "(::) <c>T.P. +</c> Al(h)le(jkh)lú(h)ia.(h) (::) <eu>E(h) u(h) o(g) u(f) a(gh) e.(g.)",
        ] {
            let chant = parse_gabc(&format!("%%\n(c4) Al(f)ma.(e) {marked}")).unwrap();
            assert_eq!(names(&chant.notes), "F3 E3", "{marked}");
        }
        assert!(paschaltide(" T. P.  Al"));
        assert!(!paschaltide(" <i>Ps.</i> Con"));
        assert!(!paschaltide("<sp>V/</sp>. Tu"));
    }

    #[test]
    fn phrases_split_at_bars() {
        // Iste confessor (mode 8): its half lines, divided by quarter, half and full bars.
        let iste = parse_gabc(ISTE).unwrap();
        assert!(!iste.is_kyrie());
        assert_eq!(
            words(&iste),
            [
                "I-ste Con-fés-sor",
                "Dó-mi-ni, co-lén-tes",
                "Quem pi-e lau-dant",
                "pó-pu-li per or-bem,",
                "Hac di-e lae-tus mé-ru-it su-pré-mos",
                "Lau-dis ho-nó-res.",
            ]
        );
        let phrases = iste.phrases();
        let ranges: Vec<Range<usize>> = phrases.iter().map(|p| p.range.clone()).collect();
        assert_eq!(ranges, [0..7, 7..14, 14..20, 20..26, 26..40, 40..48]);
        let bars: Vec<Option<Bar>> = phrases
            .iter()
            .map(|p| p.notes[p.notes.len() - 1].bar)
            .collect();
        assert_eq!(
            bars,
            [
                Bar::Quarter,
                Bar::Half,
                Bar::Quarter,
                Bar::Full,
                Bar::Quarter,
                Bar::Double
            ]
            .map(Some)
        );
        // Each phrase keeps its notes with their syllables.
        assert!(
            phrases
                .iter()
                .all(|p| p.notes == iste.notes[p.range.clone()])
        );
        assert_eq!(names(&phrases[0].notes), "G3 A3 G3 E3 F3 G3 G3");
        assert_eq!(phrases[1].notes[0].syllable.as_deref(), Some("Dó-"));
        assert_eq!(
            iste.mode(),
            Ok(ModeSpec::new(Mode::Mixolydian).with_ambitus(Ambitus::Plagal))
        );
        // A virgula is a breath within a phrase; the last phrase ends with the chant.
        let ave = parse_gabc(
            "%%\n(c4) A(d)ve(e) (`) ma(f)ri(g)a,(f) (,) gra(g)ti(h)a(g) (;) ple(f)na(e)",
        )
        .unwrap();
        assert_eq!(words(&ave), ["A-ve ma-ri-a,", "gra-ti-a", "ple-na"]);
        assert_eq!(ave.phrases()[0].notes[1].bar, Some(Bar::Virgula));
        assert_eq!(ave.phrases()[2].notes[1].bar, None);
        // In a Kyrie each invocation is a phrase, up to its double bar; the bars within it only
        // divide its melisma. To other words the same melody is cut at every bar.
        let kyrie = concat!(
            "%%\n(c4) KY(f)ri(gf)e(h) *(,) (g) e(f)lé(g)i(f)son.(e) (::) ",
            "Chri(h)ste(g) (;) e(f)lé(g)i(f)son.(e) (::) Ký(f)ri(g)e(f) e(e)lé(d)i(e)son.(d) (::)",
        );
        let chant = parse_gabc(kyrie).unwrap();
        assert!(chant.is_kyrie());
        assert_eq!(
            words(&chant),
            [
                "Ky-ri-e e-lé-i-son.",
                "Chri-ste e-lé-i-son.",
                "Ký-ri-e e-lé-i-son."
            ]
        );
        assert_eq!(chant.phrases()[0].range, 0..9);
        let other = parse_gabc(&kyrie.replace("KY(f)ri(gf)e(h)", "SAn(f)ctus(gf)que(h)")).unwrap();
        assert!(!other.is_kyrie());
        assert_eq!(
            words(&other),
            [
                "San-ctus-que",
                "e-lé-i-son.",
                "Chri-ste",
                "e-lé-i-son.",
                "Ký-ri-e e-lé-i-son."
            ]
        );
    }

    #[test]
    fn capitals_of_the_books_in_normal_case() {
        // The books print the first word in capitals, all its letters or the first two.
        let chant = parse_gabc("%%\n(c3) SAn(f)ctus(g) GLó(h)ri(g)a(h) KY(f)ri(g)e(f) A(h)GNUS(g)")
            .unwrap();
        let syllables: Vec<&str> = chant
            .notes
            .iter()
            .filter_map(|n| n.syllable.as_deref())
            .collect();
        assert_eq!(
            syllables,
            [
                "San-", "ctus", "Gló-", "ri-", "a", "Ky-", "ri-", "e", "A-", "gnus"
            ]
        );
    }

    #[test]
    fn numbers_name_the_mode_of_the_phrases() {
        // Mode 1, its last phrase ending on A, the fifth, as in Kyrie IV: a cantus firmus in the
        // Aeolian mode; by its number Dorian, the final where the phrase before it ends.
        let chant = parse_gabc(
            "mode:1;\n%%\n(c4) Glo(d)ri(f)a(e) De(d)o.(d) (::) Al(h)le(g)lu(h)ia.(h) (::)",
        )
        .unwrap();
        assert_eq!(chant.mode(), Ok(ModeSpec::new(Mode::Aeolian)));
        assert_eq!(chant.numbered_mode(), Ok(ModeSpec::new(Mode::Dorian)));
        // Mode 8 likewise, plagal by its number.
        let eighth =
            parse_gabc("mode:8;\n%%\n(c4) Al(g)le(h)lu(g)ia.(g) (;) Al(h)le(i)lu(j)ia.(h) (::)")
                .unwrap();
        let plagal = |mode| ModeSpec::new(mode).with_ambitus(Ambitus::Plagal);
        assert_eq!(eighth.mode(), Ok(plagal(Mode::Aeolian)));
        assert_eq!(eighth.numbered_mode(), Ok(plagal(Mode::Mixolydian)));
        // Without a number, or where no phrase ends on a final of the number, as a cantus firmus.
        for text in [
            "%%\n(c4) Glo(d)ri(f)a(e) De(d)o.(d) (::) Al(h)le(g)lu(h)ia.(h) (::)",
            "mode:1;\n%%\n(c4) Al(h)le(g)lu(h)ia.(h) (::)",
        ] {
            let chant = parse_gabc(text).unwrap();
            assert_eq!(chant.numbered_mode(), chant.mode(), "{text}");
        }
        for text in [AVE, VENI, PANGE, ISTE] {
            let chant = parse_gabc(text).unwrap();
            assert_eq!(chant.numbered_mode(), chant.mode());
        }
    }
}
