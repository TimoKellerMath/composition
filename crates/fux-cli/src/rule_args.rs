//! Choosing the rule profile on the command line.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use fux_core::Lang;
use fux_core::rules::Profile;

/// Rule profile and single overrides.
#[derive(Args, Debug, Default)]
pub struct RuleArgs {
    /// Regelprofil: fux (Standard), jeppesen (strenger, vorläufig) oder eine TOML-Datei mit
    /// Änderungen am Profil „fux“
    #[arg(long, value_name = "PROFIL")]
    pub profile: Option<PathBuf>,
    /// Regel einstellen: ID=STUFE (alle Stufen der Regel, z. B. MEL-REPEAT=aus) oder
    /// ID.PARAMETER=WERT (z. B. CF-LENGTH.max=24); mehrfach angebbar
    #[arg(long = "rule", value_name = "ID=WERT")]
    pub rules: Vec<String>,
}

impl RuleArgs {
    pub fn profile(&self, lang: Lang) -> Result<Profile> {
        let mut profile = Profile::fux();
        if let Some(builtin) = self
            .profile
            .as_ref()
            .and_then(|p| p.to_str())
            .and_then(Profile::builtin)
        {
            profile = builtin;
        } else if let Some(path) = &self.profile {
            let text = std::fs::read_to_string(path).with_context(|| {
                format!(
                    "{}: {}",
                    path.display(),
                    lang.pick("nicht lesbar", "not readable")
                )
            })?;
            profile = profile.merged_toml(&text).with_context(|| {
                format!(
                    "{} {}",
                    lang.pick("Regelprofil", "Rule profile"),
                    path.display()
                )
            })?;
        }
        for spec in &self.rules {
            profile
                .apply_override(spec)
                .with_context(|| format!("--rule {spec}"))?;
        }
        Ok(profile)
    }
}
