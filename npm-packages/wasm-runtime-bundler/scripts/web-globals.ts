// The entry point for the guest's web-API globals, bundled by
// `bundle-web-globals.mjs` and evaluated by the guest before a deployment's
// modules are.
//
// These come from `udf-runtime`, the same sources the V8 runtime installs.
import { setupDate } from "udf-runtime/src/00_date.js";
import { setupMisc } from "udf-runtime/src/00_misc";
import { setupWeakRefs } from "udf-runtime/src/00_weakref";
import { setupConsole } from "udf-runtime/src/02_console";
import { setupEvent } from "udf-runtime/src/02_event";
import { setupStreams } from "udf-runtime/src/06_streams";
import { setupBlob } from "udf-runtime/src/09_file";
import { setupAbortSignal } from "udf-runtime/src/03_abort_signal";
import { setupHeaders } from "udf-runtime/src/20_headers";
import { setupFormData } from "udf-runtime/src/21_formdata";
import { setupRequest } from "udf-runtime/src/23_request";
import { setupResponse } from "udf-runtime/src/23_response";

setupDate(globalThis);
setupMisc(globalThis);
setupWeakRefs(globalThis);
setupConsole(globalThis);
setupEvent(globalThis);
setupStreams(globalThis);
setupAbortSignal(globalThis);
setupBlob(globalThis);
setupHeaders(globalThis);
setupFormData(globalThis);
setupRequest(globalThis);
setupResponse(globalThis);

// TODO: implement actions
globalThis.fetch = async () => {
  throw new TypeError(
    "Can't use fetch() in queries and mutations. Please consider using an action. See https://docs.convex.dev/functions/actions for more details.",
  );
};

// Globals the wasm runtime does not implement but which deployment code
// commonly touches while its modules evaluate: feature-detection
// (`typeof Intl`), a top-level `setTimeout` reference, an SDK checking for
// `Request`. Each exists so that evaluation gets past the reference, and
// throws when actually used so a function that depends on it fails at the
// call site rather than returning something V8 would not.
const unsupported = (name: string) =>
  new TypeError(`${name} is not supported in the wasm runtime`);

const unsupportedFunction = (name: string) => {
  const fn = () => {
    throw unsupported(name);
  };
  Object.defineProperty(fn, "name", { value: name });
  return fn;
};

const unsupportedClass = (name: string) => {
  const cls = class {
    constructor() {
      throw unsupported(name);
    }
  };
  Object.defineProperty(cls, "name", { value: name });
  return cls;
};

globalThis.setTimeout = unsupportedFunction("setTimeout") as any;
globalThis.setInterval = unsupportedFunction("setInterval") as any;
// Nothing can be scheduled, so there is never a timer to clear.
globalThis.clearTimeout = () => {};
globalThis.clearInterval = () => {};

globalThis.structuredClone = unsupportedFunction("structuredClone") as any;

globalThis.WebAssembly = {
  compile: unsupportedFunction("WebAssembly.compile"),
  instantiate: unsupportedFunction("WebAssembly.instantiate"),
  validate: unsupportedFunction("WebAssembly.validate"),
  Module: unsupportedClass("WebAssembly.Module"),
  Instance: unsupportedClass("WebAssembly.Instance"),
  Memory: unsupportedClass("WebAssembly.Memory"),
  Table: unsupportedClass("WebAssembly.Table"),
} as unknown as typeof WebAssembly;
