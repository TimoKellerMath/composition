//! Versions of a fugue: the same request with the values of some aspects changed, so that the
//! versions differ in these aspects and in nothing else. Each version says what it differs in.

use anyhow::{Result, bail};
use serde_json::{Value, json};

use crate::rng::Rng;

/// An aspect in which versions can differ: the field of the request, the name on the command
/// line, its name in German and English, and its values with their names.
pub struct Aspect {
    pub key: &'static str,
    pub name: &'static str,
    pub label: [&'static str; 2],
    pub values: &'static [(&'static str, [&'static str; 2])],
}

/// The aspects, in the order in which a version names them. The seed comes last and has no
/// list of values: it counts on from the seed of the request.
pub const ASPECTS: [Aspect; 4] = [
    Aspect {
        key: "free_voices",
        name: "free-voices",
        label: ["Freie Stimmen", "Free voices"],
        values: &[
            (
                "figures",
                ["mit Figuren und Sequenzen", "with figures and sequences"],
            ),
            ("plain", ["schlicht", "plain"]),
        ],
    },
    Aspect {
        key: "profile",
        name: "sound",
        label: ["Klang", "Sound"],
        values: &[
            ("fuge", ["nach dem Ohr", "by ear"]),
            ("bach", ["nach der Zählung", "by the count"]),
        ],
    },
    Aspect {
        key: "bass",
        name: "bass",
        label: ["Bass", "Bass"],
        values: &[
            ("free", ["frei", "free"]),
            ("planned", ["geplant", "planned"]),
        ],
    },
    Aspect {
        key: "seed",
        name: "seed",
        label: ["Seed", "Seed"],
        values: &[],
    },
];

/// At most this many versions at a time.
pub const MOST: usize = 8;

/// One version: its letter, the request that sets it, and what it differs in from the others
/// (aspect and value, by name).
#[derive(Clone, Debug, PartialEq)]
pub struct Version {
    pub letter: char,
    pub request: Value,
    pub differs: Vec<(String, String)>,
}

impl Version {
    /// `Bass: geplant · Klang: nach dem Ohr`.
    pub fn describe(&self) -> String {
        self.differs
            .iter()
            .map(|(aspect, value)| format!("{aspect}: {value}"))
            .collect::<Vec<_>>()
            .join(" · ")
    }

    /// What the version is called in its files: its letter, and what it differs in unless the
    /// comparison is blind.
    pub fn name(&self, english: bool, blind: bool) -> String {
        let word = if english { "Version" } else { "Fassung" };
        if blind {
            format!("{word} {}", self.letter)
        } else {
            format!("{word} {}: {}", self.letter, self.describe())
        }
    }
}

/// The names of the aspects the versions differ in.
pub fn aspects(versions: &[Version]) -> Vec<String> {
    versions
        .first()
        .map(|v| v.differs.iter().map(|(aspect, _)| aspect.clone()).collect())
        .unwrap_or_default()
}

/// The versions of a request for a fugue: every combination of the values of the aspects in
/// `vary` (names as on the command line), the request's own value first; with `seed` among
/// them `seeds` seeds from the request's on. With `blind` the letters are drawn, so that they
/// say nothing of the versions.
pub fn versions(
    request: &Value,
    vary: &[String],
    seeds: u64,
    english: bool,
    blind: bool,
) -> Result<Vec<Version>> {
    let language = usize::from(english);
    for name in vary {
        if !ASPECTS.iter().any(|a| a.name == name) {
            let names: Vec<&str> = ASPECTS.iter().map(|a| a.name).collect();
            bail!(
                "versions cannot differ in {name:?}; the aspects are {}",
                names.join(", ")
            );
        }
    }
    let mut list: Vec<(Value, Vec<(String, String)>)> = vec![(request.clone(), Vec::new())];
    for aspect in ASPECTS.iter().filter(|a| vary.iter().any(|n| n == a.name)) {
        // The values of the aspect, as the request takes them and as they are called.
        let values: Vec<(Value, String)> = if aspect.key == "seed" {
            let first = request["seed"].as_u64().unwrap_or(1);
            (0..seeds.max(1))
                .map(|k| (json!(first + k), (first + k).to_string()))
                .collect()
        } else {
            let own = request[aspect.key].as_str().unwrap_or(aspect.values[0].0);
            let mut values: Vec<&(&str, [&str; 2])> = aspect.values.iter().collect();
            values.sort_by_key(|(value, _)| *value != own);
            values
                .into_iter()
                .map(|(value, names)| (json!(value), names[language].to_string()))
                .collect()
        };
        list = list
            .into_iter()
            .flat_map(|(base, differs)| {
                values.iter().map(move |(value, name)| {
                    let mut request = base.clone();
                    request[aspect.key] = value.clone();
                    let mut differs = differs.clone();
                    differs.push((aspect.label[language].to_string(), name.clone()));
                    (request, differs)
                })
            })
            .collect();
    }
    if list.len() < 2 {
        bail!("versions need an aspect to differ in (--vary) with at least two values");
    }
    if list.len() > MOST {
        bail!(
            "that would be {} versions; at most {MOST} at a time",
            list.len()
        );
    }
    if blind {
        // Fisher and Yates, from the seed of the request: the same call draws the same letters.
        let mut rng = Rng::new(request["seed"].as_u64().unwrap_or(1) ^ 0x5EED_B11D);
        for i in (1..list.len()).rev() {
            list.swap(i, rng.below(i as u64 + 1) as usize);
        }
    }
    Ok(list
        .into_iter()
        .zip('a'..)
        .map(|((request, differs), letter)| Version {
            letter,
            request,
            differs,
        })
        .collect())
}

/// The call of `fux-fuge fugue` that sets the fugue of a request.
pub fn command_line(request: &Value) -> String {
    let quote = |text: &str| format!("\"{}\"", text.replace('"', "\\\""));
    let mut words = vec!["fux-fuge".to_string()];
    if let Some(lang) = request["lang"].as_str().filter(|l| *l != "de") {
        words.push(format!("--lang {lang}"));
    }
    words.push("fugue".to_string());
    for (field, flag) in [
        ("theme", "--theme"),
        ("subject", "--subject"),
        ("key", "--key"),
        ("meter", "--meter"),
        ("ending", "--ending"),
        ("plan", "--plan"),
        ("answer", "--answer"),
        ("bass", "--bass"),
        ("profile", "--profile"),
    ] {
        // A subject of the program has its own meter, and `auto` is the ending anyway.
        let needless = (field == "meter" && request["theme"].is_string())
            || (field == "ending" && request[field].as_str() == Some("auto"));
        if let Some(text) = request[field]
            .as_str()
            .filter(|t| !t.is_empty() && !needless)
        {
            let plain = text
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_/#.".contains(c));
            words.push(format!(
                "{flag} {}",
                if plain { text.to_string() } else { quote(text) }
            ));
        }
    }
    for (field, flag) in [
        ("period_bars", "--period-bars"),
        ("voices", "--voices"),
        ("seed", "--seed"),
        ("beam", "--beam"),
    ] {
        if let Some(number) = request[field]
            .as_u64()
            .filter(|n| !(field == "beam" && *n == 400))
        {
            words.push(format!("{flag} {number}"));
        }
    }
    if request["free_voices"].as_str() == Some("plain") {
        words.push("--plain".to_string());
    }
    for rule in request["rules"].as_array().into_iter().flatten() {
        if let Some(rule) = rule.as_str() {
            words.push(format!("--rule {}", quote(rule)));
        }
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Value {
        json!({"command": "fugue", "theme": "bwv1080", "voices": 4, "seed": 3, "bass": "planned"})
    }

    #[test]
    fn versions_differ_in_what_is_asked_and_say_so() {
        let vary = ["bass".to_string(), "sound".to_string()];
        let list = versions(&request(), &vary, 3, false, false).unwrap();
        // The order of the aspects is the list's, the request's own value comes first.
        let said: Vec<String> = list.iter().map(|v| v.name(false, false)).collect();
        assert_eq!(
            said,
            [
                "Fassung a: Klang: nach dem Ohr · Bass: geplant",
                "Fassung b: Klang: nach dem Ohr · Bass: frei",
                "Fassung c: Klang: nach der Zählung · Bass: geplant",
                "Fassung d: Klang: nach der Zählung · Bass: frei",
            ]
        );
        assert_eq!(aspects(&list), ["Klang", "Bass"]);
        assert_eq!(list[3].request["profile"], "bach");
        assert_eq!(list[3].request["bass"], "free");
        // Everything else is the request's.
        for version in &list {
            let mut plain = version.request.clone();
            plain["bass"] = json!("planned");
            plain.as_object_mut().unwrap().remove("profile");
            assert_eq!(plain, request());
        }
        assert_eq!(list[1].name(true, true), "Version b");
    }

    #[test]
    fn seeds_count_on_and_letters_are_drawn_blind() {
        let seeds = versions(&request(), &["seed".to_string()], 3, true, false).unwrap();
        let said: Vec<String> = seeds.iter().map(Version::describe).collect();
        assert_eq!(said, ["Seed: 3", "Seed: 4", "Seed: 5"]);
        assert_eq!(seeds[2].request["seed"], 5);
        // Blind: the same versions under drawn letters, the same for the same request.
        let vary = [
            "bass".to_string(),
            "free-voices".to_string(),
            "seed".to_string(),
        ];
        let open = versions(&request(), &vary, 2, false, false).unwrap();
        let blind = versions(&request(), &vary, 2, false, true).unwrap();
        assert_eq!(open.len(), 8);
        let letters: String = blind.iter().map(|v| v.letter).collect();
        assert_eq!(letters, "abcdefgh");
        let texts = |list: &[Version]| -> Vec<String> {
            let mut texts: Vec<String> = list.iter().map(Version::describe).collect();
            texts.sort();
            texts
        };
        assert_eq!(texts(&open), texts(&blind));
        assert_ne!(
            open.iter().map(Version::describe).collect::<Vec<_>>(),
            blind.iter().map(Version::describe).collect::<Vec<_>>()
        );
        assert_eq!(blind, versions(&request(), &vary, 2, false, true).unwrap());
    }

    #[test]
    fn what_cannot_be_asked() {
        let request = request();
        assert!(versions(&request, &[], 3, false, false).is_err());
        assert!(versions(&request, &["tempo".to_string()], 3, false, false).is_err());
        assert!(versions(&request, &["seed".to_string()], 1, false, false).is_err());
        let all: Vec<String> = ASPECTS.iter().map(|a| a.name.to_string()).collect();
        let error = versions(&request, &all, 3, false, false).unwrap_err();
        assert!(error.to_string().contains("24 versions"), "{error}");
    }

    #[test]
    fn the_call_that_sets_a_version() {
        let request = json!({
            "command": "fugue", "subject": "c'4 d' e'", "key": "C", "voices": 3, "seed": 7,
            "free_voices": "plain", "bass": "planned", "profile": "bach", "lang": "en",
            "rules": ["MEL-REPEAT=P2"],
        });
        assert_eq!(
            command_line(&request),
            "fux-fuge --lang en fugue --subject \"c'4 d' e'\" --key C --bass planned --profile bach --voices 3 --seed 7 --plain --rule \"MEL-REPEAT=P2\""
        );
        // What the program does anyway is not said.
        let theme = json!({
            "command": "fugue", "theme": "bwv1080", "meter": "4/4", "ending": "auto",
            "bass": "free", "profile": "fuge", "voices": 4, "seed": 1, "beam": 400,
        });
        assert_eq!(
            command_line(&theme),
            "fux-fuge fugue --theme bwv1080 --bass free --profile fuge --voices 4 --seed 1"
        );
    }
}
