import { Watch } from "convex/react";
import ws from "ws";
const nodeWebSocket = ws as unknown as typeof WebSocket;

export const opts = {
  webSocketConstructor: nodeWebSocket,
} as const;

/**
 * Given a query watch, create a Promise that is resolved when we receive
 * a result matching a predicate.
 *
 * @param watch - A watch from `ConvexReactClient:watchQuery`
 * @param predicate - A function to check for a matching result.
 * @returns A `Promise` of the first matching result.
 */
export function awaitQueryResult<T>(
  watch: Watch<T>,
  predicate: (result: T) => boolean,
): Promise<T> {
  let onResolve: ((v: T) => void) | null = null;
  const result = new Promise<T>((resolve) => {
    onResolve = resolve;
  });
  const unsub = watch.onUpdate(() => {
    const result = watch.localQueryResult()!;
    if (predicate(result)) {
      unsub();
      onResolve!(result);
    }
  });
  return result;
}

export class AsyncQueue<T> {
  queue: T[];
  queuedShifts: ((v: T) => void)[];
  constructor() {
    this.queue = [];
    this.queuedShifts = [];
  }
  async shift(): Promise<T> {
    if (this.queue.length) return this.queue.shift()!;
    const { promise, resolve } = defer<T>();
    this.queuedShifts.push(resolve);
    return await promise;
  }
  push(v: T): void {
    if (this.queuedShifts.length) {
      const resolve = this.queuedShifts.shift()!;
      resolve(v);
    } else {
      this.queue.push(v);
    }
  }
}

export interface Deferred<T> {
  resolve: (value: T | PromiseLike<T>) => void;
  reject: (reason: unknown) => void;
  promise: Promise<T>;
  resolved: boolean;
}

/**
 * A convenient wrapper for constructing a promise and a resolve function.
 * This is similar to Promise.withResolvers
 * https://tc39.es/proposal-promise-with-resolvers/
 * but is named more similarly to TypeScript's internal utility
 * https://github.com/microsoft/TypeScript/blob/1d96eb489e559f4f61522edb3c8b5987bbe948af/src/harness/util.ts#L121
 */
export function defer<T = void>(name?: string): Deferred<T> {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason: unknown) => void;
  const ret: Deferred<T> = {
    resolve,
    reject,
    resolved: false,
    promise: undefined!,
  };
  const promise = new Promise<T>((_resolve, _reject) => {
    ret.resolve = (v: T | PromiseLike<T>) => {
      const value = _resolve(v);
      ret.resolved = true;
      return value;
    };
    ret.reject = _reject;
  });
  ret.promise = promise;
  if (name) {
    Object.defineProperty(ret.resolve, "name", { value: name + " resolve" });
    Object.defineProperty(ret.reject, "name", { value: name + " reject" });
  }
  return ret;
}
