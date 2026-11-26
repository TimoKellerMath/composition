//! The two counts of the assessment (`docs/design/bach-fugen/scripts`) again in Rust, on the
//! lines of a file as those scripts see them. They check the reader against the published
//! numbers; the study's own counts are in `figures`.

use crate::kern::KernPiece;
use crate::pitch::Pitch;
use crate::score::{Note, Voice};
use crate::time::{Meter, TPW, Tick};

/// `har_consonance_proxy.py`: the program's HAR-CONSONANCE on measure starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HarProxy {
    pub downbeats: usize,
    pub pairs: usize,
    pub violations: usize,
    pub downbeats_with_violation: usize,
}

impl std::ops::AddAssign for HarProxy {
    fn add_assign(&mut self, other: HarProxy) {
        self.downbeats += other.downbeats;
        self.pairs += other.pairs;
        self.violations += other.violations;
        self.downbeats_with_violation += other.downbeats_with_violation;
    }
}

fn sounding(lines: &[Voice], t: Tick) -> Vec<(usize, Note, bool)> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(l, line)| {
            let i = line.index_at(t)?;
            Some((l, line.notes[i], line.repeats(i)))
        })
        .collect()
}

pub fn har_proxy(read: &KernPiece) -> HarProxy {
    let meter = read.piece.meter;
    // 4/2 counts as two measures of 2/2, as `fux_core::kern` reads it.
    let per = if meter == (Meter { num: 4, den: 2 }) {
        TPW
    } else {
        meter.bar()
    };
    let mut out = HarProxy::default();
    let mut t = 0;
    while t < read.piece.end {
        let notes = sounding(&read.lines, t);
        if notes.len() >= 2 {
            out.downbeats += 1;
            let bass = notes
                .iter()
                .enumerate()
                .min_by_key(|(i, (_, n, _))| (n.pitch, *i))
                .map(|(i, _)| i)
                .unwrap_or(0);
            let (_, low, low_repeated) = notes[bass];
            let mut hit = false;
            for (i, (_, note, repeated)) in notes.iter().enumerate() {
                if i == bass {
                    continue;
                }
                out.pairs += 1;
                if note.on < t || low.on < t || *repeated || low_repeated {
                    continue;
                }
                if !low.pitch.to(note.pitch).is_consonant(false) {
                    out.violations += 1;
                    hit = true;
                }
            }
            if hit {
                out.downbeats_with_violation += 1;
            }
        }
        t += per;
    }
    out
}

/// `beat_dissonance.py`: the rough measure on notated beats.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RawMeasure {
    pub beats: usize,
    pub beats_dissonant: usize,
    pub beats_struck: usize,
    pub downbeats: usize,
    pub downbeats_dissonant: usize,
    pub downbeats_struck: usize,
}

impl std::ops::AddAssign for RawMeasure {
    fn add_assign(&mut self, other: RawMeasure) {
        self.beats += other.beats;
        self.beats_dissonant += other.beats_dissonant;
        self.beats_struck += other.beats_struck;
        self.downbeats += other.downbeats;
        self.downbeats_dissonant += other.downbeats_dissonant;
        self.downbeats_struck += other.downbeats_struck;
    }
}

/// Dissonant as the rough measure defines it: seconds, sevenths, augmented and diminished
/// intervals, and a perfect fourth whose lower tone is the lowest sounding one.
pub fn dissonant(a: Pitch, b: Pitch, lowest: Pitch) -> bool {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let iv = lo.to(hi);
    if iv.simple() == (3, 5) {
        lo.midi == lowest.midi
    } else {
        !iv.is_consonant(false)
    }
}

/// Which pairs of sounding notes are dissonant at `t`, and whether both notes of one are struck.
fn pairs_at(lines: &[Voice], t: Tick) -> Option<(bool, bool)> {
    let notes = sounding(lines, t);
    if notes.len() < 2 {
        return None;
    }
    let lowest = notes.iter().map(|(_, n, _)| n.pitch).min()?;
    let mut any = false;
    let mut struck = false;
    for a in 0..notes.len() {
        for b in a + 1..notes.len() {
            if dissonant(notes[a].1.pitch, notes[b].1.pitch, lowest) {
                any = true;
                struck |= notes[a].1.on == t && notes[b].1.on == t;
            }
        }
    }
    Some((any, struck))
}

pub fn raw_measure(read: &KernPiece) -> RawMeasure {
    let beat = read.piece.meter.beat();
    let mut out = RawMeasure::default();
    for &(t, bar_start) in &read.rows {
        let pos = t - bar_start;
        if pos % beat != 0 {
            continue;
        }
        let Some((any, struck)) = pairs_at(&read.lines, t) else {
            continue;
        };
        out.beats += 1;
        out.beats_dissonant += any as usize;
        out.beats_struck += struck as usize;
        if pos == 0 {
            out.downbeats += 1;
            out.downbeats_dissonant += any as usize;
            out.downbeats_struck += struck as usize;
        }
    }
    out
}
