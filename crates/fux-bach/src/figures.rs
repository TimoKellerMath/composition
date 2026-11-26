//! Every tone against its chord: a chord tone, or a figure of its voice that explains a tone
//! foreign to the chord (passing tone, neighbour, suspension, appoggiatura and so on), or neither.
//!
//! The chord is an input (`Harmony`), so the figure of a tone depends on its voice, the chords at
//! its onset and at its resolution, and nothing else. The same definitions serve the count on
//! Bach's fugues (T3, T4) and the search, which may write exactly these figures.

use std::collections::BTreeMap;

use crate::key::Harmony;
use crate::score::{Note, Piece};
use crate::time::Tick;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Figure {
    ChordTone,
    Passing,
    Neighbour,
    Suspension,
    Appoggiatura,
    Anticipation,
    Escape,
    Cambiata,
    DoubleNeighbour,
    /// The pitch lies or returns in its voice over at least four beats and two changes of chord.
    Pedal,
    /// A held chord tone under which the chord leaves and comes back.
    Held,
    /// A tone of the chord before, kept for at most a beat into the new chord and then left by
    /// leap for one of its tones: the voice changes chord late.
    Delayed,
    Unexplained,
}

impl Figure {
    pub const ALL: [Figure; 13] = [
        Figure::ChordTone,
        Figure::Passing,
        Figure::Neighbour,
        Figure::Suspension,
        Figure::Appoggiatura,
        Figure::Anticipation,
        Figure::Escape,
        Figure::Cambiata,
        Figure::DoubleNeighbour,
        Figure::Pedal,
        Figure::Held,
        Figure::Delayed,
        Figure::Unexplained,
    ];

    pub const fn german(self) -> &'static str {
        match self {
            Figure::ChordTone => "Akkordton",
            Figure::Passing => "Durchgang",
            Figure::Neighbour => "Wechselnote",
            Figure::Suspension => "Vorhalt",
            Figure::Appoggiatura => "Vorschlag",
            Figure::Anticipation => "Antizipation",
            Figure::Escape => "abspringende Nebennote",
            Figure::Cambiata => "Cambiata",
            Figure::DoubleNeighbour => "Wechselnotengruppe",
            Figure::Pedal => "Orgelpunkt",
            Figure::Held => "Liegeton",
            Figure::Delayed => "verspäteter Wechsel",
            Figure::Unexplained => "unerklärt",
        }
    }
}

/// A piece read against a harmony.
#[derive(Clone, Copy)]
pub struct Reading<'a> {
    pub piece: &'a Piece,
    pub harmony: &'a Harmony,
}

impl Reading<'_> {
    fn chord_tone(&self, note: &Note, t: Tick) -> bool {
        self.harmony.chord_at(t).is_some_and(|c| c.has(note.pitch))
    }

    fn at_onset(&self, note: &Note) -> bool {
        self.chord_tone(note, note.on)
    }

    /// The notes around `i` that repeat its pitch without a gap: indices of the first and last.
    fn run(&self, v: usize, i: usize) -> (usize, usize) {
        let notes = &self.piece.voices[v].notes;
        let pitch = notes[i].pitch;
        let mut first = i;
        while first > 0
            && notes[first - 1].pitch == pitch
            && notes[first - 1].off == notes[first].on
        {
            first -= 1;
        }
        let mut last = i;
        while last + 1 < notes.len()
            && notes[last + 1].pitch == pitch
            && notes[last + 1].on == notes[last].off
        {
            last += 1;
        }
        (first, last)
    }

    /// Whether the pitch of note `i` lies or is repeated in its voice long enough to be a pedal.
    fn pedal(&self, v: usize, i: usize) -> bool {
        let notes = &self.piece.voices[v].notes;
        let (first, last) = self.run(v, i);
        let (from, until) = (notes[first].on, notes[last].off);
        until - from >= 4 * self.piece.meter.beat() && self.harmony.changes(from, until - 1) >= 2
    }

    /// A pitch that has been sounding in its voice since before `t`, held or struck again, and is
    /// foreign to the chord at `t`: a suspension if it was a chord tone before and leaves by step
    /// to a chord tone, a held tone if the chord comes back to it.
    fn lying(&self, v: usize, i: usize, t: Tick) -> Figure {
        let voice = &self.piece.voices[v];
        let (first, last) = self.run(v, i);
        let (from, until) = (voice.notes[first].on, voice.notes[last].off);
        let (Some(start), Some(now)) = (self.harmony.index_at(from), self.harmony.index_at(t))
        else {
            return Figure::Unexplained;
        };
        let pitch = voice.notes[i].pitch;
        let prepared = self.harmony.spans[start..now]
            .iter()
            .any(|s| s.chord.has(pitch));
        if !prepared {
            return Figure::Unexplained;
        }
        if let Some(a) = voice.after(last)
            && pitch.to(a.pitch).is_step()
            && self.at_onset(a)
        {
            return Figure::Suspension;
        }
        let returns = self.harmony.spans[now + 1..]
            .iter()
            .take_while(|s| s.on < until)
            .any(|s| s.chord.has(pitch));
        if returns {
            return Figure::Held;
        }
        if let Some(a) = voice.after(last)
            && self.at_onset(a)
            && until - t <= self.piece.meter.beat()
        {
            return Figure::Delayed;
        }
        Figure::Unexplained
    }

    /// First tone of a group of two neighbours: a step away from a chord tone, a third across
    /// it, and a step back to it.
    fn double_neighbour(&self, v: usize, i: usize) -> bool {
        let voice = &self.piece.voices[v];
        let (Some(before), Some(second)) = (voice.before(i), voice.after(i)) else {
            return false;
        };
        let Some(back) = voice.after(i + 1) else {
            return false;
        };
        let note = &voice.notes[i];
        let out = before.pitch.to(note.pitch);
        let across = note.pitch.to(second.pitch);
        let home = second.pitch.to(back.pitch);
        out.is_step()
            && across.steps == -2 * out.dir() as i16
            && home.is_step()
            && home.dir() == out.dir()
            && back.pitch.dia == before.pitch.dia
            && self.at_onset(before)
            && self.at_onset(back)
            && !self.at_onset(second)
    }

    /// The figure of note `i` of voice `v` at its onset; `None` where no chord is known.
    pub fn struck(&self, v: usize, i: usize) -> Option<Figure> {
        let voice = &self.piece.voices[v];
        let note = &voice.notes[i];
        let chord = self.harmony.chord_at(note.on)?;
        if chord.has(note.pitch) {
            return Some(Figure::ChordTone);
        }
        if self.pedal(v, i) {
            return Some(Figure::Pedal);
        }
        let before = voice.before(i);
        let after = voice.after(i);
        if before.is_some_and(|b| b.pitch == note.pitch) {
            let lying = self.lying(v, i, note.on);
            if lying != Figure::Unexplained {
                return Some(lying);
            }
        }
        if let Some(a) = after
            && a.pitch == note.pitch
            && self.at_onset(a)
            && self.harmony.index_at(a.on) != self.harmony.index_at(note.on)
        {
            return Some(Figure::Anticipation);
        }
        if let (Some(b), Some(a)) = (before, after) {
            let arrive = b.pitch.to(note.pitch);
            let leave = note.pitch.to(a.pitch);
            if arrive.is_step() && leave.is_step() {
                if arrive.dir() == leave.dir() {
                    if self.at_onset(a) {
                        return Some(Figure::Passing);
                    }
                    // A second passing tone before the chord tone.
                    if let Some(c) = voice.after(i + 1) {
                        let on = a.pitch.to(c.pitch);
                        if on.is_step() && on.dir() == leave.dir() && self.at_onset(c) {
                            return Some(Figure::Passing);
                        }
                    }
                } else if a.pitch.dia == b.pitch.dia && self.at_onset(a) {
                    return Some(Figure::Neighbour);
                }
            }
            if self.double_neighbour(v, i) || (i > 0 && self.double_neighbour(v, i - 1)) {
                return Some(Figure::DoubleNeighbour);
            }
            if arrive.is_step()
                && !leave.is_step()
                && !leave.is_unison()
                && self.at_onset(b)
                && self.at_onset(a)
            {
                return Some(if leave.dir() == arrive.dir() && leave.steps.abs() == 2 {
                    Figure::Cambiata
                } else if leave.dir() != arrive.dir() {
                    Figure::Escape
                } else {
                    Figure::Unexplained
                });
            }
        }
        let by_leap = before.is_none_or(|b| {
            let arrive = b.pitch.to(note.pitch);
            !arrive.is_step() && !arrive.is_unison()
        });
        if by_leap
            && let Some(a) = after
            && note.pitch.to(a.pitch).is_step()
            && self.at_onset(a)
        {
            return Some(Figure::Appoggiatura);
        }
        Some(Figure::Unexplained)
    }

    /// The figure of note `i` of voice `v` at a tick after its onset, while it is held.
    pub fn held(&self, v: usize, i: usize, t: Tick) -> Option<Figure> {
        let voice = &self.piece.voices[v];
        let note = &voice.notes[i];
        let chord = self.harmony.chord_at(t)?;
        if chord.has(note.pitch) {
            return Some(Figure::ChordTone);
        }
        if self.pedal(v, i) {
            return Some(Figure::Pedal);
        }
        Some(self.lying(v, i, t))
    }

    pub fn figure(&self, v: usize, i: usize, t: Tick) -> Option<Figure> {
        if self.piece.voices[v].notes[i].on == t {
            self.struck(v, i)
        } else {
            self.held(v, i, t)
        }
    }
}

/// How a dissonant pair on a beat was struck (protocol T3).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Layer {
    /// A: both tones struck, neither repeats the tone before it.
    Free,
    /// B: both struck, at least one repeats the tone before it.
    Repeated,
    /// C: one struck against a held one.
    AgainstHeld,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Pair {
    /// A voice against the lowest sounding tone, as HAR-CONSONANCE counts.
    Bass,
    /// Two upper voices, as HAR-DISSONANCE counts.
    Upper,
}

/// The dissonant pairs on the beats of a piece, by layer, kind and figure.
#[derive(Clone, Debug, Default)]
pub struct PairCensus {
    pub beats: usize,
    /// Beats with at least two sounding voices.
    pub beats_sounding: usize,
    /// Dissonant pairs on beats before the first chord of the harmony.
    pub without_chord: usize,
    pub pairs: BTreeMap<(Layer, Pair, Figure), usize>,
}

impl PairCensus {
    pub fn add(&mut self, other: &PairCensus) {
        self.beats += other.beats;
        self.beats_sounding += other.beats_sounding;
        self.without_chord += other.without_chord;
        for (key, count) in &other.pairs {
            *self.pairs.entry(*key).or_default() += count;
        }
    }

    /// Pairs of the layers A and B (both tones struck): all of them and the unexplained ones.
    pub fn struck(&self) -> (usize, usize) {
        let mut total = 0;
        let mut rest = 0;
        for ((layer, _, figure), count) in &self.pairs {
            if *layer != Layer::AgainstHeld {
                total += count;
                if *figure == Figure::Unexplained {
                    rest += count;
                }
            }
        }
        (total, rest)
    }

    pub fn of(&self, layer: Layer, pair: Option<Pair>, figure: Option<Figure>) -> usize {
        self.pairs
            .iter()
            .filter(|((l, p, f), _)| {
                *l == layer && pair.is_none_or(|x| x == *p) && figure.is_none_or(|x| x == *f)
            })
            .map(|(_, c)| c)
            .sum()
    }
}

/// The worse of two figures decides a pair: unexplained before any figure, a figure before a
/// chord tone.
fn of_pair(a: Figure, b: Figure) -> Figure {
    a.max(b)
}

pub fn pair_census(reading: Reading) -> PairCensus {
    let piece = reading.piece;
    let mut out = PairCensus::default();
    for t in piece.beats() {
        out.beats += 1;
        // Voice, index of the note, struck, repeated.
        let sounding: Vec<(usize, usize, bool, bool)> = piece
            .voices
            .iter()
            .enumerate()
            .filter_map(|(v, voice)| {
                let i = voice.index_at(t)?;
                let struck = voice.notes[i].on == t;
                Some((v, i, struck, struck && voice.repeats(i)))
            })
            .collect();
        if sounding.len() < 2 {
            continue;
        }
        out.beats_sounding += 1;
        let pitch = |s: &(usize, usize, bool, bool)| piece.voices[s.0].notes[s.1].pitch;
        let Some(bass) =
            (0..sounding.len()).min_by_key(|k| (pitch(&sounding[*k]), usize::MAX - sounding[*k].0))
        else {
            continue;
        };
        for a in 0..sounding.len() {
            for b in a + 1..sounding.len() {
                let (x, y) = (&sounding[a], &sounding[b]);
                let kind = if a == bass || b == bass {
                    Pair::Bass
                } else {
                    Pair::Upper
                };
                let (lo, hi) = if pitch(x) <= pitch(y) {
                    (pitch(x), pitch(y))
                } else {
                    (pitch(y), pitch(x))
                };
                if lo.to(hi).is_consonant(kind == Pair::Upper) {
                    continue;
                }
                let layer = match (x.2, y.2) {
                    (true, true) if x.3 || y.3 => Layer::Repeated,
                    (true, true) => Layer::Free,
                    (false, false) => continue,
                    _ => Layer::AgainstHeld,
                };
                let (Some(fx), Some(fy)) =
                    (reading.figure(x.0, x.1, t), reading.figure(y.0, y.1, t))
                else {
                    out.without_chord += 1;
                    continue;
                };
                *out.pairs.entry((layer, kind, of_pair(fx, fy))).or_default() += 1;
            }
        }
    }
    out
}

/// Every tone of a piece by its figure and by the weight of its onset: 2 on a beat, 1 on the
/// first subdivision, 0 finer. Held tones that become foreign to the chord on a beat count as
/// events of their own under `held`.
#[derive(Clone, Debug, Default)]
pub struct NoteCensus {
    pub notes: usize,
    pub without_chord: usize,
    pub struck: BTreeMap<(u8, Figure), usize>,
    pub held: BTreeMap<Figure, usize>,
}

impl NoteCensus {
    pub fn add(&mut self, other: &NoteCensus) {
        self.notes += other.notes;
        self.without_chord += other.without_chord;
        for (key, count) in &other.struck {
            *self.struck.entry(*key).or_default() += count;
        }
        for (key, count) in &other.held {
            *self.held.entry(*key).or_default() += count;
        }
    }

    /// Struck tones with this figure, on any weight.
    pub fn count(&self, figure: Figure) -> usize {
        self.struck
            .iter()
            .filter(|((_, f), _)| *f == figure)
            .map(|(_, c)| c)
            .sum()
    }

    /// Tones with a chord at their onset.
    pub fn read(&self) -> usize {
        self.notes - self.without_chord
    }
}

pub fn note_census(reading: Reading) -> NoteCensus {
    let piece = reading.piece;
    let mut out = NoteCensus::default();
    let beat = piece.meter.beat();
    for (v, voice) in piece.voices.iter().enumerate() {
        for (i, note) in voice.notes.iter().enumerate() {
            out.notes += 1;
            match reading.struck(v, i) {
                Some(figure) => {
                    let weight = piece.weight(note.on).min(2);
                    *out.struck.entry((weight, figure)).or_default() += 1;
                }
                None => out.without_chord += 1,
            }
            // Beats inside the tone.
            let mut t = note.on + beat - (note.on - piece.origin).rem_euclid(beat);
            while t < note.off {
                if let Some(figure) = reading.held(v, i, t)
                    && figure != Figure::ChordTone
                {
                    *out.held.entry(figure).or_default() += 1;
                }
                t += beat;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::{Chord, Key, Kind};
    use crate::pitch::{Lof, Pitch};
    use crate::score::Voice;
    use crate::time::Meter;

    fn line(tones: &[(i8, i8, Tick)]) -> Voice {
        let mut voice = Voice::default();
        let mut on = 0;
        for (step, octave, ticks) in tones {
            voice.push(on, on + ticks, Pitch::new(*step, 0, *octave));
            on += ticks;
        }
        voice
    }

    fn figures(tones: &[(i8, i8, Tick)], chords: &[(Tick, i8, Kind)]) -> Vec<Figure> {
        let key = Key::parse("C").unwrap();
        let piece = Piece::new("test", key, Meter::COMMON, vec![line(tones)]);
        let mut harmony = Harmony::default();
        for (on, root, kind) in chords {
            harmony.push(*on, Chord::new(Lof(*root), *kind), key);
        }
        let reading = Reading {
            piece: &piece,
            harmony: &harmony,
        };
        (0..tones.len())
            .map(|i| reading.struck(0, i).unwrap())
            .collect()
    }

    use Figure::*;

    #[test]
    fn stepwise_figures() {
        let c = [(0, 0, Kind::Maj)];
        // C D E: passing tone; E F E: neighbour; G A B C: two passing tones.
        assert_eq!(
            figures(
                &[(0, 4, 12), (1, 4, 12), (2, 4, 12), (3, 4, 12), (2, 4, 12)],
                &c
            ),
            [ChordTone, Passing, ChordTone, Neighbour, ChordTone]
        );
        assert_eq!(
            figures(&[(4, 4, 12), (5, 4, 12), (6, 4, 12), (0, 5, 12)], &c),
            [ChordTone, Passing, Passing, ChordTone]
        );
        // Three foreign tones in a row over a chord of D flat: the first has no resolution.
        let des = [(0, -5, Kind::Maj)];
        assert_eq!(
            figures(&[(0, 4, 12), (1, 4, 12), (2, 4, 12), (3, 4, 12)], &des),
            [Unexplained, Passing, Passing, ChordTone]
        );
    }

    #[test]
    fn leaps_and_repetitions() {
        let c = [(0, 0, Kind::Maj)];
        // Leap to F, resolved to E: appoggiatura. C D, leap down to G: an escape tone.
        assert_eq!(
            figures(&[(0, 4, 24), (3, 4, 24), (2, 4, 24)], &c),
            [ChordTone, Appoggiatura, ChordTone]
        );
        assert_eq!(
            figures(&[(0, 4, 24), (1, 4, 24), (4, 3, 24)], &c),
            [ChordTone, Escape, ChordTone]
        );
        // C D B C: a group of two neighbours.
        assert_eq!(
            figures(&[(0, 4, 12), (1, 4, 12), (6, 3, 12), (0, 4, 12)], &c),
            [ChordTone, DoubleNeighbour, DoubleNeighbour, ChordTone]
        );
        // F over F major, struck again over C major and resolved to E: a suspension. The B before
        // the chord of G arrives is an anticipation.
        let changes = [(0, -1, Kind::Maj), (24, 0, Kind::Maj), (72, 1, Kind::Maj)];
        assert_eq!(
            figures(
                &[(3, 4, 24), (3, 4, 24), (2, 4, 12), (6, 3, 12), (6, 3, 24)],
                &changes
            ),
            [ChordTone, Suspension, ChordTone, Anticipation, ChordTone]
        );
        // A leap away from a foreign tone is no figure.
        assert_eq!(
            figures(&[(0, 4, 24), (3, 4, 24), (0, 5, 24)], &c),
            [ChordTone, Unexplained, ChordTone]
        );
    }

    #[test]
    fn pairs_on_beats() {
        // Upper voice F E over a held C, then D struck with a B below it: a seventh chord tone
        // pair and a suspension-like pair against a held tone.
        let key = Key::parse("C").unwrap();
        let upper = line(&[(0, 5, 24), (3, 5, 24), (2, 5, 24), (3, 5, 24)]);
        let lower = line(&[(0, 4, 48), (0, 4, 24), (4, 3, 24)]);
        let piece = Piece::new("pairs", key, Meter::COMMON, vec![upper, lower]);
        let mut harmony = Harmony::default();
        harmony.push(0, Chord::new(Lof(0), Kind::Maj), key);
        harmony.push(72, Chord::new(Lof(1), Kind::Dom7), key);
        let census = pair_census(Reading {
            piece: &piece,
            harmony: &harmony,
        });
        assert_eq!(census.beats, 4);
        // Beat 2: F against the held C is a fourth against the bass, an appoggiatura resolved to E.
        assert_eq!(
            census.of(Layer::AgainstHeld, Some(Pair::Bass), Some(Appoggiatura)),
            1
        );
        // Beat 4: F over G, both struck, chord tones of the dominant seventh.
        assert_eq!(census.of(Layer::Free, Some(Pair::Bass), Some(ChordTone)), 1);
        assert_eq!(census.struck(), (1, 0));
    }
}
