// The WebAssembly module of fux (crates/fux-web): a JSON request in, a JSON answer out.
// Runs in browsers, web workers, Node.js and Deno.

/** The first bytes of a gzip file. */
const GZIP = [0x1f, 0x8b];
/** The first bytes of a WebAssembly module, `\0asm`. */
const WASM = [0x00, 0x61, 0x73, 0x6d];

/** Whether `bytes` begin with `magic`. */
function begins(bytes, magic) {
  return magic.every((byte, at) => bytes[at] === byte);
}

/** The bytes at `url`; an error if the server does not send them. */
async function download(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${url}: ${response.status} ${response.statusText}`);
  }
  return new Uint8Array(await response.arrayBuffer());
}

/**
 * The bytes of the module from its gzip copy at `url`. A server that sends the copy with
 * `Content-Encoding: gzip` has the browser decompress it on the way: then they are the module's
 * already.
 */
async function unpacked(url) {
  const bytes = await download(url);
  if (begins(bytes, WASM)) return bytes;
  if (!begins(bytes, GZIP)) throw new Error(`${url}: neither gzip nor WebAssembly`);
  const stream = new Response(bytes).body.pipeThrough(new DecompressionStream("gzip"));
  return new Response(stream).arrayBuffer();
}

export class Fux {
  /** Instantiates the module from its bytes. */
  static async load(bytes) {
    const { instance } = await WebAssembly.instantiate(bytes, {});
    return new Fux(instance);
  }

  /**
   * Fetches and instantiates the module at `url`. The site's server sends `.wasm` files
   * uncompressed, so where the runtime decompresses gzip, this fetches the copy `url.gz` that
   * tools/build_web.sh writes beside the module, a third of its size, and decompresses it; if
   * that fails in any way (no copy, a broken one, one that is no module), it fetches `url` itself.
   */
  static async fetch(url) {
    if (typeof DecompressionStream === "function") {
      try {
        return await Fux.load(await unpacked(`${url}.gz`));
      } catch {
        // The module as it is, below.
      }
    }
    return Fux.load(await download(url));
  }

  constructor(instance) {
    this.exports = instance.exports;
    this.encoder = new TextEncoder();
    this.decoder = new TextDecoder();
  }

  /** Answers a request such as `{command: "cp", cf: "fux:dorian", species: 2, seed: 1}`. */
  ask(request) {
    const bytes = this.encoder.encode(JSON.stringify(request));
    const at = this.exports.fux_input(bytes.length);
    // The memory may grow during a call, so every view is taken right before its use.
    new Uint8Array(this.exports.memory.buffer, at, bytes.length).set(bytes);
    const length = this.exports.fux_run();
    const answer = new Uint8Array(this.exports.memory.buffer, this.exports.fux_output(), length);
    return JSON.parse(this.decoder.decode(answer));
  }
}
