//! T7: a short three-voice fugue on a given subject.
//!
//! The plan is fixed: exposition (subject in the middle voice, answer above it against the
//! countersubject, a bar of link, subject in the bass against the countersubject above), an
//! episode to the relative key, an entry there, an episode back, a last entry in the tonic and
//! a cadence. The answer comes from the rule of `answer`, the chords under an entry from
//! `harmonize`, the chords between entries are written out here. The countersubject is searched
//! once, in double counterpoint at the octave against the answer, and then moved with every
//! entry; everything else is searched over the chords.

use anyhow::{Result, anyhow};

use crate::answer::{self, Answer};
use crate::harmonize::{Ends, Rel, Transitions, harmonize};
use crate::key::{Harmony, Key, Kind};
use crate::layout::{Layout, Options, Setting, VoiceLayout, set};
use crate::melody;
use crate::pitch::{Iv, Lof, Pitch};
use crate::rhythm;
use crate::score::Note;
use crate::themes::Subject;
use crate::time::{Meter, Tick};

/// Ranges of the three voices: top, middle, bass.
const RANGES: [(Pitch, Pitch); 3] = [
    (Pitch::new(5, 0, 3), Pitch::new(0, 0, 6)),
    (Pitch::new(1, 0, 3), Pitch::new(2, 0, 5)),
    (Pitch::new(0, 0, 2), Pitch::new(2, 0, 4)),
];

pub struct Tables<'a> {
    pub transitions: &'a Transitions,
    pub rhythm: &'a rhythm::Table,
    pub intervals: &'a melody::Table,
}

/// A section of the plan with its first tick.
#[derive(Clone, Debug)]
pub struct Section {
    pub name: &'static str,
    pub from: Tick,
}

pub struct Fugue {
    pub setting: Setting,
    pub layout: Layout,
    pub harmony: Harmony,
    pub answer: Answer,
    /// The countersubject as found against the answer, from the answer's section on.
    pub countersubject: Vec<Note>,
    pub sections: Vec<Section>,
    /// Seconds for both searches.
    pub seconds: f64,
    /// Attempts of the search for the countersubject.
    pub first_attempts: usize,
}

/// The tones later by `at` ticks and higher by `iv`.
fn moved(notes: &[Note], at: Tick, iv: Iv) -> Vec<Note> {
    notes
        .iter()
        .map(|n| Note {
            on: n.on + at,
            off: n.off + at,
            pitch: n.pitch.up(iv),
        })
        .collect()
}

/// The tones in the octave in which they fit the range best.
fn fitted(notes: &[Note], range: (Pitch, Pitch)) -> Vec<Note> {
    let (low, high) = range;
    let centre = (low.midi as i32 + high.midi as i32) / 2;
    let best = (-4i8..=4)
        .min_by_key(|k| {
            let mut over = 0i32;
            let mut sum = 0i32;
            for n in notes {
                let midi = n.pitch.midi as i32 + 12 * *k as i32;
                over += (low.midi as i32 - midi).max(0) + (midi - high.midi as i32).max(0);
                sum += midi;
            }
            let mean = sum / notes.len().max(1) as i32;
            (over, (mean - centre).abs())
        })
        .unwrap_or(0);
    moved(notes, 0, Iv::new(12 * best as i16, 7 * best as i16))
}

/// The tones of a line in `from` on the same degrees of `to`. Between the modes the sixth and
/// seventh degrees follow the new scale: the raised degrees of minor become the plain ones of
/// major; towards minor the seventh is raised next to the tonic and the sixth before a raised
/// seventh.
pub fn in_key(notes: &[Note], from: Key, to: Key) -> Vec<Note> {
    let shift = (to.tonic.step() - from.tonic.step()).rem_euclid(7);
    let degrees: Vec<(usize, i8)> = notes
        .iter()
        .map(|n| {
            let class = n.pitch.lof();
            let degree = from.degree_of(class);
            (degree, (class.0 - from.degree(degree).0) / 7)
        })
        .collect();
    let mut alters: Vec<i8> = degrees
        .iter()
        .map(|(degree, alter)| match (from.minor, to.minor) {
            (true, false) if (*degree == 5 || *degree == 6) && *alter == 1 => 0,
            _ => *alter,
        })
        .collect();
    if !from.minor && to.minor {
        let count = degrees.len();
        for i in 0..count {
            let beside_tonic =
                (i > 0 && degrees[i - 1].0 == 0) || (i + 1 < count && degrees[i + 1].0 == 0);
            if degrees[i] == (6, 0) && beside_tonic {
                alters[i] = 1;
            }
        }
        for i in 0..count.saturating_sub(1) {
            if degrees[i] == (5, 0) && degrees[i + 1].0 == 6 && alters[i + 1] == 1 {
                alters[i] = 1;
            }
        }
    }
    notes
        .iter()
        .zip(degrees.iter().zip(alters))
        .map(|(n, ((degree, _), alter))| {
            let class = Lof(to.degree(*degree).0 + 7 * alter);
            let dia = n.pitch.dia + shift as i16;
            Note {
                on: n.on,
                off: n.off,
                pitch: Pitch::new(class.step(), class.alter(), dia.div_euclid(7) as i8),
            }
        })
        .collect()
}

/// The countersubject for the subject, from the one found against the answer: every tone moves
/// down by what the answer's tone above it went up. Only the tones under the answer's last
/// region are kept (the part after the change between fifth and fourth); under the head, where
/// the answer bends, the line is left to the search, as a composer adjusts it by hand.
fn under_subject(countersubject: &[Note], answer: &Answer) -> Vec<Note> {
    let tail = answer.fourth.last().copied().unwrap_or(false);
    let iv = if tail { Iv::FOURTH } else { Iv::FIFTH };
    // The first tone of the answer from which its region no longer changes.
    let settled = answer
        .fourth
        .iter()
        .rposition(|f| *f != tail)
        .map_or(0, |i| i + 1);
    let from = answer.notes.get(settled).map_or(Tick::MAX, |n| n.on);
    countersubject
        .iter()
        .filter(|n| n.on >= from)
        .map(|n| Note {
            on: n.on,
            off: n.off,
            pitch: n.pitch.down(iv),
        })
        .collect()
}

fn span(notes: &[Note]) -> Option<(Tick, Tick)> {
    Some((notes.first()?.on, notes.last()?.off))
}

/// A voice with its fixed groups of tones; it is silent until the first of them begins, or
/// throughout if it has none.
fn voice(range: (Pitch, Pitch), groups: Vec<Vec<Note>>) -> VoiceLayout {
    let spans = groups.iter().filter_map(|g| span(g)).collect();
    let mut fixed: Vec<Note> = groups.into_iter().flatten().collect();
    fixed.sort_by_key(|n| n.on);
    let silent_until = fixed.first().map_or(Tick::MAX / 2, |n| n.on);
    VoiceLayout {
        low: range.0,
        high: range.1,
        fixed,
        spans,
        silent: vec![(Tick::MIN / 2, silent_until)],
    }
}

/// Where the second chord of a bar begins.
fn half(meter: Meter) -> Tick {
    let pulses = meter.bar() / meter.pulse();
    meter.pulse() * ((pulses + 1) / 2)
}

/// A fugue on the subject with the tables that come with the program: rhythm and intervals from
/// the eight fugues nearest to the subject's note values (without the fugue it comes from). Also
/// returns the names of those fugues.
pub fn compose_embedded(subject: &Subject, options: &Options) -> Result<(Fugue, Vec<String>)> {
    let embedded = crate::tables::embedded();
    let profile = rhythm::value_profile(subject.notes.iter());
    let except = subject.from.clone().unwrap_or_default();
    let (table, intervals, learned_from) =
        embedded.nearest(subject.meter.pulse(), profile, &except, 8);
    let tables = Tables {
        transitions: &embedded.transitions,
        rhythm: &table,
        intervals: &intervals,
    };
    Ok((compose(subject, &tables, options)?, learned_from))
}

pub fn compose(subject: &Subject, tables: &Tables, options: &Options) -> Result<Fugue> {
    let begun = crate::clock::Stopwatch::start();
    let key = subject.key;
    let meter = subject.meter;
    let bar = meter.bar();
    let period = subject.period;
    let dominant = key.dominant();
    let relative = key.relative();
    let answer = answer::answer(&subject.notes, key);
    let modulating = answer::modulates(&subject.notes, key);

    // The sections.
    let e1 = 0;
    let e2 = period;
    let link = 2 * period;
    let e3 = link + bar;
    let ep1 = e3 + period;
    let m1 = ep1 + 2 * bar;
    let ep2 = m1 + period;
    let f1 = ep2 + 2 * bar;
    let cadence = f1 + period;
    let end = cadence + 2 * bar;
    let sections = vec![
        Section {
            name: "Thema",
            from: e1,
        },
        Section {
            name: "Antwort",
            from: e2,
        },
        Section {
            name: "Überleitung",
            from: link,
        },
        Section {
            name: "Thema im Bass",
            from: e3,
        },
        Section {
            name: "Zwischenspiel 1",
            from: ep1,
        },
        Section {
            name: "Einsatz in der Paralleltonart",
            from: m1,
        },
        Section {
            name: "Zwischenspiel 2",
            from: ep2,
        },
        Section {
            name: "Schlusseinsatz",
            from: f1,
        },
        Section {
            name: "Kadenz",
            from: cadence,
        },
    ];

    // The entries.
    let theme = &subject.notes;
    let theme_v1 = fitted(theme, RANGES[1]);
    let answer_v0 = fitted(&moved(&answer.notes, e2, Iv::UNISON), RANGES[0]);
    let theme_v2 = fitted(&moved(theme, e3, Iv::UNISON), RANGES[2]);
    let theme_relative = in_key(theme, key, relative);
    let middle_v0 = fitted(&moved(&theme_relative, m1, Iv::UNISON), RANGES[0]);
    let last_v0 = fitted(&moved(theme, f1, Iv::UNISON), RANGES[0]);

    // The chords.
    let chord = |rel: Rel, k: Key| rel.in_key(k);
    let triad = |degree: u8, k: Key| {
        let kind = match (k.minor, degree) {
            (false, 0 | 3 | 4) | (true, 2 | 4 | 5) => Kind::Maj,
            (false, 6) | (true, 1) => Kind::Dim,
            _ => Kind::Min,
        };
        chord(Rel::new(degree, 0, kind), k)
    };
    let seventh = |k: Key| chord(Rel::new(4, 0, Kind::Dom7), k);
    let second = half(meter);
    // The harmony of the whole piece; `with` are the countersubjects beside the entries after
    // the exposition's first two (above the bass entry, beside the middle entry, below the last).
    let build = |with: Option<[&[Note]; 3]>| -> Harmony {
        let mut harmony = Harmony::default();
        let mut put = |tick: Tick, c, k: Key| harmony.push(tick, c, k);
        let tonic_last = |notes: &[Note], k: Key| {
            notes
                .last()
                .is_some_and(|n| matches!(k.degree_of(n.pitch.lof()), 0 | 2))
        };
        let of_theme = harmonize(
            tables.transitions,
            &[theme],
            key,
            meter,
            0,
            0,
            period,
            Ends {
                tonic_first: true,
                tonic_last: !modulating && tonic_last(theme, key),
            },
        );
        for (tick, c) in &of_theme {
            put(*tick, *c, key);
        }
        // The answer stands in the dominant key; one that leads back is read in the tonic key.
        let answer_key = if modulating { key } else { dominant };
        let of_answer = harmonize(
            tables.transitions,
            &[&moved(&answer.notes, e2, Iv::UNISON)],
            answer_key,
            meter,
            0,
            e2,
            link,
            Ends {
                tonic_first: false,
                tonic_last: tonic_last(&answer.notes, answer_key),
            },
        );
        for (tick, c) in &of_answer {
            put(*tick, *c, answer_key);
        }
        put(link, triad(4, key), key);
        put(link + second, seventh(key), key);
        match with {
            // With its countersubject an entry is harmonised anew, from both lines.
            Some([above, _, _]) => {
                let ends = Ends {
                    tonic_first: true,
                    tonic_last: !modulating && tonic_last(theme, key),
                };
                for (tick, c) in harmonize(
                    tables.transitions,
                    &[&theme_v2, above],
                    key,
                    meter,
                    0,
                    e3,
                    ep1,
                    ends,
                ) {
                    put(tick, c, key);
                }
            }
            None => {
                for (tick, c) in &of_theme {
                    put(e3 + tick, *c, key);
                }
            }
        }
        // To the relative key.
        put(ep1, triad(0, key), key);
        put(ep1 + second, triad(3, key), key);
        if key.minor {
            put(ep1 + bar, triad(1, relative), relative);
        } else {
            put(ep1 + bar, triad(1, key), key);
        }
        put(ep1 + bar + second, seventh(relative), relative);
        let middle_lines: Vec<&[Note]> = match with {
            Some([_, beside, _]) => vec![&middle_v0, beside],
            None => vec![&middle_v0],
        };
        let of_middle = harmonize(
            tables.transitions,
            &middle_lines,
            relative,
            meter,
            0,
            m1,
            ep2,
            Ends {
                tonic_first: true,
                tonic_last: !modulating && tonic_last(&theme_relative, relative),
            },
        );
        for (tick, c) in &of_middle {
            put(*tick, *c, relative);
        }
        // Back to the tonic.
        put(ep2, triad(0, relative), relative);
        if key.minor {
            put(ep2 + second, triad(5, relative), relative);
            put(ep2 + bar, triad(3, key), key);
        } else {
            put(ep2 + second, triad(3, relative), relative);
            put(ep2 + bar, triad(4, key), key);
        }
        put(ep2 + bar + second, seventh(key), key);
        match with {
            Some([_, _, below]) => {
                let ends = Ends {
                    tonic_first: true,
                    tonic_last: !modulating && tonic_last(theme, key),
                };
                for (tick, c) in harmonize(
                    tables.transitions,
                    &[&last_v0, below],
                    key,
                    meter,
                    0,
                    f1,
                    cadence,
                    ends,
                ) {
                    put(tick, c, key);
                }
            }
            None => {
                for (tick, c) in &of_theme {
                    put(f1 + tick, *c, key);
                }
            }
        }
        let before_dominant = if key.minor {
            Rel::new(1, 0, Kind::HalfDim7)
        } else {
            Rel::new(1, 0, Kind::Min7)
        };
        put(cadence, chord(before_dominant, key), key);
        put(cadence + second, seventh(key), key);
        // A minor fugue ends on the major chord.
        put(cadence + bar, chord(Rel::new(0, 0, Kind::Maj), key), key);
        harmony
    };
    let harmony = build(None);

    // The countersubject: the middle voice against the answer.
    let first_end = span(&answer_v0).map_or(link, |s| s.1.max(link));
    let first = Layout {
        name: subject.name.clone(),
        key,
        meter,
        origin: 0,
        end: first_end + (bar - first_end.rem_euclid(bar)) % bar,
        voices: vec![
            voice(RANGES[0], vec![answer_v0.clone()]),
            voice(RANGES[1], vec![theme_v1.clone()]),
            voice(RANGES[2], Vec::new()),
        ],
        final_hold: None,
        invertible: vec![(0, 1, e2, link)],
        root_at_hold: false,
        longest: Some(bar),
        junctions: true,
    };
    let theme_end = span(&theme_v1).map_or(0, |s| s.1);
    // A countersubject needs tones: a drawn rhythm in which the middle voice rests through most
    // of the answer is drawn again.
    let least = (answer_v0.len() / 3).max(2);
    let mut chosen = None;
    let mut first_attempts = 0;
    for round in 0..options.attempts as u64 {
        let options = Options {
            seed: options.seed.wrapping_add(7919 * round),
            ..options.clone()
        };
        let found = set(
            &first,
            &harmony,
            tables.rhythm,
            Some(tables.intervals),
            &options,
        )
        .map_err(|dead| anyhow!("{}: no countersubject; dead ends {dead:?}", subject.name))?;
        first_attempts += found.attempts;
        // What the middle voice does after its subject, up to the end of the answer's section.
        let continuation: Vec<Note> = found.piece.voices[1]
            .notes
            .iter()
            .filter(|n| n.on >= theme_end && n.on < link)
            .map(|n| Note {
                off: n.off.min(link),
                ..*n
            })
            .collect();
        let countersubject: Vec<Note> = continuation
            .iter()
            .filter(|n| answer_v0.first().is_some_and(|a| n.on >= a.on))
            .copied()
            .collect();
        if countersubject.len() >= least {
            chosen = Some((continuation, countersubject));
            break;
        }
    }
    let Some((continuation, countersubject)) = chosen else {
        return Err(anyhow!(
            "{}: no countersubject with at least {least} tones",
            subject.name
        ));
    };
    // Relative to the answer's section, against the answer as placed there.
    let placed_answer = Answer {
        kind: answer.kind,
        notes: answer_v0.clone(),
        fourth: answer.fourth.clone(),
    };
    let for_theme = moved(
        &under_subject(&countersubject, &placed_answer),
        -e2,
        Iv::UNISON,
    );
    let cs_e3 = fitted(&moved(&for_theme, e3, Iv::UNISON), RANGES[0]);
    let cs_m1 = fitted(
        &moved(&in_key(&for_theme, key, relative), m1, Iv::UNISON),
        RANGES[1],
    );
    let cs_f1 = fitted(&moved(&for_theme, f1, Iv::UNISON), RANGES[2]);

    let harmony = build(Some([&cs_e3, &cs_m1, &cs_f1]));
    let mut middle: Vec<Vec<Note>> = vec![theme_v1.clone()];
    if !continuation.is_empty() {
        middle.push(continuation.clone());
    }
    middle.push(cs_m1);
    let layout = Layout {
        name: subject.name.clone(),
        key,
        meter,
        origin: 0,
        end,
        voices: vec![
            voice(RANGES[0], vec![answer_v0, cs_e3, middle_v0, last_v0]),
            voice(RANGES[1], middle),
            voice(RANGES[2], vec![theme_v2, cs_f1]),
        ],
        final_hold: Some(cadence + bar),
        invertible: Vec::new(),
        root_at_hold: true,
        longest: Some(bar),
        junctions: true,
    };
    let setting = set(
        &layout,
        &harmony,
        tables.rhythm,
        Some(tables.intervals),
        options,
    )
    .map_err(|dead| anyhow!("{}: no fugue; dead ends {dead:?}", subject.name))?;
    Ok(Fugue {
        setting,
        layout,
        harmony,
        answer,
        countersubject,
        sections,
        seconds: begun.seconds(),
        first_attempts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::parse;

    #[test]
    fn lines_change_key_and_mode() {
        // C minor: c b c g as; in E flat major: es d es bes c.
        let theme = parse("c''16 b' c''8 g' as'").unwrap();
        let c_minor = Key::parse("c").unwrap();
        let major = in_key(&theme, c_minor, c_minor.relative());
        let names: Vec<String> = major.iter().map(|n| n.pitch.lof().to_string()).collect();
        assert_eq!(names, ["Eb", "D", "Eb", "Bb", "C"]);
        // C major: c d e f g a b c; in A minor the seventh before the tonic is raised, and the
        // sixth before it.
        let scale = parse("c'8 d' e' f' g' a' b' c''").unwrap();
        let c_major = Key::parse("C").unwrap();
        let minor = in_key(&scale, c_major, c_major.relative());
        let names: Vec<String> = minor.iter().map(|n| n.pitch.lof().to_string()).collect();
        assert_eq!(names, ["A", "B", "C", "D", "E", "F#", "G#", "A"]);
        // The same key: an exact transposition.
        let same = in_key(&theme, c_minor, Key::parse("g").unwrap());
        let names: Vec<String> = same.iter().map(|n| n.pitch.lof().to_string()).collect();
        assert_eq!(names, ["G", "F#", "G", "D", "Eb"]);
    }

    #[test]
    fn tones_fit_their_range() {
        let theme = parse("c''16 b' c''8 g' as'").unwrap();
        let low = fitted(&theme, RANGES[2]);
        assert!(low.iter().all(|n| n.pitch.midi >= 36 && n.pitch.midi <= 64));
        assert_eq!(low[0].pitch.lof(), theme[0].pitch.lof());
        assert_eq!(half(Meter::COMMON), 48);
        assert_eq!(half(Meter { num: 3, den: 4 }), 48);
    }
}
