// The entry point for the guest's web-API globals, bundled by
// `bundle-web-globals.mjs` and evaluated by the guest before a deployment's
// modules are.
//
// These come from `udf-runtime`, the same sources the V8 runtime installs.
import { setupDate } from "udf-runtime/src/00_date.js";
import { setupWeakRefs } from "udf-runtime/src/00_weakref";
import { setupEvent } from "udf-runtime/src/02_event";
import { setupStreams } from "udf-runtime/src/06_streams";

setupDate(globalThis);
setupWeakRefs(globalThis);
setupEvent(globalThis);
setupStreams(globalThis);

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

// QuickJS ships no `Intl`; the Temporal binding covers only `Temporal`.
globalThis.Intl = Object.fromEntries(
  [
    "Collator",
    "DateTimeFormat",
    "DisplayNames",
    "DurationFormat",
    "ListFormat",
    "Locale",
    "NumberFormat",
    "PluralRules",
    "RelativeTimeFormat",
    "Segmenter",
  ].map((name) => [name, unsupportedClass(`Intl.${name}`)]),
) as unknown as typeof Intl;
Object.assign(globalThis.Intl, {
  getCanonicalLocales: unsupportedFunction("Intl.getCanonicalLocales"),
  supportedValuesOf: unsupportedFunction("Intl.supportedValuesOf"),
});

globalThis.setTimeout = unsupportedFunction("setTimeout") as any;
globalThis.setInterval = unsupportedFunction("setInterval") as any;
// Nothing can be scheduled, so there is never a timer to clear.
globalThis.clearTimeout = () => {};
globalThis.clearInterval = () => {};

globalThis.structuredClone = unsupportedFunction("structuredClone") as any;

for (const name of [
  "AbortController",
  "Blob",
  "File",
  "FormData",
  "Headers",
  "Request",
  "Response",
]) {
  (globalThis as any)[name] = unsupportedClass(name);
}

globalThis.WebAssembly = {
  compile: unsupportedFunction("WebAssembly.compile"),
  instantiate: unsupportedFunction("WebAssembly.instantiate"),
  validate: unsupportedFunction("WebAssembly.validate"),
  Module: unsupportedClass("WebAssembly.Module"),
  Instance: unsupportedClass("WebAssembly.Instance"),
  Memory: unsupportedClass("WebAssembly.Memory"),
  Table: unsupportedClass("WebAssembly.Table"),
} as unknown as typeof WebAssembly;
