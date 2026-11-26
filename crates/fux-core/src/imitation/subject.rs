//! Subjects built from the words: first the declamation of the head words (rhythm and
//! syllables, priced by the underlay model and the measure rhythms), then the pitches, priced by
//! the checker on one voice; with the answers each subject allows and the strettos that pass
//! the checks of two fixed lines.
//!
//! **Declamation** (Phase A). A beam of 32 runs over the notes of a head, at most ten, each with a
//! value and what it sings ([`Choice`]). The values follow Palestrina's transitions (a breve falls
//! to a whole or a dotted whole, a dotted half takes its quarter, …); every measure stays a prefix
//! of a measure rhythm of free counterpoint that the profile allows, and whole measures cost what
//! FREE-RHYTHM charges. The underlay model prices the syllables from the voice's state before the
//! point, on placeholder pitches a second apart. Style costs come from the soggetto study of
//! Palestrina's masses: the first three values by text (SOG-HEADS; an own text and a later point by
//! the foot of their first word), a melisma on the first syllable or on a later word of one
//! syllable, the first black figure (none at all is free, as in 44 % of his subjects), a syllable
//! on the quarter of a dotted half, and the span of the head by its share among his subjects
//! (SOG-LENGTH), whose later points' are shorter than the openings'. A head is complete once it has
//! begun every syllable of its head words, has four notes and a white last note, and lasts 24 to
//! 64 ticks (16 to 48 in a long text). Six skeletons go on.
//!
//! **Pitches** (Phase B). A beam of 24 per skeleton starts on the final or the fifth (Phrygian: E
//! or A; Ionian and Aeolian also the fourth) and moves by the steps {0, ±1, ±2, ±3, ±4, +5, ±7},
//! realised on the scale with the flattened mi against a tritone or an altered step, and in the
//! Dorian and Lydian modes on the upper neighbour of la (A B♭ A). The real checker prices each note
//! on a one-voice free score (a tenor, without FREE-AMBITUS and VOI-RANGE, which the octaves of the
//! entries decide), with the head as a one-entry plan so that IMI-SUBJECT judges its shape (and
//! IMI-DEGREE a later point's degree); the underlay model prices the syllables on the real pitches,
//! from the voice's state before the point, which a score of one voice could not begin with.
//! Repeated notes and rests cost what they cost a voice that sings, as in the search. Style costs:
//! the first interval by text (SOG-INTERVAL), going on in the direction of a leap of a fourth or
//! more, an ascending pair with the smaller interval first, the motion through a quarter after a
//! white note by its share in his heads, a highest note that is not unique or black. Four melodies
//! per skeleton go on.
//!
//! **Draws** (SOG-VARY). The head template, the start degree, the class of the first interval, a
//! leap of a fourth or more by the fourth interval, a span of a seventh or an octave and a last
//! note of a dotted half are drawn for each point by Palestrina's shares, and heads that do not
//! take the drawn ones pay two: the rules charge the long first notes, the degrees, the leaps and
//! the wide spans that he uses in a tenth to a half of his heads, which would never win
//! otherwise. A melody pays for a drawn leap or span from its first note until it takes it, so
//! that the beam keeps it against the cheaper steps.
//!
//! **Answers and stretto.** Each head is answered at the unison or octave and on the other main
//! degree (fifth against final, final against fourth; Phrygian A against E), really where it can
//! be sung, modally where it leaps between the final and the fifth. The stretto test (Fux,
//! p. 247) runs the pair checks of two fixed lines ([`check`]) on the leader and one follower at
//! the delays of a second entry; a subject that no answer can follow within two breves goes, and
//! the best stretto prices the others: on the other main degree a semibreve or a breve after the
//! leader is free. Six subjects per point remain, none beginning like an earlier point's.
//!
//! **Chant and given notes.** A head from chant ([`chant`]) runs Phase A on the chant's
//! notes, merged where they repeat, struck again only for a new syllable, a third filled at most
//! once; it has six to eight distinct notes of the chant by preference and ends on the final or
//! the fifth. A given head keeps its accidentals ([`Soggetto::pitches`]), takes its syllables
//! from the cheapest underlay of the head words ([`Model::best_prefix`]) and must pass every
//! check, or the request fails. The head motif of a mass (phase 14) opens the first point
//! through the path of the chant: Phase A declaims every one of its tones in order, none passing
//! between them, at most two struck again for syllables, and the head keeps their pitches and
//! accidentals as a given head does; where no head passes, the point takes its subjects from its
//! words, with the warning of a chant.
//!
//! Seeds: point `p` draws from the sub-seed `seed ^ (p + 1) · 0x9E37_79B9_7F4A_7C15`; the
//! melodies of the skeletons are priced in parallel, each with a generator of its own, and ranked
//! in a fixed order, so the subjects depend only on the seed.

use std::ops::{Range, RangeInclusive};
use std::sync::Arc;

use super::chant::{self, ChantFallback, Source};
use super::check::{self, Tone};
use super::points::Part;
use super::{
    Answer, AnswerChoice, FirstNote, Imitation, MOTIF_REPEATS, Origin, PointKind, Relation,
    Soggetto, SubjectNote, SubjectSource,
};
use crate::clef::VoiceType;
use crate::generate::{GenerateError, Rng};
use crate::interval::Interval;
use crate::lang::Lang;
use crate::mode::{Mode, ModeSpec};
use crate::pitch::Pitch;
use crate::rhythm::{self, Place, Token};
use crate::rules::{Analysis, Checker, Detail, Level, Profile, Violation, Window};
use crate::score::{
    Note, Role, Score, ScoreEntry, ScoreImitation, ScorePoint, ScoreText, Species, Voice,
};
use crate::text::underlay::{Case, Choice, Event, Levels, Model, UState};
use crate::text::{SylFlags, Text, TextKind};
use crate::time::{Duration, Meter};

/// Ticks per 2/2 measure.
const MEASURE: u32 = 8;

/// Partial heads kept per note of the declamation.
const DECLAMATION_BEAM: usize = 32;

/// The most notes of a head from the words or from chant.
pub(crate) const MOST_NOTES: usize = 10;

/// Declamations that go on to the pitches (R).
const SKELETONS: usize = 6;

/// Partial melodies kept per note of a skeleton.
const PITCH_BEAM: usize = 24;

/// Melodies kept per skeleton.
const MELODIES: usize = 4;

/// Subjects kept per point (K).
const SUBJECTS: usize = 6;

/// The noise added to costs when ranking, as in the search.
const NOISE: f64 = 2.0;

/// The diatonic steps from one note of a head to the next: repeated note, second, third,
/// fourth, fifth, a sixth up, an octave.
const STEPS: [i8; 12] = [0, 1, -1, 2, -2, 3, -3, 4, -4, 5, 7, -7];

/// The first values of a head: a whole, a dotted whole or a breve.
const FIRST_VALUES: [u8; 3] = [8, 12, 16];

/// The notes a given subject may have.
pub(crate) const GIVEN_NOTES: RangeInclusive<usize> = 4..=12;

/// The delays of a second entry on the downbeat, which the follower takes with the leader's
/// first value, and on the minim beat, where it shortens or syncopates its first note.
const WHOLE_DELAYS: [u32; 4] = [8, 16, 24, 32];
const MINIM_DELAYS: [u32; 4] = [4, 12, 20, 28];

/// The latest second entry the stretto test needs to pass: two breves.
const LATEST: u32 = 32;

/// What a given head costs that passes no stretto: more than any stretto ([`stretto_cost`]).
const NO_STRETTO: u32 = 3;

/// The cost of a corpus share in percent, for the style costs of the heads: from 10 % on free,
/// 5–10 % one, 2–5 % two, rarer four.
fn share_cost(share: f64) -> u32 {
    if share >= 10.0 {
        0
    } else if share >= 5.0 {
        1
    } else if share >= 2.0 {
        2
    } else {
        4
    }
}

/// What a head pays that is not the one drawn for its point: the head template and the start
/// degree are drawn by their shares, as the construction procedure of the soggetto study does
/// (SOG-VARY), so that the long first notes and the degrees that FREE-LONG and FREE-ENTRY price
/// come in Palestrina's proportions.
const DRAWN: u32 = 2;

/// One of the heads of a template drawn by its share in percent; `None` with the share of the
/// heads it does not list.
fn draw_head(heads: Heads, rng: &mut Rng) -> Option<&'static [u8]> {
    let mut u = rng.next_f64() * 100.0;
    heads.iter().find_map(|&(head, share)| {
        u -= share;
        (u < 0.0).then_some(head)
    })
}

/// How often a head from the words leaps a fourth or more in its first four intervals, drawn
/// with the first interval (SOG-VARY, steps 5 and 6): Palestrina's leaders leap a fourth or more
/// in 44 % of the openings and 33 % of the later points (a rising fourth or fifth in the first
/// four intervals in 15–34 % by text, SOG-LEAP), a fourth or a fifth as the first interval in 14
/// and 10 %. Without the draw no head would leap: a fourth costs MEL-LEAP-COST, a step nothing.
/// A first interval drawn as a fourth or more leaps anyway, and some draws find no head that
/// leaps.
const LEAP_FIRST: f64 = 0.42;
const LEAP_LATER: f64 = 0.25;

/// The interval a leap of a fourth or more must have come by: the fourth.
const LEAP_BY: usize = 4;

/// How often a head from the words spans a seventh or an octave, drawn like the leap: 13 % of
/// Palestrina's leaders in the openings, 8 % in the later points (SOG-RANGE: at most about
/// 15 %). IMI-SUBJECT `range` charges it, so it would not come otherwise. The heads of the first
/// points are short, and two in five of their draws find no head that wide; the later points'
/// longer heads also reach it undrawn.
const WIDE_FIRST: f64 = 0.22;
const WIDE_LATER: f64 = 0.08;

/// The span of a seventh, in diatonic steps.
const SEVENTH: i8 = 6;

/// How often a head from the words ends on a dotted half, which leaves its quarter to the free
/// continuation: the last imitated note of 15 % of Palestrina's subjects (SOG-DISSOLVE). Free,
/// the beam would end a third of the heads so, and a cost of one for it ends none.
const DOTTED_END: f64 = 0.15;

/// The shares in per cent of the spans of Palestrina's subjects, by the semibreves they reach
/// into from the second to the tenth: the notes that the followers imitate (their median count),
/// from the first attack to the last, and the last note up to a semibreve (the study of the
/// corpus: 1219 openings' leaders, median 44 ticks; 3628 later points', median 32).
const OPENING_SPANS: [f64; 9] = [0.2, 5.4, 18.3, 23.5, 19.9, 15.8, 8.3, 4.6, 2.1];
const LATER_SPANS: [f64; 9] = [4.0, 24.7, 28.5, 19.3, 9.9, 5.8, 3.0, 1.8, 0.9];

/// What a head from the words costs by its span (SOG-LENGTH), the attacks up to its last note and
/// that note up to a semibreve: its share among Palestrina's subjects of the openings, or of the
/// later points, which are shorter. An opening's head of four to seven semibreves is free, a later
/// point's of three to five; the beam would otherwise let long melismas on the head's last
/// syllable run on to the longest head allowed.
fn span_cost(opening: bool, values: &[u8]) -> u32 {
    let ticks: u32 = values.iter().map(|&v| u32::from(v)).sum();
    let last = values.last().map_or(0, |&v| u32::from(v));
    let span = ticks - last + last.min(MEASURE);
    let shares = if opening {
        &OPENING_SPANS
    } else {
        &LATER_SPANS
    };
    let share = (span.div_ceil(MEASURE) as usize)
        .checked_sub(2)
        .and_then(|k| shares.get(k))
        .copied()
        .unwrap_or(0.0);
    share_cost(share)
}

/// The class of an interval of `steps` diatonic steps: a repeated note 0, a second 1, a third 2,
/// a fourth or more 3.
fn class(steps: i8) -> usize {
    usize::from(steps.unsigned_abs().min(3))
}

/// Whether a melody leaps a fourth or more.
fn leapt(steps: &[i8]) -> bool {
    steps.windows(2).any(|w| (w[1] - w[0]).abs() >= 3)
}

/// The class of a head's first interval ([`class`]), drawn by the shares of `intervals`.
fn draw_class(intervals: Intervals, rng: &mut Rng) -> Option<usize> {
    let total: f64 = intervals.iter().map(|&(_, share)| share).sum();
    let mut u = rng.next_f64() * total;
    intervals.iter().find_map(|&(steps, share)| {
        u -= share;
        (u < 0.0).then_some(class(steps))
    })
}

/// The motions through a quarter that follows a white note in Palestrina's heads, the steps into
/// the quarter and out of it, with their shares in percent of such figures (the leaders' heads
/// of the points the detector finds in his masses: 2531 figures). A passing quarter descending
/// leads by far; the upper neighbour, a third into the quarter and a leap out of it are rare.
const QUARTER_FIGURES: [((i8, i8), f64); 13] = [
    ((-1, -1), 42.1),
    ((1, 1), 16.8),
    ((0, 0), 8.1),
    ((-1, 1), 5.9),
    ((-1, 0), 4.7),
    ((0, -1), 4.6),
    ((-2, 1), 4.0),
    ((0, 1), 2.7),
    ((-4, 1), 2.6),
    ((-1, -2), 2.0),
    ((1, -1), 1.7),
    ((0, -2), 0.8),
    ((-3, 1), 0.6),
];

/// The cost of a motion through a quarter after a white note: free from 2 % of Palestrina's
/// figures on, one for those of half a percent and more, four for the rarer ones. A smaller
/// difference would not do: the beam keeps the cheapest melodies, and a figure he uses must
/// not lose to one he avoids by noise.
fn quarter_cost(into: i8, out: i8) -> u32 {
    match QUARTER_FIGURES
        .iter()
        .find(|(figure, _)| *figure == (into, out))
    {
        Some(&(_, share)) if share >= 2.0 => 0,
        Some(_) => 1,
        None => 4,
    }
}

/// The sub-seed of point `p`: the seed of the request mixed with the point.
pub(crate) fn sub_seed(seed: u64, p: usize) -> u64 {
    seed ^ (p as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// The generator of a sub-seed's draws, seeded with its first output: seeds that differ in their
/// low bits only, as `seed ^ (p + 1) · 0x9E37_79B9_7F4A_7C15` does for small seeds, give first
/// draws that crowd together.
fn generator(seed: u64) -> Rng {
    Rng::new(Rng::new(seed).next_u64())
}

/// The first three values of Palestrina's dux heads by text, with their shares in percent
/// (SOG-HEADS; soggetto study, Table C, confirmed by its verifier). Kyrie II takes Kyrie I's.
type Heads = &'static [(&'static [u8], f64)];

const KYRIE_HEADS: Heads = &[
    (&[8, 4, 4], 22.0),
    (&[16, 8, 8], 21.0),
    (&[8, 8, 8], 15.0),
    (&[12, 4, 8], 8.0),
    (&[8, 6, 2], 5.0),
    (&[16, 8, 12], 5.0),
];
const CHRISTE_HEADS: Heads = &[
    (&[8, 4, 4], 24.0),
    (&[16, 8, 8], 13.0),
    (&[12, 4, 4], 11.0),
    (&[8, 12, 4], 8.0),
    (&[8, 6, 2], 7.0),
    (&[8, 8, 8], 7.0),
];
const AGNUS_HEADS: Heads = &[
    (&[16, 8, 8], 18.0),
    (&[8, 8, 8], 12.0),
    (&[16, 8, 6], 12.0),
    (&[12, 4, 4], 9.0),
    (&[8, 12, 4], 7.0),
    (&[12, 4, 8], 6.0),
];
const SANCTUS_HEADS: Heads = &[
    (&[16, 8, 8], 19.0),
    (&[8, 8, 8], 15.0),
    (&[8, 8, 12], 9.0),
    (&[8, 6, 2], 8.0),
    (&[16, 8, 12], 6.0),
    (&[8, 8, 6], 6.0),
];
const BENEDICTUS_HEADS: Heads = &[
    (&[16, 8, 8], 37.0),
    (&[8, 8, 8], 16.0),
    (&[12, 4, 8], 13.0),
    (&[8, 4, 4], 10.0),
    (&[16, 8, 12], 10.0),
];
/// Ho-sán-na with its unaccented first syllable: 8 12 4 in 18 %, a first whole in 85 %.
const OSANNA_HEADS: Heads = &[(&[8, 12, 4], 18.0)];

/// The foot grammar of own texts and later points: a dactyl (Dó-mi-nus), four syllables
/// stressed on the third (mi-se-ré-re), a trochee (A-gnus). Patterns of two values constrain
/// only the first two.
const DACTYL: &[&[u8]] = &[&[8, 4], &[12, 4, 8], &[16, 8, 8]];
const FOUR_SYLLABLES: &[&[u8]] = &[&[16, 8, 8], &[12, 4, 8], &[8, 4, 4]];
const TROCHEE: &[&[u8]] = &[&[8, 4], &[16, 8], &[12, 4], &[8, 8]];

/// The first four values of heads from chant, with their shares in percent (PAR-EVEN-VALUES).
const CHANT_HEADS: Heads = &[
    (&[8, 8, 8, 8], 11.0),
    (&[8, 8, 8, 4], 7.0),
    (&[16, 8, 8, 8], 7.0),
    (&[8, 4, 4, 4], 7.0),
    (&[12, 4, 4, 4], 6.0),
    (&[12, 4, 8, 8], 5.0),
    (&[8, 4, 4, 8], 5.0),
];

/// The first interval of dux heads by text, in diatonic steps with the share in percent
/// (SOG-INTERVAL; soggetto study, Table E).
type Intervals = &'static [(i8, f64)];

const KYRIE_INTERVALS: Intervals = &[
    (0, 39.0),
    (2, 18.0),
    (1, 16.0),
    (-2, 8.0),
    (4, 7.0),
    (-1, 6.0),
    (-3, 2.0),
];
const CHRISTE_INTERVALS: Intervals = &[
    (1, 30.0),
    (0, 25.0),
    (-1, 15.0),
    (2, 13.0),
    (4, 7.0),
    (-2, 6.0),
];
const AGNUS_INTERVALS: Intervals = &[
    (0, 34.0),
    (1, 18.0),
    (2, 13.0),
    (-1, 10.0),
    (3, 9.0),
    (-2, 7.0),
    (4, 4.0),
];
const SANCTUS_INTERVALS: Intervals = &[
    (0, 27.0),
    (2, 23.0),
    (-2, 15.0),
    (1, 11.0),
    (3, 9.0),
    (-1, 6.0),
    (-3, 5.0),
];
const BENEDICTUS_INTERVALS: Intervals = &[
    (0, 27.0),
    (1, 21.0),
    (2, 19.0),
    (-1, 16.0),
    (-2, 8.0),
    (3, 3.0),
    (4, 2.0),
];
/// The first intervals of all opening subjects (study of the corpus, IMI-HEAD).
const OPENING_INTERVALS: Intervals = &[
    (0, 28.6),
    (1, 21.7),
    (-1, 15.3),
    (2, 13.5),
    (-2, 7.1),
    (3, 5.3),
    (4, 4.4),
    (-3, 2.1),
    (-4, 2.0),
];
/// The first intervals of the leaders of all points.
const POINT_INTERVALS: Intervals = &[
    (0, 29.4),
    (1, 25.1),
    (-1, 19.8),
    (2, 7.9),
    (-2, 7.0),
    (3, 5.0),
    (4, 2.8),
    (-4, 1.7),
    (-3, 1.2),
];

/// The degrees a leader begins on by mode, in steps above the final, with their shares in
/// percent among Palestrina's duxes (SOG-DEGREE); the Phrygian takes A for its fifth.
fn start_degrees(mode: ModeSpec) -> &'static [(i8, f64)] {
    match mode.mode {
        Mode::Dorian => &[(0, 45.0), (4, 31.0)],
        Mode::Mixolydian => &[(4, 49.0), (0, 37.0)],
        Mode::Ionian => &[(0, 38.0), (4, 35.0), (3, 15.0)],
        Mode::Aeolian => &[(0, 38.0), (4, 28.0), (3, 20.0)],
        Mode::Phrygian => &[(3, 33.0), (0, 31.0)],
        Mode::Lydian => &[(0, 50.0), (4, 50.0)],
    }
}

/// The other main degree against a degree (in steps above the final), if it has one: the fifth
/// against the final and back, the final against the fourth; in the Phrygian mode A against E
/// and back ([`Relation::Dominant`]).
pub(crate) fn dominant(mode: ModeSpec, degree: i8) -> Option<i8> {
    let phrygian = mode.mode == Mode::Phrygian;
    match (phrygian, degree.rem_euclid(7)) {
        (true, 0) => Some(3),
        (true, 3) => Some(0),
        (false, 0) => Some(4),
        (false, 4 | 3) => Some(0),
        _ => None,
    }
}

/// The diatonic shifts at which an entry in `relation` follows a leader on `degree`, within an
/// octave each way.
pub(crate) fn shifts(mode: ModeSpec, degree: i8, relation: Relation) -> Vec<i8> {
    match relation {
        Relation::Same => vec![0, 7, -7],
        Relation::Dominant => match dominant(mode, degree) {
            Some(other) => {
                let up = (other - degree).rem_euclid(7);
                vec![up, up - 7]
            }
            None => Vec::new(),
        },
        Relation::Steps(n) => vec![n],
    }
}

/// The pitches of a head whose notes lie `steps` diatonic steps from a first note on the
/// diatonic index `first`: the pitches of the scale, with the flattened mi (B♭, transposed E♭)
/// where a tritone or an altered step would arise, on the later note of the two or else on the
/// earlier one; in the Dorian and Lydian modes also on a single note above la between two la's.
/// `None` where that does not help.
fn realize(mode: ModeSpec, first: i32, steps: &[i8]) -> Option<Vec<Pitch>> {
    let mut out: Vec<Pitch> = Vec::with_capacity(steps.len());
    for &s in steps {
        let p = mode.scale_pitch(first + i32::from(s));
        let p = match out.last().copied() {
            Some(prev) if bad(prev, p) => {
                if let Some(flat) = flattened(mode, p).filter(|&f| !bad(prev, f)) {
                    flat
                } else {
                    let before = out.len().checked_sub(2).map(|i| out[i]);
                    let flat = flattened(mode, prev)
                        .filter(|&f| !bad(f, p) && before.is_none_or(|b| !bad(b, f)))?;
                    *out.last_mut()? = flat;
                    p
                }
            }
            _ => p,
        };
        out.push(p);
    }
    // Una nota super la semper est canendum fa: the upper neighbour of la is sung fa (A B♭ A,
    // transposed D E♭ D), as Palestrina sings it in 63 % of such figures in the Dorian mode and
    // 60 % transposed; in the modes whose ficta has the flattened mi (FIC-BFLAT).
    if matches!(mode.mode, Mode::Dorian | Mode::Lydian) {
        for i in 1..out.len().saturating_sub(1) {
            if out[i - 1] == out[i + 1]
                && out[i].diatonic() == out[i - 1].diatonic() + 1
                && let Some(fa) = flattened(mode, out[i])
            {
                out[i] = fa;
            }
        }
    }
    Some(out)
}

/// Where a head with `steps` from the diatonic index `first` cannot be realised: the rule of the
/// interval that no flattened mi mends and the index of its later note.
fn unsingable(mode: ModeSpec, first: i32, steps: &[i8]) -> (&'static str, usize) {
    let i = (1..steps.len())
        .find(|&i| realize(mode, first, &steps[..=i]).is_none())
        .unwrap_or(1)
        .min(steps.len().saturating_sub(1));
    let at = |k: usize| mode.scale_pitch(first + i32::from(steps.get(k).copied().unwrap_or(0)));
    let rule = if Interval::between(at(i.saturating_sub(1)), at(i)).is_tritone() {
        "MEL-TRITONE"
    } else {
        "MEL-AUG-DIM"
    };
    (rule, i)
}

/// Why an entry `shift` diatonic steps from a given head's leader, whose first note lies on the
/// diatonic index `first`, cannot sing it: an altered note that lands where the mode allows no
/// accidental (FIC-SCOPE), else the interval that no flattened mi mends, with the index of the
/// note.
fn unsung(mode: ModeSpec, subject: &Soggetto, first: i32, shift: i8) -> (&'static str, usize) {
    let interval = transposition(mode, first, shift);
    let outside = (0..subject.notes.len()).position(|i| {
        let note = &subject.notes[i];
        let p = mode.scale_pitch(first + i32::from(note.step));
        note.alter != 0
            && !subject.own_flat(mode, first, i)
            && !in_ficta(mode, p.with_alter(p.alter + note.alter).transpose(interval))
    });
    match outside {
        Some(i) => ("FIC-SCOPE", i),
        None => unsingable(mode, first + i32::from(shift), &subject.steps()),
    }
}

/// The flattened mi for a scale pitch on the mi's step.
fn flattened(mode: ModeSpec, p: Pitch) -> Option<Pitch> {
    (p.step == mode.flattened_mi() && mode.is_scale_pitch(p)).then(|| p.with_alter(p.alter - 1))
}

/// The widest span of a head in diatonic steps, an octave: Palestrina's heads never go beyond it
/// (IMI-SUBJECT `beyond_octave`).
pub(crate) const OCTAVE: i32 = 7;

/// The diatonic steps from the lowest to the highest of `pitches`.
pub(crate) fn span(pitches: &[Pitch]) -> i32 {
    let low = pitches.iter().map(|p| p.diatonic()).min();
    let high = pitches.iter().map(|p| p.diatonic()).max();
    high.zip(low).map_or(0, |(high, low)| high - low)
}

/// A tritone or an altered interval between two notes.
fn bad(a: Pitch, b: Pitch) -> bool {
    let i = Interval::between(a, b);
    i.is_tritone() || !i.is_regular()
}

/// Whether two notes `at` and `at + 1` of a head on `degree` join the final and the fifth (in
/// the Phrygian mode E and A) by a fourth or a fifth: where a modal answer exchanges them.
fn joins_main_degrees(mode: ModeSpec, degree: i8, steps: &[i8], at: usize) -> bool {
    let (Some(&a), Some(&b)) = (steps.get(at), steps.get(at + 1)) else {
        return false;
    };
    let other = if mode.mode == Mode::Phrygian { 3 } else { 4 };
    let degrees = [(degree + a).rem_euclid(7), (degree + b).rem_euclid(7)];
    matches!((b - a).abs(), 3 | 4) && degrees.contains(&0) && degrees.contains(&other)
}

/// The steps of a head as an entry with `answer` sings them: the leader's, or with the fourth
/// and the fifth exchanged in interval `at` and the notes after it a step away.
fn answer_steps(steps: &[i8], answer: Answer) -> Vec<i8> {
    match answer {
        Answer::Real => steps.to_vec(),
        Answer::Modal { at } => {
            let at = usize::from(at);
            let Some(interval) = steps.get(at + 1).map(|&b| b - steps[at]) else {
                return steps.to_vec();
            };
            let exchanged = interval.signum() * (7 - interval.abs());
            let shift = exchanged - interval;
            steps
                .iter()
                .enumerate()
                .map(|(i, &s)| if i > at { s + shift } else { s })
                .collect()
        }
    }
}

/// The interval by which the altered notes of a given head move in an entry `shift` diatonic
/// steps from the leader, whose first note lies on the diatonic index `first`: perfect at the
/// unison, the fourth, the fifth and the octave, otherwise as the scale has it.
fn transposition(mode: ModeSpec, first: i32, shift: i8) -> Interval {
    let shift = i32::from(shift);
    let octaves = 12 * shift.div_euclid(7);
    match shift.rem_euclid(7) {
        0 => Interval::new(shift, octaves),
        3 => Interval::new(shift, octaves + 5),
        4 => Interval::new(shift, octaves + 7),
        _ => Interval::between(mode.scale_pitch(first), mode.scale_pitch(first + shift)),
    }
}

/// Whether the mode lets an entry sing an altered note of a given head where it lands: on a
/// pitch of the scale, on the flattened mi, or on a sharp a semitone below the scale's next
/// step, which leads up to it (FIC-SCOPE).
fn in_ficta(mode: ModeSpec, pitch: Pitch) -> bool {
    let scale = mode.scale_pitch(pitch.diatonic());
    mode.is_scale_pitch(pitch)
        || flattened(mode, scale) == Some(pitch)
        || (pitch.alter == scale.alter + 1
            && Interval::between(pitch, mode.scale_pitch(pitch.diatonic() + 1)).semitones == 1)
}

impl Soggetto {
    /// Whether note `i` of a given head whose leader begins on the diatonic index `first` is the
    /// mode's own flattened mi there (B♭, transposed E♭): an accidental of the scale's ficta, not
    /// of the head, as in fa super la (A B♭ A), which an entry on another degree sings as the
    /// scale and its flattened mi have it there (FIC-BFLAT, docs/regeln.md § 6).
    fn own_flat(&self, mode: ModeSpec, first: i32, i: usize) -> bool {
        let note = &self.notes[i];
        let scale = mode.scale_pitch(first + i32::from(note.step));
        note.alter == -1 && flattened(mode, scale).is_some()
    }

    /// The pitches of the head as an entry `shift` diatonic steps from the leader sings it with
    /// `answer`, the leader's first note on the diatonic index `first` (the leader itself at
    /// shift 0). A head from the words or from chant lies on the scale, with the flattened mi (B♭,
    /// transposed E♭) where a tritone or an altered step would arise and, in the Dorian and Lydian
    /// modes, on the upper neighbour of la. A given head keeps its accidentals: at the unison and
    /// the octave it sings its notes as given; elsewhere its unaltered notes and its flattened mi
    /// lie on the scale so, and each other altered note moves with the entry, by a perfect fourth
    /// or fifth on the other main degree, where the mode allows the pitch it lands on (a pitch of
    /// the scale, the flattened mi or a sharp that leads up a semitone), and not after the changed
    /// interval of a modal answer. A head on a head motif (phase 14) keeps its accidentals so too.
    /// `None` where the entry cannot sing the head so.
    pub fn pitches(
        &self,
        mode: ModeSpec,
        first: i32,
        shift: i8,
        answer: Answer,
    ) -> Option<Vec<Pitch>> {
        let steps = answer_steps(&self.steps(), answer);
        let at = first + i32::from(shift);
        if !matches!(self.origin, Origin::Given | Origin::Motif) {
            return realize(mode, at, &steps);
        }
        let given = |base: i32, i: usize| {
            let p = mode.scale_pitch(base + i32::from(self.notes[i].step));
            p.with_alter(p.alter + self.notes[i].alter)
        };
        if shift.rem_euclid(7) == 0 && answer == Answer::Real {
            return Some((0..self.notes.len()).map(|i| given(at, i)).collect());
        }
        let mut pitches = realize(mode, at, &steps)?;
        let interval = transposition(mode, first, shift);
        for (i, note) in self.notes.iter().enumerate() {
            if note.alter == 0 || self.own_flat(mode, first, i) {
                continue;
            }
            if matches!(answer, Answer::Modal { at } if i > usize::from(at)) {
                return None;
            }
            let pitch = given(first, i).transpose(interval);
            if !in_ficta(mode, pitch) {
                return None;
            }
            pitches[i] = pitch;
        }
        Some(pitches)
    }

    /// The diatonic steps of the head from its first note.
    pub fn steps(&self) -> Vec<i8> {
        self.notes.iter().map(|n| n.step).collect()
    }

    /// The values of the head in ticks.
    pub fn values(&self) -> Vec<u32> {
        self.notes.iter().map(|n| u32::from(n.ticks)).collect()
    }

    /// How long the head lasts as the leader sings it, in ticks.
    pub fn ticks(&self) -> u32 {
        self.notes.iter().map(|n| u32::from(n.ticks)).sum()
    }

    /// Whether two subjects begin alike (SOG-VARY): their first three intervals and their first
    /// four values are the same.
    pub fn begins_like(&self, other: &Soggetto) -> bool {
        let intervals = |s: &Soggetto| -> Vec<i8> {
            s.notes
                .windows(2)
                .take(3)
                .map(|w| w[1].step - w[0].step)
                .collect()
        };
        let values =
            |s: &Soggetto| -> Vec<u8> { s.notes.iter().take(4).map(|n| n.ticks).collect() };
        intervals(self) == intervals(other) && values(self) == values(other)
    }
}

/// The melodic moves the profile forbids outright (MEL-MAX-LEAP, MEL-TRITONE, MEL-AUG-DIM,
/// MEL-MIN6-DOWN in free counterpoint), as the search leaves them out.
#[derive(Clone, Copy, Debug)]
struct Moves {
    max_leap: bool,
    tritone: bool,
    aug_dim: bool,
    chromatic: bool,
    min6_down: bool,
}

impl Moves {
    fn new(checker: &Checker, many: bool) -> Moves {
        let forbidden = |rule, param| checker.level(rule, param) == Level::Forbidden;
        Moves {
            max_leap: forbidden("MEL-MAX-LEAP", "level"),
            tritone: forbidden("MEL-TRITONE", "level"),
            aug_dim: forbidden("MEL-AUG-DIM", "level"),
            chromatic: forbidden(
                "MEL-AUG-DIM",
                if many {
                    "chromatic"
                } else {
                    "chromatic_two_voices"
                },
            ),
            min6_down: forbidden("MEL-MIN6-DOWN", "free"),
        }
    }

    /// The rule that forbids moving from `from` to `to`, if one does.
    fn forbids(self, from: Pitch, to: Pitch) -> Option<&'static str> {
        let interval = Interval::between(from, to);
        let up = interval.abs();
        if interval.is_tritone() {
            self.tritone.then_some("MEL-TRITONE")
        } else if !interval.is_regular() {
            let forbidden = if up.steps == 0 {
                self.chromatic
            } else {
                self.aug_dim
            };
            forbidden.then_some("MEL-AUG-DIM")
        } else if (up.steps == 5 && up.semitones == 9) || up.steps == 6 || up.steps >= 8 {
            self.max_leap.then_some("MEL-MAX-LEAP")
        } else if interval.steps == -5 && interval.semitones == -8 {
            self.min6_down.then_some("MEL-MIN6-DOWN")
        } else {
            None
        }
    }

    /// The first forbidden move of a line: its rule and the index of the note it leads to.
    fn first(self, pitches: &[Pitch]) -> Option<(&'static str, usize)> {
        pitches
            .windows(2)
            .enumerate()
            .find_map(|(i, w)| self.forbids(w[0], w[1]).map(|rule| (rule, i + 1)))
    }
}

/// The measure rhythms of free counterpoint (FREE-RHYTHM) with the profile's levels.
#[derive(Clone, Debug)]
struct Rhythms {
    /// Each pattern's tokens, whether its last note is tied into the next measure, its level.
    patterns: Vec<(Vec<Token>, bool, Level)>,
    /// The level of a rhythm Palestrina does not use.
    unlisted: Level,
}

impl Rhythms {
    fn new(checker: &Checker) -> Rhythms {
        let patterns = rhythm::patterns(Species::Free, Place::Inner, true)
            .iter()
            .map(|(pattern, name)| {
                let level = name.map_or(Level::Off, |name| checker.level("FREE-RHYTHM", name));
                (rhythm::tokens(pattern), pattern.ends_with('⁀'), level)
            })
            .collect();
        Rhythms {
            patterns,
            unlisted: checker.level("FREE-RHYTHM", "level"),
        }
    }

    /// Whether a measure that begins with these tokens can still become a rhythm the profile
    /// does not forbid.
    fn prefix(&self, tokens: &[Token]) -> bool {
        self.patterns
            .iter()
            .any(|(pattern, _, level)| *level != Level::Forbidden && pattern.starts_with(tokens))
    }

    /// The level of a whole measure.
    fn level(&self, tokens: &[Token], tied: bool) -> Level {
        self.patterns
            .iter()
            .find(|(pattern, out, _)| pattern == tokens && *out == tied)
            .map_or(self.unlisted, |(_, _, level)| *level)
    }
}

/// The tokens of each measure that written notes touch, and whether its last note is tied into
/// the next measure. Written notes never cross a barline.
fn measures(written: &[Note]) -> Vec<(Vec<Token>, bool)> {
    let mut out: Vec<(Vec<Token>, bool)> = Vec::new();
    let mut tick = 0;
    let mut tied = false;
    for note in written {
        if tick % MEASURE == 0 || out.is_empty() {
            out.push((Vec::new(), false));
        }
        let measure = out.last_mut().expect("a measure");
        let tied_in = tied && measure.0.is_empty();
        measure.0.push(Token::of(note, tied_in));
        measure.1 = note.tie;
        tied = note.tie;
        tick += note.duration.ticks();
    }
    out
}

/// A head's pitches with their values from tick 0, written in measures.
fn written_of(mode: ModeSpec, pitches: &[Pitch], values: &[u32]) -> Vec<Note> {
    let mut written = Vec::new();
    let mut tick = 0;
    for (&pitch, &value) in pitches.iter().zip(values) {
        write(
            &mut written,
            pitch,
            tick,
            value,
            !mode.is_scale_pitch(pitch),
        );
        tick += value;
    }
    written
}

/// A note of `ticks` from `tick` on, written in measures: tied notes over the barlines.
fn write(written: &mut Vec<Note>, pitch: Pitch, tick: u32, ticks: u32, ficta: bool) {
    let mut at = tick;
    let end = tick + ticks;
    while at < end {
        let until = end.min((at / MEASURE + 1) * MEASURE);
        if at > tick
            && let Some(last) = written.last_mut()
        {
            last.tie = true;
        }
        written.push(Note::new(pitch, Duration::from_ticks(until - at)).with_ficta(ficta));
        at = until;
    }
}

/// The first syllables of the heads by section: how they are declaimed, and their first
/// intervals.
#[derive(Clone, Copy, Debug)]
enum Template {
    /// Palestrina's heads of the section.
    Heads(Heads),
    /// Ho-sán-na: its one head, otherwise a first whole.
    Osanna,
    /// A foot of the point's first word.
    Foot(&'static [&'static [u8]]),
    /// None.
    Free,
}

/// What Phase A knows of a point.
struct Ctx<'a> {
    p: usize,
    part: &'a Part,
    /// The voice's underlay state before the head.
    from: UState,
    /// The point's first syllable is accented: IMI-SUBJECT `not_longest` counts in the first
    /// point.
    accented: bool,
    /// The point's first unit is one word (San-ctus), whose melisma is cheap.
    lone: bool,
    template: Template,
    /// The head of the template drawn by its share (SOG-VARY), which the others give way to.
    drawn: Option<&'static [u8]>,
    /// The start degree drawn by its share (SOG-VARY), which the others give way to.
    degree: Option<i8>,
    /// The class of the first interval drawn by its share ([`class`]), for heads from the words.
    first_class: Option<usize>,
    /// A leap of a fourth or more drawn for the first four intervals, for heads from the words.
    leap: bool,
    /// A span of a seventh or an octave drawn, for heads from the words.
    wide: bool,
    /// A last note of a dotted half drawn, for heads from the words.
    dotted_end: bool,
    intervals: Intervals,
    /// How long a head lasts, in ticks.
    ticks: RangeInclusive<u32>,
    /// How many notes it has.
    notes: RangeInclusive<usize>,
    /// The notes of chant it paraphrases.
    chant: Option<&'a Source>,
}

/// A step of a passing fill between two notes of chant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Fill {
    /// None yet; one third or fourth of a head may be filled.
    #[default]
    Open,
    Used,
    /// The second passing note of a fourth must follow, on this degree.
    Then(i8),
}

/// What a note of a head from chant is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// A note of the words' head, its pitch left to Phase B.
    Free,
    /// The next distinct note of the chant, at this index of the source.
    Chant(usize),
    /// The chant's note struck again for a new syllable.
    Again(usize),
    /// A passing note within a third; with `Some` the first of two within a fourth.
    Passing(Option<i8>),
    /// The tone of a head motif before it struck again for a new syllable, which the motif does
    /// not have (at most [`MOTIF_REPEATS`]).
    Repeated,
}

/// A head as Phase A builds it: its values and what each note sings.
#[derive(Clone, Debug)]
struct Decl {
    values: Vec<u8>,
    choices: Vec<Choice>,
    /// From chant: each note's degree, in steps above the chant's final.
    degrees: Vec<i8>,
    /// From chant: each note's alteration against the scale, which only a head motif has.
    alters: Vec<i8>,
    /// From chant: each note is a passing note.
    passing: Vec<bool>,
    /// The notes written in measures: placeholder pitches a second apart, the chant's pitches.
    written: Vec<Note>,
    state: UState,
    tick: u32,
    /// Measures whose rhythm is priced.
    priced: usize,
    /// The cost of the underlay's cases.
    txt: u32,
    /// The rules that the checker prices again: measure rhythms, drops, long notes, a first note
    /// shorter than a later one, the head's end.
    rules: u32,
    /// The style costs, which the subject keeps.
    style: u32,
    /// The melisma of the first syllable is priced.
    melisma: bool,
    rank: f64,
    /// From chant: the index of the next note of the source.
    next: usize,
    /// From chant: the distinct notes of the chant sung.
    distinct: usize,
    fill: Fill,
    /// From a head motif: the notes struck again for syllables ([`Kind::Repeated`]).
    repeated: usize,
}

impl Decl {
    fn new(from: UState) -> Decl {
        Decl {
            values: Vec::new(),
            choices: Vec::new(),
            degrees: Vec::new(),
            alters: Vec::new(),
            passing: Vec::new(),
            written: Vec::new(),
            state: from,
            tick: 0,
            priced: 0,
            txt: 0,
            rules: 0,
            style: 0,
            melisma: false,
            rank: 0.0,
            next: 0,
            distinct: 0,
            fill: Fill::Open,
            repeated: 0,
        }
    }

    fn cost(&self) -> u32 {
        self.txt + self.rules + self.style
    }
}

/// A melody of a skeleton as Phase B builds it.
#[derive(Clone, Debug)]
struct Tune {
    degree: i8,
    steps: Vec<i8>,
    pitches: Vec<Pitch>,
    written: Vec<Note>,
    state: UState,
    /// The tick of the last note's attack.
    attack: u32,
    txt: u32,
    checked: u32,
    style: u32,
    low: i8,
    high: i8,
    /// What the drawn leap and span still cost: [`DRAWN`] each, the leap's until the melody
    /// leaps a fourth or more by its fourth interval (for good if it does not), the span's until
    /// it spans a seventh. Every melody pays them from the start, so that one with a leap need
    /// not survive the beam against the cheaper steps before the cost is due.
    debt: u32,
    rank: f64,
}

impl Tune {
    fn cost(&self) -> u32 {
        self.txt + self.checked + self.style + self.debt
    }
}

/// The subjects of a point, and the warning if its chant gave none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Found {
    /// The candidates, best first.
    pub subjects: Vec<Soggetto>,
    /// The point took subjects from its words, since its chant phrase gave no usable head.
    pub fallback: Option<ChantFallback>,
}

/// Builds the subjects of an imitative setting: from the words of its points, from chant or
/// from given notes, priced by the profile's rules.
pub(crate) struct Builder {
    text: Arc<Text>,
    mode: ModeSpec,
    model: Model,
    levels: Levels,
    /// The pricing checkers of the first point's leader and of later ones.
    first: Checker,
    later: Checker,
    moves: Moves,
    rhythms: Rhythms,
    drop: Level,
    end: Level,
    not_longest: Level,
    over_whole: Level,
    answer: AnswerChoice,
    head: Option<usize>,
    relations: Vec<Option<Vec<Relation>>>,
    source: SubjectSource,
    chant: Vec<Option<Source>>,
    parts: Vec<Part>,
    seed: u64,
    /// Per voice of the choir, the diatonic indices on which it sings free of FREE-AMBITUS and
    /// VOI-RANGE ([`free_range`]), and its staff with a step beyond it each way.
    ranges: Vec<((i32, i32), (i32, i32))>,
}

/// What a subject costs for each voice of the choir that cannot sing its head within its range
/// in any octave and any relation it is answered in: the planner would place it beyond, where
/// FREE-AMBITUS or VOI-RANGE charge its notes. In the Mixolydian and the Aeolian modes, whose
/// octaves of the mode fit the lower voices' staves badly, the narrower heads win so.
const BEYOND_RANGE: u32 = 2;

/// What it costs more for each voice that cannot sing the head even within a step of its staff:
/// as much as two of its notes two steps beyond the staff cost there (VOI-RANGE `two_steps`, P2).
const BEYOND_STAFF: u32 = 4;

/// The diatonic indices on which `voice` sings a note that neither FREE-AMBITUS nor VOI-RANGE
/// charges under `checker`: within its octave of the mode from a step below to a step above, and
/// on its staff, with a step beyond it where VOI-RANGE leaves that free.
fn free_range(checker: &Checker, mode: ModeSpec, voice: VoiceType) -> (i32, i32) {
    let clef = voice.clef();
    let free = |param| i32::from(checker.level("VOI-RANGE", param) == Level::Off);
    let low = clef.bottom_line().diatonic() - free("one_step_below");
    let high = clef.top_line().diatonic() + free("one_step");
    let ambitus = if crate::rules::principal(voice) {
        "principal"
    } else {
        "collateral"
    };
    if checker.level("FREE-AMBITUS", ambitus) == Level::Off {
        return (low, high);
    }
    let bottom = crate::rules::modal_octave(mode, voice);
    (low.max(bottom - 1), high.min(bottom + 8))
}

impl Builder {
    /// A builder for `parts` of `text` (from [`super::points::points_of`]) in `mode` for the
    /// voices of `choir`, priced by `profile`, with the choices of `imitation`. A chant's phrases
    /// are found for every point here; one that the chant does not have fails with
    /// [`GenerateError::NoChantPhrase`].
    pub(crate) fn new(
        text: Arc<Text>,
        mode: ModeSpec,
        choir: &[VoiceType],
        profile: &Profile,
        imitation: &Imitation,
        parts: Vec<Part>,
        seed: u64,
    ) -> Result<Builder, GenerateError> {
        let checker = Checker::new(profile);
        let levels = Levels::from(&checker);
        let mut first = profile.clone();
        for rule in ["FREE-AMBITUS", "VOI-RANGE", "IMI-OVERLAP"] {
            first
                .apply_override(&format!("{rule}=aus"))
                .expect("a rule of the catalogue");
        }
        // The head's voice sings, but its score carries no lyrics, which the underlay model
        // prices: repeated notes and rests cost what they cost a singing voice, as in the search.
        for (rule, plain, texted) in [
            ("MEL-REPEAT", "free", "free_texted"),
            ("FREE-REST", "level", "texted"),
        ] {
            let level = profile.settings(rule).level(texted);
            first
                .apply_override(&format!("{rule}.{plain}={}", level.label(Lang::De)))
                .expect("a level of the profile");
        }
        let mut later = first.clone();
        later
            .apply_override("FREE-ENTRY=aus")
            .expect("a rule of the catalogue");
        let chant = match &imitation.subject {
            SubjectSource::Chant { chant, phrases } => {
                chant::sources(chant, &text, &parts, phrases.as_deref())?
            }
            // The head motif of a mass: a source for the first point only.
            SubjectSource::Motif(motif) => {
                let mut sources = vec![None; parts.len()];
                if let Some(first) = sources.first_mut() {
                    *first = Some(chant::motif_source(motif));
                }
                sources
            }
            _ => vec![None; parts.len()],
        };
        Ok(Builder {
            model: Model::new(&text, mode, Meter::ALLA_BREVE, &[]),
            text,
            mode,
            levels,
            moves: Moves::new(&checker, choir.len() >= 3),
            rhythms: Rhythms::new(&checker),
            drop: checker.level("IMI-SUBJECT", "drop"),
            end: checker.level("IMI-SUBJECT", "end"),
            not_longest: checker.level("IMI-SUBJECT", "not_longest"),
            over_whole: checker.level("FREE-LONG", "over_whole"),
            first: Checker::new(&first),
            later: Checker::new(&later),
            answer: imitation.answer,
            head: imitation.head,
            relations: imitation.relations.clone(),
            source: imitation.subject.clone(),
            chant,
            parts,
            seed,
            ranges: choir
                .iter()
                .map(|&voice| {
                    let clef = voice.clef();
                    let staff = (
                        clef.bottom_line().diatonic() - 1,
                        clef.top_line().diatonic() + 1,
                    );
                    (free_range(&checker, mode, voice), staff)
                })
                .collect(),
        })
    }

    /// The chant source of point `p`, if its subjects come from chant.
    pub(crate) fn chant_source(&self, p: usize) -> Option<&Source> {
        self.chant.get(p).and_then(Option::as_ref)
    }

    /// The subjects of point `p`, best first, after the subjects `earlier` that the points
    /// before it took: a given head for the first points if the request gives some, the chant's
    /// heads if it names a chant (or the words', with a warning, where the chant has no phrase
    /// left for the point or its phrase gives none), for the first point the heads on a head
    /// motif if it gives one (or the words', with a warning, where the motif gives none),
    /// otherwise those of the words. A restatement takes the subject of the point it restates.
    pub(crate) fn subjects(
        &self,
        p: usize,
        earlier: &[Arc<Soggetto>],
    ) -> Result<Found, GenerateError> {
        let part = &self.parts[p];
        if let PointKind::Restatement { of } = part.kind {
            return Ok(Found {
                subjects: earlier.get(of).map(|s| (**s).clone()).into_iter().collect(),
                fallback: None,
            });
        }
        let mut fallback = None;
        match &self.source {
            SubjectSource::Given(heads) => {
                let k = self.ordinal(p);
                if let Some(head) = heads.get(k) {
                    return Ok(Found {
                        subjects: vec![self.given(p, k, head)?],
                        fallback: None,
                    });
                }
            }
            SubjectSource::Chant { .. } => {
                if let Some(source) = self.chant_source(p) {
                    let subjects = self.chant_subjects(p, source, earlier);
                    if !subjects.is_empty() {
                        return Ok(Found {
                            subjects,
                            fallback: None,
                        });
                    }
                }
                fallback = Some(ChantFallback { point: p + 1 });
            }
            // The first point on the head motif; the others from their words.
            SubjectSource::Motif(_) => {
                if let Some(source) = self.chant_source(p) {
                    let subjects = self.chant_subjects(p, source, earlier);
                    if !subjects.is_empty() {
                        return Ok(Found {
                            subjects,
                            fallback: None,
                        });
                    }
                    fallback = Some(ChantFallback { point: p + 1 });
                }
            }
            SubjectSource::Text => {}
        }
        Ok(Found {
            subjects: self.text_subjects(p, earlier),
            fallback,
        })
    }

    /// The place of point `p` among the points that are not restatements.
    fn ordinal(&self, p: usize) -> usize {
        self.parts[..p]
            .iter()
            .filter(|part| !matches!(part.kind, PointKind::Restatement { .. }))
            .count()
    }

    /// The diatonic index of the final in the tenor's octave, where the builder sings its heads.
    fn reference(&self) -> i32 {
        reference(self.mode)
    }

    /// The section of the text, without its units: `kyrie`, `gloria`, `eigen`. Sections sung one
    /// after the other as one text (phase 14) open as the first: `sanctus` for
    /// `sanctus+pleni+osanna`.
    fn section(&self) -> &str {
        self.text.id.split([':', '+']).next().unwrap_or_default()
    }

    /// What Phase A needs to know of point `p`, with the head template and the start degree
    /// drawn from `rng` by their shares.
    fn ctx<'a>(&'a self, p: usize, chant: Option<&'a Source>, rng: &mut Rng) -> Ctx<'a> {
        let part = &self.parts[p];
        let start = usize::from(part.words.start);
        let flags = self.text.syllables[start].flags;
        let unit = self.text.unit_of(start);
        let lone =
            self.text.syllables[start].word == self.text.syllables[usize::from(unit.last)].word;
        let section = self.section();
        let opening = p == 0;
        let template = match section {
            _ if chant.is_some() => Template::Heads(CHANT_HEADS),
            "kyrie" | "kyrie2" if opening => Template::Heads(KYRIE_HEADS),
            "christe" if opening => Template::Heads(CHRISTE_HEADS),
            "agnus" | "agnus3" if opening => Template::Heads(AGNUS_HEADS),
            "sanctus" if opening => Template::Heads(SANCTUS_HEADS),
            "benedictus" if opening => Template::Heads(BENEDICTUS_HEADS),
            "osanna" | "osanna2" if opening => Template::Osanna,
            _ => self.foot(start),
        };
        let intervals = match section {
            _ if !opening => POINT_INTERVALS,
            "kyrie" | "kyrie2" => KYRIE_INTERVALS,
            "christe" => CHRISTE_INTERVALS,
            "agnus" | "agnus3" => AGNUS_INTERVALS,
            "sanctus" => SANCTUS_INTERVALS,
            "benedictus" => BENEDICTUS_INTERVALS,
            _ => OPENING_INTERVALS,
        };
        let ticks = match (chant.is_some(), self.text.kind) {
            (true, _) => 40..=72,
            (false, TextKind::Short) => 24..=64,
            (false, TextKind::Long) => 16..=48,
        };
        let least = self.head.unwrap_or(4).clamp(4, MOST_NOTES);
        // A head on a head motif sings every one of its tones and strikes some again.
        let notes = match chant {
            Some(source) if source.motif => {
                let tones = source.degrees.len();
                tones..=tones + MOTIF_REPEATS
            }
            _ => least..=MOST_NOTES,
        };
        let drawn = match template {
            Template::Heads(heads) => draw_head(heads, rng),
            Template::Osanna => draw_head(OSANNA_HEADS, rng),
            Template::Foot(_) | Template::Free => None,
        };
        let degrees = start_degrees(self.mode);
        let total: f64 = degrees.iter().map(|&(_, share)| share).sum();
        let mut u = rng.next_f64() * total;
        let degree = degrees.iter().find_map(|&(degree, share)| {
            u -= share;
            (u < 0.0).then_some(degree)
        });
        // The first interval and the leap are drawn for heads from the words only: a chant
        // gives its intervals.
        let (first_class, leap, wide, dotted_end) = match chant {
            Some(_) => (None, false, false, false),
            None => {
                let first_class = draw_class(intervals, rng);
                let share = if opening { LEAP_FIRST } else { LEAP_LATER };
                let leap = rng.next_f64() < share || first_class == Some(3);
                let share = if opening { WIDE_FIRST } else { WIDE_LATER };
                let wide = rng.next_f64() < share;
                let dotted_end = rng.next_f64() < DOTTED_END;
                (first_class, leap, wide, dotted_end)
            }
        };
        Ctx {
            p,
            part,
            from: super::points::state_before(&self.model, &self.parts, p),
            accented: flags.accented(),
            lone,
            template,
            drawn,
            degree,
            first_class,
            leap,
            wide,
            dotted_end,
            intervals,
            ticks,
            notes,
            chant,
        }
    }

    /// The foot of the word a point begins with, if it begins on an accented syllable.
    fn foot(&self, start: usize) -> Template {
        let syllables = &self.text.syllables;
        let flags = |i: usize| syllables.get(i).map(|s| s.flags);
        let Some(first) = flags(start).filter(|f| f.accented()) else {
            return Template::Free;
        };
        let word = syllables[start].word;
        let same_word = |i: usize| syllables.get(i).is_some_and(|s| s.word == word);
        if same_word(start + 2) && flags(start + 2).is_some_and(|f| f.contains(SylFlags::STRESS)) {
            Template::Foot(FOUR_SYLLABLES)
        } else if same_word(start + 1)
            && flags(start + 1).is_some_and(|f| f.contains(SylFlags::SHORT_PENULT))
        {
            Template::Foot(DACTYL)
        } else if first.contains(SylFlags::STRESS)
            && same_word(start + 1)
            && flags(start + 1).is_some_and(|f| f.contains(SylFlags::WORD_END))
        {
            Template::Foot(TROCHEE)
        } else {
            Template::Free
        }
    }

    /// The cost of a head's first values after its template; two more where they are not those
    /// of the drawn head.
    fn template_cost(&self, ctx: &Ctx, values: &[u8]) -> u32 {
        let undrawn = ctx
            .drawn
            .is_some_and(|head| values.len() >= head.len() && values[..head.len()] != *head);
        DRAWN * u32::from(undrawn) + self.share_of_template(ctx, values)
    }

    /// The cost of a head's first values by the shares of its template.
    fn share_of_template(&self, ctx: &Ctx, values: &[u8]) -> u32 {
        match ctx.template {
            Template::Heads(heads) => {
                let n = heads.first().map_or(3, |(v, _)| v.len());
                if values.len() < n {
                    return 0;
                }
                heads
                    .iter()
                    .find(|(head, _)| **head == values[..n])
                    .map_or(4, |&(_, share)| share_cost(share))
            }
            Template::Osanna => match values {
                [8, _, _, ..] => OSANNA_HEADS
                    .iter()
                    .find(|(head, _)| **head == values[..3])
                    .map_or(1, |&(_, share)| share_cost(share)),
                [_, _, _, ..] => 4,
                _ => 0,
            },
            Template::Foot(feet) => {
                if feet
                    .iter()
                    .any(|foot| values.len() >= foot.len() && values[..foot.len()] == **foot)
                {
                    0
                } else {
                    4
                }
            }
            Template::Free => 0,
        }
    }

    /// Steps the underlay over the written notes from index `first` on: the attack with
    /// `choice`, its tie continuations holding the syllable. The state and the cost; `None` where
    /// the profile forbids a case.
    fn step_text(
        &self,
        mut state: UState,
        written: &[Note],
        first: usize,
        start: u32,
        choice: Choice,
    ) -> Option<(UState, u32)> {
        let mut cost = 0;
        let mut tick = start;
        let mut found = Vec::new();
        for (i, note) in written.iter().enumerate().skip(first) {
            if let Some(event) = Event::at(written, i, tick) {
                let c = if i == first { choice } else { Choice::Hold };
                found.clear();
                state = self.model.step(&state, &event, c, &mut found);
                let (add, forbidden) = self.levels.price(&found);
                if forbidden.is_some() {
                    return None;
                }
                cost += add;
            }
            tick += note.duration.ticks();
        }
        Some((state, cost))
    }

    /// The choices at the attack of note `k` of a head: the point's first syllable at its entry,
    /// later the syllable held or the next one of the head words.
    fn choices(&self, ctx: &Ctx, d: &Decl, event: &Event, kind: Kind) -> Vec<Choice> {
        let words = &ctx.part.words;
        if d.values.is_empty() {
            return vec![Choice::Start(words.start)];
        }
        let mut options = Vec::new();
        self.model.options(&d.state, event, &mut options);
        let next = d.state.syllable().map_or(0, |x| x + 1);
        options.retain(|&c| match c {
            Choice::Hold => !matches!(kind, Kind::Again(_) | Kind::Repeated),
            Choice::Start(y) => usize::from(y) == next && y < ctx.part.head.end,
        });
        options
    }

    /// The pitch options of the next note of a head from chant: the chant's note struck again
    /// for a syllable, its next distinct note, or a passing note within a third or a fourth.
    fn chant_options(&self, source: &Source, d: &Decl) -> Vec<(i8, Kind)> {
        if let Fill::Then(degree) = d.fill {
            return vec![(degree, Kind::Passing(None))];
        }
        let notes = &source.degrees;
        let Some(&current) = d.degrees.last() else {
            return vec![(notes[0], Kind::Chant(0))];
        };
        // A head motif: its next tone, or the tone before struck again for a syllable; nothing
        // passes between its tones.
        if source.motif {
            let mut out = Vec::new();
            if d.repeated < MOTIF_REPEATS {
                out.push((current, Kind::Repeated));
            }
            if let Some(&next) = notes.get(d.next) {
                out.push((next, Kind::Chant(d.next)));
            }
            return out;
        }
        let mut out = Vec::new();
        let j = d.next;
        if notes.get(j) == Some(&current) {
            out.push((current, Kind::Again(j)));
        }
        let Some(k) = (j..notes.len()).find(|&k| notes[k] != current) else {
            return out;
        };
        out.push((notes[k], Kind::Chant(k)));
        if d.fill == Fill::Open {
            let diff = notes[k] - current;
            let towards = diff.signum();
            match diff.abs() {
                2 => out.push((current + towards, Kind::Passing(None))),
                3 => out.push((
                    current + towards,
                    Kind::Passing(Some(current + 2 * towards)),
                )),
                _ => {}
            }
        }
        out
    }

    /// The pitch of a note of a head in Phase A: a placeholder a second from the note before, or
    /// the chant's degree on the scale, flattened as [`realize`] does on the later note; `None`
    /// where that does not help or the move is forbidden.
    fn placeholder(&self, d: &Decl, degree: Option<i8>) -> Option<Pitch> {
        let Some(degree) = degree else {
            let k = d.values.len() as i32;
            return Some(Pitch::from_diatonic(self.reference() + 7 + k % 2));
        };
        let first = d.degrees.first().copied().unwrap_or(degree);
        let base = self.reference() + i32::from(first.rem_euclid(7));
        let pitch = self.mode.scale_pitch(base + i32::from(degree - first));
        match d.written.last().and_then(|n| n.pitch) {
            Some(prev) => {
                let pitch = if bad(prev, pitch) {
                    flattened(self.mode, pitch).filter(|&f| !bad(prev, f))?
                } else {
                    pitch
                };
                self.moves.forbids(prev, pitch).is_none().then_some(pitch)
            }
            None => Some(pitch),
        }
    }

    /// The pitch of a note of a head on a head motif in Phase A: the motif's tone with its
    /// alteration, as Kyrie I sang it (the tone before for a note struck again); `None` where the
    /// move to it is forbidden.
    fn motif_pitch(&self, source: &Source, d: &Decl, degree: i8, kind: Kind) -> Option<Pitch> {
        let alter = match kind {
            Kind::Chant(index) => source.alters[index],
            _ => *d.alters.last()?,
        };
        let first = d.degrees.first().copied().unwrap_or(degree);
        let base = self.reference() + i32::from(first.rem_euclid(7));
        let scale = self.mode.scale_pitch(base + i32::from(degree - first));
        let pitch = scale.with_alter(scale.alter + alter);
        match d.written.last().and_then(|n| n.pitch) {
            Some(prev) => self.moves.forbids(prev, pitch).is_none().then_some(pitch),
            None => Some(pitch),
        }
    }

    /// The children of a head in Phase A: each next value and choice, and from chant each next
    /// note.
    fn children(&self, ctx: &Ctx, d: &Decl, out: &mut Vec<Decl>) {
        let k = d.values.len();
        if k >= *ctx.notes.end() {
            return;
        }
        let values: Vec<(u8, u32, bool)> = match d.values.last() {
            None => FIRST_VALUES.iter().map(|&v| (v, 0, false)).collect(),
            Some(&last) => transitions(last).to_vec(),
        };
        let pitches: Vec<(Option<i8>, Kind)> = match ctx.chant {
            None => vec![(None, Kind::Free)],
            Some(source) => self
                .chant_options(source, d)
                .into_iter()
                .map(|(degree, kind)| (Some(degree), kind))
                .collect(),
        };
        for &(degree, kind) in &pitches {
            let pitch = match (ctx.chant, degree) {
                (Some(source), Some(degree)) if source.motif => {
                    self.motif_pitch(source, d, degree, kind)
                }
                _ => self.placeholder(d, degree),
            };
            let Some(pitch) = pitch else {
                continue;
            };
            for &(value, style, drop) in &values {
                // Heads from chant begin in long values: quarters from the fifth note on.
                if ctx.chant.is_some() && value < 4 && k < 4 {
                    continue;
                }
                self.child(ctx, d, (value, style, drop), pitch, degree, kind, out);
            }
        }
    }

    /// A head with one more note.
    #[allow(clippy::too_many_arguments)]
    fn child(
        &self,
        ctx: &Ctx,
        d: &Decl,
        (value, style, drop): (u8, u32, bool),
        pitch: Pitch,
        degree: Option<i8>,
        kind: Kind,
        out: &mut Vec<Decl>,
    ) {
        let k = d.values.len();
        let mut written = d.written.clone();
        let first = written.len();
        write(
            &mut written,
            pitch,
            d.tick,
            u32::from(value),
            !self.mode.is_scale_pitch(pitch),
        );
        let tick = d.tick + u32::from(value);
        // The measure rhythms: whole measures priced once the next one has begun, the others
        // prefixes of an allowed rhythm.
        let mut rules = d.rules;
        let all = measures(&written);
        let mut priced = d.priced;
        while priced < all.len() && (priced as u32 + 1) * MEASURE < tick {
            let (tokens, tied) = &all[priced];
            match self.rhythms.level(tokens, *tied) {
                Level::Forbidden => return,
                level => rules += level.cost(),
            }
            priced += 1;
        }
        if !all[priced..]
            .iter()
            .all(|(tokens, _)| self.rhythms.prefix(tokens))
        {
            return;
        }
        // The rules the checker prices again.
        if drop {
            match self.drop {
                Level::Forbidden => return,
                level => rules += level.cost(),
            }
        }
        if value > 8 {
            match self.over_whole {
                Level::Forbidden => return,
                level => rules += level.cost(),
            }
        }
        let longer_before = d.values.iter().skip(1).any(|&v| v > d.values[0]);
        if ctx.p == 0
            && ctx.accented
            && (1..=3).contains(&k)
            && value > d.values[0]
            && !longer_before
        {
            match self.not_longest {
                Level::Forbidden => return,
                level => rules += level.cost(),
            }
        }
        // Style.
        let mut style = d.style + style;
        let mut values = d.values.clone();
        values.push(value);
        if values.len() == 3 || (values.len() == 4 && ctx.chant.is_some()) {
            style += self.template_cost(ctx, &values);
        }
        let black = value < 4;
        if black && (1..=2).contains(&k) && !(value == 2 && d.values[k - 1] == 6) {
            style += 2;
        }
        if let Kind::Passing(then) = kind {
            // A quarter on the weak half of a minim fills a third freely; a filled fourth
            // costs two, counted at its first note.
            let weak = value == 2 && d.tick % 4 == 2;
            style += match then {
                Some(_) => 2,
                None if matches!(d.fill, Fill::Then(_)) => 0,
                None => u32::from(!weak),
            };
        }
        let Some(Event::Attack(attack)) = Event::at(&written, first, d.tick) else {
            return;
        };
        for choice in self.choices(ctx, d, &Event::Attack(attack), kind) {
            let Some((state, txt)) = self.step_text(d.state, &written, first, d.tick, choice)
            else {
                continue;
            };
            let mut child = Decl {
                values: values.clone(),
                choices: d.choices.clone(),
                degrees: d.degrees.clone(),
                alters: d.alters.clone(),
                passing: d.passing.clone(),
                written: written.clone(),
                state,
                tick,
                priced,
                txt: d.txt + txt,
                rules,
                style,
                melisma: d.melisma,
                rank: 0.0,
                next: d.next,
                distinct: d.distinct,
                fill: d.fill,
                repeated: d.repeated,
            };
            // A syllable on the quarter of a dotted half: the TXT rules allow an unaccented one,
            // but Palestrina's heads leave the quarter in the melisma, at most the short penult
            // of a dactyl takes it (Jeppesen's underlay texts none of 13 such quarters,
            // docs/regeln.md § 17), and every entry repeats the head's syllables.
            if let Choice::Start(y) = choice
                && value == 2
                && d.values.last() == Some(&6)
            {
                let flags = self.text.syllables[usize::from(y)].flags;
                child.style += if flags.contains(SylFlags::SHORT_PENULT) {
                    1
                } else {
                    3
                };
            }
            child.choices.push(choice);
            if let Some(degree) = degree {
                child.degrees.push(degree);
                child.alters.push(match ctx.chant {
                    Some(source) if source.motif => pitch.alter - self.mode.scale_alter(pitch.step),
                    _ => 0,
                });
                child.passing.push(matches!(kind, Kind::Passing(_)));
            }
            match kind {
                Kind::Free => {}
                Kind::Chant(index) => {
                    child.next = index + 1;
                    child.distinct += 1;
                }
                Kind::Again(index) => child.next = index + 1,
                Kind::Passing(then) => {
                    child.fill = match then {
                        Some(next) => Fill::Then(next),
                        None => Fill::Used,
                    };
                }
                Kind::Repeated => child.repeated += 1,
            }
            // The first syllable's melisma, priced where the second syllable begins.
            if !child.melisma
                && ctx.chant.is_none()
                && matches!(choice, Choice::Start(y) if y > ctx.part.words.start)
            {
                child.style += self.melisma_cost(ctx, &d.values);
                child.melisma = true;
            }
            // A later syllable's melisma, priced where the next one begins.
            if ctx.chant.is_none()
                && let Choice::Start(y) = choice
                && y > ctx.part.words.start + 1
            {
                child.style += self.monosyllable_cost(y - 1, &d.choices);
            }
            out.push(child);
        }
    }

    /// The cost of the notes on a word of one syllable after the point's first, `y`, which the
    /// `choices` of a head end with: one free, two one, more four. Palestrina declaims such words
    /// in his heads on one note and keeps the melisma for the stressed syllable of a longer word
    /// (Jeppesen, p. 86: Ad, te, le-vá-vi; SOG-DECLAIM); a head that reaches past te or es
    /// ([`super::points`]) would otherwise load its notes on them.
    fn monosyllable_cost(&self, y: u16, choices: &[Choice]) -> u32 {
        if self.text.word_of(usize::from(y)).range().len() != 1 {
            return 0;
        }
        let held = choices
            .iter()
            .rev()
            .take_while(|c| **c == Choice::Hold)
            .count();
        match held {
            0 => 0,
            1 => 1,
            _ => 4,
        }
    }

    /// The cost of the notes on a point's first syllable: one free, two white notes two, more
    /// four; in a point of one-word units one. Ten of the twelve texted heads of Palestrina's that
    /// Jeppesen prints are syllabic (SOG-DECLAIM).
    fn melisma_cost(&self, ctx: &Ctx, values: &[u8]) -> u32 {
        match values {
            [_] => 0,
            _ if ctx.lone => 1,
            [a, b] if *a >= 4 && *b >= 4 => 2,
            _ => 4,
        }
    }

    /// A complete head with what its completion costs, if it is complete: every syllable of
    /// the head words begun, enough notes, a white last note, a length in range; from chant five
    /// to ten of its distinct notes and no passing note at the end, on a head motif every tone of
    /// the motif.
    fn finished(&self, ctx: &Ctx, d: &Decl) -> Option<Decl> {
        let last = *d.values.last()?;
        let head_end = usize::from(ctx.part.head.end);
        if d.state.syllable() != Some(head_end - 1)
            || !ctx.notes.contains(&d.values.len())
            || !ctx.ticks.contains(&d.tick)
            || last < 4
        {
            return None;
        }
        let mut done = d.clone();
        let start = d.tick - u32::from(last);
        if !start.is_multiple_of(MEASURE) {
            match self.end {
                Level::Forbidden => return None,
                level => done.rules += level.cost(),
            }
        }
        match ctx.chant {
            // A head motif: every one of its tones, in order (MASS-MOTIF).
            Some(source) if source.motif => {
                if d.distinct != source.degrees.len() {
                    return None;
                }
            }
            Some(_) => {
                if d.passing.last() == Some(&true)
                    || matches!(d.fill, Fill::Then(_))
                    || !(5..=10).contains(&d.distinct)
                {
                    return None;
                }
                // Six to eight distinct notes of the chant (PAR-HEAD-LENGTH, as its verifier
                // reads the corpus), ending on the final or the fifth, where a clausula would
                // close the head (PAR-CADENCE-ADJUST).
                done.style += match d.distinct {
                    7 | 8 => 0,
                    6 | 9 => 1,
                    _ => 2,
                };
                let last = d.degrees.last().map_or(0, |&g| g.rem_euclid(7));
                let fifth = if self.mode.mode == Mode::Phrygian {
                    3
                } else {
                    4
                };
                if last != 0 && last != fifth {
                    done.style += 1;
                }
            }
            None => {
                done.style += figure_cost(&d.values);
                done.style += span_cost(ctx.p == 0, &d.values);
                // A last note of a dotted half, as drawn.
                done.style += match (ctx.dotted_end, last == 6) {
                    (true, false) => DRAWN,
                    (false, true) => 1,
                    _ => 0,
                };
                if !done.melisma {
                    done.style += self.melisma_cost(ctx, &d.values);
                    done.melisma = true;
                }
                if head_end - 1 > usize::from(ctx.part.words.start) {
                    done.style += self.monosyllable_cost((head_end - 1) as u16, &d.choices);
                }
            }
        }
        done.rank = d.rank + f64::from(done.cost() - d.cost());
        Some(done)
    }

    /// Phase A: the declamations of a head, the cheapest first, at most [`SKELETONS`].
    fn declaim(&self, ctx: &Ctx, rng: &mut Rng) -> Vec<Decl> {
        let mut beam = vec![Decl::new(ctx.from)];
        let mut done: Vec<Decl> = Vec::new();
        while !beam.is_empty() {
            let mut next = Vec::new();
            for d in &beam {
                self.children(ctx, d, &mut next);
            }
            for child in &mut next {
                child.rank = f64::from(child.cost()) + NOISE * rng.next_f64();
            }
            done.extend(next.iter().filter_map(|child| self.finished(ctx, child)));
            next.sort_by(|a, b| a.rank.total_cmp(&b.rank));
            next.truncate(DECLAMATION_BEAM);
            beam = next;
        }
        done.sort_by(|a, b| a.rank.total_cmp(&b.rank));
        done.truncate(SKELETONS);
        done
    }

    /// The pricing checker of point `p`'s leader.
    fn checker(&self, p: usize) -> &Checker {
        if p == 0 { &self.first } else { &self.later }
    }

    /// The one-voice score of a head in the tenor, with the text and the head as the plan of
    /// point `p`'s leader.
    fn voice(&self, p: usize, words: &Range<u16>, written: &[Note]) -> Score {
        let voice = Voice::new(
            VoiceType::Tenor,
            Role::Counterpoint(Species::Free),
            written.to_vec(),
        );
        let notes = voice.soundings();
        let mut score = Score::new(self.mode, vec![voice]);
        score.text = Some(ScoreText {
            text: Arc::clone(&self.text),
            voices: vec![false],
        });
        let point = ScorePoint {
            words: words.clone(),
            entries: vec![ScoreEntry {
                voice: 0,
                start: 0,
                rest: 0,
                notes: notes.iter().map(|s| (s.pitch, s.end - s.start)).collect(),
            }],
            goal: None,
        };
        let empty = ScorePoint {
            words: 0..0,
            entries: Vec::new(),
            goal: None,
        };
        let mut points = Vec::new();
        if p > 0 {
            points.push(empty);
        }
        points.push(point);
        score.imitation = Some(Arc::new(ScoreImitation { points }));
        score
    }

    /// The violations of a head's one-voice score decided in a window.
    fn check(
        &self,
        p: usize,
        words: &Range<u16>,
        written: &[Note],
        window: Window,
    ) -> Vec<Violation> {
        let score = self.voice(p, words, written);
        let analysis = Analysis::new(&score);
        let mut out = Vec::new();
        self.checker(p)
            .check_window(&score, &analysis, window, &[true], &mut out);
        out
    }

    /// Phase B: the melodies of a skeleton, priced and answered, at most [`MELODIES`] that pass
    /// the stretto test, the cheapest first.
    fn melodies(&self, ctx: &Ctx, skeleton: &Decl, seed: u64) -> Vec<Soggetto> {
        let mut rng = generator(seed);
        let words = &ctx.part.words;
        let values: Vec<u32> = skeleton.values.iter().map(|&v| u32::from(v)).collect();
        let starts: Vec<u32> = values
            .iter()
            .scan(0, |tick, &v| {
                let at = *tick;
                *tick += v;
                Some(at)
            })
            .collect();
        let mut beam: Vec<Tune> = Vec::new();
        for &(degree, share) in start_degrees(self.mode) {
            let pitch = self.mode.scale_pitch(self.reference() + i32::from(degree));
            let mut written = Vec::new();
            write(&mut written, pitch, 0, values[0], false);
            let Some((state, txt)) = self.step_text(ctx.from, &written, 0, 0, skeleton.choices[0])
            else {
                continue;
            };
            let found = self.check(ctx.p, words, &written, Window::partial(0, 0));
            if forbidden(&found, false) {
                continue;
            }
            let mut tune = Tune {
                degree,
                steps: vec![0],
                pitches: vec![pitch],
                written,
                state,
                attack: 0,
                txt,
                checked: priced(&found, false),
                style: share_cost(share)
                    + DRAWN * u32::from(ctx.degree.is_some_and(|drawn| drawn != degree)),
                low: 0,
                high: 0,
                debt: DRAWN * (u32::from(ctx.leap) + u32::from(ctx.wide)),
                rank: 0.0,
            };
            tune.rank = f64::from(tune.cost()) + NOISE * rng.next_f64();
            beam.push(tune);
        }
        for (k, &start) in starts.iter().enumerate().skip(1) {
            let mut next = Vec::new();
            for tune in &beam {
                for &s in &STEPS {
                    if let Some(child) = self.extend(ctx, skeleton, tune, k, s, start) {
                        next.push(child);
                    }
                }
            }
            for child in &mut next {
                child.rank = f64::from(child.cost()) + NOISE * rng.next_f64();
            }
            next.sort_by(|a, b| a.rank.total_cmp(&b.rank));
            next.truncate(PITCH_BEAM);
            beam = next;
        }
        // The complete melodies: realised as the entries sing them (a mi between two la's is
        // fa), priced once more as a whole, answered, tested in stretto.
        let end = values.iter().sum();
        let mut found: Vec<(f64, Soggetto)> = Vec::new();
        for tune in beam {
            let first = self.reference() + i32::from(tune.degree);
            let Some(pitches) = realize(self.mode, first, &tune.steps) else {
                continue;
            };
            let written = written_of(self.mode, &pitches, &values);
            let violations = self.check(ctx.p, words, &written, Window::partial(0, end));
            if forbidden(&violations, true) {
                continue;
            }
            let style = tune.style + tune.debt + climax_cost(&pitches, &values);
            let cost = skeleton.style + tune.txt + priced(&violations, true) + style;
            let notes = tune
                .steps
                .iter()
                .zip(&skeleton.values)
                .zip(&skeleton.choices)
                .map(|((&step, &ticks), &sung)| SubjectNote {
                    step,
                    ticks,
                    sung,
                    alter: 0,
                })
                .collect();
            let mut subject = Soggetto {
                degree: tune.degree,
                notes,
                words: ctx.part.head.clone(),
                origin: Origin::Text,
                answers: Vec::new(),
                stretto: Vec::new(),
                cost,
            };
            if self.answer_and_test(ctx.p, &mut subject) {
                found.push((f64::from(subject.cost) + NOISE * rng.next_f64(), subject));
            }
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        found
            .into_iter()
            .take(MELODIES)
            .map(|(_, subject)| subject)
            .collect()
    }

    /// A melody of Phase B with note `k` a step `s` from the note before, priced; `None` where a
    /// prohibition or the octave forbids it.
    fn extend(
        &self,
        ctx: &Ctx,
        skeleton: &Decl,
        tune: &Tune,
        k: usize,
        s: i8,
        start: u32,
    ) -> Option<Tune> {
        let step = tune.steps[k - 1] + s;
        let (low, high) = (tune.low.min(step), tune.high.max(step));
        if high - low > 7 {
            return None;
        }
        let prev = tune.pitches[k - 1];
        let mut pitch = self
            .mode
            .scale_pitch(self.reference() + i32::from(tune.degree) + i32::from(step));
        if bad(prev, pitch) {
            pitch = flattened(self.mode, pitch).filter(|&f| !bad(prev, f))?;
        }
        if self.moves.forbids(prev, pitch).is_some() {
            return None;
        }
        let mut written = tune.written.clone();
        let first = written.len();
        let value = u32::from(skeleton.values[k]);
        write(
            &mut written,
            pitch,
            start,
            value,
            !self.mode.is_scale_pitch(pitch),
        );
        let (state, txt) =
            self.step_text(tune.state, &written, first, start, skeleton.choices[k])?;
        let window = Window::partial(tune.attack + 1, start);
        let found = self.check(ctx.p, &ctx.part.words, &written, window);
        if forbidden(&found, false) {
            return None;
        }
        let mut style = tune.style;
        if k == 1 {
            style += ctx
                .intervals
                .iter()
                .find(|(i, _)| *i == s)
                .map_or(4, |&(_, share)| share_cost(share));
            // The class of the first interval drawn by its share (SOG-VARY).
            if ctx.first_class.is_some_and(|drawn| drawn != class(s)) {
                style += DRAWN;
            }
        } else {
            let before = tune.steps[k - 1] - tune.steps[k - 2];
            // Going on in the direction of a leap of a fourth or more.
            if before.abs() >= 3 && s != 0 && s.signum() == before.signum() {
                style += 1;
            }
            // Rising, the larger interval first.
            if before > 0 && s > before && s >= 2 {
                style += 1;
            }
            // Through a quarter after a white note.
            if skeleton.values[k - 1] == 2 && skeleton.values[k - 2] >= 4 {
                style += quarter_cost(before, s);
            }
        }
        // A drawn leap, taken by the fourth interval, and a drawn span of a seventh or an
        // octave, reached by the last note.
        let mut debt = tune.debt;
        if ctx.leap && k <= LEAP_BY && s.abs() >= 3 && !leapt(&tune.steps[..k]) {
            debt -= DRAWN;
        }
        if ctx.wide && high - low >= SEVENTH && tune.high - tune.low < SEVENTH {
            debt -= DRAWN;
        }
        let mut steps = tune.steps.clone();
        steps.push(step);
        let mut pitches = tune.pitches.clone();
        pitches.push(pitch);
        Some(Tune {
            degree: tune.degree,
            steps,
            pitches,
            written,
            state,
            attack: start,
            txt: tune.txt + txt,
            checked: tune.checked + priced(&found, false),
            style,
            low,
            high,
            debt,
            rank: 0.0,
        })
    }

    /// Answers a subject and runs its stretto test; whether it passes. Adds the test's cost and
    /// what its range costs ([`Builder::range_cost`]).
    fn answer_and_test(&self, p: usize, subject: &mut Soggetto) -> bool {
        subject.answers = self.answers(p, subject);
        if subject.answers.is_empty() {
            return false;
        }
        let (stretto, cost) = stretto(self.mode, subject);
        subject.stretto = stretto;
        match cost {
            Some(cost) => {
                subject.cost += cost + self.range_cost(subject);
                true
            }
            None => false,
        }
    }

    /// What a subject costs for the voices of the choir that cannot sing its head within their
    /// ranges ([`BEYOND_RANGE`] each), and more for those that cannot sing it within a step of
    /// their staves ([`BEYOND_STAFF`]).
    fn range_cost(&self, subject: &Soggetto) -> u32 {
        let first = reference(self.mode) + i32::from(subject.degree);
        // The lowest and highest note of each answer, in some octave.
        let spans: Vec<(i32, i32)> = subject
            .answers
            .iter()
            .filter_map(|&(relation, answer)| {
                let shift = *shifts(self.mode, subject.degree, relation).first()?;
                let pitches = subject.pitches(self.mode, first, shift, answer)?;
                let steps = || pitches.iter().map(|p| p.diatonic());
                Some((steps().min()?, steps().max()?))
            })
            .collect();
        let fits = |&(low, high): &(i32, i32)| {
            spans.iter().any(|&(lo, hi)| {
                // The octave that brings the lowest note to the range's bottom or just above.
                let up = (low - lo).div_euclid(7) + i32::from((low - lo).rem_euclid(7) != 0);
                hi + 7 * up <= high
            })
        };
        self.ranges
            .iter()
            .map(|(range, staff)| match (fits(range), fits(staff)) {
                (true, _) => 0,
                (false, true) => BEYOND_RANGE,
                (false, false) => BEYOND_RANGE + BEYOND_STAFF,
            })
            .sum()
    }

    /// The relations an entry of point `p` may take besides the unison and the other main
    /// degree: those the request names.
    fn requested(&self, p: usize) -> Vec<Relation> {
        let mut out = Vec::new();
        for relation in self.relations.get(p).into_iter().flatten().flatten() {
            if matches!(relation, Relation::Steps(_)) && !out.contains(relation) {
                out.push(*relation);
            }
        }
        out
    }

    /// The answers of a subject of point `p` ([`answers`]), as the request allows them.
    fn answers(&self, p: usize, subject: &Soggetto) -> Vec<(Relation, Answer)> {
        answers(
            self.mode,
            self.moves,
            self.answer,
            &self.requested(p),
            subject,
        )
    }
}

/// The diatonic index of the final in the tenor's octave, where the builder sings its heads.
fn reference(mode: ModeSpec) -> i32 {
    mode.final_pitch(3).diatonic()
}

/// The answers of a subject: at the unison or octave, on the other main degree and at the
/// `requested` steps, each real where it has no move that `moves` forbids, modal where the head
/// leaps between the final and the fifth in its first two intervals and `choice` allows it, or
/// where the real answer cannot be sung (Fux, p. 144; Vicentino IV.32). A modal answer that
/// widens the head beyond an octave is dropped, as a leader beyond an octave is (IMI-SUBJECT
/// `beyond_octave`, never in Palestrina's heads). A given head keeps its accidentals
/// ([`Soggetto::pitches`]); a relation in which it cannot is dropped.
fn answers(
    mode: ModeSpec,
    moves: Moves,
    choice: AnswerChoice,
    requested: &[Relation],
    subject: &Soggetto,
) -> Vec<(Relation, Answer)> {
    let degree = subject.degree;
    let steps = subject.steps();
    let mut relations = vec![Relation::Same];
    match dominant(mode, degree) {
        Some(_) => relations.push(Relation::Dominant),
        // A head from chant or given notes may begin off the main degrees; Palestrina answers
        // chant heads at the fifth and the fourth all the same.
        None => relations.extend([Relation::Steps(4), Relation::Steps(-3)]),
    }
    for relation in requested {
        if !relations.contains(relation) {
            relations.push(*relation);
        }
    }
    let first = reference(mode) + i32::from(degree);
    let mut out = Vec::new();
    for relation in relations {
        let Some(&shift) = shifts(mode, degree, relation).first() else {
            continue;
        };
        let sung = |answer: Answer| {
            subject
                .pitches(mode, first, shift, answer)
                .is_some_and(|pitches| moves.first(&pitches).is_none() && span(&pitches) <= OCTAVE)
        };
        let real = sung(Answer::Real);
        let modal = (relation == Relation::Dominant)
            .then(|| {
                (0..2u8)
                    .find(|&at| joins_main_degrees(mode, degree, &steps, usize::from(at)))
                    .map(|at| Answer::Modal { at })
            })
            .flatten()
            .filter(|&answer| sung(answer));
        let mut push = |answer: Answer| out.push((relation, answer));
        match (choice, real, modal) {
            (AnswerChoice::Real, true, _) => push(Answer::Real),
            (AnswerChoice::Real, false, _) => {}
            (AnswerChoice::Modal, _, Some(modal)) => push(modal),
            (AnswerChoice::Modal, true, None) => push(Answer::Real),
            (AnswerChoice::Auto, true, modal) => {
                push(Answer::Real);
                if let Some(modal) = modal {
                    push(modal);
                }
            }
            (AnswerChoice::Auto, false, Some(modal)) => push(modal),
            (_, false, None) => {}
        }
    }
    out
}

/// What a stretto that passes costs, by its delay and its relation: on the other main degree a
/// semibreve or a breve after the leader nothing, a dotted breve or two breves later one, only on
/// the minim beat two; at the unison or the octave only, two. Palestrina's second entries come a
/// semibreve to two breves after the first (four voices: 38, 18, 21 and 16 %, on the minim beat
/// 5 %), at the fourth or fifth in 75 % and at the unison or octave in 20.5 % (docs/regeln.md
/// § 18). A subject needs a few good strettos, not all: most of his heads pass only some of the
/// delays, and most of them cost one or nothing here.
fn stretto_cost(other: bool, delay: u32, variant: FirstNote) -> u32 {
    match (other, variant) {
        (true, FirstNote::AsLeader) if delay <= 16 => 0,
        (true, FirstNote::AsLeader) => 1,
        _ => 2,
    }
}

/// The stretto test (Fux, p. 247): the pair checks of two fixed lines ([`check::alone`]) on the
/// leader and one follower, other voices silent, for every answer at both octaves, at the delays
/// 8 to 32 with the leader's first value and 4 to 28 with a shortened or syncopated first note,
/// each shorter than the leader's head. The strettos that pass, and what the best of them within
/// two breves costs ([`stretto_cost`]); `None` where none passes there.
fn stretto(mode: ModeSpec, subject: &Soggetto) -> (Vec<(i8, u32, FirstNote)>, Option<u32>) {
    let values = subject.values();
    let head: u32 = values.iter().sum();
    let first = reference(mode) + i32::from(subject.degree);
    let Some(leader_pitches) = subject.pitches(mode, first, 0, Answer::Real) else {
        return (Vec::new(), None);
    };
    let leader = tones(&leader_pitches, &values, 0, 0);
    let mut passed: Vec<(i8, u32, FirstNote)> = Vec::new();
    let mut best: Option<u32> = None;
    // Off the main degrees an answer at the fifth or the fourth counts as on the other one.
    let main = dominant(mode, subject.degree).is_some();
    let mut tested: Vec<(Relation, i8)> = Vec::new();
    for &(relation, _) in &subject.answers {
        for shift in shifts(mode, subject.degree, relation) {
            if !tested.contains(&(relation, shift)) {
                tested.push((relation, shift));
            }
        }
    }
    for (relation, shift) in tested {
        let other = match relation {
            Relation::Dominant => true,
            Relation::Same => false,
            Relation::Steps(n) => !main && matches!(n.rem_euclid(7), 3 | 4),
        };
        let followers: Vec<Vec<Pitch>> = subject
            .answers
            .iter()
            .filter(|(r, _)| *r == relation)
            .filter_map(|&(_, answer)| subject.pitches(mode, first, shift, answer))
            .collect();
        let delays = WHOLE_DELAYS
            .iter()
            .map(|&d| (d, &[FirstNote::AsLeader][..]))
            .chain(
                MINIM_DELAYS
                    .iter()
                    .map(|&d| (d, &[FirstNote::Shortened, FirstNote::Syncopated][..])),
            );
        for (delay, variants) in delays {
            if delay >= head {
                continue;
            }
            for &variant in variants {
                // A first note of a whole or longer may lose a minim (8 to 4, 12 to 8, 16 to
                // 12), a shorter one only be syncopated.
                let cut = match variant {
                    FirstNote::Shortened if values[0] < 8 => continue,
                    FirstNote::Shortened => 4,
                    _ => 0,
                };
                let good = followers.iter().any(|pitches| {
                    let follower = tones(pitches, &values, delay, cut);
                    check::alone(&leader, &follower).is_none()
                });
                if good {
                    passed.push((shift, delay, variant));
                    if delay <= LATEST && (other || relation == Relation::Same) {
                        let cost = stretto_cost(other, delay, variant);
                        best = Some(best.map_or(cost, |b| b.min(cost)));
                    }
                }
            }
        }
    }
    (passed, best)
}

impl Soggetto {
    /// Answers the head and runs the stretto test of the subject builder on it, as the builder
    /// does for every subject, for `voices` voices under `checker`'s melodic prohibitions, with
    /// real and modal answers and no other relations: sets `answers` and `stretto`, and gives what
    /// the strettos cost, `None` where the builder would not keep the head. The test is
    /// calibrated on the heads of Palestrina's points.
    pub fn test_stretto(
        &mut self,
        checker: &Checker,
        mode: ModeSpec,
        voices: usize,
    ) -> Option<u32> {
        let moves = Moves::new(checker, voices >= 3);
        self.answers = answers(mode, moves, AnswerChoice::Auto, &[], self);
        let (stretto, cost) = stretto(mode, self);
        self.stretto = stretto;
        cost.filter(|_| !self.answers.is_empty())
    }
}

impl Builder {
    /// The subjects of point `p` from its words, at most [`SUBJECTS`], best first;
    /// none that begins like the subject of an earlier point (SOG-VARY).
    pub(crate) fn text_subjects(&self, p: usize, earlier: &[Arc<Soggetto>]) -> Vec<Soggetto> {
        let seed = sub_seed(self.seed, p);
        let mut rng = generator(seed);
        let ctx = self.ctx(p, None, &mut rng);
        let skeletons = self.declaim(&ctx, &mut rng);
        let seeds: Vec<u64> = (0..skeletons.len())
            .map(|i| sub_seed(seed, i + SUBJECTS))
            .collect();
        let jobs: Vec<(&Decl, u64)> = skeletons.iter().zip(seeds).collect();
        let found: Vec<Vec<Soggetto>> = each(&jobs, |&(skeleton, seed)| {
            self.melodies(&ctx, skeleton, seed)
        });
        self.select(found.into_iter().flatten().collect(), earlier, &mut rng)
    }

    /// The subjects of point `p` from a chant phrase: Phase A on the chant's notes, then
    /// answered and tested as those from the words; at most [`SUBJECTS`], best first. On a head
    /// motif (phase 14) Phase A declaims its tones, and the head keeps their pitches with their
    /// accidentals as Kyrie I sang them, as a given head keeps its own ([`Origin::Motif`]).
    pub(crate) fn chant_subjects(
        &self,
        p: usize,
        source: &Source,
        earlier: &[Arc<Soggetto>],
    ) -> Vec<Soggetto> {
        let mut rng = generator(sub_seed(self.seed, p));
        let ctx = self.ctx(p, Some(source), &mut rng);
        let skeletons = self.declaim(&ctx, &mut rng);
        let words = &ctx.part.words;
        let mut found = Vec::new();
        for skeleton in skeletons {
            let first = skeleton.degrees[0];
            let end = skeleton.tick;
            // The head as the entries sing it: the chant's notes on the scale, a mi between two
            // la's sung fa; a head motif's as Kyrie I sang them.
            let steps: Vec<i8> = skeleton.degrees.iter().map(|&g| g - first).collect();
            let values: Vec<u32> = skeleton.values.iter().map(|&v| u32::from(v)).collect();
            let at = self.reference() + i32::from(first.rem_euclid(7));
            let pitches = if source.motif {
                let exact = |(&step, &alter): (&i8, &i8)| {
                    let pitch = self.mode.scale_pitch(at + i32::from(step));
                    pitch.with_alter(pitch.alter + alter)
                };
                let pitches: Vec<Pitch> = steps.iter().zip(&skeleton.alters).map(exact).collect();
                if self.moves.first(&pitches).is_some() {
                    continue;
                }
                pitches
            } else {
                let Some(pitches) = realize(self.mode, at, &steps) else {
                    continue;
                };
                pitches
            };
            let written = written_of(self.mode, &pitches, &values);
            let violations = self.check(p, words, &written, Window::partial(0, end));
            if forbidden(&violations, true) {
                continue;
            }
            let cost = skeleton.style
                + skeleton.txt
                + priced(&violations, true)
                + climax_cost(&pitches, &values);
            let notes = steps
                .iter()
                .zip(&skeleton.values)
                .zip(&skeleton.choices)
                .zip(&skeleton.alters)
                .map(|(((&step, &ticks), &sung), &alter)| SubjectNote {
                    step,
                    ticks,
                    sung,
                    alter,
                })
                .collect();
            let origin = if source.motif {
                Origin::Motif
            } else {
                Origin::Chant {
                    phrase: source.phrase,
                }
            };
            let mut subject = Soggetto {
                degree: first.rem_euclid(7),
                notes,
                words: ctx.part.head.clone(),
                origin,
                answers: Vec::new(),
                stretto: Vec::new(),
                cost,
            };
            if self.answer_and_test(p, &mut subject) {
                found.push(subject);
            }
        }
        self.select(found, earlier, &mut rng)
    }

    /// The best subjects of a point: without those that begin like an earlier point's, ranked by
    /// their costs with noise drawn in their order, at most [`SUBJECTS`].
    fn select(
        &self,
        found: Vec<Soggetto>,
        earlier: &[Arc<Soggetto>],
        rng: &mut Rng,
    ) -> Vec<Soggetto> {
        let mut ranked: Vec<(f64, Soggetto)> = Vec::new();
        for subject in found {
            let noise = NOISE * rng.next_f64();
            let repeated = earlier.iter().any(|e| e.begins_like(&subject))
                || ranked.iter().any(|(_, s)| s.notes == subject.notes);
            if !repeated {
                ranked.push((f64::from(subject.cost) + noise, subject));
            }
        }
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
        ranked
            .into_iter()
            .take(SUBJECTS)
            .map(|(_, subject)| subject)
            .collect()
    }

    /// A given subject for point `p`, the `index`th of the request: 4 to 12 notes of a
    /// quarter or longer, without rests, within an octave, without a forbidden melodic move in
    /// any planned answer, in measures whose rhythms can be those of free counterpoint, and with
    /// an underlay of the point's head words without a forbidden case, the cheapest
    /// ([`Model::best_prefix`]). Otherwise [`GenerateError::SubjectInvalid`], naming the
    /// subject and the note (both from 1) and the rule.
    pub(crate) fn given(
        &self,
        p: usize,
        index: usize,
        notes: &[Note],
    ) -> Result<Soggetto, GenerateError> {
        let invalid = |note: usize, rule: &'static str| GenerateError::SubjectInvalid {
            subject: index + 1,
            note: note + 1,
            rule,
        };
        // The head as sung: tied notes merged.
        let mut head: Vec<(Pitch, u32)> = Vec::new();
        let mut tied = false;
        for (i, note) in notes.iter().enumerate() {
            let Some(pitch) = note.pitch else {
                return Err(invalid(head.len().min(i), "IMI-SUBJECT"));
            };
            match head.last_mut() {
                Some(last) if tied && last.0 == pitch => last.1 += note.duration.ticks(),
                _ => head.push((pitch, note.duration.ticks())),
            }
            tied = note.tie;
        }
        if !GIVEN_NOTES.contains(&head.len()) {
            // The last note of a short head, the first one too many of a long one.
            let note = head.len().min(*GIVEN_NOTES.end() + 1).max(1) - 1;
            return Err(invalid(note, "IMI-SUBJECT"));
        }
        // A quarter at least, and a value that fits a head note.
        if let Some(i) = head
            .iter()
            .position(|&(_, ticks)| !(2..=u32::from(u8::MAX)).contains(&ticks))
        {
            return Err(invalid(i, "IMI-SUBJECT"));
        }
        let base = head[0].0.diatonic();
        let steps: Vec<i8> = head
            .iter()
            .map(|&(p, _)| (p.diatonic() - base).clamp(-60, 60) as i8)
            .collect();
        let (mut low, mut high) = (0, 0);
        for (i, &s) in steps.iter().enumerate() {
            (low, high) = (low.min(s), high.max(s));
            if high - low > 7 {
                return Err(invalid(i, "IMI-SUBJECT"));
            }
        }
        let degree = (self.mode.degree(head[0].0) - 1) as i8;
        let first = self.reference() + i32::from(degree);
        let part = &self.parts[p];
        // The head keeps its accidentals: each note's alteration against the scale.
        let mut subject = Soggetto {
            degree,
            notes: steps
                .iter()
                .zip(&head)
                .map(|(&step, &(pitch, ticks))| SubjectNote {
                    step,
                    ticks: ticks as u8,
                    sung: Choice::Hold,
                    alter: pitch.alter - self.mode.scale_alter(pitch.step),
                })
                .collect(),
            words: part.head.clone(),
            origin: Origin::Given,
            answers: Vec::new(),
            stretto: Vec::new(),
            cost: 0,
        };
        let Some(pitches) = subject.pitches(self.mode, first, 0, Answer::Real) else {
            return Err(invalid(0, "IMI-SUBJECT"));
        };
        if let Some((rule, i)) = self.moves.first(&pitches) {
            return Err(invalid(i, rule));
        }
        // Every planned answer: on the other main degree, really or modally.
        subject.answers = self.answers(p, &subject);
        if dominant(self.mode, degree).is_some()
            && !subject
                .answers
                .iter()
                .any(|(r, _)| *r == Relation::Dominant)
        {
            let shift = shifts(self.mode, degree, Relation::Dominant)[0];
            let (rule, i) = match subject.pitches(self.mode, first, shift, Answer::Real) {
                Some(answer) => self.moves.first(&answer).unwrap_or(("MEL-TRITONE", 1)),
                None => unsung(self.mode, &subject, first, shift),
            };
            return Err(invalid(i, rule));
        }
        // The measure rhythms at offset 0.
        let mut written = Vec::new();
        let mut tick = 0;
        let mut first_written = Vec::new();
        for (&(_, ticks), &pitch) in head.iter().zip(&pitches) {
            first_written.push(written.len());
            write(
                &mut written,
                pitch,
                tick,
                ticks,
                !self.mode.is_scale_pitch(pitch),
            );
            tick += ticks;
        }
        let all = measures(&written);
        let mut note_of_measure = Vec::new();
        let mut at = 0;
        for (i, &(_, ticks)) in head.iter().enumerate() {
            while note_of_measure.len() <= ((at + ticks - 1) / MEASURE) as usize {
                note_of_measure.push(i);
            }
            at += ticks;
        }
        for (m, (tokens, tied)) in all.iter().enumerate() {
            let complete = (m as u32 + 1) * MEASURE < tick;
            let good = if complete {
                self.rhythms.level(tokens, *tied) != Level::Forbidden
            } else {
                self.rhythms.prefix(tokens)
            };
            if !good {
                return Err(invalid(note_of_measure[m], "FREE-RHYTHM"));
            }
        }
        // The underlay of the head words, as many of them as the notes can declaim.
        let from = super::points::state_before(&self.model, &self.parts, p);
        let first_word = usize::from(part.words.start);
        let word_end = self.text.word_of(first_word).range().end as u16;
        let path = (word_end..=part.head.end)
            .rev()
            .filter(|&end| end > part.words.start)
            .find_map(|end| {
                self.model
                    .best_prefix(&self.levels, from, &written, part.words.start, end - 1)
                    .map(|path| (end, path))
            });
        let Some((end, path)) = path else {
            let (rule, i) = self.refusal(from, &written, part.words.start, part.head.end - 1);
            let note = first_written.iter().rposition(|&w| w <= i).unwrap_or(0);
            return Err(invalid(note, rule));
        };
        let sung: Vec<Choice> = first_written
            .iter()
            .map(|&w| {
                path.starts
                    .iter()
                    .find(|&&(note, _)| note == w)
                    .map_or(Choice::Hold, |&(_, y)| Choice::Start(y))
            })
            .collect();
        let violations = self.check(p, &part.words, &written, Window::partial(0, tick));
        if let Some(v) = violations.iter().find(|v| v.is_forbidden()) {
            let note = first_written
                .iter()
                .rposition(|&w| w <= v.note)
                .unwrap_or(0);
            return Err(invalid(note, v.rule));
        }
        let values: Vec<u32> = head.iter().map(|&(_, t)| t).collect();
        for (note, sung) in subject.notes.iter_mut().zip(sung) {
            note.sung = sung;
        }
        subject.words = part.words.start..end;
        // A given head need not pass the stretto test; without a stretto it costs the most.
        let (stretto, stretto_cost) = stretto(self.mode, &subject);
        subject.stretto = stretto;
        subject.cost = path.cost
            + priced(&violations, true)
            + climax_cost(&pitches, &values)
            + stretto_cost.unwrap_or(NO_STRETTO);
        Ok(subject)
    }

    /// Why a head's notes cannot declaim syllables `first` to `last` from `from`: the rule that
    /// cut the most underlays at the written note where the last of them ended, with that note.
    fn refusal(
        &self,
        from: UState,
        written: &[Note],
        first: u16,
        last: u16,
    ) -> (&'static str, usize) {
        let mut states = vec![from];
        let mut tick = 0;
        let mut found = Vec::new();
        let mut options = Vec::new();
        for (i, note) in written.iter().enumerate() {
            let Some(event) = Event::at(written, i, tick) else {
                tick += note.duration.ticks();
                continue;
            };
            let mut next = Vec::new();
            let mut cut: Vec<(&'static str, u32)> = Vec::new();
            for state in &states {
                self.model.options(state, &event, &mut options);
                if i == 0 {
                    options.retain(|&c| c == Choice::Start(first));
                }
                options.retain(|&c| !matches!(c, Choice::Start(y) if y > last));
                for &choice in &options {
                    found.clear();
                    let t = self.model.step(state, &event, choice, &mut found);
                    match self.levels.price(&found) {
                        (_, Some(rule)) => match cut.iter_mut().find(|(r, _)| *r == rule) {
                            Some((_, n)) => *n += 1,
                            None => cut.push((rule, 1)),
                        },
                        (_, None) => {
                            let canonical = self.model.canonical(&t, &self.levels);
                            if !next.contains(&canonical) {
                                next.push(canonical);
                            }
                        }
                    }
                }
            }
            if next.is_empty() {
                let rule = cut
                    .iter()
                    .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(a.0)))
                    .map_or(Case::Order.rule(), |(r, _)| r);
                return (rule, i);
            }
            states = next;
            tick += note.duration.ticks();
        }
        (Case::Complete.rule(), written.len().saturating_sub(1))
    }
}

/// The values that may follow a value in a head (SOG-ACCEL), each with its style cost and whether
/// it drops to a quarter of the one before or less (IMI-SUBJECT `drop`).
fn transitions(from: u8) -> &'static [(u8, u32, bool)] {
    match from {
        16 => &[
            (16, 1, false),
            (12, 0, false),
            (8, 0, false),
            (4, 0, true),
            (2, 0, true),
        ],
        12 => &[
            (4, 0, false),
            (2, 0, false),
            (8, 0, false),
            (12, 1, false),
            (16, 1, false),
        ],
        8 => &[
            (8, 0, false),
            (4, 0, false),
            (12, 0, false),
            (16, 0, false),
            (6, 0, false),
            (2, 0, true),
        ],
        6 => &[(2, 0, false)],
        4 => &[
            (4, 0, false),
            (8, 0, false),
            (12, 0, false),
            (6, 0, false),
            (2, 0, false),
        ],
        _ => &[(2, 0, false), (4, 0, false), (8, 0, false)],
    }
}

/// The cost of a head's first black figure (SOG-MELISMA): none at all, or only after more than
/// seven white notes, and the dotted half with its quarter free; a run from the minim beat one,
/// from the downbeat two. 44 % of Palestrina's subjects have no black note (SOG-DISSOLVE), so a
/// white head pays nothing; the motion through the quarter is priced with the pitches
/// ([`quarter_cost`]).
fn figure_cost(values: &[u8]) -> u32 {
    let Some(j) = values.iter().position(|&v| v < 4) else {
        return 0;
    };
    if j > 7 {
        return 0;
    }
    let start: u32 = values[..j].iter().map(|&v| u32::from(v)).sum();
    if j > 0 && values[j - 1] == 6 && values[j] == 2 {
        0
    } else if start % MEASURE == 4 {
        1
    } else {
        2
    }
}

/// One for a highest note that sounds more than once or is black.
fn climax_cost(pitches: &[Pitch], values: &[u32]) -> u32 {
    let key = |p: &Pitch| (p.diatonic(), p.midi());
    let Some(top) = pitches.iter().map(key).max() else {
        return 0;
    };
    let tops: Vec<usize> = (0..pitches.len())
        .filter(|&i| key(&pitches[i]) == top)
        .collect();
    u32::from(tops.len() > 1 || tops.iter().any(|&i| values.get(i).is_some_and(|&v| v < 4)))
}

/// Whether a violation of a head's one-voice score counts: while the head is not `complete`,
/// IMI-SUBJECT `end` judges its last note so far, which a later note will follow; the check of
/// the whole head counts it.
fn counted(violation: &Violation, complete: bool) -> bool {
    complete || !(violation.rule == "IMI-SUBJECT" && violation.detail == Detail::Param("end"))
}

/// The cost of the violations that count.
fn priced(violations: &[Violation], complete: bool) -> u32 {
    violations
        .iter()
        .filter(|v| counted(v, complete))
        .map(|v| v.level.cost())
        .sum()
}

/// Whether a violation that counts is forbidden.
fn forbidden(violations: &[Violation], complete: bool) -> bool {
    violations
        .iter()
        .any(|v| counted(v, complete) && v.is_forbidden())
}

/// The tones of a line: `pitches` with `values`, the first at `entry` and `cut` ticks shorter,
/// the others after it.
fn tones(pitches: &[Pitch], values: &[u32], entry: u32, cut: u32) -> Vec<Tone> {
    let mut out = Vec::with_capacity(pitches.len());
    let mut tick = entry;
    for (i, (&pitch, &value)) in pitches.iter().zip(values).enumerate() {
        let length = if i == 0 { value - cut } else { value };
        out.push(Tone {
            pitch,
            start: tick,
            end: tick + length,
        });
        tick += length;
    }
    out
}

/// Maps `f` over `items`, on all cores with the parallel search, in their order.
fn each<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync + Send) -> Vec<R> {
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        items.par_iter().map(f).collect()
    }
    #[cfg(not(feature = "parallel"))]
    {
        items.iter().map(f).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imitation::points::points_of;
    use crate::imitation::test_support::{choir, text};
    use crate::text::underlay::Automaton;

    fn builder(spec: &str, mode: &str, voices: usize, imitation: &Imitation, seed: u64) -> Builder {
        builder_in(spec, mode.parse().unwrap(), voices, imitation, seed)
    }

    /// A subject as its leader sings it in the tenor: pitch, value and the syllable begun.
    fn describe(builder: &Builder, subject: &Soggetto) -> String {
        let first = builder.reference() + i32::from(subject.degree);
        let pitches = subject
            .pitches(builder.mode, first, 0, Answer::Real)
            .unwrap();
        let notes: Vec<String> = pitches
            .iter()
            .zip(&subject.notes)
            .map(|(p, n)| match n.sung {
                Choice::Start(y) => format!(
                    "{p}.{}={}",
                    n.ticks,
                    builder.text.syllables[usize::from(y)].text
                ),
                Choice::Hold => format!("{p}.{}", n.ticks),
            })
            .collect();
        format!(
            "{} (cost {}, {} answers, {} strettos)",
            notes.join(" "),
            subject.cost,
            subject.answers.len(),
            subject.stretto.len()
        )
    }

    /// A builder for the first points of `spec` in `mode`.
    fn builder_in(
        spec: &str,
        mode: ModeSpec,
        voices: usize,
        imitation: &Imitation,
        seed: u64,
    ) -> Builder {
        let text = text(spec);
        let plan = crate::text::plan::plan(&text, mode, voices, None, None, true).unwrap();
        let parts =
            points_of(&text, plan.statements, true, imitation, &mut Rng::new(seed)).unwrap();
        Builder::new(
            text,
            mode,
            &choir(voices),
            &Profile::fux(),
            imitation,
            parts,
            seed,
        )
        .unwrap()
    }

    /// Twelve mode variants: the six modes untransposed and transposed by a fourth, authentic on
    /// even seeds and plagal on odd ones. The builder sings every head in the tenor's octave of
    /// the final and leaves the ambitus to the entries, so the plagal mode gives the authentic
    /// one's subjects; the transposition changes them.
    fn modes(seed: u64) -> Vec<ModeSpec> {
        use crate::mode::Ambitus;
        let ambitus = if seed.is_multiple_of(2) {
            Ambitus::Authentic
        } else {
            Ambitus::Plagal
        };
        Mode::ALL
            .into_iter()
            .flat_map(|mode| {
                [false, true].map(|transposed| {
                    ModeSpec::new(mode)
                        .with_ambitus(ambitus)
                        .with_transposition(transposed)
                })
            })
            .collect()
    }

    /// The head of a subject as its leader sings it in the tenor from tick 0, written in
    /// measures, with the choices pinned at its attacks.
    fn head(builder: &Builder, subject: &Soggetto) -> (Vec<Note>, Vec<(u32, Choice)>) {
        let first = builder.reference() + i32::from(subject.degree);
        let pitches = subject
            .pitches(builder.mode, first, 0, Answer::Real)
            .unwrap();
        let mut written = Vec::new();
        let mut pins = Vec::new();
        let mut tick = 0;
        for ((&pitch, note), value) in pitches.iter().zip(&subject.notes).zip(subject.values()) {
            pins.push((tick, note.sung));
            write(
                &mut written,
                pitch,
                tick,
                value,
                !builder.mode.is_scale_pitch(pitch),
            );
            tick += value;
        }
        (written, pins)
    }

    /// The melodic prohibitions the checker finds in a line sung in whole notes by a tenor.
    fn melodic_prohibitions(checker: &Checker, mode: ModeSpec, pitches: &[Pitch]) -> Vec<String> {
        let notes = pitches
            .iter()
            .map(|&p| Note::new(p, Duration::WHOLE).with_ficta(!mode.is_scale_pitch(p)))
            .collect();
        let voice = Voice::new(VoiceType::Tenor, Role::Counterpoint(Species::Free), notes);
        let score = Score::new(mode, vec![voice]);
        checker
            .check(&score)
            .violations
            .iter()
            .filter(|v| v.is_forbidden() && v.rule.starts_with("MEL-"))
            .map(|v| format!("{} at {}", v.rule, v.tick))
            .collect()
    }

    /// The subjects of the first point of each text in each mode follow the builder's rules: a
    /// first note of a whole or longer at the downbeat, four notes or more within an octave,
    /// syllables the underlay automaton accepts as pins, answers without a melodic prohibition
    /// at every shift, and a stretto that passes the pair checks.
    fn follow(seeds: impl Iterator<Item = u64>) -> usize {
        let checker = Checker::new(&Profile::fux());
        let texts = [
            "kyrie",
            "christe",
            "agnus",
            "sanctus",
            "benedictus",
            "Ad te le-vá-vi á-ni-mam me-am.",
        ];
        let mut built = 0;
        for seed in seeds {
            let voices = 3 + (seed % 3) as usize;
            for mode in modes(seed) {
                for spec in texts {
                    let label = format!("{spec} {} {voices} voices seed {seed}", mode.id());
                    let builder = builder_in(spec, mode, voices, &Imitation::default(), seed);
                    let part = &builder.parts[0];
                    let found = builder.subjects(0, &[]).unwrap();
                    assert!(!found.subjects.is_empty(), "{label}: no subject");
                    assert!(found.subjects.len() <= SUBJECTS, "{label}");
                    for subject in &found.subjects {
                        let shown = describe(&builder, subject);
                        let label = format!("{label}: {shown}");
                        assert!((4..=MOST_NOTES).contains(&subject.notes.len()), "{label}");
                        assert!(subject.notes[0].ticks >= 8, "{label}");
                        let steps = subject.steps();
                        let range = steps.iter().max().unwrap() - steps.iter().min().unwrap();
                        assert!(range <= 7, "{label}");
                        assert_eq!(subject.words, part.head, "{label}");
                        assert_eq!(
                            subject.notes[0].sung,
                            Choice::Start(part.words.start),
                            "{label}"
                        );
                        // The pins as the search sets them, from the voice's start.
                        let (written, pins) = head(&builder, subject);
                        let automaton = Automaton {
                            model: &builder.model,
                            levels: &builder.levels,
                            total: u32::MAX,
                            cap: None,
                            limit: None,
                            pins: Some(&pins),
                            gates: None,
                        };
                        let set = automaton.run(&written);
                        assert!(set.is_ok(), "{label}: {set:?}");
                        let last = set.unwrap()[0].0.syllable();
                        assert_eq!(last, Some(usize::from(part.head.end) - 1), "{label}");
                        // Every answer at every shift.
                        assert!(!subject.answers.is_empty(), "{label}");
                        for &(relation, answer) in &subject.answers {
                            for shift in shifts(builder.mode, subject.degree, relation) {
                                let first = builder.reference() + i32::from(subject.degree);
                                let pitches = subject.pitches(builder.mode, first, shift, answer);
                                let pitches = pitches.unwrap_or_else(|| {
                                    panic!("{label}: {relation:?} {answer:?} at {shift}")
                                });
                                let found = melodic_prohibitions(&checker, builder.mode, &pitches);
                                assert!(found.is_empty(), "{label}: {answer:?} {found:?}");
                            }
                        }
                        assert!(!subject.stretto.is_empty(), "{label}");
                    }
                    built += 1;
                }
            }
        }
        built
    }

    /// The first point's subjects of five sections and an own text in the twelve modes,
    /// for two seeds.
    #[test]
    fn subjects_follow_text_and_mode_in_brief() {
        assert_eq!(follow(1..=2), 2 * 12 * 6);
    }

    /// The same for 100 seeds.
    #[test]
    #[ignore = "slow: cargo test --release -- --ignored"]
    fn subjects_follow_text_and_mode() {
        let clock = std::time::Instant::now();
        let built = follow(1..=100);
        let seconds = clock.elapsed().as_secs_f64();
        println!(
            "{built} first points in {seconds:.1} s, {:.1} ms each",
            1000.0 * seconds / built as f64
        );
    }

    /// The seed varies the subjects: Kyrie I in four voices gives ten different heads or more
    /// over twenty seeds.
    #[test]
    fn subjects_vary_with_the_seed() {
        let dorian: ModeSpec = "dorian".parse().unwrap();
        let heads: std::collections::HashSet<Vec<SubjectNote>> = (1..=20)
            .map(|seed| {
                let builder = builder_in("kyrie", dorian, 4, &Imitation::default(), seed);
                builder.subjects(0, &[]).unwrap().subjects[0].notes.clone()
            })
            .collect();
        assert!(heads.len() >= 10, "{} heads", heads.len());
        // The same seed, the same subjects.
        let again = |seed| {
            builder_in("kyrie", dorian, 4, &Imitation::default(), seed)
                .subjects(0, &[])
                .unwrap()
        };
        assert_eq!(again(3), again(3));
    }

    /// Sections sung one after the other as one text (phase 14) open as the first of them, as
    /// they take its pace (`text::plan`): the first point of `sanctus+pleni+osanna` has the
    /// heads and the first intervals of the Sanctus.
    #[test]
    fn joined_texts_open_as_their_first() {
        let imitation = Imitation::default();
        let joined = builder("sanctus+pleni+osanna", "dorian", 4, &imitation, 1);
        let alone = builder("sanctus", "dorian", 4, &imitation, 1);
        assert_eq!(joined.section(), "sanctus");
        let opening = |builder: &Builder| {
            let ctx = builder.ctx(0, None, &mut Rng::new(1));
            format!("{:?} {:?}", ctx.template, ctx.intervals)
        };
        assert_eq!(opening(&joined), opening(&alone));
        assert!(opening(&joined).starts_with("Heads("));
    }

    /// A later point's subject does not begin like an earlier one's (SOG-VARY), unless it
    /// restates it.
    #[test]
    fn later_points_vary_their_subjects() {
        let dorian: ModeSpec = "dorian".parse().unwrap();
        let builder = builder_in("agnus", dorian, 4, &Imitation::default(), 1);
        let first: Vec<Arc<Soggetto>> = builder
            .subjects(0, &[])
            .unwrap()
            .subjects
            .into_iter()
            .map(Arc::new)
            .collect();
        let earlier = vec![Arc::clone(&first[0])];
        let later = builder.subjects(1, &earlier).unwrap().subjects;
        assert!(!later.is_empty());
        assert!(later.iter().all(|s| !s.begins_like(&first[0])));
        // A restatement sings the subject of its point.
        let imitation = Imitation {
            restate: super::super::Toggle::On,
            ..Imitation::default()
        };
        let kyrie = builder_in("kyrie", dorian, 4, &imitation, 1);
        let subject = Arc::new(kyrie.subjects(0, &[]).unwrap().subjects.remove(0));
        let restated = kyrie.subjects(1, &[Arc::clone(&subject)]).unwrap();
        assert_eq!(restated.subjects, [(*subject).clone()]);
    }

    /// The first invocation of Kyrie IX, D F G A G F E D, becomes G B♭ C D C B♭ A G in the
    /// transposed Dorian of Palestrina's Missa de beata Virgine: every candidate sings the
    /// chant from its first note without leaving a note out, the best those eight notes.
    #[test]
    fn kyrie_ix_gives_palestrinas_skeleton() {
        use crate::imitation::chant::Kyriale;
        let imitation = Imitation {
            subject: SubjectSource::Chant {
                chant: Kyriale::KyrieIX.chant(),
                phrases: None,
            },
            ..Imitation::default()
        };
        let mode: ModeSpec = "dorian-transposed".parse().unwrap();
        let names = |builder: &Builder, subject: &Soggetto| -> String {
            let first = builder.reference() + i32::from(subject.degree);
            let pitches = subject
                .pitches(builder.mode, first, 0, Answer::Real)
                .unwrap();
            let names: Vec<String> = pitches
                .iter()
                .map(|p| p.to_string().trim_end_matches(char::is_numeric).to_string())
                .collect();
            names.join(" ")
        };
        for (seed, voices) in (1..=10).flat_map(|seed| [(seed, 3), (seed, 4), (seed, 5)]) {
            let builder = builder_in("kyrie", mode, voices, &imitation, seed);
            let found = builder.subjects(0, &[]).unwrap();
            assert_eq!(found.fallback, None);
            let best = &found.subjects[0];
            let label = format!("seed {seed}, {voices} voices");
            assert_eq!(names(&builder, best), "G Bb C D C Bb A G", "{label}");
            assert_eq!(best.origin, Origin::Chant { phrase: 0 });
            assert_eq!(best.degree, 0);
            for subject in &found.subjects {
                let sung = names(&builder, subject);
                assert!("G Bb C D C Bb A G F G".starts_with(&sung), "{sung}");
                assert_eq!(subject.words, 0..4);
            }
        }
        // With the last invocation named for Kyrie I, its tail has no phrase left and takes a
        // subject from its words, with a warning.
        let imitation = Imitation {
            subject: SubjectSource::Chant {
                chant: Kyriale::KyrieIX.chant(),
                phrases: Some(vec![8]),
            },
            restate: super::super::Toggle::Off,
            ..Imitation::default()
        };
        let builder = builder_in("kyrie", mode, 4, &imitation, 1);
        let first = builder.subjects(0, &[]).unwrap();
        assert_eq!(first.fallback, None);
        let earlier = vec![Arc::new(first.subjects[0].clone())];
        let tail = builder.subjects(1, &earlier).unwrap();
        assert_eq!(tail.fallback, Some(ChantFallback { point: 2 }));
        assert!(tail.subjects.iter().all(|s| s.origin == Origin::Text));
    }

    /// The head motif of a mass (phase 14, MASS-MOTIF) opens the first point only: its heads
    /// keep the motif's pitches, its B♭ and its B alike, in order, every tone of it, none
    /// passing between, at most two of them struck again, each for a syllable of its own; they
    /// are answered and pass the stretto test as any head. The later points take their subjects
    /// from their words, and a motif that no head can sing (a tritone) gives the first point
    /// subjects from its words, with a warning.
    #[test]
    fn a_head_motif_keeps_its_pitches() {
        use crate::imitation::Motif;
        let mode: ModeSpec = "dorian".parse().unwrap();
        let pitch_names = |builder: &Builder, subject: &Soggetto| -> Vec<Pitch> {
            let first = builder.reference() + i32::from(subject.degree);
            let pitches = subject
                .pitches(builder.mode, first, 0, Answer::Real)
                .unwrap();
            let mut merged: Vec<Pitch> = Vec::new();
            for pitch in pitches {
                if merged.last() != Some(&pitch) {
                    merged.push(pitch);
                }
            }
            merged
        };
        for (motif, spec) in [
            ("4,2,3,4,5b,4", "gloria:1-5"),
            ("0,2,1,4,3,2,0,1", "credo:1-4"),
            ("4,5,4,3,2", "agnus"),
            ("4,5,4,2,3,4", "sanctus+pleni+osanna"),
        ] {
            let motif: Motif = motif.parse().unwrap();
            let imitation = Imitation {
                subject: SubjectSource::Motif(Arc::new(motif.clone())),
                ..Imitation::default()
            };
            for seed in 1..=3 {
                let label = format!("{motif} on {spec}, seed {seed}");
                let builder = builder_in(spec, mode, 4, &imitation, seed);
                let found = builder.subjects(0, &[]).unwrap();
                assert_eq!(found.fallback, None, "{label}");
                assert!(!found.subjects.is_empty(), "{label}");
                for subject in &found.subjects {
                    assert_eq!(subject.origin, Origin::Motif, "{label}");
                    let sung = pitch_names(&builder, subject);
                    assert_eq!(sung, motif.pitches(mode, sung[0].octave), "{label}");
                    let notes = &subject.notes;
                    assert!(notes.len() <= motif.tones.len() + MOTIF_REPEATS, "{label}");
                    for pair in notes.windows(2) {
                        if pair[0].step == pair[1].step {
                            assert!(matches!(pair[1].sung, Choice::Start(_)), "{label}");
                        }
                    }
                    assert!(!subject.answers.is_empty() && !subject.stretto.is_empty());
                }
                let earlier = vec![Arc::new(found.subjects[0].clone())];
                if builder.parts.len() > 1 {
                    let later = builder.subjects(1, &earlier).unwrap();
                    assert_eq!(later.fallback, None, "{label}");
                    assert!(later.subjects.iter().all(|s| s.origin == Origin::Text));
                }
            }
        }
        // F to B: no head sings a tritone.
        let tritone: Motif = "0,2,5,4,3".parse().unwrap();
        let imitation = Imitation {
            subject: SubjectSource::Motif(Arc::new(tritone)),
            ..Imitation::default()
        };
        let builder = builder_in("gloria:1-5", mode, 4, &imitation, 1);
        let found = builder.subjects(0, &[]).unwrap();
        assert_eq!(found.fallback, Some(ChantFallback { point: 1 }));
        assert!(found.subjects.iter().all(|s| s.origin == Origin::Text));
    }

    /// Given subjects: one that can be imitated takes its syllables from the cheapest
    /// underlay of the head words; any other fails, naming the note and the rule, in German and
    /// English.
    #[test]
    fn given_subjects_are_checked() {
        let given = |src: &str| -> Result<Soggetto, GenerateError> {
            let notes = crate::parse::parse_lilypond(src, Duration::HALF).unwrap();
            let imitation = Imitation {
                subject: SubjectSource::Given(vec![notes]),
                ..Imitation::default()
            };
            let builder = builder("kyrie", "dorian", 4, &imitation, 1);
            builder
                .subjects(0, &[])
                .map(|found| found.subjects[0].clone())
        };
        let subject = given("d'1 d'2 a'2 g'4 f'4 e'2 d'1").unwrap();
        assert_eq!(subject.origin, Origin::Given);
        assert_eq!(subject.degree, 0);
        assert_eq!(subject.steps(), [0, 0, 4, 3, 2, 1, 0]);
        assert_eq!(subject.values(), [8, 4, 4, 2, 2, 4, 8]);
        assert_eq!(subject.words, 0..4);
        let starts: Vec<u16> = subject
            .notes
            .iter()
            .filter_map(|n| match n.sung {
                Choice::Start(y) => Some(y),
                Choice::Hold => None,
            })
            .collect();
        assert_eq!(starts, [0, 1, 2, 3]);
        assert!(
            subject
                .answers
                .iter()
                .any(|(r, _)| *r == Relation::Dominant)
        );
        // A tie makes one note of the head.
        let tied = given("d'1~ d'2 e'2 f'1 g'1 f'1").unwrap();
        assert_eq!(tied.values(), [12, 4, 8, 8, 8]);
        // A head may begin with a half, which a follower on the minim beat cannot shorten.
        let short = given("d'2 e'2 f'1 g'1 a'1").unwrap();
        assert!(short.stretto.iter().all(|s| s.2 != FirstNote::Shortened));
        // A head keeps its accidentals: the leader and the entries at the unison and the octave
        // sing them as given, the entries on the other main degree transpose them by their
        // fifth or fourth, B♭ above A to F above E, C♯ below D to G♯ below A.
        let dorian: ModeSpec = "dorian".parse().unwrap();
        let sung = |subject: &Soggetto, shift: i8| -> String {
            let first = reference(dorian) + i32::from(subject.degree);
            let pitches = subject.pitches(dorian, first, shift, Answer::Real).unwrap();
            let names: Vec<String> = pitches.iter().map(|p| p.to_string()).collect();
            names.join(" ")
        };
        let flat = given("d'1 e'2 f'2 g'1 a'1 bes'1 a'1").unwrap();
        assert_eq!(flat.notes[5].alter, -1);
        assert_eq!(sung(&flat, 0), "D3 E3 F3 G3 A3 Bb3 A3");
        assert_eq!(sung(&flat, 7), "D4 E4 F4 G4 A4 Bb4 A4");
        assert_eq!(sung(&flat, 4), "A3 B3 C4 D4 E4 F4 E4");
        assert_ne!(flat, given("d'1 e'2 f'2 g'1 a'1 b'1 a'1").unwrap());
        let sharp = given("d'1 cis'2 d'2 e'1 f'1 e'1 d'1").unwrap();
        assert_eq!(sharp.notes[1].alter, 1);
        assert_eq!(sung(&sharp, 0), "D3 C#3 D3 E3 F3 E3 D3");
        assert_eq!(sung(&sharp, -3), "A2 G#2 A2 B2 C3 B2 A2");
        assert!(sharp.answers.contains(&(Relation::Dominant, Answer::Real)));
        assert_ne!(sharp, given("d'1 c'2 d'2 e'1 f'1 e'1 d'1").unwrap());
        // In the transposed Dorian B♭ belongs to the scale, and E♭ is its flattened mi.
        let imitation = Imitation {
            subject: SubjectSource::Given(vec![
                crate::parse::parse_lilypond("g'1 a'2 bes'2 c''1 d''1 ees''1 d''1", Duration::HALF)
                    .unwrap(),
            ]),
            ..Imitation::default()
        };
        let transposed = builder("kyrie", "dorian-transposed", 4, &imitation, 1);
        let found = transposed.subjects(0, &[]).unwrap().subjects.remove(0);
        let alters: Vec<i8> = found.notes.iter().map(|n| n.alter).collect();
        assert_eq!(alters, [0, 0, 0, 0, 0, -1, 0]);
        // The points after the given ones take subjects from the text.
        let imitation = Imitation {
            subject: SubjectSource::Given(vec![
                crate::parse::parse_lilypond("d'1 d'2 a'2 g'4 f'4 e'2 d'1", Duration::HALF)
                    .unwrap(),
            ]),
            ..Imitation::default()
        };
        let builder = builder("kyrie", "dorian", 4, &imitation, 1);
        let earlier = vec![Arc::new(subject)];
        let second = builder.subjects(1, &earlier).unwrap();
        assert!(second.subjects.iter().all(|s| s.origin == Origin::Text));
        for (src, note, rule) in [
            ("d'1 e'1 f'1", 3, "IMI-SUBJECT"),
            ("d'1 e'8 f'8 g'2 a'1 g'1", 2, "IMI-SUBJECT"),
            ("d'1 r2 e'2 f'1 g'1", 2, "IMI-SUBJECT"),
            ("d'1 e'1 f'1 g'1 a'1 b'1 c''1 d''1 e''1", 9, "IMI-SUBJECT"),
            ("e'1 b'1 f'1 e'1", 3, "MEL-TRITONE"),
            ("f'1 b'1 a'1 g'1 f'1", 2, "MEL-TRITONE"),
            ("d'1 e'1 f'1 g'1 aes'1 g'1 f'1", 5, "FIC-SCOPE"),
            ("d'1 b'1 a'1 g'1 f'1", 2, "MEL-MAX-LEAP"),
            ("d'4 e'1 f'1 g'1", 1, "FREE-RHYTHM"),
            ("d'1 d'1 d'1 d'1 d'1 d'1", 5, "TXT-REPEATED-PITCH"),
        ] {
            assert_eq!(
                given(src),
                Err(GenerateError::SubjectInvalid {
                    subject: 1,
                    note,
                    rule
                }),
                "{src}"
            );
        }
        let error = given("d'1 e'1 f'1").unwrap_err();
        assert_eq!(
            error.message(crate::lang::Lang::De),
            "Das vorgegebene Soggetto 1 scheitert an Note 3 (IMI-SUBJECT)."
        );
        assert_eq!(
            error.message(crate::lang::Lang::En),
            "The given subject 1 fails at note 3 (IMI-SUBJECT)."
        );
    }

    /// A subject on `degree` in whole notes with `steps`, its syllables left open.
    fn whole_notes(degree: i8, steps: &[i8]) -> Soggetto {
        Soggetto {
            degree,
            notes: steps
                .iter()
                .map(|&step| SubjectNote {
                    step,
                    ticks: 8,
                    sung: Choice::Hold,
                    alter: 0,
                })
                .collect(),
            words: 0..1,
            origin: Origin::Text,
            answers: Vec::new(),
            stretto: Vec::new(),
            cost: 0,
        }
    }

    /// A modal answer that would widen the head beyond an octave is not offered: the head A D D
    /// F G A G E D on the fifth spans an octave, and exchanging its first fourth for a fifth
    /// would make D A A C D E D B A, a ninth; the real answer stays, with the default options as
    /// with modal answers asked for.
    #[test]
    fn modal_answers_stay_within_an_octave() {
        let mode: ModeSpec = "hypoaeolian-transposed".parse().unwrap();
        let steps = [0, 3, 3, 5, 6, 7, 6, 4, 3];
        let subject = whole_notes(4, &steps);
        assert!(joins_main_degrees(mode, 4, &steps, 0));
        let first = reference(mode) + 4;
        let shift = shifts(mode, 4, Relation::Dominant)[0];
        let modal = subject
            .pitches(mode, first, shift, Answer::Modal { at: 0 })
            .unwrap();
        assert_eq!(span(&modal), 8);
        for choice in [AnswerChoice::Auto, AnswerChoice::Modal] {
            let imitation = Imitation {
                answer: choice,
                ..Imitation::default()
            };
            let builder = builder_in("agnus", mode, 3, &imitation, 1);
            assert_eq!(
                builder.answers(0, &subject),
                [
                    (Relation::Same, Answer::Real),
                    (Relation::Dominant, Answer::Real)
                ],
                "{choice:?}"
            );
        }
    }

    /// The mode's own flattened mi in a given head (B♭, transposed E♭) belongs to the scale's
    /// ficta, not to the head: the leader and the entries at the unison and the octave sing it
    /// as given, the entries on the other main degree as the scale and its flattened mi have it
    /// there. So fa super la on the fifth, A B♭ A in the Dorian mode, is answered on the final D
    /// E D, and C D C B♭ in the Lydian by F G F E; on the final, B♭ above A answers as F above E.
    /// Each head is a valid subject with its answer on the other main degree, where moving its
    /// flattened mi with the entry would give a flat the mode does not have (FIC-SCOPE).
    #[test]
    fn given_heads_keep_the_flattened_mi_of_their_mode() {
        for (mode, head, leader, answer) in [
            (
                "dorian",
                "a'1 bes'2 a'2 g'2 f'2 e'2 d'1",
                "A3 Bb3 A3 G3 F3 E3 D3",
                "D4 E4 D4 C4 B3 A3 G3",
            ),
            (
                "dorian",
                "d'1 e'2 f'2 g'2 a'2 bes'2 a'1",
                "D3 E3 F3 G3 A3 Bb3 A3",
                "A3 B3 C4 D4 E4 F4 E4",
            ),
            (
                "dorian-transposed",
                "d''1 ees''2 d''2 c''2 bes'2 a'2 g'1",
                "D4 Eb4 D4 C4 Bb3 A3 G3",
                "G4 A4 G4 F4 E4 D4 C4",
            ),
            (
                "lydian",
                "c''1 d''2 c''2 bes'2 a'2 g'1 f'1",
                "C4 D4 C4 Bb3 A3 G3 F3",
                "F4 G4 F4 E4 D4 C4 B3",
            ),
            (
                "lydian",
                "f'1 g'2 a'2 bes'2 a'2 g'1 f'1",
                "F3 G3 A3 Bb3 A3 G3 F3",
                "C4 D4 E4 F4 E4 D4 C4",
            ),
        ] {
            let notes = crate::parse::parse_lilypond(head, Duration::HALF).unwrap();
            let imitation = Imitation {
                subject: SubjectSource::Given(vec![notes]),
                ..Imitation::default()
            };
            let builder = builder("kyrie", mode, 4, &imitation, 1);
            let subject = builder
                .subjects(0, &[])
                .unwrap_or_else(|e| panic!("{mode} {head}: {e}"))
                .subjects
                .remove(0);
            let mode: ModeSpec = mode.parse().unwrap();
            let first = builder.reference() + i32::from(subject.degree);
            let sung = |shift: i8| -> String {
                let pitches = subject.pitches(mode, first, shift, Answer::Real).unwrap();
                let names: Vec<String> = pitches.iter().map(|p| p.to_string()).collect();
                names.join(" ")
            };
            assert_eq!(sung(0), leader, "{head}");
            let octave: Vec<String> = leader
                .split(' ')
                .map(|p| {
                    let p: Pitch = p.parse().unwrap();
                    p.transpose(Interval::new(7, 12)).to_string()
                })
                .collect();
            assert_eq!(sung(7), octave.join(" "), "{head}");
            let shift = shifts(mode, subject.degree, Relation::Dominant)[0];
            assert_eq!(sung(shift), answer, "{head}");
            assert!(
                subject
                    .answers
                    .contains(&(Relation::Dominant, Answer::Real)),
                "{head}: {:?}",
                subject.answers
            );
        }
    }

    /// The real answer keeps the steps on the scale with the flattened mi; the modal answer
    /// exchanges the fifth and the fourth where the head leaps between the final and the fifth
    /// (Agnus_01 of the corpus: G D B C B A answered by D G E F E D).
    #[test]
    fn answers_are_real_or_modal() {
        let dorian: ModeSpec = "dorian".parse().unwrap();
        let transposed: ModeSpec = "dorian-transposed".parse().unwrap();
        let names = |pitches: Vec<Pitch>| -> String {
            let names: Vec<String> = pitches.iter().map(|p| p.to_string()).collect();
            names.join(" ")
        };
        // F B becomes F B♭, B F B♭ F, in transposed Dorian B♭ E becomes B♭ E♭.
        let f = Pitch::from_diatonic(3 * 7 + 3).diatonic();
        assert_eq!(names(realize(dorian, f, &[0, 3, 2]).unwrap()), "F3 Bb3 A3");
        assert_eq!(names(realize(dorian, f + 3, &[0, -3]).unwrap()), "Bb3 F3");
        assert_eq!(
            names(realize(transposed, f + 3, &[0, 3]).unwrap()),
            "Bb3 Eb4"
        );
        // E B F: no flattened mi mends both fifths.
        assert_eq!(realize(dorian, f - 1, &[0, 4, 8]), None);
        // Una nota super la: A B A becomes A B♭ A in the Dorian and Lydian modes, transposed
        // D E D becomes D E♭ D; the Aeolian keeps the natural, and so does a passing B.
        let a = f + 2;
        assert_eq!(
            names(realize(dorian, a, &[0, 1, 0, -1]).unwrap()),
            "A3 Bb3 A3 G3"
        );
        assert_eq!(names(realize(dorian, a, &[0, 1, 2]).unwrap()), "A3 B3 C4");
        let lydian: ModeSpec = "lydian".parse().unwrap();
        assert_eq!(names(realize(lydian, a, &[0, 1, 0]).unwrap()), "A3 Bb3 A3");
        let aeolian: ModeSpec = "aeolian".parse().unwrap();
        assert_eq!(names(realize(aeolian, a, &[0, 1, 0]).unwrap()), "A3 B3 A3");
        assert_eq!(
            names(realize(transposed, a + 3, &[0, 1, 0]).unwrap()),
            "D4 Eb4 D4"
        );
        // A leader on the final G leaping to the fifth D, in transposed Dorian.
        let steps = [0, 4, 2, 3, 2, 1];
        assert!(joins_main_degrees(transposed, 0, &steps, 0));
        assert!(!joins_main_degrees(transposed, 0, &steps, 1));
        assert_eq!(
            answer_steps(&steps, Answer::Modal { at: 0 }),
            [0, 3, 1, 2, 1, 0]
        );
        let g = transposed.final_pitch(3).diatonic();
        let modal = realize(
            transposed,
            g + 4,
            &answer_steps(&steps, Answer::Modal { at: 0 }),
        );
        assert_eq!(names(modal.unwrap()), "D4 G4 E4 F4 E4 D4");
        let imitation = Imitation::default();
        let builder = builder_in("kyrie", transposed, 4, &imitation, 1);
        assert_eq!(
            builder.answers(0, &whole_notes(0, &steps)),
            [
                (Relation::Same, Answer::Real),
                (Relation::Dominant, Answer::Real),
                (Relation::Dominant, Answer::Modal { at: 0 })
            ]
        );
        let real = Imitation {
            answer: AnswerChoice::Real,
            ..Imitation::default()
        };
        let builder = builder_in("kyrie", transposed, 4, &real, 1);
        assert_eq!(builder.answers(0, &whole_notes(0, &steps)).len(), 2);
        let modal = Imitation {
            answer: AnswerChoice::Modal,
            ..Imitation::default()
        };
        let builder = builder_in("kyrie", transposed, 4, &modal, 1);
        assert_eq!(
            builder.answers(0, &whole_notes(0, &steps))[1],
            (Relation::Dominant, Answer::Modal { at: 0 })
        );
        // The other main degree: fifth and final, final and fourth; Phrygian E and A.
        assert_eq!(dominant(dorian, 0), Some(4));
        assert_eq!(dominant(dorian, 4), Some(0));
        assert_eq!(dominant(dorian, 3), Some(0));
        assert_eq!(dominant(dorian, 2), None);
        let phrygian: ModeSpec = "phrygian".parse().unwrap();
        assert_eq!(dominant(phrygian, 0), Some(3));
        assert_eq!(dominant(phrygian, 3), Some(0));
        assert_eq!(shifts(dorian, 0, Relation::Dominant), [4, -3]);
        assert_eq!(shifts(dorian, 4, Relation::Dominant), [3, -4]);
        assert_eq!(shifts(dorian, 0, Relation::Same), [0, 7, -7]);
    }

    /// The stretto test runs the pair checks on the leader and one follower. The head D D F D F
    /// in wholes passes at the unison and the octave a whole later and at the fifth three
    /// wholes later; a whole later at the fourth below it strikes a fourth on the downbeat, at
    /// the fifth a seventh, two wholes later at the fifth it moves in parallel fifths.
    #[test]
    fn stretto_tests_two_lines() {
        let dorian: ModeSpec = "dorian".parse().unwrap();
        let builder = builder_in("kyrie", dorian, 4, &Imitation::default(), 1);
        let note = |step: i8, y: Option<u16>| SubjectNote {
            step,
            ticks: 8,
            sung: y.map_or(Choice::Hold, Choice::Start),
            alter: 0,
        };
        let mut subject = Soggetto {
            degree: 0,
            notes: vec![
                note(0, Some(0)),
                note(0, Some(1)),
                note(2, Some(2)),
                note(0, Some(3)),
                note(2, None),
            ],
            words: 0..4,
            origin: Origin::Given,
            answers: Vec::new(),
            stretto: Vec::new(),
            cost: 0,
        };
        assert!(builder.answer_and_test(0, &mut subject));
        for passes in [(7, 8), (0, 8), (4, 24)] {
            assert!(
                subject
                    .stretto
                    .contains(&(passes.0, passes.1, FirstNote::AsLeader)),
                "{passes:?}"
            );
        }
        for fails in [(-3, 8), (4, 8), (4, 16)] {
            assert!(
                !subject
                    .stretto
                    .contains(&(fails.0, fails.1, FirstNote::AsLeader)),
                "{fails:?}"
            );
        }
        assert!(subject.cost <= 4);
    }

    /// Every point of every section of the Ordinary, in three modes and for three seeds, gets
    /// subjects from its words, each after the subjects of the points before it.
    #[test]
    #[ignore = "slow: cargo test --release -- --ignored --nocapture"]
    fn every_point_of_the_ordinary_has_subjects() {
        use crate::text::{TextOptions, ordinary};
        let mut empty = Vec::new();
        let mut points = 0;
        let clock = std::time::Instant::now();
        for section in ordinary::sections(&TextOptions::default()) {
            for mode in ["dorian", "phrygian", "mixolydian-transposed"] {
                for seed in 1..=3 {
                    let mode: ModeSpec = mode.parse().unwrap();
                    let builder = builder_in(&section.id, mode, 4, &Imitation::default(), seed);
                    let mut earlier = Vec::new();
                    for p in 0..builder.parts.len() {
                        points += 1;
                        let found = builder.subjects(p, &earlier).unwrap();
                        match found.subjects.first() {
                            Some(best) => earlier.push(Arc::new(best.clone())),
                            None => {
                                let words = &builder.parts[p].words;
                                empty.push(format!(
                                    "{} {} seed {seed} point {p}: {}",
                                    section.id,
                                    mode.id(),
                                    crate::imitation::points::spelled(&builder.text, words)
                                ));
                                break;
                            }
                        }
                    }
                }
            }
        }
        let seconds = clock.elapsed().as_secs_f64();
        println!(
            "{points} points in {seconds:.1} s, {:.1} ms each",
            1000.0 * seconds / points as f64
        );
        assert!(empty.is_empty(), "{empty:#?}");
    }

    /// The head template and the start degree are drawn by their shares, also for the small
    /// seeds users give: Kyrie I's breve head 16 8 8 in about a fifth of the points, the Dorian
    /// fifth in about two fifths (31 against 45).
    #[test]
    fn draws_follow_the_shares() {
        let dorian: ModeSpec = "dorian".parse().unwrap();
        let builder = builder_in("kyrie", dorian, 4, &Imitation::default(), 1);
        let (mut breves, mut fifths) = (0, 0);
        for seed in 1..=1000 {
            let mut rng = generator(sub_seed(seed, 0));
            let ctx = builder.ctx(0, None, &mut rng);
            breves += usize::from(ctx.drawn == Some(&[16, 8, 8][..]));
            fifths += usize::from(ctx.degree == Some(4));
        }
        assert!((170..250).contains(&breves), "{breves} of 1000");
        assert!((360..460).contains(&fifths), "{fifths} of 1000");
    }

    /// The figures of heads that the statistics compare with the leaders' heads of Palestrina's
    /// points, as the detector finds them in his masses (openings / later points): a leap of a
    /// fourth or more 44 / 33 %, a rising fourth or fifth in the first four intervals 24 / 18 %
    /// (by text 15–34 %), a seventh or an octave 13 / 7 %, a dotted half with its quarter 36 /
    /// 30 %, no black note 45 / 56 %, a rare motion through a quarter 1.8 / 1.6 %, an end off the
    /// minim beat 21 / 17 %; first intervals (repeat, second, third, fourth or more) 29 / 37 / 20 /
    /// 14 % and 31 / 46 / 13 / 10 %.
    #[derive(Debug, Default)]
    struct Figures {
        heads: usize,
        leap: usize,
        rising: usize,
        wide: usize,
        /// Subjects that offer a modal answer.
        modal: usize,
        dotted: usize,
        white: usize,
        rare: usize,
        off_beat: usize,
        /// A syllable begins on the quarter after a dotted half.
        texted_quarter: usize,
        /// First intervals: repeated note, second, third, fourth or more.
        first: [usize; 4],
        /// In the Dorian and Lydian modes, la–mi–la with the mi natural and flattened.
        la_mi_la: [usize; 2],
    }

    impl Figures {
        fn add(&mut self, mode: ModeSpec, subject: &Soggetto, pitches: &[Pitch]) {
            let values = subject.values();
            let n = values.len();
            let step = |i: usize| pitches[i + 1].diatonic() - pitches[i].diatonic();
            let intervals: Vec<i32> = (0..n - 1).map(step).collect();
            let lowest = pitches.iter().map(|p| p.diatonic()).min().unwrap_or(0);
            let highest = pitches.iter().map(|p| p.diatonic()).max().unwrap_or(0);
            self.heads += 1;
            self.leap += usize::from(intervals.iter().any(|i| i.abs() >= 3));
            self.rising += usize::from(intervals.iter().take(4).any(|i| (3..=4).contains(i)));
            self.wide += usize::from(highest - lowest >= 6);
            self.modal += usize::from(
                subject
                    .answers
                    .iter()
                    .any(|(_, a)| matches!(a, Answer::Modal { .. })),
            );
            self.dotted += usize::from(values.windows(2).any(|w| w == [6, 2]));
            self.white += usize::from(values.iter().all(|&v| v >= 4));
            self.rare += usize::from((1..n - 1).any(|i| {
                let figure = (intervals[i - 1] as i8, intervals[i] as i8);
                values[i - 1] >= 4
                    && values[i] == 2
                    && !QUARTER_FIGURES.iter().any(|(f, _)| *f == figure)
            }));
            self.off_beat += usize::from(values.iter().sum::<u32>() % 4 != 0);
            self.texted_quarter += usize::from((1..n).any(|i| {
                values[i] == 2
                    && values[i - 1] == 6
                    && matches!(subject.notes[i].sung, Choice::Start(_))
            }));
            if let Some(&first) = intervals.first() {
                self.first[first.unsigned_abs().min(3) as usize] += 1;
            }
            if matches!(mode.mode, Mode::Dorian | Mode::Lydian) {
                for w in pitches.windows(3) {
                    let la = w[0] == w[2] && w[0].step.index() + 1 == mode.flattened_mi().index();
                    if la && w[1].diatonic() == w[0].diatonic() + 1 {
                        self.la_mi_la[usize::from(!mode.is_scale_pitch(w[1]))] += 1;
                    }
                }
            }
        }

        fn show(&self, name: &str) {
            let share = |n: usize| format!("{:.1} %", 100.0 * n as f64 / self.heads.max(1) as f64);
            let first: Vec<String> = self.first.iter().map(|&n| share(n)).collect();
            println!(
                "{name} ({} heads): leap of a fourth or more {}, rising fourth or fifth in \
                 intervals 1-4 {}, seventh or octave {}, offering a modal answer {} ({}), \
                 dotted half and quarter {}, no black note {}, rare quarter motion {}, end off \
                 the minim beat {}, syllable on the quarter after a dotted half {}; first \
                 interval repeat / second / third / fourth or more {}; la-mi-la natural / flat \
                 {:?}",
                self.heads,
                share(self.leap),
                share(self.rising),
                share(self.wide),
                share(self.modal),
                self.modal,
                share(self.dotted),
                share(self.white),
                share(self.rare),
                share(self.off_beat),
                share(self.texted_quarter),
                first.join(" / "),
                self.la_mi_la,
            );
        }
    }

    /// The figures of the best subjects of the first points and of the later ones (restatements
    /// left out), and of all candidates, of `texts` in the twelve mode variants for `seeds`, with
    /// `voices` voices.
    fn figures(
        texts: &[&str],
        seeds: impl Iterator<Item = u64> + Clone,
        voices: usize,
    ) -> [Figures; 4] {
        let mut out: [Figures; 4] = Default::default();
        for seed in seeds {
            for mode in modes(seed) {
                for spec in texts {
                    let builder = builder_in(spec, mode, voices, &Imitation::default(), seed);
                    let mut earlier = Vec::new();
                    for p in 0..builder.parts.len() {
                        let found = builder.subjects(p, &earlier).unwrap();
                        let Some(best) = found.subjects.first() else {
                            continue;
                        };
                        earlier.push(Arc::new(best.clone()));
                        if matches!(builder.parts[p].kind, PointKind::Restatement { .. }) {
                            continue;
                        }
                        let class = usize::from(p > 0);
                        for (i, subject) in found.subjects.iter().enumerate() {
                            let first = builder.reference() + i32::from(subject.degree);
                            let pitches = subject
                                .pitches(builder.mode, first, 0, Answer::Real)
                                .unwrap();
                            if i == 0 {
                                out[class].add(builder.mode, subject, &pitches);
                            }
                            out[2 + class].add(builder.mode, subject, &pitches);
                        }
                    }
                }
            }
        }
        out
    }

    /// A head that reaches past a pronoun or a copula declaims it on one note and keeps its
    /// melisma for the stressed syllable of the word after it, as Palestrina's Ad te levavi does
    /// (Jeppesen, p. 86): te and es take one note in every subject.
    #[test]
    fn monosyllables_take_one_note() {
        for (spec, head) in [
            ("Ad te le-vá-vi á-ni-mam me-am.", "Ad te le-vá-vi á-"),
            ("Tu es Pe-trus.", "Tu es Pe-trus."),
        ] {
            for mode in ["dorian", "phrygian", "mixolydian", "ionian-transposed"] {
                for seed in 1..=5 {
                    let builder = builder(spec, mode, 4, &Imitation::default(), seed);
                    let part = &builder.parts[0];
                    let words = crate::imitation::points::spelled(&builder.text, &part.head);
                    assert_eq!(words, head);
                    for subject in builder.subjects(0, &[]).unwrap().subjects {
                        let label = format!("{mode} seed {seed}: {}", describe(&builder, &subject));
                        let second = subject
                            .notes
                            .iter()
                            .position(|n| n.sung == Choice::Start(1))
                            .unwrap();
                        let after = subject.notes.get(second + 1).map(|n| n.sung);
                        assert!(matches!(after, Some(Choice::Start(2))), "{label}");
                    }
                }
            }
        }
    }

    /// The heads of the first points leap a fourth or more about as often as Palestrina's (44 %
    /// of his openings), and some leap between the final and the fifth, where a modal answer
    /// takes them up: Kyrie I, Agnus I and an own text, twenty seeds in the twelve mode variants.
    #[test]
    fn heads_leap_and_answer_modally() {
        let texts = ["kyrie", "agnus", "Ad te le-vá-vi á-ni-mam me-am."];
        let mut figures = Figures::default();
        for seed in 1..=20 {
            for mode in modes(seed) {
                for spec in texts {
                    let builder = builder_in(spec, mode, 4, &Imitation::default(), seed);
                    let found = builder.subjects(0, &[]).unwrap();
                    let best = &found.subjects[0];
                    let first = builder.reference() + i32::from(best.degree);
                    let pitches = best.pitches(mode, first, 0, Answer::Real).unwrap();
                    figures.add(mode, best, &pitches);
                }
            }
        }
        let leaps = 100.0 * figures.leap as f64 / figures.heads as f64;
        assert!((20.0..=55.0).contains(&leaps), "{figures:?}");
        assert!(figures.modal > 0, "{figures:?}");
    }

    /// The figures of the heads of five sections and an own text, for twenty seeds in the twelve
    /// mode variants, against Palestrina's ([`Figures`]).
    #[test]
    #[ignore = "slow: cargo test --release -- --ignored --nocapture"]
    fn head_figures() {
        let texts = [
            "kyrie",
            "christe",
            "agnus",
            "sanctus",
            "benedictus",
            "Ad te le-vá-vi á-ni-mam me-am.",
        ];
        let voices: usize = std::env::var("VOICES").map_or(4, |v| v.parse().unwrap());
        let seeds: u64 = std::env::var("SEEDS").map_or(20, |v| v.parse().unwrap());
        let [first, later, all_first, all_later] = figures(&texts, 1..=seeds, voices);
        first.show("first points, best subject");
        later.show("later points, best subject");
        all_first.show("first points, all subjects");
        all_later.show("later points, all subjects");
    }

    /// Statistics of the first points' heads against the soggetto study of Palestrina's duxes,
    /// and the time per point.
    #[test]
    #[ignore = "slow: cargo test --release -- --ignored --nocapture"]
    fn subject_statistics() {
        use std::collections::BTreeMap;
        let texts = [
            "kyrie",
            "christe",
            "agnus",
            "sanctus",
            "benedictus",
            "Ad te le-vá-vi á-ni-mam me-am.",
        ];
        let mut first_values: BTreeMap<String, BTreeMap<u8, usize>> = BTreeMap::new();
        let mut first_three: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
        let mut intervals: BTreeMap<String, BTreeMap<i8, usize>> = BTreeMap::new();
        let mut ranges: BTreeMap<i8, usize> = BTreeMap::new();
        let mut degrees: BTreeMap<String, BTreeMap<i8, usize>> = BTreeMap::new();
        let mut notes: BTreeMap<usize, usize> = BTreeMap::new();
        let mut later = (0.0, 0usize);
        let mut first = (0.0, 0usize);
        for seed in 1..=20 {
            for mode in modes(seed) {
                for spec in texts {
                    let builder = builder_in(spec, mode, 4, &Imitation::default(), seed);
                    let mut earlier = Vec::new();
                    for p in 0..builder.parts.len() {
                        let clock = std::time::Instant::now();
                        let found = builder.subjects(p, &earlier).unwrap();
                        let seconds = clock.elapsed().as_secs_f64();
                        let timing = if p == 0 { &mut first } else { &mut later };
                        timing.0 += seconds;
                        timing.1 += 1;
                        let Some(best) = found.subjects.first() else {
                            continue;
                        };
                        earlier.push(Arc::new(best.clone()));
                        if p > 0 {
                            continue;
                        }
                        let key = spec.split_whitespace().next().unwrap().to_string();
                        *first_values
                            .entry(key.clone())
                            .or_default()
                            .entry(best.notes[0].ticks)
                            .or_default() += 1;
                        let three: Vec<String> = best
                            .notes
                            .iter()
                            .take(3)
                            .map(|n| n.ticks.to_string())
                            .collect();
                        *first_three
                            .entry(key.clone())
                            .or_default()
                            .entry(three.join(" "))
                            .or_default() += 1;
                        let steps = best.steps();
                        *intervals
                            .entry(key.clone())
                            .or_default()
                            .entry(steps[1] - steps[0])
                            .or_default() += 1;
                        let range = steps.iter().max().unwrap() - steps.iter().min().unwrap();
                        *ranges.entry(range).or_default() += 1;
                        *degrees
                            .entry(format!("{:?}", mode.mode))
                            .or_default()
                            .entry(best.degree)
                            .or_default() += 1;
                        *notes.entry(best.notes.len()).or_default() += 1;
                    }
                }
            }
        }
        let share = |map: &BTreeMap<String, usize>| -> String {
            let total: usize = map.values().sum();
            let mut items: Vec<(&String, &usize)> = map.iter().collect();
            items.sort_by(|a, b| b.1.cmp(a.1));
            items
                .iter()
                .take(6)
                .map(|(k, v)| format!("{k} {:.0} %", 100.0 * **v as f64 / total as f64))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let keyed = |map: &BTreeMap<String, BTreeMap<String, usize>>| {
            for (text, counts) in map {
                println!("  {text}: {}", share(counts));
            }
        };
        let stringify = |map: &BTreeMap<String, BTreeMap<i8, usize>>| {
            map.iter()
                .map(|(k, counts)| {
                    let counts = counts.iter().map(|(i, n)| (format!("{i:+}"), *n)).collect();
                    (k.clone(), counts)
                })
                .collect::<BTreeMap<String, BTreeMap<String, usize>>>()
        };
        println!("first values:");
        let values = first_values
            .iter()
            .map(|(k, counts)| {
                (
                    k.clone(),
                    counts.iter().map(|(v, n)| (v.to_string(), *n)).collect(),
                )
            })
            .collect::<BTreeMap<String, BTreeMap<String, usize>>>();
        keyed(&values);
        println!("first three values:");
        keyed(&first_three);
        println!("first intervals (diatonic steps):");
        keyed(&stringify(&intervals));
        println!("start degrees (steps above the final):");
        keyed(&stringify(&degrees));
        let total: usize = ranges.values().sum();
        let ranges: Vec<String> = ranges
            .iter()
            .map(|(r, n)| format!("{r}: {:.0} %", 100.0 * *n as f64 / total as f64))
            .collect();
        println!("ranges (diatonic steps): {}", ranges.join(", "));
        let notes: Vec<String> = notes
            .iter()
            .map(|(k, n)| format!("{k}: {:.0} %", 100.0 * *n as f64 / total as f64))
            .collect();
        println!("notes per head: {}", notes.join(", "));
        println!(
            "time per point: first {:.1} ms, later {:.1} ms",
            1000.0 * first.0 / first.1 as f64,
            1000.0 * later.0 / later.1.max(1) as f64
        );
        // Heads from the chants of the Kyriale, in their modes and a fourth higher, against the
        // paraphrase study (PAR-EVEN-VALUES, PAR-HEAD-LENGTH).
        use crate::imitation::chant::Kyriale;
        let mut four: BTreeMap<String, usize> = BTreeMap::new();
        let mut whole = [0usize; 5];
        let mut distinct: BTreeMap<usize, usize> = BTreeMap::new();
        let (mut heads, mut fallbacks) = (0, 0);
        for (kyriale, spec) in [
            (Kyriale::KyrieIX, "kyrie"),
            (Kyriale::KyrieIX, "christe"),
            (Kyriale::KyrieIX, "kyrie2"),
            (Kyriale::KyrieIV, "kyrie"),
            (Kyriale::SanctusIX, "sanctus"),
            (Kyriale::AgnusIX, "agnus"),
            (Kyriale::GloriaIX, "gloria:1-5"),
        ] {
            let chant = kyriale.chant();
            let imitation = Imitation {
                subject: SubjectSource::Chant {
                    chant: Arc::clone(&chant),
                    phrases: None,
                },
                ..Imitation::default()
            };
            let numbered = chant.numbered_mode().unwrap();
            for mode in [numbered, numbered.with_transposition(!numbered.transposed)] {
                for seed in 1..=20 {
                    let builder = builder_in(spec, mode, 4, &imitation, seed);
                    let mut earlier = Vec::new();
                    for p in 0..builder.parts.len() {
                        let found = builder.subjects(p, &earlier).unwrap();
                        let Some(best) = found.subjects.first() else {
                            continue;
                        };
                        earlier.push(Arc::new(best.clone()));
                        if found.fallback.is_some() {
                            fallbacks += 1;
                            continue;
                        }
                        if !matches!(best.origin, Origin::Chant { .. }) {
                            continue;
                        }
                        heads += 1;
                        let values = best.values();
                        let first_four: Vec<String> =
                            values.iter().take(4).map(|v| v.to_string()).collect();
                        *four.entry(first_four.join(" ")).or_default() += 1;
                        for (i, &v) in values.iter().take(5).enumerate() {
                            whole[i] += usize::from(v >= 8);
                        }
                        let steps = best.steps();
                        let sung = 1 + steps.windows(2).filter(|w| w[0] != w[1]).count();
                        *distinct.entry(sung).or_default() += 1;
                    }
                }
            }
        }
        println!("chant heads: {heads}, points that took the text instead: {fallbacks}");
        println!("  first four values: {}", share(&four));
        let shares: Vec<String> = whole
            .iter()
            .map(|&n| format!("{:.0} %", 100.0 * n as f64 / heads as f64))
            .collect();
        println!("  notes 1-5 a whole or longer: {}", shares.join(", "));
        let distinct: Vec<String> = distinct
            .iter()
            .map(|(k, n)| format!("{k}: {:.0} %", 100.0 * *n as f64 / heads as f64))
            .collect();
        println!("  distinct pitches per head: {}", distinct.join(", "));
    }
}
