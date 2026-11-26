//! Subject and first answer of a fugue: from the Algomus labels where there are some, from the
//! entries of the first two voices otherwise.

use crate::algomus::Label;
use crate::score::{Note, Piece};

/// The subject as the first voice states it and the first answer, tone for tone.
#[derive(Clone, Debug, PartialEq)]
pub struct Exposition {
    pub subject: Vec<Note>,
    pub answer: Vec<Note>,
    pub subject_voice: usize,
    pub answer_voice: usize,
}

pub fn exposition(piece: &Piece, labels: Option<&[Label]>) -> Option<Exposition> {
    match labels {
        Some(labels) => from_labels(piece, labels),
        None => from_entries(piece),
    }
}

/// The first two labels `S` in time. In all 23 files of WTC I a label ends at the onset of the
/// subject's last tone, so the tone that begins at the end of the first label belongs to the
/// subject; the answer is as many tones of the second label's voice from its start.
pub fn from_labels(piece: &Piece, labels: &[Label]) -> Option<Exposition> {
    let mut entries = labels
        .iter()
        .filter(|l| l.is_subject() && l.voice.is_some());
    let first = entries.next()?;
    let second = entries.next()?;
    let subject_voice = first.voice?;
    let answer_voice = second.voice?;
    let mut subject = first.notes(piece).to_vec();
    let leader = piece.voices.get(subject_voice)?;
    if let Some(last) = leader.notes.iter().find(|n| n.on == first.end) {
        subject.push(*last);
    }
    let from = piece
        .voices
        .get(answer_voice)?
        .range(second.start, piece.end);
    if subject.is_empty() || from.len() < subject.len() {
        return None;
    }
    Some(Exposition {
        answer: from[..subject.len()].to_vec(),
        subject,
        subject_voice,
        answer_voice,
    })
}

/// Without labels: the first voice to enter has the subject, the second the answer. The subject
/// ends on the leader's last strong beat (the start of a bar, or its middle in a bar with an
/// even number of beats) at or before the entry of the answer, and not later than the answer
/// goes on repeating the leader.
///
/// On WTC I this finds the subject of the labels in 19 of 23 fugues. It ends too early where
/// the answer enters before the subject is over (I/10, I/19, I/22) and too late where a codetta
/// leads to an answer on a strong beat (I/7).
pub fn from_entries(piece: &Piece) -> Option<Exposition> {
    let mut order: Vec<usize> = (0..piece.voices.len())
        .filter(|v| !piece.voices[*v].notes.is_empty())
        .collect();
    order.sort_by_key(|v| piece.voices[*v].notes[0].on);
    let (&subject_voice, &answer_voice) = (order.first()?, order.get(1)?);
    let leader = &piece.voices[subject_voice].notes;
    let follower = &piece.voices[answer_voice].notes;
    let entry = follower[0].on;
    let repeated = common_prefix(leader, follower);
    let strong = leader
        .iter()
        .rposition(|n| n.on <= entry && piece.weight(n.on) >= 3)
        .map_or(0, |i| i + 1);
    let length = if strong >= 3 {
        repeated.min(strong)
    } else {
        repeated
    };
    (length >= 3).then(|| Exposition {
        subject: leader[..length].to_vec(),
        answer: follower[..length].to_vec(),
        subject_voice,
        answer_voice,
    })
}

/// How many tones of `follower` repeat `leader` from the start: the same rhythm, and every
/// melodic interval the same number of steps or one step more or less (a tonal answer changes
/// an interval by a step). The last tone counted may differ in length.
pub fn common_prefix(leader: &[Note], follower: &[Note]) -> usize {
    let mut length = 0;
    for k in 0..leader.len().min(follower.len()) {
        if k > 0 {
            let same_rhythm =
                leader[k].on - leader[k - 1].on == follower[k].on - follower[k - 1].on;
            let a = leader[k - 1].pitch.to(leader[k].pitch).steps;
            let b = follower[k - 1].pitch.to(follower[k].pitch).steps;
            if !same_rhythm || (a - b).abs() > 1 {
                break;
            }
        }
        length = k + 1;
    }
    length
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
    use crate::pitch::Pitch;
    use crate::score::Voice;
    use crate::time::{Meter, Tick};

    fn voice(start: Tick, tones: &[(i8, i8, Tick)]) -> Voice {
        let mut voice = Voice::default();
        let mut on = start;
        for (step, octave, ticks) in tones {
            voice.push(on, on + ticks, Pitch::new(*step, 0, *octave));
            on += ticks;
        }
        voice
    }

    #[test]
    fn the_subject_ends_on_the_strong_beat_before_the_answer() {
        // Leader: C D E F | G (bar 2) A B, then on; the answer enters a quarter after the G and
        // repeats everything a fifth higher, also what follows the subject.
        let leader = voice(
            0,
            &[
                (0, 4, 24),
                (1, 4, 24),
                (2, 4, 24),
                (3, 4, 24),
                (4, 4, 24),
                (5, 4, 12),
                (6, 4, 12),
                (0, 5, 48),
            ],
        );
        let follower = voice(
            120,
            &[
                (4, 4, 24),
                (5, 4, 24),
                (6, 4, 24),
                (0, 5, 24),
                (1, 5, 24),
                (2, 5, 12),
                (3, 5, 12),
                (4, 5, 48),
            ],
        );
        let piece = Piece::new(
            "entries",
            Key::parse("C").unwrap(),
            Meter::COMMON,
            vec![follower, leader],
        );
        let found = from_entries(&piece).unwrap();
        assert_eq!((found.subject_voice, found.answer_voice), (1, 0));
        assert_eq!(found.subject.len(), 5);
        assert_eq!(found.answer.len(), 5);
        assert_eq!(found.answer[0].pitch, Pitch::new(4, 0, 4));
        assert_eq!(
            common_prefix(&piece.voices[1].notes, &piece.voices[0].notes),
            8
        );
    }

    #[test]
    fn labels_end_at_the_onset_of_the_last_tone() {
        let leader = voice(0, &[(0, 4, 24), (1, 4, 24), (2, 4, 48), (3, 4, 24)]);
        let follower = voice(96, &[(4, 4, 24), (5, 4, 24), (6, 4, 48), (0, 5, 24)]);
        let piece = Piece::new(
            "labels",
            Key::parse("C").unwrap(),
            Meter::COMMON,
            vec![follower, leader],
        );
        let label = |start: Tick, end: Tick, voice: usize| Label {
            kind: "S".into(),
            tag: "S".into(),
            start,
            end,
            voice: Some(voice),
        };
        let found = from_labels(&piece, &[label(0, 48, 1), label(96, 144, 0)]).unwrap();
        assert_eq!(found.subject.len(), 3);
        assert_eq!(found.subject[2].pitch, Pitch::new(2, 0, 4));
        assert_eq!(found.answer[2].pitch, Pitch::new(6, 0, 4));
    }
}
