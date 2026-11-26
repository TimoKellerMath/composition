//! `fux free`: free counterpoint without a cantus firmus, in a mode, for two to six voices, with
//! a plan of internal cadences. From phase 12 on every voice sings a text, by default Kyrie I;
//! the text plans the length and the cadences unless they are given. From phase 13 on the voices
//! imitate each other where they sing a text, with three to five voices: the points of imitation
//! are planned before the search ([`crate::imitation`]). From phase 14 on a setting may end on
//! the fifth or the fourth instead of the final (`--ending`), as the parts of a mass do.

use std::io::Write;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use fux_core::generate::{
    FreeRequest, FreeText, GenerateError, Generated, PartContext, cadences_at, choir_with,
    default_cadences, generate_free, part_endings, regular_cadences,
};
use fux_core::imitation::Imitation;
use fux_core::mass::Repeat as MassRepeat;
use fux_core::rules::{Checker, Level, Profile};
use fux_core::text::plan::{automatic_length, plan_part};
use fux_core::text::{Eleison, Text};
use fux_core::{Cadence, Lang, Mode, ModeSpec, Pitch, Score, Step, VoiceType};
use fux_export::Sheet;

use crate::commands::{localize_names, print_files, print_violations, time_seed, voice_text};
use crate::imitation::{
    Given, ImitationMode, ImitationOptions, Setting, chant_mode, plan_summary, point_lines,
    remedies,
};
use crate::output::{self, OutputArgs};
use crate::text::{DEFAULT_TEXT, restates, sung_word, text_label, text_of};

/// The length of a setting without a text unless one is given, as before phase 12.
pub const UNTEXTED_LENGTH: usize = 12;

/// Where internal cadences arrive.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CadencePlan {
    /// About one per six measures, on the regular degrees of the mode; with a text where its
    /// planned statements end sense units ([`fux_core::text::plan`]).
    #[default]
    Auto,
    None,
    /// Measures from 1, each with its target or else the next regular degree of the mode.
    Measures(Vec<(usize, Option<Step>)>),
}

impl FromStr for CadencePlan {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let error = || format!("{text:?}: auto, none or measures such as 7,12 or 7:A,12:F");
        Ok(match text.trim() {
            "auto" => CadencePlan::Auto,
            "none" | "keine" => CadencePlan::None,
            list => CadencePlan::Measures(
                list.split(',')
                    .map(|item| {
                        let (measure, target) = match item.split_once([':', '=']) {
                            Some((m, t)) => (m, Some(t)),
                            None => (item, None),
                        };
                        let measure = measure.trim().parse::<usize>().ok().filter(|&m| m >= 1)?;
                        let target = match target.map(str::trim) {
                            Some(t) => {
                                let mut chars = t.chars();
                                let step = Step::from_letter(chars.next()?)?;
                                // A flat after the letter is the mode's own (B♭).
                                matches!(chars.as_str(), "" | "b" | "♭").then_some(step)
                            }
                            None => None,
                        };
                        Some((measure, target))
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(error)?,
            ),
        })
    }
}

impl CadencePlan {
    /// The internal cadences of a setting in `mode` of `measures` measures.
    pub fn cadences(&self, mode: ModeSpec, measures: usize) -> Vec<Cadence> {
        match self {
            CadencePlan::Auto => default_cadences(mode, measures),
            CadencePlan::None => Vec::new(),
            CadencePlan::Measures(list) => {
                let goals: Vec<usize> = list.iter().map(|(m, _)| m - 1).collect();
                let mut cadences = cadences_at(mode, &goals);
                for (cadence, (_, target)) in cadences.iter_mut().zip(list) {
                    if let Some(target) = target {
                        cadence.target = *target;
                    }
                }
                cadences
            }
        }
    }

    /// The cadences given explicitly; `None` for `auto`, which leaves them to the plan of a text.
    fn given(&self, mode: ModeSpec) -> Option<Vec<Cadence>> {
        match self {
            CadencePlan::Auto => None,
            // Neither `none` nor a list of measures depends on the length.
            given => Some(given.cadences(mode, 0)),
        }
    }
}

/// A length in measures, or `auto`: as long as the text needs, 12 measures without a text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Length {
    #[default]
    Auto,
    Measures(usize),
}

impl FromStr for Length {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.trim() {
            "auto" => Ok(Length::Auto),
            number => number
                .parse()
                .map(Length::Measures)
                .map_err(|_| format!("{text:?}: auto or a number of measures such as 16")),
        }
    }
}

impl Length {
    /// The number of measures; `None` for `auto`.
    pub fn measures(self) -> Option<usize> {
        match self {
            Length::Auto => None,
            Length::Measures(measures) => Some(measures),
        }
    }
}

/// How freely the voices restate the text (`--repeat`): TXT-REPEAT as the profile has it, or
/// overridden. The choice is the mass's ([`fux_core::mass::Repeat`]), which knows its overrides,
/// its names and how it is written; this type stands for it in the frontends, which apply it to
/// their profiles ([`Repeat::apply`]), and converts into it and back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Repeat {
    /// As the profile has it: free in short texts (Kyrie, Sanctus, Agnus), rare in long ones
    /// (Gloria, Credo).
    #[default]
    Auto,
    /// Never: TXT-REPEAT `short` and `long` forbidden.
    None,
    /// Freely: TXT-REPEAT `short`, `long` and `again` off.
    Free,
}

impl From<Repeat> for MassRepeat {
    fn from(repeat: Repeat) -> MassRepeat {
        match repeat {
            Repeat::Auto => MassRepeat::Auto,
            Repeat::None => MassRepeat::None,
            Repeat::Free => MassRepeat::Free,
        }
    }
}

impl From<MassRepeat> for Repeat {
    fn from(repeat: MassRepeat) -> Repeat {
        match repeat {
            MassRepeat::Auto => Repeat::Auto,
            MassRepeat::None => Repeat::None,
            MassRepeat::Free => Repeat::Free,
        }
    }
}

impl FromStr for Repeat {
    type Err = String;

    /// `auto`, `keine` (`none`) or `frei` (`free`), as the mass reads them.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse::<MassRepeat>().map(Repeat::from)
    }
}

impl Repeat {
    /// The overrides of the profile that this choice makes.
    pub fn overrides(self) -> &'static [&'static str] {
        MassRepeat::from(self).overrides()
    }

    /// The profile with TXT-REPEAT as this choice sets it ([`MassRepeat::apply`]); it never fails,
    /// for the overrides name parameters of the catalogue.
    pub fn apply(self, profile: &Profile) -> Result<Profile> {
        Ok(MassRepeat::from(self).apply(profile))
    }

    /// The name for users: „auto“, „keine“, „frei“.
    pub fn name(self, lang: Lang) -> &'static str {
        MassRepeat::from(self).name(lang)
    }
}

/// The degree a setting ends on as `--ending` takes it: its letter, H or B for B, a flat after
/// the letter being the mode's own (`Bb` for B♭ in a transposed mode). Which degrees a mode
/// allows the search says ([`GenerateError::Ending`]).
pub fn degree(text: &str) -> Result<Step, String> {
    let mut chars = text.trim().chars();
    chars
        .next()
        .and_then(|letter| match letter {
            'h' | 'H' => Some(Step::B),
            letter => Step::from_letter(letter),
        })
        .filter(|_| matches!(chars.as_str(), "" | "b" | "♭"))
        .ok_or_else(|| format!("{text:?}: a degree from A to G (H for B), such as D, A or G"))
}

/// `an` or `aus` (`on`, `off`), as `--accents` takes it.
pub fn on_off(text: &str) -> Result<bool, String> {
    match text.trim().to_lowercase().as_str() {
        "an" | "on" => Ok(true),
        "aus" | "off" => Ok(false),
        _ => Err(format!("{text:?}: an (on) or aus (off)")),
    }
}

/// The division of eleison as `--eleison` takes it: `4` (e-lé-i-son, as in the books) or `3`
/// (e-léi-son).
pub fn eleison(text: &str) -> Result<Eleison, String> {
    match text.trim() {
        "4" => Ok(Eleison::Four),
        "3" => Ok(Eleison::Three),
        _ => Err(format!("{text:?}: 4 or 3 syllables")),
    }
}

/// How the voices sing their text, as `fux free`, `fux cp --species 0` and `fux mass` take it:
/// how freely they restate it, whether the printed syllables keep their accents, and how eleison
/// is divided.
#[derive(clap::Args, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SingingArgs {
    /// Textwiederholung: auto (Kyrie, Sanctus, Agnus frei; Gloria, Credo selten), keine, frei
    #[arg(long, value_name = "WIE", default_value = "auto")]
    pub repeat: Repeat,
    /// Akzente der liturgischen Bücher zeigen
    #[arg(
        long,
        value_name = "AN|AUS",
        default_value = "an",
        action = clap::ArgAction::Set,
        value_parser = on_off
    )]
    pub accents: bool,
    /// 4 Silben (Bücher) oder 3 (e-léi-son, Palestrina nach Jeppesen)
    #[arg(long, value_name = "4|3", default_value = "4", value_parser = eleison)]
    pub eleison: Eleison,
}

impl Default for SingingArgs {
    /// As the options leave them: restatements as the profile has them, the accents of the
    /// books, eleison in four syllables.
    fn default() -> Self {
        SingingArgs {
            repeat: Repeat::Auto,
            accents: true,
            eleison: Eleison::Four,
        }
    }
}

/// The line on the text of a setting: „Text: Kyrie I (7 Silben, 1 Sinnabschnitt), Wiederholung
/// auto“.
pub fn text_line(text: &Text, repeat: Repeat, lang: Lang) -> String {
    format!(
        "Text: {}, {} {}",
        text_label(text, lang),
        lang.pick("Wiederholung", "repetition"),
        repeat.name(lang)
    )
}

/// The usual choir for free counterpoint: soprano and tenor, three voices A T B, four S A T B,
/// five S A T T B, six S S A T T B.
pub fn free_choir(voices: usize) -> Vec<VoiceType> {
    match voices {
        0..=2 => vec![VoiceType::Soprano, VoiceType::Tenor],
        n => choir_with(n, VoiceType::Alto, None),
    }
}

/// Voice types from text such as "S A T B" or "S,A,T,B", from high to low.
pub fn parse_parts(text: &str, lang: Lang) -> Result<Vec<VoiceType>> {
    let mut parts = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .map(|t| {
            t.parse::<VoiceType>().map_err(|_| {
                anyhow!(
                    "{}: {t}",
                    lang.pick("Unbekannte Stimmlage", "Unknown voice type")
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    parts.sort();
    if !(2..=6).contains(&parts.len()) {
        return Err(anyhow!(
            "{}",
            lang.pick(
                "Die Besetzung braucht zwei bis sechs Stimmen.",
                "The parts need two to six voices."
            )
        ));
    }
    Ok(parts)
}

/// Options of `fux free`.
#[derive(Clone, Debug)]
pub struct FreeOptions {
    /// The mode; `None` for the Dorian, or the mode of the chant whose phrases give the subjects
    /// of an imitation.
    pub mode: Option<ModeSpec>,
    pub voices: usize,
    /// Voice types from top to bottom, instead of the usual choir.
    pub parts: Option<String>,
    /// The number of measures; `None` for as many as the text plans, [`UNTEXTED_LENGTH`] without
    /// a text.
    pub length: Option<usize>,
    pub cadences: CadencePlan,
    /// The degree of the final cadence; `None` for the final. Besides the final, the fifth and
    /// the fourth where the mode has a clausula on them ([`part_endings`]), as a part of a mass
    /// may end.
    pub ending: Option<Step>,
    /// What every voice sings: a section of the Ordinary (`kyrie`), some of its sense units
    /// (`gloria:1-5`), an own Latin text, or `none`.
    pub text: String,
    pub repeat: Repeat,
    /// The printed syllables keep the acute of the liturgical books.
    pub accents: bool,
    pub eleison: Eleison,
    pub seed: Option<u64>,
    pub count: usize,
    pub beam: usize,
    pub noise: f64,
    pub details: bool,
    /// Whether the voices imitate each other, and how.
    pub imitation: ImitationOptions,
}

impl Default for FreeOptions {
    /// As `fux free` without options: four voices in the Dorian mode sing Kyrie I, in imitation.
    fn default() -> Self {
        FreeOptions {
            mode: None,
            voices: 4,
            parts: None,
            length: None,
            cadences: CadencePlan::Auto,
            ending: None,
            text: DEFAULT_TEXT.to_string(),
            repeat: Repeat::Auto,
            accents: true,
            eleison: Eleison::Four,
            seed: None,
            count: 1,
            beam: 200,
            noise: 2.0,
            details: false,
            imitation: ImitationOptions::default(),
        }
    }
}

/// What `fux free` hands to the search: the request with its seed, the profile with the
/// overrides of `--repeat`, and warnings about the text and the imitation.
#[derive(Clone, Debug)]
pub struct PreparedFree {
    /// With a text, its length and cadences are those the text plans, as far as the plan
    /// succeeds; the search plans them again (and once more with more measures if it fails).
    pub request: FreeRequest,
    /// The profile for the search and the report: the one given, with TXT-REPEAT as `--repeat`
    /// sets it.
    pub profile: Profile,
    pub seed: u64,
    pub repeat: Repeat,
    /// Warnings about the text and the imitation in the user's language, such as uncertain
    /// accents.
    pub warnings: Vec<String>,
}

/// Builds the search request of `fux free`: the voices and, unless the text is `none`, the text
/// with its options, its warnings and the overrides of `--repeat`, and the imitation where it
/// applies ([`ImitationOptions::applies`]). Without a text the request is the one of phase 11:
/// [`UNTEXTED_LENGTH`] measures unless given, and the cadences of `options`. The mode is the
/// one given, else the mode of the chant whose phrases give the subjects, else the Dorian. An
/// ending other than the final goes to the request's part; one that the mode does not allow
/// fails here, before the plan.
pub fn prepare_free(options: &FreeOptions, profile: &Profile, lang: Lang) -> Result<PreparedFree> {
    let seed = options.seed.unwrap_or_else(time_seed);
    let voices = match &options.parts {
        Some(text) => parse_parts(text, lang)?,
        None => free_choir(options.voices),
    };
    let sung = text_of(&options.text, options.eleison, lang)?;
    let imitation = &options.imitation;
    let auto_cadences = options.cadences == CadencePlan::Auto;
    if imitation.mode == ImitationMode::On {
        let refused = match (&sung, auto_cadences) {
            (None, _) => Some(GenerateError::ImitationNeedsText),
            (Some(_), false) => Some(GenerateError::ImitationCadences),
            _ => None,
        };
        if let Some(error) = refused {
            let remedy = remedies(&error, Given::default(), lang).unwrap_or_default();
            bail!("{} {remedy}", error.message(lang));
        }
    }
    let auto_length = options.length.is_none();
    let imitating =
        sung.is_some() && imitation.applies(true, voices.len(), auto_cadences, auto_length);
    let mut notes = Vec::new();
    if !imitating && imitation.mode == ImitationMode::Auto && imitation.shapes() {
        notes.push(
            lang.pick(
                "Imitation aus (auto: mit Text, drei bis fünf Stimmen, automatischen Binnenschlüssen und automatischer Länge); die Angaben zu ihr gelten nicht. --imitation an schaltet sie ein.",
                "Imitation off (auto: with a text, three to five voices, automatic internal cadences and an automatic length); its options do not apply. --imitation on turns it on.",
            )
            .to_string(),
        );
    } else if !imitating
        && imitation.mode == ImitationMode::Auto
        && imitation.applies(sung.is_some(), voices.len(), auto_cadences, true)
    {
        // Only the length given keeps the setting from imitating.
        notes.push(
            lang.pick(
                "Imitation aus, denn die Länge ist gegeben (auto imitiert mit automatischer Länge); --imitation an imitiert in der gegebenen Länge.",
                "Imitation off, as the length is given (auto imitates with an automatic length); --imitation on imitates in the given length.",
            )
            .to_string(),
        );
    }
    let chant = match &sung {
        Some((text, _)) if imitating => imitation.chant(text, lang)?,
        _ => None,
    };
    let mode = match (options.mode, chant.as_deref().and_then(chant_mode)) {
        (Some(mode), Some(own)) => {
            if own.final_step() != mode.final_step() {
                let (theirs, ours) = (own.final_step().name(lang), mode.final_step().name(lang));
                notes.push(match lang {
                    Lang::De => format!(
                        "Der Choral hat die Finalis {theirs}, der Satz {ours}; seine Stufen werden von {ours} aus gesungen."
                    ),
                    Lang::En => format!(
                        "The chant's final is {theirs}, the setting's {ours}; its degrees are sung from {ours}."
                    ),
                });
            }
            mode
        }
        (Some(mode), None) => mode,
        (None, Some(own)) => own,
        (None, None) => ModeSpec::new(Mode::Dorian),
    };
    if let Some(target) = options.ending {
        if !part_endings(mode).contains(&target) {
            bail!("{}", GenerateError::Ending { mode, target }.message(lang));
        }
        // Two voices end on the octave or unison of the final (CAD-END), which the search would
        // find out only at the last measure.
        let last = profile.settings("CAD-END").level("level");
        if voices.len() == 2 && target != mode.final_step() && last == Level::Forbidden {
            let name = |step: Step| Pitch::new(step, mode.scale_alter(step), 4).class_name(lang);
            let (final_name, target_name) = (name(mode.final_step()), name(target));
            bail!(
                "{}",
                match lang {
                    Lang::De => format!(
                        "Zweistimmig schließt ein Satz auf der Finalis {final_name} (CAD-END), nicht auf {target_name}. Abhilfe: drei Stimmen oder mehr (--voices) oder --ending {final_name}."
                    ),
                    Lang::En => format!(
                        "In two voices a setting ends on the final {final_name} (CAD-END), not on {target_name}. Remedies: three voices or more (--voices) or --ending {final_name}."
                    ),
                }
            );
        }
    }
    // Without an ending, the part of a setting of its own: the plan and the search as before.
    let part = PartContext {
        ending: options.ending,
        ..PartContext::default()
    };
    let Some((text, mut warnings)) = sung else {
        let measures = options.length.unwrap_or(UNTEXTED_LENGTH);
        let request = FreeRequest {
            cadences: options.cadences.cadences(mode, measures),
            seed,
            count: options.count,
            beam: options.beam,
            noise: options.noise,
            part,
            ..FreeRequest::new(mode, voices, measures)
        };
        return Ok(PreparedFree {
            request,
            profile: profile.clone(),
            seed,
            repeat: options.repeat,
            warnings: notes,
        });
    };
    warnings.extend(notes);
    let profile = options.repeat.apply(profile)?;
    // The plan as the search makes it. If it fails, the search fails the same way and says why.
    // An imitation plans its length and its cadences itself.
    let given = options.cadences.given(mode);
    let restatements = restates(&profile, &text);
    let planned = plan_part(
        &text,
        mode,
        voices.len(),
        options.length,
        given.clone(),
        restatements,
        &part,
    )
    .ok();
    let (measures, cadences) = match planned {
        Some(planned) => (planned.measures, planned.cadences),
        None => (
            options
                .length
                .unwrap_or_else(|| automatic_length(&text, restatements)),
            given.clone().unwrap_or_default(),
        ),
    };
    let imitation: Option<Imitation> = if imitating {
        let mut choir = voices.clone();
        choir.sort();
        Some(imitation.imitation(&text, chant, &choir, lang)?)
    } else {
        None
    };
    let request = FreeRequest {
        cadences,
        seed,
        count: options.count,
        beam: options.beam,
        noise: options.noise,
        text: Some(FreeText {
            text: Arc::new(text),
            auto_length: options.length.is_none(),
            auto_cadences: given.is_none(),
            accents: options.accents,
        }),
        imitation,
        part,
        ..FreeRequest::new(mode, voices, measures)
    };
    Ok(PreparedFree {
        request,
        profile,
        seed,
        repeat: options.repeat,
        warnings,
    })
}

/// Lines describing a free setting: mode, voices, length, an ending given (`Schluss auf A`) and
/// seed, the text, and the internal cadences. With a text, length and cadences are read from the
/// first result, since the text plans them; at each cadence the line shows what the top voice
/// sings of its word as it arrives (`Takt 7 auf A: …e-lé-i-son.`), so a unit that ends there ends
/// with its punctuation. An imitative setting has a line on each of its points
/// ([`point_lines`]), the last ending on the planned ending.
pub fn free_summary(prepared: &PreparedFree, results: &[Generated], lang: Lang) -> Vec<String> {
    let request = &prepared.request;
    // Without a text the request has them, in the order given.
    let first = results
        .first()
        .map(|r| &r.score)
        .filter(|_| request.text.is_some());
    let measures = first.map_or(request.measures, Score::measure_count);
    let cadences = first.map_or(&request.cadences, |score| &score.cadences);
    let ending = match request.part.ending {
        Some(ending) => {
            let target = Pitch::new(ending, request.mode.scale_alter(ending), 4);
            format!(
                ", {} {}",
                lang.pick("Schluss auf", "ending on"),
                target.class_name(lang)
            )
        }
        None => String::new(),
    };
    let mut summary = vec![format!(
        "{} – {}, {} {}, {} {}{ending}, seed {}",
        request.mode.name(lang),
        lang.pick("freier Satz", "free counterpoint"),
        request.voices.len(),
        lang.pick("Stimmen", "voices"),
        measures,
        lang.pick("Takte", "measures"),
        prepared.seed,
    )];
    if let Some(sung) = &request.text {
        summary.push(text_line(&sung.text, prepared.repeat, lang));
    }
    if !cadences.is_empty() {
        let list: Vec<String> = cadences
            .iter()
            .map(|c| {
                let target = Pitch::new(c.target, request.mode.scale_alter(c.target), 4);
                let mut item = format!(
                    "{} {} {} {}",
                    lang.pick("Takt", "measure"),
                    c.measure + 1,
                    lang.pick("auf", "on"),
                    target.class_name(lang)
                );
                if let (Some(sung), Some(score)) = (&request.text, first) {
                    let goal = c.measure as u32 * score.meter.ticks_per_measure();
                    if let Some(word) = sung_word(score, 0, goal, sung.accents) {
                        item.push_str(&format!(": …{word}"));
                    }
                }
                item
            })
            .collect();
        // The words end with their punctuation.
        let separator = if request.text.is_some() { "; " } else { ", " };
        summary.push(format!(
            "{}: {}",
            lang.pick("Binnenschlüsse", "Internal cadences"),
            list.join(separator)
        ));
    }
    if let Some(score) = first
        && let Some(plan) = &score.imitation
    {
        let setting = Setting {
            accents: request.text.as_ref().is_none_or(|sung| sung.accents),
            ending: request.part.ending_in(request.mode),
            ..Setting::of(score, lang)
        };
        summary.extend(point_lines(plan, &setting, lang));
    }
    summary
}

/// The warnings that the results of a search carry, each once: what the generator did otherwise
/// than the request asked, such as a point of an imitation on chant whose phrase gave no usable
/// head and took its subject from its words.
pub fn result_warnings(results: &[Generated], lang: Lang) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for warning in results.iter().flat_map(|r| &r.warnings) {
        let line = warning.message(lang);
        if !lines.contains(&line) {
            lines.push(line);
        }
    }
    lines
}

/// The message of a failed search of `request`; for a texted or imitative setting with what may
/// help.
pub fn free_error(error: &GenerateError, request: &FreeRequest, lang: Lang) -> String {
    let message = error.message(lang);
    let texted = request.text.is_some();
    if request.imitation.is_some()
        && let Some(remedy) = remedies(error, Given::of(request), lang)
    {
        return format!("{message} {remedy}");
    }
    let hint = match (error, texted) {
        (GenerateError::TextTooLong { measures }, _) => match lang {
            Lang::De => format!(
                "Abhilfe: --length {measures} oder auto, ein kürzerer Text (etwa ein Bereich wie gloria:1-5) oder weniger Binnenschlüsse (--cadences)."
            ),
            Lang::En => format!(
                "Remedies: --length {measures} or auto, a shorter text (such as a range like gloria:1-5) or fewer internal cadences (--cadences)."
            ),
        },
        (GenerateError::NoCounterpoint { .. }, true) => lang
            .pick(
                "Abhilfe: mehr Takte (--length), --repeat frei, ein kürzerer Text oder andere Binnenschlüsse (--cadences).",
                "Remedies: more measures (--length), --repeat free, a shorter text or other internal cadences (--cadences).",
            )
            .to_string(),
        _ => return message,
    };
    format!("{message} {hint}")
}

/// The regular cadence degrees of a mode, as a line for the help of the windows.
pub fn regular_degrees(mode: ModeSpec, lang: Lang) -> String {
    regular_cadences(mode)
        .iter()
        .map(|&s| Pitch::new(s, mode.scale_alter(s), 4).class_name(lang))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `fux free`: generates free settings, prints them and writes the files.
pub fn free(
    out: &mut impl Write,
    options: &FreeOptions,
    profile: &Profile,
    output: &OutputArgs,
    lang: Lang,
) -> Result<()> {
    let prepared = prepare_free(options, profile, lang)?;
    let warning = lang.pick("Warnung", "Warning");
    for line in &prepared.warnings {
        writeln!(out, "{warning}: {line}")?;
    }
    let request = &prepared.request;
    if options.imitation.plan_only {
        // The plan, without a search and without files.
        let lines = plan_summary(&prepared, lang).ok_or_else(|| {
            anyhow!(
                "{}",
                lang.pick(
                    "--plan zeigt den Plan einer Imitation, aber dieser Satz imitiert nicht (auto: mit Text, drei bis fünf Stimmen, automatischen Binnenschlüssen und automatischer Länge). Abhilfe: --imitation an.",
                    "--plan shows the plan of an imitation, but this setting does not imitate (auto: with a text, three to five voices, automatic internal cadences and an automatic length). Remedy: --imitation on.",
                )
            )
        })?;
        for line in lines.map_err(|e| anyhow!("{}", free_error(&e, request, lang)))? {
            writeln!(out, "{line}")?;
        }
        return Ok(());
    }
    let results = generate_free(request, &prepared.profile)
        .map_err(|e| anyhow!("{}", free_error(&e, request, lang)))?;
    for line in result_warnings(&results, lang) {
        writeln!(out, "{warning}: {line}")?;
    }
    for line in free_summary(&prepared, &results, lang) {
        writeln!(out, "{line}")?;
    }
    let scores = print_results(out, results, options.details, &prepared.profile, lang)?;
    let sheets: Vec<Sheet> = scores
        .iter()
        .map(|score| Sheet {
            seed: Some(prepared.seed),
            ..Sheet::new(score)
        })
        .collect();
    let title = format!(
        "{} – {}",
        request.mode.name(lang),
        match &request.text {
            Some(sung) => sung.text.name(lang),
            None => lang.pick("freier Satz", "free counterpoint"),
        }
    );
    let files = output::write(Some(&title), &sheets, output, "out/frei", lang)?;
    print_files(out, &files, lang)
}

/// Prints each setting with its cost and voices, each voice with its text if it sings one, and
/// returns the scores, titled.
fn print_results(
    out: &mut impl Write,
    results: Vec<Generated>,
    details: bool,
    profile: &Profile,
    lang: Lang,
) -> Result<Vec<Score>> {
    let checker = Checker::new(profile);
    let mut scores = Vec::new();
    for (i, mut generated) in results.into_iter().enumerate() {
        localize_names(&mut generated.score, lang);
        let meter = generated.score.meter;
        writeln!(
            out,
            "{:>2}. {} {}",
            i + 1,
            lang.pick("Kosten", "cost"),
            generated.cost
        )?;
        for (v, voice) in generated.score.voices.iter().enumerate() {
            writeln!(out, "      {}: {}", voice.name, voice_text(voice, meter))?;
            if let Some(lyrics) = generated.score.voice_lyrics(v) {
                writeln!(out, "        Text: {lyrics}")?;
            }
        }
        if details {
            print_violations(
                out,
                &generated.score,
                &checker,
                &generated.report,
                lang,
                "      ",
            )?;
        }
        let mut score = generated.score;
        score.title = Some(format!(
            "{} {}",
            lang.pick("Freier Satz", "Free counterpoint"),
            i + 1
        ));
        scores.push(score);
    }
    Ok(scores)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imitation::{ImitationMode, ImitationOptions};

    /// Four voices on a text imitate unless asked not to; two and six voices, voices without a
    /// text and given cadences do not. The mode of a chant's subjects is the chant's.
    #[test]
    fn imitation_shapes_the_request() {
        let kyrie = prepared(FreeOptions::default());
        assert_eq!(kyrie.request.imitation, Some(Imitation::default()));
        assert_eq!(kyrie.request.mode.id(), "dorian");
        assert!(kyrie.warnings.is_empty());
        for voices in [2, 6] {
            let request = prepared(FreeOptions {
                voices,
                ..FreeOptions::default()
            })
            .request;
            assert!(request.imitation.is_none(), "{voices} voices");
        }
        let off = |options: FreeOptions| prepared(options).request.imitation.is_none();
        assert!(off(FreeOptions {
            cadences: "7".parse().unwrap(),
            ..FreeOptions::default()
        }));
        assert!(off(FreeOptions {
            imitation: ImitationOptions {
                mode: ImitationMode::Off,
                ..ImitationOptions::default()
            },
            ..FreeOptions::default()
        }));
        // On asks for a text and the cadences the text plans.
        let on = ImitationOptions {
            mode: ImitationMode::On,
            ..ImitationOptions::default()
        };
        let error = |options: FreeOptions, lang| {
            prepare_free(&options, &Profile::fux(), lang)
                .unwrap_err()
                .to_string()
        };
        assert_eq!(
            error(
                FreeOptions {
                    text: "none".into(),
                    imitation: on.clone(),
                    ..FreeOptions::default()
                },
                Lang::De
            ),
            "Imitation braucht einen Text, denn ihre Punkte folgen den Worten. Abhilfe: ein Text (--text) oder --imitation aus."
        );
        assert_eq!(
            error(
                FreeOptions {
                    cadences: "7".parse().unwrap(),
                    imitation: on.clone(),
                    ..FreeOptions::default()
                },
                Lang::En
            ),
            "Imitation places the internal cadences itself, at the ends of its points; they cannot be given. Remedies: --cadences auto or --imitation off."
        );
        let two = prepared(FreeOptions {
            voices: 2,
            imitation: on,
            ..FreeOptions::default()
        });
        assert!(two.request.imitation.is_some());
        // Options of an imitation that does not apply are said to go unused.
        let unused = prepared(FreeOptions {
            voices: 2,
            imitation: ImitationOptions {
                pairs: fux_core::imitation::Toggle::On,
                ..ImitationOptions::default()
            },
            ..FreeOptions::default()
        });
        assert!(unused.request.imitation.is_none());
        assert_eq!(unused.warnings.len(), 1);
        assert!(unused.warnings[0].starts_with("Imitation aus (auto: "));
        // A chant's subjects in the chant's mode, or in the mode given, with a warning where the
        // finals differ.
        let chant = ImitationOptions {
            subject: "chant".parse().unwrap(),
            ..ImitationOptions::default()
        };
        let sanctus = prepared(FreeOptions {
            text: "sanctus".into(),
            imitation: chant.clone(),
            ..FreeOptions::default()
        });
        let sanctus_ix = fux_core::imitation::chant::Kyriale::SanctusIX.chant();
        assert_eq!(
            Some(sanctus.request.mode),
            crate::imitation::chant_mode(&sanctus_ix)
        );
        assert!(matches!(
            sanctus.request.imitation.as_ref().unwrap().subject,
            fux_core::imitation::SubjectSource::Chant { .. }
        ));
        let phrygian = prepared(FreeOptions {
            mode: Some("phrygian".parse().unwrap()),
            imitation: chant,
            ..FreeOptions::default()
        });
        assert_eq!(phrygian.request.mode.id(), "phrygian");
        assert_eq!(
            phrygian.warnings,
            ["Der Choral hat die Finalis D, der Satz E; seine Stufen werden von E aus gesungen."]
        );
        // Imitative errors say what may help.
        let request = &kyrie.request;
        let failed = GenerateError::NoImitation {
            point: 2,
            words: "e-lé-i-son.".into(),
            measure: 14,
            rules: vec!["HAR-DISSONANCE"],
        };
        assert_eq!(
            free_error(&failed, request, Lang::De),
            "Keine Lösung mit Imitation: Punkt 2 („e-lé-i-son.“) scheitert in Takt 14 (vor allem HAR-DISSONANCE). Abhilfe: ein anderer --seed, andere --delays, weniger --point-voices oder --imitation aus."
        );
        // A given length: `auto` keeps the setting of phase 12 and says so; `an` imitates in it,
        // and its remedies name what was given.
        let length = prepared(FreeOptions {
            length: Some(20),
            ..FreeOptions::default()
        });
        assert!(length.request.imitation.is_none());
        assert_eq!(
            length.warnings,
            [
                "Imitation aus, denn die Länge ist gegeben (auto imitiert mit automatischer Länge); --imitation an imitiert in der gegebenen Länge."
            ]
        );
        let given = prepared(FreeOptions {
            length: Some(20),
            imitation: ImitationOptions {
                mode: ImitationMode::On,
                head: crate::imitation::AutoCount(Some(5)),
                ..ImitationOptions::default()
            },
            ..FreeOptions::default()
        });
        assert!(given.request.imitation.is_some());
        assert!(
            free_error(&failed, &given.request, Lang::En).ends_with(
                "Remedies: another --seed, other --delays, fewer --point-voices, --head auto, --length auto or --imitation off."
            )
        );
        assert_eq!(
            free_error(
                &GenerateError::ImitationTooLong { measures: 31 },
                &given.request,
                Lang::De
            ),
            "Kein Plan der Imitation passt in die gegebene Länge; der kürzeste hat 31 Takte. Abhilfe: --length 31 oder auto, oder --imitation aus."
        );
    }

    #[test]
    fn cadence_plans_parse() {
        assert_eq!("auto".parse::<CadencePlan>(), Ok(CadencePlan::Auto));
        assert_eq!("keine".parse::<CadencePlan>(), Ok(CadencePlan::None));
        assert_eq!(
            "7, 12:F".parse::<CadencePlan>(),
            Ok(CadencePlan::Measures(vec![(7, None), (12, Some(Step::F))]))
        );
        assert_eq!(
            "5=Bb".parse::<CadencePlan>(),
            Ok(CadencePlan::Measures(vec![(5, Some(Step::B))]))
        );
        assert!("0".parse::<CadencePlan>().is_err());
        assert!("7:H".parse::<CadencePlan>().is_err());
        let mode: ModeSpec = "dorian".parse().unwrap();
        let cadences = "7,12:F".parse::<CadencePlan>().unwrap().cadences(mode, 16);
        assert_eq!(
            cadences,
            [
                Cadence {
                    measure: 6,
                    target: Step::A
                },
                Cadence {
                    measure: 11,
                    target: Step::F
                }
            ]
        );
    }

    #[test]
    fn choirs_and_parts() {
        use VoiceType::*;
        assert_eq!(free_choir(2), [Soprano, Tenor]);
        assert_eq!(free_choir(3), [Alto, Tenor, Bass]);
        assert_eq!(free_choir(4), [Soprano, Alto, Tenor, Bass]);
        assert_eq!(parse_parts("B, S", Lang::De).unwrap(), [Soprano, Bass]);
        assert!(parse_parts("S", Lang::De).is_err());
    }

    #[test]
    fn text_options_parse() {
        assert_eq!("auto".parse::<Length>(), Ok(Length::Auto));
        assert_eq!("16".parse::<Length>().map(Length::measures), Ok(Some(16)));
        assert!("lang".parse::<Length>().is_err());
        assert_eq!("auto".parse::<Repeat>(), Ok(Repeat::Auto));
        for (text, repeat) in [
            ("keine", Repeat::None),
            ("None", Repeat::None),
            ("frei", Repeat::Free),
            ("free", Repeat::Free),
        ] {
            assert_eq!(text.parse::<Repeat>(), Ok(repeat));
        }
        assert!("oft".parse::<Repeat>().is_err());
        assert_eq!(Repeat::Free.name(Lang::De), "frei");
        assert_eq!(Repeat::None.name(Lang::En), "none");
        assert_eq!(on_off("an"), Ok(true));
        assert_eq!(on_off("OFF"), Ok(false));
        assert!(on_off("ja").is_err());
        assert_eq!(eleison("3"), Ok(Eleison::Three));
        assert_eq!(eleison("4"), Ok(Eleison::Four));
        assert!(eleison("2").is_err());
    }

    fn prepared(options: FreeOptions) -> PreparedFree {
        let options = FreeOptions {
            seed: Some(1),
            ..options
        };
        prepare_free(&options, &Profile::fux(), Lang::De).unwrap()
    }

    #[test]
    fn degrees_parse() {
        for (text, step) in [
            ("A", Step::A),
            (" g ", Step::G),
            ("H", Step::B),
            ("Bb", Step::B),
            ("B♭", Step::B),
            ("b", Step::B),
        ] {
            assert_eq!(degree(text), Ok(step), "{text}");
        }
        for text in ["", "X", "A#", "Dur"] {
            assert!(degree(text).is_err(), "{text}");
        }
        // `--repeat` reads as the mass reads it, and converts both ways.
        for repeat in [Repeat::Auto, Repeat::None, Repeat::Free] {
            assert_eq!(Repeat::from(MassRepeat::from(repeat)), repeat);
            assert_eq!(repeat.name(Lang::De).parse::<Repeat>(), Ok(repeat));
        }
        assert_eq!(
            "oft".parse::<Repeat>(),
            Err("\"oft\": auto, keine (none) or frei (free)".to_string())
        );
    }

    /// `--ending` (phase 14): the part of the request ends where asked, and the summary says so;
    /// an ending the mode does not allow, or two voices off the final, fail before any plan.
    /// Without it the request is a setting of its own, as before.
    #[test]
    fn endings_of_a_setting() {
        assert_eq!(
            prepared(FreeOptions::default()).request.part,
            PartContext::default()
        );
        let on_a = prepared(FreeOptions {
            ending: Some(Step::A),
            ..FreeOptions::default()
        });
        assert_eq!(on_a.request.part.ending, Some(Step::A));
        assert!(on_a.request.imitation.is_some());
        assert_eq!(
            free_summary(&on_a, &[], Lang::De)[0],
            "dorisch – freier Satz, 4 Stimmen, 16 Takte, Schluss auf A, seed 1"
        );
        let untexted = prepared(FreeOptions {
            voices: 3,
            text: "none".into(),
            ending: Some(Step::G),
            ..FreeOptions::default()
        });
        assert_eq!(untexted.request.part.ending, Some(Step::G));
        assert_eq!(
            free_summary(&untexted, &[], Lang::En)[0],
            "Dorian – free counterpoint, 3 voices, 12 measures, ending on G, seed 1"
        );
        // B♭, the fourth of the Ionian on F, as each language spells it.
        let flat = prepared(FreeOptions {
            mode: Some("ionisch-transponiert".parse().unwrap()),
            ending: Some(Step::B),
            ..FreeOptions::default()
        });
        assert!(free_summary(&flat, &[], Lang::De)[0].contains(", Schluss auf B, "));
        assert!(free_summary(&flat, &[], Lang::En)[0].contains(", ending on Bb, "));
        let error = |options: FreeOptions, profile: &Profile, lang| {
            let options = FreeOptions {
                seed: Some(1),
                ..options
            };
            prepare_free(&options, profile, lang).map(|_| ())
        };
        let fux = Profile::fux();
        let message =
            |options: FreeOptions, lang| error(options, &fux, lang).unwrap_err().to_string();
        assert_eq!(
            message(
                FreeOptions {
                    ending: Some(Step::E),
                    ..FreeOptions::default()
                },
                Lang::De
            ),
            "Ein Satz in diesem Modus schließt auf D, A oder G (Finalis, Quinte oder Quarte mit \
             einer Klausel), nicht auf E."
        );
        assert_eq!(
            message(
                FreeOptions {
                    ending: Some(Step::B),
                    ..FreeOptions::default()
                },
                Lang::En
            ),
            "A setting in this mode ends on D, A or G (the final, or the fifth or the fourth with \
             a clausula), not on B."
        );
        // Pontio: no Phrygian part ends on C.
        assert!(
            message(
                FreeOptions {
                    mode: Some("phrygisch".parse().unwrap()),
                    ending: Some(Step::C),
                    ..FreeOptions::default()
                },
                Lang::De
            )
            .starts_with("Ein Satz in diesem Modus schließt auf E oder A ")
        );
        let two = FreeOptions {
            voices: 2,
            ending: Some(Step::A),
            ..FreeOptions::default()
        };
        assert_eq!(
            message(two.clone(), Lang::De),
            "Zweistimmig schließt ein Satz auf der Finalis D (CAD-END), nicht auf A. Abhilfe: drei \
             Stimmen oder mehr (--voices) oder --ending D."
        );
        assert_eq!(
            message(two.clone(), Lang::En),
            "In two voices a setting ends on the final D (CAD-END), not on A. Remedies: three \
             voices or more (--voices) or --ending D."
        );
        // On the final, or where CAD-END allows it, two voices may.
        let mut lenient = Profile::fux();
        lenient.apply_override("CAD-END=P4").unwrap();
        assert!(error(two.clone(), &lenient, Lang::De).is_ok());
        assert!(
            error(
                FreeOptions {
                    ending: Some(Step::D),
                    ..two
                },
                &fux,
                Lang::De
            )
            .is_ok()
        );
    }

    /// The overrides a prepared profile makes to Fux's profile, as TOML lines.
    fn changes(prepared: &PreparedFree) -> Vec<String> {
        prepared
            .profile
            .to_toml(Some(&Profile::fux()))
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with("name"))
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn without_a_text_the_request_is_as_before() {
        let untexted = prepared(FreeOptions {
            voices: 2,
            text: "none".into(),
            ..FreeOptions::default()
        });
        let request = &untexted.request;
        assert!(request.text.is_none());
        assert_eq!(request.measures, UNTEXTED_LENGTH);
        assert_eq!(request.cadences, default_cadences(request.mode, 12));
        assert_eq!(request.voices, [VoiceType::Soprano, VoiceType::Tenor]);
        assert!(changes(&untexted).is_empty());
        assert!(untexted.warnings.is_empty());
        // --repeat means nothing without a text.
        let given = prepared(FreeOptions {
            length: Some(8),
            text: "keine".into(),
            repeat: Repeat::Free,
            ..FreeOptions::default()
        });
        assert_eq!(given.request.measures, 8);
        assert!(given.request.cadences.is_empty());
        assert!(changes(&given).is_empty());
    }

    #[test]
    fn a_text_plans_the_request() {
        let kyrie = prepared(FreeOptions::default());
        let request = &kyrie.request;
        let sung = request.text.as_ref().unwrap();
        assert_eq!(sung.text.id, "kyrie");
        assert!(sung.auto_length && sung.auto_cadences && sung.accents);
        // Kyrie I sung twice: 16 measures, a cadence on A after the first statement.
        assert_eq!(request.measures, 16);
        assert_eq!(
            request.cadences,
            [Cadence {
                measure: 6,
                target: Step::A
            }]
        );
        assert_eq!(request.voices.len(), 4);
        assert!(changes(&kyrie).is_empty());
        let summary = free_summary(&kyrie, &[], Lang::De);
        assert_eq!(
            summary,
            [
                "dorisch – freier Satz, 4 Stimmen, 16 Takte, seed 1",
                "Text: Kyrie I (7 Silben, 1 Sinnabschnitt), Wiederholung auto",
                "Binnenschlüsse: Takt 7 auf A"
            ]
        );
        // Without restatements Kyrie I is sung once.
        let once = prepared(FreeOptions {
            repeat: Repeat::None,
            eleison: Eleison::Three,
            accents: false,
            ..FreeOptions::default()
        });
        assert_eq!(
            changes(&once),
            ["[TXT-REPEAT]", "short = \"V\"", "long = \"V\""]
        );
        let sung = once.request.text.as_ref().unwrap();
        assert_eq!(sung.text.syllables.len(), 6);
        assert!(!sung.accents);
        assert_eq!(once.request.measures, 8);
        assert_eq!(
            free_summary(&once, &[], Lang::En)[1],
            "Text: Kyrie I (6 syllables, 1 sense unit), repetition none"
        );
        let free = prepared(FreeOptions {
            repeat: Repeat::Free,
            ..FreeOptions::default()
        });
        // `short` is off in Fux's profile already.
        assert_eq!(
            changes(&free),
            ["[TXT-REPEAT]", "long = \"aus\"", "again = \"aus\""]
        );
        // A given length and given cadences stay.
        let given = prepared(FreeOptions {
            length: Some(12),
            cadences: "none".parse().unwrap(),
            text: "Christe".into(),
            ..FreeOptions::default()
        });
        let sung = given.request.text.as_ref().unwrap();
        assert!(!sung.auto_length && !sung.auto_cadences);
        assert_eq!(given.request.measures, 12);
        assert!(given.request.cadences.is_empty());
        // An own text brings its warnings.
        let own = prepared(FreeOptions {
            text: "Ad te levavi animam meam.".into(),
            ..FreeOptions::default()
        });
        assert_eq!(
            own.warnings,
            ["Akzent unsicher: animam → á-ni-mam; mit Akut angeben"]
        );
    }

    #[test]
    fn texts_that_do_not_fit() {
        // A given length too short for the text: the search says so, with hints. Without
        // imitation, as in phase 12.
        let short = prepared(FreeOptions {
            length: Some(10),
            text: "gloria:1-5".into(),
            imitation: ImitationOptions {
                mode: ImitationMode::Off,
                ..ImitationOptions::default()
            },
            ..FreeOptions::default()
        });
        assert_eq!(short.request.measures, 10);
        assert!(short.request.imitation.is_none());
        let error = generate_free(&short.request, &short.profile).unwrap_err();
        let GenerateError::TextTooLong { measures } = error else {
            panic!("{error:?}");
        };
        assert_eq!(
            free_error(&error, &short.request, Lang::De),
            format!(
                "Der Text braucht mindestens {measures} Takte. Abhilfe: --length {measures} oder auto, ein kürzerer Text (etwa ein Bereich wie gloria:1-5) oder weniger Binnenschlüsse (--cadences)."
            )
        );
        let stuck = GenerateError::NoCounterpoint {
            measure: 9,
            rules: vec!["TXT-COMPLETE"],
        };
        assert!(
            free_error(&stuck, &short.request, Lang::En).ends_with(
                "(mostly TXT-COMPLETE). Remedies: more measures (--length), --repeat free, a shorter text or other internal cadences (--cadences)."
            )
        );
        let untexted = prepared(FreeOptions {
            text: "none".into(),
            ..FreeOptions::default()
        });
        assert_eq!(
            free_error(&stuck, &untexted.request, Lang::De),
            stuck.message(Lang::De)
        );
        // Unknown sections are errors that list the sections.
        let options = FreeOptions {
            text: "kyrei".into(),
            ..FreeOptions::default()
        };
        let error = prepare_free(&options, &Profile::fux(), Lang::En).unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("Unknown section \"kyrei\"; the sections are kyrie, christe,"),
            "{error}"
        );
    }
}
