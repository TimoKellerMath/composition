// Test of the page without a browser: web/index.html and web/app.js as they are, on the small DOM
// of page-dom.mjs, with workers that answer from the WebAssembly module, except the parts of a
// mass and their joining, which the test answers itself, so that it decides when and how each
// part ends, and with a Verovio that engraves what it loads as it is:
//   node web/page-test.mjs target/web/fux_web.wasm
//   deno run --allow-read web/page-test.mjs target/web/fux_web.wasm
// It checks the buttons of a mass while it is set and after a cancel, that „Plan zeigen“ and
// Enter in a field keep the mass shown, that the clefs and brackets of the toolbar reach the
// results of every tab, that a part whose worker broke off is set again, one whose search
// failed not, that the pool starts the longest parts of a whole mass first and then its
// movements one by one, that the list of parts shows a part as soon as it is set and its
// movement once all its parts are, and that the footer links the site's pages in both languages.
import { readFileSync } from "node:fs";
import process from "node:process";
import { Fux } from "./fux.js";
import { makeDocument, Option } from "./page-dom.mjs";

const path = process.argv[2] ?? "target/web/fux_web.wasm";
const fux = await Fux.load(readFileSync(path));
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

const document = makeDocument(readFileSync(new URL("index.html", import.meta.url), "utf8"));
define("document", document);
define("Option", Option);
define("navigator", { hardwareConcurrency: 8, deviceMemory: 8, language: "de-DE" });
const storage = new Map();
define("localStorage", {
  getItem: (key) => storage.get(key) ?? null,
  setItem: (key, value) => storage.set(key, String(value)),
  removeItem: (key) => storage.delete(key),
});
define("confirm", () => true);
if (typeof globalThis.addEventListener !== "function") define("addEventListener", () => {});
// Verovio's toolkit as the page uses it: a page that holds what it loaded.
define(
  "verovioReady",
  Promise.resolve({
    setOptions() {},
    loadData(data) {
      this.data = data;
    },
    getPageCount: () => 1,
    renderToSVG() {
      return `<svg>${this.data}</svg>`;
    },
  }),
);

/** Every request the page sent to a worker, in order. */
const requests = [];
/** The parts of a mass that workers of the pool are setting, by address, till the test ends
 * them. */
const setting = new Map();

/** A worker of the page: the module answers at once, after a timer, as a worker answers; the
 * parts of a mass wait for the test (`end`), and mass_join answers as `joined` does. */
class PageWorker {
  postMessage({ id, request }) {
    if (this.terminated) return;
    requests.push(request);
    if (request.command === "mass_part") {
      setting.set(request.part, { worker: this, id, request });
      return;
    }
    const answer = request.command === "mass_join" ? joined(request) : fux.ask(request);
    setTimeout(() => this.answer(id, answer), 0);
  }
  answer(id, answer) {
    if (!this.terminated) this.onmessage?.({ data: { id, answer } });
  }
  terminate() {
    this.terminated = true;
    for (const [address, part] of setting) {
      if (part.worker === this) setting.delete(address);
    }
  }
}
define("Worker", PageWorker);

/** mass_join as the test answers it: a result per movement whose parts are all set, named as
 * mass_join names it, its files and the book telling what they were made with. */
function joined(request) {
  const look = `${request.mass.title} clefs=${request.original_clefs} marks=${request.marks}`;
  const plan = fux.ask({ command: "mass_plan", mass: request.mass }).mass;
  const set = new Set(request.parts.filter((part) => part.musicxml).map((part) => part.part));
  const done = (m) => plan.parts.every((part) => part.movement !== m || set.has(part.address));
  return {
    ok: true,
    summary: [`joined: ${look}`],
    results: plan.movements.filter((_, m) => done(m)).map((movement) => ({
      label: movement.title,
      file: movement.file,
      musicxml: look,
      lilypond: "",
      midi: "",
      lines: [],
      notes: [],
      violations: [],
      sections: [],
    })),
    book: look,
  };
}

/** Ends the part `address` that a worker is setting: set (`done`, Kyrie I with `motif`), failed
 * (its search found nothing), or its worker breaks off (`trap` of its module, or `error`). */
function end(address, how, motif = undefined) {
  const { worker, id } = setting.get(address);
  setting.delete(address);
  if (how === "error") {
    worker.onerror?.({ message: "out of memory", preventDefault() {} });
    return;
  }
  const part = {
    address,
    musicxml: `<part ${address}/>`,
    measures: 8,
    cost: 2,
    forbidden: 0,
    seed: "1",
    attempt: 1,
    warnings: [],
    violations: [],
    ...(motif ? { motif } : {}),
  };
  worker.answer(
    id,
    {
      done: { ok: true, summary: [`Bericht ${address}`], part },
      failed: { ok: false, error: `Teil ${address}: kein Satz nach 3 Versuchen.` },
      trap: { ok: false, error: "RuntimeError: unreachable", broken: true },
    }[how],
  );
}

/** Lets the page take the answers its workers give after a timer. */
async function settle() {
  for (let i = 0; i < 5; i++) await new Promise((resolve) => setTimeout(resolve, 10));
}

/** Waits until `condition` holds, at most ten seconds. */
async function until(condition, what) {
  for (let i = 0; i < 1000 && !condition(); i++) {
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  if (!condition()) check(false, `waited for ${what}`);
}

/** A key pressed in `field`: Enter in a field of a form submits it, by its first submit button
 * where that is not disabled, unless a listener prevents it (implicit submission). */
function press(field, key) {
  const event = field.fire("keydown", { key });
  if (key === "Enter" && !event.defaultPrevented && field.localName === "input") {
    field.form?.querySelector("button[type=submit]")?.click();
  }
  return event;
}

// ---- the page ----

await import(new URL("app.js", import.meta.url).href);
const $ = (id) => document.getElementById(id);
await until(() => $("results").children.length > 0, "the first counterpoint");
check(
  /^\d+ Ergebnis\(se\)\./.test($("status").textContent),
  "the page starts with a counterpoint",
  $("status").textContent,
);

const tab = (name) => document.querySelector(`#tabs button[data-tab=${name}]`).click();
const change = (id, value) => {
  const field = $(id);
  if (typeof value === "boolean") field.checked = value;
  else field.value = value;
  field.fire("change");
};
const submit = $("form-mass").querySelector("button.primary");
const asked = (command, from = 0) => requests.slice(from).filter((r) => r.command === command);
const joins = () => asked("mass_join");
/** The item of the part `address` in the list of the mass shown. */
const item = (address) => {
  const plan = fux.ask(asked("mass_plan").at(-1)).mass;
  return $("mass-list").children[plan.parts.findIndex((p) => p.address === address)];
};
/** The line of the part `address` in the list of the mass shown: its state and what it says. */
const line = (address) =>
  `${item(address)?.className}: ${item(address)?.querySelector(".state").textContent}`;
const buttons = () =>
  JSON.stringify({
    cancel: !$("mass-cancel").disabled,
    resume: !$("mass-resume").hidden,
    plan: !$("mass-plan").disabled,
    set: !submit.disabled,
  });

// ---- a mass while it is set: „Abbrechen“ at once, „Fehlende Teile setzen“ after a cancel ----

tab("mass");
change("mass-new-seed", false);
change("mass-seed", "1");
for (const id of ["credo", "sanctus", "benedictus", "agnus"]) change(`mass-${id}`, false);
await settle();
submit.click();
await until(() => setting.size === 4, "four parts set at once");
check(
  buttons() === JSON.stringify({ cancel: true, resume: false, plan: false, set: false }),
  "while the parts are set: „Abbrechen“ works from the start, the other buttons wait",
  buttons(),
);
// A movement whose parts are all set is put together while the others are set.
while (joins().length === 0 && setting.size > 0) {
  end(setting.keys().next().value, "done");
  await settle();
}
check(
  joins()[0]?.running === true && setting.size > 0,
  "a movement put together while parts are set: the parts missing go without saying",
);
const stopped = [...setting.keys()];
check($("mass-cancel").click(), "„Abbrechen“ can be clicked while parts are set");
await settle();
check(
  buttons() === JSON.stringify({ cancel: false, resume: true, plan: true, set: true }) &&
    stopped.every((address) => line(address) === "stopped: abgebrochen") &&
    joins().at(-1).running === false,
  "a cancel stops the parts being set and offers to set them",
  `${buttons()} ${stopped.map(line)}`,
);
$("mass-resume").click();
check(
  buttons() === JSON.stringify({ cancel: true, resume: false, plan: false, set: false }),
  "„Fehlende Teile setzen“: „Abbrechen“ works at once again",
  buttons(),
);
while (setting.size > 0) {
  end(setting.keys().next().value, "done");
  await settle();
}
await settle();
check(
  $("mass-status").textContent.startsWith("Fertig: 9 von 9 Teilen") &&
    $("results").children.length === 2 && !$("save-book").disabled,
  "the mass set: Kyrie and Gloria",
  $("mass-status").textContent,
);

// ---- Enter in a field and „Plan zeigen“ keep the mass shown ----

let from = requests.length;
$("mass-title").value = "Missa prima";
const enter = press($("mass-title"), "Enter");
await settle();
check(
  enter.defaultPrevented && asked("mass_part", from).length === 0 &&
    joins().at(-1).mass.title === "Missa prima" &&
    $("mass-status").textContent.startsWith("Fertig: 9 von 9 Teilen"),
  "Enter in the title puts the mass shown together with it; it sets no new mass",
  `${asked("mass_part", from).length} parts asked; ${$("mass-status").textContent}`,
);
$("mass-plan").click();
await settle();
check(
  $("mass-status").textContent.startsWith("Plan der Messe, ohne Suche: 9 Teile.") &&
    !$("mass-result").hidden && $("results").children.length === 0,
  "„Plan zeigen“ shows the plan in place of the mass, with „Ergebnis zeigen“",
  $("mass-status").textContent,
);
$("mass-plan").click();
await settle();
check(!$("mass-result").hidden, "a plan shown again keeps the mass too");
$("mass-result").click();
await settle();
check(
  $("mass-status").textContent.startsWith("Fertig: 9 von 9 Teilen") && $("mass-result").hidden &&
    $("results").children.length === 2 && !$("save-book").disabled &&
    $("summary").textContent === "joined: Missa prima clefs=false marks=true",
  "„Ergebnis zeigen“ shows the mass set again, with all its parts",
  `${$("mass-status").textContent} ${$("summary").textContent}`,
);

// ---- the clefs and brackets of the toolbar reach the results of every tab ----

from = requests.length;
tab("cp");
await settle();
check(
  asked("cp", from).length === 0 && asked("mass_join", from).length === 0,
  "the tab changes without setting anything again where nothing changed",
);
change("original-clefs", true);
await until(() => asked("cp", from).length === 1, "the counterpoint set again");
await settle();
check(asked("cp", from)[0].original_clefs === true, "Fux' clefs on „Kontrapunkt“: set again");
check(joins().at(-1).original_clefs === false, "the mass hidden waits");
tab("mass");
await settle();
check(
  joins().at(-1).original_clefs === true &&
    $("summary").textContent === "joined: Missa prima clefs=true marks=true",
  "back on „Messe“, the mass is put together with Fux' clefs",
  $("summary").textContent,
);
change("entry-marks", false);
await settle();
check(
  asked("cp", from).length === 1 && joins().at(-1).marks === false &&
    $("summary").textContent === "joined: Missa prima clefs=true marks=false",
  "no brackets on „Messe“: the mass is put together again, the counterpoint waits",
  $("summary").textContent,
);
tab("cp");
await until(() => asked("cp", from).length === 2, "the counterpoint set again");
await settle();
check(
  asked("cp", from)[1].marks === false && asked("cp", from)[1].original_clefs === true,
  "back on „Kontrapunkt“, the counterpoint is set again without brackets",
);
change("original-clefs", false);
change("entry-marks", true);
await settle();

// ---- a worker that breaks off: its part is set again, one whose search failed not ----

tab("mass");
change("mass-motif", true);
await settle();
from = requests.length;
submit.click();
await until(() => setting.has("kyrie/1"), "Kyrie I first, which gives the head motif");
end("kyrie/1", "error");
await settle();
while (setting.size > 0) {
  const address = setting.keys().next().value;
  end(address, { "kyrie/2": "trap", "kyrie/3": "failed" }[address] ?? "done");
  await settle();
}
await settle();
const last = joins().at(-1);
check(
  line("kyrie/1") === "broken: Worker abgestürzt: out of memory" &&
    line("kyrie/2") === "broken: Worker abgestürzt: RuntimeError: unreachable" &&
    line("kyrie/3") === "failed: gescheitert: Teil kyrie/3: kein Satz nach 3 Versuchen." &&
    line("gloria/1") === "waiting: nicht gesetzt" &&
    !asked("mass_part", from).some((r) => r.part === "gloria/1") &&
    $("mass-status").textContent === "Unvollständig: 5 von 9 Teilen gesetzt." &&
    !$("mass-resume").hidden,
  "a worker that breaks off is no failed search; Gloria I waits for the motif of Kyrie I",
  `${["kyrie/1", "kyrie/2", "kyrie/3", "gloria/1"].map(line)} ${$("mass-status").textContent}`,
);
check(
  last.running === false &&
    JSON.stringify(last.parts.filter((part) => !part.musicxml)) ===
      JSON.stringify([{ part: "kyrie/3", error: "Teil kyrie/3: kein Satz nach 3 Versuchen." }]),
  "the mass put together: the failed part with its error, the others as not set",
  JSON.stringify(last.parts.filter((part) => !part.musicxml)),
);
from = requests.length;
$("mass-resume").click();
await settle();
check(
  [...setting.keys()].sort().join() === "kyrie/1,kyrie/2",
  "„Fehlende Teile setzen“ sets the parts whose worker broke off, not the failed one",
  [...setting.keys()].join(),
);
end("kyrie/1", "done", "0,1,2,1,0");
await settle();
check(
  setting.get("gloria/1")?.request.motif === "0,1,2,1,0",
  "Gloria I opens with the motif of Kyrie I set again",
);
while (setting.size > 0) {
  end(setting.keys().next().value, "done");
  await settle();
}
await settle();
check(
  $("mass-status").textContent === "Unvollständig: 8 von 9 Teilen gesetzt." &&
    $("mass-resume").hidden,
  "only the failed part is missing then, which „Fehlende Teile setzen“ leaves",
  $("mass-status").textContent,
);

// ---- the order of the pool: the longest parts first, then whole movements ----

change("mass-motif", false);
for (const id of ["credo", "sanctus", "benedictus", "agnus"]) change(`mass-${id}`, true);
await settle();
submit.click();
await until(() => setting.size === 4, "four parts of the whole mass set at once");
const firstFour = [...setting.keys()].sort().join();
end("agnus/1", "done");
await settle();
check(
  firstFour === "agnus/1,agnus/2,kyrie/1,sanctus/1" && setting.has("kyrie/3") &&
    / in Arbeit . noch etwa \d+:\d\d$/.test($("mass-status").textContent),
  "a whole mass: Sanctus and Agnus Dei, the longest, beside Kyrie I, then the rest of the Kyrie",
  `${firstFour}, then ${[...setting.keys()].join()}; ${$("mass-status").textContent}`,
);

// ---- a part shows as soon as it is set, its movement once all its parts are ----

/** What the results show: the note over the notation, the movement selected, the notation. */
const shown = () =>
  JSON.stringify({
    note: $("part-alone").hidden ? null : $("part-alone").textContent,
    selected: [...$("results").children]
      .filter((button) => button.getAttribute("aria-selected") === "true")
      .map((button) => button.textContent).join(),
    notation: $("notation").innerHTML,
  });
const show = (address) => item(address).querySelector(".line").click();
check(!show("kyrie/3"), "a part being set does not show yet");
end("kyrie/1", "done");
await settle();
show("kyrie/1");
await settle();
const alone = JSON.stringify({
  note: "Kyrie I (kyrie/1) für sich, wie gesetzt: moderne Schlüssel und Klammern über den " +
    "Einsätzen, Takte ab 1. Den Satz zeigt die Seite, sobald alle seine Teile gesetzt sind.",
  selected: "",
  notation: "<svg><part kyrie/1/></svg>",
});
check(
  shown() === alone && item("kyrie/1").getAttribute("aria-current") === "true" &&
    $("play").disabled && $("save-musicxml").disabled &&
    $("violations").textContent === "Keine Verstöße.",
  "a part set while its movement is not: the part on its own, with its violations, nothing to play",
  shown(),
);
change("show-text", true);
await settle();
check($("text").textContent === "Bericht kyrie/1", "as text the report of its search");
change("show-text", false);
change("lang", "en");
await settle();
check(
  $("part-alone").textContent.startsWith("Kyrie I (kyrie/1) on its own, as set: modern clefs"),
  "the note in English",
  $("part-alone").textContent,
);
change("lang", "de");
await settle();
end("agnus/2", "done");
await settle();
check(
  shown() === alone && $("results").children.length === 1,
  "another movement put together: the part stays",
  shown(),
);
end("kyrie/3", "done");
end("kyrie/2", "done");
await settle();
const look = "<svg>Missa prima clefs=false marks=true</svg>";
check(
  shown() === JSON.stringify({ note: null, selected: "Kyrie", notation: look }) &&
    item("kyrie/1").getAttribute("aria-current") === "false" && !$("play").disabled,
  "its movement put together: the movement takes its place",
  shown(),
);
show("agnus/1");
await settle();
check(
  shown() === JSON.stringify({ note: null, selected: "Agnus Dei", notation: look }),
  "a part of a movement put together shows the movement",
  shown(),
);
$("mass-cancel").click();
await settle();

// ---- the footer: the site's start, contact, Impressum and Datenschutzerklärung ----

const legal = () =>
  [...document.querySelectorAll("footer .legal a")]
    .map((a) => `${a.textContent} ${a.getAttribute("href")}`).join(", ");
const pages = (contact) =>
  "timo-keller.de https://timo-keller.de/, " +
  `${contact} https://timo-keller.de/contact.html, ` +
  "Impressum https://timo-keller.de/contact.html#impressum, " +
  "Datenschutzerklärung https://timo-keller.de/datenschutzerklaerung.html";
check(legal() === pages("Kontakt"), "the footer links the site's pages", legal());
change("lang", "en");
await settle();
check(legal() === pages("Contact"), "and keeps them in English", legal());
change("lang", "de");
await settle();
check(legal() === pages("Kontakt"), "and back in German", legal());

if (failures > 0) {
  console.log(`${failures} failure(s)`);
  process.exit(1);
}
// The page's timers would wait on (the notation, the preview): the test is done.
process.exit(0);
