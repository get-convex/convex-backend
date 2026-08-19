// Globals `convex-test` expects from a Node/browser-ish environment but that the
// Felix guest does not provide. Import this before anything else so the shims
// exist by the time module bodies run: `convex-test` captures `setTimeout` at
// module scope.
//
// `setTimeout` is backed by the host sleep op, and `MessageChannel` only has to
// be enough to yield a turn, so it resolves on the microtask queue the guest
// already drains.
import { console as hostConsole, sleep } from "convex:runtime";

const globals = globalThis as Record<string, unknown>;

// `convex-test` installs its syscall proxy on Node's `global`.
if (globals.global === undefined) {
  globals.global = globalThis;
}

if (globals.console === undefined) {
  globals.console = {
    ...hostConsole,
    debug: hostConsole.log,
    info: hostConsole.log,
    trace: hostConsole.log,
  };
}

if (globals.setTimeout === undefined) {
  const cancelled = new Set<number>();
  let nextTimerId = 1;

  globals.setTimeout = (
    callback: (...args: unknown[]) => void,
    delay = 0,
    ...args: unknown[]
  ) => {
    const id = nextTimerId++;
    void sleep(Math.max(0, delay)).then(() => {
      if (!cancelled.delete(id)) {
        callback(...args);
      }
    });
    return id;
  };

  globals.clearTimeout = (id: number) => {
    cancelled.add(id);
  };

  globals.setInterval = () => {
    throw new Error("setInterval is not supported in the Felix guest");
  };
  globals.clearInterval = globals.clearTimeout;
}

if (globals.MessageChannel === undefined) {
  class MessagePortShim {
    onmessage: ((event: { data: unknown }) => void) | null = null;
    other: MessagePortShim | null = null;

    postMessage(data: unknown) {
      void Promise.resolve().then(() => this.other?.onmessage?.({ data }));
    }

    close() {}
    start() {}
  }

  globals.MessageChannel = class MessageChannelShim {
    port1 = new MessagePortShim();
    port2 = new MessagePortShim();

    constructor() {
      this.port1.other = this.port2;
      this.port2.other = this.port1;
    }
  };
}

export {};
