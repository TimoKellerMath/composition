//! Rule profiles: a value for every parameter of every rule, read from TOML.
//!
//! The standard profile `profiles/fux.toml` holds the defaults derived from Fux's examples.
//! Other profiles only list what they change, such as `profiles/jeppesen.toml`, which comes with
//! the program too. On the command line, `ID=LEVEL` sets all level parameters of a rule but those
//! for settings without a plan of imitation ([`ParamInfo::follows_rule`]); `ID=aus` switches the
//! rule off, those too (`MEL-REPEAT=aus`, `IMI-HELD=aus`). `ID.PARAM=VALUE` sets one parameter.

use std::collections::BTreeMap;

use thiserror::Error;

use super::{Level, ParamInfo, ParamKind, ParamValue, RuleSettings, all_rules};

const FUX_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../profiles/fux.toml"
));

const JEPPESEN_TOML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../profiles/jeppesen.toml"
));

/// The names of the profiles that come with the program.
pub const BUILTIN_PROFILES: [&str; 2] = ["fux", "jeppesen"];

/// Problems in a profile or an override.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ProfileError {
    #[error("invalid TOML: {0}")]
    Toml(String),
    #[error("unknown rule {0}")]
    UnknownRule(String),
    #[error("rule {rule}: unknown parameter {param}")]
    UnknownParam { rule: String, param: String },
    #[error("rule {rule}, parameter {param}: invalid value {value}")]
    InvalidValue {
        rule: String,
        param: String,
        value: String,
    },
    #[error("expected ID=VALUE or ID.PARAMETER=VALUE, got {0:?}")]
    BadOverride(String),
}

/// Parameter values for all rules.
#[derive(Clone, Debug)]
pub struct Profile {
    pub name: String,
    settings: BTreeMap<&'static str, RuleSettings>,
}

impl Profile {
    /// The standard profile: defaults derived from Fux's examples (`profiles/fux.toml`).
    pub fn fux() -> Profile {
        Profile::empty("fux")
            .merged_toml(FUX_TOML)
            .expect("profiles/fux.toml is valid")
    }

    /// Jeppesen's stricter rules of the Palestrina style as changes to Fux's
    /// (`profiles/jeppesen.toml`); provisional.
    pub fn jeppesen() -> Profile {
        Profile::fux()
            .merged_toml(JEPPESEN_TOML)
            .expect("profiles/jeppesen.toml is valid")
    }

    /// A profile that comes with the program, by name (see [`BUILTIN_PROFILES`]).
    pub fn builtin(name: &str) -> Option<Profile> {
        match name.to_lowercase().as_str() {
            "fux" => Some(Profile::fux()),
            "jeppesen" => Some(Profile::jeppesen()),
            _ => None,
        }
    }

    fn empty(name: &str) -> Profile {
        Profile {
            name: name.to_string(),
            settings: all_rules()
                .iter()
                .map(|r| (r.info().id, RuleSettings::default()))
                .collect(),
        }
    }

    /// Applies a TOML profile on top of this one. Parameters it does not mention keep their
    /// values; a top-level `name` renames the profile.
    pub fn merged_toml(mut self, text: &str) -> Result<Profile, ProfileError> {
        let table: toml::Table =
            toml::from_str(text).map_err(|e| ProfileError::Toml(e.to_string()))?;
        for (key, value) in table {
            if key == "name" {
                if let toml::Value::String(name) = value {
                    self.name = name;
                }
                continue;
            }
            let toml::Value::Table(params) = value else {
                return Err(ProfileError::UnknownRule(key));
            };
            for (param, value) in params {
                let text = match &value {
                    toml::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                self.set(&key, &param, &text)?;
            }
        }
        Ok(self)
    }

    /// Applies `ID=LEVEL` (every level parameter of the rule that follows it,
    /// [`ParamInfo::follows_rule`]; with `aus` every level parameter, so that the rule is off)
    /// or `ID.PARAM=VALUE`.
    pub fn apply_override(&mut self, spec: &str) -> Result<(), ProfileError> {
        let (key, value) = spec
            .split_once('=')
            .ok_or_else(|| ProfileError::BadOverride(spec.to_string()))?;
        let (key, value) = (key.trim(), value.trim());
        match key.split_once('.') {
            Some((rule, param)) => self.set(rule, param, value),
            None => {
                let level = Level::parse(value).ok_or_else(|| ProfileError::InvalidValue {
                    rule: key.to_string(),
                    param: "*".to_string(),
                    value: value.to_string(),
                })?;
                let params = rule_params(key)?;
                let settings = self.settings_mut(key)?;
                let off = level == Level::Off;
                let set = |p: &&ParamInfo| p.follows_rule() || (off && p.kind == ParamKind::Level);
                for p in params.iter().filter(set) {
                    settings.set(p.name, ParamValue::Level(level));
                }
                Ok(())
            }
        }
    }

    fn set(&mut self, rule: &str, param: &str, value: &str) -> Result<(), ProfileError> {
        let info = rule_params(rule)?
            .iter()
            .find(|p| p.name == param)
            .ok_or_else(|| ProfileError::UnknownParam {
                rule: rule.to_string(),
                param: param.to_string(),
            })?;
        let invalid = || ProfileError::InvalidValue {
            rule: rule.to_string(),
            param: param.to_string(),
            value: value.to_string(),
        };
        let parsed = match info.kind {
            ParamKind::Level => ParamValue::Level(Level::parse(value).ok_or_else(invalid)?),
            ParamKind::Int => ParamValue::Int(value.parse().map_err(|_| invalid())?),
            ParamKind::Bool => ParamValue::Bool(match value.to_lowercase().as_str() {
                "true" | "ja" | "yes" | "an" | "on" => true,
                "false" | "nein" | "no" => false,
                _ => return Err(invalid()),
            }),
        };
        self.settings_mut(rule)?.set(info.name, parsed);
        Ok(())
    }

    fn settings_mut(&mut self, rule: &str) -> Result<&mut RuleSettings, ProfileError> {
        self.settings
            .get_mut(rule)
            .ok_or_else(|| ProfileError::UnknownRule(rule.to_string()))
    }

    /// The settings of a rule (empty, i.e. off, for an unknown id).
    pub fn settings(&self, rule: &str) -> &RuleSettings {
        static EMPTY: RuleSettings = RuleSettings { values: Vec::new() };
        self.settings.get(rule).unwrap_or(&EMPTY)
    }

    /// The profile as TOML, readable by [`Profile::merged_toml`] and `--profile`: with `base`
    /// only the parameters that differ from it, otherwise all. Rules appear in the order of the
    /// catalogue, levels as `aus`, `V`, `P1`–`P4` or `P(n)`.
    pub fn to_toml(&self, base: Option<&Profile>) -> String {
        let mut out = format!("name = {}\n", toml::Value::String(self.name.clone()));
        for rule in all_rules() {
            let id = rule.info().id;
            let changed: Vec<String> = rule
                .info()
                .params
                .iter()
                .filter_map(|param| {
                    let value = self.settings(id).get(param.name)?;
                    if base.is_some_and(|b| b.settings(id).get(param.name) == Some(value)) {
                        return None;
                    }
                    let text = match value {
                        ParamValue::Level(level) => {
                            toml::Value::String(level.label(crate::lang::Lang::De)).to_string()
                        }
                        ParamValue::Int(n) => n.to_string(),
                        ParamValue::Bool(b) => b.to_string(),
                    };
                    Some(format!("{} = {text}", param.name))
                })
                .collect();
            if !changed.is_empty() {
                out.push_str(&format!("\n[{id}]\n{}\n", changed.join("\n")));
            }
        }
        out
    }

    /// Parameters without a value, as (rule, parameter).
    pub fn missing(&self) -> Vec<(&'static str, &'static str)> {
        all_rules()
            .iter()
            .flat_map(|rule| {
                let id = rule.info().id;
                let settings = self.settings(id).clone();
                rule.info()
                    .params
                    .iter()
                    .filter(move |p| !settings.values().iter().any(|(n, _)| *n == p.name))
                    .map(move |p| (id, p.name))
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

fn rule_params(rule: &str) -> Result<&'static [ParamInfo], ProfileError> {
    all_rules()
        .iter()
        .find(|r| r.info().id == rule)
        .map(|r| r.info().params)
        .ok_or_else(|| ProfileError::UnknownRule(rule.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_profile_sets_every_parameter() {
        let profile = Profile::fux();
        assert_eq!(profile.missing(), []);
    }

    #[test]
    fn jeppesen_changes_fux() {
        let jeppesen = Profile::builtin("Jeppesen").unwrap();
        assert_eq!(jeppesen.name, "jeppesen");
        assert_eq!(jeppesen.missing(), []);
        assert_eq!(
            jeppesen.settings("MEL-MIN6-DOWN").level("level"),
            Level::Forbidden
        );
        assert_eq!(
            jeppesen.settings("G3-PASSING").level("neighbor"),
            Level::Off
        );
        assert_eq!(
            jeppesen.settings("G3-PASSING").level("upper_neighbor"),
            Level::Forbidden
        );
        // Saved as changes to Fux, the profile reads back the same.
        let diff = jeppesen.to_toml(Some(&Profile::fux()));
        assert!(diff.contains("[G3-ACCENT-LEAP]\nlevel = \"V\""), "{diff}");
        let again = Profile::fux().merged_toml(&diff).unwrap();
        assert_eq!(again.to_toml(None), jeppesen.to_toml(None));
        assert!(Profile::builtin("palestrina").is_none());
        for name in BUILTIN_PROFILES {
            assert_eq!(Profile::builtin(name).unwrap().name, name);
        }
    }

    #[test]
    fn profiles_as_toml() {
        let fux = Profile::fux();
        let again = Profile::empty("fux")
            .merged_toml(&fux.to_toml(None))
            .unwrap();
        assert_eq!(again.to_toml(None), fux.to_toml(None));
        let mut changed = fux.clone();
        changed.name = "eigen".into();
        changed.apply_override("MEL-REPEAT.g1=P3").unwrap();
        changed.apply_override("CF-LENGTH.max=24").unwrap();
        let diff = changed.to_toml(Some(&fux));
        assert_eq!(
            diff,
            "name = \"eigen\"\n\n[CF-LENGTH]\nmax = 24\n\n[MEL-REPEAT]\ng1 = \"P3\"\n"
        );
        let applied = Profile::fux().merged_toml(&diff).unwrap();
        assert_eq!(applied.to_toml(None), changed.to_toml(None));
    }

    #[test]
    fn overrides() {
        let mut profile = Profile::fux();
        profile.apply_override("MEL-REPEAT=aus").unwrap();
        assert!(profile.settings("MEL-REPEAT").is_off());
        profile.apply_override("MEL-REPEAT.g1=P3").unwrap();
        assert_eq!(profile.settings("MEL-REPEAT").level("g1"), Level::P3);
        assert_eq!(
            profile.apply_override("NO-SUCH=V"),
            Err(ProfileError::UnknownRule("NO-SUCH".into()))
        );
        assert!(matches!(
            profile.apply_override("MEL-REPEAT.nope=V"),
            Err(ProfileError::UnknownParam { .. })
        ));
        assert!(matches!(
            profile.apply_override("MEL-REPEAT"),
            Err(ProfileError::BadOverride(_))
        ));
        profile.apply_override("CF-LENGTH.max=30").unwrap();
        assert_eq!(profile.settings("CF-LENGTH").int("max"), 30);
    }

    #[test]
    fn toml_profiles_merge() {
        let profile = Profile::fux()
            .merged_toml("name = \"streng\"\n[MEL-MIN6-DOWN]\nlevel = \"V\"\n")
            .unwrap();
        assert_eq!(profile.name, "streng");
        assert_eq!(
            profile.settings("MEL-MIN6-DOWN").level("level"),
            Level::Forbidden
        );
        assert_eq!(
            Profile::fux().settings("MEL-MIN6-DOWN").level("level"),
            Level::Off
        );
    }
}
