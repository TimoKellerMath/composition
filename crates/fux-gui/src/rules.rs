//! The window with all rules: every parameter can be changed, the profile saved and loaded.

use std::collections::BTreeMap;
use std::thread::JoinHandle;

use eframe::egui;
use fux_cli::commands::corpus_counts;
use fux_cli::palestrina;
use fux_core::Lang;
use fux_core::rules::{Level, ParamKind, ParamValue, Profile, RuleInfo, all_rules};

/// The levels offered for a level parameter.
const LEVELS: [Level; 6] = [
    Level::Off,
    Level::P1,
    Level::P2,
    Level::P3,
    Level::P4,
    Level::Forbidden,
];

/// How often each rule fires in Fux's examples: (all, prohibitions).
type Counts = BTreeMap<&'static str, (usize, usize)>;

/// The state of the rules window.
#[derive(Default)]
pub struct RulesWindow {
    pub open: bool,
    filter: String,
    /// Result of the last loading or saving.
    message: Option<String>,
    /// The frequencies in Fux's examples, and their computation.
    corpus: Option<Counts>,
    counting: Option<JoinHandle<Counts>>,
    /// The same in Palestrina's masses, or why they could not be counted.
    palestrina: Option<Result<Counts, String>>,
    counting_palestrina: Option<JoinHandle<Result<Counts, String>>>,
}

impl RulesWindow {
    /// Shows the window; returns whether the profile changed.
    pub fn show(&mut self, ctx: &egui::Context, profile: &mut Profile, lang: Lang) -> bool {
        let mut open = self.open;
        let mut changed = false;
        egui::Window::new(lang.pick("Regeln", "Rules"))
            .open(&mut open)
            .default_size([620.0, 640.0])
            .vscroll(false)
            .show(ctx, |ui| {
                changed = self.contents(ui, profile, lang);
            });
        self.open = open;
        changed
    }

    fn contents(&mut self, ui: &mut egui::Ui, profile: &mut Profile, lang: Lang) -> bool {
        let mut changed = false;
        if self.counting.as_ref().is_some_and(JoinHandle::is_finished) {
            self.corpus = self.counting.take().and_then(|h| h.join().ok());
        }
        if self
            .counting_palestrina
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
        {
            self.palestrina = self.counting_palestrina.take().and_then(|h| h.join().ok());
        }
        ui.horizontal(|ui| {
            ui.label(format!(
                "{}: {}",
                lang.pick("Profil", "Profile"),
                profile.name
            ));
            if ui
                .button(lang.pick("Standard (Fux)", "Default (Fux)"))
                .clicked()
            {
                *profile = Profile::fux();
                changed = true;
                self.message = None;
            }
            if ui
                .button("Jeppesen")
                .on_hover_text(lang.pick(
                    "Jeppesens strengere Regeln des Palestrinastils (vorläufig)",
                    "Jeppesen's stricter rules of the Palestrina style (provisional)",
                ))
                .clicked()
            {
                *profile = Profile::jeppesen();
                changed = true;
                self.message = None;
            }
            if ui.button(lang.pick("Laden …", "Load …")).clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("TOML", &["toml"])
                    .pick_file()
            {
                let loaded = std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|text| Profile::fux().merged_toml(&text).map_err(|e| e.to_string()));
                match loaded {
                    Ok(p) => {
                        *profile = p;
                        changed = true;
                        self.message = Some(path.display().to_string());
                    }
                    Err(error) => self.message = Some(error),
                }
            }
            if ui.button(lang.pick("Speichern …", "Save …")).clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("TOML", &["toml"])
                    .set_file_name("profil.toml")
                    .save_file()
            {
                let text = profile.to_toml(Some(&Profile::fux()));
                self.message = Some(match std::fs::write(&path, text) {
                    Ok(()) => path.display().to_string(),
                    Err(e) => e.to_string(),
                });
            }
        });
        if let Some(message) = &self.message {
            ui.label(message.as_str());
        }
        ui.horizontal(|ui| {
            if self.counting.is_some() {
                ui.spinner();
                ui.ctx().request_repaint();
            } else if ui
                .button(lang.pick("Häufigkeit bei Fux", "Frequency in Fux"))
                .on_hover_text(lang.pick(
                    "Wie oft jede Regel in Fux' 123 Beispielen anschlägt (gesamt / Verbote)",
                    "How often each rule fires in Fux's 123 examples (all / prohibitions)",
                ))
                .clicked()
            {
                let profile = profile.clone();
                self.counting = Some(std::thread::spawn(move || corpus_counts(&profile)));
            }
            if self.counting_palestrina.is_some() {
                ui.spinner();
                ui.ctx().request_repaint();
            } else if ui
                .button(lang.pick("bei Palestrina", "in Palestrina"))
                .on_hover_text(lang.pick(
                    "Wie oft jede Regel in Palestrinas Messen anschlägt (freier Satz; gesamt / Verbote)",
                    "How often each rule fires in Palestrina's masses (free counterpoint; all / prohibitions)",
                ))
                .clicked()
            {
                let profile = profile.clone();
                self.counting_palestrina = Some(std::thread::spawn(move || {
                    palestrina::counts(&profile, lang).map_err(|e| format!("{e:#}"))
                }));
            }
        });
        if let Some(Err(error)) = &self.palestrina {
            ui.colored_label(egui::Color32::RED, error.as_str());
        }
        ui.horizontal(|ui| {
            let label = ui.label(lang.pick("Suche:", "Search:"));
            ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text(lang.pick("ID oder Titel", "id or title"))
                    .desired_width(240.0),
            )
            .labelled_by(label.id);
        });
        ui.label(lang.pick(
            "Stufen: aus, P1–P4 (Kosten 1, 2, 4, 8), V (verboten). Änderungen gelten ab dem nächsten Lauf.",
            "Levels: off, P1–P4 (cost 1, 2, 4, 8), V (forbidden). Changes apply from the next run.",
        ));
        ui.separator();
        let filter = self.filter.to_lowercase();
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for rule in all_rules() {
                    let info = rule.info();
                    let title = info.title(lang);
                    if !filter.is_empty()
                        && !info.id.to_lowercase().contains(&filter)
                        && !title.to_lowercase().contains(&filter)
                    {
                        continue;
                    }
                    let mut frequency = self.corpus.as_ref().map_or(String::new(), |counts| {
                        let (all, hard) = counts.get(info.id).copied().unwrap_or_default();
                        format!("  [{all} / {hard}]")
                    });
                    if let Some(Ok(counts)) = &self.palestrina {
                        let (all, hard) = counts.get(info.id).copied().unwrap_or_default();
                        frequency.push_str(&format!("  [Palestrina {all} / {hard}]"));
                    }
                    egui::CollapsingHeader::new(format!("{}  {title}{frequency}", info.id))
                        .id_salt(info.id)
                        .show(ui, |ui| {
                            changed |= parameters(ui, info, profile, lang);
                            ui.weak(info.source);
                        });
                }
            });
        changed
    }
}

/// The editors of one rule's parameters; returns whether one changed.
fn parameters(ui: &mut egui::Ui, info: &RuleInfo, profile: &mut Profile, lang: Lang) -> bool {
    let mut changed = false;
    egui::Grid::new(info.id).num_columns(2).show(ui, |ui| {
        for param in info.params {
            let label = ui.label(lang.pick(param.de, param.en));
            let value = profile.settings(info.id).get(param.name);
            let setting = match (param.kind, value) {
                (ParamKind::Level, Some(ParamValue::Level(level))) => {
                    let mut chosen = level;
                    egui::ComboBox::from_id_salt((info.id, param.name))
                        .selected_text(level.label(lang))
                        .show_ui(ui, |ui| {
                            for option in LEVELS {
                                ui.selectable_value(&mut chosen, option, option.label(lang));
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    (chosen != level).then(|| chosen.label(Lang::De))
                }
                (ParamKind::Int, Some(ParamValue::Int(n))) => {
                    let mut number = n;
                    ui.add(egui::DragValue::new(&mut number).range(0..=999))
                        .labelled_by(label.id);
                    (number != n).then(|| number.to_string())
                }
                (ParamKind::Bool, Some(ParamValue::Bool(b))) => {
                    let mut flag = b;
                    ui.checkbox(&mut flag, "").labelled_by(label.id);
                    (flag != b).then(|| flag.to_string())
                }
                _ => {
                    ui.label("–");
                    None
                }
            };
            if let Some(text) = setting
                && profile
                    .apply_override(&format!("{}.{}={text}", info.id, param.name))
                    .is_ok()
            {
                changed = true;
            }
            ui.end_row();
        }
    });
    changed
}
