//! The plan of a mass (docs/design/phase-14/spec.md §§ 3 and 4.1): every choice of the mass
//! made before any search, in milliseconds.
//!
//! **Parts** (MASS-PARTS). A long section is cut at the ends of its sentences (`.` `?` `!`) into
//! the fewest parts of at most `chunk` syllables; of the cuts with that many parts the one whose
//! largest part is least wins, then the one whose parts differ least in size, then the one that
//! cuts first. A Break always cuts, and a sentence longer than `chunk` is cut at its commas,
//! semicolons and colons. A short section (Kyrie, Sanctus, Benedictus, Agnus) is one part. At 45
//! syllables the Gloria of the Missa brevis falls into 3 + 3 parts, Patrem into 4, Et incarnatus
//! into 1 and Crucifixus to the Amen into 7: 26 parts, 657 syllables.
//!
//! **Draws.** The mass's number `splitmix64(seed ^ fnv("mass|preset|mode|choir"))` seeds a
//! generator whose first draw is the principal degree P, and, mixed with the movement, a
//! generator for every movement, which draws for every part of the movement in its order its
//! ending, a second ending for a redraw and its restart, whether they are used or not. So a
//! movement plans the same whichever other movements are set (`--movements credo` gives the Credo
//! of the whole mass), and an option for one part changes no draw of another. The seed of a part
//! is `splitmix64(seed ^ fnv(…) ^ place)`, so that masses with one seed in other modes or choirs
//! set their parts and draw their internal cadences otherwise.
//!
//! **Endings** (MASS-ENDING). The last part of a movement and every part in two voices end on
//! the final; the other parts draw from the shares of their kind ([`tables::ending_distribution`]).
//! Within a section an ending equal to the one of the part before is drawn once more, from shares
//! raised for what the redraw takes away ([`tables::redrawn`]), so that the seams and Breaks of a
//! long section vary their held chords and still end on each degree as often as the tables say.
//! Across a double bar nothing is drawn again: the corpus counted the end of each section after
//! the section before it, the Christe after Kyrie I on the final, so a redraw there would count
//! the change twice and put the Dorian Christe on the final in 1.6 % of the plans instead of 13 %
//! (measured over 10 000 plans in phase 14a). The redraws run on the endings as drawn: an ending
//! that `--endings` gives changes the ending of no other part, except where the next part of its
//! section would end as it does; that part is drawn once more against it (and so the next one,
//! where the ending drawn again repeats its own).
//!
//! **Internal cadences** (MASS-CADENCES). Every part gets a chain of its own
//! ([`CadenceChain::Weighted`]): seeded with the part's seed, weighted for Palestrina's internal
//! cadences in the movement's mode with its principal degree ([`tables::chain_weights`]), and
//! starting from the ending of the part before it in the movement, so that the degrees run on
//! across the parts as across the movement. The chain draws nothing in the plan; the part's plan
//! of its text or of its imitation draws its cadences from it.
//!
//! **Restarts** (MASS-RESTART). A section begins in imitation, except the Osanna II of the Missa
//! brevis; a Break and the seam at Et resurrexit restart together. Every other seam restarts in
//! imitation with p .35 in a normal mass, with p .2 in a Missa brevis and there at Et in Spiritum
//! with p .35 ([`tables::restart_seam`]). With two or six voices nothing is imitative unless
//! `--imitation` says so, as in phase 13.

use std::ops::{Range, RangeInclusive};

use super::preset::{self, Piece};
use super::tables::{self, EndingKind};
use super::{
    ChantMass, Estimate, Join, Lengths, MassOptionError, MassPlan, MassRequest, Movement,
    MovementPlan, Opening, Override, PartPlan, PartRef, Preset, SectionPlan, Target,
    movement_voices, opening_chord, voice_names, voice_places,
};
use crate::clef::VoiceType;
use crate::generate::{GenerateError, Rng, fingerprint, part_endings, splitmix64};
use crate::imitation::chant::Kyriale;
use crate::imitation::points::points_of;
use crate::imitation::{Imitation, Toggle};
use crate::lang::Lang;
use crate::mode::{Mode, ModeSpec};
use crate::pitch::{Pitch, Step};
use crate::text::plan::{CadenceChain, automatic_length, melismatic, plan};
use crate::text::{Text, TextError, TextKind, TextOptions, ordinary, resolve};

/// Plans a mass: its movements, sections and parts with every choice of § 3 made, without a
/// search. Options that the plan cannot have fail with [`GenerateError::MassOption`] before
/// anything is searched.
pub fn plan_mass(request: &MassRequest) -> Result<MassPlan, GenerateError> {
    let draft = Draft::new(request).map_err(GenerateError::MassOption)?;
    Ok(draft.draw(request.seed))
}

/// The text of a part from its spec, as [`resolve`] reads it: a section of the Ordinary, some of
/// its units, or several of these joined by `+` and sung one after the other
/// (`sanctus+pleni+osanna`, [`Text::concat`]).
pub fn part_text(spec: &str, options: &TextOptions) -> Result<Text, TextError> {
    resolve(spec, options).map(|(text, _)| text)
}

/// A part of a section as the cut of its text gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Cut {
    spec: String,
    syllables: usize,
    /// The label of the Break the part begins at.
    label: Option<&'static str>,
    /// Where its text begins: the id of a text of the Ordinary and a unit of it, from 0.
    begins: (&'static str, usize),
}

/// A stretch of a text that no part divides: a sentence, or a unit of a sentence longer than the
/// limit.
#[derive(Clone, Debug)]
struct Atom {
    /// The index of its text among the section's.
    piece: usize,
    units: RangeInclusive<usize>,
    syllables: usize,
}

/// The parts of a section singing `pieces` (MASS-PARTS): one for a short section, else the
/// stretches between its Breaks, each cut into parts of at most `limit` syllables.
fn cut_section(pieces: &[Piece], limit: Option<usize>, options: &TextOptions) -> Vec<Cut> {
    let texts: Vec<&'static Text> = pieces
        .iter()
        .map(|piece| {
            ordinary::section(piece.id, options)
                .unwrap_or_else(|| panic!("the forms name sections of ordinary.txt: {}", piece.id))
        })
        .collect();
    if texts.iter().all(|text| text.kind == TextKind::Short) {
        let ids: Vec<&str> = pieces.iter().map(|piece| piece.id).collect();
        return vec![Cut {
            spec: ids.join("+"),
            syllables: texts.iter().map(|text| text.syllables.len()).sum(),
            label: pieces[0].label,
            begins: (pieces[0].id, 0),
        }];
    }
    let mut cuts = Vec::new();
    let mut start = 0;
    while start < pieces.len() {
        let end = (start + 1..pieces.len())
            .find(|&i| pieces[i].label.is_some())
            .unwrap_or(pieces.len());
        cuts.extend(cut_stretch(&pieces[start..end], &texts[start..end], limit));
        start = end;
    }
    cuts
}

/// The parts of a stretch of a section between Breaks.
fn cut_stretch(pieces: &[Piece], texts: &[&Text], limit: Option<usize>) -> Vec<Cut> {
    let mut atoms: Vec<Atom> = Vec::new();
    for (piece, text) in texts.iter().enumerate() {
        let size = |units: RangeInclusive<usize>| -> usize {
            units.map(|u| text.units[u].range().len()).sum()
        };
        for sentence in text.sentences() {
            let syllables = size(sentence.clone());
            if limit.is_some_and(|limit| syllables > limit) {
                atoms.extend(sentence.map(|u| Atom {
                    piece,
                    units: u..=u,
                    syllables: size(u..=u),
                }));
            } else {
                atoms.push(Atom {
                    piece,
                    units: sentence,
                    syllables,
                });
            }
        }
    }
    let sizes: Vec<usize> = atoms.iter().map(|atom| atom.syllables).collect();
    let groups: Vec<Range<usize>> = match limit {
        Some(limit) => partition(&sizes, limit),
        None => std::iter::once(0..atoms.len()).collect(),
    };
    groups
        .into_iter()
        .enumerate()
        .map(|(g, group)| {
            let first = &atoms[group.start];
            Cut {
                spec: spec_of(&atoms[group.clone()], pieces, texts),
                syllables: sizes[group].iter().sum(),
                label: if g == 0 { pieces[0].label } else { None },
                begins: (pieces[first.piece].id, *first.units.start()),
            }
        })
        .collect()
}

/// The spec of a part of consecutive atoms: for every text it touches its id, with the range of
/// its units counted from 1 where it is not whole (`credo:5-7`, `gloria:7`), joined by `+`.
fn spec_of(atoms: &[Atom], pieces: &[Piece], texts: &[&Text]) -> String {
    let mut spans: Vec<(usize, usize, usize)> = Vec::new();
    for atom in atoms {
        let (first, last) = (*atom.units.start(), *atom.units.end());
        match spans.last_mut() {
            Some((piece, _, end)) if *piece == atom.piece => *end = last,
            _ => spans.push((atom.piece, first, last)),
        }
    }
    let specs: Vec<String> = spans
        .into_iter()
        .map(|(piece, first, last)| {
            let id = pieces[piece].id;
            if first == 0 && last + 1 == texts[piece].units.len() {
                id.to_string()
            } else if first == last {
                format!("{id}:{}", first + 1)
            } else {
                format!("{id}:{}-{}", first + 1, last + 1)
            }
        })
        .collect();
    specs.join("+")
}

/// The groups of consecutive atoms of `sizes` syllables for parts of at most `limit` syllables:
/// as few as possible; of those the groups whose largest is least, then whose sizes vary least
/// (the least sum of squares), then whose cuts come first. An atom longer than the limit (a unit
/// of more syllables, which the Ordinary does not have) makes every atom a group of its own.
fn partition(sizes: &[usize], limit: usize) -> Vec<Range<usize>> {
    let n = sizes.len();
    let mut prefix = vec![0; n + 1];
    for (i, &size) in sizes.iter().enumerate() {
        prefix[i + 1] = prefix[i] + size;
    }
    let size = |i: usize, j: usize| prefix[j] - prefix[i];
    // largest[k][j]: the least largest group of the first j atoms in k groups.
    let mut largest = vec![vec![usize::MAX; n + 1]; n + 1];
    largest[0][0] = 0;
    for k in 1..=n {
        for j in k..=n {
            largest[k][j] = (k - 1..j)
                .filter(|&i| largest[k - 1][i] != usize::MAX)
                .map(|i| largest[k - 1][i].max(size(i, j)))
                .min()
                .unwrap_or(usize::MAX);
        }
        if largest[k][n] <= limit {
            return evenly(&size, n, k, largest[k][n]);
        }
    }
    (0..n).map(|i| i..i + 1).collect()
}

/// The `k` groups of `n` atoms of at most `bound` syllables each whose sizes vary least, of
/// several such the one that cuts first.
fn evenly(
    size: &dyn Fn(usize, usize) -> usize,
    n: usize,
    k: usize,
    bound: usize,
) -> Vec<Range<usize>> {
    let square = |s: usize| (s * s) as u64;
    // spread[r][i]: the least sum of squares of the atoms from i on in r groups.
    let mut spread = vec![vec![u64::MAX; n + 1]; k + 1];
    spread[0][n] = 0;
    let step = |spread: &[Vec<u64>], r: usize, i: usize, j: usize| -> Option<u64> {
        (size(i, j) <= bound && spread[r - 1][j] != u64::MAX)
            .then(|| square(size(i, j)) + spread[r - 1][j])
    };
    for r in 1..=k {
        for i in (0..n).rev() {
            spread[r][i] = (i + 1..=n)
                .filter_map(|j| step(&spread, r, i, j))
                .min()
                .unwrap_or(u64::MAX);
        }
    }
    let mut groups = Vec::with_capacity(k);
    let mut i = 0;
    for r in (1..=k).rev() {
        let j = (i + 1..=n)
            .find(|&j| step(&spread, r, i, j) == Some(spread[r][i]))
            .expect("the least largest group leaves a way through");
        groups.push(i..j);
        i = j;
    }
    groups
}

/// The mode of a movement: the request's, else with a chant paraphrase the mode its chant is
/// numbered in (D12), else the Dorian.
fn movement_mode(request: &MassRequest, movement: Movement) -> ModeSpec {
    let dorian = ModeSpec::new(Mode::Dorian);
    if let Some(mode) = request.mode {
        return mode;
    }
    let Some(chant) = request.chant else {
        return dorian;
    };
    let kyriale = match (chant, movement) {
        (ChantMass::IV, Movement::Kyrie) => Kyriale::KyrieIV,
        (_, Movement::Kyrie) => Kyriale::KyrieIX,
        (_, Movement::Gloria) => Kyriale::GloriaIX,
        (_, Movement::Credo) => Kyriale::CredoI,
        (_, Movement::Sanctus | Movement::Benedictus) => Kyriale::SanctusIX,
        (_, Movement::Agnus) => Kyriale::AgnusIX,
    };
    kyriale.chant().numbered_mode().unwrap_or(dorian)
}

/// What a part keeps for its draws besides the plan.
#[derive(Clone, Debug)]
struct Extra {
    kind: EndingKind,
    /// Where its text begins: the id of a text of the Ordinary and a unit of it, from 0.
    begins: (&'static str, usize),
    /// The ending `--endings` gives.
    ending: Option<Step>,
    /// Whether it begins in imitation, as `--imitation` says.
    toggle: Toggle,
    /// The points the imitation plans by itself, where the part gets more (MASS-LENGTH).
    natural: Option<usize>,
    /// Its measures without imitation: the length its text plans, after its start.
    plain: f64,
    /// Its measures with the points the imitation plans by itself.
    imitative: f64,
}

/// A plan before its draws: the movements with their sections, voices and parts, the parts'
/// texts and joins, and the options applied.
pub(crate) struct Draft {
    /// The plan, its draws still open: every part ends on the final and begins in imitation.
    plan: MassPlan,
    /// What each part keeps for its draws, by movement, section and part.
    extras: Vec<Vec<Vec<Extra>>>,
    /// Per movement and section: it begins together where a section otherwise imitates.
    tutti: Vec<Vec<bool>>,
    /// Per movement and section: the opening chord of a part that begins together.
    chords: Vec<Vec<Vec<Pitch>>>,
    /// The request's choices that the draws mix into the seed (preset, mode and choir).
    key: u64,
}

/// Checks what a request asks for as a whole, before its parts are planned.
fn validate(request: &MassRequest) -> Result<(), MassOptionError> {
    let voices = request.choir.len();
    if !(2..=6).contains(&voices) {
        return Err(MassOptionError::Voices { voices });
    }
    if request.movements.is_empty() {
        return Err(MassOptionError::NoMovement);
    }
    for (i, &movement) in request.movements.iter().enumerate() {
        if request.movements[..i].contains(&movement) {
            return Err(MassOptionError::MovementTwice { movement });
        }
    }
    if let Some(chunk) = request.chunk
        && !tables::CHUNK_RANGE.contains(&chunk)
    {
        return Err(MassOptionError::Chunk {
            given: chunk.to_string(),
        });
    }
    if request.motif && request.chant.is_some() {
        return Err(MassOptionError::MotifWithChant);
    }
    if request.motif && !request.movements.contains(&Movement::Kyrie) {
        return Err(MassOptionError::MotifWithoutKyrie);
    }
    let repeats_sanctus = request.movements.iter().any(|&movement| {
        preset::form(request.preset, movement)
            .sections
            .iter()
            .any(|section| section.ut_supra.is_some())
    });
    if repeats_sanctus && !request.movements.contains(&Movement::Sanctus) {
        return Err(MassOptionError::UtSupraWithoutSanctus);
    }
    Ok(())
}

impl Draft {
    /// The plan of `request` up to its draws: its forms, the voices of its sections, the cut of
    /// its texts into parts, the options applied and checked, and what the estimates need.
    pub(crate) fn new(request: &MassRequest) -> Result<Draft, MassOptionError> {
        validate(request)?;
        let options = &request.text;
        let mut choir = request.choir.clone();
        choir.sort();
        let movements: Vec<Movement> = Movement::ALL
            .into_iter()
            .filter(|m| request.movements.contains(m))
            .collect();
        let mut plan = MassPlan {
            title: request
                .title
                .clone()
                .unwrap_or_else(|| request.preset.title().to_string()),
            principal: Step::D,
            movements: Vec::new(),
            preset: request.preset,
            mode: mode_key(request),
            choir: choir.clone(),
            seed: 0,
            text: request.text,
            accents: request.accents,
            chant: request.chant,
            motif: request.motif,
        };
        let mut extras: Vec<Vec<Vec<Extra>>> = Vec::new();
        let mut tutti: Vec<Vec<bool>> = Vec::new();
        // The voice types of every section, by movement.
        let mut types: Vec<Vec<Vec<VoiceType>>> = Vec::new();
        for &movement in &movements {
            let form = preset::form(request.preset, movement);
            let mode = movement_mode(request, movement);
            let mut sections = Vec::new();
            let mut section_extras = Vec::new();
            for (s, section) in form.sections.iter().enumerate() {
                let cuts = match section.ut_supra {
                    Some(_) => Vec::new(),
                    None => cut_section(section.pieces, request.chunk, options),
                };
                let count = cuts.len();
                let mut parts = Vec::with_capacity(count);
                let mut part_extras = Vec::with_capacity(count);
                for (p, cut) in cuts.into_iter().enumerate() {
                    let join = match (s, p, cut.label) {
                        (0, 0, _) => Join::Start,
                        (_, 0, _) => Join::Section,
                        (_, _, Some(label)) => Join::Break(label),
                        _ => Join::Seam,
                    };
                    parts.push(PartPlan {
                        text: cut.spec,
                        syllables: cut.syllables,
                        join,
                        ending: mode.final_step(),
                        opening: Opening::Imitative,
                        start: join.start(),
                        chord: None,
                        chain: CadenceChain::Rotation,
                        continued: p + 1 < count,
                        points: None,
                        motif: false,
                        seed: 0,
                        estimate: Estimate::default(),
                    });
                    part_extras.push(Extra {
                        kind: EndingKind::Other,
                        begins: cut.begins,
                        ending: None,
                        toggle: Toggle::Auto,
                        natural: None,
                        plain: 0.0,
                        imitative: 0.0,
                    });
                }
                sections.push(SectionPlan {
                    id: section.id,
                    title: section.title,
                    voices: Vec::new(),
                    parts,
                    ut_supra: None,
                });
                section_extras.push(part_extras);
            }
            types.push(
                form.sections
                    .iter()
                    .map(|section| tables::scoring(&choir, section.scoring))
                    .collect(),
            );
            tutti.push(form.sections.iter().map(|section| section.tutti).collect());
            extras.push(section_extras);
            plan.movements.push(MovementPlan {
                movement,
                mode,
                principal: mode.final_step(),
                voices: Vec::new(),
                names: Vec::new(),
                intonation: form.intonation,
                notes: form.notes.to_vec(),
                sections,
            });
        }
        // „Osanna ut supra“ points at the section of the Sanctus it repeats.
        for m in 0..plan.movements.len() {
            let form = preset::form(request.preset, plan.movements[m].movement);
            for (s, section) in form.sections.iter().enumerate() {
                let Some(id) = section.ut_supra else {
                    continue;
                };
                let repeated = plan.movements.iter().enumerate().find_map(|(n, movement)| {
                    let found = movement.sections.iter().position(|other| other.id == id);
                    found.map(|t| (n, t))
                });
                plan.movements[m].sections[s].ut_supra = repeated;
            }
        }
        let mut draft = Draft {
            plan,
            extras,
            tutti,
            chords: Vec::new(),
            key: fingerprint(&format!(
                "mass|{}|{}|{}",
                request.preset.id(),
                mode_key(request),
                choir
                    .iter()
                    .map(|voice| voice.abbreviation())
                    .collect::<Vec<_>>()
                    .join(",")
            )),
        };
        // The scoring first: the endings of two-voice parts depend on it.
        for change in &request.overrides {
            if let Override::Scoring { at, voices } = change {
                let Target::Section(m, s) = draft.plan.target(at)? else {
                    return Err(MassOptionError::ScoringAddress { at: at.clone() });
                };
                if draft.plan.movements[m].sections[s].ut_supra.is_some() {
                    return Err(MassOptionError::UtSupra { at: at.clone() });
                }
                if !(2..=6).contains(&voices.len()) {
                    return Err(MassOptionError::ScoringVoices {
                        at: at.clone(),
                        voices: voices.len(),
                    });
                }
                let mut voices = voices.clone();
                voices.sort();
                types[m][s] = voices;
            }
        }
        draft.assign_voices(&types);
        draft.assign_kinds();
        for change in &request.overrides {
            match change {
                Override::Scoring { .. } => {}
                Override::Imitation { at, toggle } => {
                    for part in draft.parts_of(at, false)? {
                        draft.extra_mut(part).toggle = *toggle;
                    }
                }
                Override::Ending { at, step } => {
                    let parts = draft.parts_of(at, true)?;
                    for &part in &parts {
                        draft.check_ending(at, part, *step)?;
                        draft.extra_mut(part).ending = Some(*step);
                    }
                }
            }
        }
        draft.prepare_estimates(request);
        Ok(draft)
    }

    /// The voices of every movement as the union of its sections', and of every section as
    /// indices into them: the k-th voice of a type in a section is the k-th of that type in the
    /// movement (Tenor 1, Tenor 2). Every section gets the opening chord of its voices.
    fn assign_voices(&mut self, types: &[Vec<Vec<VoiceType>>]) {
        self.chords.clear();
        for (m, movement) in self.plan.movements.iter_mut().enumerate() {
            // An Osanna ut supra has the voices of the Osanna it repeats.
            let of = |s: usize| -> &Vec<VoiceType> {
                match movement.sections[s].ut_supra {
                    Some((n, t)) => &types[n][t],
                    None => &types[m][s],
                }
            };
            let sections: Vec<Vec<VoiceType>> = (0..movement.sections.len())
                .map(|s| of(s).clone())
                .collect();
            let voices = movement_voices(&sections);
            for (section, kinds) in movement.sections.iter_mut().zip(&sections) {
                section.voices = voice_places(&voices, kinds)
                    .expect("the movement has every voice of its sections");
            }
            let mode = movement.mode;
            self.chords.push(
                sections
                    .iter()
                    .map(|kinds| opening_chord(mode, kinds, mode.final_step()))
                    .collect(),
            );
            movement.names = voice_names(&voices, Lang::De);
            movement.voices = voices;
        }
    }

    /// The kind of every part's ending, by its place in its movement (§ 3.5).
    fn assign_kinds(&mut self) {
        for (m, movement) in self.plan.movements.iter().enumerate() {
            let places: Vec<(usize, usize)> = movement
                .sections
                .iter()
                .enumerate()
                .flat_map(|(s, section)| (0..section.parts.len()).map(move |p| (s, p)))
                .collect();
            let ends_repeated = movement
                .sections
                .last()
                .is_some_and(|section| section.ut_supra.is_some());
            for (i, &(s, p)) in places.iter().enumerate() {
                let section = &movement.sections[s];
                let closes = p + 1 == section.parts.len();
                let next = places.get(i + 1).map(|&(t, q)| self.extras[m][t][q].begins);
                let kind = if next.is_none() && !ends_repeated {
                    EndingKind::Last
                } else if closes && section.id == "kyrie" {
                    EndingKind::KyrieI
                } else if closes && section.id == "christe" {
                    EndingKind::Christe
                } else if closes && section.id == "agnus" {
                    EndingKind::AgnusI
                } else if closes && section.id == "benedictus" {
                    match movement.sections.get(s + 1) {
                        Some(after) if after.ut_supra.is_some() => EndingKind::BenedictusUtSupra,
                        _ => EndingKind::BenedictusBeforeOsanna,
                    }
                } else if next == Some(("qui-tollis", 0)) {
                    EndingKind::BeforeQuiTollis
                } else {
                    EndingKind::Other
                };
                self.extras[m][s][p].kind = kind;
            }
        }
    }

    /// The parts an option at `at` changes: every part of a section, or with `ending` its last,
    /// whose ending is the section's.
    fn parts_of(&self, at: &str, ending: bool) -> Result<Vec<PartRef>, MassOptionError> {
        Ok(match self.plan.target(at)? {
            // An ending names a part or a section, never the whole mass.
            Target::All if ending => return Err(self.plan.unknown(at)),
            Target::All => self.plan.parts().map(|(at, _)| at).collect(),
            Target::Part(part) => vec![part],
            Target::Section(m, s) => {
                let section = &self.plan.movements[m].sections[s];
                if section.ut_supra.is_some() {
                    return Err(MassOptionError::UtSupra { at: at.into() });
                }
                let mut parts = (0..section.parts.len()).map(|p| PartRef {
                    movement: m,
                    section: s,
                    part: p,
                });
                if ending {
                    parts.next_back().into_iter().collect()
                } else {
                    parts.collect()
                }
            }
        })
    }

    fn extra_mut(&mut self, at: PartRef) -> &mut Extra {
        &mut self.extras[at.movement][at.section][at.part]
    }

    /// Checks an ending that `--endings` gives for a part: a degree with a clausula, the final,
    /// the fifth or the fourth, not for the last part of a movement, and the final in two
    /// voices.
    fn check_ending(&self, at: &str, part: PartRef, step: Step) -> Result<(), MassOptionError> {
        let movement = &self.plan.movements[part.movement];
        let mode = movement.mode;
        let at = at.to_string();
        if mode.clausula(step).is_none() {
            return Err(MassOptionError::EndingClausula { at, step, mode });
        }
        if !part_endings(mode).contains(&step) {
            return Err(MassOptionError::EndingDegree { at, step, mode });
        }
        if self.extras[part.movement][part.section][part.part].kind == EndingKind::Last {
            if step == mode.final_step() {
                return Ok(());
            }
            return Err(MassOptionError::EndingLast {
                at,
                movement: movement.movement,
            });
        }
        let voices = movement.sections[part.section].voices.len();
        if voices == 2 && step != mode.final_step() {
            return Err(MassOptionError::EndingTwoVoices { at });
        }
        Ok(())
    }

    /// The points the imitation plans by itself where a part gets more (MASS-LENGTH), and the
    /// measures of every part with and without imitation (§ 3.10).
    fn prepare_estimates(&mut self, request: &MassRequest) {
        let restates = request.repeat.restates();
        let lengthened = request.lengths == Lengths::Palestrina;
        for (m, movement) in self.plan.movements.iter().enumerate() {
            let mode = movement.mode;
            for (s, section) in movement.sections.iter().enumerate() {
                let voices = section.voices.len();
                for (p, part) in section.parts.iter().enumerate() {
                    let text = part_text(&part.text, &request.text)
                        .unwrap_or_else(|error| panic!("the parts' texts resolve: {error}"));
                    let planned = plan(&text, mode, voices, None, None, restates).ok();
                    let automatic = automatic_length(&text, restates);
                    let extra = &mut self.extras[m][s][p];
                    let length = planned
                        .as_ref()
                        .map_or(automatic, |planned| planned.measures);
                    extra.plain = (length + part.start.div_ceil(8) as usize) as f64;
                    // How much longer the imitation makes it, by the pace of its text.
                    let longer = match text.kind {
                        TextKind::Long => tables::ESTIMATE_IMITATIVE,
                        TextKind::Short if melismatic(&text) => tables::ESTIMATE_MELISMATIC,
                        TextKind::Short => tables::ESTIMATE_SHORT,
                    };
                    extra.imitative = automatic as f64 * longer;
                    if lengthened && tables::LENGTHENED.contains(&section.id) {
                        let statements = planned.map_or(1, |planned| planned.statements);
                        extra.natural = natural_points(&text, statements, restates);
                    }
                }
            }
        }
    }

    /// The plan with the draws of `seed`.
    pub(crate) fn draw(&self, seed: u64) -> MassPlan {
        let mut plan = self.plan.clone();
        plan.seed = seed;
        let mass = splitmix64(seed ^ self.key);
        let principal_draw = Rng::new(mass).next_f64();
        for (m, movement) in plan.movements.iter_mut().enumerate() {
            let mode = movement.mode;
            let principal = tables::principal(mode, principal_draw);
            movement.principal = principal;
            let weights = tables::chain_weights(mode, principal);
            let voices = movement.voices.clone();
            // The movement's own generator, so that it plans the same whichever other movements
            // are set.
            let mut rng = Rng::new(splitmix64(mass ^ movement_key(movement.movement)));
            // The ending of the part before as planned, from which the next part's chain starts.
            let mut before: Option<Step> = None;
            // The ending of the part before as drawn, without `--endings`, and the shares it was
            // drawn from: the redraws run on these, so that an ending that `--endings` gives
            // changes no other part's draw.
            let mut drawn_before: Option<Step> = None;
            let mut drawn_from: Vec<(Step, f64)> = Vec::new();
            for (s, section) in movement.sections.iter_mut().enumerate() {
                let types: Vec<VoiceType> = section.voices.iter().map(|&i| voices[i]).collect();
                let christe = section.id == "christe";
                for (p, part) in section.parts.iter_mut().enumerate() {
                    let extra = &self.extras[m][s][p];
                    let previous = before;
                    let (first, again, restart) = (rng.next_f64(), rng.next_f64(), rng.next_f64());
                    let shares = if extra.kind == EndingKind::Last || types.len() == 2 {
                        vec![(mode.final_step(), 1.0)]
                    } else {
                        tables::ending_distribution(extra.kind, mode, principal)
                    };
                    // Within a section an ending equal to the one before is drawn once more,
                    // from shares that keep those of the table.
                    let redraw = p > 0 && shares.len() > 1;
                    let raised = if redraw {
                        tables::redrawn(&shares, &drawn_from)
                    } else {
                        Vec::new()
                    };
                    let against = |last: Option<Step>| match last {
                        Some(last) if redraw => match tables::pick(&raised, first) {
                            drawn if drawn == last => tables::pick(&raised, again),
                            drawn => drawn,
                        },
                        _ => tables::pick(&shares, first),
                    };
                    let drawn = against(drawn_before);
                    part.ending = match extra.ending {
                        Some(step) => step,
                        // After an ending that `--endings` changed, an ending equal to it is drawn
                        // once more, as against the ending before as planned; every other part
                        // ends as drawn.
                        None if before != drawn_before && before == Some(drawn) => against(before),
                        None => drawn,
                    };
                    before = Some(part.ending);
                    drawn_before = Some(drawn);
                    drawn_from = shares;
                    part.opening = match extra.toggle {
                        Toggle::On => Opening::Imitative,
                        Toggle::Off => Opening::Tutti,
                        Toggle::Auto => restart_of(
                            plan.preset,
                            part.join,
                            extra.begins,
                            self.tutti[m][s],
                            types.len(),
                            restart,
                        ),
                    };
                    let tutti = part.opening == Opening::Tutti;
                    part.chord = tutti.then(|| self.chords[m][s].clone());
                    part.points = match extra.natural {
                        Some(natural) if !tutti => Some(natural + tables::EXTRA_POINTS),
                        _ => None,
                    };
                    part.motif = plan.motif
                        && !tutti
                        && part.join == Join::Start
                        && matches!(
                            movement.movement,
                            Movement::Gloria
                                | Movement::Credo
                                | Movement::Sanctus
                                | Movement::Agnus
                        );
                    let place = ((movement.movement as u64) << 16) | ((s as u64) << 8) | p as u64;
                    part.seed = splitmix64(seed ^ self.key ^ place);
                    part.chain = CadenceChain::Weighted {
                        seed: part.seed,
                        weights: weights.clone(),
                        before: previous,
                        christe: christe && p == 0,
                    };
                    let measures = match (tutti, part.points, extra.natural) {
                        (true, _, _) => extra.plain,
                        (false, Some(points), Some(natural)) => {
                            let more = points.saturating_sub(natural) as f64;
                            extra.imitative + tables::ESTIMATE_POINT * more
                        }
                        _ => extra.imitative,
                    };
                    part.estimate = Estimate {
                        measures: measures.round() as usize,
                        seconds: tables::estimate_seconds(measures, types.len(), tutti) as f32,
                    };
                }
            }
        }
        if let Some(first) = plan.movements.first() {
            plan.principal = first.principal;
        }
        plan
    }
}

/// What a movement mixes into the seed of its generator: its place in the order of the mass,
/// from 1, in the upper half of the number.
fn movement_key(movement: Movement) -> u64 {
    (movement as u64 + 1) << 32
}

/// How a part of a mass of the form `preset` begins by the restart table (MASS-RESTART, § 3.7),
/// with `draw` in [0, 1).
fn restart_of(
    preset: Preset,
    join: Join,
    begins: (&'static str, usize),
    tutti: bool,
    voices: usize,
    draw: f64,
) -> Opening {
    let imitative = |p: f64| {
        if draw < p {
            Opening::Imitative
        } else {
            Opening::Tutti
        }
    };
    if !(3..=5).contains(&voices) {
        return Opening::Tutti;
    }
    match join {
        Join::Start | Join::Section if tutti => Opening::Tutti,
        Join::Start | Join::Section => Opening::Imitative,
        Join::Break(_) => Opening::Tutti,
        Join::Seam => match begins {
            ("resurrexit", 0) => Opening::Tutti,
            ("spiritum", 0) => imitative(tables::RESTART_SPIRITUM),
            _ => imitative(tables::restart_seam(preset)),
        },
    }
}

/// The points of imitation a part on `text` gets by itself, where the text has room for
/// [`tables::EXTRA_POINTS`] more; `None` where it has not, as without restatements.
fn natural_points(text: &Text, statements: usize, restates: bool) -> Option<usize> {
    let mut rng = Rng::new(0);
    let natural = points_of(text, statements, restates, &Imitation::default(), &mut rng)
        .ok()?
        .len();
    let more = Imitation {
        points: Some(natural + tables::EXTRA_POINTS),
        ..Imitation::default()
    };
    points_of(text, statements, restates, &more, &mut rng).ok()?;
    Some(natural)
}

/// The mode of a request as its draws read it: its id, the chant mass whose modes the movements
/// take, or the Dorian.
fn mode_key(request: &MassRequest) -> String {
    match (request.mode, request.chant) {
        (Some(mode), _) => mode.id(),
        (None, Some(chant)) => format!("kyriale-{}", chant.id()),
        (None, None) => ModeSpec::new(Mode::Dorian).id(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{FreeRequest, PartContext};
    use crate::mass::{Preset, choir};
    use crate::score::Cadence;
    use crate::text::plan::{CHAIN_REPEAT, Ending, internal_goals, plan_part, targets_with};
    use crate::text::{Stop, SylFlags};

    fn letters(steps: impl IntoIterator<Item = Step>) -> String {
        steps.into_iter().map(|s| s.letter()).collect()
    }

    /// How the words of a point end, as the imitation's planner reads them for the target of the
    /// point's cadence: at the end of a unit by its stop, elsewhere at a marked break.
    fn point_ending(text: &Text, words: &Range<u16>) -> Ending {
        let last = usize::from(words.end.saturating_sub(1));
        if text.syllables[last].flags.contains(SylFlags::UNIT_END) {
            match text.unit_of(last).stop {
                Stop::Full => Ending::Full,
                Stop::Intermediate => Ending::Intermediate,
            }
        } else {
            Ending::Break
        }
    }

    /// A part that begins together as its goals depend on it: its text, voices, start and
    /// ending.
    type Together = (String, usize, u32, Step);

    /// The goals of the internal cadences of the parts of plans, as their settings plan them,
    /// with what they need from the parts' texts kept for the next plan.
    #[derive(Default)]
    struct Goals {
        /// The goals of the parts that begin together.
        together: Vec<(Together, Vec<(usize, Ending)>)>,
        /// The statements of a text, by its spec, voices and mode.
        statements: Vec<((String, usize, String), usize)>,
    }

    impl Goals {
        /// Where the internal cadences of the part at `at` of `plan`, a plan of `request`,
        /// arrive and how the words end there, in their order. A part that begins together has
        /// the goals its text plans with its start and ending (`plan_part`); an imitative part
        /// has a goal at the end of the words of every point but the last, which the final
        /// cadence ends, the points as its imitation lays them out (`points_of`; a restatement
        /// the imitation draws of its own comes last, so the generator that draws it changes no
        /// goal). The targets come from the part's chain ([`targets_with`]); the measures do not
        /// change them.
        fn of(
            &mut self,
            plan: &MassPlan,
            at: PartRef,
            request: &MassRequest,
        ) -> Vec<(usize, Ending)> {
            let movement = &plan.movements[at.movement];
            let mode = movement.mode;
            let voices = movement.sections[at.section].voices.len();
            let part = plan.part(at);
            let restates = request.repeat.restates();
            let text = part.sung_text(&request.text);
            if part.opening == Opening::Imitative {
                let key = (part.text.clone(), voices, mode.id());
                let statements = match self.statements.iter().find(|(k, _)| *k == key) {
                    Some(&(_, statements)) => statements,
                    None => {
                        let planned =
                            crate::text::plan::plan(&text, mode, voices, None, None, restates)
                                .expect("the parts' texts plan");
                        self.statements.push((key, planned.statements));
                        planned.statements
                    }
                };
                let imitation = Imitation {
                    points: part.points,
                    ..Imitation::default()
                };
                let mut rng = Rng::new(part.seed);
                let points = points_of(&text, statements, restates, &imitation, &mut rng)
                    .expect("the planned points exist");
                return points[..points.len() - 1]
                    .iter()
                    .enumerate()
                    .map(|(i, point)| (i, point_ending(&text, &point.words)))
                    .collect();
            }
            let key = (part.text.clone(), voices, part.start, part.ending);
            if let Some((_, goals)) = self.together.iter().find(|(k, _)| *k == key) {
                return goals.clone();
            }
            let context = PartContext {
                ending: Some(part.ending),
                start: part.start,
                chord: part.chord.clone(),
                chain: part.chain.clone(),
            };
            let planned = plan_part(&text, mode, voices, None, None, restates, &context)
                .expect("the parts' texts plan");
            let goals = internal_goals(
                &text,
                planned.measures,
                planned.statements,
                part.start,
                context.first_goal(),
            );
            self.together.push((key, goals.clone()));
            goals
        }
    }

    /// The steps above the final of `mode` of a degree.
    fn steps_of(mode: ModeSpec, step: Step) -> i32 {
        (step.index() - mode.final_step().index()).rem_euclid(7)
    }

    /// A part of a plan as its chain draws its internal cadences: its mode, the goals of its
    /// cadences and its chain.
    type Drawn = (ModeSpec, Vec<(usize, Ending)>, CadenceChain);

    /// The internal cadences of `parts` by steps above the final, drawn by their chains with the
    /// weights `row` (by steps above the final) instead of their own, and how often a cadence
    /// repeats the degree before it, of the cadences after another.
    fn cadences_with(parts: &[Drawn], row: &[(i32, f64)]) -> (Vec<(i32, f64)>, f64) {
        let mut counts: Vec<(i32, f64)> = row.iter().map(|&(steps, _)| (steps, 0.0)).collect();
        let (mut repeats, mut followers) = (0.0, 0.0);
        let total: f64 = row.iter().map(|&(_, w)| w).sum();
        for (mode, goals, chain) in parts {
            let CadenceChain::Weighted {
                seed,
                before,
                christe,
                ..
            } = chain
            else {
                unreachable!("mass parts have weighted chains")
            };
            let scaled: Vec<(i32, u16)> = row
                .iter()
                .map(|&(steps, w)| (steps, (w / total * 60_000.0).round() as u16))
                .collect();
            let chain = CadenceChain::Weighted {
                seed: *seed,
                weights: tables::degrees_of(*mode, &scaled),
                before: *before,
                christe: *christe,
            };
            let mut previous = *before;
            for Cadence { target, .. } in targets_with(*mode, goals.clone(), &chain) {
                let steps = steps_of(*mode, target);
                match counts.iter_mut().find(|(s, _)| *s == steps) {
                    Some((_, n)) => *n += 1.0,
                    None => counts.push((steps, 1.0)),
                }
                if let Some(step) = previous {
                    followers += 1.0;
                    repeats += f64::from(u8::from(step == target));
                }
                previous = Some(target);
            }
        }
        let all: f64 = counts.iter().map(|c| c.1).sum();
        for count in &mut counts {
            count.1 /= all;
        }
        (counts, repeats / followers)
    }

    /// The parts of `plans` plans per form (brevis and normal), choir (three to five voices)
    /// and mode of `modes`, with the principal degree `principal` where given, as their chains
    /// draw their cadences.
    fn drawn_parts(modes: &[ModeSpec], principal: Option<i32>, plans: u64) -> Vec<Drawn> {
        let mut parts = Vec::new();
        for &mode in modes {
            for preset in [Preset::Brevis, Preset::Normal] {
                for voices in 3..=5 {
                    let request = MassRequest {
                        preset,
                        mode: Some(mode),
                        choir: choir(voices),
                        ..MassRequest::default()
                    };
                    let draft = Draft::new(&request).unwrap();
                    let mut goals = Goals::default();
                    let mut seed = 0;
                    let mut taken = 0;
                    while taken < plans {
                        seed += 1;
                        let plan = draft.draw(seed);
                        if principal.is_some_and(|p| steps_of(mode, plan.principal) != p) {
                            continue;
                        }
                        taken += 1;
                        for (at, part) in plan.parts() {
                            let goals = goals.of(&plan, at, &request);
                            parts.push((mode, goals, part.chain.clone()));
                        }
                    }
                }
            }
        }
        parts
    }

    /// The weights of the chains, compensated (WP15, MASS-CADENCES): for each row of
    /// [`tables::chain_weights`], the weights with which the cadences of the plans come out
    /// with the shares of the corpus ([`tables::corpus_weights`]), found by drawing the cadences
    /// of 300 plans per form, choir of three to five voices and mode with weights raised where
    /// they come out below the corpus and lowered where above (each weight times its share in
    /// the corpus over its share drawn, 60 rounds), then rounded to per mille. Prints the rows
    /// with the shares the corpus weights and the compensated ones give:
    /// `cargo test --release -p fux-core --lib -- --ignored --nocapture calibrate_chain`.
    #[test]
    #[ignore = "calibration: release, a minute"]
    fn calibrate_chain_weights() {
        const PLANS: u64 = 300;
        let rows: [(&str, &[&str], Option<i32>); 10] = [
            ("dorian", &["dorian"], None),
            ("dorian on G", &["dorian-transposed"], None),
            ("phrygian", &["phrygian", "phrygian-transposed"], None),
            (
                "mixolydian, P C",
                &["mixolydian", "mixolydian-transposed"],
                Some(3),
            ),
            (
                "mixolydian, P D",
                &["mixolydian", "mixolydian-transposed"],
                Some(4),
            ),
            ("aeolian", &["aeolian"], None),
            ("aeolian on D", &["aeolian-transposed"], None),
            ("ionian", &["ionian"], None),
            ("ionian on F", &["ionian-transposed"], None),
            ("lydian", &["lydian", "lydian-transposed"], None),
        ];
        for (name, modes, principal) in rows {
            let modes: Vec<ModeSpec> = modes.iter().map(|m| m.parse().unwrap()).collect();
            let mode = modes[0];
            let p = principal.map_or(mode.final_step(), |steps| {
                Step::from_index(mode.final_step().index() + steps)
            });
            let corpus: Vec<(i32, f64)> = tables::corpus_weights(mode, p)
                .into_iter()
                .map(|(step, w)| (steps_of(mode, step), f64::from(w)))
                .collect();
            let total: f64 = corpus.iter().map(|c| c.1).sum();
            let wanted: Vec<(i32, f64)> = corpus.iter().map(|&(s, w)| (s, w / total)).collect();
            let parts = drawn_parts(&modes, principal, PLANS);
            let share = |list: &[(i32, f64)], steps: i32| {
                list.iter().find(|c| c.0 == steps).map_or(0.0, |c| c.1)
            };
            let mut row = wanted.clone();
            for _ in 0..60 {
                let (drawn, _) = cadences_with(&parts, &row);
                for (steps, w) in &mut row {
                    *w *= (share(&wanted, *steps) / share(&drawn, *steps).max(1e-6)).powf(0.8);
                }
                let sum: f64 = row.iter().map(|r| r.1).sum();
                for r in &mut row {
                    r.1 /= sum;
                }
            }
            let per_mille: Vec<(i32, f64)> = row
                .iter()
                .map(|&(s, w)| (s, (w * 1000.0).round().max(1.0)))
                .collect();
            let (before, repeated_before) = cadences_with(&parts, &corpus);
            let (after, repeated) = cadences_with(&parts, &per_mille);
            let names = |steps: i32| Step::from_index(mode.final_step().index() + steps).letter();
            println!(
                "{name}: {} parts, repeats {repeated_before:.3} / {repeated:.3}",
                parts.len()
            );
            for &(steps, w) in &wanted {
                println!(
                    "  {} corpus {:.3}, drawn with the corpus weights {:.3}, compensated {:>4} → {:.3}",
                    names(steps),
                    w,
                    share(&before, steps),
                    share(&per_mille, steps),
                    share(&after, steps)
                );
            }
            println!(
                "  row: {}",
                per_mille
                    .iter()
                    .map(|&(s, w)| format!("({s}, {w})"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    /// The goals of [`Goals`] are those of the settings: the cadences that the chains draw at
    /// them are the cadences the searches of the parts are given, those of the text's plan for a
    /// part that begins together and those of the imitation's planner for an imitative one, its
    /// first attempt (seed `splitmix64(part seed)`). Counts the parts of 10 plans per mode and
    /// form in four voices whose cadences agree, and the shares of their degrees both ways; all
    /// 12 720 parts of 20 plans agreed when the weights were calibrated (phase 14a):
    /// `cargo test --release -p fux-core --lib -- --ignored --nocapture goals_are_those`.
    #[test]
    #[ignore = "release: plans the imitation of 3000 parts, about ten minutes"]
    fn goals_are_those_of_the_settings() {
        use crate::imitation::planner::plan_imitation;
        use crate::mass::{part_profile, part_request};
        let profile = crate::mass::profile(&crate::rules::Profile::fux());
        let (mut parts, mut alike) = (0, 0);
        // The cadences by steps above the final, as the chains draw them and as the searches
        // are given them.
        let mut simulated: Vec<(i32, f64)> = Vec::new();
        let mut set: Vec<(i32, f64)> = Vec::new();
        let add = |list: &mut Vec<(i32, f64)>, steps: i32| match list
            .iter_mut()
            .find(|(s, _)| *s == steps)
        {
            Some((_, n)) => *n += 1.0,
            None => list.push((steps, 1.0)),
        };
        for m in Mode::ALL {
            for transposed in [false, true] {
                let mode = ModeSpec::new(m).with_transposition(transposed);
                for preset in [Preset::Brevis, Preset::Normal] {
                    let request = MassRequest {
                        preset,
                        mode: Some(mode),
                        ..MassRequest::default()
                    };
                    let profile = part_profile(&request, &profile);
                    let draft = Draft::new(&request).unwrap();
                    let mut goals = Goals::default();
                    for seed in 1..=10 {
                        let plan = draft.draw(seed);
                        for (at, part) in plan.parts() {
                            let mode = plan.movements[at.movement].mode;
                            let drawn: Vec<Step> =
                                targets_with(mode, goals.of(&plan, at, &request), &part.chain)
                                    .iter()
                                    .map(|c| c.target)
                                    .collect();
                            let free = FreeRequest {
                                seed: splitmix64(part.seed),
                                ..part_request(&request, &plan, at, None)
                            };
                            let given: Vec<Step> = match &free.imitation {
                                None => free.cadences.iter().map(|c| c.target).collect(),
                                Some(imitation) => match plan_imitation(&free, imitation, &profile)
                                {
                                    Ok(planner) => {
                                        planner.plan().cadences.iter().map(|c| c.target).collect()
                                    }
                                    Err(error) => {
                                        println!("{} {}: {error:?}", mode, plan.address(at));
                                        continue;
                                    }
                                },
                            };
                            parts += 1;
                            alike += usize::from(drawn == given);
                            if drawn != given {
                                println!(
                                    "{mode} {:?} seed {seed} {}: drawn {} given {}",
                                    preset,
                                    plan.address(at),
                                    letters(drawn.iter().copied()),
                                    letters(given.iter().copied())
                                );
                            }
                            for &step in &drawn {
                                add(&mut simulated, steps_of(mode, step));
                            }
                            for &step in &given {
                                add(&mut set, steps_of(mode, step));
                            }
                        }
                    }
                }
            }
        }
        let all = |list: &[(i32, f64)]| list.iter().map(|c| c.1).sum::<f64>();
        let (n, k) = (all(&simulated), all(&set));
        simulated.sort_by_key(|c| c.0);
        for (steps, count) in &simulated {
            let other = set.iter().find(|c| c.0 == *steps).map_or(0.0, |c| c.1);
            println!(
                "{steps} steps above the final: drawn {:.3}, given {:.3}",
                count / n,
                other / k
            );
        }
        println!("{alike} of {parts} parts alike; {n} cadences drawn, {k} given");
        assert!(alike as f64 >= 0.95 * parts as f64);
    }

    /// A cut with its largest part and the sum of the squares of its parts.
    type Ranked = ((usize, u64), Vec<Range<usize>>);

    /// The cuts of an exhaustive search over every set of cuts, compared as the chunk scripts of
    /// the design compare them (docs/design/phase-14/scripts/chunks.py).
    fn exhaustive(sizes: &[usize], limit: usize) -> Vec<Range<usize>> {
        let n = sizes.len();
        let mut best: Option<Ranked> = None;
        for k in 1..=n {
            // Every set of k − 1 cuts, in lexicographic order.
            let mut cuts: Vec<usize> = (1..k).collect();
            loop {
                let bounds: Vec<usize> = std::iter::once(0)
                    .chain(cuts.iter().copied())
                    .chain(std::iter::once(n))
                    .collect();
                let groups: Vec<Range<usize>> = bounds.windows(2).map(|w| w[0]..w[1]).collect();
                let group_sizes: Vec<usize> = groups
                    .iter()
                    .map(|g| sizes[g.clone()].iter().sum())
                    .collect();
                let key = (
                    *group_sizes.iter().max().unwrap(),
                    group_sizes.iter().map(|&s| (s * s) as u64).sum(),
                );
                if best.as_ref().is_none_or(|(b, _)| key < *b) {
                    best = Some((key, groups));
                }
                // The next set of cuts.
                let Some(i) = (0..cuts.len())
                    .rev()
                    .find(|&i| cuts[i] < n - (cuts.len() - i))
                else {
                    break;
                };
                cuts[i] += 1;
                for j in i + 1..cuts.len() {
                    cuts[j] = cuts[j - 1] + 1;
                }
            }
            let (key, groups) = best.take().unwrap();
            if key.0 <= limit || k == n {
                return groups;
            }
        }
        Vec::new()
    }

    /// The cut by dynamic programming finds what the exhaustive search finds, on every stretch
    /// of the Ordinary at every size of part and on drawn sizes.
    #[test]
    fn partition_matches_the_exhaustive_search() {
        let options = TextOptions::default();
        for ids in [
            &["gloria"][..],
            &["qui-tollis"],
            &["credo"],
            &["crucifixus", "resurrexit", "spiritum"],
            &["crucifixus", "resurrexit"],
            &["spiritum"],
        ] {
            let mut sizes = Vec::new();
            for id in ids {
                let text = ordinary::section(id, &options).unwrap();
                for sentence in text.sentences() {
                    sizes.push(sentence.map(|u| text.units[u].range().len()).sum::<usize>());
                }
            }
            for limit in 31..=210 {
                assert_eq!(
                    partition(&sizes, limit),
                    exhaustive(&sizes, limit),
                    "{ids:?} at {limit}"
                );
            }
        }
        let mut rng = Rng::new(7);
        for _ in 0..300 {
            let n = 1 + (rng.next_u64() % 9) as usize;
            let sizes: Vec<usize> = (0..n).map(|_| 1 + (rng.next_u64() % 12) as usize).collect();
            let limit = 1 + (rng.next_u64() % 40) as usize;
            assert_eq!(
                partition(&sizes, limit),
                exhaustive(&sizes, limit),
                "{sizes:?} {limit}"
            );
        }
    }

    /// The cuts of the design's tables (scripts/chunk2.py on ordinary.txt), at 45, 50 and 55
    /// syllables, and a sentence longer than the limit cut at its commas.
    #[test]
    fn cuts_follow_the_tables() {
        let options = TextOptions::default();
        let specs = |ids: &[&'static str], limit: Option<usize>| -> Vec<String> {
            let pieces: Vec<Piece> = ids.iter().map(|&id| Piece { id, label: None }).collect();
            cut_section(&pieces, limit, &options)
                .into_iter()
                .map(|cut| format!("{} ({})", cut.spec, cut.syllables))
                .collect()
        };
        assert_eq!(
            specs(&["gloria"], Some(45)),
            ["gloria:1-5 (36)", "gloria:6-9 (34)", "gloria:10-13 (28)"]
        );
        assert_eq!(
            specs(&["crucifixus", "resurrexit", "spiritum"], Some(50)),
            [
                "crucifixus+resurrexit:1-2 (40)",
                "resurrexit:3-7 (46)",
                "spiritum:1-4 (27)",
                "spiritum:5-8 (49)",
                "spiritum:9-12 (44)"
            ]
        );
        assert_eq!(
            specs(&["credo"], Some(55)),
            ["credo:1-6 (51)", "credo:7-10 (34)", "credo:11-16 (43)"]
        );
        assert_eq!(
            specs(&["gloria", "qui-tollis"], Some(55)),
            [
                "gloria:1-6 (53)",
                "gloria:7-13 (45)",
                "qui-tollis:1-6 (48)",
                "qui-tollis:7-13 (41)"
            ]
        );
        // At 20 syllables the sentences of the Credo longer than that are cut at their commas,
        // semicolons and colons; no part is longer.
        let pieces = [Piece {
            id: "credo",
            label: None,
        }];
        let cuts = cut_section(&pieces, Some(20), &options);
        assert!(cuts.iter().all(|cut| cut.syllables <= 20), "{cuts:?}");
        assert_eq!(cuts.iter().map(|cut| cut.syllables).sum::<usize>(), 128);
        // Nine parts at least: 11 and 10 syllables (Et in unum Dominum | Filium Dei) join
        // neither each other nor their neighbours, nor does the sentence of 20.
        assert_eq!(cuts.len(), 9, "{cuts:?}");
        assert_eq!(cuts[0].spec, "credo:1-2", "{cuts:?}");
        // Without a limit a stretch is one part; a short section always is.
        assert_eq!(
            specs(&["credo", "incarnatus"], None),
            ["credo+incarnatus (153)"]
        );
        assert_eq!(
            specs(&["sanctus", "pleni", "osanna"], Some(20)),
            ["sanctus+pleni+osanna (34)"]
        );
    }

    /// The text of a part is the text `resolve` gives its spec: sections and ranges joined by `+`
    /// sung one after the other, with names joined as `Text::concat` joins them.
    #[test]
    fn part_texts_join_their_pieces() {
        use crate::text::MAX_SYLLABLES;
        let options = TextOptions::default();
        let text = part_text("sanctus+pleni+osanna", &options).unwrap();
        assert_eq!(text.id, "sanctus+pleni+osanna");
        assert_eq!(text.name_de, "Sanctus · Pleni · Osanna");
        assert_eq!((text.syllables.len(), text.units.len()), (34, 5));
        assert_eq!(text.kind, TextKind::Short);
        assert_eq!(text.sentences().count(), 3);
        let text = part_text("crucifixus+resurrexit:1-2", &options).unwrap();
        assert_eq!((text.syllables.len(), text.kind), (40, TextKind::Long));
        // Indices run on: the units and words of the second text follow the first's.
        for (u, unit) in text.units.iter().enumerate() {
            for s in unit.range() {
                assert_eq!(usize::from(text.syllables[s].unit), u);
            }
        }
        assert_eq!(
            text.render(false, false).split(' ').count(),
            text.words.len()
        );
        assert!(part_text("credo:99", &options).is_err());
        // A joined text keeps to the parser's limit, which its 16-bit indices need.
        let too_long = vec!["credo"; MAX_SYLLABLES / 128 + 1].join("+");
        assert_eq!(part_text(&too_long, &options), Err(TextError::TooLong));
    }

    /// The draws of one request are the same every time; another seed draws otherwise. Masses
    /// with one seed in another mode or choir set their parts with other seeds, which also seed
    /// the chains of their internal cadences, so that they do not repeat each other's cadences
    /// at the same places.
    #[test]
    fn draws_are_reproducible() {
        let request = MassRequest::default();
        let draft = Draft::new(&request).unwrap();
        assert_eq!(draft.draw(1), draft.draw(1));
        assert_eq!(plan_mass(&request).unwrap(), draft.draw(1));
        let endings = |plan: &MassPlan| letters(plan.parts().map(|(_, part)| part.ending));
        let differ = (2..20).filter(|&seed| endings(&draft.draw(seed)) != endings(&draft.draw(1)));
        assert!(differ.count() > 10);
        let seeds = |request: MassRequest| -> Vec<u64> {
            let plan = plan_mass(&request).unwrap();
            plan.parts().map(|(_, part)| part.seed).collect()
        };
        let dorian = seeds(request.clone());
        for other in [
            MassRequest {
                mode: Some("phrygian".parse().unwrap()),
                ..request.clone()
            },
            MassRequest {
                choir: choir(5),
                ..request.clone()
            },
        ] {
            let other = seeds(other);
            assert!(dorian.iter().zip(&other).all(|(a, b)| a != b));
        }
    }

    /// An option for one part changes no draw of another: every part draws the same numbers.
    #[test]
    fn options_change_their_parts_only() {
        let base = MassRequest {
            seed: 3,
            ..MassRequest::default()
        };
        let plan = plan_mass(&base).unwrap();
        let changed = plan_mass(&MassRequest {
            overrides: vec![Override::Imitation {
                at: "credo/2".into(),
                toggle: Toggle::On,
            }],
            ..base.clone()
        })
        .unwrap();
        let at = plan.find("credo/2").unwrap()[0];
        for ((place, a), (_, b)) in plan.parts().zip(changed.parts()) {
            if place == at {
                assert_eq!(b.opening, Opening::Imitative);
                assert_eq!(b.chord, None);
            } else {
                assert_eq!(a, b, "{}", plan.address(place));
            }
        }
    }

    /// A movement plans the same whichever other movements are set: its parts, with their
    /// endings, restarts, chords, chains, seeds and estimates, and its principal degree, are
    /// those of the whole mass with the same seed, in both forms and in several modes and choirs.
    /// So movements set one by one into a folder are the movements of one mass (`--resume`).
    #[test]
    fn movements_plan_alike_alone() {
        for preset in [Preset::Brevis, Preset::Normal] {
            for (name, voices) in [("dorian", 4), ("mixolydian", 3), ("aeolian-transposed", 5)] {
                let whole = MassRequest {
                    preset,
                    mode: Some(name.parse().unwrap()),
                    choir: choir(voices),
                    ..MassRequest::default()
                };
                let alone: Vec<Vec<Movement>> = vec![
                    vec![Movement::Kyrie],
                    vec![Movement::Gloria],
                    vec![Movement::Credo],
                    vec![Movement::Sanctus, Movement::Benedictus],
                    vec![Movement::Kyrie, Movement::Agnus],
                    vec![Movement::Agnus],
                ];
                for seed in 1..=12 {
                    let full = plan_mass(&MassRequest {
                        seed,
                        ..whole.clone()
                    })
                    .unwrap();
                    for movements in &alone {
                        let part = plan_mass(&MassRequest {
                            seed,
                            movements: movements.clone(),
                            ..whole.clone()
                        })
                        .unwrap();
                        for planned in &part.movements {
                            let label = format!("{preset:?} {name} {seed} {}", planned.movement);
                            let same = full
                                .movements
                                .iter()
                                .find(|m| m.movement == planned.movement)
                                .unwrap();
                            assert_eq!(planned.principal, same.principal, "{label}");
                            for (a, b) in planned.sections.iter().zip(&same.sections) {
                                assert_eq!(a.parts, b.parts, "{label}");
                            }
                        }
                    }
                }
            }
        }
    }

    /// An ending that `--endings` gives changes that part's ending and no other, except that the
    /// next part of its section, where it would end as the given ending, is drawn once more
    /// against it (and so on while a redraw changes an ending); an ending given as planned
    /// changes nothing. The restarts, chords and seeds stay; of the chains only the start of the
    /// next part's, from the ending before it.
    #[test]
    fn endings_stay_where_given() {
        let (mut overrides, mut cascades) = (0, 0);
        for preset in [Preset::Brevis, Preset::Normal] {
            for name in ["dorian", "mixolydian", "aeolian"] {
                let request = MassRequest {
                    preset,
                    mode: Some(name.parse().unwrap()),
                    ..MassRequest::default()
                };
                let mut draft = Draft::new(&request).unwrap();
                let places: Vec<PartRef> = draft.plan.parts().map(|(at, _)| at).collect();
                for seed in 1..=30 {
                    let planned = draft.draw(seed);
                    for (k, &at) in places.iter().enumerate() {
                        let mode = planned.movements[at.movement].mode;
                        for step in part_endings(mode) {
                            if draft.check_ending("", at, step).is_err() {
                                continue;
                            }
                            draft.extra_mut(at).ending = Some(step);
                            let given = draft.draw(seed);
                            draft.extra_mut(at).ending = None;
                            let label = format!("{preset:?} {name} {seed} {}", given.address(at));
                            assert_eq!(given.part(at).ending, step, "{label}");
                            if step == planned.part(at).ending {
                                assert_eq!(given, planned, "{label}");
                                continue;
                            }
                            overrides += 1;
                            for (q, &other) in places.iter().enumerate() {
                                let (a, b) = (planned.part(other), given.part(other));
                                assert_eq!(
                                    (a.opening, &a.chord, a.seed, a.estimate),
                                    (b.opening, &b.chord, b.seed, b.estimate),
                                    "{label}"
                                );
                                if q == k || a.ending == b.ending {
                                    continue;
                                }
                                cascades += 1;
                                let before = places[q - 1];
                                assert!(
                                    q > k
                                        && (other.movement, other.section)
                                            == (at.movement, at.section)
                                        && other.part > 0
                                        && given.part(before).ending != planned.part(before).ending
                                        && a.ending == given.part(before).ending,
                                    "{label}: {} ends on {} instead of {}",
                                    given.address(other),
                                    b.ending.letter(),
                                    a.ending.letter()
                                );
                            }
                            for &other in &places {
                                let CadenceChain::Weighted { before, .. } = given.part(other).chain
                                else {
                                    unreachable!()
                                };
                                assert_eq!(before, given.ending_before(other), "{label}");
                            }
                        }
                    }
                }
            }
        }
        // Most changed endings stay alone; some repeat the next part's, which is drawn again.
        println!("{overrides} endings given otherwise, {cascades} other parts drawn again");
        assert!(overrides > 1000 && cascades > 0, "{overrides} {cascades}");
        assert!(cascades * 3 < overrides, "{overrides} {cascades}");
    }

    /// The plans follow the corpus (§ 8): their endings, and the internal cadences of their
    /// chains ([`chains_follow_the_corpus`]). Every kind of part ends on each degree within three
    /// points of the share that the tables of § 3.5 give it with the principal degree of its plan,
    /// in both forms and every mode: the redraw of an ending equal to the one before, within a
    /// section, keeps the shares ([`tables::redrawn`]). It does redraw: within a section a part
    /// ends as the part before at most 0.85 times as often as two draws from the tables would
    /// (about half as often, 0.75 times in the Mixolydian, whose final takes 0.70). The spec
    /// counts 2000 plans per mode; the noise of a part's share adds to that of the part before it
    /// (seeds 1–2000 put the end of Et in terra in the Aeolian 4 points off the share the next
    /// 38 000 give), so 10 000 plans keep three points well beyond chance. A plan draws in about
    /// 10 µs.
    #[test]
    fn plan_follows_the_corpus() {
        const PLANS: u64 = 10_000;
        for preset in [Preset::Brevis, Preset::Normal] {
            for m in Mode::ALL {
                for transposed in [false, true] {
                    let mode = ModeSpec::new(m).with_transposition(transposed);
                    let request = MassRequest {
                        preset,
                        mode: Some(mode),
                        ..MassRequest::default()
                    };
                    let label = format!("{preset:?} {mode}");
                    let draft = Draft::new(&request).unwrap();
                    chains_follow_the_corpus(&draft, &request, &label);
                    let kinds: Vec<(PartRef, EndingKind)> = draft
                        .plan
                        .parts()
                        .map(|(at, _)| (at, draft.extras[at.movement][at.section][at.part].kind))
                        .collect();
                    // Observed endings by kind, and those of the tables for each plan.
                    let mut seen: Vec<(EndingKind, Step, f64)> = Vec::new();
                    let mut expected: Vec<(EndingKind, Step, f64)> = Vec::new();
                    let add = |list: &mut Vec<(EndingKind, Step, f64)>, kind, step, w| match list
                        .iter_mut()
                        .find(|(k, s, _)| *k == kind && *s == step)
                    {
                        Some((_, _, sum)) => *sum += w,
                        None => list.push((kind, step, w)),
                    };
                    // Within sections: parts that end as the part before, and how many would by
                    // two independent draws from the tables.
                    let (mut alike, mut independent) = (0.0, 0.0);
                    for seed in 1..=PLANS {
                        let plan = draft.draw(seed);
                        let mut before: Option<(Step, Vec<(Step, f64)>)> = None;
                        for &(at, kind) in &kinds {
                            let movement = &plan.movements[at.movement];
                            let ending = plan.part(at).ending;
                            add(&mut seen, kind, ending, 1.0);
                            let shares = tables::ending_distribution(
                                kind,
                                movement.mode,
                                movement.principal,
                            );
                            for &(step, share) in &shares {
                                add(&mut expected, kind, step, share);
                            }
                            if at.part > 0
                                && let Some((previous, drawn_from)) = &before
                            {
                                alike += f64::from(u8::from(ending == *previous));
                                independent += shares
                                    .iter()
                                    .map(|&(x, d)| {
                                        d * drawn_from
                                            .iter()
                                            .find(|&&(y, _)| y == x)
                                            .map_or(0.0, |e| e.1)
                                    })
                                    .sum::<f64>();
                            }
                            before = Some((ending, shares));
                        }
                    }
                    let share = |list: &[(EndingKind, Step, f64)], kind, step| -> f64 {
                        let of = |k: EndingKind, s: Option<Step>| -> f64 {
                            list.iter()
                                .filter(|e| e.0 == k && s.is_none_or(|s| e.1 == s))
                                .map(|e| e.2)
                                .sum()
                        };
                        of(kind, Some(step)) / of(kind, None)
                    };
                    for &(kind, step, _) in expected.iter().chain(&seen) {
                        let observed = share(&seen, kind, step);
                        let wanted = share(&expected, kind, step);
                        assert!(
                            (observed - wanted).abs() <= 0.03,
                            "{label} {kind:?} on {}: {observed:.3} against {wanted:.3}",
                            step.letter()
                        );
                    }
                    assert!(
                        alike <= 0.85 * independent,
                        "{label}: {alike} parts end as the one before, against {independent:.0}"
                    );
                }
            }
        }
    }

    /// The chain half of [`plan_follows_the_corpus`]: every part's chain is seeded with the
    /// part's seed, weighted for its movement's mode and principal degree
    /// ([`tables::chain_weights`]), starts from the ending of the part before it in the movement
    /// (none for the first) and knows the beginning of the Christe. Drawn at the goals that the
    /// parts' texts and points plan, where the words end at a full stop, at an intermediate stop
    /// or at a break ([`Goals`]), the cadences repeat the degree before them three times in ten
    /// (±3 points), and every degree comes out within three points of its share in the corpus
    /// (§ 3.6, [`tables::corpus_weights`]), for which the weights are compensated
    /// ([`calibrate_chain_weights`]). The cadences of the first 500 plans of every mode.
    fn chains_follow_the_corpus(draft: &Draft, request: &MassRequest, label: &str) {
        const PLANS: u64 = 500;
        // Observed and expected cadences by degree, and the repeats among the cadences after
        // another one.
        let mut seen: Vec<(Step, f64)> = Vec::new();
        let mut expected: Vec<(Step, f64)> = Vec::new();
        let add = |list: &mut Vec<(Step, f64)>, step: Step, w: f64| match list
            .iter_mut()
            .find(|(s, _)| *s == step)
        {
            Some((_, sum)) => *sum += w,
            None => list.push((step, w)),
        };
        let (mut repeats, mut followers) = (0_usize, 0_usize);
        let mut goals = Goals::default();
        for seed in 1..=PLANS {
            let plan = draft.draw(seed);
            for (at, part) in plan.parts() {
                let movement = &plan.movements[at.movement];
                let mode = movement.mode;
                let CadenceChain::Weighted {
                    seed: chain_seed,
                    weights,
                    before,
                    christe,
                } = &part.chain
                else {
                    panic!("{label}: {} has no weighted chain", plan.address(at));
                };
                assert_eq!(*chain_seed, part.seed, "{label}");
                assert_eq!(
                    *weights,
                    tables::chain_weights(mode, movement.principal),
                    "{label}"
                );
                assert_eq!(*before, plan.ending_before(at), "{label}");
                let first_of_christe =
                    movement.sections[at.section].id == "christe" && at.part == 0;
                assert_eq!(*christe, first_of_christe, "{label}");
                let corpus = tables::corpus_weights(mode, movement.principal);
                let total: f64 = corpus.iter().map(|&(_, w)| f64::from(w)).sum();
                let mut previous = *before;
                for cadence in targets_with(mode, goals.of(&plan, at, request), &part.chain) {
                    add(&mut seen, cadence.target, 1.0);
                    for &(step, w) in &corpus {
                        add(&mut expected, step, f64::from(w) / total);
                    }
                    if let Some(step) = previous {
                        followers += 1;
                        repeats += usize::from(step == cadence.target);
                    }
                    previous = Some(cadence.target);
                }
            }
        }
        let share = |list: &[(Step, f64)], step: Step| -> f64 {
            let all: f64 = list.iter().map(|e| e.1).sum();
            list.iter().find(|e| e.0 == step).map_or(0.0, |e| e.1) / all
        };
        for &(step, _) in expected.iter().chain(&seen) {
            let (observed, wanted) = (share(&seen, step), share(&expected, step));
            assert!(
                (observed - wanted).abs() <= 0.03,
                "{label}, cadences on {}: {observed:.3} against {wanted:.3}",
                step.letter()
            );
        }
        let repeated = repeats as f64 / followers as f64;
        assert!(
            (repeated - CHAIN_REPEAT).abs() <= 0.03,
            "{label}: {repeated:.3} of the cadences repeat the one before"
        );
    }

    /// The principal degree is drawn once per mass: fixed in most modes, C or D in the
    /// Mixolydian and A or G in the Aeolian on D.
    #[test]
    fn principal_degrees_follow_the_mode() {
        let principals = |mode: &str| -> String {
            let request = MassRequest {
                mode: Some(mode.parse().unwrap()),
                movements: vec![Movement::Kyrie],
                ..MassRequest::default()
            };
            let draft = Draft::new(&request).unwrap();
            let mut seen: Vec<char> = (1..200)
                .map(|seed| draft.draw(seed).principal.letter())
                .collect();
            seen.sort_unstable();
            seen.dedup();
            seen.into_iter().collect()
        };
        assert_eq!(principals("dorian"), "A");
        assert_eq!(principals("mixolydian"), "CD");
        assert_eq!(principals("aeolian"), "D");
        assert_eq!(principals("aeolian-transposed"), "AG");
        assert_eq!(principals("phrygian"), "A");
    }

    /// The restarts follow the table: sections imitate, Breaks and Et resurrexit restart
    /// together, the Osanna II of the Missa brevis too, and with two or six voices nothing
    /// imitates.
    #[test]
    fn restarts_follow_the_table() {
        let draft = Draft::new(&MassRequest::default()).unwrap();
        for seed in 1..50 {
            let plan = draft.draw(seed);
            for (at, part) in plan.parts() {
                let section = &plan.movements[at.movement].sections[at.section];
                let begins = draft.extras[at.movement][at.section][at.part].begins;
                match (part.join, begins) {
                    (Join::Break(_), _) | (_, ("resurrexit", 0)) => {
                        assert_eq!(part.opening, Opening::Tutti)
                    }
                    (Join::Start | Join::Section, _) if section.id == "osanna2" => {
                        assert_eq!(part.opening, Opening::Tutti)
                    }
                    (Join::Start | Join::Section, _) => {
                        assert_eq!(part.opening, Opening::Imitative)
                    }
                    (Join::Seam, _) => {}
                }
                assert_eq!(part.chord.is_some(), part.opening == Opening::Tutti);
            }
        }
        // One in five of the other seams of a Missa brevis imitates, a third of those of a normal
        // mass, and in the Missa brevis Et in Spiritum one in three.
        for (preset, wanted, spiritum) in [
            (
                Preset::Brevis,
                tables::RESTART_SEAM_BREVIS,
                Some(tables::RESTART_SPIRITUM),
            ),
            (Preset::Normal, tables::RESTART_SEAM, None),
        ] {
            let draft = Draft::new(&MassRequest {
                preset,
                ..MassRequest::default()
            })
            .unwrap();
            let (mut imitative, mut seams) = (0, 0);
            let (mut imitative_spiritum, mut spiritum_seams) = (0, 0);
            for seed in 1..800 {
                let plan = draft.draw(seed);
                for (at, part) in plan.parts() {
                    let begins = draft.extras[at.movement][at.section][at.part].begins;
                    let opens = usize::from(part.opening == Opening::Imitative);
                    match (part.join, begins) {
                        (Join::Seam, ("spiritum", 0)) => {
                            spiritum_seams += 1;
                            imitative_spiritum += opens;
                        }
                        (Join::Seam, ("resurrexit", 0)) => {}
                        (Join::Seam, _) => {
                            seams += 1;
                            imitative += opens;
                        }
                        _ => {}
                    }
                }
            }
            let share = imitative as f64 / seams as f64;
            assert!((share - wanted).abs() < 0.03, "{preset:?}: {share}");
            match spiritum {
                Some(wanted) => {
                    let share = imitative_spiritum as f64 / spiritum_seams as f64;
                    assert!((share - wanted).abs() < 0.05, "{preset:?}: {share}");
                }
                // Et in Spiritum begins a section of a normal mass.
                None => assert_eq!(spiritum_seams, 0),
            }
        }
        // With two or six voices nothing imitates; the sections of four voices in a six-voice
        // mass (Christe, Benedictus) do.
        for voices in [2, 6] {
            let request = MassRequest {
                choir: choir(voices),
                ..MassRequest::default()
            };
            let plan = plan_mass(&request).unwrap();
            let mut imitative = Vec::new();
            for (at, part) in plan.parts() {
                let section = &plan.movements[at.movement].sections[at.section];
                if !(3..=5).contains(&section.voices.len()) {
                    assert_eq!(part.opening, Opening::Tutti, "{voices}: {}", part.text);
                } else if part.opening == Opening::Imitative {
                    imitative.push(section.id);
                }
            }
            let expected: &[&str] = if voices == 2 {
                &[]
            } else {
                &["christe", "benedictus"]
            };
            assert_eq!(imitative, expected);
        }
    }
}
