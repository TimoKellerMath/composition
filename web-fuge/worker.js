// Runs the WebAssembly module of fux-fuge off the page's thread, so the page stays usable while a
// fugue is set. Messages: {id, request} in, {id, answer} out.
import { Fuge } from "./fuge.js";

let fuge = Fuge.fetch("fuge_web.wasm");

self.onmessage = async (event) => {
  const { id, request } = event.data;
  try {
    const answer = (await fuge).ask(request);
    self.postMessage({ id, answer });
  } catch (error) {
    // A trap (a bug, or memory that ran out) leaves the module unusable: the next request gets a
    // fresh one. The answer says the module broke off (`broken`), not that it found no fugue:
    // the request may succeed another time.
    fuge = Fuge.fetch("fuge_web.wasm");
    self.postMessage({ id, answer: { ok: false, error: `${error}`, broken: true } });
  }
};
