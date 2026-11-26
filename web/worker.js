// Runs the WebAssembly module of fux off the page's thread, so the page stays usable while the
// search runs. Messages: {id, request} in, {id, answer} out.
import { Fux } from "./fux.js";

let fux = Fux.fetch("fux_web.wasm");

self.onmessage = async (event) => {
  const { id, request } = event.data;
  try {
    const answer = (await fux).ask(request);
    self.postMessage({ id, answer });
  } catch (error) {
    // A trap (a bug, or memory that ran out) leaves the module unusable: the next request gets a
    // fresh one. The answer says the module broke off (`broken`), not that its search failed:
    // the request may succeed another time.
    fux = Fux.fetch("fux_web.wasm");
    self.postMessage({ id, answer: { ok: false, error: `${error}`, broken: true } });
  }
};
