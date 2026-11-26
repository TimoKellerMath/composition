//! The rules of the style as data: every rule has an identifier, a level (forbidden, a price,
//! off), and a message in German and English. A profile gives every rule its level and the
//! machine its parameters. The catalogue with sources and counts is `docs/regeln-bach.md`.

use std::fmt;

use anyhow::{Context, Result, bail};

/// How a rule binds.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Level {
    Off,
    /// A preference with this price per finding.
    Cost(u32),
    Forbidden,
}

impl Level {
    pub const P1: Level = Level::Cost(1);
    pub const P2: Level = Level::Cost(2);
    pub const P3: Level = Level::Cost(4);
    pub const P4: Level = Level::Cost(8);

    /// `V`, `P1` to `P4`, `P(n)`, `aus`, `frei` (also `off`, `free`).
    pub fn parse(text: &str) -> Option<Level> {
        let text = text.trim().trim_matches('"');
        Some(match text.to_ascii_lowercase().as_str() {
            "v" => Level::Forbidden,
            "aus" | "off" | "frei" | "free" => Level::Off,
            "p1" => Level::P1,
            "p2" => Level::P2,
            "p3" => Level::P3,
            "p4" => Level::P4,
            other => {
                let inner = other.strip_prefix("p(")?.strip_suffix(')')?;
                match inner.trim().parse().ok()? {
                    0 => Level::Off,
                    n => Level::Cost(n),
                }
            }
        })
    }

    pub const fn cost(self) -> u32 {
        match self {
            Level::Cost(n) => n,
            _ => 0,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Level::Off => write!(f, "frei"),
            Level::Forbidden => write!(f, "V"),
            Level::Cost(1) => write!(f, "P1"),
            Level::Cost(2) => write!(f, "P2"),
            Level::Cost(4) => write!(f, "P3"),
            Level::Cost(8) => write!(f, "P4"),
            Level::Cost(n) => write!(f, "P({n})"),
        }
    }
}

macro_rules! rules {
    ($( $name:ident, $id:literal, $de:literal, $en:literal; )*) => {
        /// The rules. The order is the order of the catalogue.
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum Rule { $( $name, )* }

        impl Rule {
            pub const ALL: &'static [Rule] = &[ $( Rule::$name, )* ];
            pub const COUNT: usize = Rule::ALL.len();

            pub const fn id(self) -> &'static str {
                match self { $( Rule::$name => $id, )* }
            }

            pub const fn german(self) -> &'static str {
                match self { $( Rule::$name => $de, )* }
            }

            pub const fn english(self) -> &'static str {
                match self { $( Rule::$name => $en, )* }
            }
        }
    };
}

rules! {
    // Melody: the interval from one tone of a voice to the next.
    MelRepeat, "MEL-REPEAT", "Tonwiederholung", "repeated tone";
    MelChromatic, "MEL-CHROMATIC", "chromatischer Halbtonschritt", "chromatic semitone";
    MelLeapBig, "MEL-LEAP-BIG", "Sprung über die Oktave hinaus", "leap beyond the octave";
    MelAlteredOut, "MEL-ALTERED-OUT", "übermäßiger oder verminderter Sprung außerhalb eines Dominantklangs", "augmented or diminished leap outside a dominant chord";
    MelAlteredIn, "MEL-ALTERED-IN", "übermäßiger oder verminderter Sprung im Dominantklang", "augmented or diminished leap inside a dominant chord";
    MelSeventhOut, "MEL-SEVENTH-OUT", "Septsprung, nicht zwischen zwei Akkordtönen", "leap of a seventh, not between two chord tones";
    MelSeventhIn, "MEL-SEVENTH-IN", "Septsprung zwischen zwei Akkordtönen", "leap of a seventh between two chord tones";
    MelLeapsSame, "MEL-LEAPS-SAME", "zwei Sprünge in gleicher Richtung, nicht im selben Akkord", "two leaps in the same direction, not inside one chord";
    MelArpeggio, "MEL-ARPEGGIO", "zwei Sprünge in gleicher Richtung im Akkord", "two leaps in the same direction inside the chord";
    MelRecovery, "MEL-RECOVERY", "nach einem Sprung ab der Quarte kein Schritt zurück", "no step back after a leap of a fourth or more";
    MelBack, "MEL-BACK", "Rückkehr zum vorletzten Ton", "return to the tone before the last";
    MelTrill, "MEL-TRILL", "wiederholtes Pendeln zwischen zwei Tönen", "repeated swinging between two tones";
    MelEntryFar, "MEL-ENTRY-FAR", "Einsatz nach einer Pause mehr als eine Oktave vom letzten Ton", "entry after a rest more than an octave from the last tone";
    MelEntryWide, "MEL-ENTRY-WIDE", "Einsatz nach einer Pause mehr als eine Quinte vom letzten Ton", "entry after a rest more than a fifth from the last tone";
    MelLeading, "MEL-LEADING", "Leitton der Dominante steigt beim Akkordwechsel nicht", "the leading tone of the dominant does not rise when the chord changes";
    HarSeventhResolve, "HAR-SEVENTH-RESOLVE", "Akkordseptime nicht stufenweise abwärts weitergeführt", "the seventh of a chord does not go down by step";
    // Figures: tones foreign to the chord.
    FigUnresolved, "FIG-UNRESOLVED", "akkordfremder Ton, den keine Figur erklärt", "tone foreign to the chord that no figure explains";
    FigAccented, "FIG-ACCENTED", "schrittweise erreichter akkordfremder Ton auf dem Schlag", "foreign tone reached by step on a beat";
    FigAppoggiatura, "FIG-APPOGGIATURA", "Vorschlag: akkordfremder Ton im Sprung oder nach einer Pause", "appoggiatura: foreign tone reached by leap or after a rest";
    FigAppoggiaturaWeak, "FIG-APPOGGIATURA-WEAK", "Vorschlag abseits von Schlag und Schlaghälfte", "appoggiatura off the beat and its half";
    FigRestruck, "FIG-RESTRUCK", "akkordfremder Ton wird wieder angeschlagen", "foreign tone struck again";
    FigSuspensionUp, "FIG-SUSPENSION-UP", "Vorhalt löst sich aufwärts auf", "suspension resolved upwards";
    FigGroup, "FIG-GROUP", "Wechselnotengruppe", "group of two neighbours";
    FigEscape, "FIG-ESCAPE", "abspringende Nebennote oder Cambiata", "escape tone or cambiata";
    FigAnticipation, "FIG-ANTICIPATION", "Antizipation", "anticipation";
    FigDelayed, "FIG-DELAYED", "verspäteter Wechsel: Ton des alten Akkords springt in den neuen", "delayed change: a tone of the chord before leaps into the new one";
    FigLong, "FIG-LONG", "akkordfremder Ton länger als ein Schlag", "foreign tone longer than a beat";
    FigTooLong, "FIG-TOO-LONG", "akkordfremder Ton länger als zwei Schläge", "foreign tone longer than two beats";
    FigBeforeRest, "FIG-BEFORE-REST", "akkordfremder Ton vor einer Pause oder am Ende", "foreign tone before a rest or at the end";
    FigLying, "FIG-LYING", "liegender akkordfremder Ton über einen weiteren Akkordwechsel", "held foreign tone across a further change of chord";
    // Motion of two voices.
    MotParallel, "MOT-PARALLEL", "Quint- oder Oktavparallele", "parallel fifths or octaves";
    MotDirect, "MOT-DIRECT", "verdeckte Quinte oder Oktave mit Sprung der Oberstimme", "hidden fifth or octave with a leap in the upper voice";
    MotCrossing, "MOT-CROSSING", "Stimmkreuzung", "voice crossing";
    MotCrossingDeep, "MOT-CROSSING-DEEP", "Stimmkreuzung tiefer als eine große Terz", "voice crossing deeper than a major third";
    MotUnison, "MOT-UNISON", "Einklang zweier zugleich angeschlagener Stimmen", "unison of two voices struck together";
    MotOuterSimilar, "MOT-OUTER-SIMILAR", "Außenstimmen zugleich in gleicher Richtung", "outer voices move together in the same direction";
    // Sound.
    HarClash, "HAR-CLASH", "zwei akkordfremde Töne zugleich angeschlagen und dissonant", "two foreign tones struck together in a dissonance";
    HarStruck, "HAR-STRUCK", "angeschlagene Dissonanz auf dem Schlag", "dissonance struck on a beat";
    HarDoubledLeading, "HAR-DOUBLED-LEADING", "Leitton verdoppelt", "leading tone doubled";
    HarDoubledSeventh, "HAR-DOUBLED-SEVENTH", "Akkordseptime verdoppelt", "seventh of the chord doubled";
    HarDoubledForeign, "HAR-DOUBLED-FOREIGN", "akkordfremder Ton verdoppelt", "foreign tone doubled";
    HarHarsh, "HAR-HARSH", "angeschlagener akkordfremder Ton einen Halbton neben einem klingenden Ton", "struck foreign tone a semitone from a sounding tone";
    HarNoThird, "HAR-NO-THIRD", "Klang ohne Terz auf dem Schlag", "sound without its third on a beat";
    HarSixFour, "HAR-SIX-FOUR", "Quartsextakkord auf dem Schlag", "six-four chord on a beat";
    HarWide, "HAR-WIDE", "mehr als eine Duodezime zwischen benachbarten Oberstimmen", "more than a twelfth between neighbouring upper voices";
    HarIncomplete, "HAR-INCOMPLETE", "kein vollständiger Dreiklang auf dem Schlag bei drei und mehr Stimmen", "no complete triad on a beat with three or more voices";
    HarBass, "HAR-BASS", "tiefster Ton ist nicht der Basston, den der Akkord nennt", "the lowest tone is not the bass the chord names";
    // Double counterpoint.
    DcFifth, "DC-FIFTH", "Quinte oder Quarte auf dem Schlag im doppelten Kontrapunkt der Oktave", "fifth or fourth on a beat in double counterpoint at the octave";
    DcWide, "DC-WIDE", "Abstand über die Oktave oder Kreuzung im doppelten Kontrapunkt", "more than an octave apart or crossed in double counterpoint";
}

impl Rule {
    pub fn parse(id: &str) -> Option<Rule> {
        let id = id.trim();
        Rule::ALL
            .iter()
            .copied()
            .find(|r| r.id().eq_ignore_ascii_case(id))
    }

    pub const fn message(self, english: bool) -> &'static str {
        if english {
            self.english()
        } else {
            self.german()
        }
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// What the machine needs beside the levels: thresholds of rules and prices of the plan (what
/// the search prefers without it being a rule of the style).
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    /// A foreign tone reached by step on a strong beat costs this many halves of the price on
    /// another beat.
    pub accented_strong_halves: u32,
    /// A crossing costs once more per this many semitones of depth.
    pub crossing_per: u32,
    /// Semitones from which a crossing is deep.
    pub crossing_deep: u32,
    /// Semitones between neighbouring upper voices from which the sound is wide.
    pub wide: u32,
    /// Per class of interval away from the wished one, at most twice.
    pub wish_class: u32,
    /// The line turns where it wished to go on, or the other way round.
    pub wish_turn: u32,
    /// A free tone within a whole tone of the edge of its range.
    pub edge: u32,
    /// Per semitone a free voice is still too far from its next given tone (ranks the beam,
    /// costs nothing).
    pub far: u32,
    /// A given tone breaks a prohibition.
    pub given_fault: u32,
    /// A free tone does not resolve the foreign given tone before it.
    pub excused: u32,
    /// The interval from a free tone into a given one, by class: repetition, step, third,
    /// fourth or fifth, larger.
    pub into_given: [u32; 5],
    /// A tone of a figure is off the step the figure has there: per step of distance, at most
    /// twice.
    pub figure: u32,
    /// Semitones a free tone may lie from the tone before it, and from the last tone before a
    /// rest.
    pub reach: i16,
    pub reach_after_rest: i16,
    /// HAR-INCOMPLETE is read from this metric weight on (2 every beat, 3 the strong beats) …
    pub incomplete_weight: u32,
    /// … and only where exactly this many voices sound (0: any number from three on).
    pub incomplete_voices: u32,
    /// In the final chord of a piece, a missing third (HAR-NO-THIRD), with four and more
    /// voices an incomplete triad (HAR-INCOMPLETE) and a crossing of voices (MOT-CROSSING) count
    /// this many times, and so does the wish of the top voice, which there is a step. (8 until
    /// the free voices ran in figures up to the final chord; then one of twenty fugues for four
    /// voices closed without the fifth.)
    pub close: u32,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            accented_strong_halves: 3,
            crossing_per: 3,
            crossing_deep: 4,
            wide: 19,
            wish_class: 2,
            wish_turn: 1,
            edge: 1,
            far: 1,
            given_fault: 20,
            excused: 6,
            into_given: [3, 0, 1, 2, 4],
            figure: 6,
            reach: 12,
            reach_after_rest: 9,
            incomplete_weight: 2,
            incomplete_voices: 0,
            close: 16,
        }
    }
}

const PARAMS: [&str; 17] = [
    "accented_strong_halves",
    "crossing_per",
    "crossing_deep",
    "wide",
    "wish_class",
    "wish_turn",
    "edge",
    "far",
    "given_fault",
    "excused",
    "into_given",
    "figure",
    "reach",
    "reach_after_rest",
    "incomplete_weight",
    "incomplete_voices",
    "close",
];

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub name: String,
    levels: Vec<Level>,
    pub params: Params,
}

impl Profile {
    /// The prices of the feasibility study's prototype, rule by rule: where the calibration
    /// starts. The figures the prototype did not know are forbidden, the measures it did not
    /// price are free.
    pub fn start() -> Profile {
        use Level::{Cost, Forbidden as V, Off};
        let mut profile = Profile {
            name: "ausgang".to_string(),
            levels: vec![Off; Rule::COUNT],
            params: Params::default(),
        };
        for (rule, level) in [
            (Rule::MelRepeat, Off),
            (Rule::MelChromatic, Cost(3)),
            (Rule::MelLeapBig, V),
            (Rule::MelAlteredOut, V),
            (Rule::MelAlteredIn, Off),
            (Rule::MelSeventhOut, V),
            (Rule::MelSeventhIn, Cost(8)),
            (Rule::MelLeapsSame, Cost(4)),
            (Rule::MelArpeggio, Cost(1)),
            (Rule::MelRecovery, Cost(1)),
            (Rule::MelBack, Cost(1)),
            (Rule::MelTrill, Cost(3)),
            (Rule::MelEntryFar, V),
            (Rule::MelEntryWide, Cost(2)),
            (Rule::MelLeading, Cost(2)),
            (Rule::HarSeventhResolve, Cost(4)),
            (Rule::FigUnresolved, V),
            (Rule::FigAccented, Cost(2)),
            (Rule::FigAppoggiatura, Cost(2)),
            (Rule::FigAppoggiaturaWeak, V),
            (Rule::FigRestruck, Cost(2)),
            (Rule::FigSuspensionUp, Cost(1)),
            (Rule::FigGroup, Cost(1)),
            (Rule::FigEscape, Cost(3)),
            (Rule::FigAnticipation, V),
            (Rule::FigDelayed, V),
            (Rule::FigLong, Cost(4)),
            (Rule::FigTooLong, V),
            (Rule::FigBeforeRest, V),
            (Rule::FigLying, Cost(1)),
            (Rule::MotParallel, V),
            (Rule::MotDirect, Cost(2)),
            (Rule::MotCrossing, Cost(4)),
            (Rule::MotCrossingDeep, V),
            (Rule::MotUnison, Cost(1)),
            (Rule::MotOuterSimilar, Off),
            (Rule::HarClash, V),
            (Rule::HarStruck, Cost(1)),
            (Rule::HarDoubledLeading, Cost(4)),
            (Rule::HarDoubledSeventh, Cost(4)),
            (Rule::HarDoubledForeign, Cost(4)),
            (Rule::HarHarsh, Cost(2)),
            (Rule::HarNoThird, Cost(2)),
            (Rule::HarSixFour, Cost(3)),
            (Rule::HarWide, Cost(1)),
            (Rule::HarIncomplete, Off),
            (Rule::DcFifth, V),
            (Rule::DcWide, V),
        ] {
            profile.levels[rule as usize] = level;
        }
        profile
    }

    /// The standard profile: the levels after the count on Bach's fugues and the calibration on
    /// the development fugues (see `docs/regeln-bach.md`).
    pub fn bach() -> Profile {
        let mut profile = Profile::start();
        profile.name = "bach".to_string();
        profile
            .apply(include_str!("../data/bach.profile"))
            .expect("data/bach.profile is a profile");
        profile
    }

    /// The profile of the fugue composer: `bach` with dearer struck dissonance and complete
    /// triads where three voices sound, after the listening tests of phase B6.
    pub fn fuge() -> Profile {
        let mut profile = Profile::bach();
        profile.name = "fuge".to_string();
        profile
            .apply(include_str!("../data/fuge.profile"))
            .expect("data/fuge.profile is a profile");
        profile
    }

    /// A profile by name (`bach`, `fuge`, `ausgang`) or from a file.
    pub fn named(name: &str) -> Result<Profile> {
        match name {
            "bach" => Ok(Profile::bach()),
            "fuge" => Ok(Profile::fuge()),
            "ausgang" | "start" => Ok(Profile::start()),
            path => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let text =
                        std::fs::read_to_string(path).with_context(|| format!("profile {path}"))?;
                    let mut profile = Profile::bach();
                    profile.name = path.to_string();
                    profile.apply(&text)?;
                    Ok(profile)
                }
                #[cfg(target_arch = "wasm32")]
                bail!("no profile {path:?}")
            }
        }
    }

    pub fn level(&self, rule: Rule) -> Level {
        self.levels[rule as usize]
    }

    pub fn set(&mut self, rule: Rule, level: Level) {
        self.levels[rule as usize] = level;
    }

    /// One setting: `MEL-REPEAT=P2`, `wish_class=3`, `into_given=3,0,1,2,4`.
    pub fn set_text(&mut self, setting: &str) -> Result<()> {
        let (name, value) = setting
            .split_once('=')
            .with_context(|| format!("{setting:?}: expected NAME=VALUE"))?;
        let (name, value) = (name.trim(), value.trim().trim_matches('"'));
        if let Some(rule) = Rule::parse(name) {
            let level = Level::parse(value).with_context(|| {
                format!("{name}: {value:?} is no level (V, P1 to P4, P(n), aus)")
            })?;
            self.set(rule, level);
            return Ok(());
        }
        let number = || -> Result<u32> {
            value
                .parse()
                .with_context(|| format!("{name}: {value:?} is no number"))
        };
        let p = &mut self.params;
        match name {
            "accented_strong_halves" => p.accented_strong_halves = number()?,
            "crossing_per" => p.crossing_per = number()?.max(1),
            "crossing_deep" => p.crossing_deep = number()?,
            "wide" => p.wide = number()?,
            "wish_class" => p.wish_class = number()?,
            "wish_turn" => p.wish_turn = number()?,
            "edge" => p.edge = number()?,
            "far" => p.far = number()?,
            "given_fault" => p.given_fault = number()?,
            "excused" => p.excused = number()?,
            "figure" => p.figure = number()?,
            "reach" => p.reach = number()? as i16,
            "reach_after_rest" => p.reach_after_rest = number()? as i16,
            "incomplete_weight" => p.incomplete_weight = number()?,
            "incomplete_voices" => p.incomplete_voices = number()?,
            "close" => p.close = number()?.max(1),
            "into_given" => {
                let list: Vec<u32> = value
                    .split(',')
                    .map(|n| n.trim().parse())
                    .collect::<Result<_, _>>()
                    .with_context(|| format!("{name}: {value:?} is no list of numbers"))?;
                if list.len() != 5 {
                    bail!("{name}: five numbers expected");
                }
                p.into_given.copy_from_slice(&list);
            }
            other => bail!("{other:?} is neither a rule nor a parameter"),
        }
        Ok(())
    }

    /// The settings of a profile file: one `NAME = VALUE` per line, `#` begins a comment.
    pub fn apply(&mut self, text: &str) -> Result<()> {
        for (number, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() || line.starts_with('[') {
                continue;
            }
            self.set_text(line)
                .with_context(|| format!("line {}", number + 1))?;
        }
        Ok(())
    }

    /// The profile as a file that `apply` reads.
    pub fn text(&self) -> String {
        let mut out = format!("# Profil {}\n", self.name);
        for rule in Rule::ALL {
            out += &format!("{} = {}\n", rule.id(), self.level(*rule));
        }
        let p = &self.params;
        let values: [String; 17] = [
            p.accented_strong_halves.to_string(),
            p.crossing_per.to_string(),
            p.crossing_deep.to_string(),
            p.wide.to_string(),
            p.wish_class.to_string(),
            p.wish_turn.to_string(),
            p.edge.to_string(),
            p.far.to_string(),
            p.given_fault.to_string(),
            p.excused.to_string(),
            p.into_given.map(|n| n.to_string()).join(","),
            p.figure.to_string(),
            p.reach.to_string(),
            p.reach_after_rest.to_string(),
            p.incomplete_weight.to_string(),
            p.incomplete_voices.to_string(),
            p.close.to_string(),
        ];
        for (name, value) in PARAMS.iter().zip(values) {
            out += &format!("{name} = {value}\n");
        }
        out
    }
}

/// Who answers for a finding.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Bearer {
    /// A tone the search chose.
    Free,
    /// A given tone after a given tone or a rest: the given line itself.
    Given,
    /// A given tone after a free one: the free voice did not lead into it.
    Junction,
    /// A free tone that did not resolve the foreign given tone before it.
    Excused,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_read_and_print() {
        for text in ["V", "P1", "P2", "P3", "P4", "P(3)", "frei"] {
            assert_eq!(Level::parse(text).unwrap().to_string(), text);
        }
        assert_eq!(Level::parse("aus"), Some(Level::Off));
        assert_eq!(Level::parse("P(4)"), Some(Level::P3));
        assert_eq!(Level::parse("P5"), None);
    }

    #[test]
    fn profiles_read_what_they_write() {
        let bach = Profile::bach();
        let mut again = Profile::start();
        again.apply(&bach.text()).unwrap();
        again.name = bach.name.clone();
        assert_eq!(again, bach);
        let mut profile = Profile::start();
        profile.set_text("MEL-REPEAT = P2").unwrap();
        assert_eq!(profile.level(Rule::MelRepeat), Level::P2);
        profile.set_text("into_given=1,2,3,4,5").unwrap();
        assert_eq!(profile.params.into_given, [1, 2, 3, 4, 5]);
        assert!(profile.set_text("MEL-NOTHING=V").is_err());
        assert!(profile.set_text("MEL-REPEAT=P7").is_err());
        // The composer's profile is bach but for four levels.
        let (bach, fuge) = (Profile::bach(), Profile::named("fuge").unwrap());
        assert_eq!(fuge.level(Rule::HarStruck), Level::P4);
        assert_eq!(fuge.level(Rule::HarHarsh), Level::P3);
        assert_eq!(fuge.level(Rule::HarIncomplete), Level::P3);
        assert_eq!(
            (bach.level(Rule::HarBass), fuge.level(Rule::HarBass)),
            (Level::Off, Level::P4)
        );
        let changed = Rule::ALL
            .iter()
            .filter(|rule| fuge.level(**rule) != bach.level(**rule))
            .count();
        assert_eq!((changed, fuge.params == bach.params), (4, true));
        // Every rule has its own identifier.
        let mut ids: Vec<&str> = Rule::ALL.iter().map(|r| r.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), Rule::COUNT);
    }
}
