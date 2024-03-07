// This file is doing all kinds of wonky things so just turn off eslint
/* eslint-disable */
import { query } from "./_generated/server";

// Based on https://developer.mozilla.org/en-US/docs/Web/API
export const globals = query(() => {
  if (typeof global !== "undefined") {
    throw new Error("global defined");
  }
  if (typeof self === "undefined") {
    throw new Error("self not defined");
  }
  if (typeof self != "object" || self.Object !== Object) {
    // This check is used by https://unpkg.com/browse/lodash.clonedeep@4.5.0/index.js#L83
    throw new Error(
      "Object on self (used to check that it's the global object) doesn't work",
    );
  }
});

const globalDate = new Date();
const globalRand = Math.random();

export const getDate = query(async () => {
  return new Date().getTime();
});

export const getDateNow = query(async () => {
  return Date.now();
});

export const getGlobalDate = query(async () => {
  return globalDate.getTime();
});

export const getRandom = query(async () => {
  return Math.random();
});

export const getGlobalRandom = query(async () => {
  return globalRand;
});

export const createFinalizationRegistry = query(async () => {
  const registry = new FinalizationRegistry((value) => {
    // This callback will never actually be called.
    throw new Error("FinalizationRegistry callback called");
  });
  const obj = {};
  registry.register(obj, "hello", obj);
  registry.unregister(obj);
});

export const createWeakRef = query(async () => {
  const s = {};
  const ref = new WeakRef(s);
  if (ref.deref() !== s) {
    throw new Error("WeakRef returned a different value");
  }
});
