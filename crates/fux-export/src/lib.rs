//! Export of `fux-core` scores: LilyPond source, PDF/SVG/PNG rendered by an external `lilypond`
//! process, Standard MIDI Files and MusicXML (which can also be read back).

pub mod lilypond;
mod lyrics;
mod marks;
pub mod mensur;
pub mod midi;
pub mod musicxml;
pub mod render;

pub use lilypond::{
    BookPart, ClefStyle, Color, Highlight, LILYPOND_VERSION, LilypondOptions, MassBookOptions,
    SectionEnd, Sheet, book, document, mass_book, to_lilypond,
};
pub use mensur::{Mensur, merge_ties};
pub use midi::{MidiOptions, to_midi, to_movement_midi};
pub use musicxml::{
    FIELD_CADENCES, FIELD_IMITATION, FIELD_MODE, FIELD_TEXT, ImportedPart, ImportedScore,
    MovementHeading, MovementOptions, MusicXmlError, MusicXmlOptions, from_musicxml,
    to_movement_musicxml, to_musicxml, to_musicxml_with_fields,
};
pub use render::{Graphic, RenderError, lilypond_version, render};
