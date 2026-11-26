//! Helpers for the unit tests of the text underlay and of its rules: voices in LilyPond notation
//! with the syllables they begin, texts, and random voices and texts.

use crate::generate::Rng;
use crate::parse::parse_lilypond;
use crate::pitch::Pitch;
use crate::rhythm::{self, Place};
use crate::score::{Lyric, Note, Species};
use crate::text::{Text, TextKind, TextOptions, ordinary};
use crate::time::Duration;

/// Notes in LilyPond notation, each with the index of the syllable it begins after `=` (`?` for
/// a syllable that matches none): `d'2=0 e'4 f'4=1`.
pub(crate) fn sung(src: &str) -> Vec<Note> {
    let mut lyrics = Vec::new();
    let tokens: Vec<&str> = src
        .split_whitespace()
        .map(|token| match token.split_once('=') {
            Some((note, "?")) => {
                lyrics.push(Some(Lyric::UNKNOWN));
                note
            }
            Some((note, syllable)) => {
                lyrics.push(Some(Lyric::new(syllable.parse().unwrap())));
                note
            }
            None => {
                lyrics.push(None);
                token
            }
        })
        .collect();
    let mut notes = parse_lilypond(&tokens.join(" "), Duration::HALF).unwrap();
    assert_eq!(notes.len(), lyrics.len(), "{src}");
    for (note, lyric) in notes.iter_mut().zip(lyrics) {
        note.lyric = lyric;
    }
    notes
}

/// An own text.
pub(crate) fn own(src: &str) -> Text {
    Text::own(src, &TextOptions::default()).unwrap().0
}

/// A section of the Ordinary, with eleison in four syllables.
pub(crate) fn section(id: &str) -> Text {
    ordinary::section(id, &TextOptions::default())
        .unwrap()
        .clone()
}

/// A number below `n`.
pub(crate) fn below(rng: &mut Rng, n: usize) -> usize {
    (rng.next_u64() % n as u64) as usize
}

/// A random voice of `measures` measures in Palestrina's measure rhythms, the frequent ones more
/// often, with steps, leaps and repeated notes; with `rests` also rests, with `held` a last
/// measure that holds one note.
pub(crate) fn random_voice(rng: &mut Rng, measures: usize, rests: bool, held: bool) -> Vec<Note> {
    const MOVES: [i32; 20] = [
        0, 0, 0, 1, 1, 1, 1, -1, -1, -1, -1, 2, -2, 2, -2, 3, -3, 4, -5, 7,
    ];
    let mut notes: Vec<Note> = Vec::new();
    let mut tied = false;
    let mut diatonic = 29;
    for m in 0..measures {
        let last = m + 1 == measures;
        let place = if last { Place::Last } else { Place::Inner };
        let patterns: Vec<&str> = rhythm::patterns(Species::Free, place, true)
            .iter()
            .map(|(pattern, _)| *pattern)
            .filter(|p| p.starts_with('⁀') == tied)
            .filter(|p| rests || !p.contains('r'))
            .filter(|p| !held || !last || matches!(*p, "1" | "⁀1"))
            .collect();
        let pick = below(rng, patterns.len()).min(below(rng, patterns.len()));
        let pattern = patterns[pick];
        for token in rhythm::tokens(pattern) {
            let duration = Duration::from_ticks(token.ticks);
            if token.tied_in {
                let held = notes.last_mut().unwrap();
                held.tie = true;
                let note = Note::new(held.pitch.unwrap(), duration);
                notes.push(note);
            } else if token.rest {
                notes.push(Note::rest(duration));
            } else {
                diatonic = (diatonic + MOVES[below(rng, MOVES.len())]).clamp(24, 36);
                notes.push(Note::new(Pitch::from_diatonic(diatonic), duration));
            }
        }
        tied = pattern.ends_with('⁀');
    }
    notes
}

/// A random own text of at most `max` syllables, with breaks, tails and punctuation; now and then
/// a long one.
pub(crate) fn random_text(rng: &mut Rng, max: usize) -> Text {
    const WORDS: [&str; 12] = [
        "Ký-ri-e",
        "e-lé-i-son",
        "De-us",
        "pax",
        "et",
        "Dó-mi-ne",
        "san-ctus",
        "in",
        "te",
        "lau-dá-mus",
        "Á-men",
        "qui",
    ];
    const GAPS: [&str; 7] = [" ", " ", " | ", " / ", ", ", "; ", ". "];
    let mut src = String::new();
    let mut count = 0;
    loop {
        let word = WORDS[below(rng, WORDS.len())];
        let n = word.split('-').count();
        if count + n > max {
            if count > 0 {
                break;
            }
            continue;
        }
        if count > 0 {
            src.push_str(GAPS[below(rng, GAPS.len())]);
        }
        src.push_str(word);
        count += n;
        if below(rng, 3) == 0 {
            break;
        }
    }
    src.push('.');
    let mut text = own(&src);
    if below(rng, 4) == 0 {
        text.kind = TextKind::Long;
    }
    text
}
