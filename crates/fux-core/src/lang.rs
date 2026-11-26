//! Language of user-facing names and messages.

use std::fmt;
use std::str::FromStr;

/// Language for names of pitches, intervals, modes and voices, and for rule messages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Lang {
    /// German (default): note names with H and B, „reine Quinte“, „dorisch“.
    #[default]
    De,
    /// English: note names with B and B♭, "perfect fifth", "Dorian".
    En,
}

impl Lang {
    /// Picks the German or English variant of a text.
    pub fn pick<'a>(self, de: &'a str, en: &'a str) -> &'a str {
        match self {
            Lang::De => de,
            Lang::En => en,
        }
    }
}

impl FromStr for Lang {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "de" | "deutsch" | "german" => Ok(Lang::De),
            "en" | "englisch" | "english" => Ok(Lang::En),
            _ => Err(format!("unknown language {s:?} (expected de or en)")),
        }
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.pick("de", "en"))
    }
}
