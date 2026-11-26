//! Fux's cantus firmi and all 123 examples of the *Gradus ad Parnassum*, embedded from
//! `tests/fixtures/fux/` (transcribed by M. Gotham from the print of 1725, CC0).

use serde::Deserialize;
use thiserror::Error;

use crate::clef::{Clef, VoiceType};
use crate::error::ParseError;
use crate::mode::{Mode, ModeSpec};
use crate::parse::{parse_lilypond_measures, parse_pitches};
use crate::pitch::{Pitch, Step};
use crate::score::{Note, Role, Score, Species, Voice};
use crate::time::{Duration, Meter};

const CANTUS_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/fux/cantus.toml"
));
const EXAMPLES_TOML: [&str; 3] = [
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/fux/gradus-2st.toml"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/fux/gradus-3st.toml"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/fux/gradus-4st.toml"
    )),
];

/// Errors in a corpus file.
#[derive(Debug, Error)]
pub enum CorpusError {
    #[error("invalid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("{id}: {source}")]
    Notes {
        id: String,
        #[source]
        source: ParseError,
    },
    #[error("{id}: {message}")]
    Invalid { id: String, message: String },
}

/// One of Fux's cantus firmi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cantus {
    pub id: String,
    pub mode: ModeSpec,
    pub notes: Vec<Pitch>,
    /// For variants: the id of the standard cantus they derive from.
    pub variant_of: Option<String>,
}

/// One example from the *Gradus*.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Example {
    /// Gotham's id, e.g. `gap_005`.
    pub id: String,
    /// Figure number in Mann's translation, e.g. `5` or `84a`.
    pub figure: String,
    /// Remark such as "corrected version".
    pub note: String,
    pub mode: ModeSpec,
    /// Species named in the source (for three and four parts: those of the moving voices).
    pub species: Vec<Species>,
    /// Index of the cantus firmus part, counted from the top (from 0).
    pub cf_part: usize,
    /// Id of the matching cantus firmus.
    pub cantus: String,
    /// Original clefs, top to bottom.
    pub clefs: Vec<Clef>,
    /// Page in the print of 1725.
    pub page: u32,
    /// Notes per part, top to bottom.
    pub parts: Vec<Vec<Note>>,
}

impl Example {
    /// The example as a score: voice types from the original clefs, the cantus firmus marked, the
    /// other voices with the species their rhythm shows.
    pub fn score(&self) -> Score {
        let voices = self
            .parts
            .iter()
            .enumerate()
            .map(|(i, notes)| {
                let role = if i == self.cf_part {
                    Role::CantusFirmus
                } else {
                    Role::Counterpoint(detect_species(notes, Meter::ALLA_BREVE))
                };
                Voice::new(VoiceType::from_clef(self.clefs[i]), role, notes.clone())
            })
            .collect();
        let mut score = Score::new(self.mode, voices);
        score.title = Some(format!("Fux, Gradus ad Parnassum, Fig. {}", self.figure));
        score
    }
}

/// Infers the species of a voice from its rhythm: eighths, or quarters together with
/// syncopations, mean the fifth species; regular syncopations the fourth; quarters the third;
/// half notes in most measures the second; otherwise the first (whole notes, a few of them
/// split, as in Fux's four-part examples).
pub fn detect_species(notes: &[Note], meter: Meter) -> Species {
    let per_measure = meter.ticks_per_measure();
    let mut tick = 0;
    let (mut eighths, mut quarters, mut ties_over_barline) = (false, false, 0);
    let mut measures_with_halves = std::collections::BTreeSet::new();
    for note in notes {
        let end = tick + note.duration.ticks();
        if !note.is_rest() {
            match note.duration {
                Duration::EIGHTH => eighths = true,
                Duration::QUARTER | Duration::DOTTED_QUARTER => quarters = true,
                Duration::HALF | Duration::DOTTED_HALF => {
                    measures_with_halves.insert(tick / per_measure);
                }
                _ => {}
            }
        }
        if note.tie && end % per_measure == 0 {
            ties_over_barline += 1;
        }
        tick = end;
    }
    let measures = tick.div_ceil(per_measure) as usize;
    match () {
        _ if eighths || (quarters && ties_over_barline > 0) => Species::Fifth,
        _ if ties_over_barline >= 2 => Species::Fourth,
        _ if quarters => Species::Third,
        _ if 2 * measures_with_halves.len() >= measures => Species::Second,
        _ => Species::First,
    }
}

#[derive(Deserialize)]
struct CantusFile {
    cantus: Vec<RawCantus>,
}

#[derive(Deserialize)]
struct RawCantus {
    id: String,
    mode: String,
    notes: String,
    variant_of: Option<String>,
}

#[derive(Deserialize)]
struct ExampleFile {
    example: Vec<RawExample>,
}

#[derive(Deserialize)]
struct RawExample {
    id: String,
    figure: String,
    note: String,
    #[serde(rename = "final")]
    final_step: String,
    species: Vec<u8>,
    cf_part: usize,
    cantus: String,
    clefs: Vec<String>,
    page: u32,
    parts: Vec<String>,
}

/// Parses a cantus file in the format of `tests/fixtures/fux/cantus.toml`.
pub fn parse_cantus(toml_text: &str) -> Result<Vec<Cantus>, CorpusError> {
    let file: CantusFile = toml::from_str(toml_text)?;
    file.cantus
        .into_iter()
        .map(|raw| {
            let invalid = |message: String| CorpusError::Invalid {
                id: raw.id.clone(),
                message,
            };
            let mode = raw
                .mode
                .parse()
                .map_err(|e: ParseError| invalid(e.to_string()))?;
            let notes = parse_pitches(&raw.notes).map_err(|source| CorpusError::Notes {
                id: raw.id.clone(),
                source,
            })?;
            Ok(Cantus {
                id: raw.id,
                mode,
                notes,
                variant_of: raw.variant_of,
            })
        })
        .collect()
}

/// Parses an example file in the format of `tests/fixtures/fux/gradus-2st.toml`.
pub fn parse_examples(toml_text: &str) -> Result<Vec<Example>, CorpusError> {
    let file: ExampleFile = toml::from_str(toml_text)?;
    file.example.into_iter().map(convert_example).collect()
}

fn convert_example(raw: RawExample) -> Result<Example, CorpusError> {
    let invalid = |message: String| CorpusError::Invalid {
        id: raw.id.clone(),
        message,
    };
    let final_step = raw
        .final_step
        .chars()
        .next()
        .and_then(Step::from_letter)
        .ok_or_else(|| invalid(format!("unknown final {:?}", raw.final_step)))?;
    let mode = Mode::from_final(final_step)
        .map(ModeSpec::new)
        .ok_or_else(|| invalid(format!("no mode on {final_step:?}")))?;
    let species = raw
        .species
        .iter()
        .map(|&n| Species::from_number(n).ok_or_else(|| invalid(format!("species {n}"))))
        .collect::<Result<_, _>>()?;
    let clefs = raw
        .clefs
        .iter()
        .map(|c| c.parse().map_err(|e: ParseError| invalid(e.to_string())))
        .collect::<Result<Vec<Clef>, _>>()?;
    let parts = raw
        .parts
        .iter()
        .map(|music| {
            parse_lilypond_measures(music, Duration::WHOLE, Meter::ALLA_BREVE).map_err(|source| {
                CorpusError::Notes {
                    id: raw.id.clone(),
                    source,
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if clefs.len() != parts.len() || raw.cf_part == 0 || raw.cf_part > parts.len() {
        return Err(invalid(
            "parts, clefs and cf_part do not fit together".into(),
        ));
    }
    Ok(Example {
        figure: raw.figure,
        note: raw.note,
        mode,
        species,
        cf_part: raw.cf_part - 1,
        cantus: raw.cantus,
        clefs,
        page: raw.page,
        parts,
        id: raw.id,
    })
}

/// Fux's cantus firmi: the six standard ones and two variants.
pub fn fux_cantus() -> Vec<Cantus> {
    parse_cantus(CANTUS_TOML).expect("embedded cantus.toml is valid")
}

/// All 123 examples of the *Gradus*: 46 for two, 45 for three and 32 for four parts.
pub fn fux_examples() -> Vec<Example> {
    EXAMPLES_TOML
        .iter()
        .flat_map(|text| parse_examples(text).expect("embedded example files are valid"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn species_detection() {
        let notes = |s: &str| crate::parse::parse_lilypond(s, Duration::WHOLE).unwrap();
        let meter = Meter::ALLA_BREVE;
        assert_eq!(detect_species(&notes("d'1 e'1"), meter), Species::First);
        assert_eq!(
            detect_species(&notes("r2 d'2 e'2 f'2"), meter),
            Species::Second
        );
        assert_eq!(
            detect_species(&notes("d'4 e' f' g' a'1"), meter),
            Species::Third
        );
        assert_eq!(
            detect_species(&notes("r2 d'2~ d'2 e'2~ e'2 f'2 g'1"), meter),
            Species::Fourth
        );
        assert_eq!(
            detect_species(&notes("r2 d'2~ d'4 e'8 f' g'4 a' b'1"), meter),
            Species::Fifth
        );
    }
}
