//! The measure rhythms of the species: which patterns a voice may use where in the piece. The
//! rhythm rules (G2-RHYTHM to G5-RHYTHM) judge them, and the generator follows them.
//!
//! A pattern lists the note values of one measure: `1` whole, `2` half, `4` quarter, `8` eighth,
//! a dot for dotted values, `r` for a rest, `⁀` before a value for a note tied from the previous
//! measure and after a value for a note tied into the next one, e.g. `r2 2⁀` or `⁀4 8 8 2⁀`.

use crate::score::{Note, Species};
use crate::time::TICKS_PER_WHOLE;

/// Where a measure lies in a voice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Before the first note: a whole measure of rest.
    Silent,
    /// The measure with the first note.
    Entry,
    Inner,
    Antepenultimate,
    Penultimate,
    Last,
}

impl Place {
    /// The place of measure `m` (from 0) of `measures`, in a voice whose first note lies in
    /// measure `entry`. Before an internal cadence arriving in one of the measures `goals`, the
    /// measures are penultimate and antepenultimate as before the end.
    pub fn of(m: usize, measures: usize, entry: Option<usize>, goals: &[usize]) -> Place {
        match entry {
            _ if m + 1 == measures => Place::Last,
            Some(e) if m < e => Place::Silent,
            Some(e) if m == e => Place::Entry,
            _ if m + 2 == measures || goals.contains(&(m + 1)) => Place::Penultimate,
            _ if m + 3 == measures || goals.contains(&(m + 2)) => Place::Antepenultimate,
            _ => Place::Inner,
        }
    }
}

/// A rhythm allowed in a measure, with the parameter of the species' rhythm rule that sets its
/// level (`None`: free). Any other rhythm breaks the rule's parameter `level`.
pub type Allowed = (&'static str, Option<&'static str>);

/// The rule that judges the rhythm of a species; the first species has none.
pub fn rule_id(species: Species) -> Option<&'static str> {
    match species {
        Species::First => None,
        Species::Second => Some("G2-RHYTHM"),
        Species::Third => Some("G3-RHYTHM"),
        Species::Fourth => Some("G4-RHYTHM"),
        Species::Fifth => Some("G5-RHYTHM"),
        Species::Free => Some("FREE-RHYTHM"),
    }
}

/// Inner measures of the fifth species in two voices, by how often Fux uses them (docs/regeln.md
/// § 11): at least 10 times free, 3 to 9 times `occasional`, once or twice `rare`.
const FIFTH_INNER: &[Allowed] = &[
    ("4 4 2⁀", None),
    ("⁀4 4 2⁀", None),
    ("⁀4 4 4 4", None),
    ("⁀2 4 4", Some("occasional")),
    ("2 2⁀", Some("occasional")),
    ("4 4 4 4", Some("occasional")),
    ("⁀4 8 8 2⁀", Some("occasional")),
    ("⁀4 8 8 4 4", Some("occasional")),
    ("2 4 4", Some("occasional")),
    ("⁀2 2⁀", Some("occasional")),
    ("⁀2 2", Some("occasional")),
    ("2 2", Some("rare")),
    ("⁀4 4 4 8 8", Some("rare")),
    ("4 8 8 4 4", Some("rare")),
    ("4 4 2", Some("rare")),
];

/// The same with the rhythms Fux uses only with three or more voices.
const FIFTH_INNER_MANY: &[Allowed] = &[
    ("4 4 2⁀", None),
    ("⁀4 4 2⁀", None),
    ("⁀4 4 4 4", None),
    ("⁀2 4 4", Some("occasional")),
    ("2 2⁀", Some("occasional")),
    ("4 4 4 4", Some("occasional")),
    ("⁀4 8 8 2⁀", Some("occasional")),
    ("⁀4 8 8 4 4", Some("occasional")),
    ("2 4 4", Some("occasional")),
    ("⁀2 2⁀", Some("occasional")),
    ("⁀2 2", Some("occasional")),
    ("2 2", Some("rare")),
    ("⁀4 4 4 8 8", Some("rare")),
    ("4 8 8 4 4", Some("rare")),
    ("4 4 2", Some("rare")),
    ("2. 4", Some("rare")),
    ("⁀4 4 2", Some("rare")),
];

/// The measure rhythms of free counterpoint, by how often Palestrina uses them in his masses
/// (docs/regeln.md § 16; 519,764 measures of 1,156 movements in 4/2, three to six voices): from
/// 5 % on free, from 1 % `frequent`, from 0.1 % `uncommon`, from 0.01 % `occasional`, from
/// 0.001 % `rare`, rarer `very_rare`. A measure of rest is free anywhere.
const FREE: &[Allowed] = &[
    ("1", None),
    ("r1", None),
    ("2 2", None),
    ("⁀2 2", None),
    ("1⁀", None),
    ("2 2⁀", None),
    ("⁀1", None),
    ("2. 4", Some("frequent")),
    ("r2 2", Some("frequent")),
    ("r2 2⁀", Some("frequent")),
    ("⁀1⁀", Some("frequent")),
    ("4 4 4 4", Some("frequent")),
    ("⁀2 4 4", Some("frequent")),
    ("4 4 2⁀", Some("frequent")),
    ("⁀4 4 2⁀", Some("frequent")),
    ("⁀4 4 2", Some("frequent")),
    ("2 4 4", Some("uncommon")),
    ("⁀2 2⁀", Some("uncommon")),
    ("⁀4 4 4 4", Some("uncommon")),
    ("4 4 2", Some("uncommon")),
    ("2. 8 8", Some("uncommon")),
    ("⁀4 8 8 2", Some("uncommon")),
    ("⁀4 4 4 8 8", Some("occasional")),
    ("4 4 4 8 8", Some("occasional")),
    ("r2 4 4", Some("occasional")),
    ("4 8 8 2", Some("occasional")),
    ("4 8 8 2⁀", Some("occasional")),
    ("4 2 4", Some("rare")),
    ("2 4 8 8", Some("rare")),
    ("8 8 8 8 2⁀", Some("rare")),
    ("⁀4 8 8 2⁀", Some("rare")),
    ("⁀2 r2", Some("rare")),
    ("2 r4 4", Some("rare")),
    ("⁀4 8 8 4 4", Some("rare")),
    ("2 r2", Some("rare")),
    ("4 8 8 4 4", Some("rare")),
    ("2 4 4⁀", Some("rare")),
    ("4. 8 2", Some("rare")),
    ("2 8 8 8 8", Some("very_rare")),
    ("4. 8 2⁀", Some("very_rare")),
    ("⁀4 2 4", Some("very_rare")),
    ("4 2 8 8", Some("very_rare")),
    ("4 4 4 4⁀", Some("very_rare")),
    ("4 4 8 8 8 8", Some("very_rare")),
    ("r2 4 8 8", Some("very_rare")),
    ("r4 2 4", Some("very_rare")),
    ("⁀2 4 8 8", Some("very_rare")),
    ("⁀4 4 8 8 8 8", Some("very_rare")),
    ("⁀4 8 8 4 8 8", Some("very_rare")),
];

/// The last measure of a free voice in Palestrina (5,377 voices): the final held or struck, or a
/// voice that has ended before; rarely it still moves.
const FREE_LAST: &[Allowed] = &[
    ("⁀1", None),
    ("1", None),
    ("r1", None),
    ("2 2", Some("occasional")),
    ("⁀2 2", Some("occasional")),
    ("⁀4 8 8 2", Some("rare")),
];

/// The rhythms a species allows at a place; `many` for three or more voices.
pub fn patterns(species: Species, place: Place, many: bool) -> &'static [Allowed] {
    use Place::*;
    match (species, place) {
        (Species::Free, Last) => FREE_LAST,
        (Species::Free, Silent) => &[("r1", None)],
        (_, Last) => &[("1", None)],
        (_, Silent) => &[("r1", Some("late_entry"))],
        (Species::First, _) => &[("1", None)],
        (Species::Second, Entry) => &[("r2 2", None), ("2 2", Some("no_rest"))],
        (Species::Second, Antepenultimate) if many => {
            &[("2 2", None), ("2 2⁀", Some("cadence_tie"))]
        }
        (Species::Second, Penultimate) if many => &[
            ("2 2", None),
            ("⁀2 2", Some("cadence_tie")),
            ("1", Some("cadence_tie")),
        ],
        (Species::Second, _) => &[("2 2", None)],
        (Species::Third, Entry) if many => &[("4 4 4 4", None), ("r4 4 4 4", Some("rest"))],
        (Species::Third, Entry) => &[("4 4 4 4", None), ("r4 4 4 4", Some("rest_two_voices"))],
        (Species::Third, Penultimate) if many => &[("4 4 4 4", None), ("1", Some("cadence_whole"))],
        (Species::Third, _) => &[("4 4 4 4", None)],
        // Measures without a syncopation are judged by G4-BREAK.
        (Species::Fourth, Entry) => &[
            ("r2 2⁀", None),
            ("r2 2", None),
            ("2 2⁀", Some("no_rest")),
            ("2 2", Some("no_rest")),
        ],
        (Species::Fourth, Penultimate) => &[("⁀2 2", None), ("2 2", None)],
        (Species::Fourth, _) if many => &[
            ("⁀2 2⁀", None),
            ("⁀2 2", None),
            ("2 2⁀", None),
            ("2 2", None),
            ("r2 2⁀", Some("rest")),
            ("r2 2", Some("rest")),
        ],
        (Species::Fourth, _) => &[
            ("⁀2 2⁀", None),
            ("⁀2 2", None),
            ("2 2⁀", None),
            ("2 2", None),
        ],
        (Species::Fifth, Entry) => &[("r2 2⁀", None), ("r2 2", Some("entry_untied"))],
        (Species::Fifth, Penultimate) if many => &[("⁀2 2", None), ("⁀2 4 4", Some("occasional"))],
        (Species::Fifth, Penultimate) => &[("⁀2 2", None)],
        (Species::Fifth, _) if many => FIFTH_INNER_MANY,
        (Species::Fifth, _) => FIFTH_INNER,
        (Species::Free, _) => FREE,
    }
}

/// One note value of a rhythm; a tie into the next measure is not part of it, because the next
/// measure decides it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    /// Continues the note before, tied over the barline.
    pub tied_in: bool,
    pub rest: bool,
    pub ticks: u32,
}

impl Token {
    /// Parses `⁀2`, `r4`, `2.` or `8⁀` (the trailing tie is ignored).
    pub fn parse(text: &str) -> Option<Token> {
        let tied_in = text.starts_with('⁀');
        let text = text.trim_start_matches('⁀').trim_end_matches('⁀');
        let rest = text.starts_with('r');
        let text = text.trim_start_matches('r');
        let (number, dotted) = match text.strip_suffix('.') {
            Some(number) => (number, true),
            None => (text, false),
        };
        let value: u32 = number.parse().ok()?;
        let plain = TICKS_PER_WHOLE.checked_div(value)?;
        let ticks = if dotted { plain * 3 / 2 } else { plain };
        (ticks > 0).then_some(Token {
            tied_in,
            rest,
            ticks,
        })
    }

    /// The token of a written note.
    pub fn of(note: &Note, tied_in: bool) -> Token {
        Token {
            tied_in,
            rest: note.is_rest(),
            ticks: note.duration.ticks(),
        }
    }
}

/// The tokens of a pattern.
pub fn tokens(pattern: &str) -> Vec<Token> {
    pattern.split(' ').filter_map(Token::parse).collect()
}

/// A note value in pattern notation.
#[cfg(test)]
fn value(ticks: u32) -> String {
    let mut out = String::new();
    push_value(&mut out, ticks);
    out
}

/// Appends a note value in pattern notation to `out`.
fn push_value(out: &mut String, ticks: u32) {
    use std::fmt::Write;
    // Writing to a string cannot fail.
    let _ = if TICKS_PER_WHOLE.is_multiple_of(ticks) {
        write!(out, "{}", TICKS_PER_WHOLE / ticks)
    } else if (3 * TICKS_PER_WHOLE).is_multiple_of(2 * ticks) {
        write!(out, "{}.", 3 * TICKS_PER_WHOLE / (2 * ticks))
    } else {
        write!(out, "{ticks}/{TICKS_PER_WHOLE}")
    };
}

/// The rhythm of one measure in pattern notation, e.g. `⁀2 4 4`.
pub fn measure_rhythm(notes: &[&Note], tied_in: bool) -> String {
    // One buffer for all tokens: the rhythm rules ask for every measure of every candidate.
    let mut out = String::with_capacity(8 * notes.len());
    for (k, note) in notes.iter().enumerate() {
        if k > 0 {
            out.push(' ');
        }
        if k == 0 && tied_in {
            out.push('⁀');
        }
        if note.is_rest() {
            out.push('r');
        }
        push_value(&mut out, note.duration.ticks());
        if note.tie {
            out.push('⁀');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Duration;

    #[test]
    fn tokens_and_values() {
        assert_eq!(
            tokens("⁀4 8 8 2⁀"),
            [
                Token {
                    tied_in: true,
                    rest: false,
                    ticks: 2
                },
                Token {
                    tied_in: false,
                    rest: false,
                    ticks: 1
                },
                Token {
                    tied_in: false,
                    rest: false,
                    ticks: 1
                },
                Token {
                    tied_in: false,
                    rest: false,
                    ticks: 4
                },
            ]
        );
        assert_eq!(Token::parse("2.").map(|t| t.ticks), Some(6));
        assert_eq!(
            Token::parse("r1").map(|t| (t.rest, t.ticks)),
            Some((true, 8))
        );
        assert_eq!(Token::parse("x"), None);
        let half = Note::new("D4".parse().unwrap(), Duration::HALF).tied();
        let quarter = Note::new("C4".parse().unwrap(), Duration::QUARTER);
        assert_eq!(
            measure_rhythm(&[&quarter, &quarter, &half], true),
            "⁀4 4 2⁀"
        );
        assert_eq!(value(6), "2.");
    }

    #[test]
    fn every_pattern_fills_a_measure() {
        let places = [
            Place::Silent,
            Place::Entry,
            Place::Inner,
            Place::Antepenultimate,
            Place::Penultimate,
            Place::Last,
        ];
        for species in Species::ALL.into_iter().chain([Species::Free]) {
            for place in places {
                for many in [false, true] {
                    for (pattern, _) in patterns(species, place, many) {
                        let ticks: u32 = tokens(pattern).iter().map(|t| t.ticks).sum();
                        assert_eq!(ticks, TICKS_PER_WHOLE, "{species:?} {place:?} {pattern}");
                    }
                }
            }
        }
    }
}
