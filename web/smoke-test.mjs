// Smoke test of the WebAssembly module, as CI runs it after the build:
//   node web/smoke-test.mjs target/web/fux_web.wasm
//   deno run --allow-read --allow-net=127.0.0.1 web/smoke-test.mjs target/web/fux_web.wasm \
//     [MESSE.teile]
// It loads the module as the page does (Fux.fetch: the gzip copy beside it, else the module
// itself): with a fetch of its own for every case, over HTTP where the runtime may listen on
// 127.0.0.1 (Node.js, Deno with --allow-net), and from the files where its fetch reads them
// (Deno). The pool of workers of a mass is tested with workers of its own; where the runtime has
// web workers (Deno), also with real ones from the build's worker.js. Given the folder of the
// parts of a small mass with the head motif as `fux mass` writes them (see „The head motif“
// below), the parts set here are compared with them byte for byte.
import { existsSync, readFileSync } from "node:fs";
import { createServer } from "node:http";
import process from "node:process";
import { pathToFileURL } from "node:url";
import { Fux } from "./fux.js";
import { order, Pool, poolSize, schedule } from "./pool.js";

const path = process.argv[2] ?? "target/web/fux_web.wasm";
const wasm = readFileSync(path);
const module = new WebAssembly.Module(wasm);
const imports = WebAssembly.Module.imports(module);
let failures = 0;

function check(condition, what, detail = "") {
  if (condition) {
    console.log(`ok    ${what}`);
  } else {
    failures += 1;
    console.log(`FAIL  ${what} ${detail}`);
  }
}

check(imports.length === 0, "the module needs no imports", JSON.stringify(imports));
check(
  WebAssembly.Module.customSections(module, "name").length === 0,
  "the module has no name section (the profile wasm strips it)",
);
const fux = await Fux.load(wasm);

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

/**
 * Fux.fetch(url) with every fetch recorded and answered by `respond`, the runtime's fetch unless
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
    const loaded = await Fux.fetch(url);
    return { urls, answers: loaded.ask({ command: "info" }).ok };
  } catch (error) {
    return { urls, error: `${error}` };
  } finally {
    restore.forEach((put) => put());
  }
}

/** A fetch that answers with `files` by URL, and with 404 for any other. */
function serving(files) {
  return async (url) =>
    `${url}` in files
      ? new Response(files[`${url}`])
      : new Response("", { status: 404, statusText: "Not Found" });
}

// tools/build_web.sh writes the copy beside the module. It must hold this very module, or the page
// would answer with another one.
const copy = existsSync(`${path}.gz`) ? readFileSync(`${path}.gz`) : null;
check(
  copy?.[0] === 0x1f && copy[1] === 0x8b &&
    wasm.equals(await through(copy, new DecompressionStream("gzip"))),
  `the module in gzip beside it (${copy?.length} instead of ${wasm.length} bytes)`,
  copy ? "" : `(no ${path}.gz)`,
);
const gzip = copy ?? await through(wasm, new CompressionStream("gzip"));
const broken = gzip.subarray(0, gzip.length >> 1);
const [plain, packed] = ["fux/fux_web.wasm", "fux/fux_web.wasm.gz"];
const loads = [
  ["the copy", { [packed]: gzip }, [packed]],
  ["the copy as a server decompressed it on the way", { [packed]: wasm }, [packed]],
  ["without the copy the module", { [plain]: wasm }, [packed, plain]],
  ["for a broken copy the module", { [packed]: broken, [plain]: wasm }, [packed, plain]],
  ["for a page in place of the copy the module", { [packed]: "<!doctype html>", [plain]: wasm }, [
    packed,
    plain,
  ]],
];
for (const [what, files, fetched] of loads) {
  const loaded = await load(plain, { respond: serving(files) });
  check(
    loaded.answers && loaded.urls.join() === fetched.join(),
    `Fux.fetch: ${what}`,
    JSON.stringify(loaded),
  );
}
const without = await load(plain, {
  respond: serving({ [packed]: gzip, [plain]: wasm }),
  decompress: false,
});
check(
  without.answers && without.urls.join() === plain,
  "Fux.fetch: without DecompressionStream the module alone",
  JSON.stringify(without),
);
const neither = await load(plain, { respond: serving({}) });
check(
  neither.error === `Error: ${plain}: 404 Not Found` &&
    neither.urls.join() === `${packed},${plain}`,
  "Fux.fetch: without either file an error that names the module",
  JSON.stringify(neither),
);

// Over HTTP, from a server of the test: the copy as a file, the copy with `Content-Encoding: gzip`
// (as a server sends it that decompresses on the way; fetch then hands over the module), and no
// copy. Deno listens only with --allow-net; it is not asked for it.
const net = globalThis.Deno?.permissions.querySync({ name: "net", host: "127.0.0.1" }).state;
if (net === undefined || net === "granted") {
  const served = [];
  const server = createServer((request, response) => {
    served.push(request.url);
    const [, folder, file] = request.url.split("/");
    let headers = { "content-type": "application/wasm" };
    let body = wasm;
    if (file === "fux_web.wasm.gz" && folder === "file") {
      headers = { "content-type": "application/gzip" };
      body = gzip;
    } else if (file === "fux_web.wasm.gz" && folder === "encoded") {
      headers = { "content-type": "application/wasm", "content-encoding": "gzip" };
      body = gzip;
    } else if (file !== "fux_web.wasm") {
      headers = {};
      body = "";
    }
    response.writeHead(body === "" ? 404 : 200, { ...headers, connection: "close" });
    response.end(body);
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const site = `http://127.0.0.1:${server.address().port}`;
  const encoded = await fetch(`${site}/encoded/fux_web.wasm.gz`).then((got) => got.arrayBuffer());
  check(
    wasm.equals(new Uint8Array(encoded)),
    "fetch decompresses what the server sends with Content-Encoding: gzip",
  );
  for (
    const [folder, what, fetched] of [
      ["file", "the copy", ["/file/fux_web.wasm.gz"]],
      ["encoded", "the copy with Content-Encoding: gzip", ["/encoded/fux_web.wasm.gz"]],
      ["none", "without the copy the module", ["/none/fux_web.wasm.gz", "/none/fux_web.wasm"]],
    ]
  ) {
    served.length = 0;
    const loaded = await load(`${site}/${folder}/fux_web.wasm`);
    check(
      loaded.answers && served.join() === fetched.join(),
      `Fux.fetch over HTTP: ${what}`,
      JSON.stringify({ loaded, served }),
    );
  }
  server.close();
} else {
  console.log(
    "skip  Fux.fetch over HTTP: Deno may not listen on 127.0.0.1 (--allow-net=127.0.0.1)",
  );
}

// From the build's files, where fetch reads them (Deno), as the real workers below load it.
if (await fetch(import.meta.url).then((got) => got.arrayBuffer(), () => null)) {
  const loaded = await load(pathToFileURL(path));
  check(
    loaded.answers && loaded.urls.join() === `${pathToFileURL(path)}.gz`,
    "Fux.fetch from the files: the copy",
    JSON.stringify(loaded),
  );
} else {
  console.log("skip  Fux.fetch from the files: this runtime's fetch reads none");
}

const info = fux.ask({ command: "info", lang: "en" });
check(info.ok && info.info.cantus.length === 8 && info.info.examples.length === 123, "info");
const kyrie = info.info?.texts.find((text) => text.id === "kyrie");
check(
  info.info?.texts.length === 17 && kyrie?.syllables === 7 && kyrie?.length === 16,
  "the sections of the Ordinary",
  JSON.stringify(kyrie),
);

for (const species of [1, 2, 3, 4, 5]) {
  const started = performance.now();
  const answer = fux.ask({ command: "cp", cf: "fux:dorian", species, seed: 1 });
  const seconds = ((performance.now() - started) / 1000).toFixed(1);
  check(
    answer.ok && answer.results.length === 1 && answer.results[0].forbidden === 0,
    `species ${species} in two voices (${seconds} s)`,
    answer.error ?? "",
  );
}

const started = performance.now();
const four = fux.ask({ command: "cp", cf: "fux:phrygian", species: 4, voices: 4, seed: 2 });
const seconds = ((performance.now() - started) / 1000).toFixed(1);
check(
  four.ok && four.results[0].lines.length === 4,
  `four voices (${seconds} s)`,
  four.error ?? "",
);

const result = four.results?.[0];
check(result?.musicxml.includes("<score-partwise"), "MusicXML");
check(result?.midi.startsWith("TVRoZA"), "MIDI");
check(new Set(result?.notes.map((n) => n.voice)).size === 4, "notes to play in every voice");

const freeStarted = performance.now();
const free = fux.ask({
  command: "free",
  mode: "dorian",
  voices: 3,
  length: 10,
  text: "none",
  seed: 1,
});
const freeSeconds = ((performance.now() - freeStarted) / 1000).toFixed(1);
check(
  free.ok && free.results[0].lines.length === 3 && free.results[0].forbidden === 0,
  `free counterpoint in three voices (${freeSeconds} s)`,
  free.error ?? "",
);
check(free.results?.[0].lyrics.every((lyrics) => lyrics === null), "without text no lyrics");

// Kyrie I, the default text, in two voices: its length and cadence planned from the text.
const textStarted = performance.now();
const texted = fux.ask({ command: "free", voices: 2, text: "kyrie", seed: 1 });
const textSeconds = ((performance.now() - textStarted) / 1000).toFixed(1);
const sung = texted.results?.[0];
check(
  texted.ok && sung.forbidden === 0 &&
    texted.summary.some((line) => line.startsWith("Text: Kyrie I")),
  `free counterpoint on Kyrie I in two voices (${textSeconds} s)`,
  texted.error ?? JSON.stringify(texted.summary),
);
// Every voice sings the whole text, each syllable as a <lyric> in the MusicXML.
const syllables = (sung?.lyrics ?? []).map((lyrics) => lyrics?.split(/[\s-]+/).length ?? 0);
const lyricElements = sung?.musicxml.match(/<lyric[ >]/g)?.length ?? 0;
check(
  sung?.lyrics.every((lyrics) => lyrics?.startsWith("Ký-ri-e e-lé-i-son.")) &&
    lyricElements >= syllables.reduce((sum, n) => sum + n, 0) && lyricElements > 0,
  `lyrics in the MusicXML (${lyricElements} <lyric>)`,
  JSON.stringify(sung?.lyrics),
);
// Checked back: the file names its text and cadences, so the report is the setting's.
const back = fux.ask({ command: "check", musicxml: sung?.musicxml ?? "", lang: "en" });
const cases = (entry) => JSON.stringify(entry?.violations.map((v) => [v.rule, v.level]));
check(
  back.ok && back.summary[0].startsWith("Text: Kyrie I") && back.results[0].forbidden === 0 &&
    cases(back.results[0]) === cases(sung),
  "the setting checked back with its text",
  back.error ?? JSON.stringify(back.summary),
);

// The text command, as the page previews a text: the stressed syllables and the planned length.
const preview = fux.ask({ command: "text", text: "kyrie", voices: 2 });
const unit = preview.text?.units[0].map((s) =>
  (s.stress ? s.text.toUpperCase() : s.text) + s.after
);
check(
  preview.ok && unit?.join("") === "KÝ-ri-e / e-LÉ-i-son." && preview.text.length === 16 &&
    preview.plan.measures === 16,
  "the text command",
  JSON.stringify(preview.text?.units ?? preview.error),
);
const own = fux.ask({ command: "text", text: "Ad te levavi animam meam.", lang: "en" });
check(
  own.ok && own.text.id === "eigen" && own.text.warnings.length === 1 && own.plan.measures > 0,
  "an own text with a warning",
  JSON.stringify(own.text?.warnings ?? own.error),
);
const unknown = fux.ask({ command: "text", text: "kyrei" });
check(!unknown.ok && unknown.error.includes("kyrie"), "an unknown section is an error");

// Kyrie I in three voices, which imitate each other: a line on each point in the summary,
// brackets over the entries in LilyPond and MusicXML, and the plan in the file, which a check
// takes back. The page asks for the plan alone and for its length first.
const imitationStarted = performance.now();
const imitative = fux.ask({ command: "free", voices: 3, text: "kyrie", seed: 1, count: 1 });
const imitationSeconds = ((performance.now() - imitationStarted) / 1000).toFixed(1);
const made = imitative.results?.[0];
const points = (imitative.summary ?? []).filter((line) => line.startsWith("Punkt "));
check(
  imitative.ok && made.forbidden === 0 && points.length >= 2 &&
    points[0].startsWith("Punkt 1 („Ký-ri-e e-lé-i-son.“): "),
  `an imitative Kyrie in three voices (${imitationSeconds} s)`,
  imitative.error ?? JSON.stringify(imitative.summary),
);
const groups = made?.lilypond.match(/\\startGroup/g)?.length ?? 0;
check(
  groups > 0 && groups === (made?.lilypond.match(/\\stopGroup/g)?.length ?? 0) &&
    made.lilypond.includes('\\consists "Horizontal_bracket_engraver"'),
  `brackets over the entries in LilyPond (${groups})`,
);
check(
  made?.musicxml.includes('<miscellaneous-field name="fux-imitation">p0-7') &&
    made.musicxml.includes('<bracket type="start"') && made.musicxml.includes("<words>1</words>"),
  "brackets and the plan in MusicXML",
);
const replanned = fux.ask({ command: "check", musicxml: made?.musicxml ?? "" });
check(
  replanned.ok &&
    replanned.summary.includes(`Imitation nach dem Plan der Datei: ${points.length} Punkte`) &&
    cases(replanned.results[0]) === cases(made),
  "the imitative setting checked back with its plan",
  replanned.error ?? JSON.stringify(replanned.summary),
);
const planOnly = fux.ask({
  command: "free",
  voices: 3,
  text: "kyrie",
  seed: 1,
  imitation: { plan_only: true },
});
check(
  planOnly.ok && planOnly.results.length === 0 &&
    planOnly.summary[0].startsWith("dorisch – Plan der Imitation, 3 Stimmen, ") &&
    planOnly.summary.includes(points[0]),
  "the plan without a search",
  planOnly.error ?? JSON.stringify(planOnly.summary),
);
const measures = Number(imitative.summary?.[0].match(/(\d+) Takte/)?.[1]);
const length = fux.ask({ command: "text", voices: 3, seed: 1 });
check(
  length.plan?.imitation === true && length.plan.measures === measures,
  `the length of the imitation's plan (${measures} measures)`,
  JSON.stringify(length.plan),
);
const refused = fux.ask({ command: "free", voices: 2, imitation: { mode: "an" }, text: "none" });
check(
  !refused.ok && refused.error.startsWith("Imitation braucht einen Text"),
  "imitation needs a text",
);

const cantus = fux.ask({ command: "cf", mode: "lydian", length: 10, count: 3, seed: 4 });
check(cantus.ok && cantus.results.length === 3, "cantus firmi", cantus.error ?? "");

const checked = fux.ask({ command: "check", cf: "D4 E4", others: ["S=f'1 b'1"] });
check(
  checked.ok && checked.results[0].violations.some((v) => v.rule === "MEL-TRITONE"),
  "check finds the tritone",
);

const example = fux.ask({ command: "example", figure: "84a", profile: "jeppesen" });
check(example.ok && example.results[0].label === "Fig. 84a", "example with Jeppesen's rules");

const bad = fux.ask({ command: "cp", cf: "X9" });
check(!bad.ok && bad.error.length > 0, "errors are answers", bad.error ?? "");
// The module still works after an error.
check(fux.ask({ command: "info" }).ok, "still answering");

// ---- the mass (phase 14c) ----

// The plan of a Missa brevis, without a search: its parts with their estimates, the table of
// sections, and the estimate in the browser with four workers as `fux mass --plan` gives it.
const massPlan = fux.ask({ command: "mass_plan", mass: { seed: 1 } });
const planned = massPlan.mass;
check(
  massPlan.ok && planned.parts.length === 26 && planned.rows.length === 13 &&
    planned.parts[0].address === "kyrie/1" && planned.factor > 0,
  "mass_plan: the plan of a Missa brevis",
  massPlan.error ?? JSON.stringify(massPlan.summary),
);
// The jobs of the pool on four workers (or `workers`), in the order it starts them, as the page
// makes them for a run of `parts`.
const jobs = (view, parts = view.parts, workers = 4) =>
  order(
    parts.map((part) => ({
      key: part.address,
      movement: part.movement,
      seconds: part.seconds * view.factor,
      source: part.source,
      waits: part.motif,
    })),
    workers,
  );
const browser = Number(massPlan.summary?.[1].match(/4 Worker: etwa (\d+) min/)?.[1]);
const minutes = Math.round(schedule(jobs(planned), 4) / 60);
check(minutes === browser, `the pool's estimate is the plan's (${minutes} min)`, `${browser}`);
// The order of MassPlan::pool_order for this mass, as fux-core's test
// the_pool_starts_the_longest_then_whole_movements has it: Sanctus and Agnus Dei, the longest;
// Kyrie, Gloria and the longest parts of the Credo; the last twelve longest first.
const poolOrder = jobs(planned).map((job) => job.key).join(" ");
check(
  poolOrder === "sanctus/1 agnus/2 agnus/1 kyrie/1 kyrie/3 kyrie/2 gloria/1 gloria/5 gloria/3 " +
      "gloria/2 gloria/4 gloria/6 credo/11 credo/8 credo/1 credo/3 credo/2 benedictus/1 credo/7 " +
      "credo/10 credo/9 benedictus/2 credo/5 credo/12 credo/6 credo/4",
  "the pool's order: the longest, then whole movements, the last parts longest first",
  poolOrder,
);
const motifPlan = fux.ask({ command: "mass_plan", mass: { seed: 1, motif: true } }).mass;
const kyrieFirst = motifPlan?.parts.find((part) => part.source);
const opening = Math.max(
  ...(motifPlan?.parts.filter((part) => part.motif).map((part) => part.seconds) ?? []),
);
check(
  motifPlan?.parts.filter((part) => part.motif).length === 4 &&
    kyrieFirst?.address === "kyrie/1" && jobs(motifPlan)[0].key === "kyrie/1" &&
    schedule(jobs(motifPlan), 4) >= (kyrieFirst.seconds + opening) * motifPlan.factor,
  "the head motif: four parts wait for Kyrie I, which goes first",
);
// The jobs of a run go in the same order on its parts, as MassPlan::pool_order_of has them for
// the pool's estimate and for `fux mass` with several parts at once (fux-core's test
// the_pool_orders_the_parts_of_a_run_alike): without the Kyrie on four workers, no part first
// for the motif; Kyrie I, the Credo and the Agnus Dei on two, Kyrie I first.
const runOrder = (keep, workers) =>
  jobs(motifPlan, motifPlan?.parts.filter(keep) ?? [], workers).map((job) => job.key).join(" ");
const withoutKyrie = runOrder((part) => !part.address.startsWith("kyrie/"), 4);
const kyrieCredoAgnus = runOrder(
  (part) => part.source || /^(credo|agnus)\//.test(part.address),
  2,
);
check(
  withoutKyrie === "sanctus/1 agnus/2 agnus/1 gloria/1 gloria/5 gloria/3 gloria/2 gloria/4 " +
        "gloria/6 credo/11 credo/8 credo/1 credo/3 credo/2 benedictus/1 credo/7 credo/10 credo/9 " +
        "benedictus/2 credo/5 credo/12 credo/6 credo/4" &&
    kyrieCredoAgnus === "kyrie/1 agnus/2 credo/11 credo/8 credo/1 credo/3 credo/2 credo/7 " +
        "credo/10 agnus/1 credo/9 credo/5 credo/12 credo/6 credo/4",
  "the pool's order of the parts of a run, as fux-core has it",
  `${withoutKyrie} | ${kyrieCredoAgnus}`,
);
const wrongRow = fux.ask({
  command: "mass_plan",
  mass: { seed: 1, sections: [{ id: "christe", voices: "S X" }] },
});
check(
  !wrongRow.ok && wrongRow.error.includes("„christe“") && wrongRow.mass?.rows.length === 13,
  "a row of the table that does not fit: the error and the rows",
  wrongRow.error ?? "",
);

// A part as a worker sets it: the Christe in three voices.
const christeStarted = performance.now();
const christe = fux.ask({
  command: "mass_part",
  mass: { movements: "kyrie", voices: 3, seed: 1 },
  part: "kyrie/2",
});
const christeSeconds = ((performance.now() - christeStarted) / 1000).toFixed(1);
check(
  christe.ok && christe.part.address === "kyrie/2" && christe.part.forbidden === 0 &&
    christe.part.musicxml.includes('<miscellaneous-field name="fux-part">bars=1-') &&
    typeof christe.part.seed === "string" &&
    christe.summary[0].startsWith("dorisch – freier Satz, 3 Stimmen, "),
  `mass_part: the Christe in three voices (${christeSeconds} s)`,
  christe.error ?? JSON.stringify(christe.summary),
);

// The mass of its parts: a Kyrie in two voices, set part by part, then put together.
const small = { movements: "kyrie", voices: 2, beam: 40, seed: 1 };
const kyrieParts = ["kyrie/1", "kyrie/2", "kyrie/3"].map((part) =>
  fux.ask({ command: "mass_part", mass: small, part })
);
check(kyrieParts.every((answer) => answer.ok), "the parts of a Kyrie in two voices");
const files = kyrieParts.map((answer) => ({
  part: answer.part?.address,
  musicxml: answer.part?.musicxml ?? "",
  seconds: 1,
}));
const joined = fux.ask({ command: "mass_join", mass: small, parts: files });
const kyrieFile = joined.results?.[0];
check(
  joined.ok && joined.results.length === 1 && kyrieFile.file === "messe-1-kyrie" &&
    kyrieFile.musicxml.includes('name="fux-part.3"') &&
    kyrieFile.sections.map((section) => section.title).join() === "Kyrie I,Christe,Kyrie II" &&
    kyrieFile.midi.startsWith("TVRoZA") && joined.book.includes("\\book") &&
    kyrieFile.cost === kyrieParts.reduce((sum, answer) => sum + answer.part.cost, 0),
  "mass_join: the movement, its MIDI file and the book",
  joined.error ?? JSON.stringify(joined.summary),
);
const movementChecked = fux.ask({ command: "check", musicxml: kyrieFile?.musicxml ?? "" });
check(
  movementChecked.ok && movementChecked.summary.at(-1).endsWith(" in 3 Teilen.") &&
    movementChecked.results[0].cost === kyrieFile.cost,
  "the file of the movement checked part by part, with the costs of the parts",
  movementChecked.error ?? JSON.stringify(movementChecked.summary),
);
const partly = fux.ask({
  command: "mass_join",
  mass: small,
  parts: [files[1], { part: "kyrie/1", error: "abgebrochen" }],
});
check(
  partly.ok && partly.results.length === 0 && partly.book.includes("Teil kyrie/3 fehlt") &&
    partly.summary.at(-1).startsWith("Unvollständig, es fehlen: kyrie/1, kyrie/3.") &&
    partly.summary.at(-1).includes("Fehlende Teile setzt die Seite nach") &&
    partly.summary.at(-1).includes("„Fehlende Teile setzen“ lässt einen gescheiterten Teil aus"),
  "mass_join of a mass that lacks parts: how the page sets them, what helps a failed one",
  partly.error ?? JSON.stringify(partly.summary),
);
const meanwhile = fux.ask({ command: "mass_join", mass: small, parts: [files[1]], running: true });
check(
  meanwhile.ok && !meanwhile.summary.some((line) => line.startsWith("Unvollständig")) &&
    meanwhile.book.includes("Teil kyrie/1 fehlt") &&
    !meanwhile.book.includes("Fehlende Teile setzt die Seite nach"),
  "mass_join while the pool sets parts: the parts missing go without saying",
  meanwhile.error ?? JSON.stringify(meanwhile.summary),
);
const foreign = fux.ask({ command: "mass_join", mass: { ...small, seed: 2 }, parts: files });
check(!foreign.ok && foreign.error.includes("kyrie/1"), "a part of another mass is refused");

// The head motif, as the pool passes it on: a Kyrie and an Agnus Dei in two voices, every part
// in imitation. Kyrie I answers with the motif it gives, and Agnus Dei I opens with it; the files
// of the movements keep it. Given the folder of the parts that `fux mass` writes for this mass
// (the second argument, MESSE.teile), every part is compared with its file there, byte for byte:
//   fux mass --movements kyrie,agnus --voices 2 --beam 40 --lengths natürlich --imitation an \
//     --motif an --seed 6 -o MESSE
const motifMass = {
  movements: "kyrie,agnus",
  voices: 2,
  beam: 40,
  lengths: "natürlich",
  imitation: "an",
  motif: true,
  seed: 6,
};
const motifStarted = performance.now();
const motifPart = (part, motif = null) =>
  fux.ask({ command: "mass_part", mass: motifMass, part, motif });
const kyrieI = motifPart("kyrie/1");
const motif = kyrieI.part?.motif;
check(
  kyrieI.ok && motif === "0,1,2,1,0,-2,-3",
  "mass_part: Kyrie I gives the head motif",
  kyrieI.error ?? JSON.stringify(motif),
);
const agnusI = motifPart("agnus/1", motif);
check(
  agnusI.ok && agnusI.part.warnings.length === 0 && !("motif" in agnusI.part) &&
    agnusI.part.musicxml.includes(`;motif=${motif};`),
  "mass_part: Agnus Dei I opens with it",
  agnusI.error ?? JSON.stringify(agnusI.part?.warnings),
);
const noMotif = motifPart("agnus/1");
check(
  noMotif.ok && noMotif.part.warnings.length === 1 &&
    noMotif.part.warnings[0].startsWith("Agnus Dei: Das Kyrie I gab kein Kopfmotiv") &&
    noMotif.part.musicxml !== agnusI.part.musicxml,
  "without the motif Agnus Dei I sings its words, with a warning",
  noMotif.error ?? JSON.stringify(noMotif.part?.warnings),
);
const motifParts = [
  kyrieI,
  motifPart("kyrie/2"),
  motifPart("kyrie/3"),
  agnusI,
  motifPart("agnus/2"),
];
const motifSeconds = ((performance.now() - motifStarted) / 1000).toFixed(1);
const fileOf = (answer) => ({ part: answer.part?.address, musicxml: answer.part?.musicxml ?? "" });
const firstPart = (xml) => xml.match(/<miscellaneous-field name="fux-part\.1">([^<]*)</)?.[1] ?? "";
const motifJoined = fux.ask({
  command: "mass_join",
  mass: motifMass,
  parts: motifParts.map(fileOf),
});
check(
  motifJoined.ok && motifJoined.results.length === 2 &&
    motifJoined.results.every((entry) => firstPart(entry.musicxml).includes(`;motif=${motif};`)),
  `mass_join: the files of the movements keep the motif (${motifSeconds} s for its parts)`,
  motifJoined.error ?? JSON.stringify(motifJoined.summary),
);
const otherMotif = fux.ask({
  command: "mass_join",
  mass: motifMass,
  parts: [...motifParts.slice(0, 3), noMotif, motifParts[4]].map(fileOf),
});
check(
  !otherMotif.ok && otherMotif.error.startsWith("Teil agnus/1: ") &&
    otherMotif.error.includes("Kopfmotiv"),
  "a part without the motif that Kyrie I gives is refused",
  otherMotif.error ?? "",
);
const fuxMass = process.argv[3];
if (fuxMass) {
  const names = ["1-01-kyrie", "1-02-christe", "1-03-kyrie2", "6-01-agnus", "6-02-agnus3"];
  const same = names.filter((name, i) =>
    readFileSync(`${fuxMass}/${name}.musicxml`, "utf8") === motifParts[i].part?.musicxml
  );
  check(
    same.length === names.length,
    "the parts of the mass with the head motif are those of fux mass, byte for byte",
    `the same: ${same.join(", ")}`,
  );
} else {
  console.log("skip  the parts of fux mass: no folder of parts given");
}

// The pool with workers of its own, which answer after the milliseconds their request names:
// each worker takes the first job that is ready; the parts that wait for the motif get it; a
// cancel terminates the busy workers, and the next run starts fresh ones.
class Fake {
  static made = 0;
  static terminated = 0;
  static requests = [];
  constructor() {
    Fake.made += 1;
  }
  postMessage({ id, request }) {
    Fake.requests.push(request);
    this.timer = setTimeout(() => {
      const ok = !request.part.startsWith("fail");
      const part = { address: request.part, motif: request.part === "kyrie/1" ? "0,2,4" : null };
      this.onmessage?.({ data: { id, answer: ok ? { ok, part } : { ok, error: "gescheitert" } } });
    }, request.ms);
  }
  terminate() {
    clearTimeout(this.timer);
    Fake.terminated += 1;
  }
}
const job = (key, seconds, more = {}) => ({
  key,
  seconds,
  ...more,
  request: { part: key, ms: seconds * 10 },
});
const few = order([
  job("kyrie/1", 1, { source: true, movement: 0 }),
  job("gloria/1", 9, { waits: true, movement: 1 }),
  job("credo/2", 5, { movement: 2 }),
  job("agnus/2", 7, { movement: 5 }),
], 2).map((job) => job.key).join();
check(
  few === "kyrie/1,gloria/1,agnus/2,credo/2",
  "the order of few parts: the source of the motif first, then the longest",
  few,
);
const begun = [];
const fakePool = new Pool(1, () => new Fake());
const done = await fakePool.run([
  job("gloria/1", 9, { waits: true }),
  job("kyrie/1", 1, { source: true }),
  job("agnus/2", 7),
  job("credo/2", 5),
], { started: (job) => begun.push(job.key) });
check(
  !done.cancelled && begun.join() === "kyrie/1,gloria/1,agnus/2,credo/2",
  "the pool: the first job that is ready, a part that waits for the motif once it is known",
  begun.join(),
);
const gloria = Fake.requests.find((request) => request.part === "gloria/1");
check(
  gloria?.motif === "0,2,4" && !("motif" in Fake.requests[0]),
  "the parts that wait for the motif get it",
  JSON.stringify(gloria),
);
const waited = [];
await new Pool(2, () => ({
  postMessage({ id, request }) {
    waited.push(request.motif);
    setTimeout(() => this.onmessage({ data: { id, answer: { ok: false, error: "x" } } }), 5);
  },
  terminate() {},
})).run([job("kyrie/1", 1, { source: true }), job("gloria/1", 9, { waits: true })]);
check(
  waited.length === 2 && waited[1] === null,
  "a failed source lets the parts that wait for it go on without the motif",
  JSON.stringify(waited),
);
// A worker that breaks off (an error event, or a trap of its module) is no failed search: its job
// may be set another time, and the jobs that wait for the motif of such a source are left.
const broke = [];
const brokenRun = await new Pool(2, () => ({
  postMessage({ id, request }) {
    setTimeout(() => {
      if (request.part === "kyrie/1") {
        this.onerror({ message: "out of memory", preventDefault() {} });
        return;
      }
      const answer = request.part === "credo/2"
        ? { ok: false, error: "RuntimeError: unreachable", broken: true }
        : { ok: false, error: "kein Satz" };
      this.onmessage({ data: { id, answer } });
    }, 5);
  },
  terminate() {},
})).run([
  job("kyrie/1", 1, { source: true }),
  job("gloria/1", 9, { waits: true }),
  job("credo/2", 5),
  job("agnus/2", 3),
], { failed: (job, error, _seconds, broken) => broke.push(`${job.key} ${broken} ${error}`) });
check(
  broke.join() ===
      "kyrie/1 true out of memory,credo/2 true RuntimeError: unreachable,agnus/2 false kein Satz" &&
    !brokenRun.cancelled && brokenRun.left.map((left) => left.key).join() === "gloria/1",
  "a worker that breaks off is no failed search; what waits for its motif is left",
  JSON.stringify({ broke, left: brokenRun.left }),
);
const busyPool = new Pool(2, () => new Fake());
const spawned = Fake.made;
let stops = 0;
const cancelled = busyPool.run([job("a/1", 500), job("a/2", 400), job("a/3", 300)], {
  started: () => {
    stops += 1;
    if (stops === 2) setTimeout(() => busyPool.cancel(), 20);
  },
});
const stopped = await cancelled;
check(
  stopped.cancelled && stopped.stopped.length === 2 && stopped.left.length === 1 &&
    Fake.terminated >= 2 && !busyPool.busy,
  "cancel terminates the busy workers at once",
  JSON.stringify(stopped),
);
const after = await busyPool.run([job("a/3", 1)]);
check(
  !after.cancelled && Fake.made === spawned + 3,
  "after a cancel the pool starts fresh workers",
);
check(
  poolSize({ hardwareConcurrency: 8, deviceMemory: 8 }) === 4 &&
    poolSize({ hardwareConcurrency: 2, deviceMemory: 8 }) === 1 &&
    poolSize({ hardwareConcurrency: 16, deviceMemory: 4 }) === 2 &&
    poolSize({ hardwareConcurrency: 16, deviceMemory: 1 }) === 1 &&
    poolSize({}) === 3,
  "the size of the pool by cores and memory",
);

// Real workers, where the runtime has them (Deno; not Node.js): a run cancelled in the middle of
// a search ends at once, and a fresh worker answers. Deno (2.9) lets the thread of a terminated
// worker run on, which browsers stop; the test ends the process itself (below).
if (typeof globalThis.Worker === "function") {
  const built = new URL(".", pathToFileURL(path));
  const spawn = () => new globalThis.Worker(new URL("worker.js", built), { type: "module" });
  const realPool = new Pool(1, spawn);
  const longest = {
    key: "credo/11",
    seconds: 1,
    request: { command: "mass_part", mass: { seed: 1 }, part: "credo/11" },
  };
  const began = performance.now();
  const running = realPool.run([longest]);
  await new Promise((resolve) => setTimeout(resolve, 2000));
  realPool.cancel();
  const result = await running;
  const waitedFor = (performance.now() - began) / 1000;
  check(
    result.cancelled && result.stopped.length === 1 && waitedFor < 3,
    `a run of a real worker cancelled in its search (${waitedFor.toFixed(1)} s)`,
  );
  const answered = await new Promise((resolve) => {
    realPool.run([{
      key: "plan",
      seconds: 1,
      request: { command: "mass_plan", mass: { seed: 1, movements: "kyrie" } },
    }], {
      finished: (_job, answer) => resolve(answer),
      failed: (_job, error) => resolve({ ok: false, error }),
    });
  });
  check(answered.ok && answered.mass.parts.length === 3, "a fresh worker answers after it");
  // The head motif through real workers: Agnus Dei I waits for Kyrie I and opens with its motif.
  const answers = {};
  const request = (part) => ({ command: "mass_part", mass: motifMass, part });
  await new Pool(2, spawn).run([
    { key: "agnus/1", seconds: 2, waits: true, request: request("agnus/1") },
    { key: "kyrie/1", seconds: 1, source: true, request: request("kyrie/1") },
  ], {
    finished: (job, answer) => {
      answers[job.key] = answer;
    },
  });
  check(
    answers["kyrie/1"]?.part?.motif === motif &&
      answers["agnus/1"]?.part?.musicxml === agnusI.part?.musicxml,
    "real workers: Agnus Dei I waits for the motif of Kyrie I and opens with it",
  );
} else {
  console.log("skip  real workers: this runtime has none");
}

if (failures > 0) {
  console.log(`${failures} failure(s)`);
  process.exit(1);
}
// The thread of a terminated worker may still run (Deno): the test is done.
process.exit(0);
