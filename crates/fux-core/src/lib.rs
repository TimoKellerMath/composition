//! Music theory core for species counterpoint after Johann Joseph Fux,
//! *Gradus ad Parnassum* (1725).
//!
//! Pitches keep their spelling (step, alteration, octave), so intervals have a quality: an
//! augmented fourth stays distinct from a diminished fifth, an augmented second from a minor
//! third. Scores have any number of voices and any length, measured in integer ticks: an eighth
//! note is one tick, a measure of 2/2 holds eight.
//!
//! The crate has no I/O dependencies, so it also compiles to WebAssembly.

pub mod clef;
pub mod corpus;
pub mod error;
pub mod gabc;
pub mod generate;
pub mod imitation;
pub mod interval;
pub mod kern;
pub mod lang;
pub mod mass;
pub mod mode;
pub mod parse;
pub mod pitch;
pub mod rhythm;
pub mod rules;
pub mod score;
pub mod text;
pub mod time;

pub use clef::{Clef, VoiceType};
pub use error::{ParseError, ParseErrorKind};
pub use interval::{Interval, Quality};
pub use lang::Lang;
pub use mode::{Ambitus, Clausula, Mode, ModeSpec};
pub use parse::{parse_lilypond, parse_lilypond_measures, parse_notes, parse_pitches};
pub use pitch::{Pitch, Step};
pub use score::{
    Cadence, Lyric, Note, Role, Score, ScoreEntry, ScoreError, ScoreImitation, ScorePoint,
    ScoreText, SectionKind, SectionMark, Slice, Sounding, Species, Voice,
};
pub use time::{Duration, Meter, TICKS_PER_WHOLE};
