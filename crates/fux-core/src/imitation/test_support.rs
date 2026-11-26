//! Helpers for the unit tests of imitation and of its driver: texts, the usual choirs and
//! imitative requests, as `fux free` makes them, and lines and heads written out note by note.

use std::sync::Arc;

use crate::clef::VoiceType;
use crate::generate::{FreeRequest, FreeText};
use crate::imitation::Imitation;
use crate::imitation::check::{Head, Tone};
use crate::text::underlay::Choice;
use crate::text::{Text, TextOptions, resolve};

/// A text as `fux free --text` takes it: a section of the Ordinary or an own Latin text.
pub(crate) fn text(spec: &str) -> Arc<Text> {
    Arc::new(resolve(spec, &TextOptions::default()).unwrap().0)
}

/// The usual choir of `voices` voices, as `fux free` sets it, from the top.
pub(crate) fn choir(voices: usize) -> Vec<VoiceType> {
    use VoiceType::*;
    match voices {
        2 => vec![Soprano, Tenor],
        3 => vec![Alto, Tenor, Bass],
        4 => vec![Soprano, Alto, Tenor, Bass],
        5 => vec![Soprano, Alto, Tenor, Tenor, Bass],
        _ => vec![Soprano, Soprano, Alto, Tenor, Tenor, Bass],
    }
}

/// An imitative request on the text `spec` in `mode` for the usual choir of `voices`, its
/// length and cadences planned from the text, everything else automatic.
pub(crate) fn request(spec: &str, mode: &str, voices: usize, seed: u64) -> FreeRequest {
    FreeRequest {
        text: Some(FreeText::new(text(spec))),
        seed,
        imitation: Some(Imitation::default()),
        ..FreeRequest::new(mode.parse().unwrap(), choir(voices), 0)
    }
}

/// A line from `pitch.ticks` notes, the first at `start`, one after the other.
pub(crate) fn line(start: u32, notes: &str) -> Vec<Tone> {
    let mut tick = start;
    notes
        .split_whitespace()
        .map(|n| {
            let (pitch, ticks) = n.split_once('.').unwrap();
            let ticks: u32 = ticks.parse().unwrap();
            let tone = Tone {
                pitch: pitch.parse().unwrap(),
                start: tick,
                end: tick + ticks,
            };
            tick += ticks;
            tone
        })
        .collect()
}

/// An entry of `voice` after `rest` with `pitch.ticks` notes, singing nothing in particular.
pub(crate) fn head(voice: usize, start: u32, rest: u32, notes: &str) -> Head {
    let tones = line(start, notes);
    Head {
        voice,
        rest,
        sung: vec![Choice::Hold; tones.len()],
        tones,
    }
}
