// The pool of web workers that sets the parts of a mass side by side (phase 14c,
// docs/design/phase-14/spec.md § 4.5). Each worker runs the WebAssembly module (worker.js) and
// sets one part at a time. The part that gives the head motif goes first, then the longest, then
// whole movements one after the other (`order`); the parts that open with the motif wait for it.
// A cancel terminates the busy workers at once (the module cannot be interrupted inside a
// search), and the next run starts fresh workers. Without the page's elements, so that Node.js
// and Deno test it with workers of their own.

/**
 * The number of workers for a device: at most four, one core left to the page, about 1.5 GiB of
 * memory each (a part takes up to 0.5 GiB in four voices, 1 GiB in five); at least one.
 */
export function poolSize(device = globalThis.navigator) {
  const cores = Number(device?.hardwareConcurrency) || 4;
  const memory = Number(device?.deviceMemory);
  const byMemory = memory > 0 ? Math.floor(memory / 1.5) : 4;
  return Math.max(1, Math.min(4, Math.max(1, cores - 1), byMemory));
}

/** The jobs per worker at the end of a run that go longest first (`order`), as
 * `tables::POOL_TAIL` of fux-core has it. */
const TAIL = 3;

/**
 * The jobs of a mass in the order in which `workers` workers start them, each worker taking the
 * first that is ready once it is free (K12 (a); `MassPlan::pool_order_of` of fux-core orders them
 * alike for the estimate, and `fux mass` begins its parts so with several at once): the job that
 * gives the head motif first (`source`); then the `workers − 1` longest of the others, which
 * bound the time of the whole run; then the rest movement by movement in the order of the mass,
 * so that whole movements are done early, each movement's longest first, except that the last
 * three per worker go longest first, so that the workers finish about together. `jobs` come in
 * the order of the mass, each with the index of its `movement`; of jobs with equal `seconds` the
 * one earlier in it comes first.
 */
export function order(jobs, workers) {
  const size = Math.max(1, workers);
  const place = new Map(jobs.map((job, i) => [job, i]));
  const longer = (a, b) => b.seconds - a.seconds || place.get(a) - place.get(b);
  const longest = jobs.filter((job) => !job.source).sort(longer);
  const rest = longest.splice(size - 1).sort((a, b) => a.movement - b.movement || longer(a, b));
  const tail = rest.splice(Math.max(0, rest.length - TAIL * size)).sort(longer);
  return [...jobs.filter((job) => job.source), ...longest, ...rest, ...tail];
}

/**
 * The seconds until `jobs` are done on `workers` workers as the pool runs them, each job taking
 * its `seconds`: the jobs `running` go on first, each for the seconds it still takes; then each
 * worker, once free, takes the first of `jobs` that is ready (in the order of `order`), a job
 * that `waits` for the motif once the job that gives it (`source`) is done.
 */
export function schedule(jobs, workers, running = []) {
  const free = Array(Math.max(1, workers, running.length)).fill(0);
  let motif = jobs.some((job) => job.source) ? Infinity : 0;
  running.forEach((job, i) => {
    free[i] = Math.max(0, job.seconds);
    if (job.source) motif = free[i];
  });
  const waiting = [...jobs];
  while (waiting.length > 0) {
    let worker = 0;
    for (let i = 1; i < free.length; i++) {
      if (free[i] < free[worker]) worker = i;
    }
    let time = free[worker];
    let at = waiting.findIndex((job) => !job.waits || motif <= time);
    if (at < 0) {
      // Only parts that wait for the motif are left: the worker waits for it too.
      time = Math.max(time, motif);
      at = waiting.findIndex((job) => !job.waits || motif <= time);
      if (at < 0) break;
    }
    const [job] = waiting.splice(at, 1);
    free[worker] = time + Math.max(0, job.seconds);
    if (job.source) motif = free[worker];
  }
  return Math.max(...free);
}

const now = () => globalThis.performance?.now() ?? Date.now();

/**
 * Workers that set jobs side by side. A job is `{key, seconds, request}` with `source: true`
 * where its answer gives the head motif (`answer.part.motif`) and `waits: true` where its request
 * needs it (as `motif`).
 */
export class Pool {
  #size;
  #spawn;
  #idle = [];
  /** The busy workers, each with its job and when it started. */
  #busy = new Map();
  #run = null;
  #id = 0;

  /** `size` workers at most, each made by `spawn()`: a Worker of worker.js, or one alike. */
  constructor(size, spawn) {
    this.#size = Math.max(1, size);
    this.#spawn = spawn;
  }

  get size() {
    return this.#size;
  }

  /** Whether a run is going on. */
  get busy() {
    return this.#run !== null;
  }

  /** The jobs being set now, each with when it started (performance.now()). */
  running() {
    return [...this.#busy.values()].map(({ job, started }) => ({ job, started }));
  }

  /**
   * Sets `jobs`, each worker, once free, the first of them that is ready (a mass's in the order
   * of `order`), telling `on.started(job)`, `on.finished(job, answer, seconds)` and
   * `on.failed(job, error, seconds, broken)`, `broken` where the worker broke off (an error
   * event, or a trap of its module: `answer.broken`) rather than the search failing: such a job
   * may be set another time. `motif` is the motif where it is known already (a string, or `null`
   * for none); the jobs that wait for it get it, or else the one their source gives: none where
   * the search of the source failed; where its worker broke off, they are left waiting. Resolves
   * once every job is done, or the run is cancelled, with `{cancelled, stopped, left}`: the jobs
   * a cancel stopped and those left waiting.
   */
  run(jobs, on = {}, motif = undefined) {
    if (this.#run) {
      return Promise.reject(new Error("the pool is busy"));
    }
    return new Promise((resolve) => {
      const known = motif !== undefined || !jobs.some((job) => job.source);
      this.#run = {
        waiting: [...jobs],
        on,
        resolve,
        motif: known ? (motif ?? null) : undefined,
      };
      this.#pump();
    });
  }

  /** Ends the run at once: its busy workers are terminated, the jobs not started are left. */
  cancel() {
    if (!this.#run) return;
    const stopped = [];
    for (const [worker, { job }] of this.#busy) {
      worker.terminate();
      stopped.push(job);
    }
    this.#busy.clear();
    this.#finish(true, stopped);
  }

  /** Starts what can start; ends the run when nothing runs and nothing can start. */
  #pump() {
    const run = this.#run;
    if (!run) return;
    while (this.#busy.size < this.#size) {
      const at = run.waiting.findIndex((job) => !job.waits || run.motif !== undefined);
      if (at < 0) break;
      const [job] = run.waiting.splice(at, 1);
      this.#start(job);
    }
    if (this.#busy.size === 0) {
      this.#finish(false, []);
    }
  }

  #start(job) {
    const run = this.#run;
    const worker = this.#idle.pop() ?? this.#create();
    const request = job.waits ? { ...job.request, motif: run.motif } : job.request;
    this.#busy.set(worker, { job, started: now() });
    run.on.started?.(job);
    worker.postMessage({ id: ++this.#id, request });
  }

  #create() {
    const worker = this.#spawn();
    worker.onmessage = (event) => this.#done(worker, event.data?.answer, false);
    worker.onerror = (event) => {
      event.preventDefault?.();
      const error = event.message || "worker error";
      this.#done(worker, { ok: false, error }, true);
    };
    return worker;
  }

  /** A worker answered (or broke): its job is finished or failed, and the next may start. */
  #done(worker, answer, broken) {
    const entry = this.#busy.get(worker);
    if (!entry) return;
    this.#busy.delete(worker);
    if (broken) {
      worker.terminate();
    } else {
      this.#idle.push(worker);
    }
    const run = this.#run;
    const { job, started } = entry;
    const seconds = (now() - started) / 1000;
    if (answer?.ok) {
      if (job.source) run.motif = answer.part?.motif ?? null;
      run.on.finished?.(job, answer, seconds);
    } else {
      // Without its motif the parts that wait for it take their subjects from their words; a
      // source whose worker broke off may give it another time, so they wait for that.
      const crashed = broken || answer?.broken === true;
      if (job.source && !crashed) run.motif = null;
      run.on.failed?.(job, answer?.error ?? "?", seconds, crashed);
    }
    // A handler may have cancelled the run.
    if (this.#run === run) this.#pump();
  }

  /** The run ends; the idle workers go with it, and their memory: the next run starts fresh. */
  #finish(cancelled, stopped) {
    const run = this.#run;
    this.#run = null;
    for (const worker of this.#idle) {
      worker.terminate();
    }
    this.#idle = [];
    run.resolve({ cancelled, stopped, left: run.waiting });
  }
}
