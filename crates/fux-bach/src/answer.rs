//! The answer to a fugue subject by rule, after Prout, *Fugue* (1891), §§ 116–156.
//!
//! Every tone of the subject is answered a fifth above (the tonic key answered in the dominant
//! key) or a fourth above (the dominant answered by the tonic). The rule has four clauses,
//! developed on the 24 fugues of WTC I:
//!
//! 1. Without a reason below, a tone goes up a fifth: the answer is real.
//! 2. A subject that begins on the dominant: its first tone goes up a fourth.
//! 3. Among the first four tones, a dominant reached from the tonic or left for the tonic by a
//!    leap of a fourth or fifth goes up a fourth.
//! 4. A subject that closes in the dominant key changes to the fourth as early as possible:
//!    after its first tone and after the last tone that the dominant key lacks (the natural
//!    fourth degree in major, the natural sixth in minor). If a rest lies between that point and
//!    the first raised fourth degree, the change waits for the rest. From the change on, every
//!    tone goes up a fourth.
//!
//! A tone repeated at once shares the region of the tone it repeats, and the change of clause 4
//! is not made where it would turn a rising step into a repeated tone.

use crate::key::Key;
use crate::pitch::Iv;
use crate::score::Note;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnswerKind {
    Real,
    Tonal,
}

/// The answer in the rhythm of the subject, a fifth or a fourth above it; the caller moves it in
/// time and octave.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    pub kind: AnswerKind,
    pub notes: Vec<Note>,
    /// Per tone: answered a fourth above instead of a fifth.
    pub fourth: Vec<bool>,
}

/// How many of the first tones clause 3 looks at.
const HEAD: usize = 4;

/// Whether the subject closes in the dominant key: its last tone belongs to the dominant triad
/// and the raised fourth degree comes after the last natural one.
pub fn modulates(subject: &[Note], key: Key) -> bool {
    let Some(last) = subject.last() else {
        return false;
    };
    let class = last.pitch.lof();
    let closes = [4, 6, 1].iter().any(|d| key.degree(*d) == class);
    let position = |wanted| subject.iter().rposition(|n| n.pitch.lof() == wanted);
    let raised = position(key.degree(3).up(7));
    let natural = position(key.degree(3));
    closes && raised.is_some() && raised > natural
}

/// Index of the first tone answered a fourth above under clause 4.
fn change(subject: &[Note], key: Key, fourth: &[bool]) -> usize {
    // The tone the dominant key does not have.
    let lacking = key.degree(if key.minor { 5 } else { 3 });
    let mut at = subject
        .iter()
        .rposition(|n| n.pitch.lof() == lacking)
        .map_or(1, |i| i + 1)
        .max(1);
    let raised = key.degree(3).up(7);
    let first_raised = (at..subject.len())
        .find(|i| subject[*i].pitch.lof() == raised)
        .unwrap_or(subject.len() - 1);
    if let Some(after_rest) = (at..=first_raised)
        .rev()
        .find(|i| *i > 0 && subject[*i].on > subject[*i - 1].off)
    {
        at = after_rest;
    }
    while at < subject.len() {
        let before = &subject[at - 1];
        let step = before.pitch.to(subject[at].pitch);
        let repeats = step.is_unison();
        let flattens = !fourth[at - 1] && step.steps == 1;
        if !(repeats || flattens) {
            break;
        }
        at += 1;
    }
    at
}

pub fn answer(subject: &[Note], key: Key) -> Answer {
    let count = subject.len();
    let tonic = key.degree(0);
    let dominant = key.degree(4);
    let is = |i: usize, class| subject[i].pitch.lof() == class;
    let leap = |a: usize, b: usize| {
        let steps = subject[a].pitch.to(subject[b].pitch).steps.abs();
        steps == 3 || steps == 4
    };
    // Clause 2 for the first tone, clause 3 for the first four.
    let mut fourth: Vec<bool> = (0..count)
        .map(|i| {
            if !is(i, dominant) || i >= HEAD {
                return false;
            }
            let from_tonic = i > 0 && is(i - 1, tonic) && leap(i - 1, i);
            let to_tonic = i + 1 < count && is(i + 1, tonic) && leap(i, i + 1);
            i == 0 || from_tonic || to_tonic
        })
        .collect();
    // A tone repeated at once shares the region.
    for i in 1..count {
        if subject[i].pitch == subject[i - 1].pitch && subject[i].on == subject[i - 1].off {
            fourth[i] = fourth[i - 1];
        }
    }
    // Clause 4.
    if count > 1 && modulates(subject, key) {
        let at = change(subject, key, &fourth);
        for region in fourth.iter_mut().skip(at) {
            *region = true;
        }
    }
    let notes = subject
        .iter()
        .zip(&fourth)
        .map(|(note, fourth)| Note {
            on: note.on,
            off: note.off,
            pitch: note.pitch.up(if *fourth { Iv::FOURTH } else { Iv::FIFTH }),
        })
        .collect();
    let kind = if fourth.iter().any(|f| *f) {
        AnswerKind::Tonal
    } else {
        AnswerKind::Real
    };
    Answer {
        kind,
        notes,
        fourth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::{Lof, Pitch};
    use crate::time::Tick;

    /// Tones from names like `C5`, `F#4`, `Bb3`, as eighths; `-` is an eighth rest.
    fn line(names: &str) -> Vec<Note> {
        let mut notes = Vec::new();
        let mut on: Tick = 0;
        for name in names.split_whitespace() {
            if name != "-" {
                let (class, octave) = name.split_at(name.len() - 1);
                let class = Lof::parse(class).unwrap();
                let pitch = Pitch::new(class.step(), class.alter(), octave.parse().unwrap());
                notes.push(Note {
                    on,
                    off: on + 12,
                    pitch,
                });
            }
            on += 12;
        }
        notes
    }

    fn classes(notes: &[Note]) -> String {
        notes
            .iter()
            .map(|n| n.pitch.lof().to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn answered(subject: &str, key: &str) -> (AnswerKind, String) {
        let answer = answer(&line(subject), Key::parse(key).unwrap());
        (answer.kind, classes(&answer.notes))
    }

    #[test]
    fn a_leap_to_the_dominant_in_the_head() {
        // WTC I/2: only the G reached from the tonic changes.
        assert_eq!(
            answered("C5 B4 C5 G4 Ab4 C5 B4 C5 D5 G4 C5", "c"),
            (AnswerKind::Tonal, "G F# G C Eb G F# G A D G".into())
        );
        let answer = answer(&line("C5 B4 C5 G4 Ab4"), Key::parse("c").unwrap());
        assert_eq!(answer.fourth, [false, false, false, true, false]);
        assert_eq!(answer.notes[0].pitch, Pitch::new(4, 0, 5));
        assert_eq!(answer.notes[3].pitch, Pitch::new(0, 0, 5));
        // WTC I/17: tonic, dominant, then real.
        assert_eq!(
            answered("Ab3 Eb4 C4 Ab3 F4 Db4 Eb4", "Ab"),
            (AnswerKind::Tonal, "Eb Ab G Eb C Ab Bb".into())
        );
    }

    #[test]
    fn a_real_answer() {
        // WTC I/1: the dominant is reached by step.
        assert_eq!(
            answered("C4 D4 E4 F4 G4 F4 E4 A4 D4 G4", "C"),
            (AnswerKind::Real, "G A B C D C B E A D".into())
        );
        // WTC I/9: the dominant comes from the second degree, not from the tonic.
        assert_eq!(answered("E4 F#4 B3 C#4 D#4 E4", "E").0, AnswerKind::Real);
        // A leap between tonic and dominant after the head stays.
        assert_eq!(
            answered("G4 A4 G4 F#4 G4 A4 B4 A4 G4 D4 C5 B4", "G").0,
            AnswerKind::Real
        );
    }

    #[test]
    fn a_subject_on_the_dominant() {
        // WTC I/16: the first tone only.
        assert_eq!(
            answered("D4 Eb4 G3 F#3 G3 A3 Bb3", "g"),
            (AnswerKind::Tonal, "G Bb D C# D E F".into())
        );
        // WTC I/21: the dominant returns and leaps to the tonic.
        assert_eq!(
            answered("F4 G4 F4 Bb4 D4 C4", "Bb"),
            (AnswerKind::Tonal, "Bb D Bb F A G".into())
        );
        // A dominant repeated at once is answered by the repeated tonic.
        assert_eq!(answered("G4 G4 E4 C4", "C").1, "C C B G");
    }

    #[test]
    fn a_subject_that_closes_in_the_dominant() {
        let key = Key::parse("g#").unwrap();
        // WTC I/18: the change comes right after the first tone.
        let subject = line("G#3 F##3 G#3 A#3 B3 A#3 G#3 C##4 D#4");
        assert!(modulates(&subject, key));
        assert_eq!(
            classes(&answer(&subject, key).notes),
            "D# B# C# D# E D# C# F## G#"
        );
        // WTC I/24: after the natural sixth degree.
        assert_eq!(
            answered("F#4 D4 B3 G4 F#4 B4 A#4 E4 D#4 C5 B4 F#4 E#4 D5 C#5", "b").1,
            "B A F# D B E D# A G# F E B A# G F#"
        );
        // WTC I/7: at the rest before the raised fourth degree.
        assert_eq!(
            answered(
                "Bb4 G4 F4 G4 Eb4 Ab4 G4 Ab4 C5 Bb4 - A4 F4 Eb5 D5 C5 Bb4",
                "Eb"
            )
            .1,
            "Eb D C D Bb Eb D Eb G F D Bb Ab G F Eb"
        );
        // A rising step is not flattened into a repetition: the change waits one tone.
        assert_eq!(answered("C4 D4 F#4 G4", "C").1, "G A B C");
        // Without the raised fourth degree a close on the dominant is no modulation.
        assert!(!modulates(
            &line("D4 E4 F4 G4 E4 F4 D4 C#4 D4 Bb4 G4 A4"),
            Key::parse("d").unwrap()
        ));
    }
}
