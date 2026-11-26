//! `bach-probe`: the runs of the feasibility study.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use fux_bach::{kern, proxy};

#[derive(Parser)]
#[command(about = "Feasibility probe for Bach fugues on a given subject")]
struct Cli {
    /// The data: `wtc/`, `wir/` and `algomus/` below this directory.
    #[arg(long, default_value = "references/datasets/bach-fugen")]
    data: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// T1: read the 48 fugues and repeat the counts of the assessment.
    Read,
    /// T2: four fugues without sixteenths against today's rules (profile jeppesen).
    Today,
    /// T3: dissonant pairs on beats against Tymoczko's chords and the estimate, with the
    /// counter-test.
    Dissonances {
        /// The table of figures for every fugue.
        #[arg(long)]
        detail: bool,
        /// Only the two fugues on which the figures are developed (I/19, II/16).
        #[arg(long)]
        development: bool,
    },
    /// The tones of the 48 fugues by figure, with estimated chords.
    Notes,
    /// The unexplained pairs of the two development fugues, with their surroundings.
    ListDevelopment {
        /// Against the estimated chords instead of Tymoczko's.
        #[arg(long)]
        own: bool,
        #[arg(long, default_value_t = 60)]
        limit: usize,
    },
    /// T4: the style bands of the 48 fugues and the yardstick.
    Bands,
    /// T6 for one fugue of WTC I: the entries stay, the rest is set anew.
    Neusatz {
        /// Number of the fugue in WTC I.
        number: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 400)]
        beam: usize,
        #[arg(long, default_value_t = 12)]
        attempts: usize,
        /// The countersubjects stay as well.
        #[arg(long)]
        countersubjects: bool,
        /// The voices rest where Bach's rest.
        #[arg(long)]
        rests: bool,
        /// Directory for the score (LilyPond and MIDI).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Every measure beside the original's.
        #[arg(long)]
        detail: bool,
    },
    /// T6 on the development fugues (I/2, I/16) or the test fugues, three seeds each.
    NeusatzAll {
        /// The six test fugues instead of the two development fugues.
        #[arg(long)]
        test: bool,
        /// These fugues instead (`2,16` or the sets of phase B1, `dev` and `test`).
        #[arg(long)]
        fugues: Option<String>,
        #[arg(long, default_value_t = 3)]
        seeds: u64,
        #[arg(long, default_value_t = 400)]
        beam: usize,
        #[arg(long, default_value_t = 12)]
        attempts: usize,
        #[arg(long)]
        countersubjects: bool,
        #[arg(long)]
        rests: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// B1, Z1: every rule counted per opportunity on the 48 fugues and on the Art of Fugue.
    B1Census {
        #[command(flatten)]
        profile: ProfileArgs,
    },
    /// B1, Z2 and Z3: the prohibitions the 48 fugues break, and search against check.
    B1Originals {
        #[command(flatten)]
        profile: ProfileArgs,
        /// One line per fugue.
        #[arg(long)]
        detail: bool,
    },
    /// B1, Z4 and Z5: the re-setting with the machine on a set of fugues of WTC I.
    B1Resetting {
        /// `dev`, `test`, or numbers of fugues (`2,16`).
        #[arg(default_value = "dev")]
        set: String,
        #[command(flatten)]
        profile: ProfileArgs,
        #[arg(long, default_value_t = 3)]
        seeds: u64,
        #[arg(long, default_value_t = 400)]
        beam: usize,
        #[arg(long, default_value_t = 12)]
        attempts: usize,
        #[arg(long)]
        out: Option<PathBuf>,
        /// The three singled-out measures of every run.
        #[arg(long)]
        detail: bool,
        /// Instead of the verdicts: the means of the measures and the findings per opportunity
        /// beside Bach's.
        #[arg(long)]
        rates: bool,
        /// Over a scaffold generated from entries, cadences and pedal points (phase B2) instead
        /// of the chords estimated from the original.
        #[arg(long)]
        scaffold: bool,
    },
    /// B1: the fugues whose rhythm leaves the grid of thirty-second notes.
    B1Cells,
    /// B1: the machine on the fugues of WTC II with only their first two entries given.
    B1Robust {
        #[command(flatten)]
        profile: ProfileArgs,
        #[arg(long, default_value_t = 400)]
        beam: usize,
    },
    /// B2: counts the model of the scaffold generator and writes it to the crate's data.
    B2Tables {
        #[arg(long, default_value = "crates/fux-bach/data/harmony.json")]
        out: PathBuf,
    },
    /// B2: the chords of the model by frequency.
    B2Vocabulary,
    /// B2, Z7 to Z9: the generated scaffolds of a set of fugues in numbers.
    B2Scaffolds {
        /// `dev`, `test`, or numbers of fugues.
        #[arg(default_value = "dev")]
        set: String,
        #[command(flatten)]
        profile: ProfileArgs,
        /// Keys and chords of every scaffold.
        #[arg(long)]
        detail: bool,
    },
    /// B3, K1 to K5: fugues on a set of subjects.
    B3Fugues {
        /// `dev`, `test`, `all`, or names of subjects (`wtk1-02,bwv1080`).
        #[arg(default_value = "dev")]
        set: String,
        /// Numbers of voices.
        #[arg(long, default_value = "3,4")]
        voices: String,
        #[command(flatten)]
        profile: ProfileArgs,
        #[command(flatten)]
        composer: ComposerArgs,
        #[arg(long, default_value_t = 1)]
        seeds: u64,
        #[arg(long, default_value_t = 400)]
        beam: usize,
        #[arg(long, default_value_t = 12)]
        attempts: usize,
        #[arg(long)]
        out: Option<PathBuf>,
        /// The plan of every fugue.
        #[arg(long)]
        detail: bool,
    },
    /// B3: the plans of Bach's fugues of WTC I as numbers, for `fux-fuge fugue --plan`.
    B3Templates {
        #[arg(long, default_value = "crates/fux-bach/data/plans.json")]
        out: PathBuf,
    },
    /// B3: the subjects of pieces of the corpus as text (`wtc2f05`, `offering-001`).
    B3Extract { names: Vec<String> },
    /// B4: the exposed moments (cadences, entries, ends of episodes) of Bach's labelled fugues
    /// and of the program's, counted alike.
    B4Moments {
        /// `dev`, `test`, `all`, `fresh` (the subjects of WTC I that do not come with the
        /// program), or names of subjects.
        #[arg(default_value = "dev")]
        set: String,
        /// Numbers of voices.
        #[arg(long, default_value = "3,4")]
        voices: String,
        #[command(flatten)]
        profile: ProfileArgs,
        #[command(flatten)]
        composer: ComposerArgs,
        #[arg(long, default_value_t = 400)]
        beam: usize,
        #[arg(long, default_value_t = 12)]
        attempts: usize,
        /// Count the final chord as a cadence, as the protocol first had it.
        #[arg(long)]
        with_close: bool,
        /// The moments of every fugue.
        #[arg(long)]
        detail: bool,
    },
    /// B5: the motivic measures on Bach's labelled fugues alone, for figures of four to six
    /// tones, with each fugue's own subject and with another's.
    B5Bach,
    /// B5: the motivic measures of the program's fugues against Bach's bands.
    B5Motifs {
        /// `dev`, `test`, `all`, `fresh`, or names of subjects.
        #[arg(default_value = "dev")]
        set: String,
        /// Numbers of voices.
        #[arg(long, default_value = "3,4")]
        voices: String,
        #[command(flatten)]
        profile: ProfileArgs,
        #[command(flatten)]
        composer: ComposerArgs,
        #[arg(long, default_value_t = 400)]
        beam: usize,
        #[arg(long, default_value_t = 12)]
        attempts: usize,
    },
    /// B7: the bass in the analyses of Bach's pieces, and what sounds lowest in his analysed
    /// fugues where a chord begins.
    B7Bach,
    /// B7: what sounds lowest in the program's fugues where a chord of the scaffold begins.
    B7Bass {
        /// `dev`, `test`, `all`, `fresh`, or names of subjects.
        #[arg(default_value = "dev")]
        set: String,
        /// Numbers of voices.
        #[arg(long, default_value = "3,4")]
        voices: String,
        #[command(flatten)]
        profile: ProfileArgs,
        #[command(flatten)]
        composer: ComposerArgs,
        #[arg(long, default_value_t = 400)]
        beam: usize,
        #[arg(long, default_value_t = 12)]
        attempts: usize,
    },
    /// T7: short fugues on the ten subjects of the protocol.
    Fugues {
        /// Only this theme (`wtk1-02`, `bwv578`, `bwv1080`, `haendel` and so on).
        #[arg(long)]
        theme: Option<String>,
        #[arg(long, default_value_t = 3)]
        seeds: u64,
        #[arg(long, default_value_t = 400)]
        beam: usize,
        #[arg(long, default_value_t = 12)]
        attempts: usize,
        #[arg(long)]
        out: Option<PathBuf>,
        /// Sections, chords and the measures outside their bands.
        #[arg(long)]
        detail: bool,
    },
    /// T8: the time of a four-voice search over 30, 60 and 100 bars.
    Timing {
        #[arg(long, default_value_t = 400)]
        beam: usize,
    },
    /// T9: pieces outside the WTC against the bands of the 48.
    Outside {
        /// Directories below the data directory.
        #[arg(default_values_t = ["kdf".to_string(), "opfer".to_string()])]
        dirs: Vec<String>,
    },
    /// Counts the tables that come with the program and writes them to its data directory.
    Tables {
        #[arg(long, default_value = "crates/fux-bach/data/tables.json")]
        out: PathBuf,
    },
    /// The ten subjects of the study as typed text.
    Themes,
    /// The Algomus labels of WTC I against the notes.
    Labels,
    /// The estimate against Tymoczko on the development fugues for several settings.
    TuneEstimate,
    /// Tymoczko's chords and the estimate side by side for some bars of a fugue.
    Chords { piece: String, from: i32, to: i32 },
    /// T5: Bach's first answers against the rule.
    Answers {
        /// Only one book (1 or 2).
        #[arg(long)]
        book: Option<usize>,
        /// Instead of the comparison, subject and answer tone by tone (of the book given, or of
        /// book 1).
        #[arg(long)]
        explore: bool,
        /// A published table of answer kinds (CSV with the columns book, number, answer_prout)
        /// to check Bach's answers as read from the notes.
        #[arg(long)]
        published: Option<PathBuf>,
    },
}

/// The profile of the machine: by name or file, with single settings on top.
#[derive(clap::Args)]
struct ProfileArgs {
    /// `bach`, `fuge` (the composer's prices since phase B6), `ausgang` (the prices of the
    /// study's prototype) or a profile file.
    #[arg(long, default_value = "bach")]
    profile: String,
    /// Single settings: `MEL-REPEAT=P2`, `wish_class=3`.
    #[arg(long = "rule")]
    rules: Vec<String>,
}

/// What the composer is asked for beside its profile.
#[derive(clap::Args)]
struct ComposerArgs {
    /// The bass: `free` (found by the search) or `planned` (the scaffold names a bass for every
    /// chord, phase B7).
    #[arg(long, default_value = "free")]
    bass: String,
}

impl ComposerArgs {
    fn planned(&self) -> Result<bool> {
        match self.bass.as_str() {
            "free" => Ok(false),
            "planned" => Ok(true),
            other => anyhow::bail!("--bass is `free` or `planned`, not {other:?}"),
        }
    }
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Read => read(&cli.data),
        Command::Dissonances {
            detail,
            development,
        } => fux_bach::study::t3(&cli.data, detail, development),
        Command::Today => fux_bach::study::t2(&cli.data),
        Command::Notes => fux_bach::study::notes(&cli.data),
        Command::ListDevelopment { own, limit } => {
            fux_bach::study::list_development(&cli.data, own, limit)
        }
        Command::Bands => fux_bach::study::t4(&cli.data),
        Command::Neusatz {
            number,
            seed,
            beam,
            attempts,
            countersubjects,
            rests,
            out,
            detail,
        } => {
            let options = fux_bach::layout::Options {
                seed,
                beam,
                attempts,
                ..Default::default()
            };
            let given = fux_bach::neusatz::Given {
                countersubjects,
                rests,
            };
            fux_bach::neusatz::one(&cli.data, number, given, &options, out.as_deref(), detail)
        }
        Command::NeusatzAll {
            test,
            fugues,
            seeds,
            beam,
            attempts,
            countersubjects,
            rests,
            out,
        } => {
            let options = fux_bach::layout::Options {
                beam,
                attempts,
                ..Default::default()
            };
            let given = fux_bach::neusatz::Given {
                countersubjects,
                rests,
            };
            let numbers: Vec<usize> = match fugues {
                Some(set) => fux_bach::b1::numbers(&set)?,
                None if test => fux_bach::neusatz::TEST.to_vec(),
                None => fux_bach::neusatz::DEVELOPMENT.to_vec(),
            };
            let seeds: Vec<u64> = (1..=seeds).collect();
            fux_bach::neusatz::many(&cli.data, &numbers, &seeds, given, &options, out.as_deref())
        }
        Command::B1Census { profile } => fux_bach::b1::census(&cli.data, &profile.load()?),
        Command::B1Originals { profile, detail } => {
            fux_bach::b1::originals(&cli.data, &profile.load()?, detail)
        }
        Command::B1Resetting {
            set,
            profile,
            seeds,
            beam,
            attempts,
            out,
            detail,
            rates,
            scaffold,
        } => {
            let options = fux_bach::draft::Options {
                seed: 1,
                beam,
                attempts,
                profile: profile.load()?,
            };
            let numbers = fux_bach::b1::numbers(&set)?;
            let seeds: Vec<u64> = (1..=seeds).collect();
            if rates {
                fux_bach::b1::rates(&cli.data, &numbers, &seeds, &options)
            } else {
                fux_bach::b1::many_over(
                    &cli.data,
                    &numbers,
                    &seeds,
                    &options,
                    out.as_deref(),
                    detail,
                    scaffold,
                )
            }
        }
        Command::B1Cells => fux_bach::b1::cell_grids(&cli.data),
        Command::B3Extract { names } => fux_bach::b3::extract(&cli.data, &names),
        Command::B5Bach => fux_bach::b5::explore(&cli.data),
        Command::B7Bach => fux_bach::b7::bach(&cli.data),
        Command::B7Bass {
            set,
            voices,
            profile,
            composer,
            beam,
            attempts,
        } => {
            let voices: Vec<usize> = voices
                .split(',')
                .filter_map(|v| v.trim().parse().ok())
                .collect();
            let options = fux_bach::compose::Options {
                beam,
                attempts,
                profile: profile.load()?,
                bass: composer.planned()?,
                ..Default::default()
            };
            fux_bach::b7::run(&cli.data, &set, &voices, &options)
        }
        Command::B5Motifs {
            set,
            voices,
            profile,
            composer,
            beam,
            attempts,
        } => {
            let voices: Vec<usize> = voices
                .split(',')
                .filter_map(|v| v.trim().parse().ok())
                .collect();
            let options = fux_bach::compose::Options {
                beam,
                attempts,
                profile: profile.load()?,
                bass: composer.planned()?,
                ..Default::default()
            };
            fux_bach::b5::run(&cli.data, &set, &voices, &options)
        }
        Command::B4Moments {
            set,
            voices,
            profile,
            composer,
            beam,
            attempts,
            with_close,
            detail,
        } => {
            let voices: Vec<usize> = voices
                .split(',')
                .filter_map(|v| v.trim().parse().ok())
                .collect();
            let options = fux_bach::compose::Options {
                beam,
                attempts,
                profile: profile.load()?,
                bass: composer.planned()?,
                ..Default::default()
            };
            fux_bach::b4::run(&cli.data, &set, &voices, &options, with_close, detail)
        }
        Command::B3Templates { out } => fux_bach::b3::templates(&cli.data, &out),
        Command::B3Fugues {
            set,
            voices,
            profile,
            composer,
            seeds,
            beam,
            attempts,
            out,
            detail,
        } => {
            let voices: Vec<usize> = voices
                .split(',')
                .filter_map(|v| v.trim().parse().ok())
                .collect();
            let options = fux_bach::compose::Options {
                beam,
                attempts,
                profile: profile.load()?,
                bass: composer.planned()?,
                ..Default::default()
            };
            let seeds: Vec<u64> = (1..=seeds).collect();
            fux_bach::b3::fugues(
                &cli.data,
                &set,
                &voices,
                &seeds,
                &options,
                out.as_deref(),
                detail,
            )
        }
        Command::B2Tables { out } => fux_bach::b2::tables(&cli.data, &out),
        Command::B2Vocabulary => fux_bach::b2::vocabulary(&cli.data),
        Command::B2Scaffolds {
            set,
            profile,
            detail,
        } => fux_bach::b2::scaffolds(
            &cli.data,
            &fux_bach::b1::numbers(&set)?,
            &profile.load()?,
            detail,
        ),
        Command::B1Robust { profile, beam } => {
            let options = fux_bach::draft::Options {
                seed: 1,
                beam,
                attempts: 12,
                profile: profile.load()?,
            };
            fux_bach::b1::robust(&cli.data, &options)
        }
        Command::Fugues {
            theme,
            seeds,
            beam,
            attempts,
            out,
            detail,
        } => {
            let options = fux_bach::layout::Options {
                beam,
                attempts,
                ..Default::default()
            };
            let seeds: Vec<u64> = (1..=seeds).collect();
            fux_bach::study_fugues::t7(
                &cli.data,
                theme.as_deref(),
                &seeds,
                &options,
                out.as_deref(),
                detail,
            )
        }
        Command::Timing { beam } => {
            let options = fux_bach::layout::Options {
                beam,
                ..Default::default()
            };
            fux_bach::study_fugues::t8(&cli.data, &options)
        }
        Command::Outside { dirs } => fux_bach::study::t9(&cli.data, &dirs),
        Command::Tables { out } => fux_bach::study::write_tables(&cli.data, &out),
        Command::Themes => fux_bach::study_fugues::print_themes(&cli.data),
        Command::Labels => fux_bach::study::labels(&cli.data),
        Command::TuneEstimate => fux_bach::study::tune_estimate(&cli.data),
        Command::Chords { piece, from, to } => {
            fux_bach::study::compare_chords(&cli.data, &piece, from, to)
        }
        Command::Answers {
            book,
            explore,
            published,
        } => {
            if explore {
                fux_bach::study_answers::explore(&cli.data, book.unwrap_or(1))
            } else {
                fux_bach::study_answers::run(&cli.data, book, published.as_deref())
            }
        }
    }
}

fn read(data: &std::path::Path) -> Result<()> {
    let pieces = kern::read_dir(&data.join("wtc"))?;
    let mut counts = kern::KernCounts::default();
    let mut har = proxy::HarProxy::default();
    let mut raw = proxy::RawMeasure::default();
    println!("file      key  meter voices  notes  bars  divided ossia chord  upbeat regular");
    for read in &pieces {
        let c = read.counts;
        let p = &read.piece;
        println!(
            "{:9} {:4} {:5} {:6} {:6} {:5} {:8} {:5} {:5} {:7} {}",
            p.name,
            p.key.to_string(),
            p.meter.to_string(),
            p.voices.len(),
            c.notes,
            c.barlines,
            c.divided,
            c.ossia,
            c.chord_tones,
            p.origin,
            read.regular
        );
        counts.spines += c.spines;
        counts.notes += c.notes;
        counts.barlines += c.barlines;
        counts.divided += c.divided;
        counts.ossia += c.ossia;
        counts.chord_tones += c.chord_tones;
        har += proxy::har_proxy(read);
        raw += proxy::raw_measure(read);
    }
    println!(
        "{} files, {} voices, {} notes, {} barlines; {} notes in second parts ({} of them in alternative readings), {} further chord tones",
        pieces.len(),
        counts.spines,
        counts.notes,
        counts.barlines,
        counts.divided,
        counts.ossia,
        counts.chord_tones
    );
    println!(
        "HAR-CONSONANCE proxy: {} violations on {} measure starts ({:.2} %), {} pairs ({:.2} %), {} measure starts with a violation ({:.2} %)",
        har.violations,
        har.downbeats,
        100.0 * har.violations as f64 / har.downbeats as f64,
        har.pairs,
        100.0 * har.violations as f64 / har.pairs as f64,
        har.downbeats_with_violation,
        100.0 * har.downbeats_with_violation as f64 / har.downbeats as f64
    );
    println!(
        "rough measure: {} beats, {} dissonant, {} with a struck dissonance; {} measure starts, {} dissonant ({:.1} %), {} struck ({:.1} %)",
        raw.beats,
        raw.beats_dissonant,
        raw.beats_struck,
        raw.downbeats,
        raw.downbeats_dissonant,
        100.0 * raw.downbeats_dissonant as f64 / raw.downbeats as f64,
        raw.downbeats_struck,
        100.0 * raw.downbeats_struck as f64 / raw.downbeats as f64
    );
    Ok(())
}
