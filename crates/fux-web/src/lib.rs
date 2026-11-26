//! The web version of `fux`: a WebAssembly module without imports that answers JSON requests
//! from the page in `web/`. It sets counterpoint, generates cantus firmi, checks scores and
//! shows Fux's examples, like the command line, and answers with the notes as MusicXML (which
//! the page renders), LilyPond, MIDI and text, and the rule violations. A mass (phase 14c) it
//! plans, sets part by part (the page runs several workers side by side, each with a module of
//! its own) and puts together into the files of its movements and its book.
//!
//! Build with `tools/build_web.sh`; the module runs in a web worker, so the page stays usable
//! during a search.

mod api;
mod base64;
mod ffi;
mod mass;

pub use api::{Answer, Parts, Request, answer, answer_json};
pub use mass::{
    MassForm, MassView, MovementView, PartAnswer, PartFile, PlannedPart, SectionChoice, SectionRow,
};
