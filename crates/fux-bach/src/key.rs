//! Keys, chords and the harmony of a piece as a timeline of chords.

use std::fmt;

use crate::pitch::{Lof, Pitch};
use crate::time::Tick;

/// Major or minor key with its tonic.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Key {
    pub tonic: Lof,
    pub minor: bool,
}

/// Line-of-fifths offsets of the seven degrees from the tonic.
const MAJOR: [i8; 7] = [0, 2, 4, -1, 1, 3, 5];
const MINOR: [i8; 7] = [0, 2, -3, -1, 1, -4, -2];

impl Key {
    pub const fn major(tonic: Lof) -> Key {
        Key {
            tonic,
            minor: false,
        }
    }

    pub const fn minor(tonic: Lof) -> Key {
        Key { tonic, minor: true }
    }

    /// `C`, `f#`, `Bb`, `e-`: an upper-case letter is major.
    pub fn parse(name: &str) -> Option<Key> {
        let name = name.trim().trim_end_matches(':');
        let minor = name.chars().next()?.is_ascii_lowercase();
        Some(Key {
            tonic: Lof::parse(name)?,
            minor,
        })
    }

    /// Degree 0–6 of the major or natural minor scale.
    pub const fn degree(self, degree: usize) -> Lof {
        let table = if self.minor { &MINOR } else { &MAJOR };
        Lof(self.tonic.0 + table[degree % 7])
    }

    /// The raised sixth and seventh degrees of minor; in major the plain ones.
    pub const fn raised(self, degree: usize) -> Lof {
        if self.minor && (degree == 5 || degree == 6) {
            Lof(self.degree(degree).0 + 7)
        } else {
            self.degree(degree)
        }
    }

    /// Whether the spelled class belongs to the scale; in minor the raised sixth and seventh
    /// degrees belong to it as well.
    pub const fn contains(self, class: Lof) -> bool {
        let d = class.0 - self.tonic.0;
        if self.minor {
            (d >= -4 && d <= 2) || d == 3 || d == 5
        } else {
            d >= -1 && d <= 5
        }
    }

    /// The scale as a set of pitch classes (bit `pc`).
    pub fn mask(self) -> u16 {
        let mut mask = 0;
        for degree in 0..7 {
            mask |= 1 << self.degree(degree).pc();
            mask |= 1 << self.raised(degree).pc();
        }
        mask
    }

    /// Sharps (positive) or flats (negative) of the key signature.
    pub const fn signature(self) -> i8 {
        if self.minor {
            self.tonic.0 - 3
        } else {
            self.tonic.0
        }
    }

    /// Degree 0–6 of a spelled class by its letter.
    pub const fn degree_of(self, class: Lof) -> usize {
        (class.step() - self.tonic.step()).rem_euclid(7) as usize
    }

    /// Spelling of a pitch class: the scale's own spelling where it has one, otherwise the sharp
    /// side (leading tones) if `sharp` is set and the flat side if not.
    pub fn spell(self, pc: u8, sharp: bool) -> Lof {
        for degree in 0..7 {
            for class in [self.degree(degree), self.raised(degree)] {
                if class.pc() == pc {
                    return class;
                }
            }
        }
        let low = if self.minor {
            self.tonic.0 - 4
        } else {
            self.tonic.0 - 1
        };
        if sharp {
            Lof::spell(pc, low)
        } else {
            Lof::spell(pc, low - 5)
        }
    }

    pub const fn dominant(self) -> Key {
        Key {
            tonic: Lof(self.tonic.0 + 1),
            minor: self.minor,
        }
    }

    pub const fn subdominant(self) -> Key {
        Key {
            tonic: Lof(self.tonic.0 - 1),
            minor: self.minor,
        }
    }

    /// The relative key: a minor third below a major key, above a minor one.
    pub const fn relative(self) -> Key {
        if self.minor {
            Key::major(Lof(self.tonic.0 - 3))
        } else {
            Key::minor(Lof(self.tonic.0 + 3))
        }
    }

    /// Distance on the circle of fifths between the signatures.
    pub const fn distance(self, other: Key) -> i8 {
        (self.signature() - other.signature()).abs()
    }

    pub fn lilypond(self) -> String {
        let tonic = Pitch::new(self.tonic.step(), self.tonic.alter(), 3).lilypond();
        format!(
            "\\key {} \\{}",
            tonic.trim_end_matches([',', '\'']),
            if self.minor { "minor" } else { "major" }
        )
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.tonic.to_string();
        if self.minor {
            write!(f, "{}", name.to_lowercase())
        } else {
            write!(f, "{name}")
        }
    }
}

/// Chord types. The offsets are positions on the line of fifths relative to the root, in the
/// order root, third, fifth, seventh, ninth.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Kind {
    Maj,
    Min,
    Dim,
    Aug,
    Dom7,
    Min7,
    Maj7,
    HalfDim7,
    Dim7,
    MinMaj7,
    AugMaj7,
    Dom9,
    DomMin9,
    /// German sixth, written from the raised fourth degree: ♯4, ♭6, 1, ♭3.
    Ger,
}

impl Kind {
    pub const ALL: [Kind; 14] = [
        Kind::Maj,
        Kind::Min,
        Kind::Dim,
        Kind::Aug,
        Kind::Dom7,
        Kind::Min7,
        Kind::Maj7,
        Kind::HalfDim7,
        Kind::Dim7,
        Kind::MinMaj7,
        Kind::AugMaj7,
        Kind::Dom9,
        Kind::DomMin9,
        Kind::Ger,
    ];

    pub const fn offsets(self) -> &'static [i8] {
        match self {
            Kind::Maj => &[0, 4, 1],
            Kind::Min => &[0, -3, 1],
            Kind::Dim => &[0, -3, -6],
            Kind::Aug => &[0, 4, 8],
            Kind::Dom7 => &[0, 4, 1, -2],
            Kind::Min7 => &[0, -3, 1, -2],
            Kind::Maj7 => &[0, 4, 1, 5],
            Kind::HalfDim7 => &[0, -3, -6, -2],
            Kind::Dim7 => &[0, -3, -6, -9],
            Kind::MinMaj7 => &[0, -3, 1, 5],
            Kind::AugMaj7 => &[0, 4, 8, 5],
            Kind::Dom9 => &[0, 4, 1, -2, 2],
            Kind::DomMin9 => &[0, 4, 1, -2, -5],
            Kind::Ger => &[0, -10, -6, -9],
        }
    }

    pub const fn is_seventh(self) -> bool {
        self.offsets().len() >= 4
    }

    /// Dominant function by structure: a major third with a minor seventh, or a diminished fifth.
    pub const fn is_dominant(self) -> bool {
        matches!(
            self,
            Kind::Dom7 | Kind::Dom9 | Kind::DomMin9 | Kind::Dim | Kind::Dim7 | Kind::HalfDim7
        )
    }

    pub const fn label(self) -> &'static str {
        match self {
            Kind::Maj => "",
            Kind::Min => "m",
            Kind::Dim => "o",
            Kind::Aug => "+",
            Kind::Dom7 => "7",
            Kind::Min7 => "m7",
            Kind::Maj7 => "maj7",
            Kind::HalfDim7 => "ø7",
            Kind::Dim7 => "o7",
            Kind::MinMaj7 => "mmaj7",
            Kind::AugMaj7 => "+maj7",
            Kind::Dom9 => "9",
            Kind::DomMin9 => "b9",
            Kind::Ger => "Ger",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Chord {
    pub root: Lof,
    pub kind: Kind,
    /// The bass tone if the source names the inversion.
    pub bass: Option<Lof>,
}

impl Chord {
    pub const fn new(root: Lof, kind: Kind) -> Chord {
        Chord {
            root,
            kind,
            bass: None,
        }
    }

    pub fn tones(self) -> impl Iterator<Item = Lof> {
        self.kind
            .offsets()
            .iter()
            .map(move |o| Lof(self.root.0 + o))
    }

    /// The chord as a set of pitch classes (bit `pc`).
    pub fn mask(self) -> u16 {
        self.tones().fold(0, |m, t| m | 1 << t.pc())
    }

    pub fn has(self, pitch: Pitch) -> bool {
        self.mask() & (1 << pitch.pc()) != 0
    }

    /// 0 root, 1 third, 2 fifth, 3 seventh, 4 ninth.
    pub fn member(self, pc: u8) -> Option<usize> {
        self.tones().position(|t| t.pc() == pc)
    }

    /// The spelled class of a chord tone with this pitch class.
    pub fn spelled(self, pc: u8) -> Option<Lof> {
        self.tones().find(|t| t.pc() == pc)
    }

    /// With the inversion: 0 root position, 1 the third in the bass, and so on.
    pub fn inverted(mut self, inversion: usize) -> Chord {
        self.bass = self.tones().nth(inversion);
        self
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.root, self.kind.label())?;
        match self.bass {
            Some(bass) if bass != self.root => write!(f, "/{bass}"),
            _ => Ok(()),
        }
    }
}

/// A chord from `on` until the next span begins.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Span {
    pub on: Tick,
    pub chord: Chord,
    pub key: Key,
}

/// The harmony of a piece: spans in time order, each lasting until the next.
#[derive(Clone, Debug, Default)]
pub struct Harmony {
    pub spans: Vec<Span>,
}

impl Harmony {
    pub fn index_at(&self, t: Tick) -> Option<usize> {
        self.spans.partition_point(|s| s.on <= t).checked_sub(1)
    }

    pub fn at(&self, t: Tick) -> Option<&Span> {
        self.index_at(t).map(|i| &self.spans[i])
    }

    pub fn chord_at(&self, t: Tick) -> Option<Chord> {
        self.at(t).map(|s| s.chord)
    }

    pub fn key_at(&self, t: Tick) -> Option<Key> {
        self.at(t).map(|s| s.key)
    }

    /// Appends a span; a span with the chord and key of the last one is merged into it.
    pub fn push(&mut self, on: Tick, chord: Chord, key: Key) {
        match self.spans.last_mut() {
            Some(last) if last.on == on => {
                last.chord = chord;
                last.key = key;
            }
            Some(last) if last.chord == chord && last.key == key => {}
            _ => self.spans.push(Span { on, chord, key }),
        }
    }

    /// Number of chord changes in `(from, until]`.
    pub fn changes(&self, from: Tick, until: Tick) -> usize {
        self.spans
            .iter()
            .filter(|s| s.on > from && s.on <= until)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys() {
        let c_minor = Key::parse("c").unwrap();
        assert!(c_minor.minor);
        assert_eq!(c_minor.signature(), -3);
        assert_eq!(c_minor.degree(2), Lof::parse("Eb").unwrap());
        assert_eq!(c_minor.raised(6), Lof::parse("B").unwrap());
        assert_eq!(c_minor.raised(5), Lof::parse("A").unwrap());
        assert!(c_minor.contains(Lof::parse("B").unwrap()));
        assert!(!c_minor.contains(Lof::parse("F#").unwrap()));
        assert_eq!(c_minor.relative(), Key::parse("Eb").unwrap());
        assert_eq!(
            Key::parse("A").unwrap().relative(),
            Key::parse("f#").unwrap()
        );
        assert_eq!(c_minor.spell(6, true), Lof::parse("F#").unwrap());
        assert_eq!(c_minor.spell(1, false), Lof::parse("Db").unwrap());
        let a_major = Key::parse("A:").unwrap();
        assert_eq!(a_major.spell(0, false), Lof::parse("C").unwrap());
        assert_eq!(a_major.spell(3, true), Lof::parse("D#").unwrap());
        assert_eq!(a_major.degree_of(Lof::parse("G#").unwrap()), 6);
        assert_eq!(Key::parse("bb").unwrap().lilypond(), "\\key bes \\minor");
    }

    #[test]
    fn chords() {
        let g7 = Chord::new(Lof(1), Kind::Dom7);
        let names: Vec<String> = g7.tones().map(|t| t.to_string()).collect();
        assert_eq!(names, ["G", "B", "D", "F"]);
        assert_eq!(g7.member(5), Some(3));
        assert_eq!(g7.inverted(1).to_string(), "G7/B");
        let dim = Chord::new(Lof(5), Kind::Dim7);
        let names: Vec<String> = dim.tones().map(|t| t.to_string()).collect();
        assert_eq!(names, ["B", "D", "F", "Ab"]);
        let ger = Chord::new(Lof(6), Kind::Ger);
        let names: Vec<String> = ger.tones().map(|t| t.to_string()).collect();
        assert_eq!(names, ["F#", "Ab", "C", "Eb"]);
    }
}
