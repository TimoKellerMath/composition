//! Minimal examples for the rules of imitation (IMI-*): for every parameter a setting with one
//! case of it and one without, with the tick at which the case is decided, its voice and the note
//! it marks; every example window by window and prefix by prefix; the rules' silence without a
//! plan; the syllables only of voices that sing; levels and messages.

use std::ops::Range;
use std::sync::Arc;

use super::{Analysis, Checker, Detail, Level, ParamKind, Profile, Violation, Window, all_rules};
use crate::clef::VoiceType;
use crate::parse::parse_lilypond;
use crate::pitch::Pitch;
use crate::score::{
    Role, Score, ScoreEntry, ScoreImitation, ScorePoint, ScoreText, Species, Voice,
};
use crate::text::Text;
use crate::text::test_support::{section, sung};
use crate::time::Duration;

/// A head in LilyPond notation as the plan keeps it: pitch and value, tied notes merged.
fn head(src: &str) -> Vec<(Pitch, u32)> {
    let mut out: Vec<(Pitch, u32)> = Vec::new();
    let mut tied = false;
    for note in parse_lilypond(src, Duration::HALF).unwrap() {
        let pitch = note.pitch.expect("a head has no rests");
        let ticks = note.duration.ticks();
        match out.last_mut() {
            Some(last) if tied => last.1 += ticks,
            _ => out.push((pitch, ticks)),
        }
        tied = note.tie;
    }
    out
}

/// A point of a plan: its syllables, its goal and its entries as (voice, start, rest, head).
struct Point {
    words: Range<u16>,
    goal: Option<usize>,
    entries: Vec<(usize, u32, u32, &'static str)>,
}

fn point(
    words: Range<u16>,
    goal: Option<usize>,
    entries: &[(usize, u32, u32, &'static str)],
) -> Point {
    Point {
        words,
        goal,
        entries: entries.to_vec(),
    }
}

fn plan(points: &[Point]) -> ScoreImitation {
    ScoreImitation {
        points: points
            .iter()
            .map(|p| ScorePoint {
                words: p.words.clone(),
                entries: p
                    .entries
                    .iter()
                    .map(|&(voice, start, rest, notes)| ScoreEntry {
                        voice,
                        start,
                        rest,
                        notes: head(notes),
                    })
                    .collect(),
                goal: p.goal,
            })
            .collect(),
    }
}

/// A setting: free voices in a mode, from the top, that sing a text (`None`: no text), and the
/// plan of their imitation.
struct Setting {
    mode: &'static str,
    text: Option<Text>,
    voices: Vec<&'static str>,
    points: Vec<Point>,
}

impl Setting {
    fn score(&self) -> Score {
        const TYPES: [VoiceType; 4] = [
            VoiceType::Soprano,
            VoiceType::Alto,
            VoiceType::Tenor,
            VoiceType::Bass,
        ];
        let voices = self
            .voices
            .iter()
            .zip(TYPES)
            .map(|(notes, voice_type)| {
                Voice::new(voice_type, Role::Counterpoint(Species::Free), sung(notes))
            })
            .collect();
        let mut score = Score::new(self.mode.parse().unwrap(), voices);
        score.text = self.text.as_ref().map(|text| ScoreText {
            text: Arc::new(text.clone()),
            voices: vec![true; self.voices.len()],
        });
        score.imitation = Some(Arc::new(plan(&self.points)));
        if let Err(e) = score.validate() {
            panic!("{:?}: {e}", self.voices);
        }
        score
    }
}

fn kyrie() -> Option<Text> {
    Some(section("kyrie"))
}

/// Kyrie I in soprano and alto: the soprano leads, the alto answers a measure later at the fifth
/// below, with the same syllables on the same notes; the soprano's e of eleison is a melisma.
const SOPRANO: &str = "a'1=0 a'2=1 b'2=2 c''2=3 b'2 a'1=4 g'1=5 a'1=6";
const ALTO: &str = "r1 d'1=0 d'2=1 e'2=2 f'2=3 e'2 d'1=4 c'2=5 d'2=6";
const SOPRANO_HEAD: &str = "a'1 a'2 b'2 c''2 b'2 a'1";
const ALTO_HEAD: &str = "d'1 d'2 e'2 f'2 e'2 d'1";

/// The duo with the alto as given.
fn duo(alto: &'static str) -> Setting {
    Setting {
        mode: "dorian",
        text: kyrie(),
        voices: vec![SOPRANO, alto],
        points: vec![point(
            0..7,
            None,
            &[(0, 0, 0, SOPRANO_HEAD), (1, 8, 0, ALTO_HEAD)],
        )],
    }
}

/// The duo without a text, the alto with other notes and another head, entering at `start`.
fn answered(alto: &'static str, alto_head: &'static str, start: u32) -> Setting {
    Setting {
        mode: "dorian",
        text: None,
        voices: vec![SOPRANO, alto],
        points: vec![point(
            0..7,
            None,
            &[(0, 0, 0, SOPRANO_HEAD), (1, start, 0, alto_head)],
        )],
    }
}

/// The soprano alone with its head, as the leader of the first point.
fn solo(text: Option<Text>, voice: &'static str, notes: &'static str) -> Setting {
    Setting {
        mode: "dorian",
        text,
        voices: vec![voice],
        points: vec![point(0..7, None, &[(0, 0, 0, notes)])],
    }
}

/// The duo without a text, the alto singing a measure before it rests and enters at `start`: a
/// re-entry.
fn reentry(alto: &'static str, start: u32, alto_head: &'static str) -> Setting {
    Setting {
        mode: "dorian",
        text: None,
        voices: vec![SOPRANO, alto],
        points: vec![point(
            0..7,
            None,
            &[(0, 0, 0, SOPRANO_HEAD), (1, start, 4, alto_head)],
        )],
    }
}

/// Two points: the duo, then the tail e-lé-i-son led by the alto after a rest; the first point's
/// cadence arrives in measure `goal` (from 0).
fn two_points(
    mode: &'static str,
    soprano: &'static str,
    alto: &'static str,
    tail: &'static str,
    goal: usize,
) -> Setting {
    Setting {
        mode,
        text: None,
        voices: vec![soprano, alto],
        points: vec![
            point(
                0..7,
                Some(goal),
                &[(0, 0, 0, SOPRANO_HEAD), (1, 8, 0, ALTO_HEAD)],
            ),
            point(3..7, None, &[(1, 48, 4, tail)]),
        ],
    }
}

/// The soprano of the duo, ten measures long.
const SOPRANO_LONG: &str = "a'1 a'2 b'2 c''2 b'2 a'1 g'1 a'1 g'1 f'1 e'1 d'1";

/// The soprano of the duo, nine measures long.
const SOPRANO_ON: &str = "a'1 a'2 b'2 c''2 b'2 a'1 g'1 a'1 a'1 a'1 a'1";
/// The alto of the duo, then a measure's rest and the tail on D.
const ALTO_TAIL: &str = "r1 d'1 d'2 e'2 f'2 e'2 d'1 r1 d'1 f'2 e'2 d'1";

/// A case of a parameter: a setting with it, decided at a tick on a note of a voice, and one
/// without.
struct Example {
    rule: &'static str,
    param: &'static str,
    with: Setting,
    /// The tick, the voice and the marked note.
    at: (u32, usize, usize),
    without: Setting,
}

fn example(
    (rule, param): (&'static str, &'static str),
    with: Setting,
    at: (u32, usize, usize),
    without: Setting,
) -> Example {
    Example {
        rule,
        param,
        with,
        at,
        without,
    }
}

/// One or more examples for every level parameter of the IMI rules. Measures are 8 ticks long, a
/// half note 4.
fn examples() -> Vec<Example> {
    vec![
        // A head note at another pitch, tied over from the note before, or a rest.
        example(
            ("IMI-ENTRY", "level"),
            duo("r1 d'1=0 d'2=1 f'2=2 f'2=3 e'2 d'1=4 c'2=5 d'2=6"),
            (20, 1, 3),
            duo(ALTO),
        ),
        example(
            ("IMI-ENTRY", "level"),
            duo("r1 d'1~=0 d'2 e'2=2 f'2=3 e'2 d'1=4 c'2=5 d'2=6"),
            (16, 1, 1),
            duo(ALTO),
        ),
        example(
            ("IMI-ENTRY", "level"),
            duo("r1 d'1=0 r2 e'2=2 f'2=3 e'2 d'1=4 c'2=5 d'2=6"),
            (16, 1, 2),
            duo(ALTO),
        ),
        // A head note cut short, decided where the voice's next note begins; the next head note
        // still sounds at its tick. The last head note too, before a rest.
        example(
            ("IMI-ENTRY", "rhythm"),
            duo("r1 d'1=0 d'2=1 e'2=2 f'4=3 g'4 e'2 d'1=4 c'2=5 d'2=6"),
            (26, 1, 4),
            duo(ALTO),
        ),
        example(
            ("IMI-ENTRY", "rhythm"),
            duo("r1 d'1=0 d'2=1 e'2=2 f'2=3 e'2 d'2=4 r2 c'2=5 d'2=6"),
            (36, 1, 6),
            duo(ALTO),
        ),
        // Another syllable on the head's last note, none where the leader begins one, and one
        // where the leader holds its syllable.
        example(
            ("IMI-ENTRY", "text"),
            duo("r1 d'1=0 d'2=1 e'2=2 f'2=3 e'2 d'1=5 c'2=4 d'2=6"),
            (32, 1, 6),
            duo(ALTO),
        ),
        example(
            ("IMI-ENTRY", "text"),
            duo("r1 d'1=0 d'2=1 e'2=2 f'2=3 e'2 d'1 c'2=4 d'2=5"),
            (32, 1, 6),
            duo(ALTO),
        ),
        example(
            ("IMI-ENTRY", "text"),
            duo("r1 d'1=0 d'2=1 e'2=2 f'2=3 e'2=4 d'1=4 c'2=5 d'2=6"),
            (28, 1, 5),
            duo(ALTO),
        ),
        // A re-entry after a quarter rest and without a rest; after a minim rest it is free.
        example(
            ("IMI-REST", "level"),
            reentry("d'1 d'2. r4 d'1 d'2 e'2 f'2 e'2 d'1", 16, ALTO_HEAD),
            (16, 1, 3),
            reentry("d'1 r1 d'1 d'2 e'2 f'2 e'2 d'1", 16, ALTO_HEAD),
        ),
        example(
            ("IMI-REST", "level"),
            reentry("d'1 c'1 d'1 d'2 e'2 f'2 e'2 d'1", 16, ALTO_HEAD),
            (16, 1, 2),
            reentry("d'1 c'2 r2 d'1 d'2 e'2 f'2 e'2 d'1", 16, ALTO_HEAD),
        ),
        // On the fourth quarter.
        example(
            ("IMI-REST", "offbeat"),
            reentry(
                "d'1 r1 r2. d'4 d'2 e'2 f'2 e'2 d'2 d'2",
                22,
                "d'4 d'2 e'2 f'2 e'2 d'2",
            ),
            (22, 1, 3),
            reentry("d'1 r1 d'1 d'2 e'2 f'2 e'2 d'1", 16, ALTO_HEAD),
        ),
        // The minim before the rest is marked; a whole there is free.
        example(
            ("IMI-REST", "minim_before"),
            reentry("d'1 c'2 r2 d'1 d'2 e'2 f'2 e'2 d'1", 16, ALTO_HEAD),
            (16, 1, 1),
            reentry("d'1 r1 d'1 d'2 e'2 f'2 e'2 d'1", 16, ALTO_HEAD),
        ),
        // The soprano rests in measure 5, before the tenor's entry in measure 6; singing on, it
        // is free.
        example(
            ("IMI-SILENCE", "exposition"),
            trio("a'1 a'2 b'2 c''2 b'2 a'1 r1 a'1 g'1 f'1 e'1"),
            (32, 0, 6),
            trio("a'1 a'2 b'2 c''2 b'2 a'1 g'1 a'1 g'1 f'1 e'1"),
        ),
        // The alto silent for nine measures after its head, the last beyond four breves; for
        // eight, free.
        example(
            ("IMI-SILENCE", "long"),
            silent("r1 d'1 d'2 e'2 f'2 e'2 d'1 r1 r1 r1 r1 r1 r1 r1 r1 r1 d'1"),
            (104, 1, 15),
            silent("r1 d'1 d'2 e'2 f'2 e'2 d'1 r1 r1 r1 r1 r1 r1 r1 r1 d'1 d'1"),
        ),
        // After the bass's entry, soprano and alto rest in measure 9: half of the four voices
        // sound there; singing on, they are free.
        example(
            ("IMI-SILENCE", "thin"),
            quartet(
                "a'1 a'2 b'2 c''2 b'2 a'1 g'1 a'1 g'1 f'1 r1 e'1 d'1 e'1",
                "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1 d'1 c'1 r1 c'1 a1 a1",
            ),
            (64, 0, 10),
            quartet(
                "a'1 a'2 b'2 c''2 b'2 a'1 g'1 a'1 g'1 f'1 e'1 e'1 d'1 e'1",
                "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1 d'1 c'1 c'1 c'1 a1 a1",
            ),
        ),
        // The tenor rests, so the alto's C4, tied over measure 7, is the bass: the soprano's F4
        // a fourth above it on the downbeat, the alto rising to E4. Stepping down to B3 instead,
        // the tied note is the suspension.
        example(
            ("IMI-HELD", "struck"),
            held_trio(
                "a'1 a'2 b'2 c''2 b'2 a'1 g'1 e'1 f'1 e'1",
                "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1~ c'2 e'2 d'1",
                "r1 r1 r1 r1 r1 r1 r1 a1",
            ),
            (52, 0, 8),
            held_trio(
                "a'1 a'2 b'2 c''2 b'2 a'1 g'1 e'1 f'1 e'1",
                "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1~ c'2 b2 c'1",
                "r1 r1 r1 r1 r1 r1 r1 a1",
            ),
        ),
        // The tenor's D3 is tied over measure 7, consonant on the downbeat: the soprano leaps
        // into C5, a seventh above it. Moving by step, E5 passes.
        example(
            ("IMI-HELD", "passing"),
            held_trio(
                "a'1 a'2 b'2 c''2 b'2 a'1 g'1 e'1 f'4 c''4 b'4 a'4 g'1",
                "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1 a1 b1",
                "r1 r1 r1 r1 r1 d1~ d1 g1",
            ),
            (50, 0, 9),
            held_trio(
                "a'1 a'2 b'2 c''2 b'2 a'1 g'1 e'1 f'4 e'4 d'4 e'4 f'1",
                "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1 a1 b1",
                "r1 r1 r1 r1 r1 d1~ d1 g1",
            ),
        ),
        // F4 and G4 against the leader's A4; D4 is a fifth below.
        example(
            ("IMI-INTERVAL", "third_sixth"),
            answered(
                "r1 f'1 f'2 g'2 a'2 g'2 f'1 e'2 f'2",
                "f'1 f'2 g'2 a'2 g'2 f'1",
                8,
            ),
            (8, 1, 1),
            answered(ALTO, ALTO_HEAD, 8),
        ),
        example(
            ("IMI-INTERVAL", "second_seventh"),
            answered(
                "r1 g'1 g'2 a'2 b'2 a'2 g'1 f'2 g'2",
                "g'1 g'2 a'2 b'2 a'2 g'1",
                8,
            ),
            (8, 1, 1),
            answered(ALTO, ALTO_HEAD, 8),
        ),
        // The second entry five measures after the first, and four: the latter only after the
        // leader's head has ended.
        example(
            ("IMI-TIME", "second"),
            Setting {
                voices: vec![SOPRANO_LONG, "r1 r1 r1 r1 r1 d'1 d'2 e'2 f'2 e'2 d'1 d'1"],
                ..answered("", ALTO_HEAD, 40)
            },
            (40, 1, 5),
            Setting {
                voices: vec![SOPRANO_LONG, "r1 r1 r1 r1 d'1 d'2 e'2 f'2 e'2 d'1 d'1 d'1"],
                ..answered("", ALTO_HEAD, 32)
            },
        ),
        example(
            ("IMI-TIME", "apart"),
            Setting {
                voices: vec![SOPRANO_LONG, "r1 r1 r1 r1 d'1 d'2 e'2 f'2 e'2 d'1 d'1 d'1"],
                ..answered("", ALTO_HEAD, 32)
            },
            (32, 1, 4),
            answered(ALTO, ALTO_HEAD, 8),
        ),
        // A third entry 68 ticks after the second; 64 is free.
        example(
            ("IMI-TIME", "later"),
            three_entries(
                "r1 r1 r1 r1 r1 r1 r1 r1 r1 r2 d2~ d2 d2 e2 f2",
                76,
                "d2~ d2 d2 e2 f2",
            ),
            (76, 2, 10),
            three_entries(
                "r1 r1 r1 r1 r1 r1 r1 r1 r1 d1 d2 e2 f2 e2",
                72,
                "d1 d2 e2 f2",
            ),
        ),
        // The leader rises a fourth from A4 to D5: answered by a fifth, a modal answer; and the
        // real answer, a fourth.
        example(
            ("IMI-ANSWER", "modal"),
            answer("r1 d'1 a'1 g'2 f'2 e'1 d'1", "d'1 a'1 g'2 f'2 e'1"),
            (16, 1, 2),
            answer("r1 d'1 g'1 f'2 e'2 d'1 d'1", "d'1 g'1 f'2 e'2 d'1"),
        ),
        // A third down for a step down in interval 2, a repeat for a step in interval 3; a change
        // by two steps is none of them.
        example(
            ("IMI-ANSWER", "adjusted"),
            answer("r1 d'1 g'1 e'2 d'2 c'1 d'1", "d'1 g'1 e'2 d'2 c'1"),
            (24, 1, 3),
            answer("r1 d'1 g'1 f'2 e'2 d'1 d'1", "d'1 g'1 f'2 e'2 d'1"),
        ),
        example(
            ("IMI-ANSWER", "adjusted"),
            answer("r1 d'1 g'1 f'2 f'2 e'1 d'1", "d'1 g'1 f'2 f'2 e'1"),
            (28, 1, 4),
            answer("r1 d'1 g'1 d'2 e'2 d'1 d'1", "d'1 g'1 d'2 e'2 d'1"),
        ),
        // A half as the first note, decided where it ends.
        example(
            ("IMI-SUBJECT", "short_first"),
            solo(
                kyrie(),
                "a'2=0 b'2=1 c''1=2 b'2=3 a'2 g'1=4 a'1=5 a'1=6",
                "a'2 b'2 c''1 b'2 a'2 g'1",
            ),
            (4, 0, 0),
            solo(kyrie(), SOPRANO, SOPRANO_HEAD),
        ),
        // A dotted whole as the fourth note; after Ho- of Hosanna, which is unaccented, free.
        example(
            ("IMI-SUBJECT", "not_longest"),
            solo(
                kyrie(),
                "a'1=0 a'2=1 b'2=2 c''1~=3 c''2 b'2 a'1=4 g'1=5",
                "a'1 a'2 b'2 c''1~ c''2 b'2 a'1",
            ),
            (16, 0, 3),
            solo(
                Some(section("osanna")),
                "a'1=0 a'2=1 b'2=2 c''1~=3 c''2 b'2 a'1=4 g'1=5",
                "a'1 a'2 b'2 c''1~ c''2 b'2 a'1",
            ),
        ),
        // From a whole to quarters; after a dotted whole they are free.
        example(
            ("IMI-SUBJECT", "drop"),
            solo(
                None,
                "a'1 a'4 b'4 c''2 b'1 a'1 g'1 a'1",
                "a'1 a'4 b'4 c''2 b'1 a'1",
            ),
            (8, 0, 1),
            solo(
                None,
                "a'1~ a'2 b'4 c''4 b'1 a'1 g'1 a'1",
                "a'1~ a'2 b'4 c''4 b'1 a'1",
            ),
        ),
        // D4 to C5, decided at C5; a fifth is free.
        example(
            ("IMI-SUBJECT", "range"),
            solo(
                None,
                "d'1 e'2 f'2 a'2 c''2 b'1 a'1 d'1",
                "d'1 e'2 f'2 a'2 c''2 b'1",
            ),
            (20, 0, 4),
            solo(None, SOPRANO, SOPRANO_HEAD),
        ),
        // D4 to E5; an octave is only `range`.
        example(
            ("IMI-SUBJECT", "beyond_octave"),
            solo(
                None,
                "d'1 e'2 f'2 a'2 c''2 e''1 d''1 d''1",
                "d'1 e'2 f'2 a'2 c''2 e''1",
            ),
            (24, 0, 5),
            solo(
                None,
                "d'1 e'2 f'2 a'2 c''2 d''1 d''1 d''1",
                "d'1 e'2 f'2 a'2 c''2 d''1",
            ),
        ),
        // An answer beyond an octave too: the alto from D4 to E5, decided at E5.
        example(
            ("IMI-SUBJECT", "beyond_octave"),
            answered(
                "r1 d'1 a'2 d''2 e''2 d''2 a'1 d'1",
                "d'1 a'2 d''2 e''2 d''2 a'1",
                8,
            ),
            (24, 1, 4),
            answered(ALTO, ALTO_HEAD, 8),
        ),
        // Six halves by step, decided at the sixth and marked at the first; a leap of a third
        // makes it a figure.
        example(
            ("IMI-SUBJECT", "plain"),
            solo(
                None,
                "a'2 b'2 c''2 b'2 a'2 g'2 a'1 g'1 a'1",
                "a'2 b'2 c''2 b'2 a'2 g'2",
            ),
            (20, 0, 0),
            solo(
                None,
                "a'2 b'2 c''2 a'2 b'2 g'2 a'1 g'1 a'1",
                "a'2 b'2 c''2 a'2 b'2 g'2",
            ),
        ),
        // A fourth up to D5 and another to E5; one is free.
        example(
            ("IMI-SUBJECT", "leaps"),
            solo(
                None,
                "a'1 d''2 c''2 b'2 e''2 d''1 c''1 d''1",
                "a'1 d''2 c''2 b'2 e''2 d''1",
            ),
            (20, 0, 4),
            solo(
                None,
                "a'1 d''2 c''2 b'2 c''2 d''1 c''1 d''1",
                "a'1 d''2 c''2 b'2 c''2 d''1",
            ),
        ),
        // The last head note on the minim beat, at its attack; a quarter on the downbeat, where
        // it ends.
        example(
            ("IMI-SUBJECT", "end"),
            solo(None, SOPRANO, "a'1 a'2 b'2 c''2 b'2"),
            (20, 0, 4),
            solo(None, SOPRANO, SOPRANO_HEAD),
        ),
        example(
            ("IMI-SUBJECT", "end"),
            solo(
                None,
                "a'1 a'2 b'2 c''1 b'4 a'4 g'2 f'1 e'1",
                "a'1 a'2 b'2 c''1 b'4",
            ),
            (26, 0, 4),
            solo(None, SOPRANO, SOPRANO_HEAD),
        ),
        // The tail on G and on E; on D it is free.
        example(
            ("IMI-DEGREE", "fourth"),
            two_points(
                "dorian",
                SOPRANO_ON,
                "r1 d'1 d'2 e'2 f'2 e'2 d'1 r1 g'1 b'2 a'2 g'1",
                "g'1 b'2 a'2 g'1",
                5,
            ),
            (48, 1, 8),
            two_points("dorian", SOPRANO_ON, ALTO_TAIL, "d'1 f'2 e'2 d'1", 5),
        ),
        example(
            ("IMI-DEGREE", "level"),
            two_points(
                "dorian",
                SOPRANO_ON,
                "r1 d'1 d'2 e'2 f'2 e'2 d'1 r1 e'1 g'2 f'2 e'1",
                "e'1 g'2 f'2 e'1",
                5,
            ),
            (48, 1, 8),
            two_points("dorian", SOPRANO_ON, ALTO_TAIL, "d'1 f'2 e'2 d'1", 5),
        ),
        // In the Phrygian mode B counts as the fourth and A as the fifth.
        example(
            ("IMI-DEGREE", "fourth"),
            two_points(
                "phrygian",
                SOPRANO_ON,
                "r1 d'1 d'2 e'2 f'2 e'2 d'1 r1 b'1 d''2 c''2 b'1",
                "b'1 d''2 c''2 b'1",
                5,
            ),
            (48, 1, 8),
            two_points(
                "phrygian",
                SOPRANO_ON,
                "r1 d'1 d'2 e'2 f'2 e'2 d'1 r1 a'1 c''2 b'2 a'1",
                "a'1 c''2 b'2 a'1",
                5,
            ),
        ),
        // The tail two measures after the goal in measure 4; a measure after it is free.
        example(
            ("IMI-OVERLAP", "late"),
            two_points("dorian", SOPRANO_ON, ALTO_TAIL, "d'1 f'2 e'2 d'1", 4),
            (48, 1, 8),
            two_points("dorian", SOPRANO_ON, ALTO_TAIL, "d'1 f'2 e'2 d'1", 5),
        ),
        // The soprano rests where the alto begins the tail.
        example(
            ("IMI-OVERLAP", "bare"),
            two_points(
                "dorian",
                "a'1 a'2 b'2 c''2 b'2 a'1 g'1 a'1 r1 a'1 a'1",
                ALTO_TAIL,
                "d'1 f'2 e'2 d'1",
                5,
            ),
            (48, 1, 8),
            two_points("dorian", SOPRANO_ON, ALTO_TAIL, "d'1 f'2 e'2 d'1", 5),
        ),
    ]
}

/// The duo with a tenor that enters at `start`.
fn three_entries(tenor: &'static str, start: u32, tenor_head: &'static str) -> Setting {
    Setting {
        mode: "dorian",
        text: None,
        voices: vec![
            "a'1 a'2 b'2 c''2 b'2 a'1 g'1 a'1 g'1 f'1 e'1 d'1 e'1 f'1",
            "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1 d'1 e'1 f'1 e'1 d'1 d'1",
            tenor,
        ],
        points: vec![point(
            0..7,
            None,
            &[
                (0, 0, 0, SOPRANO_HEAD),
                (1, 8, 0, ALTO_HEAD),
                (2, start, 0, tenor_head),
            ],
        )],
    }
}

/// Three voices without a text: the soprano as given leads, the alto answers a measure later,
/// the tenor enters in measure 6.
fn trio(soprano: &'static str) -> Setting {
    Setting {
        mode: "dorian",
        text: None,
        voices: vec![
            soprano,
            "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1 d'1 e'1 f'1",
            "r1 r1 r1 r1 r1 a1 a2 b2 c'2 b2 a1",
        ],
        points: vec![point(
            0..7,
            None,
            &[
                (0, 0, 0, SOPRANO_HEAD),
                (1, 8, 0, ALTO_HEAD),
                (2, 40, 0, "a1 a2 b2 c'2 b2 a1"),
            ],
        )],
    }
}

/// The duo without a text, fifteen measures long, the alto as given after its head.
fn silent(alto: &'static str) -> Setting {
    Setting {
        mode: "dorian",
        text: None,
        voices: vec![
            "a'1 a'2 b'2 c''2 b'2 a'1 g'1 a'1 g'1 f'1 e'1 d'1 e'1 f'1 e'1 d'1 e'1",
            alto,
        ],
        points: vec![point(
            0..7,
            None,
            &[(0, 0, 0, SOPRANO_HEAD), (1, 8, 0, ALTO_HEAD)],
        )],
    }
}

/// Four voices without a text: the soprano and the alto as given lead, the tenor and the bass
/// follow, each a measure after the voice before.
fn quartet(soprano: &'static str, alto: &'static str) -> Setting {
    Setting {
        mode: "dorian",
        text: None,
        voices: vec![
            soprano,
            alto,
            "r1 r1 a1 a2 b2 c'2 b2 a1 g1 a1 a1 f1 e1 d1",
            "r1 r1 r1 d1 d2 e2 f2 e2 d1 c1 d1 f1 g1 d1",
        ],
        points: vec![point(
            0..7,
            None,
            &[
                (0, 0, 0, SOPRANO_HEAD),
                (1, 8, 0, ALTO_HEAD),
                (2, 16, 0, "a1 a2 b2 c'2 b2 a1"),
                (3, 24, 0, "d1 d2 e2 f2 e2 d1"),
            ],
        )],
    }
}

/// The duo's plan in three voices without a text, as given: a free tenor below.
fn held_trio(soprano: &'static str, alto: &'static str, tenor: &'static str) -> Setting {
    Setting {
        mode: "dorian",
        text: None,
        voices: vec![soprano, alto, tenor],
        points: vec![point(
            0..7,
            None,
            &[(0, 0, 0, SOPRANO_HEAD), (1, 8, 0, ALTO_HEAD)],
        )],
    }
}

/// The leader rises a fourth, A4 to D5, and falls by steps; the alto answers a measure later.
fn answer(alto: &'static str, alto_head: &'static str) -> Setting {
    Setting {
        mode: "dorian",
        text: None,
        voices: vec!["a'1 d''1 c''2 b'2 a'1 g'1 a'1", alto],
        points: vec![point(
            0..7,
            None,
            &[(0, 0, 0, "a'1 d''1 c''2 b'2 a'1"), (1, 8, 0, alto_head)],
        )],
    }
}

/// Fux's profile with every case of the IMI rules switched on: those free by default cost P1.
fn every_case() -> Profile {
    let mut profile = Profile::fux();
    for rule in all_rules() {
        let id = rule.info().id;
        if !id.starts_with("IMI-") {
            continue;
        }
        for param in rule.info().params.iter() {
            if param.kind == ParamKind::Level
                && profile.settings(id).level(param.name) == Level::Off
            {
                profile
                    .apply_override(&format!("{id}.{}=P1", param.name))
                    .unwrap();
            }
        }
    }
    profile
}

/// The parameter whose level a violation of an IMI rule carries.
fn param(violation: &Violation) -> &'static str {
    match violation.detail {
        Detail::Param(name) => name,
        other => panic!("{}: {other:?}", violation.rule),
    }
}

/// The cases of the IMI rules a check of the whole score reports: (rule, parameter, tick, voice,
/// marked note).
fn imi(checker: &Checker, score: &Score) -> Vec<(&'static str, &'static str, u32, usize, usize)> {
    checker
        .check(score)
        .violations
        .iter()
        .filter(|v| v.rule.starts_with("IMI-"))
        .map(|v| (v.rule, param(v), v.tick, v.voice, v.note))
        .collect()
}

/// The IMI cases of a score window by window and in its growing prefixes, as the search finds
/// them, against those of the whole score: complete windows as in the final check, prefixes of
/// the voices up to each onset with the window since the onset before.
fn decided_in_time(checker: &Checker, score: &Score, name: &str) {
    let only = |mut found: Vec<Violation>| {
        found.retain(|v| v.rule.starts_with("IMI-"));
        found.sort_by_key(|v| (v.tick, v.voice, v.rule, v.note));
        found
    };
    let analysis = Analysis::new(score);
    let ticks: Vec<u32> = analysis.slices.iter().map(|s| s.tick).collect();
    let targets = vec![true; score.voices.len()];
    let window = |k: usize, complete: bool| Window {
        from: if k == 0 { 0 } else { ticks[k - 1] + 1 },
        until: if k + 1 == ticks.len() {
            u32::MAX
        } else {
            ticks[k]
        },
        complete,
    };
    let whole = only(checker.check(score).violations);
    let mut found = Vec::new();
    for k in 0..ticks.len() {
        checker.check_window(score, &analysis, window(k, true), &targets, &mut found);
    }
    assert_eq!(only(found), whole, "{name}: windows");
    let mut open = Vec::new();
    let all = Window {
        from: 0,
        until: u32::MAX,
        complete: false,
    };
    checker.check_window(score, &analysis, all, &targets, &mut open);
    let mut found = Vec::new();
    for (k, &tick) in ticks.iter().enumerate() {
        let mut part = score.clone();
        for voice in &mut part.voices {
            let keep = voice
                .timed()
                .take_while(|(start, _)| *start <= tick)
                .count();
            voice.notes.truncate(keep);
        }
        let analysis = Analysis::new(&part);
        checker.check_window(&part, &analysis, window(k, false), &targets, &mut found);
    }
    assert_eq!(only(found), only(open), "{name}: prefixes");
}

/// Whether a parameter of an IMI rule judges scores without a plan (phase 14).
fn unplanned(name: &str) -> bool {
    name.starts_with("unplanned_")
}

/// Every parameter of the IMI rules, in a minimal score: the case is reported once, by its rule
/// and parameter, at the tick at which it is decided, in its voice and on the note it marks; the
/// setting without it has no such case. Window by window and prefix by prefix the checker finds
/// the same. The parameters for scores without a plan have examples of their own
/// ([`unplanned_params_fire_once`]).
#[test]
fn each_parameter_fires_once() {
    let checker = Checker::new(&every_case());
    let examples = examples();
    for rule in all_rules() {
        let id = rule.info().id;
        for param in rule.info().params.iter() {
            if id.starts_with("IMI-") && param.kind == ParamKind::Level && !unplanned(param.name) {
                assert!(
                    examples
                        .iter()
                        .any(|e| (e.rule, e.param) == (id, param.name)),
                    "no example for {id}.{}",
                    param.name
                );
            }
        }
    }
    for e in &examples {
        let case = (e.rule, e.param);
        let with = e.with.score();
        let found = imi(&checker, &with);
        let hits: Vec<(u32, usize, usize)> = found
            .iter()
            .filter(|f| (f.0, f.1) == case)
            .map(|f| (f.2, f.3, f.4))
            .collect();
        assert_eq!(hits, [e.at], "{case:?} in {:?}: {found:?}", e.with.voices);
        let without = e.without.score();
        let found = imi(&checker, &without);
        assert!(
            found.iter().all(|f| (f.0, f.1) != case),
            "{case:?} in {:?}: {found:?}",
            e.without.voices
        );
        decided_in_time(&checker, &with, &format!("{case:?} with"));
        decided_in_time(&checker, &without, &format!("{case:?} without"));
    }
}

/// The settings the examples start from carry out their plans without a case.
#[test]
fn planned_settings_have_no_case() {
    let checker = Checker::new(&every_case());
    for setting in [
        duo(ALTO),
        reentry("d'1 r1 d'1 d'2 e'2 f'2 e'2 d'1", 16, ALTO_HEAD),
        answer("r1 d'1 g'1 f'2 e'2 d'1 d'1", "d'1 g'1 f'2 e'2 d'1"),
        solo(kyrie(), SOPRANO, SOPRANO_HEAD),
        two_points("dorian", SOPRANO_ON, ALTO_TAIL, "d'1 f'2 e'2 d'1", 5),
    ] {
        let found = imi(&checker, &setting.score());
        assert_eq!(found, [], "{:?}", setting.voices);
    }
}

/// Without a plan the rules report nothing, whatever the notes, but the cases of the parameters
/// for scores without one; a plan without points is none. With those parameters off, as in the
/// standard profile, the rules are silent.
#[test]
fn imi_rules_are_silent_without_a_plan() {
    let checker = Checker::new(&every_case());
    let mut planned = every_case();
    for rule in all_rules() {
        let id = rule.info().id;
        for param in rule.info().params.iter().filter(|p| unplanned(p.name)) {
            planned
                .apply_override(&format!("{id}.{}=aus", param.name))
                .unwrap();
        }
    }
    let quiet = Checker::new(&planned);
    for e in examples() {
        let mut score = e.with.score();
        for imitation in [None, Some(Arc::new(ScoreImitation { points: Vec::new() }))] {
            score.imitation = imitation;
            assert_eq!(imi(&quiet, &score), [], "{:?}", e.with.voices);
            let found = imi(&checker, &score);
            assert!(
                found.iter().all(|f| unplanned(f.1)),
                "{:?}: {found:?}",
                e.with.voices
            );
        }
    }
}

/// A setting without a plan of imitation (a plan without points) of free voices in the Dorian
/// mode, without a text, as a part of a mass that begins together.
fn planless(voices: &[&'static str]) -> Setting {
    Setting {
        mode: "dorian",
        text: None,
        voices: voices.to_vec(),
        points: Vec::new(),
    }
}

/// The soprano, the alto and the tenor of the examples of IMI-HELD `struck` and `passing`, and
/// the four voices of IMI-SILENCE `thin` with a bass that enters as given.
const STRUCK: [&str; 3] = [
    "a'1 a'2 b'2 c''2 b'2 a'1 g'1 e'1 f'1 e'1",
    "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1~ c'2 e'2 d'1",
    "r1 r1 r1 r1 r1 r1 r1 a1",
];
const RESOLVED: [&str; 3] = [
    "a'1 a'2 b'2 c''2 b'2 a'1 g'1 e'1 f'1 e'1",
    "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1~ c'2 b2 c'1",
    "r1 r1 r1 r1 r1 r1 r1 a1",
];
const LEAPT: [&str; 3] = [
    "a'1 a'2 b'2 c''2 b'2 a'1 g'1 e'1 f'4 c''4 b'4 a'4 g'1",
    "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1 a1 b1",
    "r1 r1 r1 r1 r1 d1~ d1 g1",
];
const PASSED: [&str; 3] = [
    "a'1 a'2 b'2 c''2 b'2 a'1 g'1 e'1 f'4 e'4 d'4 e'4 f'1",
    "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1 a1 b1",
    "r1 r1 r1 r1 r1 d1~ d1 g1",
];

/// Four voices without a plan: soprano and alto as given, the tenor in measure 3, and the bass.
fn planless_quartet(soprano: &'static str, alto: &'static str, bass: &'static str) -> Setting {
    planless(&[
        soprano,
        alto,
        "r1 r1 a1 a2 b2 c'2 b2 a1 g1 a1 a1 f1 e1 d1",
        bass,
    ])
}

/// The soprano rests in measure 9, the alto sings on.
const RESTING: &str = "a'1 a'2 b'2 c''2 b'2 a'1 g'1 a'1 g'1 f'1 r1 e'1 d'1 e'1";
const SINGING: &str = "a'1 a'2 b'2 c''2 b'2 a'1 g'1 a'1 g'1 f'1 e'1 e'1 d'1 e'1";
const ALTO_ON: &str = "r1 d'1 d'2 e'2 f'2 e'2 d'1 c'1 d'1 c'1 c'1 c'1 a1 a1";

/// One or more examples for every parameter for scores without a plan (phase 14): the cases of
/// their counterparts with a plan.
fn unplanned_examples() -> Vec<Example> {
    vec![
        // The alto's C4, tied over measure 7, is the bass while the tenor rests: the soprano's
        // F4 a fourth above it on the downbeat, the alto rising to E4. Stepping down to B3, the
        // tied note is the suspension.
        example(
            ("IMI-HELD", "unplanned_struck"),
            planless(&STRUCK),
            (52, 0, 8),
            planless(&RESOLVED),
        ),
        // The tenor's D3, tied over measure 7, is no suspension: the soprano leaps into C5, a
        // seventh above it; moving by step, E5 passes.
        example(
            ("IMI-HELD", "unplanned_passing"),
            planless(&LEAPT),
            (50, 0, 9),
            planless(&PASSED),
        ),
        // Soprano and bass rest in measure 9, after every voice has sung: half of the four
        // voices sound there; singing on, they are free.
        example(
            ("IMI-SILENCE", "unplanned_thin"),
            planless_quartet(
                RESTING,
                ALTO_ON,
                "r1 r1 r1 d1 d2 e2 f2 e2 d1 c1 r1 f1 g1 d1",
            ),
            (64, 0, 10),
            planless_quartet(
                SINGING,
                ALTO_ON,
                "r1 r1 r1 d1 d2 e2 f2 e2 d1 c1 d1 f1 g1 d1",
            ),
        ),
        // Before the bass has sung, half of the choir may rest: the exposition is not over.
        example(
            ("IMI-SILENCE", "unplanned_thin"),
            planless_quartet(
                RESTING,
                ALTO_ON,
                "r1 r1 r1 d1 d2 e2 f2 e2 d1 c1 r1 f1 g1 d1",
            ),
            (64, 0, 10),
            planless_quartet(RESTING, ALTO_ON, "r1 r1 r1 r1 r1 r1 r1 r1 r1 f1 g1 d1"),
        ),
    ]
}

/// The parameters for scores without a plan (phase 14), each in a minimal score without one:
/// the case is reported once, at the tick at which it is decided, in its voice and on the note it
/// marks, both without a plan and with a plan without points; the setting without it has no
/// such case. Window by window and prefix by prefix the checker finds the same.
#[test]
fn unplanned_params_fire_once() {
    let checker = Checker::new(&every_case());
    let examples = unplanned_examples();
    for rule in all_rules() {
        let id = rule.info().id;
        for param in rule.info().params.iter() {
            if id.starts_with("IMI-") && param.kind == ParamKind::Level && unplanned(param.name) {
                assert!(
                    examples
                        .iter()
                        .any(|e| (e.rule, e.param) == (id, param.name)),
                    "no example for {id}.{}",
                    param.name
                );
            }
        }
    }
    let both = |setting: &Setting| {
        let empty = setting.score();
        let mut none = empty.clone();
        none.imitation = None;
        [empty, none]
    };
    for e in &examples {
        let case = (e.rule, e.param);
        for (k, with) in both(&e.with).iter().enumerate() {
            let found = imi(&checker, with);
            let hits: Vec<(u32, usize, usize)> = found
                .iter()
                .filter(|f| (f.0, f.1) == case)
                .map(|f| (f.2, f.3, f.4))
                .collect();
            assert_eq!(hits, [e.at], "{case:?} in {:?}: {found:?}", e.with.voices);
            decided_in_time(&checker, with, &format!("{case:?} with ({k})"));
        }
        for (k, without) in both(&e.without).iter().enumerate() {
            let found = imi(&checker, without);
            assert!(
                found.iter().all(|f| (f.0, f.1) != case),
                "{case:?} in {:?}: {found:?}",
                e.without.voices
            );
            decided_in_time(&checker, without, &format!("{case:?} without ({k})"));
        }
    }
}

/// Without a plan IMI-HELD finds exactly the cases it finds with one, under the parameters for
/// scores without a plan, and IMI-SILENCE finds half of the choir resting from every voice's
/// first note on as it does from the last first entry of a plan, where those are the same: the
/// unplanned parameters judge as their counterparts do.
#[test]
fn planless_cases_are_those_of_a_plan() {
    let checker = Checker::new(&every_case());
    let renamed = |param: &'static str| match param {
        "struck" => "unplanned_struck",
        "passing" => "unplanned_passing",
        "thin" => "unplanned_thin",
        other => other,
    };
    let (mut held, mut thin) = (0, 0);
    for e in examples() {
        for setting in [&e.with, &e.without] {
            let planned = setting.score();
            let plan = planned.imitation.as_deref().unwrap();
            let firsts: Vec<Option<u32>> = planned
                .voices
                .iter()
                .map(|voice| voice.soundings().first().map(|s| s.start))
                .collect();
            let entered = (0..planned.voices.len())
                .map(|v| {
                    let entries = plan.points.iter().flat_map(|p| &p.entries);
                    entries.filter(|e| e.voice == v).map(|e| e.start).min()
                })
                .collect::<Vec<_>>();
            let rules: &[&str] = if entered == firsts {
                &["IMI-HELD", "IMI-SILENCE"]
            } else {
                &["IMI-HELD"]
            };
            let judged = |score: &Score| -> Vec<(&'static str, &'static str, u32, usize, usize)> {
                imi(&checker, score)
                    .into_iter()
                    .filter(|f| {
                        rules.contains(&f.0)
                            && (matches!(f.1, "struck" | "passing" | "thin") || unplanned(f.1))
                    })
                    .map(|f| (f.0, renamed(f.1), f.2, f.3, f.4))
                    .collect()
            };
            let mut planless = planned.clone();
            planless.imitation = None;
            let with_plan = judged(&planned);
            let without = judged(&planless);
            assert_eq!(without, with_plan, "{:?}", setting.voices);
            held += with_plan.iter().filter(|f| f.0 == "IMI-HELD").count();
            thin += with_plan.iter().filter(|f| f.0 == "IMI-SILENCE").count();
        }
    }
    assert!(held >= 2 && thin >= 1, "{held} held, {thin} thin");
}

/// The parameters for scores without a plan are off in Fux's profile and in Jeppesen's, read and
/// written in TOML and set by `--rule` like every other, and listed with the rule's parameters
/// and their descriptions, as `fux rules` shows them.
#[test]
fn unplanned_params_in_the_profile() {
    use crate::lang::Lang;

    let fux = Profile::fux();
    let toml = fux.to_toml(None);
    for (id, param, counterpart) in [
        ("IMI-HELD", "unplanned_struck", "struck"),
        ("IMI-HELD", "unplanned_passing", "passing"),
        ("IMI-SILENCE", "unplanned_thin", "thin"),
    ] {
        assert_eq!(fux.settings(id).level(param), Level::Off, "{param}");
        assert_eq!(Profile::jeppesen().settings(id).level(param), Level::Off);
        assert!(toml.contains(&format!("{param} = \"aus\"")), "{param}");
        let rule = all_rules().into_iter().find(|r| r.info().id == id).unwrap();
        let info = rule.info().param(param).unwrap();
        assert_eq!(info.kind, ParamKind::Level);
        assert!(info.de.contains("ohne Imitationsplan"), "{}", info.de);
        assert!(info.en.contains("without a plan"), "{}", info.en);
        // The overlay of the mass: the level of the counterpart with a plan.
        let level = fux.settings(id).level(counterpart);
        assert_ne!(level, Level::Off);
        let overlay = Profile::fux()
            .merged_toml(&format!(
                "[{id}]\n{param} = \"{}\"\n",
                level.label(Lang::De)
            ))
            .unwrap();
        assert_eq!(overlay.settings(id).level(param), level);
        assert_eq!(
            overlay.to_toml(Some(&fux)),
            format!(
                "name = \"fux\"\n\n[{id}]\n{param} = \"{}\"\n",
                level.label(Lang::De)
            )
        );
        let mut set = Profile::fux();
        set.apply_override(&format!("{id}.{param}=P2")).unwrap();
        assert_eq!(set.settings(id).level(param), Level::P2);
    }
}

/// The syllables are compared only where both voices sing the text: not without a text, not in
/// voices that do not sing it, as in the scores of the search. The notes still count.
#[test]
fn syllables_only_where_both_voices_sing() {
    let checker = Checker::new(&every_case());
    let texts = |score: &Score| {
        imi(&checker, score)
            .into_iter()
            .filter(|f| f.1 == "text")
            .count()
    };
    let setting = duo("r1 d'1=0 d'2=1 e'2=2 f'2=3 e'2 d'1=5 c'2=4 d'2=6");
    let mut score = setting.score();
    assert_eq!(texts(&score), 1);
    score.text.as_mut().unwrap().voices = vec![true, false];
    assert_eq!(texts(&score), 0);
    score.text.as_mut().unwrap().voices = vec![false, true];
    assert_eq!(texts(&score), 0);
    score.text = None;
    assert_eq!(texts(&score), 0);
    // A syllable that matches none of the text may be any.
    let mut score = setting.score();
    score.voices[1].notes[6].lyric = Some(crate::score::Lyric::UNKNOWN);
    assert_eq!(texts(&score), 0);
    // A moved head note is found without a text too.
    let mut score = duo("r1 d'1=0 d'2=1 f'2=2 f'2=3 e'2 d'1=4 c'2=5 d'2=6").score();
    score.text = None;
    assert_eq!(imi(&checker, &score), [("IMI-ENTRY", "level", 20, 1, 3)]);
}

/// The levels of the standard profile, and messages that name the case.
#[test]
fn levels_and_messages() {
    use crate::lang::Lang;

    let checker = Checker::new(&Profile::fux());
    let score = reentry("d'1 c'1 d'1 d'2 e'2 f'2 e'2 d'1", 16, ALTO_HEAD).score();
    let report = checker.check(&score);
    let rest = report
        .violations
        .iter()
        .find(|v| v.rule == "IMI-REST")
        .unwrap();
    assert_eq!(rest.level, Level::Forbidden);
    assert_eq!(rest.other, None);
    assert_eq!(
        checker.message(rest, Lang::De),
        "Einsatz ohne Pause oder zwischen den halben Schlägen: Wiedereinsatz ohne Pause oder nach \
         einer Pause kürzer als eine Halbe"
    );
    assert_eq!(
        checker.message(rest, Lang::En),
        "Entry without a rest, or off the minim beats: a re-entry without a rest, or after a rest \
         shorter than a minim"
    );
    let levels = |setting: Setting| -> Vec<(&'static str, &'static str, Level)> {
        checker
            .check(&setting.score())
            .violations
            .iter()
            .filter(|v| v.rule.starts_with("IMI-"))
            .map(|v| (v.rule, param(v), v.level))
            .collect()
    };
    assert_eq!(
        levels(duo("r1 d'1=0 d'2=1 e'2=2 f'4=3 g'4 e'2 d'1=4 c'2=5 d'2=6")),
        [("IMI-ENTRY", "rhythm", Level::P1)]
    );
    assert_eq!(
        levels(answer("r1 d'1 a'1 g'2 f'2 e'1 d'1", "d'1 a'1 g'2 f'2 e'1")),
        [("IMI-ANSWER", "modal", Level::P1)]
    );
    // An entry five measures late is also one after the leader's head.
    let late = Setting {
        voices: vec![SOPRANO_LONG, "r1 r1 r1 r1 r1 d'1 d'2 e'2 f'2 e'2 d'1 d'1"],
        ..answered("", ALTO_HEAD, 40)
    };
    assert_eq!(
        levels(late),
        [
            ("IMI-TIME", "second", Level::P3),
            ("IMI-TIME", "apart", Level::P2),
        ]
    );
    // The Jeppesen profile takes them from Fux's.
    let jeppesen = Profile::jeppesen();
    for rule in all_rules()
        .iter()
        .filter(|r| r.info().id.starts_with("IMI-"))
    {
        let id = rule.info().id;
        assert_eq!(jeppesen.settings(id), Profile::fux().settings(id), "{id}");
    }
}
