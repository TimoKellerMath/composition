//! The parts of a mass together, and what a file keeps of them (docs/design/phase-14/spec.md
//! §§ 2.2, 4.4 and 6).
//!
//! **Sections** ([`section_score`]). A section is its parts one after the other, each part's
//! measures at its offset, in all the voices of the movement; the voices that do not sing in
//! the section rest. Its text is the parts' texts sung one after the other, every note's
//! syllable counted in it. Its internal cadences are those of its parts and the final cadence of
//! every part but the last, which inside the section is an internal one; the plans of its
//! imitative parts make one plan. Marks tell where a part begins after a Break, with its label,
//! or after a seam. So the rules judge the joins too: at every Break and seam every voice rests
//! a minim at least, and no melodic pair, tie or planned entry crosses it.
//!
//! **Movements** ([`movement_score`]). A movement is its sections one after the other, each
//! joined as above, with a mark where each section begins, its title on it, and the marks of its
//! Breaks and seams; an „Osanna ut supra“ sounds the Osanna it repeats. This is what the file of a
//! movement and its MIDI file hold (§ 6).
//!
//! **Fields** ([`MassField`], [`PartField`]). A file of a part keeps, besides its text, its
//! cadences and its plan of imitation, what the plan said of it and how it was set: `fux-mass`
//! for the mass, `fux-part` for the part (§ 2.2), each as `key=value` items separated by `;`.
//! `fux check` judges a part again from them, `--resume` takes up the parts whose plan, seed,
//! profile and version of the generator are those of the mass it goes on with
//! ([`PartField::fits`]: the ending before the part, from which its chain starts, and the way its
//! attempt set it too), and the page joins the parts its workers set ([`part_from_parts`], which
//! takes a part only where it sings its text as the plan divides it). A file of a movement keeps
//! them for every part, numbered within the movement ([`part_fields`]); [`split_movement`] gives
//! its parts back from them (D14), and [`fields_of_part`] the fields of one part as the file of
//! the part has them.

use std::fmt;
use std::ops::RangeInclusive;
use std::str::FromStr;
use std::sync::Arc;

use super::generate::{
    Fallback, Mass, PartResult, base_scoring, fallback_of, plan_warnings, profile_hash,
};
use super::{
    ChantMass, GENERATOR, Join, MassPlan, Motif, Movement, MovementPlan, Opening, PartPlan,
    PartRef, Preset, opening_chord, parse_chant, preset, voice_places,
};
use crate::clef::VoiceType;
use crate::generate::{Generated, Warning, splitmix64};
use crate::imitation::chant::ChantFallback;
use crate::lang::Lang;
use crate::pitch::{Pitch, Step};
use crate::rules::{Checker, Profile};
use crate::score::{
    Cadence, Lyric, Note, Role, Score, ScoreEntry, ScoreError, ScoreImitation, ScorePoint,
    ScoreText, SectionKind, SectionMark, Species, Voice,
};
use crate::text::{Eleison, Text, TextOptions, resolve};
use crate::time::Duration;

/// The field of a file with the mass a part or a movement belongs to ([`MassField`]).
pub const FIELD_MASS: &str = "fux-mass";

/// The field of a file of a part with what the plan said of the part and how it was set
/// ([`PartField`]); in a file of a movement numbered by the part's place in the movement, from 1
/// (`fux-part.3`), as the fields of the part's text, cadences and imitation are.
pub const FIELD_PART: &str = "fux-part";

/// The fields of a score as `fux_export` writes them for a single setting, which a file of a
/// movement keeps for each part, numbered: the text, the internal cadences and the plan of the
/// imitation.
const FIELD_TEXT: &str = "fux-text";
const FIELD_CADENCES: &str = "fux-cadences";
const FIELD_IMITATION: &str = "fux-imitation";

/// The parts of the section `section` of the movement `movement` of a mass, one after the other,
/// as one score (module documentation): all the voices of the movement, named as it names them,
/// the voices that do not sing in the section resting; the parts' texts as one, not going on
/// after it, every lyric counted in it; the parts' internal cadences and the final cadence of
/// every part but the last; one plan of imitation of the imitative parts, `None` where no part
/// imitates; a mark for each part after a Break or a seam. Its title is the section's. An
/// „Osanna ut supra“ gives the Osanna it repeats, in the voices of its own movement. `None`
/// where a part of the section is not set.
pub fn section_score(m: &Mass, movement: usize, section: usize) -> Option<Score> {
    let plan = &m.plan;
    let target = plan.movements.get(movement)?;
    let own = target.sections.get(section)?;
    let pieces = section_pieces(m, movement, section, false)?;
    let mut score = joined(target, &pieces)?;
    score.title = Some(own.title.to_string());
    Some(score)
}

/// The movement `movement` of a mass as one score (§§ 2.2 and 6, [module documentation](self)):
/// its sections one after the other, each joined as [`section_score`] joins it, in all the voices
/// of the movement, named as it names them, a voice that does not sing in a section resting
/// through it; one text of all its parts' texts, not going on after it, every lyric counted in
/// it; the internal cadences of its parts and the final cadence of every part but the last; one
/// plan of imitation of its imitative parts, `None` where no part imitates; a mark where each
/// section begins, with its title, the first at measure 0, and a mark for each part after a
/// Break, with its label, or after a seam. An „Osanna ut supra“ sounds the Osanna it repeats, in
/// the voices of its own movement. Its title is the movement's. `None` where a part of the
/// movement (or of the Osanna it repeats) is not set.
pub fn movement_score(m: &Mass, movement: usize) -> Option<Score> {
    let target = m.plan.movements.get(movement)?;
    let mut pieces = Vec::new();
    for section in 0..target.sections.len() {
        pieces.extend(section_pieces(m, movement, section, true)?);
    }
    let mut score = joined(target, &pieces)?;
    score.title = Some(target.movement.title().to_string());
    Some(score)
}

/// A part to join: the movement whose voices its result names, its plan, its result, and the
/// title of the section it begins where the score joined marks the sections.
struct Piece<'a> {
    source: &'a MovementPlan,
    part: &'a PartPlan,
    result: &'a PartResult,
    section: Option<&'a str>,
}

/// The parts of the section `section` of the movement `movement` of `m` to join, those of the
/// Osanna it repeats for an Osanna ut supra; the first with the section's title where `marked`.
/// `None` where one is not set or the section has none.
fn section_pieces(
    m: &Mass,
    movement: usize,
    section: usize,
    marked: bool,
) -> Option<Vec<Piece<'_>>> {
    let plan = &m.plan;
    let own = plan.movements.get(movement)?.sections.get(section)?;
    let (from, repeated) = own.ut_supra.unwrap_or((movement, section));
    let source = &plan.movements[from];
    let pieces = source.sections[repeated]
        .parts
        .iter()
        .enumerate()
        .map(|(p, part)| {
            let result = m.result(PartRef {
                movement: from,
                section: repeated,
                part: p,
            })?;
            Some(Piece {
                source,
                part,
                result,
                section: (marked && p == 0).then_some(own.title),
            })
        })
        .collect::<Option<Vec<Piece<'_>>>>()?;
    (!pieces.is_empty()).then_some(pieces)
}

/// The parts `pieces` one after the other, in the voices of the movement `target`; `None` where
/// a part's voices are not the target's or a part has no text.
fn joined(target: &MovementPlan, pieces: &[Piece<'_>]) -> Option<Score> {
    let voices = target.voices.len();
    let mut lines: Vec<Vec<Note>> = vec![Vec::new(); voices];
    let mut sings = vec![false; voices];
    let mut texts: Vec<Text> = Vec::new();
    let mut cadences: Vec<Cadence> = Vec::new();
    let mut points: Vec<ScorePoint> = Vec::new();
    let mut imitative = false;
    let mut marks: Vec<SectionMark> = Vec::new();
    // The measures and the syllables of the parts so far.
    let (mut measures, mut syllables) = (0_usize, 0_usize);
    for (p, piece) in pieces.iter().enumerate() {
        let (part, result) = (piece.part, piece.result);
        let score = &result.generated.score;
        let kinds: Vec<VoiceType> = result
            .voices
            .iter()
            .map(|&v| piece.source.voices[v])
            .collect();
        let places = voice_places(&target.voices, &kinds)?;
        let length = score.measure_count();
        let per = score.meter.ticks_per_measure();
        let ticks = measures as u32 * per;
        let shift = |lyric: Lyric| match lyric.index() {
            Some(i) => Lyric::new(i + syllables),
            None => lyric,
        };
        for (voice, &place) in score.voices.iter().zip(&places) {
            sings[place] = true;
            lines[place].extend(voice.notes.iter().map(|note| Note {
                lyric: note.lyric.map(shift),
                ..note.clone()
            }));
        }
        for (place, line) in lines.iter_mut().enumerate() {
            if !places.contains(&place) {
                line.extend(std::iter::repeat_n(Note::rest(Duration::WHOLE), length));
            }
        }
        let text = &score.text.as_ref()?.text;
        texts.push(Text {
            continued: false,
            ..(**text).clone()
        });
        cadences.extend(score.cadences.iter().map(|cadence| Cadence {
            measure: cadence.measure + measures,
            target: cadence.target,
        }));
        // The part's final cadence, on its ending, is an internal one of the section or the
        // movement joined.
        if p + 1 < pieces.len() {
            cadences.push(Cadence {
                measure: measures + length - 1,
                target: part.ending,
            });
        }
        if let Some(plan) = &score.imitation {
            imitative = true;
            let words = syllables as u16;
            points.extend(plan.points.iter().map(|point| {
                ScorePoint {
                    words: point.words.start + words..point.words.end + words,
                    entries: point
                        .entries
                        .iter()
                        .map(|entry| ScoreEntry {
                            voice: places[entry.voice],
                            start: entry.start + ticks,
                            // A voice's first entry in a later part follows its own silence from
                            // the part's start, after the held chord of the part before.
                            rest: if p > 0 && entry.rest == 0 {
                                entry.start
                            } else {
                                entry.rest
                            },
                            notes: entry.notes.clone(),
                        })
                        .collect(),
                    goal: point.goal.map(|goal| goal + measures),
                }
            }));
        }
        let mark = match (piece.section, part.join) {
            (Some(title), _) => Some((SectionKind::Section, Some(title.to_string()))),
            (None, _) if p == 0 => None,
            (None, Join::Break(label)) => Some((SectionKind::Break, Some(label.to_string()))),
            (None, _) => Some((SectionKind::Seam, None)),
        };
        if let Some((kind, title)) = mark {
            marks.push(SectionMark {
                measure: measures,
                kind,
                title,
            });
        }
        measures += length;
        syllables += text.syllables.len();
    }
    let parts: Vec<Voice> = lines
        .into_iter()
        .enumerate()
        .map(|(v, notes)| Voice {
            name: target.names[v].clone(),
            voice_type: target.voices[v],
            role: Role::Counterpoint(Species::Free),
            notes,
        })
        .collect();
    let mut score = Score::new(target.mode, parts);
    score.cadences = cadences;
    score.text = Some(ScoreText {
        text: Arc::new(Text::concat(texts)),
        voices: sings,
    });
    score.imitation = imitative.then(|| Arc::new(ScoreImitation { points }));
    score.sections = marks;
    Some(score)
}

/// What a file keeps of a mass ([`FIELD_MASS`]): `preset=brevis;mode=dorian;choir=S,A,T,B;
/// principal=A;seed=1`, and for a mass on chant `chant=ix`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MassField {
    pub preset: Preset,
    /// The mode of the request as the draws read it ([`MassPlan::mode`]).
    pub mode: String,
    /// The voices of the full choir, from the top.
    pub choir: Vec<VoiceType>,
    /// The principal degree beside the final.
    pub principal: Step,
    pub seed: u64,
    /// The chant of the Kyriale the mass paraphrases (MASS-CHANT, `chant=ix`), which gives every
    /// imitative part its subjects; `None` for subjects from the words, also where a file has no
    /// such item. So a part set on chant does not stand for one of a mass without (`--resume`),
    /// even where a mode is given and the draws are the same.
    pub chant: Option<ChantMass>,
    /// The mass has a head motif (MASS-MOTIF, `motif=1`), as its request says
    /// ([`MassPlan::motif`]), whichever of its movements a run sets and whether a part of them
    /// takes the motif: so the parts of one mass name it alike in every run, the file of Kyrie I
    /// keeps the motif it gives (`motif=` of `fux-part`, [`MassPlan::motif_source`]), and the
    /// movements set later take it from there. A mass without the item has no head motif, so
    /// that a part of a mass with one does not stand for a part of a mass without (`--resume`).
    pub motif: bool,
}

impl MassField {
    /// What a file keeps of the mass of `plan`.
    pub fn of(plan: &MassPlan) -> MassField {
        MassField {
            preset: plan.preset,
            mode: plan.mode.clone(),
            choir: plan.choir.clone(),
            principal: plan.principal,
            seed: plan.seed,
            chant: plan.chant,
            motif: plan.motif,
        }
    }
}

impl fmt::Display for MassField {
    /// The items as the field keeps them; `chant=` only for a mass on chant, `motif=1` only for a
    /// mass with a head motif.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let choir: Vec<&str> = self.choir.iter().map(|v| v.abbreviation()).collect();
        write!(
            f,
            "preset={};mode={};choir={};principal={};seed={}",
            self.preset.id(),
            self.mode,
            choir.join(","),
            self.principal.letter(),
            self.seed
        )?;
        if let Some(chant) = self.chant {
            write!(f, ";chant={}", chant.id())?;
        }
        if self.motif {
            f.write_str(";motif=1")?;
        }
        Ok(())
    }
}

impl FromStr for MassField {
    type Err = FieldError;

    /// Reads a field as [`MassField`] writes it; one without `chant=` or `motif=`, as every mass
    /// without chant or head motif and the files of phases 14a and 14b have it, keeps no chant or
    /// no head motif.
    fn from_str(value: &str) -> Result<MassField, FieldError> {
        let items = Items::of(FIELD_MASS, value);
        Ok(MassField {
            preset: items.parse("preset", |v| v.parse().ok())?,
            mode: items.get("mode")?.to_string(),
            choir: items.parse("choir", |v| {
                v.split(',')
                    .map(|voice| voice.parse().ok())
                    .collect::<Option<Vec<VoiceType>>>()
                    .filter(|choir| !choir.is_empty())
            })?,
            principal: items.parse("principal", step)?,
            seed: items.parse("seed", |v| v.parse().ok())?,
            chant: match items.get("chant") {
                Err(FieldError::Missing { .. }) => None,
                _ => items.parse("chant", |v| parse_chant(v).ok())?,
            },
            motif: match items.get("motif") {
                Err(FieldError::Missing { .. }) => false,
                _ => items.parse("motif", |v| match v {
                    "1" => Some(true),
                    "0" => Some(false),
                    _ => None,
                })?,
            },
        })
    }
}

/// What a file keeps of a part ([`FIELD_PART`]): where it stands, what the plan said of it and
/// how it was set, such as `bars=1-31;voices=0,1,2,3;join=seam;title=;end=A;before=D;start=4;
/// open=tutti:F4,D4,A3,D3;cont=1;seed=…;attempt=0;points=;profile=…;generator=2`; where points
/// took their subjects from the words instead of the chant or the head motif, `fell=3` after
/// `points=`; and in a mass with a head motif, for Kyrie I and the parts that take it,
/// `motif=0,2,3,4,3,2` before `profile=`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartField {
    /// Its measures in its file, from 1: from 1 in a file of the part, its measures in the
    /// movement in a file of a movement.
    pub bars: RangeInclusive<usize>,
    /// The voices of the movement it sings, from the top ([`PartResult::voices`]).
    pub voices: Vec<usize>,
    /// How it joins the part before it; `None` at the start of its movement (`join=start`).
    pub join: Option<SectionKind>,
    /// The title of the section it begins, or the label of the Break it begins at; empty after a
    /// seam.
    pub title: String,
    /// The degree it ends on.
    pub ending: Step,
    /// The ending of the part before it in the movement, from which the chain of its internal
    /// cadences starts ([`MassPlan::ending_before`]); `None` for the first part of a movement
    /// (`before=`, and so in a file that does not have the item).
    pub before: Option<Step>,
    /// The tick before which no voice sounds.
    pub start: u32,
    /// Its opening chord from the top, where it begins together (`open=tutti:…`); `None` where
    /// it begins in imitation (`open=imit`).
    pub chord: Option<Vec<Pitch>>,
    /// The text goes on after it (`cont=1`).
    pub continued: bool,
    /// The seed of the search that set it.
    pub seed: u64,
    /// The attempt that set it, from 0.
    pub attempt: u8,
    /// The points of imitation it was asked for; `None` for those its imitation plans by itself
    /// (`points=`).
    pub points: Option<usize>,
    /// The points of its imitation, from 1, that were to take their subjects from the chant of a
    /// mass on chant or from the head motif and took them from the words, as the search warned
    /// ([`Warning::ChantFallback`], [`Warning::MotifFallback`]): so a part taken from its file
    /// warns as it did when it was set ([`part_from_parts`]). `fell=3,5`, only where there are
    /// any; empty without the item, as every part of a mass without chant or head motif has it.
    pub fell: Vec<usize>,
    /// The number of the profile it was set with ([`profile_hash`]).
    pub profile: u64,
    /// The head motif ([`PartResult::motif`], MASS-MOTIF): for Kyrie I the motif it gives, so
    /// that `--only`, `--resume` and the page that joins the parts have it; for a part that takes
    /// it the motif it was given. `None` without the item: in every other part, in a mass without
    /// a head motif, and where Kyrie I gives none.
    pub motif: Option<Motif>,
    /// The version of the generator that set it ([`GENERATOR`]); 1 in a file without the item,
    /// as every part set before the version was kept has it.
    pub generator: u32,
}

impl PartField {
    /// What a file keeps of `result`, the part at `at` of `plan`, whose measures begin at
    /// `first` (from 1) in the file: set by this version of the generator, or taken up from a
    /// file that fits the plan ([`PartField::fits`]), which this version set too. A part taken
    /// up as it is ([`PartResult::kept`]) keeps the field of its file, in these measures.
    pub fn of(plan: &MassPlan, at: PartRef, result: &PartResult, first: usize) -> PartField {
        let measures = result.generated.score.measure_count().max(1);
        if let Some(kept) = &result.kept {
            return PartField {
                bars: first..=first + measures - 1,
                ..kept.clone()
            };
        }
        let movement = &plan.movements[at.movement];
        let section = &movement.sections[at.section];
        let part = plan.part(at);
        let lengthened = result
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::LengthFallback { .. }));
        // The last fallback of a part that takes the head motif sets its first point from the
        // words, and warns so itself; any other warning of the motif is the first point's.
        let staged = fallback_of(plan, at, result.attempt) == Some(Fallback::Motif);
        let fell = result
            .warnings
            .iter()
            .filter_map(|warning| match warning {
                Warning::ChantFallback(fallback) => Some(fallback.point),
                Warning::MotifFallback { .. } if !staged => Some(1),
                _ => None,
            })
            .collect();
        PartField {
            bars: first..=first + measures - 1,
            voices: result.voices.clone(),
            join: join_kind(part.join),
            title: title_of(part.join, section.title),
            ending: part.ending,
            before: plan.ending_before(at),
            start: part.start,
            chord: chord_for(movement, part, &result.voices),
            continued: part.continued,
            seed: result.seed,
            attempt: result.attempt,
            points: if lengthened { None } else { part.points },
            fell,
            profile: result.profile,
            motif: result.motif.clone(),
            generator: GENERATOR,
        }
    }

    /// Whether a file of a part with this field can stand for the part at `at` of `plan` in a
    /// mass set under the profile numbered `profile` (`--resume`): the plan gives the part the
    /// same join, title, ending, start and continuation, and the part before it the same ending,
    /// from which its chain of internal cadences starts; the recorded attempt is one of the part,
    /// and the part sings the voices and the points of the way that attempt sets it (attempts 0
    /// to 2 as planned, each fallback three more): those of the plan as planned, the points its
    /// imitation plans by itself after a fallback of the lengths, the choir of the mass after a
    /// fallback of the scoring; the chord of its voices where it begins together; the seed is
    /// that of the recorded attempt; the profile is the same; and this version of the generator
    /// set it ([`GENERATOR`]), since another may set other notes from the same plan and seed.
    /// Whether the part took the head motif of the mass, [`PartField::motif_fits`] says.
    pub fn fits(&self, plan: &MassPlan, at: PartRef, profile: u64) -> bool {
        let movement = &plan.movements[at.movement];
        let section = &movement.sections[at.section];
        let part = plan.part(at);
        let (voices, points) = match fallback_of(plan, at, self.attempt) {
            None => return false,
            Some(Fallback::Planned | Fallback::Motif) => {
                (Some(section.voices.clone()), part.points)
            }
            Some(Fallback::Lengths) => (Some(section.voices.clone()), None),
            Some(Fallback::Scoring) => (base_scoring(plan, at), part.points),
        };
        voices.as_ref() == Some(&self.voices)
            && self.points == points
            && self.join == join_kind(part.join)
            && self.title == title_of(part.join, section.title)
            && self.ending == part.ending
            && self.before == plan.ending_before(at)
            && self.start == part.start
            && self.chord == chord_for(movement, part, &self.voices)
            && self.continued == part.continued
            && self.seed == splitmix64(part.seed.wrapping_add(u64::from(self.attempt)))
            && self.profile == profile
            && self.generator == GENERATOR
    }

    /// Whether the head motif this field keeps fits the part at `at` of `plan` in a mass whose
    /// Kyrie I gives `motif` (MASS-MOTIF): a part that takes the head motif was given this one (or
    /// none, where Kyrie I gives none), so that a part set before Kyrie I was set anew with
    /// another seed is set again; any other part was given none. The motif that Kyrie I keeps
    /// is the one its notes give, whatever the mass.
    pub fn motif_fits(&self, plan: &MassPlan, at: PartRef, motif: Option<&Motif>) -> bool {
        let kyrie_i = plan.movements[at.movement].movement == Movement::Kyrie
            && (at.section, at.part) == (0, 0);
        if kyrie_i {
            return true;
        }
        match plan.part(at).motif {
            true => self.motif.as_ref() == motif,
            false => self.motif.is_none(),
        }
    }
}

/// The kind of mark a join makes; `None` at the start of a movement.
fn join_kind(join: Join) -> Option<SectionKind> {
    match join {
        Join::Start => None,
        Join::Section => Some(SectionKind::Section),
        Join::Break(_) => Some(SectionKind::Break),
        Join::Seam => Some(SectionKind::Seam),
    }
}

/// The title a part begins with: its section's at a start or a double bar, the label of a Break,
/// none after a seam.
fn title_of(join: Join, section: &str) -> String {
    match join {
        Join::Start | Join::Section => section.to_string(),
        Join::Break(label) => label.to_string(),
        Join::Seam => String::new(),
    }
}

/// The opening chord of `part` sung by `voices` of `movement`, where it begins together: the
/// plan's for the voices of its section, voiced anew for the choir of a fallback.
fn chord_for(movement: &MovementPlan, part: &PartPlan, voices: &[usize]) -> Option<Vec<Pitch>> {
    if part.opening != Opening::Tutti {
        return None;
    }
    let kinds: Vec<VoiceType> = voices
        .iter()
        .filter_map(|&v| movement.voices.get(v).copied())
        .collect();
    let mode = movement.mode;
    Some(opening_chord(mode, &kinds, mode.final_step()))
}

impl fmt::Display for PartField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let voices: Vec<String> = self.voices.iter().map(usize::to_string).collect();
        let join = match self.join {
            None => "start",
            Some(SectionKind::Section) => "section",
            Some(SectionKind::Break) => "break",
            Some(SectionKind::Seam) => "seam",
        };
        let open = match &self.chord {
            None => "imit".to_string(),
            Some(chord) => {
                let pitches: Vec<String> = chord.iter().map(Pitch::to_string).collect();
                format!("tutti:{}", pitches.join(","))
            }
        };
        let points = self.points.map_or(String::new(), |n| n.to_string());
        let before = self
            .before
            .map_or(String::new(), |step| step.letter().to_string());
        let fell = if self.fell.is_empty() {
            String::new()
        } else {
            let points: Vec<String> = self.fell.iter().map(usize::to_string).collect();
            format!(";fell={}", points.join(","))
        };
        let motif = self
            .motif
            .as_ref()
            .map_or(String::new(), |motif| format!(";motif={motif}"));
        write!(
            f,
            "bars={}-{};voices={};join={join};title={};end={};before={before};start={};\
             open={open};cont={};seed={};attempt={};points={points}{fell}{motif};\
             profile={:016x};generator={}",
            self.bars.start(),
            self.bars.end(),
            voices.join(","),
            self.title,
            self.ending.letter(),
            self.start,
            u8::from(self.continued),
            self.seed,
            self.attempt,
            self.profile,
            self.generator
        )
    }
}

impl FromStr for PartField {
    type Err = FieldError;

    /// Reads a field as [`PartField`] writes it; items it does not know are left aside. A field
    /// without `before=`, as the first files of phase 14a were written, keeps no ending before the
    /// part: it fits only the first part of a movement, and the others are set again. One without
    /// `fell=`, as every part of a mass without chant or head motif has it, keeps no point that
    /// took its subject from the words instead; one without `motif=`, as every part of a mass
    /// without a head motif has it, keeps no motif; one without `generator=`, as every part set
    /// before the version was kept, was set by version 1.
    fn from_str(value: &str) -> Result<PartField, FieldError> {
        let items = Items::of(FIELD_PART, value);
        let number = |v: &str| v.trim().parse::<usize>().ok();
        Ok(PartField {
            bars: items.parse("bars", |v| {
                let (first, last) = v.split_once('-')?;
                let (first, last) = (number(first)?, number(last)?);
                (1 <= first && first <= last).then_some(first..=last)
            })?,
            voices: items.parse("voices", |v| {
                v.split(',')
                    .map(number)
                    .collect::<Option<Vec<usize>>>()
                    .filter(|voices| !voices.is_empty())
            })?,
            join: items.parse("join", |v| match v {
                "start" => Some(None),
                "section" => Some(Some(SectionKind::Section)),
                "break" => Some(Some(SectionKind::Break)),
                "seam" => Some(Some(SectionKind::Seam)),
                _ => None,
            })?,
            title: items.get("title")?.to_string(),
            ending: items.parse("end", step)?,
            before: match items.get("before") {
                Err(FieldError::Missing { .. }) => None,
                _ => items.parse("before", |v| match v {
                    "" => Some(None),
                    v => step(v).map(Some),
                })?,
            },
            start: items.parse("start", |v| v.parse().ok())?,
            chord: items.parse("open", |v| match v.split_once(':') {
                None if v == "imit" => Some(None),
                Some(("tutti", pitches)) => pitches
                    .split(',')
                    .map(|pitch| pitch.parse().ok())
                    .collect::<Option<Vec<Pitch>>>()
                    .map(Some),
                _ => None,
            })?,
            continued: items.parse("cont", |v| match v {
                "1" => Some(true),
                "0" => Some(false),
                _ => None,
            })?,
            seed: items.parse("seed", |v| v.parse().ok())?,
            attempt: items.parse("attempt", |v| v.parse().ok())?,
            points: items.parse("points", |v| match v {
                "" => Some(None),
                n => number(n).map(Some),
            })?,
            fell: match items.get("fell") {
                Err(FieldError::Missing { .. }) => Vec::new(),
                _ => items.parse("fell", |v| {
                    v.split(',')
                        .filter(|point| !point.trim().is_empty())
                        .map(|point| number(point).filter(|&n| n >= 1))
                        .collect::<Option<Vec<usize>>>()
                })?,
            },
            profile: items.parse("profile", |v| u64::from_str_radix(v, 16).ok())?,
            motif: match items.get("motif") {
                Err(FieldError::Missing { .. }) => None,
                _ => items.parse("motif", |v| match v {
                    "" => Some(None),
                    v => v.parse().ok().map(Some),
                })?,
            },
            generator: match items.get("generator") {
                Err(FieldError::Missing { .. }) => 1,
                _ => items.parse("generator", |v| v.parse().ok().filter(|&n: &u32| n >= 1))?,
            },
        })
    }
}

/// A step by its letter, as the fields write it (B for B♭ in a mode that has it).
fn step(value: &str) -> Option<Step> {
    let mut chars = value.chars();
    let letter = chars.next()?;
    chars.next().is_none().then(|| Step::from_letter(letter))?
}

/// The items of a field: `key=value` separated by `;`.
struct Items<'a> {
    field: &'static str,
    items: Vec<(&'a str, &'a str)>,
}

impl<'a> Items<'a> {
    fn of(field: &'static str, value: &'a str) -> Items<'a> {
        let items = value
            .split(';')
            .filter_map(|item| item.split_once('='))
            .map(|(key, value)| (key.trim(), value.trim()))
            .collect();
        Items { field, items }
    }

    /// The value of `key`, if the field has it.
    fn get(&self, key: &'static str) -> Result<&'a str, FieldError> {
        self.items
            .iter()
            .find(|(k, _)| *k == key)
            .map(|&(_, value)| value)
            .ok_or(FieldError::Missing {
                field: self.field,
                key,
            })
    }

    /// The value of `key` read by `read`.
    fn parse<T>(
        &self,
        key: &'static str,
        read: impl Fn(&str) -> Option<T>,
    ) -> Result<T, FieldError> {
        let value = self.get(key)?;
        read(value).ok_or_else(|| FieldError::Value {
            field: self.field,
            key,
            value: value.to_string(),
        })
    }
}

/// Why the fields of a file, or a part read from a file, do not fit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldError {
    /// A field without one of its items.
    Missing {
        field: &'static str,
        key: &'static str,
    },
    /// An item whose value cannot be read.
    Value {
        field: &'static str,
        key: &'static str,
        value: String,
    },
    /// A part whose file sings another text than the plan gives the part, or none: the part by
    /// its address, the spec of the plan's text, and the file's (empty without a text).
    Text {
        part: String,
        planned: String,
        found: String,
    },
    /// A part whose file sings the plan's text divided otherwise (eleison in three syllables or
    /// in four, [`MassPlan::text`]): the part by its address, and the syllables of the plan's
    /// division and of the file's.
    Division {
        part: String,
        planned: usize,
        found: usize,
    },
    /// A part whose file has other voices than its field names, or voices its movement does not
    /// have.
    Voices { part: String },
    /// A part whose score does not hold together.
    Score { part: String, error: ScoreError },
    /// A field whose value cannot be read as what it keeps, such as internal cadences
    /// (`fux-cadences.3`) or the text of a mass movement: the field by its name, and its value.
    Unreadable { field: String, value: String },
    /// A file of a movement that numbers its parts with a gap: the number of the part whose field
    /// (`fux-part.N`) it lacks before a later one.
    PartMissing { number: usize },
    /// A part of a file of a movement whose measures, from 1, lie outside the file's
    /// `measures` or do not follow right after the measures of the part before it (the first
    /// part's from measure 1).
    Bars {
        part: String,
        first: usize,
        last: usize,
        measures: usize,
    },
    /// A file of a movement that goes on after its last part, the part by its address: its
    /// parts end with measure `last` (from 1), the file with measure `measures`, and the
    /// measures after them are no Osanna ut supra. `ut_supra` is the title of the Osanna ut
    /// supra the movement ends with after its parts, where its form has one: its section does not
    /// begin after them (a measure inserted or deleted before it).
    Unclaimed {
        part: String,
        last: usize,
        measures: usize,
        ut_supra: Option<String>,
    },
}

impl FieldError {
    /// Message for users in their language, a sentence without its full stop.
    pub fn message(&self, lang: Lang) -> String {
        match (self, lang) {
            (FieldError::Missing { field, key }, Lang::De) => {
                format!("Dem Feld {field} fehlt „{key}“")
            }
            (FieldError::Missing { field, key }, Lang::En) => {
                format!("the field {field} lacks \"{key}\"")
            }
            (FieldError::Value { field, key, value }, Lang::De) => {
                format!("Feld {field}: „{key}={value}“ lässt sich nicht lesen")
            }
            (FieldError::Value { field, key, value }, Lang::En) => {
                format!("field {field}: \"{key}={value}\" cannot be read")
            }
            (
                FieldError::Text {
                    part,
                    planned,
                    found,
                },
                Lang::De,
            ) => format!("Teil {part}: Die Datei singt „{found}“, der Plan „{planned}“"),
            (
                FieldError::Text {
                    part,
                    planned,
                    found,
                },
                Lang::En,
            ) => format!("part {part}: the file sings \"{found}\", the plan \"{planned}\""),
            (
                FieldError::Division {
                    part,
                    planned,
                    found,
                },
                Lang::De,
            ) => format!(
                "Teil {part}: Die Datei teilt den Text in {found} Silben, der Plan in {planned} \
                 (eleison drei- oder viersilbig, --eleison)"
            ),
            (
                FieldError::Division {
                    part,
                    planned,
                    found,
                },
                Lang::En,
            ) => format!(
                "part {part}: the file divides the text into {found} syllables, the plan into \
                 {planned} (eleison in three or four syllables, --eleison)"
            ),
            (FieldError::Voices { part }, Lang::De) => {
                format!("Teil {part}: Die Stimmen der Datei sind nicht die ihres Feldes")
            }
            (FieldError::Voices { part }, Lang::En) => {
                format!("part {part}: the voices of the file are not those of its field")
            }
            (FieldError::Score { part, error }, lang) => {
                let why = error.message(lang, Default::default());
                let why = why.trim_end_matches('.');
                match lang {
                    Lang::De => format!("Teil {part}: {why}"),
                    Lang::En => format!("part {part}: {why}"),
                }
            }
            (FieldError::Unreadable { field, value }, Lang::De) => {
                format!("Feld {field}: „{value}“ lässt sich nicht lesen")
            }
            (FieldError::Unreadable { field, value }, Lang::En) => {
                format!("field {field}: \"{value}\" cannot be read")
            }
            (FieldError::PartMissing { number }, Lang::De) => {
                format!("Der Datei fehlt das Feld {FIELD_PART}.{number}")
            }
            (FieldError::PartMissing { number }, Lang::En) => {
                format!("the file lacks the field {FIELD_PART}.{number}")
            }
            (
                FieldError::Bars {
                    part,
                    first,
                    last,
                    measures,
                },
                Lang::De,
            ) => format!(
                "Teil {part}: Die Takte {first}–{last} liegen nicht in den {measures} Takten der \
                 Datei nach denen des Teils davor"
            ),
            (
                FieldError::Bars {
                    part,
                    first,
                    last,
                    measures,
                },
                Lang::En,
            ) => format!(
                "part {part}: measures {first}–{last} do not lie within the file's {measures} \
                 measures after those of the part before"
            ),
            (
                FieldError::Unclaimed {
                    part,
                    last,
                    measures,
                    ut_supra,
                },
                Lang::De,
            ) => match ut_supra {
                None => format!(
                    "Teil {part}: Die Takte der Teile enden mit Takt {last}, die der Datei mit \
                     Takt {measures}"
                ),
                Some(title) => format!(
                    "Teil {part}: Die Takte der Teile enden mit Takt {last}, und nach ihm \
                     beginnt kein {title}"
                ),
            },
            (
                FieldError::Unclaimed {
                    part,
                    last,
                    measures,
                    ut_supra,
                },
                Lang::En,
            ) => match ut_supra {
                None => format!(
                    "part {part}: the parts' measures end with measure {last}, the file's with \
                     measure {measures}"
                ),
                Some(title) => format!(
                    "part {part}: the parts' measures end with measure {last}, and no {title} \
                     begins after it"
                ),
            },
        }
    }
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message(Lang::En))
    }
}

impl std::error::Error for FieldError {}

/// The field of a file with the mass of `plan`: (`fux-mass`, its value).
pub fn mass_field(plan: &MassPlan) -> (String, String) {
    (FIELD_MASS.to_string(), MassField::of(plan).to_string())
}

/// The field of the file of the part at `at` of `m`, its measures counted from 1:
/// (`fux-part`, its value). `None` where the part is not set. The file keeps the part's text,
/// cadences and imitation in the fields of a single setting, and the mass in [`mass_field`].
pub fn part_field(m: &Mass, at: PartRef) -> Option<(String, String)> {
    let result = m.result(at)?;
    let field = PartField::of(&m.plan, at, result, 1);
    Some((FIELD_PART.to_string(), field.to_string()))
}

/// The fields of the file of the movement `movement` of `m` (§ 2.2): the mass, and for the part
/// numbered N within the movement, from 1, `fux-part.N` with its measures in the movement and its
/// text, internal cadences and plan of imitation relative to the part (`fux-text.N`,
/// `fux-cadences.N`, `fux-imitation.N`), as the fields of a single setting have them. For a
/// movement whose parts are all set; one that is not is left out. An Osanna ut supra has no
/// parts of its own: the parts it repeats are the Sanctus'.
pub fn part_fields(m: &Mass, movement: usize) -> Vec<(String, String)> {
    let mut fields = vec![mass_field(&m.plan)];
    let mut first = 1;
    let places = m.plan.parts().filter(|(at, _)| at.movement == movement);
    for (n, (at, _)) in places.enumerate() {
        let Some(result) = m.result(at) else {
            continue;
        };
        let number = n + 1;
        let score = &result.generated.score;
        let field = PartField::of(&m.plan, at, result, first);
        first = field.bars.end() + 1;
        fields.push((format!("{FIELD_PART}.{number}"), field.to_string()));
        if let Some(sung) = &score.text {
            fields.push((format!("{FIELD_TEXT}.{number}"), sung.text.spec()));
        }
        if !score.cadences.is_empty() {
            let cadences: Vec<String> = score
                .cadences
                .iter()
                .map(|c| format!("{}:{}", c.measure + 1, c.target.letter()))
                .collect();
            fields.push((format!("{FIELD_CADENCES}.{number}"), cadences.join(",")));
        }
        if let Some(imitation) = &score.imitation {
            fields.push((format!("{FIELD_IMITATION}.{number}"), imitation.spec()));
        }
    }
    fields
}

/// The fields of the part numbered `number` (from 1) in the file of a movement with `fields`, as
/// the file of the part has them: its numbered fields without their number (`fux-part.3` as
/// `fux-part`, so `fux-text`, `fux-cadences`, `fux-imitation` and any other a frontend numbers
/// so), and the fields of the whole file (`fux-mode`, `fux-mass`), in the order of `fields`. So
/// `fux check` judges each part of a movement as it judges the file of the part.
pub fn fields_of_part(fields: &[(String, String)], number: usize) -> Vec<(String, String)> {
    fields
        .iter()
        .filter_map(|(name, value)| match numbered(name) {
            None => Some((name.clone(), value.clone())),
            Some((base, n)) if n == number => Some((base.to_string(), value.clone())),
            Some(_) => None,
        })
        .collect()
}

/// A field's name and the number of the part of a movement it belongs to, where it has one
/// (`fux-text.3`).
fn numbered(name: &str) -> Option<(&str, usize)> {
    let (base, number) = name.rsplit_once('.')?;
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((base, number.parse().ok()?))
}

/// The value of the field `name` of the part numbered `number`, where `fields` have it.
fn numbered_field<'a>(
    fields: &'a [(String, String)],
    name: &str,
    number: usize,
) -> Option<&'a str> {
    let name = format!("{name}.{number}");
    fields
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, value)| value.as_str())
}

/// The parts the file of a movement with `fields` keeps, in order: the field of each part,
/// numbered from 1 (`fux-part.1`, `fux-part.2`, …), read; none where the file has no part field.
/// Where a number occurs twice, the first counts. Fails where a field cannot be read, or where
/// the numbers have a gap ([`FieldError::PartMissing`]).
pub fn movement_parts(fields: &[(String, String)]) -> Result<Vec<PartField>, FieldError> {
    let mut found: Vec<(usize, &str)> = fields
        .iter()
        .filter_map(|(name, value)| match numbered(name) {
            Some((FIELD_PART, number)) => Some((number, value.as_str())),
            _ => None,
        })
        .collect();
    found.sort_by_key(|&(number, _)| number);
    found.dedup_by_key(|(number, _)| *number);
    let mut parts = Vec::with_capacity(found.len());
    for (i, (number, value)) in found.into_iter().enumerate() {
        if number != i + 1 {
            return Err(FieldError::PartMissing { number: i + 1 });
        }
        parts.push(value.parse()?);
    }
    Ok(parts)
}

/// The marks of the sections and parts of a movement whose file keeps `parts`
/// ([`movement_parts`]), as [`movement_score`] marks them: where each part begins, a section with
/// its title at the start of the movement and after a double bar, a Break with its label, or a
/// seam. The parts of an Osanna ut supra are the Sanctus', so its mark is not among them.
pub fn movement_sections(parts: &[PartField]) -> Vec<SectionMark> {
    parts
        .iter()
        .map(|part| SectionMark {
            measure: part.bars.start() - 1,
            kind: part.join.unwrap_or(SectionKind::Section),
            title: (!part.title.is_empty()).then(|| part.title.clone()),
        })
        .collect()
}

/// The parts of a movement as the file of the movement keeps them (§ 2.2, D14), the inverse of
/// writing [`movement_score`] with [`part_fields`]: for the part numbered N in `fields`
/// (`fux-part.N`, from 1, [`movement_parts`]) in order, its place, its score and the measures
/// before it in the movement, so that `fux check` repeats the report of every part and tells
/// where it stands.
///
/// The score of a part has the voices of `s` its field names, with their names and their notes
/// in its measures, and as the numbered fields of the part say, relative to the part, its
/// internal cadences (`fux-cadences.N`) and its plan of imitation (`fux-imitation.N`). Where `s`
/// has a text, as [`movement_score`] gives it, the part sings its own (`fux-text.N`), divided as
/// `s` divides it (eleison in four syllables or in three), going on after it as its field says
/// (`cont=1`), every lyric counted in it; where `s` has none, as a file gives the movement back,
/// the part has none either, and a check aligns its syllables with its text as for the file of
/// the part ([`fields_of_part`]).
///
/// The place of a part is its movement, by its place in the order of the mass (Kyrie 0 … Agnus
/// Dei 5, as [`Movement::ALL`]; the index in the plan of a whole mass), which the text of the
/// first part tells; and its section and its part in it, counted by their joins. The parts of an
/// Osanna ut supra are the Sanctus': its measures, after the last part, belong to no part, where
/// the form of the movement ends with one (in the preset of the mass the file names, `fux-mass`)
/// and `s` marks its section with its title right after the parts, as the reader of a file keeps
/// the mark from its notation. Every other measure of `s` belongs to a part. A file without part
/// fields gives none.
///
/// Fails where a field is missing or cannot be read, where the first part sings no text of a
/// movement of the mass, where the measures of a part lie outside `s` or not right after those of
/// the part before it (the first part's from measure 1, [`FieldError::Bars`]), where `s` goes on
/// after the parts other than with an Osanna ut supra as above ([`FieldError::Unclaimed`]: a
/// measure inserted or appended, the field of the last part missing, a measure deleted before an
/// Osanna ut supra), where the voices of a part are not voices of `s`, in order, or a voice
/// outside them sounds in its measures, where its text is not that of `s` at its place, and where
/// the score of a part does not hold together.
pub fn split_movement(
    s: &Score,
    fields: &[(String, String)],
) -> Result<Vec<(PartRef, Score, usize)>, FieldError> {
    let parts = movement_parts(fields)?;
    if parts.is_empty() {
        return Ok(Vec::new());
    }
    let movement = movement_of_fields(fields)?;
    let measures = s.measure_count();
    let per = s.meter.ticks_per_measure();
    let mut split = Vec::with_capacity(parts.len());
    // The place of the part, the last measure of the part before it and the syllables of the
    // texts of the parts before it in the text of `s`.
    let (mut section, mut part) = (0, 0);
    let (mut after, mut syllables) = (0_usize, 0_usize);
    for (i, kept) in parts.iter().enumerate() {
        let number = i + 1;
        let address = format!("{}/{number}", movement.id());
        if i > 0 {
            if matches!(kept.join, None | Some(SectionKind::Section)) {
                section += 1;
                part = 0;
            } else {
                part += 1;
            }
        }
        let (first, last) = (*kept.bars.start(), *kept.bars.end());
        if first != after + 1 || last > measures {
            return Err(FieldError::Bars {
                part: address,
                first,
                last,
                measures,
            });
        }
        after = last;
        let (from, to) = ((first - 1) as u32 * per, last as u32 * per);
        let within = |tick: u32| from <= tick && tick < to;
        let voiced = kept.voices.windows(2).all(|w| w[0] < w[1])
            && kept.voices.last().is_some_and(|&v| v < s.voices.len())
            && s.voices.iter().enumerate().all(|(v, voice)| {
                kept.voices.contains(&v)
                    || voice
                        .timed()
                        .all(|(tick, note)| !within(tick) || note.is_rest())
            });
        if !voiced {
            return Err(FieldError::Voices { part: address });
        }
        let text = match &s.text {
            None => None,
            Some(sung) => {
                let Some(spec) = numbered_field(fields, FIELD_TEXT, number) else {
                    return Err(FieldError::Unreadable {
                        field: format!("{FIELD_TEXT}.{number}"),
                        value: String::new(),
                    });
                };
                let Some(text) = text_at(&sung.text, spec, syllables) else {
                    return Err(FieldError::Text {
                        part: address,
                        planned: spec.to_string(),
                        found: sung.text.spec(),
                    });
                };
                let voices = kept
                    .voices
                    .iter()
                    .map(|&v| sung.voices.get(v).copied().unwrap_or(true))
                    .collect();
                Some((
                    Text {
                        continued: kept.continued,
                        ..text
                    },
                    voices,
                ))
            }
        };
        // The lyrics of `s`, where it has a text, counted in the part's text.
        let (offset, count) = (
            syllables,
            text.as_ref().map(|(text, _)| text.syllables.len()),
        );
        syllables += count.unwrap_or(0);
        let counted = |lyric: Lyric| match (lyric.index(), count) {
            (Some(i), Some(count)) if (offset..offset + count).contains(&i) => {
                Lyric::new(i - offset)
            }
            (Some(_), Some(_)) => Lyric::UNKNOWN,
            _ => lyric,
        };
        let voices: Vec<Voice> = kept
            .voices
            .iter()
            .map(|&v| {
                let voice = &s.voices[v];
                let notes = voice
                    .timed()
                    .filter(|&(tick, _)| within(tick))
                    .map(|(_, note)| Note {
                        lyric: note.lyric.map(counted),
                        ..note.clone()
                    })
                    .collect();
                Voice {
                    name: voice.name.clone(),
                    voice_type: voice.voice_type,
                    role: voice.role,
                    notes,
                }
            })
            .collect();
        let cadences = match numbered_field(fields, FIELD_CADENCES, number) {
            None => Vec::new(),
            Some(value) => cadences_of(value).ok_or_else(|| FieldError::Unreadable {
                field: format!("{FIELD_CADENCES}.{number}"),
                value: value.to_string(),
            })?,
        };
        let imitation = numbered_field(fields, FIELD_IMITATION, number)
            .map(ScoreImitation::parse)
            .transpose()
            .map_err(|error| FieldError::Score {
                part: address.clone(),
                error,
            })?;
        let mut score = Score::new(s.mode, voices);
        score.meter = s.meter;
        score.cadences = cadences;
        score.text = text.map(|(text, voices)| ScoreText {
            text: Arc::new(text),
            voices,
        });
        score.imitation = imitation.map(Arc::new);
        score.validate().map_err(|error| FieldError::Score {
            part: address,
            error,
        })?;
        let place = PartRef {
            movement: movement as usize,
            section,
            part,
        };
        split.push((place, score, first - 1));
    }
    // The measures after the last part: an Osanna ut supra's, where the movement ends with one
    // and its section begins right after the parts, or none.
    if after < measures {
        let ut_supra = ut_supra_title(fields, movement);
        let marked = ut_supra.is_some_and(|title| {
            s.sections.iter().any(|mark| {
                mark.measure == after
                    && mark.kind == SectionKind::Section
                    && mark.title.as_deref() == Some(title)
            })
        });
        if !marked {
            return Err(FieldError::Unclaimed {
                part: format!("{}/{}", movement.id(), parts.len()),
                last: after,
                measures,
                ut_supra: ut_supra.map(str::to_string),
            });
        }
    }
    Ok(split)
}

/// The title of the Osanna ut supra that ends the movement `movement` after its parts, where its
/// form ends with one: in the preset of the mass a file with `fields` names (`fux-mass`), or in
/// either where it names none that can be read.
fn ut_supra_title(fields: &[(String, String)], movement: Movement) -> Option<&'static str> {
    let named = fields
        .iter()
        .find(|(name, _)| name == FIELD_MASS)
        .and_then(|(_, value)| value.parse::<MassField>().ok())
        .map(|mass| mass.preset);
    let presets = match named {
        Some(preset) => vec![preset],
        None => vec![Preset::Brevis, Preset::Normal],
    };
    presets.into_iter().find_map(|preset| {
        let last = preset::form(preset, movement).sections.last()?;
        last.ut_supra.map(|_| last.title)
    })
}

/// The movement whose texts the parts of the file of a movement with `fields` sing: that of the
/// first text of the Ordinary its first part sings (`fux-text.1`).
fn movement_of_fields(fields: &[(String, String)]) -> Result<Movement, FieldError> {
    let value = numbered_field(fields, FIELD_TEXT, 1).unwrap_or_default();
    let id = value.split(['+', ':']).next().unwrap_or_default().trim();
    preset::movement_of(id).ok_or_else(|| FieldError::Unreadable {
        field: format!("{FIELD_TEXT}.1"),
        value: value.to_string(),
    })
}

/// The text `spec` as `sung`, the text of a whole movement, sings it from its syllable `offset`
/// on: divided with eleison in four syllables or in three, whichever `sung` has there. `None`
/// where it has neither, or `spec` names no text.
fn text_at(sung: &Text, spec: &str, offset: usize) -> Option<Text> {
    [Eleison::Four, Eleison::Three]
        .into_iter()
        .find_map(|eleison| {
            let (text, _) = resolve(spec, &TextOptions { eleison }).ok()?;
            let there = sung.syllables.get(offset..offset + text.syllables.len())?;
            let same = there
                .iter()
                .zip(&text.syllables)
                .all(|(a, b)| a.text == b.text && a.flags == b.flags);
            same.then_some(text)
        })
}

/// Internal cadences as the fields keep them (`13:A,22:F`: the measure from 1 and the target);
/// `None` where one cannot be read.
fn cadences_of(value: &str) -> Option<Vec<Cadence>> {
    value
        .split(',')
        .filter(|cadence| !cadence.trim().is_empty())
        .map(|cadence| {
            let (measure, target) = cadence.split_once(':')?;
            let measure: usize = measure.trim().parse().ok().filter(|&m| m >= 1)?;
            Some(Cadence {
                measure: measure - 1,
                target: step(target.trim())?,
            })
        })
        .collect()
}

/// The result of the part at `at` of `plan` from its score as a file gives it back
/// (`--resume`, the page joining the parts of its workers): `score` with its voices, notes and
/// lyrics, its text, internal cadences and plan of imitation as read, and `field`, its
/// [`PartField`]. The text is the plan's, divided as the plan divides it (eleison in three or four
/// syllables, [`MassPlan::text`]), going on after the part where it is not the last of its
/// section, and its syllables are printed as the plan says, with or without their acutes
/// ([`MassPlan::accents`]); the part is checked again under `profile`, the profile the mass sets
/// its parts with ([`part_profile`](super::part_profile)): its report and cost are those a
/// search under it gives. The seed, the attempt and the voices are the field's, and so is a
/// fallback, by the voices it sings and, for one of the lengths or of the head motif, by its
/// attempt ([`PartField::fits`]), with the warnings the plan gives the part; so are the points
/// that took their subjects from the words instead of the chant or the head motif, each with
/// the warning its setting gave ([`PartField::fell`]), and the head motif of Kyrie I and of a
/// part that takes it ([`PartField::motif`]). The warnings come in the order of the search: the
/// fallback's, the plan's, the setting's. The seconds are not kept (0), nor the subjects of its
/// points.
pub fn part_from_parts(
    plan: &MassPlan,
    at: PartRef,
    mut score: Score,
    field: &PartField,
    profile: &Profile,
) -> Result<PartResult, FieldError> {
    let part = plan.part(at);
    let movement = &plan.movements[at.movement];
    let section = &movement.sections[at.section];
    let address = plan.address(at);
    let kinds: Option<Vec<VoiceType>> = field
        .voices
        .iter()
        .map(|&v| movement.voices.get(v).copied())
        .collect();
    let voiced = kinds.is_some_and(|kinds| {
        kinds.len() == score.voices.len()
            && kinds
                .iter()
                .zip(&score.voices)
                .all(|(&kind, voice)| kind == voice.voice_type)
    });
    if !voiced {
        return Err(FieldError::Voices { part: address });
    }
    let found = score
        .text
        .as_ref()
        .map_or(String::new(), |sung| sung.text.spec());
    let Some(sung) = score.text.as_mut().filter(|_| found == part.text) else {
        return Err(FieldError::Text {
            part: address,
            planned: part.text.clone(),
            found,
        });
    };
    // The same words divided otherwise: eleison in three syllables or in four.
    let planned = part.sung_text(&plan.text);
    if sung.text.syllables != planned.syllables {
        return Err(FieldError::Division {
            part: address,
            planned: planned.syllables.len(),
            found: sung.text.syllables.len(),
        });
    }
    sung.text = Arc::new(planned);
    score.fill_syllables(plan.accents);
    score.validate().map_err(|error| FieldError::Score {
        part: address.clone(),
        error,
    })?;
    let report = Checker::new(profile).check(&score);
    let mut warnings = Vec::new();
    if part.points.is_some()
        && field.points.is_none()
        && fallback_of(plan, at, field.attempt) == Some(Fallback::Lengths)
    {
        warnings.push(Warning::LengthFallback {
            part: address.clone(),
        });
    }
    if field.voices != section.voices {
        warnings.push(Warning::ScoringFallback {
            section: section.id.to_string(),
        });
    }
    // A part that takes the head motif keeps the one it was given, Kyrie I the one it gives:
    // the field keeps them, the score no subjects.
    let given = field.motif.clone().filter(|_| part.motif);
    let staged = fallback_of(plan, at, field.attempt) == Some(Fallback::Motif);
    let motif_fallback = || Warning::MotifFallback {
        movement: movement.movement.title().to_string(),
    };
    if given.is_some() && staged {
        warnings.push(motif_fallback());
    }
    warnings.extend(plan_warnings(plan, at, given.as_ref()));
    // The points that took their subjects from the words, as the search warned of them: the first
    // point of a part set on the head motif, any point of a part on chant.
    let on_motif = given.is_some() && !staged;
    warnings.extend(field.fell.iter().map(|&point| {
        if on_motif {
            motif_fallback()
        } else {
            Warning::ChantFallback(ChantFallback { point })
        }
    }));
    let motif = if plan.motif_source() == Some(at) {
        field.motif.clone()
    } else {
        given
    };
    Ok(PartResult {
        at,
        seed: field.seed,
        attempt: field.attempt,
        generated: Generated {
            score,
            cost: report.cost(),
            report,
            warnings: Vec::new(),
            subjects: Vec::new(),
        },
        seconds: 0.0,
        warnings,
        voices: field.voices.clone(),
        profile: profile_hash(profile),
        motif,
        kept: None,
    })
}
