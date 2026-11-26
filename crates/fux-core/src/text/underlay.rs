//! Text underlay: which syllable of the text each note of a singing voice begins.
//!
//! One model, [`Model`], decides every case of the TXT rules (docs/regeln.md § 17) for one
//! voice. It walks the voice event by event ([`Event`]: an attack, a tie continuation, a rest,
//! the end of the complete voice) and keeps a small state ([`UState`]): the syllable sung, how it
//! began and what is still undecided about it. At an attack the voice holds its syllable or
//! begins another one ([`Choice`]). Each step reports the cases decided at that event
//! ([`Found`]), at the earliest tick at which the notes show them: the cases of a prefix of a
//! voice are exactly the cases of the whole voice up to the prefix's last tick. The model reads
//! no profile.
//!
//! Three users price the same cases with the same [`Levels`]:
//! - the checker runs the model along a score's own underlay ([`Model::cases`]);
//! - the search carries, per singing voice, the set of states its notes allow and cuts what the
//!   profile forbids or what can no longer finish in time ([`Automaton`], [`Capacity`]);
//! - after the search a dynamic program finds the cheapest underlay of each voice
//!   ([`Model::best`]).
//!
//! Some cases cannot arise in the search, because the automaton never offers them: a voice
//! always enters with a new syllable (TXT-ENTRY), sings the text in order (TXT-ORDER) and ends on
//! its last syllable (TXT-COMPLETE). The checker still reports them in imported scores.
//!
//! The entries of a point of imitation pin their heads ([`Pins`]): each of their attacks takes the
//! leader's choice there, so that every entry sings the same syllables on the same notes. The
//! automaton and the dynamic program keep only the pinned choice at such an attack
//! ([`Model::best_pinned`]), and the automaton also cuts the states that can no longer begin the
//! next pinned syllable in time ([`Model::to_reach`]). The planner lays the syllables of a head
//! with [`Model::best_prefix`].

use crate::generate::regular_cadences;
use crate::mode::ModeSpec;
use crate::pitch::{Pitch, Step};
use crate::rules::{Checker, Level};
use crate::score::{Cadence, Lyric, Note};
use crate::text::{Stop, SylFlags, Text, TextKind};
use crate::time::{Duration, Meter};

/// Notes shorter than a half are black. Syllables begin on the minim beats, a half apart.
const HALF: u32 = Duration::HALF.ticks();
const QUARTER: u32 = Duration::QUARTER.ticks();
const DOTTED_HALF: u32 = Duration::DOTTED_HALF.ticks();

/// The syllable of a state before the voice's first syllable.
const NONE: u16 = u16::MAX;

/// Attacks are counted up to this many per syllable, so that every limit of TXT-MELISMA `long`
/// a profile can set below it counts.
const MAX_ATTACKS: u8 = u8::MAX;

/// Restatements are counted up to this many per sentence.
const MAX_RESTARTS: u8 = 3;

/// The most states the search keeps per singing voice.
pub const LIMIT: usize = 128;

/// A case of a TXT rule: the rule and the parameter whose level it carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Case {
    /// TXT-ONSET `level`: a new syllable on an eighth, on a white note or a quarter (except the
    /// dactyl quarter) off the minim beats, on a rest or on a tie continuation.
    Onset,
    /// TXT-ONSET `run_upbeat`: a syllable on a quarter on beat 2 that starts a run there.
    OnsetRunUpbeat,
    /// TXT-ONSET `single`: a texted quarter on a minim beat without a run after it, a texted
    /// dactyl quarter without a half or longer after it.
    OnsetSingle,
    /// TXT-ONSET `figure`: an unaccented syllable on the dactyl quarter that neither opens a part
    /// of the text nor ends a unit (Dó-mi-ne).
    OnsetFigure,
    /// TXT-ONSET `figure_accent`: any other syllable on the dactyl quarter: an accented one, one
    /// that opens a part of the text (a restatement too) or the last of a unit.
    OnsetFigureAccent,
    /// TXT-ONSET `opening_black`: a syllable that opens a part of the text (a restatement too)
    /// on a black note on a minim beat, or on a black note after a rest; not at the voice's first
    /// entry.
    OnsetOpeningBlack,
    /// TXT-AFTER-BLACK `level`: a new syllable right after a black sounding.
    AfterBlack,
    /// TXT-AFTER-BLACK `figure`: the same after the texted dactyl quarter.
    AfterBlackFigure,
    /// TXT-AFTER-BLACK `unit_end`: the same for the last syllable of a unit on a half or longer.
    AfterBlackUnitEnd,
    /// TXT-AFTER-BLACK `dotted`: the same on a half or longer right after the dactyl quarter
    /// where that quarter kept the syllable before it.
    AfterBlackDotted,
    /// TXT-REPEATED-PITCH `level`: a pitch struck again after a white sounding without a new
    /// syllable.
    RepeatedPitch,
    /// TXT-ENTRY `level`: an entry without a new syllable.
    Entry,
    /// TXT-ORDER `level`: a syllable out of the order of the text.
    Order,
    /// TXT-ORDER `first`: the voice does not begin with the first syllable.
    OrderFirst,
    /// TXT-REPEAT `short`: a restatement in a short text.
    RepeatShort,
    /// TXT-REPEAT `long`: a restatement in a long text, except of its last unit where the text
    /// ends; in a text that goes on in the next part ([`Text::continued`]) of that one too.
    RepeatLong,
    /// TXT-REPEAT `again`: a restatement beyond the first within a sentence.
    RepeatAgain,
    /// TXT-REPEAT `word`: a restatement of lone words, units of one word each that make no
    /// sentence together (San-ctus, San-ctus).
    RepeatWord,
    /// TXT-REST `word`: a rest inside a word.
    RestWord,
    /// TXT-REST `short`: a rest, or the voice's end, right after a black sounding.
    RestShort,
    /// TXT-REST `clause`: a rest after a word end that is neither a unit end nor a marked break,
    /// with two words or more left before the words stop.
    RestClause,
    /// TXT-REST `last_word`: the same where only one word is left before the words stop, which
    /// the voice then sings alone after the rest (Laudámus | te.).
    RestLastWord,
    /// TXT-REST `comma`: a rest after a unit end that ends no sentence, or at a marked break.
    RestComma,
    /// TXT-REST `long`: silence longer than a half although no sentence ended.
    RestLong,
    /// TXT-COMPLETE `level`: the last note does not carry the last syllable of the text.
    Complete,
    /// TXT-MELISMA `unstressed`: a second attack on an unaccented syllable.
    MelismaUnstressed,
    /// TXT-MELISMA `function_word`: a second attack on a function word.
    MelismaFunctionWord,
    /// TXT-MELISMA `last_syllable`: several attacks on the last syllable of a unit.
    MelismaLastSyllable,
    /// TXT-MELISMA `long` in a short text: an attack beyond `max_short` on a syllable of none of
    /// the classes below (an accented one).
    MelismaLongShort,
    /// TXT-MELISMA `long` in a long text: the same beyond `max_long`.
    MelismaLongLong,
    /// TXT-MELISMA `long` on an unaccented syllable that neither opens a part of the text nor
    /// ends a unit: an attack beyond `max_weak`.
    MelismaLongWeak,
    /// TXT-MELISMA `long` on an unaccented syllable that opens a part of the text and ends no
    /// unit (the e- of eleison): an attack beyond `max_opening`.
    MelismaLongOpening,
    /// TXT-MELISMA `long` on the last syllable of a unit, once its melisma is no lead-in: an
    /// attack beyond `max_last`.
    MelismaLongLast,
    /// TXT-ACCENT `short_penult`: a second attack on a short penult.
    AccentShortPenult,
    /// TXT-ACCENT `longer`: an unaccented syllable longer than the stressed one next to it.
    AccentLonger,
    /// TXT-ACCENT `opening`: the same for an opening syllable.
    AccentOpening,
    /// TXT-ACCENT `weak`: a stressed syllable shorter than its unaccented neighbour and off the
    /// downbeat, while that one is on it.
    AccentWeak,
    /// TXT-LEAP `octave`: a syllable inside a word after an octave (or a larger leap).
    LeapOctave,
    /// TXT-LEAP `sixth`.
    LeapSixth,
    /// TXT-LEAP `fifth`.
    LeapFifth,
    /// TXT-LEAP `fourth`.
    LeapFourth,
    /// TXT-CADENCE `level`: an internal cadence that neither ends a unit nor comes before a
    /// marked break.
    Cadence,
    /// TXT-CADENCE `principal`: one that ends only an intermediate unit, or comes before a
    /// marked break, on a principal degree.
    CadencePrincipal,
    /// TXT-CADENCE `comma`: the same on another degree.
    CadenceComma,
}

impl Case {
    /// The number of cases.
    pub const COUNT: usize = 44;

    /// All cases, in the order of the rules and their parameters.
    pub const ALL: [Case; Case::COUNT] = [
        Case::Onset,
        Case::OnsetRunUpbeat,
        Case::OnsetSingle,
        Case::OnsetFigure,
        Case::OnsetFigureAccent,
        Case::OnsetOpeningBlack,
        Case::AfterBlack,
        Case::AfterBlackFigure,
        Case::AfterBlackUnitEnd,
        Case::AfterBlackDotted,
        Case::RepeatedPitch,
        Case::Entry,
        Case::Order,
        Case::OrderFirst,
        Case::RepeatShort,
        Case::RepeatLong,
        Case::RepeatAgain,
        Case::RepeatWord,
        Case::RestWord,
        Case::RestShort,
        Case::RestClause,
        Case::RestLastWord,
        Case::RestComma,
        Case::RestLong,
        Case::Complete,
        Case::MelismaUnstressed,
        Case::MelismaFunctionWord,
        Case::MelismaLastSyllable,
        Case::MelismaLongShort,
        Case::MelismaLongLong,
        Case::MelismaLongWeak,
        Case::MelismaLongOpening,
        Case::MelismaLongLast,
        Case::AccentShortPenult,
        Case::AccentLonger,
        Case::AccentOpening,
        Case::AccentWeak,
        Case::LeapOctave,
        Case::LeapSixth,
        Case::LeapFifth,
        Case::LeapFourth,
        Case::Cadence,
        Case::CadencePrincipal,
        Case::CadenceComma,
    ];

    /// The id of the rule.
    pub const fn rule(self) -> &'static str {
        use Case::*;
        match self {
            Onset | OnsetRunUpbeat | OnsetSingle | OnsetFigure | OnsetFigureAccent
            | OnsetOpeningBlack => "TXT-ONSET",
            AfterBlack | AfterBlackFigure | AfterBlackUnitEnd | AfterBlackDotted => {
                "TXT-AFTER-BLACK"
            }
            RepeatedPitch => "TXT-REPEATED-PITCH",
            Entry => "TXT-ENTRY",
            Order | OrderFirst => "TXT-ORDER",
            RepeatShort | RepeatLong | RepeatAgain | RepeatWord => "TXT-REPEAT",
            RestWord | RestShort | RestClause | RestLastWord | RestComma | RestLong => "TXT-REST",
            Complete => "TXT-COMPLETE",
            MelismaUnstressed | MelismaFunctionWord | MelismaLastSyllable | MelismaLongShort
            | MelismaLongLong | MelismaLongWeak | MelismaLongOpening | MelismaLongLast => {
                "TXT-MELISMA"
            }
            AccentShortPenult | AccentLonger | AccentOpening | AccentWeak => "TXT-ACCENT",
            LeapOctave | LeapSixth | LeapFifth | LeapFourth => "TXT-LEAP",
            Cadence | CadencePrincipal | CadenceComma => "TXT-CADENCE",
        }
    }

    /// The level parameter of the rule that sets the case's level.
    pub const fn param(self) -> &'static str {
        use Case::*;
        match self {
            Onset | AfterBlack | RepeatedPitch | Entry | Order | Complete | Cadence => "level",
            OnsetRunUpbeat => "run_upbeat",
            OnsetSingle => "single",
            OnsetFigure | AfterBlackFigure => "figure",
            OnsetFigureAccent => "figure_accent",
            OnsetOpeningBlack => "opening_black",
            AfterBlackUnitEnd => "unit_end",
            AfterBlackDotted => "dotted",
            OrderFirst => "first",
            RepeatShort | RestShort => "short",
            RepeatLong | RestLong | MelismaLongShort | MelismaLongLong | MelismaLongWeak
            | MelismaLongOpening | MelismaLongLast => "long",
            RepeatAgain => "again",
            RepeatWord | RestWord => "word",
            RestClause => "clause",
            RestLastWord => "last_word",
            RestComma => "comma",
            MelismaUnstressed => "unstressed",
            MelismaFunctionWord => "function_word",
            MelismaLastSyllable => "last_syllable",
            AccentShortPenult => "short_penult",
            AccentLonger => "longer",
            AccentOpening => "opening",
            AccentWeak => "weak",
            LeapOctave => "octave",
            LeapSixth => "sixth",
            LeapFifth => "fifth",
            LeapFourth => "fourth",
            CadencePrincipal => "principal",
            CadenceComma => "comma",
        }
    }

    /// The integer parameter of the rule beyond which a case counted per attack costs.
    pub const fn limit(self) -> Option<&'static str> {
        match self {
            Case::MelismaLongShort => Some("max_short"),
            Case::MelismaLongLong => Some("max_long"),
            Case::MelismaLongWeak => Some("max_weak"),
            Case::MelismaLongOpening => Some("max_opening"),
            Case::MelismaLongLast => Some("max_last"),
            _ => None,
        }
    }
}

/// A case found in a voice: the note it marks and the tick at which it is decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Found {
    pub case: Case,
    /// Index of the marked note in the voice.
    pub note: usize,
    pub tick: u32,
    /// The number of the attack for TXT-MELISMA `long` (up to 255); otherwise 0.
    pub value: u8,
}

impl Found {
    fn at(case: Case, note: usize, tick: u32) -> Found {
        Found {
            case,
            note,
            tick,
            value: 0,
        }
    }

    /// The id of its rule.
    pub fn rule(&self) -> &'static str {
        self.case.rule()
    }

    /// The level parameter of its rule.
    pub fn param(&self) -> &'static str {
        self.case.param()
    }
}

/// The levels a profile gives the cases, and the limits of TXT-MELISMA `long`. The checker, the
/// automaton and the dynamic program price cases only through these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Levels {
    levels: [Level; Case::COUNT],
    max_short: u8,
    max_long: u8,
    max_weak: u8,
    max_opening: u8,
    max_last: u8,
}

impl From<&Checker> for Levels {
    /// The levels of the profile a checker was built with.
    fn from(checker: &Checker) -> Levels {
        Levels::new(
            |rule, param| checker.level(rule, param),
            |rule, param| checker.int(rule, param),
        )
    }
}

impl Levels {
    /// The levels from lookups of a level parameter and an integer parameter of a rule.
    pub fn new(level: impl Fn(&str, &str) -> Level, int: impl Fn(&str, &str) -> i64) -> Levels {
        let limit = |param| int("TXT-MELISMA", param).clamp(0, i64::from(u8::MAX)) as u8;
        Levels {
            levels: Case::ALL.map(|case| level(case.rule(), case.param())),
            max_short: limit("max_short"),
            max_long: limit("max_long"),
            max_weak: limit("max_weak"),
            max_opening: limit("max_opening"),
            max_last: limit("max_last"),
        }
    }

    /// The level of a case.
    pub fn of(&self, case: Case) -> Level {
        self.levels[case as usize]
    }

    /// The most attacks a syllable takes at no cost of TXT-MELISMA `long`, by its case.
    fn limit(&self, case: Case) -> u8 {
        match case {
            Case::MelismaLongLong => self.max_long,
            Case::MelismaLongWeak => self.max_weak,
            Case::MelismaLongOpening => self.max_opening,
            Case::MelismaLongLast => self.max_last,
            _ => self.max_short,
        }
    }

    /// The level of a found case: its parameter's, for a case counted per attack only beyond the
    /// limit.
    pub fn level(&self, found: &Found) -> Level {
        if found.case.limit().is_some() && found.value <= self.limit(found.case) {
            Level::Off
        } else {
            self.of(found.case)
        }
    }

    /// The cost of the cases and the rule of the first forbidden one, if any.
    pub fn price(&self, found: &[Found]) -> (u32, Option<&'static str>) {
        let mut cost = 0;
        for f in found {
            match self.level(f) {
                Level::Off => {}
                Level::Cost(c) => cost += c,
                Level::Forbidden => return (cost, Some(f.rule())),
            }
        }
        (cost, None)
    }

    /// Attacks beyond this many on one syllable behave alike for every later case, in a text of
    /// this kind: beyond every limit that can apply to one of its syllables.
    fn attack_bound(&self, kind: TextKind) -> u8 {
        let case = match kind {
            TextKind::Short => Case::MelismaLongShort,
            TextKind::Long => Case::MelismaLongLong,
        };
        // The cases of `long` share its level.
        if self.of(case) == Level::Off {
            return 2;
        }
        let most = [
            self.limit(case),
            self.max_weak,
            self.max_opening,
            self.max_last,
        ]
        .into_iter()
        .max()
        .unwrap_or(0);
        most.saturating_add(1).clamp(2, MAX_ATTACKS)
    }
}

/// The sounding that an event follows directly, tied notes merged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prev {
    /// Index of its first note.
    pub note: usize,
    pub start: u32,
    /// Its length in ticks, ties included.
    pub len: u32,
    pub pitch: Pitch,
    /// It is the dactyl quarter: a quarter right after a sounding of exactly a dotted half that
    /// began on a minim beat.
    pub dactyl: bool,
}

impl Prev {
    /// Shorter than a half.
    pub fn black(&self) -> bool {
        self.len < HALF
    }
}

/// The first written note of a sounding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attack {
    pub note: usize,
    pub start: u32,
    /// Its written value in ticks: whether a tie continues it is not known yet.
    pub written: u32,
    pub pitch: Pitch,
    /// The voice's first attack, or its first after a rest.
    pub entry: bool,
    /// The sounding just before it, unless a rest lies between.
    pub prev: Option<Prev>,
}

/// A tie continuation: the sounding before it goes on, at least by this note's written value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    pub note: usize,
    pub start: u32,
    /// Its written value in ticks: whether a tie continues it too is not known yet.
    pub written: u32,
}

/// A rest after the voice's first note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rest {
    pub note: usize,
    pub start: u32,
    pub written: u32,
    /// The silence up to the end of this rest: its value and those of the rests before it.
    pub silence: u32,
    /// The sounding just before it, for the first rest of a silence.
    pub prev: Option<Prev>,
}

/// The end of a complete voice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct End {
    /// The voice's last sounding.
    pub last: Prev,
    /// Where the voice ends.
    pub end: u32,
    /// The last sounding lasts to the end: no rest follows it.
    pub touching: bool,
}

/// What happens in a voice at one of its notes, as far as the text is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Attack(Attack),
    /// A tie continuation: the syllable lasts longer, which may already decide TXT-ACCENT.
    Held(Held),
    Rest(Rest),
    End(End),
}

impl Event {
    /// The event of note `i`, which begins at `start`; none for a rest before the voice's first
    /// note. Reads the notes up to `i` only, and never the tie of note `i`: the search decides it
    /// later.
    pub fn at(notes: &[Note], i: usize, start: u32) -> Option<Event> {
        let note = &notes[i];
        let written = note.duration.ticks();
        match note.pitch {
            None => {
                let mut first = i;
                let mut silence = written;
                while first > 0 && notes[first - 1].is_rest() {
                    first -= 1;
                    silence += notes[first].duration.ticks();
                }
                if first == 0 {
                    return None;
                }
                let prev = if first == i {
                    sounding_before(notes, i, start)
                } else {
                    None
                };
                Some(Event::Rest(Rest {
                    note: i,
                    start,
                    written,
                    silence,
                    prev,
                }))
            }
            Some(pitch) => {
                if i > 0 && notes[i - 1].tie && notes[i - 1].pitch == Some(pitch) {
                    return Some(Event::Held(Held {
                        note: i,
                        start,
                        written,
                    }));
                }
                Some(Event::Attack(Attack {
                    note: i,
                    start,
                    written,
                    pitch,
                    entry: i == 0 || notes[i - 1].is_rest(),
                    prev: sounding_before(notes, i, start),
                }))
            }
        }
    }

    /// The event of the voice's last written note, if it makes one: also a tie continuation,
    /// whatever its own tie says.
    pub fn of_last(notes: &[Note]) -> Option<Event> {
        let last = notes.last()?;
        let end: u32 = notes.iter().map(|n| n.duration.ticks()).sum();
        Event::at(notes, notes.len() - 1, end - last.duration.ticks())
    }

    /// The end of a complete voice; `None` if it never sounds.
    pub fn end(notes: &[Note]) -> Option<Event> {
        let total: u32 = notes.iter().map(|n| n.duration.ticks()).sum();
        let mut after = notes.len();
        let mut end = total;
        while after > 0 && notes[after - 1].is_rest() {
            after -= 1;
            end -= notes[after].duration.ticks();
        }
        let last = sounding_before(notes, after, end)?;
        Some(Event::End(End {
            last,
            end: total,
            touching: after == notes.len(),
        }))
    }

    /// The events of all notes of a voice, with its end if it is complete.
    pub fn all(notes: &[Note], complete: bool) -> Vec<Event> {
        let mut events = Vec::with_capacity(notes.len() + 1);
        let mut start = 0;
        for (i, note) in notes.iter().enumerate() {
            events.extend(Event::at(notes, i, start));
            start += note.duration.ticks();
        }
        if complete {
            events.extend(Event::end(notes));
        }
        events
    }

    /// The tick of the event: where the note begins, or where the voice ends.
    pub fn start(&self) -> u32 {
        match self {
            Event::Attack(a) => a.start,
            Event::Held(h) => h.start,
            Event::Rest(r) => r.start,
            Event::End(e) => e.end,
        }
    }
}

/// The sounding that ends where note `i` begins, at `start`, if a note ends there.
fn sounding_before(notes: &[Note], i: usize, start: u32) -> Option<Prev> {
    let (note, from, pitch) = ending_at(notes, i, start)?;
    let len = start - from;
    let dactyl = len == QUARTER
        && ending_at(notes, note, from).is_some_and(|(_, before, _)| {
            from - before == DOTTED_HALF && before.is_multiple_of(HALF)
        });
    Some(Prev {
        note,
        start: from,
        len,
        pitch,
        dactyl,
    })
}

/// The sounding that ends where note `i` begins, at `start`, if a note ends there: its first
/// note, where it begins and its pitch.
fn ending_at(notes: &[Note], i: usize, start: u32) -> Option<(usize, u32, Pitch)> {
    let last = i.checked_sub(1)?;
    let pitch = notes[last].pitch?;
    let mut first = last;
    let mut from = start - notes[last].duration.ticks();
    while first > 0 && notes[first - 1].tie && notes[first - 1].pitch == Some(pitch) {
        first -= 1;
        from -= notes[first].duration.ticks();
    }
    Some((first, from, pitch))
}

/// What a voice does at an attack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Choice {
    /// Continues its syllable: a melisma.
    Hold,
    /// Begins the syllable with this index.
    Start(u16),
}

/// The underlay choices pinned in a voice as (tick, choice), sorted by tick, one per attack: the
/// voice's attack at that tick takes exactly that choice. The entries of a point of imitation pin
/// their heads, every entry with the leader's choices.
pub type Pins = [(u32, Choice)];

/// The choice pinned at an event: only an attack has one.
fn pinned(pins: Option<&Pins>, event: &Event) -> Option<Choice> {
    let (Some(pins), Event::Attack(a)) = (pins, event) else {
        return None;
    };
    pins.binary_search_by_key(&a.start, |&(tick, _)| tick)
        .ok()
        .map(|i| pins[i].1)
}

/// The syllables a voice may not begin before a tick: for each point of an imitation after the
/// first that sets words of its own, its first own syllable and the tick from which the voice
/// takes them up (its entry in the point, or the point's second entry,
/// [`crate::score::ScoreImitation::gates`]), in the order of the text, where neither falls from
/// one gate to the next. A syllable from a gate's on begins at the gate's tick or later: a voice
/// does not sing the words of a later point before the point takes it in; until then it sings
/// the words before them, again if it likes, or rests (Jeppesen, Counterpoint, p. 241: each line
/// of the text has its own theme, which is imitated before the next portion of the text comes).
pub type Gates = [(u16, u32)];

/// The syllables after which a voice of an imitation may begin its sentence again although they
/// end no sense unit ([`Model::with_restarts`]): the last syllable before each point after the
/// first that sets words of its own, where that point begins within a unit, at a tail. `words`
/// are the syllables of the plan's points in order, as the gates count them
/// ([`crate::score::ScoreImitation::gates`]). A voice that has sung the words before such a point
/// and waits for the point's gate sings them again from their start, as Palestrina's voices do,
/// rather than holding their last syllable for bars (Be-ne-dí-ctus qui ve-nit / in nó-mi-ne
/// Dó-mi-ni.; docs/regeln.md § 18).
pub fn gate_restarts<'a>(
    text: &Text,
    words: impl IntoIterator<Item = &'a std::ops::Range<u16>>,
) -> Vec<u16> {
    let mut out = Vec::new();
    let mut set = 0;
    for (p, words) in words.into_iter().enumerate() {
        let own = words.start.max(set);
        set = set.max(words.end);
        if p == 0 || own >= words.end || own == 0 {
            continue;
        }
        let before = own - 1;
        let unit_end = text
            .syllables
            .get(usize::from(before))
            .is_some_and(|s| s.flags.contains(SylFlags::UNIT_END));
        if !unit_end && !out.contains(&before) {
            out.push(before);
        }
    }
    out
}

/// The tick before which syllable `y` may not begin.
pub fn gate(gates: Option<&Gates>, y: u16) -> u32 {
    gates.map_or(0, |gates| {
        gates
            .iter()
            .filter(|&&(first, _)| first <= y)
            .map(|&(_, tick)| tick)
            .max()
            .unwrap_or(0)
    })
}

/// Whether `choice` at `event` begins a syllable before its gate.
fn early(gates: Option<&Gates>, event: &Event, choice: Choice) -> bool {
    match (event, choice) {
        (Event::Attack(a), Choice::Start(y)) => a.start < gate(gates, y),
        _ => false,
    }
}

/// The first pinned syllable start after `tick`: its tick and its syllable.
fn next_start(pins: &Pins, tick: u32) -> Option<(u32, u16)> {
    let after = pins.partition_point(|&(t, _)| t <= tick);
    pins[after..].iter().find_map(|&(t, choice)| match choice {
        Choice::Start(y) => Some((t, y)),
        Choice::Hold => None,
    })
}

/// The rule a state counts for in the diagnosis when an attack's pinned choice is not among its
/// choices: a melisma pinned at an entry, or a syllable out of the order of the text.
fn refused(pin: Choice) -> &'static str {
    match pin {
        Choice::Hold => Case::Entry.rule(),
        Choice::Start(_) => Case::Order.rule(),
    }
}

/// What is still undecided about a texted quarter: the next event decides whether it may carry
/// its syllable (TXT-ONSET `single`). Holds the quarter's note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Pending {
    None,
    /// A texted quarter on a minim beat: a black attack must follow, starting a run.
    Run(u32),
    /// The texted dactyl quarter: an attack written as a half or longer must follow.
    Dactyl(u32),
}

/// A syllable that has ended: how long it lasted, whether it began on a downbeat, its note, and
/// whether TXT-ACCENT `weak` has been reported on it, which happens once per syllable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Sung {
    len: u16,
    downbeat: bool,
    note: u32,
    weak: bool,
}

/// Where a voice stands in its text after some of its events: the state of the automaton.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UState {
    /// The syllable sung; `NONE` before the first.
    syl: u16,
    /// The first note and the start of the current syllable.
    note: u32,
    since: u32,
    /// Attacks on the current syllable, up to `MAX_ATTACKS`.
    attacks: u8,
    /// All attacks of the syllable before the latest were black, and the latest begins at most a
    /// measure after the syllable: a lead-in.
    lead_in: bool,
    /// The syllable has not ended: no rest since it began.
    open: bool,
    pending: Pending,
    /// While the syllable sounds, the syllable before it in its word, if the voice sang that
    /// one just before; once it has ended, the syllable itself, if its word goes on.
    sung: Option<Sung>,
    /// Restatements in the current sentence, up to `MAX_RESTARTS`.
    restarts: u8,
    /// The syllable sung is not in the text: an imported syllable that matches none
    /// ([`Lyric::UNKNOWN`]). `syl` stays the last syllable of the text sung before it. Only the
    /// checker's path ([`Model::cases`]) begins such a syllable.
    unknown: bool,
}

impl UState {
    /// Before the voice's first note.
    pub const START: UState = UState {
        syl: NONE,
        note: 0,
        since: 0,
        attacks: 0,
        lead_in: true,
        open: false,
        pending: Pending::None,
        sung: None,
        restarts: 0,
        unknown: false,
    };

    /// The syllable sung, by its index in the text; `None` before the first.
    pub fn syllable(&self) -> Option<usize> {
        (self.syl != NONE).then_some(usize::from(self.syl))
    }

    /// The state as one number, for sorting and merging sets quickly: one-to-one on canonical
    /// states ([`Model::canonical`]), whose notes are cleared.
    fn key(&self) -> u128 {
        let pending: u128 = match self.pending {
            Pending::None => 0,
            Pending::Run(_) => 1,
            Pending::Dactyl(_) => 2,
        };
        let sung = self.sung.map_or(0, |s| {
            1 | u128::from(s.downbeat) << 1 | u128::from(s.weak) << 2 | u128::from(s.len) << 3
        });
        u128::from(self.syl) << 112
            | u128::from(self.since) << 80
            | u128::from(self.attacks) << 72
            | u128::from(self.lead_in) << 71
            | u128::from(self.open) << 70
            | pending << 68
            | u128::from(self.restarts) << 64
            | u128::from(self.unknown) << 63
            | sung
    }
}

/// What the model needs to know about a syllable.
#[derive(Clone, Copy, Debug)]
struct Syl {
    word_start: bool,
    word_end: bool,
    unit_end: bool,
    /// The last syllable of a sentence.
    full_stop: bool,
    break_after: bool,
    stressed: bool,
    accented: bool,
    short_penult: bool,
    function_word: bool,
    opening: bool,
    /// It belongs to the text's last unit, and the words end there: a long text restates it
    /// freely, like an Amen. Not in a text that goes on in the next part ([`Text::continued`]).
    last_unit: bool,
    /// The first unit of its sentence.
    sentence: u16,
    /// The next syllable, it or a later one, after which the words stop: the last of its unit,
    /// or one before a marked break (`|`, or the words before a tail `/`) if that comes first.
    stop: u16,
    /// It ends a word before the words stop, and only one word follows up to the stop.
    before_last_word: bool,
    /// TXT-ACCENT compares it with the syllable before it in its word.
    pair_before: bool,
    /// TXT-ACCENT compares it with the syllable after it in its word.
    pair_after: bool,
}

/// Whether TXT-ACCENT compares two neighbouring syllables of a word: one is stressed, the other
/// unaccented.
fn compared(a: &Syl, b: &Syl) -> bool {
    (a.stressed && !b.accented) || (!a.accented && b.stressed)
}

/// What the onset of a syllable that is not in the text is judged by: a word of its own that
/// ends no unit. So it claims no licence of a unit's last syllable (TXT-AFTER-BLACK `unit_end`)
/// and no leap inside a word (TXT-LEAP); the cases of its value and position still count.
const UNMATCHED: Syl = Syl {
    word_start: true,
    word_end: true,
    unit_end: false,
    full_stop: false,
    break_after: false,
    stressed: false,
    accented: false,
    short_penult: false,
    function_word: false,
    opening: false,
    last_unit: false,
    sentence: 0,
    stop: 0,
    before_last_word: false,
    pair_before: false,
    pair_after: false,
};

/// A syllable that has ended, compared with its neighbour in the word.
#[derive(Clone, Copy)]
struct Span {
    syl: Syl,
    len: u16,
    downbeat: bool,
    note: u32,
    /// TXT-ACCENT `weak` has been reported on it.
    weak: bool,
}

/// An internal cadence: its target arrives at `tick`, the start of a measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Goal {
    pub tick: u32,
    pub target: Step,
}

impl Goal {
    /// The goals of a score's internal cadences ([`crate::score::Score::cadences`]) in a meter,
    /// as the checker's TXT rules take them.
    pub fn of(cadences: &[Cadence], meter: Meter) -> Vec<Goal> {
        cadences
            .iter()
            .map(|c| Goal {
                tick: c.measure as u32 * meter.ticks_per_measure(),
                target: c.target,
            })
            .collect()
    }
}

/// A goal as the model uses it.
#[derive(Clone, Copy, Debug)]
struct Arrival {
    tick: u32,
    /// The target in some octave, spelled in the mode.
    target: Pitch,
    /// The final or the first regular cadence degree of the mode.
    principal: bool,
}

/// The underlay model of one text: decides the cases of the TXT rules event by event.
#[derive(Clone, Debug)]
pub struct Model {
    syls: Vec<Syl>,
    /// Per syllable, the first syllable from it on after which a restatement may begin: the end
    /// of a unit, or a syllable before a later point of an imitation ([`Model::with_restarts`]).
    unit_end: Vec<u16>,
    /// Per syllable that ends a unit, and per syllable [`Model::with_restarts`] adds, where a
    /// restatement may begin.
    targets: Vec<Vec<u16>>,
    /// Per syllable that ends a unit, the targets from which a restatement sings lone words
    /// again (TXT-REPEAT `word`).
    lone: Vec<Vec<u16>>,
    kind: TextKind,
    per_measure: u32,
    goals: Vec<Arrival>,
}

impl Model {
    /// The model of `text` (which has a syllable, as every parsed text) in a score of this mode
    /// and meter, with its internal cadences.
    pub fn new(text: &Text, mode: ModeSpec, meter: Meter, goals: &[Goal]) -> Model {
        // A text that goes on in the next part has no last unit that ends the words.
        let last_unit = (!text.continued).then(|| text.units.len().saturating_sub(1));
        let mut syls: Vec<Syl> = text
            .syllables
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let flags = s.flags;
                Syl {
                    word_start: flags.contains(SylFlags::WORD_START),
                    word_end: flags.contains(SylFlags::WORD_END),
                    unit_end: flags.contains(SylFlags::UNIT_END),
                    full_stop: flags.contains(SylFlags::FULL_STOP),
                    break_after: flags.contains(SylFlags::BREAK_AFTER),
                    stressed: flags.contains(SylFlags::STRESS),
                    accented: flags.accented(),
                    short_penult: flags.contains(SylFlags::SHORT_PENULT),
                    function_word: flags.contains(SylFlags::FUNCTION_WORD),
                    opening: text.is_opening(i),
                    last_unit: Some(usize::from(s.unit)) == last_unit,
                    sentence: text.unit_of(i).sentence,
                    stop: text.unit_of(i).last,
                    before_last_word: false,
                    pair_before: false,
                    pair_after: false,
                }
            })
            .collect();
        for i in 1..syls.len() {
            if !syls[i].word_start {
                let pair = compared(&syls[i - 1], &syls[i]);
                syls[i].pair_before = pair;
                syls[i - 1].pair_after = pair;
            }
        }
        // A marked break stops the words before it within their unit.
        for i in (0..syls.len()).rev() {
            if syls[i].break_after {
                syls[i].stop = i as u16;
            } else if i + 1 < syls.len() && !syls[i].unit_end {
                syls[i].stop = syls[i].stop.min(syls[i + 1].stop);
            }
        }
        // The last syllable of each word, from the end.
        let mut word_last = vec![0; syls.len()];
        for i in (0..syls.len()).rev() {
            word_last[i] = if syls[i].word_end || i + 1 == syls.len() {
                i
            } else {
                word_last[i + 1]
            };
        }
        for i in 0..syls.len().saturating_sub(1) {
            let syl = syls[i];
            syls[i].before_last_word = syl.word_end
                && !syl.unit_end
                && !syl.break_after
                && word_last[i + 1] == usize::from(syl.stop);
        }
        // The text's last syllable ends its last unit.
        let mut unit_end = vec![0; syls.len()];
        let mut end = syls.len().saturating_sub(1);
        for i in (0..syls.len()).rev() {
            if syls[i].unit_end {
                end = i;
            }
            unit_end[i] = end as u16;
        }
        let targets: Vec<Vec<u16>> = (0..syls.len())
            .map(|i| text.restart_targets(i).map(|t| t as u16).collect())
            .collect();
        // Units of one word in which no tail begins.
        let one_word: Vec<bool> = text
            .units
            .iter()
            .map(|unit| {
                let range = unit.range();
                text.syllables[range.start].word == text.syllables[range.end - 1].word
                    && !range
                        .into_iter()
                        .any(|s| text.syllables[s].flags.contains(SylFlags::TAIL_START))
            })
            .collect();
        // A restatement from the end of unit u at the start of unit t sings lone words again if
        // the units t to u are of one word each, unless they make a whole sentence, whose sense
        // is complete (A-men.). A tail is restated at the level of its kind of text.
        let lone = targets
            .iter()
            .enumerate()
            .map(|(x, targets)| {
                let to = usize::from(text.syllables[x].unit);
                targets
                    .iter()
                    .copied()
                    .filter(|&y| {
                        let from = usize::from(text.syllables[usize::from(y)].unit);
                        let unit = &text.units[to];
                        let sentence =
                            from == usize::from(unit.sentence) && unit.stop == Stop::Full;
                        one_word[from..=to].iter().all(|&one| one) && !sentence
                    })
                    .collect()
            })
            .collect();
        let first = regular_cadences(mode).first().copied();
        let goals = goals
            .iter()
            .map(|g| Arrival {
                tick: g.tick,
                target: Pitch::new(g.target, mode.scale_alter(g.target), 4),
                principal: g.target == mode.final_step() || Some(g.target) == first,
            })
            .collect();
        Model {
            syls,
            unit_end,
            targets,
            lone,
            kind: text.kind,
            per_measure: meter.ticks_per_measure(),
            goals,
        }
    }

    /// The model with restatements after `after` too ([`gate_restarts`]), syllables of `text`
    /// that end no unit: from each, a voice may begin again at the starts of units and tails of
    /// its sentence up to it, as after the end of a unit; such a restatement costs as one there
    /// (TXT-REPEAT). Requests without imitation have none, so their underlay stays as it was.
    pub fn with_restarts(mut self, text: &Text, after: &[u16]) -> Model {
        for &x in after {
            let x = usize::from(x);
            if x >= self.syls.len() || !self.targets[x].is_empty() {
                continue;
            }
            let unit = text.unit_of(x);
            let from = usize::from(text.units[usize::from(unit.sentence)].first);
            self.targets[x] = (from..=x)
                .filter(|&i| {
                    text.syllables[i]
                        .flags
                        .intersects(SylFlags::UNIT_START | SylFlags::TAIL_START)
                })
                .map(|i| i as u16)
                .collect();
        }
        let mut end = self.syls.len().saturating_sub(1);
        for i in (0..self.syls.len()).rev() {
            if self.syls[i].unit_end || !self.targets[i].is_empty() {
                end = i;
            }
            self.unit_end[i] = end as u16;
        }
        self
    }

    /// The number of syllables of the text.
    pub fn syllables(&self) -> usize {
        self.syls.len()
    }

    /// Whether the text is short or long.
    pub fn kind(&self) -> TextKind {
        self.kind
    }

    /// How many syllables a voice in this state still has to begin, without restatements.
    pub fn need(&self, state: &UState) -> u32 {
        let total = self.syls.len() as u32;
        match state.syllable() {
            None => total,
            Some(x) => total - 1 - x as u32,
        }
    }

    /// As [`Model::need`], for a voice that sings to the end of the setting, as every voice of a
    /// free setting holds the last measure: where its text has ended and it has rested since, it
    /// must enter again with a restatement, the shortest of which runs from the latest start of
    /// a unit or a tail of the last sentence to the last syllable. A voice that cannot restate
    /// cannot end at all.
    pub fn need_sounding(&self, state: &UState) -> u32 {
        let last = self.syls.len() - 1;
        if state.syllable() != Some(last) || state.open {
            return self.need(state);
        }
        self.targets[last]
            .iter()
            .map(|&t| (last - usize::from(t)) as u32 + 1)
            .min()
            .unwrap_or(u32::MAX)
    }

    /// How many syllables a voice in this state still has to begin until its words stop at the
    /// end of a unit or before a marked break; 0 on such a syllable.
    pub fn to_stop(&self, state: &UState) -> u32 {
        match state.syllable() {
            None => u32::from(self.syls[0].stop) + 1,
            Some(x) => u32::from(self.syls[x].stop) - x as u32,
        }
    }

    /// How many syllables a voice in this state still has to begin before it can begin syllable
    /// `y`: straight on up to it, or once it has gone past it, the rest of its unit (up to the
    /// first syllable after which it may restate, [`Model::with_restarts`]) and then, from the
    /// latest start of a unit or tail up to `y` where a restatement from there may begin, the
    /// syllables up to `y`. A later place of restatement in the sentence offers no start up to
    /// `y` that this one does not, and no restatement leaves the sentence, so no way needs
    /// fewer. `None` if the voice can no longer begin `y`: it has gone past it into a later
    /// sentence.
    ///
    /// Each of these syllables needs a place of its own before the beat of `y` ([`Capacity`]),
    /// so a voice with fewer places left there cannot keep a pinned `Start(y)`.
    pub fn to_reach(&self, state: &UState, y: u16) -> Option<u32> {
        let y = usize::from(y);
        if y >= self.syls.len() {
            return None;
        }
        match state.syllable() {
            None => Some(y as u32),
            Some(x) if x < y => Some((y - 1 - x) as u32),
            Some(x) => {
                let u = usize::from(self.unit_end[x]);
                let t = self.targets[u]
                    .iter()
                    .map(|&t| usize::from(t))
                    .filter(|&t| t <= y)
                    .max()?;
                Some((u - x + y - t) as u32)
            }
        }
    }

    /// The syllables a voice in this state begins before it can begin syllable `y`, in the order
    /// it sings them, by the way [`Model::to_reach`] counts: straight on, or once it has gone
    /// past `y` the rest of its unit (up to its first place of restatement) and then from the
    /// latest start of a unit or tail up to `y`.
    /// For `y` one past the text's last syllable, the rest of the text. `None` if it can no longer
    /// begin `y`.
    pub fn way(&self, state: &UState, y: u16) -> Option<Vec<u16>> {
        let y = usize::from(y);
        let n = self.syls.len();
        let to = |from: usize, to: usize| (from..to).map(|s| s as u16);
        match state.syllable() {
            None => Some(to(0, y.min(n)).collect()),
            Some(x) if x < y => Some(to(x + 1, y.min(n)).collect()),
            Some(x) => {
                let u = usize::from(self.unit_end[x]);
                let t = self.targets[u]
                    .iter()
                    .map(|&t| usize::from(t))
                    .filter(|&t| t <= y)
                    .max()?;
                Some(to(x + 1, u + 1).chain(to(t, y)).collect())
            }
        }
    }

    /// The state of a voice after syllable `x` and a rest, before it enters again: the syllable
    /// has ended, nothing is pending, the voice has not restated in its sentence, and it keeps
    /// nothing of the syllable to compare the next one with, as after the end of a word
    /// (TXT-ACCENT compares syllables within a word only). Canonical under every profile
    /// ([`Model::canonical`]). Before the voice's first syllable (`None`), its start.
    pub fn after_rest(&self, x: Option<usize>) -> UState {
        match x {
            None => UState::START,
            Some(x) => {
                debug_assert!(x < self.syls.len(), "syllable {x} is not in the text");
                UState {
                    syl: x as u16,
                    ..UState::START
                }
            }
        }
    }

    /// The syllable sung, unless none has begun or it is not in the text.
    fn current(&self, state: &UState) -> Option<Syl> {
        match state.syllable() {
            Some(x) if !state.unknown => Some(self.syls[x]),
            _ => None,
        }
    }

    /// The choices at an event: before the first syllable only the first; at an entry the
    /// syllables that may follow; at any other attack also holding the syllable. The syllables
    /// that may follow are the next one and, after the last syllable of a unit, the starts of
    /// units and tails of its sentence up to that unit (after a syllable that
    /// [`Model::with_restarts`] adds, up to it). A rest and the end leave no choice: the one step
    /// they take is `Hold`.
    pub fn options(&self, state: &UState, event: &Event, out: &mut Vec<Choice>) {
        out.clear();
        let Event::Attack(attack) = event else {
            out.push(Choice::Hold);
            return;
        };
        match state.syllable() {
            None => out.push(Choice::Start(0)),
            Some(x) => {
                if !attack.entry {
                    out.push(Choice::Hold);
                }
                if x + 1 < self.syls.len() {
                    out.push(Choice::Start(x as u16 + 1));
                }
                out.extend(self.targets[x].iter().map(|&y| Choice::Start(y)));
            }
        }
    }

    /// The state after an event with a choice, reporting the cases decided there in this order:
    ///
    /// - at an attack: TXT-ONSET `single` of a quarter before; with `Hold` TXT-ENTRY,
    ///   TXT-REPEATED-PITCH and the melisma (TXT-ACCENT `short_penult`, TXT-MELISMA
    ///   `function_word` or `unstressed` at the second attack; TXT-MELISMA `last_syllable` after
    ///   the first note of a unit's last syllable that is not black, or once the syllable began
    ///   more than a measure before; TXT-ACCENT once the syllable outlasts the stressed one before
    ///   it; `long` at every attack from the second on); with `Start` the end of the syllable
    ///   before (TXT-ACCENT), TXT-ORDER, TXT-REPEAT, except at an entry TXT-ONSET or
    ///   TXT-AFTER-BLACK and TXT-LEAP (at an entry after a rest only TXT-ONSET `opening_black`),
    ///   then TXT-ACCENT if the new syllable's first note outlasts the stressed one before it;
    ///   with either TXT-CADENCE;
    /// - at a tie continuation: TXT-ACCENT once the syllable outlasts the stressed one before it;
    /// - at a rest: TXT-ONSET `single`; at its first, the end of the syllable and TXT-REST
    ///   `word`, `short`, `clause`, `last_word` or `comma`; TXT-REST `long`;
    /// - at the end: TXT-ONSET `single`, the end of the syllable, TXT-REST `short`,
    ///   TXT-COMPLETE.
    ///
    /// The written value of an attack or a tie continuation counts as soon as it begins: the
    /// syllable lasts at least that long.
    pub fn step(
        &self,
        state: &UState,
        event: &Event,
        choice: Choice,
        found: &mut Vec<Found>,
    ) -> UState {
        let mut t = *state;
        match event {
            Event::Attack(a) => {
                let after_dactyl = matches!(t.pending, Pending::Dactyl(_));
                self.resolve(&mut t, Some(a.written), a.start, found);
                match choice {
                    Choice::Hold => self.hold(&mut t, a, found),
                    Choice::Start(y) => {
                        self.begin(&mut t, y, Some((a, after_dactyl)), a.note, a.start, found);
                    }
                }
                self.cadence(&t, a, found);
            }
            // A texted quarter tied over stays undecided: the attack after the tie decides.
            Event::Held(h) => self.outlast(&mut t, h.start + h.written, h.start, found),
            Event::Rest(r) => {
                self.resolve(&mut t, None, r.start, found);
                // A sounding syllable that is not in the text ends neither a word nor a unit as
                // far as the model knows; only the value before the rest counts then.
                let current = self.current(&t);
                if t.open {
                    self.close(&mut t, r.start, found);
                    if current.is_some_and(|syl| !syl.word_end) {
                        found.push(Found::at(Case::RestWord, r.note, r.start));
                    }
                    if let Some(prev) = r.prev.filter(Prev::black) {
                        found.push(Found::at(Case::RestShort, prev.note, r.start));
                    }
                    // A rest inside a unit: one that leaves its last word alone costs more.
                    if let Some(syl) =
                        current.filter(|syl| syl.word_end && !syl.unit_end && !syl.break_after)
                    {
                        let case = if syl.before_last_word {
                            Case::RestLastWord
                        } else {
                            Case::RestClause
                        };
                        found.push(Found::at(case, r.note, r.start));
                    }
                    if current
                        .is_some_and(|syl| (syl.unit_end || syl.break_after) && !syl.full_stop)
                    {
                        found.push(Found::at(Case::RestComma, r.note, r.start));
                    }
                }
                if r.silence > HALF
                    && r.silence - r.written <= HALF
                    && current.is_some_and(|syl| !syl.full_stop)
                {
                    found.push(Found::at(Case::RestLong, r.note, r.start));
                }
            }
            Event::End(e) => {
                self.resolve(&mut t, None, e.end, found);
                if t.open {
                    self.close(&mut t, e.end, found);
                    if e.touching && e.last.black() {
                        found.push(Found::at(Case::RestShort, e.last.note, e.end));
                    }
                }
                if t.syllable() != Some(self.syls.len() - 1) {
                    found.push(Found::at(Case::Complete, e.last.note, e.last.start));
                }
            }
        }
        t
    }

    /// Decides a texted quarter at the next event: an attack of `written` ticks, or a rest or
    /// the end (`None`).
    fn resolve(&self, t: &mut UState, written: Option<u32>, at: u32, found: &mut Vec<Found>) {
        let quarter = match t.pending {
            Pending::Run(note) if !written.is_some_and(|w| w < HALF) => Some(note),
            Pending::Dactyl(note) if !written.is_some_and(|w| w >= HALF) => Some(note),
            _ => None,
        };
        if let Some(note) = quarter {
            found.push(Found::at(Case::OnsetSingle, note as usize, at));
        }
        t.pending = Pending::None;
    }

    /// An attack that continues the current syllable. After a rest (an entry without a syllable,
    /// on the checker's path only) the syllable has ended: the entry begins one that is not in
    /// the text, and the voice keeps its place in the text.
    fn hold(&self, t: &mut UState, a: &Attack, found: &mut Vec<Found>) {
        if a.entry {
            found.push(Found::at(Case::Entry, a.note, a.start));
        }
        if t.syl == NONE && !t.unknown {
            return;
        }
        if !t.open {
            *t = UState {
                note: a.note as u32,
                since: a.start,
                attacks: 1,
                lead_in: true,
                open: true,
                pending: Pending::None,
                sung: None,
                unknown: true,
                ..*t
            };
            return;
        }
        if a.prev.is_some_and(|p| p.pitch == a.pitch && !p.black()) {
            found.push(Found::at(Case::RepeatedPitch, a.note, a.start));
        }
        t.attacks = t.attacks.saturating_add(1);
        let was_lead_in = t.lead_in;
        // A lead-in is black, and it leads into the syllable's last note within the measure
        // before it (Stoquerus XXIX.4).
        t.lead_in &= a.prev.is_some_and(|p| p.black()) && a.start - t.since <= self.per_measure;
        // Where a melisma may stand depends on the syllable, which a syllable that is not in the
        // text does not tell; its length still counts.
        let current = self.current(t);
        if t.attacks == 2
            && let Some(syl) = current
        {
            let case = if syl.short_penult {
                Some(Case::AccentShortPenult)
            } else if syl.function_word && !syl.unit_end {
                Some(Case::MelismaFunctionWord)
            } else if !syl.accented && !syl.opening && !syl.unit_end {
                Some(Case::MelismaUnstressed)
            } else {
                None
            };
            found.extend(case.map(|case| Found::at(case, t.note as usize, a.start)));
        }
        // A note of the last syllable of a unit that is not black, followed by this one, or an
        // attack more than a measure after the syllable began: the melisma is no lead-in,
        // whatever follows.
        if was_lead_in && !t.lead_in && current.is_some_and(|syl| syl.unit_end) {
            found.push(Found::at(
                Case::MelismaLastSyllable,
                t.note as usize,
                a.start,
            ));
        }
        self.outlast(t, a.start + a.written, a.start, found);
        if t.attacks >= 2 {
            // The limit follows the syllable: an unaccented one takes fewer notes than the
            // accent (Zarlino 1589, Stoquerus XX.3), and a unit's last syllable should rather be
            // restated than held on (Stoquerus XXV).
            let case = match current {
                Some(syl) if syl.unit_end && !t.lead_in => Case::MelismaLongLast,
                Some(syl) if !syl.accented && !syl.unit_end && syl.opening => {
                    Case::MelismaLongOpening
                }
                Some(syl) if !syl.accented && !syl.unit_end => Case::MelismaLongWeak,
                _ => match self.kind {
                    TextKind::Short => Case::MelismaLongShort,
                    TextKind::Long => Case::MelismaLongLong,
                },
            };
            found.push(Found {
                value: t.attacks,
                ..Found::at(case, a.note, a.start)
            });
        }
    }

    /// Begins syllable `y` on note `note` at `start`: at an attack (with whether the texted
    /// dactyl quarter came just before), or on a tie continuation (`None`, imported scores only).
    fn begin(
        &self,
        t: &mut UState,
        y: u16,
        attack: Option<(&Attack, bool)>,
        note: usize,
        start: u32,
        found: &mut Vec<Found>,
    ) {
        let before = t.syllable();
        // Not the voice's first entry: it has sung a syllable before, in the text or not.
        let entered = before.is_some() || t.unknown;
        if t.open {
            self.close(t, start, found);
        }
        let syl = self.syls[usize::from(y)];
        // The syllable a restatement begins after.
        let mut restated = None;
        match before {
            None if y != 0 => found.push(Found::at(Case::OrderFirst, note, start)),
            None => {}
            Some(x) if usize::from(y) == x + 1 => {}
            Some(x) if self.targets[x].contains(&y) => restated = Some(x),
            Some(_) => found.push(Found::at(Case::Order, note, start)),
        }
        let restarts = if let Some(x) = restated {
            let case = match self.kind {
                TextKind::Short => Some(Case::RepeatShort),
                TextKind::Long if !syl.last_unit => Some(Case::RepeatLong),
                TextKind::Long => None,
            };
            let again = (t.restarts > 0).then_some(Case::RepeatAgain);
            let word = self.lone[x].contains(&y).then_some(Case::RepeatWord);
            for case in [case, again, word].into_iter().flatten() {
                found.push(Found::at(case, note, start));
            }
            (t.restarts + 1).min(MAX_RESTARTS)
        } else if before.is_some_and(|x| self.syls[x].sentence == syl.sentence) {
            t.restarts
        } else {
            0
        };
        // On a tie continuation the quarter that began the sounding stays undecided.
        let mut pending = t.pending;
        if let Some((a, after_dactyl)) = attack {
            pending = Pending::None;
            if !a.entry {
                pending = self.onset(a, after_dactyl, &syl, found);
            } else if entered && syl.opening && a.written < HALF {
                // An entry after a rest may begin on any value (Zarlino 7), but a part of the
                // text opens on a white note there too.
                found.push(Found::at(Case::OnsetOpeningBlack, a.note, a.start));
            }
        }
        let sung = match before {
            Some(x) if usize::from(y) == x + 1 && !self.syls[x].word_end => t.sung,
            _ => None,
        };
        *t = UState {
            syl: y,
            note: note as u32,
            since: start,
            attacks: 1,
            lead_in: true,
            open: true,
            pending,
            sung,
            restarts,
            unknown: false,
        };
        if let Some((a, _)) = attack {
            self.outlast(t, a.start + a.written, a.start, found);
        }
    }

    /// Begins a syllable that is not in the text on note `note` at `start`, at an attack or on a
    /// tie continuation, as [`Model::begin`] does: it ends the syllable before, and its onset
    /// counts (TXT-ONSET, TXT-AFTER-BLACK). It has no place in the text, so the voice stays at
    /// the syllable it sang last: the next syllable of the text is judged against that one
    /// (TXT-ORDER, TXT-REPEAT), and no case that depends on the syllable itself is reported.
    fn unknown(
        &self,
        t: &mut UState,
        attack: Option<(&Attack, bool)>,
        note: usize,
        start: u32,
        found: &mut Vec<Found>,
    ) {
        if t.open {
            self.close(t, start, found);
        }
        let mut pending = t.pending;
        if let Some((a, after_dactyl)) = attack {
            pending = Pending::None;
            if !a.entry {
                pending = self.onset(a, after_dactyl, &UNMATCHED, found);
            }
        }
        *t = UState {
            note: note as u32,
            since: start,
            attacks: 1,
            lead_in: true,
            open: true,
            pending,
            sung: None,
            unknown: true,
            ..*t
        };
    }

    /// The cases of a new syllable on an attack that is no entry: TXT-ONSET or else
    /// TXT-AFTER-BLACK, TXT-ONSET `opening_black` and TXT-LEAP. Returns what stays undecided
    /// about a texted quarter.
    ///
    /// The dactyl quarter is meant for the short middle syllable of Dó-mi-ne (Jeppesen, rule 2;
    /// Zacconi): an unaccented syllable that neither opens a part of the text nor ends a unit
    /// (TXT-ONSET `figure`). Any other syllable there is `figure_accent`.
    fn onset(&self, a: &Attack, after_dactyl: bool, syl: &Syl, found: &mut Vec<Found>) -> Pending {
        let quarter = (QUARTER..HALF).contains(&a.written);
        let on_beat = a.start.is_multiple_of(HALF);
        let dactyl = a.written == QUARTER
            && a.prev
                .is_some_and(|p| p.len == DOTTED_HALF && p.start.is_multiple_of(HALF));
        let after_black = a.prev.is_some_and(|p| p.black());
        let onset = if a.written < QUARTER {
            Some(Case::Onset)
        } else if !on_beat {
            Some(if !dactyl {
                Case::Onset
            } else if syl.accented || syl.opening || syl.unit_end {
                Case::OnsetFigureAccent
            } else {
                Case::OnsetFigure
            })
        } else if quarter && !a.start.is_multiple_of(self.per_measure) && !after_black {
            Some(Case::OnsetRunUpbeat)
        } else {
            None
        };
        // A white note after the dactyl quarter that kept the syllable before it may begin one
        // at the cost of `dotted`: Jeppesen prints two in Palestrina's Christe of Dies
        // sanctificatus, while Stoquerus' moderns (M3) want none there.
        let case = onset.or_else(|| {
            after_black.then_some(if after_dactyl {
                Case::AfterBlackFigure
            } else if syl.unit_end && a.written >= HALF {
                Case::AfterBlackUnitEnd
            } else if a.written >= HALF && a.prev.is_some_and(|p| p.dactyl) {
                Case::AfterBlackDotted
            } else {
                Case::AfterBlack
            })
        });
        found.extend(case.map(|case| Found::at(case, a.note, a.start)));
        // A part of the text, a restatement too, opens on a white note (Jeppesen, p. 269); the
        // dactyl quarter is `figure_accent` for it.
        if syl.opening && quarter && on_beat {
            found.push(Found::at(Case::OnsetOpeningBlack, a.note, a.start));
        }
        if let Some(p) = a.prev.filter(|_| !syl.word_start) {
            let leap = match (a.pitch.diatonic() - p.pitch.diatonic()).abs() {
                3 => Some(Case::LeapFourth),
                4 => Some(Case::LeapFifth),
                5 => Some(Case::LeapSixth),
                6.. => Some(Case::LeapOctave),
                _ => None,
            };
            found.extend(leap.map(|case| Found::at(case, a.note, a.start)));
        }
        if quarter && on_beat {
            Pending::Run(a.note as u32)
        } else if dactyl {
            Pending::Dactyl(a.note as u32)
        } else {
            Pending::None
        }
    }

    /// TXT-ACCENT as soon as the notes show that a syllable outlasts the one before it in its
    /// word, with which it is compared: at an attack or a tie continuation of it, decided at
    /// `at`, whose written value lasts until `until`. That settles the pair: an unaccented
    /// syllable after a stressed one has its case now, a stressed one after an unaccented one can
    /// no longer be the shorter. Otherwise the end of the syllable decides ([`Model::close`]).
    fn outlast(&self, t: &mut UState, until: u32, at: u32, found: &mut Vec<Found>) {
        // A syllable that is not in the text has no `sung`.
        let (Some(before), Some(x)) = (t.sung, t.syllable()) else {
            return;
        };
        let syl = self.syls[x];
        if !t.open || !syl.pair_before {
            return;
        }
        let len = u16::try_from(until - t.since).unwrap_or(u16::MAX);
        if len <= before.len {
            return;
        }
        let span = Span {
            syl,
            len,
            downbeat: t.since.is_multiple_of(self.per_measure),
            note: t.note,
            weak: false,
        };
        accent(self.ended(x - 1, before), span, at, found);
        t.sung = None;
    }

    /// Syllable `x`, which has ended, as `sung` tells it.
    fn ended(&self, x: usize, sung: Sung) -> Span {
        Span {
            syl: self.syls[x],
            len: sung.len,
            downbeat: sung.downbeat,
            note: sung.note,
            weak: sung.weak,
        }
    }

    /// Ends the current syllable at `at`: the comparison with the syllable before it in its word
    /// (TXT-ACCENT), unless that is settled.
    fn close(&self, t: &mut UState, at: u32, found: &mut Vec<Found>) {
        t.open = false;
        // A syllable that is not in the text is compared with nothing.
        let Some(x) = t.syllable().filter(|_| !t.unknown) else {
            t.sung = None;
            return;
        };
        let syl = self.syls[x];
        let mut span = Span {
            syl,
            len: u16::try_from(at.saturating_sub(t.since)).unwrap_or(u16::MAX),
            downbeat: t.since.is_multiple_of(self.per_measure),
            note: t.note,
            weak: false,
        };
        if syl.pair_before
            && let Some(before) = t.sung
        {
            span.weak = accent(self.ended(x - 1, before), span, at, found);
        }
        t.sung = (!syl.word_end).then_some(Sung {
            len: span.len,
            downbeat: span.downbeat,
            note: span.note,
            weak: span.weak,
        });
    }

    /// TXT-CADENCE at an attack on the target of an internal cadence, reached by a step or by a
    /// fourth or fifth.
    fn cadence(&self, t: &UState, a: &Attack, found: &mut Vec<Found>) {
        // Whether a syllable that is not in the text ends a unit is not known.
        if t.unknown {
            return;
        }
        let Some(goal) = self.goals.iter().find(|g| g.tick == a.start) else {
            return;
        };
        let Some(prev) = a.prev else {
            return;
        };
        let moved = (a.pitch.diatonic() - prev.pitch.diatonic()).abs();
        if !a.pitch.same_class(goal.target) || !matches!(moved, 1 | 3 | 4) {
            return;
        }
        // The last syllable before a marked break, also before a tail, stops the words as an
        // intermediate stop does: the plan puts cadences there too.
        let case = match t.syllable().map(|x| self.syls[x]) {
            Some(syl) if syl.full_stop => return,
            Some(syl) if (syl.unit_end || syl.break_after) && goal.principal => {
                Case::CadencePrincipal
            }
            Some(syl) if syl.unit_end || syl.break_after => Case::CadenceComma,
            _ => Case::Cadence,
        };
        found.push(Found::at(case, a.note, a.start));
    }

    /// The state with what only serves the reports cleared, so that states with equal futures
    /// are equal: the notes, the length and start of a syllable that is compared with no
    /// neighbour and has no lead-in that counts, attacks beyond what the profile distinguishes,
    /// a lead-in that does not count, and restatements beyond the first.
    pub fn canonical(&self, state: &UState, levels: &Levels) -> UState {
        let mut c = *state;
        c.note = 0;
        c.pending = match state.pending {
            Pending::None => Pending::None,
            Pending::Run(_) => Pending::Run(0),
            Pending::Dactyl(_) => Pending::Dactyl(0),
        };
        c.restarts = state.restarts.min(1);
        // A syllable that is not in the text arises only on the checker's path.
        let Some(x) = state.syllable().filter(|_| !state.unknown) else {
            return c;
        };
        let syl = self.syls[x];
        let keep = |sung: Option<Sung>, compared: bool| {
            sung.filter(|_| compared).map(|s| Sung { note: 0, ..s })
        };
        if state.open {
            // The lead-in of a unit's last syllable counts while it lasts, and only where its
            // melisma costs, or its notes beyond `max_last`; its bound needs the syllable's
            // start.
            let lead_in = syl.unit_end
                && (levels.of(Case::MelismaLastSyllable) != Level::Off
                    || levels.of(Case::MelismaLongLast) != Level::Off);
            if !lead_in {
                c.lead_in = true;
            }
            let compared = (syl.pair_before && state.sung.is_some()) || syl.pair_after;
            if !compared && !(lead_in && state.lead_in) {
                c.since = 0;
            }
            c.sung = keep(state.sung, syl.pair_before);
            c.attacks = state.attacks.min(levels.attack_bound(self.kind));
        } else {
            // After a rest only an entry follows, which begins a new syllable.
            c.since = 0;
            c.attacks = 0;
            c.lead_in = true;
            c.sung = keep(state.sung, syl.pair_after);
        }
        c
    }

    /// The cases of a voice's own underlay: a new syllable where a note has a lyric. With
    /// `complete` the voice is complete, and its end counts as an event. A lyric on a rest or on a
    /// tie continuation is reported (TXT-ONSET); on a tie continuation it begins its syllable
    /// there.
    ///
    /// A lyric that matches no syllable of the text ([`Lyric::UNKNOWN`], which the aligner gives
    /// an imported syllable it cannot place, or a position beyond the text) begins a syllable of
    /// unknown text. It ends the syllable before, and its own onset counts (TXT-ONSET,
    /// TXT-AFTER-BLACK); but it has no place in the text: no TXT-ORDER case for it, the next
    /// syllable of the text is judged against the one sung before it, as the aligner placed it,
    /// and no case that depends on which syllable it is gets reported. An entry after a rest
    /// without a lyric (TXT-ENTRY) begins such a syllable too, for the rest has ended the one
    /// before.
    pub fn cases(&self, notes: &[Note], complete: bool) -> Vec<Found> {
        let mut found = Vec::new();
        let mut state = UState::START;
        let mut start = 0;
        for (i, note) in notes.iter().enumerate() {
            let event = Event::at(notes, i, start);
            // The syllable a lyric begins; `Some(None)` for one that is not in the text.
            let lyric = note.lyric.map(|lyric| self.position(lyric));
            if let Some(y) = lyric {
                if note.is_rest() {
                    found.push(Found::at(Case::Onset, i, start));
                } else if let Some(Event::Held(_)) = event {
                    found.push(Found::at(Case::Onset, i, start));
                    match y {
                        Some(y) => self.begin(&mut state, y, None, i, start, &mut found),
                        None => self.unknown(&mut state, None, i, start, &mut found),
                    }
                }
            }
            if let Some(event) = event {
                state = match (event, lyric) {
                    (Event::Attack(a), Some(None)) => {
                        // As `step` takes an attack.
                        let mut t = state;
                        let after_dactyl = matches!(t.pending, Pending::Dactyl(_));
                        self.resolve(&mut t, Some(a.written), a.start, &mut found);
                        self.unknown(
                            &mut t,
                            Some((&a, after_dactyl)),
                            a.note,
                            a.start,
                            &mut found,
                        );
                        self.cadence(&t, &a, &mut found);
                        t
                    }
                    (Event::Attack(_), Some(Some(y))) => {
                        self.step(&state, &event, Choice::Start(y), &mut found)
                    }
                    _ => self.step(&state, &event, Choice::Hold, &mut found),
                };
            }
            start += note.duration.ticks();
        }
        if complete && let Some(end) = Event::end(notes) {
            self.step(&state, &end, Choice::Hold, &mut found);
        }
        found
    }

    /// The syllable a lyric begins, `None` if it is not in the text.
    fn position(&self, lyric: Lyric) -> Option<u16> {
        lyric
            .index()
            .filter(|&y| y < self.syls.len())
            .map(|y| y as u16)
    }

    /// The cheapest underlay of a complete voice: no forbidden case, the text from its first
    /// syllable to its last, the least cost and, among those, the earliest syllables. A dynamic
    /// program over all its events, without the limit and the capacity of the search.
    pub fn best(&self, levels: &Levels, notes: &[Note]) -> Option<Path> {
        self.best_pinned(levels, notes, None, None)
    }

    /// As [`Model::best`], keeping the pins and the gates: a pinned attack takes only its
    /// pinned choice, and no syllable begins before its gate.
    pub fn best_pinned(
        &self,
        levels: &Levels,
        notes: &[Note],
        pins: Option<&Pins>,
        gates: Option<&Gates>,
    ) -> Option<Path> {
        let mut events = Event::all(notes, true);
        let end = match events.pop() {
            Some(end @ Event::End(_)) => end,
            _ => return None,
        };
        let bounds = (pins, gates);
        let (cost, lateness, choices) =
            self.search_with(levels, UState::START, &events, Finish::End(&end), bounds)?;
        Some(Path::new(&events, choices, cost, lateness))
    }

    /// The cheapest underlay of a head that sings syllables `first` to `last` of the text, from
    /// a voice's state before it (its start, or [`Model::after_rest`]): `first` begins on the
    /// first attack, every syllable up to `last` begins and none after it, without a forbidden
    /// case; the least cost and, among those, the earliest syllables. The notes enter after the
    /// state, rests before the head place it in the measure. The voice goes on after them, so
    /// its end is not stepped and what the next note decides stays open. `None` if the head has
    /// no such underlay.
    pub fn best_prefix(
        &self,
        levels: &Levels,
        from: UState,
        notes: &[Note],
        first: u16,
        last: u16,
    ) -> Option<Path> {
        let events = Event::all(notes, false);
        let entry = events.iter().find_map(|event| match event {
            Event::Attack(a) => Some(a.start),
            _ => None,
        })?;
        let pins = [(entry, Choice::Start(first))];
        let bounds = (Some(&pins[..]), None);
        let (cost, lateness, choices) =
            self.search_with(levels, from, &events, Finish::Head(last), bounds)?;
        Some(Path::new(&events, choices, cost, lateness))
    }

    /// The cheapest way from `state` through `events` and the end: (cost, lateness, the choice
    /// at each event).
    #[cfg(test)]
    fn search(
        &self,
        levels: &Levels,
        state: UState,
        events: &[Event],
        end: &Event,
    ) -> Option<(u32, u32, Vec<Choice>)> {
        self.search_with(levels, state, events, Finish::End(end), (None, None))
    }

    /// The cheapest way from `state` through `events` to the finish, keeping the pins and the
    /// gates of `bounds`: (cost, lateness, the choice at each event).
    fn search_with(
        &self,
        levels: &Levels,
        state: UState,
        events: &[Event],
        finish: Finish<'_>,
        (pins, gates): (Option<&Pins>, Option<&Gates>),
    ) -> Option<(u32, u32, Vec<Choice>)> {
        /// A state reached at an event, with the best way there.
        #[derive(Clone, Copy)]
        struct Node {
            key: u128,
            state: UState,
            cost: u32,
            late: u32,
            /// Index of the node before, in the layer of the event before.
            back: u32,
            choice: Choice,
        }
        debug_assert!(
            pins.is_none_or(|pins| pins.windows(2).all(|w| w[0].0 < w[1].0)),
            "pins out of order"
        );
        // Attacks after each event: a state that needs more syllables cannot finish.
        let mut later = vec![0; events.len() + 1];
        for (i, event) in events.iter().enumerate().rev() {
            later[i] = later[i + 1] + u32::from(matches!(event, Event::Attack(_)));
        }
        // The syllables a state still has to begin.
        let need = |t: &UState| match finish {
            Finish::End(_) => self.need(t),
            Finish::Head(last) => match t.syllable() {
                None => u32::from(last) + 1,
                Some(x) => u32::from(last).saturating_sub(x as u32),
            },
        };
        let state = self.canonical(&state, levels);
        let mut layers = vec![vec![Node {
            key: state.key(),
            state,
            cost: 0,
            late: 0,
            back: 0,
            choice: Choice::Hold,
        }]];
        let mut found = Vec::new();
        let mut choices = Vec::new();
        for (i, event) in events.iter().enumerate() {
            let mut next = Vec::new();
            let pin = pinned(pins, event);
            for (j, node) in layers[i].iter().enumerate() {
                self.options(&node.state, event, &mut choices);
                if let Some(pin) = pin {
                    choices.retain(|&c| c == pin);
                }
                choices.retain(|&c| !early(gates, event, c));
                if let Finish::Head(last) = finish {
                    choices.retain(|&c| !matches!(c, Choice::Start(y) if y > last));
                }
                for &choice in &choices {
                    found.clear();
                    let t = self.step(&node.state, event, choice, &mut found);
                    let (cost, forbidden) = levels.price(&found);
                    if forbidden.is_some() || need(&t) > later[i + 1] {
                        continue;
                    }
                    let late = match choice {
                        Choice::Start(_) => node.late + event.start(),
                        Choice::Hold => node.late,
                    };
                    let state = self.canonical(&t, levels);
                    next.push(Node {
                        key: state.key(),
                        state,
                        cost: node.cost + cost,
                        late,
                        back: j as u32,
                        choice,
                    });
                }
            }
            // The best way to each state; on a tie the first.
            next.sort_by_key(|node| (node.key, node.cost, node.late));
            next.dedup_by_key(|node| node.key);
            if next.is_empty() {
                return None;
            }
            layers.push(next);
        }
        let mut best: Option<(u32, u32, usize)> = None;
        for (j, node) in layers[events.len()].iter().enumerate() {
            let key = match finish {
                Finish::End(end) => {
                    found.clear();
                    self.step(&node.state, end, Choice::Hold, &mut found);
                    if found.iter().any(|f| f.case == Case::Complete) {
                        continue;
                    }
                    match levels.price(&found) {
                        (cost, None) => (node.cost + cost, node.late),
                        (_, Some(_)) => continue,
                    }
                }
                Finish::Head(last) if node.state.syllable() == Some(usize::from(last)) => {
                    (node.cost, node.late)
                }
                Finish::Head(_) => continue,
            };
            if best.is_none_or(|(c, l, _)| key < (c, l)) {
                best = Some((key.0, key.1, j));
            }
        }
        let (cost, lateness, mut j) = best?;
        let mut path = vec![Choice::Hold; events.len()];
        for i in (0..events.len()).rev() {
            let node = layers[i + 1][j];
            path[i] = node.choice;
            j = node.back as usize;
        }
        Some((cost, lateness, path))
    }
}

/// Where the dynamic program ends.
#[derive(Clone, Copy)]
enum Finish<'a> {
    /// At the end of a complete voice, which it steps: the voice sings the text to its last
    /// syllable.
    End(&'a Event),
    /// After a head, which sings up to this syllable and begins no later one.
    Head(u16),
}

/// TXT-ACCENT for two neighbouring syllables of a word, decided at `at`, where the second ends
/// or outlasts the first. Returns whether `b` is reported weak.
fn accent(a: Span, b: Span, at: u32, found: &mut Vec<Found>) -> bool {
    let mut push = |case, note: u32| found.push(Found::at(case, note as usize, at));
    let lengthened = |s: &Span| {
        if s.syl.opening {
            Case::AccentOpening
        } else {
            Case::AccentLonger
        }
    };
    // An unaccented syllable longer than the stressed one next to it, unless it ends the word.
    if !a.syl.accented && b.syl.stressed && a.len > b.len {
        push(lengthened(&a), a.note);
    }
    if a.syl.stressed && !b.syl.accented && !b.syl.word_end && b.len > a.len {
        push(lengthened(&b), b.note);
    }
    // A stressed syllable shorter than an unaccented one next to it and off the downbeat, while
    // that one begins on it: once per stressed syllable, at the first pair that shows it.
    if a.syl.stressed && !b.syl.accented && a.len < b.len && !a.downbeat && b.downbeat && !a.weak {
        push(Case::AccentWeak, a.note);
    }
    let weak = b.syl.stressed && !a.syl.accented && b.len < a.len && !b.downbeat && a.downbeat;
    if weak {
        push(Case::AccentWeak, b.note);
    }
    weak
}

/// The underlay the dynamic program chose for a voice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    /// The attacks that begin a syllable: (note index, syllable index).
    pub starts: Vec<(usize, u16)>,
    /// The cost of its cases.
    pub cost: u32,
    /// The sum of the ticks at which its syllables begin: the less, the earlier the text.
    pub lateness: u32,
}

impl Path {
    /// The path of the choices at the events, with what it costs and its lateness.
    fn new(events: &[Event], choices: Vec<Choice>, cost: u32, lateness: u32) -> Path {
        let starts = events
            .iter()
            .zip(choices)
            .filter_map(|(event, choice)| match (event, choice) {
                (Event::Attack(a), Choice::Start(y)) => Some((a.note, y)),
                _ => None,
            })
            .collect();
        Path {
            starts,
            cost,
            lateness,
        }
    }

    /// Writes the syllables into the voice's notes as lyrics.
    pub fn apply(&self, notes: &mut [Note]) {
        for &(i, y) in &self.starts {
            notes[i].lyric = Some(Lyric::new(usize::from(y)));
        }
    }
}

/// The underlay states a voice's notes allow, each with its least cost and lateness (the sum of
/// the ticks its syllables begin at). The cheapest, then earliest, comes first; the order of the
/// others means nothing.
pub type Set = Vec<(UState, u32, u32)>;

/// The automaton of a singing voice in the search: it steps the voice's set of underlay states
/// over each new event, keeping only states without a forbidden case.
#[derive(Clone, Copy)]
pub struct Automaton<'a> {
    pub model: &'a Model,
    pub levels: &'a Levels,
    /// Where the voice ends: once its notes reach this tick, its end is stepped too.
    pub total: u32,
    /// Cuts states that need more syllables than the voice has beats left for.
    pub cap: Option<&'a Capacity>,
    /// The most states kept, the cheapest; [`LIMIT`] in the search.
    pub limit: Option<usize>,
    /// The choices pinned at attacks of the voice. With a capacity, the next pinned syllable
    /// start is also a milestone: states that can no longer begin it in time are cut.
    pub pins: Option<&'a Pins>,
    /// The syllables the voice may not begin before the points of an imitation that set them
    /// begin.
    pub gates: Option<&'a Gates>,
}

impl Automaton<'_> {
    /// The set before the voice's first note.
    pub fn start() -> Set {
        vec![(UState::START, 0, 0)]
    }

    /// The set after the voice's last written note, from the set before it: every state with
    /// every choice, at a pinned attack only the pinned one and no syllable before its gate,
    /// without those that break a prohibition, need more syllables than the capacity leaves room
    /// for after the note ([`Capacity::after`]) or, with pins and a capacity, cannot begin the next
    /// pinned syllable in time: they still have to begin more syllables before it
    /// ([`Model::to_reach`]) than the capacity leaves room for before its beat. The cheapest entry
    /// of each state, at most `limit` of them (the cheapest, then the earliest). A tie
    /// continuation is stepped too, as an event of its own ([`Event::Held`]). Once the notes reach
    /// `total`, the end is stepped too, and only states that have sung the text to its last
    /// syllable remain. Fails with the rule that cut the most states if none is left: TXT-ORDER
    /// for a state that misses a pinned syllable or could only go on before a gate, TXT-ENTRY for
    /// one that cannot hold its syllable where a melisma is pinned.
    pub fn advance(&self, set: &[(UState, u32, u32)], notes: &[Note]) -> Result<Set, &'static str> {
        let mut blocked = Blocked::default();
        let mut found = Vec::new();
        let mut next: Vec<Entry> = match Event::of_last(notes) {
            None => set
                .iter()
                .map(|&(s, cost, late)| (s.key(), cost, late, s))
                .collect(),
            Some(event) => {
                let mut out = Vec::with_capacity(set.len() * 3);
                let mut choices = Vec::new();
                let room = self.cap.map(|cap| cap.after(&event));
                let pin = pinned(self.pins, &event);
                // The next pinned syllable start, with the room left before its beat.
                let milestone = self.pins.zip(self.cap).and_then(|(pins, cap)| {
                    let (tick, y) = next_start(pins, event.start())?;
                    Some((y, cap.after(&event).saturating_sub(cap.at(tick))))
                });
                for &(state, cost, late) in set {
                    self.model.options(&state, &event, &mut choices);
                    if let Some(pin) = pin {
                        if !choices.contains(&pin) {
                            blocked.add(refused(pin));
                            continue;
                        }
                        choices.clear();
                        choices.push(pin);
                    }
                    // A syllable before its gate is out of the order of the plan.
                    let open = choices.len();
                    choices.retain(|&c| !early(self.gates, &event, c));
                    for _ in choices.len()..open {
                        blocked.add(Case::Order.rule());
                    }
                    for &choice in &choices {
                        found.clear();
                        let t = self.model.step(&state, &event, choice, &mut found);
                        let (add, forbidden) = self.levels.price(&found);
                        if let Some(rule) = forbidden {
                            blocked.add(rule);
                            continue;
                        }
                        if room.is_some_and(|room| self.model.need(&t) > room) {
                            blocked.add(Case::Complete.rule());
                            continue;
                        }
                        if let Some((y, before)) = milestone
                            && self.model.to_reach(&t, y).is_none_or(|n| n > before)
                        {
                            blocked.add(Case::Order.rule());
                            continue;
                        }
                        let late = match choice {
                            Choice::Start(_) => late + event.start(),
                            Choice::Hold => late,
                        };
                        let t = self.model.canonical(&t, self.levels);
                        out.push((t.key(), cost + add, late, t));
                    }
                }
                merge(out)
            }
        };
        let end: u32 = notes.iter().map(|n| n.duration.ticks()).sum();
        if end == self.total
            && let Some(event) = Event::end(notes)
        {
            let mut done = Vec::with_capacity(next.len());
            for (_, cost, late, state) in next {
                found.clear();
                let t = self.model.step(&state, &event, Choice::Hold, &mut found);
                if found.iter().any(|f| f.case == Case::Complete) {
                    blocked.add(Case::Complete.rule());
                    continue;
                }
                match self.levels.price(&found) {
                    (_, Some(rule)) => blocked.add(rule),
                    (add, None) => {
                        let t = self.model.canonical(&t, self.levels);
                        done.push((t.key(), cost + add, late, t));
                    }
                }
            }
            next = merge(done);
        }
        select(&mut next, self.limit);
        if next.is_empty() {
            Err(blocked.most())
        } else {
            Ok(next
                .into_iter()
                .map(|(_, cost, late, state)| (state, cost, late))
                .collect())
        }
    }

    /// The set after all notes of a voice, from its start, as the search would advance it note
    /// by note.
    pub fn run(&self, notes: &[Note]) -> Result<Set, &'static str> {
        let mut set = Automaton::start();
        for i in 0..notes.len() {
            set = self.advance(&set, &notes[..=i])?;
        }
        Ok(set)
    }
}

/// A state of a set with its key, its cost and its lateness.
type Entry = (u128, u32, u32, UState);

/// The order in which a set keeps its states: the cheapest, then the earliest; the key makes it
/// total.
fn order(&(key, cost, late, _): &Entry) -> (u32, u32, u128) {
    (cost, late, key)
}

/// Keeps the first `limit` entries in [`order`], sorted; without a limit, or where it cuts
/// nothing, only puts the first in front. Nothing reads the order of the others: a set costs
/// what its first entry costs, and merging does not depend on the order. So a set is sorted only
/// where the limit cuts it, and then only the part it keeps.
fn select(entries: &mut Vec<Entry>, limit: Option<usize>) {
    match limit {
        Some(limit) if entries.len() > limit => {
            if limit > 0 {
                entries.select_nth_unstable_by_key(limit - 1, order);
            }
            entries.truncate(limit);
            entries.sort_unstable_by_key(order);
        }
        _ => {
            let first = (0..entries.len()).min_by_key(|&i| order(&entries[i]));
            if let Some(first) = first {
                entries.swap(0, first);
            }
        }
    }
}

/// Keeps the cheapest entry of each state, then the earliest, in the order the states first
/// occur. A small hash table on the keys makes this one pass.
fn merge(entries: Vec<Entry>) -> Vec<Entry> {
    const EMPTY: u32 = u32::MAX;
    let size = (2 * entries.len()).next_power_of_two().max(16);
    let mut table = vec![EMPTY; size];
    let mut merged: Vec<Entry> = Vec::with_capacity(entries.len());
    for entry in entries {
        let folded = (entry.0 as u64) ^ ((entry.0 >> 64) as u64).rotate_left(29);
        let mut slot = (folded.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40) as usize & (size - 1);
        loop {
            match table[slot] {
                EMPTY => {
                    table[slot] = merged.len() as u32;
                    merged.push(entry);
                    break;
                }
                i => {
                    let kept = &mut merged[i as usize];
                    if kept.0 == entry.0 {
                        if (entry.1, entry.2) < (kept.1, kept.2) {
                            *kept = entry;
                        }
                        break;
                    }
                    slot = (slot + 1) & (size - 1);
                }
            }
        }
    }
    merged
}

/// How many states each rule cut, for the diagnosis.
#[derive(Default)]
struct Blocked(Vec<(&'static str, u32)>);

impl Blocked {
    fn add(&mut self, rule: &'static str) {
        match self.0.iter_mut().find(|(r, _)| *r == rule) {
            Some((_, count)) => *count += 1,
            None => self.0.push((rule, 1)),
        }
    }

    /// The rule that cut the most states; on a tie the first by id.
    fn most(&self) -> &'static str {
        self.0
            .iter()
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(a.0)))
            .map_or(Case::Complete.rule(), |(rule, _)| rule)
    }
}

/// How many syllables a voice can still begin from a tick on: per minim beat at most one,
/// because a syllable begins on a minim beat or uses the beat of the dotted half or the rest
/// before it (the dactyl quarter, an entry after a quarter rest). Beats on which no note of the
/// voice can begin do not count: those held by a fixed note that began before them or by a fixed
/// tie continuation. Where the profile does not forbid syllables off the minim beats
/// (TXT-ONSET `level`), every tick counts instead. So the bound never cuts a state that could
/// still finish.
///
/// After an attack, where the profile also forbids rests right after black notes (TXT-REST
/// `short`), the bound counts from the next beat on ([`Capacity::after_attack`]): later in the
/// attack's beat neither the dactyl quarter (whose dotted half would have to begin before the
/// attack) nor an entry (whose rest would follow a black note) can begin a syllable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capacity {
    /// Ticks per counted place: a minim beat, or one tick.
    unit: u32,
    /// No syllable can begin later in the beat of an attack.
    tight: bool,
    /// Per place, the places from it to the end on which a syllable can begin.
    from: Vec<u32>,
}

impl Capacity {
    /// The capacity of a voice of `total` ticks with fixed notes given as (start, ticks, tied
    /// in). A last measure the voice holds counts as a fixed note.
    pub fn new(total: u32, fixed: &[(u32, u32, bool)], levels: &Levels) -> Capacity {
        let unit = if levels.of(Case::Onset) == Level::Forbidden {
            HALF
        } else {
            1
        };
        // Where a note of the voice can begin.
        let mut free = vec![true; total as usize];
        for &(start, ticks, tied_in) in fixed {
            let from = if tied_in { start } else { start + 1 };
            for tick in from..(start + ticks).min(total) {
                free[tick as usize] = false;
            }
        }
        let places = total.div_ceil(unit) as usize;
        let mut from = vec![0; places + 1];
        for place in (0..places).rev() {
            let ticks = place * unit as usize..((place + 1) * unit as usize).min(total as usize);
            from[place] = from[place + 1] + u32::from(free[ticks].iter().any(|&f| f));
        }
        let tight = unit == HALF && levels.of(Case::RestShort) == Level::Forbidden;
        Capacity { unit, tight, from }
    }

    /// How many syllables can begin from the beat of `tick` on, that beat included.
    pub fn at(&self, tick: u32) -> u32 {
        self.from
            .get((tick / self.unit) as usize)
            .copied()
            .unwrap_or(0)
    }

    /// How many syllables can begin after an attack at `tick`: from the next beat on where none
    /// can begin later in the attack's beat, otherwise from its beat on.
    pub fn after_attack(&self, tick: u32) -> u32 {
        if self.tight {
            self.from
                .get((tick / self.unit) as usize + 1)
                .copied()
                .unwrap_or(0)
        } else {
            self.at(tick)
        }
    }

    /// How many syllables a voice can still begin after an event, the bound the automaton cuts
    /// with: after an attack [`Capacity::after_attack`]; otherwise from the event's beat on, in
    /// which an entry may still follow a rest and the dactyl quarter a tie continuation.
    pub fn after(&self, event: &Event) -> u32 {
        match event {
            Event::Attack(a) => self.after_attack(a.start),
            _ => self.at(event.start()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::Rng;
    use crate::rules::Profile;
    use crate::text::test_support::{below, own, random_text, random_voice, section, sung};

    fn fux() -> Levels {
        Levels::from(&Checker::new(&Profile::fux()))
    }

    fn dorian() -> ModeSpec {
        "dorian".parse().unwrap()
    }

    fn model(text: &Text) -> Model {
        Model::new(text, dorian(), Meter::ALLA_BREVE, &[])
    }

    /// The cases of a voice's underlay as (case, tick, marked note), without TXT-MELISMA `long`
    /// within its limit.
    fn found(model: &Model, src: &str, complete: bool) -> Vec<(Case, u32, usize)> {
        let levels = fux();
        model
            .cases(&sung(src), complete)
            .into_iter()
            .filter(|f| f.case.limit().is_none() || levels.level(f) != Level::Off)
            .map(|f| (f.case, f.tick, f.note))
            .collect()
    }

    fn pitch(name: &str) -> Pitch {
        name.parse().unwrap()
    }

    #[test]
    fn events_of_a_voice() {
        // r2 d'2 | e'2~ e'4 e'4 | r4 f'4 g'1
        let notes = sung("r2 d'2 e'2~ e'4 e'4 r4 f'4 g'1");
        let prev = |note, start, len, name| {
            Some(Prev {
                note,
                start,
                len,
                pitch: pitch(name),
                dactyl: false,
            })
        };
        let attack = |note, start, written, name, entry, prev| {
            Event::Attack(Attack {
                note,
                start,
                written,
                pitch: pitch(name),
                entry,
                prev,
            })
        };
        assert_eq!(
            Event::all(&notes, true),
            [
                attack(1, 4, 4, "D4", true, None),
                attack(2, 8, 4, "E4", false, prev(1, 4, 4, "D4")),
                Event::Held(Held {
                    note: 3,
                    start: 12,
                    written: 2,
                }),
                attack(4, 14, 2, "E4", false, prev(2, 8, 6, "E4")),
                // The quarter after the tied dotted half is the dactyl quarter.
                Event::Rest(Rest {
                    note: 5,
                    start: 16,
                    written: 2,
                    silence: 2,
                    prev: prev(4, 14, 2, "E4").map(|p| Prev { dactyl: true, ..p }),
                }),
                attack(6, 18, 2, "F4", true, None),
                attack(7, 20, 8, "G4", false, prev(6, 18, 2, "F4")),
                Event::End(End {
                    last: prev(7, 20, 8, "G4").unwrap(),
                    end: 28,
                    touching: true,
                }),
            ]
        );
        // Silences add up; a voice that ends with a rest does not touch its end.
        let notes = sung("d'2 r2 r1 e'1 r1");
        let events = Event::all(&notes, true);
        assert_eq!(
            events[1..3],
            [
                Event::Rest(Rest {
                    note: 1,
                    start: 4,
                    written: 4,
                    silence: 4,
                    prev: prev(0, 0, 4, "D4"),
                }),
                Event::Rest(Rest {
                    note: 2,
                    start: 8,
                    written: 8,
                    silence: 12,
                    prev: None,
                }),
            ]
        );
        assert_eq!(
            events.last(),
            Some(&Event::End(End {
                last: prev(3, 16, 8, "E4").unwrap(),
                end: 32,
                touching: false,
            }))
        );
        // The last note's event, whatever its tie says.
        let mut notes = sung("d'2 e'2");
        assert_eq!(Event::of_last(&notes).unwrap().start(), 4);
        notes[1].tie = true;
        assert_eq!(Event::of_last(&notes).unwrap().start(), 4);
        assert_eq!(Event::of_last(&sung("r1")), None);
        assert_eq!(
            Event::of_last(&sung("d'2~ d'2")),
            Some(Event::Held(Held {
                note: 1,
                start: 4,
                written: 4
            }))
        );
        assert_eq!(Event::end(&sung("r1 r1")), None);
        // The quarter after a dotted half on a minim beat is the dactyl quarter, also where a
        // half tied to a quarter makes the dotted half; after a dotted half on beat 2 too.
        let dactyl = |src: &str| match Event::of_last(&sung(src)) {
            Some(Event::Attack(a)) => a.prev.map(|p| p.dactyl),
            _ => None,
        };
        assert_eq!(dactyl("d'2. e'4 f'2"), Some(true));
        assert_eq!(dactyl("d'2~ d'4 e'4 f'2"), Some(true));
        assert_eq!(dactyl("d'2 e'2~ e'4 f'4 g'2"), Some(true));
        // Not after a dotted half off the minim beats or after a half, nor a quarter tied on.
        assert_eq!(dactyl("d'4 e'2. f'4 g'2"), Some(false));
        assert_eq!(dactyl("d'2 e'4 f'4 g'2"), Some(false));
        assert_eq!(dactyl("d'2. e'4~ e'4 f'2"), Some(false));
    }

    #[test]
    fn flags_of_kyrie() {
        let model = model(&section("kyrie"));
        let s = &model.syls;
        assert!(s[0].stressed && s[0].opening && !s[0].pair_before && s[0].pair_after);
        assert!(s[1].short_penult && !s[1].accented && s[1].pair_before && !s[1].pair_after);
        assert!(s[2].word_end && s[2].break_after && !s[2].opening);
        assert!(s[3].opening && !s[3].accented && s[3].pair_after);
        assert!(s[4].stressed && s[4].pair_before && s[4].pair_after);
        assert!(s[5].short_penult && !s[5].pair_after);
        assert!(s[6].unit_end && s[6].full_stop && s[6].last_unit);
        assert_eq!(model.targets[6], [0, 3]);
        assert!(model.targets[..6].iter().all(Vec::is_empty));
    }

    #[test]
    fn syllables_still_to_sing() {
        let model = model(&own(
            "A-gnus De-i, qui tol-lis pec-cá-ta mun-di: mi-se-ré-re no-bis.",
        ));
        let state = UState::START;
        assert_eq!((model.need(&state), model.to_stop(&state)), (18, 4));
        let sing = |model: &Model, src: &str| {
            let mut found = Vec::new();
            let notes = sung(src);
            let mut state = UState::START;
            for event in Event::all(&notes, false) {
                let Event::Attack(a) = event else { continue };
                let y = notes[a.note].lyric.unwrap().index().unwrap() as u16;
                state = model.step(&state, &event, Choice::Start(y), &mut found);
            }
            state
        };
        let state = sing(&model, "d'1=0 e'1=1 f'1=2 g'1=3");
        assert_eq!(state.syllable(), Some(3));
        assert_eq!((model.need(&state), model.to_stop(&state)), (14, 0));
        // A marked break stops the words before it: Ký-ri-e before its tail, pax before a |.
        let kyrie = Model::new(&section("kyrie"), dorian(), Meter::ALLA_BREVE, &[]);
        assert_eq!(kyrie.to_stop(&UState::START), 3);
        let state = sing(&kyrie, "d'1=0 e'1=1");
        assert_eq!((kyrie.need(&state), kyrie.to_stop(&state)), (5, 1));
        let state = sing(&kyrie, "d'1=0 e'1=1 f'1=2 g'1=3");
        assert_eq!((kyrie.need(&state), kyrie.to_stop(&state)), (3, 3));
        let broken = Model::new(
            &own("pax | pax pax, pax."),
            dorian(),
            Meter::ALLA_BREVE,
            &[],
        );
        assert_eq!(broken.to_stop(&UState::START), 1);
        assert_eq!(broken.to_stop(&sing(&broken, "d'1=0 e'1=1")), 1);
    }

    /// A voice that sings to the end needs nothing more while it holds its last syllable; once
    /// it has rested after it, it needs the shortest restatement: in Kyrie I e-lé-i-son from its
    /// tail, in Agnus Dei I mi-se-ré-re no-bis from the start of the last unit.
    #[test]
    fn voices_that_rested_at_the_end_restate() {
        let run = |model: &Model, src: &str| {
            let notes = sung(src);
            let mut found = Vec::new();
            let mut state = UState::START;
            for event in Event::all(&notes, false) {
                let choice = match event {
                    Event::Attack(a) => match notes[a.note].lyric.and_then(|l| l.index()) {
                        Some(y) => Choice::Start(y as u16),
                        None => Choice::Hold,
                    },
                    _ => Choice::Hold,
                };
                state = model.step(&state, &event, choice, &mut found);
            }
            state
        };
        let kyrie = Model::new(&section("kyrie"), dorian(), Meter::ALLA_BREVE, &[]);
        let text = "d'1=0 e'1=1 f'1=2 g'1=3 a'1=4 g'1=5 f'1=6";
        let held = run(&kyrie, text);
        assert_eq!((kyrie.need(&held), kyrie.need_sounding(&held)), (0, 0));
        let rested = run(&kyrie, &format!("{text} r1"));
        assert_eq!((kyrie.need(&rested), kyrie.need_sounding(&rested)), (0, 4));
        // Before the end a rest changes nothing.
        let early = run(&kyrie, "d'1=0 e'1=1 f'1=2 r1");
        assert_eq!((kyrie.need(&early), kyrie.need_sounding(&early)), (4, 4));
        let agnus = model(&own(
            "A-gnus De-i, qui tol-lis pec-cá-ta mun-di: mi-se-ré-re no-bis.",
        ));
        let mut whole: Vec<String> = (0..18).map(|y| format!("d'1={y}")).collect();
        whole.push("r1".to_string());
        let rested = run(&agnus, &whole.join(" "));
        assert_eq!(agnus.need_sounding(&rested), 6);
    }

    #[test]
    fn onsets() {
        let pax = model(&own("pax pax pax pax."));
        // An eighth, a white note off the minim beats, a quarter off them.
        assert_eq!(
            found(&pax, "d'2=0 e'4 f'8=1 g'8 a'2", false),
            [(Case::Onset, 6, 2)]
        );
        assert_eq!(
            found(&pax, "d'4=0 e'2=1 f'4 g'2", false),
            [(Case::Onset, 2, 1)]
        );
        assert_eq!(
            found(&pax, "d'2=0 e'4 f'4=1 g'2", false),
            [(Case::Onset, 6, 2)]
        );
        // A lyric on a rest, and on a tie continuation, where the syllable then begins.
        assert_eq!(
            found(&pax, "d'2=0 r2=1 e'2=1", false),
            [(Case::Onset, 4, 1), (Case::RestClause, 4, 1)]
        );
        assert_eq!(
            found(&pax, "d'2=0 e'2~ e'2=1 f'2", false),
            [(Case::Onset, 8, 2)]
        );
        // An entry takes its syllable on any value.
        assert_eq!(found(&pax, "r4 d'2=0 e'4 f'2", false), []);
    }

    #[test]
    fn quarters_runs_and_the_dactyl() {
        let pax = model(&own("pax pax pax pax."));
        // A run begun on beat 2.
        assert_eq!(
            found(&pax, "d'2=0 e'4=1 f'4 g'2", false),
            [(Case::OnsetRunUpbeat, 4, 1)]
        );
        // A texted quarter without a run, decided where the next note begins.
        assert_eq!(
            found(&pax, "d'1=0 e'4=1 f'2. g'2", false),
            [(Case::OnsetSingle, 10, 1)]
        );
        // The dactyl quarter takes the short middle syllable of Dó-mi-ne, and the note after it
        // a syllable too.
        let domine = model(&own("Dó-mi-ne De-us."));
        assert_eq!(
            found(&domine, "d'2.=0 e'4=1 f'1=2 g'1=3", false),
            [(Case::OnsetFigure, 6, 1), (Case::AfterBlackFigure, 8, 2)]
        );
        // Any other syllable there costs: a stressed one, a secondary accent, one that opens a
        // part of the text, the last of a unit.
        assert_eq!(
            found(&pax, "d'2.=0 e'4=1 f'2=2 g'2", false),
            [
                (Case::OnsetFigureAccent, 6, 1),
                (Case::AfterBlackFigure, 8, 2)
            ]
        );
        for (text, figure) in [
            ("pax vo-lun-tá-tis.", "d'2.=0 e'4=1 f'1=2 g'1=3 a'1=4"),
            ("pax / et pax.", "d'2.=0 e'4=1 f'1=2"),
            ("et in, pax.", "d'2.=0 e'4=1 f'1=2"),
        ] {
            let found = found(&model(&own(text)), figure, false);
            assert_eq!(found[0], (Case::OnsetFigureAccent, 6, 1), "{text}");
            assert!(!found.iter().any(|f| f.0 == Case::OnsetFigure), "{text}");
        }
        // A half or longer must follow the texted dactyl quarter.
        assert_eq!(
            found(&pax, "d'2.=0 e'4=1 f'4 g'4 a'2", false),
            [(Case::OnsetFigureAccent, 6, 1), (Case::OnsetSingle, 8, 1)]
        );
        // At the end of a complete voice.
        assert_eq!(
            found(&pax, "d'2=0 e'2=1 f'2=2 g'4=3", true),
            [
                (Case::OnsetRunUpbeat, 12, 3),
                (Case::OnsetSingle, 14, 3),
                (Case::RestShort, 14, 3),
            ]
        );
        // Without the end the quarter stays undecided.
        assert_eq!(
            found(&pax, "d'2=0 e'2=1 f'2=2 g'4=3", false),
            [(Case::OnsetRunUpbeat, 12, 3)]
        );
    }

    #[test]
    fn openings_on_black_notes() {
        let opening_black = |model: &Model, src: &str| -> Vec<(u32, usize)> {
            found(model, src, false)
                .into_iter()
                .filter(|f| f.0 == Case::OnsetOpeningBlack)
                .map(|f| (f.1, f.2))
                .collect()
        };
        let pax = model(&own("pax pax pax pax."));
        let statement = "d'1=0 e'1=1 f'1=2 g'1=3";
        // A restatement on a quarter that begins a run on the downbeat; on a half it is free.
        assert_eq!(
            found(&pax, &format!("{statement} a'4=0 b'4 c''2 b'1=1"), false),
            [(Case::RepeatShort, 32, 4), (Case::OnsetOpeningBlack, 32, 4)]
        );
        assert_eq!(
            opening_black(&pax, &format!("{statement} a'2=0 b'2 c''1=1")),
            []
        );
        // The tail of Kyrie I on a quarter on beat 2, which begins a run there too.
        let kyrie = model(&section("kyrie"));
        assert_eq!(
            found(&kyrie, "d'1=0 e'1=1 f'2=2 g'4=3 a'4 b'2 c''2=4", false)[..2],
            [
                (Case::OnsetRunUpbeat, 20, 3),
                (Case::OnsetOpeningBlack, 20, 3)
            ]
        );
        // After a rest, which frees the value of the entry, a part of the text still opens on a
        // white note; a syllable that opens nothing does not count.
        assert_eq!(
            opening_black(&pax, &format!("{statement} r2 a'4=0 b'4 c''2 b'2=1")),
            [(36, 5)]
        );
        assert_eq!(opening_black(&pax, "d'1=0 r2 e'4=1 f'4 g'1=2"), []);
        // The voice's first entry takes any value, after rests too.
        assert_eq!(opening_black(&pax, "d'4=0 e'4 f'2 g'1=1"), []);
        assert_eq!(opening_black(&pax, "r2 d'4=0 e'4 f'1=1"), []);
        // On the dactyl quarter an opening syllable is `figure_accent`.
        let tail = model(&own("pax / et pax."));
        assert_eq!(
            found(&tail, "d'2.=0 e'4=1 f'1=2", false),
            [
                (Case::OnsetFigureAccent, 6, 1),
                (Case::AfterBlackFigure, 8, 2)
            ]
        );
    }

    #[test]
    fn after_black_notes() {
        let pax = model(&own("pax pax pax pax."));
        assert_eq!(
            found(&pax, "d'2=0 e'4 f'4 g'2=1 a'2", false),
            [(Case::AfterBlack, 8, 3)]
        );
        // The last syllable of a unit on a half is a licence; on a quarter it is not.
        assert_eq!(
            found(&pax, "d'2=0 e'2=1 f'2=2 g'4 a'4 b'2=3", false),
            [(Case::AfterBlackUnitEnd, 16, 5)]
        );
        assert_eq!(
            found(&pax, "d'2=0 e'2=1 f'2=2 g'4 a'4 b'4=3 a'4", false),
            [(Case::AfterBlack, 16, 5)]
        );
        // After the dactyl quarter that keeps the syllable before it, a white note pays
        // `dotted` for a new one (alto and tenor of Palestrina's Christe in Jeppesen's print);
        // a black note there, or a white one after other quarters, pays `level`. After the
        // texted dactyl quarter it is `figure`, and the last syllable of a unit `unit_end`.
        assert_eq!(
            found(&pax, "d'2.=0 e'4 f'2=1 g'2", false),
            [(Case::AfterBlackDotted, 8, 2)]
        );
        assert_eq!(
            found(&pax, "d'1=0 e'2.=1 f'4 g'1=2", false),
            [(Case::AfterBlackDotted, 16, 3)]
        );
        assert_eq!(
            found(&pax, "d'2.=0 e'4 f'4=1 g'4 a'2", false),
            [(Case::AfterBlack, 8, 2)]
        );
        assert_eq!(
            found(&pax, "d'2=0 e'4 f'4 g'2=1 a'2", false),
            [(Case::AfterBlack, 8, 3)]
        );
        assert_eq!(
            found(&pax, "d'2.=0 e'4=1 f'2=2 g'2", false),
            [
                (Case::OnsetFigureAccent, 6, 1),
                (Case::AfterBlackFigure, 8, 2)
            ]
        );
        assert_eq!(
            found(&pax, "d'1=0 e'1=1 f'2.=2 g'4 a'1=3", false),
            [(Case::AfterBlackUnitEnd, 24, 4)]
        );
    }

    #[test]
    fn repeated_pitches() {
        let pax = model(&own("pax pax pax pax."));
        assert_eq!(
            found(&pax, "d'2=0 d'2 e'2=1", false),
            [(Case::RepeatedPitch, 4, 1)]
        );
        // After a black note it is an anticipation; with a new syllable it is fine.
        assert_eq!(found(&pax, "d'2=0 e'4 e'4 f'2", false), []);
        assert_eq!(found(&pax, "d'2=0 d'2=1", false), []);
    }

    #[test]
    fn entries_and_order() {
        let pax = model(&own("pax pax pax pax."));
        assert_eq!(
            found(&pax, "d'2=0 e'2=1 r2 f'2", false),
            [(Case::RestClause, 8, 2), (Case::Entry, 12, 3)]
        );
        assert_eq!(found(&pax, "d'2 e'2=0", false), [(Case::Entry, 0, 0)]);
        // An entry without a syllable after a rest begins one that is not in the text: the rest
        // has ended ri, whose melisma it does not continue, and e follows ri.
        let kyrie = model(&section("kyrie"));
        assert_eq!(
            found(&kyrie, "d'1=0 e'1=1 r2 f'2 g'2 a'2 g'1=2", false),
            [(Case::RestWord, 16, 2), (Case::Entry, 20, 3)]
        );
        assert_eq!(found(&pax, "d'2=0 e'2=2", false), [(Case::Order, 4, 1)]);
        assert_eq!(found(&pax, "d'2=1", false), [(Case::OrderFirst, 0, 0)]);
        // A syllable that is not in the text has no place in it: the next syllable of the text
        // follows the one before it. Its onset counts, but nothing that depends on the syllable.
        assert_eq!(found(&pax, "d'2=0 e'2=? f'2=1", false), []);
        assert_eq!(found(&pax, "d'2=? e'2=0 f'2=1", false), []);
        assert_eq!(
            found(&pax, "d'2=0 e'2=? f'2=2", false),
            [(Case::Order, 8, 2)]
        );
        assert_eq!(
            found(&pax, "d'2=0 e'4 f'4=? g'2=1", false),
            [(Case::Onset, 6, 2), (Case::AfterBlack, 8, 3)]
        );
        assert_eq!(found(&pax, "d'2=0 e'2=? r2 f'2=1 g'2 f'2", false), []);
        let dominus = model(&own("Dó-mi-nus pax."));
        assert_eq!(found(&dominus, "d'2=0 e'2=? f'2 g'1=1", false), []);
        // Its melisma counts its notes but is not judged by the syllable before it, here the
        // last of the unit; a pitch struck again on it still needs a syllable.
        assert_eq!(found(&pax, "d'2=0 e'2=1 f'2=2 g'2=3 a'2=? b'2", true), []);
        assert_eq!(
            found(&pax, "d'2=? d'2 e'1=0", false),
            [(Case::RepeatedPitch, 4, 1)]
        );
    }

    #[test]
    fn restatements() {
        let kyrie = model(&section("kyrie"));
        // Kyrie eleison, Kyrie eleison, eleison.
        let voice = "d'1=0 e'1=1 f'1=2 g'1=3 a'1=4 g'1=5 f'1=6 \
                     e'1=0 f'1=1 g'1=2 a'1=3 b'1=4 a'1=5 g'1=6 f'1=3";
        assert_eq!(
            found(&kyrie, voice, false),
            [
                (Case::RepeatShort, 56, 7),
                (Case::RepeatShort, 112, 14),
                (Case::RepeatAgain, 112, 14),
            ]
        );
        // In a long text; its last unit is restated freely, but a second restatement in the
        // sentence costs.
        let mut text = own("pax pax, pax pax.");
        text.kind = TextKind::Long;
        let long = model(&text);
        assert_eq!(
            found(&long, "d'1=0 e'1=1 f'1=0 g'1=1 a'1=2 b'1=3 a'1=2", false),
            [(Case::RepeatLong, 16, 2), (Case::RepeatAgain, 48, 6)]
        );
        let pax = model(&own("pax pax pax pax."));
        assert_eq!(
            found(
                &pax,
                "d'1=0 e'1=1 f'1=2 g'1=3 a'1=0 b'1=1 a'1=2 g'1=3",
                true
            ),
            [(Case::RepeatShort, 32, 4)]
        );
        // Lone words sung again: San-ctus, San-ctus, then the first or the second again, and
        // again. The third unit has more words, its tail is restated freely.
        let sanctus = model(&section("sanctus"));
        for restart in [0, 2] {
            assert_eq!(
                found(
                    &sanctus,
                    &format!("d'1=0 e'1=1 f'1=2 g'1=3 a'1={restart} b'1"),
                    false
                ),
                [(Case::RepeatShort, 32, 4), (Case::RepeatWord, 32, 4)]
            );
        }
        assert_eq!(
            found(&sanctus, "d'1=0 e'1=1 a'1=0 b'1=1 a'1=0 g'1=1", false),
            [
                (Case::RepeatShort, 16, 2),
                (Case::RepeatWord, 16, 2),
                (Case::RepeatShort, 32, 4),
                (Case::RepeatAgain, 32, 4),
                (Case::RepeatWord, 32, 4)
            ]
        );
        let statement = "d'1=0 e'1=1 f'1=2 g'1=3 a'1=4 g'1=5 f'1=6 e'1=7 d'1=8 e'1=9 \
                         f'1=10 g'1=11 a'1=12 g'1=13";
        for restart in [0, 4, 6] {
            assert_eq!(
                found(&sanctus, &format!("{statement} f'1={restart}"), false),
                [(Case::RepeatShort, 112, 14)],
                "{restart}"
            );
        }
        // A word that is a sentence of its own makes sense alone: A-men.
        let mut amen = own("Glo-ri-a. A-men.");
        let voice = "d'1=0 e'1=1 f'1=2 g'1=3 a'1=4 b'1=3 a'1=4";
        assert_eq!(
            found(&model(&amen), voice, false),
            [(Case::RepeatShort, 40, 5)]
        );
        amen.kind = TextKind::Long;
        assert_eq!(found(&model(&amen), voice, false), []);
        let genitum = model(&own("Gé-ni-tum, non fa-ctum."));
        assert_eq!(
            found(&genitum, "d'1=0 e'1=1 f'1=2 g'1=0 a'1=1 b'1=2", false),
            [(Case::RepeatShort, 24, 3), (Case::RepeatWord, 24, 3)]
        );
    }

    /// A long text whose words go on in the next part of its section (phase 14) ends in no Amen:
    /// a restatement of its last unit costs as one of any other unit (TXT-REPEAT `long`), among
    /// the cases of a voice and in the dynamic program. A short text restates at the level of
    /// `short` either way.
    #[test]
    fn continued_texts_restate_their_last_unit_at_a_price() {
        let mut text = own("et in, pax pax.");
        text.kind = TextKind::Long;
        let mut continued = text.clone();
        continued.continued = true;
        let voice = "d'1=0 e'1=1 f'1=2 g'1=3 a'1=2 b'1=3";
        assert_eq!(found(&model(&text), voice, false), []);
        assert_eq!(
            found(&model(&continued), voice, false),
            [(Case::RepeatLong, 32, 4)]
        );
        // The first unit costs the same either way.
        let first = "d'1=0 e'1=1 f'1=0 g'1=1 a'1=2 b'1=3";
        assert_eq!(
            found(&model(&continued), first, false),
            [(Case::RepeatLong, 16, 2)]
        );
        assert_eq!(
            found(&model(&text), first, false),
            [(Case::RepeatLong, 16, 2)]
        );
        // After the rest the voice enters with a syllable. A melisma on a function word or on
        // the last syllable of a unit and a rest after the comma cost more than a restatement,
        // so the dynamic program restates the last unit: free where the words end, at the level
        // of `long` where they go on.
        let levels = fux();
        let notes = sung("d'1 e'1 f'1 g'1 r1 a'1 b'1");
        let ends = model(&text).best(&levels, &notes).unwrap();
        let goes_on = model(&continued).best(&levels, &notes).unwrap();
        let restated = [(0, 0), (1, 1), (2, 2), (3, 3), (5, 2), (6, 3)];
        assert_eq!((ends.starts.as_slice(), ends.cost), (&restated[..], 0));
        assert_eq!(goes_on.starts, restated);
        assert_eq!(goes_on.cost, levels.of(Case::RepeatLong).cost());
        let mut short = own("et in, pax pax.");
        assert_eq!(
            found(&model(&short), voice, false),
            [(Case::RepeatShort, 32, 4)]
        );
        short.continued = true;
        assert_eq!(
            found(&model(&short), voice, false),
            [(Case::RepeatShort, 32, 4)]
        );
    }

    #[test]
    fn rests() {
        let dominus = model(&own("Dó-mi-nus pax."));
        assert_eq!(
            found(&dominus, "d'2=0 r2 e'2=1 f'2=2", false),
            [(Case::RestWord, 4, 1)]
        );
        let pax = model(&own("pax pax pax pax."));
        assert_eq!(
            found(&pax, "d'2=0 e'4 f'4 r2 g'2=1", false),
            [(Case::RestShort, 8, 2), (Case::RestClause, 8, 3)]
        );
        // A rest at a marked break or after a comma; silence longer than a half, decided by the
        // rest that makes it so. After a full stop both are free.
        let broken = model(&own("pax | pax pax pax."));
        assert_eq!(
            found(&broken, "d'2=0 r2 r2 e'2=1", false),
            [(Case::RestComma, 4, 1), (Case::RestLong, 8, 2)]
        );
        assert_eq!(
            found(&broken, "d'1=0 r1 e'1=1", false),
            [(Case::RestComma, 8, 1), (Case::RestLong, 8, 1)]
        );
        let comma = model(&own("pax, pax pax pax."));
        assert_eq!(
            found(&comma, "d'1=0 r2 e'2=1", false),
            [(Case::RestComma, 8, 1)]
        );
        let stop = model(&own("pax. pax pax pax."));
        assert_eq!(found(&stop, "d'1=0 r1 e'1=1", false), []);
        // A rest that leaves only the last word before the words stop: at the end of a unit
        // (Lau-dá-mus | te.) and before a tail (qui | ve-nit / in …); with two words left it is
        // `clause`.
        assert_eq!(
            found(&pax, "d'2=0 e'2=1 f'2=2 r2 g'2=3", false),
            [(Case::RestLastWord, 12, 3)]
        );
        let laudamus = model(&own("Lau-dá-mus te."));
        assert_eq!(
            found(&laudamus, "d'1=0 e'1=1 f'1=2 r2 g'2=3", false),
            [(Case::RestLastWord, 24, 3)]
        );
        let venit = model(&own("qui ve-nit / in nó-mi-ne Dó-mi-ni."));
        assert_eq!(
            found(&venit, "d'1=0 r2 e'2=1 f'2=2", false),
            [(Case::RestLastWord, 8, 1)]
        );
        assert_eq!(
            found(&venit, "d'1=0 e'1=1 f'1=2 g'1=3 r2 a'2=4", false),
            [(Case::RestClause, 32, 4)]
        );
        assert_eq!(
            found(
                &venit,
                "d'1=0 e'1=1 f'1=2 g'1=3 a'1=4 b'1=5 c''1=6 r2 b'2=7",
                false
            ),
            [(Case::RestLastWord, 56, 7)]
        );
    }

    #[test]
    fn complete_texts() {
        let pax = model(&own("pax pax pax pax."));
        assert_eq!(found(&pax, "d'1=0 e'1=1", true), [(Case::Complete, 8, 1)]);
        assert_eq!(found(&pax, "d'1=0 e'1=1", false), []);
        assert_eq!(found(&pax, "d'1=0 e'1=1 f'1=2 g'1=3", true), []);
    }

    #[test]
    fn melismas() {
        let kyrie = model(&section("kyrie"));
        assert_eq!(
            found(&kyrie, "d'1=0 e'1=1 f'1=2 g'1", false),
            [(Case::MelismaUnstressed, 24, 2)]
        );
        let et = model(&own("et in ter-ra pax."));
        assert_eq!(
            found(&et, "d'1=0 e'1=1 f'1", false),
            [(Case::MelismaFunctionWord, 16, 1)]
        );
        // On the last syllable of a unit, decided where a note follows one that is not black; a
        // lead-in in quarters is free.
        let pax = model(&own("pax pax pax pax."));
        assert_eq!(
            found(&pax, "d'1=0 e'1=1 f'1=2 g'2=3 a'1", true),
            [(Case::MelismaLastSyllable, 28, 3)]
        );
        assert_eq!(found(&pax, "d'1=0 e'1=1 f'1=2 g'4=3 a'4 b'1", true), [],);
        assert_eq!(
            found(&pax, "d'1=0 e'1=1 f'1=2 g'4=3 a'4 b'2 a'2", false),
            [(Case::MelismaLastSyllable, 32, 3)]
        );
        // The lead-in lies within the measure before the last note: decided at the first note
        // after it.
        assert_eq!(
            found(&pax, "d'1=0 e'1=1 f'1=2 g'4=3 a'4 b'4 c''4 d''1", true),
            []
        );
        // Its sixth and seventh notes are no lead-in and beyond `max_last`.
        assert_eq!(
            found(
                &pax,
                "d'1=0 e'1=1 f'1=2 g'4=3 a'4 b'4 c''4 d''4 e''4 d''1",
                true
            ),
            [
                (Case::MelismaLastSyllable, 34, 3),
                (Case::MelismaLongLast, 34, 8),
                (Case::MelismaLongLast, 36, 9)
            ]
        );
        // Once, however many notes follow, besides the notes beyond `max_last`; notes after a
        // rest belong to no melisma.
        assert_eq!(
            found(&pax, "d'1=0 e'1=1 f'1=2 g'2=3 a'2 b'2 a'4 g'4 f'1", true),
            [
                (Case::MelismaLastSyllable, 28, 3),
                (Case::MelismaLongLast, 40, 8)
            ]
        );
        assert_eq!(
            found(&pax, "d'1=0 e'1=1 f'1=2 g'2=3 r2 a'1 b'1", true),
            [(Case::Entry, 32, 5)]
        );
        // The eleventh note of a syllable in a short text.
        assert_eq!(
            found(
                &kyrie,
                "d'2=0 e'2 d'2 e'2 d'2 e'2 d'2 e'2 d'2 e'2 d'2",
                false
            ),
            [(Case::MelismaLongShort, 40, 10)]
        );
        // Every limit counts: 45 or 300 quarters on Ký, the notes beyond the limit cost.
        for (quarters, limit, beyond) in [
            (45, 10, 35),
            (45, 30, 15),
            (45, 31, 14),
            (45, 40, 5),
            (45, 45, 0),
            (300, 254, 46),
            (300, 255, 0),
        ] {
            let mut profile = Profile::fux();
            profile
                .apply_override(&format!("TXT-MELISMA.max_short={limit}"))
                .unwrap();
            let levels = Levels::from(&Checker::new(&profile));
            let melisma = ["d'4", "e'4"].repeat(quarters / 2 + 1)[..quarters].join(" ");
            let notes = sung(&format!("{melisma} f'1 g'1 a'1 g'1 f'1 e'1"));
            let mut notes = notes;
            notes[0].lyric = Some(Lyric::new(0));
            for (i, note) in notes[quarters..].iter_mut().enumerate() {
                note.lyric = Some(Lyric::new(i + 1));
            }
            let costly = kyrie
                .cases(&notes, true)
                .iter()
                .filter(|f| f.case == Case::MelismaLongShort && levels.level(f) != Level::Off)
                .count();
            assert_eq!(costly, beyond, "{quarters} quarters, at most {limit}");
        }
    }

    /// The most notes a syllable takes at no cost follow the syllable: an accented one by the
    /// kind of the text, an unaccented one that neither opens nor ends a unit four, an unaccented
    /// opening one five, a unit's last one five beyond a lead-in.
    #[test]
    fn melisma_limits_follow_the_syllable() {
        // The cases of TXT-MELISMA `long` that cost, as (case, tick, note).
        let beyond = |model: &Model, src: &str| -> Vec<(Case, u32, usize)> {
            found(model, src, false)
                .into_iter()
                .filter(|f| f.0.limit().is_some())
                .collect()
        };
        // Six notes on the unaccented ter of pa-ter: the fifth and the sixth cost.
        let pater = model(&own("pa-ter pax."));
        assert_eq!(
            found(&pater, "d'1=0 e'2=1 f'2 g'2 a'2 b'2 a'2 g'1=2", false),
            [
                (Case::MelismaUnstressed, 12, 1),
                (Case::MelismaLongWeak, 24, 5),
                (Case::MelismaLongWeak, 28, 6)
            ]
        );
        // As many notes on the stressed pa are free in a short text.
        assert_eq!(beyond(&pater, "d'2=0 e'2 f'2 g'2 a'2 b'2 a'1=1 g'1=2"), []);
        // In a long text up to `max_long`.
        let mut long = own("pa-ter pax.");
        long.kind = TextKind::Long;
        let long = model(&long);
        assert_eq!(
            beyond(&long, "d'2=0 e'2 f'2 g'2 a'2 b'2 a'2 g'2 f'2 e'2=1 d'1=2"),
            [(Case::MelismaLongLong, 32, 8)]
        );
        // Six notes on the e- of eleison, which opens the tail: the sixth costs.
        let kyrie = model(&section("kyrie"));
        assert_eq!(
            beyond(&kyrie, "d'1=0 e'1=1 f'1=2 g'2=3 a'2 g'2 f'2 e'2 f'2 g'1=4"),
            [(Case::MelismaLongOpening, 44, 8)]
        );
        // The last syllable of a unit after a white note: from its sixth note on.
        let pax = model(&own("pax pax."));
        assert_eq!(
            beyond(&pax, "d'1=0 e'2=1 f'2 g'2 a'2 b'2 a'2 g'1"),
            [
                (Case::MelismaLongLast, 28, 6),
                (Case::MelismaLongLast, 32, 7)
            ]
        );
        // A lead-in of six quarters and eighths within a measure into the last note: seven
        // notes, within `max_short`.
        assert_eq!(beyond(&pax, "d'1=0 e'4=1 f'8 g'8 a'4 b'8 c''8 d''1"), []);
        // Without a profile's limits nothing is counted beyond them.
        let levels = Levels::new(|_, _| Level::P1, |_, _| 255);
        assert!(
            pater
                .cases(&sung("d'1=0 e'2=1 f'2 g'2 a'2 b'2 a'2 g'1=2"), false)
                .iter()
                .filter(|f| f.case.limit().is_some())
                .all(|f| levels.level(f) == Level::Off)
        );
    }

    #[test]
    fn accents() {
        let kyrie = model(&section("kyrie"));
        assert_eq!(
            found(&kyrie, "d'1=0 e'1 f'2=1 g'2 a'1=2", false),
            [(Case::AccentShortPenult, 20, 2)]
        );
        // Decided as soon as the notes show it: by the written value of ri's first note, or by
        // its tie continuation.
        assert_eq!(
            found(&kyrie, "d'2=0 e'1=1 f'2=2", false),
            [(Case::AccentLonger, 4, 1)]
        );
        assert_eq!(
            found(&kyrie, "d'2=0 e'2~=1 e'2 f'2=2", false),
            [(Case::AccentLonger, 8, 1)]
        );
        // Also where ri begins on a tie continuation, which TXT-ONSET reports: at the next one.
        assert_eq!(
            found(&kyrie, "d'4=0 e'4~ e'2~=1 e'2 f'2=2", false),
            [(Case::Onset, 4, 2), (Case::AccentLonger, 8, 2)]
        );
        assert_eq!(
            found(&kyrie, "d'2=0 e'2=1 d'2 f'2=2", false),
            [(Case::AccentShortPenult, 8, 1), (Case::AccentLonger, 8, 1)]
        );
        assert_eq!(
            found(&kyrie, "d'1=0 e'1=1 f'1=2 g'1=3 a'2=4 g'2=5 f'1=6", false),
            [(Case::AccentOpening, 36, 3)]
        );
        let deus = model(&own("De-us."));
        assert_eq!(
            found(&deus, "r2 e'2=0 f'1=1", true),
            [(Case::AccentWeak, 8, 1)]
        );
        // A syllable a rest has ended is not compared again, and an entry after the rest without
        // a syllable is no note of it.
        assert_eq!(
            found(&kyrie, "d'1=0 e'1=1 r1 f'1 g'1=2", false),
            [
                (Case::RestWord, 16, 2),
                (Case::RestLong, 16, 2),
                (Case::Entry, 24, 3),
            ]
        );
        // A stressed syllable is weak once, though both its neighbours show it: lau, a dotted
        // half on the downbeat, dá on the dactyl quarter, mus on the next downbeat.
        let laudamus = model(&own("lau-dá-mus te."));
        assert_eq!(
            found(&laudamus, "d'2.=0 e'4=1 f'1=2 g'1=3", false),
            [
                (Case::OnsetFigureAccent, 6, 1),
                (Case::AccentOpening, 8, 0),
                (Case::AccentWeak, 8, 1),
                (Case::AfterBlackFigure, 8, 2)
            ]
        );
        // Where only the later neighbour shows it, then.
        assert_eq!(
            found(&laudamus, "d'2=0 e'2=1 f'1=2 g'1=3", false),
            [(Case::AccentWeak, 8, 1)]
        );
        // Also by a later note of the syllable.
        assert_eq!(
            found(&deus, "r2 e'2=0 f'2=1 g'2", true),
            [
                (Case::MelismaLastSyllable, 12, 2),
                (Case::AccentWeak, 12, 1)
            ]
        );
        assert_eq!(found(&deus, "r2 e'2=0 f'2=1 r2", true), []);
    }

    #[test]
    fn leaps_inside_words() {
        let kyrie = model(&section("kyrie"));
        for (to, case) in [
            ("d''", Some(Case::LeapOctave)),
            ("b'", Some(Case::LeapSixth)),
            ("a'", Some(Case::LeapFifth)),
            ("g'", Some(Case::LeapFourth)),
            ("f'", None),
        ] {
            let expected: Vec<_> = case.map(|case| (case, 8, 1)).into_iter().collect();
            assert_eq!(found(&kyrie, &format!("d'1=0 {to}1=1"), false), expected);
        }
        // Between words a leap is free.
        let pax = model(&own("pax pax pax pax."));
        assert_eq!(found(&pax, "d'1=0 d''1=1", false), []);
    }

    #[test]
    fn cadences() {
        let goal = |target| [Goal { tick: 16, target }];
        let with =
            |text: &Text, target| Model::new(text, dorian(), Meter::ALLA_BREVE, &goal(target));
        let kyrie = section("kyrie");
        // On the ri of Ký-ri-e, inside the unit.
        assert_eq!(
            found(&with(&kyrie, Step::D), "f'1=0 e'1 d'1=1", false),
            [(Case::Cadence, 16, 2)]
        );
        // By a fifth; by an octave it is no clausula.
        assert_eq!(
            found(&with(&kyrie, Step::D), "f'1=0 a'1 d'1=1", false),
            [(Case::LeapFifth, 16, 2), (Case::Cadence, 16, 2)]
        );
        assert_eq!(
            found(&with(&kyrie, Step::D), "f'1=0 d''1=1 d'1=2", false),
            [(Case::LeapSixth, 8, 1), (Case::LeapOctave, 16, 2)]
        );
        // Before a marked break as at a comma: on the e of Ký-ri-e before its tail, and before a
        // |, on the final and on a lesser degree.
        assert_eq!(
            found(&with(&kyrie, Step::D), "f'1=0 e'1=1 d'1=2", false),
            [(Case::CadencePrincipal, 16, 2)]
        );
        assert_eq!(
            found(&with(&kyrie, Step::F), "a'1=0 g'1=1 f'1=2", false),
            [(Case::CadenceComma, 16, 2)]
        );
        let broken = own("pax | pax pax pax.");
        assert_eq!(
            found(&with(&broken, Step::F), "a'1=0 g'1 f'1", false),
            [(Case::CadenceComma, 16, 2)]
        );
        // At a comma: on the final, on a lesser degree; at a full stop nothing.
        let comma = own("pax pax, pax pax.");
        assert_eq!(
            found(&with(&comma, Step::D), "f'1=0 e'1=1 d'1", false),
            [
                (Case::MelismaLastSyllable, 16, 1),
                (Case::CadencePrincipal, 16, 2)
            ]
        );
        assert_eq!(
            found(&with(&comma, Step::F), "a'1=0 g'1=1 f'1", false),
            [
                (Case::MelismaLastSyllable, 16, 1),
                (Case::CadenceComma, 16, 2)
            ]
        );
        let stop = own("pax pax. pax pax.");
        assert_eq!(
            found(&with(&stop, Step::D), "f'1=0 e'1=1 d'1", false),
            [(Case::MelismaLastSyllable, 16, 1)]
        );
        // A syllable that is not in the text tells nothing about the end of a unit.
        assert_eq!(found(&with(&kyrie, Step::D), "f'1=0 e'1=? d'1", false), []);
        assert_eq!(
            found(&with(&kyrie, Step::D), "f'1=0 e'1=1 d'1=?", false),
            []
        );
    }

    #[test]
    fn levels_of_the_profiles() {
        let levels = fux();
        assert_eq!(levels.of(Case::Onset), Level::Forbidden);
        assert_eq!(levels.of(Case::OnsetRunUpbeat), Level::P1);
        assert_eq!(levels.of(Case::RepeatShort), Level::Off);
        assert_eq!(levels.of(Case::RepeatLong), Level::P2);
        assert_eq!(levels.of(Case::AccentLonger), Level::P3);
        assert_eq!(levels.of(Case::Cadence), Level::P3);
        assert_eq!(levels.of(Case::OnsetOpeningBlack), Level::P1);
        assert_eq!(levels.of(Case::RestComma), Level::P1);
        assert_eq!(levels.of(Case::RestClause), Level::P2);
        assert_eq!(levels.of(Case::RestLastWord), Level::P3);
        assert_eq!(levels.of(Case::AfterBlackDotted), Level::P3);
        let long = |case, value| Found {
            value,
            ..Found::at(case, 0, 0)
        };
        assert_eq!(levels.level(&long(Case::MelismaLongShort, 10)), Level::Off);
        assert_eq!(levels.level(&long(Case::MelismaLongShort, 11)), Level::P1);
        assert_eq!(levels.level(&long(Case::MelismaLongLong, 8)), Level::Off);
        assert_eq!(levels.level(&long(Case::MelismaLongLong, 9)), Level::P1);
        for (case, limit) in [
            (Case::MelismaLongWeak, 4),
            (Case::MelismaLongOpening, 5),
            (Case::MelismaLongLast, 5),
        ] {
            assert_eq!(levels.level(&long(case, limit)), Level::Off, "{case:?}");
            assert_eq!(levels.level(&long(case, limit + 1)), Level::P1, "{case:?}");
        }
        // Attacks are told apart up to the largest limit of a text's syllables.
        assert_eq!(levels.attack_bound(TextKind::Short), 11);
        assert_eq!(levels.attack_bound(TextKind::Long), 9);
        assert_eq!(
            levels.price(&[
                Found::at(Case::AccentLonger, 0, 0),
                Found::at(Case::RestClause, 0, 0),
                Found::at(Case::Onset, 0, 0),
                Found::at(Case::RestWord, 0, 0),
            ]),
            (6, Some("TXT-ONSET"))
        );
        // Jeppesen's profile takes the text rules from Fux's.
        assert_eq!(Levels::from(&Checker::new(&Profile::jeppesen())), levels);
    }

    #[test]
    fn capacities() {
        let levels = fux();
        // Four measures: the cantizans holds the target into the penultimate measure (a fixed
        // tie continuation), resolves it and holds the last measure.
        let fixed = [(16, 4, true), (20, 4, false), (24, 8, false)];
        let cap = Capacity::new(32, &fixed, &levels);
        assert_eq!(
            [0, 4, 15, 16, 19, 20, 24, 28, 31].map(|t| cap.at(t)),
            [6, 5, 3, 2, 2, 2, 1, 0, 0]
        );
        // After an attack from the next beat on; after a rest or a tie continuation from the
        // event's beat on.
        assert_eq!(
            [0, 4, 15, 16, 19, 20, 24, 28, 31].map(|t| cap.after_attack(t)),
            [5, 4, 2, 2, 2, 1, 0, 0, 0]
        );
        let notes = sung("d'2 r4 e'4~ e'2");
        let events = Event::all(&notes, false);
        assert_eq!(
            events.iter().map(|e| cap.after(e)).collect::<Vec<_>>(),
            [5, 5, 4, 4]
        );
        // Where a rest may follow a black note, an entry may begin a syllable later in the beat.
        let rests = Levels::new(
            |rule, param| match (rule, param) {
                ("TXT-REST", "short") => Level::P1,
                _ => Level::Forbidden,
            },
            |_, _| 0,
        );
        let cap = Capacity::new(32, &fixed, &rests);
        assert_eq!((cap.at(0), cap.after_attack(0)), (6, 6));
        // Without the prohibition of syllables off the minim beats, every free tick counts.
        let loose = Levels::new(
            |rule, param| match (rule, param) {
                ("TXT-ONSET", "level") => Level::P1,
                _ => Level::Forbidden,
            },
            |_, _| 0,
        );
        let cap = Capacity::new(32, &fixed, &loose);
        assert_eq!(cap.at(0), 32 - 4 - 3 - 7);
        assert_eq!(cap.at(24), 1);
    }

    #[test]
    fn best_underlay_of_a_kyrie() {
        let kyrie = model(&section("kyrie"));
        let levels = fux();
        // One note per syllable.
        let notes = sung("d'1 e'1 f'1 g'1 a'1 g'1 f'1");
        let path = kyrie.best(&levels, &notes).unwrap();
        assert_eq!(
            path.starts,
            (0..7).map(|i| (i, i as u16)).collect::<Vec<_>>()
        );
        assert_eq!((path.cost, path.lateness), (0, 168));
        // One note more: the melisma goes where it is free, on the stress of eleison, and as
        // late as it can.
        let notes = sung("d'1 e'1 f'1 g'1 a'1 b'1 a'1 g'1");
        let path = kyrie.best(&levels, &notes).unwrap();
        assert_eq!(
            path.starts,
            [(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (6, 5), (7, 6)]
        );
        assert_eq!((path.cost, path.lateness), (0, 184));
        // Applied, the checker's path finds the same cost.
        let mut texted = notes.clone();
        path.apply(&mut texted);
        assert_eq!(levels.price(&kyrie.cases(&texted, true)), (0, None));
        // Too few notes for the text.
        assert!(kyrie.best(&levels, &sung("d'1 e'1 f'1")).is_none());
        let total = 24;
        let automaton = Automaton {
            model: &kyrie,
            levels: &levels,
            total,
            cap: None,
            limit: Some(LIMIT),
            pins: None,
            gates: None,
        };
        assert_eq!(automaton.run(&sung("d'1 e'1 f'1")), Err("TXT-COMPLETE"));
        // Notes struck again after a half must each take a syllable: the eighth restates the
        // text, which then ends too early. The rule that cut the most states is named.
        let automaton = Automaton {
            total: 32,
            ..automaton
        };
        assert_eq!(
            automaton.run(&sung("d'2 d'2 d'2 d'2 d'2 d'2 d'2 d'2")),
            Err("TXT-COMPLETE")
        );
        // A rest inside Dó-mi-nus.
        let dominus = model(&own("Dó-mi-nus."));
        let automaton = Automaton {
            model: &dominus,
            total: 16,
            ..automaton
        };
        assert_eq!(automaton.run(&sung("d'2 r2 e'2 f'2")), Err("TXT-REST"));
    }

    /// Random levels, also for the cases the automaton never offers whatever their level. They
    /// take one number of `rng`, however many cases there are.
    fn random_levels(rng: &mut Rng) -> Levels {
        const LEVELS: [Level; 5] = [
            Level::Off,
            Level::P1,
            Level::P2,
            Level::P3,
            Level::Forbidden,
        ];
        let rng = &mut Rng::new(rng.next_u64());
        let picks: Vec<Level> = Case::ALL
            .iter()
            .map(|_| LEVELS[below(rng, LEVELS.len())])
            .collect();
        let limits: Vec<i64> = (0..5).map(|_| 1 + below(rng, 4) as i64).collect();
        Levels::new(
            |rule, param| {
                let i = Case::ALL
                    .iter()
                    .position(|c| c.rule() == rule && c.param() == param)
                    .unwrap();
                picks[i]
            },
            |_, param| match param {
                "max_short" => limits[0],
                "max_long" => limits[1],
                "max_weak" => limits[2],
                "max_opening" => limits[3],
                _ => limits[4],
            },
        )
    }

    /// Random internal cadences at the measures of a voice.
    fn random_goals(rng: &mut Rng, measures: usize) -> Vec<Goal> {
        const TARGETS: [Step; 5] = [Step::D, Step::A, Step::F, Step::G, Step::E];
        let mut goals = Vec::new();
        for m in 1..measures {
            if below(rng, 2) == 0 {
                goals.push(Goal {
                    tick: m as u32 * 8,
                    target: TARGETS[below(rng, TARGETS.len())],
                });
            }
        }
        goals
    }

    /// Whether the automaton offers no choice that leads to this case, whatever its level.
    fn by_construction(case: Case) -> bool {
        matches!(
            case,
            Case::Entry | Case::Order | Case::OrderFirst | Case::Complete
        )
    }

    /// The cost of a step's cases, unless one is forbidden or never offered.
    fn allowed(levels: &Levels, found: &[Found]) -> Option<u32> {
        if found.iter().any(|f| by_construction(f.case)) {
            return None;
        }
        match levels.price(found) {
            (cost, None) => Some(cost),
            _ => None,
        }
    }

    /// The least (cost, lateness) of all underlays, trying every syllable at every attack.
    fn brute(
        model: &Model,
        levels: &Levels,
        state: UState,
        events: &[Event],
        end: &Event,
        (cost, late): (u32, u32),
    ) -> Option<(u32, u32)> {
        let mut found = Vec::new();
        let Some((event, rest)) = events.split_first() else {
            model.step(&state, end, Choice::Hold, &mut found);
            return allowed(levels, &found).map(|add| (cost + add, late));
        };
        let mut choices = vec![Choice::Hold];
        if let Event::Attack(_) = event {
            choices.extend((0..model.syllables() as u16).map(Choice::Start));
        }
        let mut best: Option<(u32, u32)> = None;
        for choice in choices {
            found.clear();
            let next = model.step(&state, event, choice, &mut found);
            let Some(add) = allowed(levels, &found) else {
                continue;
            };
            let late = match choice {
                Choice::Start(_) => late + event.start(),
                Choice::Hold => late,
            };
            if let Some(r) = brute(model, levels, next, rest, end, (cost + add, late)) {
                best = Some(best.map_or(r, |b| b.min(r)));
            }
        }
        best
    }

    fn attacks(notes: &[Note]) -> usize {
        Event::all(notes, false)
            .iter()
            .filter(|e| matches!(e, Event::Attack(_)))
            .count()
    }

    /// The cases that phase 12's reviews added or changed, which the property tests must meet.
    const REVIEWED: [Case; 14] = [
        Case::OnsetFigure,
        Case::OnsetFigureAccent,
        Case::OnsetOpeningBlack,
        Case::AfterBlackDotted,
        Case::CadenceComma,
        Case::RepeatWord,
        Case::RestComma,
        Case::RestLastWord,
        Case::MelismaLastSyllable,
        Case::MelismaLongWeak,
        Case::MelismaLongOpening,
        Case::MelismaLongLast,
        Case::AccentLonger,
        Case::AccentWeak,
    ];

    /// Asserts that the cases met include every case of [`REVIEWED`].
    fn assert_met(met: &[Case], test: &str) {
        for case in REVIEWED {
            let n = met.iter().filter(|&&c| c == case).count();
            assert!(n > 0, "{test}: no {case:?}");
        }
    }

    #[test]
    fn dp_matches_brute_force() {
        let mut rng = Rng::new(12);
        let mut feasible = 0;
        let mut met = Vec::new();
        for case in 0..400 {
            // Now and then lone words on one pitch: a note struck again after a white one needs a
            // syllable, so the voice sings words again.
            let lone = case % 8 == 7;
            let text = match lone {
                true => own("San-ctus, San-ctus."),
                false => random_text(&mut rng, 5),
            };
            let mut notes = loop {
                let measures = 1 + below(&mut rng, 3);
                let held = below(&mut rng, 2) == 0;
                let notes = random_voice(&mut rng, measures, true, held);
                if (1..=10).contains(&attacks(&notes)) {
                    break notes;
                }
            };
            if lone {
                for note in notes.iter_mut().filter(|n| !n.is_rest()) {
                    note.pitch = Some(pitch("D4"));
                }
            }
            let measures = notes.iter().map(|n| n.duration.ticks()).sum::<u32>() as usize / 8;
            let levels = if case % 2 == 0 || lone {
                fux()
            } else {
                random_levels(&mut rng)
            };
            let goals = random_goals(&mut rng, measures);
            // Now and then what the random voices seldom bring about at the least cost: under
            // random levels a white note after the dactyl quarter taking the next syllable and a
            // cadence before a marked break; under Fux's levels lone words restated on one
            // pitch, a rest before a unit's last word, and melismas beyond the limits of an
            // unaccented syllable, of an unaccented opening one and of a unit's last one.
            let (text, notes, goals) = match case % 16 {
                3 => (own("pax pax pax."), sung("d'2. e'4 f'2 g'2"), Vec::new()),
                4 => (
                    own("San-ctus, San-ctus."),
                    sung("d'1 d'1 d'1 d'1 d'1 d'1"),
                    Vec::new(),
                ),
                6 => (own("pax pax pax."), sung("d'1 e'1 r1 f'1"), Vec::new()),
                8 => (
                    own("pa-ter pax."),
                    sung("d'1 e'4 f'4 g'4 a'4 b'4 a'4 g'2 f'1"),
                    Vec::new(),
                ),
                10 => (
                    own("ad te pax."),
                    sung("d'4 e'4 f'4 g'4 a'4 b'4 c''2 b'1 a'1"),
                    Vec::new(),
                ),
                12 => (
                    own("pax pax."),
                    sung("d'1 e'1 f'4 g'4 a'4 b'4 c''4 b'4 a'4 g'4 f'1"),
                    Vec::new(),
                ),
                11 => (
                    own("pax | pax."),
                    sung("g'1 f'1 g'1"),
                    vec![Goal {
                        tick: 8,
                        target: Step::F,
                    }],
                ),
                _ => (text, notes, goals),
            };
            let model = Model::new(&text, dorian(), Meter::ALLA_BREVE, &goals);
            let mut events = Event::all(&notes, true);
            let end = events.pop().unwrap();
            let expected = brute(&model, &levels, UState::START, &events, &end, (0, 0));
            let path = model.best(&levels, &notes);
            let context = || format!("case {case}: {} / {notes:?}", text.render(true, true));
            assert_eq!(
                path.as_ref().map(|p| (p.cost, p.lateness)),
                expected,
                "{}",
                context()
            );
            if let Some(path) = path {
                feasible += 1;
                // The checker's path finds the cost of the dynamic program's underlay.
                let mut texted = notes.clone();
                path.apply(&mut texted);
                let cases = model.cases(&texted, true);
                assert_eq!(levels.price(&cases), (path.cost, None), "{}", context());
                met.extend(cases.iter().map(|f| f.case));
            }
        }
        assert!(feasible >= 100, "{feasible} feasible");
        assert_met(&met, "dp_matches_brute_force");
    }

    #[test]
    fn automaton_equals_dp() {
        let mut rng = Rng::new(7);
        let mut accepted = 0;
        for case in 0..200 {
            let text = match case % 8 {
                0 | 4 => section("kyrie"),
                1 | 5 => section("christe"),
                7 => own("San-ctus, San-ctus, San-ctus."),
                _ => random_text(&mut rng, 8),
            };
            let measures = 2 + below(&mut rng, 7);
            let notes = random_voice(&mut rng, measures, case % 3 != 0, true);
            let levels = if case % 3 == 0 {
                random_levels(&mut rng)
            } else {
                fux()
            };
            let goals = random_goals(&mut rng, measures);
            let model = Model::new(&text, dorian(), Meter::ALLA_BREVE, &goals);
            let automaton = Automaton {
                model: &model,
                levels: &levels,
                total: measures as u32 * 8,
                cap: None,
                limit: None,
                pins: None,
                gates: None,
            };
            let run = automaton.run(&notes).ok().map(|set| (set[0].1, set[0].2));
            let path = model.best(&levels, &notes).map(|p| (p.cost, p.lateness));
            assert_eq!(run, path, "case {case}: {}", text.render(true, true));
            accepted += usize::from(run.is_some());
        }
        assert!(accepted >= 40, "{accepted} accepted");
    }

    /// A set keeps what sorting it in full and cutting it to the limit keeps, and its cheapest
    /// state comes first, with or without a limit.
    #[test]
    fn sets_keep_the_cheapest() {
        let mut rng = Rng::new(9);
        for case in 0..500 {
            let len = below(&mut rng, 300);
            // Distinct keys, as after merging; costs and lateness with many ties.
            let entries: Vec<Entry> = (0..len)
                .map(|i| {
                    let state = UState {
                        syl: i as u16,
                        ..UState::START
                    };
                    let cost = below(&mut rng, 6) as u32;
                    let late = below(&mut rng, 4) as u32;
                    (state.key(), cost, late, state)
                })
                .collect();
            let mut sorted = entries.clone();
            sorted.sort_unstable_by_key(order);
            for limit in [None, Some(0), Some(1), Some(16), Some(LIMIT)] {
                let mut kept = entries.clone();
                select(&mut kept, limit);
                let n = limit.map_or(len, |limit| limit.min(len));
                assert_eq!(kept.len(), n, "case {case}, limit {limit:?}");
                if n > 0 {
                    assert_eq!(kept[0], sorted[0], "case {case}, limit {limit:?}");
                }
                kept.sort_unstable_by_key(order);
                assert_eq!(kept, sorted[..n], "case {case}, limit {limit:?}");
            }
        }
    }

    /// The bound the automaton cuts with ([`Capacity::after`]) cuts no state that could still
    /// finish, by an exhaustive search of each cut state's continuations; also where it counts
    /// from the beat after an attack.
    #[test]
    fn capacity_never_cuts_a_finishable_state() {
        let cases = std::env::var("CAPACITY_CASES").map_or(150, |n| n.parse().unwrap());
        let mut rng = Rng::new(3);
        let (mut checked, mut cut, mut tighter) = (0, 0, 0);
        for case in 0..cases {
            let text = match case % 3 {
                0 => section("kyrie"),
                _ => random_text(&mut rng, 8),
            };
            let measures = 2 + below(&mut rng, 5);
            let notes = random_voice(&mut rng, measures, true, true);
            let total = measures as u32 * 8;
            // The held last measure and some notes of the voice are fixed.
            let mut fixed = vec![(total - 8, 8, false)];
            let mut start = 0;
            for (i, note) in notes.iter().enumerate() {
                if below(&mut rng, 4) == 0 {
                    let tied_in = matches!(Event::at(&notes, i, start), Some(Event::Held(_)));
                    fixed.push((start, note.duration.ticks(), tied_in));
                }
                start += note.duration.ticks();
            }
            let levels = match case % 3 {
                2 => random_levels(&mut rng),
                _ => fux(),
            };
            let cap = Capacity::new(total, &fixed, &levels);
            let model = Model::new(&text, dorian(), Meter::ALLA_BREVE, &[]);
            let mut events = Event::all(&notes, true);
            let end = events.pop().unwrap();
            let mut layer = vec![UState::START];
            let (mut found, mut choices) = (Vec::new(), Vec::new());
            for (i, event) in events.iter().enumerate() {
                let mut next = Vec::new();
                for state in &layer {
                    model.options(state, event, &mut choices);
                    for &choice in &choices {
                        found.clear();
                        let t = model.step(state, event, choice, &mut found);
                        if levels.price(&found).1.is_none() {
                            next.push(model.canonical(&t, &levels));
                        }
                    }
                }
                next.sort_unstable();
                next.dedup();
                for state in &next {
                    checked += 1;
                    if model.need(state) > cap.after(event) {
                        cut += 1;
                        tighter += usize::from(model.need(state) <= cap.at(event.start()));
                        assert!(
                            model
                                .search(&levels, *state, &events[i + 1..], &end)
                                .is_none(),
                            "case {case}: {state:?} at {} cut",
                            event.start()
                        );
                    }
                }
                layer = next;
            }
        }
        assert!(
            cut >= 100 && tighter >= 30 && checked >= 2000,
            "{cut} of {checked} cut, {tighter} from the beat after an attack"
        );
        println!("{cut} of {checked} states cut, {tighter} from the beat after an attack");
    }

    #[test]
    fn cases_of_a_prefix_are_a_prefix() {
        let mut rng = Rng::new(5);
        let mut met = Vec::new();
        for case in 0..300 {
            let text = match case % 3 {
                0 => section("kyrie"),
                _ => random_text(&mut rng, 8),
            };
            let measures = 1 + below(&mut rng, 6);
            let mut notes = random_voice(&mut rng, measures, true, false);
            let goals = random_goals(&mut rng, measures);
            let model = Model::new(&text, dorian(), Meter::ALLA_BREVE, &goals);
            // Lyrics on most notes: mostly the next syllable, sometimes a restart, a wrong or an
            // unknown one, also on rests and tie continuations.
            let syllables = model.syllables();
            let mut next = 0;
            for note in &mut notes {
                if below(&mut rng, 3) == 0 {
                    continue;
                }
                let y = match below(&mut rng, 8) {
                    0 => below(&mut rng, syllables),
                    1 => 0,
                    _ => next.min(syllables - 1),
                };
                note.lyric = Some(if below(&mut rng, 20) == 0 {
                    Lyric::UNKNOWN
                } else {
                    Lyric::new(y)
                });
                next = y + 1;
            }
            let whole = model.cases(&notes, false);
            let mut start = 0;
            for i in 0..notes.len() {
                let expected: Vec<Found> =
                    whole.iter().filter(|f| f.tick <= start).copied().collect();
                // As the checker sees a prefix, with the tie of its last note, and as the
                // search sees it, before the tie is decided.
                let mut prefix = notes[..=i].to_vec();
                assert_eq!(
                    model.cases(&prefix, false),
                    expected,
                    "case {case}, note {i}"
                );
                prefix[i].tie = false;
                assert_eq!(
                    model.cases(&prefix, false),
                    expected,
                    "case {case}, note {i}"
                );
                assert_eq!(Event::of_last(&prefix), Event::at(&notes, i, start));
                start += notes[i].duration.ticks();
            }
            // The end adds cases after all others.
            let complete = model.cases(&notes, true);
            assert!(complete.starts_with(&whole), "case {case}");
            met.extend(complete.iter().map(|f| f.case));
        }
        assert_met(&met, "cases_of_a_prefix_are_a_prefix");
    }

    /// What an underlay does at an attack: the syllable it begins there, or a melisma.
    fn laid_at(path: &Path, note: usize) -> Choice {
        path.starts
            .iter()
            .find(|&&(n, _)| n == note)
            .map_or(Choice::Hold, |&(_, y)| Choice::Start(y))
    }

    /// The attacks, by note, at which an underlay does not take the pinned choice.
    fn broken(notes: &[Note], path: &Path, pins: &Pins) -> Vec<usize> {
        Event::all(notes, false)
            .iter()
            .filter_map(|event| match (event, pinned(Some(pins), event)) {
                (Event::Attack(a), Some(pin)) if laid_at(path, a.note) != pin => Some(a.note),
                _ => None,
            })
            .collect()
    }

    /// A melisma or any syllable.
    fn random_choice(rng: &mut Rng, syllables: usize) -> Choice {
        match below(rng, 3) {
            0 => Choice::Hold,
            _ => Choice::Start(below(rng, syllables) as u16),
        }
    }

    /// Pins on about a third of the attacks of a voice: what an underlay of it does there, in
    /// every other voice with one pin changed; random ones without an underlay.
    fn random_pins(
        rng: &mut Rng,
        notes: &[Note],
        path: Option<&Path>,
        syllables: usize,
    ) -> Vec<(u32, Choice)> {
        let mut pins = Vec::new();
        for event in Event::all(notes, false) {
            let Event::Attack(a) = event else { continue };
            if below(rng, 3) != 0 {
                continue;
            }
            let choice = match path {
                Some(path) => laid_at(path, a.note),
                None => random_choice(rng, syllables),
            };
            pins.push((a.start, choice));
        }
        if path.is_some() && !pins.is_empty() && below(rng, 2) == 0 {
            let i = below(rng, pins.len());
            pins[i].1 = random_choice(rng, syllables);
        }
        pins
    }

    /// Now and then a syllable inside a unit after which a voice may restate, as before a later
    /// point of an imitation ([`Model::with_restarts`]).
    fn random_restarts(rng: &mut Rng, text: &Text) -> Vec<u16> {
        let inner: Vec<u16> = (0..text.syllables.len().saturating_sub(1))
            .filter(|&i| !text.syllables[i].flags.contains(SylFlags::UNIT_END))
            .map(|i| i as u16)
            .collect();
        if inner.is_empty() || below(rng, 2) == 0 {
            return Vec::new();
        }
        vec![inner[below(rng, inner.len())]]
    }

    /// Gates of up to two points for a voice of `total` ticks on a text of `syllables`: in half
    /// the cases none, otherwise syllables from the second on with ticks on minim beats, neither
    /// falling from one gate to the next.
    fn random_gates(rng: &mut Rng, syllables: usize, total: u32) -> Vec<(u16, u32)> {
        if syllables < 2 || below(rng, 2) == 0 {
            return Vec::new();
        }
        let mut gates: Vec<(u16, u32)> = (0..1 + below(rng, 2))
            .map(|_| {
                let y = 1 + below(rng, syllables - 1);
                (y as u16, 4 * below(rng, total as usize / 4 + 1) as u32)
            })
            .collect();
        gates.sort_unstable();
        gates.dedup_by_key(|g| g.0);
        for i in 1..gates.len() {
            gates[i].1 = gates[i].1.max(gates[i - 1].1);
        }
        gates
    }

    /// The attacks, by note, at which an underlay begins a syllable before its gate.
    fn early_starts(notes: &[Note], path: &Path, gates: &Gates) -> Vec<usize> {
        Event::all(notes, false)
            .iter()
            .filter_map(|event| match event {
                Event::Attack(a) if early(Some(gates), event, laid_at(path, a.note)) => {
                    Some(a.note)
                }
                _ => None,
            })
            .collect()
    }

    /// Fixed notes of a capacity that fit a voice, as the search holds them: its held last
    /// measure, some of its notes, the silence before its first note and, before some entries
    /// after a rest, a window of the rest.
    fn fitting(rng: &mut Rng, notes: &[Note], total: u32) -> Vec<(u32, u32, bool)> {
        let mut fixed = vec![(total - 8, 8, false)];
        let (mut start, mut silence) = (0, 0);
        for (i, note) in notes.iter().enumerate() {
            let ticks = note.duration.ticks();
            if note.is_rest() {
                silence += ticks;
            } else {
                if silence > 0 && (start == silence || below(rng, 2) == 0) {
                    let window = if start == silence {
                        silence
                    } else {
                        silence.min(4 * (1 + below(rng, 2) as u32))
                    };
                    fixed.push((start - window, window, true));
                }
                silence = 0;
                if below(rng, 4) == 0 {
                    let tied_in = matches!(Event::at(notes, i, start), Some(Event::Held(_)));
                    fixed.push((start, ticks, tied_in));
                }
            }
            start += ticks;
        }
        fixed
    }

    /// The least (cost, lateness) of all underlays that keep the pins and the gates, trying every
    /// syllable at every attack: of a complete voice, whose end is stepped, or of a head, which
    /// sings up to its last syllable and no further.
    fn brute_pinned(
        model: &Model,
        levels: &Levels,
        state: UState,
        events: &[Event],
        finish: Finish<'_>,
        (pins, gates): (&Pins, &Gates),
        (cost, late): (u32, u32),
    ) -> Option<(u32, u32)> {
        let mut found = Vec::new();
        let Some((event, rest)) = events.split_first() else {
            return match finish {
                Finish::End(end) => {
                    model.step(&state, end, Choice::Hold, &mut found);
                    allowed(levels, &found).map(|add| (cost + add, late))
                }
                Finish::Head(last) => {
                    (state.syllable() == Some(usize::from(last))).then_some((cost, late))
                }
            };
        };
        let mut choices = vec![Choice::Hold];
        if let Event::Attack(_) = event {
            choices.extend((0..model.syllables() as u16).map(Choice::Start));
        }
        if let Some(pin) = pinned(Some(pins), event) {
            choices.retain(|&c| c == pin);
        }
        choices.retain(|&c| !early(Some(gates), event, c));
        if let Finish::Head(last) = finish {
            choices.retain(|&c| !matches!(c, Choice::Start(y) if y > last));
        }
        let mut best: Option<(u32, u32)> = None;
        for choice in choices {
            found.clear();
            let next = model.step(&state, event, choice, &mut found);
            let Some(add) = allowed(levels, &found) else {
                continue;
            };
            let late = match choice {
                Choice::Start(_) => late + event.start(),
                Choice::Hold => late,
            };
            let bounds = (pins, gates);
            let r = brute_pinned(
                model,
                levels,
                next,
                rest,
                finish,
                bounds,
                (cost + add, late),
            );
            if let Some(r) = r {
                best = Some(best.map_or(r, |b| b.min(r)));
            }
        }
        best
    }

    /// With pins and gates, the dynamic program finds what trying every syllable at every attack
    /// while keeping the pins and the gates finds. Its underlay keeps them, the checker finds its
    /// cost, and pins and gates that the cheapest underlay keeps leave it the cheapest.
    #[test]
    fn pinned_dp_matches_brute_force() {
        let mut rng = Rng::new(31);
        let (mut feasible, mut infeasible, mut kept, mut gated) = (0, 0, 0, 0);
        for case in 0..400 {
            let text = match case % 8 {
                3 => section("kyrie"),
                7 => own("San-ctus, San-ctus."),
                _ => random_text(&mut rng, 5),
            };
            let levels = if case % 2 == 0 {
                fux()
            } else {
                random_levels(&mut rng)
            };
            let restarts = random_restarts(&mut Rng::new(1000 + case as u64), &text);
            // Mostly a voice that has an underlay, so that most pins come from one.
            let mut tries = 0;
            let (notes, model, free) = loop {
                let measures = 1 + below(&mut rng, 3);
                let held = below(&mut rng, 2) == 0;
                let notes = random_voice(&mut rng, measures, true, held);
                if !(1..=10).contains(&attacks(&notes)) {
                    continue;
                }
                let goals = random_goals(&mut rng, measures);
                let model = Model::new(&text, dorian(), Meter::ALLA_BREVE, &goals)
                    .with_restarts(&text, &restarts);
                let free = model.best(&levels, &notes);
                tries += 1;
                if free.is_some() || case % 4 == 0 || tries == 20 {
                    break (notes, model, free);
                }
            };
            let pins = random_pins(&mut rng, &notes, free.as_ref(), model.syllables());
            let total: u32 = notes.iter().map(|n| n.duration.ticks()).sum();
            let gates = random_gates(&mut rng, model.syllables(), total);
            gated += usize::from(!gates.is_empty());
            let mut events = Event::all(&notes, true);
            let end = events.pop().unwrap();
            let finish = Finish::End(&end);
            let expected = brute_pinned(
                &model,
                &levels,
                UState::START,
                &events,
                finish,
                (&pins, &gates),
                (0, 0),
            );
            let path = model.best_pinned(&levels, &notes, Some(&pins), Some(&gates));
            let context = format!(
                "case {case}: {} / {notes:?} / {pins:?} / {gates:?}",
                text.render(true, true)
            );
            assert_eq!(
                path.as_ref().map(|p| (p.cost, p.lateness)),
                expected,
                "{context}"
            );
            let Some(path) = path else {
                infeasible += 1;
                continue;
            };
            feasible += 1;
            assert_eq!(broken(&notes, &path, &pins), [], "{context}");
            assert_eq!(early_starts(&notes, &path, &gates), [], "{context}");
            let mut texted = notes.clone();
            path.apply(&mut texted);
            assert_eq!(
                levels.price(&model.cases(&texted, true)),
                (path.cost, None),
                "{context}"
            );
            if let Some(free) = free.filter(|free| {
                broken(&notes, free, &pins).is_empty()
                    && early_starts(&notes, free, &gates).is_empty()
            }) {
                kept += 1;
                assert_eq!(
                    (path.cost, path.lateness),
                    (free.cost, free.lateness),
                    "{context}"
                );
            }
        }
        assert!(
            feasible >= 120 && infeasible >= 120 && kept >= 100 && gated >= 150,
            "{feasible} feasible, {infeasible} not, {kept} keeping the cheapest, {gated} gated"
        );
    }

    /// With pins and gates, the automaton keeps the cheapest underlay of the dynamic program:
    /// without a capacity, and with a capacity that fits the voice, whose milestones then cut
    /// too.
    #[test]
    fn pinned_automaton_equals_dp() {
        let mut rng = Rng::new(19);
        let (mut accepted, mut refused) = (0, 0);
        for case in 0..300 {
            let text = match case % 8 {
                0 | 4 => section("kyrie"),
                1 | 5 => section("christe"),
                7 => own("San-ctus, San-ctus, San-ctus."),
                _ => random_text(&mut rng, 8),
            };
            let levels = if case % 3 == 0 {
                random_levels(&mut rng)
            } else {
                fux()
            };
            let restarts = random_restarts(&mut Rng::new(1000 + case as u64), &text);
            // Mostly a voice that has an underlay, so that most pins come from one.
            let mut tries = 0;
            let (notes, model, free) = loop {
                let measures = 2 + below(&mut rng, 7);
                let notes = random_voice(&mut rng, measures, case % 3 != 0, true);
                let goals = random_goals(&mut rng, measures);
                let model = Model::new(&text, dorian(), Meter::ALLA_BREVE, &goals)
                    .with_restarts(&text, &restarts);
                let free = model.best(&levels, &notes);
                tries += 1;
                if free.is_some() || case % 4 == 0 || tries == 20 {
                    break (notes, model, free);
                }
            };
            let total: u32 = notes.iter().map(|n| n.duration.ticks()).sum();
            let pins = random_pins(&mut rng, &notes, free.as_ref(), model.syllables());
            let gates = random_gates(&mut rng, model.syllables(), total);
            let cap = Capacity::new(total, &fitting(&mut rng, &notes, total), &levels);
            let path = model
                .best_pinned(&levels, &notes, Some(&pins), Some(&gates))
                .map(|p| (p.cost, p.lateness));
            for cap in [None, Some(&cap)] {
                let automaton = Automaton {
                    model: &model,
                    levels: &levels,
                    total,
                    cap,
                    limit: None,
                    pins: Some(&pins),
                    gates: Some(&gates),
                };
                let run = automaton.run(&notes).ok().map(|set| (set[0].1, set[0].2));
                assert_eq!(
                    run,
                    path,
                    "case {case}, capacity {}: {} / {pins:?} / {gates:?}",
                    cap.is_some(),
                    text.render(true, true)
                );
            }
            match path {
                Some(_) => accepted += 1,
                None => refused += 1,
            }
        }
        assert!(
            accepted >= 100 && refused >= 100,
            "{accepted} accepted, {refused} refused"
        );
    }

    /// The milestone cut ([`Automaton::advance`]) cuts no state that could still finish with the
    /// pins and the gates, by an exhaustive search of each cut state's continuations, and the
    /// automaton with its capacity, pins and gates keeps every state that can still finish: in
    /// voices with rests, rest windows in the capacity, pins from an underlay they have or
    /// random ones, gates, and restatements.
    #[test]
    fn milestones_never_cut_a_finishable_state() {
        let cases = std::env::var("MILESTONE_CASES").map_or(600, |n| n.parse().unwrap());
        let mut rng = Rng::new(37);
        let (mut checked, mut cut, mut beyond, mut restated, mut lost) = (0, 0, 0, 0, 0);
        let (mut kept, mut gated) = (0, 0);
        let (mut found, mut choices) = (Vec::new(), Vec::new());
        for case in 0..cases {
            // Short texts, restated in longer voices, and texts of two sentences.
            let text = match case % 5 {
                0 => section("kyrie"),
                1 => own("pax pax, pax. pax pax."),
                2 => own("Glo-ri-a. A-men."),
                _ => random_text(&mut rng, 8),
            };
            let measures = 2 + below(&mut rng, 5);
            let notes = random_voice(&mut rng, measures, true, true);
            let total = measures as u32 * 8;
            let levels = match case % 3 {
                2 => random_levels(&mut rng),
                _ => fux(),
            };
            let restarts = random_restarts(&mut Rng::new(1000 + case as u64), &text);
            let model =
                Model::new(&text, dorian(), Meter::ALLA_BREVE, &[]).with_restarts(&text, &restarts);
            let free = model.best(&levels, &notes);
            let pins = random_pins(&mut rng, &notes, free.as_ref(), model.syllables());
            let gates = random_gates(&mut rng, model.syllables(), total);
            gated += usize::from(!gates.is_empty());
            let cap = Capacity::new(total, &fitting(&mut rng, &notes, total), &levels);
            let mut events = Event::all(&notes, true);
            let end = events.pop().unwrap();
            // Every state the notes allow with the pins and the gates: before the first event and
            // after each.
            let mut layers = vec![vec![UState::START]];
            for event in &events {
                let pin = pinned(Some(&pins), event);
                let mut next = Vec::new();
                for state in &layers[layers.len() - 1] {
                    model.options(state, event, &mut choices);
                    if let Some(pin) = pin {
                        choices.retain(|&c| c == pin);
                    }
                    choices.retain(|&c| !early(Some(&gates), event, c));
                    for &choice in &choices {
                        found.clear();
                        let t = model.step(state, event, choice, &mut found);
                        if levels.price(&found).1.is_none() {
                            next.push(model.canonical(&t, &levels));
                        }
                    }
                }
                next.sort_unstable();
                next.dedup();
                layers.push(next);
            }
            // Those among them that can still finish, from the end back.
            let mut finishable = vec![Vec::new(); layers.len()];
            finishable[events.len()] = layers[events.len()]
                .iter()
                .copied()
                .filter(|state| {
                    found.clear();
                    model.step(state, &end, Choice::Hold, &mut found);
                    !found.iter().any(|f| f.case == Case::Complete)
                        && levels.price(&found).1.is_none()
                })
                .collect();
            for (i, event) in events.iter().enumerate().rev() {
                let pin = pinned(Some(&pins), event);
                finishable[i] = layers[i]
                    .iter()
                    .copied()
                    .filter(|state| {
                        model.options(state, event, &mut choices);
                        choices
                            .iter()
                            .filter(|&&c| pin.is_none_or(|pin| pin == c))
                            .filter(|&&c| !early(Some(&gates), event, c))
                            .any(|&choice| {
                                found.clear();
                                let t = model.step(state, event, choice, &mut found);
                                levels.price(&found).1.is_none()
                                    && finishable[i + 1]
                                        .binary_search(&model.canonical(&t, &levels))
                                        .is_ok()
                            })
                    })
                    .collect();
            }
            let automaton = Automaton {
                model: &model,
                levels: &levels,
                total,
                cap: Some(&cap),
                limit: None,
                pins: Some(&pins),
                gates: Some(&gates),
            };
            let mut set = Ok(Automaton::start());
            let mut i = 0;
            for k in 0..notes.len() {
                set = set.and_then(|set| automaton.advance(&set, &notes[..=k]));
                let Some(event) = Event::of_last(&notes[..=k]) else {
                    continue;
                };
                assert_eq!(event, events[i], "case {case}, note {k}");
                i += 1;
                let context = format!(
                    "case {case}, note {k}: {} / {pins:?} / {gates:?}",
                    text.render(true, true)
                );
                if let Some((tick, y)) = next_start(&pins, event.start()) {
                    let room = cap.after(&event).saturating_sub(cap.at(tick));
                    for state in &layers[i] {
                        checked += 1;
                        let reach = model.to_reach(state, y);
                        if reach.is_some_and(|n| n <= room) {
                            continue;
                        }
                        cut += 1;
                        beyond += usize::from(model.need(state) <= cap.after(&event));
                        restated += usize::from(state.syllable() >= Some(usize::from(y)));
                        lost += usize::from(reach.is_none());
                        let rest = &events[i..];
                        assert!(
                            finishable[i].binary_search(state).is_err()
                                && model
                                    .search_with(
                                        &levels,
                                        *state,
                                        rest,
                                        Finish::End(&end),
                                        (Some(&pins), Some(&gates))
                                    )
                                    .is_none(),
                            "{context}: {state:?} cut before {y} at {tick}"
                        );
                    }
                }
                // Until the notes reach the end, which the automaton steps too.
                if k + 1 < notes.len() {
                    let states = set.as_deref().unwrap_or_default();
                    for state in &finishable[i] {
                        kept += 1;
                        assert!(
                            states.iter().any(|s| s.0 == *state),
                            "{context}: {state:?} lost"
                        );
                    }
                }
            }
        }
        assert!(
            cut >= 500
                && beyond >= 300
                && restated >= 200
                && lost >= 50
                && checked >= 2500
                && kept >= 2000
                && gated >= 200,
            "{cut} of {checked} cut, {beyond} of them not by the capacity, {restated} past the \
             syllable, {lost} for good; {kept} that can finish kept; {gated} voices gated"
        );
        println!(
            "{cut} of {checked} states cut, {beyond} of them not by the capacity, {restated} past \
             the syllable, {lost} for good; {kept} that can finish kept"
        );
    }

    /// The fewest syllables a voice in this state begins before it can begin `y`, by a search
    /// over the model's own choices and steps at attacks; `None` if none lead there.
    fn fewest(model: &Model, from: &UState, y: u16) -> Option<u32> {
        let attack = Event::Attack(Attack {
            note: 1,
            start: 8,
            written: 8,
            pitch: pitch("D4"),
            entry: false,
            prev: None,
        });
        let (mut found, mut choices) = (Vec::new(), Vec::new());
        let mut seen = vec![from.syllable()];
        let mut layer = vec![*from];
        let mut begun = 0;
        while !layer.is_empty() {
            let mut next = Vec::new();
            for state in &layer {
                model.options(state, &attack, &mut choices);
                if choices.contains(&Choice::Start(y)) {
                    return Some(begun);
                }
                for &choice in choices.iter().filter(|&&c| c != Choice::Hold) {
                    let t = model.step(state, &attack, choice, &mut found);
                    if !seen.contains(&t.syllable()) {
                        seen.push(t.syllable());
                        next.push(t);
                    }
                }
            }
            layer = next;
            begun += 1;
        }
        None
    }

    /// The syllables a state still has to begin before a syllable are the fewest that the
    /// model's choices lead through: straight on, by restating from the end of a unit or from a
    /// syllable before a later point of an imitation, also after a marked break, and none into an
    /// earlier sentence.
    #[test]
    fn to_reach_matches_brute_force() {
        let mut rng = Rng::new(23);
        let (mut restated, mut lost) = (0, 0);
        for case in 0..200 {
            let text = match case % 8 {
                0 => section("kyrie"),
                1 => section("sanctus"),
                2 => section("agnus"),
                3 => section("benedictus"),
                4 => own("pax | pax pax, pax / pax pax. pax pax / pax."),
                _ => random_text(&mut rng, 16),
            };
            let restarts = random_restarts(&mut Rng::new(1000 + case as u64), &text);
            let model = model(&text).with_restarts(&text, &restarts);
            let n = model.syllables();
            for from in (0..=n).map(|x| model.after_rest(x.checked_sub(1))) {
                for y in 0..n as u16 {
                    let reach = model.to_reach(&from, y);
                    assert_eq!(
                        reach,
                        fewest(&model, &from, y),
                        "{}: {:?} to {y}",
                        text.render(true, true),
                        from.syllable()
                    );
                    // The way there has as many syllables, each one the model offers next.
                    let way = model.way(&from, y);
                    assert_eq!(way.as_ref().map(|w| w.len() as u32), reach);
                    let mut state = from;
                    for &s in way.iter().flatten().chain([y].iter()) {
                        let mut choices = Vec::new();
                        let attack = Event::Attack(Attack {
                            note: 1,
                            start: 8,
                            written: 8,
                            pitch: pitch("D4"),
                            entry: false,
                            prev: None,
                        });
                        model.options(&state, &attack, &mut choices);
                        if reach.is_none() {
                            break;
                        }
                        assert!(choices.contains(&Choice::Start(s)), "{s} after {state:?}");
                        state = model.step(&state, &attack, Choice::Start(s), &mut Vec::new());
                    }
                    restated += usize::from(from.syllable() >= Some(usize::from(y)));
                    lost += usize::from(reach.is_none());
                }
                assert_eq!(model.to_reach(&from, n as u16), None);
                // The rest of the text.
                let rest = model.way(&from, n as u16).unwrap();
                assert_eq!(rest.len() as u32, model.need(&from));
            }
        }
        assert!(
            restated >= 5000 && lost >= 400,
            "{restated} restated, {lost} lost"
        );
    }

    /// Before the second point of the Benedictus, which begins at the tail in nó-mi-ne within
    /// the unit, a voice may sing Be-ne-dí-ctus qui ve-nit again after ve-nit, as after the end of
    /// a unit; without the plan's restarts only straight on. A point of earlier words (a tail of
    /// the Kyrie, a restatement) adds none, and neither does a point after the end of a unit.
    #[test]
    fn restarts_before_later_points() {
        let benedictus = section("benedictus");
        let nit = benedictus
            .syllables
            .iter()
            .position(|s| s.text == "nit")
            .unwrap() as u16;
        let points = [0..nit + 1, nit + 1..benedictus.syllables.len() as u16];
        assert_eq!(gate_restarts(&benedictus, &points), [nit]);
        let plain = model(&benedictus);
        let restating = model(&benedictus).with_restarts(&benedictus, &[nit]);
        let attack = Event::Attack(Attack {
            note: 1,
            start: 8,
            written: 8,
            pitch: pitch("D4"),
            entry: false,
            prev: None,
        });
        let after = plain.after_rest(Some(usize::from(nit)));
        let mut choices = Vec::new();
        plain.options(&after, &attack, &mut choices);
        assert!(!choices.contains(&Choice::Start(0)));
        restating.options(&after, &attack, &mut choices);
        assert!(choices.contains(&Choice::Start(0)), "{choices:?}");
        // Restating Be- after ve-nit is a restatement there, no jump out of order.
        let mut found = Vec::new();
        restating.step(&after, &attack, Choice::Start(0), &mut found);
        assert!(found.iter().all(|f| f.case != Case::Order), "{found:?}");
        // Without the restart only past Dó-mi-ni: in nó-mi-ne Dó-mi-ni first.
        assert_eq!(restating.to_reach(&after, 0), Some(0));
        assert_eq!(plain.to_reach(&after, 0), Some(7));
        let kyrie = section("kyrie");
        assert_eq!(gate_restarts(&kyrie, &[0..7, 3..7]), []);
        assert_eq!(gate_restarts(&kyrie, &[0..7, 0..7]), []);
        let agnus = section("agnus");
        let units: Vec<std::ops::Range<u16>> =
            agnus.units.iter().map(|u| u.first..u.last + 1).collect();
        assert_eq!(gate_restarts(&agnus, &units), []);
    }

    /// A gate holds the words of a later point back until the point begins: before it the voice
    /// sings the words before them, again where it likes, or holds its syllable; the cheapest
    /// underlay begins the gated words at the gate, where without the gate it would begin them
    /// earlier; and a voice whose only way on lies before its gate is cut, naming TXT-ORDER.
    #[test]
    fn gates_hold_later_words_back() {
        // A-gnus De-i, | qui tol-lis pec-cá-ta mun-di: | mi-se-ré-re no-bis.
        let agnus = model(&section("agnus"));
        assert_eq!(agnus.syllables(), 18);
        let levels = fux();
        let notes = sung(
            "d'2 e'2 f'2 g'2 a'2 g'2 f'2 e'2 d'2 e'2 f'2 g'2 a'2 g'2 f'2 e'2 d'2 e'2 f'2 g'2 \
             a'2 g'2 f'2 e'2 d'2 e'2 f'2 g'2 a'2 g'2 f'2 e'2 d'2 e'2 f'2 e'2 d'1",
        );
        let total: u32 = notes.iter().map(|n| n.duration.ticks()).sum();
        // qui not before measure 5, mi- not before measure 10.
        let gates = [(4, 32), (12, 72)];
        assert_eq!((gate(Some(&gates), 3), gate(Some(&gates), 4)), (0, 32));
        assert_eq!((gate(Some(&gates), 11), gate(Some(&gates), 17)), (32, 72));
        let automaton = Automaton {
            model: &agnus,
            levels: &levels,
            total,
            cap: None,
            limit: None,
            pins: None,
            gates: Some(&gates),
        };
        let mut set = Automaton::start();
        let mut tick = 0;
        let mut restated = false;
        for i in 0..notes.len() {
            set = automaton.advance(&set, &notes[..=i]).unwrap();
            for (state, _, _) in &set {
                let y = state.syllable().unwrap() as u16;
                assert!(gate(Some(&gates), y) <= tick, "{y} at {tick}");
                restated |= tick < 32 && state.restarts > 0;
            }
            tick += notes[i].duration.ticks();
        }
        assert!(restated, "Agnus Dei sung again before the gate");
        // The cheapest underlay keeps the gates; without them it sings qui earlier.
        let starts = |path: &Path| -> Vec<(u32, u16)> {
            let ticks: Vec<u32> = notes
                .iter()
                .scan(0, |t, n| {
                    let at = *t;
                    *t += n.duration.ticks();
                    Some(at)
                })
                .collect();
            path.starts.iter().map(|&(n, y)| (ticks[n], y)).collect()
        };
        let gated = agnus
            .best_pinned(&levels, &notes, None, Some(&gates))
            .unwrap();
        assert!(
            starts(&gated)
                .iter()
                .all(|&(t, y)| t >= gate(Some(&gates), y))
        );
        assert!(starts(&gated).contains(&(32, 4)), "{:?}", starts(&gated));
        let free = agnus.best(&levels, &notes).unwrap();
        assert!(
            starts(&free)
                .iter()
                .any(|&(t, y)| t < gate(Some(&gates), y)),
            "{:?}",
            starts(&free)
        );
        assert_eq!(
            automaton.run(&notes).unwrap()[0].1,
            gated.cost,
            "the automaton keeps the cheapest"
        );
        // A pin of qui before its gate leaves no way on.
        let pins = [(16, Choice::Start(4))];
        let pinned = Automaton {
            pins: Some(&pins),
            ..automaton
        };
        assert_eq!(pinned.run(&notes), Err(Case::Order.rule()));
    }

    /// The automaton keeps only the pinned choice at a pinned attack and cuts the states that
    /// can no longer begin the next pinned syllable in time, naming TXT-ORDER, or TXT-ENTRY for
    /// a melisma pinned at an entry.
    #[test]
    fn pins_cut_states_and_name_the_rule() {
        /// The syllables of the states after the notes.
        fn syllables(automaton: &Automaton, notes: &[Note]) -> Vec<Option<usize>> {
            let mut syllables: Vec<_> = automaton
                .run(notes)
                .unwrap()
                .iter()
                .map(|s| s.0.syllable())
                .collect();
            syllables.sort_unstable();
            syllables.dedup();
            syllables
        }
        let kyrie = model(&section("kyrie"));
        let levels = fux();
        // Halves on the minim beats, a place for a syllable each. The e- of eleison (3) is pinned
        // on the fifth: Ký-ri-e take three of the four halves before it.
        let notes = sung("d'2 e'2 f'2 g'2 a'2 b'2 a'2 g'2 f'2 e'2 d'2 e'2 f'1 e'1");
        let cap = Capacity::new(64, &[], &levels);
        let pins = [(16, Choice::Start(3))];
        let automaton = Automaton {
            model: &kyrie,
            levels: &levels,
            total: 64,
            cap: Some(&cap),
            limit: None,
            pins: Some(&pins),
            gates: None,
        };
        let unpinned = Automaton {
            pins: None,
            gates: None,
            ..automaton
        };
        // After the third half a voice that held Ký on the second can no longer make it; the
        // capacity alone keeps it.
        assert_eq!(
            syllables(&unpinned, &notes[..3]),
            [Some(0), Some(1), Some(2)]
        );
        assert_eq!(syllables(&automaton, &notes[..3]), [Some(1), Some(2)]);
        // On the fourth half no voice begins the e- early, which it would have to begin again
        // after a restatement; on the fifth every voice begins it.
        assert_eq!(syllables(&automaton, &notes[..4]), [Some(2)]);
        assert_eq!(syllables(&automaton, &notes[..5]), [Some(3)]);
        // Without a capacity the pin is no milestone, and only the pinned attack decides.
        let loose = Automaton {
            cap: None,
            ..automaton
        };
        assert_eq!(syllables(&loose, &notes[..3]), [Some(0), Some(1), Some(2)]);
        assert_eq!(syllables(&loose, &notes[..5]), [Some(3)]);
        // The lé (4) pinned on the fourth half is out of reach from the first note on.
        let early = [(12, Choice::Start(4))];
        let automaton = Automaton {
            pins: Some(&early),
            gates: None,
            ..automaton
        };
        assert_eq!(automaton.run(&notes[..1]), Err("TXT-ORDER"));
        // A melisma pinned at an entry, or a syllable out of order.
        let entry = sung("r1 d'1 e'1 f'1 g'1");
        for (pin, rule) in [(Choice::Hold, "TXT-ENTRY"), (Choice::Start(1), "TXT-ORDER")] {
            let pins = [(8, pin)];
            let automaton = Automaton {
                pins: Some(&pins),
                gates: None,
                ..loose
            };
            assert_eq!(automaton.run(&entry[..2]), Err(rule), "{pin:?}");
        }
    }

    /// The state after a syllable and a rest is the state a voice reaches there, once the word
    /// has ended.
    #[test]
    fn after_rest_is_where_a_voice_rests() {
        let kyrie = model(&section("kyrie"));
        let levels = fux();
        for (src, x) in [
            ("d'1 e'1 f'1 r2", 2),
            ("d'1 e'1 f'1 g'2 a'2 b'1 c''1 d''1 r1", 6),
        ] {
            let notes = sung(src);
            let automaton = Automaton {
                model: &kyrie,
                levels: &levels,
                total: 1000,
                cap: None,
                limit: None,
                pins: None,
                gates: None,
            };
            let set = automaton.run(&notes).unwrap();
            assert!(
                set.iter().any(|s| s.0 == kyrie.after_rest(Some(x))),
                "{src}: {set:?}"
            );
            assert_eq!(
                kyrie.canonical(&kyrie.after_rest(Some(x)), &levels),
                kyrie.after_rest(Some(x))
            );
        }
        assert_eq!(kyrie.after_rest(None), UState::START);
    }

    /// The cheapest underlay of a given head ([`Model::best_prefix`]): its first syllable on its
    /// first attack, each of its syllables and no later one, at the voice's start, after a rest
    /// and off the downbeat, as trying every syllable at every attack finds it.
    #[test]
    fn best_prefix_lays_given_heads() {
        let levels = fux();
        let kyrie = model(&section("kyrie"));
        let agnus = model(&section("agnus"));
        let laid = |model: &Model, from: UState, src: &str, first: u16, last: u16| {
            model
                .best_prefix(&levels, from, &sung(src), first, last)
                .map(|path| (path.starts, path.cost))
        };
        // Ký-ri-e e- on a given head: the repeated d' takes ri, which takes no melisma, and
        // the e- of eleison begins as early as it costs least, on the quarter on the downbeat
        // (TXT-ONSET `opening_black`), its melisma running on to the head's end.
        let head = "d'1 d'2 a'2 g'4 f'4 e'2 d'1";
        let kyrie_e = vec![(0, 0), (1, 1), (2, 2), (3, 3)];
        assert_eq!(
            laid(&kyrie, UState::START, head, 0, 3),
            Some((kyrie_e.clone(), 1))
        );
        // The checker finds the same cost on the head.
        let mut texted = sung(head);
        let path = kyrie.best_prefix(&levels, UState::START, &texted, 0, 3);
        path.unwrap().apply(&mut texted);
        assert_eq!(levels.price(&kyrie.cases(&texted, false)), (1, None));
        // From the minim beat the quarter would begin a run on beat 2 as well: the e takes the
        // melisma, and the e- waits for the last note.
        assert_eq!(
            laid(&kyrie, UState::START, &format!("r2 {head}"), 0, 3),
            Some((vec![(1, 0), (2, 1), (3, 2), (7, 3)], 1))
        );
        // After a rest: the tail e-lé-i-son after the first statement, the head restated, and
        // qui tol-lis pec- after A-gnus De-i.
        let after_son = kyrie.after_rest(Some(6));
        assert_eq!(
            laid(&kyrie, after_son, "a'1 g'2 f'2 e'1 d'1", 3, 6),
            Some((vec![(0, 3), (1, 4), (3, 5), (4, 6)], 0))
        );
        assert_eq!(laid(&kyrie, after_son, head, 0, 3), Some((kyrie_e, 1)));
        let after_dei = agnus.after_rest(Some(3));
        assert_eq!(
            laid(&agnus, after_dei, "a'1 a'2 g'2 f'2 e'4 d'4 e'1", 4, 7),
            Some((vec![(0, 4), (1, 5), (2, 6), (3, 7)], 1))
        );
        // None: a voice begins with Ký; the e of Ký-ri-e ends no unit, after which the head
        // could begin again; three notes for four syllables.
        assert_eq!(laid(&kyrie, UState::START, head, 3, 3), None);
        assert_eq!(laid(&kyrie, kyrie.after_rest(Some(2)), head, 0, 3), None);
        assert_eq!(laid(&kyrie, UState::START, "d'1 e'1 f'1", 0, 3), None);
        // Random heads from random states, mostly with a first syllable that may follow there.
        let mut rng = Rng::new(41);
        let (mut heads, mut none) = (0, 0);
        let mut options = Vec::new();
        for case in 0..400 {
            let text = match case % 4 {
                0 => section("kyrie"),
                1 => section("agnus"),
                _ => random_text(&mut rng, 8),
            };
            let levels = if case % 3 == 0 {
                random_levels(&mut rng)
            } else {
                fux()
            };
            let notes = loop {
                let measures = 1 + below(&mut rng, 3);
                let notes = random_voice(&mut rng, measures, true, false);
                if (1..=10).contains(&attacks(&notes)) {
                    break notes;
                }
            };
            let measures = notes.iter().map(|n| n.duration.ticks()).sum::<u32>() as usize / 8;
            let goals = random_goals(&mut rng, measures);
            let model = Model::new(&text, dorian(), Meter::ALLA_BREVE, &goals);
            let n = model.syllables();
            let from = model.after_rest(below(&mut rng, n + 1).checked_sub(1));
            let events = Event::all(&notes, false);
            let Some(&Event::Attack(entry)) = events.iter().find(|e| matches!(e, Event::Attack(_)))
            else {
                unreachable!("a head without an attack");
            };
            model.options(&from, &Event::Attack(entry), &mut options);
            let first = match below(&mut rng, 4) {
                0 => below(&mut rng, n) as u16,
                _ => match options[below(&mut rng, options.len())] {
                    Choice::Start(y) => y,
                    Choice::Hold => unreachable!("a melisma at an entry"),
                },
            };
            let last = match case % 10 {
                9 => first.saturating_sub(1),
                _ => (usize::from(first) + below(&mut rng, 5)).min(n - 1) as u16,
            };
            let pins = [(entry.start, Choice::Start(first))];
            let finish = Finish::Head(last);
            let bounds = (&pins[..], &[][..]);
            let expected = brute_pinned(&model, &levels, from, &events, finish, bounds, (0, 0));
            let path = model.best_prefix(&levels, from, &notes, first, last);
            let context = format!(
                "case {case}: {} / {:?} {first}..={last} / {notes:?}",
                text.render(true, true),
                from.syllable()
            );
            assert_eq!(
                path.as_ref().map(|p| (p.cost, p.lateness)),
                expected,
                "{context}"
            );
            let Some(path) = path else {
                none += 1;
                continue;
            };
            heads += 1;
            assert_eq!(path.starts.first(), Some(&(entry.note, first)), "{context}");
            assert_eq!(path.starts.last().map(|s| s.1), Some(last), "{context}");
            assert!(path.starts.iter().all(|s| s.1 <= last), "{context}");
            if from == UState::START {
                let mut texted = notes.clone();
                path.apply(&mut texted);
                let price = levels.price(&model.cases(&texted, false));
                assert_eq!(price, (path.cost, None), "{context}");
            }
        }
        assert!(heads >= 90 && none >= 150, "{heads} heads, {none} without");
    }

    /// Times `advance` as the search calls it, along each voice of four-voice free settings of
    /// Kyrie I's length (16 measures, untexted, as in phase 11) that admits an underlay of
    /// Kyrie I, with the capacity and the limit of the search: once as the voices come, then
    /// replayed. Run with
    /// `cargo test --release -p fux-core advance_timing -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn advance_timing() {
        use crate::clef::VoiceType;
        use crate::generate::{FreeRequest, cadence_formula, default_cadences, generate_free};
        use std::time::Instant;

        const ROUNDS: u32 = 20;
        let mode = dorian();
        let meter = Meter::ALLA_BREVE;
        let text = section("kyrie");
        let levels = fux();
        let total = 16 * 8;
        let (mut voices, mut dp_seconds) = (0u32, 0.0);
        let mut admitted = Vec::new();
        for seed in 1..=6 {
            let request = FreeRequest {
                cadences: default_cadences(mode, 16),
                seed,
                ..FreeRequest::new(
                    mode,
                    vec![
                        VoiceType::Soprano,
                        VoiceType::Alto,
                        VoiceType::Tenor,
                        VoiceType::Bass,
                    ],
                    16,
                )
            };
            let score = &generate_free(&request, &Profile::fux()).unwrap()[0].score;
            let goals: Vec<Goal> = score
                .cadences
                .iter()
                .map(|c| Goal {
                    tick: c.measure as u32 * 8,
                    target: c.target,
                })
                .collect();
            let mut formulas = cadence_formula(mode, mode.final_step(), 15, 4, meter);
            for c in &score.cadences {
                formulas.extend(cadence_formula(mode, c.target, c.measure, 4, meter));
            }
            for (v, voice) in score.voices.iter().enumerate() {
                let model = Model::new(&text, mode, meter, &goals);
                voices += 1;
                let clock = Instant::now();
                let path = model.best(&levels, &voice.notes);
                dp_seconds += clock.elapsed().as_secs_f64();
                if path.is_some() {
                    let mut fixed: Vec<(u32, u32, bool)> = formulas
                        .iter()
                        .filter(|f| f.voice == v)
                        .map(|f| (f.start, f.note.duration.ticks(), f.tied_in))
                        .collect();
                    fixed.push((total - 8, 8, false));
                    let cap = Capacity::new(total, &fixed, &levels);
                    admitted.push((model, cap, voice.notes.clone()));
                }
            }
        }
        let mut seconds = [0.0; 2];
        let (mut calls, mut states, mut largest) = (0u32, 0u32, 0);
        for round in 0..ROUNDS {
            for (model, cap, notes) in &admitted {
                let automaton = Automaton {
                    model,
                    levels: &levels,
                    total,
                    cap: Some(cap),
                    limit: Some(LIMIT),
                    pins: None,
                    gates: None,
                };
                let mut set = Automaton::start();
                for i in 0..notes.len() {
                    let clock = Instant::now();
                    let next = automaton.advance(&set, &notes[..=i]);
                    seconds[usize::from(round > 0)] += clock.elapsed().as_secs_f64();
                    set = next.expect("the limit keeps a state that finishes");
                    if round == 0 {
                        calls += 1;
                        states += set.len() as u32;
                        largest = largest.max(set.len());
                    }
                }
            }
        }
        println!(
            "{} of {voices} voices admit Kyrie I; advance: {calls} calls, {:.1} states on \
             average, at most {largest}; {:.2} µs per call the first time, {:.2} µs replayed; \
             dynamic program: {:.0} µs per voice",
            admitted.len(),
            f64::from(states) / f64::from(calls),
            seconds[0] * 1e6 / f64::from(calls),
            seconds[1] * 1e6 / f64::from(calls * (ROUNDS - 1)),
            dp_seconds * 1e6 / f64::from(voices),
        );
    }
}
