//! The window driven like a user would, without a display.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fux_cli::output::Format;
use fux_gui::{FuxApp, PartState, Tab};

fn harness(app: FuxApp) -> Harness<'static, FuxApp> {
    Harness::builder()
        .with_size(egui::vec2(1360.0, 880.0))
        // Rule texts use signs such as ⁀ that the test fonts lack; the app has the system's.
        .allow_missing_glyphs()
        .build_ui_state(|ui, app: &mut FuxApp| app.show(ui), app)
}

/// Lets the task finish and the window show its result.
fn settle(harness: &mut Harness<'static, FuxApp>) {
    harness.state_mut().finish();
    harness.run();
}

#[test]
fn compose_in_the_window() {
    let mut app = FuxApp::headless();
    app.cp.random_seed = false;
    app.cp.seed = 2;
    let mut harness = harness(app);
    harness.get_by_label("Setzen").click();
    // One frame: while the task runs, the spinner keeps the window busy.
    harness.step();
    assert!(harness.state().is_busy());
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(state.entries.len(), 3, "{:?}", state.message);
    assert!(state.summary[0].starts_with("dorisch – 1. Gattung, 2 Stimmen, seed 2"));
    // The results, the notes as text (no LilyPond here) and the violations of the first.
    harness.get_by_label_contains("1. Kosten");
    harness.get_by_label_contains("C#5 | D5");
    harness.get_by_label_contains("Verbot(e), Kosten");
    harness.get_by_label_contains("2. Kosten").click();
    harness.run();
    assert_eq!(harness.state().selected, 1);
    // Export of the second solution as LilyPond and MIDI.
    let base = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("gui-export");
    harness.state_mut().output = base.display().to_string();
    harness.state_mut().formats = vec![Format::Ly, Format::Midi];
    harness.get_by_label("Exportieren").click();
    harness.run();
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert!(ly.contains("Kontrapunkt 2"), "{ly}");
    let midi = std::fs::read(base.with_extension("mid")).unwrap();
    assert!(midi.starts_with(b"MThd"));
}

#[test]
fn free_counterpoint_in_the_window() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Free;
    app.free.random_seed = false;
    app.free.seed = 4;
    app.free.voices = 2;
    // As before phase 12: no text, the length given.
    app.free.text = fux_gui::NO_TEXT.into();
    app.free.length_auto = false;
    app.free.length = 8;
    app.free.count = 2;
    let mut harness = harness(app);
    harness.get_by_label_contains("Kadenzstufen des Modus: A, F, G");
    harness.get_by_label("Setzen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(state.entries.len(), 2, "{:?}", state.message);
    assert!(
        state.summary[0].starts_with("dorisch – freier Satz, 2 Stimmen, 8 Takte, seed 4"),
        "{:?}",
        state.summary
    );
    assert!(state.entries[0].score.is_free());
    assert_eq!(
        state.entries[0].score.title.as_deref(),
        Some("Freier Satz 1")
    );
    harness.get_by_label_contains("1. Kosten");
}

/// Opens the choice of texts of the free setting and picks one.
fn choose_text(harness: &mut Harness<'static, FuxApp>, label: &str) {
    harness.get_by_label("Text:").click();
    harness.run();
    // The list scrolls: the entry is brought into view first.
    harness.get_by_label(label).scroll_to_me();
    harness.run();
    harness.get_by_label(label).click();
    harness.run();
    assert_eq!(harness.query_all_by_label(label).count(), 0, "still open");
}

#[test]
fn free_form_text() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Free;
    app.free.random_seed = false;
    app.free.seed = 4;
    app.free.voices = 2;
    app.free.count = 1;
    let mut harness = harness(app);
    // Kyrie I by default, as long as it needs.
    harness.get_by_value("Kyrie I (7 Silben)");
    assert!(harness.state().free.length_auto);
    // Another section of the Ordinary, sung in two voices.
    choose_text(&mut harness, "Christe (6 Silben)");
    assert_eq!(harness.state().free.text, "christe");
    harness.get_by_value("Christe (6 Silben)");
    harness.get_by_label("Setzen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(state.entries.len(), 1, "{:?}", state.message);
    assert_eq!(
        state.summary[..2],
        [
            "dorisch – freier Satz, 2 Stimmen, 14 Takte, seed 4",
            "Text: Christe (6 Silben, 1 Sinnabschnitt), Wiederholung auto"
        ],
        "{:?}",
        state.summary
    );
    let score = &state.entries[0].score;
    assert!(score.text.is_some());
    // Under each voice what it sings, in the text view (no LilyPond here).
    let lyrics: Vec<String> = (0..2).map(|v| score.voice_lyrics(v).unwrap()).collect();
    for line in &lyrics {
        assert!(line.starts_with("Chri-ste e-lé-i-son."), "{line}");
        let label = format!("Text: {line}");
        assert!(harness.query_all_by_label(&label).count() > 0, "{label}");
    }
    assert_eq!(
        harness
            .query_all_by_label_contains("Text: Chri-ste")
            .count(),
        2
    );
    // An own text, with a preview of its syllables, stresses, warnings and length.
    choose_text(&mut harness, "Eigener Text …");
    assert_eq!(harness.state().free.text, fux_gui::OWN_TEXT);
    harness.get_by_label("Eigener lateinischer Text:").focus();
    harness.run();
    harness
        .get_by_label("Eigener lateinischer Text:")
        .type_text("Ad te levavi animam meam.");
    harness.run();
    assert_eq!(harness.state().free.own_text, "Ad te levavi animam meam.");
    harness.get_by_label("Ad te le-vá-vi á-ni-mam me-am.");
    harness.get_by_label("Eigener Text (10 Silben, 1 Sinnabschnitt)");
    harness.get_by_label("Warnung: Akzent unsicher: animam → á-ni-mam; mit Akut angeben");
    harness.get_by_label("Automatische Länge: 12 Takte");
    // Set, the warning follows the summary.
    harness.get_by_label("Setzen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(
        state.summary.last().map(String::as_str),
        Some("Warnung: Akzent unsicher: animam → á-ni-mam; mit Akut angeben"),
        "{:?} {:?}",
        state.summary,
        state.message
    );
    assert_eq!(
        harness
            .query_all_by_label_contains("Text: Ad te le-vá-vi á-ni-mam me-am.")
            .count(),
        2
    );
    // Without a text, the setting is as before phase 12: 12 measures, no lyrics.
    choose_text(&mut harness, "ohne Text");
    assert_eq!(harness.state().free.text, fux_gui::NO_TEXT);
    assert_eq!(harness.query_all_by_label("Wiederholung:").count(), 0);
    harness.get_by_label("Setzen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    assert!(
        state.summary[0].starts_with("dorisch – freier Satz, 2 Stimmen, 12 Takte, seed 4"),
        "{:?}",
        state.summary
    );
    assert!(state.summary.iter().all(|line| !line.starts_with("Text")));
    let score = &state.entries[0].score;
    assert!(score.text.is_none());
    assert!((0..2).all(|v| score.voice_lyrics(v).is_none()));
    assert_eq!(harness.query_all_by_label_contains("Text: ").count(), 0);
}

/// Free voices around a cantus sing the text chosen in the form of the counterpoint, the cantus
/// sings none; the species sing no text, so the choice shows only for free counterpoint.
#[test]
fn counterpoint_form_text() {
    let mut app = FuxApp::headless();
    app.cp.random_seed = false;
    app.cp.seed = 1;
    app.cp.count = 1;
    let mut harness = harness(app);
    // The first species sings nothing.
    assert_eq!(harness.query_all_by_label("Text:").count(), 0);
    harness.state_mut().cp.species = 0;
    harness.run();
    // Free counterpoint: without a text unless one is chosen, as before.
    harness.get_by_value("ohne Text");
    assert_eq!(harness.query_all_by_label("Wiederholung:").count(), 0);
    choose_text(&mut harness, "Kyrie I (7 Silben)");
    assert_eq!(harness.state().cp.text, "kyrie");
    harness.get_by_value("Kyrie I (7 Silben)");
    harness.get_by_label("Wiederholung:");
    harness.get_by_label("Setzen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(state.entries.len(), 1, "{:?}", state.message);
    assert_eq!(
        state.summary[..3],
        [
            "dorisch – freier Satz, 2 Stimmen, seed 1",
            "C. f. (Alt): D4 | F4 | E4 | D4 | G4 | F4 | A4 | G4 | F4 | E4 | D4",
            "Text: Kyrie I (7 Silben, 1 Sinnabschnitt), Wiederholung auto"
        ],
        "{:?}",
        state.summary
    );
    // The soprano sings the whole text, the alto, which is the cantus, nothing.
    let score = &state.entries[0].score;
    assert_eq!(score.text.as_ref().unwrap().voices, [true, false]);
    let lyrics = score.voice_lyrics(0).unwrap();
    assert!(lyrics.starts_with("Ký-ri-e e-lé-i-son."), "{lyrics}");
    assert!(lyrics.ends_with(" e-lé-i-son."), "{lyrics}");
    assert_eq!(score.voice_lyrics(1), None);
    assert_eq!(
        harness
            .query_all_by_label(&format!("Text: {lyrics}"))
            .count(),
        1
    );
    // Taken over with its text and checked: the soprano, free, has the violations it had; the
    // check also counts those of the cantus.
    let generated = state.entries[0].clone();
    harness.state_mut().tab = Tab::Check;
    harness.run();
    harness.get_by_label("Ergebnis übernehmen").click();
    harness.run();
    assert!(harness.state().check.texted.is_some());
    harness.state_mut().start_check();
    settle(&mut harness);
    let checked = &harness.state().entries[0];
    assert!(checked.score.text.is_some());
    let soprano: Vec<_> = checked
        .report
        .violations
        .iter()
        .filter(|v| v.voice == 0)
        .collect();
    assert_eq!(
        soprano,
        generated.report.violations.iter().collect::<Vec<_>>()
    );
    assert_eq!(checked.report.forbidden(), 0);
    harness.state_mut().tab = Tab::Counterpoint;
    harness.run();
    // An own text with its preview: syllables, stresses, warnings and the cantus it needs.
    choose_text(&mut harness, "Eigener Text …");
    harness.state_mut().cp.own_text = "Ad te levavi animam meam.".into();
    harness.run();
    harness.get_by_label("Ad te le-vá-vi á-ni-mam me-am.");
    harness.get_by_label("Warnung: Akzent unsicher: animam → á-ni-mam; mit Akut angeben");
    harness.get_by_label("Länge: die des Cantus; dieser Text braucht mindestens 6 Takte.");
    // A text too long for the cantus: the run says so, and the results stay.
    choose_text(&mut harness, "Gloria: Et in terra (98 Silben)");
    harness.get_by_label("Setzen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    let (error, message) = state.message.clone().unwrap();
    assert!(
        error
            && message.starts_with(
                "Der Text braucht mindestens 50 Takte. Der Cantus hat 11 Takte. Abhilfe: "
            ),
        "{message}"
    );
    assert_eq!(state.entries.len(), 1);
}

/// The group „Imitation“ of the free form: three voices on Kyrie I imitate by default; „Plan
/// zeigen“ plans the points with the choices of the group, without a search; set, the result
/// shows its points, and taken over into the check it is checked with its plan, with the
/// setting's report.
#[test]
fn free_form_imitation() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Free;
    app.free.random_seed = false;
    app.free.seed = 1;
    app.free.voices = 3;
    app.free.count = 1;
    app.free.beam = 40;
    let mut harness = harness(app);
    harness.get_by_label("Imitation:");
    harness.get_by_label("(imitiert)");
    harness.get_by_value("aus dem Text");
    // The length comes from the plan.
    harness.get_by_label("aus dem Plan");
    // The choices of the points and their entries.
    harness.get_by_label("Punkte und Einsätze").click();
    harness.run();
    for label in [
        "Punkte:",
        "Stimmen späterer Punkte:",
        "Reihenfolge:",
        "Intervalle:",
        "Abstände:",
        "Paare:",
        "Punkt 1 wiederholen:",
        "Antwort:",
        "Kopf:",
    ] {
        harness.get_by_label(label);
    }
    harness.get_by_label("Reihenfolge:").focus();
    harness.run();
    harness.get_by_label("Reihenfolge:").type_text("B A T");
    harness.run();
    assert_eq!(harness.state().free.imitation.order, "B A T");
    // The plan, without a search.
    harness.get_by_label("Plan zeigen").scroll_to_me();
    harness.run();
    harness.get_by_label("Plan zeigen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    assert!(state.entries.is_empty());
    assert_eq!(
        state.message,
        Some((false, "Plan der Imitation, ohne Suche.".to_string()))
    );
    assert!(
        state.summary[0].starts_with("dorisch – Plan der Imitation, 3 Stimmen, "),
        "{:?}",
        state.summary
    );
    let first = state
        .summary
        .iter()
        .find(|l| l.starts_with("Punkt 1 "))
        .cloned()
        .unwrap_or_default();
    assert!(
        first.starts_with("Punkt 1 („Ký-ri-e e-lé-i-son.“): Bass T. 1 "),
        "{:?}",
        state.summary
    );
    harness.get_by_label(&first);
    // Set: the result shows its points as planned. The form is long now: the button is
    // brought into view first.
    harness.get_by_label("Setzen").scroll_to_me();
    harness.run();
    harness.get_by_label("Setzen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(state.entries.len(), 1, "{:?}", state.message);
    let score = &state.entries[0].score;
    assert!(score.imitation.is_some());
    let points: Vec<String> = state
        .summary
        .iter()
        .filter(|l| l.starts_with("Punkt "))
        .cloned()
        .collect();
    assert!(
        points.len() >= 2 && points[0].starts_with("Punkt 1 („Ký-ri-e e-lé-i-son.“): Bass T. 1 "),
        "{:?}",
        state.summary
    );
    // Taken over and checked: the plan comes along in its MusicXML, the report is the setting's.
    let generated = state.entries[0].clone();
    harness.state_mut().tab = Tab::Check;
    harness.run();
    harness.get_by_label("Ergebnis übernehmen").click();
    harness.run();
    let texted = harness.state().check.texted.clone().unwrap();
    assert!(texted.musicxml.contains("fux-imitation"));
    harness.state_mut().start_check();
    settle(&mut harness);
    let state = harness.state();
    let checked = &state.entries[0];
    assert_eq!(checked.score.imitation, generated.score.imitation);
    assert_eq!(checked.report, generated.report);
    assert!(
        state
            .summary
            .iter()
            .any(|l| l == &format!("Imitation nach dem Plan der Datei: {} Punkte", points.len())),
        "{:?}",
        state.summary
    );
    // Subjects from the chant of the Kyriale; notes that are still missing.
    harness.state_mut().tab = Tab::Free;
    harness.run();
    harness.get_by_label("Soggetto:").click();
    harness.run();
    harness.get_by_label("Choral des Kyriale").click();
    harness.run();
    harness.get_by_label("Kyrie IX (dorisch)");
    harness.get_by_label("Phrasen:");
    harness.get_by_label("im Modus des Chorals");
    harness.get_by_label("Soggetto:").click();
    harness.run();
    harness.get_by_label("eigene Noten").click();
    harness.run();
    harness.get_by_label_contains("Soggetto: bitte Noten eingeben");
    assert!(
        harness
            .get_by_label("Plan zeigen")
            .accesskit_node()
            .is_disabled()
    );
    // Off: the group shrinks to its switch.
    harness.state_mut().free.imitation.subject = fux_gui::SubjectKind::Text;
    harness.get_by_label("Imitation:").click();
    harness.run();
    harness.get_by_label("aus").click();
    harness.run();
    harness.get_by_label("(imitiert nicht)");
    assert_eq!(harness.query_all_by_label("Soggetto:").count(), 0);
    assert_eq!(harness.query_all_by_label("Plan zeigen").count(), 0);
}

/// The seed at the end of a summary's first line, as in „…, 19 Takte, seed 4711“.
fn summary_seed(summary: &[String]) -> u64 {
    summary[0]
        .rsplit_once("seed ")
        .and_then(|(_, seed)| seed.trim().parse().ok())
        .unwrap_or_else(|| panic!("{summary:?}"))
}

/// With a new seed for every run, as the form starts, „Plan zeigen“ draws the seed and the next
/// „Setzen“ keeps it, so that it sets the plan shown; the one after draws anew.
#[test]
fn the_plan_shown_is_the_plan_set() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Free;
    assert!(app.free.random_seed);
    app.free.voices = 3;
    app.free.count = 1;
    app.free.beam = 40;
    let mut harness = harness(app);
    harness.get_by_label("Plan zeigen").scroll_to_me();
    harness.run();
    harness.get_by_label("Plan zeigen").click();
    harness.step();
    settle(&mut harness);
    let planned = summary_seed(&harness.state().summary);
    harness.get_by_label("Setzen").scroll_to_me();
    harness.run();
    harness.get_by_label("Setzen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(state.entries.len(), 1, "{:?}", state.message);
    assert_eq!(summary_seed(&state.summary), planned, "{:?}", state.summary);
    // The same seed plans the same points; only a probe that fails plans one anew, which the
    // lines of the setting then show.
    assert_eq!(state.free.seed, planned);
    harness.get_by_label("Setzen").scroll_to_me();
    harness.run();
    harness.get_by_label("Setzen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(state.entries.len(), 1, "{:?}", state.message);
    assert_ne!(summary_seed(&state.summary), planned, "{:?}", state.summary);
}

/// An imitation switched off hides its group: what its fields hold stops no setting, even notes
/// still missing for a subject of own notes.
#[test]
fn hidden_imitation_inputs_stop_no_setting() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Free;
    app.free.random_seed = false;
    app.free.seed = 4;
    app.free.voices = 2;
    app.free.text = fux_gui::NO_TEXT.to_string();
    app.free.length_auto = false;
    app.free.length = 8;
    app.free.count = 1;
    app.free.imitation.subject = fux_gui::SubjectKind::Notes;
    app.free.imitation.order = "A X".into();
    app.free.imitation.mode = fux_gui::Switch::Off;
    let mut harness = harness(app);
    harness.get_by_label("(imitiert nicht)");
    harness.get_by_label("Setzen").scroll_to_me();
    harness.run();
    harness.get_by_label("Setzen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(state.entries.len(), 1, "{:?}", state.message);
}

#[test]
fn free_form_text_errors() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Free;
    app.free.voices = 2;
    let mut harness = harness(app);
    // Some sense units of the Gloria, in fewer measures than they need.
    choose_text(&mut harness, "Gloria: Et in terra (98 Silben)");
    assert_eq!(harness.state().free.text, "gloria");
    harness.get_by_label("bis").set_value_accesskit(5.0);
    harness.run();
    assert_eq!(harness.state().free.text, "gloria:1-5");
    harness.get_by_value("Gloria: Et in terra 1–5 (36 Silben)");
    harness.get_by_label("automatisch").click();
    harness.run();
    // The length given starts from the planned one.
    assert_eq!(
        (
            harness.state().free.length_auto,
            harness.state().free.length
        ),
        (false, 32)
    );
    harness.state_mut().free.length = 10;
    harness.run();
    let least = harness
        .get_by_label_contains("Der Text braucht mindestens ")
        .accesskit_node()
        .label()
        .unwrap_or_default();
    harness.get_by_label("Setzen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    let (error, message) = state.message.clone().unwrap();
    assert!(error && message.starts_with(&least), "{message}");
    assert!(message.contains("Abhilfe: mehr Takte"), "{message}");
    assert!(state.entries.is_empty());
    // A character that is no Latin: the form says so, and nothing starts.
    choose_text(&mut harness, "Eigener Text …");
    harness.state_mut().free.own_text = "Ad te #levavi.".into();
    harness.run();
    harness.get_by_label_contains("Unerlaubtes Zeichen „#“");
    harness.get_by_label("Setzen").click();
    harness.run();
    let state = harness.state();
    assert!(!state.is_busy());
    let (error, message) = state.message.clone().unwrap();
    assert!(
        error && message.starts_with("Unerlaubtes Zeichen „#“"),
        "{message}"
    );
}

#[test]
fn a_texted_result_is_checked_with_its_text() {
    let mut app = FuxApp::headless();
    app.free.text = "christe".into();
    app.free.voices = 2;
    app.free.count = 1;
    app.free.random_seed = false;
    app.free.seed = 4;
    app.start_free();
    app.finish();
    let generated = app.entries[0].clone();
    assert!(generated.score.text.is_some(), "{:?}", app.message);
    app.tab = Tab::Check;
    let mut harness = harness(app);
    let base = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("gui-christe");
    harness.state_mut().output = base.display().to_string();
    harness.state_mut().formats = vec![Format::Musicxml];
    harness.get_by_label("Exportieren").click();
    harness.run();
    // The file names the seed of the setting, in the window's tempo (72 halves a minute).
    let xml = std::fs::read_to_string(base.with_extension("musicxml")).unwrap();
    assert!(
        xml.contains(">fux-kontrapunkt, Seed 4</credit-words>"),
        "{xml}"
    );
    assert!(xml.contains("<sound tempo=\"144\"/>"), "{xml}");
    // Taken over with its text, and checked with the cadences of the result: the same report.
    // The MusicXML names the text, so the field stays empty for files checked later.
    harness.get_by_label("Ergebnis übernehmen").click();
    harness.run();
    assert_eq!(harness.state().check.text, "");
    assert!(harness.state().check.texted.is_some());
    harness.get_by_label("Übernommener Satz mit Text: Freier Satz 1 (2 Stimmen)");
    harness.state_mut().start_check();
    settle(&mut harness);
    let state = harness.state();
    let entry = &state.entries[0];
    assert!(entry.marked && entry.score.text.is_some());
    assert_eq!(entry.report, generated.report);
    assert!(
        state
            .summary
            .iter()
            .any(|l| l.starts_with("Text: Christe; von ")),
        "{:?}",
        state.summary
    );
    assert_eq!(
        harness
            .query_all_by_label_contains("Text: Chri-ste")
            .count(),
        2
    );
    // To edit the notes the text is dropped.
    harness.get_by_label("Noten bearbeiten (ohne Text)").click();
    harness.run();
    let check = &harness.state().check;
    assert!(check.texted.is_none() && check.text.is_empty());
    assert_eq!(check.voices.len(), 2);
    harness.get_by_label("+ Stimme");
    // Saved as MusicXML and checked: the file names its text, its mode and its cadences, so the
    // report is the result's.
    harness
        .state_mut()
        .start_job_file(base.with_extension("musicxml"));
    settle(&mut harness);
    let state = harness.state();
    assert!(state.entries[0].score.text.is_some());
    assert_eq!(state.entries[0].report, generated.report);
    assert!(
        state
            .summary
            .iter()
            .any(|line| line.starts_with("Text: Christe; von ")),
        "{:?}",
        state.summary
    );
}

#[test]
fn check_examples_and_rules() {
    let mut harness = harness(FuxApp::headless());
    harness.state_mut().tab = Tab::Examples;
    harness.run();
    harness.get_by_label_contains("Fig. 5 –").click();
    harness.run();
    let entry = &harness.state().entries[0];
    assert!(entry.marked);
    assert_eq!(entry.score.voices.len(), 2);
    // A score typed in: the tritone F4–B4 is forbidden.
    harness.state_mut().tab = Tab::Check;
    harness.state_mut().check.voices[0].notes = "f'1 b'1".into();
    harness.state_mut().check.cantus = "D4 E4".into();
    harness.run();
    harness.state_mut().start_check();
    settle(&mut harness);
    let report = &harness.state().entries[0].report;
    assert!(report.violations.iter().any(|v| v.rule == "MEL-TRITONE"));
    // The rules window lists the rules; the search narrows them.
    harness.get_by_label("Regeln …").click();
    harness.run();
    assert!(harness.query_all_by_label_contains("G5-EIGHTHS").count() > 0);
    harness.get_by_label("Suche:").focus();
    harness.run();
    harness.get_by_label("Suche:").type_text("TRITONE-OUT");
    harness.run();
    assert!(
        harness
            .query_all_by_label_contains("MEL-TRITONE-OUTLINE")
            .count()
            > 0
    );
    assert_eq!(harness.query_all_by_label_contains("G5-EIGHTHS").count(), 0);
    // Jeppesen's profile, and back to Fux.
    harness.get_by_label("Jeppesen").click();
    harness.run();
    assert_eq!(harness.state().profile.name, "jeppesen");
    harness.get_by_label_contains("Profil: jeppesen");
    harness.get_by_label("Standard (Fux)").click();
    harness.run();
    assert_eq!(harness.state().profile.name, "fux");
}

#[test]
fn english_and_generated_cantus() {
    let mut app = FuxApp::headless();
    app.cf.random_seed = false;
    let mut harness = harness(app);
    harness.get_by_label("English").click();
    harness.state_mut().tab = Tab::Cantus;
    harness.run();
    harness.get_by_label("Generate").click();
    harness.step();
    settle(&mut harness);
    assert_eq!(harness.state().entries.len(), 5);
    harness
        .get_by_label("Use as cantus for counterpoint")
        .click();
    harness.run();
    let state = harness.state();
    assert_eq!(state.tab, Tab::Counterpoint);
    assert_eq!(state.cp.notes.split(' ').count(), 11);
}

/// Renders the window with notes by LilyPond into `target/gui-*.png`, to look at it. Needs a
/// graphics adapter and LilyPond: `cargo test -p fux-gui --test ui -- --ignored`.
#[test]
#[ignore]
fn screenshots() {
    let shots = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("gui");
    std::fs::create_dir_all(&shots).unwrap();
    let mut app = FuxApp::headless().with_lilypond();
    app.cp.random_seed = false;
    app.cp.seed = 3;
    app.cp.voices = 4;
    app.cp.species = 2;
    app.cp.count = 2;
    let mut harness = harness(app);
    harness.state_mut().start_counterpoint();
    harness.state_mut().finish();
    // LilyPond renders in the background; the spinner keeps the window busy meanwhile.
    for _ in 0..300 {
        harness.step();
        if harness.state().preview_ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    harness.run_steps(2);
    harness
        .render()
        .unwrap()
        .save(shots.join("counterpoint.png"))
        .unwrap();
    harness.get_by_label("Regeln …").click();
    harness.run_steps(2);
    harness
        .render()
        .unwrap()
        .save(shots.join("rules.png"))
        .unwrap();
    // A hymn with its internal cadences and text.
    let mut harness = self::harness(FuxApp::headless().with_lilypond());
    {
        let app = harness.state_mut();
        app.cp.random_seed = false;
        app.cp.seed = 3;
        app.cp.voices = 4;
        app.cp.source = fux_gui::SourceKind::Gabc;
        // Relative to the crate, where the tests run.
        app.cp.gabc = "../../tests/fixtures/chant/veni-creator.gabc".into();
        app.cp.cf_voice = Some(fux_core::VoiceType::Tenor);
        app.cp.species = 1;
        app.cp.count = 1;
        app.start_counterpoint();
        app.finish();
    }
    for _ in 0..300 {
        harness.step();
        if harness.state().preview_ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    harness.run_steps(2);
    harness
        .render()
        .unwrap()
        .save(shots.join("chant.png"))
        .unwrap();
    // A workbook with its exercises.
    let mut harness = self::harness(FuxApp::headless().with_lilypond());
    {
        let app = harness.state_mut();
        app.tab = Tab::Book;
        app.book.random_seed = false;
        app.book.seed = 3;
        app.book.species = [false, false, true, false, false];
        app.start_book();
        app.finish();
    }
    for _ in 0..300 {
        harness.step();
        if harness.state().preview_ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    harness.run_steps(2);
    harness
        .render()
        .unwrap()
        .save(shots.join("book.png"))
        .unwrap();
    // A free setting of the Christe with its text, and the form with the preview of an own text.
    let mut harness = self::harness(FuxApp::headless().with_lilypond());
    {
        let app = harness.state_mut();
        app.tab = Tab::Free;
        app.free.random_seed = false;
        app.free.seed = 4;
        app.free.voices = 4;
        app.free.count = 1;
        app.free.text = "christe".into();
        app.start_free();
        app.finish();
        app.free.text = fux_gui::OWN_TEXT.into();
        app.free.own_text = "Ad te le-vá-vi | á-ni-mam me-am: / De-us me-us.".into();
    }
    for _ in 0..300 {
        harness.step();
        if harness.state().preview_ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    harness.run_steps(2);
    harness
        .render()
        .unwrap()
        .save(shots.join("free-text.png"))
        .unwrap();
    // An imitative Kyrie in three voices with the brackets over its entries, and the group of
    // the imitation with its points and entries.
    let mut harness = self::harness(FuxApp::headless().with_lilypond());
    {
        let app = harness.state_mut();
        app.tab = Tab::Free;
        app.free.random_seed = false;
        app.free.seed = 1;
        app.free.voices = 3;
        app.free.count = 1;
        app.start_free();
        app.finish();
    }
    // LilyPond renders meanwhile: steps, as the spinner keeps the window busy.
    harness.step();
    harness.get_by_label("Punkte und Einsätze").click();
    for _ in 0..300 {
        harness.step();
        if harness.state().preview_ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    harness.run_steps(2);
    harness
        .render()
        .unwrap()
        .save(shots.join("free-imitation.png"))
        .unwrap();
    eprintln!("screenshots in {}", shots.display());
}

#[test]
fn job_files_and_frequencies() {
    let job = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("gui-job.toml");
    std::fs::write(
        &job,
        "title = \"Übung\"\nmode = \"dorian\"\n\n[[voice]]\ntype = \"S\"\nnotes = \"a'1 a' g' a' b' c'' c'' b' d'' cis'' d''\"\n\n[[voice]]\ntype = \"A\"\ncf = true\nnotes = \"fux:dorian\"\n",
    )
    .unwrap();
    let mut harness = harness(FuxApp::headless());
    harness.state_mut().start_job_file(job.clone());
    settle(&mut harness);
    let entry = &harness.state().entries[0];
    assert_eq!(entry.score.voices.len(), 2);
    assert_eq!(entry.score.title.as_deref(), Some("Übung"));
    // Exported as MusicXML and checked again, it is the same score.
    let notes = |harness: &Harness<'static, FuxApp>| -> Vec<Vec<fux_core::Note>> {
        harness.state().entries[0]
            .score
            .voices
            .iter()
            .map(|v| v.notes.clone())
            .collect()
    };
    let before = notes(&harness);
    let base = job.with_file_name("gui-job-export");
    harness.state_mut().output = base.display().to_string();
    harness.state_mut().formats = vec![Format::Musicxml];
    harness.get_by_label("Exportieren").click();
    harness.run();
    // A score from a file names no seed.
    let xml = std::fs::read_to_string(base.with_extension("musicxml")).unwrap();
    assert!(
        !xml.contains("<credit-type>composer</credit-type>"),
        "{xml}"
    );
    harness
        .state_mut()
        .start_job_file(base.with_extension("musicxml"));
    settle(&mut harness);
    assert_eq!(notes(&harness), before);
    assert_eq!(
        harness.state().entries[0].score.title.as_deref(),
        Some("Übung")
    );
    // The frequencies in Fux's examples next to the rules.
    harness.get_by_label("Regeln …").click();
    harness.run();
    harness.get_by_label("Häufigkeit bei Fux").click();
    for _ in 0..600 {
        harness.step();
        if harness
            .query_all_by_label_contains("MEL-LEAP-COST  Sprung statt Schritt  [")
            .count()
            > 0
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("no frequencies");
}

#[test]
fn a_workbook_in_the_window() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Book;
    app.book.species = [true, false, false, false, false];
    app.book.random_seed = false;
    app.book.seed = 3;
    let mut harness = harness(app);
    harness.get_by_label("Erstellen").click();
    harness.step();
    settle(&mut harness);
    let state = harness.state();
    // Fux's six modes, the counterpoint above and below.
    assert_eq!(state.entries.len(), 12, "{:?}", state.message);
    assert!(
        state.summary[0].starts_with("Übungsheft – Kontrapunkt im Palestrinastil"),
        "{:?}",
        state.summary
    );
    harness.get_by_label_contains("dorisch – 1. Gattung, Gegenstimme oben (Kosten");
    // Exported whole: the exercises of each mode, then the solutions.
    let base = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("gui-book");
    harness.state_mut().output = base.display().to_string();
    harness.state_mut().formats = vec![Format::Ly];
    harness.get_by_label("Exportieren").click();
    harness.run();
    let ly = std::fs::read_to_string(base.with_extension("ly")).unwrap();
    assert_eq!(ly.matches("\\bookpart").count(), 12);
    assert!(ly.contains("Aufgaben – Dorisch") && ly.contains("Lösungen – Ionisch"));
}

/// Steps the window until `done` holds of it, sleeping between the steps, then lets it settle
/// as far as it does while a job runs (a spinner keeps it busy); fails after `seconds`.
fn wait_for(
    harness: &mut Harness<'static, FuxApp>,
    seconds: u64,
    what: &str,
    done: impl Fn(&FuxApp) -> bool,
) {
    let start = std::time::Instant::now();
    while !done(harness.state()) {
        assert!(
            start.elapsed().as_secs() < seconds,
            "{what}: not within {seconds} s; {:?}",
            harness.state().message
        );
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    harness.run_ok();
}

/// The seeds and costs of the parts of the mass in the window, in order; `None` for a part not
/// set.
fn mass_parts(app: &FuxApp) -> Vec<Option<(u64, u32)>> {
    app.mass_run()
        .unwrap()
        .mass
        .results
        .iter()
        .map(|result| result.as_ref().map(|r| (r.seed, r.generated.cost)))
        .collect()
}

/// The tab „Messe“ (phase 14): the Kyrie of a Missa brevis in three voices with a narrow search,
/// which takes seconds. The form plans as it changes; „Plan zeigen“ shows the plan without a
/// search; „Messe setzen“ sets the parts, several at once as `fux mass` sets them (K6), with a bar
/// and a line on the parts being searched, and shows the movement with the violations of its
/// parts; exported under the title the form gives it afterwards, it is the book, the files of the
/// movement, which `fux check` takes apart into its parts, and those of the parts. A part of the
/// movement checked, or of its own file, taken over into the check again is judged as its search
/// judged it. Set again one part at a time and cancelled after its first part, the mass is
/// incomplete, written so, and „Fehlende Teile setzen“ sets the rest as the first run did; „Teil
/// neu setzen“ gives a part a new seed.
#[test]
fn a_mass_in_the_window() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Mass;
    app.mass.movements = [true, false, false, false, false, false];
    app.mass.voices = 3;
    app.mass.beam = 40;
    app.mass.random_seed = false;
    app.mass.seed = 1;
    let mut harness = harness(app);
    // What the form plans, and its sections with their voices, restarts and endings.
    harness.get_by_label_contains("3 Teile, 20 Silben; geschätzt ");
    harness.get_by_label("Abschnitte").click();
    harness.run();
    // Each section's voices, imitation and ending; Kyrie II ends the movement on the final.
    assert_eq!(harness.query_all_by_label("Kyrie I").count(), 3);
    assert_eq!(harness.query_all_by_label("Christe").count(), 3);
    assert_eq!(harness.query_all_by_label("Kyrie II").count(), 2);
    // The plan without a search.
    harness.get_by_label("Plan zeigen").scroll_to_me();
    harness.run();
    harness.get_by_label("Plan zeigen").click();
    harness.run();
    let state = harness.state();
    assert_eq!(
        state.message,
        Some((false, "Plan der Messe, ohne Suche.".to_string()))
    );
    assert_eq!(
        state.summary[0],
        "Missa brevis – dorisch, A T B, Seed 1: 1 Satz, 3 Abschnitte, 3 Teile, 20 Silben, \
         Hauptstufe A"
    );
    assert!(
        state.summary[2].starts_with("Kyrie: Kyrie I (Imitation) → D ‖ Christe (Imitation)"),
        "{:?}",
        state.summary
    );
    assert!(state.mass_run().is_none() && state.entries.is_empty());
    // Set: the parts at once (one where the machine has few threads), the parts searched with
    // their measures, the bar.
    harness.get_by_label("Messe setzen").click();
    harness.step();
    assert!(harness.state().mass_busy());
    harness.get_by_label("Abbrechen");
    let run = harness.state().mass_run().unwrap();
    let (_, done, total) = run.progress().unwrap();
    assert_eq!((done, total), (0, 3));
    let status = run.status(fux_core::Lang::De).unwrap();
    assert!(
        status == "Setze die Messe …"
            || status.starts_with("Teil 1/3 · Kyrie · Takt ")
            || status.starts_with("kyrie/1 T. "),
        "{status}"
    );
    wait_for(&mut harness, 120, "the mass", |app| !app.mass_busy());
    let state = harness.state();
    let run = state.mass_run().unwrap();
    assert_eq!(run.set_count(), 3, "{:?}", state.message);
    assert_eq!(
        state.message,
        Some((false, "Messe gesetzt: 3 von 3 Teilen.".to_string()))
    );
    let first = mass_parts(state);
    // One result per movement, with its measures and the cost of its parts.
    let (measures, cost): (usize, u32) = run
        .mass
        .results
        .iter()
        .flatten()
        .map(|r| (r.generated.score.measure_count(), r.generated.cost))
        .fold((0, 0), |(m, c), (a, b)| (m + a, c + b));
    assert_eq!(state.entries.len(), 1);
    let label = format!("Kyrie ({measures} Takte, Kosten {cost})");
    assert_eq!(state.entries[0].label, label);
    assert!(state.entries[0].score.text.is_some());
    assert!(
        state.summary[2].starts_with(&format!(
            "Gesamt: 3 von 3 Teilen, {measures} Takte, Kosten {cost}, Suchzeit "
        )),
        "{:?}",
        state.summary
    );
    harness.get_by_label(&label);
    // The list of parts, and the violations of the movement part by part.
    harness.get_by_label("kyrie/2 · Christe (Imitation) → A");
    harness.get_by_label_contains("Teil kyrie/1: Kyrie I, Takte 1–");
    harness.get_by_label_contains("Kyrie – Kadenzplan: T. ");
    // Exported under the title of the form, which the mass set takes as the rest of its
    // notation: the book, the files of the movement and of its parts, the report.
    harness.state_mut().mass.title = "Missa prima".into();
    harness.run();
    assert!(
        harness.state().summary[0].starts_with("Missa prima (Form brevis) – dorisch, A T B"),
        "{:?}",
        harness.state().summary
    );
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("gui-messe");
    std::fs::remove_dir_all(&dir).ok();
    let base = dir.join("messe");
    harness.state_mut().output = base.display().to_string();
    harness.state_mut().formats = vec![Format::Ly, Format::Midi, Format::Musicxml];
    harness.get_by_label("Exportieren").click();
    harness.run();
    let (error, written) = harness.state().message.clone().unwrap();
    assert!(!error && written.starts_with("Geschrieben: "), "{written}");
    let ly = std::fs::read_to_string(dir.join("messe.ly")).unwrap();
    assert!(ly.contains("\\bookpart") && ly.contains("Christe"), "{ly}");
    assert!(
        ly.contains("Missa prima") && !ly.contains("Missa brevis"),
        "{ly}"
    );
    let midi = std::fs::read(dir.join("messe-1-kyrie.mid")).unwrap();
    assert!(midi.starts_with(b"MThd"));
    let report = std::fs::read_to_string(dir.join("messe.txt")).unwrap();
    assert!(report.contains("Gesamt: 3 von 3 Teilen"), "{report}");
    assert!(report.starts_with("Missa prima (Form brevis)"), "{report}");
    assert_eq!(
        std::fs::read_dir(dir.join("messe.teile")).unwrap().count(),
        3
    );
    harness.state_mut().mass.title.clear();
    harness.run();
    // `fux check` takes the movement apart into its parts and repeats their reports.
    let xml = std::fs::read_to_string(dir.join("messe-1-kyrie.musicxml")).unwrap();
    assert!(xml.contains("Missa prima"));
    let lang = fux_core::Lang::De;
    let (read, fields) = fux_cli::input::musicxml_text_fields(&xml, None, lang).unwrap();
    let check = fux_cli::mass::check_movement(
        &read,
        &fields,
        None,
        &fux_core::rules::Profile::fux(),
        &[],
        fux_cli::free::Repeat::Auto,
        fux_cli::imitation::ImitationSource::File,
        lang,
    )
    .unwrap()
    .unwrap();
    let costs: Vec<u32> = check.pieces.iter().map(|p| p.report.cost()).collect();
    let set: Vec<u32> = first.iter().map(|part| part.unwrap().1).collect();
    assert_eq!(costs, set);
    // Taken over into the check, the movement is checked part by part, each as its search
    // judged it, its measures counted through the movement.
    let first_measures = harness.state().mass_run().unwrap().mass.results[0]
        .as_ref()
        .unwrap()
        .generated
        .score
        .measure_count();
    harness.state_mut().tab = Tab::Check;
    harness.run();
    harness.get_by_label("Ergebnis übernehmen").click();
    harness.run();
    let texted = harness.state().check.texted.clone().unwrap();
    assert!(texted.musicxml.contains("fux-part.3"));
    harness.state_mut().start_check();
    settle(&mut harness);
    let state = harness.state();
    let checked: Vec<u32> = state.entries.iter().map(|e| e.report.cost()).collect();
    assert_eq!(checked, set, "{:?}", state.summary);
    assert_eq!(state.entries[1].before, first_measures);
    assert!(
        state.entries[1].label.starts_with(&format!(
            "Teil kyrie/2: Christe, Takte {}–",
            first_measures + 1
        )),
        "{}",
        state.entries[1].label
    );
    assert!(
        state.summary[0].starts_with(&format!("Gesamt: 0 Verbot(e), Kosten {cost} aus "))
            && state.summary[0].ends_with(" in 3 Teilen."),
        "{:?}",
        state.summary
    );
    // A part of it taken over in turn goes with the fields of its file, its measures from 1,
    // and is judged as the file of the part is below: Christe under the overlay of the mass,
    // its syllables where they were, its last point ending on A, not on the final.
    harness.state_mut().selected = 1;
    harness.run();
    harness.get_by_label("Ergebnis übernehmen").click();
    harness.run();
    let texted = harness.state().check.texted.clone().unwrap();
    for field in ["fux-mass\">", "fux-part\">bars=1-", "fux-lyrics\">"] {
        let field = format!("<miscellaneous-field name=\"{field}");
        assert!(texted.musicxml.contains(&field), "{field}");
    }
    harness.state_mut().start_check();
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(state.entries.len(), 1, "{:?}", state.message);
    let taken = (state.entries[0].report.clone(), state.summary.clone());
    // The file of a part, under the overlay of the mass: the report of its search; so again
    // once taken over.
    harness
        .state_mut()
        .start_job_file(dir.join("messe.teile").join("1-02-christe.musicxml"));
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(state.entries.len(), 1, "{:?}", state.message);
    assert_eq!(
        state.entries[0].report.cost(),
        set[1],
        "{:?}",
        state.summary
    );
    let file = (state.entries[0].report.clone(), state.summary.clone());
    assert!(
        file.1.iter().any(|line| line.contains(" auf A")),
        "{:?}",
        file.1
    );
    assert_eq!(taken, file);
    harness.get_by_label("Ergebnis übernehmen").click();
    harness.run();
    harness.state_mut().start_check();
    settle(&mut harness);
    let state = harness.state();
    assert_eq!(
        (state.entries[0].report.clone(), state.summary.clone()),
        file
    );
    // Back in the tab of the mass, its result is shown again.
    harness.state_mut().tab = Tab::Mass;
    harness.run();
    harness.get_by_label("Ergebnis zeigen").scroll_to_me();
    harness.run();
    harness.get_by_label("Ergebnis zeigen").click();
    harness.run();
    assert_eq!(harness.state().entries[0].label, label);
    assert_eq!(harness.query_all_by_label("Ergebnis zeigen").count(), 0);
    // Set again, one part at a time, and cancelled once its first part is set: the mass is
    // incomplete.
    harness.state_mut().mass_jobs = 1;
    harness.get_by_label("Messe setzen").scroll_to_me();
    harness.run();
    harness.get_by_label("Messe setzen").click();
    harness.step();
    assert!(harness.state().mass_busy());
    assert_eq!(mass_parts(harness.state()), [None, None, None]);
    wait_for(&mut harness, 60, "the first part", |app| {
        mass_parts(app)[0].is_some()
    });
    harness.get_by_label("Abbrechen").click();
    wait_for(&mut harness, 60, "the cancel", |app| !app.mass_busy());
    let state = harness.state();
    let run = state.mass_run().unwrap();
    let set = run.set_count();
    assert!((1..3).contains(&set), "{set}");
    assert_eq!(
        state.message,
        Some((
            false,
            format!(
                "Messe unvollständig: {set} von 3 Teilen gesetzt; „Fehlende Teile setzen“ setzt \
                 die übrigen."
            )
        ))
    );
    assert_eq!(
        state.entries[0].label,
        format!("Kyrie – unvollständig, {set} von 3 Teilen")
    );
    assert_eq!(harness.query_all_by_label("abgebrochen").count(), 3 - set);
    // Written as it is: the book says what is missing; the movement has no file of its own.
    harness.get_by_label("Exportieren").click();
    harness.run();
    let ly = std::fs::read_to_string(dir.join("messe.ly")).unwrap();
    assert!(
        ly.contains("Teil kyrie/3 fehlt: Die Suche wurde abgebrochen."),
        "{ly}"
    );
    assert!(!dir.join("messe-1-kyrie.musicxml").exists());
    let report = std::fs::read_to_string(dir.join("messe.txt")).unwrap();
    assert!(report.contains("Unvollständig, es fehlen: "), "{report}");
    // The missing parts, set with the seeds of the plan, are those of the first run.
    harness.get_by_label("Fehlende Teile setzen").scroll_to_me();
    harness.run();
    harness.get_by_label("Fehlende Teile setzen").click();
    harness.step();
    assert!(harness.state().mass_busy());
    wait_for(&mut harness, 120, "the missing parts", |app| {
        !app.mass_busy()
    });
    assert_eq!(mass_parts(harness.state()), first);
    assert_eq!(harness.state().entries[0].label, label);
    // A part set again with a new seed: set anew, or as it was where no attempt sets it.
    harness
        .get_by_label("kyrie/2 · Christe (Imitation) → A")
        .scroll_to_me();
    harness.run();
    harness
        .get_by_label("kyrie/2 · Christe (Imitation) → A")
        .click();
    harness.run();
    harness.get_by_label("Teil neu setzen").scroll_to_me();
    harness.run();
    harness.get_by_label("Teil neu setzen").click();
    harness.step();
    assert!(harness.state().mass_busy());
    wait_for(&mut harness, 120, "the part", |app| !app.mass_busy());
    let state = harness.state();
    let again = mass_parts(state);
    assert_eq!((again[0], again[2]), (first[0], first[2]));
    match &state.message {
        Some((false, message)) => {
            assert_eq!(message, "Messe gesetzt: 3 von 3 Teilen.");
            assert_ne!(again[1].unwrap().0, first[1].unwrap().0);
        }
        other => {
            assert!(
                other
                    .as_ref()
                    .is_some_and(|(_, m)| m.starts_with("Teil kyrie/2 bleibt, wie er war.")),
                "{other:?}"
            );
            assert_eq!(again[1], first[1]);
        }
    }
}

/// The head motif in the window (MASS-MOTIF): a Kyrie and an Agnus Dei in two voices, every part
/// in imitation, with a narrow search, which takes seconds. The list of parts names Kyrie I,
/// which gives the motif, and Agnus Dei I, which opens with it. Kyrie I of seed 1 gives none: its
/// line says so, and Agnus Dei I takes its subject from its words, with a warning in the list of
/// parts, over the results and with the violations of its movement. „Teil neu setzen“ on Kyrie I
/// sets Agnus Dei I again after it, as `fux mass --only kyrie/1` does, with the motif Kyrie I
/// gives now; the other parts stay as they were.
#[test]
fn a_mass_with_the_head_motif_in_the_window() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Mass;
    app.mass.movements = [true, false, false, false, false, true];
    app.mass.voices = 2;
    app.mass.imitation = "an".into();
    app.mass.lengths = fux_core::mass::Lengths::Natural;
    app.mass.motif = true;
    app.mass.beam = 40;
    app.mass.random_seed = false;
    app.mass.seed = 1;
    let mut harness = harness(app);
    harness.get_by_label("Messe setzen").scroll_to_me();
    harness.run();
    harness.get_by_label("Messe setzen").click();
    harness.step();
    assert!(harness.state().mass_busy());
    wait_for(&mut harness, 120, "the mass", |app| !app.mass_busy());
    let state = harness.state();
    let run = state.mass_run().unwrap();
    assert_eq!(run.set_count(), 5, "{:?}", state.message);
    let first = mass_parts(state);
    let (kyrie, agnus) = (&run.mass.results[0], &run.mass.results[3]);
    assert!(kyrie.as_ref().unwrap().motif.is_none() && agnus.as_ref().unwrap().motif.is_none());
    let warning = fux_core::generate::Warning::NoMotif {
        movement: "Agnus Dei".into(),
    }
    .message(fux_core::Lang::De);
    let warning = format!("Warnung: {warning}");
    assert!(state.summary.contains(&warning), "{:?}", state.summary);
    let kyrie_i = "kyrie/1 · Kyrie I (Imitation, gibt das Kopfmotiv) → D";
    harness.get_by_label(kyrie_i);
    harness.get_by_label("agnus/1 · Agnus Dei I (Imitation, Kopfmotiv) → D");
    harness.get_by_label_contains(" · kein Kopfmotiv");
    harness.get_by_label_contains(" · 1 Warnung");
    // The violations of the Agnus Dei, part by part: the warning with Agnus Dei I.
    assert_eq!(harness.query_all_by_label(&warning).count(), 1);
    let agnus_dei = harness.state().entries[1].label.clone();
    harness.get_by_label(&agnus_dei).click();
    harness.run();
    assert_eq!(harness.query_all_by_label(&warning).count(), 2);
    // Kyrie I set again: with Agnus Dei I, which takes the motif it gives now.
    harness.get_by_label(kyrie_i).scroll_to_me();
    harness.run();
    harness.get_by_label(kyrie_i).click();
    harness.run();
    harness.get_by_label("Teil neu setzen").scroll_to_me();
    harness.run();
    harness.get_by_label("Teil neu setzen").click();
    harness.step();
    assert!(harness.state().mass_busy());
    let (_, _, total) = harness.state().mass_run().unwrap().progress().unwrap();
    assert_eq!(total, 2);
    wait_for(&mut harness, 120, "Kyrie I and Agnus Dei I", |app| {
        !app.mass_busy()
    });
    let state = harness.state();
    let run = state.mass_run().unwrap();
    let again = mass_parts(state);
    assert_eq!(
        [again[1], again[2], again[4]],
        [first[1], first[2], first[4]]
    );
    let (kyrie, agnus) = (&run.mass.results[0], &run.mass.results[3]);
    let motif = &kyrie.as_ref().unwrap().motif;
    assert_eq!(&agnus.as_ref().unwrap().motif, motif);
    match &state.message {
        Some((false, message)) => {
            assert_eq!(message, "Messe gesetzt: 5 von 5 Teilen.");
            assert_ne!(again[0], first[0]);
        }
        // No attempt set Kyrie I with its new seed: it stays, and so does Agnus Dei I.
        other => {
            assert!(
                other
                    .as_ref()
                    .is_some_and(|(_, m)| m.starts_with("Teil kyrie/1 bleibt, wie er war.")),
                "{other:?}"
            );
            assert_eq!((again[0], again[3]), (first[0], first[3]));
        }
    }
}

/// A part whose search failed is not searched again with its seed, which would fail again
/// (MASS-SEED): a Kyrie in two voices with a narrow search under a rule made a prohibition
/// (HAR-PERFECT-COST), which no attempt of any part passes. Set one part at a time and cancelled
/// once its first part has failed, the mass names that part, and „Fehlende Teile setzen“ sets the
/// parts cancelled, not it; once they have failed too, the button sets nothing, and the window
/// points to „Teil neu setzen“.
#[test]
fn a_part_that_failed_is_not_set_again_with_its_seed() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Mass;
    app.mass_jobs = 1;
    app.mass.movements = [true, false, false, false, false, false];
    app.mass.voices = 2;
    app.mass.beam = 20;
    app.mass.random_seed = false;
    app.mass.seed = 1;
    app.profile
        .apply_override("HAR-PERFECT-COST.level=V")
        .unwrap();
    let mut harness = harness(app);
    harness.get_by_label("Messe setzen").scroll_to_me();
    harness.run();
    harness.get_by_label("Messe setzen").click();
    harness.step();
    assert!(harness.state().mass_busy());
    wait_for(&mut harness, 300, "the first part to fail", |app| {
        let run = app.mass_run().unwrap();
        matches!(run.states[0], PartState::Failed(_))
    });
    harness.get_by_label("Abbrechen").click();
    wait_for(&mut harness, 60, "the cancel", |app| !app.mass_busy());
    let state = harness.state();
    let run = state.mass_run().unwrap();
    assert_eq!(run.set_count(), 0);
    let failed: Vec<String> = run
        .failed()
        .into_iter()
        .map(|at| run.plan().address(at))
        .collect();
    let resumable = run.resumable().len();
    assert!(failed[0] == "kyrie/1" && resumable > 0, "{failed:?}");
    let failed = failed.join(", ");
    assert_eq!(
        state.message,
        Some((
            true,
            format!(
                "Messe unvollständig: 0 von 3 Teilen gesetzt; gescheitert: {failed}. „Fehlende \
                 Teile setzen“ setzt die übrigen, „Teil neu setzen“ einen gescheiterten mit \
                 einem anderen Seed."
            )
        ))
    );
    let totals = state.summary.last().unwrap();
    assert!(
        totals.starts_with("Unvollständig, es fehlen: kyrie/1 (gescheitert), ")
            && totals.contains("„Fehlende Teile setzen“ setzt die nicht gescheiterten"),
        "{totals}"
    );
    // The parts cancelled are searched, not the one that failed.
    harness.get_by_label("Fehlende Teile setzen").scroll_to_me();
    harness.run();
    let button = harness.get_by_label("Fehlende Teile setzen");
    assert!(!button.accesskit_node().is_disabled());
    button.click();
    harness.step();
    assert!(harness.state().mass_busy());
    let (_, _, total) = harness.state().mass_run().unwrap().progress().unwrap();
    assert_eq!(total, resumable);
    wait_for(&mut harness, 300, "the parts cancelled", |app| {
        !app.mass_busy()
    });
    let state = harness.state();
    assert_eq!(state.mass_run().unwrap().failed().len(), 3);
    assert_eq!(
        state.message,
        Some((
            true,
            "Messe unvollständig: 0 von 3 Teilen gesetzt; gescheitert: kyrie/1, kyrie/2, \
             kyrie/3. „Teil neu setzen“ versucht einen mit einem anderen Seed."
                .to_string()
        ))
    );
    // Only parts that failed are missing: the button sets none.
    harness.get_by_label("Fehlende Teile setzen").scroll_to_me();
    harness.run();
    let button = harness.get_by_label("Fehlende Teile setzen");
    assert!(button.accesskit_node().is_disabled());
    harness.state_mut().set_missing_parts();
    assert!(!harness.state().mass_busy());
}

/// The chant and the head motif in the form of the mass: a chant gives each movement the mode
/// of its chant („nach dem Choral“) unless a mode is chosen, and the plan shows it; the head
/// motif with a chant is refused before any search, in the user's words.
#[test]
fn the_mass_form_offers_chant_and_motif() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Mass;
    app.mass.random_seed = false;
    let mut harness = harness(app);
    harness.get_by_value("dorisch");
    harness.get_by_label("Choral:").click();
    harness.run();
    harness.get_by_label("IX (Cum jubilo)").click();
    harness.run();
    assert_eq!(
        harness.state().mass.chant,
        Some(fux_core::mass::ChantMass::IX)
    );
    harness.get_by_value("nach dem Choral");
    harness.get_by_label("Plan zeigen").scroll_to_me();
    harness.run();
    harness.get_by_label("Plan zeigen").click();
    harness.run();
    let state = harness.state();
    assert!(
        state.summary[0].starts_with("Missa brevis – Modus je Satz, S A T B, Seed 1: 6 Sätze"),
        "{:?}",
        state.summary
    );
    // One mode for the mass after all.
    harness.get_by_label("Modus:").click();
    harness.run();
    harness.get_by_label("phrygisch").click();
    harness.run();
    assert!(!harness.state().mass.chant_mode);
    harness.get_by_value("phrygisch");
    // The head motif and the chant exclude each other.
    harness.get_by_label("Kopfmotiv").click();
    harness.run();
    harness.get_by_label("Kopfmotiv und Choralparaphrase schließen einander aus.");
    harness.get_by_label("Messe setzen").scroll_to_me();
    harness.run();
    harness.get_by_label("Messe setzen").click();
    harness.run();
    let state = harness.state();
    assert!(!state.mass_busy() && state.mass_run().is_none());
    assert_eq!(
        state.message,
        Some((
            true,
            "Kopfmotiv und Choralparaphrase schließen einander aus.".to_string()
        ))
    );
}

/// A whole mass in the window: the form normal for A T B with a narrow search, whose Benedictus
/// ends with the Osanna of the Sanctus („Osanna ut supra“). Every movement is a result with the
/// pages of its book and its sound; exported, the mass is the book and a file per movement, each
/// taken apart into its parts again by the check.
#[test]
#[ignore = "sets a whole mass of 27 parts: minutes"]
fn a_whole_mass_in_the_window() {
    let mut app = FuxApp::headless();
    app.tab = Tab::Mass;
    app.mass.preset = fux_core::mass::Preset::Normal;
    app.mass.voices = 3;
    app.mass.beam = 40;
    app.mass.random_seed = false;
    app.mass.seed = 1;
    let mut harness = harness(app);
    harness.state_mut().start_mass();
    let started = std::time::Instant::now();
    wait_for(&mut harness, 3600, "the mass", |app| !app.mass_busy());
    eprintln!("set in {:.0} s", started.elapsed().as_secs_f64());
    let state = harness.state();
    let run = state.mass_run().unwrap();
    assert_eq!(
        (run.set_count(), run.mass.results.len()),
        (27, 27),
        "{:?}",
        state.message
    );
    let titles: Vec<&str> = state
        .entries
        .iter()
        .map(|e| e.label.split(" (").next().unwrap())
        .collect();
    assert_eq!(
        titles,
        [
            "Kyrie",
            "Gloria",
            "Credo",
            "Sanctus",
            "Benedictus",
            "Agnus Dei"
        ]
    );
    let lang = fux_core::Lang::De;
    let options = fux_export::LilypondOptions {
        lang,
        ..fux_export::LilypondOptions::default()
    };
    let notation = fux_cli::mass::MassNotation::default();
    // A score per section, in every movement: Kyrie I, Christe, Kyrie II; Et in terra pax, Qui
    // tollis; Patrem, Crucifixus, Et in Spiritum; Sanctus, Pleni, Osanna; Benedictus, Osanna ut
    // supra; Agnus Dei I and II.
    for (movement, scores) in [3, 2, 3, 3, 2, 2].into_iter().enumerate() {
        assert_eq!(run.plan().movements[movement].sections.len(), scores);
        let book = run.movement_book(movement, &options, &notation, lang);
        assert_eq!(book.matches("\\score {").count(), scores, "{book}");
        assert!(!book.contains("fehlt"), "{book}");
        let midi = run
            .movement_midi(movement, &fux_export::MidiOptions::default(), lang)
            .unwrap();
        assert!(midi.starts_with(b"MThd"));
    }
    // The Benedictus prints the Osanna of the Sanctus after its own section.
    let benedictus = run.movement_book(4, &options, &notation, lang);
    assert!(benedictus.contains("Osanna ut supra"), "{benedictus}");
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("gui-missa");
    std::fs::remove_dir_all(&dir).ok();
    harness.state_mut().output = dir.join("missa").display().to_string();
    harness.state_mut().formats = vec![Format::Ly, Format::Midi, Format::Musicxml];
    harness.get_by_label("Exportieren").click();
    harness.run();
    let (error, written) = harness.state().message.clone().unwrap();
    assert!(!error, "{written}");
    for (n, id) in ["kyrie", "gloria", "credo", "sanctus", "benedictus", "agnus"]
        .into_iter()
        .enumerate()
    {
        let file = dir.join(format!("missa-{}-{id}.musicxml", n + 1));
        let xml = std::fs::read_to_string(&file).unwrap();
        let (read, fields) = fux_cli::input::musicxml_text_fields(&xml, None, lang).unwrap();
        let check = fux_cli::mass::check_movement(
            &read,
            &fields,
            None,
            &fux_core::rules::Profile::fux(),
            &[],
            fux_cli::free::Repeat::Auto,
            fux_cli::imitation::ImitationSource::File,
            lang,
        )
        .unwrap()
        .unwrap();
        let forbidden: usize = check.pieces.iter().map(|p| p.report.forbidden()).sum();
        assert_eq!(forbidden, 0, "{id}");
        assert!(dir.join(format!("missa-{}-{id}.mid", n + 1)).exists());
    }
    assert_eq!(
        std::fs::read_dir(dir.join("missa.teile")).unwrap().count(),
        27
    );
}

/// Renders the tab „Messe“ into `target/tmp/gui/mass-*.png`, to look at it: the form with its
/// table of sections, a Kyrie in three voices being set, and set, its book's pages by LilyPond
/// with the violations of its parts. Needs a graphics adapter and LilyPond:
/// `cargo test -p fux-gui --test ui mass_screenshots -- --ignored`.
#[test]
#[ignore]
fn mass_screenshots() {
    let shots = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("gui");
    std::fs::create_dir_all(&shots).unwrap();
    let mut app = FuxApp::headless().with_lilypond();
    app.tab = Tab::Mass;
    app.mass.movements = [true, false, false, false, false, false];
    app.mass.voices = 3;
    app.mass.beam = 40;
    app.mass.random_seed = false;
    app.mass.seed = 1;
    let mut harness = harness(app);
    harness.get_by_label("Abschnitte").click();
    harness.run();
    harness
        .render()
        .unwrap()
        .save(shots.join("mass-form.png"))
        .unwrap();
    harness.state_mut().start_mass();
    wait_for(&mut harness, 120, "the first part", |app| {
        app.mass_run().unwrap().mass.results[0].is_some()
    });
    harness.run_steps(2);
    harness
        .render()
        .unwrap()
        .save(shots.join("mass-running.png"))
        .unwrap();
    wait_for(&mut harness, 300, "the mass", |app| {
        !app.mass_busy() && app.preview_ready()
    });
    harness.run_steps(2);
    harness
        .render()
        .unwrap()
        .save(shots.join("mass-set.png"))
        .unwrap();
    // In English, the list of its parts in view.
    harness.get_by_label("English").click();
    harness.run();
    harness
        .get_by_label("kyrie/3 · Kyrie II (imitation) → D")
        .scroll_to_me();
    for _ in 0..300 {
        harness.step();
        if harness.state().preview_ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    harness.run_steps(2);
    harness
        .render()
        .unwrap()
        .save(shots.join("mass-english.png"))
        .unwrap();
    eprintln!("screenshots in {}", shots.display());
}

/// Whether Palestrina's masses are there (`python3 tools/fetch_palestrina.py`). Without them a
/// test that needs them says so and returns; with `FUX_REQUIRE_CORPUS` set their absence fails it.
fn corpus_present(path: &std::path::Path) -> bool {
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
fn a_movement_of_palestrina_is_checked() {
    let kern = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../references/datasets/palestrina-masses/kern/Kyrie_00.krn");
    if !corpus_present(&kern) {
        return;
    }
    let mut harness = harness(FuxApp::headless());
    harness.state_mut().start_job_file(kern);
    settle(&mut harness);
    let state = harness.state();
    let entry = &state.entries[0];
    assert!(entry.marked);
    assert_eq!(entry.score.voices.len(), 4);
    assert!(entry.score.is_free());
    assert_eq!(entry.report.forbidden(), 0, "{:?}", state.summary);
}
