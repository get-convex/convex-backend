import { setupURL } from "./00_url.js";
import { setupCrypto } from "./00_crypto.js";
import { setupDOMException } from "./01_dom_exception.js";
import { setupConsole } from "./02_console";
import { setupEvent } from "./02_event";
import { setupTimers } from "./02_timers.js";
import { setupAbortSignal } from "./03_abort_signal.js";
import { setupStreams } from "./06_streams.js";
import { setupTextEncoding } from "./08_text_encoding.js";
import { setupBlob } from "./09_file.js";
import { setupHeaders } from "./20_headers.js";
import { setupFormData } from "./21_formdata.js";
import { requestFromConvexJson, setupRequest } from "./23_request.js";
import { convexJsonFromResponse, setupResponse } from "./23_response.js";
import { setupFetch } from "./26_fetch.js";
import { setupSourceMapping } from "./errors.js";
import { throwUncatchableDeveloperError } from "./helpers.js";
import { getBlob, getResponse, storeBlob, storeRequest } from "./storage.js";
import { performOp } from "udf-syscall-ffi";
import { setupStructuredClone } from "./02_structured_clone.js";

/**
 * Set up the global object for a UDF context with deterministic Convex APIs.
 *
 * This initializes certain JS globals, and patches existing globals such as the
 * `Math.random` function, the `Date` object, and the `console` object.
 */
export function setup(global: any) {
  setupSourceMapping();
  setupDate(global);
  // NB: It's important we call into `setupMisc` before the other setup functions
  // since those may call into 3rd party libraries we bundle, which may then
  // retain references to globals we modify, like `Date` or `FinalizationRegistry`.
  setupMisc(global);

  // These need to be set up in order of the numbers in their filenames (taken
  // from Deno) since later ones depend on the earlier ones.
  setupURL(global);
  setupCrypto(global);
  setupDOMException(global);
  setupConsole(global);
  setupEvent(global);
  setupStructuredClone(global);
  setupTimers(global);
  setupAbortSignal(global);
  setupStreams(global);
  setupTextEncoding(global);
  setupBlob(global);
  setupHeaders(global);
  setupFormData(global);
  setupRequest(global);
  setupResponse(global);
  setupFetch(global);

  global.Convex.jsSyscall = (op: string, args: Record<string, any>) => {
    switch (op) {
      case "requestFromConvexJson":
        return requestFromConvexJson(args as any);
      case "convexJsonFromResponse":
        return convexJsonFromResponse(args as any);
      case "storage/storeBlob":
        return storeBlob(args as any);
      case "storage/getBlob":
        return getBlob(args as any);
      // Deprecated APIs, used prior to Convex 0.13.0
      case "storage/storeFile":
        return storeRequest(args as any);
      case "storage/getFile":
        return getResponse(args as any);
      default:
        return throwUncatchableDeveloperError(`Unknown JS syscall: ${op}`);
    }
  };
}

function setupDate(global) {
  // Patch `Date` with our own version that returns a consistent result.
  // We only patch the paths that refer to the current time because for all
  // other paths, we have already ensured determinism by pinning the system
  // time to UTC via the TZ environment variable.
  const originalDate = global.Date;
  delete global.Date;

  function Date(...args) {
    // `Date()` was called directly, not as a constructor.
    if (!(this instanceof Date)) {
      const date = new (Date as any)();
      return date.toString();
    }
    if (args.length === 0) {
      const unixTsMs = Date.now();
      return new originalDate(unixTsMs);
    }
    return new originalDate(...args);
  }
  Date.now = function () {
    return performOp("now");
  };
  Date.parse = originalDate.parse;
  Date.UTC = originalDate.UTC;
  Date.prototype = originalDate.prototype;
  Date.prototype.constructor = Date;

  global.Date = Date;
}

function setupMisc(global) {
  // Patch `Math.random` with our own deterministic RNG.
  delete global.Math.random;
  global.Math.random = function () {
    return performOp("random");
  };

  // Proxy process.env. with a syscall that gets the environment variable's value.
  const handler = {
    get(_target: any, prop: any, receiver: any) {
      if (typeof prop === "string") {
        const value = performOp("environmentVariables/get", prop);
        // Map null to undefined in case other libraries check explicitly for undefined
        // Note serde Value enum in rust does not have an undefined variant, only null.
        if (value === null) {
          if (prop === "inspect") {
            return () => "[process.env]";
          }
          return undefined;
        }
        return value;
      } else {
        return Reflect.get(_target, prop, receiver);
      }
    },
  };
  const env = new Proxy({}, handler);
  global.process = { env };

  // defined in browsers and required by the WinterCG Minimum Common Web Platform API draft
  // https://common-min-api.proposal.wintercg.org/
  global.self = global;

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
