//! `fux-fuge`: fugues in the style of Bach on a given subject.

use std::path::{Path, PathBuf};
use std::process::Command as Process;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};

use fux_bach::api;

#[derive(Parser)]
#[command(about = "Fugen im Stil Bachs zu einem gegebenen Thema")]
struct Cli {
    /// Language of the report and of the words in the score: de or en.
    #[arg(long, global = true, default_value = "de")]
    lang: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
// One value per run of the program: the size of the largest variant does not matter.
#[allow(clippy::large_enum_variant)]
enum Command {
    /// The subjects that come with the program.
    Themes,
    /// Sets a fugue on a subject and writes it as MusicXML, LilyPond and MIDI.
    Fugue(FugueArgs),
    /// Sets several versions of a fugue that differ in the aspects named and in nothing else,
    /// writes each under the path of --out with its letter (`OUT-a`, `OUT-b`, ...) and a
    /// table of them (`OUT-fassungen.md`). Every version carries in its files what it
    /// differs in.
    Variants {
        #[command(flatten)]
        fugue: FugueArgs,
        /// The aspects the versions differ in, with commas: `bass`, `sound` (the profile),
        /// `free-voices`, `seed`. Every combination of their values is a version; the values
        /// of the call itself make version a.
        #[arg(long, value_delimiter = ',', required = true)]
        vary: Vec<String>,
        /// With `seed` among the aspects: so many seeds, counted on from --seed.
        #[arg(long, default_value_t = 3)]
        seeds: u64,
        /// For a blind comparison: the letters are drawn, the files of a version say only its
        /// letter, and what the versions differ in stands in `OUT-schluessel.md` alone.
        #[arg(long)]
        blind: bool,
    },
    /// Takes one of the versions that `variants` wrote: its files are copied to the path of
    /// --out without a letter, and the call that sets it is shown.
    Choose {
        /// The path given to `variants --out`.
        out: PathBuf,
        /// The letter of the version.
        letter: char,
    },
    /// Checks a piece against given chords and reports what the rules find (exit status 2 if a
    /// prohibition is broken).
    Check {
        /// The piece: kern (.krn) or MusicXML (.musicxml, .xml).
        file: PathBuf,
        /// The chords: RomanText, or chord lines bar by bar (`1: Dm A7 | 2: Dm`). Without them
        /// the chords are estimated from the notes.
        #[arg(long)]
        chords: Option<PathBuf>,
        #[command(flatten)]
        profile: ProfileArgs,
        /// The report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// The rules with their levels in a profile.
    Rules {
        #[command(flatten)]
        profile: ProfileArgs,
        /// The profile as a file to edit and pass to --profile.
        #[arg(long)]
        file: bool,
    },
}

/// The subject and the options of a fugue, as `fugue` and `variants` take them.
#[derive(clap::Args)]
struct FugueArgs {
    /// One of the subjects that come with the program (see `themes`).
    #[arg(long, conflicts_with = "subject")]
    theme: Option<String>,
    /// The subject's tones: Dutch note names with octave marks and durations as in LilyPond,
    /// `r` for rests, `~` for ties; tick 0 is the barline before the first tone.
    #[arg(long)]
    subject: Option<String>,
    /// The subject from a file instead: kern or MusicXML, of which the voice that enters
    /// first is read, up to --period-bars (or to its first rest of a beat). Key and meter
    /// come from the file unless given.
    #[arg(long, conflicts_with_all = ["theme", "subject"])]
    subject_file: Option<PathBuf>,
    /// The key: a letter with # or b, upper case for major (C, f#, Bb).
    #[arg(long)]
    key: Option<String>,
    /// The meter (default 4/4, or that of --subject-file).
    #[arg(long)]
    meter: Option<String>,
    /// Bars from one entry of the exposition to the next; by default the subject's length.
    #[arg(long)]
    period_bars: Option<f64>,
    /// Two to four voices.
    #[arg(long, default_value_t = 3)]
    voices: u64,
    /// The end: `auto` (a stretto if the subject gives one, else a pedal point), `pedal`
    /// or `stretto`.
    #[arg(long, default_value = "auto")]
    ending: String,
    /// The entries after the exposition as in a fugue of WTC I (`wtk1-02`; see `themes`)
    /// instead of the planner's.
    #[arg(long)]
    plan: Option<String>,
    /// Without the sequences in all voices and without the figures of the free voices: one
    /// setting instead of two, on average in a third to a fifth of the time.
    #[arg(long)]
    plain: bool,
    /// Your own answer instead of the rule's: tones as for --subject, in the rhythm of the
    /// subject.
    #[arg(long)]
    answer: Option<String>,
    /// The bass: `free` (found by the search) or `planned` (the scaffold names a bass for
    /// every chord, drawn from what the analyses of Bach's fugues say).
    #[arg(long, default_value = "free")]
    bass: String,
    /// The profile of the rules: `fuge` (the composer's prices after blind listening tests),
    /// `bach` (the prices calibrated on Bach's rates) or a profile file.
    #[arg(long, default_value = "fuge")]
    profile: String,
    /// Single settings of the rules: `MEL-REPEAT=P2`, `wish_class=6`.
    #[arg(long = "rule")]
    rules: Vec<String>,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[arg(long, default_value_t = 400)]
    beam: u64,
    /// Where to write: this path with the endings .musicxml, .ly, .mid and .akkorde.txt (the
    /// chords of the scaffold, for `check --chords`).
    #[arg(long, default_value = "out/fuge")]
    out: PathBuf,
    /// Also a PDF, engraved by LilyPond.
    #[arg(long)]
    pdf: bool,
    /// Also a MuseScore file, converted from the MusicXML by MuseScore.
    #[arg(long)]
    mscz: bool,
}

impl FugueArgs {
    /// The request for the fugue, as the interface takes it.
    fn request(&self, lang: &str) -> Result<Value> {
        let mut request = json!({
            "command": "fugue", "seed": self.seed, "beam": self.beam, "lang": lang,
            "voices": self.voices, "ending": self.ending, "profile": self.profile,
            "rules": self.rules, "bass": self.bass,
            "free_voices": if self.plain { "plain" } else { "figures" },
        });
        let (mut subject, mut key, mut meter) =
            (self.subject.clone(), self.key.clone(), self.meter.clone());
        if let Some(file) = &self.subject_file {
            let read = subject_from(file, self.period_bars)?;
            subject = Some(read.0);
            key = key.or(Some(read.1));
            meter = meter.or(Some(read.2));
        }
        request["meter"] = json!(meter.unwrap_or("4/4".to_string()));
        if let Some(plan) = &self.plan {
            request["plan"] = json!(plan);
        }
        if let Some(reply) = &self.answer {
            request["answer"] = json!(reply);
        }
        if let Some(theme) = &self.theme {
            request["theme"] = json!(theme);
        }
        if let Some(subject) = subject {
            request["subject"] = json!(subject);
            request["key"] = json!(key.context("--subject needs --key")?);
        }
        if let Some(bars) = self.period_bars {
            request["period_bars"] = json!(bars);
        }
        Ok(request)
    }
}

/// The profile of the rules: by name or file, with single settings on top.
#[derive(clap::Args)]
struct ProfileArgs {
    /// `bach`, `fuge` (the composer's prices), `ausgang` (the prices before the calibration) or
    /// a profile file.
    #[arg(long, default_value = "bach")]
    profile: String,
    /// Single settings: `MEL-REPEAT=P2`, `FIG-ANTICIPATION=V`, `wish_class=3`.
    #[arg(long = "rule")]
    rules: Vec<String>,
}

impl ProfileArgs {
    fn load(&self) -> Result<fux_bach::rules::Profile> {
        let mut profile = fux_bach::rules::Profile::named(&self.profile)?;
        for setting in &self.rules {
            profile.set_text(setting)?;
        }
        Ok(profile)
    }
}

/// A piece from a kern or MusicXML file.
fn read_piece(file: &Path) -> Result<fux_bach::score::Piece> {
    let text =
        String::from_utf8_lossy(&std::fs::read(file).with_context(|| file.display().to_string())?)
            .into_owned();
    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("satz");
    match file.extension().and_then(|e| e.to_str()) {
        Some("krn") => Ok(fux_bach::kern::read(stem, &text)?.piece),
        Some("musicxml" | "xml") => {
            let (piece, remarks) = fux_bach::musicxml_read::read(stem, &text)?;
            if remarks.chord_tones > 0 {
                eprintln!(
                    "{}: {} tones of chords inside a voice were left out",
                    file.display(),
                    remarks.chord_tones
                );
            }
            Ok(piece)
        }
        _ => bail!(
            "{}: kern (.krn) and MusicXML (.musicxml, .xml) are read",
            file.display()
        ),
    }
}

/// A subject from a kern or MusicXML file: the tones of the voice that enters first, from the
/// bar line before its first tone up to `bars` bars, or to its first rest of a beat or more.
/// With them the key and the meter of the file.
fn subject_from(file: &Path, bars: Option<f64>) -> Result<(String, String, String)> {
    let piece = read_piece(file)?;
    let voice = piece
        .voices
        .iter()
        .filter(|v| !v.notes.is_empty())
        .min_by_key(|v| v.notes[0].on)
        .context("the file has no notes")?;
    let first = voice.notes[0].on;
    let start = first - piece.pos(first);
    let until = bars.map(|b| start + (b * piece.meter.bar() as f64).round() as i32);
    let mut notes = Vec::new();
    for (i, note) in voice.notes.iter().enumerate() {
        let gap = i > 0 && note.on - voice.notes[i - 1].off >= piece.meter.beat();
        match until {
            // The tone that begins where the subject ends still belongs to it.
            Some(until) if note.on > until => break,
            None if gap => break,
            _ => {}
        }
        notes.push(fux_bach::score::Note {
            on: note.on - start,
            off: note.off - start,
            pitch: note.pitch,
        });
    }
    if notes.len() < 3 {
        bail!("{}: fewer than three tones of a subject", file.display());
    }
    Ok((
        fux_bach::themes::text(&notes),
        piece.key.to_string(),
        piece.meter.to_string(),
    ))
}

/// The chords of a file: RomanText (lines `m1 …`) or chord lines (`1: Dm A7`).
fn read_chords(file: &Path, piece: &fux_bach::score::Piece) -> Result<fux_bach::key::Harmony> {
    let text = std::fs::read_to_string(file).with_context(|| file.display().to_string())?;
    let roman = text.lines().any(|l| {
        let mut chars = l.trim_start().chars();
        chars.next() == Some('m') && chars.next().is_some_and(|c| c.is_ascii_digit())
    });
    if roman {
        let analysis = fux_bach::roman::parse(&text)?;
        Ok(fux_bach::roman::harmony(&analysis, piece))
    } else {
        fux_bach::chordline::parse(&text, piece)
    }
}

fn check(
    file: &Path,
    chords: Option<&Path>,
    profile: &fux_bach::rules::Profile,
    as_json: bool,
    english: bool,
) -> Result<usize> {
    let piece = read_piece(file)?;
    let harmony = match chords {
        Some(path) => read_chords(path, &piece)?,
        None => {
            if !as_json {
                println!(
                    "{}",
                    if english {
                        "No chords given: estimated from the notes, a makeshift (--chords)."
                    } else {
                        "Keine Akkorde gegeben: aus den Noten geschätzt, ein Notbehelf (--chords)."
                    }
                );
            }
            fux_bach::chord::estimate(&piece)
        }
    };
    let report = fux_bach::check::check(&piece, &harmony, profile);
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&fux_bach::check::json(&report, &piece, english))?
        );
    } else {
        for line in fux_bach::check::lines(&report, &piece, english) {
            println!("{line}");
        }
        println!("{}", fux_bach::check::summary(&report, english));
    }
    Ok(report.forbidden())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let english = cli.lang == "en";
    match cli.command {
        Command::Themes => {
            let answer = api::answer(r#"{"command": "themes"}"#)?;
            let plans: Vec<&str> = answer["plans"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|p| p.as_str())
                .collect();
            for theme in answer["themes"].as_array().into_iter().flatten() {
                println!(
                    "{:9} {:4} {:5} {}\n          {}",
                    theme["name"].as_str().unwrap_or(""),
                    theme["key"].as_str().unwrap_or(""),
                    theme["meter"].as_str().unwrap_or(""),
                    theme["title"].as_str().unwrap_or(""),
                    theme["subject"].as_str().unwrap_or("")
                );
            }
            println!(
                "{}: {}",
                if english {
                    "Plans for --plan"
                } else {
                    "Pläne für --plan"
                },
                plans.join(", ")
            );
            Ok(())
        }
        Command::Check {
            file,
            chords,
            profile,
            json,
        } => {
            let forbidden = check(&file, chords.as_deref(), &profile.load()?, json, english)?;
            if forbidden > 0 {
                std::process::exit(2);
            }
            Ok(())
        }
        Command::Rules { profile, file } => {
            let profile = profile.load()?;
            if file {
                print!("{}", profile.text());
            } else {
                for rule in fux_bach::rules::Rule::ALL {
                    println!(
                        "{:22} {:6} {}",
                        rule.id(),
                        profile.level(*rule).to_string(),
                        rule.message(english)
                    );
                }
            }
            Ok(())
        }
        Command::Fugue(fugue) => {
            let request = fugue.request(&cli.lang)?;
            let result = api::answer(&request.to_string())?;
            report(&result, english);
            write(&result, &fugue.out, fugue.pdf, fugue.mscz)
        }
        Command::Variants {
            fugue,
            vary,
            seeds,
            blind,
        } => variants(&fugue, &cli.lang, &vary, seeds, blind),
        Command::Choose { out, letter } => choose(&out, letter),
    }
}

/// The path of a version: the path of --out with the letter of the version.
fn lettered(out: &Path, letter: char) -> PathBuf {
    PathBuf::from(format!("{}-{letter}", out.display()))
}

/// Sets the versions of a fugue, writes them and the table of them.
fn variants(fugue: &FugueArgs, lang: &str, vary: &[String], seeds: u64, blind: bool) -> Result<()> {
    let english = lang == "en";
    let request = fugue.request(lang)?;
    let list = fux_bach::variants::versions(&request, vary, seeds, english, blind)?;
    let aspects = fux_bach::variants::aspects(&list);
    let word = |de: &'static str, en: &'static str| if english { en } else { de };
    // The table: a row per version with what it differs in and what came out.
    let mut head = vec![word("Fassung", "Version").to_string()];
    head.extend(aspects.iter().cloned());
    for name in [
        word("Takte", "Bars"),
        word("Töne", "Tones"),
        word("Kosten", "Cost"),
        word("Kennzahlen im Band", "Measures in band"),
        word("Zeit", "Time"),
    ] {
        head.push(name.to_string());
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut title = String::new();
    let mut stored: Vec<Value> = Vec::new();
    for version in &list {
        let mut request = version.request.clone();
        request["variant"] = json!(version.name(english, blind));
        let path = lettered(&fugue.out, version.letter);
        println!("{}", version.name(english, blind));
        let mut row = vec![version.letter.to_string()];
        row.extend(version.differs.iter().map(|(_, value)| value.clone()));
        match api::answer(&request.to_string()) {
            Ok(result) => {
                write(&result, &path, fugue.pdf, fugue.mscz)?;
                title = result["title"].as_str().unwrap_or("").to_string();
                row.push(result["bars"].to_string());
                row.push(result["tones"].to_string());
                row.push(result["check"]["cost"].to_string());
                row.push(result["hits"].to_string());
                let seconds = format!("{:.1} s", result["seconds"].as_f64().unwrap_or(0.0));
                row.push(if english {
                    seconds
                } else {
                    seconds.replace('.', ",")
                });
            }
            Err(error) => {
                println!("  {error:#}");
                row.push(format!(
                    "{} ({error:#})",
                    word("nicht entstanden", "did not come out")
                ));
                row.extend(vec![String::new(); 4]);
            }
        }
        rows.push(row);
        stored.push(json!({
            "letter": version.letter.to_string(),
            "name": version.name(english, false),
            "request": version.request,
            "call": fux_bach::variants::command_line(&version.request),
        }));
    }
    let name = |path: &Path| -> String {
        path.file_name()
            .map_or(String::new(), |n| n.to_string_lossy().into_owned())
    };
    // The columns of the numbers stand to the right: the last five of the full table.
    let table = |rows: &[Vec<String>], head: &[String]| -> String {
        let numbers = if head.len() > 1 { 5 } else { 0 };
        let mut text = format!(
            "| {} |\n|{}{}\n",
            head.join(" | "),
            "---|".repeat(head.len() - numbers),
            "---:|".repeat(numbers)
        );
        for row in rows {
            text.push_str(&format!("| {} |\n", row.join(" | ")));
        }
        text
    };
    let differ = if english {
        format!(
            "The versions differ in: {}. Everything else is the same.",
            aspects.join(", ")
        )
    } else {
        format!(
            "Die Fassungen unterscheiden sich in: {}. Alles andere ist gleich.",
            aspects.join(", ")
        )
    };
    let calls: String = stored
        .iter()
        .map(|v| {
            format!(
                "- {}: `{}`\n",
                v["letter"].as_str().unwrap_or(""),
                v["call"].as_str().unwrap_or("")
            )
        })
        .collect();
    let stem = name(&fugue.out);
    let take = if english {
        format!(
            "To take one: `fux-fuge choose {} b` copies the files of version b to `{stem}.*`.",
            fugue.out.display()
        )
    } else {
        format!(
            "Eine nehmen: `fux-fuge choose {} b` kopiert die Dateien der Fassung b nach `{stem}.*`.",
            fugue.out.display()
        )
    };
    let files = if english {
        format!("Files: `{stem}-a.*`, `{stem}-b.*` and so on.")
    } else {
        format!("Dateien: `{stem}-a.*`, `{stem}-b.*` und so weiter.")
    };
    let heading = format!("# {}: {title}", word("Fassungen", "Versions"));
    let again = word(
        "Dieselben Fassungen einzeln setzen:",
        "The calls that set the versions one by one:",
    );
    let full = format!(
        "{heading}\n\n{differ}\n\n{}\n{files}\n\n{take}\n\n{again}\n\n{calls}",
        table(&rows, &head)
    );
    let path = |ending: &str| PathBuf::from(format!("{}-{ending}", fugue.out.display()));
    if blind {
        // What the versions differ in stands in the key alone.
        let letters: Vec<Vec<String>> = rows.iter().map(|row| vec![row[0].clone()]).collect();
        let hidden = word(
            "Blind: Worin sich die Fassungen unterscheiden, steht im Schlüssel. Erst hören, dann lesen.",
            "Blind: what the versions differ in stands in the key. Listen first, then read.",
        );
        let key = name(&path("schluessel.md"));
        std::fs::write(
            path("fassungen.md"),
            format!(
                "{heading}\n\n{hidden}\n\n{}\n{files}\n\n{}: `{key}`\n\n{take}\n",
                table(&letters, &head[..1]),
                word("Schlüssel", "Key")
            ),
        )?;
        std::fs::write(path("schluessel.md"), &full)?;
        println!("{}", path("schluessel.md").display());
    } else {
        std::fs::write(path("fassungen.md"), &full)?;
        println!();
        println!("{differ}");
        print!("{}", table(&rows, &head));
    }
    std::fs::write(
        path("fassungen.json"),
        serde_json::to_string_pretty(&json!({ "title": title, "versions": stored }))?,
    )?;
    println!("{}", path("fassungen.md").display());
    Ok(())
}

/// Takes one of the versions that `variants` wrote under `out`: copies its files to the path
/// without a letter and says what it is and how it is set.
fn choose(out: &Path, letter: char) -> Result<()> {
    let list = PathBuf::from(format!("{}-fassungen.json", out.display()));
    let text = std::fs::read_to_string(&list)
        .with_context(|| format!("{}: no versions here (see `variants`)", list.display()))?;
    let stored: Value = serde_json::from_str(&text)?;
    let version = stored["versions"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|v| v["letter"].as_str() == Some(letter.to_string().as_str()))
        .with_context(|| format!("{}: no version {letter}", list.display()))?;
    let from = lettered(out, letter);
    let mut copied = 0;
    for ending in ["musicxml", "ly", "mid", "akkorde.txt", "pdf", "mscz"] {
        let source = PathBuf::from(format!("{}.{ending}", from.display()));
        if source.is_file() {
            let target = PathBuf::from(format!("{}.{ending}", out.display()));
            std::fs::copy(&source, &target).with_context(|| source.display().to_string())?;
            println!("{}", target.display());
            copied += 1;
        }
    }
    if copied == 0 {
        bail!(
            "{}.*: the files of version {letter} are gone",
            from.display()
        );
    }
    println!("{}", version["name"].as_str().unwrap_or(""));
    println!("{}", version["call"].as_str().unwrap_or(""));
    Ok(())
}

fn report(answer: &Value, english: bool) {
    let text = |key: &str| answer[key].as_str().unwrap_or("").to_string();
    println!("{}", text("title"));
    println!(
        "{} {}, {} {}, {} {}, {} {} ({} {}), {:.1} s",
        text("key"),
        text("meter"),
        answer["voices"],
        if english { "voices" } else { "Stimmen" },
        answer["bars"],
        if english { "bars" } else { "Takte" },
        answer["tones"],
        if english { "tones" } else { "Töne" },
        answer["fixed_tones"],
        if english { "given" } else { "vorgegeben" },
        answer["seconds"].as_f64().unwrap_or(0.0)
    );
    println!(
        "{}: {} ({})",
        if english { "Answer" } else { "Antwort" },
        answer["answer"]["text"].as_str().unwrap_or(""),
        match (answer["answer"]["kind"].as_str(), english) {
            (Some("tonal"), _) => "tonal",
            (_, _) => "real",
        }
    );
    let sections: Vec<String> = answer["sections"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| format!("{} {}", s["bar"], s["name"].as_str().unwrap_or("")))
        .collect();
    println!(
        "{}: {}",
        if english { "Bars" } else { "Takte" },
        sections.join(", ")
    );
    println!(
        "{}: {}{}; {} {} {} {}",
        if english { "End" } else { "Schluss" },
        match (answer["ending"].as_str(), english) {
            (Some("stretto"), true) => "stretto",
            (Some("stretto"), false) => "Engführung",
            (_, true) => "pedal point",
            (_, false) => "Orgelpunkt",
        },
        answer["stretto_bars"]
            .as_f64()
            .map_or(String::new(), |b| if english {
                format!(" at {b} bars")
            } else {
                format!(" im Abstand von {b} Takten")
            }),
        answer["entries"]["found"],
        if english { "of" } else { "von" },
        answer["entries"]["planned"],
        if english {
            "entries found again"
        } else {
            "Einsätzen wiedergefunden"
        }
    );
    println!("{}", answer["check"]["summary"].as_str().unwrap_or(""));
    let outside: Vec<&str> = answer["measures"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["inside"] == false)
        .filter_map(|m| m["name"].as_str())
        .collect();
    println!(
        "{}: {} {} {} ({} {})",
        if english {
            "Style measures inside Bach's bands"
        } else {
            "Kennzahlen in Bachs Bändern"
        },
        answer["hits"],
        if english { "of" } else { "von" },
        answer["measures"].as_array().map_or(0, Vec::len),
        if english { "yardstick" } else { "Maßstab" },
        answer["yardstick"]
    );
    if !outside.is_empty() {
        println!(
            "{}: {}",
            if english { "Outside" } else { "Außerhalb" },
            outside.join("; ")
        );
    }
}

fn write(answer: &Value, out: &Path, pdf: bool, mscz: bool) -> Result<()> {
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| dir.display().to_string())?;
    }
    let path = |ending: &str| PathBuf::from(format!("{}.{ending}", out.display()));
    let files = &answer["files"];
    std::fs::write(path("musicxml"), files["musicxml"].as_str().unwrap_or(""))?;
    std::fs::write(path("ly"), files["lilypond"].as_str().unwrap_or(""))?;
    let midi = files["midi"].as_str().unwrap_or("");
    std::fs::write(path("mid"), decode(midi)?)?;
    std::fs::write(path("akkorde.txt"), files["chords"].as_str().unwrap_or(""))?;
    let mut written = vec![
        path("musicxml"),
        path("ly"),
        path("mid"),
        path("akkorde.txt"),
    ];
    if pdf {
        // Without the links from every note to the source: the file is a quarter the size.
        let status = Process::new("lilypond")
            .arg("-s")
            .arg("-dno-point-and-click")
            .arg("-o")
            .arg(out)
            .arg(path("ly"))
            .status()
            .context("lilypond is not installed")?;
        if !status.success() {
            bail!("lilypond failed on {}", path("ly").display());
        }
        written.push(path("pdf"));
    }
    if mscz {
        let status = Process::new("mscore")
            .env("QT_QPA_PLATFORM", "offscreen")
            .arg("-o")
            .arg(path("mscz"))
            .arg(path("musicxml"))
            .status()
            .context("MuseScore (mscore) is not installed")?;
        if !status.success() {
            bail!("MuseScore failed on {}", path("musicxml").display());
        }
        written.push(path("mscz"));
    }
    for file in written {
        println!("{}", file.display());
    }
    Ok(())
}

/// The bytes of base64 text.
fn decode(text: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut bits = 0u32;
    let mut count = 0;
    for c in text.bytes() {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => bail!("not base64"),
        };
        bits = bits << 6 | value as u32;
        count += 1;
        if count == 4 {
            out.extend_from_slice(&[(bits >> 16) as u8, (bits >> 8) as u8, bits as u8]);
            bits = 0;
            count = 0;
        }
    }
    match count {
        2 => out.push((bits >> 4) as u8),
        3 => out.extend_from_slice(&[(bits >> 10) as u8, (bits >> 2) as u8]),
        _ => {}
    }
    Ok(out)
}
