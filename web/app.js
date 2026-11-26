// The page of the web version: forms for the tasks, the results with notes, playback and
// downloads, and the rule violations. The search runs in worker.js; the parts of a mass run in
// a pool of such workers side by side (pool.js).

import { order, Pool, poolSize, schedule } from "./pool.js";

const $ = (id) => document.getElementById(id);

// ---- texts ----

const TEXTS = {
  de: {
    tab_cp: "Kontrapunkt",
    tab_free: "Freier Satz",
    tab_check: "Prüfen",
    tab_example: "Fux' Beispiele",
    profile: "Regeln",
    cantus: "Cantus firmus",
    source_fux: "von Fux",
    source_notes: "eigene Noten",
    source_generate: "erzeugen",
    length: "Länge",
    mode: "Modus",
    from_cantus: "aus dem Cantus",
    cf_voice: "Stimmlage",
    auto: "automatisch",
    counterpoint: "Gegenstimmen",
    species: "Gattung",
    species_free: "frei",
    voices: "Stimmen",
    measures: "Takte",
    cadences: "Binnenschlüsse",
    free_help: "Alle Stimmen frei im Stil Palestrinas, ohne Cantus firmus; die Schlüsse als " +
      "Klauseln. Die Stimmen singen den gewählten Text; er bestimmt Länge und Binnenschlüsse, " +
      "solange sie automatisch sind. Binnenschlüsse: auto, none oder Takte wie 7, 12:F.",
    text: "Text",
    text_section: (s) => `${s.name} (${s.syllables} Silben, ${s.length} Takte)`,
    text_own: "Eigener Text …",
    text_none: "ohne Text",
    own_help: "Lateinisch, Silben mit Bindestrich und Betonung mit Akut, wo nötig " +
      "(le-vá-vi). Ein Komma beendet einen Sinnabschnitt, ein Punkt einen Satz; | markiert " +
      "eine Zäsur, / einen Schlussteil.",
    own_missing: "Bitte einen eigenen Text eingeben.",
    units: "Sinnabschnitte",
    repeat: "Wiederholung",
    repeat_help: "automatisch: kurze Texte (Kyrie, Sanctus, Agnus) frei, Gloria und Credo selten",
    repeat_auto: "automatisch",
    repeat_none: "keine",
    repeat_free: "frei",
    accents: "Akzente zeigen",
    eleison3: "e-léi-son (3 Silben)",
    breaks: "Zäsuren (|) nach",
    tails: "Schlussteile (/)",
    preview_legend: "Fett: betonte Silben. An einer Zäsur darf eine Stimme pausieren; ein " +
      "Schlussteil darf für sich wiederholt werden.",
    warning: "Warnung",
    planned: (n, auto) => auto ? `Länge automatisch: ${n} Takte` : `Länge: ${n} Takte`,
    long_hint: (n, v) =>
      `${n} Takte, ${v} Stimmen: Im Browser rechnet die Suche ohne Threads, etwa fünfmal ` +
      "langsamer als fux auf der Kommandozeile; das kann viele Minuten dauern.",
    long_confirm: (n, v) =>
      `Der Satz wird ${n} Takte lang, ${v}-stimmig. Im Browser rechnet die Suche ohne Threads, ` +
      "etwa fünfmal langsamer als fux auf der Kommandozeile; das kann viele Minuten dauern. " +
      "Schneller wird er mit weniger Stimmen, weniger Sinnabschnitten oder ohne " +
      "Wiederholung.\n\nTrotzdem setzen?",
    planned_imitation: (n) => `Länge nach dem Plan der Imitation: ${n} Takte (für den Seed des nächsten Satzes)`,
    imitation_auto: "automatisch",
    on: "an",
    off: "aus",
    all: "alle",
    imitates: "Die Stimmen imitieren einander: Punkt für Punkt nehmen sie den Kopf eines " +
      "Soggetto auf, geplant vor der Suche.",
    imitates_not: "Ohne Imitation (automatisch: mit Text, drei bis fünf Stimmen, " +
      "automatischen Binnenschlüssen und automatischer Länge).",
    subject_text: "aus dem Text",
    subject_kyriale: "Choral des Kyriale",
    subject_gabc: "Choral als GABC",
    subject_notes: "eigene Noten",
    gabc_help: "GABC-Text des Chorals, etwa von gregobase.selapa.net",
    notes_help: "Köpfe in LilyPond, | zwischen den Punkten",
    phrases: "Phrasen",
    chant_mode: "im Modus des Chorals",
    entries: "Punkte und Einsätze",
    points: "Punkte (0: auto)",
    later_voices: "Stimmen späterer Punkte",
    order: "Reihenfolge",
    intervals: "Intervalle",
    delays: "Abstände (Takte)",
    pairs: "Paare",
    restate: "Punkt 1 wiederholen",
    answer: "Antwort",
    head: "Kopf, Töne (0: auto)",
    entries_help: "Reihenfolge: die Stimmen jedes Punktes, der Führer zuerst, ; trennt die " +
      "Punkte, T1 und T2 bei gleichen Stimmlagen. Intervalle zum Führer: 0 gleiche Stufe, d " +
      "andere Hauptstufe (Quinte, Quarte), +n/-n Stufen. Abstände von Einsatz zu Einsatz in " +
      "Takten, Halbe erlaubt.",
    show_plan: "Plan zeigen",
    plan_shown: "Plan der Imitation, ohne Suche.",
    below: "zweistimmig: Gegenstimme unten",
    solutions: "Lösungen",
    count: "Anzahl",
    new_seed: "neuer Seed",
    run_cp: "Setzen",
    run_cf: "Erzeugen",
    use_cantus: "Als Cantus für den Kontrapunkt",
    check_cantus: "Cantus: Noten oder fux:NAME",
    check_others: "Weitere Stimmen, eine pro Zeile als TYP=NOTEN",
    check_file: "oder eine MusicXML- oder Humdrum-Datei (**kern)",
    check_text: "Text der Stimmen",
    check_text_help: "Für Liedtext in MusicXML: leer, um den Text zu nehmen, den eine Datei von " +
      "fux-kontrapunkt nennt, oder den Abschnitt oder Bereich des Ordinariums zu erkennen; sonst " +
      "ein Abschnitt (kyrie, gloria:1-5) oder ein eigener Text; none prüft ohne Textregeln.",
    check_repeat: "Wiederholung beim Erzeugen",
    run_check: "Prüfen",
    example: "Beispiel aus dem Gradus (1725)",
    run_example: "Zeigen",
    rules: "Regeln ändern",
    rules_help:
      "Eine Änderung pro Zeile: <code>ID=STUFE</code> oder <code>ID.PARAMETER=WERT</code>; " +
      "Stufen aus, P1–P4, V. Die Suche unten fügt eine Regel ein.",
    rule_search: "Regel suchen",
    play: "Abspielen",
    stop: "Stopp",
    original_clefs: "Fux' Schlüssel",
    entry_marks: "Einsätze markieren",
    as_text: "als Text",
    violations: "Verstöße",
    only_forbidden: "nur Verbote",
    credits: "Regelbasiert nach J. J. Fux, <i>Gradus ad Parnassum</i> (1725). " +
      'Notensatz: <a href="https://www.verovio.org">Verovio</a> ' +
      '(<a href="verovio/README.txt">LGPL</a>).',
    contact: "Kontakt",
    loading: "Lade das Programm …",
    working: "Rechne …",
    ready: "Bereit.",
    results: (n) => `${n} Ergebnis(se).`,
    no_violations: "Keine Verstöße.",
    no_notation: "Ohne Notenbild (Verovio nicht geladen):",
    loading_notation: "Lade den Notensatz …",
    no_file: "Datei nicht lesbar",
    tab_mass: "Messe",
    mass_help: "Eine ganze Messe in den Formen von Palestrinas Messen: Kyrie, Gloria, Credo, " +
      "Sanctus, Benedictus und Agnus Dei. Ein Plan teilt sie vor der Suche in Teile; mehrere " +
      "Worker setzen sie zugleich, die längsten zuerst, dann Satz für Satz.",
    mass_preset: "Form",
    preset_brevis: "Missa brevis",
    preset_normal: "normal",
    mass_mode_default: "dorisch (Standard)",
    mass_mode_chant: "nach dem Choral",
    mass_choir: "Besetzung",
    movements: "Sätze",
    chant_and_motif: "Choral und Kopfmotiv",
    chant: "Choral",
    chant_none: "keiner",
    chant_ix: "Messe IX (Cum jubilo)",
    chant_iv: "Kyrie IV, sonst Messe IX",
    motif: "Kopfmotiv des Kyrie I in Gloria, Credo, Sanctus und Agnus Dei I",
    sections: "Abschnitte",
    sections_help: "Stimmen, Neubeginn und Schlussstufe je Abschnitt und je Halteklang; „auto“ " +
      "wie geplant. Ein Abschnitt nennt bei der Schlussstufe seinen letzten Teil.",
    section_imitation: "Neubeginn",
    section_ending: "Schluss",
    imitative: "Imitation",
    together: "gemeinsam",
    ut_supra: "wie das Osanna des Sanctus",
    chunk: "Größe der Teile (Silben)",
    chunk_off: "ungeteilt",
    lengths: "Längen",
    lengths_palestrina: "wie bei Palestrina",
    lengths_natural: "natürlich",
    notation: "Notenbild",
    mensurstriche: "Mensurstriche",
    intonation: "Intonation vor Gloria und Credo",
    tacet: "pausierende Stimmen in den Dateien verbergen",
    title: "Titel",
    run_mass: "Messe setzen",
    no_movement: "Bitte mindestens einen Satz wählen.",
    about: (seconds) => {
      if (seconds < 59.5) return `etwa ${Math.max(1, Math.round(seconds))} s`;
      const minutes = Math.round(seconds / 60);
      return `etwa ${minutes} ${minutes === 1 ? "Minute" : "Minuten"}`;
    },
    mass_estimate: (measures, about, workers) =>
      `Geschätzt ${measures} Takte, im Browser ${about} mit ${workers} ` +
      `${workers === 1 ? "Worker" : "Workern"}.`,
    cancel: "Abbrechen",
    resume: "Fehlende Teile setzen",
    show_result: "Ergebnis zeigen",
    show_result_help: "Die Messe wieder zeigen, die vor dem Plan gesetzt war, mit ihren Teilen",
    save_book: "Buch (LilyPond)",
    part_waiting: "wartet",
    part_motif: "wartet auf das Kopfmotiv",
    part_unset: "nicht gesetzt",
    part_planned: (estimate) => `geschätzt ${estimate}`,
    part_running: (elapsed, estimate) => `läuft ${elapsed} von ≈ ${estimate}`,
    part_done: (measures, cost, time) => `${measures} Takte, Kosten ${cost}, ${time}`,
    part_warnings: (count) => (count === 1 ? "1 Warnung" : `${count} Warnungen`),
    part_takes_motif: "Kopfmotiv",
    part_gives_motif: "gibt das Kopfmotiv",
    part_no_motif: "kein Kopfmotiv",
    part_failed: "gescheitert",
    part_broken: "Worker abgestürzt",
    part_stopped: "abgebrochen",
    part_show: "Teil zeigen",
    part_alone: (label, address) =>
      `${label} (${address}) für sich, wie gesetzt: moderne Schlüssel und Klammern über den ` +
      "Einsätzen, Takte ab 1. Den Satz zeigt die Seite, sobald alle seine Teile gesetzt sind.",
    mass_running: (done, total, running, left) =>
      `${done} von ${total} Teilen fertig, ${running} in Arbeit – noch etwa ${left}`,
    mass_done: (done, total, time) => `Fertig: ${done} von ${total} Teilen in ${time}.`,
    mass_incomplete: (done, total) => `Unvollständig: ${done} von ${total} Teilen gesetzt.`,
    mass_cancelled: (done, total) =>
      `Abgebrochen: ${done} von ${total} Teilen gesetzt; „Fehlende Teile setzen“ setzt die übrigen.`,
    mass_planned: (parts) => `Plan der Messe, ohne Suche: ${parts} Teile.`,
    mass_join_failed: "Die Sätze ließen sich nicht zusammensetzen",
    toolbar_kept: "Die Ergebnisse zeigen noch die vorigen Schlüssel und Klammern.",
    remember: "Eingaben in diesem Browser merken",
    forget: "Gespeicherte Eingaben löschen",
    forgotten: "Gespeicherte Eingaben gelöscht.",
  },
  en: {
    tab_cp: "Counterpoint",
    tab_free: "Free counterpoint",
    tab_check: "Check",
    tab_example: "Fux's examples",
    profile: "Rules",
    cantus: "Cantus firmus",
    source_fux: "by Fux",
    source_notes: "own notes",
    source_generate: "generate",
    length: "Length",
    mode: "Mode",
    from_cantus: "from the cantus",
    cf_voice: "Voice",
    auto: "automatic",
    counterpoint: "Counterpoint",
    species: "Species",
    species_free: "free",
    voices: "Voices",
    measures: "Measures",
    cadences: "Internal cadences",
    free_help: "Every voice free in Palestrina's style, without a cantus firmus; the cadences " +
      "as clausulae. The voices sing the chosen text, which sets the length and the internal " +
      "cadences while they are automatic. Internal cadences: auto, none or measures such as " +
      "7, 12:F.",
    text: "Text",
    text_section: (s) => `${s.name} (${s.syllables} syllables, ${s.length} measures)`,
    text_own: "Own text …",
    text_none: "no text",
    own_help: "Latin, with hyphens between syllables and an acute on the stress where needed " +
      "(le-vá-vi). A comma ends a sense unit, a full stop a sentence; | marks a break, / a tail.",
    own_missing: "Please enter an own text.",
    units: "Sense units",
    repeat: "Repetition",
    repeat_help: "automatic: short texts (Kyrie, Sanctus, Agnus) freely, Gloria and Credo rarely",
    repeat_auto: "automatic",
    repeat_none: "none",
    repeat_free: "free",
    accents: "Show accents",
    eleison3: "e-léi-son (3 syllables)",
    breaks: "Breaks (|) after",
    tails: "Tails (/)",
    preview_legend: "Bold: stressed syllables. A voice may rest at a break; a tail may be " +
      "restated on its own.",
    warning: "Warning",
    planned: (n, auto) => auto ? `Automatic length: ${n} measures` : `Length: ${n} measures`,
    long_hint: (n, v) =>
      `${n} measures, ${v} voices: in the browser the search runs without threads, about five ` +
      "times slower than fux on the command line; this may take many minutes.",
    long_confirm: (n, v) =>
      `The setting will be ${n} measures long, in ${v} voices. In the browser the search runs ` +
      "without threads, about five times slower than fux on the command line; this may take " +
      "many minutes. Fewer voices, fewer sense units or no repetition make it faster." +
      "\n\nCompose anyway?",
    planned_imitation: (n) => `Length by the plan of the imitation: ${n} measures (for the seed of the next setting)`,
    imitation_auto: "automatic",
    on: "on",
    off: "off",
    all: "all",
    imitates: "The voices imitate each other: point by point they take up the head of a " +
      "subject, planned before the search.",
    imitates_not: "Without imitation (automatic: with a text, three to five voices, " +
      "automatic internal cadences and an automatic length).",
    subject_text: "from the text",
    subject_kyriale: "chant of the Kyriale",
    subject_gabc: "chant as GABC",
    subject_notes: "own notes",
    gabc_help: "The chant's GABC text, such as from gregobase.selapa.net",
    notes_help: "Heads in LilyPond, | between the points",
    phrases: "Phrases",
    chant_mode: "in the chant's mode",
    entries: "Points and entries",
    points: "Points (0: auto)",
    later_voices: "Voices of later points",
    order: "Order",
    intervals: "Intervals",
    delays: "Delays (measures)",
    pairs: "Pairs",
    restate: "Restate point 1",
    answer: "Answer",
    head: "Head, notes (0: auto)",
    entries_help: "Order: the voices of each point, the leader first, ; separates the points, " +
      "T1 and T2 for voices of one type. Intervals to the leader: 0 the same degree, d the " +
      "other main degree (fifth, fourth), +n/-n steps. Delays from entry to entry in measures, " +
      "halves allowed.",
    show_plan: "Show the plan",
    plan_shown: "Plan of the imitation, without a search.",
    below: "two voices: counterpoint below",
    solutions: "Solutions",
    count: "Count",
    new_seed: "new seed",
    run_cp: "Compose",
    run_cf: "Generate",
    use_cantus: "Use as cantus for counterpoint",
    check_cantus: "Cantus: notes or fux:NAME",
    check_others: "Further voices, one per line as TYPE=NOTES",
    check_file: "or a MusicXML or Humdrum file (**kern)",
    check_text: "Text of the voices",
    check_text_help: "For lyrics in MusicXML: empty to take the text a file of fux-kontrapunkt " +
      "names, or to recognise the section or range of the Ordinary; else a section (kyrie, " +
      "gloria:1-5) or an own text; none checks without the text rules.",
    check_repeat: "Repetition when composed",
    run_check: "Check",
    example: "Example from the Gradus (1725)",
    run_example: "Show",
    rules: "Change rules",
    rules_help: "One change per line: <code>ID=LEVEL</code> or <code>ID.PARAMETER=VALUE</code>; " +
      "levels aus (off), P1–P4, V (forbidden). The search below inserts a rule.",
    rule_search: "Search a rule",
    play: "Play",
    stop: "Stop",
    original_clefs: "Fux's clefs",
    entry_marks: "Mark the entries",
    as_text: "as text",
    violations: "Violations",
    only_forbidden: "prohibitions only",
    credits: "Rule-based after J. J. Fux, <i>Gradus ad Parnassum</i> (1725). " +
      'Engraving: <a href="https://www.verovio.org">Verovio</a> ' +
      '(<a href="verovio/README.txt">LGPL</a>).',
    contact: "Contact",
    loading: "Loading the program …",
    working: "Working …",
    ready: "Ready.",
    results: (n) => `${n} result(s).`,
    no_violations: "No violations.",
    no_notation: "No notation (Verovio not loaded):",
    loading_notation: "Loading the engraving …",
    no_file: "File not readable",
    tab_mass: "Mass",
    mass_help: "A whole mass in the forms of Palestrina's masses: Kyrie, Gloria, Credo, " +
      "Sanctus, Benedictus and Agnus Dei. A plan divides it into parts before the search; " +
      "several workers set them at once, the longest first, then movement by movement.",
    mass_preset: "Form",
    preset_brevis: "Missa brevis",
    preset_normal: "normal",
    mass_mode_default: "Dorian (default)",
    mass_mode_chant: "from the chant",
    mass_choir: "Choir",
    movements: "Movements",
    chant_and_motif: "Chant and head motif",
    chant: "Chant",
    chant_none: "none",
    chant_ix: "Mass IX (Cum jubilo)",
    chant_iv: "Kyrie IV, else Mass IX",
    motif: "Head motif of Kyrie I in Gloria, Credo, Sanctus and Agnus Dei I",
    sections: "Sections",
    sections_help: "Voices, restart and ending per section and per break; “auto” as planned. " +
      "A section's ending is that of its last part.",
    section_imitation: "Restart",
    section_ending: "Ending",
    imitative: "imitation",
    together: "together",
    ut_supra: "as the Osanna of the Sanctus",
    chunk: "Size of the parts (syllables)",
    chunk_off: "undivided",
    lengths: "Lengths",
    lengths_palestrina: "as Palestrina's",
    lengths_natural: "natural",
    notation: "Notation",
    mensurstriche: "Mensurstriche",
    intonation: "Intonation before Gloria and Credo",
    tacet: "hide resting voices in the files",
    title: "Title",
    run_mass: "Compose the mass",
    no_movement: "Please choose one movement at least.",
    about: (seconds) => {
      if (seconds < 59.5) return `about ${Math.max(1, Math.round(seconds))} s`;
      const minutes = Math.round(seconds / 60);
      return `about ${minutes} ${minutes === 1 ? "minute" : "minutes"}`;
    },
    mass_estimate: (measures, about, workers) =>
      `Estimated ${measures} measures, in the browser ${about} with ${workers} ` +
      `${workers === 1 ? "worker" : "workers"}.`,
    cancel: "Cancel",
    resume: "Set missing parts",
    show_result: "Show the result",
    show_result_help: "Show the mass set before the plan again, with its parts",
    save_book: "Book (LilyPond)",
    part_waiting: "waiting",
    part_motif: "waiting for the head motif",
    part_unset: "not set",
    part_planned: (estimate) => `estimated ${estimate}`,
    part_running: (elapsed, estimate) => `running ${elapsed} of ≈ ${estimate}`,
    part_done: (measures, cost, time) => `${measures} measures, cost ${cost}, ${time}`,
    part_warnings: (count) => (count === 1 ? "1 warning" : `${count} warnings`),
    part_takes_motif: "head motif",
    part_gives_motif: "gives the head motif",
    part_no_motif: "no head motif",
    part_failed: "failed",
    part_broken: "worker crashed",
    part_stopped: "cancelled",
    part_show: "Show the part",
    part_alone: (label, address) =>
      `${label} (${address}) on its own, as set: modern clefs and brackets over the entries, ` +
      "measures from 1. The page shows the movement once all its parts are set.",
    mass_running: (done, total, running, left) =>
      `${done} of ${total} parts done, ${running} in progress – about ${left} left`,
    mass_done: (done, total, time) => `Done: ${done} of ${total} parts in ${time}.`,
    mass_incomplete: (done, total) => `Incomplete: ${done} of ${total} parts set.`,
    mass_cancelled: (done, total) =>
      `Cancelled: ${done} of ${total} parts set; “Set missing parts” sets the others.`,
    mass_planned: (parts) => `Plan of the mass, without a search: ${parts} parts.`,
    mass_join_failed: "The movements could not be put together",
    toolbar_kept: "The results still show the former clefs and brackets.",
    remember: "Remember inputs in this browser",
    forget: "Delete saved inputs",
    forgotten: "Saved inputs deleted.",
  },
};

/** Texts that contain markup, all from TEXTS above. */
const MARKUP = new Set(["rules_help", "credits"]);

let lang = "de";

function t(key) {
  return TEXTS[lang][key] ?? TEXTS.de[key] ?? key;
}

function translate() {
  document.documentElement.lang = lang;
  for (const element of document.querySelectorAll("[data-t]")) {
    const key = element.dataset.t;
    if (MARKUP.has(key)) {
      element.innerHTML = t(key);
    } else {
      element.textContent = t(key);
    }
  }
  $("rule-search").placeholder = t("rule_search");
  $("free-repeat").title = t("repeat_help");
  $("mass-result").title = t("show_result_help");
  updatePlayButton();
  updateMassFields();
  showMassRun();
  showAlone(shownEntry());
}

// ---- the module in the worker ----

const worker = new Worker("worker.js", { type: "module" });
const pending = new Map();
let nextId = 0;

worker.onmessage = (event) => {
  const { id, answer } = event.data;
  pending.get(id)?.(answer);
  pending.delete(id);
};
worker.onerror = (event) => {
  setBusy(false);
  setStatus(event.message ?? "worker error", true);
};

/** The workers that set the parts of a mass side by side, each with a module of its own: as
 * many as the device has cores and memory for, at most four. */
const pool = new Pool(poolSize(navigator), () => new Worker("worker.js", { type: "module" }));

/** Sends a request with the common settings; resolves with the answer. */
function ask(request) {
  return new Promise((resolve) => {
    const id = nextId++;
    pending.set(id, resolve);
    worker.postMessage({
      id,
      request: {
        lang,
        profile: $("profile").value,
        rules: overrides(),
        ...toolbar(),
        tempo: Number($("tempo").value) || 72,
        ...request,
      },
    });
  });
}

/** The clefs and the brackets over the entries as the toolbar has them, which the files of every
 * result show. */
function toolbar() {
  return { original_clefs: $("original-clefs").checked, marks: $("entry-marks").checked };
}

function overrides() {
  return $("rule-overrides")
    .value.split("\n")
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#"));
}

// ---- status ----

function setStatus(text, error = false) {
  const status = $("status");
  status.textContent = text;
  status.classList.toggle("error", error);
}

function setBusy(busy, text) {
  document.body.classList.toggle("busy", busy);
  for (const button of document.querySelectorAll("button.primary")) {
    // A mass sets its parts in workers of its own, one mass at a time.
    button.disabled = busy || (button.form?.id === "form-mass" && pool.busy);
  }
  if (text) {
    setStatus(text);
  }
}

// ---- choices ----

let info = null;

function fillSelect(select, items, keepFirst = false) {
  const value = select.value;
  const first = keepFirst ? select.options[0] : null;
  select.replaceChildren(...(first ? [first] : []));
  for (const item of items) {
    select.add(new Option(item.name, item.id));
  }
  if ([...select.options].some((o) => o.value === value)) {
    select.value = value;
  }
}

async function loadInfo() {
  const reply = await ask({ command: "info", ...repetition() });
  if (!reply.ok) {
    setStatus(reply.error, true);
    return false;
  }
  info = reply.info;
  fillSelect($("cp-fux"), info.cantus);
  fillSelect($("cp-mode"), info.modes, true);
  fillSelect($("cf-mode"), info.modes);
  fillSelect($("free-mode"), info.modes);
  fillSelect($("mass-mode"), info.modes, true);
  fillSelect($("check-mode"), info.modes, true);
  fillSelect($("cp-cf-voice"), info.voices, true);
  fillSelect($("cf-voice"), info.voices);
  fillSelect($("example-figure"), info.examples);
  fillTexts();
  $("rule-list").replaceChildren(
    ...info.rules.map((rule) => new Option(`${rule.id} – ${rule.name}`, rule.id)),
  );
  return true;
}

// ---- the text of a free setting ----

/** The choices of the text select besides the sections of the Ordinary. */
const OWN = "own";
const NONE = "none";

/** Above this many measures, or from this many voices on, the page asks before it sets: in the
 * browser the search runs without threads, about five times slower than fux on the command line,
 * and five or six voices take minutes on short texts too. */
const LONG_SETTING = 30;
const LONG_VOICES = 5;

/** Whether a free setting of this many measures and voices takes long in the browser. */
function isLong(measures, voices) {
  return measures > LONG_SETTING || voices >= LONG_VOICES;
}

/** The sections with their syllables and lengths, then an own text and none. */
function fillTexts() {
  const select = $("free-text");
  const value = select.value;
  select.replaceChildren(
    ...info.texts.map((section) => new Option(t("text_section")(section), section.id)),
    new Option(t("text_own"), OWN),
    new Option(t("text_none"), NONE),
  );
  if ([...select.options].some((o) => o.value === value)) {
    select.value = value;
  }
}

/** The choices that change the syllables and lengths of the sections. */
function repetition() {
  return {
    repeat: $("free-repeat").value,
    eleison: $("free-eleison").checked ? 3 : 4,
  };
}

/** Lists the sections again with the syllables and lengths of the current choices. */
async function refreshTexts() {
  const reply = await ask({ command: "info", ...repetition() });
  if (reply.ok && info) {
    info.texts = reply.info.texts;
    fillTexts();
  }
}

function selectedSection() {
  return info?.texts.find((section) => section.id === $("free-text").value) ?? null;
}

/** What the voices sing, as the request names it: a section, some of its sense units, an own
 * text (empty if none is given yet) or none. */
function textSpec() {
  const choice = $("free-text").value;
  if (choice === OWN) return $("free-own").value.trim();
  if (choice === NONE) return NONE;
  const section = selectedSection();
  const units = section?.units ?? 1;
  const from = Number($("free-from").value) || 1;
  const to = Number($("free-to").value) || units;
  if (units > 1 && (from !== 1 || to !== units)) {
    return from === to ? `${choice}:${from}` : `${choice}:${from}-${to}`;
  }
  return choice;
}

/** The imitation as the free request takes it (`imitation`): where the subjects come from and
 * the choices of the points and their entries, each automatic unless set. */
function imitationChoices() {
  const subject = $("free-subject").value;
  return {
    mode: $("free-imitation-mode").value,
    subject: subject === "gabc"
      ? $("free-gabc").value
      : subject === "notes"
      ? $("free-notes").value
      : subject,
    phrases: isChant(subject) ? $("free-phrases").value : "",
    points: Number($("free-points").value) || 0,
    later_voices: $("free-later-voices").value,
    order: $("free-order").value,
    intervals: $("free-intervals").value,
    delays: $("free-delays").value,
    pairs: $("free-pairs").value,
    restate: $("free-restate").value,
    answer: $("free-answer").value,
    head: Number($("free-head").value) || 0,
  };
}

/** Whether the subjects come from a chant. */
function isChant(subject) {
  return subject === "kyriale" || subject === "gabc";
}

/** Whether a free setting imitates: always with `an`, never with `aus`, automatically where
 * every voice sings a text, with three to five voices, automatic internal cadences and an
 * automatic length. */
function imitates(choices) {
  const mode = $("free-imitation-mode").value;
  if (mode !== "auto") return mode === "an";
  const cadences = choices.cadences.trim().toLowerCase();
  return choices.text !== NONE && choices.text !== "" && choices.voices >= 3 &&
    choices.voices <= 5 && (cadences === "" || cadences === "auto") && !choices.length;
}

/** The choices of the free form that a setting and the preview of its text share. A chant's
 * subjects are sung in the chant's mode if the form asks for it: the mode is left empty. */
function freeChoices() {
  const choices = {
    mode: $("free-mode").value,
    voices: Number($("free-voices").value),
    length: $("free-length-auto").checked ? 0 : Number($("free-length").value),
    cadences: $("free-cadences").value,
    text: textSpec(),
    accents: $("free-accents").checked,
    imitation: imitationChoices(),
    ...repetition(),
  };
  if (imitates(choices) && isChant($("free-subject").value) && $("free-chant-mode").checked) {
    choices.mode = "";
  }
  return choices;
}

/** Shows the fields that the chosen text and the imitation need. */
function updateTextFields() {
  const choice = $("free-text").value;
  const units = selectedSection()?.units ?? 1;
  $("free-own-box").hidden = choice !== OWN;
  $("free-units-box").hidden = units <= 1;
  $("free-from").max = units;
  $("free-to").max = units;
  $("free-length").disabled = $("free-length-auto").checked;
  const subject = $("free-subject").value;
  $("free-imitation-box").hidden = $("free-imitation-mode").value === "aus";
  $("free-gabc-box").hidden = subject !== "gabc";
  $("free-notes-box").hidden = subject !== "notes";
  $("free-chant-box").hidden = !isChant(subject);
  // Later points have two voices up to all the choir has.
  const voices = Number($("free-voices").value) || 4;
  for (const option of $("free-later-voices").options) {
    const count = Number(option.value);
    option.disabled = count > voices;
  }
  if ($("free-later-voices").selectedOptions[0]?.disabled) {
    $("free-later-voices").value = "auto";
  }
  const imitating = imitates(freeChoices());
  $("free-imitates").textContent = imitating ? t("imitates") : t("imitates_not");
  $("free-plan").disabled = !imitating;
  // A chant's subjects in the chant's mode: the mode chosen does not count then.
  $("free-mode").disabled = imitating && isChant(subject) && $("free-chant-mode").checked;
}

let previewTimer = 0;
let previewTicket = 0;

function schedulePreview() {
  clearTimeout(previewTimer);
  previewTimer = setTimeout(updatePreview, 250);
}

/** The replies to `text` requests by their choices and seed, the latest few. */
const textReplies = new Map();

/**
 * The reply to a `text` request for these choices and seed: planning an imitation takes seconds
 * for long texts, so the same request is answered once.
 */
async function textReply(choices, seed) {
  const key = JSON.stringify([choices, seed]);
  if (!textReplies.has(key)) {
    const reply = await ask({ command: "text", ...choices, seed });
    textReplies.set(key, reply);
    if (textReplies.size > 32) {
      textReplies.delete(textReplies.keys().next().value);
    }
  }
  return textReplies.get(key);
}

/** A preview is being asked for; another change meanwhile asks again once it is answered. */
let previewing = false;
let previewAgain = false;

/** Previews the text with its stressed syllables and the planned length. */
async function updatePreview() {
  clearTimeout(previewTimer);
  if (!info) return;
  updateTextFields();
  // One preview at a time: the worker answers in turn, and a setting waits behind each.
  if (previewing) {
    previewAgain = true;
    return;
  }
  const ticket = ++previewTicket;
  const choices = freeChoices();
  if (!choices.text) {
    showPreview(null, choices);
    return;
  }
  previewing = true;
  try {
    // The plan of an imitation depends on the seed: that of the next run.
    const reply = await textReply(choices, freeSeed());
    // A newer preview may have been asked for meanwhile.
    if (ticket === previewTicket) {
      showPreview(reply, choices);
    }
  } finally {
    previewing = false;
  }
  if (previewAgain) {
    previewAgain = false;
    updatePreview();
  }
}

function paragraph(text, className) {
  const element = document.createElement("p");
  if (className) element.className = className;
  element.textContent = text;
  return element;
}

/** Syllables as the text command gives them, the stressed ones in bold. */
function syllableNodes(syllables) {
  return syllables.flatMap((syllable) => {
    if (!syllable.stress) return [syllable.text + syllable.after];
    const bold = document.createElement("b");
    bold.textContent = syllable.text;
    return [bold, syllable.after];
  });
}

function syllableLines(label, lines, separator) {
  const element = document.createElement("p");
  element.append(`${label}: `);
  lines.forEach((line, i) => {
    if (i > 0) element.append(separator);
    element.append(...syllableNodes(line));
  });
  return element;
}

function showPreview(reply, choices) {
  const box = $("free-preview");
  if (!reply) {
    box.replaceChildren(paragraph(t("own_missing"), "hint"));
    showPlanned(null);
    return;
  }
  if (!reply.ok) {
    box.replaceChildren(paragraph(reply.error, "error"));
    showPlanned(null);
    return;
  }
  const parts = [];
  const text = reply.text;
  if (text) {
    parts.push(paragraph(`${text.label}, ${text.kind}`, "head"));
    const units = document.createElement("ol");
    units.append(
      ...text.units.map((unit) => {
        const item = document.createElement("li");
        item.append(...syllableNodes(unit));
        return item;
      }),
    );
    parts.push(units);
    if (text.breaks.length) parts.push(syllableLines(t("breaks"), text.breaks, ", "));
    if (text.tails.length) parts.push(syllableLines(t("tails"), text.tails, " · "));
    for (const warning of text.warnings) {
      parts.push(paragraph(`${t("warning")}: ${warning}`, "warning"));
    }
    parts.push(paragraph(t("preview_legend"), "hint"));
  }
  const plan = reply.plan ?? {};
  if (plan.measures && plan.imitation) {
    parts.push(paragraph(t("planned_imitation")(plan.measures)));
  } else if (plan.measures) {
    parts.push(paragraph(t("planned")(plan.measures, choices.length === 0)));
  }
  if (plan.problem) parts.push(paragraph(plan.problem, "warning"));
  box.replaceChildren(...parts);
  showPlanned(plan.measures ?? null);
}

/** The planned length: in the length field while it is automatic, and a warning if it is long
 * for the browser. */
function showPlanned(measures) {
  if (measures && $("free-length-auto").checked) {
    $("free-length").value = measures;
  }
  const voices = Number($("free-voices").value);
  const long = $("free-long");
  long.hidden = !(measures && isLong(measures, voices));
  long.textContent = long.hidden ? "" : t("long_hint")(measures, voices);
}

$("free-text").addEventListener("change", () => {
  // A new section is sung whole.
  $("free-from").value = 1;
  $("free-to").value = selectedSection()?.units ?? 1;
  updateTextFields();
  if ($("free-text").value === OWN) {
    $("free-own").focus();
  }
});
$("free-length-auto").addEventListener("change", updateTextFields);
$("form-free").addEventListener("input", schedulePreview);
$("form-free").addEventListener("change", schedulePreview);
for (const id of ["free-repeat", "free-eleison", "profile", "rule-overrides"]) {
  // The syllables and lengths of the sections follow these.
  $(id).addEventListener("change", refreshTexts);
}
$("profile").addEventListener("change", schedulePreview);
$("rule-overrides").addEventListener("change", schedulePreview);

// ---- remembered inputs ----

// The page remembers its inputs in this browser only when asked to (decided 2026-10-27): without
// „Eingaben merken“ nothing is written to the storage of the browser, and turning it off deletes
// what was written. The choice itself is stored only while it is on, with the inputs.
const STORE = "fux-kontrapunkt";
const remembered = () =>
  document.querySelectorAll(
    "form input, form select, form textarea, header select, #rules textarea, .toolbar input",
  );

function save() {
  if (!$("remember").checked) return;
  const values = {};
  for (const element of remembered()) {
    if (!element.id && element.type !== "radio") continue;
    if (element.type === "file") continue;
    if (element.type === "radio") {
      if (element.checked) values[`radio:${element.name}`] = element.value;
    } else if (element.type === "checkbox") {
      values[element.id] = element.checked;
    } else {
      values[element.id] = element.value;
    }
  }
  values.tab = currentTab;
  values.remember = true;
  try {
    localStorage.setItem(STORE, JSON.stringify(values));
  } catch {
    // Without storage the page forgets its inputs; that is all.
  }
}

/** Deletes the inputs this browser keeps. */
function forget() {
  try {
    localStorage.removeItem(STORE);
  } catch {
    // Nothing kept, or nothing to reach.
  }
}

/** The inputs this browser keeps because the page was asked to; inputs that the page kept before
 * it asked (without `remember`) are deleted, not restored. */
function stored() {
  let values = null;
  try {
    values = JSON.parse(localStorage.getItem(STORE) ?? "null");
  } catch {
    values = null;
  }
  if (values && values.remember !== true) {
    forget();
    return {};
  }
  return values ?? {};
}

function restore() {
  const values = stored();
  $("remember").checked = values.remember === true;
  for (const element of remembered()) {
    if (element.type === "radio") {
      const value = values[`radio:${element.name}`];
      if (value !== undefined) element.checked = element.value === value;
    } else if (element.id in values && element.type !== "file") {
      if (element.type === "checkbox") {
        element.checked = Boolean(values[element.id]);
      } else {
        element.value = values[element.id];
      }
    }
  }
  return values;
}

// ---- tasks ----

let currentTab = "cp";

function showTab(tab) {
  currentTab = tab;
  for (const button of document.querySelectorAll("#tabs button")) {
    button.classList.toggle("active", button.dataset.tab === tab);
  }
  for (const form of document.querySelectorAll("form.task")) {
    form.hidden = form.dataset.tab !== tab;
  }
  showView(tab === "mass" ? "mass" : "other");
}

function drawSeed() {
  return Math.floor(Math.random() * 1_000_000);
}

function seed(prefix) {
  const field = $(`${prefix}-seed`);
  if ($(`${prefix}-new-seed`).checked) {
    field.value = drawSeed();
  }
  return Number(field.value) || 0;
}

/**
 * The seed of the next free setting where each one takes a new seed: drawn in advance, so that
 * the length preview and „Plan zeigen“ describe the setting the next „Setzen“ makes.
 */
let pendingSeed = drawSeed();

/** The seed of the next free setting, or plan: the pending one, or the one in the field. */
function freeSeed() {
  return $("free-new-seed").checked ? pendingSeed : Number($("free-seed").value) || 0;
}

let lastRequest = null;
/** The planned measures of the last free setting, which a run again asks about as well. */
let lastMeasures = 0;
/** The clefs and brackets of the toolbar (as JSON) that the results of the other tabs show, and
 * those their last request was made with. */
let otherToolbar = null;
let askedToolbar = null;

async function run(request) {
  stop();
  lastRequest = request;
  setBusy(true, t("working"));
  const started = performance.now();
  const made = JSON.stringify(toolbar());
  askedToolbar = made;
  const reply = await ask(request);
  setBusy(false);
  if (!reply.ok) {
    setStatus(reply.error, true);
    return;
  }
  const seconds = ((performance.now() - started) / 1000).toFixed(1);
  if (request.imitation?.plan_only) {
    setStatus(`${t("plan_shown")} (${seconds} s)`);
  } else {
    setStatus(`${t("results")(reply.results.length)} (${seconds} s)`);
  }
  otherToolbar = made;
  present("other", reply, request.command);
}

$("form-cp").onsubmit = (event) => {
  event.preventDefault();
  const source = document.querySelector("input[name=source]:checked").value;
  run({
    command: "cp",
    cf: source === "fux" ? $("cp-fux").value : source === "notes" ? $("cp-notes").value : "",
    mode: $("cp-mode").value,
    cf_voice: $("cp-cf-voice").value,
    length: Number($("cp-length").value),
    species: Number($("cp-species").value),
    voices: Number($("cp-voices").value),
    below: $("cp-below").checked,
    count: Number($("cp-count").value),
    seed: seed("cp"),
  });
};

/** A free setting is being planned before it is set. */
let planning = false;

$("form-free").onsubmit = async (event) => {
  event.preventDefault();
  if (planning || document.body.classList.contains("busy")) return;
  const choices = freeChoices();
  if (!choices.text) {
    setStatus(t("own_missing"), true);
    $("free-own").focus();
    return;
  }
  // Long settings take minutes in the browser: the page asks first. An imitation's length is
  // that of its plan for the seed of this run, the one a plan shown before had.
  const runSeed = freeSeed();
  $("free-seed").value = runSeed;
  planning = true;
  const planned = await textReply(choices, runSeed);
  planning = false;
  const measures = planned.plan?.measures ?? 0;
  if (isLong(measures, choices.voices) && !confirm(t("long_confirm")(measures, choices.voices))) {
    return;
  }
  lastMeasures = measures;
  // The next setting takes a new seed again, which the preview then shows.
  pendingSeed = drawSeed();
  schedulePreview();
  run({
    command: "free",
    ...choices,
    count: Number($("free-count").value),
    seed: runSeed,
  });
};

$("free-plan").onclick = () => {
  if (planning || document.body.classList.contains("busy")) return;
  const choices = freeChoices();
  // The plan of the next setting: „Setzen“ keeps its seed.
  const planSeed = freeSeed();
  $("free-seed").value = planSeed;
  run({
    command: "free",
    ...choices,
    imitation: { ...choices.imitation, plan_only: true },
    seed: planSeed,
  });
};

$("form-cf").onsubmit = (event) => {
  event.preventDefault();
  run({
    command: "cf",
    mode: $("cf-mode").value,
    length: Number($("cf-length").value),
    cf_voice: $("cf-voice").value,
    count: Number($("cf-count").value),
    seed: seed("cf"),
  });
};

$("form-check").onsubmit = async (event) => {
  event.preventDefault();
  const file = $("check-file").files[0];
  let musicxml = "";
  let kern = "";
  if (file) {
    try {
      const text = await file.text();
      if (file.name.toLowerCase().endsWith(".krn")) {
        kern = text;
      } else {
        musicxml = text;
      }
    } catch {
      setStatus(`${t("no_file")}: ${file.name}`, true);
      return;
    } finally {
      // Used once: the next check takes the notes above again.
      $("check-file").value = "";
    }
  }
  run({
    command: "check",
    cf: $("check-cantus").value,
    others: $("check-others").value.split("\n"),
    mode: $("check-mode").value,
    musicxml,
    kern,
    text: $("check-text").value,
    repeat: $("check-repeat").value,
  });
};

$("form-example").onsubmit = (event) => {
  event.preventDefault();
  run({ command: "example", figure: $("example-figure").value });
};
$("example-figure").ondblclick = () => $("form-example").requestSubmit();

$("cf-use").onclick = () => {
  const entry = answer?.results[selected];
  if (!entry) return;
  const notes = entry.lines[0].slice(entry.lines[0].indexOf(":") + 1).replaceAll("|", " ");
  $("cp-notes").value = notes.trim().replace(/\s+/g, " ");
  document.querySelector("input[name=source][value=notes]").checked = true;
  $("cp-mode").value = $("cf-mode").value;
  $("cp-cf-voice").value = $("cf-voice").value;
  showTab("cp");
  save();
};

// ---- the mass ----

/** The movements by their ids, in the order of the mass. */
const MOVEMENTS = ["kyrie", "gloria", "credo", "sanctus", "benedictus", "agnus"];

/** The usual choirs by their number of voices, as `fux mass` takes them. */
const CHOIRS = { 2: "S T", 3: "A T B", 4: "S A T B", 5: "S A T T B", 6: "S S A T T B" };

/**
 * The mass shown in the tab „Messe“: the form and the settings every request of it carries, its
 * plan (`view`, as mass_plan gives it), the state of each part, the motif of Kyrie I once known,
 * and the files of its movements and its book (`joined`, as mass_join gives them).
 */
let massRun = null;

/** The mass that „Plan zeigen“ put a plan in place of, with the results it showed (`view`):
 * „Ergebnis zeigen“ shows it again; a new mass ends it. */
let keptMass = null;

/** A mass is being planned before it is set. */
let massStarting = false;

/** The seed of the next mass where each takes a new seed: drawn in advance, as that of a free
 * setting, so that the plan and the estimate describe the mass that „Messe setzen“ sets. */
let pendingMassSeed = drawSeed();

function massSeed() {
  return $("mass-new-seed").checked ? pendingMassSeed : Number($("mass-seed").value) || 0;
}

/** The rows of the table of sections that the last plan gave. */
let massRows = [];

/** The choices of the table of sections by row id, each `{voices, imitation, ending}` with what
 * differs from „auto“; kept in a hidden field of the form, so that they are remembered with it. */
function sectionChoices() {
  try {
    const choices = JSON.parse($("mass-section-choices").value || "{}");
    return choices && typeof choices === "object" && !Array.isArray(choices) ? choices : {};
  } catch {
    return {};
  }
}

function setSectionChoices(choices) {
  $("mass-section-choices").value = JSON.stringify(choices);
  save();
}

/** What the files of a mass print; it applies to the mass shown as well. */
function massNotation() {
  return {
    title: $("mass-title").value.trim(),
    mensurstriche: $("mass-mensurstriche").checked,
    intonation: $("mass-intonation").checked,
    hide_tacet: $("mass-tacet").checked,
  };
}

/** The mass as the form gives it, without its seed; `null` without a movement. The rows of the
 * table go in the order of the plan, a Break after its section. */
function massForm() {
  const movements = MOVEMENTS.filter((id) => $(`mass-${id}`).checked);
  if (movements.length === 0) return null;
  const choices = sectionChoices();
  const ids = massRows.map((row) => row.id);
  const place = (id) => (ids.includes(id) ? ids.indexOf(id) : ids.length);
  const motif = $("mass-motif");
  return {
    preset: $("mass-preset").value,
    movements: movements.join(","),
    voices: Number($("mass-voices").value),
    parts: $("mass-parts").value.trim(),
    mode: $("mass-mode").value,
    sections: Object.keys(choices)
      .sort((a, b) => place(a) - place(b))
      .map((id) => ({ id, ...choices[id] })),
    chunk: $("mass-chunk-off").checked ? "aus" : String(Number($("mass-chunk").value) || 45),
    lengths: $("mass-lengths").value,
    chant: $("mass-chant").value,
    motif: motif.checked && !motif.disabled,
    repeat: $("mass-repeat").value,
    accents: $("mass-accents").checked,
    eleison: $("mass-eleison").checked ? 3 : 4,
    ...massNotation(),
  };
}

/** Fits the fields of the form to each other: the mode „nach dem Choral“ with a chant, the head
 * motif without a chant and with the Kyrie, the usual choir of the voices chosen. */
function updateMassFields() {
  const chant = $("mass-chant").value !== "keiner";
  $("mass-mode").options[0].textContent = chant ? t("mass_mode_chant") : t("mass_mode_default");
  $("mass-motif").disabled = chant || !$("mass-kyrie").checked;
  $("mass-parts").placeholder = CHOIRS[$("mass-voices").value] ?? CHOIRS[4];
  $("mass-chunk").disabled = $("mass-chunk-off").checked;
  $("mass-title").placeholder = $("mass-preset").value === "normal" ? "Missa" : "Missa brevis";
}

/** A time as a clock shows it: „3:40“, „1:02:05“. */
function clock(seconds) {
  const total = Math.max(0, Math.round(seconds));
  const two = (n) => String(n).padStart(2, "0");
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor(total / 60) % 60;
  return hours > 0
    ? `${hours}:${two(minutes)}:${two(total % 60)}`
    : `${minutes}:${two(total % 60)}`;
}

/** The parts of a planned mass as jobs of the pool, each with its seconds in the browser, in the
 * order the pool starts them. */
function massJobs(view, parts = view.parts) {
  const jobs = parts.map((part) => ({
    key: part.address,
    movement: part.movement,
    seconds: part.seconds * view.factor,
    source: part.source,
    waits: part.motif,
  }));
  return order(jobs, pool.size);
}

/** „Geschätzt 942 Takte, im Browser etwa 4 Minuten mit 4 Workern.“, as the pool will set them. */
function massEstimate(view) {
  const seconds = schedule(massJobs(view), pool.size);
  return t("mass_estimate")(view.measures, t("about")(seconds), pool.size);
}

let massPreviewTimer = 0;
let massPreviewTicket = 0;

function scheduleMassPreview() {
  clearTimeout(massPreviewTimer);
  massPreviewTimer = setTimeout(updateMassPreview, 250);
}

/** The plan of the next mass under the form: its first line and the estimate for this
 * browser's pool, and the table of sections; where it cannot be planned why, with the table
 * of the plan without the changes of the table. */
async function updateMassPreview() {
  clearTimeout(massPreviewTimer);
  if (!info) return;
  updateMassFields();
  const box = $("mass-preview");
  const form = massForm();
  if (!form) {
    box.replaceChildren(paragraph(t("no_movement"), "error"));
    return;
  }
  const ticket = ++massPreviewTicket;
  const reply = await ask({ command: "mass_plan", mass: { ...form, seed: massSeed() } });
  if (ticket !== massPreviewTicket) return;
  if (reply.mass) {
    massRows = reply.mass.rows;
    showSections(massRows);
  }
  if (!reply.ok) {
    box.replaceChildren(paragraph(reply.error, "error"));
    return;
  }
  box.replaceChildren(paragraph(reply.summary[0], "head"), paragraph(massEstimate(reply.mass)));
}

/** The table of sections: per row a section's voices, how it begins and the degree it ends on,
 * „auto“ with what the plan has where nothing else is chosen. Built anew where the rows change,
 * else only its values. */
function showSections(rows) {
  const box = $("mass-sections");
  const key = `${lang}|${rows.map((row) => row.id + row.endings.map((e) => e.id).join("")).join()}`;
  if (box.dataset.key !== key) {
    box.dataset.key = key;
    box.replaceChildren(...sectionElements(rows));
  }
  const choices = sectionChoices();
  let changed = false;
  for (const row of rows) {
    const element = [...box.children].find((child) => child.dataset.id === row.id);
    if (!element || row.ut_supra) continue;
    const choice = choices[row.id] ?? {};
    const voices = element.querySelector("[data-field=voices]");
    if (voices) {
      voices.placeholder = row.voices;
      if (document.activeElement !== voices) voices.value = choice.voices ?? "";
    }
    const imitation = element.querySelector("[data-field=imitation]");
    imitation.value = choice.imitation ?? "auto";
    imitation.options[0].textContent = choice.imitation
      ? "auto"
      : `auto (${row.imitative ? t("imitative") : t("together")})`;
    const ending = element.querySelector("[data-field=ending]");
    ending.value = choice.ending ?? "auto";
    if (ending.selectedIndex < 0) {
      // A degree the plan no longer allows: the row ends as drawn again.
      ending.value = "auto";
      delete choice.ending;
      if (Object.keys(choice).length) choices[row.id] = choice;
      else delete choices[row.id];
      changed = true;
    }
    const drawn = row.endings.find((degree) => degree.id === row.ending)?.name ?? row.ending;
    ending.options[0].textContent = ending.value === "auto" ? `auto (${drawn})` : "auto";
  }
  if (changed) setSectionChoices(choices);
}

/** The elements of the table of sections, each movement under its title. */
function sectionElements(rows) {
  const elements = [];
  let movement = null;
  for (const row of rows) {
    if (row.movement !== movement) {
      movement = row.movement;
      elements.push(paragraph(movement, "movement"));
    }
    const element = document.createElement("div");
    element.className = `section ${row.kind}`;
    element.dataset.id = row.id;
    const head = document.createElement("div");
    head.className = "head";
    const title = document.createElement("span");
    title.textContent = row.title;
    head.append(title);
    if (row.kind === "section" && !row.ut_supra) {
      const voices = document.createElement("input");
      voices.type = "text";
      voices.dataset.field = "voices";
      voices.setAttribute("aria-label", `${row.title}: ${t("voices")}`);
      head.append(voices);
    }
    element.append(head);
    if (row.ut_supra) {
      element.append(paragraph(t("ut_supra"), "hint"));
    } else {
      const line = document.createElement("div");
      line.className = "choices";
      line.append(
        sectionSelect("section_imitation", "imitation", [["an", t("on")], ["aus", t("off")]]),
        sectionSelect(
          "section_ending",
          "ending",
          row.endings.map((degree) => [degree.id, degree.name]),
        ),
      );
      element.append(line);
    }
    elements.push(element);
  }
  return elements;
}

/** A choice of a row of the table: „auto“, then `options` as [value, text]. */
function sectionSelect(label, field, options) {
  const element = document.createElement("label");
  const text = document.createElement("span");
  text.textContent = t(label);
  const select = document.createElement("select");
  select.dataset.field = field;
  select.add(new Option("auto", "auto"));
  for (const [value, name] of options) {
    select.add(new Option(name, value));
  }
  element.append(text, " ", select);
  return element;
}

$("mass-sections").addEventListener("change", (event) => {
  const field = event.target.dataset?.field;
  const row = event.target.closest("[data-id]");
  if (!field || !row) return;
  const choices = sectionChoices();
  const choice = { ...choices[row.dataset.id] };
  const value = event.target.value.trim();
  if (value === "" || value === "auto") {
    delete choice[field];
  } else {
    choice[field] = value;
  }
  if (Object.keys(choice).length) {
    choices[row.dataset.id] = choice;
  } else {
    delete choices[row.dataset.id];
  }
  setSectionChoices(choices);
});

/**
 * Plans a mass as the form gives it and sets it, or only shows its plan (`planOnly`): every
 * request of the mass carries the form with its seed, and the profile and rules of the page now,
 * which the files of its parts name; the parts go to the pool.
 */
async function startMass(planOnly) {
  if (pool.busy || massStarting) return;
  const form = massForm();
  if (!form) {
    setStatus(t("no_movement"), true);
    return;
  }
  const seed = massSeed();
  $("mass-seed").value = seed;
  const mass = { ...form, seed };
  const settings = { profile: $("profile").value, rules: overrides() };
  massStarting = true;
  let reply;
  try {
    reply = await ask({ command: "mass_plan", ...settings, mass });
  } finally {
    massStarting = false;
  }
  if (!reply.ok) {
    setStatus(reply.error, true);
    return;
  }
  stop();
  // A plan takes the place of the mass shown without losing it, nor what it set: „Ergebnis
  // zeigen“ shows it again, as long as no new mass is set.
  if (!planOnly) {
    keptMass = null;
  } else if (massRun && !massRun.planned) {
    keptMass = { run: massRun, view: resultView === "mass" ? { answer, selected } : views.mass };
  }
  massRun = {
    mass,
    settings,
    view: reply.mass,
    states: new Map(
      reply.mass.parts.map((part) => [
        part.address,
        { part, state: "waiting", started: 0, seconds: 0, answer: null, error: null },
      ]),
    ),
    planned: planOnly,
    running: false,
    cancelled: false,
    motif: undefined,
    measured: 0,
    estimated: 0,
    began: 0,
    wall: 0,
    joining: false,
    joinAgain: false,
    joined: null,
    notation: null,
    /** The parts of its last run in the order the pool starts them. */
    order: [],
    /** The address of the part that the results show on its own, chosen in the list of parts
     * while its movement is not put together; `null` where a result shows. */
    alone: null,
  };
  present("mass", { ok: true, summary: reply.summary, results: [] }, "mass");
  showMassRun();
  if (planOnly) {
    setStatus(t("mass_planned")(reply.mass.parts.length));
    return;
  }
  // The next mass takes a new seed again, which the preview then shows.
  pendingMassSeed = drawSeed();
  scheduleMassPreview();
  setStatus("");
  setMassParts(massRun, [...massRun.states.values()]);
}

/** Sets the parts `states` of the mass `run` in the pool, the motif of Kyrie I given to the parts
 * that open with it where it is known; then puts the mass together with what is set. */
async function setMassParts(run, states) {
  run.planned = false;
  run.running = true;
  run.cancelled = false;
  run.began = performance.now();
  for (const state of states) {
    state.state = "waiting";
    state.error = null;
  }
  const source = [...run.states.values()].find((state) => state.part.source);
  const motif = !source || states.includes(source)
    ? undefined
    : (source.answer?.part?.motif ?? null);
  const jobs = massJobs(run.view, states.map((state) => state.part)).map((job) => ({
    ...job,
    request: { command: "mass_part", lang, ...run.settings, mass: run.mass, part: job.key },
  }));
  // The time left follows the pool through its order.
  run.order = jobs.map((job) => job.key);
  const running = pool.run(
    jobs,
    {
      started: (job) => {
        const state = run.states.get(job.key);
        state.state = "running";
        state.started = performance.now();
        showMassProgress();
      },
      finished: (job, reply, seconds) => partSet(run, job, "done", seconds, reply, null),
      // A part whose worker broke off, where memory ran out say, may be set another time; one
      // whose search failed would fail again with its seed.
      failed: (job, error, seconds, broken) =>
        partSet(run, job, broken ? "broken" : "failed", seconds, null, error),
    },
    motif,
  );
  // The pool is busy from here on: „Abbrechen“ stops it, the other buttons wait for it.
  updateMassButtons();
  showMassProgress();
  const timer = setInterval(showMassProgress, 1000);
  const result = await running;
  clearInterval(timer);
  run.running = false;
  run.wall += (performance.now() - run.began) / 1000;
  if (result.cancelled) {
    run.cancelled = true;
    for (const job of result.stopped) run.states.get(job.key).state = "stopped";
  }
  updateMassButtons();
  showMassProgress();
  joinMass(run);
}

/** A part of the mass `run` is through: set („done“), failed, or its worker broke off
 * („broken“), which says nothing of how long the part takes. A movement whose parts are all set
 * is put together at once, so that it shows while the others are set. */
function partSet(run, job, state, seconds, reply, error) {
  const part = run.states.get(job.key);
  Object.assign(part, { state, seconds, answer: reply, error });
  if (state !== "broken") {
    run.measured += seconds;
    run.estimated += job.seconds;
    // Kyrie I gives its motif, or none; where its worker broke off, the parts that open with the
    // motif wait for it to be set again (the pool leaves them).
    if (part.part.source) run.motif = reply?.part?.motif ?? null;
  }
  showMassProgress();
  const movement = part.part.movement;
  const parts = [...run.states.values()].filter((other) => other.part.movement === movement);
  if (state === "done" && parts.every((other) => other.state === "done")) {
    joinMass(run);
  }
}

/** How the files of the mass shown look: the notation of the form, and the clefs and brackets of
 * the toolbar. */
function massLook() {
  return JSON.stringify([massNotation(), toolbar()]);
}

/** Puts the mass `run` together from the parts set (mass_join on the page's worker): the files of
 * its movements whose parts are all set, which the page shows, and its book; with the notation
 * of the form and the clefs and brackets of the toolbar now. A part whose search failed comes
 * with its error; one not set, or whose worker broke off, is left out: the page may still set
 * it. One at a time; a request meanwhile joins once more after it. */
async function joinMass(run) {
  if (run !== massRun) return;
  if (run.joining) {
    run.joinAgain = true;
    return;
  }
  if (![...run.states.values()].some((state) => state.state === "done")) return;
  run.joining = true;
  try {
    do {
      run.joinAgain = false;
      const parts = [...run.states.values()].flatMap((state) => {
        if (state.state === "done") {
          return [{
            part: state.part.address,
            musicxml: state.answer.part.musicxml,
            seconds: state.seconds,
          }];
        }
        return state.state === "failed" ? [{ part: state.part.address, error: state.error }] : [];
      });
      const notation = massNotation();
      const look = massLook();
      const reply = await ask({
        command: "mass_join",
        ...run.settings,
        mass: { ...run.mass, ...notation },
        parts,
        // While the pool sets parts, the parts missing go without saying.
        running: run.running,
      });
      if (run !== massRun) return;
      if (!reply.ok) {
        setStatus(`${t("mass_join_failed")}: ${reply.error}`, true);
        break;
      }
      run.joined = reply;
      run.notation = look;
      // The movement shown stays shown when another comes; a part shown on its own gives way to
      // its movement once that is put together.
      const shown = resultView === "mass" ? answer : views.mass?.answer;
      const at = resultView === "mass" ? selected : (views.mass?.selected ?? 0);
      let file = shown?.results?.[at]?.file;
      const alone = run.states.get(run.alone);
      const own = alone && run.view.movements[alone.part.movement].file;
      if (own && reply.results.some((entry) => entry.file === own)) {
        run.alone = null;
        file = own;
      }
      const keep = Math.max(0, reply.results.findIndex((entry) => entry.file === file));
      present("mass", reply, "mass", keep);
      updateMassButtons();
      showMassProgress();
    } while (run.joinAgain);
  } finally {
    run.joining = false;
  }
}

/** The notation of the form, or the clefs or brackets of the toolbar, differ from those the mass
 * was put together with: it is put together with them again, without a search. */
function applyMassNotation() {
  const run = massRun;
  if (run?.joined && run.notation !== massLook()) joinMass(run);
}

/** The seconds the mass `run` still takes, as the pool sets its parts in the order of the run:
 * the plan's estimates in the browser, scaled by how long the parts set so far took against
 * theirs. */
function massRemaining(run, now) {
  const ratio = run.estimated > 0 ? run.measured / run.estimated : 1;
  const running = [];
  const waiting = [];
  // Where the worker of Kyrie I broke off, the parts that wait for its motif wait past this run.
  const source = [...run.states.values()].find((state) => state.part.source);
  const stuck = source?.state === "broken";
  for (const state of run.order.map((key) => run.states.get(key))) {
    const seconds = state.part.seconds * run.view.factor * ratio;
    if (state.state === "running") {
      const left = Math.max(0, seconds - (now - state.started) / 1000);
      running.push({ seconds: left, source: state.part.source });
    } else if (state.state === "waiting") {
      const waits = state.part.motif && run.motif === undefined;
      if (!(waits && stuck)) waiting.push({ seconds, source: state.part.source, waits });
    }
  }
  return schedule(waiting, pool.size, running);
}

/** The panel of the mass shown, in the tab „Messe“: a line per part, which shows the part once
 * it is set (`showPart`). */
function showMassRun() {
  const panel = $("mass-run");
  panel.hidden = resultView !== "mass" || !massRun;
  if (!massRun) return;
  const items = [...massRun.states.values()].map((state) => {
    const item = document.createElement("li");
    const line = document.createElement("button");
    line.type = "button";
    line.className = "line";
    // The parts that open with the head motif, and Kyrie I, which gives it.
    const motif = state.part.motif
      ? t("part_takes_motif")
      : (state.part.source ? t("part_gives_motif") : "");
    line.textContent = motif ? `${state.part.line} · ${motif}` : state.part.line;
    line.onclick = () => showPart(state.part.address);
    const bar = document.createElement("progress");
    bar.max = 1;
    const status = document.createElement("span");
    status.className = "state";
    item.append(line, " ", bar, status);
    return item;
  });
  $("mass-list").replaceChildren(...items);
  updateMassButtons();
  showMassProgress();
}

/** Shows the part `address` of the mass shown once it is set (K11 (d)): its movement where that
 * is put together, else the part on its own (`alone`), with nothing to play: what plays stops. */
function showPart(address) {
  const run = massRun;
  const state = run?.states.get(address);
  if (resultView !== "mass" || state?.state !== "done") return;
  const file = run.view.movements[state.part.movement].file;
  const at = answer?.results.findIndex((entry) => entry.file === file) ?? -1;
  if (at >= 0) {
    run.alone = null;
    selected = at;
  } else {
    stop();
    run.alone = address;
  }
  showSelected();
  showMassProgress();
}

/** How far the mass shown is: per part, and in all with the time it still takes. */
function showMassProgress() {
  const run = massRun;
  if (!run) return;
  const now = performance.now();
  const ratio = run.estimated > 0 ? run.measured / run.estimated : 1;
  const items = $("mass-list").children;
  let done = 0;
  let running = 0;
  [...run.states.values()].forEach((state, i) => {
    const item = items[i];
    if (!item) return;
    const estimate = state.part.seconds * run.view.factor;
    const bar = item.querySelector("progress");
    const status = item.querySelector(".state");
    item.className = state.state;
    // A part set shows on a click on its line.
    const button = item.querySelector(".line");
    button.disabled = state.state !== "done";
    button.title = button.disabled ? "" : t("part_show");
    item.setAttribute("aria-current", String(state.part.address === run.alone));
    bar.hidden = state.state !== "running";
    status.title = "";
    if (state.state === "running") {
      running += 1;
      const elapsed = (now - state.started) / 1000;
      bar.value = Math.min(0.95, elapsed / Math.max(1, estimate * ratio));
      status.textContent = t("part_running")(clock(elapsed), clock(estimate * ratio));
    } else if (state.state === "done") {
      done += 1;
      const part = state.answer.part;
      // Kyrie I that gives no head motif, and what the part did otherwise than planned, such as
      // a first point without the motif: the warnings counted here, each in full on hover and in
      // the lines over the results.
      const warnings = part.warnings ?? [];
      const notes = [];
      if (state.part.source && !part.motif) notes.push(t("part_no_motif"));
      if (warnings.length > 0) notes.push(t("part_warnings")(warnings.length));
      status.textContent = t("part_done")(part.measures, part.cost, clock(state.seconds)) +
        notes.map((note) => ` · ${note}`).join("");
      status.title = warnings.join("\n");
      item.classList.toggle("warned", warnings.length > 0);
    } else if (state.state === "failed") {
      status.textContent = `${t("part_failed")}: ${state.error}`;
    } else if (state.state === "broken") {
      status.textContent = `${t("part_broken")}: ${state.error}`;
    } else if (state.state === "stopped") {
      status.textContent = t("part_stopped");
    } else if (run.planned) {
      status.textContent = t("part_planned")(clock(estimate));
    } else if (!run.running) {
      status.textContent = t("part_unset");
    } else {
      const waits = state.part.motif && run.motif === undefined;
      status.textContent = waits ? t("part_motif") : t("part_waiting");
    }
  });
  const total = run.states.size;
  const progress = $("mass-progress");
  let line;
  if (run.planned) {
    line = `${t("mass_planned")(total)} ${massEstimate(run.view)}`;
    progress.value = 0;
  } else if (run.running) {
    const left = massRemaining(run, now);
    const elapsed = (now - run.began) / 1000;
    line = t("mass_running")(done, total, running, clock(left));
    progress.value = elapsed + left > 0 ? elapsed / (elapsed + left) : 0;
  } else if (run.cancelled) {
    line = t("mass_cancelled")(done, total);
    progress.value = done / total;
  } else if (done === total) {
    line = t("mass_done")(done, total, clock(run.wall));
    progress.value = 1;
  } else {
    line = t("mass_incomplete")(done, total);
    progress.value = done / total;
  }
  $("mass-status").textContent = line;
}

/** The parts of the mass `run` that „Fehlende Teile setzen“ sets: those not set yet, cancelled, or
 * whose worker broke off; not those whose search failed, which their seeds would fail again. */
function missingParts(run) {
  return [...run.states.values()].filter((state) =>
    state.state === "waiting" || state.state === "stopped" || state.state === "broken"
  );
}

/** The buttons of the mass: one mass at a time; „Fehlende Teile setzen“ for a mass with parts
 * not set; „Ergebnis zeigen“ for the mass a plan took the place of; the book once the mass is put
 * together. */
function updateMassButtons() {
  const run = massRun;
  const busy = pool.busy;
  $("mass-cancel").disabled = !busy;
  $("mass-resume").hidden = !run || busy || run.planned || missingParts(run).length === 0;
  $("mass-result").hidden = !keptMass || busy;
  $("save-book").disabled = !run?.joined;
  $("mass-plan").disabled = busy;
  $("form-mass").querySelector("button.primary").disabled = busy ||
    document.body.classList.contains("busy");
}

$("form-mass").onsubmit = (event) => {
  event.preventDefault();
  startMass(false);
};
// Enter in a field of the mass takes its value, as leaving the field does, instead of setting a
// new mass in place of the one shown, which took minutes: only „Messe setzen“ sets one.
$("form-mass").addEventListener("keydown", (event) => {
  if (event.key !== "Enter" || event.isComposing || event.target.localName !== "input") return;
  event.preventDefault();
  event.target.dispatchEvent(new Event("change", { bubbles: true }));
});
$("mass-plan").onclick = () => startMass(true);
$("mass-cancel").onclick = () => pool.cancel();
$("mass-resume").onclick = () => {
  const run = massRun;
  if (!run || pool.busy) return;
  const missing = missingParts(run);
  if (missing.length) setMassParts(run, missing);
};
$("mass-result").onclick = () => {
  if (!keptMass || pool.busy || massStarting) return;
  const { run, view } = keptMass;
  keptMass = null;
  stop();
  massRun = run;
  present("mass", view?.answer ?? null, "mass", view?.selected ?? 0);
  showMassRun();
  setStatus("");
  // Put together again: with the notation and the toolbar now, and with every part set.
  joinMass(run);
};
$("save-book").onclick = () => {
  if (massRun?.joined) {
    saveFile(`${massRun.view.file}.ly`, massRun.joined.book, "text/x-lilypond");
  }
};
$("form-mass").addEventListener("input", scheduleMassPreview);
$("form-mass").addEventListener("change", () => {
  updateMassFields();
  scheduleMassPreview();
  applyMassNotation();
});
$("profile").addEventListener("change", scheduleMassPreview);
$("rule-overrides").addEventListener("change", scheduleMassPreview);

// ---- results ----

let answer = null;
let answerCommand = null;
let selected = 0;

/** The results of the tab „Messe“ and those of the other tabs, each kept while the other shows. */
const views = { mass: null, other: null };
let resultView = "other";

/** Shows the results of `next` ("mass" or "other"), keeping those shown. Results made with other
 * clefs or brackets than the toolbar has now, changed while they were hidden, are made again. */
function showView(next) {
  if (next === resultView) return;
  views[resultView] = { answer, answerCommand, selected };
  resultView = next;
  ({ answer, answerCommand, selected } = views[next] ??
    { answer: null, answerCommand: null, selected: 0 });
  stop();
  showMassRun();
  showResults();
  if (next === "mass") {
    applyMassNotation();
  } else {
    refreshOther();
  }
}

/** Shows `reply`, the answer of `command`, as the results of `target` ("mass" or "other"), the
 * result `keep` selected; or keeps it for when `target` shows. */
function present(target, reply, command, keep = 0) {
  if (target !== resultView) {
    views[target] = { answer: reply, answerCommand: command, selected: keep };
    return;
  }
  answer = reply;
  answerCommand = command;
  selected = keep;
  showResults();
}

function showResults() {
  $("summary").textContent = answer?.summary.join("\n") ?? "";
  const list = $("results");
  list.replaceChildren(
    ...(answer?.results ?? []).map((entry, i) => {
      const button = document.createElement("button");
      button.type = "button";
      button.setAttribute("role", "option");
      button.textContent = entry.label;
      button.onclick = () => {
        selected = i;
        if (resultView === "mass" && massRun) massRun.alone = null;
        showSelected();
        if (resultView === "mass") showMassProgress();
      };
      return button;
    }),
  );
  showSelected();
}

/** What the results show: in the tab „Messe“ the part that the list of parts shows on its own
 * (`alone` of the mass shown), else the result selected. */
function shownEntry() {
  const state = resultView === "mass" ? massRun?.states.get(massRun.alone) : undefined;
  return state?.state === "done" ? partEntry(state) : answer?.results[selected];
}

/** A part of the mass on its own as the results show an entry (`alone`: the part as planned):
 * its file as mass_part gave it, the file of `fux mass` whatever the toolbar shows, and its
 * violations; as text the report of its search, as its answer has no voices as text; nothing to
 * play or to save. */
function partEntry(state) {
  return {
    label: state.part.label,
    musicxml: state.answer.part.musicxml,
    violations: state.answer.part.violations,
    lines: state.answer.summary ?? [],
    alone: state.part,
  };
}

function showSelected() {
  const entry = shownEntry();
  [...$("results").children].forEach((button, i) =>
    button.setAttribute("aria-selected", String(i === selected && !entry?.alone))
  );
  for (const id of ["play", "save-musicxml", "save-midi", "save-ly"]) {
    $(id).disabled = !entry || Boolean(entry.alone);
  }
  $("cf-use").disabled = !(entry && answerCommand === "cf");
  showAlone(entry);
  showViolations(entry);
  showNotation(entry);
}

/** The line over the notation that says it shows a part on its own. */
function showAlone(entry) {
  const note = $("part-alone");
  note.hidden = !entry?.alone;
  note.textContent = entry?.alone ? t("part_alone")(entry.alone.label, entry.alone.address) : "";
}

function showViolations(entry) {
  const list = $("violations");
  if (!entry) {
    list.replaceChildren();
    return;
  }
  const only = $("only-forbidden").checked;
  const shown = entry.violations.filter((v) => !only || v.forbidden);
  if (shown.length === 0) {
    const item = document.createElement("li");
    item.textContent = t("no_violations");
    list.replaceChildren(item);
    return;
  }
  list.replaceChildren(
    ...shown.map((v) => {
      const item = document.createElement("li");
      item.classList.toggle("forbidden", v.forbidden);
      const level = document.createElement("span");
      level.className = "level";
      level.textContent = v.level;
      const rule = document.createElement("span");
      rule.className = "rule";
      rule.textContent = v.rule;
      const message = document.createElement("span");
      message.className = "message";
      message.textContent = v.message;
      item.append(level, ` ${v.position}`, rule, message);
      return item;
    }),
  );
}

let renderCount = 0;

/** Verovio's toolkit, or null if it does not come within half a minute. */
const notationToolkit = Promise.race([
  globalThis.verovioReady ?? Promise.resolve(null),
  new Promise((resolve) => setTimeout(() => resolve(null), 30_000)),
]);

/** The voices as text, each with what it sings. */
function voicesAsText(entry) {
  return entry.lines
    .map((line, i) => (entry.lyrics?.[i] ? `${line}\n    Text: ${entry.lyrics[i]}` : line))
    .join("\n");
}

/** What the notation shows: the MusicXML and the width it was engraved for. */
let engraved = null;

async function showNotation(entry) {
  const paper = $("notation");
  const text = $("text");
  text.textContent = entry ? voicesAsText(entry) : "";
  if (!entry) {
    paper.replaceChildren();
    engraved = null;
    return;
  }
  const asText = $("show-text").checked;
  text.hidden = !asText;
  paper.hidden = asText;
  if (asText) return;
  const ticket = ++renderCount;
  if (!paper.querySelector("svg")) {
    const hint = document.createElement("p");
    hint.className = "hint";
    hint.textContent = t("loading_notation");
    paper.replaceChildren(hint);
  }
  const toolkit = await notationToolkit;
  // A newer result may have come in while Verovio loaded.
  if (ticket !== renderCount) return;
  if (!toolkit) {
    const hint = document.createElement("p");
    hint.className = "hint";
    hint.textContent = t("no_notation");
    paper.replaceChildren(hint);
    text.hidden = false;
    return;
  }
  const scale = 40;
  const width = Math.max(500, paper.clientWidth - 20);
  // A movement of a mass shows section by section, each with the staves of the voices that sing
  // in it: Verovio would draw the staff of a voice that rests through a section all the same.
  const scores = entry.sections?.length
    ? entry.sections.map((section) => section.musicxml)
    : [entry.musicxml];
  const music = scores.join("\n");
  // A movement of a mass comes again as each of the others is put together: engraved once.
  if (engraved?.music === music && engraved.width === width && paper.querySelector("svg")) {
    return;
  }
  toolkit.setOptions({
    pageWidth: Math.round((width * 100) / scale),
    scale,
    adjustPageHeight: true,
    breaks: "auto",
    header: "none",
    footer: "none",
    pageMarginTop: 40,
    pageMarginBottom: 40,
  });
  let svg = "";
  for (const score of scores) {
    toolkit.loadData(score);
    for (let page = 1; page <= toolkit.getPageCount(); page++) {
      svg += toolkit.renderToSVG(page);
    }
  }
  paper.innerHTML = svg;
  engraved = { music, width };
}

let resizeTimer = 0;
globalThis.addEventListener("resize", () => {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => showNotation(shownEntry()), 250);
});

$("only-forbidden").onchange = () => showViolations(shownEntry());
$("show-text").onchange = () => showNotation(shownEntry());
/** Whether the last request may be made again: a long free setting is asked about first. */
function mayRerun() {
  const voices = lastRequest.voices ?? 0;
  return lastRequest.command !== "free" || !isLong(lastMeasures, voices) ||
    confirm(t("long_confirm")(lastMeasures, voices));
}

/** The clefs and the brackets over the entries need the results shown made again: the mass put
 * together again from its parts, without a search; the setting of another tab made again, a long
 * free setting asked about first. The results hidden follow when they show (showView). */
function rerun(checkbox) {
  if (resultView === "mass") {
    applyMassNotation();
    return;
  }
  if (!lastRequest) return;
  if (!mayRerun()) {
    checkbox.checked = !checkbox.checked;
    return;
  }
  run(lastRequest);
}

/** The results of the other tabs, shown again after the mass: made again where the clefs or the
 * brackets changed meanwhile, unless the last request has them already; where a long free
 * setting is not to be made again, they stay with a hint. */
function refreshOther() {
  const now = JSON.stringify(toolbar());
  if (!lastRequest || otherToolbar === null || otherToolbar === now || askedToolbar === now) {
    return;
  }
  if (!mayRerun()) {
    setStatus(t("toolbar_kept"));
    return;
  }
  run(lastRequest);
}

$("original-clefs").onchange = () => rerun($("original-clefs"));
$("entry-marks").onchange = () => rerun($("entry-marks"));

// ---- playback ----

let audio = null;
let sounding = [];
let playTimer = 0;

function organ(context) {
  // A soft organ: the fundamental, the octave and a little of the twelfth.
  const real = new Float32Array([0, 1, 0.45, 0.2, 0.08]);
  return context.createPeriodicWave(real, new Float32Array(real.length));
}

function play(entry) {
  stop();
  audio ??= new AudioContext();
  if (audio.state === "suspended") {
    audio.resume();
  }
  const wave = organ(audio);
  const tempo = Number($("tempo").value) || 72;
  // The tempo counts halves per minute; an eighth is a quarter of a half.
  const eighth = 60 / tempo / 4;
  const voices = new Set(entry.notes.map((n) => n.voice)).size || 1;
  const loudness = 0.3 / voices;
  const begin = audio.currentTime + 0.1;
  let last = 0;
  for (const note of entry.notes) {
    const start = begin + note.start * eighth;
    const end = begin + note.end * eighth;
    last = Math.max(last, end);
    const oscillator = audio.createOscillator();
    oscillator.setPeriodicWave(wave);
    oscillator.frequency.value = 440 * 2 ** ((note.key - 69) / 12);
    const gain = audio.createGain();
    gain.gain.setValueAtTime(0, start);
    gain.gain.linearRampToValueAtTime(loudness, start + 0.03);
    gain.gain.setValueAtTime(loudness, Math.max(start + 0.03, end - 0.06));
    gain.gain.linearRampToValueAtTime(0, end);
    oscillator.connect(gain).connect(audio.destination);
    oscillator.start(start);
    oscillator.stop(end + 0.02);
    sounding.push(oscillator);
  }
  playTimer = setTimeout(stop, (last - audio.currentTime) * 1000 + 200);
  updatePlayButton();
}

function stop() {
  clearTimeout(playTimer);
  for (const oscillator of sounding) {
    try {
      oscillator.stop();
    } catch {
      // Already stopped.
    }
  }
  sounding = [];
  updatePlayButton();
}

function updatePlayButton() {
  $("play").textContent = sounding.length ? t("stop") : t("play");
}

$("play").onclick = () => {
  if (sounding.length) {
    stop();
  } else if (answer?.results[selected]) {
    play(answer.results[selected]);
  }
};

// ---- downloads ----

/** A file of the selected result: named by the result where it has a name (a movement of a mass:
 * `messe-3-credo`), else by the task and its number. */
function download(extension, data, type) {
  const file = answer?.results[selected]?.file ?? `fux-${answerCommand ?? "satz"}-${selected + 1}`;
  saveFile(`${file}.${extension}`, data, type);
}

function saveFile(name, data, type) {
  const url = URL.createObjectURL(new Blob([data], { type }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

$("save-musicxml").onclick = () =>
  download("musicxml", answer.results[selected].musicxml, "application/vnd.recordare.musicxml+xml");
$("save-ly").onclick = () => {
  // The LilyPond of a mass is its book, with all its movements.
  if (answer?.book !== undefined) {
    saveFile(`${massRun?.view.file ?? "messe"}.ly`, answer.book, "text/x-lilypond");
    return;
  }
  download("ly", answer.results[selected].lilypond, "text/x-lilypond");
};
$("save-midi").onclick = () => {
  const bytes = Uint8Array.from(atob(answer.results[selected].midi), (c) => c.charCodeAt(0));
  download("mid", bytes, "audio/midi");
};

// ---- rules ----

$("rule-search").onchange = () => {
  const id = $("rule-search").value.trim().split(" ")[0];
  if (info?.rules.some((rule) => rule.id === id)) {
    const field = $("rule-overrides");
    field.value = `${field.value.trimEnd()}${field.value.trim() ? "\n" : ""}${id}=`;
    field.focus();
    $("rule-search").value = "";
    save();
  }
};

// ---- start ----

for (const button of document.querySelectorAll("#tabs button")) {
  button.onclick = () => {
    showTab(button.dataset.tab);
    save();
  };
}

$("lang").onchange = async () => {
  lang = $("lang").value;
  translate();
  save();
  await loadInfo();
  updatePreview();
  scheduleMassPreview();
};

$("remember").onchange = () => {
  if ($("remember").checked) {
    save();
  } else {
    forget();
  }
};
$("forget").onclick = () => {
  forget();
  $("remember").checked = false;
  setStatus(t("forgotten"));
};

document.addEventListener("change", save);

async function start() {
  const values = restore();
  lang = $("lang").value || (navigator.language?.startsWith("de") ? "de" : "en");
  if (!values.lang && !navigator.language?.startsWith("de")) {
    lang = "en";
  }
  $("lang").value = lang;
  translate();
  showTab(values.tab ?? "cp");
  setBusy(true, t("loading"));
  const loaded = await loadInfo();
  setBusy(false);
  if (!loaded) return;
  // The selects are filled now; their remembered values can be set.
  restore();
  setStatus(t("ready"));
  // Before a first search, which would hold it up.
  updatePreview();
  updateMassPreview();
  // Something to look at from the start.
  if (currentTab === "cp") {
    $("form-cp").requestSubmit();
  }
}

start();
