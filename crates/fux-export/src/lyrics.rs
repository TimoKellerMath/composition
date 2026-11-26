//! How the syllables of a voice fall on its notes, for the lyrics in the LilyPond and MusicXML
//! output.
//!
//! The syllables follow the convention of the GABC reader: a syllable ending in `-` belongs to a
//! word that goes on, any other syllable ends its word, and a note without a syllable continues
//! the syllable before it as a melisma. A rest ends a syllable; tied notes count as one note.

use fux_core::Note;

/// Whether the word goes on after the syllable, i.e. it ends in a hyphen.
pub(crate) fn word_goes_on(syllable: &str) -> bool {
    syllable.len() > 1 && syllable.ends_with('-')
}

/// An end of the extender line under a melisma on a word-final syllable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Extend {
    /// The note with the syllable.
    Start,
    /// The last note sung to the syllable: its last attack or the last tie continuation after
    /// it, so that the line reaches under the held note.
    Stop,
}

/// For each note of a voice, whether an extender line starts or stops there: a word-final
/// syllable gets one when further notes continue it, attacks or a tie, as editions draw the
/// line under a held word end. Within a word, the hyphen to the next syllable shows the
/// melisma.
pub(crate) fn extenders(notes: &[Note]) -> Vec<Option<Extend>> {
    let mut ends = vec![None; notes.len()];
    let mut close = |melisma: Option<(usize, usize)>| {
        if let Some((first, last)) = melisma
            && last > first
        {
            ends[first] = Some(Extend::Start);
            ends[last] = Some(Extend::Stop);
        }
    };
    // The note with the word-final syllable sung now, and the last note sung to it.
    let mut melisma = None;
    for (i, note) in notes.iter().enumerate() {
        match &note.syllable {
            _ if note.is_rest() => close(melisma.take()),
            Some(syllable) => {
                close(melisma.take());
                melisma = (!word_goes_on(syllable)).then_some((i, i));
            }
            None => {
                if let Some((_, last)) = &mut melisma {
                    *last = i;
                }
            }
        }
    }
    close(melisma);
    ends
}

#[cfg(test)]
mod tests {
    use super::*;
    use fux_core::{Duration, parse_lilypond};

    /// Notes from LilyPond input, with one syllable or `_` per note, rests and tied notes
    /// included.
    fn texted(music: &str, syllables: &str) -> Vec<Note> {
        let mut notes = parse_lilypond(music, Duration::WHOLE).unwrap();
        let syllables: Vec<&str> = syllables.split_whitespace().collect();
        assert_eq!(notes.len(), syllables.len());
        for (note, syllable) in notes.iter_mut().zip(syllables) {
            note.syllable = (syllable != "_").then(|| syllable.to_string());
        }
        notes
    }

    fn ends(notes: &[Note]) -> Vec<(usize, Extend)> {
        extenders(notes)
            .into_iter()
            .enumerate()
            .filter_map(|(i, e)| e.map(|e| (i, e)))
            .collect()
    }

    #[test]
    fn a_word_final_melisma_gets_a_line() {
        let notes = texted("d'2 e'2 f'4 g'4 a'2 d'1", "Ký- ri- e _ _ son.");
        assert_eq!(ends(&notes), [(2, Extend::Start), (4, Extend::Stop)]);
    }

    #[test]
    fn a_melisma_within_a_word_gets_none() {
        let notes = texted("d'2 e'4 f'4 g'1", "Ký- _ ri- e");
        assert!(ends(&notes).is_empty());
    }

    #[test]
    fn the_line_reaches_under_tied_notes() {
        // A word end held by a tie alone gets a line; a melisma may begin with a tied note.
        let notes = texted("d'2 e'2~ e'2 f'2~ f'2 g'2 d'1", "Ký- e _ son. _ _ _");
        assert_eq!(
            ends(&notes),
            [
                (1, Extend::Start),
                (2, Extend::Stop),
                (3, Extend::Start),
                (6, Extend::Stop)
            ]
        );
        // A melisma that ends in a tied note stops the line on the tie continuation.
        let notes = texted("d'2 e'2 f'2~ f'2 g'1", "Ký- e _ _ son.");
        assert_eq!(ends(&notes), [(1, Extend::Start), (3, Extend::Stop)]);
        // Within a word a tie draws no line.
        let notes = texted("d'2~ d'2 e'1", "Ký- _ e");
        assert!(ends(&notes).is_empty());
    }

    #[test]
    fn a_rest_ends_the_melisma() {
        let notes = texted("d'2 e'2 r2 f'2 g'1", "e _ _ _ son.");
        assert_eq!(ends(&notes), [(0, Extend::Start), (1, Extend::Stop)]);
        // Notes after the rest without a syllable continue nothing; neither does a syllable on a
        // rest.
        let notes = texted("d'2 r2 f'2 g'2 d'1", "e son. _ _ _");
        assert!(ends(&notes).is_empty());
    }
}
