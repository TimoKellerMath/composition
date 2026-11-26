//! Building a score from command-line arguments, a job file or Fux's examples.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, anyhow, bail};
use clap::Args;
use fux_core::corpus::{detect_species, fux_cantus, fux_examples};
use fux_core::gabc::{CadenceBars, Chant, parse_gabc};
use fux_core::mass::FIELD_PART;
use fux_core::text::{TextOptions, resolve};
use fux_core::{
    Cadence, Duration, Lang, Mode, ModeSpec, Note, Pitch, Role, Score, ScoreError, ScoreImitation,
    SectionMark, Species, Step, Voice, VoiceType, parse_notes,
};
use fux_export::{
    FIELD_CADENCES, FIELD_MODE, FIELD_TEXT, ImportedPart, ImportedScore, from_musicxml,
};
use serde::Deserialize;

/// Where internal cadences go: at the bars of a chant, or in the given measures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cadences {
    Bars(CadenceBars),
    /// Measures from 1 in which a cadence arrives, with its target if given; otherwise the
    /// target is the note of the cantus there (in free counterpoint of the lowest voice).
    Measures(Vec<(usize, Option<Step>)>),
}

impl Default for Cadences {
    fn default() -> Self {
        Cadences::Bars(CadenceBars::Half)
    }
}

impl FromStr for Cadences {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(match text.trim() {
            "auto" | "half" | "halb" => Cadences::Bars(CadenceBars::Half),
            "full" | "ganz" => Cadences::Bars(CadenceBars::Full),
            "all" | "alle" => Cadences::Bars(CadenceBars::Quarter),
            "none" | "keine" => Cadences::Bars(CadenceBars::None),
            list => Cadences::Measures(
                list.split(',')
                    .map(|item| {
                        let (measure, target) = match item.split_once([':', '=']) {
                            Some((m, t)) => (m, Some(t.trim())),
                            None => (item, None),
                        };
                        let measure = measure.trim().parse::<usize>().ok().filter(|&m| m >= 1)?;
                        let target = match target {
                            Some(t) => {
                                let mut chars = t.chars();
                                let step = Step::from_letter(chars.next()?)?;
                                // A flat after the letter is the mode's own (B♭).
                                Some(matches!(chars.as_str(), "" | "b" | "♭").then_some(step)?)
                            }
                            None => None,
                        };
                        Some((measure, target))
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        format!(
                            "{text:?}: auto, full, all, none or measures such as 8,15 or 8:A,15:F"
                        )
                    })?,
            ),
        })
    }
}

/// Where the score comes from.
#[derive(Args, Debug, Default)]
pub struct ScoreInput {
    /// Fux-Beispiel nach Figurennummer, z. B. 5 oder 84a (siehe `fux corpus`)
    #[arg(long, value_name = "FIG")]
    pub example: Option<String>,
    /// Jobdatei (TOML) mit Titel, Modus und Stimmen
    #[arg(long, value_name = "DATEI")]
    pub job: Option<PathBuf>,
    /// Satz aus einer MusicXML-Datei; Cantus ist die Stimme „C. f.“ oder „Cantus“, sonst die erste
    /// in Ganzen
    #[arg(long, value_name = "DATEI")]
    pub musicxml: Option<PathBuf>,
    /// Satz im freien Satz aus einer Humdrum-Datei (**kern), z. B. aus Palestrinas Messen
    #[arg(long, value_name = "DATEI")]
    pub kern: Option<PathBuf>,
    /// Modus: dorisch, phrygisch, lydisch, mixolydisch, äolisch, ionisch; mit Vorsilbe hypo-
    /// für plagal und Endung -transponiert für ein ♭ (sonst aus der Finalis erschlossen)
    #[arg(long)]
    pub mode: Option<ModeSpec>,
    /// Cantus firmus: Notennamen ("D4 F4 E4"), LilyPond ("d'1 f' e'"), fux:NAME
    /// (fux:dorian, fux:phrygian, …, siehe `fux corpus --cantus`), GABC- oder MusicXML-Datei
    #[arg(long, value_name = "NOTEN")]
    pub cf: Option<String>,
    /// Stimmlage des Cantus firmus: S, A, T, B, Mz, Bar oder S+ (sonst nach Tonumfang)
    #[arg(long, value_name = "TYP")]
    pub cf_voice: Option<VoiceType>,
    /// Binnenschlüsse: auto (bei GABC an Halb- und Ganzstrichen, bei MusicXML von fux-kontrapunkt
    /// die der Datei), full (nur Ganzstriche), all (auch Viertelstriche), none, oder Takte wie 8,15
    /// oder mit Ziel 8:A,15:F
    #[arg(long, value_name = "WO", default_value = "auto")]
    pub cadences: Cadences,
    /// Weitere Stimme als TYP=NOTEN, z. B. "S=a'1 a' g'"; mehrfach angebbar
    #[arg(long = "voice", value_name = "TYP=NOTEN")]
    pub voices: Vec<String>,
    /// Titel über dem Satz
    #[arg(long)]
    pub title: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Job {
    title: Option<String>,
    mode: Option<String>,
    /// Measures (from 1) in which internal cadences arrive.
    cadences: Option<Vec<usize>>,
    voice: Vec<JobVoice>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JobVoice {
    #[serde(rename = "type")]
    voice_type: String,
    notes: String,
    #[serde(default)]
    cf: bool,
    name: Option<String>,
    species: Option<u8>,
}

/// A score as read, with what a MusicXML file written by fux-kontrapunkt keeps for its check
/// besides the notes.
#[derive(Clone, Debug)]
pub struct ReadScore {
    pub score: Score,
    /// The text its lyrics sing ([`fux_export::FIELD_TEXT`]), if the file names one that can be
    /// read.
    pub text: Option<String>,
    /// The plan of its imitation ([`fux_export::FIELD_IMITATION`]): `None` without one, an error
    /// if it cannot be read. It is not yet in the score: whether it fits the score shows once the
    /// lyrics have their text ([`crate::imitation::attach`]).
    pub imitation: Option<Result<ScoreImitation, ScoreError>>,
}

impl ScoreInput {
    /// Builds and validates the score.
    pub fn score(&self, lang: Lang) -> Result<Score> {
        Ok(self.score_and_text(lang)?.0)
    }

    /// Builds and validates the score, with the text its lyrics sing if the file names it: a
    /// MusicXML file written by fux-kontrapunkt keeps it ([`fux_export::FIELD_TEXT`]).
    pub fn score_and_text(&self, lang: Lang) -> Result<(Score, Option<String>)> {
        let read = self.read(lang)?;
        Ok((read.score, read.text))
    }

    /// Builds and validates the score, with what a MusicXML file of fux-kontrapunkt keeps for
    /// its check: the text of its lyrics and the plan of its imitation.
    pub fn read(&self, lang: Lang) -> Result<ReadScore> {
        Ok(self.read_with_fields(lang)?.0)
    }

    /// Builds and validates the score as [`ScoreInput::read`] does, with all the
    /// `miscellaneous-field`s of a MusicXML file as (name, value), such as those of a part of a
    /// mass (`fux-mass`, `fux-part`); none for other input.
    pub fn read_with_fields(&self, lang: Lang) -> Result<(ReadScore, Vec<(String, String)>)> {
        let mut stored = None;
        let mut imitation = None;
        let mut fields = Vec::new();
        let mut score = if let Some(figure) = &self.example {
            example_score(figure, lang)?
        } else if let Some(path) = &self.job {
            let text = std::fs::read_to_string(path).with_context(|| {
                format!(
                    "{}: {}",
                    path.display(),
                    lang.pick("nicht lesbar", "not readable")
                )
            })?;
            let dir = path.parent().unwrap_or(Path::new("."));
            job_score(&text, dir, self.mode, &self.cadences, lang).with_context(|| {
                format!("{} {}", lang.pick("Jobdatei", "Job file"), path.display())
            })?
        } else if let Some(path) = &self.musicxml {
            let text = std::fs::read_to_string(path).with_context(|| {
                format!(
                    "{}: {}",
                    path.display(),
                    lang.pick("nicht lesbar", "not readable")
                )
            })?;
            let (read, kept) = musicxml_score(&text, self.mode, &self.cadences, lang)
                .with_context(|| path.display().to_string())?;
            stored = read.text;
            imitation = read.imitation;
            fields = kept;
            read.score
        } else if let Some(path) = &self.kern {
            let text = std::fs::read_to_string(path).with_context(|| {
                format!(
                    "{}: {}",
                    path.display(),
                    lang.pick("nicht lesbar", "not readable")
                )
            })?;
            kern_text_score(&text, self.mode).with_context(|| path.display().to_string())?
        } else {
            self.argument_score(lang)?
        };
        if let Some(title) = &self.title {
            score.title = Some(title.clone());
        }
        let read = ReadScore {
            score: validated(score, lang)?,
            text: stored,
            imitation,
        };
        Ok((read, fields))
    }

    fn argument_score(&self, lang: Lang) -> Result<Score> {
        let mut parts = Vec::new();
        let mut mode = self.mode;
        let mut chant = None;
        let mut title = None;
        if let Some(cf) = &self.cf {
            let cantus = cantus_notes(cf, Path::new("."), lang)?;
            mode = mode.or(cantus.mode);
            chant = cantus.chant;
            title = cantus.title;
            parts.push(Part {
                voice_type: self.cf_voice,
                notes: cantus.notes,
                cf: true,
                name: None,
                species: None,
            });
        }
        for spec in &self.voices {
            let (voice_type, notes) = spec.split_once('=').ok_or_else(|| match lang {
                Lang::De => anyhow!("Stimme {spec:?}: erwartet TYP=NOTEN, z. B. \"S=a'1 g'\""),
                Lang::En => anyhow!("Voice {spec:?}: expected TYPE=NOTES, e.g. \"S=a'1 g'\""),
            })?;
            let voice_type: VoiceType = voice_type.parse()?;
            parts.push(Part {
                voice_type: Some(voice_type),
                notes: parse_part(notes)?,
                cf: false,
                name: None,
                species: None,
            });
        }
        if parts.is_empty() {
            bail!(
                "{}",
                lang.pick(
                    "Keine Noten angegeben: --cf, --voice, --job oder --example",
                    "No notes given: --cf, --voice, --job or --example"
                )
            );
        }
        let mut score = assemble(mode, parts, lang);
        score.title = title;
        add_cadences(&mut score, chant.as_ref(), &self.cadences);
        Ok(score)
    }
}

/// Marks the internal cadences: at the bars of a chant, or in the given measures on the note of
/// the cantus there (in free counterpoint on the note of the lowest voice).
pub fn add_cadences(score: &mut Score, chant: Option<&Chant>, cadences: &Cadences) {
    let given = if score.is_free() {
        score.voices.last()
    } else {
        score.voices.iter().find(|v| v.is_cantus_firmus())
    };
    let Some(cantus) = given else {
        return;
    };
    score.cadences = match cadences {
        Cadences::Bars(bars) => chant.map_or_else(Vec::new, |c| c.cadences(*bars, score.mode)),
        Cadences::Measures(measures) => {
            let per = score.meter.ticks_per_measure();
            measures
                .iter()
                .filter_map(|&(m, target)| {
                    let target = match target {
                        Some(target) => target,
                        None => {
                            let start = (m as u32 - 1) * per;
                            let (_, note) = cantus.timed().find(|(t, _)| *t == start)?;
                            note.pitch?.step
                        }
                    };
                    Some(Cadence {
                        measure: m - 1,
                        target,
                    })
                })
                .collect()
        }
    };
    if chant.is_some() && score.title.is_none() {
        score.title = chant.and_then(|c| c.name.clone());
    }
}

/// One voice before the score is assembled.
struct Part {
    voice_type: Option<VoiceType>,
    notes: Vec<Note>,
    cf: bool,
    name: Option<String>,
    species: Option<Species>,
}

fn parse_part(text: &str) -> Result<Vec<Note>> {
    Ok(parse_notes(
        text,
        Duration::WHOLE,
        fux_core::Meter::ALLA_BREVE,
    )?)
}

/// A score with only the cantus firmus: notes, voice type (from the range if not given) and
/// mode (Fux's, the given one, or from the final).
pub fn cantus_score(
    text: &str,
    mode: Option<ModeSpec>,
    voice_type: Option<VoiceType>,
    cadences: &Cadences,
    lang: Lang,
) -> Result<Score> {
    let CantusInput {
        mut notes,
        mode: cantus_mode,
        chant,
        title,
    } = cantus_notes(text, Path::new("."), lang)?;
    // As in Fux, the cantus goes to the alto if it fits there; that leaves room above and below.
    let pitches: Vec<Pitch> = notes.iter().filter_map(|n| n.pitch).collect();
    let voice_type =
        voice_type.or_else(|| (VoiceType::Alto.misfit(&pitches) == 0).then_some(VoiceType::Alto));
    // A cantus given for a voice moves by octaves into its range.
    if let Some(voice_type) = voice_type {
        let shift = voice_type.octave_shift_for(&pitches) as i8;
        for note in &mut notes {
            if let Some(p) = note.pitch {
                note.pitch = Some(Pitch::new(p.step, p.alter, p.octave + shift));
            }
        }
    }
    let part = Part {
        voice_type,
        notes,
        cf: true,
        name: None,
        species: None,
    };
    let mut score = assemble(mode.or(cantus_mode), vec![part], lang);
    score.title = title;
    add_cadences(&mut score, chant.as_ref(), cadences);
    if let Err(error) = score.validate() {
        bail!("{}", error.message(lang, score.meter));
    }
    Ok(score)
}

/// A cantus firmus as given: its notes, its mode if it is one of Fux's or a chant's or named in
/// MusicXML, the chant it comes from, and a title.
struct CantusInput {
    notes: Vec<Note>,
    mode: Option<ModeSpec>,
    chant: Option<Chant>,
    title: Option<String>,
}

/// Reads a cantus firmus: notes, LilyPond, `fux:NAME`, a GABC or a MusicXML file (relative to
/// `dir`).
fn cantus_notes(text: &str, dir: &Path, lang: Lang) -> Result<CantusInput> {
    let lower = text.to_ascii_lowercase();
    let read = |path: &Path| {
        std::fs::read_to_string(path).with_context(|| {
            format!(
                "{}: {}",
                path.display(),
                lang.pick("nicht lesbar", "not readable")
            )
        })
    };
    if lower.ends_with(".gabc") {
        let path = dir.join(text);
        let chant = parse_gabc(&read(&path)?).map_err(|e| anyhow!("{}: {e}", path.display()))?;
        let mode = chant
            .mode()
            .map_err(|e| anyhow!("{}: {e}", path.display()))?;
        return Ok(CantusInput {
            notes: chant.cantus_notes(),
            mode: Some(mode),
            title: chant.name.clone(),
            chant: Some(chant),
        });
    }
    if lower.ends_with(".musicxml") || lower.ends_with(".xml") {
        let path = dir.join(text);
        let imported =
            from_musicxml(&read(&path)?).map_err(|e| anyhow!("{}: {e}", path.display()))?;
        let part = cantus_part(&imported.parts)
            .or((!imported.parts.is_empty()).then_some(0))
            .map(|i| &imported.parts[i])
            .ok_or_else(|| {
                anyhow!(
                    "{}: {}",
                    path.display(),
                    lang.pick("keine Stimme", "no part")
                )
            })?;
        return Ok(CantusInput {
            notes: whole_notes(&part.notes),
            mode: imported_mode(&imported),
            chant: None,
            title: imported.title.clone(),
        });
    }
    let (notes, mode) = fux_or_notes(text, lang)?;
    Ok(CantusInput {
        notes,
        mode,
        chant: None,
        title: None,
    })
}

/// The part of the cantus firmus in MusicXML: the one named "C. f." or "Cantus", otherwise the
/// first in whole notes.
fn cantus_part(parts: &[ImportedPart]) -> Option<usize> {
    let named = parts.iter().position(|p| {
        let name = p.name.to_lowercase();
        name.contains("c. f.") || name.contains("c.f.") || name.contains("cantus")
    });
    named.or_else(|| {
        parts.iter().position(|p| {
            p.notes.iter().any(|n| !n.is_rest())
                && p.notes
                    .iter()
                    .all(|n| n.is_rest() || n.duration == Duration::WHOLE)
        })
    })
}

/// The mode named in the key of a MusicXML score, transposed with one flat.
fn imported_mode(imported: &ImportedScore) -> Option<ModeSpec> {
    imported
        .mode
        .map(|m| ModeSpec::new(m).with_transposition(imported.fifths == -1))
}

/// The pitches of a melody as whole notes, as a cantus firmus: tied notes sound once, rests are
/// left out.
fn whole_notes(notes: &[Note]) -> Vec<Note> {
    let mut whole = Vec::new();
    let mut tied = false;
    for note in notes {
        if note.pitch.is_some() && !tied {
            whole.push(Note {
                duration: Duration::WHOLE,
                tie: false,
                ..note.clone()
            });
        }
        tied = note.tie;
    }
    whole
}

/// Notes of a cantus firmus, and its mode if it is one of Fux's.
fn fux_or_notes(text: &str, lang: Lang) -> Result<(Vec<Note>, Option<ModeSpec>)> {
    match text.strip_prefix("fux:") {
        Some(id) => {
            let cantus = fux_cantus();
            let found = cantus.iter().find(|c| c.id == id).ok_or_else(|| {
                let ids: Vec<&str> = cantus.iter().map(|c| c.id.as_str()).collect();
                anyhow!(
                    "{} fux:{id}; {}: {}",
                    lang.pick("Unbekannter Cantus", "Unknown cantus"),
                    lang.pick("vorhanden", "available"),
                    ids.join(", ")
                )
            })?;
            let notes = found
                .notes
                .iter()
                .map(|&p| Note::new(p, Duration::WHOLE))
                .collect();
            Ok((notes, Some(found.mode)))
        }
        None => Ok((parse_part(text)?, None)),
    }
}

fn job_score(
    text: &str,
    dir: &Path,
    mode_override: Option<ModeSpec>,
    cadences: &Cadences,
    lang: Lang,
) -> Result<Score> {
    let job: Job = toml::from_str(text)?;
    let mut mode = match &job.mode {
        Some(m) => Some(m.parse::<ModeSpec>()?),
        None => None,
    };
    let mut parts = Vec::new();
    let mut chant = None;
    let mut title = job.title;
    for v in job.voice {
        let (notes, cantus_mode) = if v.cf {
            let cantus = cantus_notes(&v.notes, dir, lang)?;
            chant = chant.or(cantus.chant);
            title = title.or(cantus.title);
            (cantus.notes, cantus.mode)
        } else {
            (parse_part(&v.notes)?, None)
        };
        mode = mode.or(cantus_mode);
        // 0 is free counterpoint, as `Species::number` counts it.
        let species = match v.species {
            Some(0) => Some(Species::Free),
            Some(n) => Some(Species::from_number(n).ok_or_else(|| {
                anyhow!("{} {n}", lang.pick("Unbekannte Gattung", "Unknown species"))
            })?),
            None => None,
        };
        parts.push(Part {
            voice_type: Some(v.voice_type.parse()?),
            notes,
            cf: v.cf,
            name: v.name,
            species,
        });
    }
    let mut score = assemble(mode_override.or(mode), parts, lang);
    score.title = title;
    let cadences = match job.cadences {
        Some(measures) => Cadences::Measures(measures.into_iter().map(|m| (m, None)).collect()),
        None => cadences.clone(),
    };
    add_cadences(&mut score, chant.as_ref(), &cadences);
    Ok(score)
}

/// A score from MusicXML: voice types from the part names or Fux's clefs (otherwise from the
/// range), the cantus by its name or as the first part in whole notes, the mode from the key or
/// the final. What fux-kontrapunkt keeps in its own files counts unless given otherwise: the
/// mode with its ambitus ([`fux_export::FIELD_MODE`]) and, with the cadences `auto`, the
/// internal cadences ([`fux_export::FIELD_CADENCES`]). Returns the score (not yet validated)
/// with the text the file names for its lyrics ([`fux_export::FIELD_TEXT`]), if it is one that
/// can be read, and the plan of its imitation ([`fux_export::FIELD_IMITATION`]); and all the
/// fields of the file.
fn musicxml_score(
    text: &str,
    mode: Option<ModeSpec>,
    cadences: &Cadences,
    lang: Lang,
) -> Result<(ReadScore, Vec<(String, String)>)> {
    let imported = from_musicxml(text)?;
    let fields = imported.fields.clone();
    let imitation = imported.imitation();
    let stored_mode = imported
        .field(FIELD_MODE)
        .and_then(|m| m.parse::<ModeSpec>().ok());
    let mode = mode.or(stored_mode).or_else(|| imported_mode(&imported));
    let stored_cadences = imported.field(FIELD_CADENCES).and_then(stored_cadences);
    let stored_text = imported
        .field(FIELD_TEXT)
        .filter(|spec| resolve(spec, &TextOptions::default()).is_ok())
        .map(str::to_string);
    let (title, meter) = (imported.title.clone(), imported.meter);
    let sections = kept_sections(&imported);
    let cantus = cantus_part(&imported.parts);
    // Around a cantus only free voices sing a text (`fux cp --species 0`): in a file of
    // fux-kontrapunkt that names the text of its lyrics, the voices besides the cantus are free.
    let free = cantus.is_some() && imported.field(FIELD_TEXT).is_some();
    let parts = imported
        .parts
        .into_iter()
        .enumerate()
        .map(|(i, part)| {
            let cf = Some(i) == cantus;
            // "Tenor 2 (C. f.)" → "Tenor".
            let base: String = part
                .name
                .split(['(', ','])
                .next()
                .unwrap_or_default()
                .trim()
                .trim_end_matches(|c: char| c.is_ascii_digit() || c.is_whitespace())
                .to_string();
            let voice_type = base.parse::<VoiceType>().ok().or_else(|| {
                part.clef
                    .filter(|c| *c != fux_core::Clef::G2)
                    .map(VoiceType::from_clef)
            });
            Part {
                voice_type,
                notes: part.notes,
                cf,
                name: (!part.name.is_empty())
                    .then(|| part.name.replace(" (C. f.)", "").trim().to_string()),
                species: (free && !cf).then_some(Species::Free),
            }
        })
        .collect();
    let mut score = assemble(mode, parts, lang);
    score.meter = meter;
    score.title = title;
    match stored_cadences {
        Some(stored) if *cadences == Cadences::default() => score.cadences = stored,
        _ => add_cadences(&mut score, None, cadences),
    }
    // Marks of sections that do not fit the measures of the score leave it a single score, as
    // it was read before phase 14, rather than make it unreadable.
    if !sections.is_empty() {
        score.sections = sections;
        if matches!(
            score.validate(),
            Err(ScoreError::SectionOutside { .. } | ScoreError::SectionOrder { .. })
        ) {
            score.sections.clear();
        }
    }
    let read = ReadScore {
        score,
        text: stored_text,
        imitation,
    };
    Ok((read, fields))
}

/// The beginning of the name of every `miscellaneous-field` that fux-kontrapunkt writes
/// ([`FIELD_MODE`], [`FIELD_TEXT`], [`FIELD_PART`], …).
const FUX_FIELDS: &str = "fux-";

/// The sections of a MusicXML file that its score keeps ([`Score::sections`], phase 14), as the
/// reader finds them ([`ImportedScore::sections`]): those of the file of a mass movement, whose
/// fields name its parts (`fux-part.1`, …), and those of a file without the fields of
/// fux-kontrapunkt in two sections or more, from its rehearsal marks after double bars, each of
/// which `fux check` checks on its own (§ 6). Any other file is a single score, as before phase
/// 14 (D15): a file with one section at most, and a file that fux-kontrapunkt wrote as a single
/// setting or as the part of a mass, with its fields (`fux-mode`, `fux-text`, `fux-cadences`,
/// `fux-imitation`, `fux-part`), whatever marks a program has added to its notation since.
fn kept_sections(imported: &ImportedScore) -> Vec<SectionMark> {
    let parts = format!("{FIELD_PART}.");
    let movement = imported
        .fields
        .iter()
        .any(|(name, _)| name.starts_with(&parts));
    let foreign = !imported
        .fields
        .iter()
        .any(|(name, _)| name.starts_with(FUX_FIELDS));
    if movement || (foreign && imported.sections.len() > 1) {
        imported.sections.clone()
    } else {
        Vec::new()
    }
}

/// The internal cadences as [`fux_export::FIELD_CADENCES`] keeps them: `13:A,22:F`, the
/// measure from 1 and the target. `None` if one cannot be read.
fn stored_cadences(field: &str) -> Option<Vec<Cadence>> {
    field
        .split(',')
        .filter(|c| !c.trim().is_empty())
        .map(|c| {
            let (measure, target) = c.trim().split_once(':')?;
            let measure: usize = measure.trim().parse().ok().filter(|&m| m >= 1)?;
            let letter = target.trim().chars().next()?.to_ascii_uppercase();
            let target = Step::ALL.into_iter().find(|s| s.letter() == letter)?;
            Some(Cadence {
                measure: measure - 1,
                target,
            })
        })
        .collect()
}

/// A score in free counterpoint from the text of a Humdrum `**kern` file; a mode given overrides
/// the one of the file.
pub fn kern_text_score(text: &str, mode: Option<ModeSpec>) -> Result<Score> {
    let mut score = fux_core::kern::parse_kern(text)?.score;
    if let Some(mode) = mode {
        score.mode = mode;
    }
    Ok(score)
}

/// A score from the text of a MusicXML file, as `--musicxml` reads it (e.g. for the web version,
/// which has no files), with the text the file names for its lyrics.
pub fn musicxml_text_score(
    text: &str,
    mode: Option<ModeSpec>,
    lang: Lang,
) -> Result<(Score, Option<String>)> {
    let read = musicxml_text_read(text, mode, lang)?;
    Ok((read.score, read.text))
}

/// A score from the text of a MusicXML file as [`musicxml_text_score`] reads it, with the plan
/// of its imitation, if the file keeps one.
pub fn musicxml_text_read(text: &str, mode: Option<ModeSpec>, lang: Lang) -> Result<ReadScore> {
    Ok(musicxml_text_fields(text, mode, lang)?.0)
}

/// A score from the text of a MusicXML file as [`musicxml_text_read`] reads it, with all the
/// `miscellaneous-field`s of the file as (name, value), such as those of a part of a mass.
pub fn musicxml_text_fields(
    text: &str,
    mode: Option<ModeSpec>,
    lang: Lang,
) -> Result<(ReadScore, Vec<(String, String)>)> {
    let (read, fields) = musicxml_score(text, mode, &Cadences::default(), lang)?;
    let read = ReadScore {
        score: validated(read.score, lang)?,
        ..read
    };
    Ok((read, fields))
}

/// The score if its voices fit together.
fn validated(score: Score, lang: Lang) -> Result<Score> {
    if let Err(error) = score.validate() {
        bail!(
            "{}: {}",
            lang.pick(
                "Die Stimmen passen nicht zusammen",
                "The voices do not fit together"
            ),
            error.message(lang, score.meter)
        );
    }
    Ok(score)
}

/// Orders the voices from high to low, fills in voice types and species, and finds the mode.
/// Two or more voices without a cantus firmus are free counterpoint.
fn assemble(mode: Option<ModeSpec>, parts: Vec<Part>, lang: Lang) -> Score {
    let all_pitches: Vec<Pitch> = parts
        .iter()
        .flat_map(|p| p.notes.iter().filter_map(|n| n.pitch))
        .collect();
    let mode = mode.unwrap_or_else(|| guess_mode(&parts, &all_pitches));
    let free = parts.len() >= 2 && !parts.iter().any(|p| p.cf);
    let mut voices: Vec<Voice> = parts
        .into_iter()
        .map(|part| {
            let pitches: Vec<Pitch> = part.notes.iter().filter_map(|n| n.pitch).collect();
            let voice_type = part
                .voice_type
                .unwrap_or_else(|| VoiceType::best_for(&pitches));
            let role = if part.cf {
                Role::CantusFirmus
            } else {
                Role::Counterpoint(part.species.unwrap_or_else(|| {
                    if free {
                        Species::Free
                    } else {
                        detect_species(&part.notes, fux_core::Meter::ALLA_BREVE)
                    }
                }))
            };
            let mut voice = Voice::new(voice_type, role, part.notes);
            voice.name = part
                .name
                .unwrap_or_else(|| voice_type.name(lang).to_string());
            voice
        })
        .collect();
    voices.sort_by_key(|v| v.voice_type);
    Score::new(mode, voices)
}

/// The mode from the last pitch of the cantus firmus (or of the lowest voice): its step is the
/// final. A final B♭ means the transposed Lydian.
fn guess_mode(parts: &[Part], all_pitches: &[Pitch]) -> ModeSpec {
    let last = parts
        .iter()
        .find(|p| p.cf)
        .or_else(|| parts.last())
        .and_then(|p| p.notes.iter().rev().find_map(|n| n.pitch))
        .or_else(|| all_pitches.last().copied());
    match last {
        Some(p) if p.step == Step::B && p.alter == -1 => {
            ModeSpec::new(Mode::Lydian).with_transposition(true)
        }
        Some(p) => Mode::from_final(p.step)
            .map(ModeSpec::new)
            .unwrap_or(ModeSpec::new(Mode::Dorian)),
        None => ModeSpec::new(Mode::Dorian),
    }
}

fn example_score(figure: &str, lang: Lang) -> Result<Score> {
    let examples = fux_examples();
    let example = examples
        .iter()
        .find(|e| e.figure.eq_ignore_ascii_case(figure) || e.id == figure)
        .ok_or_else(|| match lang {
            Lang::De => anyhow!("Kein Beispiel Fig. {figure} (siehe `fux corpus`)"),
            Lang::En => anyhow!("No example Fig. {figure} (see `fux corpus`)"),
        })?;
    Ok(example.score())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_files() {
        let job = r#"
            title = "Übung"
            [[voice]]
            type = "S"
            notes = "a'1 a' g' a'"
            [[voice]]
            type = "A"
            cf = true
            notes = "D4 F4 E4 D4"
        "#;
        let score = job_score(job, Path::new("."), None, &Cadences::default(), Lang::De).unwrap();
        assert_eq!(score.title.as_deref(), Some("Übung"));
        assert_eq!(score.mode, ModeSpec::new(Mode::Dorian));
        assert_eq!(score.voices[0].voice_type, VoiceType::Soprano);
        assert_eq!(score.voices[1].role, Role::CantusFirmus);
        assert_eq!(score.voices[0].role, Role::Counterpoint(Species::First));
    }

    #[test]
    fn fux_cantus_sets_the_mode() {
        let (notes, mode) = fux_or_notes("fux:phrygian", Lang::De).unwrap();
        assert_eq!(notes.len(), 10);
        assert_eq!(mode, Some(ModeSpec::new(Mode::Phrygian)));
        assert!(fux_or_notes("fux:locrian", Lang::De).is_err());
    }

    #[test]
    fn voices_are_ordered_and_typed() {
        let input = ScoreInput {
            cf: Some("D4 F4 E4 D4".into()),
            voices: vec!["S=a'1 a' g' a'".into()],
            ..ScoreInput::default()
        };
        let score = input.score(Lang::De).unwrap();
        // The cantus goes to the lowest standard voice whose range holds it.
        let types: Vec<VoiceType> = score.voices.iter().map(|v| v.voice_type).collect();
        assert_eq!(types, [VoiceType::Soprano, VoiceType::Tenor]);
        assert_eq!(score.voices[1].name, "Tenor");
    }

    #[test]
    fn mismatched_lengths_are_reported() {
        let input = ScoreInput {
            cf: Some("D4 F4 E4 D4".into()),
            voices: vec!["S=a'1 a'".into()],
            ..ScoreInput::default()
        };
        let message = format!("{:#}", input.score(Lang::De).unwrap_err());
        assert_eq!(
            message,
            "Die Stimmen passen nicht zusammen: Stimme 2 dauert 4 Takte, Stimme 1 aber 2 Takte."
        );
        let message = format!("{:#}", input.score(Lang::En).unwrap_err());
        assert!(
            message.starts_with("The voices do not fit together"),
            "{message}"
        );
    }

    /// A file keeps the sections the reader finds where it is the file of a mass movement, which
    /// names its parts, or has two sections or more and none of the fields of fux-kontrapunkt; a
    /// single rehearsal mark at its start, and any marks in a single setting or a part that
    /// fux-kontrapunkt wrote, leave it a single score, as before phase 14.
    #[test]
    fn sections_of_a_file() {
        let mark = |measure| SectionMark {
            measure,
            kind: fux_core::SectionKind::Section,
            title: Some("A".into()),
        };
        let imported = |fields: Vec<(String, String)>, sections| ImportedScore {
            title: None,
            meter: fux_core::Meter::ALLA_BREVE,
            mode: None,
            fifths: 0,
            parts: Vec::new(),
            fields,
            sections,
        };
        assert!(kept_sections(&imported(Vec::new(), vec![mark(0)])).is_empty());
        let two = vec![mark(0), mark(4)];
        assert_eq!(kept_sections(&imported(Vec::new(), two.clone())), two);
        let part = vec![("fux-part.1".to_string(), "bars=1-8".to_string())];
        assert_eq!(kept_sections(&imported(part, vec![mark(0)])), [mark(0)]);
        let single = vec![("fux-part".to_string(), "bars=1-8".to_string())];
        assert!(kept_sections(&imported(single.clone(), vec![mark(0)])).is_empty());
        assert!(kept_sections(&imported(single, two.clone())).is_empty());
        // A single setting of fux-kontrapunkt, also one with no field but its mode.
        for name in [FIELD_MODE, FIELD_TEXT, FIELD_CADENCES, "fux-imitation"] {
            let fields = vec![(name.to_string(), "dorian".to_string())];
            assert!(
                kept_sections(&imported(fields, two.clone())).is_empty(),
                "{name}"
            );
        }
        // Fields of another program leave a file foreign.
        let other = vec![("description".to_string(), "Kyrie".to_string())];
        assert_eq!(kept_sections(&imported(other, two.clone())), two);
    }
}
