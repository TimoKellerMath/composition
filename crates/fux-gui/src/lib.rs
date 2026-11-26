//! The graphical interface of fux-kontrapunkt: sets counterpoint, generates cantus firmi, sets
//! masses and checks scores like the program `fux`, shows the result as notes (rendered by
//! LilyPond), plays it with a SoundFont and writes LilyPond, PDF, SVG, PNG, MIDI and MusicXML
//! files. All settings of the rules can be changed in a table.
//!
//! The work runs in background threads: the search, LilyPond and the loading of the SoundFont.
//! [`FuxApp::show`] draws the whole window; tests drive it without a display.

mod app;
mod audio;
mod forms;
mod jobs;
mod mass;
mod preview;
mod rules;

pub use app::{FuxApp, Tab};
pub use forms::{
    AnswerKind, CadenceChoice, CantusForm, CheckForm, CounterpointForm, CounterpointPreview,
    FreeForm, FreePreview, ImitationForm, LaterVoices, MassForm, NO_TEXT, OWN_TEXT, SectionChoice,
    SourceKind, SubjectKind, Switch, TextedScore,
};
pub use jobs::{CheckJob, Entry, PreparedMass};
pub use mass::{MassRun, PartReport, PartState};

/// Storage in memory, as eframe keeps it on disk; for tests.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct Memory(std::collections::HashMap<String, String>);

#[cfg(test)]
impl eframe::Storage for Memory {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }

    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }

    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }

    fn flush(&mut self) {}
}
