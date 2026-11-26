// Test of the page without a browser: web-fuge/index.html and web-fuge/app.js as they are, on the
// small DOM of web/page-dom.mjs, with a worker that answers from the WebAssembly module (or as
// the test tells it), a Verovio that engraves what it loads as it is, and an audio context that
// keeps what is scheduled:
//   node web-fuge/page-test.mjs target/web-fuge/fuge_web.wasm
//   deno run --allow-read web-fuge/page-test.mjs target/web-fuge/fuge_web.wasm
// It checks that the form is built from the subjects of the module, what a request holds for a
// subject of the module and for own tones, the fugue shown with its report, playback, downloads,
// errors of the module and of the worker, a cancel, both languages, an answer with unknown and
// missing fields, and a page without Verovio.
import { readFileSync } from "node:fs";
import process from "node:process";
import { makeDocument, Option } from "../web/page-dom.mjs";
import { Fuge } from "./fuge.js";

const path = process.argv[2] ?? "target/web-fuge/fuge_web.wasm";
const fuge = await Fuge.load(readFileSync(path));
let failures = 0;

function check(condition, what, detail = "") {
  if (condition) {
    console.log(`ok    ${what}`);
  } else {
    failures += 1;
    console.log(`FAIL  ${what} ${detail}`);
  }
}

// ---- the browser around the page ----

/** Gives the page a global; where the runtime has one of that name, this one takes its place. */
function define(name, value) {
  try {
    Object.defineProperty(globalThis, name, { value, configurable: true, writable: true });
  } catch {
    // A global the runtime keeps: the page does without this one.
  }
}

const html = readFileSync(new URL("index.html", import.meta.url), "utf8");
/** Every `a` element the page made: the downloads. */
const links = [];

/** A fresh document of the page, which the next import of app.js takes. */
function open(language) {
  const document = makeDocument(html);
  const create = document.createElement;
  document.createElement = (tag) => {
    const element = create(tag);
    if (tag === "a") links.push(element);
    return element;
  };
  define("document", document);
  define("navigator", { language });
  return document;
}

define("Option", Option);
if (typeof globalThis.addEventListener !== "function") define("addEventListener", () => {});

/** Verovio's toolkit as the page uses it: a page that holds what it loaded. */
const toolkit = {
  fails: false,
  setOptions(options) {
    this.options = options;
  },
  loadData(data) {
    this.data = data;
    return !this.fails;
  },
  getPageCount: () => 1,
  renderToSVG() {
    return `<svg>${this.data}</svg>`;
  },
};
define("verovioReady", Promise.resolve(toolkit));

/** Every request the page sent to a worker, and every worker it made. */
const requests = [];
const workers = [];
/** How the next request is answered: by the module, by this function, or ("hold") not until the
 * test answers it. */
let next;
/** The request a worker holds. */
let held = null;
/** Whether the browser refuses to start a worker. */
let refuses = false;
/** The last answer a worker gave. */
let answered = null;

/** A worker of the page: the module answers after a timer, as a worker answers. */
class PageWorker {
  constructor(url, options) {
    if (refuses) throw new Error("no workers here");
    this.made = [url, options];
    workers.push(this);
  }
  postMessage({ id, request }) {
    if (this.terminated) return;
    requests.push(request);
    const how = next;
    next = undefined;
    if (how === "hold") {
      held = { worker: this, id };
      return;
    }
    answered = how ? how(request) : fuge.ask(request);
    const answer = answered;
    setTimeout(() => this.answer(id, answer), 0);
  }
  answer(id, answer) {
    if (!this.terminated) this.onmessage?.({ data: { id, answer } });
  }
  terminate() {
    this.terminated = true;
  }
}
define("Worker", PageWorker);

/** An audio context that keeps the oscillators made, with their pitches and times. */
class Audio {
  constructor() {
    this.currentTime = 10;
    this.state = "suspended";
    this.destination = {};
    this.oscillators = [];
    Audio.last = this;
  }
  resume() {
    this.state = "running";
  }
  createPeriodicWave(real) {
    return [...real];
  }
  createOscillator() {
    const oscillator = {
      frequency: { value: 0 },
      setPeriodicWave(wave) {
        this.wave = wave;
      },
      connect: (node) => node,
      start(time) {
        this.from = time;
      },
      stop(time) {
        if (time === undefined) this.cut = true;
        else this.until = time;
      },
    };
    this.oscillators.push(oscillator);
    return oscillator;
  }
  createGain() {
    return { gain: { setValueAtTime() {}, linearRampToValueAtTime() {} }, connect: (node) => node };
  }
}
define("AudioContext", Audio);

/** The files the page handed over to be saved. */
const blobs = [];
URL.createObjectURL = (blob) => {
  blobs.push(blob);
  return `blob:test/${blobs.length}`;
};
URL.revokeObjectURL = () => {};

/** Waits until `condition` holds, at most a minute: a fugue takes seconds, more on a busy
 * machine. */
async function until(condition, what) {
  for (let i = 0; i < 6000 && !condition(); i++) {
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  if (!condition()) check(false, `waited for ${what}`);
}

// ---- the page ----

let document = open("de-DE");
const $ = (id) => document.getElementById(id);
const text = (id) => $(id).textContent;
const status = () =>
  `${text("status")}${$("status").classList.contains("error") ? " (error)" : ""}`;
const submit = () => $("form").querySelector("button.primary");
const idle = () => !document.body.classList.contains("busy");
/** Sets a field as a visitor does: its value, then the events of an entry or a choice. */
const enter = (id, value) => {
  const field = $(id);
  if (typeof value === "boolean") field.checked = value;
  else field.value = value;
  field.fire(field.localName === "select" || typeof value === "boolean" ? "change" : "input");
};
/** Clicks „Fuge setzen“ and waits for the answer; returns the request sent, if any. */
const set = async () => {
  const before = requests.length;
  submit().click();
  await until(idle, "the answer");
  return requests[before];
};
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const rows = () => $("measure-rows").children;
const cells = (row) => row.children.map((cell) => cell.textContent);
const row = (name) => rows().find((one) => one.children[0].textContent === name);

check(
  submit().disabled && text("status") === "Lade das Programm …" &&
    $("fields").children.length === 0,
  "before its script runs, the page says that it loads, and its button waits",
  text("status"),
);
/** The elements of the markup with a text of the script, and what they read: without the tags
 * of a text with markup, and without white space, which the markup breaks elsewhere. */
const translated = document.querySelectorAll("[data-t]");
const reading = () =>
  translated.map((element) =>
    `${element.dataset.t}: ${element.innerHTML.replace(/<[^>]+>|\s+/g, "")}`
  );
const markup = reading();
await import(new URL("app.js", import.meta.url).href);
await until(() => text("status") === "Bereit.", "the subjects of the module");
check(!submit().disabled && idle(), "with the subjects of the module the button is free");
check(
  translated.length > 15 && same(reading(), markup),
  "the German texts of the markup are those of the script",
  reading().filter((line, i) => line !== markup[i]).join("; "),
);

const { themes, plans } = fuge.ask({ command: "themes" });
const first = themes[0];
check(
  same(requests, [{ lang: "de", command: "themes" }]) && workers.length === 1 &&
    same(workers[0].made, ["worker.js", { type: "module" }]),
  "the page starts a module worker and asks for the subjects",
  JSON.stringify(requests),
);
const options = $("f-theme").options.map((option) => `${option.value}=${option.textContent}`);
check(
  same(options, [...themes.map((theme) => `${theme.name}=${theme.title}`), "=Eigene Töne …"]),
  "the select lists the subjects of the module, then own tones",
  options.join("; "),
);
const shown = () =>
  ["subject", "key", "meter", "period_bars", "title"].map((key) => $(`f-${key}`).value);
check(
  $("f-theme").value === first.name &&
    same(shown(), [first.subject, first.key, first.meter, String(first.period_bars), first.title]),
  "the first subject is chosen and shows its tones, key, meter and distance",
  JSON.stringify(shown()),
);
const labels = () =>
  $("fields").querySelectorAll("label").map((label) => label.querySelector(".name").textContent);
check(
  same(labels(), [
    "Auswahl",
    "Töne",
    "Tonart",
    "Takt",
    "Einsatzabstand (Takte)",
    "Titel",
    "Eigene Antwort",
    "Stimmen",
    "Schluss",
    "Freie Stimmen",
    "Klang",
    "Bass",
    "Plan der Einsätze",
    "Qualität",
  ]) &&
    same($("fields").querySelectorAll("legend").map((one) => one.textContent), ["Thema", "Fuge"]) &&
    submit().textContent === "Fuge setzen",
  "the form is built from the list of fields, in German",
  JSON.stringify(labels()),
);
/** The choices of a select, each as value=text. */
const choices = (id) => $(id).options.map((option) => `${option.value}=${option.textContent}`);
check(
  same(choices("f-voices"), ["2=2", "3=3", "4=4"]) && $("f-voices").value === "3" &&
    same(choices("f-ending"), ["auto=automatisch", "pedal=Orgelpunkt", "stretto=Engführung"]) &&
    $("f-ending").value === "auto" && plans.length > 0 &&
    same(choices("f-free_voices"), [
      "figures=mit Figuren und Sequenzen",
      "plain=schlicht (schneller)",
    ]) && $("f-free_voices").value === "figures" &&
    same(choices("f-profile"), ["fuge=nach dem Ohr", "bach=nach der Zählung"]) &&
    $("f-profile").value === "fuge" &&
    same(choices("f-bass"), ["free=frei", "planned=geplant"]) && $("f-bass").value === "free" &&
    same(choices("f-plan"), ["=vom Programm", ...plans.map((plan) => `${plan}=${plan}`)]) &&
    $("f-plan").value === "" &&
    same(choices("f-beam"), ["200=schnell", "400=normal", "800=gründlich"]) &&
    $("f-beam").value === "400",
  "voices, ending, free voices, sound, bass, plan and quality are choices; the plans are " +
    "those of the module",
  JSON.stringify(
    ["f-voices", "f-ending", "f-free_voices", "f-profile", "f-bass", "f-plan", "f-beam"].map(
      (id) => $(id).value,
    ),
  ),
);
const keyboard = (id) =>
  ["autocapitalize", "autocorrect", "spellcheck"].map((name) =>
    name === "spellcheck" ? $(id).spellcheck : $(id).getAttribute(name)
  );
check(
  ["f-subject", "f-key", "f-meter", "f-answer"].every((id) =>
    same(keyboard(id), ["off", "off", false])
  ) && same(keyboard("f-title"), [null, null, undefined]) &&
    $("f-period_bars").type === "number" && $("f-key").type === "text" &&
    $("form").getAttribute("novalidate") !== null,
  "the notation is kept from a phone's capitals and corrections; the browser validates nothing",
  JSON.stringify(["f-subject", "f-key", "f-meter", "f-title"].map(keyboard)),
);
const hints = () =>
  $("fields").querySelectorAll(".hint").map((hint) => (hint.hidden ? null : hint.textContent));
check(
  same(hints(), [
    "Wer ein Feld darunter ändert, setzt ein eigenes Thema.",
    null,
    null,
    null,
    "Leer: nach der Länge des Themas.",
    null,
    "Leer: nach der Regel. Sonst so viele Töne wie das Thema, in dessen Rhythmus.",
    "Drei Stimmen brauchen im Browser etwa 3 bis 15 Sekunden, vier 10 bis 70 (je nach Thema " +
    "und Gerät).",
    "Automatisch: eine Engführung, wenn das Thema eine hergibt, sonst ein Orgelpunkt.",
    "Mit Figuren: Sequenzen in allen Stimmen der Zwischenspiele und Figuren aus Thema und " +
    "Kontrasubjekt; die Fuge wird dafür zwei- bis viermal gesetzt. Schlicht: ein Satz aus " +
    "Rhythmustabellen, im Mittel drei- bis fünfmal so schnell.",
    "Nach dem Ohr: zusammen angeschlagene Dissonanz kostet mehr, und drei Stimmen bilden öfter " +
    "den vollen Dreiklang (nach blinden Hörproben). Nach der Zählung: die Preise, die an Bachs " +
    "Raten geeicht sind.",
    "Geplant: das Gerüst nennt je Akkord einen Basston, nach der Bezifferung von Bachs Fugen. " +
    "Gezählt liegt dann der Grundton so oft zuunterst wie bei Bach; in einer blinden Hörprobe " +
    "klang es gleich.",
    "Die Einsätze nach der Exposition wie in einer Fuge aus WTK I.",
    "Die Breite der Suche: mehr ist langsamer und selten besser.",
  ]),
  "a field with a hint shows it under its control",
  JSON.stringify(hints()),
);
check(
  $("play").disabled && $("stop").disabled && $("save-midi").disabled && $("report-body").hidden &&
    !$("report-empty").hidden && $("cancel").hidden && text("title") === "Fuge" &&
    text("summary") === "" && $("help").open !== false &&
    text("notation") === "Ein Thema wählen oder eigene Töne eingeben, dann „Fuge setzen“.",
  "before a fugue: nothing to play or to save, a hint in place of the notes",
  text("notation"),
);

// ---- a fugue on a subject of the module ----

enter("f-theme", "bwv1080");
enter("f-beam", "200");
enter("new-seed", false);
$("seed").value = "1";
next = "hold";
submit().click();
check(
  submit().disabled && !$("cancel").hidden && $("lang").disabled &&
    status() === "Setze die Fuge …" && /^\d,\d s$/.test(text("elapsed")) &&
    text("notation") === "Setze die Fuge …",
  "while a fugue is set: the status with the time, „Abbrechen“, no second fugue",
  `${status()} ${text("elapsed")} ${text("notation")}`,
);
const asked = requests.at(-1);
const art = fuge.ask(asked);
held.worker.answer(held.id, art);
await until(idle, "the fugue");
check(
  same(asked, {
    lang: "de",
    command: "fugue",
    theme: "bwv1080",
    voices: 3,
    ending: "auto",
    free_voices: "figures",
    profile: "fuge",
    bass: "free",
    beam: 200,
    seed: 1,
  }),
  "a subject of the module: the request names it, not its tones, and leaves out empty fields",
  JSON.stringify(asked),
);
check(
  /^Fuge gesetzt in \d+,\d s\.$/.test(status()) && text("elapsed") === "" && $("cancel").hidden &&
    !submit().disabled,
  "the status tells the time the page measured",
  status(),
);
check(
  text("title") === art.title && art.title.includes("Kunst der Fuge") &&
    text("summary") ===
      `${art.key} ${art.meter}, ${art.voices} Stimmen, ${art.bars} Takte, ${art.tones} Töne ` +
        `(${art.fixed_tones} vorgegeben), Seed 1` &&
    art.voices === 3 &&
    $("notation").innerHTML === `<svg>${art.files.musicxml}</svg>` &&
    toolkit.options.pageWidth === 1950 && toolkit.options.header === "none",
  "the fugue: its title, its voices, size and seed, its MusicXML engraved for the page's width",
  `${text("title")}; ${text("summary")}`,
);
const sections = () => $("sections").children.map((item) => item.textContent);
/** The sections of an answer as the list shows them, each with the word for its bar. */
const listed = (answer, bar) => answer.sections.map((s) => `${bar} ${s.bar}: ${s.name}`);
check(
  !$("report-body").hidden && $("report-empty").hidden &&
    text("answer") === `${art.answer.kind}: ${art.answer.text}` &&
    text("countersubject") === `Kontrasubjekt: ${art.countersubject}` &&
    same(sections(), listed(art, "Takt")) && sections().length > 2,
  "the report: the answer, the countersubject and the sections as they come",
  `${text("answer")}; ${sections().join(", ")}`,
);
/** The lines on prohibitions that the search could not help. */
const counts = () => $("counts").children.map((line) => line.textContent);
check(
  text("ending") === `Schluss: ${{ pedal: "Orgelpunkt", stretto: "Engführung" }[art.ending]}.` &&
    art.stretto_bars === null && !$("ending").hidden &&
    text("entries") ===
      `${art.entries.found} von ${art.entries.planned} Einsätzen wiedergefunden.` &&
    art.entries.planned > 3 && !$("entries").classList.contains("warn") &&
    sections().some((section) => /^Takt \d+: \S+ \(\S+\)$/.test(section)),
  "the ending, the entries found again, and sections that name the key of their entry",
  `${text("ending")} ${text("entries")} ${sections().join(", ")}`,
);
check(
  !$("check-box").hidden && text("check") === art.check.summary &&
    /^\d+ Verbot/.test(art.check.summary) &&
    $("check").classList.contains("warn") === art.check.forbidden > 0 &&
    art.counts.own === 0 && art.counts.forced === 0 && counts().length === 0,
  "the check in the module's words; no line on prohibitions of given tones where there is none",
  `${text("check")} ${counts().join(" ")}`,
);
const outside = art.measures.filter((measure) => !measure.inside);
const count = art.measures.length;
check(
  rows().length === count && count > 0 &&
    same(rows().map((one) => cells(one)[0]), art.measures.map((m) => m.name)) &&
    rows().filter((one) => one.classList.contains("outside")).length === outside.length &&
    outside.length === count - art.hits,
  `the table has the ${count} measures of the answer, those outside their band marked`,
  `${rows().length} rows`,
);
/** A number of the module as German or English writes it, with `digits` decimals if given. */
const decimal = (value, digits, english = false) => {
  const written = digits === undefined ? String(value) : value.toFixed(digits);
  return english ? written : written.replace(".", ",");
};
/** Whether a measure of the module is a rate, as its name says; the others are shares. */
const isRate = (measure) => / (je|per) /.test(measure.name);
/** The value and the band of a measure as the table shows them: a rate as it is, a share in
 * percent. */
const written = ({ name, value, low, high }, english = false) => {
  if (isRate({ name })) {
    const [is, from, to] = [value, low, high].map((part) => decimal(part, 2, english));
    return [is, `${from}–${to}`];
  }
  const [is, from, to] = [value, low, high].map((part) => decimal(100 * part, 1, english));
  return english ? [`${is}%`, `${from}–${to}%`] : [`${is} %`, `${from}–${to} %`];
};
const numbers = () => rows().map((one) => cells(one).slice(1, 3));
const rates = art.measures.filter(isRate).length;
check(
  same(numbers(), art.measures.map((measure) => written(measure))) && rates > 0 && rates < count &&
    numbers().some(([value, band]) => /^\d+,\d %$/.test(value) && /^\d+,\d–\d+,\d %$/.test(band)) &&
    numbers().some(([value, band]) => /^\d+,\d\d$/.test(value) && /^\d+,\d\d–\d+,\d\d$/.test(band)),
  `shares show in percent with Bach's band, the ${rates} rates as they are`,
  JSON.stringify(numbers()),
);
const marks = outside.map((measure) => {
  const mark = row(measure.name).children[3];
  return `${mark.textContent} ${mark.title} ${mark.getAttribute("aria-label")}`;
});
check(
  outside.length > 0 && same(
    marks,
    outside.map((m) =>
      m.value > m.high ? "▲ über dem Band über dem Band" : "▼ unter dem Band unter dem Band"
    ),
  ),
  "a value outside its band is marked as above or below it",
  marks.join("; "),
);
const reached = art.hits >= art.yardstick;
check(
  text("hits") ===
      `${art.hits} von ${count} Kennzahlen liegen in Bachs Bändern. Der Maßstab ist ` +
        `${decimal(art.yardstick)}: ${reached ? "erreicht" : "nicht erreicht"}.` &&
    $("hits").classList.contains("warn") === !reached,
  "the hits against the yardstick",
  text("hits"),
);

// ---- downloads ----

const buttons = () => $("downloads").children.map((button) => button.textContent);
check(
  same(buttons(), ["MusicXML", "MIDI", "LilyPond", "Akkorde des Gerüsts"]) &&
    $("downloads").children.every((button) => !button.disabled),
  "a button for each file of the fugue",
  buttons().join(", "),
);
for (const button of $("downloads").children) button.click();
const saved = links.map((link, i) => `${link.download} ${link.href} ${blobs[i]?.type}`);
check(
  same(saved, [
    "fuge-bwv1080-1.musicxml blob:test/1 application/vnd.recordare.musicxml+xml",
    "fuge-bwv1080-1.mid blob:test/2 audio/midi",
    "fuge-bwv1080-1.ly blob:test/3 text/x-lilypond",
    "fuge-bwv1080-1.akkorde.txt blob:test/4 text/plain",
  ]) && links.every((link) => link.parentNode === null),
  "the four files are saved under the name of the subject and the seed",
  saved.join("; "),
);
const midi = new Uint8Array(await blobs[1].arrayBuffer());
check(
  await blobs[0].text() === art.files.musicxml && await blobs[2].text() === art.files.lilypond &&
    await blobs[3].text() === art.files.chords && art.files.chords.startsWith("1: ") &&
    new TextDecoder().decode(midi.subarray(0, 4)) === "MThd" &&
    midi.length === atob(art.files.midi).length,
  "with the MusicXML, the LilyPond and the chords of the answer, and its MIDI as bytes",
  `${midi.length} bytes of MIDI`,
);

// ---- playback ----

const notes = [...art.playback.notes].sort((a, b) => a[1] - b[1]);
/** The seconds of a tick at `tempo` quarters per minute. */
const tick = (tempo) => 60 / (tempo * art.playback.ticks_per_quarter);
const near = (a, b) => Math.abs(a - b) < 1e-9;
const usual = art.playback.quarters_per_minute;
check(
  $("tempo").value === String(usual),
  "the tempo of the fugue shows in the toolbar",
  $("tempo").value,
);
$("play").click();
let audio = Audio.last;
// A round schedules what begins within a second and a half; the first tone 0.1 s from now.
const early = notes.filter((note) => 10.1 + note[1] * tick(usual) < 11.5);
check(
  audio.state === "running" && audio.oscillators.length === early.length && early.length > 0 &&
    early.length < notes.length && !$("stop").disabled &&
    near(audio.oscillators[0].from, 10.1) &&
    near(audio.oscillators[0].frequency.value, 440 * 2 ** ((notes[0][3] - 69) / 12)) &&
    near(audio.oscillators[0].until, 10.1 + notes[0][2] * tick(usual) + 0.02) &&
    same(audio.oscillators[0].wave, [0, 1, 0.45, 0.2, 0.08].map(Math.fround)),
  "„Abspielen“ schedules the first tones in the fugue's tempo, with the organ's wave",
  `${audio.oscillators.length} of ${notes.length} tones`,
);
/** Lets a round of the playback run with the audio clock at `time`. */
const clock = async (time) => {
  audio.currentTime = time;
  await new Promise((resolve) => setTimeout(resolve, 300));
};
// A round that comes late, in the middle of the fugue (the browser put the tab to sleep): the
// tones that are over are left out, those that still sound or begin soon are scheduled.
const length = Math.max(...notes.map((note) => note[2]));
const late = 10.1 + (length / 2) * tick(usual);
await clock(late);
const due = notes.filter((note) =>
  !early.includes(note) && 10.1 + note[1] * tick(usual) < late + 1.5
);
const sound = due.filter((note) => 10.1 + note[2] * tick(usual) > late);
check(
  sound.length > 0 && sound.length < due.length &&
    audio.oscillators.length === early.length + sound.length &&
    audio.oscillators.slice(early.length).every((oscillator, i) =>
      near(oscillator.from, 10.1 + sound[i][1] * tick(usual))
    ) && !$("stop").disabled,
  "a round that comes late leaves out the tones that are over",
  `${audio.oscillators.length - early.length} tones, ${sound.length} of ${due.length} expected`,
);
await clock(10.1 + length * tick(usual) + 1);
check(
  $("stop").disabled && audio.oscillators.length === early.length + sound.length,
  "after the last tone the playback ends by itself",
);
// From a section, in another tempo: a tone held over its start sounds from there, at once.
const slow = usual / 2;
$("tempo").value = String(slow);
audio.oscillators.length = 0;
audio.currentTime = 20;
const heldOver = (at) => notes.filter((note) => note[1] < at && note[2] > at);
const entry = art.sections.findIndex((section) => heldOver(section.tick).length > 0);
const from = art.sections[entry]?.tick;
const button = $("sections").children[entry]?.querySelector("button");
check(
  entry > 0 && button?.title === "Ab hier abspielen",
  "a section is a button that plays from it",
  `section ${entry}`,
);
button.click();
const begins = notes.filter((note) =>
  note[2] > from && 20.1 + (note[1] - from) * tick(slow) < 21.5
);
check(
  audio.oscillators.length === begins.length &&
    audio.oscillators.every((oscillator, i) =>
      near(oscillator.from, 20.1 + Math.max(0, begins[i][1] - from) * tick(slow)) &&
      near(oscillator.until, 20.1 + (begins[i][2] - from) * tick(slow) + 0.02)
    ) && audio.oscillators.filter((oscillator) => near(oscillator.from, 20.1)).length >=
      heldOver(from).length,
  "a section plays from its bar in the tempo of the toolbar, with the tones held over it",
  `${audio.oscillators.length} tones, ${begins.length} expected, ${heldOver(from).length} held`,
);
$("stop").click();
check(
  $("stop").disabled && audio.oscillators.every((oscillator) => oscillator.cut),
  "„Stopp“ ends every tone scheduled",
);
// The last section whole, the audio clock a second on with every round: every tone at its time.
// The audio context was interrupted meanwhile, as a phone does it for another app.
$("tempo").value = String(usual);
audio.oscillators.length = 0;
audio.currentTime = 30;
audio.state = "interrupted";
const close = art.sections.at(-1).tick;
$("sections").children.at(-1).querySelector("button").click();
const tail = notes.filter((note) => note[2] > close);
const ending = 30.1 + (length - close) * tick(usual);
for (let time = 31; time < ending; time += 1) await clock(time);
check(
  tail.length > 0 && audio.state === "running" && audio.oscillators.length === tail.length &&
    audio.oscillators.every((oscillator, i) =>
      near(oscillator.from, 30.1 + Math.max(0, tail[i][1] - close) * tick(usual))
    ) && !$("stop").disabled,
  "with the clock going on, the rounds schedule every tone at its time",
  `${audio.oscillators.length} of ${tail.length} tones`,
);
await clock(ending + 1);
check($("stop").disabled, "and the playback ends after the last tone");
// A fugue that takes the place of the one shown ends its playback.
next = "hold";
submit().click();
$("play").click();
const sounding = audio.oscillators.slice(tail.length);
check(!$("stop").disabled && sounding.length > 0, "the fugue shown plays while another is set");
held.worker.answer(held.id, fuge.ask(requests.at(-1)));
await until(idle, "the other fugue");
check(
  $("stop").disabled && sounding.every((oscillator) => oscillator.cut),
  "the fugue that takes its place ends the playback",
);

// ---- own tones ----

const own = "d'2 a' | f' d' | cis' d'4 e' | f'2~ f'8 g' f' e' | d'4";
enter("f-title", "Probe");
check($("f-theme").value === "", "a changed field makes the subject one's own");
let request = await set();
check(
  same(request, {
    lang: "de",
    command: "fugue",
    subject: own,
    key: "d",
    meter: "2/2",
    period_bars: 4,
    title: "Probe",
    voices: 3,
    ending: "auto",
    free_voices: "figures",
    profile: "fuge",
    bass: "free",
    beam: 200,
    seed: 1,
  }),
  "own tones: the request holds them with key, meter, distance and title, and no subject's name",
  JSON.stringify(request),
);
const probe = answered;
check(
  text("title") === probe.title && probe.title.includes("Probe") &&
    /^Fuge gesetzt in /.test(status()) &&
    text("summary").startsWith(`${probe.key} ${probe.meter}, 3 Stimmen, ${probe.bars} Takte, `),
  "the fugue on own tones, under its title",
  `${text("title")}; ${status()}`,
);
$("save-midi").click();
check(
  links.at(-1).download === `fuge-${probe.name}-1.mid` &&
    !themes.some((t) => t.name === probe.name),
  "its files are named as the module names a subject of one's own",
  links.at(-1).download,
);
enter("f-period_bars", "");
enter("f-title", "");
enter("f-subject", own.replaceAll("'", "’"));
request = await set();
check(
  !("period_bars" in request) && !("title" in request) && request.subject === own &&
    text("title") === answered.title,
  "an empty field is left out of the request; typographic octave marks are read as plain ones",
  JSON.stringify(request),
);
enter("f-subject", own);
enter("f-theme", first.name);
check(
  $("f-subject").value === first.subject,
  "a subject of the module takes the place of the tones",
);
enter("f-theme", "");
check(
  same(shown(), [own, "d", "2/2", "", ""]),
  "„Eigene Töne“ brings back what was entered",
  JSON.stringify(shown()),
);
enter("new-seed", true);
const drawn = [(await set()).seed, (await set()).seed];
check(
  drawn.every((seed) => Number.isInteger(seed) && seed >= 0) && drawn[0] !== drawn[1] &&
    $("seed").value === String(drawn[1]),
  "„neuer Seed“ draws a seed for every fugue and shows it",
  drawn.join(),
);
enter("new-seed", false);
$("seed").value = "1";

// ---- voices, a plan, an ending, an answer of one's own ----

enter("f-theme", "bwv1080");
enter("f-voices", "4");
enter("f-plan", plans[1]);
request = await set();
check(
  same(request, {
    lang: "de",
    command: "fugue",
    theme: "bwv1080",
    voices: 4,
    ending: "auto",
    free_voices: "figures",
    profile: "fuge",
    bass: "free",
    plan: plans[1],
    beam: 200,
    seed: 1,
  }) && answered.voices === 4 &&
    new Set(answered.playback.notes.map((note) => note[0])).size === 4 &&
    text("summary").startsWith(`${answered.key} ${answered.meter}, 4 Stimmen, `) &&
    text("entries") ===
      `${answered.entries.found} von ${answered.entries.planned} Einsätzen wiedergefunden.` &&
    answered.entries.planned !== art.entries.planned &&
    $("notation").innerHTML === `<svg>${answered.files.musicxml}</svg>`,
  "four voices after a plan of the module: the request names both, the fugue shows them",
  `${JSON.stringify(request)} ${text("summary")} ${text("entries")}`,
);
enter("f-voices", "3");
enter("f-plan", "");
enter("f-ending", "stretto");
request = await set();
check(
  request.ending === "stretto" && !("plan" in request) && answered.ok === false &&
    answered.error.includes("stretto") && status() === `Fehler: ${answered.error} (error)`,
  "an ending that the subject does not give is an error of the module, shown in its words",
  status(),
);
enter("f-ending", "pedal");
enter("f-answer", art.answer.text.replaceAll("'", "’"));
request = await set();
check(
  request.ending === "pedal" && request.answer === art.answer.text && answered.ok &&
    text("ending") === "Schluss: Orgelpunkt." &&
    text("answer") === `${answered.answer.kind}: ${art.answer.text}`,
  "an answer of one's own goes with the request, its octave marks plain, and is the fugue's",
  `${JSON.stringify(request)} ${text("answer")}`,
);
const figured = answered;
enter("f-free_voices", "plain");
request = await set();
check(
  request.free_voices === "plain" && answered.ok && answered.bars === figured.bars &&
    answered.files.midi !== figured.files.midi &&
    text("summary").includes(`${answered.tones} Töne`),
  "the free voices plain: the request says so, and the fugue is another one of the same plan",
  `${JSON.stringify(request)} ${text("summary")}`,
);
enter("f-free_voices", "figures");
enter("f-profile", "bach");
request = await set();
check(
  request.profile === "bach" && answered.ok && answered.bars === figured.bars &&
    answered.files.midi !== figured.files.midi,
  "the sound by the count: the request names the profile, and the fugue is another one",
  JSON.stringify(request),
);
enter("f-profile", "fuge");

// ---- versions of a fugue ----

/** The buttons of the versions, each as its letter and what it says. */
const tabs = () => $("version-tabs").children.map((button) => button.textContent);
/** Clicks „Fassungen setzen“ and waits; returns the requests sent. */
const setVersions = async () => {
  const before = requests.length;
  $("run-versions").click();
  await until(idle, "the versions");
  return requests.slice(before);
};
check(
  same(
    $("axes").querySelectorAll("label").map((label) => label.textContent.trim()),
    ["Freie Stimmen", "Klang", "Bass"],
  ) && $("versions").hidden && !$("run-versions").disabled,
  "versions can differ in the fields marked for it: a checkbox for each",
  JSON.stringify($("axes").querySelectorAll("label").map((label) => label.textContent)),
);
let sent = await setVersions();
check(
  sent.length === 0 && $("versions").hidden &&
    status() === "Für Fassungen einen Aspekt ankreuzen oder weitere Seeds wählen. (error)",
  "without an aspect to differ in there are no versions",
  status(),
);
enter("vary-bass", true);
sent = await setVersions();
const askedForVersions = requests.length;
check(
  sent.length === 2 && sent[0].bass === "free" && sent[1].bass === "planned" &&
    sent[0].variant === "Fassung a: Bass: frei" &&
    sent[1].variant === "Fassung b: Bass: geplant" &&
    same({ ...sent[0], bass: 0, variant: 0 }, { ...sent[1], bass: 0, variant: 0 }),
  "two versions that differ in the bass alone; each request names its version",
  JSON.stringify(sent),
);
check(
  !$("versions").hidden &&
    text("versions-differ") ===
      "Die Fassungen unterscheiden sich in: Bass. Alles andere ist gleich." &&
    same(tabs(), ["aBass: frei", "bBass: geplant"]) &&
    $("version-a").getAttribute("aria-pressed") === "true" &&
    $("version-b").getAttribute("aria-pressed") === "false" && $("reveal").hidden &&
    text("summary").endsWith(", Fassung a: Bass: frei") &&
    /^2 von 2 Fassungen gesetzt in [\d,]+ s\.$/.test(status()),
  "the page says what the versions differ in, names each by it and shows the first",
  `${text("versions-differ")} ${JSON.stringify(tabs())} ${text("summary")} ${status()}`,
);
const firstVersion = $("notation").innerHTML;
$("version-b").click();
await until(() => $("notation").innerHTML !== firstVersion, "the notes of version b");
check(
  $("version-b").getAttribute("aria-pressed") === "true" &&
    text("summary").endsWith(", Fassung b: Bass: geplant") &&
    $("notation").innerHTML !== firstVersion && requests.length === askedForVersions &&
    $("notation").innerHTML.includes("Fassung b: Bass: geplant"),
  "a click shows the other version without setting it again; its files carry its name",
  text("summary"),
);
$("save-midi").click();
check(
  links.at(-1).download === "fuge-bwv1080-1-b.mid",
  "the files of a version are named with its letter",
  links.at(-1).download,
);
// The form may have changed since the versions were set: taking one brings all of it back.
enter("f-voices", "4");
enter("f-theme", first.name);
$("keep").click();
check(
  $("f-bass").value === "planned" && $("seed").value === "1" && !$("new-seed").checked &&
    $("f-voices").value === "3" && $("f-theme").value === "bwv1080" &&
    $("f-answer").value === art.answer.text && $("f-ending").value === "pedal" &&
    status() === "Fassung b steht im Formular." && !$("versions").hidden,
  "the version taken stands in the form, with all its request held",
  `${$("f-bass").value} ${$("f-voices").value} ${$("f-theme").value} ${status()}`,
);
// Blind: the letters are drawn, and what the versions differ in shows only when asked for.
enter("f-bass", "free");
enter("blind", true);
const random = Math.random;
const draws = [0.9, 0.1];
Math.random = () => draws.shift() ?? 0.5;
sent = await setVersions();
Math.random = random;
check(
  sent.length === 2 && sent[0].bass === "planned" && sent[1].bass === "free" &&
    sent[0].variant === "Fassung a" && sent[1].variant === "Fassung b" &&
    same(tabs(), ["a", "b"]) && !$("reveal").hidden &&
    text("versions-differ") ===
      "Blind: Worin sich die Fassungen unterscheiden, zeigt „Auflösen“." &&
    text("summary").endsWith(", Fassung a"),
  "blind versions: drawn letters, and nothing says what they differ in",
  `${JSON.stringify(sent.map((one) => [one.bass, one.variant]))} ${JSON.stringify(tabs())}`,
);
$("reveal").click();
check(
  same(tabs(), ["aBass: geplant", "bBass: frei"]) && $("reveal").hidden &&
    text("summary").endsWith(", Fassung a: Bass: geplant") &&
    text("versions-differ").startsWith("Die Fassungen unterscheiden sich in: Bass."),
  "„Auflösen“ says what the versions differ in",
  JSON.stringify(tabs()),
);
enter("blind", false);
// Seeds are an aspect too; too many versions are refused before any is set.
enter("vary-bass", false);
enter("more-seeds", "1");
sent = await setVersions();
check(
  sent.length === 2 && sent[0].seed === 1 && sent[1].seed === 2 &&
    same(tabs(), ["aSeed: 1", "bSeed: 2"]) &&
    text("versions-differ").startsWith("Die Fassungen unterscheiden sich in: Seed."),
  "further seeds give versions that differ in the seed",
  JSON.stringify(tabs()),
);
for (const key of ["free_voices", "profile", "bass"]) enter(`vary-${key}`, true);
sent = await setVersions();
check(
  sent.length === 0 &&
    status() === "Das wären 16 Fassungen; es gehen höchstens 8 auf einmal. (error)" &&
    same(tabs(), ["aSeed: 1", "bSeed: 2"]),
  "more versions than the page sets at a time are refused; those shown stay",
  status(),
);
for (const key of ["free_voices", "profile", "bass"]) enter(`vary-${key}`, false);
enter("more-seeds", "0");
// A fugue set alone takes the place of the versions.
await set();
check(
  $("versions").hidden && tabs().length === 0 && !text("summary").includes("Fassung"),
  "a single fugue takes the place of the versions",
  text("summary"),
);
enter("f-ending", "auto");
enter("f-theme", first.name);
check(
  $("f-answer").value === "" && $("f-subject").value === first.subject,
  "another subject empties the answer, which belonged to the last one",
  $("f-answer").value,
);
enter("f-theme", "");
check(same(shown(), [own, "d", "2/2", "", ""]), "and own tones come back as they were");

// ---- errors ----

const kept = text("title");
enter("f-subject", "c' d'");
request = await set();
const refused = answered;
check(
  refused.ok === false && status() === `Fehler: ${refused.error} (error)` &&
    text("title") === kept && !submit().disabled,
  "an error of the module shows as text, in its words; the fugue shown stays",
  status(),
);
enter("f-subject", "c' d' x4");
await set();
check(
  status().startsWith("Fehler: ") && status().includes("x4") && status().endsWith(" (error)"),
  "so does a wrong tone, which the error names",
  status(),
);
enter("f-subject", "  ");
let before = requests.length;
submit().click();
check(
  status() === "Bitte ausfüllen: Töne. (error)" && requests.length === before && idle() &&
    document.activeElement === $("f-subject"),
  "without tones the page asks for them, in their field, and sends nothing",
  status(),
);
enter("f-subject", own);
enter("f-key", "");
submit().click();
check(
  status() === "Bitte ausfüllen: Tonart. (error)" && requests.length === before &&
    document.activeElement === $("f-key"),
  "and for the key",
);
enter("f-key", "d");

// A module that broke off: the worker says so and loads a fresh one, the page goes on with it.
next = () => ({ ok: false, error: "RuntimeError: unreachable", broken: true });
await set();
check(
  status() ===
    "Das Programm ist abgebrochen und wird neu geladen: RuntimeError: unreachable (error)",
  "a module that broke off is told apart from an error of the request",
  status(),
);
await set();
check(
  /^Fuge gesetzt in /.test(status()) && workers.length === 1,
  "the next fugue is set by the same worker",
);

// A worker that failed: its request is answered, and the next one starts another worker.
next = "hold";
submit().click();
held.worker.onerror({ message: "out of memory", preventDefault() {} });
await until(idle, "the failed worker");
check(
  status() === "Das Programm ist abgebrochen und wird neu geladen: out of memory (error)" &&
    workers[0].terminated && workers.length === 1,
  "a worker that failed ends its request with the error",
  status(),
);
refuses = true;
await set();
check(
  status() ===
      "Das Programm ist abgebrochen und wird neu geladen: Error: no workers here (error)" &&
    !submit().disabled && workers.length === 1,
  "a worker that the browser does not start is an error shown, and the page stays usable",
  status(),
);
refuses = false;
await set();
check(
  /^Fuge gesetzt in /.test(status()) && workers.length === 2 && !workers[1].terminated,
  "the next fugue starts a fresh worker",
  `${status()} ${workers.length} workers`,
);

// ---- a cancel ----

next = "hold";
submit().click();
$("cancel").click();
await until(idle, "the cancel");
check(
  status() === "Abgebrochen." && workers[1].terminated && workers.length === 3 &&
    $("cancel").hidden && !submit().disabled && text("elapsed") === "",
  "„Abbrechen“ terminates the worker at once and starts a fresh one",
  `${status()} ${workers.length} workers`,
);
request = await set();
check(/^Fuge gesetzt in /.test(status()) && workers.length === 3, "which sets the next fugue");

// ---- English ----

const legal = () =>
  document.querySelectorAll("footer .legal a")
    .map((a) => `${a.textContent} ${a.getAttribute("href")}`).join(", ");
const pages = (contact) =>
  "timo-keller.de https://timo-keller.de/, " +
  `${contact} https://timo-keller.de/contact.html, ` +
  "Impressum https://timo-keller.de/contact.html#impressum, " +
  "Datenschutzerklärung https://timo-keller.de/datenschutzerklaerung.html";
check(legal() === pages("Kontakt"), "the footer links the site's pages", legal());
$("tempo").value = "100";
before = requests.length;
enter("lang", "en");
check(
  status() === "Composing the fugue …" && labels()[2] === "Key" &&
    submit().textContent === "Compose the fugue" && document.documentElement.lang === "en" &&
    $("task").getAttribute("aria-label") === "Subject and search" &&
    $("f-theme").options.at(-1).textContent === "Own tones …" && $("f-theme").value === "" &&
    $("f-beam").options[0].textContent === "fast" && $("f-beam").value === "200",
  "English: the form and the status change at once, the choices stay",
  `${status()} ${labels().join(", ")}`,
);
await until(idle, "the fugue in English");
check(
  requests.length === before + 1 && same(requests.at(-1), { ...request, lang: "en" }),
  "and the fugue shown is set again in English, with its seed",
  JSON.stringify(requests.at(-1)),
);
const again = answered;
const english = {
  hits: `${again.hits} of ${count} measures lie inside Bach's bands. The yardstick is ` +
    `${again.yardstick}: ${again.hits >= again.yardstick ? "met" : "not met"}.`,
  numbers: again.measures.map((measure) => written(measure, true)),
};
check(
  /^Fugue composed in \d+\.\d s\.$/.test(status()) && text("title") === again.title &&
    same(sections(), listed(again, "Bar")) && again.sections[1].name !== art.sections[1].name &&
    text("summary").includes(` ${again.bars} bars, `) &&
    same(numbers(), english.numbers) &&
    same(rows().map((one) => cells(one)[0]), again.measures.map((measure) => measure.name)) &&
    text("hits") === english.hits && $("tempo").value === "100" &&
    text("countersubject").startsWith("Countersubject: ") &&
    text("ending") === `Ending: ${{ pedal: "pedal point", stretto: "stretto" }[again.ending]}.` &&
    text("entries") ===
      `${again.entries.found} of ${again.entries.planned} entries found again.` &&
    text("check") === again.check.summary && /prohibition/.test(again.check.summary) &&
    text("summary").includes(", 3 voices, ") &&
    $("f-ending").options[1].textContent === "pedal point" &&
    buttons().at(-1) === "Chords of the scaffold" &&
    $("f-plan").options[0].textContent === "by the program" &&
    $("report-body").querySelectorAll("h2").map((one) => one.textContent).join() ===
      "Answer,Sections,Check,Style measures",
  "the report in English, the measures as the module names them; the tempo chosen stays",
  `${status()}; ${sections()[1]}; ${cells(rows()[0]).join(" | ")}; ${text("hits")}`,
);
check(legal() === pages("Contact"), "the footer keeps the site's pages in English", legal());
// The versions in English: the aspects by their English names, the version shown set again in
// the page's language when the language changes.
enter("vary-bass", true);
sent = await setVersions();
check(
  same(
    $("axes").querySelectorAll("label").map((label) => label.textContent.trim()),
    ["Free voices", "Sound", "Bass"],
  ) && $("run-versions").textContent === "Compose the versions" &&
    sent.length === 2 && sent[1].variant === "Version b: Bass: planned" &&
    text("versions-differ") === "The versions differ in: Bass. Everything else is the same." &&
    same(tabs(), ["aBass: free", "bBass: planned"]) &&
    text("summary").endsWith(", Version a: Bass: free"),
  "the versions in English",
  `${JSON.stringify(tabs())} ${text("summary")}`,
);
enter("lang", "de");
await until(() => idle() && /^Fuge gesetzt in /.test(status()), "version a in German");
check(
  requests.at(-1).variant === "Fassung a: Bass: frei" && requests.at(-1).lang === "de" &&
    same(tabs(), ["aBass: frei", "bBass: geplant"]) &&
    text("summary").endsWith(", Fassung a: Bass: frei") && !$("versions").hidden,
  "another language keeps the versions and sets the one shown again in it",
  `${JSON.stringify(tabs())} ${text("summary")}`,
);
$("version-b").click();
await until(
  () => idle() && requests.at(-1).variant === "Fassung b: Bass: geplant",
  "version b in German",
);
check(
  text("summary").endsWith(", Fassung b: Bass: geplant") && requests.at(-1).lang === "de",
  "a version set in the other language is set again when it is shown",
  text("summary"),
);
enter("vary-bass", false);
enter("lang", "en");
await until(() => idle() && /^Fugue composed in /.test(status()), "English again");
await set();
enter("lang", "de");
await until(() => idle() && /^Fuge gesetzt in /.test(status()), "the fugue in German again");
check(
  same(sections(), listed(answered, "Takt")) && requests.at(-1).lang === "de" &&
    legal() === pages("Kontakt") && labels()[2] === "Tonart",
  "and back in German",
  sections()[1],
);

// ---- an answer with unknown fields, and without known ones ----

next = () => ({
  ok: true,
  title: "Fuge von morgen",
  codas: 5,
  stretto: { bars: [12, 30] },
  ending: "coda",
  stretto_bars: 1,
  sections: [{ name: "Engführung", bar: 12, weight: 3 }],
  measures: [
    { name: "Einsätze je Takt", value: 2.5, low: 1, high: 2, inside: false, unit: "?" },
    { name: "Neuer Anteil", value: 0.25, low: 0.5, high: 0.75, inside: false },
    { name: "Ohne Band", value: 0.5 },
  ],
  files: { musicxml: "<score-partwise/>", pdf: "JVBERi0=" },
});
await set();
check(
  text("title") === "Fuge von morgen" && text("summary") === "Seed 1" &&
    same(sections(), ["Takt 12: Engführung"]) && !$("sections").querySelector("button") &&
    same(rows().map(cells), [
      ["Einsätze je Takt", "2,50", "1,00–2,00", "▲"],
      ["Neuer Anteil", "25,0 %", "50,0–75,0 %", "▼"],
      ["Ohne Band", "50,0 %", "", ""],
    ]) && text("hits") === "" && $("answer").hidden && $("countersubject").hidden &&
    text("ending") === "Schluss: coda im Abstand von 1 Takt." && $("entries").hidden &&
    $("check-box").hidden &&
    $("notation").innerHTML === "<svg><score-partwise/></svg>" && $("play").disabled &&
    !$("save-musicxml").disabled && $("save-midi").disabled && $("save-lilypond").disabled &&
    $("save-chords").disabled,
  "an answer of a later module: what the page knows shows, the rest is left out",
  `${text("summary")}; ${JSON.stringify(rows().map(cells))}; ${text("hits")}`,
);
// A fugue with troubles: entries not found again, prohibitions, and who answers for them.
next = () => ({
  ...art,
  ending: "stretto",
  stretto_bars: 0.5,
  entries: { planned: 7, found: 5 },
  counts: { own: 2, forced: 1, excused: 3, tries: [1, 2] },
  check: { forbidden: 3, cost: 9, preferences: 2, summary: "3 Verbot(e), Kosten 9." },
});
await set();
check(
  text("ending") === "Schluss: Engführung im Abstand von 0,5 Takten." &&
    text("entries") === "5 von 7 Einsätzen wiedergefunden." &&
    $("entries").classList.contains("warn") && !$("check-box").hidden &&
    text("check") === "3 Verbot(e), Kosten 9." && $("check").classList.contains("warn") &&
    same(counts(), [
      "Verbote, die das vorgegebene Material selbst bricht (Einsätze, Kontrasubjekte): 2.",
      "Verbote an Nahtstellen, wo eine freie Stimme nicht in den Einsatz führt: 1.",
    ]),
  "a fugue with troubles: missing entries and prohibitions are marked, with who answers for them",
  `${text("ending")} ${text("entries")} ${text("check")} ${counts().join(" ")}`,
);
toolkit.fails = true;
next = () => ({ ...art, files: { musicxml: "<broken" } });
await set();
check(
  text("notation") === "Verovio hat die Noten nicht gesetzt. (loadData)" && !$("play").disabled &&
    !$("save-musicxml").disabled,
  "notes that Verovio does not take: a line says so, the rest of the fugue is there",
  text("notation"),
);
toolkit.fails = false;

// ---- a page in English from the start, on a narrow screen, without Verovio ----

define("verovioReady", Promise.resolve(null));
define("matchMedia", (query) => ({ matches: query === "(max-width: 720px)" }));
document = open("en-GB");
requests.length = 0;
await import(`${new URL("app.js", import.meta.url).href}?english`);
await until(() => text("status") === "Ready.", "the page in English");
check(
  requests[0].lang === "en" && $("lang").value === "en" && labels()[1] === "Tones" &&
    text("notation") === "Pick a subject or enter tones of your own, then “Compose the fugue”." &&
    document.querySelector("footer .credits").innerHTML.startsWith("The program composes") &&
    $("help").open === false,
  "a browser in another language gets the page in English; a narrow one, the help closed",
  `${labels().join(", ")}; ${text("notation")}`,
);
enter("new-seed", false);
enter("f-theme", "bwv1080");
enter("f-ending", "stretto");
await set();
check(
  status() === `Error: ${answered.error} (error)` && answered.error.includes("stretto") &&
    text("notation") === "Pick a subject or enter tones of your own, then “Compose the fugue”.",
  "a first fugue that fails leaves the hint in place of the notes",
  `${status()}; ${text("notation")}`,
);
enter("f-ending", "auto");
request = await set();
check(
  same(request, {
    lang: "en",
    command: "fugue",
    theme: "bwv1080",
    voices: 3,
    ending: "auto",
    free_voices: "figures",
    profile: "fuge",
    bass: "free",
    beam: 400,
    seed: 1,
  }) &&
    /^Fugue composed in /.test(status()) && rows().length === count &&
    !$("save-musicxml").disabled &&
    text("notation") ===
      "No notation: Verovio is not loaded. The files can be downloaded all the same.",
  "without Verovio the fugue comes with its report and its files, and a line in place of the notes",
  `${status()}; ${text("notation")}`,
);

if (failures > 0) {
  console.log(`${failures} failure(s)`);
  process.exit(1);
}
// The page's timers would wait on (the notation): the test is done.
process.exit(0);
