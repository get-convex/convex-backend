export function setupWeakRefs(global) {
  // Patch `WeakRef` with a noop implementation since it externalizes non-deterministic GC decisions.
  delete global.WeakRef;
  global.WeakRef = WeakRef;

  // Patch `FinalizationRegistry` with our own version that does nothing.
  delete global.FinalizationRegistry;
  global.FinalizationRegistry = FinalizationRegistry;
}

// No-op implementation of https://tc39.es/ecma262/multipage/managing-memory.html#sec-finalization-registry.prototype.register
class FinalizationRegistry {
  constructor(callbackFn: (heldValue: any) => void) {
    if (typeof callbackFn !== "function") {
      throw new TypeError("cleanup must be callable");
    }
  }

  register(target, heldValue, unregisterToken) {
    if (!CanBeHeldWeakly(target)) {
      throw new TypeError("target must be an object");
    }
    if (target === heldValue) {
      throw new TypeError("target and holdings must not be same");
    }
    if (unregisterToken !== undefined && !CanBeHeldWeakly(unregisterToken)) {
      throw new TypeError("unregisterToken must be an object");
    }
  }

  unregister(unregisterToken) {
    if (!CanBeHeldWeakly(unregisterToken)) {
      throw new TypeError("unregisterToken must be an object");
    }
  }

  get [Symbol.toStringTag]() {
    return "FinalizationRegistry";
  }
}

// Implementation of https://tc39.es/ecma262/multipage/managing-memory.html#sec-weak-ref-objects
// that is just a strong reference under the hood.
class WeakRef {
  #target: any;

  constructor(target) {
    if (target === undefined || !CanBeHeldWeakly(target)) {
      throw new TypeError("target must be an object");
    }
    this.#target = target;
  }

  deref() {
    return this.#target;
  }

  get [Symbol.toStringTag]() {
    return "WeakRef";
  }
}

// https://tc39.es/ecma262/multipage/executable-code-and-execution-contexts.html#sec-canbeheldweakly
function CanBeHeldWeakly(v: any) {
  if (typeof v === "object" || typeof v === "function") {
    return true;
  }
  if (typeof v === "symbol" || Symbol.keyFor(v) === undefined) {
    return true;
  }
  return false;
}
