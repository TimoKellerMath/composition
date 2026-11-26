//! A mass assembled from the files of its parts as they are (`fux mass --rewrite`;
//! docs/naechste-schritte.md, D9).
//!
//! After a change of the rules, the tables, the plan or the generator, the parts of a finished mass
//! no longer fit the plan that this program makes of its options ([`PartField::fits`]), yet the
//! files of the mass may be written anew, in another notation or checked under the profile of
//! today. [`mass_of_parts`] assembles the mass as it was set. Everything its notes depend on comes
//! from the files, nothing from a plan of this program, which may cut, end, score or seed the
//! parts otherwise:
//! - from the field of the mass (`fux-mass`): its form, the mode of its draws, its choir, its
//!   principal degree, its seed, its chant and its head motif;
//! - from the file of each part: its movement and its number in it (the name of the file), its
//!   text and how it divides eleison, its join with the title of its section or the label of its
//!   Break, its ending, start, opening chord and continuation, the voices it sings, the seed and
//!   the attempt of its search, its points, its head motif, its profile and the version of the
//!   generator that set it (`fux-part`), its notes, lyrics, internal cadences and plan of
//!   imitation;
//! - for each movement: its mode, that of its parts; its sections, as the joins of its parts begin
//!   them; and its voices, those its parts sing.
//!
//! The form of the mass ([`preset::form`]) gives what no draw decides and what the files only
//! name: the ids of the sections, found by the titles the files keep, the labels of the Breaks, the
//! texts each section sings, by which a missing part shows, the Osanna ut supra, the intonation and
//! the remarks for the performers; and the voices of a section whose every part was set in the
//! choir of the mass by the fallback of the scoring (MASS-SCORING). The plan serves to write the
//! mass, not to set it: it has no chains of internal cadences and no estimates, and every movement
//! has the principal degree of the mass, which is all the files keep of it.

use std::cmp::Ordering;
use std::fmt;
use std::sync::Arc;

use super::generate::Mass;
use super::preset::{self, Scoring, SectionForm};
use super::{
    Estimate, FieldError, Join, MassField, MassPlan, Movement, MovementPlan, Opening, PartField,
    PartPlan, PartRef, SectionPlan, movement_voices, opening_chord, part_from_parts, tables,
    voice_names, voice_places,
};
use crate::clef::VoiceType;
use crate::generate::splitmix64_inverse;
use crate::lang::Lang;
use crate::mode::ModeSpec;
use crate::rules::Profile;
use crate::score::{Score, SectionKind};
use crate::text::plan::CadenceChain;
use crate::text::{Eleison, Text, TextOptions, ordinary, resolve};

/// A part of a mass as its file keeps it ([`mass_of_parts`]): its movement and its number in it,
/// from 1, as the name of its file gives them (`3-02-credo_5-7.musicxml`), what the file keeps of
/// it ([`PartField`]), and its score as the file gives it back, as `fux check` reads it: its
/// lyrics aligned with the text the file names (`fux-text`), divided as the file divides it
/// (eleison in four syllables or in three), singing the positions the file keeps (`fux-lyrics`),
/// and its plan of imitation attached.
#[derive(Clone, Debug)]
pub struct KeptPart {
    pub movement: Movement,
    pub number: usize,
    pub field: PartField,
    pub score: Score,
}

/// The mass `record` (the field of the mass that its files name) as the parts `kept` were set:
/// those of the movements `movements`, in the order of the mass, with the plan their files give
/// ([module documentation](self)), titled `title` (or as its form, without one), its syllables
/// printed with their acutes or, without `accents`, without. Each part is taken up as
/// [`part_from_parts`] takes up a part from its file, checked again under `profile`, and keeps
/// its field as its file has it ([`PartResult::kept`](super::PartResult::kept)): the files of the
/// mass keep every part as it was set, its profile and its version of the generator included,
/// whatever this program would plan.
///
/// Fails where the parts do not make the movements as they were set ([`KeptError`]): a movement
/// without parts, a part missing (a number of its movement that no part has, or the parts of the
/// movement ending before its form does), two parts with one number, parts whose joins, titles or
/// texts are not the sections of the form, a part set after another ending of the part before it
/// or with another head motif than Kyrie I gives (set before a part it follows was set anew), the
/// parts of a movement in different modes, divisions of eleison or choirs, an Osanna ut supra
/// without the Sanctus it repeats, and a part that its file does not give back.
pub fn mass_of_parts(
    record: &MassField,
    movements: &[Movement],
    kept: Vec<KeptPart>,
    title: Option<String>,
    accents: bool,
    profile: &Profile,
) -> Result<Mass, KeptError> {
    let mut chosen = movements.to_vec();
    chosen.sort();
    chosen.dedup();
    let mut grouped: Vec<Vec<KeptPart>> = vec![Vec::new(); chosen.len()];
    for part in kept {
        if let Some(m) = chosen
            .iter()
            .position(|&movement| movement == part.movement)
        {
            grouped[m].push(part);
        }
    }
    for (&movement, parts) in chosen.iter().zip(&mut grouped) {
        if parts.is_empty() {
            return Err(KeptError::NoParts { movement });
        }
        parts.sort_by_key(|part| part.number);
        for (i, part) in parts.iter().enumerate() {
            match part.number.cmp(&(i + 1)) {
                Ordering::Equal => {}
                Ordering::Less => {
                    return Err(KeptError::Twice {
                        part: address(movement, part.number),
                    });
                }
                Ordering::Greater => {
                    return Err(KeptError::Missing {
                        part: address(movement, i + 1),
                        later: false,
                    });
                }
            }
        }
    }
    let text = division(&chosen, &grouped)?;
    let choir = &record.choir;
    let enlarged = tables::scoring(choir, Scoring::Enlarged);
    let mut plan = MassPlan {
        title: title.unwrap_or_else(|| record.preset.title().to_string()),
        principal: record.principal,
        movements: Vec::new(),
        preset: record.preset,
        mode: record.mode.clone(),
        choir: choir.clone(),
        seed: record.seed,
        text,
        accents,
        chant: record.chant,
        motif: record.motif,
    };
    // The voice types of every section, by movement, and the parts in the order of the plan, each
    // with whether it sang the choir of the mass after the fallback of the scoring.
    let mut types: Vec<Vec<Vec<VoiceType>>> = Vec::new();
    let mut placed: Vec<(PartRef, KeptPart, bool)> = Vec::new();
    for (m, (&movement, parts)) in chosen.iter().zip(grouped).enumerate() {
        let form = preset::form(record.preset, movement);
        let mode = parts[0].score.mode;
        if parts.iter().any(|part| part.score.mode != mode) {
            return Err(KeptError::Apart { movement });
        }
        let count = parts.len();
        let mut parts = parts.into_iter().peekable();
        let mut sections = Vec::with_capacity(form.sections.len());
        let mut kinds = Vec::with_capacity(form.sections.len());
        for (s, section) in form.sections.iter().enumerate() {
            let mut planned = SectionPlan {
                id: section.id,
                title: section.title,
                voices: Vec::new(),
                parts: Vec::new(),
                ut_supra: None,
            };
            // An Osanna ut supra has no parts of its own; it ends its movement (MASS-FORM).
            if section.ut_supra.is_some() {
                sections.push(planned);
                kinds.push(Vec::new());
                continue;
            }
            let Some(first) = parts.next() else {
                return Err(KeptError::Missing {
                    part: address(movement, count + 1),
                    later: true,
                });
            };
            let begins = if s == 0 {
                None
            } else {
                Some(SectionKind::Section)
            };
            if first.field.join != begins || first.field.title != section.title {
                return Err(KeptError::Form {
                    part: address(movement, first.number),
                });
            }
            let mut members = vec![first];
            while let Some(next) = parts.next_if(|part| {
                matches!(
                    part.field.join,
                    Some(SectionKind::Break | SectionKind::Seam)
                )
            }) {
                members.push(next);
            }
            sung_through(movement, section, &members, &plan.text)?;
            // The parts set in the choir of the mass by the fallback of the scoring, which sing
            // not their section's voices (`base_scoring`).
            let fell: Vec<bool> = members
                .iter()
                .map(|part| {
                    part.field.attempt >= 3
                        && section.scoring == Scoring::Enlarged
                        && enlarged != *choir
                        && kinds_of(&part.score) == *choir
                })
                .collect();
            let own: Vec<Vec<VoiceType>> = members
                .iter()
                .zip(&fell)
                .filter(|&(_, &fell)| !fell)
                .map(|(part, _)| kinds_of(&part.score))
                .collect();
            let section_kinds = match own.split_first() {
                Some((first, rest)) if rest.iter().all(|other| other == first) => first.clone(),
                Some(_) => return Err(KeptError::Apart { movement }),
                None => enlarged.clone(),
            };
            let place = Place {
                movement,
                mode,
                section,
                kinds: &section_kinds,
            };
            for (p, (part, fell)) in members.into_iter().zip(fell).enumerate() {
                planned
                    .parts
                    .push(part_plan(record.motif, &place, &part, fell)?);
                let at = PartRef {
                    movement: m,
                    section: s,
                    part: p,
                };
                placed.push((at, part, fell));
            }
            sections.push(planned);
            kinds.push(section_kinds);
        }
        if let Some(extra) = parts.next() {
            return Err(KeptError::Form {
                part: address(movement, extra.number),
            });
        }
        types.push(kinds);
        plan.movements.push(MovementPlan {
            movement,
            mode,
            principal: record.principal,
            voices: Vec::new(),
            names: Vec::new(),
            intonation: form.intonation,
            notes: form.notes.to_vec(),
            sections,
        });
    }
    // „Osanna ut supra“ repeats the section of the Sanctus it names, with its voices.
    for m in 0..plan.movements.len() {
        let form = preset::form(record.preset, plan.movements[m].movement);
        for (s, section) in form.sections.iter().enumerate() {
            let Some(id) = section.ut_supra else {
                continue;
            };
            let repeated = plan.movements.iter().enumerate().find_map(|(n, movement)| {
                let found = movement.sections.iter().position(|other| other.id == id);
                found.map(|t| (n, t))
            });
            let Some((n, t)) = repeated else {
                return Err(KeptError::UtSupra);
            };
            plan.movements[m].sections[s].ut_supra = Some((n, t));
            types[m][s] = types[n][t].clone();
        }
    }
    for (movement, kinds) in plan.movements.iter_mut().zip(&types) {
        let voices = movement_voices(kinds);
        for (section, kinds) in movement.sections.iter_mut().zip(kinds) {
            section.voices =
                voice_places(&voices, kinds).expect("the movement has every voice of its sections");
        }
        movement.names = voice_names(&voices, Lang::De);
        movement.voices = voices;
    }
    for (at, part, fell) in &placed {
        let movement = &plan.movements[at.movement];
        let sung = if *fell {
            voice_places(&movement.voices, choir)
        } else {
            Some(movement.sections[at.section].voices.clone())
        };
        if sung.as_ref() != Some(&part.field.voices) {
            return Err(KeptError::Apart {
                movement: movement.movement,
            });
        }
    }
    let plan = Arc::new(plan);
    // The head motif as the file of Kyrie I keeps it, where the mass has one and Kyrie I is here.
    let motif = plan.motif_source().map(|kyrie| {
        placed
            .iter()
            .find(|(at, ..)| *at == kyrie)
            .and_then(|(_, part, _)| part.field.motif.clone())
    });
    let mut mass = Mass::new(Arc::clone(&plan));
    for (at, KeptPart { field, score, .. }, _) in placed {
        // A field without the ending before its part (the first files of phase 14a) does not say
        // which ending its chain started from.
        if field.before.is_some() && field.before != plan.ending_before(at) {
            return Err(KeptError::Before {
                part: plan.address(at),
            });
        }
        if motif
            .as_ref()
            .is_some_and(|motif| !field.motif_fits(&plan, at, motif.as_ref()))
        {
            return Err(KeptError::Motif {
                part: plan.address(at),
            });
        }
        let mut result =
            part_from_parts(&plan, at, score, &field, profile).map_err(KeptError::Field)?;
        result.kept = Some(field);
        mass.insert(result);
    }
    Ok(mass)
}

/// Where a part stands, for its plan ([`part_plan`]): its movement and the movement's mode, the
/// form of its section and the voice types its section sings, from the top.
struct Place<'a> {
    movement: Movement,
    mode: ModeSpec,
    section: &'a SectionForm,
    kinds: &'a [VoiceType],
}

/// The address of the part numbered `number` (from 1) in `movement`: `credo/3`.
fn address(movement: Movement, number: usize) -> String {
    format!("{}/{number}", movement.id())
}

/// The voice types a score sings, from the top.
fn kinds_of(score: &Score) -> Vec<VoiceType> {
    score.voices.iter().map(|voice| voice.voice_type).collect()
}

/// Whether two texts have the same syllables, as their words are written.
fn same_syllables(a: &Text, b: &Text) -> bool {
    a.syllables.len() == b.syllables.len()
        && a.syllables
            .iter()
            .zip(&b.syllables)
            .all(|(x, y)| x.text == y.text)
}

/// How the parts `parts` of the movements `movements` divide eleison: in three syllables or in
/// four, as the texts of their files are divided where the two divisions differ (the Kyrie);
/// four where no text tells. Parts that divide it otherwise than one another do not go together.
fn division(movements: &[Movement], parts: &[Vec<KeptPart>]) -> Result<TextOptions, KeptError> {
    let mut found: Option<Eleison> = None;
    for (&movement, parts) in movements.iter().zip(parts) {
        for part in parts {
            let Some(sung) = &part.score.text else {
                continue;
            };
            let spec = sung.text.spec();
            let read = |eleison| resolve(&spec, &TextOptions { eleison }).ok();
            let (Some((four, _)), Some((three, _))) = (read(Eleison::Four), read(Eleison::Three))
            else {
                continue;
            };
            if same_syllables(&four, &three) {
                continue;
            }
            let this = if same_syllables(&sung.text, &three) {
                Eleison::Three
            } else {
                Eleison::Four
            };
            match found {
                Some(other) if other != this => return Err(KeptError::Apart { movement }),
                _ => found = Some(this),
            }
        }
    }
    Ok(match found {
        Some(eleison) => TextOptions { eleison },
        None => TextOptions::default(),
    })
}

/// Checks that the parts `members` of the section `section` of `movement` sing its texts one
/// after the other, as `options` divide them, and to their end: a part that sings another text
/// where the parts before it leave off does not belong there ([`KeptError::Text`]); parts that
/// end before the section's text does lack the parts after them ([`KeptError::Missing`]).
fn sung_through(
    movement: Movement,
    section: &SectionForm,
    members: &[KeptPart],
    options: &TextOptions,
) -> Result<(), KeptError> {
    let expected: Vec<&str> = section
        .pieces
        .iter()
        .filter_map(|piece| ordinary::section(piece.id, options))
        .flat_map(|text| text.syllables.iter().map(|syllable| syllable.text.as_str()))
        .collect();
    let mut at = 0;
    for part in members {
        let fits = part.score.text.as_ref().is_some_and(|sung| {
            let syllables = &sung.text.syllables;
            expected.get(at..at + syllables.len()).is_some_and(|there| {
                there
                    .iter()
                    .zip(syllables)
                    .all(|(planned, sung)| *planned == sung.text)
            })
        });
        if !fits {
            return Err(KeptError::Text {
                part: address(movement, part.number),
            });
        }
        at += part
            .score
            .text
            .as_ref()
            .map_or(0, |sung| sung.text.syllables.len());
    }
    if at < expected.len() {
        let last = members.last().map_or(0, |part| part.number);
        return Err(KeptError::Missing {
            part: address(movement, last + 1),
            later: true,
        });
    }
    Ok(())
}

/// How the part `part` of the section `section` of `movement` joins the part before it, as its
/// field says: a Break with the label of the text it begins with, which the section's form must
/// have there ([`KeptError::Form`]).
fn join_of(movement: Movement, section: &SectionForm, part: &KeptPart) -> Result<Join, KeptError> {
    Ok(match part.field.join {
        None => Join::Start,
        Some(SectionKind::Section) => Join::Section,
        Some(SectionKind::Seam) => Join::Seam,
        Some(SectionKind::Break) => {
            let spec = part
                .score
                .text
                .as_ref()
                .map_or(String::new(), |sung| sung.text.spec());
            let begins = spec.split(['+', ':']).next().unwrap_or_default();
            let label = section.pieces.iter().find_map(|piece| {
                piece
                    .label
                    .filter(|&label| label == part.field.title && piece.id == begins)
            });
            match label {
                Some(label) => Join::Break(label),
                None => {
                    return Err(KeptError::Form {
                        part: address(movement, part.number),
                    });
                }
            }
        }
    })
}

/// The plan of the part `part` at `place`, as its file keeps it, in a mass with a head motif
/// where `motif` (MASS-PARTS): its text, join ([`join_of`]), ending, start, opening and
/// continuation; its opening chord for the voices of its section, as the plan gave it, which the
/// fallback of the scoring (`fell`) voiced anew for the choir; its points, and where the fallback
/// of its lengths set it with the points its imitation plans by itself, those and the extra
/// points the plan gave it ([`tables::EXTRA_POINTS`], as with `--lengths palestrina`, the
/// default: a part of Agnus Dei I with the head motif that its fourth to sixth attempt set counts
/// so, though where the plan gave it no extra points (`--lengths natürlich`, or `--repeat keine`,
/// under which its text has no room for them) the fallback of the motif set it, which only its
/// warning tells); whether it takes the head motif, as the plan gives it to the first part of
/// Gloria, Credo, Sanctus and Agnus Dei I where it begins in imitation; and the seed of the part,
/// from which the seed of its attempt came (MASS-SEED), its own after `--only … --seed`.
fn part_plan(
    motif: bool,
    place: &Place<'_>,
    part: &KeptPart,
    fell: bool,
) -> Result<PartPlan, KeptError> {
    let Place {
        movement,
        mode,
        section,
        kinds,
    } = *place;
    let field = &part.field;
    let join = join_of(movement, section, part)?;
    let text = part.score.text.as_ref();
    let opening = match field.chord {
        Some(_) => Opening::Tutti,
        None => Opening::Imitative,
    };
    let chord = match (fell, opening) {
        (true, Opening::Tutti) => Some(opening_chord(mode, kinds, mode.final_step())),
        _ => field.chord.clone(),
    };
    let lengthened = field.points.is_none()
        && field.attempt >= 3
        && opening == Opening::Imitative
        && !fell
        && tables::LENGTHENED.contains(&section.id);
    let points = if lengthened {
        let natural = part
            .score
            .imitation
            .as_ref()
            .map_or(0, |imitation| imitation.points.len());
        Some(natural + tables::EXTRA_POINTS)
    } else {
        field.points
    };
    Ok(PartPlan {
        text: text.map_or(String::new(), |sung| sung.text.spec()),
        syllables: text.map_or(0, |sung| sung.text.syllables.len()),
        join,
        ending: field.ending,
        opening,
        start: field.start,
        chord,
        chain: CadenceChain::Rotation,
        continued: field.continued,
        points,
        motif: motif
            && opening == Opening::Imitative
            && join == Join::Start
            && matches!(
                movement,
                Movement::Gloria | Movement::Credo | Movement::Sanctus | Movement::Agnus
            ),
        // Attempt a searched with `splitmix64(part seed + a)`.
        seed: splitmix64_inverse(field.seed).wrapping_sub(u64::from(field.attempt)),
        estimate: Estimate::default(),
    })
}

/// Why the parts that the files of a mass keep do not make the mass as it was set
/// ([`mass_of_parts`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeptError {
    /// A movement asked for that no file keeps a part of.
    NoParts { movement: Movement },
    /// A part missing, by its address (`credo/5`): a number of its movement that no part has, or,
    /// where `later`, the one after the last part kept, as the parts end before the form of
    /// their movement does.
    Missing { part: String, later: bool },
    /// Two parts with the number of this address.
    Twice { part: String },
    /// The part at this address does not begin a section of the form of its movement, or the
    /// Break of its label, where its join says it does; or it follows the last section of the
    /// form. Its parts were planned for another form.
    Form { part: String },
    /// The part at this address does not sing the text with which its section goes on after the
    /// parts before it.
    Text { part: String },
    /// The part at this address was set after another ending of the part before it than the
    /// files keep: before that part was set anew.
    Before { part: String },
    /// The part at this address took another head motif than Kyrie I in the files gives: it was
    /// set before Kyrie I was set anew.
    Motif { part: String },
    /// The parts of this movement are in different modes, divide eleison otherwise than one
    /// another, or sing voices that make no choir of the movement.
    Apart { movement: Movement },
    /// An Osanna ut supra (the form normal) without the Sanctus whose Osanna it repeats.
    UtSupra,
    /// A part that its file does not give back as its field says ([`part_from_parts`]).
    Field(FieldError),
}

impl KeptError {
    /// Message for users in their language, a sentence without its full stop.
    pub fn message(&self, lang: Lang) -> String {
        match (self, lang) {
            (KeptError::NoParts { movement }, Lang::De) => {
                format!("Vom {} liegt kein Teil vor", movement.title())
            }
            (KeptError::NoParts { movement }, Lang::En) => {
                format!("no part of the {} is there", movement.title())
            }
            (KeptError::Missing { part, later: false }, Lang::De) => format!("Teil {part} fehlt"),
            (KeptError::Missing { part, later: false }, Lang::En) => {
                format!("part {part} is missing")
            }
            (KeptError::Missing { part, later: true }, Lang::De) => {
                format!("Die Teile ab {part} fehlen")
            }
            (KeptError::Missing { part, later: true }, Lang::En) => {
                format!("the parts from {part} on are missing")
            }
            (KeptError::Twice { part }, Lang::De) => {
                format!("Zwei Dateien halten den Teil {part}")
            }
            (KeptError::Twice { part }, Lang::En) => format!("two files hold part {part}"),
            (KeptError::Form { part }, Lang::De) => format!(
                "Teil {part} beginnt keinen Abschnitt oder Halteklang der Form der Messe, wo sein \
                 Übergang es sagt, oder folgt ihrem letzten Abschnitt"
            ),
            (KeptError::Form { part }, Lang::En) => format!(
                "part {part} begins no section or Break of the form of the mass where its join \
                 says so, or follows its last section"
            ),
            (KeptError::Text { part }, Lang::De) => format!(
                "Teil {part} singt nicht den Text, mit dem sein Abschnitt nach den Teilen davor \
                 weitergeht"
            ),
            (KeptError::Text { part }, Lang::En) => format!(
                "part {part} does not sing the text with which its section goes on after the \
                 parts before it"
            ),
            (KeptError::Before { part }, Lang::De) => format!(
                "Teil {part} wurde nach einer anderen Schlussstufe des Teils davor gesetzt, als \
                 dessen Datei hält"
            ),
            (KeptError::Before { part }, Lang::En) => format!(
                "part {part} was set after another ending of the part before it than that part's \
                 file keeps"
            ),
            (KeptError::Motif { part }, Lang::De) => {
                format!("Teil {part} nahm ein anderes Kopfmotiv, als das Kyrie I gibt")
            }
            (KeptError::Motif { part }, Lang::En) => {
                format!("part {part} took another head motif than Kyrie I gives")
            }
            (KeptError::Apart { movement }, Lang::De) => format!(
                "Die Teile des {} passen nicht zusammen: Modus, Teilung von eleison oder Stimmen",
                movement.title()
            ),
            (KeptError::Apart { movement }, Lang::En) => format!(
                "the parts of the {} do not go together: their modes, their division of eleison \
                 or their voices",
                movement.title()
            ),
            (KeptError::UtSupra, Lang::De) => "Das Osanna ut supra des Benedictus wiederholt das \
                                                Osanna des Sanctus, und das fehlt"
                .to_string(),
            (KeptError::UtSupra, Lang::En) => "the Osanna ut supra of the Benedictus repeats the \
                                                Osanna of the Sanctus, which is missing"
                .to_string(),
            (KeptError::Field(error), lang) => error.message(lang),
        }
    }
}

impl fmt::Display for KeptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message(Lang::En))
    }
}

impl std::error::Error for KeptError {}
