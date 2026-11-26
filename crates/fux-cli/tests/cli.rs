//! Runs the `fux` binary like a user would.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use clap::Parser;
use fux_cli::mass::{
    self, FIELD_LYRICS, MassOptions, PartLine, default_jobs, mass_summary, movement_path,
    part_label, part_path, parts_dir,
};
use fux_cli::output::with_extension;
use fux_core::mass::{
    FIELD_MASS, FIELD_PART, GENERATOR, MassField, MassPlan, Movement, Opening, PartField, PartRef,
};
use fux_core::rules::Profile;
use fux_core::{Lang, Pitch, SectionKind, Step};
use fux_export::from_musicxml;
use midly::{MetaMessage, Smf, TrackEventKind};

fn fux(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fux"))
        .args(args)
        .output()
        .expect("fux runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn tmp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn corpus_lists_examples_and_cantus() {
    let examples = fux(&["corpus"]);
    assert!(examples.status.success());
    assert_eq!(stdout(&examples).lines().count(), 1 + 123);
    let cantus = fux(&["corpus", "--cantus"]);
    assert_eq!(stdout(&cantus).lines().count(), 8);
    assert!(stdout(&cantus).contains("fux:dorian"));
}

#[test]
fn render_an_example() {
    let base = tmp("fig5");
    let output = fux(&[
        "render",
        "--example",
        "5",
        "-f",
        "ly,midi",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(ly.contains("Fig. 5"));
    let midi = std::fs::read(base.with_extension("mid")).unwrap();
    assert!(midi.starts_with(b"MThd"));
}

#[test]
fn render_a_job_file() {
    let job = tmp("job.toml");
    std::fs::write(
        &job,
        r#"
title = "Übung"
mode = "dorisch"

[[voice]]
type = "S"
notes = "a'1 a' g' a' b' c'' c'' b' d'' cis'' d''"

[[voice]]
type = "A"
cf = true
notes = "fux:dorian"
"#,
    )
    .unwrap();
    let base = tmp("job");
    let output = fux(&[
        "render",
        "--job",
        job.to_str().unwrap(),
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(ly.contains("title = \"Übung\""));
    assert!(ly.contains("C. f."));
}

#[test]
fn musicxml_out_and_in() {
    let base = tmp("fig5-xml");
    let output = fux(&[
        "render",
        "--example",
        "5",
        "-f",
        "musicxml",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let path = base.with_extension("musicxml");
    let xml = std::fs::read_to_string(&path).unwrap();
    assert!(xml.contains("<score-partwise version=\"4.0\">"), "{xml}");
    // Choir voices in the tempo and with the sound of the MIDI file; the head names the mode
    // and the voices, but no seed, which a rendered file does not know.
    assert!(xml.contains("<instrument-sound>voice.alto</instrument-sound>"));
    assert_eq!(xml.matches("<midi-program>53</midi-program>").count(), 2);
    assert!(xml.contains("<sound tempo=\"144\"/>"));
    assert!(xml.contains(">dorisch, S A</credit-words>"), "{xml}");
    assert!(
        !xml.contains("<credit-type>composer</credit-type>"),
        "{xml}"
    );
    // Read back, it is Fux's score again.
    let checked = fux(&["check", "--musicxml", path.to_str().unwrap()]);
    assert!(checked.status.success(), "{checked:?}");
    assert_eq!(stdout(&checked), stdout(&fux(&["check", "--example", "5"])));
    // Its cantus takes a new counterpoint, whose file names its seed, and has the tempo and the
    // sound given.
    let cp_base = tmp("cp-xml");
    let cp = fux(&[
        "cp",
        "--cf",
        path.to_str().unwrap(),
        "--seed",
        "2",
        "--tempo",
        "60",
        "--program",
        "20",
        "-f",
        "musicxml",
        "-o",
        cp_base.to_str().unwrap(),
    ]);
    assert!(cp.status.success(), "{cp:?}");
    assert!(
        stdout(&cp).contains("C. f. (Alt): D4 F4 E4 D4 G4 F4 A4 G4 F4 E4 D4\n"),
        "{}",
        stdout(&cp)
    );
    let xml = std::fs::read_to_string(cp_base.with_extension("musicxml")).unwrap();
    assert!(
        xml.contains(">fux-kontrapunkt, Seed 2</credit-words>"),
        "{xml}"
    );
    assert!(xml.contains("<sound tempo=\"120\"/>"), "{xml}");
    assert_eq!(xml.matches("<midi-program>20</midi-program>").count(), 2);
}

#[test]
fn a_workbook() {
    let base = tmp("book");
    let output = fux(&[
        "book",
        "--modes",
        "dorisch,hypophrygisch",
        "--species",
        "1,3",
        "--voices",
        "2,3",
        "--exercises",
        "--seed",
        "2",
        "-f",
        "ly,midi",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let text = stdout(&output);
    assert!(
        text.starts_with("Übungsheft, Profil fux, seed 2\n"),
        "{text}"
    );
    // Two modes, two species: above, below and in three voices each.
    assert_eq!(text.matches(": Kosten ").count(), 12, "{text}");
    assert!(text.contains("dorisch – 1. Gattung, Gegenstimme unten: Kosten"));
    assert!(text.contains("hypophrygisch – 3. Gattung, 3 Stimmen: Kosten"));
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert_eq!(ly.matches("\\bookpart").count(), 4);
    assert!(ly.contains("Aufgaben – Dorisch") && ly.contains("Lösungen – Hypophrygisch"));
    assert_eq!(ly.matches("\\score").count(), 24);
    // MIDI for the solutions only.
    assert!(base.with_extension("12.mid").is_file());
    assert!(!base.with_extension("13.mid").exists());
    let bad = fux(&["book", "--modes", "locrian"]);
    assert!(!bad.status.success());
}

/// Whether Palestrina's masses are there (`python3 tools/fetch_palestrina.py`). Without them a
/// test that needs them says so and returns; with `FUX_REQUIRE_CORPUS` set their absence fails it.
fn corpus_present(path: &Path) -> bool {
    if path.exists() {
        return true;
    }
    assert!(
        std::env::var_os("FUX_REQUIRE_CORPUS").is_none_or(|v| v.is_empty() || v == "0"),
        "{} is missing, and FUX_REQUIRE_CORPUS is set",
        path.display()
    );
    println!(
        "skipped: {} is missing (python3 tools/fetch_palestrina.py)",
        path.display()
    );
    false
}

#[test]
fn palestrina_in_free_counterpoint() {
    let kern = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../references/datasets/palestrina-masses/kern/Kyrie_00.krn"
    );
    if !corpus_present(Path::new(kern)) {
        return;
    }
    let checked = fux(&["check", "--kern", kern]);
    assert!(checked.status.success(), "{checked:?}");
    assert!(
        stdout(&checked).contains("0 Verbot(e)"),
        "{}",
        stdout(&checked)
    );
    let list = stdout(&fux(&["corpus", "--palestrina"]));
    // A header and every movement that reads (1318 files, 7 with tuplets or open endings).
    assert_eq!(list.lines().count(), 1 + 1311);
    assert!(list.contains("Missa Ad fugam"));
    // A file with a section of a divided movement: the movement, then the section.
    let crucifixus = list.lines().find(|l| l.starts_with("Credo_18_b")).unwrap();
    assert!(
        crucifixus.contains(" Credo  ") && crucifixus.contains(" Crucifixus "),
        "{crucifixus}"
    );
    let counts = stdout(&fux(&["rules", "--corpus", "palestrina", "--lang", "en"]));
    assert!(
        counts.starts_with("Violations in Palestrina's masses"),
        "{counts}"
    );
    assert!(counts.lines().any(|l| l.starts_with("FREE-RHYTHM ")));
    // The corpus has no text: the text rules never fire.
    let txt: Vec<Vec<&str>> = counts
        .lines()
        .filter(|l| l.starts_with("TXT-"))
        .map(|l| l.split_whitespace().skip(1).collect())
        .collect();
    assert_eq!(txt.len(), 12, "{counts}");
    assert!(txt.iter().all(|counts| counts == &["0", "0"]), "{counts}");
}

#[test]
fn errors_are_reported_in_the_chosen_language() {
    let args = [
        "render", "--cf", "D4 F4 E4", "--voice", "S=a'1 a'", "-f", "ly",
    ];
    let german = fux(&args);
    assert!(!german.status.success());
    let message = String::from_utf8_lossy(&german.stderr);
    assert!(
        message.starts_with("Fehler: Die Stimmen passen nicht zusammen"),
        "{message}"
    );
    let mut english_args = vec!["--lang", "en"];
    english_args.extend(args);
    let english = fux(&english_args);
    let message = String::from_utf8_lossy(&english.stderr);
    assert!(
        message.starts_with("Error: The voices do not fit together"),
        "{message}"
    );
}

#[test]
fn generate_cantus_firmi() {
    let base = tmp("cf");
    let output = fux(&[
        "cf",
        "--mode",
        "dorisch",
        "--seed",
        "5",
        "--count",
        "2",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let text = stdout(&output);
    assert!(text.starts_with("dorisch – 11 Töne, seed 5"), "{text}");
    assert!(
        text.contains("\n 1. D4 ") && text.contains("\n 2. D4 "),
        "{text}"
    );
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(ly.contains("Cantus firmus 2"));
    // The same seed gives the same result.
    let again = fux(&[
        "cf",
        "--mode",
        "dorisch",
        "--seed",
        "5",
        "--count",
        "2",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert_eq!(stdout(&again), text);
}

#[test]
fn check_reports_violations_and_exit_code() {
    let clean = fux(&["check", "--example", "5"]);
    assert_eq!(clean.status.code(), Some(0), "{clean:?}");
    assert!(stdout(&clean).contains("0 Verbot(e)"));
    let tritone = [
        "check",
        "--cf",
        "D4 F4 B4 A4 G4 F4 E4 D4",
        "--cf-voice",
        "A",
    ];
    let bad = fux(&tritone);
    assert_eq!(bad.status.code(), Some(2), "{bad:?}");
    assert!(stdout(&bad).contains("MEL-TRITONE"));
    let mut relaxed = tritone.to_vec();
    relaxed.extend(["--rule", "MEL-TRITONE=aus"]);
    assert_eq!(fux(&relaxed).status.code(), Some(0));
    let base = tmp("marked");
    let mut marked = tritone.to_vec();
    marked.extend(["--mark", "-f", "ly", "-o", base.to_str().unwrap()]);
    fux(&marked);
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(ly.contains("NoteHead.color") && ly.contains("MEL-TRITONE"));
}

#[test]
fn rules_are_listed() {
    let list = stdout(&fux(&["rules"]));
    assert!(list.contains("MEL-TRITONE") && list.contains("CF-CLIMAX"));
    let details = stdout(&fux(&["rules", "--id", "mel-octave"]));
    assert!(details.contains("g1_two_voices = V"), "{details}");
    let changed = stdout(&fux(&[
        "rules",
        "--id",
        "MEL-OCTAVE",
        "--rule",
        "MEL-OCTAVE.g1_two_voices=P2",
    ]));
    assert!(changed.contains("g1_two_voices = P2"), "{changed}");
    let corpus = stdout(&fux(&["rules", "--corpus"]));
    assert!(corpus.contains("123"));
    let bad = fux(&["rules", "--rule", "NOPE=V"]);
    assert_eq!(bad.status.code(), Some(1));
    // Jeppesen's profile comes with the program.
    let id = ["rules", "--id", "G3-ACCENT-LEAP"];
    assert!(stdout(&fux(&id)).contains("level = aus"));
    let jeppesen = stdout(&fux(&[&id[..], &["--profile", "jeppesen"]].concat()));
    assert!(jeppesen.contains("level = V"), "{jeppesen}");
    let missing = fux(&["rules", "--profile", "no-such-profile.toml"]);
    assert_eq!(missing.status.code(), Some(1));
    // The rules of the text underlay, provisional for want of a texted corpus.
    assert!(
        list.contains("TXT-ONSET") && list.contains("TXT-CADENCE"),
        "{list}"
    );
    let repeat = stdout(&fux(&["rules", "--id", "TXT-REPEAT"]));
    assert!(repeat.contains("Korpus ohne Text, vorläufig"), "{repeat}");
    assert!(repeat.contains("long = P2"), "{repeat}");
}

#[test]
fn counterpoint_to_a_fux_cantus_and_to_a_generated_one() {
    let base = tmp("cp");
    let output = fux(&[
        "cp",
        "--cf",
        "fux:dorian",
        "--seed",
        "2",
        "-f",
        "ly,midi",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let text = stdout(&output);
    assert!(text.contains("C. f. (Alt): D4 F4 E4"), "{text}");
    assert!(text.contains("1. Gegenstimme (Sopran):"), "{text}");
    assert!(
        text.contains("C#5 | D5"),
        "the cadence needs the leading tone: {text}"
    );
    let below = fux(&[
        "cp",
        "--cf",
        "fux:dorian",
        "--below",
        "--seed",
        "2",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(stdout(&below).contains("Gegenstimme (Tenor)"));
    let generated = fux(&[
        "cp",
        "--mode",
        "mixolydisch",
        "--seed",
        "4",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(generated.status.success(), "{generated:?}");
    assert!(stdout(&generated).starts_with("mixolydisch – 1. Gattung, seed 4"));
}

#[test]
fn counterpoint_in_the_second_to_fifth_species() {
    let base = tmp("species");
    for (species, sign) in [("2", "r "), ("3", " | "), ("4", "~ | "), ("5", "~ | ")] {
        let output = fux(&[
            "cp",
            "--cf",
            "fux:dorian",
            "--species",
            species,
            "--seed",
            "1",
            "-f",
            "ly",
            "-o",
            base.to_str().unwrap(),
        ]);
        assert!(output.status.success(), "{output:?}");
        let text = stdout(&output);
        assert!(text.contains(&format!("{species}. Gattung")), "{text}");
        let line = text
            .lines()
            .find(|l| l.contains("1. Gegenstimme"))
            .expect("a counterpoint");
        assert!(line.contains(sign), "{line}");
    }
}

#[test]
fn counterpoint_in_three_to_six_voices() {
    let base = tmp("voices");
    let four = fux(&[
        "cp",
        "--cf",
        "fux:dorian",
        "--voices",
        "4",
        "--species",
        "2",
        "--seed",
        "1",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(four.status.success(), "{four:?}");
    let text = stdout(&four);
    assert!(
        text.starts_with("dorisch – 2. Gattung, 4 Stimmen, seed 1"),
        "{text}"
    );
    assert!(text.contains("C. f. (Alt): D4 F4 E4"), "{text}");
    assert!(text.contains("Sopran (2. Gattung): r "), "{text}");
    assert!(
        text.contains("      Tenor: ") && text.contains("      Bass: "),
        "{text}"
    );
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert_eq!(ly.matches("\\new Staff =").count(), 4);
    let three = fux(&[
        "cp",
        "--cf",
        "fux:phrygian",
        "--parts",
        "S A B",
        "--cf-voice",
        "B",
        "--species",
        "4",
        "--voice",
        "S",
        "--seed",
        "2",
        "--lang",
        "en",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(three.status.success(), "{three:?}");
    let text = stdout(&three);
    assert!(
        text.starts_with("Phrygian – 4. species, 3 voices, seed 2"),
        "{text}"
    );
    assert!(
        text.contains("C. f. (Bass): E3"),
        "the cantus moves down into the bass: {text}"
    );
    assert!(
        text.contains("Soprano (4. species): r ") && text.contains("      Alto: "),
        "{text}"
    );
    let too_many = fux(&["cp", "--cf", "fux:dorian", "--voices", "7"]);
    assert!(!too_many.status.success());
}

#[test]
fn counterpoint_to_a_chant_with_internal_cadences() {
    let base = tmp("chant");
    let gabc = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/chant/veni-creator.gabc"
    );
    let run = |extra: &[&str]| {
        let mut args = vec![
            "cp",
            "--cf",
            gabc,
            "--cf-voice",
            "T",
            "--seed",
            "3",
            "-f",
            "ly",
            "-o",
            base.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        fux(&args)
    };
    let output = run(&["--voices", "4"]);
    assert!(output.status.success(), "{output:?}");
    let text = stdout(&output);
    assert!(
        text.starts_with("hypomixolydisch – 1. Gattung, 4 Stimmen, seed 3\nVeni Creator Spiritus"),
        "{text}"
    );
    assert!(
        text.contains("Binnenschlüsse: Takt 10 auf C, Takt 20 auf D"),
        "{text}"
    );
    assert!(
        text.contains("F#3 G3\n"),
        "the cantus gets the leading tone: {text}"
    );
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(
        ly.contains("\\lyricsto") && ly.contains("\"Ve\" -- \"ni\""),
        "{ly}"
    );
    let none = run(&["--voices", "3", "--cadences", "none"]);
    assert!(none.status.success(), "{none:?}");
    assert!(!stdout(&none).contains("Binnenschlüsse"));
    let fux_cantus = fux(&[
        "cp",
        "--cf",
        "fux:dorian",
        "--cadences",
        "4",
        "--seed",
        "1",
        "--lang",
        "en",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(fux_cantus.status.success(), "{fux_cantus:?}");
    assert!(stdout(&fux_cantus).contains("Internal cadences: measure 4 on D"));
    let bad = fux(&["cp", "--cf", "fux:dorian", "--cadences", "1"]);
    assert!(!bad.status.success());
}

#[test]
fn a_job_file_with_an_internal_cadence() {
    let job = tmp("cadence.toml");
    let write = |soprano: &str| {
        std::fs::write(
            &job,
            format!(
                "mode = \"dorian\"\ncadences = [4]\n\n[[voice]]\ntype = \"S\"\nnotes = \"{soprano}\"\n\n[[voice]]\ntype = \"A\"\nnotes = \"D4 C4 B3 A3 C4 E4 D4\"\ncf = true\n"
            ),
        )
        .unwrap();
    };
    write("d''1 a' gis' a' e'' cis'' d''");
    let good = fux(&["check", "--job", job.to_str().unwrap()]);
    assert_eq!(good.status.code(), Some(0), "{good:?}");
    write("d''1 a' g' a' e'' cis'' d''");
    let bad = fux(&["check", "--job", job.to_str().unwrap()]);
    assert_eq!(bad.status.code(), Some(2), "{bad:?}");
    assert!(stdout(&bad).contains("CAD-INNER-CLAUSULA"));
}

/// The lines of an output up to the written files.
fn listing(output: &Output) -> Vec<String> {
    stdout(output)
        .lines()
        .take_while(|l| !l.starts_with("geschrieben:") && !l.starts_with("written:"))
        .map(str::to_string)
        .collect()
}

#[test]
fn free_with_text() {
    let base = tmp("kyrie");
    let run = |accents: &str| {
        fux(&[
            "free",
            "--voices",
            "2",
            "--seed",
            "1",
            "--accents",
            accents,
            "-f",
            "ly",
            "-o",
            base.to_str().unwrap(),
        ])
    };
    let output = run("an");
    assert!(output.status.success(), "{output:?}");
    let text = stdout(&output);
    // Kyrie I by default, at the length and with the cadence it plans: after the first statement.
    assert!(
        text.starts_with(
            "dorisch – freier Satz, 2 Stimmen, 16 Takte, seed 1\nText: Kyrie I (7 Silben, 1 Sinnabschnitt), Wiederholung auto\nBinnenschlüsse: Takt 7 auf A: …"
        ),
        "{text}"
    );
    // Under each voice its text, the whole of it, with the acutes of the books.
    let lines = listing(&output);
    for name in ["Sopran", "Tenor"] {
        let at = lines
            .iter()
            .position(|l| l.starts_with(&format!("      {name}: ")))
            .expect("the voice");
        let sung = &lines[at + 1];
        assert!(
            sung.starts_with("        Text: Ký-ri-e e-lé-i-son."),
            "{text}"
        );
        assert!(sung.ends_with(" e-lé-i-son."), "{text}");
    }
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    for v in 0..2 {
        assert!(
            ly.contains(&format!(
                "\\new Lyrics = \"text{v}\" \\lyricsto \"voice{v}\" {{ \"Ký\" --"
            )),
            "{ly}"
        );
    }
    assert!(ly.contains("title = \"dorisch – Kyrie I\""), "{ly}");
    // Without the accents: the same notes, the syllables without the acute.
    let plain = run("aus");
    assert!(plain.status.success(), "{plain:?}");
    let plain_lines = listing(&plain);
    assert_eq!(plain_lines.len(), lines.len());
    for (with, without) in lines.iter().zip(&plain_lines) {
        assert_eq!(without, &with.replace('ý', "y").replace('é', "e"));
        assert!(!without.contains(['ý', 'é']), "{without}");
    }
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(
        ly.contains("\\lyricsto \"voice0\" { \"Ky\" --") && !ly.contains("Ký"),
        "{ly}"
    );
}

/// Free voices around a cantus (`fux cp --species 0`) sing a text if one is given; the cantus
/// sings none.
#[test]
fn free_voices_around_a_cantus_sing_a_text() {
    let base = tmp("cp-kyrie");
    let output = fux(&[
        "cp",
        "--cf",
        "fux:dorian",
        "--species",
        "0",
        "--voices",
        "3",
        "--text",
        "kyrie",
        "--seed",
        "1",
        "--details",
        "-f",
        "ly,musicxml",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let lines = listing(&output);
    assert_eq!(
        lines[..3],
        [
            "dorisch – freier Satz, 3 Stimmen, seed 1",
            "C. f. (Alt): D4 F4 E4 D4 G4 F4 A4 G4 F4 E4 D4",
            "Text: Kyrie I (7 Silben, 1 Sinnabschnitt), Wiederholung auto",
        ],
        "{lines:?}"
    );
    // Checked again from its MusicXML file, which names the text, the free voices have the
    // same violations; the check also reports those of the cantus, which the search does not
    // count.
    let violations: Vec<&str> = lines
        .iter()
        .filter(|l| l.starts_with("      Takt "))
        .map(|l| l.trim())
        .collect();
    assert!(!violations.is_empty(), "{lines:?}");
    let path = base.with_extension("musicxml");
    let checked = fux(&["check", "--musicxml", path.to_str().unwrap()]);
    assert_eq!(checked.status.code(), Some(0), "{checked:?}");
    let report = stdout(&checked);
    let mut report = report.lines();
    assert_eq!(
        report.next(),
        Some("Text: Kyrie I; von 18 gesungenen Silben weichen 0 ab"),
        "{checked:?}"
    );
    let found: Vec<&str> = report
        .filter(|l| l.starts_with("Takt ") && !l.contains(", Alt: "))
        .collect();
    assert_eq!(found, violations);
    // Under each free voice its text, the whole of it, with the acutes of the books.
    for name in ["Tenor", "Bass"] {
        let at = lines
            .iter()
            .position(|l| l.starts_with(&format!("      {name} (freier Satz): ")))
            .expect("the voice");
        let sung = &lines[at + 1];
        assert!(
            sung.starts_with("        Text: Ký-ri-e e-lé-i-son."),
            "{lines:?}"
        );
        assert!(sung.ends_with(" e-lé-i-son."), "{lines:?}");
    }
    assert_eq!(
        lines.iter().filter(|l| l.contains("Text: ")).count(),
        3,
        "{lines:?}"
    );
    // Lyrics under the tenor and the bass; the alto is the cantus, without any.
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    for v in 1..3 {
        assert!(
            ly.contains(&format!(
                "\\new Lyrics = \"text{v}\" \\lyricsto \"voice{v}\" {{ \"Ký\" --"
            )),
            "{ly}"
        );
    }
    assert_eq!(ly.matches("\\lyricsto").count(), 2, "{ly}");
    assert!(
        ly.contains("title = \"dorisch – freier Satz, Kyrie I\""),
        "{ly}"
    );
    // Two voices, in English, Kyrie I sung once, without accents and with e-léi-son.
    let once = fux(&[
        "--lang",
        "en",
        "cp",
        "--cf",
        "fux:dorian",
        "--species",
        "0",
        "--text",
        "kyrie",
        "--repeat",
        "keine",
        "--accents",
        "aus",
        "--eleison",
        "3",
        "--seed",
        "1",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(once.status.success(), "{once:?}");
    let lines = listing(&once);
    assert_eq!(
        lines[2],
        "Text: Kyrie I (6 syllables, 1 sense unit), repetition none"
    );
    assert!(
        lines[3].starts_with(" 1. Counterpoint (Soprano): "),
        "{lines:?}"
    );
    assert_eq!(lines[4], "      Text: Ky-ri-e e-lei-son.", "{lines:?}");
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(
        ly.contains("\\lyricsto \"voice0\" { \"Ky\" --") && !ly.contains("Ký"),
        "{ly}"
    );
    // Without a text, as before (the output of commit 49eed13).
    let untexted = fux(&[
        "cp",
        "--cf",
        "fux:dorian",
        "--species",
        "0",
        "--seed",
        "1",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(untexted.status.success(), "{untexted:?}");
    assert_eq!(
        listing(&untexted),
        [
            "dorisch – freier Satz, seed 1",
            "C. f. (Alt): D4 F4 E4 D4 G4 F4 A4 G4 F4 E4 D4",
            " 1. Gegenstimme (Sopran): r A4~ | A4 C5 B4 | C5 B4~ | B4 D5 C5 | B4 C5 | D5 | C5 E5~ | E5 D5 B4~ | B4 A4 B4 | C#5 | D5   (Kosten 0)",
        ]
    );
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(!ly.contains("\\new Lyrics"), "{ly}");
    // A text in the species, and a text too long for the cantus.
    let species = fux(&[
        "cp",
        "--cf",
        "fux:dorian",
        "--species",
        "2",
        "--text",
        "kyrie",
    ]);
    assert_eq!(species.status.code(), Some(1), "{species:?}");
    assert_eq!(
        stderr(&species),
        "Fehler: Einen Text singen nur freie Gegenstimmen (Gattung 0).\n"
    );
    let long = fux(&[
        "cp",
        "--cf",
        "fux:dorian",
        "--species",
        "0",
        "--text",
        "gloria:1-5",
    ]);
    assert_eq!(long.status.code(), Some(1), "{long:?}");
    assert_eq!(
        stderr(&long),
        "Fehler: Der Text braucht mindestens 19 Takte. Der Cantus hat 11 Takte. Abhilfe: ein \
         Cantus von mindestens 19 Tönen oder ein kürzerer Text (etwa weniger Sinnabschnitte wie \
         gloria:1-2).\n"
    );
}

#[test]
fn free_without_text() {
    // As before phase 12, note for note (the output of commit fe4c744).
    let base = tmp("untexted");
    let output = fux(&[
        "free",
        "--text",
        "none",
        "--voices",
        "2",
        "--seed",
        "1",
        "--length",
        "8",
        "--count",
        "2",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        listing(&output),
        [
            "dorisch – freier Satz, 2 Stimmen, 8 Takte, seed 1",
            " 1. Kosten 0",
            "      Sopran: r D4~ | D4 E4 | F4 A4 | G4 | F4 E4 | F4 D4~ | D4 C#4 | D4",
            "      Tenor: D4 | B3 C4 | A3 Bb3 D4 C4 | E4 D4 B3 C4 | D4 A3 G3 E3 | D3 F3 | E3 | D3",
            " 2. Kosten 0",
            "      Sopran: r D4~ | D4 E4 | F4 A4 | G4 | B4 A4 | G4 E4 D4~ | D4 C#4 | D4",
            "      Tenor: D4 | B3 C4 | A3 Bb3 D4 C4 | E4 D4 B3 C4 | G3 A3 | B3 C4 F3 G3 | E3 | D3",
        ]
    );
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(!ly.contains("\\new Lyrics"), "{ly}");
    assert!(ly.contains("title = \"dorisch – freier Satz\""), "{ly}");
    // Without a text the length is 12 measures unless given, with the usual cadence.
    let usual = fux(&[
        "free",
        "--text",
        "keine",
        "--voices",
        "2",
        "--seed",
        "3",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(usual.status.success(), "{usual:?}");
    assert_eq!(
        listing(&usual),
        [
            "dorisch – freier Satz, 2 Stimmen, 12 Takte, seed 3",
            "Binnenschlüsse: Takt 7 auf A",
            " 1. Kosten 1",
            "      Sopran: D5 | E5 C5~ | C5 Bb4 A4 | B4 G4 A4 B4 | C5 Bb4 A4~ | A4 G#4 | A4 | G4 F4 A4 G4 | F4 D4~ | D4 C4 D4~ | D4 C#4 | D4",
            "      Tenor: D4 | C4 Bb3 A3~ | A3 C4 D4~ | D4 B3 C4 | A3 Bb3 C4 D4 | B3 | A3 | E4 D4 C4~ | C4 A3 Bb3 | A3 F3 | E3 | D3",
        ]
    );
}

#[test]
fn text_command() {
    let kyrie = fux(&["text", "kyrie"]);
    assert!(kyrie.status.success(), "{kyrie:?}");
    assert_eq!(
        stdout(&kyrie),
        "Kyrie I (kyrie), kurz: 7 Silben, 2 Wörter, 1 Sinnabschnitt, 1 Satz\n  1  KÝ-ri-e / e-LÉ-i-son.\nZäsuren (|): keine\nSchlussteile (/): e-LÉ-i-son.\nBetonte Silben in Großbuchstaben. An einer Zäsur darf eine Stimme pausieren; ein Schlussteil darf für sich wiederholt werden.\nAutomatische Länge: 16 Takte; ohne Wiederholung (--repeat keine) 9 Takte\n"
    );
    let gloria = stdout(&fux(&["text", "gloria:1-5"]));
    assert!(
        gloria.starts_with(
            "Gloria: Et in terra 1–5 (gloria:1-5), lang: 36 Silben, 15 Wörter, 5 Sinnabschnitte, 5 Sätze\n  1  Et in TER-ra PAX | ho-MÍ-ni-bus BO-nae vo-lun-TÁ-tis.\n  2  Lau-DÁ-mus TE.\n"
        ),
        "{gloria}"
    );
    assert!(gloria.contains("\nZäsuren (|): nach PAX\n"), "{gloria}");
    assert!(
        gloria.ends_with("Automatische Länge: 32 Takte\n"),
        "{gloria}"
    );
    // An own text, also unquoted: an accent the program has to guess is a warning.
    let own = stdout(&fux(&["text", "Ad", "te", "levavi", "animam", "meam."]));
    assert!(
        own.starts_with("Eigener Text (eigen), kurz: 10 Silben"),
        "{own}"
    );
    assert!(
        own.contains("\nWarnung: Akzent unsicher: animam → "),
        "{own}"
    );
    let marked = stdout(&fux(&["text", "Ad te le-vá-vi á-ni-mam me-am."]));
    assert!(!marked.contains("Warnung"), "{marked}");
    // The sections with their syllables, sense units and automatic lengths.
    let list = fux(&["text", "--list"]);
    assert!(list.status.success(), "{list:?}");
    let list = stdout(&list);
    assert_eq!(list.lines().count(), 1 + 17 + 1, "{list}");
    let row = |id: &str| -> Vec<String> {
        list.lines()
            .find(|l| l.starts_with(&format!("{id} ")))
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect()
    };
    assert_eq!(
        row("kyrie"),
        ["kyrie", "Kyrie", "I", "kurz", "7", "1", "16"]
    );
    assert_eq!(
        row("gloria"),
        [
            "gloria", "Gloria:", "Et", "in", "terra", "lang", "98", "13", "84"
        ]
    );
    assert_eq!(
        row("agnus3"),
        ["agnus3", "Agnus", "Dei", "III", "kurz", "18", "3", "20"]
    );
    // e-léi-son in three syllables.
    let three = stdout(&fux(&["--lang", "en", "text", "kyrie", "--eleison", "3"]));
    assert!(
        three.starts_with("Kyrie I (kyrie), short: 6 syllables, 2 words, 1 sense unit, 1 sentence\n  1  KÝ-ri-e / e-LÉI-son.\n"),
        "{three}"
    );
}

#[test]
fn check_musicxml_with_text() {
    let base = tmp("kyrie-xml");
    let path = base.with_extension("musicxml");
    // Melismas beyond two notes cost, so that the setting has text cases to report.
    let rule = [
        "--rule",
        "TXT-MELISMA.max_short=2",
        "--rule",
        "TXT-MELISMA.max_last=2",
    ];
    let generated = fux(&[
        &[
            "free",
            "--voices",
            "2",
            "--seed",
            "1",
            "--mode",
            "phrygisch",
            "--details",
            "-f",
            "musicxml",
            "-o",
            base.to_str().unwrap(),
        ][..],
        &rule,
    ]
    .concat());
    assert!(generated.status.success(), "{generated:?}");
    let lines = listing(&generated);
    // The violations of the setting, as `--details` lists them, and its cost.
    let violations: Vec<&str> = lines
        .iter()
        .filter(|l| l.starts_with("      Takt "))
        .map(|l| l.trim_start())
        .collect();
    assert!(violations.iter().any(|l| l.contains("[TXT-")), "{lines:?}");
    let cost = lines
        .iter()
        .find_map(|l| l.strip_prefix(" 1. Kosten "))
        .expect("a cost");
    // The planned cadence, which the file keeps for the check besides its text and mode.
    let cadence = lines[2]
        .strip_prefix("Binnenschlüsse: Takt ")
        .and_then(|l| l.split_whitespace().next())
        .expect("a cadence");
    let xml = std::fs::read_to_string(&path).unwrap();
    assert!(
        xml.contains("<miscellaneous-field name=\"fux-text\">kyrie</miscellaneous-field>"),
        "{xml}"
    );
    // The same file as another program might write it, without these fields.
    let foreign = base.with_extension("foreign.musicxml");
    let start = xml.find("<miscellaneous>").unwrap();
    let end = xml.find("</miscellaneous>").unwrap() + "</miscellaneous>".len();
    std::fs::write(&foreign, format!("{}{}", &xml[..start], &xml[end..])).unwrap();
    let check_file = |file: &Path, extra: &[&str]| {
        fux(&[
            &["check", "--musicxml", file.to_str().unwrap()][..],
            &rule,
            extra,
        ]
        .concat())
    };
    let check = |extra: &[&str]| check_file(&path, extra);
    // The text the file names or the text given, or in another file the section recognised
    // and the cadence given: the same report as the setting's, with the syllables.
    for (file, extra, head) in [
        (&path, &[][..], "Text: Kyrie I; "),
        (&path, &["--text", "kyrie"], "Text: Kyrie I; "),
        (
            &foreign,
            &["--cadences", cadence],
            "Text erkannt: Kyrie I; ",
        ),
    ] {
        let checked = check_file(file, extra);
        assert_eq!(checked.status.code(), Some(0), "{checked:?}");
        let text = stdout(&checked);
        let checked: Vec<&str> = text.lines().collect();
        assert!(checked[0].starts_with(head), "{text}");
        assert!(checked[0].ends_with(" weichen 0 ab"), "{text}");
        assert_eq!(checked[1..checked.len() - 1], violations, "{text}");
        assert_eq!(
            checked[checked.len() - 1],
            format!(
                "0 Verbot(e), Kosten {cost} aus {} Präferenz(en).",
                violations.len()
            )
        );
    }
    // Without the cadence plan, the same but for TXT-CADENCE.
    let text = stdout(&check_file(&foreign, &[]));
    let checked: Vec<&str> = text.lines().filter(|l| l.contains("[TXT-")).collect();
    let expected: Vec<&str> = violations
        .iter()
        .copied()
        .filter(|l| l.contains("[TXT-") && !l.contains("[TXT-CADENCE"))
        .collect();
    assert_eq!(checked, expected, "{text}");
    // `none`: no text rules, and positions without syllables.
    let none = check(&["--text", "none"]);
    let text = stdout(&none);
    assert!(
        !text.contains("[TXT-") && !text.contains("Text") && !text.contains("(„"),
        "{text}"
    );
}

/// A setting of some units of a long text, exported and checked without naming the text: the
/// file names it, and a file without it is recognised as that range, not as an incomplete
/// Gloria (TXT-COMPLETE).
#[test]
fn check_a_range_without_its_text() {
    let base = tmp("gloria-1-3");
    let path = base.with_extension("musicxml");
    let generated = fux(&[
        "free",
        "--text",
        "gloria:1-3",
        "--voices",
        "2",
        "--seed",
        "1",
        "-f",
        "musicxml",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(generated.status.success(), "{generated:?}");
    let cost = listing(&generated)
        .iter()
        .find_map(|l| l.strip_prefix(" 1. Kosten ").map(str::to_string))
        .expect("a cost");
    let checked = fux(&["check", "--musicxml", path.to_str().unwrap()]);
    assert_eq!(checked.status.code(), Some(0), "{checked:?}");
    let text = stdout(&checked);
    // The same report as the setting's: its cost, the cadences of its plan included.
    assert!(
        text.starts_with("Text: Gloria: Et in terra 1–3; "),
        "{text}"
    );
    assert!(
        text.lines()
            .last()
            .unwrap()
            .starts_with(&format!("0 Verbot(e), Kosten {cost} aus ")),
        "{text}"
    );
    // Without the fields of the program the range is recognised.
    let xml = std::fs::read_to_string(&path).unwrap();
    let start = xml.find("<miscellaneous>").unwrap();
    let end = xml.find("</miscellaneous>").unwrap() + "</miscellaneous>".len();
    let foreign = base.with_extension("foreign.musicxml");
    std::fs::write(&foreign, format!("{}{}", &xml[..start], &xml[end..])).unwrap();
    let checked = fux(&["check", "--musicxml", foreign.to_str().unwrap()]);
    assert_eq!(checked.status.code(), Some(0), "{checked:?}");
    let text = stdout(&checked);
    assert!(
        text.starts_with("Text erkannt: Gloria: Et in terra 1–3; "),
        "{text}"
    );
    assert!(
        !text.contains("TXT-COMPLETE") && text.contains("0 Verbot(e)"),
        "{text}"
    );
}

#[test]
fn unknown_text_id() {
    let base = tmp("unknown");
    let free = fux(&[
        "free",
        "--text",
        "kyrei",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert_eq!(free.status.code(), Some(1), "{free:?}");
    let message = stderr(&free);
    assert!(
        message.starts_with(
            "Fehler: Unbekannter Abschnitt „kyrei“; die Abschnitte sind kyrie, christe, kyrie2, gloria, qui-tollis, credo, incarnatus, crucifixus, resurrexit, spiritum, sanctus, pleni, osanna, benedictus, osanna2, agnus, agnus3;"
        ),
        "{message}"
    );
    let text = fux(&["--lang", "en", "text", "kyrei"]);
    assert_eq!(text.status.code(), Some(1), "{text:?}");
    assert!(
        stderr(&text)
            .starts_with("Error: Unknown section \"kyrei\"; the sections are kyrie, christe,"),
        "{text:?}"
    );
    let check = fux(&["check", "--example", "5", "--text", "kyrei"]);
    assert_eq!(check.status.code(), Some(1), "{check:?}");
    let range = fux(&["text", "gloria:1-20"]);
    assert!(
        stderr(&range).contains(
            "Ungültiger Bereich „gloria:1-20“: Der Abschnitt hat die Sinnabschnitte 1 bis 13."
        ),
        "{range:?}"
    );
    // A text too long for the length given fails at once, with what may help; `auto` does not
    // imitate in a given length; an imitation asked for says how long its shortest plan is.
    let short = |imitation: &str| {
        fux(&[
            "free",
            "--text",
            "gloria:1-5",
            "--length",
            "10",
            "--imitation",
            imitation,
            "-f",
            "ly",
            "-o",
            base.to_str().unwrap(),
        ])
    };
    let plain = short("aus");
    assert_eq!(plain.status.code(), Some(1), "{plain:?}");
    let message = stderr(&plain);
    assert!(
        message.starts_with("Fehler: Der Text braucht mindestens ")
            && message.contains(" Takte. Abhilfe: --length "),
        "{message}"
    );
    let automatic = short("auto");
    assert_eq!(automatic.status.code(), Some(1), "{automatic:?}");
    assert!(
        stderr(&automatic).starts_with("Fehler: Der Text braucht mindestens "),
        "{automatic:?}"
    );
    let imitative = short("an");
    assert_eq!(imitative.status.code(), Some(1), "{imitative:?}");
    let message = stderr(&imitative);
    assert!(
        message.starts_with(
            "Fehler: Kein Plan der Imitation passt in die gegebene Länge; der kürzeste hat "
        ) && message.ends_with(" oder auto, oder --imitation aus.\n"),
        "{message}"
    );
}

/// `fux free --plan` shows the plan of the imitation without a search and writes no files: the
/// points with their words, entries, heads and cadences, where the subjects come from and every
/// entry's head. The options shape it; options that cannot apply are errors that say why.
#[test]
fn free_plan_and_imitation_options() {
    let base = tmp("plan");
    let ly = base.with_extension("ly");
    std::fs::remove_file(&ly).ok();
    let plan = |extra: &[&str]| {
        fux(&[
            &[
                "free",
                "--voices",
                "3",
                "--seed",
                "1",
                "--plan",
                "-f",
                "ly",
                "-o",
                base.to_str().unwrap(),
            ][..],
            extra,
        ]
        .concat())
    };
    let output = plan(&[]);
    assert!(output.status.success(), "{output:?}");
    assert!(!ly.exists(), "no files");
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines[0].starts_with("dorisch – Plan der Imitation, 3 Stimmen, ")
            && lines[0].ends_with(" Takte, seed 1"),
        "{text}"
    );
    assert_eq!(
        lines[1],
        "Text: Kyrie I (7 Silben, 1 Sinnabschnitt), Wiederholung auto"
    );
    assert!(lines[2].starts_with("Binnenschlüsse: Takt "), "{text}");
    // Kyrie I sung twice: the whole of it in every voice, then its tail or point 1 again.
    let points: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| l.starts_with("Punkt "))
        .collect();
    assert_eq!(points.len(), 2, "{text}");
    assert!(
        points[0].starts_with("Punkt 1 („Ký-ri-e e-lé-i-son.“): ")
            && points[0].contains("; Kopf ")
            && points[0].contains(" Töne, ")
            && points[0].contains("; Binnenschluss T. "),
        "{text}"
    );
    assert!(points[1].starts_with("Punkt 2 („"), "{text}");
    assert!(points[1].contains("; Schluss T. "), "{text}");
    // Point 1 has every voice, each with its head.
    for voice in ["Alt", "Tenor", "Bass"] {
        assert!(points[0].contains(&format!("{voice} T. ")), "{text}");
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with(&format!("  {voice} T. "))),
            "{text}"
        );
    }
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("  Soggetto aus dem Text: ") && l.contains(" „Ký-“")),
        "{text}"
    );
    // The order given: the bass leads, the alto follows; in English.
    let ordered = fux(&[
        "--lang", "en", "free", "--voices", "3", "--seed", "1", "--plan", "--order", "B A T",
    ]);
    assert!(ordered.status.success(), "{ordered:?}");
    let text = stdout(&ordered);
    let first = text
        .lines()
        .find(|l| l.starts_with("Point 1 "))
        .unwrap_or_default();
    assert!(
        first.starts_with("Point 1 (\"Ký-ri-e e-lé-i-son.\"): Bass m. 1 ")
            && first.find(" · Alto m. ") < first.find(" · Tenor m. "),
        "{text}"
    );
    // Subjects from the chant of the Kyriale, and from notes.
    let chant = stdout(&plan(&["--subject", "chant"]));
    assert!(
        chant.contains("  Soggetto aus dem Choral, Phrase 1: "),
        "{chant}"
    );
    let given = stdout(&plan(&["--subject", "d'1 d'2 a'2 g'4 f'4 e'2 d'1"]));
    // The leader sings the head given, in the octave of its voice.
    let head = given
        .lines()
        .find_map(|l| l.strip_prefix("  Vorgegebenes Soggetto: "))
        .unwrap_or_default()
        .replace(['\'', ','], "");
    assert_eq!(head, "d1 „Ký-“ d2 „ri-“ a2 „e“ g4 „e-“ f4 e2 d1", "{given}");
    // A hymn as a file, its mode the chant's, its phrases named.
    let hymn = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/chant/iste-confessor.gabc"
    );
    let iste = fux(&[
        "free",
        "--voices",
        "4",
        "--seed",
        "1",
        "--plan",
        "--text",
        "Lau-dá-te Dó-mi-num, / om-nes gen-tes.",
        "--subject",
        &format!("chant:{hymn}:2"),
    ]);
    assert!(iste.status.success(), "{iste:?}");
    let text = stdout(&iste);
    assert!(text.contains(", Phrase 2: "), "{text}");
    // A point whose chant phrase gives no usable head takes its subject from its words: the
    // plan says so, and so do the results of the search, once, for the plan they were set to.
    let fallback = "Warnung: Punkt 2: kein brauchbarer Kopf aus dem Choral; Soggetto aus dem Text";
    let tail = [
        "--mode",
        "dorian-transposed",
        "--subject",
        "chant:kyrie-ix:9",
        "--restate",
        "aus",
    ];
    let planned = stdout(&plan(&tail));
    assert!(planned.lines().any(|l| l == fallback), "{planned}");
    let set = tmp("fallback");
    let searched = fux(&[
        &[
            "free",
            "--voices",
            "3",
            "--seed",
            "1",
            "--beam",
            "40",
            "-f",
            "ly",
            "-o",
            set.to_str().unwrap(),
        ][..],
        &tail,
    ]
    .concat());
    assert!(searched.status.success(), "{searched:?}");
    let text = stdout(&searched);
    assert_eq!(text.lines().filter(|l| *l == fallback).count(), 1, "{text}");
    // Options that cannot apply.
    let error = |args: &[&str]| {
        let output = fux(args);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        stderr(&output)
    };
    // Options that the planner refuses, with what may help: a restatement of a text sung once,
    // and delays for more points than the text has.
    assert_eq!(
        error(&["free", "--plan", "--text", "gloria:1-5", "--restate", "an"]),
        "Fehler: Der Text wird nur einmal gesungen; es gibt keinen Punkt zu wiederholen. Abhilfe: --restate auto oder aus.\n"
    );
    assert_eq!(
        error(&[
            "--lang",
            "en",
            "free",
            "--voices",
            "3",
            "--seed",
            "1",
            "--plan",
            "--delays",
            "1; 1; 1; 1; 1; 1",
        ]),
        "Error: There is no point 6, the text has 2 points. Remedies: --order, --intervals and --delays for 2 points at most, separated by ;.\n"
    );
    assert_eq!(
        error(&["free", "--voices", "2", "--plan"]),
        "Fehler: --plan zeigt den Plan einer Imitation, aber dieser Satz imitiert nicht (auto: mit Text, drei bis fünf Stimmen, automatischen Binnenschlüssen und automatischer Länge). Abhilfe: --imitation an.\n"
    );
    assert_eq!(
        error(&["free", "--voices", "3", "--plan", "--order", "S A"]),
        "Fehler: --order: Die Besetzung hat keine Stimme S. Abhilfe: eine andere --order oder --order auto.\n"
    );
    // What the planner refuses, with its remedy: a voice twice, more later voices than the
    // choir has, a head longer than a subject.
    assert_eq!(
        error(&["free", "--plan", "--order", "A A"]),
        "Fehler: Die Reihenfolge von Punkt 1 nennt Stimme 2 zweimal. Abhilfe: eine andere --order oder --order auto.\n"
    );
    assert_eq!(
        error(&["free", "--plan", "--point-voices", "5"]),
        "Fehler: Spätere Punkte können nicht 5 Stimmen haben, nur zwei bis 4. Abhilfe: --point-voices auto, alle oder 2 bis 4.\n"
    );
    assert_eq!(
        error(&["free", "--plan", "--head", "11"]),
        "Fehler: Ein Kopf hat vier bis 10 Töne, nicht 11. Abhilfe: --head auto oder 4 bis 10.\n"
    );
    assert_eq!(
        error(&["free", "--imitation", "an", "--text", "none"]),
        "Fehler: Imitation braucht einen Text, denn ihre Punkte folgen den Worten. Abhilfe: ein Text (--text) oder --imitation aus.\n"
    );
    assert!(
        error(&[
            "free",
            "--plan",
            "--text",
            "Ad te le-vá-vi.",
            "--subject",
            "chant"
        ])
        .starts_with(
            "Fehler: Für den eigenen Text hat das Kyriale keinen Choral. Abhilfe: --subject chant:DATEI "
        )
    );
    // Intervals that begin with a sign, heads as note names, and the words without acutes.
    let plan = |args: &[&str]| {
        let output = fux(&[&["free", "--plan", "--seed", "1"][..], args].concat());
        assert!(output.status.success(), "{args:?}: {output:?}");
        stdout(&output)
    };
    assert!(plan(&["--intervals", "-3 d"]).contains("Punkt 1 („"));
    assert!(plan(&["--subject", "D4 F4 E4 D4 C4"]).contains("Vorgegebenes Soggetto: d'1"));
    assert!(plan(&["--accents", "aus"]).contains("Punkt 1 („Ky-ri-e e-le-i-son.“)"));
    // Syntax errors are clap's.
    for bad in [
        ["--delays", "1 x"],
        ["--delays", "1,5"],
        ["--intervals", "4"],
        ["--head", "3"],
        ["--imitation", "ja"],
        ["--subject", "x'1"],
    ] {
        let output = fux(&[&["free", "--plan"][..], &bad].concat());
        assert_eq!(output.status.code(), Some(2), "{bad:?}: {output:?}");
    }
    // Options of an imitation that does not apply are said to go unused.
    let unused = fux(&[
        "free",
        "--voices",
        "2",
        "--seed",
        "1",
        "--length",
        "8",
        "--pairs",
        "an",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(unused.status.success(), "{unused:?}");
    assert!(
        stdout(&unused)
            .starts_with("Warnung: Imitation aus (auto: mit Text, drei bis fünf Stimmen"),
        "{unused:?}"
    );
    let help = stdout(&fux(&["free", "--help"]));
    for option in [
        "--imitation",
        "--subject",
        "--points",
        "--restate",
        "--point-voices",
        "--order",
        "--intervals",
        "--delays",
        "--pairs",
        "--answer",
        "--head",
        "--plan",
        "--marks",
    ] {
        assert!(help.contains(option), "{option}");
    }
}

/// An imitative setting: a line on each of its points; brackets over its entries in LilyPond and
/// MusicXML, unless `--marks aus`; and the plan in the MusicXML file, so that a check of the
/// export repeats the setting's report, IMI cases included. Recognising the points instead
/// lists what it finds and checks no IMI rule.
#[test]
fn check_of_an_exported_setting_repeats_its_report() {
    let base = tmp("imitation");
    let path = base.with_extension("musicxml");
    let generate = |marks: &str| {
        fux(&[
            "free",
            "--voices",
            "3",
            "--seed",
            "1",
            "--beam",
            "40",
            "--details",
            "--marks",
            marks,
            "-f",
            "ly,musicxml",
            "-o",
            base.to_str().unwrap(),
        ])
    };
    let generated = generate("an");
    assert!(generated.status.success(), "{generated:?}");
    let lines = listing(&generated);
    let points: Vec<&String> = lines.iter().filter(|l| l.starts_with("Punkt ")).collect();
    assert!(points.len() >= 2, "{lines:?}");
    assert!(
        points[0].starts_with("Punkt 1 („Ký-ri-e e-lé-i-son.“): ")
            && points[0].contains("; Binnenschluss T. "),
        "{lines:?}"
    );
    // The point lines follow the cadences.
    let at = |prefix: &str| lines.iter().position(|l| l.starts_with(prefix));
    assert!(at("Binnenschlüsse: ") < at("Punkt 1 "), "{lines:?}");
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(
        ly.contains("\\consists \"Horizontal_bracket_engraver\""),
        "{ly}"
    );
    for point in 1..=points.len() {
        assert!(
            ly.contains(&format!("\\startGroup^\\markup \\small \"{point}\"")),
            "{ly}"
        );
    }
    assert_eq!(
        ly.matches("\\startGroup").count(),
        ly.matches("\\stopGroup").count()
    );
    let xml = std::fs::read_to_string(&path).unwrap();
    assert!(
        xml.contains("<miscellaneous-field name=\"fux-imitation\">p0-7"),
        "{xml}"
    );
    assert!(xml.contains("<words>1</words>") && xml.contains("<bracket type=\"start\""));
    assert_eq!(
        xml.matches("<bracket type=\"start\"").count(),
        xml.matches("<bracket type=\"stop\"").count()
    );
    // The violations of the setting, as `--details` lists them, and its cost.
    let violations: Vec<&str> = lines
        .iter()
        .filter(|l| l.starts_with("      Takt "))
        .map(|l| l.trim_start())
        .collect();
    let cost = lines
        .iter()
        .find_map(|l| l.strip_prefix(" 1. Kosten "))
        .expect("a cost");
    let check = |extra: &[&str]| {
        fux(&[&["check", "--musicxml", path.to_str().unwrap()][..], extra].concat())
    };
    let checked = check(&[]);
    assert_eq!(checked.status.code(), Some(0), "{checked:?}");
    let text = stdout(&checked);
    let report: Vec<&str> = text.lines().collect();
    assert!(report[0].starts_with("Text: Kyrie I; "), "{text}");
    assert_eq!(
        report[1],
        format!("Imitation nach dem Plan der Datei: {} Punkte", points.len())
    );
    // The same points as the setting's, and the same violations.
    for (line, point) in report[2..].iter().zip(&points) {
        assert_eq!(line, point);
    }
    let found: Vec<&str> = report
        .iter()
        .copied()
        .filter(|l| l.starts_with("Takt "))
        .collect();
    assert_eq!(found, violations, "{text}");
    assert_eq!(
        report.last().copied().unwrap_or_default(),
        format!(
            "0 Verbot(e), Kosten {cost} aus {} Präferenz(en).",
            violations.len()
        )
    );
    // Recognised instead: the points found, as information only, and no IMI case.
    let detected = stdout(&check(&["--imitation", "erkennen"]));
    assert!(
        detected.contains("\nImitation erkannt, nur zur Information: "),
        "{detected}"
    );
    assert!(detected.contains("\nPunkt 1: "), "{detected}");
    assert!(!detected.contains("[IMI-"), "{detected}");
    let off = stdout(&check(&["--imitation", "aus"]));
    assert!(
        !off.contains("[IMI-") && !off.contains("Imitation"),
        "{off}"
    );
    // A plan that does not fit the notes: an error that says what may help.
    let broken = base.with_extension("broken.musicxml");
    let field = "<miscellaneous-field name=\"fux-imitation\">p0-7";
    let start = xml.find(field).unwrap() + field.len();
    let spec_end = start + xml[start..].find('<').unwrap();
    let spec = &xml[start..spec_end];
    let at = spec.find('@').unwrap();
    let misfit = format!(
        "{}{}9{}",
        &xml[..start],
        &spec[..at - 1],
        &xml[start + at..]
    );
    std::fs::write(&broken, misfit).unwrap();
    let refused = fux(&["check", "--musicxml", broken.to_str().unwrap()]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    let message = stderr(&refused);
    assert!(
        message.starts_with("Fehler: Der Imitationsplan der Datei passt nicht zum Satz: ")
            && message.ends_with(" Abhilfe: --imitation aus.\n"),
        "{message}"
    );
    let ignored = fux(&[
        "check",
        "--musicxml",
        broken.to_str().unwrap(),
        "--imitation",
        "aus",
    ]);
    assert_eq!(ignored.status.code(), Some(0), "{ignored:?}");
    // The export rendered again keeps its brackets.
    let again = base.with_extension("again");
    let rendered = fux(&[
        "render",
        "--musicxml",
        path.to_str().unwrap(),
        "-f",
        "ly",
        "-o",
        again.to_str().unwrap(),
    ]);
    assert!(rendered.status.success(), "{rendered:?}");
    let ly = std::fs::read_to_string(again.with_extension("ly")).unwrap();
    assert!(ly.contains("\\startGroup"), "{ly}");
    // Without the marks: no brackets, but the plan stays in the file.
    let unmarked = generate("aus");
    assert!(unmarked.status.success(), "{unmarked:?}");
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(
        !ly.contains("startGroup") && !ly.contains("Horizontal_bracket"),
        "{ly}"
    );
    let xml = std::fs::read_to_string(&path).unwrap();
    assert!(!xml.contains("<bracket") && xml.contains("fux-imitation"));
}

/// `fux free --ending` (phase 14): the setting ends on the degree given, the bass on it, and the
/// summary says so; with imitation the last point closes there. An ending that the mode does not
/// allow, or two voices off the final (CAD-END), fail before the search, in either language.
#[test]
fn free_with_an_ending() {
    let base = tmp("ending");
    let output = fux(&[
        "free",
        "--voices",
        "3",
        "--text",
        "none",
        "--ending",
        "A",
        "--seed",
        "1",
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let text = stdout(&output);
    assert!(
        text.starts_with("dorisch – freier Satz, 3 Stimmen, 12 Takte, Schluss auf A, seed 1\n"),
        "{text}"
    );
    let bass = text
        .lines()
        .find_map(|l| l.strip_prefix("      Bass: "))
        .expect("the bass");
    let last = bass.split_whitespace().last().unwrap();
    assert!(last.starts_with('A') && !last.starts_with("Ab"), "{text}");
    // The plan of an imitation ends its last point on the ending.
    let plan = fux(&[
        "free", "--voices", "3", "--seed", "1", "--plan", "--ending", "A",
    ]);
    assert!(plan.status.success(), "{plan:?}");
    let text = stdout(&plan);
    let points: Vec<&str> = text.lines().filter(|l| l.starts_with("Punkt ")).collect();
    let last = points.last().expect("points");
    assert!(
        last.contains("; Schluss T. ") && last.ends_with(" auf A"),
        "{text}"
    );
    // What the mode or the voices cannot have.
    let refused = |args: &[&str]| {
        let output = fux(args);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        stderr(&output)
    };
    assert_eq!(
        refused(&["free", "--ending", "E"]),
        "Fehler: Ein Satz in diesem Modus schließt auf D, A oder G (Finalis, Quinte oder Quarte \
         mit einer Klausel), nicht auf E.\n"
    );
    assert_eq!(
        refused(&[
            "--lang", "en", "free", "--mode", "phrygian", "--ending", "C"
        ]),
        "Error: A setting in this mode ends on E or A (the final, or the fifth or the fourth with \
         a clausula), not on C.\n"
    );
    assert_eq!(
        refused(&["free", "--voices", "2", "--ending", "A"]),
        "Fehler: Zweistimmig schließt ein Satz auf der Finalis D (CAD-END), nicht auf A. \
         Abhilfe: drei Stimmen oder mehr (--voices) oder --ending D.\n"
    );
    let unknown = fux(&["free", "--ending", "X"]);
    assert!(!unknown.status.success());
    assert!(stderr(&unknown).contains("--ending"), "{unknown:?}");
}

/// `fux mass --plan` (phase 14, docs/design/phase-14/spec.md § 3.10) plans the mass without a
/// search and writes no files: the title with the mode, the choir and the seed, the movements,
/// sections, parts and syllables with the principal degree; the estimate natively with the parts
/// set at once (K6; with `--jobs 1` the sum of the parts) and in the browser; per movement its
/// parts, ‖ between sections and · between parts, each with how it begins (in imitation or
/// together, after a minim's breath or a measure's rest) and the degree it ends on.
#[test]
fn mass_plan() {
    let base = tmp("messe");
    let ly = base.with_extension("ly");
    std::fs::remove_file(&ly).ok();
    let plan = |args: &[&str]| {
        let output = fux(&[&["mass", "--plan", "--seed", "1"][..], args].concat());
        assert!(output.status.success(), "{output:?}");
        stdout(&output)
    };
    let brevis = plan(&["-o", base.to_str().unwrap(), "--jobs", "4"]);
    assert!(!ly.exists(), "no files");
    assert_eq!(
        brevis.lines().collect::<Vec<_>>(),
        [
            "Missa brevis – dorisch, S A T B, Seed 1: 6 Sätze, 10 Abschnitte, 26 Teile, 657 \
             Silben, Hauptstufe A",
            "geschätzt 942 Takte, etwa 2 min (Browser, 4 Worker: etwa 4 min)",
            "Kyrie: Kyrie I (Imitation) → D ‖ Christe (Imitation) → A ‖ Kyrie II (Imitation) → D",
            "Gloria: gloria:1-5 (Imitation) → A · gloria:6-9 (gemeinsam, Halbe Atem) → D · \
             gloria:10-13 (Imitation, Halbe Atem) → A · Qui tollis (gemeinsam, Takt Pause) → D · \
             qui-tollis:5-8 (Imitation, Halbe Atem) → D · qui-tollis:9-13 (gemeinsam, Halbe Atem) \
             → D",
            "Credo: credo:1-4 (Imitation) → D · credo:5-7 (gemeinsam, Halbe Atem) → D · credo:8-14 \
             (gemeinsam, Halbe Atem) → A · credo:15-16 (gemeinsam, Halbe Atem) → D · Et incarnatus \
             est (gemeinsam, Takt Pause) → D · Crucifixus (gemeinsam, Takt Pause) → A · \
             resurrexit:1-4 (gemeinsam, Halbe Atem) → D · resurrexit:5-7 (Imitation, Halbe Atem) → \
             A · spiritum:1-4 (gemeinsam, Halbe Atem) → D · spiritum:5-7 (gemeinsam, Halbe Atem) → \
             A · spiritum:8-9 (Imitation, Halbe Atem) → D · spiritum:10-12 (gemeinsam, Halbe Atem) \
             → D",
            "Sanctus: Sanctus (Imitation) → D",
            "Benedictus: Benedictus (S A T, Imitation) → A ‖ Osanna (gemeinsam) → D",
            "Agnus Dei: Agnus Dei I (Imitation) → D ‖ Agnus Dei II (S A T T B, Imitation) → D",
        ]
    );
    // One part after the other: the sum of the parts' estimates, as before K6.
    assert_eq!(
        plan(&["--jobs", "1"]).lines().nth(1),
        Some("geschätzt 942 Takte, etwa 3 min (Browser, 4 Worker: etwa 4 min)")
    );
    // The form normal in three voices: Gloria, Credo and Sanctus in sections, the Benedictus
    // with „Osanna ut supra“; three voices keep every section.
    assert_eq!(
        plan(&["--preset", "normal", "--voices", "3", "--jobs", "4"])
            .lines()
            .collect::<Vec<_>>(),
        [
            "Missa – dorisch, A T B, Seed 1: 6 Sätze, 15 Abschnitte, 27 Teile, 650 Silben, \
             Hauptstufe A",
            "geschätzt 965 Takte, etwa 1 min (Browser, 4 Worker: etwa 2 min)",
            "Kyrie: Kyrie I (Imitation) → D ‖ Christe (Imitation) → A ‖ Kyrie II (Imitation) → D",
            "Gloria: gloria:1-5 (Imitation) → D · gloria:6-9 (gemeinsam, Halbe Atem) → D · \
             gloria:10-13 (gemeinsam, Halbe Atem) → A ‖ qui-tollis:1-4 (Imitation) → A · \
             qui-tollis:5-8 (Imitation, Halbe Atem) → D · qui-tollis:9-13 (gemeinsam, Halbe Atem) \
             → D",
            "Credo: credo:1-4 (Imitation) → A · credo:5-7 (gemeinsam, Halbe Atem) → D · credo:8-14 \
             (gemeinsam, Halbe Atem) → A · credo:15-16 (gemeinsam, Halbe Atem) → D · Et incarnatus \
             est (gemeinsam, Takt Pause) → A ‖ crucifixus (Imitation) → D · resurrexit:1-4 \
             (gemeinsam, Halbe Atem) → D · resurrexit:5-7 (gemeinsam, Halbe Atem) → A ‖ \
             spiritum:1-4 (Imitation) → D · spiritum:5-7 (Imitation, Halbe Atem) → D · \
             spiritum:8-9 (Imitation, Halbe Atem) → D · spiritum:10-12 (gemeinsam, Halbe Atem) → D",
            "Sanctus: Sanctus (Imitation) → D ‖ Pleni sunt caeli (Imitation) → A ‖ Osanna \
             (Imitation) → D",
            "Benedictus: Benedictus (Imitation) → D ‖ Osanna ut supra",
            "Agnus Dei: Agnus Dei I (Imitation) → D ‖ Agnus Dei II (Imitation) → D",
        ]
    );
    // In English, a title of one's own names the form; with --details a line per part with
    // its address.
    let english = fux(&[
        "--lang",
        "en",
        "mass",
        "--plan",
        "--details",
        "--seed",
        "1",
        "--movements",
        "kyrie",
        "--mode",
        "ionisch-transponiert",
        "--title",
        "Missa prima",
    ]);
    assert!(english.status.success(), "{english:?}");
    let text = stdout(&english);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 6, "{text}");
    assert_eq!(
        lines[0],
        "Missa prima (form brevis) – Ionian, transposed to F, S A T B, seed 1: 1 movement, 3 \
         sections, 3 parts, 20 syllables, principal degree C",
        "{text}"
    );
    assert!(lines[1].starts_with("estimated "), "{text}");
    assert!(
        lines[2].starts_with("Kyrie: Kyrie I (imitation) → F ‖ Christe (imitation) → "),
        "{text}"
    );
    assert!(
        lines[3].starts_with("kyrie/1: Kyrie I (imitation), S A T B, ending F, seed ")
            && lines[5].starts_with("kyrie/3: Kyrie II (imitation), S A T B, ending F, seed "),
        "{text}"
    );
}

/// Every option of `fux mass` that the plan cannot have fails before any search, with its
/// message in the language chosen: an unknown place, an ending the mode does not allow, the
/// scoring of a section the form does not have, a size of parts out of range, a part beyond a
/// movement in `--only`.
#[test]
fn mass_options_fail_early() {
    let refused = |args: &[&str]| {
        let output = fux(args);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        stderr(&output)
    };
    let both = |args: &[&str], de: &str, en: &str| {
        assert_eq!(refused(&[&["mass", "--plan"][..], args].concat()), de);
        assert_eq!(
            refused(&[&["--lang", "en", "mass", "--plan"][..], args].concat()),
            en
        );
    };
    let valid = "kyrie, christe, kyrie2, gloria, qui-tollis, credo, incarnatus, crucifixus, \
                 sanctus, benedictus, osanna2, agnus, agnus3";
    both(
        &["--imitation", "auto; benedictus=an; pleni=aus"],
        &format!(
            "Fehler: Unbekannte Stelle „pleni“; gültig sind Satz/Teil wie credo/3 und {valid}.\n"
        ),
        &format!(
            "Error: Unknown place \"pleni\"; valid are movement/part such as credo/3 and {valid}.\n"
        ),
    );
    both(
        &["--endings", "christe=E"],
        "Fehler: „christe“: Ein Teil endet nicht auf E, sondern auf der Finalis, der Quinte oder \
         der Quarte des Modus mit einer Klausel: D, A oder G.\n",
        "Error: \"christe\": a part ends not on E but on the final, the fifth or the fourth of \
         the mode with a clausula: D, A or G.\n",
    );
    both(
        &["--endings", "kyrie2=A"],
        "Fehler: „kyrie2“: Der letzte Teil des Kyrie endet auf der Finalis.\n",
        "Error: \"kyrie2\": the last part of the Kyrie ends on the final.\n",
    );
    both(
        &["--scoring", "pleni=S A T"],
        &format!(
            "Fehler: Unbekannte Stelle „pleni“; gültig sind Satz/Teil wie credo/3 und {valid}.\n"
        ),
        &format!(
            "Error: Unknown place \"pleni\"; valid are movement/part such as credo/3 and {valid}.\n"
        ),
    );
    both(
        &["--scoring", "incarnatus=S A T"],
        "Fehler: --scoring nimmt Abschnitte wie benedictus oder agnus3, nicht „incarnatus“.\n",
        "Error: --scoring takes sections such as benedictus or agnus3, not \"incarnatus\".\n",
    );
    both(
        &["--chunk", "10"],
        "Fehler: Die Größe der Teile „10“ ist weder eine Zahl von 20 bis 200 Silben noch \
         „aus“.\n",
        "Error: The size of the parts \"10\" is neither a number of 20 to 200 syllables nor \
         \"aus\" (off).\n",
    );
    both(
        &["--only", "credo/2,credo/13"],
        "Fehler: --only: „credo/13“: Das Credo hat 12 Teile.\n",
        "Error: --only: \"credo/13\": the Credo has 12 parts.\n",
    );
    both(
        &["--movements", "kyrie", "--only", "gloria/1"],
        "Fehler: --only: „gloria/1“: Das Gloria wird nicht gesetzt.\n",
        "Error: --only: \"gloria/1\": the Gloria is not set.\n",
    );
    // The same checks run without --plan.
    assert_eq!(
        refused(&["mass", "--chunk", "300"]),
        "Fehler: Die Größe der Teile „300“ ist weder eine Zahl von 20 bis 200 Silben noch \
         „aus“.\n"
    );
}

/// The options of the small mass that the tests of `fux mass` set: the Kyrie of a Missa brevis
/// for A T B with a narrow search, which takes seconds in a debug build. The Christe ends on the
/// fifth, so that a part ends off the final, and Kyrie II begins together, so that a part has
/// no plan of imitation.
const SMALL: [&str; 10] = [
    "--movements",
    "kyrie",
    "--voices",
    "3",
    "--beam",
    "40",
    "--endings",
    "christe=A",
    "--imitation",
    "kyrie2=aus",
];

/// The plan of `fux mass` with the options of a mass `args`, as the program plans it: what the
/// tests expect of its output follows from it, not from the draws of one version of the tables.
fn plan_of(args: &[&str]) -> MassPlan {
    #[derive(Parser)]
    struct Line {
        #[command(flatten)]
        options: MassOptions,
    }
    let options = Line::parse_from(std::iter::once("mass").chain(args.iter().copied())).options;
    let (request, _) = mass::prepare_mass(&options, &Profile::fux(), &[], Lang::De).unwrap();
    mass::mass_plan(&request, Lang::De).unwrap()
}

/// A mass as `fux mass` wrote it to `base`, with what it printed.
struct Written {
    base: PathBuf,
    output: Output,
}

/// The formats of the masses of the tests: all four, the default, where LilyPond is installed;
/// else all but the PDF.
fn mass_formats() -> &'static [&'static str] {
    static FORMATS: OnceLock<bool> = OnceLock::new();
    if *FORMATS.get_or_init(|| fux_export::lilypond_version().is_some()) {
        &[]
    } else {
        &["-f", "ly,midi,musicxml"]
    }
}

/// Sets a mass with the options `args` and writes it to `NAME` in the fresh folder `folder`.
fn set_mass(folder: &str, args: &[&str]) -> Written {
    let dir = tmp(folder);
    std::fs::remove_dir_all(&dir).ok();
    let base = dir.join("messe");
    let output = fux(&[
        &["mass", "-o", base.to_str().unwrap()][..],
        args,
        mass_formats(),
    ]
    .concat());
    Written { base, output }
}

/// The small mass with seed 1 and `--details`, set once for the tests that read it.
fn small_mass() -> &'static Written {
    static MASS: OnceLock<Written> = OnceLock::new();
    MASS.get_or_init(|| {
        set_mass(
            "messe-klein",
            &[&SMALL[..], &["--seed", "1", "--details"]].concat(),
        )
    })
}

/// The plan of the small mass.
fn small_plan() -> MassPlan {
    plan_of(&[&SMALL[..], &["--seed", "1"]].concat())
}

/// The options of a second small mass, seed 1, whose Christe sings in two voices, A T: the bass
/// rests through it, as a voice does through a section it does not sing in. Kyrie II begins
/// together. Some seconds in a debug build.
const TACET: [&str; 12] = [
    "--movements",
    "kyrie",
    "--voices",
    "3",
    "--beam",
    "40",
    "--imitation",
    "kyrie2=aus",
    "--scoring",
    "christe=A T",
    "--seed",
    "1",
];

/// The mass of [`TACET`] with Mensurstriche and the bass hidden where it rests, set once for the
/// tests that read it.
fn tacet_mass() -> &'static Written {
    static MASS: OnceLock<Written> = OnceLock::new();
    MASS.get_or_init(|| {
        set_mass(
            "messe-tacet",
            &[&TACET[..], &["--mensurstriche", "--tacet", "verbergen"]].concat(),
        )
    })
}

/// The names of the tempo track of a MIDI file, the number of its tracks, and the markers of the
/// tempo track: the titles of its sections and Breaks.
fn midi_markers(midi: &[u8]) -> (Vec<String>, usize, Vec<String>) {
    let smf = Smf::parse(midi).unwrap();
    let mut names = Vec::new();
    let mut markers = Vec::new();
    for event in &smf.tracks[0] {
        match event.kind {
            TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                names.push(String::from_utf8_lossy(name).into_owned());
            }
            TrackEventKind::Meta(MetaMessage::Marker(text)) => {
                markers.push(String::from_utf8_lossy(text).into_owned());
            }
            _ => {}
        }
    }
    (names, smf.tracks.len(), markers)
}

/// Validates a MusicXML file with xmllint against the MusicXML 4.0 schema, where
/// `FUX_MUSICXML_XSD` names a copy of `musicxml.xsd` (with `xml.xsd` and `xlink.xsd` beside it);
/// returns whether it did.
fn validate_musicxml(path: &Path) -> bool {
    let Some(xsd) = std::env::var_os("FUX_MUSICXML_XSD") else {
        return false;
    };
    let output = Command::new("xmllint")
        .args(["--noout", "--nonet", "--schema"])
        .arg(xsd)
        .arg(path)
        .output()
        .expect("xmllint");
    assert!(
        output.status.success(),
        "{}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

/// The files of the movements of the mass of `plan` written to `base`, whose parts have the
/// measures and costs `parts` (§ 6): the MusicXML file of each movement with the title of the
/// mass and of the movement, its sections by their titles and the fields of its mass and of
/// every part, with its measures in the movement, its text and the positions of its syllables;
/// its MIDI file named after the mass and the movement, a track per voice, a marker per section
/// and Break.
fn assert_movements(base: &Path, plan: &MassPlan, parts: &[(usize, u32)]) {
    let mass = MassField::of(plan).to_string();
    for (m, movement) in plan.movements.iter().enumerate() {
        let title = movement.movement.title();
        let xml =
            std::fs::read_to_string(movement_path(base, movement.movement, "musicxml")).unwrap();
        assert!(xml.contains(&format!("<work-title>{}</work-title>", plan.title)));
        assert!(xml.contains(&format!("<movement-title>{title}</movement-title>")));
        let read = from_musicxml(&xml).unwrap();
        assert_eq!(read.field(FIELD_MASS), Some(mass.as_str()));
        let mut first = 1;
        let places = plan
            .parts()
            .zip(parts)
            .filter(|((at, _), _)| at.movement == m);
        for (n, ((_, part), (measures, _))) in places.enumerate() {
            let number = n + 1;
            let field = read.field(&format!("{FIELD_PART}.{number}")).unwrap();
            let field: PartField = field.parse().unwrap();
            assert_eq!(
                (field.bars, field.ending, field.continued),
                (first..=first + measures - 1, part.ending, part.continued)
            );
            let text = read.field(&format!("fux-text.{number}"));
            assert_eq!(text, Some(part.text.as_str()));
            let lyrics = read.field(&format!("{FIELD_LYRICS}.{number}"));
            assert!(lyrics.is_some_and(|lyrics| !lyrics.is_empty()));
            first += measures;
        }
        let sections: Vec<&str> = read
            .sections
            .iter()
            .filter(|mark| mark.kind == SectionKind::Section)
            .filter_map(|mark| mark.title.as_deref())
            .collect();
        let planned: Vec<&str> = movement.sections.iter().map(|s| s.title).collect();
        assert_eq!(sections, planned, "{title}");
        let marked: Vec<String> = read
            .sections
            .iter()
            .filter(|mark| mark.kind != SectionKind::Seam)
            .filter_map(|mark| mark.title.clone())
            .collect();
        let midi = std::fs::read(movement_path(base, movement.movement, "mid")).unwrap();
        let (names, tracks, markers) = midi_markers(&midi);
        assert_eq!(names, [format!("{} – {title}", plan.title)]);
        assert_eq!(tracks, 1 + read.parts.len());
        assert_eq!(markers, marked, "{title}");
    }
}

/// `fux check` of the file of every movement of the mass of `plan` written to `base`, whose parts
/// have the measures and costs `parts`, repeats the report of every part (D14): part by part, in
/// the measures of the movement, each with the cost of its search, no prohibition; the totals
/// last. Returns the reports.
fn assert_checked(base: &Path, plan: &MassPlan, parts: &[(usize, u32)]) -> Vec<String> {
    let mut reports = Vec::new();
    for (m, movement) in plan.movements.iter().enumerate() {
        let file = movement_path(base, movement.movement, "musicxml");
        let checked = fux(&["check", "--musicxml", file.to_str().unwrap()]);
        assert_eq!(checked.status.code(), Some(0), "{checked:?}");
        let report = stdout(&checked);
        let mut first = 1;
        let mut expected = Vec::new();
        let places = plan
            .parts()
            .zip(parts)
            .filter(|((at, _), _)| at.movement == m);
        for ((at, _), (measures, cost)) in places {
            let last = first + measures - 1;
            let heading = format!(
                "Teil {}: {}, Takte {first}–{last}",
                plan.address(at),
                part_label(plan, at)
            );
            assert!(
                report.lines().any(|line| line == heading),
                "{heading}\n{report}"
            );
            expected.push(*cost);
            first = last + 1;
        }
        let costs: Vec<u32> = report
            .lines()
            .filter_map(|line| line.strip_prefix("  0 Verbot(e), Kosten "))
            .filter_map(|rest| rest.split(' ').next()?.parse().ok())
            .collect();
        assert_eq!(costs, expected, "{report}");
        let total: u32 = expected.iter().sum();
        let n = expected.len();
        let teile = if n == 1 { "Teil" } else { "Teilen" };
        let last = report.lines().last().unwrap_or_default();
        assert!(
            last.starts_with(&format!("Gesamt: 0 Verbot(e), Kosten {total} aus "))
                && last.ends_with(&format!(" in {n} {teile}.")),
            "{report}"
        );
        reports.push(report);
    }
    reports
}

/// The files of the parts of the mass written to `from`, copied to a mass in the fresh folder
/// `folder`, whose base it returns.
fn copy_parts(from: &Path, folder: &str) -> PathBuf {
    let dir = tmp(folder);
    std::fs::remove_dir_all(&dir).ok();
    let base = dir.join("messe");
    let parts = parts_dir(&base);
    std::fs::create_dir_all(&parts).unwrap();
    for entry in std::fs::read_dir(parts_dir(from)).unwrap() {
        let path = entry.unwrap().path();
        std::fs::copy(&path, parts.join(path.file_name().unwrap())).unwrap();
    }
    base
}

/// The measures and the cost after the heading of the line of a part: „28 Takte, Kosten 35, …“.
fn measures_and_cost(line: &str) -> (usize, u32) {
    let mut items = line.split(", ");
    let measures = items
        .next()
        .and_then(|item| item.strip_suffix(" Takte"))
        .and_then(|n| n.parse().ok());
    let cost = items
        .next()
        .and_then(|item| item.strip_prefix("Kosten "))
        .and_then(|n| n.parse().ok());
    measures.zip(cost).unwrap_or_else(|| panic!("{line}"))
}

/// A degree as a mode spells it in German.
fn degree(plan: &MassPlan, movement: usize, step: Step) -> String {
    let mode = plan.movements[movement].mode;
    Pitch::new(step, mode.scale_alter(step), 4).class_name(Lang::De)
}

/// The heading of the line of each part of the mass of `plan` written to `base`, as `fux mass`
/// prints it once the part is set: with the voices that the file of the part says it sings,
/// which after a fallback of the scoring are not those of its section.
fn headings(base: &Path, plan: &MassPlan) -> Vec<(String, usize)> {
    plan.parts()
        .map(|(at, _)| {
            let file = std::fs::read_to_string(part_path(base, plan, at)).unwrap();
            let read = from_musicxml(&file).unwrap();
            let field: PartField = read.field(FIELD_PART).unwrap().parse().unwrap();
            let movement = &plan.movements[at.movement];
            let voices: Vec<&str> = field
                .voices
                .iter()
                .map(|&v| movement.voices[v].abbreviation())
                .collect();
            let line = PartLine {
                voices: voices.join(" "),
                ..PartLine::of(plan, at, Lang::De)
            };
            (line.heading(Lang::De), voices.len())
        })
        .collect()
}

/// What `fux mass` prints and writes for a complete mass of `plan` (§ 7): the plan, a line per
/// part in the order of the mass, with `details` the summary of each part under it, the cadence
/// plan of each movement through its measures, the totals with the warnings, and the files: the
/// book with a part per movement and a score per section, MusicXML and MIDI per movement
/// ([`assert_movements`]), the report as text, and the file of every part with the fields of its
/// mass and its part. On stderr one line per part, with the time the mass still takes until its
/// last part. Returns the measures and the cost of each part.
fn assert_written(written: &Written, plan: &MassPlan, details: bool) -> Vec<(usize, u32)> {
    let output = &written.output;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = stdout(output);
    let summary = mass_summary(plan, default_jobs(), Lang::De);
    assert!(text.starts_with(&format!("{summary}\n")), "{text}");
    let mut lines = text.lines().skip(summary.lines().count()).peekable();
    let headings = headings(&written.base, plan);
    let mut parts = Vec::new();
    for ((at, part), (heading, voices)) in plan.parts().zip(&headings) {
        let line = lines.next().unwrap_or_default();
        let set = line
            .strip_prefix(&format!("{heading}: "))
            .unwrap_or_else(|| panic!("{heading}\n{text}"));
        assert!(set.ends_with(" s"), "{line}");
        let found = measures_and_cost(set);
        parts.push(found);
        let below: Vec<&str> =
            std::iter::from_fn(|| lines.next_if(|l| l.starts_with("  "))).collect();
        if details {
            let mode = plan.movements[at.movement].mode.name(Lang::De);
            let first = format!(
                "  {mode} – freier Satz, {voices} Stimmen, {} Takte, Schluss auf {}, seed {}",
                found.0,
                degree(plan, at.movement, part.ending),
                part.seed
            );
            assert_eq!(below.first().copied(), Some(first.as_str()), "{text}");
        } else {
            assert!(below.is_empty(), "{text}");
        }
    }
    for (m, movement) in plan.movements.iter().enumerate() {
        let measures: usize = plan
            .parts()
            .zip(&parts)
            .filter(|((at, _), _)| at.movement == m)
            .map(|(_, (measures, _))| measures)
            .sum();
        let (_, last) = plan
            .parts()
            .filter(|(at, _)| at.movement == m)
            .last()
            .unwrap();
        let line = lines.next().unwrap_or_default();
        assert!(
            line.starts_with(&format!("{} – Kadenzplan: T. ", movement.movement.title()))
                && line.ends_with(&format!(" T. {measures} {}", degree(plan, m, last.ending))),
            "{line}"
        );
    }
    let (measures, cost) = parts
        .iter()
        .fold((0, 0), |(m, c), &(measures, cost)| (m + measures, c + cost));
    let n = plan.part_count();
    let totals = lines.next().unwrap_or_default();
    let warnings = std::iter::from_fn(|| lines.next_if(|l| l.starts_with("Warnung: "))).count();
    assert!(
        totals.starts_with(&format!(
            "Gesamt: {n} von {n} Teilen, {measures} Takte, Kosten {cost}, Suchzeit "
        )) && totals.ends_with(&match warnings {
            0 => "keine Warnung".to_string(),
            1 => "1 Warnung".to_string(),
            n => format!("{n} Warnungen"),
        }),
        "{totals}"
    );
    // The files, the book first, the folder of the parts last.
    let base = &written.base;
    let mut files = vec![with_extension(base, "ly")];
    if mass_formats().is_empty() {
        files.push(with_extension(base, "pdf"));
    }
    for kind in ["musicxml", "mid"] {
        files.extend(
            plan.movements
                .iter()
                .map(|movement| movement_path(base, movement.movement, kind)),
        );
    }
    files.extend([with_extension(base, "txt"), parts_dir(base)]);
    let listed: Vec<&str> = lines
        .map(|line| line.strip_prefix("geschrieben: ").expect("a file"))
        .collect();
    let expected: Vec<String> = files.iter().map(|f| f.display().to_string()).collect();
    assert_eq!(listed, expected);
    for file in &files {
        assert!(file.exists(), "{}", file.display());
    }
    let report: String = text
        .lines()
        .filter(|line| !line.starts_with("geschrieben: "))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_eq!(
        std::fs::read_to_string(with_extension(base, "txt")).unwrap(),
        report
    );
    // The book: the title with the mode, the choir and the seed, a part per movement under its
    // title, a score per section under its own.
    let ly = std::fs::read_to_string(with_extension(base, "ly")).unwrap();
    let mut choir = plan.choir.clone();
    choir.sort();
    let choir: Vec<&str> = choir.iter().map(|voice| voice.abbreviation()).collect();
    let mode = plan.movements[0].mode.name(Lang::De);
    for head in [
        format!("title = \"{}\"", plan.title),
        format!("subtitle = \"{mode}, {}\"", choir.join(" ")),
        format!("composer = \"fux-kontrapunkt, Seed {}\"", plan.seed),
    ] {
        assert!(ly.contains(&head), "{head}\n{ly}");
    }
    for movement in &plan.movements {
        assert!(ly.contains(&format!("\\bold \"{}\"", movement.movement.title())));
        for section in &movement.sections {
            let piece = format!("piece = \"{}\"", section.title);
            assert!(ly.contains(&piece), "{piece}");
        }
    }
    // Every part in its file, with the fields of its mass and its part.
    let mass = MassField::of(plan).to_string();
    for ((at, part), (measures, _)) in plan.parts().zip(&parts) {
        let file = std::fs::read_to_string(part_path(base, plan, at)).unwrap();
        let read = from_musicxml(&file).unwrap();
        assert_eq!(read.field(FIELD_MASS), Some(mass.as_str()));
        assert!(
            read.field(FIELD_LYRICS)
                .is_some_and(|lyrics| !lyrics.is_empty())
        );
        assert_eq!(read.field("fux-text"), Some(part.text.as_str()));
        let field: PartField = read.field(FIELD_PART).unwrap().parse().unwrap();
        assert_eq!(
            (field.bars, field.ending, field.start, field.continued),
            (1..=*measures, part.ending, part.start, part.continued)
        );
    }
    // On stderr one line per part as it is set.
    let told = stderr(output);
    let told: Vec<&str> = told.lines().collect();
    assert_eq!(told.len(), n, "{told:?}");
    let parted = plan.parts().zip(&headings).zip(&told).zip(&parts);
    for ((((at, _), (heading, _)), line), found) in parted {
        let set = line
            .strip_prefix(&format!("{heading}: "))
            .unwrap_or_else(|| panic!("{line}"));
        assert_eq!(measures_and_cost(set), *found);
        assert_eq!(
            set.contains(" – noch etwa "),
            at != plan.parts().last().unwrap().0,
            "{line}"
        );
    }
    assert_movements(base, plan, &parts);
    parts
}

/// `fux mass` sets the mass part by part and writes it: the plan and a line per part with its
/// summary (`--details`), the cadence plan, the totals, the book with a part per movement and a
/// score per section, MusicXML and MIDI per movement, the report as text and the file of every
/// part. By default the bar lines go through the staves, in the book and in MusicXML, and every
/// voice shows its rests.
#[test]
fn a_mass_is_written() {
    let plan = small_plan();
    let written = small_mass();
    assert_written(written, &plan, true);
    let xml =
        std::fs::read_to_string(movement_path(&written.base, Movement::Kyrie, "musicxml")).unwrap();
    assert!(xml.contains("<group-barline>yes</group-barline>"));
    assert!(!xml.contains("print-object=\"no\""));
    let ly = std::fs::read_to_string(with_extension(&written.base, "ly")).unwrap();
    assert!(!ly.contains("measureBarType"));
    // The Christe ends on the fifth as asked, Kyrie II begins together.
    let christe = plan.find("christe").unwrap()[0];
    assert_eq!(plan.part(christe).ending, Step::A);
    let kyrie2 = plan.find("kyrie2").unwrap()[0];
    assert_eq!(plan.part(kyrie2).opening, Opening::Tutti);
}

/// `text` without the seconds of the searches, which change from run to run, as
/// tools/compare_outputs.sh leaves them out: „9,9 s“ after the cost of a part, the time the mass
/// still takes after it („noch etwa 6:14“) and the search time of the totals („Suchzeit 0:12,“)
/// become „T s“, „noch etwa T“ and „Suchzeit T,“.
fn without_seconds(text: &str) -> String {
    let decimal = |word: &str| {
        word.split_once(',').is_some_and(|(whole, tenth)| {
            !whole.is_empty()
                && whole.bytes().all(|b| b.is_ascii_digit())
                && tenth.len() == 1
                && tenth.bytes().all(|b| b.is_ascii_digit())
        })
    };
    let time = |word: &str| {
        let word = word.trim_end_matches(',');
        word.split_once(':').is_some_and(|(minutes, seconds)| {
            !minutes.is_empty()
                && minutes.bytes().all(|b| b.is_ascii_digit())
                && seconds.len() == 2
                && seconds.bytes().all(|b| b.is_ascii_digit())
        })
    };
    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            let words: Vec<&str> = line.split(' ').collect();
            let masked: Vec<String> = (0..words.len())
                .map(|i| {
                    let (word, next) = (words[i], words.get(i + 1).copied());
                    let before = i.checked_sub(1).map(|k| words[k]);
                    if decimal(word) && next.is_some_and(|next| next.starts_with('s')) {
                        "T".to_string()
                    } else if time(word) && matches!(before, Some("etwa" | "Suchzeit")) {
                        format!("T{}", if word.ends_with(',') { "," } else { "" })
                    } else {
                        word.to_string()
                    }
                })
                .collect();
            masked.join(" ")
        })
        .collect();
    lines.join("\n")
}

/// The parts of a mass are the same however many are set at once (K6, D13): set one after the
/// other (`--jobs 1`), the small mass writes every file as with its three parts set at once
/// (`--jobs 3`, also on a machine whose default is one part at a time), byte for byte, the files
/// of its parts too, and prints the same report, and on stderr the same lines in the order of the
/// mass, but for the seconds of the searches and for the plan's estimate, which counts the parts
/// set at once.
#[test]
fn parts_at_once_write_what_parts_one_after_the_other_write() {
    let plan = small_plan();
    let set = |folder: &str, jobs: &str| {
        let options = [&SMALL[..], &["--seed", "1", "--details", "--jobs", jobs]].concat();
        let written = set_mass(folder, &options);
        let output = &written.output;
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        written
    };
    let at_once = set("messe-zugleich", "3");
    let alone = set("messe-einzeln", "1");
    let files = |base: &Path| -> Vec<(String, Vec<u8>)> {
        let dir = base.parent().unwrap();
        let mut files = Vec::new();
        for folder in [dir.to_path_buf(), parts_dir(base)] {
            for entry in std::fs::read_dir(&folder).unwrap() {
                let path = entry.unwrap().path();
                let extension = path.extension().and_then(|e| e.to_str());
                if path.is_file() && !matches!(extension, Some("pdf" | "txt")) {
                    let name = path.strip_prefix(dir).unwrap().display().to_string();
                    files.push((name, std::fs::read(&path).unwrap()));
                }
            }
        }
        files.sort();
        files
    };
    let (theirs, ours) = (files(&at_once.base), files(&alone.base));
    let names = |files: &[(String, Vec<u8>)]| -> Vec<String> {
        files.iter().map(|(name, _)| name.clone()).collect()
    };
    assert_eq!(names(&ours), names(&theirs));
    assert!(ours.len() >= 2 + plan.part_count(), "{:?}", names(&ours));
    for ((name, ours), (_, theirs)) in ours.iter().zip(&theirs) {
        assert!(ours == theirs, "{name} differs");
    }
    // The report, but for the seconds and the estimate of the plan.
    let report = |base: &Path| {
        let text = std::fs::read_to_string(with_extension(base, "txt")).unwrap();
        without_seconds(&text)
    };
    let (theirs, ours) = (report(&at_once.base), report(&alone.base));
    let estimate = |text: &str| text.lines().nth(1).unwrap_or_default().to_string();
    let one = mass_summary(&plan, 1, Lang::De);
    assert_eq!(estimate(&ours), estimate(&one));
    let at_once_summary = mass_summary(&plan, 3, Lang::De);
    assert_eq!(estimate(&theirs), estimate(&at_once_summary));
    let rest = |text: &str| -> Vec<String> {
        let lines = text.lines().map(str::to_string);
        lines
            .enumerate()
            .filter(|(i, _)| *i != 1)
            .map(|(_, l)| l)
            .collect()
    };
    assert_eq!(rest(&ours), rest(&theirs));
    assert!(ours.contains("Kosten ") && ours.contains(" T s"), "{ours}");
    // On stderr a line per part in the order of the mass, each but the last with the time left.
    let told = |written: &Written| without_seconds(&stderr(&written.output));
    assert_eq!(told(&alone), told(&at_once));
    let lines: Vec<String> = told(&alone).lines().map(str::to_string).collect();
    assert_eq!(lines.len(), plan.part_count(), "{lines:?}");
    for (i, line) in lines.iter().enumerate() {
        let number = format!("[{}/{}]", i + 1, plan.part_count());
        assert!(line.starts_with(&number), "{line}");
        assert_eq!(
            line.ends_with(" – noch etwa T"),
            i + 1 < lines.len(),
            "{line}"
        );
    }
}

/// `fux check` of the file of a part repeats the report of its search (D14): the cost `fux mass`
/// gave it, under the overlay of the mass, and the points of its imitation as its summary gave
/// them, the last ending on the part's ending, the fifth for the Christe. Kyrie II, which begins
/// together, has no plan of imitation.
#[test]
fn check_of_a_part_file_repeats_its_report() {
    let written = small_mass();
    let plan = small_plan();
    let parts = assert_written(written, &plan, true);
    let text = stdout(&written.output);
    for ((at, part), (_, cost)) in plan.parts().zip(&parts) {
        let file = part_path(&written.base, &plan, at);
        let checked = fux(&["check", "--musicxml", file.to_str().unwrap()]);
        assert_eq!(checked.status.code(), Some(0), "{checked:?}");
        let report = stdout(&checked);
        let last = report.lines().last().unwrap_or_default();
        assert!(
            last.starts_with(&format!("0 Verbot(e), Kosten {cost} aus ")),
            "{report}"
        );
        let points: Vec<&str> = report.lines().filter(|l| l.starts_with("Punkt ")).collect();
        let heading = PartLine::of(&plan, at, Lang::De).heading(Lang::De);
        let summary: Vec<&str> = text
            .lines()
            .skip_while(|line| !line.starts_with(&heading))
            .skip(1)
            .take_while(|line| line.starts_with("  "))
            .map(str::trim_start)
            .filter(|line| line.starts_with("Punkt "))
            .collect();
        assert_eq!(points, summary, "{report}");
        let ending = format!(" auf {}", degree(&plan, at.movement, part.ending));
        match part.opening {
            Opening::Imitative => assert!(
                points.last().is_some_and(|point| point.ends_with(&ending)),
                "{report}"
            ),
            Opening::Tutti => assert!(points.is_empty() && !report.contains("Imitation")),
        }
    }
}

/// `--resume` (D16) takes up the parts whose files fit the mass and sets only the others: the
/// seed and the form of the mass come from its files, the part set again is the part set before,
/// byte for byte, and so are the files of the movement and the book, whose parts taken from
/// their files are printed as they were set (the musica ficta of tied notes in Kyrie I); the
/// files of the parts taken stay.
#[test]
fn resume_sets_only_missing_parts() {
    let written = small_mass();
    let plan = small_plan();
    let base = copy_parts(&written.base, "messe-weiter");
    let christe = plan.find("christe").unwrap()[0];
    std::fs::remove_file(part_path(&base, &plan, christe)).unwrap();
    let at_base = ["-o", base.to_str().unwrap()];
    // Without a seed the plan is the one the files record.
    let planned = fux(&[&["mass", "--plan", "--resume"][..], &at_base, &SMALL].concat());
    assert!(
        stdout(&planned).starts_with(&format!(
            "{}\n",
            mass_summary(&plan, default_jobs(), Lang::De)
        )),
        "{planned:?}"
    );
    let resumed = fux(&[
        &["mass", "--resume", "-f", "ly,musicxml,midi"][..],
        &at_base,
        &SMALL,
    ]
    .concat());
    assert_eq!(resumed.status.code(), Some(0), "{resumed:?}");
    let dir = parts_dir(&base).display().to_string();
    let told = stderr(&resumed);
    let (taken, set): (Vec<&str>, Vec<&str>) = told
        .lines()
        .partition(|line| line.ends_with(&format!(", aus {dir}")));
    assert_eq!(taken.len(), 2, "{told}");
    let heading = PartLine::of(&plan, christe, Lang::De).heading(Lang::De);
    assert!(set.len() == 1 && set[0].starts_with(&heading), "{told}");
    for (at, _) in plan.parts() {
        let file = |base: &Path| std::fs::read(part_path(base, &plan, at)).unwrap();
        assert_eq!(file(&base), file(&written.base), "{}", plan.address(at));
    }
    for movement in &plan.movements {
        for kind in ["musicxml", "mid"] {
            let file = |base: &Path| std::fs::read(movement_path(base, movement.movement, kind));
            assert_eq!(
                file(&base).unwrap(),
                file(&written.base).unwrap(),
                "{} {kind}",
                movement.movement.id()
            );
        }
    }
    let ly = |base: &Path| std::fs::read_to_string(with_extension(base, "ly")).unwrap();
    assert_eq!(ly(&base), ly(&written.base));
    let text = stdout(&resumed);
    assert!(
        text.contains(&format!("\nGesamt: 3 von 3 Teilen (2 aus {dir}), ")),
        "{text}"
    );
}

/// `--resume` with a seed whose options plan another mass than the one in `NAME.teile/` sets
/// nothing and leaves the parts there as they are, in either language, since it would set the
/// other mass over them: the error names the seed recorded and the way to give a part another
/// seed (`--only`). With the seed recorded it goes on with the mass.
#[test]
fn resume_with_another_seed_sets_nothing() {
    let written = small_mass();
    let plan = small_plan();
    let base = copy_parts(&written.base, "messe-anderer-seed");
    let at_base = ["-o", base.to_str().unwrap()];
    let dir = parts_dir(&base).display().to_string();
    for (lang, message) in [
        (
            "de",
            format!(
                "Fehler: In {dir} liegen Teile der Messe mit Seed 1, die --resume mit diesen \
                 Angaben (Seed 2) durch Teile einer anderen Messe ersetzen würde. "
            ),
        ),
        (
            "en",
            format!(
                "Error: {dir} holds parts of the mass with seed 1, which --resume with these \
                 options (seed 2) would replace with parts of another mass. "
            ),
        ),
    ] {
        let refused = fux(&[
            &[
                "--lang", lang, "mass", "--resume", "--seed", "2", "-f", "musicxml",
            ][..],
            &at_base,
            &SMALL,
        ]
        .concat());
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        let told = stderr(&refused);
        assert!(
            told.starts_with(&message) && told.contains(" --only "),
            "{told}"
        );
        assert!(stdout(&refused).is_empty(), "{refused:?}");
    }
    for (at, _) in plan.parts() {
        let file = |base: &Path| std::fs::read(part_path(base, &plan, at)).unwrap();
        assert_eq!(file(&base), file(&written.base), "{}", plan.address(at));
    }
    assert!(!with_extension(&base, "txt").exists());
    // The seed recorded goes on with the mass: every part comes from its file.
    let resumed = fux(&[
        &["mass", "--resume", "--seed", "1", "-f", "musicxml"][..],
        &at_base,
        &SMALL,
    ]
    .concat());
    assert_eq!(resumed.status.code(), Some(0), "{resumed:?}");
    assert!(
        stdout(&resumed).contains(&format!("\nGesamt: 3 von 3 Teilen (3 aus {dir}), ")),
        "{resumed:?}"
    );
}

/// `--resume` with other options than the mass was set with, here without `--imitation
/// kyrie2=aus`, as when the notation of a mass is written anew (`--mensurstriche`): where every
/// part of a movement is in its file, it sets nothing and leaves the files as they are, in either
/// language, since it would set the parts that the options plan otherwise again over their files;
/// the error names them with what differs, and the ways on. The remedy for a part that failed
/// still sets the parts after it again: without the file of the Christe, ending it on another
/// degree sets Kyrie II again too, whose cadences go from the Christe's ending.
#[test]
fn resume_with_other_options_replaces_nothing() {
    let written = small_mass();
    let plan = small_plan();
    let base = copy_parts(&written.base, "messe-andere-angaben");
    let at_base = ["-o", base.to_str().unwrap()];
    let dir = parts_dir(&base).display().to_string();
    let other = &SMALL[..8];
    for (lang, message, part) in [
        (
            "de",
            format!(
                "Fehler: In {dir} liegen Teile dieser Messe, deren Dateien nicht zu diesen \
                 Angaben passen: "
            ),
            "kyrie/3 (Imitation).",
        ),
        (
            "en",
            format!("Error: {dir} holds parts of this mass whose files do not fit these options: "),
            "kyrie/3 (imitation).",
        ),
    ] {
        let refused = fux(&[
            &[
                "--lang",
                lang,
                "mass",
                "--resume",
                "--mensurstriche",
                "-f",
                "musicxml",
            ][..],
            &at_base,
            other,
        ]
        .concat());
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        let told = stderr(&refused);
        assert!(
            told.starts_with(&format!("{message}{part} ")) && told.contains(" --only kyrie/3 "),
            "{told}"
        );
        assert!(stdout(&refused).is_empty(), "{refused:?}");
    }
    let file = |base: &Path, at| std::fs::read(part_path(base, &plan, at)).unwrap();
    for (at, _) in plan.parts() {
        assert_eq!(
            file(&base, at),
            file(&written.base, at),
            "{}",
            plan.address(at)
        );
    }
    assert!(!with_extension(&base, "txt").exists());
    // The remedy for a Christe that failed: another ending.
    let christe = plan.find("christe").unwrap()[0];
    std::fs::remove_file(part_path(&base, &plan, christe)).unwrap();
    let mut remedy = SMALL.to_vec();
    remedy[7] = "christe=G";
    let resumed = fux(&[
        &["mass", "--resume", "-f", "musicxml"][..],
        &at_base,
        &remedy,
    ]
    .concat());
    assert_eq!(resumed.status.code(), Some(0), "{resumed:?}");
    let told = stderr(&resumed);
    let taken: Vec<bool> = told
        .lines()
        .map(|line| line.ends_with(&format!(", aus {dir}")))
        .collect();
    assert_eq!(taken, [true, false, false], "{told}");
    let first = plan.find("kyrie/1").unwrap()[0];
    assert_eq!(file(&base, first), file(&written.base, first));
}

/// Writes the file of the part `at` of `plan` in the mass at `base` with `from` in its text
/// replaced by `to`, such as an item of its field.
fn edit_part(base: &Path, plan: &MassPlan, at: PartRef, from: &str, to: &str) {
    let path = part_path(base, plan, at);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "{}", path.display());
    std::fs::write(&path, text.replace(from, to)).unwrap();
}

/// The files of the parts and of the movements record the version of the generator that set
/// each part (`generator=` of `fux-part`). A part of another version is set again like a part
/// whose plan changed: where every part of its movement is there, `--resume` sets nothing and
/// leaves the files as they are, in either language, naming the part with the version as what
/// differs and saying that no option makes it fit; where its movement lacks a part, `--resume`
/// sets it again, byte for byte as this version set it. A file without the item, as every part
/// set before the version was kept, was set by version 1, which `--resume` takes up as long as
/// this program is version 1.
#[test]
fn resume_knows_the_version_of_the_generator() {
    let written = small_mass();
    let plan = small_plan();
    let item = format!(";generator={GENERATOR}");
    let field = |xml: &str, name: &str| {
        let read = from_musicxml(xml).unwrap();
        read.field(name).unwrap().to_string()
    };
    for (at, _) in plan.parts() {
        let xml = std::fs::read_to_string(part_path(&written.base, &plan, at)).unwrap();
        let part = field(&xml, FIELD_PART);
        assert!(part.ends_with(&item), "{part}");
    }
    let movement = movement_path(&written.base, Movement::Kyrie, "musicxml");
    let xml = std::fs::read_to_string(movement).unwrap();
    for n in 1..=plan.part_count() {
        let part = field(&xml, &format!("{FIELD_PART}.{n}"));
        assert!(part.ends_with(&item), "{part}");
    }
    // Another version set the Christe.
    let base = copy_parts(&written.base, "messe-andere-version");
    let at_base = ["-o", base.to_str().unwrap()];
    let dir = parts_dir(&base).display().to_string();
    let christe = plan.find("christe").unwrap()[0];
    let other = format!(";generator={}", GENERATOR + 1);
    edit_part(&base, &plan, christe, &item, &other);
    let edited = std::fs::read(part_path(&base, &plan, christe)).unwrap();
    for (lang, message) in [
        (
            "de",
            format!(
                "Fehler: In {dir} liegen Teile dieser Messe, deren Dateien nicht zu diesen \
                 Angaben passen: kyrie/2 (Generatorversion). --resume würde sie neu setzen und \
                 ihre Dateien ersetzen. Sie stammen von einer anderen Version des Generators, die \
                 aus demselben Plan andere Noten setzen kann, und passen darum mit keinen \
                 Angaben. --only kyrie/2 setzt sie mit diesen Angaben neu; "
            ),
        ),
        (
            "en",
            format!(
                "Error: {dir} holds parts of this mass whose files do not fit these options: \
                 kyrie/2 (generator version). --resume would set them again and replace their \
                 files. They come from another version of the generator, which may set other \
                 notes from the same plan, and so fit with no options. --only kyrie/2 sets them \
                 again with these options; "
            ),
        ),
    ] {
        let refused = fux(&[
            &["--lang", lang, "mass", "--resume", "-f", "musicxml"][..],
            &at_base,
            &SMALL,
        ]
        .concat());
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        let told = stderr(&refused);
        assert!(told.starts_with(&message), "{told}");
        assert!(stdout(&refused).is_empty(), "{refused:?}");
    }
    assert_eq!(
        std::fs::read(part_path(&base, &plan, christe)).unwrap(),
        edited
    );
    assert!(!with_extension(&base, "txt").exists());
    // Without the file of Kyrie I the Kyrie lacks a part: the Christe of the other version is set
    // again with it, as this version sets it, and Kyrie II comes from its file.
    let first = plan.find("kyrie/1").unwrap()[0];
    std::fs::remove_file(part_path(&base, &plan, first)).unwrap();
    let resumed = fux(&[
        &["mass", "--resume", "-f", "musicxml"][..],
        &at_base,
        &SMALL,
    ]
    .concat());
    assert_eq!(resumed.status.code(), Some(0), "{resumed:?}");
    let told = stderr(&resumed);
    let (taken, set): (Vec<&str>, Vec<&str>) = told
        .lines()
        .partition(|line| line.ends_with(&format!(", aus {dir}")));
    let last = plan.find("kyrie2").unwrap()[0];
    let heading = PartLine::of(&plan, last, Lang::De).heading(Lang::De);
    assert!(
        taken.len() == 1 && taken[0].starts_with(&heading) && set.len() == 2,
        "{told}"
    );
    let file = |base: &Path, at| std::fs::read(part_path(base, &plan, at)).unwrap();
    for (at, _) in plan.parts() {
        assert_eq!(
            file(&base, at),
            file(&written.base, at),
            "{}",
            plan.address(at)
        );
    }
    // The files of a mass set before the version was kept.
    let older = copy_parts(&written.base, "messe-ohne-version");
    for (at, _) in plan.parts() {
        edit_part(&older, &plan, at, &item, "");
    }
    let at_older = ["-o", older.to_str().unwrap()];
    let resumed = fux(&[
        &["mass", "--resume", "-f", "musicxml"][..],
        &at_older,
        &SMALL,
    ]
    .concat());
    let dir = parts_dir(&older).display().to_string();
    if GENERATOR == 1 {
        assert_eq!(resumed.status.code(), Some(0), "{resumed:?}");
        assert!(
            stdout(&resumed).contains(&format!("\nGesamt: 3 von 3 Teilen (3 aus {dir}), ")),
            "{resumed:?}"
        );
        // The file of the movement names the version of its parts.
        let xml =
            std::fs::read_to_string(movement_path(&older, Movement::Kyrie, "musicxml")).unwrap();
        assert!(field(&xml, &format!("{FIELD_PART}.1")).ends_with(&item));
    } else {
        assert_eq!(resumed.status.code(), Some(1), "{resumed:?}");
        assert!(
            stderr(&resumed).contains(" (Generatorversion)"),
            "{resumed:?}"
        );
    }
}

/// The report of `fux mass` without the lines of the files written, which name the folder.
fn reported(output: &Output) -> Vec<String> {
    stdout(output)
        .lines()
        .filter(|line| !line.starts_with("geschrieben: "))
        .map(str::to_string)
        .collect()
}

/// `fux mass --rewrite` writes the mass anew from the parts in `NAME.teile/` as they are, without
/// a search and without the options of its plan: the files of its movements and its book byte for
/// byte as `fux mass` wrote them; the report as `fux mass` printed it, its second line saying that
/// the mass was written anew, every part from its file, the summaries of `--details` as they were
/// (with the `--repeat` of the setting, the default here); `NAME.txt` with it, and nothing on
/// stderr. With another notation (`--mensurstriche`, `--tacet verbergen`) it writes what
/// `--resume` with the options of the setting writes.
#[test]
fn rewrite_writes_the_mass_from_its_parts() {
    let written = small_mass();
    let plan = small_plan();
    let base = copy_parts(&written.base, "messe-neu-geschrieben");
    let at_base = ["-o", base.to_str().unwrap()];
    let dir = parts_dir(&base).display().to_string();
    let rewritten = fux(&[
        &["mass", "--rewrite", "--details", "-f", "ly,musicxml,midi"][..],
        &at_base,
    ]
    .concat());
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    assert!(stderr(&rewritten).is_empty(), "{rewritten:?}");
    let file = |base: &Path, movement: Movement, kind: &str| {
        std::fs::read(movement_path(base, movement, kind)).unwrap()
    };
    for movement in &plan.movements {
        for kind in ["musicxml", "mid"] {
            assert_eq!(
                file(&base, movement.movement, kind),
                file(&written.base, movement.movement, kind),
                "{} {kind}",
                movement.movement.id()
            );
        }
    }
    let ly = |base: &Path| std::fs::read_to_string(with_extension(base, "ly")).unwrap();
    assert_eq!(ly(&base), ly(&written.base));
    // The report: the plan without its estimate, each part's line from its file as `--resume`
    // writes it, the totals with every part from the folder.
    let expected: Vec<String> = reported(&written.output)
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if i == 1 {
                format!("aus {dir} neu geschrieben, ohne Suche")
            } else if line.starts_with('[') {
                let (head, _) = line.rsplit_once(", ").unwrap();
                format!("{head}, aus {dir}")
            } else if let Some(rest) = line.strip_prefix("Gesamt: 3 von 3 Teilen, ") {
                let (measures, after) = rest.split_once(", Suchzeit ").unwrap();
                let (_, warnings) = after.split_once(", ").unwrap();
                format!(
                    "Gesamt: 3 von 3 Teilen (3 aus {dir}), {measures}, Suchzeit 0:00, {warnings}"
                )
            } else {
                line.clone()
            }
        })
        .collect();
    assert_eq!(reported(&rewritten), expected);
    let txt = std::fs::read_to_string(with_extension(&base, "txt")).unwrap();
    assert_eq!(txt, format!("{}\n", expected.join("\n")));
    let text = stdout(&rewritten);
    let listed: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix("geschrieben: "))
        .collect();
    let files = [
        with_extension(&base, "ly"),
        movement_path(&base, Movement::Kyrie, "musicxml"),
        movement_path(&base, Movement::Kyrie, "mid"),
        with_extension(&base, "txt"),
    ];
    let files: Vec<String> = files.iter().map(|f| f.display().to_string()).collect();
    assert_eq!(listed, files);
    // Another notation, as `--resume` with the options of the setting writes it.
    let notation = [
        "--mensurstriche",
        "--tacet",
        "verbergen",
        "-f",
        "ly,musicxml,midi",
    ];
    let rewritten = fux(&[&["mass", "--rewrite"][..], &notation, &at_base].concat());
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    let resumed = copy_parts(&written.base, "messe-neu-fortgesetzt");
    let at_resumed = ["-o", resumed.to_str().unwrap()];
    let again = fux(&[&["mass", "--resume"][..], &notation, &at_resumed, &SMALL].concat());
    assert_eq!(again.status.code(), Some(0), "{again:?}");
    assert_eq!(ly(&base), ly(&resumed));
    assert_ne!(ly(&base), ly(&written.base));
    for kind in ["musicxml", "mid"] {
        assert_eq!(
            file(&base, Movement::Kyrie, kind),
            file(&resumed, Movement::Kyrie, kind),
            "{kind}"
        );
    }
}

/// After a change of the profile (`--rule`, a level that makes a rule the Christe breaks a
/// prohibition), `--resume` refuses every part of the small mass, but `fux mass --rewrite` writes
/// them as they are: the file of the movement keeps every part's field as its file has it, the
/// profile it was set under included; the costs in the report are those of the profile of today,
/// as `fux check` of the file of the movement under the same `--rule` gives them; and it warns
/// that another profile set every part and that parts break a prohibition now. It ends with 0.
/// The summaries of `--details` name the repetition of the check (`--repeat keine`), which the
/// files do not keep for their parts, beside the warning of another profile.
#[test]
fn rewrite_after_a_change_of_the_profile() {
    let written = small_mass();
    let plan = small_plan();
    let base = copy_parts(&written.base, "messe-neues-profil");
    let at_base = ["-o", base.to_str().unwrap()];
    let christe = plan.find("christe").unwrap()[0];
    let christe_file = part_path(&base, &plan, christe);
    let checked = fux(&["check", "--musicxml", christe_file.to_str().unwrap()]);
    let rule = stdout(&checked)
        .lines()
        .find_map(|line| {
            let (_, rest) = line.rsplit_once(" [")?;
            Some(rest.split_once(", ")?.0.to_string())
        })
        .expect("the Christe breaks a preference");
    let level = format!("{rule}=V");
    let refused = fux(&[
        &["mass", "--resume", "--rule", &level, "-f", "musicxml"][..],
        &at_base,
        &SMALL,
    ]
    .concat());
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(
        stderr(&refused)
            .contains("kyrie/1 (Regelprofil), kyrie/2 (Regelprofil), kyrie/3 (Regelprofil). "),
        "{refused:?}"
    );
    let rewritten = fux(&[
        &["mass", "--rewrite", "--rule", &level, "-f", "musicxml,midi"][..],
        &at_base,
    ]
    .concat());
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    let text = stdout(&rewritten);
    assert!(
        text.contains(
            "\nWarnung: Ein anderes Regelprofil oder eine andere Version des Generators setzte \
             alle Teile: Die Kosten hier und in fux check der Satzdateien sind die einer \
             Prüfung mit dem heutigen Profil und können von denen beim Setzen abweichen\n"
        ),
        "{text}"
    );
    let forbidden = text
        .lines()
        .find_map(|line| {
            line.strip_prefix("Warnung: Unter dem heutigen Profil brechen Teile Verbote: ")
        })
        .unwrap_or_else(|| panic!("{text}"));
    assert!(forbidden.contains("kyrie/2 ("), "{forbidden}");
    // The file of the movement keeps the fields of the parts' files.
    let movement = movement_path(&base, Movement::Kyrie, "musicxml");
    let read = from_musicxml(&std::fs::read_to_string(&movement).unwrap()).unwrap();
    let mut first = 1;
    for (n, (at, _)) in plan.parts().enumerate() {
        let xml = std::fs::read_to_string(part_path(&base, &plan, at)).unwrap();
        let kept: PartField = from_musicxml(&xml)
            .unwrap()
            .field(FIELD_PART)
            .unwrap()
            .parse()
            .unwrap();
        let field: PartField = read
            .field(&format!("{FIELD_PART}.{}", n + 1))
            .unwrap()
            .parse()
            .unwrap();
        let last = first + kept.bars.end() - 1;
        assert_eq!(
            field,
            PartField {
                bars: first..=last,
                ..kept
            }
        );
        first = last + 1;
    }
    // The costs of the report are those of the check of the file of the movement.
    let costs: Vec<u32> = text
        .lines()
        .filter(|line| line.starts_with('['))
        .map(|line| {
            let (head, _) = line.rsplit_once(", aus ").unwrap();
            let (_, cost) = head.rsplit_once("Kosten ").unwrap();
            cost.parse().unwrap()
        })
        .collect();
    let checked = fux(&[
        "check",
        "--rule",
        &level,
        "--musicxml",
        movement.to_str().unwrap(),
    ]);
    assert_eq!(checked.status.code(), Some(2), "{checked:?}");
    let found: Vec<u32> = stdout(&checked)
        .lines()
        .filter_map(|line| line.strip_prefix("  ")?.split_once(" Verbot(e), Kosten "))
        .filter_map(|(_, rest)| rest.split(' ').next()?.parse().ok())
        .collect();
    assert_eq!(found, costs, "{checked:?}");
    // The repetition of the check in the summaries.
    let rewritten = fux(&[
        &["mass", "--rewrite", "--repeat", "keine", "--details"][..],
        &["-f", "musicxml"],
        &at_base,
    ]
    .concat());
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    let text = stdout(&rewritten);
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("  Text: "))
        .collect();
    assert!(
        lines.len() == plan.part_count()
            && lines
                .iter()
                .all(|line| line.ends_with(", Wiederholung keine"))
            && text.contains(
                "\nWarnung: Ein anderes Regelprofil oder eine andere Version des Generators \
                 setzte alle Teile: "
            ),
        "{text}"
    );
}

/// The run of `fux mass` that the German error `told` of `fux mass --rewrite` names as its way on,
/// as one takes it up with `setting`, the options the mass was set with, each with its value: the
/// options of the run, then those of `setting` that the run does not name, without `--seed` where
/// the way on says so, and the items it names at the end of `--endings`.
fn way_on(told: &str, setting: &[&str]) -> Vec<String> {
    let (_, rest) = told.split_once(" fux mass ").expect("a way on");
    let (command, after) = rest
        .split_once(" mit den übrigen Angaben")
        .expect("the options of the setting");
    let (options, _) = after.split_once(". ").unwrap_or((after, ""));
    let named: Vec<&str> = command.split(' ').collect();
    let items = told
        .split_once(" Ans Ende von --endings gehören dabei ")
        .and_then(|(_, rest)| rest.split_once(':'))
        .map(|(items, _)| items);
    let mut args: Vec<String> = std::iter::once("mass")
        .chain(named.iter().copied())
        .map(str::to_string)
        .collect();
    let mut ended = false;
    for pair in setting.chunks(2) {
        let (option, value) = (pair[0], pair[1]);
        if named.contains(&option) || (option == "--seed" && options.contains("(ohne --seed)")) {
            continue;
        }
        let value = match (option, items) {
            ("--endings", Some(items)) => {
                ended = true;
                format!("{value};{items}")
            }
            _ => value.to_string(),
        };
        args.extend([option.to_string(), value]);
    }
    if let (false, Some(items)) = (ended, items) {
        args.extend(["--endings".to_string(), items.to_string()]);
    }
    args
}

/// `fux mass` with the arguments `run` (as [`way_on`] gives them) and `more`.
fn fux_run(run: &[String], more: &[&str]) -> Output {
    let run: Vec<&str> = run.iter().map(String::as_str).collect();
    fux(&[&run[..], more].concat())
}

/// `fux mass --rewrite` refuses the options of the plan and the seed, which cannot change a mass
/// set already, in either language; it fails where the mass lacks a part, saying which and how to
/// go on, in either language, and where the folder keeps no mass; it writes nothing then. Where
/// this program and profile set the parts kept, `--resume` with the movements and the other
/// options of the setting sets the part missing; where another version of the generator set
/// them, `--only` with the part and the endings that the files keep around it, which sets it as
/// it was set, leaves the files of the others as they are and ends incomplete, so that
/// `--rewrite` then writes the mass.
#[test]
fn rewrite_needs_the_parts_and_no_plan() {
    let written = small_mass();
    let plan = small_plan();
    let base = copy_parts(&written.base, "messe-neu-unvollstaendig");
    let at_base = ["-o", base.to_str().unwrap()];
    let dir = parts_dir(&base).display().to_string();
    for (lang, message) in [
        (
            "de",
            format!(
                "Fehler: --rewrite schreibt die Messe aus ihren Teilen in {dir} neu, wie sie \
                 gesetzt wurde: Form, Plan und Seeds kommen aus den Dateien, und nichts wird \
                 gesetzt. --rewrite nimmt --voices, --imitation, --seed nicht an; Teile mit \
                 anderen Angaben setzt fux mass --only neu, eine neue Messe fux mass ohne \
                 --rewrite.\n"
            ),
        ),
        (
            "en",
            format!(
                "Error: --rewrite writes the mass anew from its parts in {dir} as it was set: \
                 form, plan and seeds come from the files, and nothing is set. Leave out \
                 --voices, --imitation, --seed; fux mass --only sets parts again with other \
                 options, fux mass without --rewrite sets a new mass.\n"
            ),
        ),
    ] {
        let refused = fux(&[
            &[
                "--lang",
                lang,
                "mass",
                "--rewrite",
                "--voices",
                "3",
                "--imitation",
                "kyrie2=aus",
                "--seed",
                "1",
            ][..],
            &at_base,
        ]
        .concat());
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        assert_eq!(stderr(&refused), message);
    }
    // The Christe missing.
    let christe = plan.find("christe").unwrap()[0];
    std::fs::remove_file(part_path(&base, &plan, christe)).unwrap();
    let chosen = [
        "Mit --movements schreibt --rewrite nur Sätze, deren Teile alle vorliegen, und lässt die \
         Dateien der übrigen unverändert.",
        "With --movements, --rewrite writes only movements whose parts are all there and leaves \
         the files of the others unchanged.",
    ];
    let missing = |remedies: [&str; 2]| {
        for (lang, message) in [
            (
                "de",
                format!(
                    "Fehler: --rewrite kann die Messe mit Seed 1 in {dir} nicht neu schreiben. \
                     Teil kyrie/2 fehlt. {} {}\n",
                    remedies[0], chosen[0]
                ),
            ),
            (
                "en",
                format!(
                    "Error: --rewrite cannot write the mass with seed 1 in {dir} anew. Part \
                     kyrie/2 is missing. {} {}\n",
                    remedies[1], chosen[1]
                ),
            ),
        ] {
            let refused = fux(&[&["--lang", lang, "mass", "--rewrite"][..], &at_base].concat());
            assert_eq!(refused.status.code(), Some(1), "{refused:?}");
            assert_eq!(stderr(&refused), message);
            assert!(stdout(&refused).is_empty(), "{refused:?}");
        }
        assert!(!with_extension(&base, "txt").exists());
        assert!(!movement_path(&base, Movement::Kyrie, "musicxml").exists());
    };
    missing([
        "Fehlende Teile setzt fux mass --resume --movements kyrie mit den übrigen Angaben des \
         Setzens nach.",
        "fux mass --resume --movements kyrie with the other options of the setting sets missing \
         parts.",
    ]);
    // Another version of the generator set the parts kept: --resume would set them again, --only
    // sets the Christe as this version sets it and leaves their files as they are.
    let item = format!(";generator={GENERATOR}");
    let other = format!(";generator={}", GENERATOR + 1);
    let kept: Vec<PartRef> = plan
        .parts()
        .map(|(at, _)| at)
        .filter(|&at| at != christe)
        .collect();
    for &at in &kept {
        edit_part(&base, &plan, at, &item, &other);
    }
    let first = plan.find("kyrie/1").unwrap()[0];
    let ending = degree(&plan, 0, plan.part(first).ending);
    missing([
        &format!(
            "fux mass --only kyrie/2 --movements kyrie mit den übrigen Angaben des Setzens (ohne \
             --seed) setzt ihn nach. Ans Ende von --endings gehören dabei \
             kyrie/1={ending};kyrie/2=A: die Schlussstufen, die die Dateien halten. Nach einer \
             Änderung von Programm oder Regelprofil nähme --resume die übrigen Teile nicht mehr \
             aus ihren Dateien. Teile, die nicht passen, lässt --only, wie sie sind; dann endet es \
             unvollständig (Rückgabewert 3), und statt seines Hinweises auf --resume schreibt \
             --rewrite danach die Messe."
        ),
        &format!(
            "fux mass --only kyrie/2 --movements kyrie with the other options of the setting \
             (without --seed) sets it. At the end of --endings, add kyrie/1={ending};kyrie/2=A: \
             the endings the files keep. After a change of program or rule profile, --resume would \
             no longer take the other parts from their files. --only leaves the parts that do not \
             fit as they are; it then ends incomplete (exit code 3), and instead of its hint at \
             --resume, --rewrite then writes the mass."
        ),
    ]);
    let file = |base: &Path, at| std::fs::read(part_path(base, &plan, at)).unwrap();
    let edited: Vec<Vec<u8>> = kept.iter().map(|&at| file(&base, at)).collect();
    let refused = fux(&[&["mass", "--rewrite"][..], &at_base].concat());
    let run = way_on(&stderr(&refused), &SMALL);
    let again = fux_run(&run, &[&["-f", "musicxml"][..], &at_base].concat());
    assert_eq!(again.status.code(), Some(3), "{run:?} {again:?}");
    assert_eq!(file(&base, christe), file(&written.base, christe));
    for (&at, edited) in kept.iter().zip(&edited) {
        assert_eq!(&file(&base, at), edited, "{}", plan.address(at));
    }
    let rewritten = fux(&[&["mass", "--rewrite", "-f", "musicxml"][..], &at_base].concat());
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    assert!(
        stdout(&rewritten).contains(
            "\nWarnung: Ein anderes Regelprofil oder eine andere Version des Generators setzte \
             die Teile kyrie/1, kyrie/3: "
        ),
        "{rewritten:?}"
    );
    // No mass at all.
    let none = tmp("messe-neu-ohne").join("messe");
    let dir = parts_dir(&none).display().to_string();
    for (lang, message) in [
        (
            "de",
            format!("Fehler: In {dir} liegen keine Teile einer Messe; --rewrite schreibt "),
        ),
        (
            "en",
            format!("Error: {dir} holds no parts of a mass; --rewrite writes "),
        ),
    ] {
        let at_none = ["-o", none.to_str().unwrap()];
        let refused = fux(&[&["--lang", lang, "mass", "--rewrite"][..], &at_none].concat());
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        assert!(stderr(&refused).starts_with(&message), "{refused:?}");
    }
}

/// A part set after another ending of the part before it than that part's file keeps, here Kyrie
/// II as if set after a Christe ending on D: `fux mass --rewrite` fails, writing nothing, in either
/// language, and names `--only` with the part, the options the part before it was set with and
/// the ending that its file keeps, since `--resume` refuses whichever of the two endings its
/// options give (one of the two parts differs from the plan, and the movement is whole). `--only`
/// so sets Kyrie II as it was set, byte for byte, and `--rewrite` writes the mass.
#[test]
fn rewrite_names_the_way_to_a_part_set_after_another_ending() {
    let written = small_mass();
    let plan = small_plan();
    let base = copy_parts(&written.base, "messe-neu-andere-schlussstufe");
    let at_base = ["-o", base.to_str().unwrap()];
    let dir = parts_dir(&base).display().to_string();
    let last = plan.find("kyrie2").unwrap()[0];
    edit_part(&base, &plan, last, ";before=A;", ";before=D;");
    for (lang, message) in [
        (
            "de",
            format!(
                "Fehler: --rewrite kann die Messe mit Seed 1 in {dir} nicht neu schreiben. Teil \
                 kyrie/3 wurde nach einer anderen Schlussstufe des Teils davor gesetzt, als dessen \
                 Datei hält. Den Teil setzt fux mass --only kyrie/3 --movements kyrie mit den \
                 übrigen Angaben neu, mit denen der Teil davor gesetzt wurde (ohne --seed). Ans \
                 Ende von --endings gehören dabei kyrie/2=A: die Schlussstufe, die die Dateien \
                 halten.\n"
            ),
        ),
        (
            "en",
            format!(
                "Error: --rewrite cannot write the mass with seed 1 in {dir} anew. Part kyrie/3 \
                 was set after another ending of the part before it than that part's file keeps. \
                 fux mass --only kyrie/3 --movements kyrie with the other options the part before \
                 it was set with (without --seed) sets it again. At the end of --endings, add \
                 kyrie/2=A: the ending the files keep.\n"
            ),
        ),
    ] {
        let refused = fux(&[&["--lang", lang, "mass", "--rewrite"][..], &at_base].concat());
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        assert_eq!(stderr(&refused), message);
    }
    for christe in ["christe=A", "christe=D"] {
        let mut options = SMALL.to_vec();
        options[7] = christe;
        let refused = fux(&[
            &["mass", "--resume", "-f", "musicxml"][..],
            &at_base,
            &options,
        ]
        .concat());
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        assert!(
            stderr(&refused).contains(" (Schlussstufe). "),
            "{refused:?}"
        );
    }
    assert!(!with_extension(&base, "txt").exists());
    let refused = fux(&[&["mass", "--rewrite"][..], &at_base].concat());
    let run = way_on(&stderr(&refused), &SMALL);
    let again = fux_run(&run, &[&["-f", "musicxml"][..], &at_base].concat());
    assert_eq!(again.status.code(), Some(0), "{run:?} {again:?}");
    let file = |base: &Path| std::fs::read(part_path(base, &plan, last)).unwrap();
    assert_eq!(file(&base), file(&written.base));
    let rewritten = fux(&[&["mass", "--rewrite", "-f", "musicxml"][..], &at_base].concat());
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    let kyrie = |base: &Path| std::fs::read(movement_path(base, Movement::Kyrie, "musicxml"));
    assert_eq!(kyrie(&base).unwrap(), kyrie(&written.base).unwrap());
}

/// `--only` plans the parts it sets after the endings that the plan of today draws; so a way on of
/// `fux mass --rewrite` that names it also names the endings that the files around its part keep,
/// for the end of `--endings`. Here another version of the generator (the item `generator=` of the
/// files) set Kyrie I to end on another degree than this plan draws, and the Christe is missing.
/// `--only kyrie/2` without the items sets the Christe after the ending of the plan, so that the
/// next `--rewrite` fails on it as a part set after another ending of Kyrie I; with the items that
/// its way on names it sets the Christe after the ending that the file of Kyrie I keeps, and
/// `--rewrite` writes the mass.
#[test]
fn rewrite_names_the_endings_the_files_keep() {
    let written = small_mass();
    let plan = small_plan();
    let base = copy_parts(&written.base, "messe-neu-schlussstufen");
    let at_base = ["-o", base.to_str().unwrap()];
    let first = plan.find("kyrie/1").unwrap()[0];
    let christe = plan.find("christe").unwrap()[0];
    let last = plan.find("kyrie2").unwrap()[0];
    let planned = plan.part(first).ending;
    let other = [Step::D, Step::A, Step::G]
        .into_iter()
        .find(|&step| step != planned)
        .unwrap();
    let ends = |step: Step| format!(";end={};before=;", step.letter());
    edit_part(&base, &plan, first, &ends(planned), &ends(other));
    let item = format!(";generator={GENERATOR}");
    let version = format!(";generator={}", GENERATOR + 1);
    for at in [first, last] {
        edit_part(&base, &plan, at, &item, &version);
    }
    std::fs::remove_file(part_path(&base, &plan, christe)).unwrap();
    let items = format!("kyrie/1={};kyrie/2=A", degree(&plan, 0, other));
    let rewrite = || fux(&[&["mass", "--rewrite", "-f", "musicxml"][..], &at_base].concat());
    let refused = rewrite();
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    let told = stderr(&refused);
    assert!(
        told.contains(&format!(
            " Teil kyrie/2 fehlt. fux mass --only kyrie/2 --movements kyrie mit den übrigen \
             Angaben des Setzens (ohne --seed) setzt ihn nach. Ans Ende von --endings gehören \
             dabei {items}: "
        )),
        "{told}"
    );
    // Without the items.
    let unpinned = fux(&[
        &["mass", "--only", "kyrie/2", "-f", "musicxml"][..],
        &at_base,
        &SMALL,
    ]
    .concat());
    assert_eq!(unpinned.status.code(), Some(3), "{unpinned:?}");
    let refused = rewrite();
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    let told = stderr(&refused);
    assert!(
        told.contains(&format!(
            " Teil kyrie/2 wurde nach einer anderen Schlussstufe des Teils davor gesetzt, als \
             dessen Datei hält. Den Teil setzt fux mass --only kyrie/2 --movements kyrie mit den \
             übrigen Angaben neu, mit denen der Teil davor gesetzt wurde (ohne --seed). Ans Ende \
             von --endings gehören dabei {items}: "
        )),
        "{told}"
    );
    // With them.
    let run = way_on(&told, &SMALL);
    let endings = format!("christe=A;{items}");
    assert!(run.contains(&endings), "{run:?}");
    let pinned = fux_run(&run, &[&["-f", "musicxml"][..], &at_base].concat());
    assert_eq!(pinned.status.code(), Some(3), "{run:?} {pinned:?}");
    let rewritten = rewrite();
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
}

/// `fux mass --rewrite` removes no file of a movement that it does not write. Where `NAME.teile/`
/// keeps no part of a movement whose files are there, here the Agnus Dei of the mass with the
/// head motif, it writes the mass without it and warns, naming its files, which stay as they are,
/// also with `--movements` (all six named are no choice; a choice with the Agnus Dei fails on
/// it). Where the folder keeps some of its parts, `--rewrite` fails, writing nothing; with the
/// Kyrie in `--movements` it writes the Kyrie, without a warning, and the files of the Agnus Dei
/// stay as they are. Where a part of the Kyrie is missing too, the error names those files, and
/// its way on keeps them, with `-f ly`, whether this program or another version set the Kyrie:
/// the run it names and the `--rewrite` after it leave them byte for byte. The error also says
/// that the files of the Kyrie, which that `--rewrite` wrote, may keep the last copy of the part,
/// and the run leaves them as they are too.
#[test]
fn rewrite_keeps_the_files_of_the_movements_it_does_not_write() {
    let written = motif_mass();
    assert_eq!(
        written.output.status.code(),
        Some(0),
        "{:?}",
        written.output
    );
    let base = copy_parts(&written.base, "messe-neu-ohne-agnus");
    let at_base = ["-o", base.to_str().unwrap()];
    let dir = parts_dir(&base).display().to_string();
    let kinds = ["musicxml", "mid"];
    for kind in kinds {
        let from = movement_path(&written.base, Movement::Agnus, kind);
        std::fs::copy(from, movement_path(&base, Movement::Agnus, kind)).unwrap();
    }
    let files = |base: &Path| -> Vec<Vec<u8>> {
        kinds
            .iter()
            .map(|kind| std::fs::read(movement_path(base, Movement::Agnus, kind)).unwrap())
            .collect()
    };
    let mut agnus: Vec<PathBuf> = std::fs::read_dir(parts_dir(&base))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("6-")
        })
        .collect();
    agnus.sort();
    assert_eq!(agnus.len(), 2, "{agnus:?}");
    let rewrite = |movements: &[&str]| {
        let at = ["mass", "--rewrite", "-f", "musicxml,midi"];
        let rewritten = fux(&[&at[..], movements, &at_base].concat());
        assert_eq!(files(&base), files(&written.base));
        rewritten
    };
    // Agnus Dei II missing.
    std::fs::remove_file(&agnus[1]).unwrap();
    let refused = rewrite(&[]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(
        stderr(&refused).contains(" Die Teile ab agnus/2 fehlen. "),
        "{refused:?}"
    );
    let kyrie = rewrite(&["--movements", "kyrie"]);
    assert_eq!(kyrie.status.code(), Some(0), "{kyrie:?}");
    assert!(stdout(&kyrie).contains(", keine Warnung\n"), "{kyrie:?}");
    // No part of the Agnus Dei.
    std::fs::remove_file(&agnus[0]).unwrap();
    let named: Vec<String> = kinds
        .iter()
        .map(|kind| {
            movement_path(&base, Movement::Agnus, kind)
                .display()
                .to_string()
        })
        .collect();
    let warning = format!(
        "\nWarnung: In {dir} liegt kein Teil des Agnus Dei: --rewrite schreibt die Messe ohne \
         diesen Satz und lässt {} unverändert\n",
        named.join(", ")
    );
    let six = ["--movements", "agnus,kyrie,gloria,credo,sanctus,benedictus"];
    for movements in [&[][..], &["--movements", "kyrie"], &six] {
        let rewritten = rewrite(movements);
        assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
        let text = stdout(&rewritten);
        assert!(
            text.contains(", 1 Warnung\n") && text.contains(&warning),
            "{text}"
        );
        let report = std::fs::read_to_string(with_extension(&base, "txt")).unwrap();
        assert!(report.contains(&warning), "{report}");
    }
    let chosen = rewrite(&["--movements", "kyrie,agnus"]);
    assert_eq!(chosen.status.code(), Some(1), "{chosen:?}");
    assert!(
        stderr(&chosen).contains(" Vom Agnus Dei liegt kein Teil vor. "),
        "{chosen:?}"
    );
    // The Christe missing as well: the way on, as this program or another version set the Kyrie.
    let plan = plan_of(&MOTIF);
    let christe = plan.find("christe").unwrap()[0];
    let ending = degree(&plan, 0, plan.part(christe).ending);
    let kyrie_i = degree(&plan, 0, plan.part(plan.find("kyrie/1").unwrap()[0]).ending);
    let item = format!(";generator={GENERATOR}");
    let others: Vec<PartRef> = plan
        .parts()
        .map(|(at, _)| at)
        .filter(|&at| plan.movements[at.movement].movement == Movement::Kyrie && at != christe)
        .collect();
    let kept = format!(
        " In {dir} liegt kein Teil des Agnus Dei, nur {}: --rewrite lässt sie unverändert, fux \
         mass ersetzt oder löscht sie, wo es Satzdateien schreibt. Mit -f ly allein (ohne musicxml \
         und midi) schreibt und löscht fux mass keine Satzdatei, und --rewrite schreibt danach die \
         übrigen Sätze. ",
        named.join(", ")
    );
    // The files of the Kyrie, which the rewrite of the Kyrie wrote, keep the Christe.
    let kyrie_files = |base: &Path| -> Vec<Vec<u8>> {
        kinds
            .iter()
            .map(|kind| std::fs::read(movement_path(base, Movement::Kyrie, kind)).unwrap())
            .collect()
    };
    let kyrie_named: Vec<String> = kinds
        .iter()
        .map(|kind| {
            movement_path(&base, Movement::Kyrie, kind)
                .display()
                .to_string()
        })
        .collect();
    let copy = format!(
        " Teil kyrie/2 fehlt. Wie er gesetzt wurde, kann der Teil nur noch in {} stehen: Ist er neu \
         gesetzt, schreibt --rewrite diese Dateien neu; eine Kopie an anderem Ort behält ihn.",
        kyrie_named.join(", ")
    );
    for (version, way, exit) in [
        (
            GENERATOR,
            format!(
                "{copy} Fehlende Teile setzt fux mass --resume --movements kyrie -f ly mit den \
                 übrigen Angaben des Setzens nach. In "
            ),
            0,
        ),
        (
            GENERATOR + 1,
            format!(
                "{copy} fux mass --only kyrie/2 --movements kyrie -f ly mit den übrigen Angaben des \
                 Setzens (ohne --seed) setzt ihn nach. Ans Ende von --endings gehören dabei \
                 kyrie/1={kyrie_i};kyrie/2={ending}: "
            ),
            3,
        ),
    ] {
        let other = format!(";generator={version}");
        for &at in &others {
            let text = std::fs::read_to_string(part_path(&base, &plan, at)).unwrap();
            std::fs::write(part_path(&base, &plan, at), text.replace(&item, &other)).unwrap();
        }
        std::fs::remove_file(part_path(&base, &plan, christe)).unwrap();
        let refused = rewrite(&[]);
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        let told = stderr(&refused);
        assert!(told.contains(&way) && told.contains(&kept), "{told}");
        let run = way_on(&told, &MOTIF);
        assert!(run.contains(&"ly".to_string()), "{run:?}");
        let before = kyrie_files(&base);
        let again = fux_run(&run, &at_base);
        assert_eq!(again.status.code(), Some(exit), "{run:?} {again:?}");
        assert_eq!(files(&base), files(&written.base));
        assert_eq!(kyrie_files(&base), before);
        let rewritten = rewrite(&[]);
        assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
        assert!(stdout(&rewritten).contains(&warning), "{rewritten:?}");
    }
}

/// The parts of the movement `movement` of the mass of `plan`.
fn parts_of(plan: &MassPlan, movement: Movement) -> Vec<PartRef> {
    plan.parts()
        .map(|(at, _)| at)
        .filter(|at| plan.movements[at.movement].movement == movement)
        .collect()
}

/// Where `NAME.teile/` keeps no part of a movement that `--movements` chooses, `fux mass
/// --rewrite` names a run that sets it and takes the parts of the other movements from their
/// files; in a mass with the head motif, which `fux mass` sets only with the Kyrie, a run with
/// the Kyrie: here the Agnus Dei of the mass with the head motif. Where this program set the
/// Kyrie, `--resume --movements kyrie,agnus` sets the Agnus Dei as it was set, byte for byte, and
/// takes the Kyrie from its files. Where another version set it, `--resume` refuses that, and
/// the way on is `--only` with Kyrie I and the sections of the Agnus Dei, which sets them and
/// Agnus Dei I with the head motif of the new Kyrie I, leaves the other parts of the Kyrie as they
/// are and ends incomplete. Either way `--rewrite` then writes the mass.
#[test]
fn rewrite_names_a_way_to_a_movement_without_parts() {
    let written = motif_mass();
    assert_eq!(
        written.output.status.code(),
        Some(0),
        "{:?}",
        written.output
    );
    let plan = plan_of(&MOTIF);
    let file = |base: &Path, at| std::fs::read(part_path(base, &plan, at)).unwrap();
    let agnus = parts_of(&plan, Movement::Agnus);
    let kyrie = parts_of(&plan, Movement::Kyrie);
    let kyrie_i = degree(&plan, 0, plan.part(kyrie[0]).ending);
    let item = format!(";generator={GENERATOR}");
    for (version, way, exit) in [
        (
            GENERATOR,
            " Vom Agnus Dei liegt kein Teil vor. fux mass --resume --movements kyrie,agnus mit den \
             übrigen Angaben des Setzens setzt das Agnus Dei und nimmt die Teile des Kyrie aus \
             ihren Dateien, ohne Teile anderer Sätze zu ersetzen. Mit --movements "
                .to_string(),
            0,
        ),
        (
            GENERATOR + 1,
            format!(
                " Vom Agnus Dei liegt kein Teil vor. fux mass --only kyrie/1,agnus,agnus3 \
                 --movements kyrie,agnus mit den übrigen Angaben des Setzens (ohne --seed) setzt \
                 das Agnus Dei und das Kyrie I und mit ihm jeden Teil mit dem Kopfmotiv neu und \
                 ersetzt deren Dateien. Ans Ende von --endings gehören dabei kyrie/1={kyrie_i}: "
            ),
            3,
        ),
    ] {
        let base = copy_parts(&written.base, &format!("messe-neu-agnus-{version}"));
        let at_base = ["-o", base.to_str().unwrap()];
        for &at in &agnus {
            std::fs::remove_file(part_path(&base, &plan, at)).unwrap();
        }
        let other = format!(";generator={version}");
        for &at in &kyrie {
            edit_part(&base, &plan, at, &item, &other);
        }
        let edited: Vec<Vec<u8>> = kyrie.iter().map(|&at| file(&base, at)).collect();
        let at = ["mass", "--rewrite", "--movements", "kyrie,agnus", "-f", "musicxml"];
        let refused = fux(&[&at[..], &at_base].concat());
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        let told = stderr(&refused);
        assert!(told.contains(&way), "{told}");
        if version != GENERATOR {
            let resumed = fux(&[
                &["mass", "--resume", "-f", "musicxml"][..],
                &at_base,
                &MOTIF,
            ]
            .concat());
            assert_eq!(resumed.status.code(), Some(1), "{resumed:?}");
        }
        let run = way_on(&told, &MOTIF);
        let again = fux_run(&run, &[&["-f", "musicxml"][..], &at_base].concat());
        assert_eq!(again.status.code(), Some(exit), "{run:?} {again:?}");
        // Kyrie I set again by this program is the one it set.
        for (i, &at) in kyrie.iter().enumerate() {
            let kept = if version == GENERATOR || i > 0 {
                edited[i].clone()
            } else {
                file(&written.base, at)
            };
            assert_eq!(file(&base, at), kept, "{}", plan.address(at));
        }
        for &at in &agnus {
            assert_eq!(file(&base, at), file(&written.base, at));
        }
        let rewritten = fux(&[&at[..], &at_base].concat());
        assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    }
}

/// Where a part with the head motif is missing and another version of the generator set Kyrie I,
/// here Agnus Dei I of the mass with the head motif with every other part as if set by another
/// version: `--only agnus/1` would set it without the motif, as `--only` takes the motif only from
/// a Kyrie I that fits, and the next `fux mass --rewrite` fails on its motif. So the way on of the
/// missing part names Kyrie I as well, and that of the motif Kyrie I, each of which sets every
/// part with the head motif again; `--rewrite` then writes the mass.
#[test]
fn rewrite_names_kyrie_i_for_a_part_with_the_head_motif() {
    let written = motif_mass();
    assert_eq!(
        written.output.status.code(),
        Some(0),
        "{:?}",
        written.output
    );
    let plan = plan_of(&MOTIF);
    let agnus = plan.find("agnus/1").unwrap()[0];
    let kyrie_i = plan.find("kyrie/1").unwrap()[0];
    let items = format!(
        "kyrie/1={};agnus/1={}",
        degree(&plan, 0, plan.part(kyrie_i).ending),
        degree(&plan, 1, plan.part(agnus).ending)
    );
    let item = format!(";generator={GENERATOR}");
    let version = format!(";generator={}", GENERATOR + 1);
    let set = |folder: &str| {
        let base = copy_parts(&written.base, folder);
        for (at, _) in plan.parts().filter(|&(at, _)| at != agnus) {
            edit_part(&base, &plan, at, &item, &version);
        }
        std::fs::remove_file(part_path(&base, &plan, agnus)).unwrap();
        base
    };
    let rewrite = |base: &Path| {
        fux(&[
            "mass",
            "--rewrite",
            "-f",
            "musicxml",
            "-o",
            base.to_str().unwrap(),
        ])
    };
    // The way on of the missing part.
    let base = set("messe-neu-kopfmotiv");
    let refused = rewrite(&base);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    let told = stderr(&refused);
    assert!(
        told.contains(&format!(
            " Teil agnus/1 fehlt. fux mass --only kyrie/1,agnus/1 --movements kyrie,agnus mit den \
             übrigen Angaben des Setzens (ohne --seed) setzt agnus/1 und das Kyrie I und mit ihm \
             jeden Teil mit dem Kopfmotiv neu und ersetzt deren Dateien. Ans Ende von --endings \
             gehören dabei {items}: "
        )) && told.contains(
            " Das Kopfmotiv nehmen --resume und --only nur aus der Datei eines Kyrie I, das passt. "
        ),
        "{told}"
    );
    let at_base = ["-f", "musicxml", "-o", base.to_str().unwrap()];
    let run = way_on(&told, &MOTIF);
    let again = fux_run(&run, &at_base);
    assert_eq!(again.status.code(), Some(3), "{run:?} {again:?}");
    let rewritten = rewrite(&base);
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    // The part alone, then the way on of its motif.
    let base = set("messe-neu-kopfmotiv-allein");
    let at_base = ["-f", "musicxml", "-o", base.to_str().unwrap()];
    let options = ["--voices", "3", "--beam", "40", "--lengths", "natürlich"];
    let alone = fux(&[
        &["mass", "--only", "agnus/1", "--movements", "kyrie,agnus"][..],
        &options,
        &at_base,
    ]
    .concat());
    assert_eq!(alone.status.code(), Some(3), "{alone:?}");
    let refused = rewrite(&base);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    let told = stderr(&refused);
    assert!(
        told.contains(&format!(
            " Teil agnus/1 nahm ein anderes Kopfmotiv, als das Kyrie I gibt. Nach einer Änderung \
             von Programm oder Regelprofil nehmen --resume und --only das Kopfmotiv nicht mehr aus \
             der Datei des Kyrie I; fux mass --only kyrie/1 --movements kyrie,agnus mit den \
             übrigen Angaben des Setzens (ohne --seed) setzt das Kyrie I und mit ihm jeden Teil \
             mit dem Kopfmotiv neu und ersetzt deren Dateien. Ans Ende von --endings gehören dabei \
             {items}: "
        )),
        "{told}"
    );
    let run = way_on(&told, &MOTIF);
    let again = fux_run(&run, &at_base);
    assert_eq!(again.status.code(), Some(3), "{run:?} {again:?}");
    let rewritten = rewrite(&base);
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
}

/// The options of a mass in the form normal: its Sanctus and Benedictus for A T B with a narrow
/// search, seed 1, which take seconds in a debug build. The Benedictus ends with the Osanna ut
/// supra, which repeats the Osanna of the Sanctus, so that `fux mass` sets it only with the
/// Sanctus.
const NORMAL: [&str; 10] = [
    "--preset",
    "normal",
    "--movements",
    "sanctus,benedictus",
    "--voices",
    "3",
    "--beam",
    "40",
    "--seed",
    "1",
];

/// Where `NAME.teile/` keeps no part of the Benedictus of a mass in the form normal, here the mass
/// of [`NORMAL`], the way on of `fux mass --rewrite` names the Sanctus as well, without which
/// `fux mass` refuses the Benedictus, whose Osanna ut supra repeats the Osanna of the Sanctus.
/// Where this program set the Sanctus, `--resume --movements sanctus,benedictus` sets the
/// Benedictus as it was set, byte for byte, and takes the Sanctus from its files; where another
/// version set it, `--resume` would refuse that, and `--only benedictus` with both movements sets
/// the Benedictus and leaves the Sanctus as it is. Either way `--rewrite` then writes the mass.
#[test]
fn rewrite_names_the_sanctus_beside_the_benedictus() {
    let written = set_mass_as("messe-normal", &NORMAL, "musicxml");
    assert_eq!(
        written.output.status.code(),
        Some(0),
        "{:?}",
        written.output
    );
    let refused = fux(&[
        "mass",
        "--plan",
        "--preset",
        "normal",
        "--movements",
        "benedictus",
    ]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(
        stderr(&refused).ends_with("; dazu gehört das Sanctus.\n"),
        "{refused:?}"
    );
    let plan = plan_of(&NORMAL);
    let file = |base: &Path, at| std::fs::read(part_path(base, &plan, at)).unwrap();
    let sanctus = parts_of(&plan, Movement::Sanctus);
    let benedictus = parts_of(&plan, Movement::Benedictus);
    let item = format!(";generator={GENERATOR}");
    for (version, way, exit) in [
        (
            GENERATOR,
            " Vom Benedictus liegt kein Teil vor. fux mass --resume --movements sanctus,benedictus \
             mit den übrigen Angaben des Setzens setzt das Benedictus und nimmt die Teile des \
             Sanctus aus ihren Dateien, ohne Teile anderer Sätze zu ersetzen. ",
            0,
        ),
        (
            GENERATOR + 1,
            " Vom Benedictus liegt kein Teil vor. fux mass --only benedictus --movements \
             sanctus,benedictus mit den übrigen Angaben des Setzens (ohne --seed) setzt das \
             Benedictus, ohne Teile anderer Sätze zu ersetzen. ",
            3,
        ),
    ] {
        let base = copy_parts(&written.base, &format!("messe-normal-{version}"));
        let at_base = ["-o", base.to_str().unwrap()];
        for &at in &benedictus {
            std::fs::remove_file(part_path(&base, &plan, at)).unwrap();
        }
        let other = format!(";generator={version}");
        for &at in &sanctus {
            edit_part(&base, &plan, at, &item, &other);
        }
        let edited: Vec<Vec<u8>> = sanctus.iter().map(|&at| file(&base, at)).collect();
        let at = ["mass", "--rewrite", "--movements", "sanctus,benedictus"];
        let rewrite = || fux(&[&at[..], &["-f", "musicxml"], &at_base].concat());
        let refused = rewrite();
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        let told = stderr(&refused);
        assert!(told.contains(way), "{told}");
        let run = way_on(&told, &NORMAL);
        let again = fux_run(&run, &[&["-f", "musicxml"][..], &at_base].concat());
        assert_eq!(again.status.code(), Some(exit), "{run:?} {again:?}");
        for (&at, edited) in sanctus.iter().zip(&edited) {
            assert_eq!(&file(&base, at), edited, "{}", plan.address(at));
        }
        for &at in &benedictus {
            assert_eq!(file(&base, at), file(&written.base, at));
        }
        let rewritten = rewrite();
        assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    }
}

/// Where `NAME.teile/` keeps no part of the Kyrie of a mass with the head motif, here the mass of
/// [`MOTIF`], a run that sets the Kyrie sets Kyrie I anew, whose motif every part kept that takes
/// it must take. Where this program set the Agnus Dei, the way on of `fux mass --rewrite` is
/// `--resume --movements kyrie`, which sets the Kyrie as it was set, byte for byte. Where another
/// version set it, `--only` with the sections of the Kyrie and every movement kept sets the Kyrie
/// and with its Kyrie I Agnus Dei I at once, as they were set, and leaves Agnus Dei II as it is;
/// with the Kyrie alone, the next `--rewrite` would fail on the motif of each such part, one after
/// the other, where the new Kyrie I gave another. Either way `--rewrite` then writes the mass.
#[test]
fn rewrite_sets_the_kyrie_with_every_part_with_the_head_motif() {
    let written = motif_mass();
    assert_eq!(
        written.output.status.code(),
        Some(0),
        "{:?}",
        written.output
    );
    let plan = plan_of(&MOTIF);
    let file = |base: &Path, at| std::fs::read(part_path(base, &plan, at)).unwrap();
    let kyrie = parts_of(&plan, Movement::Kyrie);
    let agnus = parts_of(&plan, Movement::Agnus);
    let agnus_i = degree(&plan, 1, plan.part(agnus[0]).ending);
    let item = format!(";generator={GENERATOR}");
    for (version, way, exit) in [
        (
            GENERATOR,
            " Vom Kyrie liegt kein Teil vor. fux mass --resume --movements kyrie mit den übrigen \
             Angaben des Setzens setzt das Kyrie, ohne Teile anderer Sätze zu ersetzen. "
                .to_string(),
            0,
        ),
        (
            GENERATOR + 1,
            format!(
                " Vom Kyrie liegt kein Teil vor. fux mass --only kyrie,christe,kyrie2 --movements \
                 kyrie,agnus mit den übrigen Angaben des Setzens (ohne --seed) setzt das Kyrie und \
                 mit dessen Kyrie I jeden Teil mit dem Kopfmotiv neu und ersetzt deren Dateien. \
                 Ans Ende von --endings gehören dabei agnus/1={agnus_i}: die Schlussstufe, die die \
                 Dateien halten. "
            ),
            3,
        ),
    ] {
        let base = copy_parts(&written.base, &format!("messe-neu-kyrie-{version}"));
        let at_base = ["-o", base.to_str().unwrap()];
        for &at in &kyrie {
            std::fs::remove_file(part_path(&base, &plan, at)).unwrap();
        }
        let other = format!(";generator={version}");
        for &at in &agnus {
            edit_part(&base, &plan, at, &item, &other);
        }
        let edited: Vec<Vec<u8>> = agnus.iter().map(|&at| file(&base, at)).collect();
        let at = [
            "mass",
            "--rewrite",
            "--movements",
            "kyrie,agnus",
            "-f",
            "musicxml",
        ];
        let refused = fux(&[&at[..], &at_base].concat());
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        let told = stderr(&refused);
        assert!(told.contains(&way), "{told}");
        let run = way_on(&told, &MOTIF);
        let again = fux_run(&run, &[&["-f", "musicxml"][..], &at_base].concat());
        assert_eq!(again.status.code(), Some(exit), "{run:?} {again:?}");
        for &at in &kyrie {
            assert_eq!(
                file(&base, at),
                file(&written.base, at),
                "{}",
                plan.address(at)
            );
        }
        for (&at, edited) in agnus.iter().zip(&edited) {
            let kept = if version != GENERATOR && plan.part(at).motif {
                file(&written.base, at)
            } else {
                edited.clone()
            };
            assert_eq!(file(&base, at), kept, "{}", plan.address(at));
        }
        let rewritten = fux(&[&at[..], &at_base].concat());
        assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    }
}

/// Where a part is missing whose movement's files are there, here the Christe of the small mass
/// with the files of its Kyrie, these may keep its last copy: `fux mass --rewrite` says so, and
/// the run it names, with `-f ly` as wherever a file of a movement is there, leaves them byte for
/// byte, whether this program or another version set the other parts, so that a copy elsewhere
/// keeps the part as it was set; `--rewrite` then writes the mass.
#[test]
fn rewrite_names_the_last_copy_of_a_missing_part() {
    let written = small_mass();
    let plan = small_plan();
    let base = copy_parts(&written.base, "messe-neu-letzte-fassung");
    let at_base = ["-o", base.to_str().unwrap()];
    let dir = parts_dir(&base).display().to_string();
    let kinds = ["musicxml", "mid"];
    for kind in kinds {
        let from = movement_path(&written.base, Movement::Kyrie, kind);
        std::fs::copy(from, movement_path(&base, Movement::Kyrie, kind)).unwrap();
    }
    let files = |base: &Path| -> Vec<Vec<u8>> {
        kinds
            .iter()
            .map(|kind| std::fs::read(movement_path(base, Movement::Kyrie, kind)).unwrap())
            .collect()
    };
    let named: Vec<String> = kinds
        .iter()
        .map(|kind| {
            movement_path(&base, Movement::Kyrie, kind)
                .display()
                .to_string()
        })
        .collect();
    let copy = format!(
        " Teil kyrie/2 fehlt. Wie er gesetzt wurde, kann der Teil nur noch in {} stehen: Ist er neu \
         gesetzt, schreibt --rewrite diese Dateien neu; eine Kopie an anderem Ort behält ihn. ",
        named.join(", ")
    );
    let lean = format!(
        " Mit -f ly allein (ohne musicxml und midi) schreibt und löscht fux mass keine Satzdatei, \
         und --rewrite schreibt danach die Messe: Satzdateien können Teile halten, die in {dir} \
         fehlen. "
    );
    let christe = plan.find("christe").unwrap()[0];
    let others: Vec<PartRef> = plan
        .parts()
        .map(|(at, _)| at)
        .filter(|&at| at != christe)
        .collect();
    let item = format!(";generator={GENERATOR}");
    let rewrite = || fux(&[&["mass", "--rewrite", "-f", "musicxml,midi"][..], &at_base].concat());
    for (version, way, exit) in [
        (
            GENERATOR,
            "Fehlende Teile setzt fux mass --resume --movements kyrie -f ly mit den übrigen \
             Angaben des Setzens nach.",
            0,
        ),
        (
            GENERATOR + 1,
            "fux mass --only kyrie/2 --movements kyrie -f ly mit den übrigen Angaben des Setzens \
             (ohne --seed) setzt ihn nach.",
            3,
        ),
    ] {
        let other = format!(";generator={version}");
        for &at in &others {
            let path = part_path(&base, &plan, at);
            let text = std::fs::read_to_string(&path).unwrap();
            std::fs::write(&path, text.replace(&item, &other)).unwrap();
        }
        std::fs::remove_file(part_path(&base, &plan, christe)).unwrap();
        let refused = rewrite();
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        let told = stderr(&refused);
        assert!(
            told.contains(&format!("{copy}{way}")) && told.contains(&lean),
            "{told}"
        );
        let run = way_on(&told, &SMALL);
        let again = fux_run(&run, &at_base);
        assert_eq!(again.status.code(), Some(exit), "{run:?} {again:?}");
        assert_eq!(files(&base), files(&written.base));
        assert_eq!(
            std::fs::read(part_path(&base, &plan, christe)).unwrap(),
            std::fs::read(part_path(&written.base, &plan, christe)).unwrap()
        );
        let rewritten = rewrite();
        assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    }
}

/// Where the parts at the end of a movement are missing after a change, here the Christe and
/// Kyrie II of the small mass with Kyrie I as if another version set it, the files do not say how
/// many parts today's plan has after the last one kept: `fux mass --rewrite` names `--only` with
/// the first and says that `fux mass --plan --details` lists the others. As it names it, the run
/// sets the Christe, and the next `--rewrite` names Kyrie II in the same way; with the addresses
/// that `--plan --details` lists, one run sets both. Either way the parts come out as they were
/// set, byte for byte, and `--rewrite` then writes the mass.
#[test]
fn rewrite_names_the_first_of_the_parts_missing_at_the_end() {
    let written = small_mass();
    let plan = small_plan();
    let first = plan.find("kyrie/1").unwrap()[0];
    let tail: Vec<PartRef> = plan
        .parts()
        .map(|(at, _)| at)
        .filter(|&at| at != first)
        .collect();
    let file = |base: &Path, at| std::fs::read(part_path(base, &plan, at)).unwrap();
    let item = format!(";generator={GENERATOR}");
    let version = format!(";generator={}", GENERATOR + 1);
    let set = |folder: &str| {
        let base = copy_parts(&written.base, folder);
        edit_part(&base, &plan, first, &item, &version);
        for &at in &tail {
            std::fs::remove_file(part_path(&base, &plan, at)).unwrap();
        }
        base
    };
    let rewrite = |base: &Path| {
        fux(&[
            "mass",
            "--rewrite",
            "-f",
            "musicxml",
            "-o",
            base.to_str().unwrap(),
        ])
    };
    let ending = degree(&plan, 0, plan.part(first).ending);
    // As the way on names it: the first, then the next.
    let base = set("messe-neu-ende");
    let at_base = ["-f", "musicxml", "-o", base.to_str().unwrap()];
    let refused = rewrite(&base);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    let told = stderr(&refused);
    assert!(
        told.contains(&format!(
            " Die Teile ab kyrie/2 fehlen. fux mass --only kyrie/2 --movements kyrie mit den \
             übrigen Angaben des Setzens (ohne --seed) setzt kyrie/2 nach. Ans Ende von --endings \
             gehören dabei kyrie/1={ending}: die Schlussstufe, die die Dateien halten. Die Teile \
             des Kyrie danach nennt fux mass --plan --details mit denselben Angaben; mit ihren \
             Adressen in --only setzt es auch sie, sonst nennt das nächste --rewrite den nächsten \
             fehlenden Teil. "
        )),
        "{told}"
    );
    let run = way_on(&told, &SMALL);
    let again = fux_run(&run, &at_base);
    assert_eq!(again.status.code(), Some(3), "{run:?} {again:?}");
    let refused = rewrite(&base);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    let told = stderr(&refused);
    assert!(
        told.contains(
            " Die Teile ab kyrie/3 fehlen. fux mass --only kyrie/3 --movements kyrie mit den \
             übrigen Angaben des Setzens (ohne --seed) setzt kyrie/3 nach. "
        ),
        "{told}"
    );
    let run = way_on(&told, &SMALL);
    let again = fux_run(&run, &at_base);
    assert_eq!(again.status.code(), Some(3), "{run:?} {again:?}");
    for &at in &tail {
        assert_eq!(
            file(&base, at),
            file(&written.base, at),
            "{}",
            plan.address(at)
        );
    }
    let rewritten = rewrite(&base);
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    // With the addresses that --plan --details lists from the first on.
    let base = set("messe-neu-ende-alle");
    let at_base = ["-f", "musicxml", "-o", base.to_str().unwrap()];
    let refused = rewrite(&base);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    let mut run = way_on(&stderr(&refused), &SMALL);
    let planned = fux_run(&run, &[&["--plan", "--details"][..], &at_base].concat());
    assert_eq!(planned.status.code(), Some(0), "{planned:?}");
    let text = stdout(&planned);
    let listed: Vec<&str> = text
        .lines()
        .filter_map(|line| Some(line.split_once(": ")?.0))
        .filter(|address| address.starts_with("kyrie/"))
        .skip_while(|&address| address != "kyrie/2")
        .collect();
    assert_eq!(listed, ["kyrie/2", "kyrie/3"], "{text}");
    let only = run.iter().position(|arg| arg == "--only").unwrap() + 1;
    run[only] = listed.join(",");
    let again = fux_run(&run, &at_base);
    assert_eq!(again.status.code(), Some(3), "{run:?} {again:?}");
    for &at in &tail {
        assert_eq!(
            file(&base, at),
            file(&written.base, at),
            "{}",
            plan.address(at)
        );
    }
    let rewritten = rewrite(&base);
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
}

/// `--only` sets the parts it names again and takes the others from their files: with the
/// recorded seed a part comes out as before, byte for byte. A part whose file is missing stays
/// missing: the mass is incomplete (exit code 3), says so and how to set it, with another seed
/// through `--only`, and the book has a remark where the part would stand, in lines the page has
/// room for; its movement has no file of its own, and the file of an earlier mass under its name
/// is gone. With `--seed` a part takes a seed of its own, which `--plan --details` shows, and a
/// later `--resume` keeps it. Without the files of a mass `--only` fails.
#[test]
fn only_sets_parts_again() {
    let written = small_mass();
    let plan = small_plan();
    let base = copy_parts(&written.base, "messe-teil");
    let first = plan.find("kyrie/1").unwrap()[0];
    let last = plan.find("kyrie2").unwrap()[0];
    std::fs::remove_file(part_path(&base, &plan, first)).unwrap();
    // The file of the Kyrie of an earlier mass.
    let kyrie = movement_path(&base, Movement::Kyrie, "musicxml");
    std::fs::copy(
        movement_path(&written.base, Movement::Kyrie, "musicxml"),
        &kyrie,
    )
    .unwrap();
    let at_base = ["-o", base.to_str().unwrap()];
    let again = fux(&[
        &["mass", "--only", "kyrie2", "-f", "ly,musicxml"][..],
        &at_base,
        &SMALL,
    ]
    .concat());
    assert_eq!(again.status.code(), Some(3), "{again:?}");
    assert!(!kyrie.exists());
    assert!(!stdout(&again).contains("-1-kyrie.musicxml"), "{again:?}");
    let file = |base: &Path, at| std::fs::read(part_path(base, &plan, at)).unwrap();
    assert_eq!(file(&base, last), file(&written.base, last));
    let told = stderr(&again);
    let heading = |at, seed: Option<u64>| {
        let line = PartLine::of(&plan, at, Lang::De);
        let seed = seed.unwrap_or(line.seed);
        PartLine { seed, ..line }.heading(Lang::De)
    };
    assert!(
        told.lines().count() == 2 && told.contains(&heading(last, None)),
        "{told}"
    );
    let text = stdout(&again);
    let another_seed = "einem einzelnen Teil gibt --only TEIL --seed N einen anderen Seed.";
    assert!(
        text.contains(&format!("\n{}: fehlt: nicht gesetzt", heading(first, None)))
            && text.contains("\nUnvollständig, es fehlen: kyrie/1. fux mass --resume ")
            && text.contains(&format!("; {another_seed}\n")),
        "{text}"
    );
    // The remarks under the section, in lines the page has room for.
    let ly = std::fs::read_to_string(with_extension(&base, "ly")).unwrap();
    let remarks: Vec<&str> = ly
        .lines()
        .filter_map(|line| line.trim().strip_prefix("\\line { \""))
        .filter_map(|line| line.strip_suffix("\" }"))
        .collect();
    assert!(
        remarks
            .first()
            .is_some_and(|first| first.starts_with("Teil kyrie/1 fehlt: nicht gesetzt"))
            && remarks.join(" ").ends_with(another_seed)
            && remarks.iter().all(|line| line.chars().count() <= 85),
        "{ly}"
    );
    // A seed of its own, which --plan shows and --resume keeps.
    let own = copy_parts(&written.base, "messe-eigen");
    let at_own = ["-o", own.to_str().unwrap()];
    let christe = plan.find("christe").unwrap()[0];
    let planned = fux(&[
        &[
            "mass",
            "--plan",
            "--details",
            "--only",
            "christe",
            "--seed",
            "5",
        ][..],
        &at_own,
        &SMALL,
    ]
    .concat());
    let details = |at, seed: Option<u64>| {
        let line = PartLine::of(&plan, at, Lang::De);
        let seed = seed.unwrap_or(line.seed);
        let line = PartLine { seed, ..line };
        format!("\n{}: {}; ", plan.address(at), line.description(Lang::De))
    };
    let text = stdout(&planned);
    assert!(
        text.starts_with(&format!(
            "{}\n",
            mass_summary(&plan, default_jobs(), Lang::De)
        )) && text.contains(&details(first, None))
            && text.contains(&details(christe, Some(5)))
            && text.contains(&details(last, None)),
        "{planned:?}"
    );
    let reseeded = fux(&[
        &["mass", "--only", "christe", "--seed", "5", "-f", "musicxml"][..],
        &at_own,
        &SMALL,
    ]
    .concat());
    assert_eq!(reseeded.status.code(), Some(0), "{reseeded:?}");
    assert!(stderr(&reseeded).contains(&format!("{}: ", heading(christe, Some(5)))));
    let kept = file(&own, christe);
    assert_ne!(kept, file(&written.base, christe));
    let resumed = fux(&[&["mass", "--resume", "-f", "musicxml"][..], &at_own, &SMALL].concat());
    assert_eq!(resumed.status.code(), Some(0), "{resumed:?}");
    let dir = parts_dir(&own).display().to_string();
    let told = stderr(&resumed);
    assert!(
        told.lines()
            .all(|line| line.ends_with(&format!(", aus {dir}"))),
        "{told}"
    );
    assert_eq!(file(&own, christe), kept);
    // No mass to go on with.
    let none = tmp("messe-ohne").join("messe");
    for (lang, message) in [
        ("de", "Fehler: --only setzt Teile einer Messe in "),
        ("en", "Error: --only sets parts of a mass in "),
    ] {
        let refused = fux(&[
            "--lang",
            lang,
            "mass",
            "--only",
            "christe",
            "-o",
            none.to_str().unwrap(),
        ]);
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        assert!(stderr(&refused).starts_with(message), "{refused:?}");
    }
}

/// The measures of the parts of the check of the file of a movement, `report`, as their headings
/// name them („Teil kyrie/2: Christe, Takte 33–57“), first and last; every measure that a line on
/// a violation („  Takt 39, Viertel 3 …“) or a point („  Punkt 1 …: Alt T. 33 A4 · …“) of a part
/// names lies in them, counted through the movement.
fn measures_of_parts(report: &str) -> Vec<(usize, usize)> {
    let mut parts = Vec::new();
    for line in report.lines() {
        if let Some((_, measures)) = line
            .strip_prefix("Teil ")
            .and_then(|heading| heading.rsplit_once(", Takte "))
        {
            let (first, last) = measures.split_once('–').unwrap();
            parts.push((first.parse().unwrap(), last.parse().unwrap()));
            continue;
        }
        let Some(&(first, last)) = parts.last() else {
            continue;
        };
        if !line.starts_with("  Takt ") && !line.starts_with("  Punkt ") {
            continue;
        }
        // A message may say „im Takt vor dem letzten Ton“.
        let named = line
            .match_indices("Takt ")
            .chain(line.match_indices("T. "))
            .filter_map(|(at, marker)| {
                let digits: String = line[at + marker.len()..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect();
                digits.parse::<usize>().ok()
            });
        for measure in named {
            assert!((first..=last).contains(&measure), "{line}\n{report}");
        }
    }
    parts
}

/// `fux check` of the file of a movement repeats the report of every part (D14, § 6): part by
/// part under the overlay of the mass, each with the cost `fux mass` gave it and its measures in
/// the movement, also where a voice rests through a section and is hidden there; the totals
/// last, in German and in English. Its violations and points, also those `--imitation erkennen`
/// finds, count the measures of the movement, and so do the scores that `--mark` writes, a score
/// per part. A file whose fields no longer fit its measures fails with a message that names the
/// part. A file without the fields of the parts is checked section by section from its double
/// bars and rehearsal marks, each on its own.
#[test]
fn check_of_an_exported_mass_repeats_its_report() {
    for (written, plan, details) in [
        (small_mass(), small_plan(), true),
        (tacet_mass(), plan_of(&TACET), false),
    ] {
        let parts = assert_written(written, &plan, details);
        let reports = assert_checked(&written.base, &plan, &parts);
        // Kyrie I begins in imitation: its plan comes from the file, as from its part's file.
        assert!(reports[0].contains("\n  Imitation nach dem Plan der Datei: "));
        for report in &reports {
            assert_eq!(measures_of_parts(report).len(), parts.len(), "{report}");
        }
    }
    // The points found, the marked scores, and fields that no longer fit.
    let file = movement_path(&small_mass().base, Movement::Kyrie, "musicxml");
    let found = fux(&[
        "check",
        "--imitation",
        "erkennen",
        "--musicxml",
        file.to_str().unwrap(),
    ]);
    assert!(matches!(found.status.code(), Some(0 | 2)), "{found:?}");
    let report = stdout(&found);
    let measures = measures_of_parts(&report);
    assert_eq!(measures.len(), 3, "{report}");
    assert!(
        report.contains("\n  Imitation erkannt, nur zur Information: ")
            && report.contains("\n  Punkt 1: "),
        "{report}"
    );
    let marked = tmp("messe-markiert");
    let checked = fux(&[
        "check",
        "--mark",
        "-f",
        "ly",
        "-o",
        marked.to_str().unwrap(),
        "--musicxml",
        file.to_str().unwrap(),
    ]);
    assert_eq!(checked.status.code(), Some(0), "{checked:?}");
    let ly = std::fs::read_to_string(marked.with_extension("ly")).unwrap();
    let pieces: Vec<&str> = ly
        .lines()
        .filter_map(|line| line.trim().strip_prefix("\\header { piece = \"Teil kyrie/"))
        .collect();
    assert_eq!(pieces.len(), 3, "{ly}");
    for (piece, (first, last)) in pieces.iter().zip(&measures) {
        assert!(
            piece.ends_with(&format!(", Takte {first}–{last}\" }}")),
            "{piece}"
        );
    }
    let numbered: Vec<usize> = ly
        .lines()
        .filter_map(|line| line.trim().strip_prefix("\\set Score.currentBarNumber = #"))
        .map(|first| first.parse().unwrap())
        .collect();
    let firsts: Vec<usize> = measures.iter().map(|&(first, _)| first).skip(1).collect();
    assert_eq!(numbered, firsts, "{ly}");
    let xml = std::fs::read_to_string(&file).unwrap();
    let field = from_musicxml(&xml).unwrap();
    let christe = field.field(&format!("{FIELD_PART}.2")).unwrap();
    let (bars, _) = christe.split_once(';').unwrap();
    let (first, total) = (measures[1].0, measures[2].1);
    let broken = tmp("messe-falsche-takte.musicxml");
    std::fs::write(&broken, xml.replacen(bars, &format!("bars={first}-400"), 1)).unwrap();
    for (lang, message) in [
        (
            "de",
            format!(
                "Fehler: Teil kyrie/2: Die Takte {first}–400 liegen nicht in den {total} Takten \
                 der Datei nach denen des Teils davor.\n"
            ),
        ),
        (
            "en",
            format!(
                "Error: Part kyrie/2: measures {first}–400 do not lie within the file's {total} \
                 measures after those of the part before.\n"
            ),
        ),
    ] {
        let failed = fux(&[
            "--lang",
            lang,
            "check",
            "--musicxml",
            broken.to_str().unwrap(),
        ]);
        assert_eq!(failed.status.code(), Some(1), "{failed:?}");
        assert_eq!(stderr(&failed), message);
    }
    // In English.
    let written = tacet_mass();
    let plan = plan_of(&TACET);
    let parts = assert_written(written, &plan, false);
    let file = movement_path(&written.base, Movement::Kyrie, "musicxml");
    let english = fux(&[
        "--lang",
        "en",
        "check",
        "--musicxml",
        file.to_str().unwrap(),
    ]);
    assert_eq!(english.status.code(), Some(0), "{english:?}");
    let text = stdout(&english);
    assert!(
        text.lines()
            .any(|line| line.starts_with("Part kyrie/2: Christe, measures ")),
        "{text}"
    );
    assert!(
        text.lines().last().is_some_and(|line| {
            line.starts_with("Total: 0 prohibition(s), cost ") && line.ends_with(" in 3 parts.")
        }),
        "{text}"
    );
    // Without the fields of its parts: a section from each double bar on, each on its own, the
    // Christe in the two voices that sing it.
    let xml = std::fs::read_to_string(&file).unwrap();
    let (head, rest) = xml.split_once("<miscellaneous>").unwrap();
    let (_, tail) = rest.split_once("</miscellaneous>").unwrap();
    let bare = tmp("messe-ohne-felder.musicxml");
    std::fs::write(&bare, format!("{head}{tail}")).unwrap();
    let checked = fux(&["check", "--musicxml", bare.to_str().unwrap()]);
    assert!(matches!(checked.status.code(), Some(0 | 2)), "{checked:?}");
    let report = stdout(&checked);
    let headings: Vec<&str> = report
        .lines()
        .filter(|line| line.starts_with("Abschnitt "))
        .collect();
    let mut first = 1;
    let mut expected = Vec::new();
    for ((at, _), (measures, _)) in plan.parts().zip(&parts) {
        let section = &plan.movements[at.movement].sections[at.section];
        let last = first + measures - 1;
        expected.push(format!(
            "Abschnitt {}: {}, Takte {first}–{last}",
            at.section + 1,
            section.title
        ));
        first = last + 1;
    }
    assert_eq!(headings, expected, "{report}");
    let christe: Vec<&str> = report
        .lines()
        .skip_while(|line| !line.starts_with("Abschnitt 2: "))
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .collect();
    assert!(
        !christe.is_empty() && !christe.iter().any(|line| line.contains(", Bass: ")),
        "{report}"
    );
    assert!(
        report.lines().last().is_some_and(|line| {
            line.starts_with("Gesamt: ") && line.ends_with(" in 3 Abschnitten.")
        }),
        "{report}"
    );
}

/// `fux render` of the file of a movement writes the movement as one score in its sections, as
/// the book has it: under the movement's title before the mode, a section after a double bar
/// under its title, fermatas on the last notes of every section, the last one's too.
#[test]
fn render_of_a_movement_file() {
    let written = small_mass();
    let file = movement_path(&written.base, Movement::Kyrie, "musicxml");
    let base = tmp("satz-kyrie");
    let rendered = fux(&[
        "render",
        "--musicxml",
        file.to_str().unwrap(),
        "-f",
        "ly",
        "-o",
        base.to_str().unwrap(),
    ]);
    assert!(rendered.status.success(), "{rendered:?}");
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(ly.contains("title = \"Missa brevis\""), "{ly}");
    assert!(ly.contains("piece = \"Kyrie – dorisch\""), "{ly}");
    for title in ["Kyrie I", "Christe", "Kyrie II"] {
        assert!(
            ly.contains(&format!("\\mark \\markup \\bold \"{title}\"")),
            "{ly}"
        );
    }
    let lines: Vec<&str> = ly.lines().map(str::trim).collect();
    let ends: Vec<&str> = lines
        .windows(2)
        .filter(|pair| pair[1] == "\\bar \"|.\"")
        .map(|pair| pair[0])
        .collect();
    assert_eq!(ends.len(), 3, "{ly}");
    for staff in ends {
        assert!(staff.ends_with("\\fermata"), "{staff}");
        assert_eq!(staff.matches("\\fermata").count(), 3, "{staff}");
    }
}

/// Example 01, a single setting of fux-kontrapunkt (Kyrie I in 25 measures, its internal cadence
/// in measure 17), as another program writes it again with rehearsal marks: `A` at its first
/// measure, a double bar after measure `bar` and `B` after it; with the fields that
/// fux-kontrapunkt keeps in it (`fux-mode`, `fux-cadences`, `fux-text`, `fux-imitation`) or
/// without them. Written to `name`, whose path it returns.
fn marked_example(name: &str, bar: usize, fields: bool) -> PathBuf {
    let xml = std::fs::read_to_string(EXAMPLE_01).unwrap();
    let xml = if fields {
        xml
    } else {
        let (head, rest) = xml.split_once("<miscellaneous>").unwrap();
        let (_, tail) = rest.split_once("</miscellaneous>").unwrap();
        format!("{head}{tail}")
    };
    let rehearsal = |mark: &str| {
        format!(
            "      <direction placement=\"above\"><direction-type><rehearsal>{mark}</rehearsal>\
             </direction-type></direction>\n"
        )
    };
    let (measure, next) = (
        format!("<measure number=\"{bar}\">"),
        format!("<measure number=\"{}\">", bar + 1),
    );
    let mut marked = String::new();
    let (mut part, mut double, mut first) = (0, false, true);
    for line in xml.lines() {
        let tag = line.trim();
        part += usize::from(tag.starts_with("<part id="));
        if double && tag == "</measure>" {
            marked.push_str(
                "      <barline location=\"right\"><bar-style>light-light</bar-style></barline>\n",
            );
            double = false;
        }
        marked.push_str(line);
        marked.push('\n');
        double |= tag == measure;
        if part == 1 && first && tag == "</attributes>" {
            marked.push_str(&rehearsal("A"));
            first = false;
        }
        if part == 1 && tag == next {
            marked.push_str(&rehearsal("B"));
        }
    }
    assert_eq!(marked.matches("<rehearsal>").count(), 2);
    assert_eq!(marked.matches("light-light").count(), 4);
    let path = tmp(name);
    std::fs::write(&path, marked).unwrap();
    path
}

/// The file of example 01.
const EXAMPLE_01: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/01-kyrie-imitation-dorisch-4stimmig.musicxml"
);

/// A single setting that fux-kontrapunkt wrote stays one score, whatever marks of sections a
/// program adds to it (D15): with a double bar at its internal cadence and rehearsal marks, or
/// with only a rehearsal mark after the double bar, `fux check` gives the report of the file as
/// written, with its text, its cadence and its plan of imitation, and `fux render` the same
/// LilyPond source.
#[test]
fn marks_leave_a_setting_one_score() {
    let check = |path: &Path| fux(&["check", "--musicxml", path.to_str().unwrap()]);
    let render = |path: &Path, name: &str| {
        let base = tmp(name);
        let rendered = fux(&[
            "render",
            "--musicxml",
            path.to_str().unwrap(),
            "-f",
            "ly",
            "-o",
            base.to_str().unwrap(),
        ]);
        assert!(rendered.status.success(), "{rendered:?}");
        std::fs::read_to_string(base.with_extension("ly")).unwrap()
    };
    let original = Path::new(EXAMPLE_01);
    let written = check(original);
    assert_eq!(written.status.code(), Some(0), "{written:?}");
    assert!(stdout(&written).contains("Imitation nach dem Plan der Datei: "));
    let ly = render(original, "beispiel-01");
    let marked = marked_example("beispiel-01-markiert.musicxml", 17, true);
    let checked = check(&marked);
    assert_eq!(checked.status.code(), written.status.code(), "{checked:?}");
    assert_eq!(stdout(&checked), stdout(&written));
    assert_eq!(render(&marked, "beispiel-01-markiert"), ly);
    // Only `B` after the double bar: a section at the first measure too, for the reader.
    let xml = std::fs::read_to_string(&marked).unwrap();
    let alone = tmp("beispiel-01-b.musicxml");
    std::fs::write(&alone, xml.replacen("<rehearsal>A</rehearsal>", "", 1)).unwrap();
    let checked = check(&alone);
    assert_eq!(checked.status.code(), written.status.code(), "{checked:?}");
    assert_eq!(stdout(&checked), stdout(&written));
}

/// A file without the fields of fux-kontrapunkt, such as one of another program, is checked
/// section by section from its rehearsal marks after double bars (§ 6), each section on its own:
/// a note tied over the double bar ends its section (the bass of example 01 after measure 20),
/// and an internal cadence given for the file (`--cadences`) that a section has no room for, at
/// its end, is its final cadence.
#[test]
fn sections_of_a_foreign_file_are_checked_on_their_own() {
    let headings = |output: &Output| -> Vec<String> {
        stdout(output)
            .lines()
            .filter(|line| line.starts_with("Abschnitt "))
            .map(str::to_string)
            .collect()
    };
    let tied = marked_example("fremd-20.musicxml", 20, false);
    // The last note of the bass in measure 20 is tied over the double bar.
    let xml = std::fs::read_to_string(&tied).unwrap();
    let bass = xml.split_once("<part id=\"P4\">").unwrap().1;
    let twenty = bass.split_once("<measure number=\"20\">").unwrap().1;
    let (twenty, _) = twenty.split_once("</measure>").unwrap();
    let last = twenty.rsplit("<note>").next().unwrap();
    assert!(last.contains("<tied type=\"start\"/>"), "{twenty}");
    let checked = fux(&["check", "--musicxml", tied.to_str().unwrap()]);
    assert!(matches!(checked.status.code(), Some(0 | 2)), "{checked:?}");
    assert_eq!(
        headings(&checked),
        ["Abschnitt 1: A, Takte 1–20", "Abschnitt 2: B, Takte 21–25"],
        "{checked:?}"
    );
    let cadence = marked_example("fremd-17.musicxml", 17, false);
    let checked = fux(&[
        "check",
        "--musicxml",
        cadence.to_str().unwrap(),
        "--cadences",
        "17:A",
    ]);
    assert!(matches!(checked.status.code(), Some(0 | 2)), "{checked:?}");
    assert_eq!(
        headings(&checked),
        ["Abschnitt 1: A, Takte 1–17", "Abschnitt 2: B, Takte 18–25"],
        "{checked:?}"
    );
}

/// The notation of a mass (§ 1.4) reaches its files: `--mensurstriche` draws the bar lines
/// between the staves only, in the book and in MusicXML; `--tacet verbergen` hides the bass in
/// every measure of the Christe, which it does not sing, and shows it again where it sings. The
/// MIDI file is the same either way, and so are the notes; from the same parts the defaults draw
/// the bar lines through the staves and show every voice.
#[test]
fn notation_reaches_the_files() {
    let written = tacet_mass();
    let plan = plan_of(&TACET);
    let parts = assert_written(written, &plan, false);
    let christe = plan.find("christe").unwrap()[0];
    let measures = plan
        .parts()
        .zip(&parts)
        .find(|((at, _), _)| *at == christe)
        .map(|(_, (measures, _))| *measures)
        .unwrap();
    let xml = |base: &Path| {
        std::fs::read_to_string(movement_path(base, Movement::Kyrie, "musicxml")).unwrap()
    };
    let ly = |base: &Path| std::fs::read_to_string(with_extension(base, "ly")).unwrap();
    let midi = |base: &Path| std::fs::read(movement_path(base, Movement::Kyrie, "mid")).unwrap();
    let hidden = xml(&written.base);
    assert!(hidden.contains("<group-barline>Mensurstrich</group-barline>"));
    assert_eq!(
        hidden
            .matches("<staff-details print-object=\"no\"/>")
            .count(),
        measures
    );
    assert_eq!(
        hidden
            .matches("<staff-details print-object=\"yes\"/>")
            .count(),
        1
    );
    assert!(ly(&written.base).contains("measureBarType = \"-span|\""));
    // The defaults from the same parts.
    let base = copy_parts(&written.base, "messe-tacet-standard");
    let resumed = fux(&[
        &["mass", "--resume", "-f", "ly,musicxml,midi", "-o"][..],
        &[base.to_str().unwrap()],
        &TACET,
    ]
    .concat());
    assert_eq!(resumed.status.code(), Some(0), "{resumed:?}");
    let shown = xml(&base);
    assert!(shown.contains("<group-barline>yes</group-barline>"));
    assert!(!shown.contains("print-object="));
    assert!(!ly(&base).contains("measureBarType"));
    assert_eq!(midi(&base), midi(&written.base));
    let notes = |xml: &str| {
        let read = from_musicxml(xml).unwrap();
        read.parts
            .into_iter()
            .map(|part| part.notes)
            .collect::<Vec<_>>()
    };
    assert_eq!(notes(&shown), notes(&hidden));
}

/// `--motif an|aus` and `--chant keiner|ix|iv` (phase 14c) reach the plan, which marks the parts
/// that take the head motif and names the chant of each movement, and fail as the plan cannot
/// have them, before any search, in either language: the head motif with a chant, the head
/// motif without the Kyrie, an unknown chant; an unknown value of `--motif` is no option.
#[test]
fn motif_and_chant_reach_the_plan() {
    let planned = |args: &[&str]| {
        let output = fux(&[&["mass", "--plan", "--seed", "1"][..], args].concat());
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        stdout(&output)
    };
    let text = planned(&["--motif", "an", "--movements", "kyrie,gloria,agnus"]);
    assert!(
        text.contains("\nGloria: gloria:1-5 (Imitation, Kopfmotiv) → ")
            && text.contains("\nAgnus Dei: Agnus Dei I (Imitation, Kopfmotiv) → "),
        "{text}"
    );
    assert!(!text.contains("Choral"), "{text}");
    assert!(!planned(&["--motif", "aus"]).contains("Kopfmotiv"));
    let text = planned(&[
        "--chant",
        "ix",
        "--movements",
        "kyrie,gloria,sanctus,benedictus",
    ]);
    assert!(
        text.starts_with("Missa brevis – Modus je Satz, S A T B, Seed 1: 4 Sätze, ")
            && text.contains("\nGloria (mixolydisch): gloria:1-5 (Imitation) → ")
            && text.ends_with("\nChoral: Kyrie IX, Gloria IX, Sanctus IX\n"),
        "{text}"
    );
    let text = planned(&["--chant", "IV", "--movements", "kyrie,credo"]);
    assert!(text.ends_with("\nChoral: Kyrie IV, Credo I\n"), "{text}");
    let english = fux(&[
        "--lang",
        "en",
        "mass",
        "--plan",
        "--seed",
        "1",
        "--motif",
        "on",
        "--movements",
        "kyrie,agnus",
    ]);
    assert!(
        stdout(&english).contains("\nAgnus Dei: Agnus Dei I (imitation, head motif) → "),
        "{english:?}"
    );
    let english = fux(&[
        "--lang",
        "en",
        "mass",
        "--plan",
        "--chant",
        "ix",
        "--movements",
        "kyrie",
    ]);
    assert!(
        stdout(&english).ends_with("\nChant: Kyrie IX\n"),
        "{english:?}"
    );
    // With a mode of its own the mass has one mode.
    let text = planned(&[
        "--chant",
        "ix",
        "--mode",
        "dorisch",
        "--movements",
        "gloria",
    ]);
    assert!(text.starts_with("Missa brevis – dorisch, ") && text.contains("\nGloria: "));
    let refused = |args: &[&str]| {
        let output = fux(args);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        stderr(&output)
    };
    for (args, de, en) in [
        (
            &["--motif", "an", "--chant", "ix"][..],
            "Fehler: Kopfmotiv und Choralparaphrase schließen einander aus.\n",
            "Error: The head motif and the chant paraphrase exclude each other.\n",
        ),
        (
            &["--motif", "an", "--movements", "gloria,credo"],
            "Fehler: Das Kopfmotiv kommt aus dem Kyrie I; dazu gehört das Kyrie.\n",
            "Error: The head motif comes from Kyrie I; choose the Kyrie as well.\n",
        ),
        (
            &["--chant", "xi"],
            "Fehler: Unbekannter Choral „xi“; zur Wahl stehen keiner, ix und iv.\n",
            "Error: Unknown chant \"xi\"; the choices are keiner (none), ix and iv.\n",
        ),
    ] {
        assert_eq!(refused(&[&["mass", "--plan"][..], args].concat()), de);
        assert_eq!(
            refused(&[&["--lang", "en", "mass", "--plan"][..], args].concat()),
            en
        );
    }
    let unknown = fux(&["mass", "--plan", "--motif", "vielleicht"]);
    assert_eq!(unknown.status.code(), Some(2), "{unknown:?}");
    assert!(stderr(&unknown).contains("an (on) or aus (off)"));
}

/// The options of a small mass with the head motif (D11): Kyrie and Agnus Dei for A T B, seed 4,
/// the lengths natural and a narrow search, some seconds in a debug build; Kyrie I gives a motif
/// of five tones.
const MOTIF: [&str; 12] = [
    "--movements",
    "kyrie,agnus",
    "--voices",
    "3",
    "--beam",
    "40",
    "--lengths",
    "natürlich",
    "--motif",
    "an",
    "--seed",
    "4",
];

/// Sets a mass with the options `args` and writes it to `NAME` in the fresh folder `folder`, in
/// the formats `formats` only.
fn set_mass_as(folder: &str, args: &[&str], formats: &str) -> Written {
    let dir = tmp(folder);
    std::fs::remove_dir_all(&dir).ok();
    let base = dir.join("messe");
    let at = ["mass", "-o", base.to_str().unwrap(), "-f", formats];
    let output = fux(&[&at[..], args].concat());
    Written { base, output }
}

/// The mass of [`MOTIF`], set once for the tests that read it.
fn motif_mass() -> &'static Written {
    static MASS: OnceLock<Written> = OnceLock::new();
    MASS.get_or_init(|| set_mass_as("messe-kopfmotiv", &MOTIF, "ly,musicxml,midi"))
}

/// The value of the field `name` of a MusicXML file.
fn field_of(path: &Path, name: &str) -> String {
    let file = std::fs::read_to_string(path).unwrap();
    from_musicxml(&file)
        .unwrap()
        .field(name)
        .unwrap_or_else(|| panic!("{}: no {name}", path.display()))
        .to_string()
}

/// `fux mass --motif an` (MASS-MOTIF): Kyrie I gives the head motif of its first subject, and
/// Agnus Dei I opens with it, as the plan says; the files keep it, the mass (`motif=1` of
/// `fux-mass`) and Kyrie I and Agnus Dei I the motif itself (`motif=` of `fux-part`), as does the
/// file of the Kyrie; no warning. `--resume` takes it from there: a part that takes the motif is
/// set again as before, byte for byte, and where Kyrie I is set again, so is every part that
/// takes its motif, with the same files; without `--motif` and `--seed` the files' mass goes on.
#[test]
fn a_mass_with_the_head_motif() {
    let written = motif_mass();
    let output = &written.output;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let plan = plan_of(&MOTIF);
    let text = stdout(output);
    assert!(
        text.starts_with(&format!(
            "{}\n",
            mass_summary(&plan, default_jobs(), Lang::De)
        )) && text.contains("Agnus Dei I (Imitation, Kopfmotiv)")
            && text.contains(", keine Warnung\n"),
        "{text}"
    );
    let kyrie = plan.find("kyrie/1").unwrap()[0];
    let agnus = plan.find("agnus/1").unwrap()[0];
    let part_of = |base: &Path, at| -> PartField {
        field_of(&part_path(base, &plan, at), FIELD_PART)
            .parse()
            .unwrap()
    };
    let given = part_of(&written.base, kyrie).motif.expect("a head motif");
    assert_eq!(part_of(&written.base, agnus).motif.as_ref(), Some(&given));
    let christe = plan.find("christe").unwrap()[0];
    assert_eq!(part_of(&written.base, christe).motif, None);
    let mass: MassField = field_of(&part_path(&written.base, &plan, agnus), FIELD_MASS)
        .parse()
        .unwrap();
    assert!(mass.motif && mass.to_string().ends_with(";motif=1"));
    let kyrie_file = movement_path(&written.base, Movement::Kyrie, "musicxml");
    assert!(field_of(&kyrie_file, "fux-part.1").contains(&format!(";motif={given};")));
    // Agnus Dei I set again from its file's absence.
    let files = |base: &Path| -> Vec<Vec<u8>> {
        plan.parts()
            .map(|(at, _)| std::fs::read(part_path(base, &plan, at)).unwrap())
            .collect()
    };
    let at_base = |base: &Path| ["-o".to_string(), base.display().to_string()];
    let resume = |base: &Path, args: &[&str]| {
        let at = at_base(base);
        let at: Vec<&str> = at.iter().map(String::as_str).collect();
        let formats = ["-f", "ly,musicxml,midi"];
        let resumed = fux(&[&["mass", "--resume"][..], &at, &formats, args].concat());
        assert_eq!(resumed.status.code(), Some(0), "{resumed:?}");
        resumed
    };
    let base = copy_parts(&written.base, "messe-kopfmotiv-weiter");
    std::fs::remove_file(part_path(&base, &plan, agnus)).unwrap();
    let resumed = resume(&base, &MOTIF);
    let dir = parts_dir(&base).display().to_string();
    let told = stderr(&resumed);
    let set: Vec<&str> = told
        .lines()
        .filter(|line| !line.ends_with(&format!(", aus {dir}")))
        .collect();
    let heading = PartLine::of(&plan, agnus, Lang::De).heading(Lang::De);
    assert!(set.len() == 1 && set[0].starts_with(&heading), "{told}");
    assert_eq!(files(&base), files(&written.base));
    // Kyrie I set again: Agnus Dei I with it, the same.
    std::fs::remove_file(part_path(&base, &plan, kyrie)).unwrap();
    let resumed = resume(&base, &MOTIF);
    let told = stderr(&resumed);
    let taken = told
        .lines()
        .filter(|line| line.ends_with(&format!(", aus {dir}")))
        .count();
    assert_eq!(taken, 3, "{told}");
    assert_eq!(files(&base), files(&written.base));
    let kyrie_again = movement_path(&base, Movement::Kyrie, "musicxml");
    assert_eq!(
        std::fs::read(&kyrie_again).unwrap(),
        std::fs::read(&kyrie_file).unwrap()
    );
    // Without --motif and --seed the mass of the files goes on, every part from its file.
    let without: Vec<&str> = MOTIF
        .iter()
        .copied()
        .filter(|&arg| !["--motif", "an", "--seed", "4"].contains(&arg))
        .collect();
    let resumed = resume(&base, &without);
    assert!(
        stdout(&resumed).contains(&format!("\nGesamt: 5 von 5 Teilen (5 aus {dir}), ")),
        "{resumed:?}"
    );
}

/// `fux mass --chant ix` (MASS-CHANT): the Kyrie on Kyrie IX in the mode of its chant, the files
/// naming the mass on chant (`mode=kyriale-ix`, `chant=ix` of `fux-mass`); `--resume` without
/// `--chant` and `--seed` goes on with it, setting a missing part again as before, byte for byte.
/// Some seconds in a debug build.
#[test]
fn a_mass_on_chant() {
    let args = [
        "--movements",
        "kyrie",
        "--voices",
        "3",
        "--beam",
        "40",
        "--chant",
        "ix",
        "--imitation",
        "kyrie2=aus",
        "--seed",
        "1",
    ];
    let written = set_mass_as("messe-choral", &args, "musicxml");
    let output = &written.output;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let plan = plan_of(&args);
    let text = stdout(output);
    assert!(
        text.starts_with(&format!(
            "{}\n",
            mass_summary(&plan, default_jobs(), Lang::De)
        )) && text.contains("\nKyrie: Kyrie I (Imitation) → ")
            && text.contains("\nChoral: Kyrie IX\n"),
        "{text}"
    );
    let christe = plan.find("christe").unwrap()[0];
    let mass: MassField = field_of(&part_path(&written.base, &plan, christe), FIELD_MASS)
        .parse()
        .unwrap();
    assert_eq!(mass.mode, "kyriale-ix");
    assert!(mass.to_string().ends_with(";chant=ix"), "{mass}");
    let base = copy_parts(&written.base, "messe-choral-weiter");
    std::fs::remove_file(part_path(&base, &plan, christe)).unwrap();
    let without: Vec<&str> = args
        .iter()
        .copied()
        .filter(|&arg| !["--chant", "ix", "--seed", "1"].contains(&arg))
        .collect();
    let resumed = fux(&[
        &[
            "mass",
            "--resume",
            "-f",
            "musicxml",
            "-o",
            base.to_str().unwrap(),
        ][..],
        &without,
    ]
    .concat());
    assert_eq!(resumed.status.code(), Some(0), "{resumed:?}");
    let dir = parts_dir(&base).display().to_string();
    assert!(
        stdout(&resumed).contains(&format!("\nGesamt: 3 von 3 Teilen (2 aus {dir}), ")),
        "{resumed:?}"
    );
    for (at, _) in plan.parts() {
        let file = |base: &Path| std::fs::read(part_path(base, &plan, at)).unwrap();
        assert_eq!(file(&base), file(&written.base), "{}", plan.address(at));
    }
}

/// The head motif is the mass's, whichever movements a run sets (MASS-MOTIF): the files of the
/// mass of [`MOTIF`] name it (`motif=1` of `fux-mass`) in every run, so that `--resume` and
/// `--only` with the Kyrie alone take its parts from their files and change none. Options that
/// plan Agnus Dei I without the motif (`--imitation agnus/1=aus`) are refused as other options
/// are, and the remedy the error names, `--only agnus/1` with them, sets that part alone; the
/// mass then refuses `--resume` with the options it was set with first, the other way round.
/// `--movements` without the Kyrie is refused, since the motif comes from Kyrie I. A mass of the
/// Kyrie alone with the motif writes the files of the whole mass's Kyrie, and `--resume` with the
/// Agnus Dei opens Agnus Dei I with the motif of Kyrie I's file, as the whole mass does, byte for
/// byte. Some seconds in a debug build.
#[test]
fn the_head_motif_is_the_mass_s_in_every_run() {
    let written = motif_mass();
    assert_eq!(
        written.output.status.code(),
        Some(0),
        "{:?}",
        written.output
    );
    let plan = plan_of(&MOTIF);
    let files = |base: &Path| -> Vec<Vec<u8>> {
        plan.parts()
            .map(|(at, _)| std::fs::read(part_path(base, &plan, at)).unwrap())
            .collect()
    };
    let base = copy_parts(&written.base, "messe-kopfmotiv-saetze");
    let dir = parts_dir(&base).display().to_string();
    // The options of the mass but its movements, motif and seed, which come from the files.
    let options = ["--voices", "3", "--beam", "40", "--lengths", "natürlich"];
    let run = |args: &[&str]| -> Output {
        let at = ["mass", "-f", "musicxml", "-o", base.to_str().unwrap()];
        fux(&[&at[..], args, &options].concat())
    };
    let totals = |output: &Output, total: usize, taken: usize| {
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let parts = format!("\nGesamt: {total} von {total} Teilen ({taken} aus {dir}), ");
        assert!(stdout(output).contains(&parts), "{output:?}");
    };
    let kyrie = run(&["--resume", "--movements", "kyrie"]);
    totals(&kyrie, 3, 3);
    let christe = run(&["--only", "christe", "--movements", "kyrie"]);
    totals(&christe, 3, 2);
    assert_eq!(files(&base), files(&written.base));
    // Agnus Dei I planned together: its file opens with the motif.
    let together = ["--movements", "kyrie,agnus", "--imitation", "agnus/1=aus"];
    let refused = run(&[&["--resume"][..], &together].concat());
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(
        stderr(&refused).contains(" passen: agnus/1 (Imitation). ") && stdout(&refused).is_empty(),
        "{refused:?}"
    );
    assert_eq!(files(&base), files(&written.base));
    let remedy = run(&[&["--only", "agnus/1"][..], &together].concat());
    totals(&remedy, 5, 4);
    let agnus = plan.find("agnus/1").unwrap()[0];
    let opened: PartField = field_of(&part_path(&base, &plan, agnus), FIELD_PART)
        .parse()
        .unwrap();
    assert!(opened.chord.is_some() && opened.motif.is_none(), "{opened}");
    let remedied = files(&base);
    let refused = run(&["--resume", "--movements", "kyrie,agnus"]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(
        stderr(&refused).contains(" passen: agnus/1 (Imitation). "),
        "{refused:?}"
    );
    assert_eq!(files(&base), remedied);
    // Without the Kyrie, in both languages.
    for (lang, message) in [
        (
            "de",
            format!(
                "Fehler: Die Messe in {dir} hat ein Kopfmotiv, und das kommt aus dem Kyrie I; dazu \
                 gehört das Kyrie in --movements. Seine Teile nehmen --resume und --only aus ihren \
                 Dateien.\n"
            ),
        ),
        (
            "en",
            format!(
                "Error: The mass in {dir} has a head motif, which comes from Kyrie I; choose the \
                 Kyrie in --movements as well. --resume and --only take its parts from their \
                 files.\n"
            ),
        ),
    ] {
        let at = ["--lang", lang, "mass", "-o", base.to_str().unwrap()];
        let refused = fux(&[&at[..], &["--resume", "--movements", "agnus"], &options].concat());
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        assert_eq!(stderr(&refused), message);
    }
    assert_eq!(files(&base), remedied);
    // The Kyrie alone with the motif, then with the Agnus Dei.
    let alone = [
        "--movements",
        "kyrie",
        "--voices",
        "3",
        "--beam",
        "40",
        "--lengths",
        "natürlich",
        "--motif",
        "an",
        "--seed",
        "4",
    ];
    let kyrie_alone = set_mass_as("messe-kopfmotiv-kyrie", &alone, "musicxml");
    assert_eq!(
        kyrie_alone.output.status.code(),
        Some(0),
        "{:?}",
        kyrie_alone.output
    );
    let kyrie_plan = plan_of(&alone);
    for (at, _) in kyrie_plan.parts() {
        assert_eq!(
            std::fs::read(part_path(&kyrie_alone.base, &kyrie_plan, at)).unwrap(),
            std::fs::read(part_path(&written.base, &plan, at)).unwrap(),
            "{}",
            kyrie_plan.address(at)
        );
    }
    let at = ["mass", "--resume", "-f", "musicxml", "-o"];
    let extended = fux(&[
        &at[..],
        &[
            kyrie_alone.base.to_str().unwrap(),
            "--movements",
            "kyrie,agnus",
        ],
        &options,
    ]
    .concat());
    assert_eq!(extended.status.code(), Some(0), "{extended:?}");
    assert!(
        stdout(&extended).contains("Agnus Dei I (Imitation, Kopfmotiv)"),
        "{extended:?}"
    );
    assert_eq!(files(&kyrie_alone.base), files(&written.base));
}

/// A point of a mass on chant whose phrase gave no usable head takes its subject from the words
/// and says so (MASS-CHANT), when it is set and again when `--resume` or `--only` takes its part
/// from its file (`fell=2` of `fux-part`), in the report and in `NAME.txt`, in either language:
/// Kyrie I of Kyrie IV for A T B, seed 4. Some seconds in a debug build.
#[test]
fn a_chant_point_from_the_words_warns_from_its_file() {
    let args = [
        "--movements",
        "kyrie",
        "--voices",
        "3",
        "--beam",
        "40",
        "--chant",
        "iv",
        "--seed",
        "4",
    ];
    let written = set_mass_as("messe-choral-text", &args, "musicxml");
    let plan = plan_of(&args);
    let first = plan.find("kyrie/1").unwrap()[0];
    let field: PartField = field_of(&part_path(&written.base, &plan, first), FIELD_PART)
        .parse()
        .unwrap();
    assert_eq!(field.fell, [2], "{field}");
    let warned = |output: &Output, lang: &str| {
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let (count, warning) = match lang {
            "de" => (
                ", 1 Warnung\n",
                "\nWarnung: kyrie/1: Punkt 2: kein brauchbarer Kopf aus dem Choral; Soggetto aus \
                 dem Text\n",
            ),
            _ => (
                ", 1 warning\n",
                "\nWarning: kyrie/1: Point 2: no usable head from the chant; subject from the \
                 text\n",
            ),
        };
        let text = stdout(output);
        assert!(text.contains(count) && text.contains(warning), "{text}");
        let kept = std::fs::read_to_string(with_extension(&written.base, "txt")).unwrap();
        assert!(kept.contains(count) && kept.contains(warning), "{kept}");
    };
    warned(&written.output, "de");
    let dir = parts_dir(&written.base).display().to_string();
    // Kyrie I from its file: every part, or all but the Christe.
    for (lang, again, taken) in [
        ("de", &["--resume"][..], 3),
        ("en", &["--resume"][..], 3),
        ("de", &["--only", "christe"][..], 2),
    ] {
        let at = ["--lang", lang, "mass", "-f", "musicxml", "-o"];
        let base = written.base.to_str().unwrap();
        let options = ["--movements", "kyrie", "--voices", "3", "--beam", "40"];
        let output = fux(&[&at[..], &[base], again, &options].concat());
        warned(&output, lang);
        let parts = match lang {
            "de" => format!("3 von 3 Teilen ({taken} aus {dir}), "),
            _ => format!("3 of 3 parts ({taken} from {dir}), "),
        };
        assert!(stdout(&output).contains(&parts), "{output:?}");
    }
}

/// The Kyrie for three voices with seed 1 as § 8 of the design names it, with the search at its
/// full width: about a minute in a debug build. `cargo test -p fux-cli --test cli -- --ignored
/// --nocapture a_kyrie_in_three_voices_is_written`.
#[test]
#[ignore = "slow in a debug build: sets three parts with the full search"]
fn a_kyrie_in_three_voices_is_written() {
    let args = ["--movements", "kyrie", "--voices", "3", "--seed", "1"];
    let written = set_mass("messe-kyrie", &args);
    assert_written(&written, &plan_of(&args), false);
}

/// The acceptance of `fux mass` (§ 8): a Missa brevis for S A T B in the Dorian mode, seed 1,
/// set and written whole, exit code 0, with every file, and every part's file checked again with
/// the cost of its search. The exports (§ 8): the book compiles where LilyPond is installed (the
/// PDF is written), the six MusicXML files validate against the MusicXML 4.0 schema where
/// `FUX_MUSICXML_XSD` names it, with the intonation over the Gloria and the Credo, and `fux check`
/// of each repeats the cost of every part; the six MIDI files parse with their markers. Written
/// again from the parts with Mensurstriche, the tacet voices hidden and without the intonation,
/// the files validate too and their check repeats every cost. Some minutes in release: `cargo
/// test --release -p fux-cli --test cli -- --ignored --nocapture a_missa_brevis_written`.
#[test]
#[ignore = "release: sets a whole mass, 5 to 8 minutes"]
fn a_missa_brevis_written() {
    let args = [
        "--preset", "brevis", "--mode", "dorisch", "--parts", "S A T B", "--seed", "1",
    ];
    let clock = std::time::Instant::now();
    let written = set_mass("messe-brevis", &args);
    println!("{:.0} s", clock.elapsed().as_secs_f64());
    print!("{}", stderr(&written.output));
    let plan = plan_of(&args);
    let parts = assert_written(&written, &plan, false);
    assert_eq!(parts.len(), 26);
    assert_eq!(plan.movements.len(), 6);
    for ((at, _), (_, cost)) in plan.parts().zip(&parts) {
        let file = part_path(&written.base, &plan, at);
        let checked = fux(&["check", "--musicxml", file.to_str().unwrap()]);
        assert_eq!(checked.status.code(), Some(0), "{checked:?}");
        let report = stdout(&checked);
        assert!(
            report
                .lines()
                .last()
                .is_some_and(|last| last.starts_with(&format!("0 Verbot(e), Kosten {cost} aus "))),
            "{}: {report}",
            plan.address(at)
        );
    }
    // The exports, as written and again with the other notation.
    let clock = std::time::Instant::now();
    if fux_export::lilypond_version().is_some() {
        assert!(with_extension(&written.base, "pdf").exists());
    }
    let again = copy_parts(&written.base, "messe-brevis-mensur");
    let formats = if mass_formats().is_empty() {
        "ly,pdf,musicxml,midi"
    } else {
        "ly,musicxml,midi"
    };
    let rewritten = fux(&[
        &[
            "mass",
            "--resume",
            "--mensurstriche",
            "--tacet",
            "verbergen",
        ][..],
        &["--intonation", "aus", "-f", formats, "-o"],
        &[again.to_str().unwrap()],
        &args,
    ]
    .concat());
    assert_eq!(rewritten.status.code(), Some(0), "{rewritten:?}");
    if mass_formats().is_empty() {
        assert!(with_extension(&again, "pdf").exists());
    }
    assert_movements(&again, &plan, &parts);
    let mut validated = 0;
    for (base, plain) in [(&written.base, true), (&again, false)] {
        assert_checked(base, &plan, &parts);
        for movement in &plan.movements {
            let path = movement_path(base, movement.movement, "musicxml");
            validated += usize::from(validate_musicxml(&path));
            let xml = std::fs::read_to_string(&path).unwrap();
            let intonation = movement.intonation.map(|text| format!("Intonatio: {text}"));
            if let Some(intonation) = &intonation {
                assert_eq!(xml.contains(intonation.as_str()), plain, "{intonation}");
            }
            let barline = if plain { "yes" } else { "Mensurstrich" };
            assert!(xml.contains(&format!("<group-barline>{barline}</group-barline>")));
            // Tacet voices: the bass in the Benedictus, the second tenor in Agnus Dei I.
            let tacet = movement.sections.iter().any(|section| {
                section.ut_supra.is_none() && section.voices.len() < movement.voices.len()
            });
            assert_eq!(
                xml.contains("<staff-details print-object=\"no\"/>"),
                tacet && !plain,
                "{}",
                movement.movement.id()
            );
        }
        let ly = std::fs::read_to_string(with_extension(base, "ly")).unwrap();
        assert_eq!(ly.contains("Intonatio: "), plain);
        assert_eq!(ly.contains("measureBarType"), !plain);
    }
    if std::env::var_os("FUX_MUSICXML_XSD").is_some() {
        assert_eq!(validated, 12);
    }
    println!("exports: {:.0} s", clock.elapsed().as_secs_f64());
}

/// The measures and the cost of each part of the mass of `plan` from the report of `fux mass`,
/// `text`: the line after the plan that names it, set („…: 34 Takte, Kosten 45, 11,3 s“) or taken
/// from its file („…: 34 Takte, Kosten 45, aus …“).
fn parts_reported(plan: &MassPlan, text: &str) -> Vec<(usize, u32)> {
    let summary = mass_summary(plan, default_jobs(), Lang::De);
    assert!(text.starts_with(&format!("{summary}\n")), "{text}");
    text.lines()
        .skip(summary.lines().count())
        .take(plan.part_count())
        .map(|line| {
            let (_, found) = line.split_once(": ").unwrap_or_else(|| panic!("{line}"));
            measures_and_cost(found)
        })
        .collect()
}

/// Compiles the LilyPond source of the book of a mass written to `base` into its PDF, as `fux
/// mass` does, and returns the warnings of LilyPond, which `fux mass` does not show.
fn lilypond_warnings(base: &Path) -> Vec<String> {
    let output = Command::new(fux_export::render::lilypond_command())
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .args(["-dno-point-and-click", "--pdf", "-o"])
        .arg(base)
        .arg(with_extension(base, "ly"))
        .output()
        .expect("lilypond");
    let log = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{}: {log}", base.display());
    assert!(with_extension(base, "pdf").exists(), "{}", base.display());
    log.lines()
        .filter(|line| line.to_lowercase().contains("warning"))
        .map(str::to_string)
        .collect()
}

/// The Missa brevis in the four modes of § 8 (`missa_brevis_in_four_modes`), S A T B, seed 1, in
/// its files as `a_missa_brevis_written` has the Dorian one, as written and again with
/// Mensurstriche and the tacet voices hidden: complete; the MusicXML file of every movement with
/// its fields and sections, valid against the MusicXML 4.0 schema where `FUX_MUSICXML_XSD` names
/// it, its check repeating the cost of every part; its MIDI file with its markers; the book
/// compiled by LilyPond into its PDF, where LilyPond is installed, without a warning: since E4
/// and M2, in staves of 18 pt, none of the eight books warns, neither with the notes of 14a nor
/// with those after L6, L3 (regeln.md „Notenbild und Ausgabe“), and the compressed page of the
/// Phrygian book with Mensurstriche, the warning LilyPond gave there at 20 pt, is no longer
/// tolerated (E4 (b)). The masses are set, a quarter of an hour in release (874 s on 5.11.,
/// beside the other tests of this file); where `FUX_MASS_PARTS` names a folder with the parts of
/// these masses (`missa-dorisch.teile`, …) that this program set (those of 14a fit no longer since
/// L6, L3 and the version 2 of the generator), `--resume` takes them from there, which takes some
/// minutes: `FUX_MASS_PARTS=… cargo test --release -p fux-cli --test cli -- --ignored --nocapture
/// missa_brevis_in_four_modes_written`.
#[test]
#[ignore = "release: sets four whole masses, a quarter of an hour; minutes with FUX_MASS_PARTS"]
fn missa_brevis_in_four_modes_written() {
    let given = std::env::var_os("FUX_MASS_PARTS").map(PathBuf::from);
    let lilypond = fux_export::lilypond_version().is_some();
    // The book is compiled below, where its warnings show.
    let formats = ["-f", "ly,midi,musicxml"];
    let (mut validated, mut checked) = (0, 0);
    for mode in [
        "dorisch",
        "phrygisch",
        "mixolydisch",
        "ionisch-transponiert",
    ] {
        let args = [
            "--preset", "brevis", "--mode", mode, "--parts", "S A T B", "--seed", "1",
        ];
        let plan = plan_of(&args);
        let folder = format!("messen-{mode}");
        let clock = std::time::Instant::now();
        let (base, output) = match &given {
            Some(dir) => {
                let base = copy_parts(&dir.join(format!("missa-{mode}")), &folder);
                let at = ["mass", "--resume", "-o", base.to_str().unwrap()];
                let output = fux(&[&at[..], &args, &formats].concat());
                (base, output)
            }
            None => {
                let written = set_mass(&folder, &[&args[..], &formats].concat());
                (written.base, written.output)
            }
        };
        assert_eq!(output.status.code(), Some(0), "{mode}: {output:?}");
        let parts = parts_reported(&plan, &stdout(&output));
        assert_eq!(parts.len(), 26, "{mode}");
        println!("{mode}: {:.0} s", clock.elapsed().as_secs_f64());
        let again = copy_parts(&base, &format!("{folder}-mensur"));
        let rewritten = fux(&[
            &[
                "mass",
                "--resume",
                "--mensurstriche",
                "--tacet",
                "verbergen",
            ][..],
            &["-o", again.to_str().unwrap()],
            &args,
            &formats,
        ]
        .concat());
        assert_eq!(rewritten.status.code(), Some(0), "{mode}: {rewritten:?}");
        for (written, mensur) in [(&base, false), (&again, true)] {
            assert_movements(written, &plan, &parts);
            checked += assert_checked(written, &plan, &parts).len();
            for movement in &plan.movements {
                let path = movement_path(written, movement.movement, "musicxml");
                validated += usize::from(validate_musicxml(&path));
            }
            if lilypond {
                let warnings = lilypond_warnings(written);
                assert!(
                    warnings.is_empty(),
                    "{mode}, Mensurstriche {mensur}: {warnings:?}"
                );
            }
        }
    }
    assert_eq!(checked, 48);
    if std::env::var_os("FUX_MUSICXML_XSD").is_some() {
        assert_eq!(validated, 48);
    }
}
