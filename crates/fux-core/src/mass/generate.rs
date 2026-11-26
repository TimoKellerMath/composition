//! Setting a mass (docs/design/phase-14/spec.md §§ 4.1, 4.4 and 4.5): every part of a plan as a
//! free setting of its own (D1), with three seeds and the fallbacks of § 3.8, under the profile
//! of the mass (D7), and the whole mass part after part, telling what it does as it goes (D17).
//!
//! **A part** ([`part_request`]) sings its text in the voices of its section, the text going on
//! after it where it is not the last part of its section (D8). It begins in imitation with as
//! many points as the plan gives it, or together on the plan's chord; it ends on its ending,
//! begins at its start and takes the degrees of its internal cadences from its chain. Its search
//! has the portable budget, so that a seed sets the same part natively and in WebAssembly (D13).
//!
//! **Attempts** ([`generate_part`]). Attempt a searches with the seed `splitmix64(part seed + a)`
//! (MASS-SEED). The part as planned takes attempts 0 to 2. Where all of them fail, a part
//! lengthened by restated points (MASS-LENGTH) takes three more with the points its imitation
//! plans by itself, and an Agnus Dei II with a voice added to the choir (MASS-SCORING) three more
//! in the choir of the mass, each with a warning. A cancel, an option the mass cannot have and an
//! invalid request end the part at once. Where every attempt fails, the error names the part, its
//! words, the measure and the rules where its last search ended, and what may help.
//!
//! **The mass** ([`generate_mass`]) is planned, then its parts are set in the order of the plan.
//! A part that fails stays unset and the mass goes on, incomplete; a cancel, by the control or by
//! the callback, ends it with the parts set so far. A finished part is never replaced by one that
//! breaks a prohibition or misses its text: every part keeps the guarantee of a free setting.
//!
//! **Head motif** (MASS-MOTIF, D11). Where the request asks for it, the first parts of Gloria,
//! Credo, Sanctus and Agnus Dei I open with the head motif of Kyrie I ([`motif_of`]): the pitches
//! of its first subject, declaimed anew on their own words ([`SubjectSource::Motif`]). Kyrie I is
//! set first, and its result carries the motif ([`PartResult::motif`]) for the parts that take it.
//! Where the motif gives the first point no subject, the point takes one from its words; where no
//! attempt sets the part with the motif, three more set it with a subject from its words; and
//! where Kyrie I gives no motif, the part begins without one; each with a warning.
//!
//! **Chant** (MASS-CHANT, D12). A mass on chant IX or IV paraphrases the Kyriale: every imitative
//! part takes the subjects of its points from the chant of its section ([`part_chant`]), found by
//! their words from where the part's words stand in the chant, so that the chant runs on through
//! the parts of a long section.

use std::ops::ControlFlow;
use std::sync::Arc;

use super::preset::{self, Scoring};
use super::{
    ChantMass, MassPlan, MassRequest, Movement, Opening, PartField, PartPlan, PartRef,
    opening_chord, tables, voice_places,
};
use crate::clef::VoiceType;
use crate::gabc::Chant;
use crate::generate::{
    Budget, FreeRequest, FreeText, GenerateError, Generated, PartContext, SearchControl, Warning,
    fingerprint, generate_free, part_endings, splitmix64,
};
use crate::imitation::chant::{Kyriale, from_note, section_note};
use crate::imitation::{Answer, Imitation, Motif, SubjectSource};
use crate::lang::Lang;
use crate::mode::ModeSpec;
use crate::pitch::Pitch;
use crate::rules::Profile;
use crate::text::plan::{automatic_length, plan_part};
use crate::text::{Text, TextKind};
use crate::time::Meter;

/// The attempts of a part as planned, and of each of its fallbacks: three seeds (D13).
const ATTEMPTS: u8 = 3;

/// The parameters that judge settings without a plan of imitation, each with the parameter of
/// the same case with a plan, whose level the profile of a mass gives it: IMI-HELD `struck` and
/// `passing`, IMI-SILENCE `thin`.
const UNPLANNED: [(&str, &str, &str); 3] = [
    ("IMI-HELD", "unplanned_struck", "struck"),
    ("IMI-HELD", "unplanned_passing", "passing"),
    ("IMI-SILENCE", "unplanned_thin", "thin"),
];

/// The profile of a mass on `base` (D7). The parts that begin together have no plan of
/// imitation, yet their notes held over the barline and their halves of the choir are those of
/// the imitative parts. So IMI-HELD and IMI-SILENCE `thin` judge them as they judge the parts
/// with a plan: the parameters for settings without a plan take the levels of their
/// counterparts in `base`, V, P3 and P1 in Fux's profile. Everything else is `base`'s, whose
/// calibrations stay as they were. The frontends apply it to the profile the user chose, before
/// the overrides of `--rule`.
pub fn profile(base: &Profile) -> Profile {
    let mut profile = base.clone();
    for (rule, unplanned, planned) in UNPLANNED {
        let level = base.settings(rule).level(planned);
        profile
            .apply_override(&format!("{rule}.{unplanned}={}", level.label(Lang::De)))
            .expect("the rules of imitation have the parameters for settings without a plan");
    }
    profile
}

/// The profile the parts of `request` are set and checked with: `profile`, the profile of the
/// mass with the user's overrides, with TXT-REPEAT as the request's `--repeat` sets it.
/// [`generate_part`] applies it itself; a part checked again
/// ([`part_from_parts`](super::part_from_parts)) and the number by which a file knows the
/// profile of its part ([`profile_hash`]) need it too.
pub fn part_profile(request: &MassRequest, profile: &Profile) -> Profile {
    request.repeat.apply(profile)
}

/// The number of a profile, FNV-1a over all its parameters: a file of a part keeps it
/// (`profile=` of `fux-part`), so that `--resume` takes up only the parts set with the profile it
/// sets the others with.
pub fn profile_hash(profile: &Profile) -> u64 {
    fingerprint(&profile.to_toml(None))
}

/// A part as set: the result of its search, with the seed and the attempt that found it.
#[derive(Clone, Debug)]
pub struct PartResult {
    pub at: PartRef,
    /// The seed of the search that set it: `splitmix64(part seed + attempt)` (MASS-SEED).
    pub seed: u64,
    /// The attempt that set it, from 0: 0 to 2 as planned, 3 to 5 in its first fallback.
    pub attempt: u8,
    pub generated: Generated,
    /// The seconds the part took, all its attempts together; 0 in WebAssembly, which has no
    /// clock without imports, so that the page measures the time itself.
    pub seconds: f32,
    /// What the part did otherwise than planned: its fallback (lengths or scoring) first, then
    /// the warnings of its setting (`generated.warnings`).
    pub warnings: Vec<Warning>,
    /// The voices of its movement it sings, from the top, as indices into the movement's
    /// voices: those of its section, or the choir of the mass after a fallback of the scoring.
    pub voices: Vec<usize>,
    /// The number of the profile it was checked with ([`profile_hash`]): the one it was set with,
    /// also where its file fits the plan ([`PartField::fits`]); a part taken up as it is keeps
    /// the one it was set with in its field ([`PartResult::kept`]).
    pub profile: u64,
    /// The head motif of the mass as the part knows it (MASS-MOTIF): for Kyrie I of a mass whose
    /// later movements take it, the motif it gives ([`motif_of`]); for a part that takes it, the
    /// motif it was given; `None` where Kyrie I gives none, and for every other part. The files
    /// of the parts keep it (`motif=` of `fux-part`).
    pub motif: Option<Motif>,
    /// What the file it was taken from keeps of it, where it was taken up as it is, without its
    /// plan and profile being compared (`fux mass --rewrite`,
    /// [`mass_of_parts`](super::mass_of_parts)): the files of the mass keep that field for it
    /// ([`PartField::of`]), as it was set, its profile and its version of the generator included.
    /// `None` for a part set by this program or taken up because its file fits the plan, whose
    /// field the plan gives.
    pub kept: Option<PartField>,
}

/// A mass as set: its plan and the result of every part.
#[derive(Clone, Debug)]
pub struct Mass {
    pub plan: Arc<MassPlan>,
    /// The parts in the order of [`MassPlan::parts`]; `None` for a part that failed, or that a
    /// cancel left unset.
    pub results: Vec<Option<PartResult>>,
    /// Every part is set.
    pub complete: bool,
}

impl Mass {
    /// A mass of `plan` with no part set yet, to which the parts are added as they come
    /// ([`Mass::insert`]): from the files of `--resume`, from the workers of the page.
    pub fn new(plan: Arc<MassPlan>) -> Mass {
        let parts = plan.part_count();
        Mass {
            plan,
            results: vec![None; parts],
            complete: parts == 0,
        }
    }

    /// The result of the part at `at`, if it is set.
    pub fn result(&self, at: PartRef) -> Option<&PartResult> {
        let i = self.plan.parts().position(|(place, _)| place == at)?;
        self.results.get(i)?.as_ref()
    }

    /// Puts `result` in the place of its part, instead of what was there; the mass is complete
    /// once every part is set. A result for a part the plan does not have is left out.
    pub fn insert(&mut self, result: PartResult) {
        if let Some(i) = self.plan.parts().position(|(place, _)| place == result.at) {
            self.results[i] = Some(result);
        }
        self.complete = self.results.iter().all(Option::is_some);
    }
}

/// What [`generate_mass`] tells as it goes. It owns what it holds, so that another thread can
/// take it (a channel to a window).
#[derive(Clone, Debug)]
pub enum MassEvent {
    /// The mass is planned; its parts follow in this order.
    Planned(Arc<MassPlan>),
    /// An attempt at a part begins, with its seed.
    Started { at: PartRef, attempt: u8, seed: u64 },
    /// A part is set.
    Finished(Box<PartResult>),
    /// An attempt at a part failed. After the part's last attempt the error is
    /// [`GenerateError::MassPart`], which names the part and what may help, and the part stays
    /// unset; so too after an option the mass cannot have or an invalid request.
    Failed {
        at: PartRef,
        attempt: u8,
        error: GenerateError,
    },
    /// The mass was cancelled, by its control or by the callback; no part follows.
    Cancelled,
}

/// The head motif that the later movements of a mass take from Kyrie I (MASS-MOTIF, D11): the
/// pitches of the subject of the first point of `kyrie`, the result of setting Kyrie I, as the
/// subject the driver set it with has them ([`Generated::subjects`], all its notes, though an
/// entry may sing fewer of them), a pitch repeated at once counting once, as degrees above the
/// final with their alterations ([`Motif::of_pitches`]). `None` where Kyrie I has no imitation or
/// its first subject has not five to ten tones, and for a part read back from its file, which
/// keeps no subjects: its file keeps the motif instead ([`PartField::motif`](super::PartField)).
pub fn motif_of(kyrie: &Generated) -> Option<Motif> {
    let mode = kyrie.score.mode;
    let subject = kyrie.subjects.first()?;
    let first = mode.final_pitch(3).diatonic() + i32::from(subject.degree);
    let pitches = subject.pitches(mode, first, 0, Answer::Real)?;
    Motif::of_pitches(mode, &pitches)
}

/// The chant whose phrases the part at `at` of `plan` paraphrases, where the mass is set on the
/// chant of the Kyriale (MASS-CHANT, D12): the chant of the section of the Ordinary it begins
/// with ([`Kyriale::for_section`]), for the Kyrie of a mass on IV Kyrie IV. A part that begins
/// within a long section takes it from where the chant sings the part's first unit
/// ([`section_note`]), so that its points find their notes by their words there and not at an
/// earlier place with the same words, and the chant runs on through the parts of a long section;
/// Agnus Dei III and the Osanna after the Benedictus take it from their last place in the chant,
/// as Kyrie II takes the last invocation. `None` for a mass without a chant.
pub fn part_chant(plan: &MassPlan, at: PartRef) -> Option<Arc<Chant>> {
    let mass = plan.chant?;
    let part = plan.part(at);
    let kyriale = match (mass, plan.movements[at.movement].movement) {
        (ChantMass::IV, Movement::Kyrie) => Kyriale::KyrieIV,
        _ => Kyriale::for_section(part.first_text())?,
    };
    let chant = kyriale.chant();
    let (id, unit) = first_unit(&part.text);
    let last = unit == 0 && matches!(id, "agnus3" | "osanna2");
    if unit == 0 && !last {
        return Some(chant);
    }
    match section_note(&chant, id, unit, last) {
        Some(note) => Some(Arc::new(from_note(&chant, note))),
        None => Some(chant),
    }
}

/// The section of the Ordinary a part's text begins with and its unit there, from 0:
/// `("credo", 4)` for `credo:5-7`, `("sanctus", 0)` for `sanctus+pleni+osanna`.
fn first_unit(spec: &str) -> (&str, usize) {
    let first = spec.split('+').next().unwrap_or_default();
    match first.split_once(':') {
        Some((id, range)) => {
            let from = range.split('-').next().unwrap_or_default();
            let unit = from
                .trim()
                .parse::<usize>()
                .map_or(0, |u| u.saturating_sub(1));
            (id.trim(), unit)
        }
        None => (first.trim(), 0),
    }
}

/// Where the subjects of the part at `at` of `plan` come from, where it imitates: for the first
/// point the head motif `motif`, where the part takes the head motif and Kyrie I gave one
/// (MASS-MOTIF); the chant of the Kyriale, where the mass paraphrases one (MASS-CHANT); otherwise
/// the words.
fn subject_of(plan: &MassPlan, at: PartRef, motif: Option<&Motif>) -> SubjectSource {
    if let Some(motif) = motif.filter(|_| plan.part(at).motif) {
        return SubjectSource::Motif(Arc::new(motif.clone()));
    }
    match part_chant(plan, at) {
        Some(chant) => SubjectSource::Chant {
            chant,
            phrases: None,
        },
        None => SubjectSource::Text,
    }
}

/// The free setting of the part at `at` of `plan`, a plan of `request` (D1): in its movement's
/// mode, for the voices of its section from the top; singing its text, with the request's
/// division of eleison and accents, going on after the part where it is not the last of its
/// section; in imitation where it begins so, as `fux free --imitation auto` asks for it, with the
/// plan's number of points, its subjects from its words, from the chant of a mass on chant
/// ([`part_chant`]) or, for the first point of a part that takes the head motif, from `motif`,
/// the motif of Kyrie I ([`motif_of`]; without one, from the words); with its ending, start, chord
/// and chain, the portable budget, its seed and the request's beam and noise. Its length and its
/// internal cadences are those its text plans, as `fux free` gives them; the search plans them
/// again. It has no control.
pub fn part_request(
    request: &MassRequest,
    plan: &MassPlan,
    at: PartRef,
    motif: Option<&Motif>,
) -> FreeRequest {
    let movement = &plan.movements[at.movement];
    let section = &movement.sections[at.section];
    let part = plan.part(at);
    let voices = section.voices.iter().map(|&v| movement.voices[v]).collect();
    setting_of(
        request,
        movement.mode,
        part,
        voices,
        part.points,
        part.chord.clone(),
        subject_of(plan, at, motif),
    )
}

/// The free setting of `part` in `mode` for `voices` from the top, with `points` points where it
/// imitates, its subjects from `subject`, and the opening `chord` where it begins together: as
/// planned or as a fallback.
fn setting_of(
    request: &MassRequest,
    mode: ModeSpec,
    part: &PartPlan,
    voices: Vec<VoiceType>,
    points: Option<usize>,
    chord: Option<Vec<Pitch>>,
    subject: SubjectSource,
) -> FreeRequest {
    let text = part.sung_text(&request.text);
    let context = PartContext {
        ending: Some(part.ending),
        start: part.start,
        chord,
        chain: part.chain.clone(),
    };
    let restates = request.repeat.restates();
    let planned = plan_part(&text, mode, voices.len(), None, None, restates, &context);
    let (measures, cadences) = match planned {
        Ok(planned) => (planned.measures, planned.cadences),
        // The search plans again and fails as the plan did, saying why.
        Err(_) => {
            let silent = part.start.div_ceil(Meter::ALLA_BREVE.ticks_per_measure()) as usize;
            (automatic_length(&text, restates) + silent, Vec::new())
        }
    };
    FreeRequest {
        cadences,
        seed: part.seed,
        beam: request.beam,
        noise: request.noise,
        text: Some(FreeText {
            text: Arc::new(text),
            auto_length: true,
            auto_cadences: true,
            accents: request.accents,
        }),
        imitation: (part.opening == Opening::Imitative).then(|| Imitation {
            subject,
            points,
            ..Imitation::default()
        }),
        part: context,
        budget: Budget::PORTABLE,
        control: None,
        ..FreeRequest::new(mode, voices, measures)
    }
}

/// How a part is set (§ 3.8): as planned, or by one of its fallbacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fallback {
    /// As planned.
    Planned,
    /// A part lengthened by restated points, with the points its imitation plans by itself
    /// (MASS-LENGTH).
    Lengths,
    /// A section with a voice added to the choir by its form, in the choir of the mass
    /// (MASS-SCORING).
    Scoring,
    /// A part that takes the head motif, its first subject from its words (MASS-MOTIF).
    Motif,
}

/// The ways to set the part at `at` of `plan`, in the order its attempts take them, three
/// attempts each: as planned; with the points its imitation plans by itself where the plan
/// lengthens it; in the choir of the mass where its section has a voice added by its form; with a
/// first subject from its words where it takes the head motif.
pub(crate) fn fallbacks(plan: &MassPlan, at: PartRef) -> Vec<Fallback> {
    let mut ways = vec![Fallback::Planned];
    if plan.part(at).points.is_some() {
        ways.push(Fallback::Lengths);
    }
    if base_scoring(plan, at).is_some() {
        ways.push(Fallback::Scoring);
    }
    if plan.part(at).motif {
        ways.push(Fallback::Motif);
    }
    ways
}

/// How attempt `attempt` (from 0) sets the part at `at` of `plan`: attempts 0 to 2 as planned,
/// each fallback three more ([`fallbacks`]); `None` beyond its last attempt.
pub(crate) fn fallback_of(plan: &MassPlan, at: PartRef, attempt: u8) -> Option<Fallback> {
    fallbacks(plan, at)
        .get(usize::from(attempt / ATTEMPTS))
        .copied()
}

/// A way to set a part: as planned, or one of its fallbacks (§ 3.8).
struct Stage {
    request: FreeRequest,
    /// The voices of the movement it sings, from the top.
    voices: Vec<usize>,
    /// The warning of a fallback; `None` as planned.
    warning: Option<Warning>,
}

/// The ways to set the part at `at`, in the order of [`fallbacks`]: as planned; a part
/// lengthened by restated points with the points its imitation plans by itself; a section with
/// a voice added to the choir by its form in the choir of the mass, its opening chord voiced
/// anew where it begins together; a part that takes the head motif `motif` with its first subject
/// from its words.
fn stages(
    request: &MassRequest,
    plan: &MassPlan,
    at: PartRef,
    motif: Option<&Motif>,
) -> Vec<Stage> {
    let movement = &plan.movements[at.movement];
    let section = &movement.sections[at.section];
    let part = plan.part(at);
    let planned = part_request(request, plan, at, motif);
    fallbacks(plan, at)
        .into_iter()
        .map(|fallback| match fallback {
            Fallback::Planned => Stage {
                request: planned.clone(),
                voices: section.voices.clone(),
                warning: None,
            },
            Fallback::Lengths => Stage {
                request: FreeRequest {
                    imitation: planned.imitation.as_ref().map(|imitation| Imitation {
                        points: None,
                        ..imitation.clone()
                    }),
                    ..planned.clone()
                },
                voices: section.voices.clone(),
                warning: Some(Warning::LengthFallback {
                    part: plan.address(at),
                }),
            },
            Fallback::Scoring => {
                let mode = movement.mode;
                let chord = part
                    .chord
                    .as_ref()
                    .map(|_| opening_chord(mode, &plan.choir, mode.final_step()));
                Stage {
                    request: setting_of(
                        request,
                        mode,
                        part,
                        plan.choir.clone(),
                        part.points,
                        chord,
                        subject_of(plan, at, motif),
                    ),
                    voices: base_scoring(plan, at).expect("the fallback of the scoring has them"),
                    warning: Some(Warning::ScoringFallback {
                        section: section.id.to_string(),
                    }),
                }
            }
            // Without a motif from Kyrie I the part has taken its subjects from its words already;
            // it takes three more seeds so.
            Fallback::Motif => Stage {
                request: FreeRequest {
                    imitation: planned.imitation.as_ref().map(|imitation| Imitation {
                        subject: SubjectSource::Text,
                        ..imitation.clone()
                    }),
                    ..planned.clone()
                },
                voices: section.voices.clone(),
                warning: motif.map(|_| Warning::MotifFallback {
                    movement: movement.movement.title().to_string(),
                }),
            },
        })
        .collect()
}

/// The warnings the plan gives the part at `at` of `plan`, besides those of its fallbacks: where
/// the part takes the head motif and Kyrie I gave none (`motif` is `None`), that it begins without
/// one; where the first part of a movement that takes the motif begins together, that the
/// movement carries none; and for the first part of a movement of a mass on chant, that the
/// movement takes the chant of Mass IX in a mass on IV (other than the Kyrie, D12), and that its
/// chant is sung from the final of the mass where the mode of the mass has another final than the
/// chant's.
pub(crate) fn plan_warnings(plan: &MassPlan, at: PartRef, motif: Option<&Motif>) -> Vec<Warning> {
    let movement = &plan.movements[at.movement];
    let title = movement.movement.title().to_string();
    let mut warnings = Vec::new();
    let first = at.section == 0 && at.part == 0;
    if plan.part(at).motif && motif.is_none() {
        warnings.push(Warning::NoMotif {
            movement: title.clone(),
        });
    } else if plan.motif
        && first
        && plan.part(at).opening == Opening::Tutti
        && matches!(
            movement.movement,
            Movement::Gloria | Movement::Credo | Movement::Sanctus | Movement::Agnus
        )
    {
        warnings.push(Warning::MotifTogether {
            movement: title.clone(),
        });
    }
    let Some(mass) = plan.chant.filter(|_| first) else {
        return warnings;
    };
    if mass == ChantMass::IV && movement.movement != Movement::Kyrie {
        warnings.push(Warning::ChantIV);
    }
    let kyriale = match (mass, movement.movement) {
        (ChantMass::IV, Movement::Kyrie) => Some(Kyriale::KyrieIV),
        _ => Kyriale::for_section(plan.part(at).first_text()),
    };
    if let Some(own) = kyriale.and_then(|k| k.chant().numbered_mode().ok())
        && own.final_step() != movement.mode.final_step()
    {
        warnings.push(Warning::ChantMode {
            movement: title,
            chant: own,
            mass: movement.mode,
        });
    }
    warnings
}

/// The choir of the mass as voices of the movement, where the section of the part at `at` has a
/// voice added to the choir by its form (Agnus Dei II, MASS-SCORING); `None` where it has not,
/// also where `--scoring` gave the section its voices.
pub(crate) fn base_scoring(plan: &MassPlan, at: PartRef) -> Option<Vec<usize>> {
    let movement = &plan.movements[at.movement];
    let section = &movement.sections[at.section];
    let form = preset::form(plan.preset, movement.movement);
    if form.sections.get(at.section)?.scoring != Scoring::Enlarged {
        return None;
    }
    let kinds: Vec<VoiceType> = section.voices.iter().map(|&v| movement.voices[v]).collect();
    let enlarged = tables::scoring(&plan.choir, Scoring::Enlarged);
    if kinds != enlarged || enlarged == plan.choir {
        return None;
    }
    voice_places(&movement.voices, &plan.choir)
}

/// Sets the part at `at` of `plan`, a plan of `request`, under `profile`, the profile of the mass
/// ([`profile`]) with the user's overrides; TXT-REPEAT as the request's `--repeat` sets it
/// ([`part_profile`]). A part that takes the head motif (MASS-MOTIF) takes `motif`, the motif of
/// Kyrie I ([`motif_of`], [`PartResult::motif`]); where it is `None`, the part begins without one
/// and says so. Attempt a searches with the seed `splitmix64(part seed + a)`: the part as planned
/// takes three attempts, each of its fallbacks three more (lengths, scoring, then the head motif;
/// module documentation). The control cancels the search ([`GenerateError::Cancelled`]); an
/// option the mass cannot have ([`GenerateError::MassOption`]) and an invalid request end the part
/// at once. Where every attempt fails, [`GenerateError::MassPart`] names the part, its words, the
/// error of its last attempt and what may help. The result keeps every guarantee of a free
/// setting: no prohibition of the profile, every voice sings the whole text, every planned entry
/// and the opening chord sound as planned, the bass ends on the ending, no voice sounds before the
/// start. For Kyrie I of a mass whose later movements take the head motif it carries the motif
/// they take ([`PartResult::motif`]).
pub fn generate_part(
    request: &MassRequest,
    plan: &MassPlan,
    at: PartRef,
    profile: &Profile,
    motif: Option<&Motif>,
    control: Option<Arc<SearchControl>>,
) -> Result<PartResult, GenerateError> {
    set_part(request, plan, at, profile, motif, control, &mut |_| {
        ControlFlow::Continue(())
    })
}

/// Sets the part at `at` as [`generate_part`] does, telling `on` of each attempt as it begins and
/// as it fails. Where `on` breaks off, the part ends as cancelled.
fn set_part(
    request: &MassRequest,
    plan: &MassPlan,
    at: PartRef,
    profile: &Profile,
    motif: Option<&Motif>,
    control: Option<Arc<SearchControl>>,
    on: &mut dyn FnMut(MassEvent) -> ControlFlow<()>,
) -> Result<PartResult, GenerateError> {
    let clock = stopwatch();
    let profile = part_profile(request, profile);
    let hash = profile_hash(&profile);
    let part = plan.part(at);
    let motif = motif.filter(|_| part.motif);
    let stages = stages(request, plan, at, motif);
    let noted = plan_warnings(plan, at, motif);
    let movement = plan.movements[at.movement].movement.title();
    let attempts = stages.len() as u8 * ATTEMPTS;
    let mut attempt: u8 = 0;
    let mut failed = None;
    for stage in &stages {
        for _ in 0..ATTEMPTS {
            if control.as_deref().is_some_and(SearchControl::is_cancelled) {
                return Err(GenerateError::Cancelled);
            }
            let seed = splitmix64(part.seed.wrapping_add(u64::from(attempt)));
            if on(MassEvent::Started { at, attempt, seed }).is_break() {
                return Err(GenerateError::Cancelled);
            }
            let search = FreeRequest {
                seed,
                control: control.clone(),
                ..stage.request.clone()
            };
            let error = match generate_free(&search, &profile) {
                Ok(mut found) => {
                    let mut generated = found.swap_remove(0);
                    // The first point of a part on the head motif, which gave it no subject.
                    let on_motif = search
                        .imitation
                        .as_ref()
                        .is_some_and(|i| matches!(i.subject, SubjectSource::Motif(_)));
                    for warning in &mut generated.warnings {
                        if on_motif && matches!(warning, Warning::ChantFallback(_)) {
                            *warning = Warning::MotifFallback {
                                movement: movement.to_string(),
                            };
                        }
                    }
                    let warnings = stage
                        .warning
                        .iter()
                        .chain(&noted)
                        .chain(&generated.warnings)
                        .cloned()
                        .collect();
                    // Kyrie I gives the head motif to the later movements; a part that takes it
                    // keeps the one it was given.
                    let motif = if plan.motif_source() == Some(at) {
                        motif_of(&generated)
                    } else {
                        motif.cloned()
                    };
                    return Ok(PartResult {
                        at,
                        seed,
                        attempt,
                        generated,
                        seconds: clock(),
                        warnings,
                        voices: stage.voices.clone(),
                        profile: hash,
                        motif,
                        kept: None,
                    });
                }
                Err(GenerateError::Cancelled) => return Err(GenerateError::Cancelled),
                Err(error @ (GenerateError::MassOption(_) | GenerateError::Invalid(_))) => {
                    // Another seed or a fallback changes nothing; the part ends here either way.
                    let _ = on(MassEvent::Failed {
                        at,
                        attempt,
                        error: error.clone(),
                    });
                    return Err(error);
                }
                Err(error) if attempt + 1 == attempts => {
                    failure(request, plan, at, attempts, error)
                }
                Err(error) => error,
            };
            let flow = on(MassEvent::Failed {
                at,
                attempt,
                error: error.clone(),
            });
            if flow.is_break() {
                return Err(GenerateError::Cancelled);
            }
            failed = Some(error);
            attempt += 1;
        }
    }
    Err(failed.expect("every part has its attempts"))
}

/// A clock for the seconds a part takes: natively the monotonic clock of the system.
#[cfg(not(target_arch = "wasm32"))]
fn stopwatch() -> impl Fn() -> f32 {
    let start = std::time::Instant::now();
    move || start.elapsed().as_secs_f32()
}

/// WebAssembly has no clock without imports (`Instant::now` panics there): the seconds stay 0,
/// and the page measures the parts itself.
#[cfg(target_arch = "wasm32")]
fn stopwatch() -> impl Fn() -> f32 {
    || 0.0
}

/// The error of the part at `at` when none of its `attempts` set it (§ 4.4): the part by its
/// address and its words, the error of its last attempt with the measure and the rules where its
/// search ended, and the options that may help besides another seed: the other restart, another
/// ending where the part may have one, and smaller parts of a long text where they cut the
/// part's text.
fn failure(
    request: &MassRequest,
    plan: &MassPlan,
    at: PartRef,
    attempts: u8,
    error: GenerateError,
) -> GenerateError {
    let address = plan.address(at);
    let movement = &plan.movements[at.movement];
    let section = &movement.sections[at.section];
    let part = plan.part(at);
    let text = part.sung_text(&request.text);
    let voices = section.voices.len();
    let mut remedies = Vec::new();
    // `--imitation an` imitates in two to six voices, `auto` in three to five only (phase 13):
    // a part that begins together may begin in imitation instead, in two voices too, where the
    // chord's search has the rules of a two-voice setting against it.
    remedies.push(match part.opening {
        Opening::Imitative => format!("--imitation {address}=aus"),
        Opening::Tutti => format!("--imitation {address}=an"),
    });
    // The last part of a movement ends on the final, and so does a part in two voices
    // (MASS-ENDING); another part may end on the final instead, or on the principal degree.
    if voices > 2 && !ends_movement(plan, at) {
        let mode = movement.mode;
        let allowed = part_endings(mode);
        let other = if part.ending == mode.final_step() {
            [movement.principal]
                .into_iter()
                .chain(allowed.iter().copied())
                .find(|&step| step != part.ending && allowed.contains(&step))
        } else {
            Some(mode.final_step())
        };
        if let Some(step) = other {
            remedies.push(format!("--endings {address}={}", step.letter()));
        }
    }
    // Smaller parts of a long text are shorter searches (D3). A size below the part's own
    // syllables cuts its text where it has more than one unit: no part may hold more, and a unit
    // longer than the size stands alone. (Two thirds of the request's size left a part shorter
    // than that as it was.)
    if text.kind == TextKind::Long && text.units.len() > 1 {
        let smaller = (part.syllables * 2 / 3).max(*tables::CHUNK_RANGE.start());
        if smaller < part.syllables {
            remedies.push(format!("--chunk {smaller}"));
        }
    }
    GenerateError::MassPart {
        part: address,
        words: words(&text),
        attempts,
        error: Box::new(error),
        remedies,
    }
}

/// Whether the part at `at` is the last of its movement, which ends on the final: no part
/// follows it, and no Osanna ut supra.
fn ends_movement(plan: &MassPlan, at: PartRef) -> bool {
    let movement = &plan.movements[at.movement];
    let section = &movement.sections[at.section];
    at.part + 1 == section.parts.len()
        && movement.sections[at.section + 1..]
            .iter()
            .all(|later| later.parts.is_empty() && later.ut_supra.is_none())
}

/// The words of a text for a message, as the books print them: in full up to seven words, else
/// its first four and its last two with `…` between.
fn words(text: &Text) -> String {
    let rendered = text.render(true, false);
    let words: Vec<&str> = rendered.split_whitespace().collect();
    if words.len() <= 7 {
        return words.join(" ");
    }
    format!(
        "{} … {}",
        words[..4].join(" "),
        words[words.len() - 2..].join(" ")
    )
}

/// Plans the mass of `request` and sets its parts one after the other in the order of the plan
/// (Kyrie I first), each as [`generate_part`] sets it under `profile` (the profile of the mass
/// with the user's overrides), the parts that take the head motif with the motif of Kyrie I,
/// telling `on` what happens: [`MassEvent::Planned`], then for every attempt
/// [`MassEvent::Started`] and [`MassEvent::Failed`] or, once a part is set,
/// [`MassEvent::Finished`]. A part that no attempt sets stays `None`, and the mass goes on. The
/// control, read between the attempts and by every search, and `on` breaking off end the mass
/// with [`MassEvent::Cancelled`]; the parts set so far stay. The mass is complete when every part
/// is set. Only an option that the plan cannot have fails the whole mass, before any search.
pub fn generate_mass(
    request: &MassRequest,
    profile: &Profile,
    control: Option<Arc<SearchControl>>,
    on: &mut dyn FnMut(MassEvent) -> ControlFlow<()>,
) -> Result<Mass, GenerateError> {
    let plan = Arc::new(super::plan_mass(request)?);
    let mut mass = Mass::new(Arc::clone(&plan));
    if on(MassEvent::Planned(Arc::clone(&plan))).is_break() {
        let _ = on(MassEvent::Cancelled);
        return Ok(mass);
    }
    let source = plan.motif_source();
    // The head motif, once Kyrie I is set.
    let mut motif: Option<Motif> = None;
    for (at, _) in plan.parts() {
        if control.as_deref().is_some_and(SearchControl::is_cancelled) {
            let _ = on(MassEvent::Cancelled);
            break;
        }
        match set_part(
            request,
            &plan,
            at,
            profile,
            motif.as_ref(),
            control.clone(),
            on,
        ) {
            Ok(result) => {
                if source == Some(at) {
                    motif.clone_from(&result.motif);
                }
                let flow = on(MassEvent::Finished(Box::new(result.clone())));
                mass.insert(result);
                if flow.is_break() {
                    let _ = on(MassEvent::Cancelled);
                    break;
                }
            }
            Err(GenerateError::Cancelled) => {
                let _ = on(MassEvent::Cancelled);
                break;
            }
            // The event of its last attempt told of it; the part stays unset.
            Err(_) => {}
        }
    }
    mass.complete = mass.results.iter().all(Option::is_some);
    Ok(mass)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mass::{Movement, Preset, choir, parse_imitation, parse_scoring, plan_mass};
    use crate::pitch::Step;

    fn find(plan: &MassPlan, at: &str) -> PartRef {
        plan.find(at).unwrap()[0]
    }

    /// The fallbacks of § 3.8 follow the plan: a lengthened part is set again with the points its
    /// imitation plans by itself, Agnus Dei II with its added tenor in the choir of the mass, its
    /// opening chord voiced for those voices where it begins together, and every other part only
    /// as planned. A scoring that `--scoring` gives has no fallback.
    #[test]
    fn fallbacks_follow_the_plan() {
        let request = MassRequest {
            overrides: parse_imitation("agnus3=aus").unwrap(),
            ..MassRequest::default()
        };
        let plan = plan_mass(&request).unwrap();
        let kyrie = stages(&request, &plan, find(&plan, "kyrie/1"), None);
        assert_eq!(kyrie.len(), 1);
        assert_eq!(kyrie[0].request.imitation, Some(Imitation::default()));
        let benedictus = stages(&request, &plan, find(&plan, "benedictus/1"), None);
        let points: Vec<Option<usize>> = benedictus
            .iter()
            .map(|stage| stage.request.imitation.as_ref().unwrap().points)
            .collect();
        assert_eq!(points, [Some(4), None]);
        assert_eq!(
            benedictus[1].warning,
            Some(Warning::LengthFallback {
                part: "benedictus/1".into()
            })
        );
        let at = find(&plan, "agnus3");
        let agnus = stages(&request, &plan, at, None);
        assert_eq!(agnus.len(), 2);
        assert_eq!(agnus[0].voices, [0, 1, 2, 3, 4]);
        assert_eq!(agnus[1].voices, [0, 1, 2, 4]);
        assert_eq!(agnus[1].request.voices, choir(4));
        let mode = plan.movements[at.movement].mode;
        assert_eq!(
            agnus[1].request.part.chord,
            Some(opening_chord(mode, &choir(4), mode.final_step()))
        );
        assert_eq!(
            agnus[1].warning,
            Some(Warning::ScoringFallback {
                section: "agnus3".into()
            })
        );
        let given = MassRequest {
            overrides: parse_scoring("agnus3=S A T B B").unwrap(),
            movements: vec![Movement::Agnus],
            ..MassRequest::default()
        };
        let plan = plan_mass(&given).unwrap();
        assert_eq!(stages(&given, &plan, find(&plan, "agnus3"), None).len(), 1);
    }

    /// The attempts of a part take its ways in order, three each: the lengthened Benedictus as
    /// planned in attempts 0 to 2 and with its natural points in 3 to 5, Agnus Dei II in its
    /// enlarged choir, then in the choir of the mass; a part without a fallback has three
    /// attempts only.
    #[test]
    fn attempts_take_the_fallbacks_in_order() {
        let plan = plan_mass(&MassRequest::default()).unwrap();
        let ways = |at: &str| -> Vec<Option<Fallback>> {
            (0..7)
                .map(|attempt| fallback_of(&plan, find(&plan, at), attempt))
                .collect()
        };
        use Fallback::*;
        assert_eq!(
            ways("benedictus/1"),
            [
                Some(Planned),
                Some(Planned),
                Some(Planned),
                Some(Lengths),
                Some(Lengths),
                Some(Lengths),
                None
            ]
        );
        assert_eq!(
            ways("agnus3")[2..],
            [
                Some(Planned),
                Some(Scoring),
                Some(Scoring),
                Some(Scoring),
                None
            ]
        );
        assert_eq!(ways("credo/4")[2..4], [Some(Planned), None]);
        let request = MassRequest::default();
        for (at, _) in plan.parts() {
            assert_eq!(
                fallbacks(&plan, at).len(),
                stages(&request, &plan, at, None).len(),
                "{}",
                plan.address(at)
            );
        }
    }

    /// A part that no attempt set names what may help besides another seed: the other restart,
    /// a part that begins together in imitation in two voices too, another ending where the part
    /// may end elsewhere, and smaller parts of a long text where they cut the part's text.
    #[test]
    fn failures_name_their_remedies() {
        let request = MassRequest {
            overrides: parse_imitation("credo/2=an; credo/3=aus").unwrap(),
            ..MassRequest::default()
        };
        let plan = plan_mass(&request).unwrap();
        let stuck = || GenerateError::NoCounterpoint {
            measure: 12,
            rules: vec!["TXT-ORDER"],
        };
        let remedies = |at: &str| match failure(&request, &plan, find(&plan, at), 3, stuck()) {
            GenerateError::MassPart {
                part,
                remedies,
                attempts,
                error,
                ..
            } => {
                assert_eq!((part.as_str(), attempts, *error), (at, 3, stuck()));
                remedies
            }
            other => panic!("{other:?}"),
        };
        let other = |at: &str| {
            let place = find(&plan, at);
            let movement = &plan.movements[place.movement];
            if plan.part(place).ending == movement.mode.final_step() {
                movement.principal
            } else {
                movement.mode.final_step()
            }
        };
        let syllables = plan.part(find(&plan, "credo/2")).syllables;
        assert_eq!(syllables, 35);
        assert_eq!(
            remedies("credo/2"),
            [
                "--imitation credo/2=aus".to_string(),
                format!("--endings credo/2={}", other("credo/2").letter()),
                "--chunk 23".into()
            ]
        );
        assert_eq!(remedies("credo/3")[0], "--imitation credo/3=an");
        // The last part of a movement ends on the final; a short text is one part.
        assert_eq!(remedies("kyrie/3"), ["--imitation kyrie/3=aus"]);
        let failed = failure(&request, &plan, find(&plan, "credo/2"), 3, stuck());
        let GenerateError::MassPart { words, .. } = &failed else {
            unreachable!()
        };
        assert_eq!(words, "Et in u-num Dó-mi-num … ó-mni-a saé-cu-la.");
        // Every size that a failed part of a long text names cuts its text: with it the part
        // is gone from the plan, its syllables in smaller parts. A part of 20 syllables or of a
        // single unit is named none.
        let mut named = 0;
        for (at, part) in plan.parts() {
            let chunk = remedies(&plan.address(at))
                .iter()
                .find_map(|remedy| remedy.strip_prefix("--chunk ")?.parse::<usize>().ok());
            let text = part.sung_text(&request.text);
            let Some(chunk) = chunk else {
                assert!(
                    text.kind == TextKind::Short || text.units.len() == 1 || part.syllables <= 20,
                    "{}",
                    part.text
                );
                continue;
            };
            named += 1;
            let smaller = plan_mass(&MassRequest {
                chunk: Some(chunk),
                ..request.clone()
            })
            .unwrap();
            assert!(
                smaller.parts().all(|(_, other)| other.text != part.text),
                "{} at {chunk}",
                part.text
            );
        }
        assert!(named >= 10, "{named}");
        // A part in two voices begins together, and may begin in imitation instead.
        let two = MassRequest {
            choir: choir(2),
            movements: vec![Movement::Kyrie],
            ..MassRequest::default()
        };
        let plan = plan_mass(&two).unwrap();
        let at = find(&plan, "christe");
        assert_eq!(plan.part(at).opening, Opening::Tutti);
        let GenerateError::MassPart { remedies, .. } = failure(&two, &plan, at, 3, stuck()) else {
            unreachable!()
        };
        assert_eq!(remedies, ["--imitation kyrie/2=an"]);
    }

    /// The chant runs on through the parts of a movement (MASS-CHANT): every point of every part
    /// finds its notes by its words after those of the point before, the first point of a part
    /// after the last of the part before, whatever the size of the parts; so does Agnus Dei III
    /// after Agnus Dei I and the Osanna after the Benedictus, at their last places in the chant.
    /// The chant of a part that begins within a long section begins with its words. With
    /// `phrases: None` alone, a part that begins with Gé-ni-tum took the notes of u-ni-gé-ni-tum,
    /// one with the Gloria's second Dó-mi-ne De-us those of the first.
    #[test]
    fn chant_runs_on_through_the_parts() {
        use crate::generate::Rng;
        use crate::imitation::chant::sources;
        use crate::imitation::points::points_of;
        use crate::imitation::{Toggle, chant::Kyriale};
        use crate::text::plan::plan as plan_text;
        for preset in [Preset::Brevis, Preset::Normal] {
            for chunk in [Some(20), Some(30), Some(45), Some(55), None] {
                for chant in [ChantMass::IX, ChantMass::IV] {
                    let request = MassRequest {
                        preset,
                        chunk,
                        chant: Some(chant),
                        ..MassRequest::default()
                    };
                    let plan = plan_mass(&request).unwrap();
                    for (m, movement) in plan.movements.iter().enumerate() {
                        // The last note a point took up in the movement's chant.
                        let mut last: Option<usize> = None;
                        for (at, part) in plan.parts().filter(|(at, _)| at.movement == m) {
                            let label = format!("{preset:?} {chunk:?} {chant:?} {}", part.text);
                            let own = part_chant(&plan, at).unwrap();
                            let whole = match (chant, movement.movement) {
                                (ChantMass::IV, Movement::Kyrie) => Kyriale::KyrieIV,
                                _ => Kyriale::for_section(part.first_text()).unwrap(),
                            }
                            .chant();
                            let offset = whole.notes.len() - own.notes.len();
                            assert_eq!(own.notes[..], whole.notes[offset..], "{label}");
                            let text = part.sung_text(&request.text);
                            if offset > 0 {
                                let first = own.notes.iter().find_map(|n| n.syllable.as_deref());
                                let wanted = text.syllables[0].text.to_lowercase();
                                let found = first.unwrap().to_lowercase();
                                assert_eq!(
                                    found.trim_end_matches('-').replace('é', "e"),
                                    wanted.replace('é', "e"),
                                    "{label}"
                                );
                            }
                            let section = &movement.sections[at.section];
                            let statements = plan_text(
                                &text,
                                movement.mode,
                                section.voices.len(),
                                None,
                                None,
                                true,
                            )
                            .map_or(1, |planned| planned.statements);
                            let imitation = Imitation {
                                restate: Toggle::Off,
                                ..Imitation::default()
                            };
                            let mut rng = Rng::new(1);
                            let points =
                                points_of(&text, statements, true, &imitation, &mut rng).unwrap();
                            for source in sources(&own, &text, &points, None).unwrap() {
                                let Some(source) = source else { continue };
                                let note = offset + source.start;
                                assert!(
                                    last.is_none_or(|last| note > last),
                                    "{label}: note {note} after {last:?}"
                                );
                                last = Some(note);
                            }
                        }
                    }
                }
            }
        }
    }

    /// A mass on chant gives every imitative part the chant of its section (MASS-CHANT): Mass
    /// IX and Credo I, Kyrie IV for the Kyrie of a mass on IV; a part that begins together has no
    /// imitation; a mass without chant takes its subjects from the words. A part that takes the
    /// head motif takes it where Kyrie I gives one, else its words.
    #[test]
    fn parts_take_their_subjects() {
        let subject = |request: &MassRequest, at: &str, motif: Option<&Motif>| {
            let plan = plan_mass(request).unwrap();
            let at = find(&plan, at);
            part_request(request, &plan, at, motif)
                .imitation
                .map(|imitation| imitation.subject)
        };
        let chant_of = |request: &MassRequest, at: &str| match subject(request, at, None) {
            Some(SubjectSource::Chant { chant, phrases }) => {
                assert_eq!(phrases, None);
                chant.name.clone()
            }
            other => panic!("{at}: {other:?}"),
        };
        let ix = MassRequest {
            chant: Some(ChantMass::IX),
            overrides: parse_imitation("gloria/2=an").unwrap(),
            ..MassRequest::default()
        };
        assert_eq!(chant_of(&ix, "kyrie/1").as_deref(), Some("Kyrie IX"));
        assert_eq!(chant_of(&ix, "gloria/2").as_deref(), Some("Gloria IX"));
        assert_eq!(chant_of(&ix, "credo/1").as_deref(), Some("Credo I"));
        assert_eq!(chant_of(&ix, "sanctus/1").as_deref(), Some("Sanctus IX"));
        assert_eq!(chant_of(&ix, "agnus/2").as_deref(), Some("Agnus Dei IX"));
        // Et incarnatus restarts together.
        assert_eq!(subject(&ix, "incarnatus", None), None);
        let iv = MassRequest {
            chant: Some(ChantMass::IV),
            ..ix.clone()
        };
        assert_eq!(chant_of(&iv, "christe").as_deref(), Some("Kyrie IV"));
        assert_eq!(chant_of(&iv, "gloria/1").as_deref(), Some("Gloria IX"));
        assert_eq!(
            subject(&MassRequest::default(), "gloria/1", None),
            Some(SubjectSource::Text)
        );
        let motif: Motif = "0,2,3,4,3,2".parse().unwrap();
        let with = MassRequest {
            motif: true,
            overrides: parse_imitation("gloria/2=an").unwrap(),
            ..MassRequest::default()
        };
        assert_eq!(
            subject(&with, "gloria/1", Some(&motif)),
            Some(SubjectSource::Motif(Arc::new(motif.clone())))
        );
        assert_eq!(subject(&with, "gloria/1", None), Some(SubjectSource::Text));
        // Only the first parts of the later movements take it.
        assert_eq!(
            subject(&with, "gloria/2", Some(&motif)),
            Some(SubjectSource::Text)
        );
        assert_eq!(
            subject(&with, "christe", Some(&motif)),
            Some(SubjectSource::Text)
        );
    }

    /// A part that takes the head motif has a last fallback with a subject from its words, which
    /// warns where Kyrie I gave a motif (MASS-MOTIF): after its fallback of the lengths for Agnus
    /// Dei I. Without a motif the part warns that Kyrie I gave none.
    #[test]
    fn the_head_motif_falls_back_last() {
        let request = MassRequest {
            motif: true,
            ..MassRequest::default()
        };
        let plan = plan_mass(&request).unwrap();
        let motif: Motif = "0,2,3,4,3,2".parse().unwrap();
        let agnus = find(&plan, "agnus/1");
        assert_eq!(
            fallbacks(&plan, agnus),
            [Fallback::Planned, Fallback::Lengths, Fallback::Motif]
        );
        assert_eq!(
            fallbacks(&plan, find(&plan, "credo/1")),
            [Fallback::Planned, Fallback::Motif]
        );
        assert_eq!(
            fallbacks(&plan, find(&plan, "credo/2")),
            [Fallback::Planned]
        );
        let ways = stages(&request, &plan, agnus, Some(&motif));
        let subjects: Vec<&SubjectSource> = ways
            .iter()
            .map(|stage| &stage.request.imitation.as_ref().unwrap().subject)
            .collect();
        let on_motif = SubjectSource::Motif(Arc::new(motif.clone()));
        assert_eq!(subjects, [&on_motif, &on_motif, &SubjectSource::Text]);
        assert_eq!(
            ways[2].warning,
            Some(Warning::MotifFallback {
                movement: "Agnus Dei".into()
            })
        );
        assert_eq!(ways[2].request.imitation.as_ref().unwrap().points, Some(5));
        let without = stages(&request, &plan, agnus, None);
        assert_eq!(without[2].warning, None);
        assert_eq!(
            plan_warnings(&plan, agnus, None),
            [Warning::NoMotif {
                movement: "Agnus Dei".into()
            }]
        );
        assert!(plan_warnings(&plan, agnus, Some(&motif)).is_empty());
        assert!(plan_warnings(&plan, find(&plan, "kyrie/1"), None).is_empty());
    }

    /// A movement that is to open with the head motif but begins together carries none and says
    /// so (MASS-MOTIF): in two voices every movement, which begins together like every part
    /// there; in four voices the movement whose first part `--imitation` sets together. A
    /// movement that begins in imitation does not warn.
    #[test]
    fn movements_that_begin_together_carry_no_motif() {
        let warned = |request: &MassRequest| -> Vec<(String, Vec<Warning>)> {
            let plan = plan_mass(request).unwrap();
            plan.parts()
                .map(|(at, _)| (plan.address(at), plan_warnings(&plan, at, None)))
                .filter(|(_, warnings)| !warnings.is_empty())
                .collect()
        };
        let together = |movement: &str| Warning::MotifTogether {
            movement: movement.into(),
        };
        let duo = MassRequest {
            motif: true,
            choir: choir(2),
            ..MassRequest::default()
        };
        assert_eq!(
            warned(&duo),
            [
                ("gloria/1".to_string(), vec![together("Gloria")]),
                ("credo/1".to_string(), vec![together("Credo")]),
                ("sanctus/1".to_string(), vec![together("Sanctus")]),
                ("agnus/1".to_string(), vec![together("Agnus Dei")]),
            ]
        );
        let gloria_together = MassRequest {
            motif: true,
            overrides: parse_imitation("gloria/1=aus").unwrap(),
            ..MassRequest::default()
        };
        let warnings = warned(&gloria_together);
        assert_eq!(
            warnings[0],
            ("gloria/1".to_string(), vec![together("Gloria")])
        );
        assert!(
            warnings[1..]
                .iter()
                .all(|(_, w)| !w.iter().any(|w| matches!(w, Warning::MotifTogether { .. })))
        );
        assert!(warned(&MassRequest::default()).is_empty());
    }

    /// A mass on chant warns once per movement where it does not take the mode of its chant
    /// (D12): with a mode of its own, where the chant's final differs from the mass's; on chant
    /// IV, where a movement other than the Kyrie takes the chant of Mass IX.
    #[test]
    fn chant_masses_warn_once_per_movement() {
        let warned = |request: &MassRequest| -> Vec<(String, Vec<Warning>)> {
            let plan = plan_mass(request).unwrap();
            plan.parts()
                .map(|(at, _)| (plan.address(at), plan_warnings(&plan, at, None)))
                .filter(|(_, warnings)| !warnings.is_empty())
                .collect()
        };
        let dorian = ModeSpec::new(crate::mode::Mode::Dorian);
        let ix = MassRequest {
            chant: Some(ChantMass::IX),
            mode: Some(dorian),
            ..MassRequest::default()
        };
        let modes: Vec<(String, Vec<Warning>)> = warned(&ix);
        let addresses: Vec<&str> = modes.iter().map(|(at, _)| at.as_str()).collect();
        assert_eq!(
            addresses,
            [
                "gloria/1",
                "credo/1",
                "sanctus/1",
                "benedictus/1",
                "agnus/1"
            ]
        );
        let Warning::ChantMode {
            movement,
            chant,
            mass,
        } = &modes[0].1[0]
        else {
            panic!("{modes:?}")
        };
        assert_eq!(
            (movement.as_str(), chant.final_step(), *mass),
            ("Gloria", Step::G, dorian)
        );
        assert_eq!(
            modes[0].1[0].message(Lang::De),
            "Gloria: Der Choral hat die Finalis G, die Messe D; seine Stufen werden von D aus \
             gesungen"
        );
        // Each movement in the mode of its chant: no warning, but for IV the other movements.
        assert!(
            warned(&MassRequest {
                mode: None,
                ..ix.clone()
            })
            .is_empty()
        );
        let iv = warned(&MassRequest {
            chant: Some(ChantMass::IV),
            mode: None,
            ..ix
        });
        assert_eq!(iv.len(), 5);
        assert!(
            iv.iter()
                .all(|(_, warnings)| warnings == &[Warning::ChantIV])
        );
        assert_eq!(
            Warning::ChantIV.message(Lang::En),
            "Chant IV: the Kyriale has a Kyrie IV only; the other movements paraphrase the chants \
             of Mass IX and Credo I"
        );
    }
}
