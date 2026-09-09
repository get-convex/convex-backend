import { setupURL } from "./00_url.js";
import { setupCrypto } from "./00_crypto.js";
import { setupDate } from "./00_date.js";
import { setupWeakRefs } from "./00_weakref.js";
import { setupDOMException } from "./01_dom_exception.js";
import { setupConsole } from "./02_console";
import { setupEvent } from "./02_event";
import { setupTimers } from "./02_timers.js";
import { setupAbortSignal } from "./03_abort_signal.js";
import { setupAsyncHooks } from "./04_async_hooks.js";
import { setupStreams } from "./06_streams.js";
import { setupTextEncoding } from "./08_text_encoding.js";
import { setupBlob } from "./09_file.js";
import { setupHeaders } from "./20_headers.js";
import { setupFormData } from "./21_formdata.js";
import { requestFromConvexJson, setupRequest } from "./23_request.js";
import { convexJsonFromResponse, setupResponse } from "./23_response.js";
import { setupFetch } from "./26_fetch.js";
import { setupPerformance } from "./27_performance.js";
import { setupSourceMapping } from "./errors.js";
import { throwUncatchableDeveloperError } from "./helpers.js";
import { getBlob, storeBlob } from "./storage.js";
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
  setupWeakRefs(global);

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
  setupAsyncHooks(global);
  setupStreams(global);
  setupTextEncoding(global);
  setupBlob(global);
  setupHeaders(global);
  setupFormData(global);
  setupRequest(global);
  setupResponse(global);
  setupFetch(global);
  setupPerformance(global);

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
      default:
        return throwUncatchableDeveloperError(`Unknown JS syscall: ${op}`);
    }
  };
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
}
