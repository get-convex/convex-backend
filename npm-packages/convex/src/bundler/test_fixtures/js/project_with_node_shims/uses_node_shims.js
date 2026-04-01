import { AsyncLocalStorage, createHook } from "node:async_hooks";
import EventEmitter from "node:events";

const als = new AsyncLocalStorage();
const emitter = new EventEmitter();

export default async function usesNodeShims() {
  const hook = createHook({});
  hook.enable();
  hook.disable();

  return als.run("context", () => emitter.listenerCount("event"));
}
