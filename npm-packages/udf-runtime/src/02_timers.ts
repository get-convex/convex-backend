// Implementation of Timer spec defined at
// https://html.spec.whatwg.org/multipage/timers-and-user-prompts.html

import { throwNotImplementedMethodError } from "./helpers";
import { performAsyncOp } from "./syscall";

const ACTIVE_TIMERS = new Map();

const setTimeout = (
  handler: (...args: any[]) => void,
  timeoutMs: number,
  ...args: any[]
): number => {
  return timerInitialization("setTimeout", handler, timeoutMs, args, false);
};

const setInterval = (
  handler: (...args: any[]) => void,
  timeoutMs: number,
  ...args: any[]
): number => {
  return timerInitialization("setInterval", handler, timeoutMs, args, true);
};

const clearTimeout = (id: number) => {
  ACTIVE_TIMERS.delete(id);
};
const clearInterval = (id: number) => {
  ACTIVE_TIMERS.delete(id);
};

const timerInitialization = (
  name: string,
  handler: (...args: any[]) => void,
  timeoutMs: number,
  args: any[],
  repeat: boolean,
  previousId?: number,
): number => {
  if (typeof handler === "string") {
    throwNotImplementedMethodError("code string argument", name);
  }
  const id = previousId ?? unusedTimerId();
  // TODO(CX-4532) calculate nesting level and use it to lower bound timeout.
  timeoutMs = timeoutMs ? Number(timeoutMs) : 0;
  if (timeoutMs < 0) {
    timeoutMs = 0;
  }
  const task = () => {
    if (!ACTIVE_TIMERS.has(id)) {
      return;
    }
    handler(...args);
    if (!ACTIVE_TIMERS.has(id)) {
      return;
    }
    if (repeat) {
      timerInitialization(name, handler, timeoutMs, args, true, id);
    } else {
      ACTIVE_TIMERS.delete(id);
    }
  };
  runAfterTimeout(name, timeoutMs, task, id);
  return id;
};

const runAfterTimeout = (
  name: string,
  timeoutMs: number,
  task: () => void,
  timerKey: number,
) => {
  const startTime = Date.now();
  ACTIVE_TIMERS.set(timerKey, startTime + timeoutMs);
  void performAsyncOp("sleep", name, timeoutMs).then(() => {
    // TODO(CX-4534) Wait until any invocations of this algorithm,
    // that started before this one, and whose milliseconds is equal to or less
    // than this one's, have completed.

    task();
  });
};

const unusedTimerId = () => {
  while (true) {
    const id =
      1 + Math.floor(Math.random() * Math.max(1000, ACTIVE_TIMERS.size * 2));
    if (!ACTIVE_TIMERS.has(id)) {
      return id;
    }
  }
};

// Implementation of https://html.spec.whatwg.org/multipage/timers-and-user-prompts.html#dom-queuemicrotask
// V8 drains its native microtask queue at the isolate's explicit checkpoints,
// the same way Promise reaction jobs run, so unlike the timers above this
// needs no syscall and is deterministic in every environment. Dispatching via
// a resolved Promise means an exception thrown by the callback surfaces as an
// unhandled rejection rather than a reported exception; both fail the UDF.
const queueMicrotask = (callback: (...args: any[]) => void) => {
  if (typeof callback !== "function") {
    throw new TypeError(
      "The callback provided as parameter 1 is not a function",
    );
  }
  void Promise.resolve().then(() => callback());
};

export const setupTimers = (global) => {
  global.setTimeout = setTimeout;
  global.setInterval = setInterval;
  global.clearTimeout = clearTimeout;
  global.clearInterval = clearInterval;
  global.queueMicrotask = queueMicrotask;
};
