//! Standard MIDI files: one track per voice, for a single score ([`to_midi`]) and for the
//! movement of a mass ([`to_movement_midi`]).

use fux_core::{Score, SectionKind, TICKS_PER_WHOLE};
use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

/// MIDI ticks per quarter note.
const PPQ: u32 = 480;

/// Options for the MIDI output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MidiOptions {
    /// Half notes per minute.
    pub tempo: u32,
    /// General MIDI program, 1–128 as printed in GM lists (53 = Choir Aahs).
    pub program: u8,
    pub velocity: u8,
}

impl Default for MidiOptions {
    fn default() -> Self {
        MidiOptions {
            tempo: 72,
            program: 53,
            velocity: 80,
        }
    }
}

/// The score as a Standard MIDI File (format 1): a tempo track, then one track per voice on its
/// own channel. Tied notes sound as one note.
pub fn to_midi(score: &Score, options: &MidiOptions) -> Vec<u8> {
    let title = score.title.clone().unwrap_or_default();
    write(score, &title, &[], &[], false, options)
}

/// The movement of a mass as a Standard MIDI File (format 1, docs/design/phase-14/spec.md § 6):
/// the score of the movement as [`to_midi`] writes a score, its tempo track named `name` („Missa
/// brevis – Gloria“) with a marker (FF 06) where each section begins and where each Break
/// begins, its title or label as text, and a silent measure after every double bar, before the
/// next section, at the same tempo throughout; the tempo track lasts as long as the movement, as
/// the tracks of the voices do. A voice sounds where it sings and is silent where it does not
/// sing in a section. Seams get no marker. A score without marks of sections is written as
/// [`to_midi`] writes it, with `name` as its name.
pub fn to_movement_midi(score: &Score, name: &str, options: &MidiOptions) -> Vec<u8> {
    let per_measure = score.meter.ticks_per_measure();
    // The ticks at which a section begins after a double bar.
    let breaks: Vec<u32> = score
        .sections
        .iter()
        .filter(|mark| mark.kind == SectionKind::Section && mark.measure > 0)
        .map(|mark| mark.measure as u32 * per_measure)
        .collect();
    let markers: Vec<(u32, &[u8])> = score
        .sections
        .iter()
        .filter(|mark| mark.kind != SectionKind::Seam)
        .filter_map(|mark| {
            let title = mark.title.as_deref()?;
            Some((mark.measure as u32 * per_measure, title.as_bytes()))
        })
        .collect();
    let lasting = !score.sections.is_empty();
    write(score, name, &breaks, &markers, lasting, options)
}

/// The score as a Standard MIDI File: a tempo track named `name` with the tempo, the meter and
/// the `markers` (tick of the score, text), ending with its last marker or, where `lasting`,
/// with the score, then one track per voice. Every tick of the score from one of `breaks` on
/// sounds a measure later for each of them.
fn write(
    score: &Score,
    name: &str,
    breaks: &[u32],
    markers: &[(u32, &[u8])],
    lasting: bool,
    options: &MidiOptions,
) -> Vec<u8> {
    let per_tick = PPQ * 4 / TICKS_PER_WHOLE;
    let per_measure = score.meter.ticks_per_measure();
    let gap = |tick: u32| breaks.iter().filter(|&&b| b <= tick).count() as u32 * per_measure;
    let mut smf = Smf::new(Header::new(
        Format::Parallel,
        Timing::Metrical(u15::new(PPQ as u16)),
    ));
    let quarters_per_minute = 2 * options.tempo.max(1);
    let micros_per_quarter = 60_000_000 / quarters_per_minute;
    let denominator_power = score.meter.denominator.trailing_zeros() as u8;
    let mut tempo_track = vec![
        meta(0, MetaMessage::TrackName(name.as_bytes())),
        meta(0, MetaMessage::Tempo(u24::new(micros_per_quarter))),
        meta(
            0,
            MetaMessage::TimeSignature(score.meter.numerator as u8, denominator_power, 24, 8),
        ),
    ];
    let mut last = 0;
    for &(tick, text) in markers {
        let time = (tick + gap(tick)) * per_tick;
        tempo_track.push(meta(time - last, MetaMessage::Marker(text)));
        last = time;
    }
    let end = if lasting {
        let ticks = score.total_ticks();
        ((ticks + gap(ticks)) * per_tick).max(last)
    } else {
        last
    };
    tempo_track.push(meta(end - last, MetaMessage::EndOfTrack));
    smf.tracks.push(tempo_track);
    let program = u7::new(options.program.clamp(1, 128) - 1);
    let velocity = u7::new(options.velocity.min(127));
    for (v, voice) in score.voices.iter().enumerate() {
        let channel = u4::new(midi_channel(v));
        let mut events: Vec<(u32, bool, u8)> = Vec::new();
        for s in voice.soundings() {
            let key = s.pitch.midi().clamp(0, 127) as u8;
            // A note never crosses a double bar: it ends where its section ends at the latest.
            let later = gap(s.start);
            events.push(((s.start + later) * per_tick, true, key));
            events.push(((s.end + later) * per_tick, false, key));
        }
        // Note-offs before note-ons at the same time, so repeated pitches are restruck.
        events.sort_by_key(|&(time, on, key)| (time, on, key));
        let mut track = vec![
            meta(0, MetaMessage::TrackName(voice.name.as_bytes())),
            TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::ProgramChange { program },
                },
            },
        ];
        let mut last = 0;
        for (time, on, key) in events {
            let message = if on {
                MidiMessage::NoteOn {
                    key: u7::new(key),
                    vel: velocity,
                }
            } else {
                MidiMessage::NoteOff {
                    key: u7::new(key),
                    vel: u7::new(0),
                }
            };
            track.push(TrackEvent {
                delta: u28::new(time - last),
                kind: TrackEventKind::Midi { channel, message },
            });
            last = time;
        }
        track.push(meta(0, MetaMessage::EndOfTrack));
        smf.tracks.push(track);
    }
    let mut bytes = Vec::new();
    smf.write_std(&mut bytes)
        .expect("writing to a Vec cannot fail");
    bytes
}

/// Channels 1–16 without the percussion channel 10, from 0.
pub(crate) fn midi_channel(voice: usize) -> u8 {
    let c = (voice % 15) as u8;
    if c >= 9 { c + 1 } else { c }
}

fn meta(delta: u32, message: MetaMessage<'_>) -> TrackEvent<'_> {
    TrackEvent {
        delta: u28::new(delta),
        kind: TrackEventKind::Meta(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_skip_percussion() {
        let channels: Vec<u8> = (0..12).map(midi_channel).collect();
        assert_eq!(channels, [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12]);
    }
}
