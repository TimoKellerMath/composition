//! Feasibility probe: can the approach of this project (rules calibrated on a corpus, a search
//! with fixed notes) write fugues in Bach's style on a given subject?
//!
//! The crate is a measuring device beside the Fux code, which it leaves untouched: its own time
//! grid (96 ticks to the whole note), keys instead of modes, chords as an input of every check,
//! and a prototype of answer, countersubject, exposition and free voices. The protocol and the
//! results are in `docs/design/bach-fugen/machbarkeit/`.

pub mod algomus;
pub mod answer;
pub mod api;
#[cfg(feature = "tools")]
pub mod b1;
#[cfg(feature = "tools")]
pub mod b2;
#[cfg(feature = "tools")]
pub mod b3;
#[cfg(feature = "tools")]
pub mod b4;
#[cfg(feature = "tools")]
pub mod b5;
#[cfg(feature = "tools")]
pub mod b7;
pub mod base64;
pub mod bass;
pub mod cells;
pub mod check;
pub mod chord;
pub mod chordline;
pub mod clock;
pub mod compose;
pub mod corrupt;
pub mod draft;
pub mod engine;
pub mod export;
pub mod figures;
pub mod finder;
pub mod fugue;
pub mod hands;
pub mod harmonize;
pub mod kern;
pub mod key;
pub mod layout;
pub mod melody;
pub mod moments;
pub mod motifs;
pub mod musicxml;
#[cfg(feature = "tools")]
pub mod musicxml_read;
#[cfg(feature = "tools")]
pub mod neusatz;
pub mod pitch;
pub mod proxy;
pub mod rhythm;
pub mod rng;
pub mod roman;
pub mod rules;
pub mod scaffold;
pub mod score;
pub mod search;
pub mod stats;
pub mod stretto;
#[cfg(feature = "tools")]
pub mod study;
#[cfg(feature = "tools")]
pub mod study_answers;
#[cfg(feature = "tools")]
pub mod study_fugues;
pub mod subject;
pub mod tables;
pub mod themes;
pub mod time;
pub mod variants;
