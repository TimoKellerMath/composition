//! The mass in the page's tab „Messe“ (phase 14c, docs/design/phase-14/spec.md § 7): `mass_plan`
//! plans a mass without a search, `mass_part` sets one of its parts, as a worker of the page's pool
//! asks for it, and `mass_join` puts the parts set together into the files of the movements whose
//! parts are all set and into the book of the mass.
//!
//! Every request of a run carries the same form ([`MassForm`]), the options of `fux mass`, so
//! that each plans the same mass: the parts, their seeds and everything else the plan draws. A
//! part is set as `fux mass` sets it ([`generate_part`], with the portable budget, so that its
//! seed gives the same part natively and in WebAssembly, D13), and its file is the one `fux
//! mass` writes to `NAME.teile/`; the page keeps it and gives it back to `mass_join`, which takes
//! it up as `fux mass --resume` does ([`part_from_parts`]).
//!
//! **The head motif** (MASS-MOTIF). The answer for Kyrie I gives the motif of its first subject
//! ([`PartAnswer::motif`]); the page's pool sets the parts that open with it after Kyrie I and
//! passes it on in their requests (`motif`), as `fux mass` passes it from Kyrie I to them. Their
//! files keep it, and `mass_join` takes such a part only with the motif that the file of Kyrie I
//! keeps ([`PartField::motif_fits`]).

use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use fux_cli::commands::localize_names;
use fux_cli::free::SingingArgs;
use fux_cli::imitation::{ImitationSource, attach};
use fux_cli::input::musicxml_text_fields;
use fux_cli::mass::{
    DEFAULT_BASE, FIELD_LYRICS, JOBS_MAX, MassNotation, MassOptions, PartLine, cadence_plans_of,
    lyrics_field, mass_plan, mass_summary, mass_totals, mass_warnings, movement_file, part_label,
    part_summary, prepare_mass, restore_part,
};
use fux_cli::text::align_lyrics;
use fux_core::generate::part_endings;
use fux_core::mass::{
    FIELD_MASS, FIELD_PART, GENERATOR, Join, Mass, MassField, MassPlan, MassRequest, Motif,
    Movement, Opening, PartField, PartRef, PartResult, generate_part, mass_field, part_from_parts,
    part_profile, profile_hash, section_score, tables, voice_names,
};
use fux_core::rules::{Checker, Profile};
use fux_core::{Lang, ModeSpec, Pitch, Score, SectionKind, SectionMark, Step};
use fux_export::{
    MassBookOptions, MovementHeading, MovementOptions, MusicXmlOptions, mass_book,
    to_movement_midi, to_movement_musicxml, to_musicxml_with_fields,
};
use serde::{Deserialize, Serialize};

use crate::api::{
    Answer, Entry, Finding, Named, Request, eleison_number, findings_after, lilypond_options,
    midi_options, musicxml_options, optional, rules, sounds, text_or_none, voice_lines,
    voice_lyrics,
};
use crate::base64;

/// A mass as the page's form gives it: the options of `fux mass` (spec § 1.4), each empty (or 0)
/// for its default, except the seed, which the page always gives.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct MassForm {
    /// `brevis` (the default) or `normal`.
    pub preset: String,
    /// The movements by their ids, separated by commas: `kyrie,agnus`; empty for all six.
    pub movements: String,
    /// The voices of the choir, 2–6; 0 for four.
    pub voices: u8,
    /// The choir from the top, such as `S A T T B`; empty for the usual choir of `voices`.
    pub parts: String,
    /// The mode of the mass, such as `phrygisch`; empty for the Dorian, or with a chant for the
    /// mode of each movement's chant.
    pub mode: String,
    /// The rows of the page's table of sections ([`SectionRow`]), as the user changed them.
    pub sections: Vec<SectionChoice>,
    /// More changes as `fux mass` takes them, after those of the table: `--scoring`
    /// (`benedictus=S A T`), `--imitation` (`credo/3=an`), `--endings` (`gloria/2=D`).
    pub scoring: String,
    pub imitation: String,
    pub endings: String,
    /// The most syllables of a part of a long section, 20–200, or `aus`; empty for 45.
    pub chunk: String,
    /// `palestrina` (the default) or `natürlich`.
    pub lengths: String,
    /// The chant of the Kyriale the voices paraphrase: `keiner` or empty, `ix` or `iv`.
    pub chant: String,
    /// Gloria, Credo, Sanctus and Agnus Dei I open with the head motif of Kyrie I.
    pub motif: bool,
    /// How freely the voices restate their text: `auto` or empty, `keine` or `frei`.
    pub repeat: String,
    /// The printed syllables keep the acutes of the books (default on).
    pub accents: Option<bool>,
    /// eleison in 4 syllables (0, the default) or 3.
    pub eleison: u8,
    pub seed: u64,
    /// The breadth of the search; 0 for 200.
    pub beam: usize,
    /// The title; empty for that of the form (Missa brevis, Missa).
    pub title: String,
    /// `mass_join`: bar lines only between the staves (Mensurstriche), in the book and in the
    /// files of the movements.
    pub mensurstriche: bool,
    /// `mass_join`: the priest's intonation before Gloria and Credo (default on).
    pub intonation: Option<bool>,
    /// `mass_join`: the voices that rest through a section hidden there in the files of the
    /// movements (`--tacet verbergen`); the page shows them hidden anyway.
    pub hide_tacet: bool,
}

/// What the user chose in a row of the page's table of sections ([`SectionRow`]).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SectionChoice {
    /// The row: a section's id (`christe`) or a Break's (`incarnatus`).
    pub id: String,
    /// The voices of a section from the top (`S A T`); empty as planned.
    pub voices: String,
    /// Whether its parts (a Break's part) begin in imitation: `auto` or empty, `an`, `aus`.
    pub imitation: String,
    /// The degree its last part (a Break's part) ends on, by its letter; `auto` or empty as
    /// drawn.
    pub ending: String,
}

/// A part of a mass as the page keeps it, for `mass_join`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct PartFile {
    /// Its address in the plan: `credo/3`.
    pub part: String,
    /// Its file as `mass_part` gave it ([`PartAnswer::musicxml`]); empty for a part not set.
    pub musicxml: String,
    /// The seconds its search took, as the page measured them.
    pub seconds: f32,
    /// Why it is missing, where its search failed.
    pub error: String,
}

/// The plan of a mass as the page shows it and sets it (`mass_plan`).
#[derive(Clone, Debug, Serialize)]
pub struct MassView {
    pub title: String,
    /// The name of the files of the mass without extension, as `fux mass` names them: `messe`.
    pub file: String,
    /// The measures of all parts, as estimated.
    pub measures: usize,
    /// The seconds of all parts one after the other natively, as estimated.
    pub seconds: f32,
    /// A part takes about this many times as long in WebAssembly as natively.
    pub factor: f32,
    pub movements: Vec<MovementView>,
    /// The rows of the table of sections.
    pub rows: Vec<SectionRow>,
    /// The parts in the order of the mass.
    pub parts: Vec<PlannedPart>,
}

/// A movement of a planned mass.
#[derive(Clone, Debug, Serialize)]
pub struct MovementView {
    /// `kyrie` … `agnus`.
    pub id: String,
    /// Kyrie … Agnus Dei.
    pub title: String,
    /// Its place in the mass, Kyrie 1 … Agnus Dei 6, whichever movements are set.
    pub number: usize,
    /// The name of its files without extension: `messe-3-credo`.
    pub file: String,
    /// Its mode, as the page's language names it.
    pub mode: String,
    pub parts: usize,
}

/// A part of a planned mass, for the page's pool and its list of parts.
#[derive(Clone, Debug, Serialize)]
pub struct PlannedPart {
    /// Its address, by its number in its movement: `credo/3`.
    pub address: String,
    /// Its movement, as an index into [`MassView::movements`].
    pub movement: usize,
    /// How the lines of a mass name it: „Kyrie I“, „credo:5-7“, „Et incarnatus est“.
    pub label: String,
    /// The line of `fux mass` on it: „[ 7/26] Credo · credo:5-7 (gemeinsam), S A T B, Schluss
    /// D, Seed 4711“.
    pub line: String,
    /// Its measures and the seconds of its search natively, as estimated.
    pub measures: usize,
    pub seconds: f32,
    /// It opens with the head motif of Kyrie I: it is set after Kyrie I, with its motif.
    pub motif: bool,
    /// It gives the head motif to the parts that open with it (Kyrie I).
    pub source: bool,
}

/// A row of the page's table of sections (spec § 7): a section, or a part that begins at a
/// Break inside one, with what the plan has for it and what may be chosen.
#[derive(Clone, Debug, Serialize)]
pub struct SectionRow {
    /// Its address in the changes ([`SectionChoice`]): a section's id (`christe`), or the id of
    /// the text that begins at a Break (`incarnatus`).
    pub id: String,
    /// The section's title, or the Break's label: „Christe“, „Et incarnatus est“.
    pub title: String,
    /// The title of its movement.
    pub movement: String,
    /// `section`, or `break` for a part that begins at a Break, whose voices are its section's.
    pub kind: &'static str,
    /// Its voices from the top as planned: „S A T“.
    pub voices: String,
    /// Its first part (a Break's part) begins in imitation as planned; else together.
    pub imitative: bool,
    /// The degree its last part (a Break's part) ends on as planned, by its letter.
    pub ending: String,
    /// The degrees it may end on, by letter and as its mode spells them: the final, the fifth and
    /// the fourth with a clausula; the final alone at the end of a movement and in two voices.
    pub endings: Vec<Named>,
    /// „Osanna ut supra“: it repeats the Osanna of the Sanctus and has nothing to choose.
    pub ut_supra: bool,
}

/// A part as set (`mass_part`).
#[derive(Clone, Debug, Serialize)]
pub struct PartAnswer {
    /// Its address: `credo/3`.
    pub address: String,
    /// Its file as `fux mass` writes it to `NAME.teile/`, for `mass_join`.
    pub musicxml: String,
    pub measures: usize,
    pub cost: u32,
    pub forbidden: usize,
    /// The seed of the search that set it, as text (a number of JavaScript has 53 bits, a seed
    /// 64), and its attempt from 0 (MASS-SEED).
    pub seed: String,
    pub attempt: u8,
    /// What it did otherwise than planned, such as a fallback.
    pub warnings: Vec<String>,
    /// Its violations, its measures counted from 1.
    pub violations: Vec<Finding>,
    /// Kyrie I of a mass with a head motif: the motif of its first subject, in the text of the
    /// `motif=` item of `fux-part` (`0,2,3,4,5b,4`), which the page gives to the parts that open
    /// with it (`motif` of their requests). Left out where Kyrie I gives none (it begins
    /// together, or its first subject has not five to ten tones), and for every other part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motif: Option<String>,
}

/// A section of a movement as the page shows it (`mass_join`): the score of the voices that sing
/// in it, as the book prints it, its title as a rehearsal mark and the labels of its Breaks over
/// the barlines, its measures numbered through the movement.
#[derive(Clone, Debug, Serialize)]
pub struct ShownSection {
    pub title: String,
    pub musicxml: String,
}

/// A mass as a request asks for it: the request of the mass, the profile its parts are set with
/// (the profile of the mass with the changed rules and `--repeat`) and its plan.
struct Prepared {
    request: MassRequest,
    profile: Profile,
    plan: MassPlan,
}

/// The rows of a plan's table of sections that may be changed, by id, each with whether it is a
/// section, whose voices may change.
type Rows = Vec<(String, bool)>;

/// `mass_plan`: the plan of the mass of the request's form, without a search: the lines of `fux
/// mass --plan` and the plan as the page shows and sets it. Where the changes of the table of
/// sections do not fit (a degree without a clausula, an unknown voice), the answer is the error
/// together with the plan without them, so that the page shows the table to change them.
pub(crate) fn plan(request: &Request, base: &Profile, lang: Lang) -> Result<Answer> {
    let plain = prepare(request, base, None, lang)?;
    let rows = changeable(&plain.plan);
    Ok(match prepare(request, base, Some(&rows), lang) {
        Ok(mass) => Answer {
            ok: true,
            // Natively the time of `fux mass` on a machine of eight cores or more, which sets
            // four parts at once (K6).
            summary: mass_summary(&mass.plan, JOBS_MAX, lang)
                .lines()
                .map(str::to_string)
                .collect(),
            mass: Some(view(&mass.plan, lang)),
            ..Answer::default()
        },
        Err(error) => Answer {
            mass: Some(view(&plain.plan, lang)),
            ..Answer::failed(format!("{error:#}"))
        },
    })
}

/// `mass_part`: sets the part `request.part` (`credo/3`) of the mass of the request's form, as
/// `fux mass` sets it: three seeds and the fallbacks, the profile of the mass, the portable
/// budget. A part that opens with the head motif takes the motif of the request
/// ([`given_motif`]); without one its first subject comes from its words, with a warning, as in
/// `fux mass` where Kyrie I gives none. The answer has its file, which the page gives back to
/// `mass_join`, its summary as `fux mass --details` prints it, its violations, and for Kyrie I of
/// a mass with a head motif the motif it gives, for the parts that open with it. A part that no
/// attempt sets is an error that names the part, its words and what may help.
pub(crate) fn part(request: &Request, base: &Profile, lang: Lang) -> Result<Answer> {
    let mass = prepared(request, base, lang)?;
    let plan = &mass.plan;
    let at = one_part(plan, &request.part, lang)?;
    let motif = given_motif(request, plan, at, lang)?;
    let result = generate_part(&mass.request, plan, at, &mass.profile, motif.as_ref(), None)
        .map_err(|error| anyhow!("{}", error.message(lang)))?;
    let profile = part_profile(&mass.request, &mass.profile);
    let generated = &result.generated;
    let score = &generated.score;
    let summary = part_summary(&mass.request, plan, &result, &mass.profile, lang);
    Ok(Answer {
        ok: true,
        summary,
        part: Some(PartAnswer {
            address: plan.address(at),
            musicxml: part_file(plan, &result, lang),
            measures: score.measure_count(),
            cost: generated.cost,
            forbidden: generated.report.forbidden(),
            seed: result.seed.to_string(),
            attempt: result.attempt,
            warnings: result.warnings.iter().map(|w| w.message(lang)).collect(),
            violations: findings_after(score, &generated.report, &Checker::new(&profile), 0, lang),
            motif: result
                .motif
                .as_ref()
                .filter(|_| plan.motif_source() == Some(at))
                .map(Motif::to_string),
        }),
        ..Answer::default()
    })
}

/// The head motif the part at `at` of `plan` opens with (MASS-MOTIF): for a part that takes it,
/// the motif of the request (`motif`), as the answer for Kyrie I gave it ([`PartAnswer::motif`]),
/// in the text of the `motif=` item of `fux-part` (`0,2,3,4,5b,4`); `None` without one (`null` or
/// empty, where Kyrie I gave none or was not set) and for every other part. A motif the text does
/// not give is an error.
fn given_motif(
    request: &Request,
    plan: &MassPlan,
    at: PartRef,
    lang: Lang,
) -> Result<Option<Motif>> {
    let text = match request.motif.as_deref().map(str::trim) {
        Some(text) if plan.part(at).motif && !text.is_empty() => text,
        _ => return Ok(None),
    };
    text.parse().map(Some).map_err(|_| match lang {
        Lang::De => anyhow!(
            "„{text}“ ist kein Kopfmotiv: fünf bis zehn Stufen über der Finalis, durch Kommas \
             getrennt, eine alterierte mit b oder # dahinter."
        ),
        Lang::En => anyhow!(
            "\"{text}\" is no head motif: five to ten degrees above the final, separated by \
             commas, an altered one followed by b or #."
        ),
    })
}

/// `mass_join`: the mass of the request's form from its parts set so far (`request.parts`, as
/// `mass_part` gave them), each taken up as `fux mass --resume` takes up a file of a part and
/// checked again, Kyrie I first: a part that opens with the head motif only with the motif that
/// Kyrie I keeps, where it is set. The answer has a result per movement whose parts are all set,
/// with the file of the movement as `fux mass` writes it (`musicxml`; for the page's notation
/// its `sections`, each with the voices that sing in it), its MIDI file, its voices, the notes to
/// play and the violations of its parts in the measures of the movement; the LilyPond source of
/// the book with every movement, a section that lacks a part with why (`error` of the part, or
/// not set) and what helps ([`missing_remarks`]); the first line of the plan, the cadence plans,
/// the totals, the warnings and, unless the page is still setting parts (`request.running`), the
/// parts missing ([`incomplete`]).
pub(crate) fn join(request: &Request, base: &Profile, lang: Lang) -> Result<Answer> {
    let mass = prepared(request, base, lang)?;
    let profile = part_profile(&mass.request, &mass.profile);
    let plan = Arc::new(mass.plan);
    let mut m = Mass::new(Arc::clone(&plan));
    let mut failed = Vec::new();
    let mut given = Vec::new();
    for part in request.parts.set() {
        given.push((one_part(&plan, &part.part, lang)?, part));
    }
    // Kyrie I first, so that the motif it gives is known before the parts that take it.
    let source = plan.motif_source();
    given.sort_by_key(|&(at, _)| Some(at) != source);
    // The head motif as the file of Kyrie I keeps it, once it is taken; unknown before.
    let mut motif: Option<Option<Motif>> = None;
    for (at, given) in given {
        if given.musicxml.trim().is_empty() {
            let error = given.error.trim();
            if !error.is_empty() {
                failed.push((at, error.to_string()));
            }
            continue;
        }
        let kyrie_i = motif.as_ref().map(Option::as_ref);
        let mut result = taken(&plan, at, &given.musicxml, &profile, kyrie_i, lang)?;
        if source == Some(at) {
            motif = Some(result.motif.clone());
        }
        result.seconds = given.seconds.max(0.0);
        m.insert(result);
    }
    let checker = Checker::new(&profile);
    let results = (0..plan.movements.len())
        .filter_map(|movement| movement_entry(&m, movement, &checker, request, lang))
        .collect();
    let mut summary: Vec<String> = mass_summary(&plan, JOBS_MAX, lang)
        .lines()
        .take(1)
        .map(str::to_string)
        .collect();
    summary.extend(cadence_plans_of(&m, lang));
    let warnings = mass_warnings(&m, lang);
    summary.push(mass_totals(&m, 0, "", warnings.len(), lang));
    let warning = lang.pick("Warnung", "Warning");
    summary.extend(warnings.iter().map(|line| format!("{warning}: {line}")));
    if !request.running {
        summary.extend(incomplete(&m, &failed, lang));
    }
    Ok(Answer {
        book: Some(book(&m, &failed, request, lang)),
        ..Answer::results(summary, results)
    })
}

/// The mass of the request's form: the changes of its table of sections applied that name rows of
/// the plan without them (a row of another form, or of a movement not set, is left out), then the
/// form's own text of each change.
fn prepared(request: &Request, base: &Profile, lang: Lang) -> Result<Prepared> {
    let form = &request.mass;
    let unchanged = form.sections.is_empty()
        && [&form.scoring, &form.imitation, &form.endings]
            .iter()
            .all(|text| text.trim().is_empty());
    let plain = prepare(request, base, None, lang)?;
    if unchanged {
        return Ok(plain);
    }
    let rows = changeable(&plain.plan);
    prepare(request, base, Some(&rows), lang)
}

/// The mass of the request's form as `fux mass` prepares and plans it, with the changes to the
/// plan that `rows` admit; without changes for `None`. The plan checks the chant and the head
/// motif together (MASS-MOTIF: not with a chant, not without the Kyrie).
fn prepare(request: &Request, base: &Profile, rows: Option<&Rows>, lang: Lang) -> Result<Prepared> {
    let form = &request.mass;
    let (mass, profile) = prepare_mass(&options(form, rows)?, base, &rules(request), lang)?;
    let plan = mass_plan(&mass, lang).map_err(|error| sentence(&format!("{error:#}")))?;
    Ok(Prepared {
        request: mass,
        profile,
        plan,
    })
}

/// The options of `fux mass` that a form gives, with the changes to the plan that `rows` admit.
fn options(form: &MassForm, rows: Option<&Rows>) -> Result<MassOptions> {
    let defaults = MassOptions::default();
    let given = |text: &str, default: &str| match text.trim() {
        "" => default.to_string(),
        text => text.to_string(),
    };
    let [scoring, imitation, endings] = match rows {
        Some(rows) => changes(form, rows),
        None => [None, None, None],
    };
    Ok(MassOptions {
        preset: given(&form.preset, &defaults.preset),
        movements: given(&form.movements, &defaults.movements),
        voices: if form.voices == 0 {
            defaults.voices
        } else {
            form.voices
        },
        parts: text_or_none(&form.parts),
        mode: optional(&form.mode)?,
        scoring,
        imitation,
        endings,
        chunk: given(&form.chunk, &defaults.chunk),
        lengths: given(&form.lengths, &defaults.lengths),
        motif: form.motif,
        chant: given(&form.chant, &defaults.chant),
        singing: SingingArgs {
            repeat: optional(&form.repeat)?.unwrap_or_default(),
            accents: form.accents.unwrap_or(true),
            eleison: eleison_number(form.eleison)?,
        },
        // Always given: the time is unknown in WebAssembly.
        seed: Some(form.seed),
        beam: if form.beam == 0 {
            defaults.beam
        } else {
            form.beam
        },
        title: text_or_none(&form.title),
        notation: MassNotation {
            mensurstriche: form.mensurstriche,
            intonation: form.intonation.unwrap_or(true),
            hide_tacet: form.hide_tacet,
        },
        ..defaults
    })
}

/// The changes of `form` to the plan as `fux mass` takes them in `--scoring`, `--imitation` and
/// `--endings`: first the rows of its table that `rows` has, the voices of a section only, then
/// the form's own text of each option; later items win, so a Break's row wins over its section's
/// and the text over the table.
fn changes(form: &MassForm, rows: &Rows) -> [Option<String>; 3] {
    let (mut scoring, mut imitation, mut endings) = (Vec::new(), Vec::new(), Vec::new());
    for choice in &form.sections {
        let id = choice.id.trim();
        let Some(&(_, section)) = rows.iter().find(|(row, _)| row == id) else {
            continue;
        };
        let voices = choice.voices.trim();
        if section && !voices.is_empty() {
            scoring.push(format!("{id}={voices}"));
        }
        match choice.imitation.trim() {
            "" | "auto" => {}
            value => imitation.push(format!("{id}={value}")),
        }
        match choice.ending.trim() {
            "" | "auto" => {}
            value => endings.push(format!("{id}={value}")),
        }
    }
    let joined = |mut items: Vec<String>, text: &str| {
        if !text.trim().is_empty() {
            items.push(text.trim().to_string());
        }
        (!items.is_empty()).then(|| items.join("; "))
    };
    [
        joined(scoring, &form.scoring),
        joined(imitation, &form.imitation),
        joined(endings, &form.endings),
    ]
}

/// The rows of the table of sections of `plan` that may be changed: every row but an Osanna ut
/// supra, each with whether it is a section.
fn changeable(plan: &MassPlan) -> Rows {
    section_rows(plan, Lang::De)
        .into_iter()
        .filter(|row| !row.ut_supra)
        .map(|row| (row.id, row.kind == "section"))
        .collect()
}

/// The part that `address` names in `plan`: one part, by its number in its movement (`credo/3`)
/// or as the only part of a section or the part that begins at a Break.
fn one_part(plan: &MassPlan, address: &str, lang: Lang) -> Result<PartRef> {
    let address = address.trim();
    let found = plan
        .find(address)
        .map_err(|error| sentence(&error.message(lang)))?;
    match found.as_slice() {
        [at] if !address.is_empty() => Ok(*at),
        _ => bail!(
            "{}",
            match lang {
                Lang::De => format!(
                    "„{address}“ nennt nicht einen Teil der Messe; ein Teil heißt wie credo/3."
                ),
                Lang::En => format!(
                    "\"{address}\" names no single part of the mass; a part is named such as \
                     credo/3."
                ),
            }
        ),
    }
}

/// A message without its full stop, as the mass layer words them, as an error: with its capital
/// and its full stop.
fn sentence(message: &str) -> anyhow::Error {
    let message = message.trim();
    let mut chars = message.chars();
    let mut text: String = match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    };
    if !text.ends_with(['.', '!', '?']) {
        text.push('.');
    }
    anyhow!("{text}")
}

/// The plan as the page shows and sets it.
fn view(plan: &MassPlan, lang: Lang) -> MassView {
    let estimate = plan.estimate();
    let source = plan.motif_source();
    let base = base_name();
    MassView {
        title: plan.title.clone(),
        file: base.to_string(),
        measures: estimate.measures,
        seconds: estimate.seconds,
        factor: tables::ESTIMATE_WASM,
        movements: plan
            .movements
            .iter()
            .map(|movement| MovementView {
                id: movement.movement.id().to_string(),
                title: movement.movement.title().to_string(),
                number: movement.movement as usize + 1,
                file: movement_name(movement.movement),
                mode: movement.mode.name(lang),
                parts: movement.part_count(),
            })
            .collect(),
        rows: section_rows(plan, lang),
        parts: plan
            .parts()
            .map(|(at, part)| PlannedPart {
                address: plan.address(at),
                movement: at.movement,
                label: part_label(plan, at),
                line: PartLine::of(plan, at, lang).heading(lang),
                measures: part.estimate.measures,
                seconds: part.estimate.seconds,
                motif: part.motif,
                source: source == Some(at),
            })
            .collect(),
    }
}

/// The name of the files of a mass without extension, as `fux mass` names them by default:
/// `messe`.
fn base_name() -> &'static str {
    DEFAULT_BASE.rsplit('/').next().unwrap_or(DEFAULT_BASE)
}

/// The name of the files of a movement without extension, as `fux mass` names them: numbered as
/// the movements of a mass are (Kyrie 1 … Agnus Dei 6), whichever of them are set:
/// `messe-3-credo`.
fn movement_name(movement: Movement) -> String {
    format!(
        "{}-{}-{}",
        base_name(),
        movement as usize + 1,
        movement.id()
    )
}

/// The rows of the table of sections of `plan`: each section with its voices, how its first part
/// begins and the degree its last part ends on, and after it each part of it that begins at a
/// Break.
fn section_rows(plan: &MassPlan, lang: Lang) -> Vec<SectionRow> {
    let mut rows = Vec::new();
    for (m, movement) in plan.movements.iter().enumerate() {
        for (s, section) in movement.sections.iter().enumerate() {
            let voices: Vec<&str> = section
                .voices
                .iter()
                .map(|&v| movement.voices[v].abbreviation())
                .collect();
            let row = |id: &str, title: &str, kind, imitative, at: Option<PartRef>| SectionRow {
                id: id.to_string(),
                title: title.to_string(),
                movement: movement.movement.title().to_string(),
                kind,
                voices: voices.join(" "),
                imitative,
                ending: at.map_or(String::new(), |at| {
                    plan.part(at).ending.letter().to_string()
                }),
                endings: at.map_or(Vec::new(), |at| endings_of(plan, at, lang)),
                ut_supra: at.is_none(),
            };
            let Some(first) = section.parts.first() else {
                // „Osanna ut supra“: the Osanna of the Sanctus again.
                rows.push(row(section.id, section.title, "section", false, None));
                continue;
            };
            let at = |part| PartRef {
                movement: m,
                section: s,
                part,
            };
            rows.push(row(
                section.id,
                section.title,
                "section",
                first.opening == Opening::Imitative,
                Some(at(section.parts.len() - 1)),
            ));
            for (p, part) in section.parts.iter().enumerate() {
                if let Join::Break(label) = part.join {
                    rows.push(row(
                        part.first_text(),
                        label,
                        "break",
                        part.opening == Opening::Imitative,
                        Some(at(p)),
                    ));
                }
            }
        }
    }
    rows
}

/// The degrees the part at `at` may end on, as `--endings` takes them: the final alone for the
/// last part of a movement and for two voices, else the final, the fifth and the fourth where the
/// mode has a clausula (MASS-ENDING).
fn endings_of(plan: &MassPlan, at: PartRef, lang: Lang) -> Vec<Named> {
    let movement = &plan.movements[at.movement];
    let mode = movement.mode;
    let steps = if ends_movement(plan, at) || movement.sections[at.section].voices.len() == 2 {
        vec![mode.final_step()]
    } else {
        part_endings(mode)
    };
    steps
        .into_iter()
        .map(|step| Named {
            id: step.letter().to_string(),
            name: degree_name(mode, step, lang),
        })
        .collect()
}

/// Whether the part at `at` is the last of its movement, which ends on the final: no part follows
/// it, and no Osanna ut supra.
fn ends_movement(plan: &MassPlan, at: PartRef) -> bool {
    let movement = &plan.movements[at.movement];
    at.part + 1 == movement.sections[at.section].parts.len()
        && movement.sections[at.section + 1..]
            .iter()
            .all(|later| later.parts.is_empty() && later.ut_supra.is_none())
}

/// A degree as the scale of `mode` spells it: H and B in German, B and Bb in English.
fn degree_name(mode: ModeSpec, step: Step, lang: Lang) -> String {
    Pitch::new(step, mode.scale_alter(step), 4).class_name(lang)
}

/// The file of a part as `fux mass` writes it to `NAME.teile/` (`fux_cli::mass::write_part`):
/// its score as MusicXML, its voices and its head in `lang` and its movement and label as its
/// title, with the fields of its mass, of the part and of its lyrics. It is written with the
/// defaults of `fux mass` (modern clefs, the brackets over the entries, 72 halves a minute,
/// choir), whatever the page shows, so that a seed gives the same file natively and in
/// WebAssembly (D13).
fn part_file(plan: &MassPlan, result: &PartResult, lang: Lang) -> String {
    let at = result.at;
    let mut score = result.generated.score.clone();
    localize_names(&mut score, lang);
    score.title = Some(format!(
        "{} – {}",
        plan.movements[at.movement].movement.title(),
        part_label(plan, at)
    ));
    let part = PartField::of(plan, at, result, 1);
    let fields = [
        mass_field(plan),
        (FIELD_PART.to_string(), part.to_string()),
        (FIELD_LYRICS.to_string(), lyrics_field(&score)),
    ];
    let options = MusicXmlOptions {
        lang,
        ..MusicXmlOptions::default()
    };
    to_musicxml_with_fields(&score, &options, &fields)
}

/// The part at `at` of `plan` from its file `xml`, as `fux mass --resume` takes up the file of a
/// part: the file names the mass of the plan (`fux-mass`) and its part fits the plan's
/// ([`PartField::fits`]) under `profile`, the profile its parts are set with, and took the head
/// motif `motif`, the motif of the mass as the file of Kyrie I keeps it (`Some(None)` where it
/// keeps none), where that is known ([`PartField::motif_fits`]); its lyrics are aligned with
/// their text and sing the positions the file keeps ([`restore_part`]), its plan of imitation is
/// attached, and the part is checked again ([`part_from_parts`]). A file of the mass that another
/// version of the generator set ([`GENERATOR`]), as a worker that loaded an older module from the
/// browser's cache might set it, is refused with both versions.
fn taken(
    plan: &MassPlan,
    at: PartRef,
    xml: &str,
    profile: &Profile,
    motif: Option<Option<&Motif>>,
    lang: Lang,
) -> Result<PartResult> {
    let address = plan.address(at);
    let named = || format!("{} {address}", lang.pick("Teil", "Part"));
    let (read, fields) = musicxml_text_fields(xml, None, lang).with_context(named)?;
    let field = |name: &str| {
        fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, value)| value.as_str())
    };
    let mass: Option<MassField> = field(FIELD_MASS).and_then(|value| value.parse().ok());
    let part: Option<PartField> = field(FIELD_PART).and_then(|value| value.parse().ok());
    if let Some(other) = part.as_ref().map(|part| part.generator)
        && other != GENERATOR
        && mass == Some(MassField::of(plan))
    {
        bail!(
            "{}",
            match lang {
                Lang::De => format!(
                    "Teil {address}: Die Datei stammt von einer anderen Version des Generators \
                     (Version {other}, diese Seite {GENERATOR}). Bitte die Seite neu laden und die \
                     Messe noch einmal setzen."
                ),
                Lang::En => format!(
                    "Part {address}: the file comes from another version of the generator \
                     (version {other}, this page {GENERATOR}). Reload the page and compose the \
                     mass again."
                ),
            }
        );
    }
    let Some(part) = part.filter(|part| {
        mass == Some(MassField::of(plan)) && part.fits(plan, at, profile_hash(profile))
    }) else {
        bail!(
            "{}",
            match lang {
                Lang::De => format!(
                    "Teil {address}: Die Datei gehört nicht zu dieser Messe (andere Angaben oder \
                     ein anderer Seed)."
                ),
                Lang::En => format!(
                    "Part {address}: the file does not belong to this mass (other choices or \
                     another seed)."
                ),
            }
        );
    };
    if motif.is_some_and(|motif| !part.motif_fits(plan, at, motif)) {
        bail!(
            "{}",
            match lang {
                Lang::De => format!(
                    "Teil {address}: Die Datei wurde mit einem anderen Kopfmotiv gesetzt, als das \
                     Kyrie I gibt."
                ),
                Lang::En => format!(
                    "Part {address}: the file was set with another head motif than Kyrie I gives."
                ),
            }
        );
    }
    let mut score = read.score;
    align_lyrics(&mut score, read.text.as_deref(), lang).with_context(named)?;
    restore_part(&mut score, &part, &fields);
    attach(&mut score, read.imitation, ImitationSource::File, "", lang).with_context(named)?;
    part_from_parts(plan, at, score, &part, profile).map_err(|error| sentence(&error.message(lang)))
}

/// The movement `movement` of `m` as a result of the page, where all its parts are set: its file
/// as `fux mass` writes it, with the notation of the request's form and the clefs, brackets and
/// sound of the page; for the page's notation the same with the voices that rest through a
/// section hidden there; its MIDI file; its voices as text and what they sing; the notes to
/// play; and the violations of its parts, each checked on its own as `fux check` checks the file
/// of the movement, in the measures of the movement.
fn movement_entry(
    m: &Mass,
    movement: usize,
    checker: &Checker,
    request: &Request,
    lang: Lang,
) -> Option<Entry> {
    let (score, fields) = movement_file(m, movement, lang)?;
    let plan = &m.plan;
    let form = &request.mass;
    let planned = &plan.movements[movement];
    let title = planned.movement.title();
    let heading = MovementHeading {
        mass: plan.title.clone(),
        number: planned.movement as usize + 1,
        title: title.to_string(),
        byline: Some(format!(
            "{}, {} · {}",
            planned.mode.name(lang),
            choir_of(plan),
            byline(plan, lang)
        )),
        intonation: planned
            .intonation
            .filter(|_| form.intonation.unwrap_or(true))
            .map(str::to_string),
    };
    let midi = midi_options(request);
    let options = MovementOptions {
        musicxml: musicxml_options(request, lang),
        mensurstriche: form.mensurstriche,
        hide_tacet: form.hide_tacet,
    };
    let musicxml = to_movement_musicxml(&score, &heading, &options, &fields);
    let shown = MovementHeading {
        mass: String::new(),
        number: 0,
        title: String::new(),
        byline: None,
        ..heading
    };
    let sections = shown_sections(m, movement, &shown, &options, lang);
    let name = format!("{} – {title}", plan.title);
    let (mut violations, mut cost, mut forbidden, mut before) = (Vec::new(), 0, 0, 0);
    for (at, _) in plan.parts().filter(|(at, _)| at.movement == movement) {
        let part = &m.result(at)?.generated;
        violations.extend(findings_after(
            &part.score,
            &part.report,
            checker,
            before,
            lang,
        ));
        cost += part.report.cost();
        forbidden += part.report.forbidden();
        before += part.score.measure_count();
    }
    Some(Entry {
        label: format!("{title}, {} {cost}", lang.pick("Kosten", "cost")),
        cost,
        forbidden,
        lines: voice_lines(&score),
        lyrics: voice_lyrics(&score),
        violations,
        musicxml,
        // The book of the mass is the LilyPond of all its movements (`Answer::book`).
        lilypond: String::new(),
        midi: base64::encode(&to_movement_midi(&score, &name, &midi)),
        notes: sounds(&score),
        measure: score.meter.ticks_per_measure(),
        sections,
        file: Some(movement_name(planned.movement)),
    })
}

/// The sections of the movement `movement` of `m` as the page shows them (spec § 7: the voices
/// that rest through a section hidden there): each the score of the voices that sing in it, as
/// the book prints it ([`section_score`]), its voices named as the movement names them in `lang`,
/// its title as a rehearsal mark, the labels of its Breaks over the barlines, the intonation of
/// `heading` over the first, in the notation of `options`, its measures numbered through the
/// movement. Verovio (6.3) draws a staff that the file of the movement hides for some measures
/// (`print-object="no"`) all the same, so the page engraves the sections one after the other.
fn shown_sections(
    m: &Mass,
    movement: usize,
    heading: &MovementHeading,
    options: &MovementOptions,
    lang: Lang,
) -> Vec<ShownSection> {
    let planned = &m.plan.movements[movement];
    let names = voice_names(&planned.voices, lang);
    let mut first = 1;
    let mut shown = Vec::with_capacity(planned.sections.len());
    for (s, section) in planned.sections.iter().enumerate() {
        let Some(mut score) = section_score(m, movement, s) else {
            return Vec::new();
        };
        let measures = score.measure_count();
        for (voice, name) in score.voices.iter_mut().zip(&names) {
            voice.name = name.clone();
        }
        let mut score = singing(score);
        score.sections.insert(
            0,
            SectionMark {
                measure: 0,
                kind: SectionKind::Section,
                title: Some(section.title.to_string()),
            },
        );
        let heading = MovementHeading {
            intonation: heading.intonation.clone().filter(|_| s == 0),
            ..heading.clone()
        };
        let xml = to_movement_musicxml(&score, &heading, options, &[]);
        shown.push(ShownSection {
            title: section.title.to_string(),
            musicxml: numbered_from(&xml, first),
        });
        first += measures;
    }
    shown
}

/// `score` with only the voices that sing in it, a section of a movement as the book prints it:
/// which voices sing the text and the voices of the entries of its imitation follow.
fn singing(mut score: Score) -> Score {
    let sings: Vec<bool> = score
        .voices
        .iter()
        .map(|voice| voice.notes.iter().any(|note| !note.is_rest()))
        .collect();
    if sings.iter().all(|&sings| sings) {
        return score;
    }
    let mut places = Vec::with_capacity(sings.len());
    let mut kept = 0;
    for &voice in &sings {
        places.push(kept);
        kept += usize::from(voice);
    }
    let mut voices = sings.iter();
    score.voices.retain(|_| *voices.next().unwrap_or(&true));
    if let Some(text) = &mut score.text {
        let mut voices = sings.iter();
        text.voices.retain(|_| *voices.next().unwrap_or(&true));
    }
    if let Some(imitation) = &score.imitation {
        let mut plan = (**imitation).clone();
        for entry in plan.points.iter_mut().flat_map(|point| &mut point.entries) {
            entry.voice = places.get(entry.voice).copied().unwrap_or(entry.voice);
        }
        score.imitation = Some(Arc::new(plan));
    }
    score
}

/// MusicXML of a score with its measures numbered from `first` instead of 1, for a section shown
/// with the measures of its movement.
fn numbered_from(xml: &str, first: usize) -> String {
    const MEASURE: &str = "<measure number=\"";
    let mut numbered = String::with_capacity(xml.len() + xml.len() / 50);
    let mut rest = xml;
    while let Some(at) = rest.find(MEASURE) {
        let (before, after) = rest.split_at(at + MEASURE.len());
        numbered.push_str(before);
        let end = after.find('"').unwrap_or(0);
        match after[..end].parse::<usize>() {
            Ok(number) => numbered.push_str(&(number + first - 1).to_string()),
            Err(_) => numbered.push_str(&after[..end]),
        }
        rest = &after[end..];
    }
    numbered.push_str(rest);
    numbered
}

/// The LilyPond source of the book of `m` as `fux mass` writes it, with the notation of the
/// request's form and the clefs and brackets of the page: a section that lacks a part shows why,
/// the error of its search from `failed`, or that it is not set, and what helps
/// ([`missing_remarks`]).
fn book(m: &Mass, failed: &[(PartRef, String)], request: &Request, lang: Lang) -> String {
    let plan = &m.plan;
    let form = &request.mass;
    let options = MassBookOptions {
        subtitle: Some(format!("{}, {}", mode_name(plan, lang), choir_of(plan))),
        byline: Some(byline(plan, lang)),
        intonation: form.intonation.unwrap_or(true),
    };
    let lilypond = fux_export::LilypondOptions {
        mensurstriche: form.mensurstriche,
        ..lilypond_options(request, lang)
    };
    let missing =
        |movement, section| missing_remarks(m, failed, request.running, movement, section, lang);
    mass_book(m, &options, &missing, &lilypond)
}

/// The mode of a mass as its lines name it, as `fux mass` names it: the one mode of its
/// movements, or „Modus je Satz“ where they have several (with a chant).
fn mode_name(plan: &MassPlan, lang: Lang) -> String {
    match plan.movements.first().map(|movement| movement.mode) {
        Some(mode) if plan.movements.iter().all(|movement| movement.mode == mode) => {
            mode.name(lang)
        }
        _ => lang.pick("Modus je Satz", "mode per movement").to_string(),
    }
}

/// The choir of a mass by its abbreviations: „S A T B“.
fn choir_of(plan: &MassPlan) -> String {
    let mut choir = plan.choir.clone();
    choir.sort();
    choir
        .iter()
        .map(|voice| voice.abbreviation())
        .collect::<Vec<_>>()
        .join(" ")
}

/// „fux-kontrapunkt, Seed 4711“.
fn byline(plan: &MassPlan, lang: Lang) -> String {
    format!(
        "fux-kontrapunkt, {} {}",
        lang.pick("Seed", "seed"),
        plan.seed
    )
}

/// How the page sets the parts a mass lacks that did not fail: after a cancel, or where a worker
/// broke off.
fn page_hint(lang: Lang) -> &'static str {
    lang.pick(
        "Fehlende Teile setzt die Seite nach, solange sie geöffnet ist („Fehlende Teile setzen“).",
        "The page sets missing parts while it is open (“Set missing parts”).",
    )
}

/// What helps a part whose search failed: „Fehlende Teile setzen“ leaves it, as its seed would
/// fail again; another seed for the mass or other choices may set it.
fn failed_hint(lang: Lang) -> &'static str {
    lang.pick(
        "„Fehlende Teile setzen“ lässt einen gescheiterten Teil aus, denn mit seinem Seed \
         scheiterte er wieder; es helfen ein anderer Seed der Messe oder eine andere Wahl in der \
         Tabelle der Abschnitte oder bei der Größe der Teile, dann „Messe setzen“.",
        "“Set missing parts” leaves out a part that failed, as its seed would fail again; another \
         seed for the mass or another choice in the table of sections or for the size of the \
         parts may help, then “Compose the mass”.",
    )
}

/// The hints after the parts `missing` of a mass: how the page sets those not set, unless it is
/// setting them now (`running`), and what helps those whose search failed (in `failed`).
fn hints(
    missing: &[PartRef],
    failed: &[(PartRef, String)],
    running: bool,
    lang: Lang,
) -> Vec<&'static str> {
    let failing = |at: &PartRef| failed.iter().any(|(place, _)| place == at);
    let mut hints = Vec::new();
    if !running && missing.iter().any(|at| !failing(at)) {
        hints.push(page_hint(lang));
    }
    if missing.iter().any(failing) {
        hints.push(failed_hint(lang));
    }
    hints
}

/// The line on a mass that lacks parts: which, and how the page sets them or what helps where
/// their search failed (in `failed`); `None` for a complete mass.
fn incomplete(m: &Mass, failed: &[(PartRef, String)], lang: Lang) -> Option<String> {
    let missing: Vec<PartRef> = m
        .plan
        .parts()
        .zip(&m.results)
        .filter(|(_, result)| result.is_none())
        .map(|((at, _), _)| at)
        .collect();
    if missing.is_empty() {
        return None;
    }
    let addresses: Vec<String> = missing.iter().map(|&at| m.plan.address(at)).collect();
    let mut line = format!(
        "{}: {}.",
        lang.pick("Unvollständig, es fehlen", "Incomplete, missing"),
        addresses.join(", ")
    );
    for hint in hints(&missing, failed, false, lang) {
        line.push(' ');
        line.push_str(hint);
    }
    Some(line)
}

/// The widest line of a remark under a section in the book, in characters, as `fux mass` breaks
/// them.
const REMARK_WIDTH: usize = 85;

/// The remarks under the section `section` of the movement `movement` of `m` where it lacks
/// parts (those of the Osanna it repeats for an Osanna ut supra), as `fux mass` writes them but
/// with the page's ways to set them: for each part missing why, the error of its search where
/// `failed` has one, else that it is not set; then how the page sets the parts not set, unless
/// it is setting them now (`running`), and what helps a part whose search failed ([`hints`]);
/// each in lines the page has room for.
fn missing_remarks(
    m: &Mass,
    failed: &[(PartRef, String)],
    running: bool,
    movement: usize,
    section: usize,
    lang: Lang,
) -> Vec<String> {
    let plan = &m.plan;
    let own = &plan.movements[movement].sections[section];
    let (from, repeated) = own.ut_supra.unwrap_or((movement, section));
    let parts = plan.movements[from].sections[repeated].parts.len();
    let missing: Vec<PartRef> = (0..parts)
        .map(|part| PartRef {
            movement: from,
            section: repeated,
            part,
        })
        .filter(|&at| m.result(at).is_none())
        .collect();
    let mut remarks: Vec<String> = missing
        .iter()
        .map(|&at| match failed.iter().find(|(place, _)| *place == at) {
            Some((_, error)) => error.clone(),
            None => {
                let address = plan.address(at);
                match lang {
                    Lang::De => format!("Teil {address} fehlt: nicht gesetzt."),
                    Lang::En => format!("Part {address} is missing: not set."),
                }
            }
        })
        .collect();
    remarks.extend(
        hints(&missing, failed, running, lang)
            .into_iter()
            .map(str::to_string),
    );
    remarks
        .iter()
        .flat_map(|remark| wrapped(remark, REMARK_WIDTH))
        .collect()
}

/// `text` in lines of at most `width` characters, broken between words; a word wider than
/// `width` has a line of its own.
fn wrapped(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::answer_json;
    use serde_json::{Value, json};

    fn ask(request: Value) -> Value {
        serde_json::from_str(&answer_json(&request.to_string())).unwrap()
    }

    fn strings(value: &Value) -> Vec<String> {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap().to_string())
            .collect()
    }

    /// A small mass to set in the tests: a Kyrie in two voices, whose parts begin together and
    /// take less than a second each with a narrow search.
    fn kyrie(seed: u64) -> Value {
        json!({"movements": "kyrie", "voices": 2, "beam": 40, "seed": seed})
    }

    /// The plan as the page shows and sets it: the lines of `fux mass --plan`, the movements with
    /// the names of their files, the parts with their lines and estimates, the table of sections
    /// with the parts that begin at Breaks and the endings each row may choose.
    #[test]
    fn the_plan_as_the_page_shows_it() {
        let plan = ask(json!({"command": "mass_plan", "mass": {"seed": 1}}));
        assert_eq!(plan["ok"], true, "{plan}");
        assert_eq!(
            plan["summary"][0],
            "Missa brevis – dorisch, S A T B, Seed 1: 6 Sätze, 10 Abschnitte, 26 Teile, 657 \
             Silben, Hauptstufe A"
        );
        let mass = &plan["mass"];
        assert_eq!(mass["file"], "messe");
        assert_eq!(mass["factor"], f64::from(tables::ESTIMATE_WASM));
        let files: Vec<&str> = mass["movements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["file"].as_str().unwrap())
            .collect();
        assert_eq!(
            files,
            [
                "messe-1-kyrie",
                "messe-2-gloria",
                "messe-3-credo",
                "messe-4-sanctus",
                "messe-5-benedictus",
                "messe-6-agnus"
            ]
        );
        let parts = mass["parts"].as_array().unwrap();
        assert_eq!(parts.len(), 26);
        assert_eq!(parts[0]["address"], "kyrie/1");
        assert_eq!(parts[0]["label"], "Kyrie I");
        assert!(
            parts[0]["line"]
                .as_str()
                .unwrap()
                .starts_with("[ 1/26] Kyrie · Kyrie I (Imitation), S A T B, Schluss D, Seed "),
            "{plan}"
        );
        let seconds: f64 = parts.iter().map(|p| p["seconds"].as_f64().unwrap()).sum();
        let total = mass["seconds"].as_f64().unwrap();
        assert!((seconds - total).abs() < 0.01 * total, "{seconds} {total}");
        assert!(
            parts
                .iter()
                .all(|p| p["motif"] == false && p["source"] == false)
        );
        let rows = mass["rows"].as_array().unwrap();
        let ids: Vec<&str> = rows.iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert_eq!(
            ids,
            [
                "kyrie",
                "christe",
                "kyrie2",
                "gloria",
                "qui-tollis",
                "credo",
                "incarnatus",
                "crucifixus",
                "sanctus",
                "benedictus",
                "osanna2",
                "agnus",
                "agnus3"
            ]
        );
        let row = |id: &str| rows.iter().find(|r| r["id"] == id).unwrap().clone();
        let endings = |id: &str| -> Vec<String> {
            row(id)["endings"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["id"].as_str().unwrap().to_string())
                .collect()
        };
        // The Christe may end on the final, the fifth or the fourth; the end of a movement only
        // on the final.
        assert_eq!(endings("christe"), ["D", "A", "G"]);
        assert_eq!(endings("kyrie2"), ["D"]);
        assert_eq!(row("benedictus")["voices"], "S A T");
        assert_eq!(row("agnus3")["voices"], "S A T T B");
        let incarnatus = row("incarnatus");
        assert_eq!(
            [
                &incarnatus["kind"],
                &incarnatus["title"],
                &incarnatus["imitative"]
            ],
            [&json!("break"), &json!("Et incarnatus est"), &json!(false)]
        );
        // The form normal repeats the Osanna of the Sanctus: nothing to choose there.
        let normal = ask(json!({"command": "mass_plan", "lang": "en",
            "mass": {"seed": 1, "preset": "normal", "voices": 3, "movements": "sanctus,benedictus"}}));
        assert_eq!(normal["ok"], true, "{normal}");
        let rows = normal["mass"]["rows"].as_array().unwrap();
        let osanna = rows.iter().find(|r| r["id"] == "osanna2").unwrap();
        assert_eq!(osanna["ut_supra"], true);
        assert_eq!(osanna["endings"], json!([]));
        assert!(
            normal["summary"][0]
                .as_str()
                .unwrap()
                .starts_with("Missa – Dorian, A T B, seed 1: 2 movements"),
            "{normal}"
        );
    }

    /// The rows of the table of sections change the plan as `--scoring`, `--imitation` and
    /// `--endings` do, a Break's row after its section's; a row the plan does not have (another
    /// form, a movement not set) is left out. A change that does not fit fails with the plan
    /// without the changes, whose rows the page shows.
    #[test]
    fn the_table_of_sections_changes_the_plan() {
        let planned = |mass: Value| ask(json!({"command": "mass_plan", "mass": mass}));
        let changed = planned(json!({"seed": 1, "sections": [
            {"id": "christe", "ending": "D"},
            {"id": "credo", "imitation": "aus"},
            {"id": "incarnatus", "imitation": "an"},
            {"id": "benedictus", "voices": "S A T B"},
            {"id": "pleni", "imitation": "aus"},
            {"id": "osanna2", "ending": "auto", "imitation": "auto", "voices": ""}
        ]}));
        assert_eq!(changed["ok"], true, "{changed}");
        let rows = changed["mass"]["rows"].as_array().unwrap();
        let row = |id: &str| rows.iter().find(|r| r["id"] == id).unwrap().clone();
        assert_eq!(row("christe")["ending"], "D");
        assert_eq!(row("credo")["imitative"], false);
        assert_eq!(row("incarnatus")["imitative"], true);
        assert_eq!(row("benedictus")["voices"], "S A T B");
        let credo = changed["summary"][4].as_str().unwrap();
        assert!(
            credo.starts_with("Credo: credo:1-4 (gemeinsam) → ")
                && credo.contains("Et incarnatus est (Imitation, Takt Pause)"),
            "{credo}"
        );
        // The rows of a movement not set are left out; the form's own text comes after them.
        let alone = planned(json!({"seed": 1, "movements": "kyrie",
            "sections": [{"id": "incarnatus", "imitation": "an"}, {"id": "christe", "ending": "A"}],
            "endings": "christe=G"}));
        assert_eq!(alone["ok"], true, "{alone}");
        assert_eq!(alone["mass"]["rows"][1]["ending"], "G");
        // What does not fit fails, with the rows of the plan without the changes.
        let wrong = planned(json!({"seed": 1, "sections": [{"id": "christe", "voices": "S X"}]}));
        assert_eq!(wrong["ok"], false);
        assert_eq!(wrong["error"], "„christe“: Unbekannte Stimmlage „X“.");
        assert_eq!(wrong["mass"]["rows"].as_array().unwrap().len(), 13);
        let last = planned(json!({"seed": 1, "sections": [{"id": "kyrie2", "ending": "A"}]}));
        assert_eq!(
            last["error"],
            "„kyrie2“: Der letzte Teil des Kyrie endet auf der Finalis."
        );
        // A form that cannot be planned at all fails without a plan.
        let chunk = ask(json!({"command": "mass_plan", "lang": "en", "mass": {"chunk": "10"}}));
        assert_eq!(
            chunk["error"],
            "The size of the parts \"10\" is neither a number of 20 to 200 syllables nor \"aus\" \
             (off)."
        );
        assert!(chunk["mass"].is_null());
    }

    /// The chant and the head motif reach the plan: each movement in the mode of its chant, the
    /// parts that open with the motif and the part that gives it; what cannot go together fails.
    #[test]
    fn chant_and_motif_reach_the_plan() {
        let planned = |mass: Value| ask(json!({"command": "mass_plan", "mass": mass}));
        let motif = planned(json!({"seed": 1, "motif": true}));
        assert_eq!(motif["ok"], true, "{motif}");
        let parts = motif["mass"]["parts"].as_array().unwrap();
        let with: Vec<&str> = parts
            .iter()
            .filter(|p| p["motif"] == true)
            .map(|p| p["address"].as_str().unwrap())
            .collect();
        assert_eq!(with, ["gloria/1", "credo/1", "sanctus/1", "agnus/1"]);
        let sources: Vec<&str> = parts
            .iter()
            .filter(|p| p["source"] == true)
            .map(|p| p["address"].as_str().unwrap())
            .collect();
        assert_eq!(sources, ["kyrie/1"]);
        let chant = planned(json!({"seed": 1, "chant": "ix"}));
        assert_eq!(chant["ok"], true, "{chant}");
        assert!(
            chant["summary"][0]
                .as_str()
                .unwrap()
                .starts_with("Missa brevis – Modus je Satz, "),
            "{chant}"
        );
        let modes: Vec<&str> = chant["mass"]["movements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["mode"].as_str().unwrap())
            .collect();
        assert_ne!(modes[0], modes[1], "{modes:?}");
        for (mass, error) in [
            (
                json!({"seed": 1, "motif": true, "chant": "ix"}),
                "Kopfmotiv und Choralparaphrase schließen einander aus.",
            ),
            (
                json!({"seed": 1, "motif": true, "movements": "gloria"}),
                "Das Kopfmotiv kommt aus dem Kyrie I; dazu gehört das Kyrie.",
            ),
            (
                json!({"seed": 1, "chant": "x"}),
                "Unbekannter Choral „x“; zur Wahl stehen keiner, ix und iv.",
            ),
        ] {
            assert_eq!(planned(mass)["error"], error);
        }
        assert_eq!(planned(json!({"seed": 1, "chant": "keiner"}))["ok"], true);
    }

    /// A part as a worker sets it, and the mass of its parts: the file of the part is the one
    /// `fux mass` writes, with the version of the generator that set it; the movement whose
    /// parts are all set comes back as its files with the costs of its parts, the version of each
    /// in its field, the book with every movement; a part missing shows in the book and in the
    /// lines; a part of another mass, or of another version of the generator, is refused.
    #[test]
    fn a_part_and_the_mass_of_its_parts() {
        let set = |address: &str| {
            let answer = ask(json!({"command": "mass_part", "mass": kyrie(1), "part": address}));
            assert_eq!(answer["ok"], true, "{answer}");
            answer["part"].clone()
        };
        let parts: Vec<Value> = ["kyrie/1", "kyrie/2", "kyrie/3"]
            .into_iter()
            .map(set)
            .collect();
        let version = format!(";generator={GENERATOR}");
        for (part, address) in parts.iter().zip(["kyrie/1", "kyrie/2", "kyrie/3"]) {
            assert_eq!(part["address"], address);
            assert_eq!(part["forbidden"], 0);
            assert!(part["measures"].as_u64().unwrap() > 0);
            let xml = part["musicxml"].as_str().unwrap();
            assert!(xml.contains("<miscellaneous-field name=\"fux-part\">bars=1-"));
            assert!(xml.contains("<miscellaneous-field name=\"fux-mass\">preset=brevis;"));
            let read = fux_export::from_musicxml(xml).unwrap();
            let field = read.field(FIELD_PART).unwrap();
            assert!(field.ends_with(&version), "{field}");
        }
        // The same seed sets the same part.
        assert_eq!(set("kyrie/2"), parts[1]);
        let files: Vec<Value> = parts
            .iter()
            .map(|part| json!({"part": part["address"], "musicxml": part["musicxml"], "seconds": 1.5}))
            .collect();
        let joined = ask(json!({"command": "mass_join", "mass": kyrie(1), "parts": files}));
        assert_eq!(joined["ok"], true, "{joined}");
        let results = joined["results"].as_array().unwrap();
        assert_eq!(results.len(), 1);
        let kyrie_file = &results[0];
        assert_eq!(kyrie_file["file"], "messe-1-kyrie");
        let cost: u64 = parts.iter().map(|p| p["cost"].as_u64().unwrap()).sum();
        assert_eq!(kyrie_file["cost"], cost);
        assert_eq!(kyrie_file["label"], format!("Kyrie, Kosten {cost}"));
        let violations: usize = parts
            .iter()
            .map(|p| p["violations"].as_array().unwrap().len())
            .sum();
        assert_eq!(
            kyrie_file["violations"].as_array().unwrap().len(),
            violations
        );
        let xml = kyrie_file["musicxml"].as_str().unwrap();
        assert!(xml.contains("<movement-title>Kyrie</movement-title>"));
        assert!(xml.contains("name=\"fux-part.3\""));
        let read = fux_export::from_musicxml(xml).unwrap();
        for n in 1..=3 {
            let field = read.field(&format!("{FIELD_PART}.{n}")).unwrap();
            assert!(field.ends_with(&version), "{field}");
        }
        // The page shows the sections one after the other, their measures counted on.
        let sections = kyrie_file["sections"].as_array().unwrap();
        let titles: Vec<&str> = sections
            .iter()
            .map(|section| section["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, ["Kyrie I", "Christe", "Kyrie II"]);
        let first = parts[0]["measures"].as_u64().unwrap();
        let christe = sections[1]["musicxml"].as_str().unwrap();
        assert!(
            christe.contains(&format!("<measure number=\"{}\">", first + 1))
                && !christe.contains("<measure number=\"1\">")
                && christe.contains("<rehearsal>Christe</rehearsal>"),
            "{christe}"
        );
        assert!(kyrie_file["midi"].as_str().unwrap().starts_with("TVRoZA"));
        assert_eq!(kyrie_file["lines"].as_array().unwrap().len(), 2);
        let book = joined["book"].as_str().unwrap();
        assert!(book.contains("\\book") && book.contains("Kyrie"), "{book}");
        let summary = strings(&joined["summary"]);
        assert!(summary[0].starts_with("Missa brevis – dorisch, S T, Seed 1: 1 Satz"));
        assert!(
            summary[1].starts_with("Kyrie – Kadenzplan: "),
            "{summary:?}"
        );
        assert!(
            summary[2].starts_with("Gesamt: 3 von 3 Teilen, ")
                && summary[2].ends_with(", Suchzeit 0:05, keine Warnung"),
            "{summary:?}"
        );
        // A part missing: no file of its movement; the book and the lines say why.
        let some = json!([
            {"part": "kyrie/1", "error": "Teil kyrie/1 ließ sich nicht setzen."},
            files[1],
        ]);
        let incomplete = ask(json!({"command": "mass_join", "mass": kyrie(1), "parts": some}));
        assert_eq!(incomplete["ok"], true, "{incomplete}");
        assert_eq!(incomplete["results"], json!([]));
        let book = incomplete["book"].as_str().unwrap();
        assert!(
            book.contains("Teil kyrie/1 ließ sich nicht setzen.")
                && book.contains("Teil kyrie/3 fehlt: nicht gesetzt."),
            "{book}"
        );
        let summary = strings(&incomplete["summary"]);
        assert!(
            summary
                .last()
                .unwrap()
                .starts_with("Unvollständig, es fehlen: kyrie/1, kyrie/3. "),
            "{summary:?}"
        );
        // „Fehlende Teile setzen“ sets a part not set, not one whose search failed: for that one
        // what helps instead. While the page is setting parts, those not set go without saying.
        let set_later = "Fehlende Teile setzt die Seite nach";
        let left_out = "„Fehlende Teile setzen“ lässt einen gescheiterten Teil aus";
        let last = summary.last().unwrap();
        assert!(
            last.contains(set_later) && last.contains(left_out),
            "{last}"
        );
        assert!(
            book.contains(set_later) && book.contains(left_out),
            "{book}"
        );
        let join = |parts: Value, running: bool, lang: &str| {
            let request = json!({"command": "mass_join", "mass": kyrie(1), "parts": parts,
                "running": running, "lang": lang});
            let answer = ask(request);
            assert_eq!(answer["ok"], true, "{answer}");
            let book = answer["book"].as_str().unwrap().to_string();
            (strings(&answer["summary"]), book)
        };
        let failed = json!([
            {"part": "kyrie/1", "error": "Teil kyrie/1 ließ sich nicht setzen."},
            files[1],
            files[2],
        ]);
        let (summary, book) = join(failed.clone(), false, "de");
        assert_eq!(
            summary.last().unwrap(),
            "Unvollständig, es fehlen: kyrie/1. „Fehlende Teile setzen“ lässt einen gescheiterten \
             Teil aus, denn mit seinem Seed scheiterte er wieder; es helfen ein anderer Seed der \
             Messe oder eine andere Wahl in der Tabelle der Abschnitte oder bei der Größe der \
             Teile, dann „Messe setzen“."
        );
        assert!(
            book.contains(left_out) && !book.contains(set_later),
            "{book}"
        );
        let (summary, _) = join(failed, false, "en");
        assert_eq!(
            summary.last().unwrap(),
            "Incomplete, missing: kyrie/1. “Set missing parts” leaves out a part that failed, as \
             its seed would fail again; another seed for the mass or another choice in the table \
             of sections or for the size of the parts may help, then “Compose the mass”."
        );
        let (summary, book) = join(json!([files[1]]), false, "de");
        let last = summary.last().unwrap();
        assert!(
            last.contains(set_later) && !last.contains(left_out),
            "{last}"
        );
        assert!(
            book.contains(set_later) && !book.contains(left_out),
            "{book}"
        );
        let (summary, book) = join(json!([files[1]]), true, "de");
        assert!(
            summary
                .iter()
                .all(|line| !line.starts_with("Unvollständig")),
            "{summary:?}"
        );
        assert!(
            book.contains("Teil kyrie/1 fehlt: nicht gesetzt.") && !book.contains(set_later),
            "{book}"
        );
        // A part of another mass, or at another place.
        let other = ask(json!({"command": "mass_join", "mass": kyrie(2), "parts": [files[1]]}));
        assert_eq!(
            other["error"],
            "Teil kyrie/2: Die Datei gehört nicht zu dieser Messe (andere Angaben oder ein \
             anderer Seed)."
        );
        let moved = json!([{"part": "kyrie/3", "musicxml": parts[1]["musicxml"]}]);
        let moved = ask(json!({"command": "mass_join", "mass": kyrie(1), "parts": moved}));
        assert_eq!(moved["ok"], false, "{moved}");
        let newer = parts[1]["musicxml"]
            .as_str()
            .unwrap()
            .replace(&version, &format!(";generator={}", GENERATOR + 1));
        let newer = json!([{"part": "kyrie/2", "musicxml": newer}]);
        for (lang, error) in [
            (
                "de",
                format!(
                    "Teil kyrie/2: Die Datei stammt von einer anderen Version des Generators \
                     (Version {}, diese Seite {GENERATOR}). Bitte die Seite neu laden und die \
                     Messe noch einmal setzen.",
                    GENERATOR + 1
                ),
            ),
            (
                "en",
                format!(
                    "Part kyrie/2: the file comes from another version of the generator (version \
                     {}, this page {GENERATOR}). Reload the page and compose the mass again.",
                    GENERATOR + 1
                ),
            ),
        ] {
            let refused = ask(
                json!({"command": "mass_join", "mass": kyrie(1), "parts": newer,
                "lang": lang}),
            );
            assert_eq!(refused["error"], error, "{refused}");
        }
        // The page checks the file of the movement part by part, as `fux check` does: every part
        // with the cost of its search, the violations in the measures of the movement; and the
        // file of a part as it was set.
        let checked = ask(json!({"command": "check", "musicxml": xml}));
        assert_eq!(checked["ok"], true, "{checked}");
        let lines = strings(&checked["summary"]);
        assert!(
            lines[0].starts_with("Teil kyrie/1: Kyrie I, Takte 1–"),
            "{lines:?}"
        );
        assert_eq!(
            lines.last().unwrap(),
            &format!(
                "Gesamt: 0 Verbot(e), Kosten {cost} aus {violations} Präferenz(en) in 3 Teilen."
            )
        );
        let found = &checked["results"][0];
        assert_eq!(found["cost"], cost);
        assert_eq!(found["violations"], kyrie_file["violations"]);
        for part in &parts {
            let xml = part["musicxml"].as_str().unwrap();
            let alone = ask(json!({"command": "check", "musicxml": xml, "lang": "en"}));
            assert_eq!(alone["results"][0]["cost"], part["cost"], "{alone}");
            assert_eq!(
                alone["results"][0]["violations"].as_array().unwrap().len(),
                { part["violations"].as_array().unwrap().len() }
            );
        }
    }

    /// A section shows the voices that sing in it: a voice that rests through it goes, and the
    /// voices of its text and of the entries of its imitation follow; its measures are numbered
    /// on from where it begins.
    #[test]
    fn a_section_shows_the_voices_that_sing() {
        use fux_core::{
            Duration, Role, ScoreEntry, ScoreImitation, ScorePoint, ScoreText, Species, Voice,
            VoiceType, parse_lilypond,
        };
        let voice = |kind, notes: &str| {
            let notes = parse_lilypond(notes, Duration::HALF).unwrap();
            Voice::new(kind, Role::Counterpoint(Species::Free), notes)
        };
        let mut score = Score::new(
            "dorian".parse().unwrap(),
            vec![
                voice(VoiceType::Soprano, "a'1 a'1"),
                voice(VoiceType::Alto, "r1 r1"),
                voice(VoiceType::Bass, "d1 d1"),
            ],
        );
        let (text, _) = fux_core::text::resolve("kyrie", &Default::default()).unwrap();
        score.text = Some(ScoreText {
            text: Arc::new(text),
            voices: vec![true, false, true],
        });
        let entry = |voice| ScoreEntry {
            voice,
            start: 0,
            rest: 0,
            notes: Vec::new(),
        };
        score.imitation = Some(Arc::new(ScoreImitation {
            points: vec![ScorePoint {
                words: 0..7,
                entries: vec![entry(2), entry(0)],
                goal: None,
            }],
        }));
        let shown = singing(score.clone());
        let kinds: Vec<VoiceType> = shown.voices.iter().map(|v| v.voice_type).collect();
        assert_eq!(kinds, [VoiceType::Soprano, VoiceType::Bass]);
        assert_eq!(shown.text.as_ref().unwrap().voices, [true, true]);
        let voices: Vec<usize> = shown.imitation.as_ref().unwrap().points[0]
            .entries
            .iter()
            .map(|entry| entry.voice)
            .collect();
        assert_eq!(voices, [1, 0]);
        // Every voice sings: the score stays as it is.
        let all = Score::new(score.mode, vec![voice(VoiceType::Soprano, "a'1 a'1")]);
        assert_eq!(singing(all.clone()), all);
        assert_eq!(
            numbered_from("<measure number=\"1\">x<measure number=\"2\">", 31),
            "<measure number=\"31\">x<measure number=\"32\">"
        );
        assert_eq!(
            numbered_from("<measure number=\"1\">", 1),
            "<measure number=\"1\">"
        );
    }

    /// The file of a part is the one `fux mass` writes to `NAME.teile/`, byte for byte.
    #[test]
    fn the_file_of_a_part_is_the_one_of_fux_mass() {
        let request: Request = serde_json::from_value(json!({"mass": kyrie(1)})).unwrap();
        let mass = prepared(&request, &Profile::fux(), Lang::De).unwrap();
        let at = one_part(&mass.plan, "christe", Lang::De).unwrap();
        let result =
            generate_part(&mass.request, &mass.plan, at, &mass.profile, None, None).unwrap();
        let dir = std::env::temp_dir().join(format!("fux-web-part-{}", std::process::id()));
        let base = dir.join("messe");
        let path = fux_cli::mass::write_part(
            &mass.plan,
            &result,
            &base,
            &MusicXmlOptions::default(),
            Lang::De,
        )
        .unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(part_file(&mass.plan, &result, Lang::De), written);
        assert!(
            path.ends_with("messe.teile/1-02-christe.musicxml"),
            "{path:?}"
        );
    }

    /// A small mass with the head motif: a Kyrie and an Agnus Dei in two voices, every part in
    /// imitation, whose Kyrie I gives a motif of seven tones; each part takes a second or less.
    fn motif_mass() -> Value {
        json!({"movements": "kyrie,agnus", "voices": 2, "beam": 40, "lengths": "natürlich",
            "imitation": "an", "motif": true, "seed": 6})
    }

    /// The head motif (MASS-MOTIF) from part to part, as the page's pool passes it: Kyrie I
    /// answers with the motif it gives, and Agnus Dei I opens with it; both files are those `fux
    /// mass` writes, byte for byte (two parts at once, Agnus Dei I waiting for Kyrie I and its
    /// motif). The files of the movements keep the motif. Without it Agnus Dei I takes its
    /// subject from its words, with a warning, and the join of a mass whose Kyrie I gives a motif
    /// refuses its file; without Kyrie I the join takes it.
    #[test]
    fn the_head_motif_goes_from_kyrie_i_to_the_parts_that_open_with_it() {
        use fux_cli::mass::{run_mass, write_part};
        use fux_core::generate::Warning;
        use std::ops::ControlFlow;
        let set = |part: &str, motif: Value| {
            let mut request = json!({"command": "mass_part", "mass": motif_mass(), "part": part});
            request["motif"] = motif;
            let answer = ask(request);
            assert_eq!(answer["ok"], true, "{answer}");
            answer["part"].clone()
        };
        let kyrie = set("kyrie/1", Value::Null);
        let motif = kyrie["motif"]
            .as_str()
            .expect("Kyrie I gives a motif")
            .to_string();
        assert_eq!(motif, "0,1,2,1,0,-2,-3");
        let agnus = set("agnus/1", json!(motif));
        assert_eq!(agnus["warnings"], json!([]), "{agnus}");
        assert!(agnus.get("motif").is_none(), "{agnus}");
        let item = format!(";motif={motif};");
        for part in [&kyrie, &agnus] {
            assert!(part["musicxml"].as_str().unwrap().contains(&item));
        }
        // The files of `fux mass`.
        let request: Request = serde_json::from_value(json!({"mass": motif_mass()})).unwrap();
        let mass = prepared(&request, &Profile::fux(), Lang::De).unwrap();
        let plan = Arc::new(mass.plan);
        let places =
            ["kyrie/1", "agnus/1"].map(|address| one_part(&plan, address, Lang::De).unwrap());
        let mut on = |_| ControlFlow::Continue(());
        let m = Mass::new(Arc::clone(&plan));
        let m = run_mass(&mass.request, m, &places, &mass.profile, None, 2, &mut on);
        let dir = std::env::temp_dir().join(format!("fux-web-motif-{}", std::process::id()));
        let base = dir.join("messe");
        for (at, part) in places.into_iter().zip([&kyrie, &agnus]) {
            let result = m.result(at).unwrap();
            let options = MusicXmlOptions::default();
            let path = write_part(&plan, result, &base, &options, Lang::De).unwrap();
            let written = std::fs::read_to_string(&path).unwrap();
            assert_eq!(written, part["musicxml"].as_str().unwrap(), "{path:?}");
        }
        std::fs::remove_dir_all(&dir).ok();
        // Without the motif: from the words, with a warning.
        let without = set("agnus/1", Value::Null);
        let none = Warning::NoMotif {
            movement: "Agnus Dei".into(),
        };
        assert_eq!(without["warnings"], json!([none.message(Lang::De)]));
        assert!(!without["musicxml"].as_str().unwrap().contains(";motif=0"));
        let wrong = ask(
            json!({"command": "mass_part", "mass": motif_mass(), "part": "agnus/1",
            "motif": "0,1", "lang": "en"}),
        );
        assert_eq!(
            wrong["error"],
            "\"0,1\" is no head motif: five to ten degrees above the final, separated by commas, \
             an altered one followed by b or #."
        );
        // The mass of its parts: the file of each movement keeps the motif of its first part.
        let file = |part: &Value| json!({"part": part["address"], "musicxml": part["musicxml"]});
        let rest =
            ["kyrie/2", "kyrie/3", "agnus/2"].map(|address| file(&set(address, Value::Null)));
        let parts = |agnus: &Value| json!([file(agnus), rest[2], rest[0], file(&kyrie), rest[1]]);
        let joined =
            ask(json!({"command": "mass_join", "mass": motif_mass(), "parts": parts(&agnus)}));
        assert_eq!(joined["ok"], true, "{joined}");
        let movements = joined["results"].as_array().unwrap();
        assert_eq!(movements.len(), 2);
        for movement in movements {
            let xml = movement["musicxml"].as_str().unwrap();
            let read = fux_export::from_musicxml(xml).unwrap();
            let first = read.field("fux-part.1").unwrap();
            assert!(first.contains(&item), "{first}");
            assert!(!read.field("fux-part.2").unwrap().contains(";motif="));
        }
        assert!(
            joined["summary"]
                .as_array()
                .unwrap()
                .iter()
                .any(|line| line.as_str().unwrap().ends_with(", keine Warnung")),
            "{joined}"
        );
        // A part that took no motif where Kyrie I gives one does not belong to the mass.
        let refused = ask(json!({"command": "mass_join", "mass": motif_mass(),
            "parts": parts(&without)}));
        assert_eq!(
            refused["error"],
            "Teil agnus/1: Die Datei wurde mit einem anderen Kopfmotiv gesetzt, als das Kyrie I \
             gibt."
        );
        let alone = ask(json!({"command": "mass_join", "mass": motif_mass(),
            "parts": [file(&without), rest[2]]}));
        assert_eq!(alone["ok"], true, "{alone}");
        assert_eq!(alone["results"].as_array().unwrap().len(), 1);
    }

    /// The address of a part: one part, in a movement the mass sets.
    #[test]
    fn a_part_by_its_address() {
        let part = |address: &str| {
            ask(json!({"command": "mass_part", "mass": kyrie(1), "part": address}))["error"].clone()
        };
        assert_eq!(part("credo/3"), "„credo/3“: Das Credo wird nicht gesetzt.");
        assert_eq!(part("kyrie/4"), "„kyrie/4“: Das Kyrie hat 3 Teile.");
        assert_eq!(
            part(""),
            "„“ nennt nicht einen Teil der Messe; ein Teil heißt wie credo/3."
        );
        let request: Request =
            serde_json::from_value(json!({"mass": {"seed": 1, "movements": "credo"}})).unwrap();
        let plan = prepared(&request, &Profile::fux(), Lang::En).unwrap().plan;
        assert!(one_part(&plan, "credo", Lang::En).is_err());
        assert_eq!(
            plan.address(one_part(&plan, "incarnatus", Lang::En).unwrap()),
            "credo/5"
        );
    }

    /// The voices of a free setting are still given as text in `parts`.
    #[test]
    fn parts_as_voices_and_as_parts_of_a_mass() {
        let free = ask(
            json!({"command": "free", "parts": "S T", "length": 8, "text": "none",
            "seed": 1, "count": 1}),
        );
        assert_eq!(free["ok"], true, "{free}");
        assert!(
            free["results"][0]["lines"][1]
                .as_str()
                .unwrap()
                .starts_with("Tenor: ")
        );
        let request: Request =
            serde_json::from_value(json!({"parts": [{"part": "kyrie/1"}]})).unwrap();
        assert_eq!(request.parts.voices(), "");
        assert_eq!(request.parts.set().len(), 1);
    }
}
