// Minimal `node:async_hooks` stand-in for the Wasm guest.
//
// The guest has no async context tracking, so this keeps a single current value
// per storage and restores it when the callback settles rather than when it
// returns. That is correct as long as two `run()` calls for the same storage are
// not interleaved, which holds for Felix's one-request-per-instance model.
export class AsyncLocalStorage {
  #store = undefined;

  getStore() {
    return this.#store;
  }

  enterWith(store) {
    this.#store = store;
  }

  run(store, callback, ...args) {
    const previous = this.#store;
    this.#store = store;

    let result;
    try {
      result = callback(...args);
    } catch (error) {
      this.#store = previous;
      throw error;
    }

    if (result !== null && typeof result?.then === "function") {
      return result.then(
        (value) => {
          this.#store = previous;
          return value;
        },
        (error) => {
          this.#store = previous;
          throw error;
        },
      );
    }

    this.#store = previous;
    return result;
  }

  exit(callback, ...args) {
    return this.run(undefined, callback, ...args);
  }

  disable() {
    this.#store = undefined;
  }
}

export class AsyncResource {
  runInAsyncScope(callback, thisArg, ...args) {
    return callback.apply(thisArg, args);
  }
}

export function executionAsyncId() {
  return 0;
}

export function triggerAsyncId() {
  return 0;
}

export default {
  AsyncLocalStorage,
  AsyncResource,
  executionAsyncId,
  triggerAsyncId,
};
