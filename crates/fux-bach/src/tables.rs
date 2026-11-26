//! The tables the program draws from, as numbers: per fugue of the corpus its rhythm patterns
//! and intervals with the profile of its note values, the successions of chords of the analysed
//! pieces, and the bands of the style measures. They are counted once by `bach-probe tables`
//! and kept in `data/tables.json`, so that the program needs no corpus at run time (and none of
//! the encodings it was counted on).

use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::cells;
use crate::harmonize::Transitions;
use crate::melody;
use crate::rhythm::{self, Pattern};
use crate::stats::{Bands, COUNT};
use crate::time::{Meter, Tick};

/// What one fugue of the corpus gives.
#[derive(Clone, Debug)]
pub struct Source {
    pub name: String,
    pub meter: Meter,
    /// The pulse of its rhythm table in ticks.
    pub pulse: Tick,
    /// Shares of short, middle and long note values.
    pub profile: [f64; 3],
    pub rhythm: rhythm::Table,
    pub intervals: melody::Table,
    /// The pulse of its cells (the rhythm of the machine, on ticks).
    pub unit: Tick,
    /// Shares of short, middle and long note values in its subject.
    pub subject_profile: [f64; 3],
    pub cells: cells::Table,
}

#[derive(Clone, Debug)]
pub struct Tables {
    pub sources: Vec<Source>,
    pub transitions: Transitions,
    pub bands: Bands,
    /// The tenth percentile of the hits of the 48 fugues against the bands of the others.
    pub yardstick: f64,
}

fn numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default()
}

fn fixed<const N: usize>(value: &Value) -> [f64; N] {
    let list = numbers(value);
    let mut out = [0.0; N];
    for (slot, number) in out.iter_mut().zip(list) {
        *slot = number;
    }
    out
}

impl Tables {
    pub fn to_json(&self) -> Value {
        let sources: Vec<Value> = self
            .sources
            .iter()
            .map(|s| {
                let rhythm: Vec<Value> = s
                    .rhythm
                    .counts()
                    .into_iter()
                    .map(|(before, beside, now, count)| json!([before, beside, now, count]))
                    .collect();
                let cell_rows: Vec<Value> = s
                    .cells
                    .counts()
                    .into_iter()
                    .map(|(before, beside, now, count)| {
                        json!([
                            cells::text(before, s.unit),
                            beside,
                            cells::text(now, s.unit),
                            count
                        ])
                    })
                    .collect();
                json!({
                    "name": s.name,
                    "meter": s.meter.to_string(),
                    "pulse": s.pulse,
                    "profile": s.profile,
                    "slots": s.rhythm.slots,
                    "rhythm": rhythm,
                    "intervals": s.intervals.flat(),
                    "unit": s.unit,
                    "subject_profile": s.subject_profile,
                    "cells": cell_rows,
                })
            })
            .collect();
        let (pairs, firsts) = self.transitions.rows();
        json!({
            "note": "Counted by `bach-probe tables` on the corpus in references/datasets/bach-fugen; numbers only, none of the encodings.",
            "fugues": sources,
            "transitions": { "pairs": pairs, "firsts": firsts },
            "bands": {
                "low": self.bands.low,
                "high": self.bands.high,
                "median": self.bands.median,
                "yardstick": self.yardstick,
            },
        })
    }

    pub fn from_json(value: &Value) -> Result<Tables> {
        let mut sources = Vec::new();
        for fugue in value["fugues"].as_array().context("fugues")? {
            let slots = fugue["slots"].as_u64().context("slots")? as usize;
            let counts: Vec<(Pattern, u8, Pattern, f64)> = fugue["rhythm"]
                .as_array()
                .context("rhythm")?
                .iter()
                .map(|row| {
                    let r = numbers(row);
                    (
                        r.first().copied().unwrap_or(0.0) as Pattern,
                        r.get(1).copied().unwrap_or(0.0) as u8,
                        r.get(2).copied().unwrap_or(0.0) as Pattern,
                        r.get(3).copied().unwrap_or(0.0),
                    )
                })
                .collect();
            let meter = Meter::parse(fugue["meter"].as_str().context("meter")?).context("meter")?;
            let unit = fugue["unit"].as_i64().map_or(meter.unit(), |u| u as Tick);
            let cell_counts: Vec<(cells::Cell, u8, cells::Cell, f64)> = fugue["cells"]
                .as_array()
                .map(|rows| {
                    rows.iter()
                        .filter_map(|row| {
                            Some((
                                cells::parse(row[0].as_str()?)?,
                                row[1].as_u64()? as u8,
                                cells::parse(row[2].as_str()?)?,
                                row[3].as_f64()?,
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default();
            sources.push(Source {
                name: fugue["name"].as_str().context("name")?.to_string(),
                meter,
                pulse: fugue["pulse"].as_i64().context("pulse")? as Tick,
                profile: fixed(&fugue["profile"]),
                rhythm: rhythm::Table::from_counts(slots, &counts),
                intervals: melody::Table::from_flat(&numbers(&fugue["intervals"])),
                unit,
                subject_profile: fixed(&fugue["subject_profile"]),
                cells: cells::Table::from_counts(unit, &cell_counts),
            });
        }
        let rows = |value: &Value| -> Vec<Vec<f64>> {
            value
                .as_array()
                .map(|a| a.iter().map(numbers).collect())
                .unwrap_or_default()
        };
        let pairs: Vec<[f64; 8]> = rows(&value["transitions"]["pairs"])
            .iter()
            .map(|r| fixed(&json!(r)))
            .collect();
        let firsts: Vec<[f64; 5]> = rows(&value["transitions"]["firsts"])
            .iter()
            .map(|r| fixed(&json!(r)))
            .collect();
        Ok(Tables {
            sources,
            transitions: Transitions::from_rows(&pairs, &firsts),
            bands: Bands {
                low: fixed::<COUNT>(&value["bands"]["low"]),
                high: fixed::<COUNT>(&value["bands"]["high"]),
                median: fixed::<COUNT>(&value["bands"]["median"]),
            },
            yardstick: value["bands"]["yardstick"].as_f64().unwrap_or(0.0),
        })
    }

    /// The tables of patterns and intervals from the `nearest` fugues with this pulse whose note
    /// values are closest to `profile`, without the fugue named `except`; with their names.
    pub fn nearest(
        &self,
        pulse: Tick,
        profile: [f64; 3],
        except: &str,
        nearest: usize,
    ) -> (rhythm::Table, melody::Table, Vec<String>) {
        let mut ranked: Vec<(f64, &Source)> = self
            .sources
            .iter()
            .filter(|s| s.pulse == pulse && s.name != except)
            .map(|s| {
                let distance: f64 = s
                    .profile
                    .iter()
                    .zip(profile)
                    .map(|(a, b)| (a - b).abs())
                    .sum();
                (distance, s)
            })
            .collect();
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
        let slots = (pulse / crate::time::SIXTEENTH) as usize;
        let mut table = rhythm::Table::new(slots);
        let mut intervals = melody::Table::default();
        let mut names = Vec::new();
        for (_, source) in ranked.into_iter().take(nearest) {
            table.add(&source.rhythm);
            intervals.add(&source.intervals);
            names.push(source.name.clone());
        }
        (table, intervals, names)
    }
}

impl Tables {
    /// For the machine: the cells and intervals of the `nearest` fugues with this unit whose
    /// subjects are closest to `profile` in their note values, without the fugue named `except`.
    pub fn nearest_cells(
        &self,
        unit: Tick,
        profile: [f64; 3],
        except: &str,
        nearest: usize,
    ) -> (cells::Table, melody::Table, Vec<String>) {
        let mut ranked: Vec<(f64, &Source)> = self
            .sources
            .iter()
            .filter(|s| s.unit == unit && s.name != except)
            .map(|s| {
                let distance: f64 = s
                    .subject_profile
                    .iter()
                    .zip(profile)
                    .map(|(a, b)| (a - b).abs())
                    .sum();
                (distance, s)
            })
            .collect();
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut table = cells::Table::new(unit);
        let mut intervals = melody::Table::default();
        let mut names = Vec::new();
        for (_, source) in ranked.into_iter().take(nearest) {
            table.add(&source.cells);
            intervals.add(&source.intervals);
            names.push(source.name.clone());
        }
        (table, intervals, names)
    }
}

/// The tables that come with the program.
pub fn embedded() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let value: Value = serde_json::from_str(include_str!("../data/tables.json"))
            .expect("data/tables.json is JSON");
        Tables::from_json(&value).expect("data/tables.json holds the tables")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables_come_with_the_program() {
        let tables = embedded();
        assert_eq!(tables.sources.len(), 48);
        assert!(tables.transitions.size() > 1000);
        assert!(tables.yardstick >= 15.0);
        let (rhythm, intervals, names) = tables.nearest(24, [0.5, 0.4, 0.1], "wtc1f02", 8);
        assert_eq!(names.len(), 8);
        assert!(!names.contains(&"wtc1f02".to_string()));
        assert!(rhythm.size() > 1000);
        assert!(intervals.shares()[1] > 0.5);
        // The cells of the machine: by the unit of the meter, chosen by the subject.
        let (cells, _, from) = tables.nearest_cells(24, [0.5, 0.4, 0.1], "wtc1f02", 8);
        assert_eq!(from.len(), 8);
        assert!(!from.contains(&"wtc1f02".to_string()));
        assert!(cells.size() > 1000);
        let (compound, _, from) = tables.nearest_cells(36, [0.5, 0.4, 0.1], "", 8);
        assert!(from.len() >= 5 && compound.size() > 500);
        // Writing and reading gives the same numbers.
        let again = Tables::from_json(&tables.to_json()).unwrap();
        assert_eq!(again.to_json(), tables.to_json());
    }
}
