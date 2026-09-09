// The entry point for the guest's web-API globals, bundled by
// `bundle-web-globals.mjs` and evaluated by the guest before a deployment's
// modules are.
//
// These come from `udf-runtime` — the same sources the V8 runtime installs —
// so the two runtimes agree on behavior rather than each carrying its own
// implementation. Only the setups that need nothing from the host are here;
// the rest reach for syscalls that are unavailable while a bundle evaluates.
import { setupWeakRefs } from "udf-runtime/src/00_weakref";
import { setupEvent } from "udf-runtime/src/02_event";
import { setupStreams } from "udf-runtime/src/06_streams";

setupWeakRefs(globalThis);
setupEvent(globalThis);
setupStreams(globalThis);
