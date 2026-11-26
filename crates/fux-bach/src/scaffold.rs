//! The scaffold: the chords of a piece from its plan. Under the given lines the chords are the
//! ones that explain their tones and follow each other as chords do in Bach's analysed pieces;
//! where nothing is given a sequence leads to the next key; cadences and pedal points stand
//! where the plan puts them. One best path through all of it (Viterbi) on a grid of half beats.
//!
//! The model is counted from the Roman numeral analyses of When in Rome (371 chorales, the 24
//! preludes of WTC I, seven fugues): how chords follow each other inside a key, and at which
//! places of the bar the chord changes in the keyboard pieces.

use std::sync::OnceLock;

use serde_json::{Value, json};

use crate::chord::weights;
use crate::harmonize::{Rel, Transitions};
use crate::key::{Chord, Harmony, Key, Kind};
use crate::rng::Rng;
use crate::roman;
use crate::score::{Note, Piece, Voice};
use crate::time::{Meter, Tick};

/// What the generator learned.
#[derive(Clone, Debug, Default)]
pub struct Model {
    pub transitions: Transitions,
    /// By metric weight (0 to 4): how often the chord changed at a step of the grid, and how
    /// many steps there were.
    pub changes: [[f64; 2]; 5],
    /// The bass of the chords, from the figures of the analysed fugues.
    pub bass: crate::bass::Model,
}

/// The step of the grid: the half beat, in compound meters the third of the beat.
pub fn step_of(meter: Meter) -> Tick {
    if meter.compound() {
        meter.beat() / 3
    } else {
        meter.beat() / 2
    }
}

impl Model {
    /// Counts the successions of an analysis, and with `rhythm` where its chords change.
    pub fn learn(&mut self, analysis: &roman::Analysis, rhythm: bool) {
        self.transitions.learn(analysis);
        if !rhythm {
            return;
        }
        let meter = analysis.meter.unwrap_or(Meter::COMMON);
        let step = step_of(meter);
        let line = roman::timeline(analysis);
        let (Some(first), Some(last)) = (line.first(), line.last()) else {
            return;
        };
        let mut t = first.0.div_euclid(step) * step + step;
        let end = last.0;
        let mut index = 0;
        while t <= end {
            // The chord in force just before this step and at it.
            while index + 1 < line.len() && line[index + 1].0 < t {
                index += 1;
            }
            let before = line[index].1;
            let now = line
                .iter()
                .skip(index)
                .take_while(|l| l.0 <= t)
                .last()
                .map_or(before, |l| l.1);
            let weight = meter.weight(t.rem_euclid(meter.bar())) as usize;
            self.changes[weight][1] += 1.0;
            if now.mask() != before.mask() {
                self.changes[weight][0] += 1.0;
            }
            t += step;
        }
    }

    /// The share of steps of this weight at which the chord changes.
    pub fn change(&self, weight: u8) -> f64 {
        let [changed, steps] = self.changes[(weight as usize).min(4)];
        ((changed + 1.0) / (steps + 2.0)).clamp(0.02, 0.98)
    }

    pub fn to_json(&self) -> Value {
        let (pairs, firsts) = self.transitions.rows();
        json!({
            "note": "Counted by `bach-probe b2-tables` on the analyses of When in Rome in references/datasets/bach-fugen/wir; numbers only.",
            "transitions": { "pairs": pairs, "firsts": firsts },
            "changes": self.changes,
            "bass": self.bass.to_json(),
        })
    }

    pub fn from_json(value: &Value) -> Model {
        let rows = |value: &Value| -> Vec<Vec<f64>> {
            value
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|r| {
                            r.as_array()
                                .map(|r| r.iter().filter_map(Value::as_f64).collect())
                                .unwrap_or_default()
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let fixed = |row: &Vec<f64>, n: usize| -> Vec<f64> {
            (0..n).map(|k| row.get(k).copied().unwrap_or(0.0)).collect()
        };
        let pairs: Vec<[f64; 8]> = rows(&value["transitions"]["pairs"])
            .iter()
            .map(|r| fixed(r, 8).try_into().unwrap_or([0.0; 8]))
            .collect();
        let firsts: Vec<[f64; 5]> = rows(&value["transitions"]["firsts"])
            .iter()
            .map(|r| fixed(r, 5).try_into().unwrap_or([0.0; 5]))
            .collect();
        let mut changes = [[0.0; 2]; 5];
        for (k, row) in rows(&value["changes"]).iter().enumerate().take(5) {
            changes[k] = [
                row.first().copied().unwrap_or(0.0),
                row.get(1).copied().unwrap_or(0.0),
            ];
        }
        Model {
            transitions: Transitions::from_rows(&pairs, &firsts),
            changes,
            bass: crate::bass::Model::from_json(&value["bass"]),
        }
    }
}

/// The model that comes with the program.
pub fn embedded() -> &'static Model {
    static MODEL: OnceLock<Model> = OnceLock::new();
    MODEL.get_or_init(|| {
        let value: Value = serde_json::from_str(include_str!("../data/harmony.json"))
            .expect("data/harmony.json is JSON");
        Model::from_json(&value)
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CadenceKind {
    /// Dominant in root position to the tonic, the tonic on top.
    Perfect,
    /// Dominant in root position to the tonic, another tone on top.
    RootImperfect,
    /// A dominant to the tonic, one of them inverted.
    Imperfect,
    /// A rest on the dominant.
    Half,
    /// Dominant to the sixth degree.
    Deceptive,
}

impl CadenceKind {
    /// `PAC`, `rIAC`, `IAC`, `HC`, `DC`.
    pub fn parse(text: &str) -> Option<CadenceKind> {
        Some(match text.trim() {
            "PAC" => CadenceKind::Perfect,
            "rIAC" => CadenceKind::RootImperfect,
            "IAC" => CadenceKind::Imperfect,
            "HC" => CadenceKind::Half,
            "DC" => CadenceKind::Deceptive,
            _ => return None,
        })
    }

    pub const fn label(self) -> &'static str {
        match self {
            CadenceKind::Perfect => "PAC",
            CadenceKind::RootImperfect => "rIAC",
            CadenceKind::Imperfect => "IAC",
            CadenceKind::Half => "HC",
            CadenceKind::Deceptive => "DC",
        }
    }
}

/// A cadence that arrives at `at`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cadence {
    pub at: Tick,
    pub key: Key,
    pub kind: CadenceKind,
}

impl Cadence {
    /// The key the cadence is read in and the chord it arrives on, in a piece in `home`: a
    /// cadence on the major tonic of a piece in minor is one in its key with the major chord.
    pub fn goal(&self, home: Key) -> (Key, Chord) {
        let picardy = home.minor && !self.key.minor && self.key.tonic == home.tonic;
        let key = if picardy { home } else { self.key };
        let chord = match self.kind {
            CadenceKind::Half => Rel::new(4, 0, Kind::Maj).in_key(key),
            CadenceKind::Deceptive => triad(5, key).in_key(key),
            _ if picardy => Chord::new(key.tonic, Kind::Maj),
            _ => triad(0, key).in_key(key),
        };
        (key, chord)
    }
}

/// A pedal point: the bass holds the degree (0 the tonic, 4 the dominant) of `key`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pedal {
    pub from: Tick,
    pub until: Tick,
    pub key: Key,
    pub degree: usize,
}

impl Pedal {
    /// The tone of the pedal point in a voice whose range begins at `low`: the degree in the
    /// lowest octave of the range.
    pub fn tone(&self, low: crate::pitch::Pitch) -> crate::pitch::Pitch {
        let class = self.key.degree(self.degree);
        let mut pitch = crate::pitch::Pitch::new(class.step(), class.alter(), low.octave() - 1);
        while pitch.midi < low.midi {
            pitch = crate::pitch::Pitch {
                midi: pitch.midi + 12,
                dia: pitch.dia + 7,
            };
        }
        pitch
    }
}

/// What the plan of a piece gives the scaffold.
#[derive(Clone, Debug)]
pub struct Plan {
    pub key: Key,
    pub meter: Meter,
    pub origin: Tick,
    pub end: Tick,
    /// The given tones, voice by voice: what the chords have to explain.
    pub lines: Vec<Vec<Note>>,
    /// The key from each tick on, in time order.
    pub keys: Vec<(Tick, Key)>,
    pub cadences: Vec<Cadence>,
    pub pedals: Vec<Pedal>,
    /// Tones the chord of their tick has to hold (tick, pitch class): given tones that no
    /// figure explains against the chords chosen without this.
    pub must: Vec<(Tick, u8)>,
    /// A piece in minor ends on the major chord.
    pub picardy: bool,
    /// The plan ends on its tonic (not so for a passage that leads on).
    pub close: bool,
    /// From here to the end the final chord holds (the tonic, major with `picardy`): where the
    /// voices of a piece strike their last tones together.
    pub hold: Option<Tick>,
    /// Where a given voice is the lowest that sounds: tick and pitch class of its tone. A chord
    /// that begins there is the likelier the likelier this tone is its bass. Empty: the bass
    /// plays no part in the choice of the chords.
    pub lowest: Vec<(Tick, u8)>,
    pub seed: u64,
}

/// The triad on a degree of a key: the natural degrees, with the major dominant in minor.
pub fn triad(degree: usize, key: Key) -> Rel {
    let kind = match (key.minor, degree % 7) {
        (false, 0 | 3 | 4) | (true, 2 | 4 | 5 | 6) => Kind::Maj,
        (false, 6) | (true, 1) => Kind::Dim,
        _ => Kind::Min,
    };
    Rel::new((degree % 7) as u8, 0, kind)
}

fn tonic(key: Key) -> Rel {
    triad(0, key)
}

/// Whether a chord has dominant function in its key: V, V7, or the diminished chord on the
/// leading tone.
pub fn is_dominant(rel: Rel, key: Key) -> bool {
    match rel.degree {
        4 => rel.alter == 0 && matches!(rel.kind, Kind::Maj | Kind::Dom7),
        6 => {
            rel.alter == i8::from(key.minor)
                && matches!(rel.kind, Kind::Dim | Kind::Dim7 | Kind::HalfDim7)
        }
        _ => false,
    }
}

/// The chord a sequence puts on a degree: the triad, but the seventh chords that Bach's
/// sequences favour where the triad would be diminished (and on the second degree of major), and
/// the dominant seventh right before the goal. The leading-tone triad of major stays a triad.
fn in_sequence(degree: usize, key: Key, last: bool) -> Rel {
    match (key.minor, degree % 7) {
        (_, 4) if last => Rel::new(4, 0, Kind::Dom7),
        (false, 1) => Rel::new(1, 0, Kind::Min7),
        (true, 1) => Rel::new(1, 0, Kind::HalfDim7),
        _ => triad(degree, key),
    }
}

/// How long one chord of a sequence lasts in a free run of this length: half a bar where the
/// bar has two halves, a bar in triple time, a beat in a run shorter than two bars.
pub fn sequence_unit(meter: Meter, length: Tick) -> Tick {
    let beats = meter.bar() / meter.beat();
    if length < 2 * meter.bar() {
        meter.beat()
    } else if beats % 2 == 0 {
        meter.bar() / 2
    } else {
        meter.bar()
    }
}

/// The models of a sequence, as degrees read backwards from the tonic it leads to.
const SEQUENCES: [(&[usize], f64); 3] = [
    // Falling fifths: … I IV vii iii vi ii V | I.
    (&[4, 1, 5, 2, 6, 3, 0], 0.6),
    // Falling thirds with their dominants: I V vi iii IV I ii V | I.
    (&[4, 1, 0, 3, 2, 5, 4, 0], 0.2),
    // Rising steps, each with the chord a third below before it: I vi ii vii iii I IV ii V | I.
    (&[4, 1, 3, 0, 2, 6, 1, 5, 0], 0.2),
];

/// What one step of the grid knows before the path is searched.
struct Step {
    at: Tick,
    key: Key,
    weight: u8,
    /// Weight of every pitch class among the given tones here.
    tones: [f32; 12],
    /// The chord a sequence proposes here.
    proposal: Option<Rel>,
    /// If set, only these chords may stand here.
    only: Option<Vec<Chord>>,
    /// These chords may not stand here.
    never: Vec<Chord>,
    /// Pitch classes the chord here has to hold.
    must: u16,
    /// The pitch class of the lowest sounding tone, if a given voice has it.
    low: Option<u8>,
}

fn same(a: Chord, b: Chord) -> bool {
    a.root == b.root && a.kind == b.kind
}

const FIT: f64 = 4.0;
/// What a chord costs by its kind where tones are given: four tones cover more than three, and
/// the plain triads come first.
const fn plain(kind: Kind) -> f64 {
    match kind {
        Kind::Maj | Kind::Min => 0.0,
        Kind::Dom7 => 0.2,
        Kind::Dim => 0.1,
        Kind::Dim7 => 0.4,
        Kind::HalfDim7 => 0.48,
        Kind::Min7 => 0.56,
        _ => 0.88,
    }
}
const PROPOSED: f64 = 3.0;
/// How much the successions of the analyses weigh against what the tones and the sequences ask
/// for. The analyses are mostly chorales, whose chords are plainer than those of a fugue.
const FLOW: f64 = 0.5;
const MAJOR_IN_MAJOR: f64 = 0.25;
const MODULATE: f64 = -4.0;
/// What a chord costs that lacks a tone it has to hold.
const MUST: f64 = 12.0;
/// The weight of the bass where a chord begins over a given lowest tone (see `Plan::lowest`).
const BASS: f64 = 1.0;
const FORBIDDEN: f64 = -1.0e6;

/// The chords of a piece from its plan.
pub fn generate(plan: &Plan, model: &Model) -> Harmony {
    let meter = plan.meter;
    let step = step_of(meter);
    let first = plan.origin - (plan.origin + step - 1).div_euclid(step) * step;
    let count = ((plan.end - first + step - 1) / step).max(1) as usize;
    let voices: Vec<Voice> = plan
        .lines
        .iter()
        .map(|notes| {
            let mut notes = notes.clone();
            notes.sort_by_key(|n| n.on);
            Voice { notes }
        })
        .collect();
    let mut piece = Piece::new("plan", plan.key, meter, voices);
    piece.origin = plan.origin;
    piece.end = plan.end;
    let key_at = |t: Tick| -> Key {
        plan.keys
            .iter()
            .rev()
            .find(|(from, _)| *from <= t)
            .map_or(plan.key, |(_, k)| *k)
    };
    let mut steps: Vec<Step> = (0..count)
        .map(|i| {
            let at = first + i as Tick * step;
            Step {
                at,
                key: key_at(at),
                weight: meter.weight((at - plan.origin).rem_euclid(meter.bar())),
                tones: weights(&piece, at, at + step),
                proposal: None,
                only: None,
                never: Vec::new(),
                must: 0,
                low: plan
                    .lowest
                    .iter()
                    .find(|(t, _)| *t == at)
                    .map(|(_, pc)| *pc),
            }
        })
        .collect();
    let index_of = |t: Tick| -> usize { (((t - first) / step).max(0) as usize).min(count - 1) };
    let given = |s: &Step| s.tones.iter().sum::<f32>() > 0.0;
    for (tick, pc) in &plan.must {
        if *tick >= first && *tick < plan.end {
            steps[index_of(*tick)].must |= 1 << pc;
        }
    }

    // Cadences: the goal at its place, a dominant on the beat before it. A cadence on the major
    // tonic of a piece in minor is one in its key with the major chord.
    let beat_steps = (meter.beat() / step).max(1) as usize;
    let mut cadence_steps: Vec<usize> = Vec::new();
    let mut cadences: Vec<&Cadence> = plan.cadences.iter().collect();
    cadences.sort_by_key(|c| c.at);
    for cadence in cadences {
        if cadence.at < first || cadence.at >= plan.end {
            continue;
        }
        let at = index_of(cadence.at);
        let picardy = plan.key.minor && !cadence.key.minor && cadence.key.tonic == plan.key.tonic;
        let key = if picardy { plan.key } else { cadence.key };
        let chord = |rel: Rel| rel.in_key(key);
        let dominant = vec![
            chord(Rel::new(4, 0, Kind::Maj)),
            chord(Rel::new(4, 0, Kind::Dom7)),
        ];
        let leading = vec![
            chord(Rel::new(6, i8::from(key.minor), Kind::Dim)),
            chord(Rel::new(6, i8::from(key.minor), Kind::Dim7)),
        ];
        let home = if picardy {
            Chord::new(key.tonic, Kind::Maj)
        } else {
            chord(tonic(key))
        };
        let (goal, approach): (Chord, Option<Vec<Chord>>) = match cadence.kind {
            CadenceKind::Perfect | CadenceKind::RootImperfect => (home, Some(dominant.clone())),
            CadenceKind::Imperfect => (home, Some([dominant.clone(), leading].concat())),
            CadenceKind::Half => (dominant[0], None),
            CadenceKind::Deceptive => (chord(triad(5, key)), Some(dominant.clone())),
        };
        // The key of the cadence holds from the bar before it, as far as nothing is set there.
        let from = index_of(cadence.at - meter.bar());
        for s in &mut steps[from..=at] {
            if s.only.is_none() {
                s.key = key;
            }
        }
        if steps[at].only.is_some() {
            continue;
        }
        steps[at].only = Some(vec![goal]);
        for s in &mut steps[at.saturating_sub(beat_steps)..at] {
            if s.only.is_some() {
                continue;
            }
            match &approach {
                Some(chords) => s.only = Some(chords.clone()),
                None => s.never = dominant.clone(),
            }
        }
        cadence_steps.push(at);
    }
    // Pedal points begin and end on the chord of their tone.
    for pedal in &plan.pedals {
        if pedal.until <= first || pedal.from >= plan.end || pedal.until <= pedal.from {
            continue;
        }
        let (a, b) = (index_of(pedal.from), index_of(pedal.until - 1));
        let root = pedal.key.degree(pedal.degree);
        let chords: Vec<Chord> = match pedal.degree {
            4 => vec![Chord::new(root, Kind::Maj), Chord::new(root, Kind::Dom7)],
            _ => vec![Chord::new(root, Kind::Maj), Chord::new(root, Kind::Min)],
        };
        for s in &mut steps[a..=b] {
            if s.only.is_none() {
                s.key = pedal.key;
            }
        }
        for i in [a, b] {
            if steps[i].only.is_none() {
                steps[i].only = Some(chords.clone());
            }
        }
    }
    // The piece begins and ends on its tonic.
    let home = tonic(plan.key).in_key(plan.key);
    if steps[0].only.is_none() && steps[0].key == plan.key {
        steps[0].only = Some(vec![home]);
    }
    if plan.close && steps[count - 1].only.is_none() {
        steps[count - 1].key = plan.key;
        steps[count - 1].only = Some(vec![home, Chord::new(plan.key.tonic, Kind::Maj)]);
    }
    // The final chord stands from where the voices strike it: a chord that came later in their
    // last tones would leave them only what it shares with the chord before.
    if let Some(hold) = plan.hold.filter(|t| (first..plan.end).contains(t)) {
        let last = if plan.picardy && plan.key.minor {
            Chord::new(plan.key.tonic, Kind::Maj)
        } else {
            home
        };
        for s in &mut steps[index_of(hold)..] {
            s.key = plan.key;
            s.only = Some(vec![last]);
        }
    }

    // Free runs: where nothing is given for at least a bar, a sequence leads to what follows.
    let mut rng = Rng::new(plan.seed);
    let bar_steps = (meter.bar() / step) as usize;
    let mut i = 0;
    while i < count {
        if given(&steps[i]) {
            i += 1;
            continue;
        }
        let from = i;
        while i < count && !given(&steps[i]) {
            i += 1;
        }
        // Split the run at the cadences inside it: each part leads to its own goal.
        let mut bounds: Vec<usize> = cadence_steps
            .iter()
            .copied()
            .filter(|c| *c > from && *c < i)
            .collect();
        bounds.sort_unstable();
        bounds.push(i);
        let mut start = from;
        for end in bounds {
            let length = end - start;
            if length >= bar_steps && end < count {
                // The run stands in the key of its goal.
                let key = steps[end].key;
                let unit = (sequence_unit(meter, length as Tick * step) / step).max(1) as usize;
                let weights: Vec<f64> = SEQUENCES.iter().map(|s| s.1).collect();
                let model = SEQUENCES[rng.weighted(&weights).unwrap_or(0)].0;
                for (k, s) in steps.iter_mut().enumerate().take(end).skip(start) {
                    let back = (end - 1 - k) / unit;
                    let rel = in_sequence(model[back % model.len()], key, back == 0);
                    // What a cadence or a pedal point has set keeps its key.
                    if s.only.is_none() {
                        s.key = key;
                        s.proposal = Some(rel);
                    }
                }
            } else if end < count {
                // A short gap: in the key of what follows.
                let key = steps[end].key;
                for s in &mut steps[start..end] {
                    if s.only.is_none() {
                        s.key = key;
                    }
                }
            }
            start = end;
        }
    }

    // The best path.
    let vocabulary = [
        model.transitions.vocabulary(false, LEAST),
        model.transitions.vocabulary(true, LEAST),
    ];
    let states = |s: &Step| -> &Vec<Rel> { &vocabulary[usize::from(s.key.minor)] };
    let emission = |s: &Step, rel: Rel| -> f64 {
        let chord = rel.in_key(s.key);
        if let Some(only) = &s.only
            && !only.iter().any(|c| same(*c, chord))
        {
            return FORBIDDEN;
        }
        if s.never.iter().any(|c| same(*c, chord)) {
            return FORBIDDEN;
        }
        let mut score = 0.0;
        if s.must & !chord.mask() != 0 {
            score -= MUST;
        }
        let total: f32 = s.tones.iter().sum();
        if total > 0.0 {
            let mask = rel.in_key(s.key).mask();
            let inside: f32 = (0..12)
                .filter(|pc| mask & (1 << pc) != 0)
                .map(|pc| s.tones[pc])
                .sum();
            score += FIT * ((2.0 * inside - total) / total) as f64 - plain(rel.kind);
            // In a major key the major triads crowd out the others: a little weight against.
            if !s.key.minor && rel.kind == Kind::Maj {
                score -= MAJOR_IN_MAJOR;
            }
        }
        if let Some(proposal) = s.proposal
            && proposal.degree == rel.degree
            && proposal.alter == rel.alter
        {
            score += if proposal.kind == rel.kind {
                PROPOSED
            } else {
                PROPOSED / 2.0
            };
        }
        score
    };
    // Where a chord begins over a given lowest tone: how likely that tone is its bass.
    let onset = |s: &Step, rel: Rel| -> f64 {
        s.low.map_or(0.0, |pc| {
            BASS * model.bass.log_against_root(rel.in_key(s.key), pc)
        })
    };
    let mut best: Vec<f64> = states(&steps[0])
        .iter()
        .map(|rel| emission(&steps[0], *rel) + onset(&steps[0], *rel))
        .collect();
    let mut back: Vec<Vec<usize>> = Vec::with_capacity(count);
    for i in 1..count {
        let (before, now) = (&steps[i - 1], &steps[i]);
        let (from_states, to_states) = (states(before), states(now));
        let same_key = before.key == now.key;
        let change = model.change(now.weight);
        let (stay, moved) = ((1.0 - change).ln(), change.ln());
        let minor = now.key.minor;
        let size = to_states.len();
        // The chords of the step before, read in the key of this one.
        let read: Vec<Option<Rel>> = from_states
            .iter()
            .map(|p| {
                if same_key {
                    Some(*p)
                } else {
                    let rel = Rel::of(p.in_key(before.key), now.key);
                    to_states.contains(&rel).then_some(rel)
                }
            })
            .collect();
        let mut next = vec![f64::NEG_INFINITY; size];
        let mut from = vec![0usize; size];
        for (k, to) in to_states.iter().enumerate() {
            let e = emission(now, *to);
            if e <= FORBIDDEN {
                next[k] = FORBIDDEN * 2.0;
                continue;
            }
            let begins = onset(now, *to);
            for (p, origin) in read.iter().enumerate() {
                let step_score = match origin {
                    Some(rel) if rel == to => stay,
                    Some(rel) => {
                        moved + begins + FLOW * model.transitions.log(minor, *rel, *to, size)
                    }
                    None => moved + begins + MODULATE,
                };
                let total = best[p] + step_score;
                if total > next[k] {
                    next[k] = total;
                    from[k] = p;
                }
            }
            next[k] += e;
        }
        best = next;
        back.push(from);
    }
    let mut state = (0..best.len())
        .max_by(|a, b| best[*a].total_cmp(&best[*b]))
        .unwrap_or(0);
    let mut path = vec![0usize; count];
    path[count - 1] = state;
    for i in (1..count).rev() {
        state = back[i - 1][state];
        path[i - 1] = state;
    }
    let mut harmony = Harmony::default();
    for (i, s) in steps.iter().enumerate() {
        let mut chord: Chord = states(s)[path[i]].in_key(s.key);
        if plan.picardy && plan.key.minor && i + 1 == count {
            chord = Chord::new(plan.key.tonic, Kind::Maj);
        }
        harmony.push(s.at.max(0), chord, s.key);
    }
    // The last chord holds from where it was reached.
    if plan.picardy
        && plan.key.minor
        && let Some(last) = harmony.spans.len().checked_sub(1)
        && last > 0
    {
        let minor_tonic = Chord::new(plan.key.tonic, Kind::Min);
        if harmony.spans[last - 1].chord == minor_tonic {
            harmony.spans[last - 1].chord = Chord::new(plan.key.tonic, Kind::Maj);
            harmony.spans.pop();
        }
    }
    harmony
}

/// Chords seen less often than this in the analyses are not part of the vocabulary.
const LEAST: f64 = 40.0;

/// Key profiles after Temperley (2001), major and minor from the tonic upwards.
const PROFILE_MAJOR: [f64; 12] = [5.0, 2.0, 3.5, 2.0, 4.5, 4.0, 2.0, 4.5, 2.0, 3.5, 1.5, 4.0];
const PROFILE_MINOR: [f64; 12] = [5.0, 2.0, 3.5, 4.5, 2.0, 4.0, 2.0, 4.5, 3.5, 2.0, 1.5, 4.0];

/// The key that fits a group of tones best, among the keys near `home` on the circle of
/// fifths; `home` itself has a small advantage.
pub fn key_of(notes: &[Note], home: Key) -> Key {
    let mut weight = [0.0f64; 12];
    for note in notes {
        weight[note.pitch.pc() as usize] += note.dur() as f64;
    }
    let total: f64 = weight.iter().sum::<f64>().max(1.0);
    let centred = |profile: [f64; 12]| -> [f64; 12] {
        let mean = profile.iter().sum::<f64>() / 12.0;
        let c = profile.map(|p| p - mean);
        let norm = c.iter().map(|p| p * p).sum::<f64>().sqrt();
        c.map(|p| p / norm)
    };
    let (major, minor) = (centred(PROFILE_MAJOR), centred(PROFILE_MINOR));
    let signature = home.signature();
    let mut best = (f64::MIN, home);
    for shift in -2i8..=2 {
        for is_minor in [false, true] {
            // The major key with this signature, or its relative minor.
            let tonic = crate::pitch::Lof(signature + shift + if is_minor { 3 } else { 0 });
            let key = Key {
                tonic,
                minor: is_minor,
            };
            let profile = if is_minor { &minor } else { &major };
            let fit: f64 = (0..12)
                .map(|pc| weight[pc] * profile[(pc + 12 - tonic.pc() as usize) % 12])
                .sum::<f64>()
                / total;
            let score = fit + if key == home { 0.02 } else { 0.0 };
            if score > best.0 {
                best = (score, key);
            }
        }
    }
    best.1
}

/// The keys of a plan from its entries and cadences: every entry stands in the key that fits
/// its tones (the first in the home key), the stretch before an entry in the key of that entry,
/// the bar before a cadence in the key of the cadence, the end in the home key.
pub fn key_plan(
    entries: &[Vec<Note>],
    home: Key,
    meter: Meter,
    origin: Tick,
    end: Tick,
    cadences: &[Cadence],
) -> Vec<(Tick, Key)> {
    let mut entries: Vec<&Vec<Note>> = entries.iter().filter(|e| !e.is_empty()).collect();
    entries.sort_by_key(|e| e[0].on);
    let mut keys: Vec<(Tick, Key)> = Vec::new();
    let mut from = origin.min(0);
    for (k, entry) in entries.iter().enumerate() {
        let key = if k == 0 { home } else { key_of(entry, home) };
        // The stretch since the entry before belongs to this one; entries that overlap change
        // key where the later one begins.
        keys.push((from.min(entry[0].on), key));
        from = from.max(entry.last().map_or(from, |n| n.off));
    }
    keys.push((from, home));
    keys.sort_by_key(|(t, _)| *t);
    // Cadences: their key from the bar before them to their arrival.
    let bar = meter.bar();
    let mut cadences: Vec<&Cadence> = cadences.iter().collect();
    cadences.sort_by_key(|c| c.at);
    for cadence in cadences {
        let picardy = home.minor && !cadence.key.minor && cadence.key.tonic == home.tonic;
        let key = if picardy { home } else { cadence.key };
        let (a, b) = (cadence.at - bar, cadence.at + 1);
        // The key that was in force at the end of the stretch comes back after it.
        let after = keys
            .iter()
            .rev()
            .find(|(t, _)| *t <= b)
            .map_or(home, |(_, k)| *k);
        keys.retain(|(t, _)| *t < a || *t > b);
        keys.push((a, key));
        keys.push((b, after));
        keys.sort_by_key(|(t, _)| *t);
    }
    let _ = end;
    keys
}

/// What a scaffold looks like in numbers (Z8 of the protocol of B2): the shares of major, minor
/// and diminished triads, of dominant sevenths and of other sevenths in the sounding time, and
/// the changes of chord per beat.
pub fn profile(harmony: &Harmony, meter: Meter, end: Tick) -> [f64; 6] {
    let mut time = [0.0f64; 5];
    let mut total = 0.0;
    for (i, span) in harmony.spans.iter().enumerate() {
        let until = harmony.spans.get(i + 1).map_or(end, |n| n.on);
        let length = (until - span.on).max(0) as f64;
        let class = match span.chord.kind {
            Kind::Maj => 0,
            Kind::Min => 1,
            Kind::Dim => 2,
            Kind::Dom7 => 3,
            Kind::Aug => continue,
            _ => 4,
        };
        time[class] += length;
        total += length;
    }
    let first = harmony.spans.first().map_or(0, |s| s.on);
    let beats = ((end - first) as f64 / meter.beat() as f64).max(1.0);
    let changes = harmony
        .spans
        .windows(2)
        .filter(|w| w[0].chord.mask() != w[1].chord.mask())
        .count() as f64;
    let share = |k: usize| if total > 0.0 { time[k] / total } else { 0.0 };
    [
        share(0),
        share(1),
        share(2),
        share(3),
        share(4),
        changes / beats,
    ]
}

pub const PROFILE_NAMES: [&str; 6] = [
    "Durdreiklang",
    "Molldreiklang",
    "verminderter Dreiklang",
    "Dominantseptakkord",
    "andere Septakkorde",
    "Akkordwechsel je Schlag",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::Pitch;

    fn learned() -> Model {
        let text = "Time Signature: 4/4\n\
            m1 C: I b2 IV b3 V7 b4 I\n\
            m2 I b2 ii6 b3 V b4 I\n\
            m3 I b2 vi b3 ii b4 V7\n\
            m4 I b2 IV b3 V b4 I\n\
            m5 I b2 V6 b3 I b4 IV\n\
            m6 V7 b2 I b3 ii b4 V\n\
            m7 I b2 vi b3 IV b4 V\n\
            m8 I b3 iii b4 vi\n\
            m9 ii b2 V b3 I b4 viio\n\
            m10 I\n";
        let mut model = Model::default();
        let analysis = roman::parse(text).unwrap();
        // Often enough for the vocabulary.
        for _ in 0..40 {
            model.learn(&analysis, true);
        }
        model
    }

    #[test]
    fn a_model_knows_where_chords_change() {
        let model = learned();
        // In the analysis the chord changes on every beat and never between.
        assert!(model.change(2) > 0.9);
        assert!(model.change(1) < 0.1);
        let again = Model::from_json(&model.to_json());
        assert_eq!(again.changes, model.changes);
        assert_eq!(again.transitions.size(), model.transitions.size());
    }

    #[test]
    fn a_plan_gets_its_cadence_and_a_sequence() {
        let model = learned();
        let key = Key::parse("C").unwrap();
        // One bar of a given line (C E G E in quarters), then four free bars, a cadence in G at
        // the start of bar 4 and the end in C.
        let line: Vec<Note> = [(0, 4), (2, 4), (4, 4), (2, 4)]
            .iter()
            .enumerate()
            .map(|(i, (step, octave))| Note {
                on: 24 * i as Tick,
                off: 24 * i as Tick + 24,
                pitch: Pitch::new(*step, 0, *octave),
            })
            .collect();
        let g = Key::parse("G").unwrap();
        let plan = Plan {
            key,
            meter: Meter::COMMON,
            origin: 0,
            end: 96 * 6,
            lines: vec![line],
            keys: vec![(0, key), (96, g), (96 * 4, key)],
            cadences: vec![
                Cadence {
                    at: 96 * 3,
                    key: g,
                    kind: CadenceKind::Perfect,
                },
                Cadence {
                    at: 96 * 5,
                    key,
                    kind: CadenceKind::Perfect,
                },
            ],
            pedals: Vec::new(),
            must: Vec::new(),
            picardy: true,
            close: true,
            hold: None,
            lowest: Vec::new(),
            seed: 1,
        };
        let harmony = generate(&plan, &model);
        let at = |t: Tick| harmony.chord_at(t).unwrap().to_string();
        assert_eq!(at(0), "C");
        // D7 or D before G at bar 4, G7 or G before C at bar 6.
        assert!(at(96 * 3 - 1).starts_with('D'), "{}", at(96 * 3 - 1));
        assert_eq!(at(96 * 3), "G");
        assert!(at(96 * 5 - 1).starts_with('G'));
        assert_eq!(at(96 * 5), "C");
        assert_eq!(at(96 * 6 - 1), "C");
        let numbers = profile(&harmony, Meter::COMMON, plan.end);
        assert!(numbers[0] > 0.3 && numbers[5] > 0.1, "{numbers:?}");
        assert_eq!(
            triad(4, Key::parse("c").unwrap()),
            Rel::new(4, 0, Kind::Maj)
        );
        assert_eq!(triad(1, Key::parse("c").unwrap()).kind, Kind::Dim);
        assert!(is_dominant(
            Rel::new(6, 1, Kind::Dim7),
            Key::parse("c").unwrap()
        ));
    }
}
