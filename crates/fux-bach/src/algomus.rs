//! The Algomus labels of the fugues of WTC I (`.dez`, JSON): subjects, countersubjects, cadences
//! and pedals with start and duration in quarter notes; `staff` counts the voices from the top.

use anyhow::{Context, Result};

use crate::score::{Note, Piece};
use crate::time::{QUARTER, Tick};

#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    /// `S`, `CS1`, `CS2`, `Cadence`, `Pedal` and others.
    pub kind: String,
    pub tag: String,
    pub start: Tick,
    pub end: Tick,
    /// The voice from the top, if the label belongs to one.
    pub voice: Option<usize>,
}

pub fn read(text: &str) -> Result<Vec<Label>> {
    let json: serde_json::Value = serde_json::from_str(text)?;
    let labels = json
        .get("labels")
        .and_then(|l| l.as_array())
        .context("no labels")?;
    let ticks = |value: Option<&serde_json::Value>| -> Tick {
        (value.and_then(|v| v.as_f64()).unwrap_or(0.0) * QUARTER as f64).round() as Tick
    };
    let text_of = |value: Option<&serde_json::Value>| -> String {
        value.and_then(|v| v.as_str()).unwrap_or("").to_string()
    };
    let mut out = Vec::new();
    for label in labels {
        let start = ticks(label.get("start"));
        let voice = label
            .get("staff")
            .and_then(|s| {
                s.as_str()
                    .map(str::to_string)
                    .or_else(|| s.as_i64().map(|n| n.to_string()))
            })
            .and_then(|s| s.parse::<usize>().ok())
            .and_then(|n| n.checked_sub(1));
        out.push(Label {
            kind: text_of(label.get("type")),
            tag: text_of(label.get("tag")),
            start,
            end: start + ticks(label.get("duration")),
            voice,
        });
    }
    out.sort_by_key(|l| (l.start, l.voice));
    Ok(out)
}

/// The file of a fugue of WTC I: `02-bwv847-ref.dez` for number 2.
pub fn file_name(number: usize) -> String {
    format!("{number:02}-bwv{}-ref.dez", 845 + number)
}

pub fn read_for(data: &std::path::Path, number: usize) -> Result<Vec<Label>> {
    let path = data.join("algomus").join(file_name(number));
    let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    read(&text)
}

impl Label {
    /// The notes of the label's voice that begin inside it.
    pub fn notes<'a>(&self, piece: &'a Piece) -> &'a [Note] {
        match self.voice.and_then(|v| piece.voices.get(v)) {
            Some(voice) => voice.range(self.start, self.end),
            None => &[],
        }
    }

    pub fn is_subject(&self) -> bool {
        self.kind == "S"
    }

    pub fn is_countersubject(&self) -> bool {
        self.kind.starts_with("CS")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        let text = r#"{"labels": [
            {"type": "S", "start": 8.5, "duration": 7.5, "staff": "1", "tag": "S"},
            {"type": "CS1", "start": 0.25, "duration": 1.75, "staff": "2", "tag": "CS1a"},
            {"type": "Cadence", "start": 16, "tag": "PAC"}]}"#;
        let labels = read(text).unwrap();
        assert_eq!(labels.len(), 3);
        assert_eq!(
            (
                labels[0].kind.as_str(),
                labels[0].start,
                labels[0].end,
                labels[0].voice
            ),
            ("CS1", 6, 48, Some(1))
        );
        assert_eq!(
            (labels[1].start, labels[1].end, labels[1].voice),
            (204, 384, Some(0))
        );
        assert!(labels[1].is_subject() && labels[0].is_countersubject());
        assert_eq!(labels[2].voice, None);
        assert_eq!(file_name(2), "02-bwv847-ref.dez");
    }
}
