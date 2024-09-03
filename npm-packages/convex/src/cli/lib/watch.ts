import chokidar from "chokidar";
import path from "path";
import { Observations, RecordingFs, WatchEvent } from "../../bundler/fs.js";
import {
  Context,
  ErrorType,
  logFailure,
  logWarning,
} from "../../bundler/context.js";
import * as Sentry from "@sentry/node";
import { Ora } from "ora";

export class Watcher {
  private watch: chokidar.FSWatcher;
  private readyCb: Promise<void>;

  private bufferedEvents: WatchEvent[];
  private waiters: (() => void)[];

  constructor(observations: Observations) {
    this.bufferedEvents = [];
    this.waiters = [];

    const watch = chokidar.watch(observations.paths(), { persistent: true });
    watch.on("all", (eventName, eventPath) => {
      const absPath = path.resolve(eventPath);
      this.bufferedEvents.push({ name: eventName, absPath });
      for (const waiter of drain(this.waiters)) {
        waiter();
      }
    });
    this.readyCb = new Promise<void>((resolve) => {
      watch.on("ready", () => resolve());
    });
    this.watch = watch;
  }

  update(observations: Observations) {
    const watchedDirs = new Set(Object.keys(this.watch.getWatched()));
    for (const newPath of observations.paths()) {
      if (!this.isWatched(watchedDirs, newPath)) {
        this.watch.add(newPath);
      }
    }
  }

  isWatched(watchedDirs: Set<string>, observedPath: string) {
    // Walk over all of path's parents (inclusively) to see if any of them are in the watch set.
    // This function assumes that Chokidar recursively watches all directories, which is
    // definitely true on Mac with its FSEvents-based watcher.
    // TODO (CX-2151): Verify this condition on Windows and Linux.
    let curPath = observedPath;
    while (true) {
      const parsed = path.parse(curPath);

      // TODO(CX-2152): Check to see if this condition for walking parents works on Windows.
      if (parsed.dir === curPath) {
        break;
      }
      if (watchedDirs.has(curPath)) {
        return true;
      }
      curPath = parsed.dir;
    }
    return false;
  }

  async ready(): Promise<void> {
    await this.readyCb;
  }

  async waitForEvent(): Promise<void> {
    while (this.bufferedEvents.length === 0) {
      const newEvent = new Promise<void>((resolve) => {
        this.waiters.push(resolve);
      });
      await newEvent;
    }
  }

  drainEvents(): WatchEvent[] {
    return drain(this.bufferedEvents);
  }

  async close() {
    await this.watch.close();
  }
}
function drain<T>(l: T[]): T[] {
  return l.splice(0, l.length);
}

export class Crash extends Error {
  errorType?: ErrorType;

  constructor(errorType?: ErrorType, err?: any) {
    super(err?.message);
    this.errorType = errorType;
  }
}

export class WatchContext implements Context {
  fs: RecordingFs;
  deprecationMessagePrinted: boolean;
  spinner: Ora | undefined;

  constructor(traceEvents: boolean) {
    this.fs = new RecordingFs(traceEvents);
    this.deprecationMessagePrinted = false;
  }

  crash(args: {
    exitCode: number;
    errorType?: ErrorType;
    errForSentry?: any;
    printedMessage: string | null;
    messageLevel?: "error" | "warning";
  }): Promise<never> {
    if (args.errForSentry) {
      Sentry.captureException(args.errForSentry);
    }
    if (args.printedMessage !== null) {
      if (args.messageLevel === "warning") {
        logWarning(this, args.printedMessage);
      } else {
        logFailure(this, args.printedMessage);
      }
    }
    // Okay to throw here. We've wrapped it in a Crash that we'll catch later.
    // eslint-disable-next-line no-restricted-syntax
    throw new Crash(args.errorType, args.errForSentry);
  }
}
