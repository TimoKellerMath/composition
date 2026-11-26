//! Errors for parsing user input.

use thiserror::Error;

/// What went wrong while parsing a token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// The token is not a pitch, note, rest or bar check.
    UnknownToken,
    /// A pitch without an octave number in scientific notation.
    MissingOctave,
    /// A duration that cannot be expressed in eighth-note ticks, e.g. a sixteenth.
    UnsupportedDuration,
    /// A tie `~` without a preceding note.
    DanglingTie,
    /// A bar check `|` that does not fall on a barline; carries the ticks into the measure.
    BarCheck(u32),
    /// Unknown mode name.
    UnknownMode,
    /// Unknown clef name.
    UnknownClef,
    /// Unknown voice type.
    UnknownVoiceType,
}

/// A parse error with the offending token and its byte offset in the input.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{}: {token:?} (position {position})", describe(.kind))]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub token: String,
    pub position: usize,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, token: impl Into<String>, position: usize) -> ParseError {
        ParseError {
            kind,
            token: token.into(),
            position,
        }
    }
}

fn describe(kind: &ParseErrorKind) -> String {
    match kind {
        ParseErrorKind::UnknownToken => "unknown token".into(),
        ParseErrorKind::MissingOctave => "pitch without octave number".into(),
        ParseErrorKind::UnsupportedDuration => "unsupported duration".into(),
        ParseErrorKind::DanglingTie => "tie without a preceding note".into(),
        ParseErrorKind::BarCheck(ticks) => {
            format!("bar check {ticks} eighths after the barline")
        }
        ParseErrorKind::UnknownMode => "unknown mode".into(),
        ParseErrorKind::UnknownClef => "unknown clef".into(),
        ParseErrorKind::UnknownVoiceType => "unknown voice type".into(),
    }
}
