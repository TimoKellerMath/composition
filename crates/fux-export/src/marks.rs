//! The brackets over the entries of an imitation (phase 13, docs/regeln.md § 18): where a score
//! carries the plan of its imitation, the head of every planned entry gets a bracket, labelled
//! with the number of its point, as analyses of imitative polyphony mark them.

use fux_core::Score;

/// A bracket over the head of an entry: its first and its last note in the voice, by their
/// indices, and its point, counted from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Mark {
    pub first: usize,
    pub last: usize,
    pub point: usize,
}

/// The brackets in voice `v` of `score`, in the order of their notes: one over the head of each
/// planned entry of the voice, from the note that begins at the entry's tick to the last note
/// that begins within its head. None without a plan; none for an entry whose head does not begin
/// with a note at its tick, has a single note, or overlaps a bracket before it, as in a file
/// edited by hand.
pub(crate) fn entry_marks(score: &Score, v: usize) -> Vec<Mark> {
    let (Some(plan), Some(voice)) = (score.imitation.as_deref(), score.voices.get(v)) else {
        return Vec::new();
    };
    // The start of each note, `None` for a rest.
    let starts: Vec<Option<u32>> = voice
        .timed()
        .map(|(start, note)| (!note.is_rest()).then_some(start))
        .collect();
    let mut marks: Vec<Mark> = Vec::new();
    for (p, point) in plan.points.iter().enumerate() {
        for entry in point.entries.iter().filter(|e| e.voice == v) {
            let end = entry.start + entry.notes.iter().map(|&(_, ticks)| ticks).sum::<u32>();
            let Some(first) = starts.iter().position(|&s| s == Some(entry.start)) else {
                continue;
            };
            let Some(last) = starts
                .iter()
                .rposition(|&s| s.is_some_and(|s| s >= entry.start && s < end))
            else {
                continue;
            };
            if last > first {
                marks.push(Mark {
                    first,
                    last,
                    point: p + 1,
                });
            }
        }
    }
    marks.sort_by_key(|m| m.first);
    let mut kept: Vec<Mark> = Vec::with_capacity(marks.len());
    for mark in marks {
        if kept.last().is_none_or(|before| before.last < mark.first) {
            kept.push(mark);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fux_core::{
        Duration, Pitch, Role, ScoreEntry, ScoreImitation, ScorePoint, Species, Voice, VoiceType,
        parse_lilypond,
    };

    use super::*;

    /// An entry of `voice` at `start` whose head has the given pitches and values.
    fn entry(voice: usize, start: u32, head: &[(&str, u32)]) -> ScoreEntry {
        ScoreEntry {
            voice,
            start,
            rest: 0,
            notes: head
                .iter()
                .map(|&(pitch, ticks)| (pitch.parse::<Pitch>().unwrap(), ticks))
                .collect(),
        }
    }

    /// A head tied over the barline ends at the last note that begins within it; a rest at the
    /// tick of an entry, a head of one note and a bracket overlapping the one before give none.
    #[test]
    fn brackets_span_the_heads() {
        let notes = |src: &str| parse_lilypond(src, Duration::HALF).unwrap();
        let free = |voice_type, src: &str| {
            Voice::new(voice_type, Role::Counterpoint(Species::Free), notes(src))
        };
        let mut score = Score::new(
            "dorian".parse().unwrap(),
            vec![
                free(
                    VoiceType::Soprano,
                    "a'1 a'2 bes'2 c''2 bes'2 a'1 g'1 a'1 r1 a'1",
                ),
                free(
                    VoiceType::Alto,
                    "r1 d'1 d'2 e'2 f'1~ f'2 e'2 cis'2 d'2 d'1 d'1",
                ),
            ],
        );
        assert!(entry_marks(&score, 0).is_empty());
        let head = [("A4", 8), ("A4", 4), ("Bb4", 4), ("C5", 4)];
        score.imitation = Some(Arc::new(ScoreImitation {
            points: vec![
                ScorePoint {
                    words: 0..7,
                    entries: vec![
                        entry(0, 0, &head),
                        entry(1, 8, &[("D4", 8), ("D4", 4), ("E4", 4), ("F4", 12)]),
                        // A rest at its tick, and a head of one note.
                        entry(0, 48, &head),
                        entry(0, 56, &[("A4", 8)]),
                    ],
                    goal: Some(5),
                },
                ScorePoint {
                    words: 3..7,
                    // Overlaps the soprano's first bracket.
                    entries: vec![entry(0, 12, &[("C5", 4), ("Bb4", 4), ("A4", 8)])],
                    goal: None,
                },
            ],
        }));
        let mark = |first, last, point| Mark { first, last, point };
        assert_eq!(entry_marks(&score, 0), [mark(0, 3, 1)]);
        // F4 tied over the barline: the bracket ends at its second note.
        assert_eq!(entry_marks(&score, 1), [mark(1, 5, 1)]);
        assert!(entry_marks(&score, 2).is_empty());
    }
}
