//! `fux`: generates cantus firmi and counterpoint after Fux, checks scores and writes them as
//! LilyPond, PDF, SVG, PNG and MIDI.

use std::io::{self, Write};

use anyhow::Result;
use clap::{Parser, Subcommand};
use fux_cli::free::SingingArgs;
use fux_cli::imitation::{ImitationOptions, ImitationSource};
use fux_cli::input::{self, ScoreInput};
use fux_cli::mass::MassOptions;
use fux_cli::output::OutputArgs;
use fux_cli::rule_args::RuleArgs;
use fux_cli::{commands, output};
use fux_core::text::Eleison;
use fux_core::{Lang, ModeSpec, Step, VoiceType};

/// Kontrapunkt nach Fux: Cantus firmi erzeugen, Gegenstimmen setzen, Sätze prüfen und als
/// LilyPond, PDF und MIDI ausgeben.
#[derive(Parser, Debug)]
#[command(name = "fux", version)]
struct Cli {
    /// Sprache der Meldungen und Namen: de oder en
    #[arg(long, global = true, default_value = "de")]
    lang: Lang,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Erzeugt Cantus firmi in einem Modus
    Cf {
        /// Modus, z. B. dorisch, hypophrygisch, lydisch-transponiert
        #[arg(long, default_value = "dorisch")]
        mode: ModeSpec,
        /// Anzahl der Töne
        #[arg(long, default_value_t = 11)]
        length: usize,
        /// Stimmlage: S, A, T, B, Mz, Bar oder S+
        #[arg(long, default_value = "A")]
        voice: VoiceType,
        /// Startwert des Zufalls; gleicher Wert, gleiches Ergebnis (sonst zufällig)
        #[arg(long)]
        seed: Option<u64>,
        /// Anzahl verschiedener Cantus, beste zuerst
        #[arg(long, default_value_t = 1)]
        count: usize,
        /// Breite der Suche; größer ist langsamer und findet günstigere Lösungen
        #[arg(long, default_value_t = 300)]
        beam: usize,
        /// Zufallsanteil bei der Auswahl (0 = immer die günstigste Lösung)
        #[arg(long, default_value_t = 3.0)]
        noise: f64,
        /// Regelverstöße jedes Cantus anzeigen
        #[arg(long)]
        details: bool,
        #[command(flatten)]
        rules: RuleArgs,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Setzt Gegenstimmen zu einem Cantus firmus (ohne --cf wird einer erzeugt)
    Cp {
        /// Cantus firmus: Notennamen, LilyPond, fux:NAME, GABC- oder MusicXML-Datei
        #[arg(long, value_name = "NOTEN")]
        cf: Option<String>,
        /// Modus (sonst aus dem Cantus); auch für einen erzeugten Cantus
        #[arg(long)]
        mode: Option<ModeSpec>,
        /// Stimmlage des Cantus (sonst nach Tonumfang, beim Erzeugen Alt)
        #[arg(long, value_name = "TYP")]
        cf_voice: Option<VoiceType>,
        /// Binnenschlüsse: auto (bei GABC an Halb- und Ganzstrichen), full (nur Ganzstriche), all
        /// (auch Viertelstriche), none, oder Takte wie 8,15
        #[arg(long, value_name = "WO", default_value = "auto")]
        cadences: input::Cadences,
        /// Länge eines erzeugten Cantus
        #[arg(long, default_value_t = 11)]
        length: usize,
        /// Gattung der Gegenstimme (bei mehr Stimmen: der Gattungsstimme; die übrigen setzen Ganze);
        /// 0 für freien Satz: alle Gegenstimmen frei im Stil Palestrinas
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(0..=5))]
        species: u8,
        /// Anzahl der Stimmen mit dem Cantus (2–6)
        #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(2..=6))]
        voices: u8,
        /// Besetzung von oben nach unten, z. B. "S A T B" (sonst eine übliche Besetzung mit der Lage des Cantus)
        #[arg(long, value_name = "STIMMEN")]
        parts: Option<String>,
        /// Zweistimmig: Gegenstimme unter den Cantus statt darüber
        #[arg(long)]
        below: bool,
        /// Stimmlage der Gegenstimme bzw. der Gattungsstimme (sonst die Nachbarstimme bzw. die oberste)
        #[arg(long, value_name = "TYP")]
        voice: Option<VoiceType>,
        /// Text der freien Stimmen (nur mit --species 0): Abschnitt des Ordinariums (kyrie,
        /// christe, gloria:1-5, agnus …), eigener lateinischer Text in Anführungszeichen oder
        /// none; der Cantus behält seinen eigenen Text oder bleibt ohne
        #[arg(long, value_name = "TEXT", default_value = "none")]
        text: String,
        #[command(flatten)]
        singing: SingingArgs,
        /// Startwert des Zufalls (sonst zufällig)
        #[arg(long)]
        seed: Option<u64>,
        /// Anzahl verschiedener Lösungen, beste zuerst
        #[arg(long, default_value_t = 1)]
        count: usize,
        /// Breite der Suche
        #[arg(long, default_value_t = 200)]
        beam: usize,
        /// Zufallsanteil bei der Auswahl (0 = immer die günstigste Lösung)
        #[arg(long, default_value_t = 2.0)]
        noise: f64,
        /// Regelverstöße jeder Lösung anzeigen
        #[arg(long)]
        details: bool,
        #[command(flatten)]
        rules: RuleArgs,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Freier Satz ohne Cantus firmus: alle Stimmen frei im Stil Palestrinas, mit Kadenzplan und
    /// Text (Standard: Kyrie I), mit drei bis fünf Stimmen in Imitation
    Free {
        /// Modus, z. B. dorisch, hypophrygisch, lydisch-transponiert (sonst dorisch, mit
        /// --subject chant der Modus des Chorals)
        #[arg(long)]
        mode: Option<ModeSpec>,
        /// Anzahl der Stimmen (2–6)
        #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u8).range(2..=6))]
        voices: u8,
        /// Besetzung von oben nach unten, z. B. "S A T B" (sonst S T, A T B, S A T B, S A T T B
        /// bzw. S S A T T B)
        #[arg(long, value_name = "STIMMEN")]
        parts: Option<String>,
        /// Abschnitt des Ordinariums (kyrie, christe, gloria:1-5, credo, sanctus, benedictus,
        /// agnus …), eigener lateinischer Text in Anführungszeichen oder none
        #[arg(long, value_name = "TEXT", default_value = fux_cli::text::DEFAULT_TEXT)]
        text: String,
        /// Anzahl der Takte oder auto (aus dem Text; ohne Text 12)
        #[arg(long, value_name = "TAKTE", default_value = "auto")]
        length: fux_cli::free::Length,
        /// Binnenschlüsse: auto (mit Text an Enden von Sinnabschnitten, sonst etwa alle sechs
        /// Takte, auf den Kadenzstufen des Modus), none, oder Takte, in denen das Ziel erreicht
        /// wird, wie 7,12 oder mit Ziel 7:A,12:F
        #[arg(long, value_name = "WO", default_value = "auto")]
        cadences: fux_cli::free::CadencePlan,
        /// Schlussstufe: die Finalis (Standard) oder die Quinte oder Quarte, wo der Modus auf ihr
        /// eine Klausel hat (dorisch D, A oder G), wie ein Teil einer Messe
        #[arg(long, value_name = "STUFE", value_parser = fux_cli::free::degree)]
        ending: Option<Step>,
        #[command(flatten)]
        singing: SingingArgs,
        #[command(flatten)]
        imitation: ImitationOptions,
        /// Startwert des Zufalls (sonst zufällig)
        #[arg(long)]
        seed: Option<u64>,
        /// Anzahl verschiedener Lösungen, beste zuerst
        #[arg(long, default_value_t = 1)]
        count: usize,
        /// Breite der Suche
        #[arg(long, default_value_t = 200)]
        beam: usize,
        /// Zufallsanteil bei der Auswahl (0 = immer die günstigste Lösung)
        #[arg(long, default_value_t = 2.0)]
        noise: f64,
        /// Regelverstöße jeder Lösung anzeigen
        #[arg(long)]
        details: bool,
        #[command(flatten)]
        rules: RuleArgs,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Messe nach Palestrinas Satzformen: Kyrie, Gloria, Credo, Sanctus, Benedictus und Agnus Dei
    /// mit Text, Teil für Teil gesetzt (Standard: Missa brevis, dorisch, S A T B; --plan zeigt den
    /// Plan ohne Suche)
    Mass {
        #[command(flatten)]
        options: MassOptions,
        #[command(flatten)]
        rules: RuleArgs,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Übungsheft: Sätze in allen Modi und Gattungen in einem Dokument
    Book {
        /// Modi, kommagetrennt, oder „alle“ für alle zwölf (sonst die sechs Modi von Fux' Cantus)
        #[arg(long, value_delimiter = ',', value_name = "MODI")]
        modes: Vec<String>,
        /// Gattungen, kommagetrennt
        #[arg(
            long,
            value_delimiter = ',',
            default_value = "1,2,3,4,5",
            value_parser = clap::value_parser!(u8).range(1..=5)
        )]
        species: Vec<u8>,
        /// Stimmenzahlen, kommagetrennt; zweistimmig je eine Gegenstimme oben und unten
        #[arg(
            long,
            value_delimiter = ',',
            default_value = "2",
            value_parser = clap::value_parser!(u8).range(2..=6)
        )]
        voices: Vec<u8>,
        /// Für jeden Modus einen Cantus erzeugen, statt Fux' Cantus zu nehmen
        #[arg(long)]
        generate: bool,
        /// Länge erzeugter Cantus
        #[arg(long, default_value_t = 11)]
        length: usize,
        /// Vor den Lösungen die Aufgaben: der Cantus mit leeren Systemen
        #[arg(long)]
        exercises: bool,
        /// Startwert des Zufalls (sonst zufällig)
        #[arg(long)]
        seed: Option<u64>,
        /// Breite der Suche
        #[arg(long, default_value_t = 200)]
        beam: usize,
        /// Zufallsanteil bei der Auswahl (0 = immer die günstigste Lösung)
        #[arg(long, default_value_t = 2.0)]
        noise: f64,
        #[command(flatten)]
        rules: RuleArgs,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Zeigt einen Text mit Silben, Betonungen und Sinnabschnitten oder listet die Abschnitte des
    /// Ordinariums
    Text {
        /// Abschnitt des Ordinariums (kyrie, gloria:1-5 …) oder eigener lateinischer Text; ohne
        /// Angabe die Liste der Abschnitte
        #[arg(value_name = "TEXT")]
        text: Vec<String>,
        /// Die Abschnitte mit Silben, Sinnabschnitten und automatischer Länge auflisten
        #[arg(long, conflicts_with = "text")]
        list: bool,
        /// 4 Silben (Bücher) oder 3 (e-léi-son, Palestrina nach Jeppesen)
        #[arg(long, value_name = "4|3", default_value = "4", value_parser = fux_cli::free::eleison)]
        eleison: Eleison,
    },
    /// Prüft einen Satz und meldet Regelverstöße (Rückgabewert 2 bei Verboten)
    Check {
        #[command(flatten)]
        input: ScoreInput,
        /// Text der Stimmen mit Liedtext (MusicXML): Abschnitt des Ordinariums (kyrie, gloria:1-5
        /// …), eigener lateinischer Text oder none (ohne Textregeln); sonst der Text, den eine
        /// Datei von fux-kontrapunkt nennt, oder der erkannte Abschnitt oder Bereich
        #[arg(long, value_name = "TEXT")]
        text: Option<String>,
        /// Textwiederholung wie beim Erzeugen des Satzes: auto, keine, frei
        #[arg(long, value_name = "WIE", default_value = "auto")]
        repeat: fux_cli::free::Repeat,
        /// Imitation: datei (der Plan, den eine Datei von fux-kontrapunkt nennt, mit den
        /// IMI-Regeln), erkennen (die Punkte, die die Erkennung findet, nur zur Information) oder
        /// aus
        #[arg(long, value_name = "WOHER", default_value = "datei")]
        imitation: ImitationSource,
        #[command(flatten)]
        rules: RuleArgs,
        /// Zusätzlich den Satz mit markierten Verstößen ausgeben (siehe --output, --format)
        #[arg(long)]
        mark: bool,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Setzt einen Satz als LilyPond, PDF, SVG, PNG oder MIDI
    Render {
        #[command(flatten)]
        input: ScoreInput,
        #[command(flatten)]
        output: OutputArgs,
    },
    /// Zeigt die Regeln mit ihren Einstellungen
    Rules {
        /// Nur diese Regel ausführlich zeigen
        #[arg(long, value_name = "ID")]
        id: Option<String>,
        /// Häufigkeit der Verstöße in einem Korpus zeigen: fux (Fux' 123 Beispiele, Standard) oder
        /// palestrina (seine Messen, freier Satz)
        #[arg(long, value_name = "KORPUS", num_args = 0..=1, default_missing_value = "fux")]
        corpus: Option<CorpusName>,
        #[command(flatten)]
        rules: RuleArgs,
    },
    /// Listet Fux' Notenbeispiele, seine Cantus firmi oder Palestrinas Messen
    Corpus {
        /// Cantus firmi statt Beispielen anzeigen
        #[arg(long)]
        cantus: bool,
        /// Palestrinas Messen (Humdrum, references/datasets/palestrina-masses oder $FUX_PALESTRINA)
        #[arg(long, conflicts_with = "cantus")]
        palestrina: bool,
    },
}

/// The corpora whose rule frequencies `fux rules --corpus` shows.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum CorpusName {
    Fux,
    Palestrina,
}

/// Exit code when the checked score breaks a prohibition.
const EXIT_FORBIDDEN: i32 = 2;

fn main() {
    let cli = Cli::parse();
    let lang = cli.lang;
    match run(cli) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            // A closed pipe (e.g. `fux corpus | head`) is not an error.
            let broken_pipe = error
                .downcast_ref::<io::Error>()
                .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe);
            if !broken_pipe {
                eprintln!("{}: {error:#}", lang.pick("Fehler", "Error"));
                std::process::exit(1);
            }
        }
    }
}

fn run(cli: Cli) -> Result<i32> {
    let lang = cli.lang;
    let mut out = io::stdout().lock();
    match cli.command {
        Command::Cf {
            mode,
            length,
            voice,
            seed,
            count,
            beam,
            noise,
            details,
            rules,
            output,
        } => {
            let options = commands::CantusOptions {
                mode,
                length,
                voice,
                seed,
                count,
                beam,
                noise,
                details,
            };
            commands::cantus(&mut out, &options, &rules.profile(lang)?, &output, lang)?;
        }
        Command::Check {
            input,
            text,
            repeat,
            imitation,
            rules,
            mark,
            output,
        } => {
            let (read, fields) = input.read_with_fields(lang)?;
            // The file of a part of a mass is checked as the part was set (D14): under the
            // overlay of the mass, its text going on where it went on, its syllables singing the
            // positions they sang, its last point ending on the part's ending.
            let base = RuleArgs {
                profile: rules.profile.clone(),
                rules: Vec::new(),
            }
            .profile(lang)?;
            // The file of a mass movement is checked part by part, each part as the file of the
            // part is; another file in two sections or more section by section (§ 6).
            if let Some(check) = fux_cli::mass::check_movement(
                &read,
                &fields,
                text.as_deref(),
                &base,
                &rules.rules,
                repeat,
                imitation,
                lang,
            )? {
                let forbidden =
                    fux_cli::mass::print_movement_check(&mut out, &check, mark, &output, lang)?;
                return Ok(if forbidden > 0 { EXIT_FORBIDDEN } else { 0 });
            }
            let mut score = read.score;
            let (part, profile) =
                fux_cli::mass::check_profile(&fields, &base, &rules.rules, repeat, lang)?;
            let text = text.or(read.text);
            for line in fux_cli::text::align_lyrics(&mut score, text.as_deref(), lang)? {
                writeln!(out, "{line}")?;
            }
            if let Some(part) = &part {
                fux_cli::mass::restore_part(&mut score, part, &fields);
            }
            let ending = part
                .as_ref()
                .map_or_else(|| score.mode.final_step(), |part| part.ending);
            let remedy = lang.pick("Abhilfe: --imitation aus.", "Remedy: --imitation off.");
            let lines = fux_cli::imitation::attach_ending(
                &mut score,
                read.imitation,
                imitation,
                remedy,
                ending,
                lang,
            )?;
            for line in lines {
                writeln!(out, "{line}")?;
            }
            let forbidden = commands::check(&mut out, &score, &profile, mark, &output, lang)?;
            if forbidden > 0 {
                return Ok(EXIT_FORBIDDEN);
            }
        }
        Command::Render { input, output } => {
            let (read, fields) = input.read_with_fields(lang)?;
            let mut score = read.score;
            // The plan of an imitation, for the brackets over its entries, if it fits.
            if fux_cli::imitation::attach(
                &mut score,
                read.imitation,
                ImitationSource::File,
                "",
                lang,
            )
            .is_err()
            {
                score.imitation = None;
            }
            let sheets = [fux_cli::mass::rendered_sheet(&score, &fields, lang)];
            let files = output::write(None, &sheets, &output, "out/satz", lang)?;
            commands::print_files(&mut out, &files, lang)?;
        }
        Command::Cp {
            cf,
            mode,
            cf_voice,
            cadences,
            length,
            species,
            voices,
            parts,
            below,
            voice,
            text,
            singing,
            seed,
            count,
            beam,
            noise,
            details,
            rules,
            output,
        } => {
            let options = commands::CounterpointOptions {
                cf,
                mode,
                cf_voice,
                cadences,
                length,
                species,
                voices: usize::from(voices),
                parts,
                below,
                voice,
                seed,
                count,
                beam,
                noise,
                details,
                text,
                repeat: singing.repeat,
                accents: singing.accents,
                eleison: singing.eleison,
            };
            commands::counterpoint(&mut out, &options, &rules.profile(lang)?, &output, lang)?;
        }
        Command::Free {
            mode,
            voices,
            parts,
            text,
            length,
            cadences,
            ending,
            singing,
            imitation,
            seed,
            count,
            beam,
            noise,
            details,
            rules,
            output,
        } => {
            let options = fux_cli::free::FreeOptions {
                mode,
                voices: usize::from(voices),
                parts,
                length: length.measures(),
                cadences,
                ending,
                text,
                repeat: singing.repeat,
                accents: singing.accents,
                eleison: singing.eleison,
                seed,
                count,
                beam,
                noise,
                details,
                imitation,
            };
            fux_cli::free::free(&mut out, &options, &rules.profile(lang)?, &output, lang)?;
        }
        Command::Mass {
            options,
            rules,
            output,
        } => {
            // The profile chosen without the single overrides: the overlay of the mass comes
            // between them, so that `--rule` can change it too.
            let base = RuleArgs {
                profile: rules.profile.clone(),
                rules: Vec::new(),
            }
            .profile(lang)?;
            return fux_cli::mass::mass(&mut out, &options, &base, &rules.rules, &output, lang);
        }
        Command::Text {
            text,
            list,
            eleison,
        } => {
            if list || text.is_empty() {
                fux_cli::text::list_texts(&mut out, eleison, lang)?;
            } else {
                fux_cli::text::show_text(&mut out, &text.join(" "), eleison, lang)?;
            }
        }
        Command::Book {
            modes,
            species,
            voices,
            generate,
            length,
            exercises,
            seed,
            beam,
            noise,
            rules,
            output,
        } => {
            let options = fux_cli::book::BookOptions {
                modes,
                species,
                voices,
                generate,
                length,
                exercises,
                seed,
                beam,
                noise,
            };
            fux_cli::book::book(&mut out, &options, &rules.profile(lang)?, &output, lang)?;
        }
        Command::Rules { id, corpus, rules } => {
            let profile = rules.profile(lang)?;
            match (id, corpus) {
                (Some(id), _) => commands::rule_details(&mut out, &profile, &id, lang)?,
                (None, Some(CorpusName::Fux)) => commands::rule_corpus(&mut out, &profile, lang)?,
                (None, Some(CorpusName::Palestrina)) => {
                    fux_cli::palestrina::rule_corpus(&mut out, &profile, lang)?;
                }
                (None, None) => commands::rule_list(&mut out, &profile, lang)?,
            }
        }
        Command::Corpus { cantus, palestrina } => {
            if palestrina {
                fux_cli::palestrina::print_corpus(&mut out, lang)?;
            } else if cantus {
                commands::print_cantus(&mut out, lang)?;
            } else {
                commands::print_examples(&mut out, lang)?;
            }
        }
    }
    Ok(0)
}
