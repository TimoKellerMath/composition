//! Playback: the MIDI of a score through a SoundFont (rustysynth) to the sound card (rodio).

use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};

const SAMPLE_RATE: u32 = 44_100;
/// Samples per channel rendered at once.
const BLOCK: usize = 512;

/// Where SoundFonts usually lie: `$FUX_SOUNDFONT`, then the common places on Linux.
pub fn find_soundfont() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("FUX_SOUNDFONT") {
        return Some(PathBuf::from(path));
    }
    [
        "/usr/share/soundfonts/FluidR3_GM.sf2",
        "/usr/share/sounds/sf2/FluidR3_GM.sf2",
        "/usr/share/soundfonts/default.sf2",
        "/usr/share/sounds/sf2/default-GM.sf2",
        "/usr/share/soundfonts/GeneralUser-GS.sf2",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|p| p.is_file())
}

enum Font {
    None,
    Loading(PathBuf, JoinHandle<Result<Arc<SoundFont>, String>>),
    Ready(PathBuf, Arc<SoundFont>),
    Failed(String),
}

/// The SoundFont and the playing sound.
pub struct Audio {
    font: Font,
    /// Opened on the first playback; the sound stops when it is dropped.
    device: Option<rodio::MixerDeviceSink>,
    player: Option<rodio::Player>,
}

impl Default for Audio {
    fn default() -> Self {
        Audio {
            font: Font::None,
            device: None,
            player: None,
        }
    }
}

impl Audio {
    /// Loads the SoundFont at `path` in the background, unless it is loaded or loading.
    pub fn load(&mut self, path: &Path) {
        match &self.font {
            Font::Loading(p, _) | Font::Ready(p, _) if p == path => return,
            _ => {}
        }
        let owned = path.to_path_buf();
        let handle = std::thread::spawn(move || {
            let mut file = std::io::BufReader::new(
                std::fs::File::open(&owned).map_err(|e| format!("{}: {e}", owned.display()))?,
            );
            SoundFont::new(&mut file)
                .map(Arc::new)
                .map_err(|e| format!("{}: {e}", owned.display()))
        });
        self.font = Font::Loading(path.to_path_buf(), handle);
    }

    /// Takes a loaded SoundFont over.
    pub fn poll(&mut self) {
        if let Font::Loading(_, handle) = &self.font
            && handle.is_finished()
        {
            let Font::Loading(path, handle) = std::mem::replace(&mut self.font, Font::None) else {
                unreachable!()
            };
            self.font = match handle.join() {
                Ok(Ok(font)) => Font::Ready(path, font),
                Ok(Err(error)) => Font::Failed(error),
                Err(_) => Font::Failed("SoundFont: internal error".into()),
            };
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.font, Font::Loading(..))
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.font, Font::Ready(..))
    }

    /// Why the SoundFont is missing, if it failed to load.
    pub fn error(&self) -> Option<&str> {
        match &self.font {
            Font::Failed(error) => Some(error),
            _ => None,
        }
    }

    /// Plays a Standard MIDI File, stopping what plays.
    pub fn play(&mut self, midi: &[u8]) -> Result<(), String> {
        let Font::Ready(_, font) = &self.font else {
            return Err("SoundFont".into());
        };
        let song = Song::from_midi(font, midi)?;
        self.stop();
        if self.device.is_none() {
            self.device =
                Some(rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?);
        }
        let device = self.device.as_ref().expect("opened");
        let player = rodio::Player::connect_new(device.mixer());
        player.append(song);
        self.player = Some(player);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(player) = self.player.take() {
            player.stop();
        }
    }

    pub fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(|p| !p.empty())
    }
}

/// A sequencer as a source of stereo samples; it ends a second after the last note.
struct Song {
    sequencer: MidiFileSequencer,
    left: Vec<f32>,
    right: Vec<f32>,
    /// Next sample in the block, counting both channels.
    index: usize,
    /// Blocks still to render after the end, for the notes to die away.
    tail: usize,
}

impl Song {
    /// A Standard MIDI File through the SoundFont.
    fn from_midi(font: &Arc<SoundFont>, midi: &[u8]) -> Result<Song, String> {
        let settings = SynthesizerSettings::new(SAMPLE_RATE as i32);
        let synthesizer = Synthesizer::new(font, &settings).map_err(|e| e.to_string())?;
        let file = MidiFile::new(&mut std::io::Cursor::new(midi)).map_err(|e| e.to_string())?;
        let mut sequencer = MidiFileSequencer::new(synthesizer);
        sequencer.play(&Arc::new(file), false);
        Ok(Song::new(sequencer))
    }

    fn new(sequencer: MidiFileSequencer) -> Song {
        Song {
            sequencer,
            left: vec![0.0; BLOCK],
            right: vec![0.0; BLOCK],
            index: 2 * BLOCK,
            tail: SAMPLE_RATE as usize / BLOCK,
        }
    }
}

impl Iterator for Song {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.index >= 2 * BLOCK {
            if self.sequencer.end_of_sequence() {
                if self.tail == 0 {
                    return None;
                }
                self.tail -= 1;
            }
            self.sequencer.render(&mut self.left, &mut self.right);
            self.index = 0;
        }
        let frame = self.index / 2;
        let sample = if self.index.is_multiple_of(2) {
            self.left[frame]
        } else {
            self.right[frame]
        };
        self.index += 1;
        Some(sample)
    }
}

impl rodio::Source for Song {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> rodio::ChannelCount {
        NonZero::new(2).expect("two channels")
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        NonZero::new(SAMPLE_RATE).expect("a rate")
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders Fux's first example through the system's SoundFont, without a sound card. Skipped
    /// without a SoundFont.
    #[test]
    fn a_score_sounds_and_ends() {
        let Some(path) = find_soundfont() else {
            return;
        };
        let font = Arc::new(
            SoundFont::new(&mut std::io::BufReader::new(
                std::fs::File::open(path).unwrap(),
            ))
            .unwrap(),
        );
        let example = fux_core::corpus::fux_examples().remove(0);
        let options = fux_export::MidiOptions {
            tempo: 240,
            ..fux_export::MidiOptions::default()
        };
        let midi = fux_export::to_midi(&example.score(), &options);
        let samples: Vec<f32> = Song::from_midi(&font, &midi).unwrap().collect();
        let seconds = samples.len() as f32 / 2.0 / SAMPLE_RATE as f32;
        let measures = example.score().measure_count() as f32;
        // Two half notes per measure at 240 halves a minute, and a second to die away.
        let expected = measures * 2.0 * 60.0 / 240.0 + 1.0;
        assert!(
            (seconds - expected).abs() < 0.5,
            "{seconds} s, not {expected} s"
        );
        let loudest = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(loudest > 0.01, "silent: {loudest}");
    }
}
