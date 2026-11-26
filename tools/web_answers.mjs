// The answers of the WebAssembly module to a fixed set of requests, one JSON file each, for
// tools/compare_outputs.sh:
//   deno run --allow-read --allow-write tools/web_answers.mjs FUX_WEB.WASM FUX.JS OUT
//   node tools/web_answers.mjs FUX_WEB.WASM FUX.JS OUT
// FUX.JS is the web/fux.js of the same build. The requests are those of web/smoke-test.mjs:
// every command, the five species, a free setting with imitation, plans of masses and two small
// masses set part by part as the page's pool sets them, then put together.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import process from "node:process";
import { pathToFileURL } from "node:url";
import { resolve } from "node:path";

const args = process.argv.slice(2);
if (args.length !== 3) {
  console.error("Usage: tools/web_answers.mjs FUX_WEB.WASM FUX.JS OUT");
  process.exit(1);
}
const [wasm, fuxJs, out] = args;
const { Fux } = await import(pathToFileURL(resolve(fuxJs)).href);
const fux = await Fux.load(readFileSync(wasm));
mkdirSync(out, { recursive: true });
const encoder = new TextEncoder();

// As Fux.ask, but the answer is written as the module's bytes, before JSON.parse could round a
// number or merge keys.
function ask(name, request) {
  const { exports } = fux;
  const bytes = encoder.encode(JSON.stringify(request));
  const at = exports.fux_input(bytes.length);
  new Uint8Array(exports.memory.buffer, at, bytes.length).set(bytes);
  const length = exports.fux_run();
  const answer = new Uint8Array(exports.memory.buffer, exports.fux_output(), length).slice();
  writeFileSync(`${out}/${name}.json`, answer);
  return JSON.parse(new TextDecoder().decode(answer));
}

ask("info", { command: "info", lang: "en" });
ask("cf", { command: "cf", mode: "lydian", length: 10, count: 3, seed: 4 });
for (const species of [1, 2, 3, 4, 5]) {
  ask(`cp-${species}`, { command: "cp", cf: "fux:dorian", species, seed: 1 });
}
ask("cp-four", { command: "cp", cf: "fux:phrygian", species: 4, voices: 4, seed: 2 });
ask("cp-error", { command: "cp", cf: "X9" });
ask("free-two", { command: "free", voices: 2, text: "kyrie", seed: 1 });
const imitative = ask("free-imitative", {
  command: "free",
  voices: 3,
  text: "kyrie",
  seed: 1,
  count: 1,
});
ask("check-imitative", { command: "check", musicxml: imitative.results?.[0]?.musicxml ?? "" });
ask("check-lines", { command: "check", cf: "D4 E4", others: ["S=f'1 b'1"] });
ask("example", { command: "example", figure: "84a", profile: "jeppesen" });
ask("text-own", { command: "text", text: "Ad te levavi animam meam.", lang: "en" });
ask("text-unknown", { command: "text", text: "kyrei" });
ask("text-kyrie", { command: "text", text: "kyrie", voices: 2 });
ask("text-default", { command: "text", voices: 3, seed: 1 });
ask("mass-plan", { command: "mass_plan", mass: { seed: 1 } });
ask("mass-plan-motif", { command: "mass_plan", mass: { seed: 1, motif: true } });
ask("mass-plan-kyrie", { command: "mass_plan", mass: { seed: 1, movements: "kyrie" } });

// A mass set part by part in the order of its plan, the head motif passed on to the parts that
// wait for it as the page does, then put together; the seconds the page would send are fixed.
function mass(name, settings) {
  const plan = ask(`${name}-plan`, { command: "mass_plan", mass: settings });
  let motif = null;
  const files = [];
  for (const part of plan.mass?.parts ?? []) {
    const address = part.address;
    const answer = ask(`${name}-${address.replace("/", "-")}`, {
      command: "mass_part",
      mass: settings,
      part: address,
      motif: part.motif ? motif : null,
    });
    motif ??= answer.part?.motif ?? null;
    files.push({ part: address, musicxml: answer.part?.musicxml ?? "", seconds: 1 });
  }
  ask(`${name}-join`, { command: "mass_join", mass: settings, parts: files });
}
mass("kyrie", { movements: "kyrie", voices: 2, beam: 40, seed: 1 });
mass("motif", {
  movements: "kyrie,agnus",
  voices: 2,
  beam: 40,
  lengths: "natürlich",
  imitation: "an",
  motif: true,
  seed: 6,
});
