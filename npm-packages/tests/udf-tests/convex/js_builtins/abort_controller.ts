// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";
import { query } from "../_generated/server";

export default query(async () => {
  return await wrapInTests({
    basicAbortController,
    signalCallsOnabort,
    signalEventListener,
    onlyAbortsOnce,
    controllerHasProperToString,
    // abortReason,
  });
});

function basicAbortController() {
  const controller = new AbortController();
  assert(controller);
  const { signal } = controller;
  assert(signal);
  assert.strictEqual(signal.aborted, false);
  controller.abort();
  assert.strictEqual(signal.aborted, true);
}

function signalCallsOnabort() {
  const controller = new AbortController();
  const { signal } = controller;
  let called = false;
  signal.onabort = (evt) => {
    assert(evt);
    assert.strictEqual(evt.type, "abort");
    called = true;
  };
  controller.abort();
  assert(called);
}

function signalEventListener() {
  const controller = new AbortController();
  const { signal } = controller;
  let called = false;
  signal.addEventListener("abort", function (ev) {
    assert(this === signal);
    assert.strictEqual(ev.type, "abort");
    called = true;
  });
  controller.abort();
  assert(called);
}

function onlyAbortsOnce() {
  const controller = new AbortController();
  const { signal } = controller;
  let called = 0;
  signal.addEventListener("abort", () => called++);
  signal.onabort = () => {
    called++;
  };
  controller.abort();
  assert.strictEqual(called, 2);
  // TODO: the AbortController polyfill doesn't have the aborts-once guarantee.
  // controller.abort();
  // assert.strictEqual(called, 2);
}

function controllerHasProperToString() {
  const actual = Object.prototype.toString.call(new AbortController());
  assert.strictEqual(actual, "[object AbortController]");
}

// TODO: the AbortSignal polyfill doesn't implement abort().
// function abortReason() {
//   const signal = AbortSignal.abort("hey!");
//   assert.strictEqual(signal.aborted, true);
//   assert.strictEqual(signal.reason, "hey!");
// }
