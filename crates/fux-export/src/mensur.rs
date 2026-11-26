//! Mensurstriche (phase 14, docs/design/phase-14/spec.md § 6): editions of Renaissance polyphony
//! draw the barlines between the staves only, so that a note held over a barline needs no tie
//! and is written as one note, as the sources write it.
//!
//! A note tied over a barline to the same pitch is merged with the note after it where their
//! values together have a single written form (1, 2, 3, 4, 6, 8, 12, 16 or 24 ticks: an eighth
//! up to a dotted breve), step by step along a chain of ties: a half from the middle of a
//! measure, tied to a whole and that to a half, becomes a dotted whole, then a breve
//! (LY-MENSUR-MERGE in docs/design/phase-14/research/research_notation.json).
//!
//! LilyPond breaks a line only at a barline that no note crosses, and gives no warning where a
//! system has no such barline and runs off the page. So not every tie that could be merged is
//! (LY-MENSUR-BREAKS, with the correction of verify_notation.json): the barlines of a score are
//! walked in order, and with `run` the number of barlines in a row at which notes were merged,
//! - a barline that no tie crosses can break, `run` is 0;
//! - at a crossed barline where `run + 1` reaches [`BREAK_RUN`] every tie is kept, so that the
//!   line can break there, and `run` is 0;
//! - at a crossed barline where no tie can be merged, every tie is kept, `run` is 0;
//! - otherwise the ties that can be merged are, the others kept, and `run` grows by one.
//!
//! A tie into a note with a fermata, the last note of a section, is kept too: written as one with
//! the note before it, the note would carry its fermata where it begins, a measure or half a
//! measure before the fermatas of the voices whose last notes begin at the last barline.
//!
//! With the default style of LilyPond an accidental holds to the next barline, which no longer
//! shows in the staff: after a merged note with an alteration other than the scale's, in the
//! staff or above it as musica ficta, the next note of the same step and octave in the measure
//! where it ends takes its accidental again as a cautionary one (`?`) where it sings the step of
//! the scale and is no ficta, which LilyPond would not mark (LY-MENSUR-ACCIDENTALS, option B): the
//! plain notation shows that accidental, as the tie carries the alteration over the barline.

use std::ops::RangeInclusive;

use fux_core::{Duration, Note, Score};

/// K of the rule above: of [`BREAK_RUN`] crossed barlines in a row, at least one keeps its ties,
/// so that LilyPond can break the line there. Measured on the examples 01, 02, 03, 07 and 08 as set
/// in Phase 13, in 2/2: 104 of 125 ties merged (106 of 128 in those set anew on 5.11.2026), 10 to
/// 13 measures in a system of four voices on A4.
pub const BREAK_RUN: usize = 4;

/// A note as Mensurstriche print it: one note of a voice, or several tied over barlines to the
/// same pitch and written as one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    /// The note as written: pitch, syllable and ficta of the first note, the value of all, the
    /// tie of the last.
    pub note: Note,
    /// Its start in ticks.
    pub start: u32,
    /// The notes of the voice it stands for, by their indices.
    pub notes: RangeInclusive<usize>,
    /// It gets a cautionary accidental: it follows a merged note with an alteration other than
    /// the scale's (in the staff, or above it as musica ficta) on its step and in its octave,
    /// within the measure where that note ends.
    pub cautionary: bool,
}

impl Shown {
    /// Where it ends, in ticks.
    pub fn end(&self) -> u32 {
        self.start + self.note.duration.ticks()
    }
}

/// The notes of a score as Mensurstriche print them ([`merge_ties`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mensur {
    /// Per voice of the score, its notes as printed.
    pub voices: Vec<Vec<Shown>>,
    /// Ties over a barline that are written as one note.
    pub merged: usize,
    /// Ties over a barline that stay: where the values have no single written form together,
    /// and where a line may break.
    pub kept: usize,
}

impl Mensur {
    /// Every note of `score` as it is, without merging a tie: the notes as printed with bar
    /// lines through the staves.
    pub fn plain(score: &Score) -> Mensur {
        let voices = score
            .voices
            .iter()
            .map(|voice| {
                voice
                    .timed()
                    .enumerate()
                    .map(|(n, (start, note))| Shown {
                        note: note.clone(),
                        start,
                        notes: n..=n,
                        cautionary: false,
                    })
                    .collect()
            })
            .collect();
        Mensur {
            voices,
            merged: 0,
            kept: 0,
        }
    }

    /// The index of the printed note of voice `voice` that stands for its note `note`.
    pub fn shown(&self, voice: usize, note: usize) -> Option<usize> {
        let shown = self.voices.get(voice)?;
        let i = shown.partition_point(|s| *s.notes.end() < note);
        shown.get(i).filter(|s| s.notes.contains(&note)).map(|_| i)
    }
}

/// The notes of `score` with the ties over its barlines merged as the module documentation says,
/// but a tie into a note that ends at one of the ticks `held`, where the notes take a fermata:
/// per voice the notes as printed, and how many ties were merged and kept.
pub fn merge_ties(score: &Score, held: &[u32]) -> Mensur {
    let per = score.meter.ticks_per_measure();
    let timed: Vec<Vec<(u32, &Note)>> = score.voices.iter().map(|v| v.timed().collect()).collect();
    // Per voice and note: whether it is written together with the note before it.
    let mut joined: Vec<Vec<bool>> = timed.iter().map(|notes| vec![false; notes.len()]).collect();
    // The ties over barlines, in the order of their barlines: (tick, voice, index of the note
    // that ends there tied to the same pitch). A tie always has a note after it, so none lies at
    // the end.
    let mut crossing: Vec<(u32, usize, usize)> = Vec::new();
    for (v, notes) in timed.iter().enumerate() {
        for (n, &(start, note)) in notes.iter().enumerate() {
            let end = start + note.duration.ticks();
            let tied = note.tie
                && note.pitch.is_some()
                && notes
                    .get(n + 1)
                    .is_some_and(|&(_, next)| next.pitch == note.pitch);
            if tied && end.is_multiple_of(per) {
                crossing.push((end, v, n));
            }
        }
    }
    crossing.sort_unstable();
    let (mut merged, mut kept, mut run) = (0, 0, 0);
    let mut next = 0;
    let measures = score.measure_count() as u32;
    for barline in (1..measures).map(|m| m * per) {
        // Ties that end where no barline is walked, as in a score that does not validate, are
        // passed over.
        while crossing
            .get(next)
            .is_some_and(|&(tick, _, _)| tick < barline)
        {
            next += 1;
        }
        let first = next;
        while crossing
            .get(next)
            .is_some_and(|&(tick, _, _)| tick == barline)
        {
            next += 1;
        }
        let here = &crossing[first..next];
        if here.is_empty() {
            run = 0;
            continue;
        }
        // The ties whose notes, as written so far, take the next note into one value, where it
        // has no fermata.
        let mergeable: Vec<(usize, usize)> = here
            .iter()
            .map(|&(_, v, n)| (v, n))
            .filter(|&(v, n)| {
                let so_far = written(&timed[v], &joined[v], n);
                let (start, next) = timed[v][n + 1];
                let ticks = next.duration.ticks();
                !held.contains(&(start + ticks))
                    && Duration::from_ticks(so_far + ticks).lilypond().is_some()
            })
            .collect();
        if mergeable.is_empty() || run + 1 >= BREAK_RUN {
            kept += here.len();
            run = 0;
            continue;
        }
        for &(v, n) in &mergeable {
            joined[v][n + 1] = true;
        }
        merged += mergeable.len();
        kept += here.len() - mergeable.len();
        run += 1;
    }
    let voices = timed
        .iter()
        .zip(&joined)
        .map(|(notes, joined)| {
            let mut shown = shown_notes(notes, joined);
            mark_cautionary(score, &mut shown);
            shown
        })
        .collect();
    Mensur {
        voices,
        merged,
        kept,
    }
}

/// The ticks of the note as written that ends with note `n` of a voice: `n` and the notes before
/// it that it is written together with.
fn written(notes: &[(u32, &Note)], joined: &[bool], n: usize) -> u32 {
    let mut first = n;
    while first > 0 && joined[first] {
        first -= 1;
    }
    notes[first..=n]
        .iter()
        .map(|(_, note)| note.duration.ticks())
        .sum()
}

/// The notes of a voice as written, each note joined to the one before it where `joined` says.
fn shown_notes(notes: &[(u32, &Note)], joined: &[bool]) -> Vec<Shown> {
    let mut shown: Vec<Shown> = Vec::with_capacity(notes.len());
    for (n, &(start, note)) in notes.iter().enumerate() {
        match shown.last_mut() {
            Some(last) if joined[n] => {
                last.note.duration =
                    Duration::from_ticks(last.note.duration.ticks() + note.duration.ticks());
                last.note.tie = note.tie;
                last.notes = *last.notes.start()..=n;
            }
            _ => shown.push(Shown {
                note: note.clone(),
                start,
                notes: n..=n,
                cautionary: false,
            }),
        }
    }
    shown
}

/// Marks the notes of a voice that take a cautionary accidental: after each merged note with an
/// alteration other than the scale's, whether its accidental stands in the staff or above it as
/// musica ficta, the first note of its step and octave in the measure where it ends, if that note
/// sings the step of the scale and is no ficta. A note of that step with another alteration
/// shows its accidental anyway.
fn mark_cautionary(score: &Score, shown: &mut [Shown]) {
    let per = score.meter.ticks_per_measure();
    for i in 0..shown.len() {
        let held = &shown[i];
        let Some(pitch) = held.note.pitch else {
            continue;
        };
        let merged = held.notes.start() != held.notes.end();
        let altered = pitch.alter != score.mode.scale_alter(pitch.step);
        let end = held.end();
        if !merged || !altered || end.is_multiple_of(per) {
            continue;
        }
        let measure_end = (end / per + 1) * per;
        let after = shown[i + 1..]
            .iter()
            .take_while(|s| s.start < measure_end)
            .position(|s| {
                s.note
                    .pitch
                    .is_some_and(|p| p.step == pitch.step && p.octave == pitch.octave)
            });
        if let Some(j) = after.map(|j| i + 1 + j) {
            let next = &shown[j];
            let plain = next
                .note
                .pitch
                .is_some_and(|p| p.alter == score.mode.scale_alter(p.step));
            if plain && !next.note.ficta {
                shown[j].cautionary = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use fux_core::{Mode, ModeSpec, Role, Species, Voice, VoiceType, parse_lilypond};

    use super::*;

    fn score(voices: &[&str]) -> Score {
        let voices = voices
            .iter()
            .map(|src| {
                Voice::new(
                    VoiceType::Tenor,
                    Role::Counterpoint(Species::Free),
                    parse_lilypond(src, Duration::WHOLE).unwrap(),
                )
            })
            .collect();
        let score = Score::new(ModeSpec::new(Mode::Dorian), voices);
        score.validate().unwrap();
        score
    }

    /// The notes of `score` with its ties merged, where no note takes a fermata.
    fn merged(score: &Score) -> Mensur {
        merge_ties(score, &[])
    }

    /// The values of the printed notes of a voice, in ticks.
    fn values(mensur: &Mensur, voice: usize) -> Vec<u32> {
        mensur.voices[voice]
            .iter()
            .map(|s| s.note.duration.ticks())
            .collect()
    }

    /// A half from the middle of a measure tied to a half is a whole; a chain of ties merges
    /// step by step while the value has a single written form.
    #[test]
    fn ties_merge_step_by_step() {
        let mensur = merged(&score(&["d'1 | f'2 g'2~ | g'2 a'2~ | a'1~ | a'2 r2 | d'1"]));
        assert_eq!(values(&mensur, 0), [8, 4, 8, 16, 4, 8]);
        assert_eq!((mensur.merged, mensur.kept), (3, 0));
        let a = &mensur.voices[0][3];
        assert_eq!((a.start, a.notes.clone(), a.note.tie), (20, 4..=6, false));
        assert_eq!(mensur.shown(0, 5), Some(3));
        assert_eq!(mensur.shown(0, 7), Some(4));
        assert_eq!(mensur.shown(0, 9), None);
        // 4 + 8 + 8 = 20 ticks have no single value: the second tie stays.
        let mensur = merged(&score(&["d'2 f'2~ | f'1~ | f'1 | d'1"]));
        assert_eq!(values(&mensur, 0), [4, 12, 8, 8]);
        assert_eq!((mensur.merged, mensur.kept), (1, 1));
        assert!(mensur.voices[0][1].note.tie);
    }

    /// After three barlines in a row with merged notes, the fourth keeps its ties, so that a
    /// line can break there; a barline that no tie crosses resets the count.
    #[test]
    fn every_fourth_crossed_barline_can_break() {
        let held = "d'2 e'2~ | e'2 f'2~ | f'2 g'2~ | g'2 a'2~ | a'2 b'2~ | b'2 c''2 | d''1";
        let mensur = merged(&score(&[held]));
        assert_eq!(values(&mensur, 0), [4, 8, 8, 8, 4, 4, 8, 4, 8]);
        assert_eq!((mensur.merged, mensur.kept), (4, 1));
        let rested = "d'2 e'2~ | e'2 f'2~ | f'2 g'2~ | g'2 a'2 | b'2 c''2~ | c''2 d''2 | d''1";
        let mensur = merged(&score(&[rested]));
        assert_eq!((mensur.merged, mensur.kept), (4, 0));
    }

    /// A barline where no tie can be merged keeps its ties and resets the count, as one that no
    /// tie crosses (the reset case of the rule): a breve from the barline tied to a whole and a
    /// half has no single value at its third barline, so the three crossed barlines after it
    /// merge and the fourth keeps its tie. Without the reset the third barline would count as
    /// merged: the next would keep its tie, the three after it merge, and the last keep its tie
    /// again (5 merged, 3 kept).
    #[test]
    fn an_unmergeable_barline_resets_the_count() {
        let src = "d'1~ | d'1~ | d'1~ | d'2 e'2~ | e'2 f'2~ | f'2 g'2~ | g'2 a'2~ | a'2 b'2~ | \
                   b'2 c''2 | d''1";
        let mensur = merged(&score(&[src]));
        assert_eq!(values(&mensur, 0), [24, 4, 8, 8, 8, 4, 4, 8, 4, 8]);
        assert_eq!((mensur.merged, mensur.kept), (6, 2));
    }

    /// The count runs over all voices: a barline is merged where any voice merges there. Alone
    /// the upper voice merges all its ties, since its second and third tie lie two barlines apart;
    /// with the lower voice merging in between, its third tie comes fourth in a row and stays.
    #[test]
    fn the_count_runs_over_all_voices() {
        let upper = "d'2 e'2~ | e'2 f'2~ | f'1 | g'2 a'2~ | a'1";
        let mensur = merged(&score(&[upper]));
        assert_eq!((mensur.merged, mensur.kept), (3, 0));
        let mensur = merged(&score(&[upper, "d1 | e2 f2~ | f2 g2~ | g2 a2 | a1"]));
        assert_eq!((mensur.merged, mensur.kept), (4, 1));
        assert_eq!(values(&mensur, 0), [4, 8, 12, 4, 4, 8]);
    }

    /// F sharp held over the barline that no longer shows: the F after it in the same measure
    /// gets a cautionary natural, also where the F sharp is musica ficta, whose sign stands above
    /// the staff (the plain notation shows the natural, as the tie carries the sharp over the
    /// barline); an F in the next measure, in another octave, or itself ficta does not.
    #[test]
    fn cautionary_accidentals_after_merged_notes() {
        let mensur = merged(&score(&["a'1 | r2 fis'2~ | fis'2 f'2 | f'1"]));
        let cautionary: Vec<bool> = mensur.voices[0].iter().map(|s| s.cautionary).collect();
        assert_eq!(cautionary, [false, false, false, true, false]);
        let mensur = merged(&score(&["a'1 | r2 fis'2~ | fis'1 | f'1"]));
        assert!(mensur.voices[0].iter().all(|s| !s.cautionary));
        let mensur = merged(&score(&["a'1 | r2 fis'2~ | fis'2 f''2 | f'1"]));
        assert!(mensur.voices[0].iter().all(|s| !s.cautionary));
        let mut ficta = score(&["a'1 | r2 fis'2~ | fis'2 f'2 | f'1"]);
        ficta.voices[0].notes[2].ficta = true;
        ficta.voices[0].notes[3].ficta = true;
        let cautionary: Vec<bool> = merged(&ficta).voices[0]
            .iter()
            .map(|s| s.cautionary)
            .collect();
        assert_eq!(cautionary, [false, false, false, true, false]);
        ficta.voices[0].notes[4].ficta = true;
        assert!(merged(&ficta).voices[0].iter().all(|s| !s.cautionary));
        // The same F sharp again shows its sharp anyway.
        let mensur = merged(&score(&["a'1 | r2 fis'2~ | fis'4 fis'4 f'2 | f'1"]));
        assert!(mensur.voices[0].iter().all(|s| !s.cautionary));
    }

    /// A tie into a note with a fermata stays, so that the fermata stands over the last measure
    /// as in the other voices; the ties before it merge as before.
    #[test]
    fn a_note_with_a_fermata_keeps_its_tie() {
        let held = score(&[
            "d'1 | e'1~ | e'1~ | e'1",
            "d'1 | a2 b2~ | b2 c'2~ | c'2 cis'2",
        ]);
        let end = held.total_ticks();
        let mensur = merged(&held);
        assert_eq!(values(&mensur, 0), [8, 24]);
        assert_eq!(values(&mensur, 1), [8, 4, 8, 8, 4]);
        let mensur = merge_ties(&held, &[end]);
        assert_eq!(values(&mensur, 0), [8, 16, 8]);
        assert!(mensur.voices[0][1].note.tie);
        assert_eq!(mensur.voices[0][2].end(), end);
        // The other voice's tie over the last barline, into a note before its last, merges.
        assert_eq!(values(&mensur, 1), [8, 4, 8, 8, 4]);
        assert_eq!((mensur.merged, mensur.kept), (3, 1));
        // A half tied into a whole with a fermata stays a half.
        let mensur = merge_ties(&score(&["d'1 | e'2 f'2~ | f'1"]), &[24]);
        assert_eq!(values(&mensur, 0), [8, 4, 4, 8]);
        assert_eq!((mensur.merged, mensur.kept), (0, 1));
    }

    #[test]
    fn plain_keeps_every_note() {
        let score = score(&["d'1 | f'2 g'2~ | g'2 a'2 | d'1"]);
        let plain = Mensur::plain(&score);
        assert_eq!(values(&plain, 0), [8, 4, 4, 4, 4, 8]);
        assert_eq!(plain.shown(0, 3), Some(3));
    }
}
