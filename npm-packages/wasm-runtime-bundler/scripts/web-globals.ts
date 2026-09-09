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
