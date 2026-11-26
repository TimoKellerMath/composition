// The page of fux-fuge: a form for the subject, the fugue as notes with playback and downloads,
// and its report with the answer, the sections and the style measures. The fugue is set in
// worker.js. The page shows what the module answers as it comes: the subjects, the sections and
// the measures are lists of the module, and the form is the list FIELDS below.

const $ = (id) => document.getElementById(id);

// ---- texts ----

const TEXTS = {
  de: {
    tagline: "Fugen im Stil Bachs zu einem gegebenen Thema",
    task: "Thema und Suche",
    report: "Bericht",
    fugue: "Fuge",
    group_subject: "Thema",
    group_fugue: "Fuge",
    new_seed: "neuer Seed",
    run: "Fuge setzen",
    cancel: "Abbrechen",
    help: "Schreibweise der Töne",
    help_text:
      "<p>Die Töne stehen wie in LilyPond, mit Leerzeichen dazwischen: niederländische Tonnamen " +
      "(<code>c d e f g a b</code>, erhöht mit <code>is</code>, erniedrigt mit <code>es</code>: " +
      "<code>fis</code>, <code>bes</code>, <code>es</code>, <code>as</code>; <code>b</code> ist " +
      "das deutsche h), dahinter die Oktave (<code>c</code> kleines c, <code>c'</code> " +
      "eingestrichenes, <code>c''</code> zweigestrichenes, <code>c,</code> großes C) und der " +
      "Notenwert (<code>1</code> Ganze, <code>2</code> Halbe, <code>4</code> Viertel, " +
      "<code>8</code>, <code>16</code>, <code>32</code>; ein Punkt verlängert: <code>4.</code>). " +
      "Ein Notenwert gilt, bis ein anderer kommt.</p>" +
      "<p><code>r</code> ist eine Pause, <code>~</code> hinter einem Ton bindet ihn an den " +
      "nächsten gleichen, <code>|</code> darf zwischen zwei Tönen als Taktstrich stehen und " +
      "wird überlesen. Das Thema beginnt an einem Taktstrich: Pausen am Anfang setzen einen " +
      "Auftakt.</p>" +
      "<p>Tonart: ein Buchstabe mit <code>#</code> oder <code>b</code>, groß für Dur, klein für " +
      "Moll (<code>C</code>, <code>f#</code>, <code>Bb</code>). Der Einsatzabstand zählt die " +
      "Takte von einem Einsatz der Exposition zum nächsten.</p>" +
      "<p>Beispiel (Kunst der Fuge, Tonart <code>d</code>, Takt <code>2/2</code>):<br>" +
      "<code>d'2 a' | f' d' | cis' d'4 e' | f'2~ f'8 g' f' e' | d'4</code></p>",
    play: "Abspielen",
    stop: "Stopp",
    tempo: "Tempo (Viertel je Minute)",
    loading: "Lade das Programm …",
    ready: "Bereit.",
    working: "Setze die Fuge …",
    done: (seconds) => `Fuge gesetzt in ${seconds} s.`,
    cancelled: "Abgebrochen.",
    error: (message) => `Fehler: ${message}`,
    broken: (message) => `Das Programm ist abgebrochen und wird neu geladen: ${message}`,
    missing: (label) => `Bitte ausfüllen: ${label}.`,
    voices: (n) => `${n} Stimmen`,
    bars: (n) => `${n} Takte`,
    tones: (n) => `${n} Töne`,
    given: (n) => `${n} vorgegeben`,
    empty: "Ein Thema wählen oder eigene Töne eingeben, dann „Fuge setzen“.",
    loading_notation: "Lade den Notensatz …",
    no_notation: "Ohne Notenbild: Verovio ist nicht geladen. Die Dateien lassen sich " +
      "trotzdem herunterladen.",
    notation_failed: "Verovio hat die Noten nicht gesetzt.",
    report_empty: "Nach dem Setzen stehen hier die Antwort, die Abschnitte, die Prüfung und " +
      "die Kennzahlen der Fuge.",
    answer: "Antwort",
    countersubject: "Kontrasubjekt",
    sections: "Abschnitte",
    ending: (end) => `Schluss: ${end}.`,
    endings: { pedal: "Orgelpunkt", stretto: "Engführung" },
    stretto_at: (bars, one) => `im Abstand von ${bars} ${one ? "Takt" : "Takten"}`,
    entries: (found, planned) => `${found} von ${planned} Einsätzen wiedergefunden.`,
    check: "Prüfung",
    check_help: "Geprüft ist die fertige Fuge gegen ihr eigenes Gerüst aus Akkorden. " +
      "Wiedergefunden ist ein Einsatz, den ein Themenfinder aus den Noten allein erkennt.",
    counts: {
      own: (n) =>
        `Verbote, die das vorgegebene Material selbst bricht (Einsätze, Kontrasubjekte): ${n}.`,
      forced: (n) =>
        `Verbote an Nahtstellen, wo eine freie Stimme nicht in den Einsatz führt: ${n}.`,
    },
    section: (bar, name) => `Takt ${bar}: ${name}`,
    play_from: "Ab hier abspielen",
    measures: "Kennzahlen",
    measure: "Kennzahl",
    value: "Wert",
    band: "Bachs Band",
    percent: (text) => `${text} %`,
    hits: (hits, count) => `${hits} von ${count} Kennzahlen liegen in Bachs Bändern.`,
    yardstick: (mark, reached) =>
      `Der Maßstab ist ${mark}: ${reached ? "erreicht" : "nicht erreicht"}.`,
    above: "über dem Band",
    below: "unter dem Band",
    outside: "außerhalb des Bandes",
    measures_help: "Das Band einer Kennzahl reicht vom 10. bis zum 90. Perzentil der 48 Fugen " +
      "des Wohltemperierten Klaviers. Der Maßstab ist das 10. Perzentil ihrer Trefferzahlen, " +
      "jede Fuge gegen die Bänder der übrigen gezählt.",
    credits: "Die Fugen setzt das Programm, nicht Bach: mit Regeln und Tabellen, gezählt an " +
      "den 48 Fugen des Wohltemperierten Klaviers. " +
      'Notensatz: <a href="https://www.verovio.org">Verovio</a> ' +
      '(<a href="verovio/README.txt">LGPL</a>). ' +
      'Die Pläne aus WTK I enthalten Informationen aus ' +
      '<a href="https://gitlab.com/algomus.fr/algomus-data">algomus-data</a> (<a href="https://opendatacommons.org/licenses/odbl/1-0/">ODbL 1.0</a>).',
    contact: "Kontakt",
    versions: "Fassungen",
    versions_hint: "Mehrere Fassungen derselben Fuge, die sich nur in den angekreuzten Aspekten " +
      "unterscheiden.",
    more_seeds: "weitere Seeds",
    blind: "blind: worin sie sich unterscheiden, erst nach dem Hören zeigen",
    run_versions: "Fassungen setzen",
    reveal: "Auflösen",
    keep: "Diese Fassung übernehmen",
    version: (letter) => `Fassung ${letter}`,
    versions_differ: (aspects) =>
      `Die Fassungen unterscheiden sich in: ${aspects}. Alles andere ist gleich.`,
    versions_hidden: "Blind: Worin sich die Fassungen unterscheiden, zeigt „Auflösen“.",
    versions_none: "Für Fassungen einen Aspekt ankreuzen oder weitere Seeds wählen.",
    versions_many: (count, most) =>
      `Das wären ${count} Fassungen; es gehen höchstens ${most} auf einmal.`,
    working_version: (letter, index, count) => `Setze Fassung ${letter} (${index} von ${count}) …`,
    versions_done: (made, count, seconds) =>
      `${made} von ${count} Fassungen gesetzt in ${seconds} s.`,
    version_failed: "nicht entstanden",
    version_waiting: "wartet",
    kept: (letter) => `Fassung ${letter} steht im Formular.`,
  },
  en: {
    tagline: "Fugues in the style of Bach on a given subject",
    task: "Subject and search",
    report: "Report",
    fugue: "Fugue",
    group_subject: "Subject",
    group_fugue: "Fugue",
    new_seed: "new seed",
    run: "Compose the fugue",
    cancel: "Cancel",
    help: "How to write the tones",
    help_text:
      "<p>The tones are written as in LilyPond, with spaces between them: Dutch note names " +
      "(<code>c d e f g a b</code>, raised with <code>is</code>, lowered with <code>es</code>: " +
      "<code>fis</code>, <code>bes</code>, <code>es</code>, <code>as</code>), then the octave " +
      "(<code>c</code> below middle C, <code>c'</code> middle C, <code>c''</code> the octave " +
      "above it, <code>c,</code> the octave below <code>c</code>) and the value (<code>1</code> " +
      "whole, <code>2</code> half, <code>4</code> quarter, <code>8</code>, <code>16</code>, " +
      "<code>32</code>; a dot lengthens: <code>4.</code>). A value holds until another one " +
      "comes.</p>" +
      "<p><code>r</code> is a rest, <code>~</code> after a tone ties it to the next of the same " +
      "pitch, <code>|</code> may stand between two tones for a barline and is ignored. The " +
      "subject begins at a barline: rests at its start place an upbeat.</p>" +
      "<p>Key: a letter with <code>#</code> or <code>b</code>, upper case for major, lower case " +
      "for minor (<code>C</code>, <code>f#</code>, <code>Bb</code>). The distance of the " +
      "entries counts the bars from one entry of the exposition to the next.</p>" +
      "<p>Example (The Art of Fugue, key <code>d</code>, meter <code>2/2</code>):<br>" +
      "<code>d'2 a' | f' d' | cis' d'4 e' | f'2~ f'8 g' f' e' | d'4</code></p>",
    play: "Play",
    stop: "Stop",
    tempo: "Tempo (quarters per minute)",
    loading: "Loading the program …",
    ready: "Ready.",
    working: "Composing the fugue …",
    done: (seconds) => `Fugue composed in ${seconds} s.`,
    cancelled: "Cancelled.",
    error: (message) => `Error: ${message}`,
    broken: (message) => `The program broke off and is loaded again: ${message}`,
    missing: (label) => `Please fill in: ${label}.`,
    voices: (n) => `${n} voices`,
    bars: (n) => `${n} bars`,
    tones: (n) => `${n} tones`,
    given: (n) => `${n} given`,
    empty: "Pick a subject or enter tones of your own, then “Compose the fugue”.",
    loading_notation: "Loading the engraving …",
    no_notation: "No notation: Verovio is not loaded. The files can be downloaded all the same.",
    notation_failed: "Verovio did not engrave the notes.",
    report_empty: "Once a fugue is composed, its answer, its sections, its check and its " +
      "style measures stand here.",
    answer: "Answer",
    countersubject: "Countersubject",
    sections: "Sections",
    ending: (end) => `Ending: ${end}.`,
    endings: { pedal: "pedal point", stretto: "stretto" },
    stretto_at: (bars, one) => `at a distance of ${bars} ${one ? "bar" : "bars"}`,
    entries: (found, planned) => `${found} of ${planned} entries found again.`,
    check: "Check",
    check_help: "The finished fugue is checked against its own scaffold of chords. An entry is " +
      "found again if a finder of subjects recognises it from the notes alone.",
    counts: {
      own: (n) =>
        `Prohibitions that the given material itself breaks (entries, countersubjects): ${n}.`,
      forced: (n) =>
        `Prohibitions at junctions, where a free voice does not lead into the entry: ${n}.`,
    },
    section: (bar, name) => `Bar ${bar}: ${name}`,
    play_from: "Play from here",
    measures: "Style measures",
    measure: "Measure",
    value: "Value",
    band: "Bach's band",
    percent: (text) => `${text}%`,
    hits: (hits, count) => `${hits} of ${count} measures lie inside Bach's bands.`,
    yardstick: (mark, reached) => `The yardstick is ${mark}: ${reached ? "met" : "not met"}.`,
    above: "above the band",
    below: "below the band",
    outside: "outside the band",
    measures_help: "The band of a measure reaches from the 10th to the 90th percentile of the " +
      "48 fugues of the Well-Tempered Clavier. The yardstick is the 10th percentile of their " +
      "numbers of hits, each fugue counted against the bands of the others.",
    credits: "The program composes the fugues, not Bach: with rules and tables counted on " +
      "the 48 fugues of the Well-Tempered Clavier. " +
      'Engraving: <a href="https://www.verovio.org">Verovio</a> ' +
      '(<a href="verovio/README.txt">LGPL</a>). ' +
      'The plans from WTC I contain information from ' +
      '<a href="https://gitlab.com/algomus.fr/algomus-data">algomus-data</a> (<a href="https://opendatacommons.org/licenses/odbl/1-0/">ODbL 1.0</a>).',
    contact: "Contact",
    versions: "Versions",
    versions_hint: "Several versions of the same fugue that differ only in the aspects ticked.",
    more_seeds: "further seeds",
    blind: "blind: show what they differ in only after listening",
    run_versions: "Compose the versions",
    reveal: "Reveal",
    keep: "Take this version",
    version: (letter) => `Version ${letter}`,
    versions_differ: (aspects) =>
      `The versions differ in: ${aspects}. Everything else is the same.`,
    versions_hidden: "Blind: “Reveal” shows what the versions differ in.",
    versions_none: "For versions tick an aspect or choose further seeds.",
    versions_many: (count, most) => `That would be ${count} versions; at most ${most} at a time.`,
    working_version: (letter, index, count) =>
      `Composing version ${letter} (${index} of ${count}) …`,
    versions_done: (made, count, seconds) =>
      `${made} of ${count} versions composed in ${seconds} s.`,
    version_failed: "did not come out",
    version_waiting: "waiting",
    kept: (letter) => `Version ${letter} is in the form.`,
  },
};

/** Texts that contain markup, all from TEXTS above. */
const MARKUP = new Set(["help_text", "credits"]);

let lang = "de";

function t(key) {
  return TEXTS[lang][key] ?? TEXTS.de[key] ?? key;
}

/** A number as the language writes it, with `most` decimals and at least `least`. */
function number(value, most = 0, least = most) {
  return value.toLocaleString(lang, {
    minimumFractionDigits: least,
    maximumFractionDigits: most,
  });
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
  for (const element of document.querySelectorAll("[data-label]")) {
    element.setAttribute("aria-label", t(element.dataset.label));
  }
  translateForm();
  showStatus();
  showFugue();
}

// ---- the module in the worker ----

let worker = null;
const pending = new Map();
let nextId = 0;

/** Answers every request that waits with `answer`. */
function settle(answer) {
  for (const resolve of pending.values()) resolve(answer);
  pending.clear();
}

/** The worker with the module; a fresh one after a cancel, or after the last one failed. */
function theWorker() {
  if (worker) return worker;
  const made = new Worker("worker.js", { type: "module" });
  made.onmessage = (event) => {
    const { id, answer } = event.data;
    pending.get(id)?.(answer);
    pending.delete(id);
  };
  // The worker itself failed (its script did not load, or memory ran out): the request is
  // answered with the error, and the next one starts another worker. Not at once: a worker
  // that cannot load would fail again and again.
  made.onerror = (event) => {
    event.preventDefault?.();
    if (worker !== made) return;
    made.terminate();
    worker = null;
    settle({ ok: false, error: event.message ?? "worker error", broken: true });
  };
  worker = made;
  return made;
}

/** Sends a request in the page's language; resolves with the answer. A worker that the browser
 * does not start is an answer too. */
function ask(request) {
  return new Promise((resolve) => {
    const id = nextId++;
    pending.set(id, resolve);
    try {
      theWorker().postMessage({ id, request: { lang, ...request } });
    } catch (error) {
      pending.delete(id);
      resolve({ ok: false, error: `${error}`, broken: true });
    }
  });
}

/** Ends the fugue being set: its worker is terminated, and a fresh one loads the module. */
function cancel() {
  worker?.terminate();
  worker = null;
  settle({ ok: false, cancelled: true });
  theWorker();
}

// ---- status ----

let status = { text: () => "", error: false };
let busy = false;

/** Shows `text` in the status line; a function is asked again when the language changes. */
function setStatus(text, error = false) {
  status = { text: typeof text === "function" ? text : () => text, error };
  showStatus();
}

function showStatus() {
  const line = $("status");
  line.textContent = status.text();
  line.classList.toggle("error", status.error);
}

function setBusy(now) {
  busy = now;
  document.body.classList.toggle("busy", now);
  for (const button of document.querySelectorAll("button.primary")) {
    button.disabled = now;
  }
  $("run-versions").disabled = now;
  // A fugue shown is set again in another language, which waits for the one being set.
  $("lang").disabled = now;
}

// ---- the form ----

/** The value of the select of subjects that stands for tones of one's own. */
const OWN = "";

/** The select of the subjects that come with the module, which `themes` lists; its last option
 * stands for tones of one's own. */
const THEME = {
  key: "theme",
  kind: "select",
  group: "subject",
  wide: true,
  label: { de: "Auswahl", en: "Choice" },
  hint: {
    de: "Wer ein Feld darunter ändert, setzt ein eigenes Thema.",
    en: "A change in a field below makes it a subject of your own.",
  },
  options: [{ value: OWN, de: "Eigene Töne …", en: "Own tones …" }],
};

/** The select of the plans that come with the module, which `themes` lists too; its first
 * option leaves the plan to the program. */
const PLAN = {
  key: "plan",
  kind: "select",
  group: "fugue",
  label: { de: "Plan der Einsätze", en: "Plan of the entries" },
  hint: {
    de: "Die Einsätze nach der Exposition wie in einer Fuge aus WTK I.",
    en: "The entries after the exposition as in a fugue of WTC I.",
  },
  options: [{ value: "", de: "vom Programm", en: "by the program" }],
};

/** What the fields of a subject of one's own share: their group in the form, and `own`. */
const SUBJECT = { group: "subject", own: true };

/**
 * The fields of the form in their order, each a field of the request to the module: `key` its
 * name there, `kind` its control (select, number, text for prose, textarea and code for the
 * notation in several lines and in one), `value` what it holds at first, `label` and `hint` in
 * both languages, `options` the choices of a select. A field with `own` describes a subject of
 * one's own: it is sent in place of `theme`, and a subject of the module shows in it what
 * `themes` lists under its key. An empty field, like the empty choice of a select, is left out of
 * the request; if it is `required`, the page asks for it instead. A field with `bound` belongs
 * to the subject chosen and is emptied when another one is chosen. A new field of the request is
 * one more entry here.
 */
const FIELDS = [
  THEME,
  {
    ...SUBJECT,
    key: "subject",
    kind: "textarea",
    required: true,
    wide: true,
    rows: 4,
    label: { de: "Töne", en: "Tones" },
    placeholder: "d'2 a' | f' d' | cis' d'4 e' | f'2~ f'8 g' f' e' | d'4",
  },
  { ...SUBJECT, key: "key", kind: "code", required: true, label: { de: "Tonart", en: "Key" } },
  { ...SUBJECT, key: "meter", kind: "code", value: "4/4", label: { de: "Takt", en: "Meter" } },
  {
    ...SUBJECT,
    key: "period_bars",
    kind: "number",
    min: 0.5,
    step: 0.5,
    label: { de: "Einsatzabstand (Takte)", en: "Distance of the entries (bars)" },
    hint: { de: "Leer: nach der Länge des Themas.", en: "Empty: by the length of the subject." },
  },
  { ...SUBJECT, key: "title", kind: "text", wide: true, label: { de: "Titel", en: "Title" } },
  {
    key: "answer",
    kind: "code",
    group: "subject",
    wide: true,
    bound: true,
    label: { de: "Eigene Antwort", en: "Own answer" },
    hint: {
      de: "Leer: nach der Regel. Sonst so viele Töne wie das Thema, in dessen Rhythmus.",
      en: "Empty: by the rule. Else as many tones as the subject, in its rhythm.",
    },
  },
  {
    key: "voices",
    kind: "select",
    group: "fugue",
    value: 3,
    label: { de: "Stimmen", en: "Voices" },
    hint: {
      de: "Drei Stimmen brauchen im Browser etwa 3 bis 15 Sekunden, vier 10 bis 70 (je nach " +
        "Thema und Gerät).",
      en: "Three voices take about 3 to 15 seconds in the browser, four 10 to 70 (depending " +
        "on subject and device).",
    },
    options: [{ value: 2 }, { value: 3 }, { value: 4 }],
  },
  {
    key: "ending",
    kind: "select",
    group: "fugue",
    value: "auto",
    label: { de: "Schluss", en: "Ending" },
    hint: {
      de: "Automatisch: eine Engführung, wenn das Thema eine hergibt, sonst ein Orgelpunkt.",
      en: "Automatic: a stretto if the subject gives one, else a pedal point.",
    },
    options: [
      { value: "auto", de: "automatisch", en: "automatic" },
      { value: "pedal", de: "Orgelpunkt", en: "pedal point" },
      { value: "stretto", de: "Engführung", en: "stretto" },
    ],
  },
  {
    key: "free_voices",
    kind: "select",
    group: "fugue",
    value: "figures",
    vary: true,
    label: { de: "Freie Stimmen", en: "Free voices" },
    hint: {
      de: "Mit Figuren: Sequenzen in allen Stimmen der Zwischenspiele und Figuren aus Thema " +
        "und Kontrasubjekt; die Fuge wird dafür zwei- bis viermal gesetzt. Schlicht: ein Satz " +
        "aus Rhythmustabellen, im Mittel drei- bis fünfmal so schnell.",
      en: "With figures: sequences in all voices of the episodes and figures of the subject " +
        "and the countersubject; for that the fugue is set two to four times. Plain: one " +
        "setting from rhythm tables, three to five times as fast on average.",
    },
    options: [
      { value: "figures", de: "mit Figuren und Sequenzen", en: "with figures and sequences" },
      { value: "plain", de: "schlicht (schneller)", en: "plain (faster)" },
    ],
  },
  {
    key: "profile",
    kind: "select",
    group: "fugue",
    value: "fuge",
    vary: true,
    label: { de: "Klang", en: "Sound" },
    hint: {
      de: "Nach dem Ohr: zusammen angeschlagene Dissonanz kostet mehr, und drei Stimmen bilden " +
        "öfter den vollen Dreiklang (nach blinden Hörproben). Nach der Zählung: die Preise, " +
        "die an Bachs Raten geeicht sind.",
      en: "By ear: dissonance struck together costs more, and three voices form the full " +
        "triad more often (after blind listening tests). By the count: the prices calibrated " +
        "on Bach's rates.",
    },
    options: [
      { value: "fuge", de: "nach dem Ohr", en: "by ear" },
      { value: "bach", de: "nach der Zählung", en: "by the count" },
    ],
  },
  {
    key: "bass",
    kind: "select",
    group: "fugue",
    value: "free",
    vary: true,
    label: { de: "Bass", en: "Bass" },
    hint: {
      de: "Geplant: das Gerüst nennt je Akkord einen Basston, nach der Bezifferung von Bachs " +
        "Fugen. Gezählt liegt dann der Grundton so oft zuunterst wie bei Bach; in einer " +
        "blinden Hörprobe klang es gleich.",
      en: "Planned: the scaffold names a bass for every chord, after the figures of Bach's " +
        "fugues. By count the root then lies lowest as often as in Bach; in a blind listening " +
        "test it sounded the same.",
    },
    options: [
      { value: "free", de: "frei", en: "free" },
      { value: "planned", de: "geplant", en: "planned" },
    ],
  },
  PLAN,
  {
    key: "beam",
    kind: "select",
    group: "fugue",
    value: 400,
    label: { de: "Qualität", en: "Quality" },
    hint: {
      de: "Die Breite der Suche: mehr ist langsamer und selten besser.",
      en: "The width of the search: more is slower and seldom better.",
    },
    options: [
      { value: 200, de: "schnell", en: "fast" },
      { value: 400, de: "normal", en: "normal" },
      { value: 800, de: "gründlich", en: "thorough" },
    ],
  },
];

/** The elements of each field by its key: its control, and its name and hint in the label. */
const parts = new Map();
/** Per field that can be varied: its checkbox in the form of the versions and the caption. */
const axes = new Map();
const control = (field) => parts.get(field.key).control;

/** The subjects of the module, as `themes` lists them. */
let themes = [];
/** The subject that the own fields show (OWN: what was entered), and what they held before a
 * subject of the module took its place. */
let shownTheme = OWN;
const ownValues = {};

/** The kinds of fields that hold notation. */
const NOTATION = ["textarea", "code"];

function buildForm() {
  const groups = new Map();
  for (const field of FIELDS) {
    const input = field.kind !== "select" && field.kind !== "textarea";
    const element = document.createElement(input ? "input" : field.kind);
    if (input) element.type = field.kind === "number" ? "number" : "text";
    element.setAttribute("id", `f-${field.key}`);
    for (const name of ["min", "max", "step", "rows", "placeholder"]) {
      if (field[name] !== undefined) element[name] = field[name];
    }
    if (field.kind !== "select") element.value = field.value ?? "";
    if (NOTATION.includes(field.kind)) {
      // Not prose: a phone would begin it with a capital and correct its words.
      element.spellcheck = false;
      for (const name of ["autocapitalize", "autocorrect", "autocomplete"]) {
        element.setAttribute(name, "off");
      }
    }
    const name = document.createElement("span");
    name.className = "name";
    const hint = document.createElement("span");
    hint.className = "hint";
    const label = document.createElement("label");
    label.className = field.wide ? "wide" : "row";
    label.append(name, element, hint);
    parts.set(field.key, { control: element, name, hint });
    if (field.own) {
      element.oninput = () => {
        control(THEME).value = OWN;
        shownTheme = OWN;
      };
    }
    let box = $("fields");
    if (field.group && !groups.has(field.group)) {
      const group = document.createElement("fieldset");
      const legend = document.createElement("legend");
      legend.dataset.t = `group_${field.group}`;
      group.append(legend);
      box.append(group);
      groups.set(field.group, group);
    }
    box = groups.get(field.group) ?? box;
    box.append(label);
  }
  control(THEME).onchange = showTheme;
  // A checkbox per aspect in which versions of a fugue can differ.
  for (const field of FIELDS.filter((one) => one.vary)) {
    const box = document.createElement("input");
    box.type = "checkbox";
    box.setAttribute("id", `vary-${field.key}`);
    const name = document.createElement("span");
    const label = document.createElement("label");
    label.append(box, " ", name);
    axes.set(field.key, { box, name });
    $("axes").append(label);
  }
}

/** The options of a select as its field lists them, in the page's language; its choice stays,
 * and is at first the field's `value`. */
function fillOptions(field) {
  const select = control(field);
  const value = select.options.length ? select.value : String(field.value ?? "");
  select.replaceChildren();
  for (const option of field.options) {
    select.add(new Option(option[lang] ?? option.text ?? String(option.value), option.value));
  }
  if (field.options.some((option) => String(option.value) === value)) {
    select.value = value;
  }
}

function translateForm() {
  for (const field of FIELDS) {
    const { name, hint } = parts.get(field.key);
    name.textContent = field.label[lang];
    hint.textContent = field.hint?.[lang] ?? "";
    hint.hidden = !field.hint;
    if (field.options) fillOptions(field);
    if (axes.has(field.key)) axes.get(field.key).name.textContent = field.label[lang];
  }
}

/** Shows the subject chosen in the own fields: one of the module with what `themes` lists of
 * it, „Eigene Töne“ with what the fields held before. */
function showTheme() {
  const name = control(THEME).value;
  const own = FIELDS.filter((field) => field.own);
  if (shownTheme === OWN) {
    for (const field of own) ownValues[field.key] = control(field).value;
  }
  const theme = themes.find((one) => one.name === name);
  for (const field of own) {
    control(field).value = theme ? theme[field.key] ?? "" : ownValues[field.key] ?? "";
  }
  for (const field of FIELDS.filter((one) => one.bound)) control(field).value = "";
  shownTheme = theme ? name : OWN;
}

/** What a field holds, as the request takes it; undefined if it is empty. */
function read(field) {
  const text = control(field).value.trim();
  if (text === "") return undefined;
  if (field.options) {
    return field.options.find((option) => String(option.value) === text)?.value;
  }
  // The octave marks as phones and word processors write them.
  if (NOTATION.includes(field.kind)) return text.replace(/[’‘′]/g, "'");
  if (field.kind !== "number") return text;
  return Number.isFinite(Number(text)) ? Number(text) : undefined;
}

/** The seed of the next fugue: a new one, which the field then shows, or the one in the field. */
function seed() {
  const field = $("seed");
  if ($("new-seed").checked) {
    field.value = Math.floor(Math.random() * 1_000_000);
  }
  return Math.max(0, Math.floor(Number(field.value) || 0));
}

/** The request for a fugue as the form has it, or the required field that is empty
 * (`missing`). */
function formRequest() {
  const own = control(THEME).value === OWN;
  const request = { command: "fugue" };
  for (const field of FIELDS) {
    if (field.own && !own) continue;
    const value = read(field);
    if (value !== undefined) {
      request[field.key] = value;
    } else if (field.required) {
      return { missing: field };
    }
  }
  request.seed = seed();
  return { request };
}

// ---- setting a fugue ----

/** The fugue shown, as the module answered, the request it answers and the name of its files
 * without their ending. */
let fugue = null;
let shownRequest = null;
let fugueFile = "";
/** Whether a fugue is being set. */
let setting = false;

/** Sets the fugue of `request` and shows it; the fugue shown stays if this one fails. With a
 * `version` it is that version set again (in another language); else the versions go. */
async function run(request, version = null) {
  stop();
  if (!version) dropVersions();
  const started = performance.now();
  const elapsed = () => (performance.now() - started) / 1000;
  const showElapsed = () => {
    $("elapsed").textContent = `${number(elapsed(), 1)} s`;
  };
  setBusy(true);
  setting = true;
  $("cancel").hidden = false;
  setStatus(() => t("working"));
  showElapsed();
  showNotation();
  const ticker = setInterval(showElapsed, 100);
  const answer = await ask(version ? { ...request, variant: variantText(version) } : request);
  clearInterval(ticker);
  $("elapsed").textContent = "";
  $("cancel").hidden = true;
  setting = false;
  setBusy(false);
  if (!answer.ok) showNotation();
  if (version && answer.ok) {
    version.answer = answer;
    version.lang = lang;
  }
  // The module has no clock: the time is the page's.
  const seconds = elapsed();
  if (answer.cancelled) {
    setStatus(() => t("cancelled"));
  } else if (!answer.ok) {
    setStatus(() => t(answer.broken ? "broken" : "error")(answer.error), true);
  } else {
    setStatus(() => t("done")(number(seconds, 1)));
    show(answer, request);
  }
}

/** Shows the fugue that answers `request`, in place of the one shown, which no longer plays. */
function show(answer, request) {
  stop();
  // Set again for another language, the fugue keeps the tempo chosen for it.
  const tempo = answer.playback?.quarters_per_minute;
  if (tempo && request !== shownRequest) $("tempo").value = tempo;
  fugue = answer;
  shownRequest = request;
  const letter = versions[versionShown]?.request === request
    ? `-${versions[versionShown].letter}`
    : "";
  fugueFile = `fuge-${answer.name ?? "thema"}-${request.seed}${letter}`;
  showFugue();
}

// ---- versions of a fugue ----

/** At most this many versions at a time. */
const MOST_VERSIONS = 8;

/** The versions set together: each with its letter, its request, the aspects it differs in
 * from the others (`differs`: key and value), the module's answer and the language of that
 * answer. `versionShown` is the index of the one shown, `revealed` whether the page says what
 * they differ in (not at once in a blind comparison). */
let versions = [];
let versionShown = -1;
let revealed = true;

/** What a version differs in, in the page's language: `Bass: geplant · Klang: nach dem Ohr`. */
function describe(version) {
  return version.differs
    .map(({ key, value }) => {
      if (key === "seed") return `Seed: ${value}`;
      const field = FIELDS.find((one) => one.key === key);
      const option = field.options.find((one) => one.value === value);
      return `${field.label[lang]}: ${option?.[lang] ?? value}`;
    })
    .join(" · ");
}

/** The aspects the versions differ in, by name. */
function aspects() {
  return (versions[0]?.differs ?? [])
    .map(({ key }) => (key === "seed" ? "Seed" : FIELDS.find((one) => one.key === key).label[lang]))
    .join(", ");
}

/** The words a version carries into its files: its letter, and what it differs in unless the
 * comparison is still blind. */
function variantText(version) {
  return t("version")(version.letter) + (revealed ? `: ${describe(version)}` : "");
}

/** The versions of `request` for the aspects ticked in the form: every combination of their
 * values, the form's own first, and of the further seeds. */
function versionsOf(request) {
  let list = [{ request, differs: [] }];
  for (const field of FIELDS.filter((one) => one.vary && axes.get(one.key).box.checked)) {
    const own = request[field.key] ?? field.value;
    const values = [own, ...field.options.map((one) => one.value).filter((one) => one !== own)];
    list = list.flatMap((version) =>
      values.map((value) => ({
        request: { ...version.request, [field.key]: value },
        differs: [...version.differs, { key: field.key, value }],
      }))
    );
  }
  const more = Math.max(0, Math.floor(Number($("more-seeds").value) || 0));
  if (more > 0) {
    list = list.flatMap((version) =>
      Array.from({ length: more + 1 }, (_, k) => ({
        request: { ...version.request, seed: request.seed + k },
        differs: [...version.differs, { key: "seed", value: request.seed + k }],
      }))
    );
  }
  return list;
}

/** The bar of the versions above the fugue: what they differ in, a button per version. */
function showVersions() {
  $("versions").hidden = versions.length === 0;
  $("versions-differ").textContent = versions.length === 0
    ? ""
    : revealed
    ? t("versions_differ")(aspects())
    : t("versions_hidden");
  $("version-tabs").replaceChildren(
    ...versions.map((version, index) => {
      const button = document.createElement("button");
      button.type = "button";
      button.setAttribute("id", `version-${version.letter}`);
      button.setAttribute("aria-pressed", String(index === versionShown));
      const letter = document.createElement("span");
      letter.className = "letter";
      letter.textContent = version.letter;
      const state = version.answer === null
        ? t("version_waiting")
        : version.answer.ok
        ? ""
        : t("version_failed");
      const text = [revealed ? describe(version) : "", state].filter(Boolean).join(" · ");
      button.append(letter, text);
      button.disabled = !version.answer?.ok;
      if (version.answer && !version.answer.ok) button.title = version.answer.error ?? "";
      button.onclick = () => showVersion(index);
      return button;
    }),
  );
  $("reveal").hidden = revealed || versions.length === 0;
  $("keep").disabled = versionShown < 0;
}

/** Shows a version; one set in another language is set again in the page's. */
function showVersion(index) {
  const version = versions[index];
  if (!version?.answer?.ok || busy) return;
  versionShown = index;
  showVersions();
  if (version.lang === lang) {
    show(version.answer, version.request);
  } else {
    run(version.request, version);
  }
}

/** Without versions: the fugue shown stands alone. */
function dropVersions() {
  versions = [];
  versionShown = -1;
  revealed = true;
  showVersions();
}

/** Sets the versions of `request` one after the other; each can be shown as soon as it is
 * there, and the first that comes out is. */
async function runVersions(request) {
  let list = versionsOf(request);
  if (list.length < 2) {
    setStatus(() => t("versions_none"), true);
    return;
  }
  if (list.length > MOST_VERSIONS) {
    const count = list.length;
    setStatus(() => t("versions_many")(count, MOST_VERSIONS), true);
    return;
  }
  const blind = $("blind").checked;
  if (blind) {
    // In a blind comparison the letters say nothing of the versions.
    list = list
      .map((version) => [Math.random(), version])
      .sort((a, b) => a[0] - b[0])
      .map((pair) => pair[1]);
  }
  stop();
  versions = list.map((version, index) => ({
    ...version,
    letter: "abcdefgh"[index],
    answer: null,
    lang,
  }));
  versionShown = -1;
  revealed = !blind;
  showVersions();
  const started = performance.now();
  const elapsed = () => (performance.now() - started) / 1000;
  const showElapsed = () => {
    $("elapsed").textContent = `${number(elapsed(), 1)} s`;
  };
  setBusy(true);
  setting = true;
  $("cancel").hidden = false;
  showElapsed();
  if (!fugue) showNotation();
  const ticker = setInterval(showElapsed, 100);
  let cancelled = false;
  for (const [index, version] of versions.entries()) {
    setStatus(() => t("working_version")(version.letter, index + 1, versions.length));
    const answer = await ask({ ...version.request, variant: variantText(version) });
    if (answer.cancelled) {
      cancelled = true;
      break;
    }
    version.answer = answer;
    version.lang = lang;
    if (answer.ok && versionShown < 0) {
      versionShown = index;
      show(answer, version.request);
    }
    showVersions();
  }
  clearInterval(ticker);
  $("elapsed").textContent = "";
  $("cancel").hidden = true;
  setting = false;
  setBusy(false);
  // What was not set does not stay in the bar.
  const shown = versions[versionShown];
  versions = versions.filter((version) => version.answer !== null);
  versionShown = versions.indexOf(shown);
  showVersions();
  if (!fugue) showNotation();
  const made = versions.filter((version) => version.answer.ok).length;
  const [count, seconds] = [list.length, elapsed()];
  if (cancelled) {
    setStatus(() => t("cancelled"));
  } else {
    setStatus(() => t("versions_done")(made, count, number(seconds, 1)), made === 0);
  }
}

/** Says what the versions of a blind comparison differ in. */
function reveal() {
  revealed = true;
  showVersions();
  showFugue();
}

/** Puts the version shown into the form, with everything its request holds: the next fugue
 * set is this one, whatever the form was changed to meanwhile. */
function keepVersion() {
  const version = versions[versionShown];
  if (!version) return;
  reveal();
  const request = version.request;
  control(THEME).value = request.theme ?? OWN;
  showTheme();
  for (const field of FIELDS) {
    // A subject of the module shows its own tones, key and meter.
    if (field === THEME || (field.own && request.theme !== undefined)) continue;
    control(field).value = String(request[field.key] ?? "");
  }
  $("seed").value = request.seed;
  $("new-seed").checked = false;
  const letter = version.letter;
  setStatus(() => t("kept")(letter));
}

/** Whether the fugue shown has tones to play. */
function playable() {
  return Boolean(fugue?.playback?.notes?.length);
}

function updateButtons() {
  $("play").disabled = !playable();
  $("stop").disabled = !playing;
  for (const file of FILES) {
    const button = $(`save-${file.key}`);
    button.textContent = file.label[lang] ?? file.label;
    button.disabled = typeof fugue?.files?.[file.key] !== "string";
  }
}

/** The fugue shown in the page's language: its title and size, the buttons, the report and the
 * notation. */
function showFugue() {
  $("title").textContent = fugue?.title ?? t("fugue");
  const size = fugue
    ? [
      `${fugue.key ?? ""} ${fugue.meter ?? ""}`.trim(),
      fugue.voices !== undefined && t("voices")(fugue.voices),
      fugue.bars !== undefined && t("bars")(fugue.bars),
      fugue.tones !== undefined && t("tones")(fugue.tones) +
        (fugue.fixed_tones !== undefined ? ` (${t("given")(fugue.fixed_tones)})` : ""),
      // The seed in the form is that of the next fugue, or of one that failed.
      `Seed ${shownRequest.seed}`,
      versions[versionShown]?.request === shownRequest && variantText(versions[versionShown]),
    ]
    : [];
  $("summary").textContent = size.filter(Boolean).join(", ");
  updateButtons();
  showReport();
  showNotation();
}

// ---- the report ----

/** `label: notes` in `line`, the notes as they are entered; hidden without notes. */
function showNotes(line, label, notes) {
  const code = document.createElement("code");
  code.textContent = notes ?? "";
  line.replaceChildren(`${label}: `, code);
  line.hidden = !notes;
}

/** Whether a measure is a share, shown in percent. The others are rates: their name says so
 * (»je«), or a number of theirs lies beyond one. */
function isShare(measure) {
  return !/ (je|per) /.test(measure.name ?? "") &&
    [measure.value, measure.low, measure.high].every((n) => typeof n !== "number" || n <= 1);
}

function measureRow(measure) {
  const share = isShare(measure);
  const shown = (value) => number(share ? 100 * value : value, share ? 1 : 2);
  const unit = (text) => (share ? t("percent")(text) : text);
  const has = (key) => typeof measure[key] === "number";
  const outside = measure.inside === false;
  const side = !outside
    ? ""
    : measure.value > measure.high
    ? "above"
    : measure.value < measure.low
    ? "below"
    : "outside";
  const row = document.createElement("tr");
  row.classList.toggle("outside", outside);
  const cell = (tag, className, text) => {
    const made = document.createElement(tag);
    made.className = className;
    made.textContent = text;
    row.append(made);
    return made;
  };
  cell("th", "name", measure.name ?? "").setAttribute("scope", "row");
  cell("td", "number value", has("value") ? unit(shown(measure.value)) : "");
  cell(
    "td",
    "number band",
    has("low") && has("high") ? unit(`${shown(measure.low)}–${shown(measure.high)}`) : "",
  );
  const mark = cell("td", "mark", { above: "▲", below: "▼", outside: "✗" }[side] ?? "");
  if (side) {
    mark.title = t(side);
    mark.setAttribute("aria-label", t(side));
  }
  return row;
}

function showReport() {
  $("report-empty").hidden = Boolean(fugue);
  $("report-body").hidden = !fugue;
  if (!fugue) return;
  showNotes($("answer"), fugue.answer?.kind ?? t("answer"), fugue.answer?.text);
  showNotes($("countersubject"), t("countersubject"), fugue.countersubject);
  $("sections").replaceChildren(
    ...(fugue.sections ?? []).map((section) => {
      const item = document.createElement("li");
      const text = t("section")(section.bar, section.name);
      if (!playable() || typeof section.tick !== "number") {
        item.textContent = text;
        return item;
      }
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = text;
      button.title = t("play_from");
      button.onclick = () => play(section.tick);
      item.append(button);
      return item;
    }),
  );
  // The ending, a stretto with the distance of its entries in bars.
  const apart = fugue.stretto_bars;
  const end = [
    fugue.ending && (t("endings")[fugue.ending] ?? fugue.ending),
    typeof apart === "number" && t("stretto_at")(number(apart, 2, 0), apart === 1),
  ].filter(Boolean).join(" ");
  $("ending").textContent = end ? t("ending")(end) : "";
  $("ending").hidden = !end;
  // The entries of the plan that a search for the subject finds again in the fugue.
  const { found, planned } = fugue.entries ?? {};
  const told = typeof found === "number" && typeof planned === "number";
  $("entries").textContent = told ? t("entries")(found, planned) : "";
  $("entries").hidden = !told;
  $("entries").classList.toggle("warn", told && found < planned);
  // The checker's words, and the prohibitions that the search could not help.
  const broken = Object.keys(t("counts")).filter((key) => fugue.counts?.[key] > 0);
  $("check").textContent = fugue.check?.summary ?? "";
  $("check").classList.toggle("warn", fugue.check?.forbidden > 0);
  $("counts").replaceChildren(
    ...broken.map((key) => {
      const line = document.createElement("p");
      line.textContent = t("counts")[key](fugue.counts[key]);
      return line;
    }),
  );
  $("check-box").hidden = !fugue.check?.summary && broken.length === 0;
  const measures = fugue.measures ?? [];
  $("measure-rows").replaceChildren(...measures.map(measureRow));
  const hits = $("hits");
  const counted = typeof fugue.hits === "number";
  const marked = counted && typeof fugue.yardstick === "number";
  const reached = !marked || fugue.hits >= fugue.yardstick;
  hits.textContent = [
    counted && t("hits")(fugue.hits, measures.length),
    marked && t("yardstick")(number(fugue.yardstick, 1, 0), reached),
  ].filter(Boolean).join(" ");
  hits.classList.toggle("warn", !reached);
}

// ---- the notation ----

let renderCount = 0;

/** Verovio's toolkit, or null if it does not come within half a minute. */
const notationToolkit = Promise.race([
  globalThis.verovioReady ?? Promise.resolve(null),
  new Promise((resolve) => setTimeout(() => resolve(null), 30_000)),
]);

/** What the notation shows: the MusicXML and the width it was engraved for. */
let engraved = null;

/** A line in place of the notation. */
function paperHint(text) {
  const hint = document.createElement("p");
  hint.className = "hint";
  hint.textContent = text;
  $("notation").replaceChildren(hint);
  engraved = null;
}

async function showNotation() {
  const paper = $("notation");
  const music = fugue?.files?.musicxml;
  const ticket = ++renderCount;
  if (typeof music !== "string") {
    // Before the first fugue: what to do, or that it is being set.
    paperHint(fugue ? "" : t(setting ? "working" : "empty"));
    return;
  }
  if (!engraved) paperHint(t("loading_notation"));
  const toolkit = await notationToolkit;
  // A newer fugue may have come in while Verovio loaded.
  if (ticket !== renderCount) return;
  if (!toolkit) {
    paperHint(t("no_notation"));
    return;
  }
  const scale = 40;
  const width = Math.max(500, paper.clientWidth - 20);
  if (engraved?.music === music && engraved.width === width) return;
  try {
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
    if (!toolkit.loadData(music)) throw new Error("loadData");
    let svg = "";
    for (let page = 1; page <= toolkit.getPageCount(); page++) {
      svg += toolkit.renderToSVG(page);
    }
    paper.innerHTML = svg;
    engraved = { music, width };
  } catch (error) {
    paperHint(`${t("notation_failed")} (${error.message ?? error})`);
  }
}

let resizeTimer = 0;
globalThis.addEventListener("resize", () => {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(showNotation, 250);
});

// ---- playback ----

/** Seconds that tones are scheduled ahead of the audio clock, and from one round of it to the
 * next. */
const AHEAD = 1.5;
const ROUND = 0.25;

let audio = null;
/** What plays: the oscillators scheduled and the timer of the rounds. */
let playing = null;

function organ(context) {
  // A soft organ: the fundamental, the octave and a little of the twelfth.
  const real = new Float32Array([0, 1, 0.45, 0.2, 0.08]);
  return context.createPeriodicWave(real, new Float32Array(real.length));
}

/** Plays the fugue shown from the tick `from`, in the tempo of the toolbar. The tones are
 * scheduled round by round, a little ahead of the audio clock: a fugue has hundreds. */
function play(from = 0) {
  stop();
  if (!playable()) return;
  const { notes, ticks_per_quarter: perQuarter = 24, quarters_per_minute: usual = 72 } =
    fugue.playback;
  audio ??= new AudioContext();
  // Suspended until a click, and on a phone interrupted by another app.
  if (audio.state !== "running") {
    audio.resume();
  }
  const wave = organ(audio);
  const tempo = Math.min(300, Math.max(20, Number($("tempo").value) || usual));
  const tick = 60 / (tempo * perQuarter);
  // [voice, on, off, key, given], on and off in ticks: those still to sound, by their onset.
  const left = notes.filter((note) => note[2] > from).sort((a, b) => a[1] - b[1]);
  const loudness = 0.3 / (new Set(notes.map((note) => note[0])).size || 1);
  const begin = audio.currentTime + 0.1;
  const time = (at) => begin + Math.max(0, at - from) * tick;
  const sounding = new Set();
  let next = 0;
  let last = begin;
  const round = () => {
    while (next < left.length && time(left[next][1]) < audio.currentTime + AHEAD) {
      const [, on, off, key] = left[next++];
      const start = time(on);
      const end = time(off);
      last = Math.max(last, end);
      // A round that came late (a tab the browser put to sleep) leaves out what is over.
      if (end <= audio.currentTime) continue;
      const oscillator = audio.createOscillator();
      oscillator.setPeriodicWave(wave);
      oscillator.frequency.value = 440 * 2 ** ((key - 69) / 12);
      const gain = audio.createGain();
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(loudness, start + 0.03);
      gain.gain.setValueAtTime(loudness, Math.max(start + 0.03, end - 0.06));
      gain.gain.linearRampToValueAtTime(0, end);
      oscillator.connect(gain).connect(audio.destination);
      oscillator.start(start);
      oscillator.stop(end + 0.02);
      oscillator.onended = () => sounding.delete(oscillator);
      sounding.add(oscillator);
    }
    if (next >= left.length && audio.currentTime > last + 0.2) stop();
  };
  playing = { sounding, timer: setInterval(round, ROUND * 1000) };
  round();
  updateButtons();
}

function stop() {
  if (!playing) return;
  clearInterval(playing.timer);
  for (const oscillator of playing.sounding) {
    try {
      oscillator.stop();
    } catch {
      // Already stopped.
    }
  }
  playing = null;
  updateButtons();
}

// ---- downloads ----

/** The files of a fugue, each with a button: `key` names it in the answer's `files`, `label` the
 * button, with one word or with one in each language; MIDI comes as base64. The chords are the
 * scaffold of the fugue, as `fux-fuge check --chords` reads it. */
const FILES = [
  {
    key: "musicxml",
    label: "MusicXML",
    ending: "musicxml",
    type: "application/vnd.recordare.musicxml+xml",
  },
  { key: "midi", label: "MIDI", ending: "mid", type: "audio/midi", base64: true },
  { key: "lilypond", label: "LilyPond", ending: "ly", type: "text/x-lilypond" },
  {
    key: "chords",
    label: { de: "Akkorde des Gerüsts", en: "Chords of the scaffold" },
    ending: "akkorde.txt",
    type: "text/plain",
  },
];

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

function buildDownloads() {
  $("downloads").replaceChildren(
    ...FILES.map((file) => {
      const button = document.createElement("button");
      button.type = "button";
      button.setAttribute("id", `save-${file.key}`);
      button.disabled = true;
      button.onclick = () => {
        const data = fugue?.files?.[file.key];
        if (typeof data !== "string") return;
        const bytes = file.base64 ? Uint8Array.from(atob(data), (c) => c.charCodeAt(0)) : data;
        saveFile(`${fugueFile}.${file.ending}`, bytes, file.type);
      };
      return button;
    }),
  );
}

// ---- start ----

$("form").onsubmit = (event) => {
  event.preventDefault();
  if (busy) return;
  const { request, missing } = formRequest();
  if (missing) {
    setStatus(() => t("missing")(missing.label[lang]), true);
    control(missing).focus();
    return;
  }
  run(request);
};

$("run-versions").onclick = () => {
  if (busy) return;
  const { request, missing } = formRequest();
  if (missing) {
    setStatus(() => t("missing")(missing.label[lang]), true);
    control(missing).focus();
    return;
  }
  runVersions(request);
};
$("reveal").onclick = reveal;
$("keep").onclick = keepVersion;

$("cancel").onclick = cancel;
$("play").onclick = () => play();
$("stop").onclick = stop;

$("lang").onchange = () => {
  lang = $("lang").value;
  translate();
  // The words of the module in the fugue shown (its title, its sections, those in its files)
  // follow with the same fugue set again: the same request gives the same tones. The other
  // versions follow when they are shown.
  showVersions();
  if (shownRequest && !busy) {
    const version = versions[versionShown];
    run(shownRequest, version?.request === shownRequest ? version : null);
  }
};

async function start() {
  buildForm();
  buildDownloads();
  // Where the page is one column, the help would stand between the button and the fugue.
  if (globalThis.matchMedia?.("(max-width: 720px)").matches) $("help").open = false;
  lang = navigator.language?.startsWith("de") ? "de" : "en";
  $("lang").value = lang;
  translate();
  setBusy(true);
  setStatus(() => t("loading"));
  const answer = await ask({ command: "themes" });
  setBusy(false);
  if (!answer.ok) {
    setStatus(() => t(answer.broken ? "broken" : "error")(answer.error), true);
    return;
  }
  themes = answer.themes ?? [];
  THEME.options = [
    ...themes.map((theme) => ({ value: theme.name, text: theme.title ?? theme.name })),
    ...THEME.options,
  ];
  fillOptions(THEME);
  PLAN.options = [...PLAN.options, ...(answer.plans ?? []).map((name) => ({ value: name }))];
  fillOptions(PLAN);
  // Something to set from the start. Tones entered meanwhile come back with „Eigene Töne“.
  if (themes.length) {
    control(THEME).value = themes[0].name;
    showTheme();
  }
  setStatus(() => t("ready"));
}

start();
