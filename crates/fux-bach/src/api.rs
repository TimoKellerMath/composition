//! Requests as JSON, answers as JSON: the interface of the web page (`crates/fux-bach-web`) and
//! of anything else that wants a fugue without linking the types.
//!
//! ```json
//! {"command": "themes"}
//! {"command": "fugue", "theme": "bwv1080", "voices": 4, "seed": 1}
//! {"command": "fugue", "subject": "d'2 a' | f' d' | cis' d'4 e' | f'2~ f'8 g' f' e' | d'4",
//!  "key": "d", "meter": "2/2", "period_bars": 4, "voices": 3, "ending": "auto",
//!  "free_voices": "figures", "plan": "wtk1-02", "answer": "…", "profile": "fuge",
//!  "rules": ["MEL-REPEAT=P2"], "seed": 1, "lang": "de"}
//! ```
//!
//! `free_voices` is `figures` (the default: the episodes as sequences in all voices, the free
//! voices with figures, the fugue set two to four times) or `plain` (one setting from the rhythm
//! tables, in the manner of phase B4). `profile` is `fuge` (the default: the composer's prices
//! after the listening tests of phase B6) or `bach` (the prices calibrated on Bach's rates);
//! `rules` change single levels on top. `bass` is `free` (the default: the search finds what
//! lies lowest) or `planned` (the scaffold names a bass for every chord, phase B7). `variant`
//! is a name for the fugue among versions of it (`Fassung b: Bass: geplant`), which its files
//! then carry.
//!
//! An answer has `"ok": true` and its content, or `"ok": false` and `"error"`.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::base64;
use crate::check;
use crate::compose::{self, Ending, Fugue, Options, Template};
use crate::export;
use crate::key::Key;
use crate::musicxml::musicxml;
use crate::rules::Profile;
use crate::score::Note;
use crate::stats::{self, COUNT, NAMES};
use crate::tables;
use crate::themes::{self, BUILTIN, Subject};
use crate::time::Meter;

/// The answer to a request, as text. Never fails: an error is an answer too.
pub fn answer_json(request: &str) -> String {
    match answer(request) {
        Ok(value) => value.to_string(),
        Err(error) => json!({ "ok": false, "error": format!("{error:#}") }).to_string(),
    }
}

pub fn answer(request: &str) -> Result<Value> {
    let request: Value = serde_json::from_str(request).context("the request is not JSON")?;
    match request["command"].as_str() {
        Some("themes") => Ok(themes_answer()),
        Some("version") => Ok(json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") })),
        Some("fugue") => fugue_answer(&request),
        Some(other) => bail!("unknown command {other:?}"),
        None => bail!("the request names no command"),
    }
}

fn themes_answer() -> Value {
    let list: Vec<Value> = BUILTIN
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "title": t.title,
                "key": t.key,
                "meter": format!("{}/{}", t.meter.0, t.meter.1),
                "period_bars": t.period as f64 / Meter { num: t.meter.0, den: t.meter.1 }.bar() as f64,
                "subject": t.text,
            })
        })
        .collect();
    json!({ "ok": true, "themes": list, "plans": Template::names() })
}

/// The subject a request names: one that comes with the program, or its own tones.
pub fn subject_of(request: &Value) -> Result<Subject> {
    if let Some(name) = request["theme"].as_str() {
        let builtin = BUILTIN
            .iter()
            .find(|t| t.name == name)
            .with_context(|| format!("no theme {name:?}"))?;
        return builtin.subject();
    }
    let text = request["subject"]
        .as_str()
        .context("the request has neither theme nor subject")?;
    let notes = themes::parse(text)?;
    if notes.len() < 3 {
        bail!("a subject needs at least three tones");
    }
    let key = Key::parse(
        request["key"]
            .as_str()
            .context("the request names no key")?,
    )
    .context("the key is a letter with # or b, upper case for major: C, f#, Bb")?;
    let meter = Meter::parse(request["meter"].as_str().unwrap_or("4/4"))
        .context("the meter is two numbers: 4/4")?;
    let bar = meter.bar();
    let period = match request["period_bars"].as_f64() {
        Some(bars) if bars > 0.0 => {
            let ticks = (bars * bar as f64).round() as i32;
            // Entries keep the subject's place in the bar: whole bars, or half bars where the
            // two halves of a bar weigh alike.
            let unit = if meter.num.is_multiple_of(2) && !meter.compound() {
                bar / 2
            } else {
                bar
            };
            (ticks / unit).max(1) * unit
        }
        _ => themes::default_period(&notes, meter),
    };
    Ok(Subject {
        name: request["name"].as_str().unwrap_or("thema").to_string(),
        title: request["title"].as_str().unwrap_or("").to_string(),
        key,
        meter,
        notes,
        period,
        from: None,
        development: false,
    })
}

fn section_name(name: &'static str, english: bool) -> &'static str {
    if !english {
        return name;
    }
    match name {
        "Thema" => "Subject",
        "Antwort" => "Answer",
        "Zwischenspiel" => "Episode",
        "Einsatz" => "Entry",
        "Engführung" => "Stretto",
        "Schlusseinsatz" => "Final entry",
        "Kadenz" => "Cadence",
        other => other,
    }
}

/// The name of a section with the key of its entry: `Einsatz (Es)`.
fn section_label(fugue: &Fugue, name: &'static str, from: i32, english: bool) -> String {
    let label = section_name(name, english);
    match fugue
        .design
        .entries
        .iter()
        .find(|e| e.at == from && e.group > 0 && name != "Zwischenspiel")
    {
        Some(entry) => format!("{label} ({})", entry.key),
        None => label.to_string(),
    }
}

/// What a fugue looks like to the outside: its files and what was measured. `variant` names it
/// among the versions it is one of (empty: it stands alone).
pub fn describe(
    subject: &Subject,
    fugue: &Fugue,
    profile: &Profile,
    english: bool,
    variant: &str,
) -> Value {
    let piece = &fugue.piece;
    let embedded = tables::embedded();
    let row = stats::measures(piece);
    let measures: Vec<Value> = (0..COUNT)
        .map(|k| {
            json!({
                "name": NAMES[k],
                "value": row[k],
                "low": embedded.bands.low[k],
                "high": embedded.bands.high[k],
                "inside": embedded.bands.contains(k, row[k]),
            })
        })
        .collect();
    let bar = piece.meter.bar();
    let sections: Vec<Value> = fugue
        .design
        .sections
        .iter()
        .map(|s| {
            json!({
                "name": section_label(fugue, s.name, s.from, english),
                "bar": 1 + s.from / bar,
                "tick": s.from,
            })
        })
        .collect();
    let marks: Vec<(i32, String)> = fugue
        .design
        .sections
        .iter()
        .map(|s| (s.from, section_label(fugue, s.name, s.from, english)))
        .collect();
    let grey = |voice: usize, note: &Note| fugue.is_planned(voice, note);
    let title = if subject.title.is_empty() {
        if english {
            "Fugue".to_string()
        } else {
            "Fuge".to_string()
        }
    } else if english {
        format!("Fugue on {}", subject.title)
    } else {
        format!("Fuge über {}", subject.title)
    };
    let given: usize = fugue.planned();
    let report = fugue.report(profile);
    let hits = crate::finder::find(piece, &subject.notes);
    let found = fugue
        .design
        .entries
        .iter()
        .filter(|e| hits.iter().any(|h| h.voice == e.voice && h.at == e.at))
        .count();
    let countersubject = themes::text(&fugue.countersubject);
    json!({
        "ok": true,
        "name": subject.name,
        "title": title,
        "variant": variant,
        "key": subject.key.to_string(),
        "meter": subject.meter.to_string(),
        "voices": piece.voices.len(),
        "bars": piece.end / bar,
        "tones": piece.note_count(),
        "fixed_tones": given,
        "seconds": fugue.seconds,
        "answer": {
            "kind": format!("{:?}", fugue.answer.kind).to_lowercase(),
            "text": themes::text(&fugue.answer.notes),
        },
        "countersubject": countersubject,
        "ending": if fugue.design.stretto.is_some() { "stretto" } else { "pedal" },
        "stretto_bars": fugue.design.stretto.map(|d| d as f64 / bar as f64),
        "entries": { "planned": fugue.design.entries.len(), "found": found },
        "sections": sections,
        "tables_from": fugue.learned_from,
        "counts": {
            "forced": fugue.tally.count.junction,
            "own": fugue.tally.count.own,
            "excused": fugue.tally.count.excused,
            "tries": [fugue.attempts.0, fugue.attempts.1],
        },
        "check": {
            "forbidden": report.forbidden(),
            "cost": report.cost(),
            "preferences": report.preferences(),
            "summary": check::summary(&report, english),
        },
        "measures": measures,
        "hits": embedded.bands.hits(&row),
        "yardstick": embedded.yardstick,
        "playback": {
            "ticks_per_quarter": crate::time::QUARTER,
            "quarters_per_minute": export::tempo(piece),
            // Voice, onset and end in ticks, MIDI key, whether the tone was given.
            "notes": piece.voices.iter().enumerate().flat_map(|(v, voice)| {
                voice.notes.iter().map(move |n| json!([v, n.on, n.off, n.pitch.midi, grey(v, n)]))
            }).collect::<Vec<Value>>(),
        },
        "files": {
            // The name of a version stands in the files: MuseScore shows one title only.
            "musicxml": musicxml(
                piece,
                &if variant.is_empty() { title.clone() } else { format!("{title} · {variant}") },
                "fux-fuge",
                &marks,
            ),
            "lilypond": export::lilypond(piece, &title, variant, &marks, &grey),
            "midi": base64::encode(&export::midi(piece, export::tempo(piece))),
            // The scaffold as chord lines, for `fux-fuge check --chords`.
            "chords": crate::chordline::write(
                &fugue.harmony,
                piece,
                crate::scaffold::step_of(piece.meter),
            ),
        },
    })
}

/// The options a request names.
pub fn options_of(request: &Value) -> Result<Options> {
    // The composer's own profile unless the request names another (`bach`: the prices of the
    // calibration on Bach's rates).
    let mut profile = match request["profile"].as_str() {
        None | Some("") => Profile::fuge(),
        Some(name) => Profile::named(name)?,
    };
    for setting in request["rules"].as_array().into_iter().flatten() {
        profile.set_text(setting.as_str().unwrap_or(""))?;
    }
    let template =
        match request["plan"].as_str() {
            None | Some("") => None,
            Some(name) => Some(Template::named(name).with_context(|| {
                format!("no plan {name:?}; the plans are {:?}", Template::names())
            })?),
        };
    let ending = match request["ending"].as_str() {
        None | Some("" | "auto") => Ending::Auto,
        Some("pedal" | "orgelpunkt") => Ending::Pedal,
        Some("stretto" | "engfuehrung" | "engführung") => Ending::Stretto,
        Some(other) => bail!("the ending is auto, pedal or stretto, not {other:?}"),
    };
    let answer = match request["answer"].as_str() {
        None | Some("") => None,
        Some(text) => Some(themes::parse(text).context("the answer")?),
    };
    let plain = match request["free_voices"].as_str() {
        None | Some("") | Some("figures") => false,
        Some("plain") => true,
        Some(other) => bail!("free_voices is `figures` or `plain`, not {other:?}"),
    };
    let bass = match request["bass"].as_str() {
        None | Some("") | Some("free") => false,
        Some("planned") => true,
        Some(other) => bail!("bass is `free` or `planned`, not {other:?}"),
    };
    Ok(Options {
        voices: request["voices"].as_u64().map_or(3, |v| v as usize),
        seed: request["seed"].as_u64().unwrap_or(1),
        beam: request["beam"]
            .as_u64()
            .map_or(400, |b| b.clamp(20, 4000) as usize),
        attempts: request["attempts"]
            .as_u64()
            .map_or(12, |a| a.clamp(1, 50) as usize),
        profile,
        answer,
        template,
        ending,
        // `free_voices: "plain"` leaves out the sequences in all voices and the figures of the
        // free voices (one setting, in the manner of phase B4); each of the two can be named
        // alone.
        sequences: request["sequences"].as_bool().unwrap_or(!plain),
        figures: request["figures"].as_bool().unwrap_or(!plain),
        bass,
    })
}

fn fugue_answer(request: &Value) -> Result<Value> {
    let subject = subject_of(request)?;
    let options = options_of(request)?;
    let english = request["lang"].as_str() == Some("en");
    let fugue = compose::compose(&subject, &options)?;
    let variant = request["variant"].as_str().unwrap_or("");
    Ok(describe(
        &subject,
        &fugue,
        &options.profile,
        english,
        variant,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_are_listed() {
        let answer: Value = serde_json::from_str(&answer_json(r#"{"command": "themes"}"#)).unwrap();
        assert_eq!(answer["ok"], true);
        assert_eq!(answer["themes"].as_array().unwrap().len(), BUILTIN.len());
        assert_eq!(answer["themes"][3]["name"], "bwv1080");
        assert_eq!(answer["themes"][3]["period_bars"], 4.0);
        for theme in &BUILTIN {
            assert!(theme.subject().is_ok(), "{}", theme.name);
        }
    }

    #[test]
    fn errors_are_answers() {
        for request in [
            "{",
            r#"{"command": "nothing"}"#,
            r#"{"command": "fugue"}"#,
            r#"{"command": "fugue", "subject": "c d", "key": "C"}"#,
            r#"{"command": "fugue", "theme": "bwv1080", "voices": 7}"#,
            r#"{"command": "fugue", "theme": "bwv1080", "plan": "nothing"}"#,
            r#"{"command": "fugue", "theme": "bwv1080", "ending": "loud"}"#,
            r#"{"command": "fugue", "theme": "bwv1080", "free_voices": "ornate"}"#,
        ] {
            let answer: Value = serde_json::from_str(&answer_json(request)).unwrap();
            assert_eq!(answer["ok"], false, "{request}");
            assert!(answer["error"].as_str().is_some_and(|e| !e.is_empty()));
        }
    }

    #[test]
    fn a_version_carries_its_name_and_the_sections_stand_in_the_files() {
        let request = r#"{"command": "fugue", "theme": "bwv1080", "beam": 60,
            "variant": "Fassung b: Bass: geplant"}"#;
        let answer: Value = serde_json::from_str(&answer_json(request)).unwrap();
        assert_eq!(answer["ok"], true, "{}", answer["error"]);
        assert_eq!(answer["variant"], "Fassung b: Bass: geplant");
        let (xml, ly) = (
            answer["files"]["musicxml"].as_str().unwrap(),
            answer["files"]["lilypond"].as_str().unwrap(),
        );
        assert!(xml.contains("d-Moll · Fassung b: Bass: geplant</work-title>"));
        assert!(ly.contains("subtitle = \"Fassung b: Bass: geplant\""));
        // The words of the sections stand over the notes in both.
        assert!(xml.contains(">Antwort</words>"));
        assert!(ly.contains("^\\markup \\italic \"Thema\""));
        assert!(ly.contains("^\\markup \\italic \"Antwort\""));
        // Alone, a fugue has its title and no name of a version.
        let alone: Value = serde_json::from_str(&answer_json(
            r#"{"command": "fugue", "theme": "bwv1080", "beam": 60}"#,
        ))
        .unwrap();
        assert_eq!(alone["variant"], "");
        assert!(
            alone["files"]["musicxml"]
                .as_str()
                .unwrap()
                .contains("d-Moll</work-title>")
        );
    }

    #[test]
    fn the_free_voices_of_a_request() {
        let options = |text: &str| options_of(&serde_json::from_str(text).unwrap()).unwrap();
        let default = options(r#"{"command": "fugue"}"#);
        assert!(default.sequences && default.figures);
        let figures = options(r#"{"free_voices": "figures"}"#);
        assert!(figures.sequences && figures.figures);
        let plain = options(r#"{"free_voices": "plain"}"#);
        assert!(!plain.sequences && !plain.figures);
        // Each of the two can be named alone, also against the word.
        let mixed = options(r#"{"free_voices": "plain", "sequences": true}"#);
        assert!(mixed.sequences && !mixed.figures);
    }

    #[test]
    fn the_profile_of_a_request() {
        use crate::rules::{Level, Rule};
        let options = |text: &str| options_of(&serde_json::from_str(text).unwrap());
        // The composer's own prices unless the request names the calibrated ones.
        let default = options(r#"{"command": "fugue"}"#).unwrap();
        assert_eq!(default.profile.name, "fuge");
        assert_eq!(default.profile.level(Rule::HarStruck), Level::P4);
        let bach = options(r#"{"profile": "bach"}"#).unwrap();
        assert_eq!(bach.profile.level(Rule::HarStruck), Level::P2);
        // Single rules come on top of either.
        let both = options(r#"{"profile": "bach", "rules": ["HAR-STRUCK=P3"]}"#).unwrap();
        assert_eq!(both.profile.level(Rule::HarStruck), Level::P3);
        assert!(options(r#"{"profile": "no-such-profile"}"#).is_err());
        // The bass is found by the search unless the request asks for a planned one.
        assert!(!default.bass);
        assert!(options(r#"{"bass": "planned"}"#).unwrap().bass);
        assert!(!options(r#"{"bass": "free"}"#).unwrap().bass);
        assert!(options(r#"{"bass": "walking"}"#).is_err());
    }

    #[test]
    fn a_fugue_on_a_typed_subject() {
        let request = r#"{"command": "fugue", "subject": "d'2 a' | f' d' | cis' d'4 e' | f'2~ f'8 g' f' e' | d'4",
            "key": "d", "meter": "2/2", "seed": 1, "lang": "en"}"#;
        let answer: Value = serde_json::from_str(&answer_json(request)).unwrap();
        assert_eq!(answer["ok"], true, "{}", answer["error"]);
        assert_eq!(answer["voices"], 3);
        let bars = answer["bars"].as_u64().unwrap();
        assert!((30..=80).contains(&bars), "{bars} bars");
        assert_eq!(answer["answer"]["kind"], "tonal");
        assert_eq!(answer["sections"][1]["name"], "Answer");
        assert_eq!(answer["counts"]["forced"], 0);
        assert_eq!(answer["entries"]["planned"], answer["entries"]["found"]);
        assert!(
            answer["check"]["summary"]
                .as_str()
                .unwrap()
                .contains("prohibition")
        );
        assert!(
            answer["files"]["musicxml"]
                .as_str()
                .unwrap()
                .contains("<score-partwise")
        );
        assert!(
            answer["files"]["lilypond"]
                .as_str()
                .unwrap()
                .contains("\\key d \\minor")
        );
        assert!(
            answer["files"]["midi"]
                .as_str()
                .unwrap()
                .starts_with("TVRoZA")
        );
        assert_eq!(answer["measures"].as_array().unwrap().len(), COUNT);
        let notes = answer["playback"]["notes"].as_array().unwrap();
        assert_eq!(notes.len() as u64, answer["tones"].as_u64().unwrap());
        assert_eq!(notes[0].as_array().unwrap().len(), 5);
        // Four voices after the plan of a fugue of Bach's, with a pedal point at the end.
        let four = r#"{"command": "fugue", "theme": "wtk1-02", "voices": 4, "plan": "wtk1-02",
            "ending": "pedal", "beam": 60}"#;
        let four: Value = serde_json::from_str(&answer_json(four)).unwrap();
        assert_eq!(four["ok"], true, "{}", four["error"]);
        assert_eq!(four["voices"], 4);
        assert_eq!(four["ending"], "pedal");
        // The same request gives the same fugue.
        let again: Value = serde_json::from_str(&answer_json(request)).unwrap();
        assert_eq!(again["files"]["midi"], answer["files"]["midi"]);
    }
}
