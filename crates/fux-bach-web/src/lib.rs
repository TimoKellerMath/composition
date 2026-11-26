//! The web version of `fux-fuge`: a WebAssembly module without imports that answers the JSON
//! requests of the page in `web-fuge/` as `fux_bach::api` does: with the subjects and the plans
//! that come with the program, and with a fugue for two to four voices on one of the subjects or
//! on typed tones, as MusicXML (which the page engraves), LilyPond and MIDI, with the tones to
//! play, the check and the style measures.
//!
//! Build with `tools/build_web_fuge.sh`; the module runs in a web worker, so the page stays
//! usable while a fugue is set. It has no clock: the page measures the time.

mod ffi;

pub use fux_bach::api::answer_json;

#[cfg(test)]
mod tests {
    use serde_json::Value;

    fn ask(request: &str) -> Value {
        serde_json::from_str(&super::answer_json(request)).expect("an answer is JSON")
    }

    #[test]
    fn the_themes_have_what_the_page_shows() {
        assert_eq!(ask(r#"{"command": "version"}"#)["ok"], true);
        let themes = ask(r#"{"command": "themes"}"#);
        let list = themes["themes"].as_array().expect("themes");
        assert!(list.iter().any(|theme| theme["name"] == "bwv1080"));
        // A theme fills the fields of the form by these names.
        for theme in list {
            for field in ["name", "title", "key", "meter", "subject"] {
                assert!(theme[field].is_string(), "{field} of {theme}");
            }
            assert!(theme["period_bars"].is_number(), "{theme}");
        }
        // The plans are the choices of a field of the form.
        let plans = themes["plans"].as_array().expect("plans");
        assert!(!plans.is_empty() && plans.iter().all(Value::is_string));
    }

    #[test]
    fn a_fugue_has_what_the_page_shows() {
        let fugue = ask(
            r#"{"command": "fugue", "theme": "bwv1080", "voices": 4, "ending": "pedal",
                "plan": "wtk1-02", "seed": 1, "beam": 100}"#,
        );
        assert_eq!(fugue["ok"], true, "{}", fugue["error"]);
        assert_eq!(fugue["voices"], 4);
        assert_eq!(fugue["ending"], "pedal");
        assert!(fugue["stretto_bars"].is_null());
        for number in ["planned", "found"] {
            assert!(fugue["entries"][number].is_u64(), "{number}");
        }
        assert!(fugue["check"]["summary"].is_string());
        assert!(fugue["check"]["forbidden"].is_u64());
        for number in ["own", "forced"] {
            assert!(fugue["counts"][number].is_u64(), "{number}");
        }
        for file in ["musicxml", "lilypond", "midi", "chords"] {
            assert!(fugue["files"][file].is_string(), "{file}");
        }
        let playback = &fugue["playback"];
        for number in ["ticks_per_quarter", "quarters_per_minute"] {
            assert!(
                playback[number].as_f64().is_some_and(|n| n > 0.0),
                "{number}"
            );
        }
        let notes = playback["notes"].as_array().expect("notes");
        assert!(!notes.is_empty());
        for note in notes {
            let [voice, on, off, key, given] = &note.as_array().expect("a note")[..] else {
                panic!("{note}");
            };
            assert!(
                voice.is_u64() && key.is_u64() && given.is_boolean(),
                "{note}"
            );
            assert!(on.as_i64() < off.as_i64(), "{note}");
        }
        let sections = fugue["sections"].as_array().expect("sections");
        assert!(!sections.is_empty());
        for section in sections {
            assert!(
                section["name"].is_string() && section["bar"].is_u64() && section["tick"].is_i64(),
                "{section}"
            );
        }
        for measure in fugue["measures"].as_array().expect("measures") {
            assert!(
                measure["name"].is_string() && measure["inside"].is_boolean(),
                "{measure}"
            );
            for number in ["value", "low", "high"] {
                assert!(measure[number].is_number(), "{number} of {measure}");
            }
        }
        assert!(fugue["hits"].is_number() && fugue["yardstick"].is_number());
    }

    #[test]
    fn an_error_is_an_answer() {
        for request in [
            r#"{"command": "fugue", "subject": "c' d' x", "key": "C"}"#,
            r#"{"command": "fugue", "theme": "bwv1080", "ending": "stretto"}"#,
            r#"{"command": "fugue", "theme": "bwv1080", "answer": "a' d''"}"#,
        ] {
            let answer = ask(request);
            assert_eq!(answer["ok"], false, "{request}");
            assert!(
                answer["error"]
                    .as_str()
                    .is_some_and(|error| !error.is_empty()),
                "{request}"
            );
        }
    }
}
