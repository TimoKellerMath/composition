//! The rules of the text underlay (TXT-*) window by window and prefix by prefix: on texted scores
//! the checker must find in the windows of the search, and in growing prefixes of the voices,
//! exactly what it finds in the whole score. A case reported later than the notes show it would
//! be missed by the prefixes; one reported twice would break the windows.

mod common;

use std::sync::Arc;

use common::{assert_prefixes_collect, assert_windows_partition, text_rules};

use fux_core::generate::Rng;
use fux_core::rhythm::{self, Place};
use fux_core::rules::{Checker, Profile};
use fux_core::text::underlay::{Goal, Levels, Model};
use fux_core::text::{Text, TextKind, TextOptions, ordinary};
use fux_core::{
    Cadence, Duration, Lyric, Meter, Note, Pitch, Role, Score, ScoreText, Species, Step, Voice,
    VoiceType, parse_lilypond,
};

/// Notes in LilyPond notation, each with the index of the syllable it begins after `=` (`?` for
/// a syllable that is not in the text): `d'2=0 e'4 f'4=1`.
fn sung(src: &str) -> Vec<Note> {
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

fn dorian() -> fux_core::ModeSpec {
    "dorian".parse().unwrap()
}

/// A Dorian score of free voices that all sing `text`, with internal cadences.
fn texted(text: &Text, voices: Vec<Vec<Note>>, cadences: Vec<Cadence>) -> Score {
    const TYPES: [VoiceType; 4] = [
        VoiceType::Soprano,
        VoiceType::Alto,
        VoiceType::Tenor,
        VoiceType::Bass,
    ];
    let voices: Vec<Voice> = voices
        .into_iter()
        .zip(TYPES)
        .map(|(notes, voice_type)| Voice::new(voice_type, Role::Counterpoint(Species::Free), notes))
        .collect();
    let mut score = Score::new(dorian(), voices);
    score.cadences = cadences;
    score.text = Some(ScoreText {
        text: Arc::new(text.clone()),
        voices: vec![true; score.voices.len()],
    });
    score
}

/// The TXT cases a checker reports in a score.
fn txt_cases(checker: &Checker, score: &Score) -> usize {
    checker
        .check(score)
        .violations
        .iter()
        .filter(|v| v.rule.starts_with("TXT-"))
        .count()
}

/// Kyrie I in four voices of eight measures, full of cases: a texted dactyl quarter and syllables
/// after it, rests inside words and units, a lyric on a rest, on a tie continuation and one that
/// is not in the text, melismas on short penults and last syllables, leaps inside words, a
/// cadence on the e of Kyrie, re-struck pitches, restatements and a voice that does not finish
/// the text.
fn kyrie_full_of_cases() -> Score {
    let kyrie = ordinary::section("kyrie", &TextOptions::default())
        .unwrap()
        .clone();
    let voices = [
        "d''2.=0 c''4=1 bes'2=2 a'4 g'4 a'2=3 r2 a'1=4 g'2=5 f'4 e'4 d'1=6 r2 a'2=0 d''1=1",
        "r1 r2 f'2=0 e'2~=1 e'4 d'4=? c'2=2 d'2=3 e'2=4 r2=5 f'2=5 e'2 d'1=6 d'1",
        "d'1=0 a'1=1 d'1=2 d''1=3 c''2=4 bes'2 a'1=5 g'2=6 r2 r1",
        "r1 r1 d2=0 a,2=1 d1=2 r2 a,2=3 d2=4 d2 c2=5 bes,2=6 a,1=3",
    ];
    let score = texted(
        &kyrie,
        voices.iter().map(|v| sung(v)).collect(),
        vec![Cadence {
            measure: 2,
            target: Step::D,
        }],
    );
    score.validate().unwrap();
    score
}

/// Scores without a text, as the readers give them, have no TXT case, whatever their notes:
/// Fux's examples and a sample of Palestrina's movements, whose `**kern` files have no text.
#[test]
fn untexted_corpora_have_no_text_cases() {
    let checker = text_rules();
    let mut scores = 0;
    for example in fux_core::corpus::fux_examples() {
        let score = example.score();
        assert_eq!(txt_cases(&checker, &score), 0, "Fig. {}", example.figure);
        scores += 1;
    }
    if !common::corpus_present() {
        return;
    }
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(common::corpus_dir())
        .expect("the corpus (python3 tools/fetch_palestrina.py)")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "krn"))
        .collect();
    files.sort();
    for path in files.iter().step_by(25) {
        let Ok(piece) = fux_core::kern::parse_kern(&std::fs::read_to_string(path).unwrap()) else {
            continue;
        };
        assert!(piece.score.text.is_none());
        assert_eq!(txt_cases(&checker, &piece.score), 0, "{path:?}");
        scores += 1;
    }
    assert!(scores >= 150, "{scores} scores");
}

#[test]
fn texted_scores_check_window_by_window() {
    let score = kyrie_full_of_cases();
    let text_rules = text_rules();
    assert!(txt_cases(&text_rules, &score) >= 25);
    assert_windows_partition(&text_rules, &score, "Kyrie, TXT rules");
    assert_prefixes_collect(&text_rules, &score, "Kyrie, TXT rules");
    // With all rules of Fux's profile, MEL-REPEAT `free_texted` included.
    let fux = Checker::new(&Profile::fux());
    assert!(txt_cases(&fux, &score) >= 15);
    assert_windows_partition(&fux, &score, "Kyrie, Fux");
    assert_prefixes_collect(&fux, &score, "Kyrie, Fux");
}

/// A long text whose words go on in the next part of its section (phase 14) ends in no Amen:
/// TXT-REPEAT `long` prices a restatement of its last unit as one of any other unit, where the
/// text that ends there restates it freely. The checker decides the case at the restatement's
/// attack, window by window and prefix by prefix, and the underlay the dynamic program lays on
/// the same notes costs what the checker finds in it: one model decides for both.
#[test]
fn continued_prices_the_last_unit() {
    let (ends, _) = fux_core::text::resolve("gloria:4-5", &TextOptions::default()).unwrap();
    let mut goes_on = ends.clone();
    goes_on.continued = true;
    // Ad-o-rá-mus te. Glo-ri-fi-cá-mus te., a rest, and the last sentence again.
    let voices = [
        "a'1=0 b'1=1 c''1=2 b'1=3 a'1=4 g'1=5 a'1=6 b'1=7 c''1=8 b'1=9 a'1=10 r1 a'1=5 g'1=6 \
         f'1=7 e'1=8 f'1=9 d'1=10",
        "d'1=0 e'1=1 f'1=2 e'1=3 d'1=4 c'1=5 d'1=6 e'1=7 f'1=8 e'1=9 d'1=10 r1 d'1=5 c'1=6 \
         b1=7 a1=8 b1=9 a1=10",
    ];
    let fux = Checker::new(&Profile::fux());
    let repeats = |text: &Text| -> Vec<(u32, usize, usize, fux_core::rules::Level)> {
        let score = texted(text, voices.iter().map(|v| sung(v)).collect(), Vec::new());
        fux.check(&score)
            .violations
            .iter()
            .filter(|v| v.rule == "TXT-REPEAT")
            .map(|v| (v.tick, v.voice, v.note, v.level))
            .collect()
    };
    assert_eq!(repeats(&ends), []);
    let long = Profile::fux().settings("TXT-REPEAT").level("long");
    assert_eq!(long, fux_core::rules::Level::P2);
    assert_eq!(repeats(&goes_on), [(96, 0, 12, long), (96, 1, 12, long)]);
    let score = texted(
        &goes_on,
        voices.iter().map(|v| sung(v)).collect(),
        Vec::new(),
    );
    score.validate().unwrap();
    for (checker, name) in [(&text_rules(), "TXT rules"), (&fux, "Fux")] {
        assert_windows_partition(checker, &score, &format!("continued, {name}"));
        assert_prefixes_collect(checker, &score, &format!("continued, {name}"));
    }
    // The cheapest underlay of each voice's notes, at the cost the checker finds in it.
    let levels = Levels::from(&fux);
    let meter = Meter::ALLA_BREVE;
    for text in [&ends, &goes_on] {
        let model = Model::new(text, dorian(), meter, &[]);
        let mut laid = score.clone();
        laid.text.as_mut().unwrap().text = Arc::new(text.clone());
        let mut costs = Vec::new();
        for voice in &mut laid.voices {
            for note in &mut voice.notes {
                note.lyric = None;
            }
            let path = model.best(&levels, &voice.notes).unwrap();
            path.apply(&mut voice.notes);
            costs.push(path.cost);
        }
        let found = fux.check(&laid);
        for (v, &cost) in costs.iter().enumerate() {
            let checked: u32 = found
                .violations
                .iter()
                .filter(|x| x.voice == v && x.rule.starts_with("TXT-"))
                .map(|x| x.level.cost())
                .sum();
            assert_eq!(checked, cost, "continued {}, voice {v}", text.continued);
        }
    }
}

/// A quarter repeated as a quarter tied over the barline (`a'2 g'4 g'4~ | g'2`), as a texted
/// voice sings it within a melisma: MEL-REPEAT decides the anticipation by the written value at
/// the repeat, where the tie is not yet known, so the prefixes find the level of the whole score.
#[test]
fn a_quarter_tied_over_is_decided_at_the_repeat() {
    let kyrie = ordinary::section("kyrie", &TextOptions::default())
        .unwrap()
        .clone();
    let score = texted(
        &kyrie,
        vec![sung(
            "d'1=0 a'2=1 g'4 g'4~ g'2 f'2=2 e'1=3 f'2=4 e'2=5 d'1=6",
        )],
        Vec::new(),
    );
    score.validate().unwrap();
    let fux = Checker::new(&Profile::fux());
    let repeats: Vec<_> = fux
        .check(&score)
        .violations
        .into_iter()
        .filter(|v| v.rule == "MEL-REPEAT")
        .map(|v| (v.tick, v.note, v.level))
        .collect();
    // A repeated black note (`free_black`), not the anticipation.
    assert_eq!(repeats, [(14, 3, fux_core::rules::Level::P1)]);
    assert_windows_partition(&fux, &score, "quarter tied over");
    assert_prefixes_collect(&fux, &score, "quarter tied over");
}

fn below(rng: &mut Rng, n: usize) -> usize {
    (rng.next_u64() % n as u64) as usize
}

/// A random voice of `measures` measures in Palestrina's measure rhythms, with rests and ties;
/// its last measure holds one note.
fn random_voice(rng: &mut Rng, measures: usize) -> Vec<Note> {
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
            .filter(|p| !last || matches!(*p, "1" | "⁀1"))
            .collect();
        let pick = below(rng, patterns.len()).min(below(rng, patterns.len()));
        for token in rhythm::tokens(patterns[pick]) {
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
        tied = patterns[pick].ends_with('⁀');
    }
    notes
}

/// A text for the random scores: a section of the Ordinary or an own text with breaks, tails,
/// commas and full stops, sometimes taken as a long text.
fn random_text(rng: &mut Rng) -> Text {
    const OWN: [&str; 4] = [
        "Ký-ri-e, e-lé-i-son | De-us; pax / in ex-cél-sis.",
        "et in ter-ra pax, Dó-mi-ne De-us. A-men.",
        "lau-dá-mus te | qui sedes / ad déx-te-ram, mi-se-ré-re no-bis.",
        "an-te De-um.",
    ];
    let options = TextOptions::default();
    let mut text = match below(rng, 6) {
        0 => ordinary::section("kyrie", &options).unwrap().clone(),
        1 => ordinary::section("christe", &options).unwrap().clone(),
        2 => ordinary::section("agnus", &options).unwrap().clone(),
        n => Text::own(OWN[n - 3 + below(rng, 2)], &options).unwrap().0,
    };
    if below(rng, 4) == 0 {
        text.kind = TextKind::Long;
    }
    text
}

/// Random internal cadences, on degrees of the Dorian mode with a clausula.
fn random_cadences(rng: &mut Rng, measures: usize) -> Vec<Cadence> {
    const TARGETS: [Step; 5] = [Step::D, Step::A, Step::F, Step::G, Step::E];
    let mut cadences = Vec::new();
    for measure in 1..measures {
        if below(rng, 3) == 0 {
            cadences.push(Cadence {
                measure,
                target: TARGETS[below(rng, TARGETS.len())],
            });
        }
    }
    cadences
}

/// Lyrics as an imported score might have them: on most notes, mostly the next syllable, now and
/// then a restart, a wrong syllable or one that is not in the text, also on rests and tie
/// continuations.
fn random_lyrics(rng: &mut Rng, notes: &mut [Note], syllables: usize) {
    let mut next = 0;
    for note in notes {
        if below(rng, 3) == 0 {
            continue;
        }
        let y = match below(rng, 10) {
            0 => below(rng, syllables),
            1 => 0,
            _ => next.min(syllables - 1),
        };
        note.lyric = Some(if below(rng, 25) == 0 {
            Lyric::UNKNOWN
        } else {
            next = y + 1;
            Lyric::new(y)
        });
    }
}

/// Random scores of two to four voices with random lyrics: every case the model can find, in
/// every window and every prefix exactly once.
#[test]
fn random_lyrics_check_window_by_window() {
    let checker = text_rules();
    let mut rng = Rng::new(23);
    let mut cases = 0;
    for case in 0..60 {
        let text = random_text(&mut rng);
        let measures = 2 + below(&mut rng, 7);
        let voices = (0..2 + below(&mut rng, 3))
            .map(|_| {
                let mut notes = random_voice(&mut rng, measures);
                random_lyrics(&mut rng, &mut notes, text.syllables.len());
                notes
            })
            .collect();
        let cadences = random_cadences(&mut rng, measures);
        let score = texted(&text, voices, cadences);
        cases += txt_cases(&checker, &score);
        let name = format!("case {case}: {}", text.render(true, true));
        assert_windows_partition(&checker, &score, &name);
        assert_prefixes_collect(&checker, &score, &name);
    }
    assert!(cases >= 1000, "{cases} cases");
}

/// Random voices with the underlay the dynamic program finds for them, as generated scores will
/// have it: the cases are those of the dynamic program's path, in every window and prefix once.
#[test]
fn underlaid_voices_check_window_by_window() {
    let checker = text_rules();
    let levels = Levels::from(&checker);
    let meter = Meter::ALLA_BREVE;
    let options = TextOptions::default();
    let texts = [
        ordinary::section("kyrie", &options).unwrap().clone(),
        ordinary::section("christe", &options).unwrap().clone(),
        Text::own("an-te De-um.", &options).unwrap().0,
        Text::own("De-us, pax / in ex-cél-sis.", &options)
            .unwrap()
            .0,
    ];
    let mut rng = Rng::new(29);
    let (mut scores, mut cases) = (0, 0);
    for case in 0..80 {
        let mut text = texts[below(&mut rng, texts.len())].clone();
        if below(&mut rng, 4) == 0 {
            text.kind = TextKind::Long;
        }
        let measures = 6 + below(&mut rng, 8);
        let cadences = random_cadences(&mut rng, measures);
        let model = Model::new(&text, dorian(), meter, &Goal::of(&cadences, meter));
        let voices: Vec<Vec<Note>> = (0..2 + below(&mut rng, 2))
            .filter_map(|_| {
                let mut notes = random_voice(&mut rng, measures);
                model.best(&levels, &notes)?.apply(&mut notes);
                Some(notes)
            })
            .collect();
        if voices.is_empty() {
            continue;
        }
        scores += 1;
        let score = texted(&text, voices, cadences);
        cases += txt_cases(&checker, &score);
        let name = format!("case {case}: {}", text.render(true, true));
        assert_windows_partition(&checker, &score, &name);
        assert_prefixes_collect(&checker, &score, &name);
    }
    assert!(
        scores >= 20 && cases >= 40,
        "{scores} scores, {cases} cases"
    );
}
