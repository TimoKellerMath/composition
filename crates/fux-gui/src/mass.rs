//! The mass in the window (phase 14, docs/design/phase-14/spec.md § 7): the mass the form plans,
//! as its job sets it part by part ([`crate::jobs::MassJob`]), and what the window shows of it:
//! where every part stands, how far the mass has come and how long it still takes, a result per
//! movement with the pages the book prints for it, its sound and the violations of its parts,
//! and its files. A mass the user cancels keeps the parts set so far: it is shown, played and
//! written as incomplete, and its missing parts can be set afterwards.

use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use fux_cli::free::Repeat;
use fux_cli::mass::{
    DEFAULT_BASE, Eta, EtaPart, MassNotation, PartLine, cadence_plans_of, decimal, default_jobs,
    mass_summary, mass_totals, mass_warnings, movement_file, parts_dir, running_parts, start_order,
    write_mass, write_part,
};
use fux_cli::output::{OutputArgs, base_of};
use fux_core::generate::GenerateError;
use fux_core::mass::{
    Mass, MassEvent, MassPlan, MovementPlan, PartRef, PartResult, section_score, tables,
};
use fux_core::rules::Report;
use fux_core::text::plan::CadenceChain;
use fux_core::{Duration, Lang, Note, Role, Score, Species, Voice, VoiceType};
use fux_export::{
    LilypondOptions, MassBookOptions, MidiOptions, MovementHeading, MovementOptions,
    MusicXmlOptions, mass_book, to_midi, to_movement_midi, to_movement_musicxml,
};

use crate::jobs::{Entry, MassJob, PreparedMass};

/// Where a part of the mass stands.
#[derive(Clone, Debug)]
pub enum PartState {
    /// Not set, and not searched yet.
    Waiting,
    /// Being searched since `started`.
    Running { started: Instant },
    /// Set: the mass has its result.
    Done,
    /// No attempt set it: the error of its last attempt, which names the part and what may help.
    Failed(GenerateError),
    /// The search was cancelled before it set the part.
    Cancelled,
}

/// A mass in the window: its request, its plan with the parts set so far, where each part
/// stands, and the job that sets parts of it.
pub struct MassRun {
    pub prepared: PreparedMass,
    /// The plan and the parts set.
    pub mass: Mass,
    /// Where each part stands, in the order of the plan.
    pub states: Vec<PartState>,
    /// How long the parts searched took against their estimates, for the time still needed.
    eta: Eta,
    job: Option<MassJob>,
    /// The parts the job sets, in order.
    todo: Vec<PartRef>,
    /// The parts a job sets at once, unless it has fewer to set: those of the window, as many as
    /// `fux mass` without `--jobs` ([`default_jobs`]).
    pub jobs: usize,
    /// The part to which „Teil neu setzen“ gave a seed of its own, and the plan before: it comes
    /// back where no attempt sets the part with it, so that the part keeps the seed it was set
    /// with.
    reseeded: Option<(PartRef, Arc<MassPlan>)>,
    /// The part chosen in the list of parts, for „Teil neu setzen“.
    pub chosen: Option<PartRef>,
    /// A part set before that the last job could not set again with a seed of its own, and
    /// why: it stays as it was.
    pub kept: Option<(PartRef, GenerateError)>,
    /// Why the job ended without its mass (a panic of the search).
    pub error: Option<String>,
    /// The title the window prints the mass with ([`MassRun::set_title`]).
    title: String,
}

impl MassRun {
    /// The mass of `plan`, a plan of `prepared`, before any part is set.
    pub fn new(prepared: PreparedMass, plan: MassPlan) -> MassRun {
        let title = plan.title.clone();
        let mass = Mass::new(Arc::new(plan));
        let states = vec![PartState::Waiting; mass.results.len()];
        MassRun {
            prepared,
            mass,
            states,
            eta: Eta::default(),
            job: None,
            todo: Vec::new(),
            jobs: default_jobs(),
            reseeded: None,
            chosen: None,
            kept: None,
            error: None,
            title,
        }
    }

    pub fn plan(&self) -> &MassPlan {
        &self.mass.plan
    }

    /// The title the window prints the mass with: over its book and its movements, in its files
    /// and its report.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Gives the mass the title `title` of the form, empty for the title of its form (Missa
    /// brevis, Missa): from now on the window prints it so, as it prints the mass with the
    /// Mensurstriche and the intonation of the form, without a new search. Its parts and their
    /// files do not name it. Returns whether the title changed.
    pub fn set_title(&mut self, title: &str) -> bool {
        let title = match title.trim() {
            "" => self.plan().preset.title(),
            title => title,
        };
        if self.title == title {
            return false;
        }
        self.title = title.to_string();
        true
    }

    /// The plan with the title the window prints the mass with.
    fn titled_plan(&self) -> Cow<'_, MassPlan> {
        let plan = self.plan();
        if plan.title == self.title {
            Cow::Borrowed(plan)
        } else {
            Cow::Owned(MassPlan {
                title: self.title.clone(),
                ..plan.clone()
            })
        }
    }

    /// The mass with the title the window prints it with, for its files.
    fn titled(&self) -> Cow<'_, Mass> {
        match self.titled_plan() {
            Cow::Borrowed(_) => Cow::Borrowed(&self.mass),
            Cow::Owned(plan) => Cow::Owned(Mass {
                plan: Arc::new(plan),
                results: self.mass.results.clone(),
                complete: self.mass.complete,
            }),
        }
    }

    /// The plan of the mass as `fux mass --plan` prints it ([`mass_summary`]), with the title
    /// the window prints the mass with, its estimate with the parts set at once as the window
    /// sets them.
    pub fn summary(&self, lang: Lang) -> String {
        mass_summary(&self.titled_plan(), self.jobs, lang)
    }

    /// The place of the part `at` in the order of the mass.
    pub fn index(&self, at: PartRef) -> Option<usize> {
        self.mass.plan.parts().position(|(place, _)| place == at)
    }

    /// Whether a job sets parts of the mass now.
    pub fn is_running(&self) -> bool {
        self.job.is_some()
    }

    /// The parts set.
    pub fn set_count(&self) -> usize {
        self.mass.results.iter().flatten().count()
    }

    /// The parts not set, in the order of the mass.
    pub fn missing(&self) -> Vec<PartRef> {
        self.mass
            .plan
            .parts()
            .zip(&self.mass.results)
            .filter(|(_, result)| result.is_none())
            .map(|((at, _), _)| at)
            .collect()
    }

    /// The parts not set whose search failed, in the order of the mass. Searched again with the
    /// same seed and the same request, each would fail again.
    pub fn failed(&self) -> Vec<PartRef> {
        let missing = self.missing().into_iter();
        missing.filter(|&at| self.has_failed(at)).collect()
    }

    /// The parts not set that „Fehlende Teile setzen“ sets, in the order of the mass: those whose
    /// search was cancelled or has not run, as `fux mass --resume` with the same options sets
    /// them. Not those whose search failed, which would fail again with the same seed and the
    /// same request; „Teil neu setzen“ gives one of them another seed.
    pub fn resumable(&self) -> Vec<PartRef> {
        let missing = self.missing().into_iter();
        missing.filter(|&at| !self.has_failed(at)).collect()
    }

    /// Whether the search of the part `at` failed.
    fn has_failed(&self, at: PartRef) -> bool {
        self.index(at)
            .is_some_and(|i| matches!(self.states[i], PartState::Failed(_)))
    }

    /// Sets `parts` in the background, several at once as `fux mass` sets them, in the order of
    /// the pool of the web version ([`fux_cli::mass::run_mass`]); a part set before keeps its
    /// result until a new one replaces it. Where Kyrie I of a mass with a head motif is among
    /// them, so is every part that opens with the motif (MASS-MOTIF), after it, as `fux mass` sets
    /// them again where it sets Kyrie I again (`--only kyrie/1`, `--resume` without its file):
    /// they begin once Kyrie I is set or has failed, and take the motif the mass then has from it
    /// (the old one where no attempt sets a new Kyrie I).
    pub fn start(&mut self, parts: Vec<PartRef>) {
        let parts = with_motif_parts(self.plan(), parts);
        for &at in &parts {
            if let Some(i) = self.index(at) {
                self.states[i] = PartState::Waiting;
            }
        }
        self.eta = Eta::default();
        self.error = None;
        self.todo = parts.clone();
        self.job = Some(MassJob::spawn(
            self.prepared.clone(),
            self.mass.clone(),
            parts,
            self.at_once(),
        ));
    }

    /// The parts the job sets at once: [`MassRun::jobs`], as far as it has parts to set.
    fn at_once(&self) -> usize {
        self.jobs.clamp(1, self.todo.len().max(1))
    }

    /// The seconds the plan expects of the part `at` beside the others the job sets at once
    /// ([`tables::estimate_jobs`]).
    fn expected(&self, at: PartRef) -> f64 {
        let factor = tables::estimate_jobs(self.at_once());
        f64::from(self.plan().part(at).estimate.seconds) * f64::from(factor)
    }

    /// Sets the part `at` again with the seed `seed` instead of its own, as `fux mass --only PART
    /// --seed N` does: the seed of its search and of the chain of its internal cadences. The part
    /// keeps its result until the new one replaces it; where no attempt sets it, it stays as it
    /// was, with its seed. Kyrie I of a mass with a head motif is set again with the parts that
    /// open with its motif, as `fux mass --only kyrie/1` sets them, each with its own seed
    /// ([`MassRun::start`]).
    pub fn set_again(&mut self, at: PartRef, seed: u64) {
        let mut plan = (*self.mass.plan).clone();
        let part = &mut plan.movements[at.movement].sections[at.section].parts[at.part];
        part.seed = seed;
        if let CadenceChain::Weighted { seed: chain, .. } = &mut part.chain {
            *chain = seed;
        }
        let before = std::mem::replace(&mut self.mass.plan, Arc::new(plan));
        self.reseeded = Some((at, before));
        self.start(vec![at]);
    }

    /// Asks the job to stop: the parts being searched end within a second, the parts set stay.
    pub fn cancel(&self) {
        if let Some(job) = &self.job {
            job.cancel();
        }
    }

    /// Takes over what the job did since the last call; returns whether a part changed. When
    /// the job has ended, the parts it did not set are cancelled, a part that it was to set with
    /// a seed of its own and did not set gets back the seed it was set with, and a part that it
    /// was to set with the head motif of a Kyrie I set anew and did not set is missing
    /// (`MassRun::drop_other_motifs`).
    pub fn poll(&mut self) -> bool {
        let Some(job) = &self.job else {
            return false;
        };
        let events: Vec<MassEvent> = job.events.try_iter().collect();
        let finished = job.is_finished();
        let mut changed = false;
        for event in events {
            changed |= self.take(event);
        }
        if !finished {
            return changed;
        }
        let job = self.job.take().expect("checked");
        // What the thread sent between the events above and its end.
        let rest: Vec<MassEvent> = job.events.try_iter().collect();
        for event in rest {
            self.take(event);
        }
        match job.join() {
            Ok(mass) => self.mass = mass,
            Err(error) => self.error = Some(error),
        }
        self.drop_other_motifs();
        // The parts the job set anew; a part it was to set with a seed of its own and did not
        // keeps the seed it was set with.
        let anew: Vec<bool> = self
            .todo
            .iter()
            .map(|&at| {
                self.index(at)
                    .is_some_and(|i| matches!(self.states[i], PartState::Done))
            })
            .collect();
        if let Some((reseeded, before)) = self.reseeded.take()
            && !self
                .todo
                .iter()
                .zip(&anew)
                .any(|(&at, &set)| at == reseeded && set)
        {
            self.mass.plan = before;
        }
        self.kept = None;
        for (&at, set) in self.todo.iter().zip(anew) {
            let Some(i) = self.index(at) else {
                continue;
            };
            if set {
                continue;
            }
            let state = &self.states[i];
            match (state, self.mass.result(at).is_some()) {
                // A part set before stays as it was.
                (PartState::Failed(error), true) => {
                    self.kept = Some((at, error.clone()));
                    self.states[i] = PartState::Done;
                }
                (_, true) => self.states[i] = PartState::Done,
                (PartState::Waiting | PartState::Running { .. }, false) => {
                    self.states[i] = PartState::Cancelled;
                }
                (PartState::Done | PartState::Failed(_) | PartState::Cancelled, false) => {}
            }
        }
        true
    }

    /// Drops the result of every part the job was to set that opens with the head motif and took
    /// another motif than Kyrie I gives now, where Kyrie I is set (MASS-MOTIF): it no longer
    /// belongs to the mass, as `fux mass --resume` sets such a part again. So a part that the job
    /// did not set again after Kyrie I (cancelled, or no attempt set it with the new motif) is
    /// missing instead of opening with the motif that Kyrie I gave before.
    fn drop_other_motifs(&mut self) {
        let plan = Arc::clone(&self.mass.plan);
        let Some(motif) = plan
            .motif_source()
            .and_then(|kyrie| self.mass.result(kyrie))
            .map(|kyrie| kyrie.motif.clone())
        else {
            return;
        };
        for ((at, part), result) in plan.parts().zip(&mut self.mass.results) {
            let other = result.as_ref().is_some_and(|result| result.motif != motif);
            if part.motif && other && self.todo.contains(&at) {
                *result = None;
            }
        }
        self.mass.complete = self.mass.results.iter().all(Option::is_some);
    }

    /// Takes over one event of the job.
    fn take(&mut self, event: MassEvent) -> bool {
        match event {
            MassEvent::Planned(_) => false,
            MassEvent::Started { at, .. } => {
                if let Some(i) = self.index(at) {
                    self.states[i] = PartState::Running {
                        started: Instant::now(),
                    };
                }
                true
            }
            MassEvent::Finished(result) => {
                let at = result.at;
                let seconds = f64::from(result.seconds);
                self.eta.record(self.expected(at), seconds);
                self.mass.insert(*result);
                if let Some(i) = self.index(at) {
                    self.states[i] = PartState::Done;
                }
                true
            }
            MassEvent::Failed { at, error, .. } => {
                let Some(i) = self.index(at) else {
                    return false;
                };
                if let PartState::Running { started } = &self.states[i] {
                    let elapsed = started.elapsed().as_secs_f64();
                    self.eta.record(self.expected(at), elapsed);
                }
                self.states[i] = PartState::Failed(error);
                true
            }
            MassEvent::Cancelled => {
                for &at in &self.todo {
                    if let Some(i) = self.index(at)
                        && matches!(
                            self.states[i],
                            PartState::Waiting | PartState::Running { .. }
                        )
                    {
                        self.states[i] = PartState::Cancelled;
                    }
                }
                true
            }
        }
    }

    /// The parts being searched, in the order of the mass, each with when it began.
    fn running(&self) -> Vec<(PartRef, Instant)> {
        self.mass
            .plan
            .parts()
            .zip(&self.states)
            .filter_map(|((at, _), state)| match state {
                PartState::Running { started } => Some((at, *started)),
                _ => None,
            })
            .collect()
    }

    /// The parts the job has yet to begin, in the order the job begins them, the order of the
    /// pool of the web version ([`start_order`]), each with the seconds the plan expects of it
    /// beside the others and how it stands to the head motif.
    fn waiting(&self) -> Vec<EtaPart> {
        start_order(self.plan(), &self.todo, self.at_once())
            .into_iter()
            .filter(|&at| {
                self.index(at)
                    .is_some_and(|i| matches!(self.states[i], PartState::Waiting))
            })
            .map(|at| EtaPart::of(self.plan(), at, self.expected(at)))
            .collect()
    }

    /// The line on the parts being searched (§ 4.5), from the measure each search has reached and
    /// the measures the plan expects, with the time the mass still takes, scaled by how long the
    /// parts searched took against their estimates: for one part „Teil 7/26 · Credo · Takt
    /// 23/≈40 · noch etwa 3:40“, for several „kyrie/3 T. 12/≈30 · gloria/2 T. 4/≈38 · noch etwa
    /// 3:40“ ([`running_parts`]); `None` without a job.
    pub fn status(&self, lang: Lang) -> Option<String> {
        let job = self.job.as_ref()?;
        let running = self.running();
        if running.is_empty() {
            return Some(
                lang.pick("Setze die Messe …", "Setting the mass …")
                    .to_string(),
            );
        }
        let times: Vec<(EtaPart, f64)> = running
            .iter()
            .map(|&(at, started)| {
                let part = EtaPart::of(self.plan(), at, self.expected(at));
                (part, started.elapsed().as_secs_f64())
            })
            .collect();
        let remaining = Some(self.eta.remaining(self.at_once(), &times, &self.waiting()));
        let measure = |at: PartRef| job.control.reached(at).unwrap_or(0);
        if let [(at, _)] = running.as_slice() {
            let measures = self.plan().part(*at).estimate.measures;
            let line = PartLine::of(self.plan(), *at, lang);
            return Some(line.running(measure(*at), measures, remaining, lang));
        }
        let parts: Vec<(String, u32, usize)> = running
            .iter()
            .map(|&(at, _)| {
                let measures = self.plan().part(at).estimate.measures;
                (self.plan().address(at), measure(at), measures)
            })
            .collect();
        Some(running_parts(&parts, remaining, lang))
    }

    /// How far the job has come: the share of the seconds its parts are expected to take that
    /// lies behind it (each running part's by its time, at most nine tenths of it), the parts it
    /// has done and the parts it sets; `None` without a job.
    pub fn progress(&self) -> Option<(f32, usize, usize)> {
        self.job.as_ref()?;
        let ratio = self.eta.ratio();
        let (mut total, mut behind, mut done) = (0.0, 0.0, 0);
        for &at in &self.todo {
            let Some(i) = self.index(at) else {
                continue;
            };
            let estimate = self.expected(at).max(0.1);
            total += estimate;
            match &self.states[i] {
                PartState::Waiting => {}
                PartState::Running { started } => {
                    let share = started.elapsed().as_secs_f64() / (estimate * ratio);
                    behind += estimate * share.min(0.9);
                }
                _ => {
                    behind += estimate;
                    done += 1;
                }
            }
        }
        let fraction = if total > 0.0 { behind / total } else { 0.0 };
        Some((fraction as f32, done, self.todo.len()))
    }

    /// A part as the list of parts shows it: its address, how it begins and the degree it ends
    /// on, „kyrie/2 · Christe (Imitation) → A“, with the head motif where it opens with it or
    /// gives it (Kyrie I); then where it stands, „31 Takte, Kosten 71, 8,8 s“, with the head
    /// motif that Kyrie I gives and the number of warnings of a part set
    /// ([`MassRun::warnings`]), „läuft, Takt 12/≈31“, „wartet, ≈ 31 Takte“, „gescheitert“ or
    /// „abgebrochen“.
    pub fn part_lines(&self, lang: Lang) -> Vec<(PartRef, String, String)> {
        let plan = self.plan();
        let source = plan.motif_source();
        plan.parts()
            .zip(&self.states)
            .map(|((at, part), state)| {
                let line = PartLine::of(plan, at, lang);
                // A part that opens with the head motif of Kyrie I says so, and Kyrie I that it
                // gives it.
                let motif = if part.motif {
                    Some(lang.pick("Kopfmotiv", "head motif"))
                } else if source == Some(at) {
                    Some(lang.pick("gibt das Kopfmotiv", "gives the head motif"))
                } else {
                    None
                };
                let opening = match motif {
                    Some(motif) => format!("{}, {motif}", line.opening.name(lang)),
                    None => line.opening.name(lang).to_string(),
                };
                let name = format!(
                    "{} · {} ({opening}) → {}",
                    plan.address(at),
                    line.label,
                    line.ending
                );
                let measures = lang.pick("Takte", "measures");
                let how = match (state, self.mass.result(at)) {
                    (PartState::Running { .. }, _) => {
                        let reached = self.job.as_ref().and_then(|job| job.control.reached(at));
                        let measure = reached.unwrap_or(0);
                        format!(
                            "{}, {} {measure}/≈{}",
                            lang.pick("läuft", "running"),
                            lang.pick("Takt", "measure"),
                            part.estimate.measures
                        )
                    }
                    (PartState::Failed(_), _) => lang.pick("gescheitert", "failed").to_string(),
                    (PartState::Cancelled, None) => {
                        lang.pick("abgebrochen", "cancelled").to_string()
                    }
                    (_, Some(result)) => {
                        let generated = &result.generated;
                        let mut how = format!(
                            "{} {measures}, {} {}, {} s",
                            generated.score.measure_count(),
                            lang.pick("Kosten", "cost"),
                            generated.cost,
                            decimal(f64::from(result.seconds), lang)
                        );
                        if result.attempt >= 3 {
                            how.push_str(&format!(" · {}", lang.pick("Ausweichen", "fallback")));
                        }
                        if source == Some(at) {
                            how.push_str(&format!(" · {}", motif_of(plan, result, lang)));
                        }
                        let warnings = result.warnings.len();
                        if warnings > 0 {
                            how.push_str(&match (lang, warnings) {
                                (Lang::De, 1) => " · 1 Warnung".to_string(),
                                (Lang::De, n) => format!(" · {n} Warnungen"),
                                (Lang::En, 1) => " · 1 warning".to_string(),
                                (Lang::En, n) => format!(" · {n} warnings"),
                            });
                        }
                        how
                    }
                    (_, None) => format!(
                        "{}, ≈ {} {measures}",
                        lang.pick("wartet", "waiting"),
                        part.estimate.measures
                    ),
                };
                (at, name, how)
            })
            .collect()
    }

    /// The warnings of the part `at` where it is set, in `lang`, as the totals list them
    /// ([`mass_warnings`]): what it did otherwise than planned, such as a fallback, a first point
    /// without the head motif of Kyrie I (MASS-MOTIF), or a chant sung from another final
    /// (MASS-CHANT).
    pub fn warnings(&self, at: PartRef, lang: Lang) -> Vec<String> {
        self.mass.result(at).map_or_else(Vec::new, |result| {
            result.warnings.iter().map(|w| w.message(lang)).collect()
        })
    }

    /// Why the part `at` is missing, where it is, as the book says it under its section: the
    /// error of its search in the words of the window (`failure`), which names the part, or
    /// that it is missing because its search was cancelled or has not run yet.
    pub fn why_missing(&self, at: PartRef, lang: Lang) -> Option<String> {
        let reason = self.reason(at, lang)?;
        let address = self.plan().address(at);
        let named = self
            .index(at)
            .is_some_and(|i| matches!(&self.states[i], PartState::Failed(error) if matches!(error, GenerateError::MassPart { .. })));
        Some(match (named, lang) {
            (true, _) => reason,
            (false, Lang::De) => format!("Teil {address} fehlt: {reason}"),
            (false, Lang::En) => format!("Part {address} is missing: {reason}"),
        })
    }

    /// Why the part `at` is missing, as its line in the report says it after „fehlt:“: the error
    /// of its search, or that the search was cancelled or has not run yet; `None` for a part
    /// set.
    fn reason(&self, at: PartRef, lang: Lang) -> Option<String> {
        if self.mass.result(at).is_some() {
            return None;
        }
        let state = self.index(at).map(|i| &self.states[i]);
        Some(match state {
            Some(PartState::Failed(error)) => failure(error, lang),
            Some(PartState::Cancelled) => lang
                .pick("Die Suche wurde abgebrochen.", "The search was cancelled.")
                .to_string(),
            _ => lang
                .pick("Er ist noch nicht gesetzt.", "It is not set yet.")
                .to_string(),
        })
    }

    /// What the window reports of the mass, as `fux mass` prints it and writes it to `NAME.txt`:
    /// the plan, a line per part (set, or missing with why), the cadence plan of every movement,
    /// the totals, the warnings and, for a mass that lacks parts, which and how to set them.
    pub fn report(&self, lang: Lang) -> Vec<String> {
        let m = &self.mass;
        let plan = self.plan();
        let mut report: Vec<String> = self.summary(lang).lines().map(str::to_string).collect();
        for ((at, _), result) in plan.parts().zip(&m.results) {
            report.push(match result {
                Some(result) => {
                    let generated = &result.generated;
                    PartLine::set(plan, result, lang).finished(
                        generated.score.measure_count(),
                        generated.cost,
                        f64::from(result.seconds),
                        None,
                        lang,
                    )
                }
                None => {
                    let why = self.reason(at, lang).unwrap_or_default();
                    PartLine::of(plan, at, lang).missing(&why, lang)
                }
            });
        }
        report.extend(cadence_plans_of(m, lang));
        report.extend(self.totals(lang));
        report
    }

    /// The totals of the mass (§ 7), its warnings, and for a mass that lacks parts which, those
    /// whose search failed marked, and how they are set: „Fehlende Teile setzen“ sets those that
    /// did not fail ([`MassRun::resumable`]); one that failed takes another seed („Teil neu
    /// setzen“) or another of its remedies (`fux mass --resume` after the export). While a job
    /// sets parts, that some are missing goes without saying.
    pub fn totals(&self, lang: Lang) -> Vec<String> {
        let warnings = mass_warnings(&self.mass, lang);
        let mut lines = vec![mass_totals(&self.mass, 0, "", warnings.len(), lang)];
        let warning = lang.pick("Warnung", "Warning");
        lines.extend(warnings.iter().map(|line| format!("{warning}: {line}")));
        let missing = self.missing();
        if missing.is_empty() || self.is_running() {
            return lines;
        }
        let failed = self.failed();
        let named: Vec<String> = missing
            .iter()
            .map(|&at| {
                let address = self.plan().address(at);
                if failed.contains(&at) {
                    format!("{address} ({})", lang.pick("gescheitert", "failed"))
                } else {
                    address
                }
            })
            .collect();
        let named = named.join(", ");
        let resumable = failed.len() < missing.len();
        lines.push(match (lang, failed.is_empty(), resumable) {
            (Lang::De, true, _) => format!(
                "Unvollständig, es fehlen: {named}. „Fehlende Teile setzen“ setzt sie, „Teil neu \
                 setzen“ einen einzelnen mit einem anderen Seed."
            ),
            (Lang::En, true, _) => format!(
                "Incomplete, missing: {named}. “Set the missing parts” sets them, “Set the part \
                 again” a single one with another seed."
            ),
            (Lang::De, false, resumable) => format!(
                "Unvollständig, es fehlen: {named}. {}„Teil neu setzen“ setzt einen gescheiterten \
                 Teil mit einem anderen Seed, fux mass --resume nach dem Export mit einer seiner \
                 übrigen Abhilfen.",
                if resumable {
                    "„Fehlende Teile setzen“ setzt die nicht gescheiterten; "
                } else {
                    ""
                }
            ),
            (Lang::En, false, resumable) => format!(
                "Incomplete, missing: {named}. {}“Set the part again” sets a part that failed \
                 with another seed, fux mass --resume after the export with one of its other \
                 remedies.",
                if resumable {
                    "“Set the missing parts” sets those that did not fail; "
                } else {
                    ""
                }
            ),
        });
        lines
    }

    /// The result of each movement for the list of results: its label, and its score — the
    /// movement whole, its voices named in `lang`, where every part is set, else the sections
    /// that are, one after the other (`set_sections`) — with the repetition of the mass. The
    /// label of a movement that lacks a part counts the parts it prints, those of the Osanna it
    /// repeats for an Osanna ut supra too. The violations are the parts' own
    /// ([`MassRun::part_reports`]).
    pub fn entries(&self, lang: Lang) -> Vec<Entry> {
        let plan = self.plan();
        let repeat = Repeat::from(self.prepared.request.repeat);
        plan.movements
            .iter()
            .enumerate()
            .map(|(mi, movement)| {
                let places: Vec<PartRef> = plan
                    .parts()
                    .filter(|(at, _)| at.movement == mi)
                    .map(|(at, _)| at)
                    .collect();
                let set: Vec<&PartResult> = places
                    .iter()
                    .filter_map(|&at| self.mass.result(at))
                    .collect();
                let title = movement.movement.title();
                let (label, score) = match movement_file(&self.mass, mi, lang) {
                    Some((score, _)) => {
                        let cost: u32 = set.iter().map(|result| result.generated.cost).sum();
                        let label = match lang {
                            Lang::De => {
                                format!("{title} ({} Takte, Kosten {cost})", score.measure_count())
                            }
                            Lang::En => {
                                format!("{title} ({} measures, cost {cost})", score.measure_count())
                            }
                        };
                        (label, score)
                    }
                    None => {
                        let printed = self.printed_parts(mi);
                        let done = printed
                            .iter()
                            .filter(|&&at| self.mass.result(at).is_some())
                            .count();
                        let label = match lang {
                            Lang::De => format!(
                                "{title} – unvollständig, {done} von {} Teilen",
                                printed.len()
                            ),
                            Lang::En => {
                                format!("{title} – incomplete, {done} of {} parts", printed.len())
                            }
                        };
                        (label, set_sections(&self.mass, mi, lang))
                    }
                };
                Entry {
                    label,
                    score,
                    report: Report::default(),
                    marked: false,
                    repeat,
                    before: 0,
                    fields: Vec::new(),
                    seed: None,
                }
            })
            .collect()
    }

    /// The parts whose notes the book prints in the section `si` of the movement `mi`: its own,
    /// or those of the Osanna it repeats for an Osanna ut supra.
    fn section_parts(&self, mi: usize, si: usize) -> Vec<PartRef> {
        let plan = self.plan();
        let (from, repeated) = plan.movements[mi].sections[si].ut_supra.unwrap_or((mi, si));
        let parts = plan.movements[from].sections[repeated].parts.len();
        (0..parts)
            .map(|part| PartRef {
                movement: from,
                section: repeated,
                part,
            })
            .collect()
    }

    /// The parts whose notes the book prints in the movement `mi`, section by section: its own,
    /// and for an Osanna ut supra those of the Osanna it repeats. Its file has them all
    /// ([`movement_file`]).
    fn printed_parts(&self, mi: usize) -> Vec<PartRef> {
        let sections = self.plan().movements[mi].sections.len();
        (0..sections)
            .flat_map(|si| self.section_parts(mi, si))
            .collect()
    }

    /// Whether every part the movement `mi` prints is set (those of the Osanna it repeats for an
    /// Osanna ut supra too), so that it has a file of its own ([`movement_file`]).
    pub fn complete(&self, mi: usize) -> bool {
        self.printed_parts(mi)
            .into_iter()
            .all(|at| self.mass.result(at).is_some())
    }

    /// Whether any section of the movement `mi` has all its parts set (those of the Osanna it
    /// repeats for an Osanna ut supra), so that the book prints notes for it.
    pub fn has_notes(&self, mi: usize) -> bool {
        (0..self.plan().movements[mi].sections.len()).any(|si| {
            let parts = self.section_parts(mi, si);
            !parts.is_empty() && parts.iter().all(|&at| self.mass.result(at).is_some())
        })
    }

    /// A number that changes with what the book prints of the parts of the movement `mi`
    /// ([`MassRun::movement_book`]), those of the Osanna it repeats for an Osanna ut supra too:
    /// for a part set, the seed and the attempt that set it, the head motif it took, its profile
    /// and its cost; for a part missing, why, in `lang`. So a part set anew changes it even where
    /// its seed and attempt stay, as those of a part that opens with the head motif stay when
    /// Kyrie I gives another motif.
    pub fn movement_key(&self, mi: usize, lang: Lang) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.plan().seed.hash(&mut hasher);
        for at in self.printed_parts(mi) {
            let set = self.mass.result(at).map(|result| {
                let cost = result.generated.cost;
                (
                    result.seed,
                    result.attempt,
                    &result.motif,
                    result.profile,
                    cost,
                )
            });
            (set, self.reason(at, lang)).hash(&mut hasher);
        }
        hasher.finish()
    }

    /// The LilyPond source of the movement `mi` as the book of the mass prints it
    /// ([`fux_export::mass_book`]): the mass's title over it ([`MassRun::title`]), the movement's
    /// heading, its intonation (where `notation` asks for it) and remarks, a score per section,
    /// and under a section that lacks a part why it is missing; with the Mensurstriche of
    /// `notation`.
    pub fn movement_book(
        &self,
        mi: usize,
        options: &LilypondOptions,
        notation: &MassNotation,
        lang: Lang,
    ) -> String {
        let alone = movement_mass(&self.mass, mi, &self.title);
        let movement = &self.plan().movements[mi];
        let book = MassBookOptions {
            subtitle: Some(format!(
                "{}, {}",
                movement.mode.name(lang),
                abbreviated(&self.plan().choir)
            )),
            byline: Some(format!(
                "fux-kontrapunkt, {} {}",
                lang.pick("Seed", "seed"),
                self.plan().seed
            )),
            intonation: notation.intonation,
        };
        let missing = |_: usize, section: usize| self.remarks(mi, section, lang);
        let options = LilypondOptions {
            mensurstriche: notation.mensurstriche,
            ..options.clone()
        };
        mass_book(&alone, &book, &missing, &options)
    }

    /// The remarks under the section `section` of the movement `movement` where it lacks parts
    /// (those of the Osanna it repeats for an Osanna ut supra): why each is missing.
    fn remarks(&self, movement: usize, section: usize, lang: Lang) -> Vec<String> {
        self.section_parts(movement, section)
            .into_iter()
            .filter_map(|at| self.why_missing(at, lang))
            .collect()
    }

    /// The file of the movement `mi` as `fux mass` writes it (§ 6): its score as MusicXML with
    /// its heading under the title the window prints the mass with, and the fields of its parts,
    /// from which a check takes it apart again; `None` where a part of it is not set.
    pub fn movement_musicxml(
        &self,
        mi: usize,
        options: &MusicXmlOptions,
        notation: &MassNotation,
        lang: Lang,
    ) -> Option<String> {
        let (score, fields) = movement_file(&self.mass, mi, lang)?;
        let plan = self.plan();
        let movement = &plan.movements[mi];
        let heading = MovementHeading {
            mass: self.title.clone(),
            number: movement.movement as usize + 1,
            title: movement.movement.title().to_string(),
            byline: Some(format!(
                "{}, {} · fux-kontrapunkt, {} {}",
                movement.mode.name(lang),
                abbreviated(&plan.choir),
                lang.pick("Seed", "seed"),
                plan.seed
            )),
            intonation: movement
                .intonation
                .filter(|_| notation.intonation)
                .map(str::to_string),
        };
        let options = MovementOptions {
            musicxml: *options,
            mensurstriche: notation.mensurstriche,
            hide_tacet: notation.hide_tacet,
        };
        Some(to_movement_musicxml(&score, &heading, &options, &fields))
    }

    /// The movement `mi` as a Standard MIDI File: the movement whole as `fux mass` writes it
    /// where every part is set ([`fux_export::to_movement_midi`]), else the sections that are,
    /// one after the other; `None` where no section is.
    pub fn movement_midi(&self, mi: usize, options: &MidiOptions, lang: Lang) -> Option<Vec<u8>> {
        let title = self.plan().movements[mi].movement.title();
        if let Some((score, _)) = movement_file(&self.mass, mi, lang) {
            let name = format!("{} – {title}", self.title);
            return Some(to_movement_midi(&score, &name, options));
        }
        self.has_notes(mi)
            .then(|| to_midi(&set_sections(&self.mass, mi, lang), options))
    }

    /// The parts of the movement `mi` with what the list of violations shows of each: its
    /// heading, its result where it is set, the measures of the movement before it where every
    /// part before it is set (so that its measures count through the movement, as the book and
    /// the cadence plan count them), and the names of its voices in the movement.
    pub fn part_reports(&self, mi: usize, lang: Lang) -> Vec<PartReport<'_>> {
        let plan = self.plan();
        let movement = &plan.movements[mi];
        let names = fux_core::mass::voice_names(&movement.voices, lang);
        let mut before = Some(0);
        let mut reports = Vec::new();
        for (at, _) in plan.parts().filter(|(at, _)| at.movement == mi) {
            let result = self.mass.result(at);
            let label = PartLine::of(plan, at, lang).label;
            let heading = match (result, before) {
                (Some(result), Some(first)) => {
                    let last = first + result.generated.score.measure_count();
                    match lang {
                        Lang::De => format!(
                            "Teil {}: {label}, Takte {}–{last}",
                            plan.address(at),
                            first + 1
                        ),
                        Lang::En => format!(
                            "Part {}: {label}, measures {}–{last}",
                            plan.address(at),
                            first + 1
                        ),
                    }
                }
                (Some(result), None) => {
                    let measures = result.generated.score.measure_count();
                    match lang {
                        Lang::De => {
                            format!("Teil {}: {label}, {measures} Takte", plan.address(at))
                        }
                        Lang::En => {
                            format!("Part {}: {label}, {measures} measures", plan.address(at))
                        }
                    }
                }
                (None, _) => match lang {
                    Lang::De => format!("Teil {}: {label}", plan.address(at)),
                    Lang::En => format!("Part {}: {label}", plan.address(at)),
                },
            };
            let voices = result
                .map(|result| {
                    result
                        .voices
                        .iter()
                        .map(|&v| names.get(v).cloned().unwrap_or_default())
                        .collect()
                })
                .unwrap_or_default();
            reports.push(PartReport {
                at,
                heading,
                result,
                before: before.filter(|_| result.is_some()),
                voices,
            });
            before = match (before, result) {
                (Some(first), Some(result)) => Some(first + result.generated.score.measure_count()),
                _ => None,
            };
        }
        reports
    }

    /// The cadence plan of the movement `mi` ([`fux_cli::mass::cadence_plan`]).
    pub fn cadence_plan(&self, mi: usize, lang: Lang) -> Option<String> {
        let title = self.plan().movements[mi].movement.title();
        cadence_plans_of(&self.mass, lang)
            .into_iter()
            .find(|line| line.starts_with(&format!("{title} –")))
    }

    /// Writes the mass to the base of `output` (§§ 6 and 7, D16): first the file of every part
    /// set to `NAME.teile/`, as `fux mass` writes them, so that `fux mass --resume` and `fux
    /// check` take them up; then `NAME.txt` with the report, the files of every movement whose
    /// parts are all set and the book, in the formats of `output`
    /// ([`fux_cli::mass::write_mass`]), under the title the window prints the mass with. A
    /// section that lacks a part shows in the book with why. Returns the files written, the
    /// folder of the parts last.
    pub fn export(
        &self,
        output: &OutputArgs,
        notation: &MassNotation,
        lang: Lang,
    ) -> anyhow::Result<Vec<PathBuf>> {
        let base = base_of(output, DEFAULT_BASE);
        let xml = output.musicxml_options(lang);
        let plan = self.plan();
        let mut parts = 0;
        for result in self.mass.results.iter().flatten() {
            write_part(plan, result, &base, &xml, lang)?;
            parts += 1;
        }
        let failed: Vec<(PartRef, String)> = self
            .missing()
            .into_iter()
            .filter_map(|at| Some((at, self.why_missing(at, lang)?)))
            .collect();
        let mut files = write_mass(
            &self.titled(),
            &failed,
            &self.report(lang),
            output,
            notation,
            lang,
        )?;
        if parts > 0 {
            files.push(parts_dir(&base));
        }
        Ok(files)
    }
}

/// A part of a movement as the list of violations shows it ([`MassRun::part_reports`]).
pub struct PartReport<'a> {
    pub at: PartRef,
    /// „Teil kyrie/2: Christe, Takte 33–57“.
    pub heading: String,
    pub result: Option<&'a PartResult>,
    /// The measures of the movement before it, where they are known.
    pub before: Option<usize>,
    /// The names of its voices in the movement, from the top.
    pub voices: Vec<String>,
}

/// Why a part of a mass could not be set, in the words of the window: the message of its search
/// (the part, its words, the measure and the rules where it got stuck, what may help), its remedy
/// of another seed being „Teil neu setzen“. The other remedies name the options of `fux mass`,
/// which the fields of the form take as they are.
pub fn failure(error: &GenerateError, lang: Lang) -> String {
    let (any, own) = match lang {
        Lang::De => (
            "ein anderer Seed (--seed)",
            "„Teil neu setzen“ (ein anderer Seed)",
        ),
        Lang::En => (
            "another seed (--seed)",
            "“Set the part again” (another seed)",
        ),
    };
    error.message(lang).replacen(any, own, 1)
}

/// The head motif that Kyrie I gives, `result`, a part of `plan`, as the list of parts names it:
/// „Kopfmotiv D F E D C A“, its tones in the mode of the Kyrie; „kein Kopfmotiv“ where it gives
/// none (it begins together, or its first subject has not five to ten tones).
fn motif_of(plan: &MassPlan, result: &PartResult, lang: Lang) -> String {
    let Some(motif) = &result.motif else {
        return lang.pick("kein Kopfmotiv", "no head motif").to_string();
    };
    let mode = plan.movements[result.at.movement].mode;
    let tones: Vec<String> = motif
        .pitches(mode, 4)
        .iter()
        .map(|pitch| pitch.class_name(lang))
        .collect();
    format!(
        "{} {}",
        lang.pick("Kopfmotiv", "head motif"),
        tones.join(" ")
    )
}

/// The parts to set where `parts` of a mass of `plan` are to be set, in the order of the mass:
/// with Kyrie I of a mass with a head motif every part that opens with the motif (MASS-MOTIF),
/// which takes the motif that Kyrie I gives then, as `fux mass` sets them again with Kyrie I.
fn with_motif_parts(plan: &MassPlan, parts: Vec<PartRef>) -> Vec<PartRef> {
    if !plan
        .motif_source()
        .is_some_and(|kyrie| parts.contains(&kyrie))
    {
        return parts;
    }
    plan.parts()
        .filter(|(at, part)| part.motif || parts.contains(at))
        .map(|(at, _)| at)
        .collect()
}

/// Voices from the top by their abbreviations: „S A T B“.
fn abbreviated(voices: &[VoiceType]) -> String {
    let mut voices = voices.to_vec();
    voices.sort();
    voices
        .iter()
        .map(|voice| voice.abbreviation())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The sections of the movement `mi` of `m` whose parts are all set, one after the other, in
/// all the voices of the movement named in `lang`; a measure's rest in every voice where none
/// is. For the movement of a mass that lacks parts.
fn set_sections(m: &Mass, mi: usize, lang: Lang) -> Score {
    let movement = &m.plan.movements[mi];
    let names = fux_core::mass::voice_names(&movement.voices, lang);
    let sections: Vec<Score> = (0..movement.sections.len())
        .filter_map(|si| section_score(m, mi, si))
        .collect();
    let voices = movement
        .voices
        .iter()
        .zip(&names)
        .enumerate()
        .map(|(v, (&voice_type, name))| {
            let mut notes: Vec<Note> = sections
                .iter()
                .flat_map(|section| section.voices[v].notes.iter().cloned())
                .collect();
            if notes.is_empty() {
                notes.push(Note::rest(Duration::WHOLE));
            }
            let mut voice = Voice::new(voice_type, Role::Counterpoint(Species::Free), notes);
            voice.name = name.clone();
            voice
        })
        .collect();
    let mut score = Score::new(movement.mode, voices);
    score.title = Some(movement.movement.title().to_string());
    score
}

/// The movement `mi` of `m` as a mass of its own titled `title`, for the pages of its book: its
/// plan with this movement alone, and the results of its parts. An Osanna ut supra, which
/// repeats the Osanna of the Sanctus, gets the Osanna's parts as its own, their voices those of
/// the movement, so that the book prints it as the book of the whole mass does.
fn movement_mass(m: &Mass, mi: usize, title: &str) -> Mass {
    let plan = &m.plan;
    let mut movement: MovementPlan = plan.movements[mi].clone();
    let mut results = Vec::new();
    for (si, section) in movement.sections.iter_mut().enumerate() {
        let (from, repeated) = section.ut_supra.take().unwrap_or((mi, si));
        let source = &plan.movements[from];
        if (from, repeated) != (mi, si) {
            section.parts = source.sections[repeated].parts.clone();
        }
        for part in 0..section.parts.len() {
            let found = m.result(PartRef {
                movement: from,
                section: repeated,
                part,
            });
            results.push(found.map(|result| PartResult {
                at: PartRef {
                    movement: 0,
                    section: si,
                    part,
                },
                voices: places_in(&source.voices, &result.voices, &plan.movements[mi].voices),
                ..result.clone()
            }));
        }
    }
    let alone = MassPlan {
        movements: vec![movement],
        title: title.to_string(),
        ..(**plan).clone()
    };
    let complete = results.iter().all(Option::is_some);
    Mass {
        plan: Arc::new(alone),
        results,
        complete,
    }
}

/// The places among `target` of the voices `places` of `source`: the k-th voice of a type is the
/// k-th of that type; a voice the target lacks keeps its place.
fn places_in(source: &[VoiceType], places: &[usize], target: &[VoiceType]) -> Vec<usize> {
    places
        .iter()
        .map(|&v| {
            let Some(&kind) = source.get(v) else {
                return v;
            };
            let k = source[..v].iter().filter(|&&other| other == kind).count();
            target
                .iter()
                .enumerate()
                .filter(|&(_, &other)| other == kind)
                .nth(k)
                .map_or(v, |(i, _)| i)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use fux_core::rules::Profile;

    use super::*;
    use crate::forms::MassForm;

    /// A mass planned and not yet set: every part waits, with the measures the plan expects;
    /// the parts that open with the head motif say so, and Kyrie I that it gives it; nothing
    /// runs; each part is missing, and the report and the book say why. The voices of a part map
    /// onto those of another movement by type and order.
    #[test]
    fn a_planned_mass_waits() {
        let form = MassForm {
            movements: [true, true, false, false, false, false],
            motif: true,
            ..MassForm::default()
        };
        let (prepared, plan) =
            crate::jobs::prepare_mass(&form, 1, &Profile::fux(), Lang::De).unwrap();
        let run = MassRun::new(prepared, plan);
        assert!(!run.is_running() && run.status(Lang::De).is_none());
        assert!(run.progress().is_none());
        assert_eq!(run.missing().len(), run.mass.results.len());
        let lines = run.part_lines(Lang::De);
        assert_eq!(
            lines[0].1,
            "kyrie/1 · Kyrie I (Imitation, gibt das Kopfmotiv) → D"
        );
        assert!(
            lines[1].1.starts_with("kyrie/2 · Christe (Imitation) → "),
            "{lines:?}"
        );
        assert!(lines[0].2.starts_with("wartet, ≈ "), "{lines:?}");
        assert!(
            lines[3]
                .1
                .starts_with("gloria/1 · gloria:1-5 (Imitation, Kopfmotiv) → "),
            "{lines:?}"
        );
        let first = PartRef {
            movement: 0,
            section: 0,
            part: 0,
        };
        assert_eq!(
            run.why_missing(first, Lang::De).unwrap(),
            "Teil kyrie/1 fehlt: Er ist noch nicht gesetzt."
        );
        let report = run.report(Lang::De);
        assert!(
            report
                .iter()
                .any(|line| line.contains("Kyrie I (Imitation)")
                    && line.ends_with("fehlt: Er ist noch nicht gesetzt.")),
            "{report:?}"
        );
        assert!(
            report
                .last()
                .unwrap()
                .starts_with("Unvollständig, es fehlen: kyrie/1, kyrie/2, kyrie/3, gloria/1"),
            "{report:?}"
        );
        let entries = run.entries(Lang::De);
        assert_eq!(entries[0].label, "Kyrie – unvollständig, 0 von 3 Teilen");
        assert!(!run.has_notes(0) && !run.complete(0));
        use VoiceType::*;
        assert_eq!(
            places_in(
                &[Soprano, Tenor, Tenor, Bass],
                &[0, 2, 3],
                &[Tenor, Tenor, Bass]
            ),
            [0, 1, 2]
        );
        assert_eq!(
            places_in(&[Alto, Tenor, Bass], &[0, 1, 2], &[Alto, Tenor, Bass]),
            [0, 1, 2]
        );
    }

    /// A small mass with the head motif: a Kyrie and an Agnus Dei in two voices, every part in
    /// imitation, with a narrow search; its Kyrie I gives a motif of seven tones.
    fn motif_form() -> MassForm {
        MassForm {
            movements: [true, false, false, false, false, true],
            voices: 2,
            imitation: "an".into(),
            lengths: fux_core::mass::Lengths::Natural,
            motif: true,
            beam: 40,
            random_seed: false,
            seed: 6,
            ..MassForm::default()
        }
    }

    /// Polls the job of `run` until `done` holds of it; fails after five minutes.
    fn poll_until(run: &mut MassRun, done: impl Fn(&MassRun) -> bool) {
        let start = Instant::now();
        while !done(run) {
            assert!(start.elapsed().as_secs() < 300, "not within five minutes");
            run.poll();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// The seeds and costs of the parts of `run`, in order; `None` for a part not set.
    fn set_parts(run: &MassRun) -> Vec<Option<(u64, u32)>> {
        let results = run.mass.results.iter();
        results
            .map(|result| result.as_ref().map(|r| (r.seed, r.generated.cost)))
            .collect()
    }

    /// The head motif in the window (MASS-MOTIF). Set whole, Agnus Dei I opens with the motif
    /// that Kyrie I gives, as `fux mass` sets it, and the list of parts names the motif. Kyrie I
    /// set again with a seed of its own is set with Agnus Dei I, as `fux mass --only kyrie/1
    /// --seed N` sets them, and Agnus Dei I takes the new motif; the other parts stay. Its seed
    /// and attempt stay too, but its notes change, and so do the pages of the Agnus Dei. A part
    /// that opens with the motif and was not set again after Kyrie I (its job cancelled, or no
    /// attempt set it) is missing instead of keeping the motif Kyrie I gave before.
    #[test]
    fn kyrie_i_set_again_sets_the_parts_of_its_motif() {
        let form = motif_form();
        let (prepared, plan) =
            crate::jobs::prepare_mass(&form, form.seed, &Profile::fux(), Lang::De).unwrap();
        let mut run = MassRun::new(prepared, plan);
        let at = |run: &MassRun, address: &str| run.plan().find(address).unwrap()[0];
        let (kyrie, agnus) = (at(&run, "kyrie/1"), at(&run, "agnus/1"));
        let christe = at(&run, "christe");
        let planned = run.plan().part(kyrie).seed;
        let all: Vec<PartRef> = run.plan().parts().map(|(at, _)| at).collect();
        assert_eq!(with_motif_parts(run.plan(), vec![christe]), [christe]);
        assert_eq!(with_motif_parts(run.plan(), vec![kyrie]), [kyrie, agnus]);
        assert_eq!(with_motif_parts(run.plan(), all.clone()), all);
        run.start(all);
        poll_until(&mut run, |run| !run.is_running());
        assert_eq!(run.set_count(), 5);
        let motif = run.mass.result(kyrie).unwrap().motif.clone();
        assert_eq!(
            motif.as_ref().map(ToString::to_string).as_deref(),
            Some("0,1,2,1,0,-2,-3")
        );
        assert_eq!(run.mass.result(agnus).unwrap().motif, motif);
        assert!(run.warnings(agnus, Lang::De).is_empty());
        let lines = run.part_lines(Lang::De);
        assert!(
            lines[0].2.ends_with(" · Kopfmotiv D E F E D H A")
                && lines[3].1 == "agnus/1 · Agnus Dei I (Imitation, Kopfmotiv) → D",
            "{lines:?}"
        );
        // Kyrie I with a seed of its own, and Agnus Dei I with its motif.
        let first = set_parts(&run);
        let pages = run.movement_key(agnus.movement, Lang::De);
        run.set_again(kyrie, 1);
        assert_eq!(run.todo, [kyrie, agnus]);
        poll_until(&mut run, |run| !run.is_running());
        let again = set_parts(&run);
        assert_eq!(
            [again[1], again[2], again[4]],
            [first[1], first[2], first[4]]
        );
        let other = run.mass.result(kyrie).unwrap().motif.clone();
        assert_ne!(again[0], first[0]);
        assert_ne!(other, motif);
        assert_eq!(run.mass.result(agnus).unwrap().motif, other);
        // Agnus Dei I keeps its seed and attempt, but sings the new motif: its pages are new.
        assert_eq!(
            again[3].map(|(seed, _)| seed),
            first[3].map(|(seed, _)| seed)
        );
        let anew = run.movement_key(agnus.movement, Lang::De);
        assert_ne!(anew, pages);
        // A part that opens with the motif and keeps another than Kyrie I gives is missing once
        // a job that was to set it ends; the pages say so.
        run.todo = vec![agnus];
        run.mass.results[3].as_mut().unwrap().motif = motif.clone();
        assert_ne!(run.movement_key(agnus.movement, Lang::De), anew);
        run.drop_other_motifs();
        assert!(run.mass.result(agnus).is_none() && !run.mass.complete);
        assert_ne!(run.movement_key(agnus.movement, Lang::De), anew);
        // Cancelled once Kyrie I is set anew: Agnus Dei I is set with its motif, or missing.
        run.start(vec![agnus]);
        poll_until(&mut run, |run| !run.is_running());
        assert_eq!(run.mass.result(agnus).unwrap().motif, other);
        run.set_again(kyrie, planned);
        poll_until(&mut run, |run| matches!(run.states[0], PartState::Done));
        run.cancel();
        poll_until(&mut run, |run| !run.is_running());
        let kyrie_i = run.mass.result(kyrie).unwrap();
        assert_eq!(kyrie_i.motif, motif);
        match run.mass.result(agnus) {
            Some(result) => assert_eq!(result.motif, motif),
            None => assert!(matches!(run.states[3], PartState::Cancelled)),
        }
    }

    /// The Benedictus of the form normal ends with the Osanna of the Sanctus („Osanna ut
    /// supra“), in three voices with a narrow search. Its book prints that Osanna, so its pages
    /// change with it: where it is missing, why, and where it is set anew. The movement is
    /// complete, with a file of its own, only once the Osanna is set, and till then its label
    /// counts it among its parts. The title of the form goes over its pages, its file and the
    /// report without a new search.
    #[test]
    fn the_benedictus_prints_the_osanna_of_the_sanctus() {
        let form = MassForm {
            preset: fux_core::mass::Preset::Normal,
            movements: [false, false, false, true, true, false],
            voices: 3,
            beam: 40,
            random_seed: false,
            seed: 1,
            ..MassForm::default()
        };
        let (prepared, plan) =
            crate::jobs::prepare_mass(&form, form.seed, &Profile::fux(), Lang::De).unwrap();
        let mut run = MassRun::new(prepared, plan);
        let at = |run: &MassRun, address: &str| run.plan().find(address).unwrap()[0];
        let (osanna, benedictus) = (at(&run, "sanctus/3"), at(&run, "benedictus/1"));
        let (sanctus, own) = (osanna.movement, benedictus.movement);
        assert!(!run.complete(own) && !run.has_notes(own));
        assert_eq!(
            run.entries(Lang::De)[1].label,
            "Benedictus – unvollständig, 0 von 2 Teilen"
        );
        // Why the Osanna is missing changes the pages of both movements.
        let (pages, of_sanctus) = (
            run.movement_key(own, Lang::De),
            run.movement_key(sanctus, Lang::De),
        );
        let i = run.index(osanna).unwrap();
        run.states[i] = PartState::Cancelled;
        assert_ne!(run.movement_key(own, Lang::De), pages);
        assert_ne!(run.movement_key(sanctus, Lang::De), of_sanctus);
        // The Benedictus without the Osanna: notes, but no file of its own.
        run.start(vec![benedictus]);
        poll_until(&mut run, |run| !run.is_running());
        assert!(run.mass.result(benedictus).is_some());
        assert!(run.has_notes(own) && !run.complete(own));
        let notation = MassNotation::default();
        let xml = MusicXmlOptions::default();
        assert!(
            run.movement_musicxml(own, &xml, &notation, Lang::De)
                .is_none()
        );
        assert_eq!(
            run.entries(Lang::De)[1].label,
            "Benedictus – unvollständig, 1 von 2 Teilen"
        );
        let book = run.movement_book(own, &LilypondOptions::default(), &notation, Lang::De);
        assert!(book.contains("Teil sanctus/3 fehlt: Die Suche wurde abgebrochen."));
        // With the Osanna: complete, and new pages; so once the Osanna is set anew.
        let pages = run.movement_key(own, Lang::De);
        run.start(vec![osanna]);
        poll_until(&mut run, |run| !run.is_running());
        assert!(run.complete(own));
        assert!(run.entries(Lang::De)[1].label.starts_with("Benedictus ("));
        let anew = run.movement_key(own, Lang::De);
        assert_ne!(anew, pages);
        let seed = run.mass.result(osanna).unwrap().seed;
        run.set_again(osanna, 12345);
        poll_until(&mut run, |run| !run.is_running());
        assert!(run.kept.is_none(), "{:?}", run.kept);
        assert_ne!(run.mass.result(osanna).unwrap().seed, seed);
        assert_ne!(run.movement_key(own, Lang::De), anew);
        // The title of the form, without a new search; empty, that of the form of the mass.
        assert_eq!(run.title(), "Missa");
        assert!(!run.set_title(" "));
        assert!(run.set_title(" Missa prima "));
        assert_eq!(run.title(), "Missa prima");
        assert!(
            run.summary(Lang::De)
                .starts_with("Missa prima (Form normal) – dorisch")
        );
        let book = run.movement_book(own, &LilypondOptions::default(), &notation, Lang::De);
        assert!(book.contains("Missa prima"), "{book}");
        let file = run.movement_musicxml(own, &xml, &notation, Lang::De);
        assert!(file.unwrap().contains("Missa prima"));
        assert!(run.report(Lang::De)[0].starts_with("Missa prima (Form normal)"));
        assert_eq!(run.plan().title, "Missa");
        assert!(run.set_title(""));
        assert_eq!(run.title(), "Missa");
    }

    /// A part whose search failed is left to „Teil neu setzen“: with the same seed and the same
    /// request it would fail again (MASS-SEED). „Fehlende Teile setzen“ sets the parts cancelled
    /// or not searched yet, and the totals mark the parts that failed and say what helps them.
    #[test]
    fn parts_that_failed_are_left_to_another_seed() {
        let form = MassForm {
            movements: [true, false, false, false, false, false],
            voices: 3,
            ..MassForm::default()
        };
        let (prepared, plan) =
            crate::jobs::prepare_mass(&form, 1, &Profile::fux(), Lang::De).unwrap();
        let mut run = MassRun::new(prepared, plan);
        let parts: Vec<PartRef> = run.plan().parts().map(|(at, _)| at).collect();
        let error = || PartState::Failed(GenerateError::Invalid("no".into()));
        run.states = vec![error(), PartState::Cancelled, PartState::Waiting];
        assert_eq!(run.failed(), [parts[0]]);
        assert_eq!(run.resumable(), [parts[1], parts[2]]);
        assert_eq!(
            run.totals(Lang::De).last().unwrap(),
            "Unvollständig, es fehlen: kyrie/1 (gescheitert), kyrie/2, kyrie/3. „Fehlende Teile \
             setzen“ setzt die nicht gescheiterten; „Teil neu setzen“ setzt einen gescheiterten \
             Teil mit einem anderen Seed, fux mass --resume nach dem Export mit einer seiner \
             übrigen Abhilfen."
        );
        assert_eq!(
            run.totals(Lang::En).last().unwrap(),
            "Incomplete, missing: kyrie/1 (failed), kyrie/2, kyrie/3. “Set the missing parts” \
             sets those that did not fail; “Set the part again” sets a part that failed with \
             another seed, fux mass --resume after the export with one of its other remedies."
        );
        run.states = vec![error(), error(), error()];
        assert!(run.resumable().is_empty());
        assert_eq!(
            run.totals(Lang::De).last().unwrap(),
            "Unvollständig, es fehlen: kyrie/1 (gescheitert), kyrie/2 (gescheitert), kyrie/3 \
             (gescheitert). „Teil neu setzen“ setzt einen gescheiterten Teil mit einem anderen \
             Seed, fux mass --resume nach dem Export mit einer seiner übrigen Abhilfen."
        );
        run.states = vec![PartState::Cancelled; 3];
        assert_eq!(run.resumable(), parts);
        assert_eq!(
            run.totals(Lang::De).last().unwrap(),
            "Unvollständig, es fehlen: kyrie/1, kyrie/2, kyrie/3. „Fehlende Teile setzen“ setzt \
             sie, „Teil neu setzen“ einen einzelnen mit einem anderen Seed."
        );
    }
}
