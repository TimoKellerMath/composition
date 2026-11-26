//! Jeppesen's underlay of Palestrina's Missa Dies sanctificatus (Counterpoint, 1939,
//! pp. 254–258; tests/fixtures/text/dies-sanctificatus.toml) on the notes of the corpus
//! (references/datasets/palestrina-masses): the TXT rules find no prohibition in it except where
//! the print itself goes against a rule, and they decide every case window by window and prefix
//! by prefix as in the whole excerpt. Every case is printed with its cost:
//! `cargo test -p fux-core --test jeppesen -- --nocapture`.

mod common;

use std::path::PathBuf;
use std::sync::Arc;

use common::{assert_prefixes_collect, assert_windows_partition, text_rules};
use fux_core::kern::parse_kern;
use fux_core::rules::{Analysis, Checker, Detail, Level, Profile, Violation, Window};
use fux_core::text::{Eleison, Text, TextOptions, resolve};
use fux_core::{Lang, Lyric, Meter, Note, Score, ScoreText};
use serde::Deserialize;

/// The fixture: one excerpt per table Jeppesen prints.
#[derive(Deserialize)]
struct Fixture {
    excerpt: Vec<Excerpt>,
}

#[derive(Deserialize)]
struct Excerpt {
    id: String,
    /// The kern file of the corpus.
    file: String,
    /// The section of the Ordinary the voices sing.
    text: String,
    /// The measures of the kern file, inclusive: breves of 4/2.
    first_measure: usize,
    last_measure: usize,
    /// „3“ or „4“ syllables of eleison, as printed.
    eleison: String,
    voice: Vec<Part>,
}

/// A voice of an excerpt: its kern spine (1 = the lowest), its name and per attack the printed
/// syllable, or `_` where the attack continues the syllable before.
#[derive(Deserialize)]
struct Part {
    spine: usize,
    name: String,
    syllables: Vec<String>,
}

/// The prohibitions of the TXT rules in Jeppesen's underlay, each where his print goes against a
/// rule: (excerpt, voice, rule, case, kern measure of the marked note).
const PROHIBITED: [(&str, &str, &str, &str, usize); 3] = [
    // Soprano and alto enter with the tenor's „fa-ctó-rem“ instead of the first words (Jeppesen,
    // p. 257), and the bass leaves out „fa-ctó-rem cae-li et ter-rae,“.
    ("credo", "Bassus", "TXT-ORDER", "level", 7),
    ("credo", "Altus", "TXT-ORDER", "first", 3),
    ("credo", "Cantus", "TXT-ORDER", "first", 3),
];

/// The licences of TXT-AFTER-BLACK `dotted` in Jeppesen's underlay, each at its level in Fux's
/// profile: (excerpt, voice, kern measure of the marked note, level).
const DOTTED: [(&str, &str, usize, Level); 2] = [
    // A new syllable on the half after the untexted quarter of a dotted half: the tenor's „ste“
    // and the alto's „lei-“ (printed so; see the fixture).
    ("christe", "Tenor", 27, Level::P3),
    ("christe", "Altus", 27, Level::P3),
];

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../references/datasets/palestrina-masses/kern")
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/text/dies-sanctificatus.toml")
}

fn fixture() -> Fixture {
    toml::from_str(&std::fs::read_to_string(fixture_path()).unwrap()).unwrap()
}

/// The measures of an excerpt, `first` to `last` of the kern file: the reader reads kern measure
/// N as its measures 2N-1 and 2N. The excerpt ends open: a note tied on beyond it loses its tie.
fn cut(score: &Score, first: usize, last: usize, label: &str) -> Score {
    let breve = 2 * score.meter.ticks_per_measure();
    let (from, to) = ((first as u32 - 1) * breve, last as u32 * breve);
    let mut cut = score.clone();
    for voice in &mut cut.voices {
        let before = voice.timed().take_while(|(start, _)| *start < from).last();
        assert!(
            before.is_none_or(|(_, note)| !note.tie),
            "{label}, {}: a tie into the excerpt",
            voice.name
        );
        let mut notes: Vec<Note> = voice
            .timed()
            .filter(|(start, _)| (from..to).contains(start))
            .map(|(_, note)| note.clone())
            .collect();
        if let Some(last) = notes.last_mut() {
            last.tie = false;
        }
        voice.notes = notes;
    }
    cut
}

/// A syllable as letters only, in lower case and without accents, j read as i: „Ký-“ is „ky“.
fn plain(syllable: &str) -> String {
    syllable
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' => 'u',
            'ý' => 'y',
            'j' => 'i',
            c => c,
        })
        .filter(char::is_ascii_alphabetic)
        .collect()
}

/// The positions in the text of a voice's printed syllables, by their order: each is the next
/// syllable of the text, spelled as the books spell it or not (Jeppesen prints e-lei-son,
/// o-mni-po-ten-tem, coe-li). Only where the print departs from that order do the spellings
/// decide: right after the last syllable of a unit, a syllable that is the start of a unit or a
/// tail its sentence allows begins a restatement there; a voice that enters in the middle of
/// the text or leaves out some of it goes on where the print and the text next agree on two
/// syllables.
fn positions(text: &Text, printed: &[&str], label: &str) -> Vec<usize> {
    let count = text.syllables.len();
    let same = |p: &str, y: usize| y < count && plain(p) == plain(&text.syllables[y].text);
    let mut out: Vec<usize> = Vec::new();
    for (k, &p) in printed.iter().enumerate() {
        let last = out.last().copied();
        let next = last.map_or(0, |x| x + 1);
        // Restatements begin only after the last syllable of a unit, where its targets lie.
        let y = if same(p, next) {
            next
        } else if let Some(t) = last.and_then(|x| text.restart_targets(x).find(|&t| same(p, t))) {
            t
        } else if let Some(y) = (next + 1..count)
            .find(|&y| same(p, y) && printed.get(k + 1).is_none_or(|q| same(q, y + 1)))
        {
            y
        } else {
            next
        };
        assert!(y < count, "{label}: „{p}“ beyond the text");
        out.push(y);
    }
    out
}

/// Where a tick of an excerpt lies in the kern file: its measure and the tick in that breve.
fn kern_place(meter: Meter, first: usize, tick: u32) -> (usize, u32) {
    let breve = 2 * meter.ticks_per_measure();
    (first + (tick / breve) as usize, tick % breve)
}

/// The parameter of a TXT rule that a case belongs to.
fn param(violation: &Violation) -> &'static str {
    match violation.detail {
        Detail::Param(param) => param,
        Detail::Count(_) => "long",
        _ => "",
    }
}

/// Jeppesen's underlay on the notes of the corpus, excerpt by excerpt, each cut to its measures
/// and open at its end (a check that is not complete: the voices go on): the TXT rules prohibit
/// only what [`PROHIBITED`] lists, and windows and prefixes find the same as the whole excerpt,
/// with Fux's profile and with every case of the TXT rules on. Prints the underlay of each voice
/// as the text has it and every case with its cost. Skipped without the corpus or the fixture.
#[test]
fn dies_sanctificatus_underlay() {
    if !common::present(&corpus_dir()) || !common::present(&fixture_path()) {
        return;
    }
    let fux = Checker::new(&Profile::fux());
    let all_text = text_rules();
    let mut prohibited = Vec::new();
    let mut dotted = Vec::new();
    for excerpt in fixture().excerpt {
        let label = format!(
            "{} ({} m{}-{})",
            excerpt.id, excerpt.file, excerpt.first_measure, excerpt.last_measure
        );
        let kern = std::fs::read_to_string(corpus_dir().join(&excerpt.file)).unwrap();
        let piece = parse_kern(&kern).unwrap();
        assert_eq!(piece.score.meter, Meter::ALLA_BREVE, "{label}");
        let mut score = cut(
            &piece.score,
            excerpt.first_measure,
            excerpt.last_measure,
            &label,
        );
        let eleison = match excerpt.eleison.as_str() {
            "3" => Eleison::Three,
            _ => Eleison::Four,
        };
        let text = resolve(&excerpt.text, &TextOptions { eleison }).unwrap().0;
        println!("{label}, eleison in {} syllables:", excerpt.eleison);
        let mut sings = vec![false; score.voices.len()];
        for part in &excerpt.voice {
            // The reader lists the voices from the top, kern from the bottom.
            let v = score.voices.len() - part.spine;
            let notes = &mut score.voices[v].notes;
            let attacks: Vec<usize> = (0..notes.len())
                .filter(|&i| !notes[i].is_rest() && !(i > 0 && notes[i - 1].tie))
                .collect();
            let name = format!("{label}, {}", part.name);
            assert_eq!(attacks.len(), part.syllables.len(), "{name}: attacks");
            let sung: Vec<(usize, &str)> = attacks
                .into_iter()
                .zip(&part.syllables)
                .filter(|(_, printed)| *printed != "_")
                .map(|(i, printed)| (i, printed.as_str()))
                .collect();
            let printed: Vec<&str> = sung.iter().map(|&(_, p)| p).collect();
            let places = positions(&text, &printed, &name);
            for (&(i, p), &y) in sung.iter().zip(&places) {
                notes[i].lyric = Some(Lyric::new(y));
                notes[i].syllable = Some(p.to_string());
            }
            let read: Vec<String> = places
                .iter()
                .map(|&y| text.syllable_label(y, true))
                .collect();
            println!("  {:<7} {}", part.name, read.join(" ").replace("- ", "-"));
            assert_eq!(
                score.voices[v].name, part.name,
                "{label}: spine {}",
                part.spine
            );
            sings[v] = true;
        }
        score.text = Some(ScoreText {
            text: Arc::new(text),
            voices: sings,
        });
        score.validate().unwrap_or_else(|e| panic!("{label}: {e}"));
        // The excerpt ends open: TXT-COMPLETE and the cases of a voice's end do not arise.
        let mut found = Vec::new();
        let open = Window {
            from: 0,
            until: u32::MAX,
            complete: false,
        };
        let targets = vec![true; score.voices.len()];
        fux.check_window(&score, &Analysis::new(&score), open, &targets, &mut found);
        let mut cases: Vec<&Violation> = found
            .iter()
            .filter(|v| v.rule.starts_with("TXT-"))
            .collect();
        cases.sort_by_key(|v| (v.voice, v.tick, v.note, v.rule));
        let mut cost = 0;
        for case in &cases {
            let voice = &score.voices[case.voice];
            let start = score.note_start(case.voice, case.note).unwrap();
            let (measure, at) = kern_place(score.meter, excerpt.first_measure, start);
            let syllable = voice.notes[case.note].syllable.as_deref().unwrap_or("_");
            println!(
                "    {:<7} m{measure} +{at:<2} {syllable:<6} {:<3} {}",
                voice.name,
                case.level.label(Lang::En),
                fux.message(case, Lang::En)
            );
            cost += case.level.cost();
            if case.is_forbidden() {
                prohibited.push((
                    excerpt.id.clone(),
                    voice.name.clone(),
                    case.rule,
                    param(case),
                    measure,
                ));
            }
            if (case.rule, param(case)) == ("TXT-AFTER-BLACK", "dotted") {
                dotted.push((excerpt.id.clone(), voice.name.clone(), measure, case.level));
            }
        }
        println!("    TXT cases: {}, cost {cost}", cases.len());
        let name = label.as_str();
        assert_windows_partition(&fux, &score, name);
        assert_prefixes_collect(&fux, &score, name);
        assert_windows_partition(&all_text, &score, name);
        assert_prefixes_collect(&all_text, &score, name);
    }
    let mut expected: Vec<(String, String, &str, &str, usize)> = PROHIBITED
        .iter()
        .map(|&(id, voice, rule, case, measure)| {
            (id.to_string(), voice.to_string(), rule, case, measure)
        })
        .collect();
    prohibited.sort();
    expected.sort();
    assert_eq!(prohibited, expected);
    let mut licensed: Vec<(String, String, usize, Level)> = DOTTED
        .iter()
        .map(|&(id, voice, measure, level)| (id.to_string(), voice.to_string(), measure, level))
        .collect();
    dotted.sort_by_key(|d| (d.0.clone(), d.1.clone(), d.2));
    licensed.sort_by_key(|d| (d.0.clone(), d.1.clone(), d.2));
    assert_eq!(dotted, licensed);
}
