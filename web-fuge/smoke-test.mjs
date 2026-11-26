// Smoke test of the WebAssembly module of fux-fuge, after tools/build_web_fuge.sh:
//   node web-fuge/smoke-test.mjs target/web-fuge/fuge_web.wasm
//   deno run --allow-read web-fuge/smoke-test.mjs target/web-fuge/fuge_web.wasm
// It loads the module as the page does (Fuge.fetch: the gzip copy beside it, else the module
// itself), asks for the subjects and the plans, for fugues on a subject of the module (in three
// and in four voices, after a plan, with either ending, with an answer of one's own) and for one
// on typed tones, and for answers to wrong requests. Verovio, taken from the build beside the
// module, engraves the MusicXML of three fugues. worker.js answers here with a fetch of the
// test's own, also after a module that traps; where the runtime has web workers and its fetch
// reads files (Deno), the worker.js of the build answers as a real worker.
import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import process from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";
import { Fuge } from "./fuge.js";

const path = process.argv[2] ?? "target/web-fuge/fuge_web.wasm";
const wasm = readFileSync(path);
const module = new WebAssembly.Module(wasm);
let failures = 0;

function check(condition, what, detail = "") {
  if (condition) {
    console.log(`ok    ${what}`);
  } else {
    failures += 1;
    console.log(`FAIL  ${what} ${detail}`);
  }
}

const imports = WebAssembly.Module.imports(module);
const exports = WebAssembly.Module.exports(module).map((entry) => entry.name).sort();
check(imports.length === 0, "the module needs no imports", JSON.stringify(imports));
check(
  exports.join() === "fuge_input,fuge_output,fuge_run,memory",
  "it exports its memory and the three functions of the buffers",
  exports.join(),
);
check(
  WebAssembly.Module.customSections(module, "name").length === 0,
  "the module has no name section (the profile wasm strips it)",
);

// ---- the module as the page loads it ----

/** `bytes` through a stream that compresses or decompresses them. */
async function through(bytes, transform) {
  const stream = new Response(bytes).body.pipeThrough(transform);
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** Puts `value` in the place of the global `name`; returns what puts the old one back. */
function replace(name, value) {
  const old = Object.getOwnPropertyDescriptor(globalThis, name);
  Object.defineProperty(globalThis, name, { value, configurable: true, writable: true });
  return () => old ? Object.defineProperty(globalThis, name, old) : delete globalThis[name];
}

/** A fetch that answers with `files` by URL, and with 404 for any other. */
function serving(files) {
  return async (url) =>
    `${url}` in files
      ? new Response(files[`${url}`])
      : new Response("", { status: 404, statusText: "Not Found" });
}

/**
 * Fuge.fetch(url) with every fetch recorded and answered by `respond`, the runtime's fetch unless
 * given, and without DecompressionStream if `decompress` is false: the URLs fetched, and whether
 * the module answers, or the error.
 */
async function load(url, { respond = globalThis.fetch, decompress = true } = {}) {
  const urls = [];
  const restore = [replace("fetch", (input, init) => {
    urls.push(`${input}`);
    return respond(input, init);
  })];
  if (!decompress) restore.push(replace("DecompressionStream", undefined));
  try {
    const loaded = await Fuge.fetch(url);
    return { urls, answers: loaded.ask({ command: "version" }).ok };
  } catch (error) {
    return { urls, error: `${error}` };
  } finally {
    restore.forEach((put) => put());
  }
}

// tools/build_web_fuge.sh writes the copy beside the module. It must hold this very module, or
// the page would answer with another one.
const copy = existsSync(`${path}.gz`) ? readFileSync(`${path}.gz`) : null;
check(
  copy?.[0] === 0x1f && copy[1] === 0x8b &&
    wasm.equals(await through(copy, new DecompressionStream("gzip"))),
  `the module in gzip beside it (${copy?.length} instead of ${wasm.length} bytes)`,
  copy ? "" : `(no ${path}.gz)`,
);
const gzip = copy ?? await through(wasm, new CompressionStream("gzip"));
const [plain, packed] = ["fuge/fuge_web.wasm", "fuge/fuge_web.wasm.gz"];
const loads = [
  ["the copy", { [packed]: gzip }, [packed]],
  ["the copy as a server decompressed it on the way", { [packed]: wasm }, [packed]],
  ["without the copy the module", { [plain]: wasm }, [packed, plain]],
  ["for a broken copy the module", {
    [packed]: gzip.subarray(0, gzip.length >> 1),
    [plain]: wasm,
  }, [packed, plain]],
  ["for a page in place of the copy the module", { [packed]: "<!doctype html>", [plain]: wasm }, [
    packed,
    plain,
  ]],
];
for (const [what, files, fetched] of loads) {
  const loaded = await load(plain, { respond: serving(files) });
  check(
    loaded.answers && loaded.urls.join() === fetched.join(),
    `Fuge.fetch: ${what}`,
    JSON.stringify(loaded),
  );
}
const without = await load(plain, {
  respond: serving({ [packed]: gzip, [plain]: wasm }),
  decompress: false,
});
check(
  without.answers && without.urls.join() === plain,
  "Fuge.fetch: without DecompressionStream the module alone",
  JSON.stringify(without),
);
const neither = await load(plain, { respond: serving({}) });
check(
  neither.error === `Error: ${plain}: 404 Not Found` &&
    neither.urls.join() === `${packed},${plain}`,
  "Fuge.fetch: without either file an error that names the module",
  JSON.stringify(neither),
);
// From the build's files, where fetch reads them (Deno), as the real worker below loads it.
const readsFiles = await fetch(import.meta.url).then((got) => got.arrayBuffer(), () => null);
if (readsFiles) {
  const loaded = await load(pathToFileURL(path));
  check(
    loaded.answers && loaded.urls.join() === `${pathToFileURL(path)}.gz`,
    "Fuge.fetch from the files: the copy",
    JSON.stringify(loaded),
  );
} else {
  console.log("skip  Fuge.fetch from the files: this runtime's fetch reads none");
}

// ---- the subjects ----

const fuge = await Fuge.load(wasm);
/** The answer to `request` and the seconds it took: the module has no clock. */
function timed(request) {
  const started = performance.now();
  const answer = fuge.ask(request);
  return [answer, ((performance.now() - started) / 1000).toFixed(1)];
}

const version = fuge.ask({ command: "version" });
check(version.ok && /^\d+\.\d+\.\d+/.test(version.version), "version", JSON.stringify(version));
const themes = fuge.ask({ command: "themes" });
const texts = ["name", "title", "key", "meter", "subject"];
check(
  themes.ok && themes.themes.length >= 20 &&
    themes.themes.every((theme) =>
      texts.every((field) => typeof theme[field] === "string" && theme[field]) &&
      theme.period_bars > 0
    ) && new Set(themes.themes.map((theme) => theme.name)).size === themes.themes.length,
  `themes: ${themes.themes?.length} subjects with what the form shows of them`,
  JSON.stringify(themes).slice(0, 300),
);
check(
  themes.plans?.length > 0 && themes.plans.every((plan) => typeof plan === "string" && plan) &&
    new Set(themes.plans).size === themes.plans.length && themes.plans.includes("wtk1-02"),
  `themes: ${themes.plans?.length} plans by their names`,
  JSON.stringify(themes.plans),
);
const art = themes.themes?.find((theme) => theme.name === "bwv1080");
check(
  art?.key === "d" && art.meter === "2/2" && art.period_bars === 4 &&
    art.subject === "d'2 a' | f' d' | cis' d'4 e' | f'2~ f'8 g' f' e' | d'4",
  "among them the subject of the Art of Fugue",
  JSON.stringify(art),
);

// ---- a fugue on a subject of the module ----

/** What the page shows of a fugue, and what it plays and saves. */
function whole(answer) {
  const notes = answer.playback?.notes ?? [];
  const ticks = (answer.sections ?? []).map((section) => section.tick);
  const end = Math.max(...notes.map((note) => note[2]));
  return answer.ok && answer.bars > 0 && answer.tones === notes.length &&
    answer.voices === new Set(notes.map((note) => note[0])).size &&
    (answer.ending === "stretto"
      ? answer.stretto_bars > 0
      : answer.ending === "pedal" && answer.stretto_bars === null) &&
    answer.entries.planned >= answer.voices && answer.entries.found <= answer.entries.planned &&
    Number.isInteger(answer.entries.found) && typeof answer.check.summary === "string" &&
    [answer.check.forbidden, answer.check.cost, answer.check.preferences].every(Number.isInteger) &&
    answer.check.summary.startsWith(`${answer.check.forbidden} `) &&
    [answer.counts.own, answer.counts.forced, answer.counts.excused].every(Number.isInteger) &&
    answer.fixed_tones > 0 && answer.fixed_tones < answer.tones &&
    notes.every(([voice, on, off, key, given]) =>
      Number.isInteger(voice) && 0 <= on && on < off && 20 < key && key < 110 &&
      typeof given === "boolean"
    ) && notes.filter((note) => note[4]).length === answer.fixed_tones &&
    answer.playback.ticks_per_quarter > 0 && answer.playback.quarters_per_minute > 0 &&
    answer.sections.length >= 3 && ticks[0] === 0 && answer.sections[0].bar === 1 &&
    ticks.every((tick, i) => i === 0 || (tick > ticks[i - 1] && tick < end)) &&
    answer.sections.every((section) => section.name && section.bar <= answer.bars) &&
    ["real", "tonal"].includes(answer.answer.kind) && answer.answer.text.length > 0 &&
    answer.countersubject.length > 0 &&
    answer.measures.every((measure) =>
      measure.name && [measure.value, measure.low, measure.high].every(Number.isFinite) &&
      measure.low <= measure.high &&
      measure.inside ===
        (measure.low - 1e-9 <= measure.value && measure.value <= measure.high + 1e-9)
    ) && answer.hits === answer.measures.filter((measure) => measure.inside).length &&
    answer.yardstick > 0 && answer.files.musicxml.includes("<score-partwise") &&
    answer.files.lilypond.includes("\\version") && answer.files.midi.startsWith("TVRoZA") &&
    // The scaffold: a line of chords for every bar.
    answer.files.chords.startsWith("1: ") &&
    answer.files.chords.includes(`\n${answer.bars}: `);
}

/** The size of a fugue and what was found of its plan: what the pins below hold on to. */
const size = (answer) =>
  `${answer.voices} voices, ${answer.bars} bars, ${answer.tones} tones, ending ${answer.ending}, ` +
  `${answer.entries?.found} of ${answer.entries?.planned} entries`;
/** An answer without its long parts, for the line of a failed check. */
const brief = (answer) =>
  answer.error ?? JSON.stringify({ ...answer, files: 0, playback: 0, measures: 0 }).slice(0, 700);

// The fugue of seed 1 in three voices, as the composer of phase B6 sets it: a change of these
// numbers is a change of the composer.
const request = { command: "fugue", theme: "bwv1080", seed: 1, beam: 400, lang: "de" };
const [fugue, seconds] = timed(request);
check(
  whole(fugue) && fugue.measures.length >= 29 &&
    fugue.title.startsWith("Fuge über Kunst der Fuge") &&
    fugue.key === "d" && fugue.meter === "2/2" && fugue.sections[1].name === "Antwort" &&
    fugue.sections[1].bar === 5 &&
    size(fugue) === "3 voices, 40 bars, 452 tones, ending pedal, 6 of 6 entries" &&
    fugue.check.forbidden === 0 && /^0 Verbot/.test(fugue.check.summary) &&
    fugue.sections.some((section) => /^Einsatz \(\S+\)$/.test(section.name)) &&
    fugue.sections.at(-1).name === "Kadenz",
  `a fugue on the subject of the Art of Fugue (${seconds} s): ${size(fugue)}, ` +
    `${fugue.hits} of ${fugue.measures?.length} measures in Bach's bands`,
  brief(fugue),
);
check(fugue.seconds === 0, "the module has no clock: the page measures the time", fugue.seconds);
check(
  fuge.ask(request).files?.midi === fugue.files?.midi,
  "the same request gives the same fugue",
);
check(
  fuge.ask({ ...request, seed: 2 }).files?.midi !== fugue.files?.midi,
  "another seed gives another one",
);
// A request with the theme alone, and with fields that only a later module knows.
const bare = fuge.ask({ command: "fugue", theme: "bwv1080", not_yet: [1], later: "x" });
check(
  bare.files?.midi === fugue.files?.midi,
  "seed 1, three voices, beam 400, the ending left to the program, no plan and German are the " +
    "defaults; fields the module does not know are ignored",
  brief(bare),
);
// The free voices: with figures and sequences unless the request says plain.
const [unfigured, unfiguredSeconds] = timed({ ...request, free_voices: "plain" });
check(
  whole(unfigured) && unfigured.check.forbidden === 0 && unfigured.bars === fugue.bars &&
    unfigured.files.midi !== fugue.files.midi &&
    fuge.ask({ ...request, free_voices: "figures" }).files?.midi === fugue.files.midi,
  `the free voices plain, without figures and sequences (${unfiguredSeconds} s against ` +
    `${seconds} s with them): ${size(unfigured)}`,
  brief(unfigured),
);
// The profile: the composer's own unless the request names the calibrated one.
const counted = fuge.ask({ ...request, profile: "bach" });
check(
  whole(counted) && counted.check.forbidden === 0 && counted.bars === fugue.bars &&
    counted.files.midi !== fugue.files.midi &&
    fuge.ask({ ...request, profile: "fuge" }).files?.midi === fugue.files.midi,
  `the prices calibrated on Bach's rates instead of the composer's: ${size(counted)}`,
  brief(counted),
);
// The bass: found by the search unless the request asks for a planned one, which the chords
// of the scaffold then name.
const grounded = fuge.ask({ ...request, bass: "planned" });
check(
  whole(grounded) && grounded.check.forbidden === 0 && grounded.bars === fugue.bars &&
    grounded.files.midi !== fugue.files.midi && grounded.files.chords.includes("/") &&
    !fugue.files.chords.includes("/") &&
    fuge.ask({ ...request, bass: "free" }).files?.midi === fugue.files.midi,
  `a planned bass, named in the chords of the scaffold: ${size(grounded)}`,
  brief(grounded),
);
// A fugue named as one of several versions carries the name in its files, the tones are the
// same; and the words of the sections stand over the notes in both scores.
const named = fuge.ask({ ...request, variant: "Fassung b: Bass: frei" });
check(
  named.files?.midi === fugue.files.midi && named.variant === "Fassung b: Bass: frei" &&
    named.files.lilypond.includes('subtitle = "Fassung b: Bass: frei"') &&
    named.files.musicxml.includes(" · Fassung b: Bass: frei</work-title>") &&
    fugue.variant === "" && !fugue.files.musicxml.includes("Fassung") &&
    fugue.files.lilypond.includes('^\\markup \\italic "Antwort"') &&
    fugue.files.musicxml.includes(">Antwort</words>"),
  "a version carries its name in its files; the sections stand over the notes in both scores",
  brief(named),
);
// The answer of the rule, given back as an answer of one's own, changes nothing.
const answered = fuge.ask({ ...request, answer: fugue.answer?.text });
check(
  answered.files?.midi === fugue.files?.midi && answered.answer?.text === fugue.answer.text,
  "an answer of one's own takes the place of the rule's",
  brief(answered),
);

// ---- four voices, a plan, the endings ----

const [four, fourSeconds] = timed({ ...request, voices: 4, lang: "en" });
check(
  whole(four) && size(four) === "4 voices, 44 bars, 690 tones, ending pedal, 7 of 7 entries" &&
    four.check.forbidden === 0 &&
    /^\d+ prohibition/.test(four.check.summary) && four.sections[1].name === "Answer",
  `the same subject in four voices, in English (${fourSeconds} s): ${size(four)}`,
  brief(four),
);
const [planned, plannedSeconds] = timed({ ...request, plan: "wtk1-02" });
check(
  whole(planned) &&
    size(planned) === "3 voices, 60 bars, 728 tones, ending pedal, 9 of 9 entries" &&
    planned.check.forbidden === 0,
  `after the plan of a fugue of Bach's (${plannedSeconds} s): ${size(planned)}`,
  brief(planned),
);
// A subject that gives a stretto ends with it, unless a pedal point is asked for.
const narrow = { command: "fugue", theme: "wtk1-16", seed: 1, beam: 200 };
const [stretto, pedal] = [fuge.ask(narrow), fuge.ask({ ...narrow, ending: "pedal" })];
check(
  whole(stretto) && stretto.ending === "stretto" && stretto.stretto_bars === 0.5 &&
    fuge.ask({ ...narrow, ending: "stretto" }).files?.midi === stretto.files.midi &&
    whole(pedal) && pedal.ending === "pedal" && pedal.files.midi !== stretto.files.midi,
  `the endings: ${size(stretto)} at ${stretto.stretto_bars} bars; ${size(pedal)}`,
  `${brief(stretto)} ${brief(pedal)}`,
);
const two = fuge.ask({ ...narrow, voices: 2 });
check(whole(two) && two.voices === 2, `two voices: ${size(two)}`, brief(two));

// ---- a fugue on typed tones ----

const typed = {
  command: "fugue",
  subject: "r8 e'8 a' c'' b' e' gis' b' | e''4. d''8 c'' b'16 a' gis'4 | a'4",
  key: "a",
  meter: "4/4",
  period_bars: 2,
  title: "Probe",
  seed: 3,
  beam: 200,
  lang: "en",
};
const [own, ownSeconds] = timed(typed);
check(
  whole(own) && own.title === "Fugue on Probe" && own.key === "a" && own.meter === "4/4" &&
    own.sections[0].name === "Subject" && own.sections[1].name === "Answer" &&
    own.sections[1].bar === 3,
  `a fugue on typed tones with an upbeat, in English (${ownSeconds} s): ${size(own)}, ` +
    `${own.hits} of ${own.measures?.length} measures in Bach's bands`,
  brief(own),
);
const { period_bars: _, ...free } = typed;
check(whole(fuge.ask(free)), "the distance of the entries is optional");

// ---- answers to wrong requests ----

// What is wrong with a request, and what its error names of it; the wording is the module's.
const wrong = [
  ["no command", {}, "command"],
  ["an unknown command", { command: "sonata" }, "sonata"],
  ["neither subject nor tones", { command: "fugue" }, "subject"],
  ["an unknown subject", { command: "fugue", theme: "bwv9999" }, "bwv9999"],
  ["two tones", { command: "fugue", subject: "c' d'", key: "C" }, "three"],
  ["a wrong tone", { command: "fugue", subject: "c' d' x4", key: "C" }, "x4"],
  ["a wrong value", { command: "fugue", subject: "c'5 d' e'", key: "C" }, "c'5"],
  ["no key", { command: "fugue", subject: "c' d' e'" }, "key"],
  ["a wrong key", { command: "fugue", subject: "c' d' e'", key: "?" }, "key"],
  ["a wrong meter", { command: "fugue", subject: "c' d' e'", key: "C", meter: "4" }, "meter"],
  ["seven voices", { ...request, voices: 7 }, "7"],
  ["an unknown plan", { ...request, plan: "wtk9-99" }, "wtk9-99"],
  ["an unknown ending", { ...request, ending: "loud" }, "loud"],
  ["an unknown way of the free voices", { ...request, free_voices: "ornate" }, "ornate"],
  ["an unknown profile", { ...request, profile: "mozart" }, "mozart"],
  ["an unknown bass", { ...request, bass: "walking" }, "walking"],
  ["a stretto that the subject does not give", { ...request, ending: "stretto" }, "stretto"],
  ["an answer of three tones", { ...request, answer: "a' d'' c''" }, "3 tones"],
  ["a wrong tone in the answer", { ...request, answer: "a' d'' h'" }, "h'"],
];
for (const [what, bad, named] of wrong) {
  const answer = fuge.ask(bad);
  check(
    answer.ok === false && typeof answer.error === "string" && answer.error.includes(named),
    `an error as an answer for ${what}: ${answer.error}`,
    JSON.stringify(answer).slice(0, 200),
  );
}
check(fuge.ask({ command: "version" }).ok === true, "the module answers on after errors");

// ---- Verovio engraves the MusicXML ----

/** Verovio's toolkit from the build beside the module, once its WebAssembly runs. */
async function verovioToolkit(file) {
  const verovio = createRequire(import.meta.url)(file);
  return await new Promise((resolve, reject) => {
    setTimeout(() => reject(new Error("Verovio's WebAssembly did not start")), 30_000);
    if (verovio.module.calledRun) resolve(new verovio.toolkit());
    else verovio.module.onRuntimeInitialized = () => resolve(new verovio.toolkit());
  });
}

const toolkitFile = fileURLToPath(new URL("verovio/verovio-toolkit-wasm.js", pathToFileURL(path)));
if (existsSync(toolkitFile)) {
  try {
    const toolkit = await verovioToolkit(toolkitFile);
    // The options of the page (web-fuge/app.js), for a paper 800 pixels wide.
    toolkit.setOptions({
      pageWidth: 1950,
      scale: 40,
      adjustPageHeight: true,
      breaks: "auto",
      header: "none",
      footer: "none",
      pageMarginTop: 40,
      pageMarginBottom: 40,
    });
    const count = (svg, name) => svg.match(new RegExp(`class="${name}"`, "g"))?.length ?? 0;
    const engraved = [
      ["of the module", fugue],
      ["of the module in four voices", four],
      ["of typed tones", own],
    ];
    for (const [what, answer] of engraved) {
      const loaded = toolkit.loadData(answer.files.musicxml);
      const pages = toolkit.getPageCount();
      let svg = "";
      for (let page = 1; page <= pages; page++) svg += toolkit.renderToSVG(page);
      // A tone tied over a barline is two notes; every section has its word over the staff.
      check(
        loaded && pages >= 1 && svg.startsWith("<svg") && count(svg, "measure") === answer.bars &&
          count(svg, "note") >= answer.tones && count(svg, "dir") === answer.sections.length &&
          answer.sections.every((section) => svg.includes(section.name)),
        `Verovio ${toolkit.getVersion()} engraves the fugue on the subject ${what}: ${pages} ` +
          `page(s), ${count(svg, "measure")} measures, ${count(svg, "note")} notes`,
        `loaded ${loaded}, ${count(svg, "dir")} words for ${answer.sections.length} sections`,
      );
    }
  } catch (error) {
    check(false, "Verovio engraves the fugues", `${error}`);
  }
} else {
  console.log(`skip  Verovio: no ${toolkitFile} (tools/build_web_fuge.sh puts it there)`);
}

// ---- the worker ----

// A module with the three exports whose fuge_run traps:
//   (module (memory (export "memory") 1)
//     (func (export "fuge_input") (param i32) (result i32) i32.const 0)
//     (func (export "fuge_run") (result i32) unreachable)
//     (func (export "fuge_output") (result i32) i32.const 0))
// deno-fmt-ignore
const trapping = Uint8Array.of(
  0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  0x01, 0x0a, 0x02, 0x60, 0x01, 0x7f, 0x01, 0x7f, 0x60, 0x00, 0x01, 0x7f,
  0x03, 0x04, 0x03, 0x00, 0x01, 0x01,
  0x05, 0x03, 0x01, 0x00, 0x01,
  0x07, 0x30, 0x04,
  0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00,
  0x0a, 0x66, 0x75, 0x67, 0x65, 0x5f, 0x69, 0x6e, 0x70, 0x75, 0x74, 0x00, 0x00,
  0x08, 0x66, 0x75, 0x67, 0x65, 0x5f, 0x72, 0x75, 0x6e, 0x00, 0x01,
  0x0b, 0x66, 0x75, 0x67, 0x65, 0x5f, 0x6f, 0x75, 0x74, 0x70, 0x75, 0x74, 0x00, 0x02,
  0x0a, 0x0f, 0x03,
  0x04, 0x00, 0x41, 0x00, 0x0b, 0x03, 0x00, 0x00, 0x0b, 0x04, 0x00, 0x41, 0x00, 0x0b,
);

// worker.js here, as a worker runs it: its scope takes the messages, and its fetch gives it the
// trapping module first, then, for each later load, the copy of the real one.
{
  const posted = [];
  const fetched = [];
  let loadsBegun = 0;
  const restore = [
    replace("self", { postMessage: (message) => posted.push(message) }),
    replace("fetch", async (url) => {
      fetched.push(`${url}`);
      if (`${url}`.endsWith(".gz")) loadsBegun += 1;
      const files = loadsBegun === 1 ? { "fuge_web.wasm": trapping } : { "fuge_web.wasm.gz": gzip };
      return serving(files)(url);
    }),
  ];
  try {
    await import(new URL("worker.js", import.meta.url).href);
    const scope = globalThis.self;
    await scope.onmessage({ data: { id: 7, request: { command: "version" } } });
    check(
      JSON.stringify(posted) === JSON.stringify([{
        id: 7,
        answer: { ok: false, error: "RuntimeError: unreachable", broken: true },
      }]),
      "worker.js: a module that traps is answered as broken off, with the trap",
      JSON.stringify(posted),
    );
    await scope.onmessage({ data: { id: 8, request: { command: "version" } } });
    await scope.onmessage({ data: { id: 9, request: { command: "sonata" } } });
    check(
      posted[1]?.id === 8 && posted[1].answer.ok === true && !("broken" in posted[1].answer) &&
        posted[2]?.id === 9 && posted[2].answer.ok === false && !("broken" in posted[2].answer) &&
        fetched.join() === "fuge_web.wasm.gz,fuge_web.wasm,fuge_web.wasm.gz",
      "worker.js: it loads a fresh module after the trap, which answers; an error is no trap",
      `${JSON.stringify(posted.slice(1))} ${fetched.join()}`,
    );
  } catch (error) {
    check(false, "worker.js answers", `${error}`);
  } finally {
    restore.forEach((put) => put());
  }
}

// The worker.js of the build as a real worker, where the runtime has web workers and they fetch
// the module from the files (Deno; not Node.js).
if (typeof globalThis.Worker === "function" && readsFiles) {
  const built = new URL(".", pathToFileURL(path));
  const worker = new globalThis.Worker(new URL("worker.js", built), { type: "module" });
  const ask = (id, request) =>
    new Promise((resolve) => {
      const timer = setTimeout(() => resolve({ error: "no answer within 60 s" }), 60_000);
      worker.onerror = (event) => {
        event.preventDefault?.();
        resolve({ error: event.message });
      };
      worker.onmessage = (event) => {
        clearTimeout(timer);
        resolve(event.data);
      };
      worker.postMessage({ id, request });
    });
  const listed = await ask(1, { command: "themes" });
  const set = await ask(2, request);
  check(
    listed.id === 1 && listed.answer?.themes?.length === themes.themes.length &&
      set.id === 2 && set.answer?.files?.midi === fugue.files.midi,
    "a real worker with the build's worker.js lists the subjects and sets the same fugue",
    JSON.stringify([listed, set]).slice(0, 300),
  );
  worker.terminate();
} else {
  console.log("skip  a real worker: this runtime has none that fetch the module from the files");
}

if (failures > 0) {
  console.log(`${failures} failure(s)`);
  process.exit(1);
}
// Timers of the test may still wait: the test is done.
process.exit(0);
