// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";
import { query } from "../_generated/server";

export default query(async () => {
  return await wrapInTests({
    eventInitializedWithType,
    eventInitializedWithTypeAndDict,
    eventComposedPathSuccess,
    eventStopPropagationSuccess,
    eventStopImmediatePropagationSuccess,
    eventPreventDefaultSuccess,
    eventInitializedWithNonStringType,
    eventIsTrusted,
  });
});

function eventInitializedWithType() {
  const type = "click";
  const event = new Event(type);

  assert.strictEqual(event.isTrusted, false);
  assert.strictEqual(event.target, null);
  assert.strictEqual(event.currentTarget, null);
  assert.strictEqual(event.type, "click");
  assert.strictEqual(event.bubbles, false);
  assert.strictEqual(event.cancelable, false);
}

function eventInitializedWithTypeAndDict() {
  const init = "submit";
  const eventInit = { bubbles: true, cancelable: true } as EventInit;
  const event = new Event(init, eventInit);

  assert.strictEqual(event.isTrusted, false);
  assert.strictEqual(event.target, null);
  assert.strictEqual(event.currentTarget, null);
  assert.strictEqual(event.type, "submit");
  assert.strictEqual(event.bubbles, true);
  assert.strictEqual(event.cancelable, true);
}

function eventComposedPathSuccess() {
  const type = "click";
  const event = new Event(type);
  const composedPath = event.composedPath();

  assert.deepEqual(composedPath, []);
}

function eventStopPropagationSuccess() {
  const type = "click";
  const event = new Event(type);

  assert.strictEqual(event.cancelBubble, false);
  event.stopPropagation();
  assert.strictEqual(event.cancelBubble, true);
}

function eventStopImmediatePropagationSuccess() {
  const type = "click";
  const event = new Event(type);

  assert.strictEqual(event.cancelBubble, false);
  event.stopImmediatePropagation();
  assert.strictEqual(event.cancelBubble, true);
}

function eventPreventDefaultSuccess() {
  const type = "click";
  const event = new Event(type);

  assert.strictEqual(event.defaultPrevented, false);
  event.preventDefault();
  assert.strictEqual(event.defaultPrevented, false);

  const eventInit = { bubbles: true, cancelable: true } as EventInit;
  const cancelableEvent = new Event(type, eventInit);
  assert.strictEqual(cancelableEvent.defaultPrevented, false);
  cancelableEvent.preventDefault();
  assert.strictEqual(cancelableEvent.defaultPrevented, true);
}

function eventInitializedWithNonStringType() {
  // deno-lint-ignore no-explicit-any
  const type: any = undefined;
  const event = new Event(type);

  assert.strictEqual(event.isTrusted, false);
  assert.strictEqual(event.target, null);
  assert.strictEqual(event.currentTarget, null);
  assert.strictEqual(event.type, "undefined");
  assert.strictEqual(event.bubbles, false);
  assert.strictEqual(event.cancelable, false);
}

// ref https://github.com/web-platform-tests/wpt/blob/master/dom/events/Event-isTrusted.any.js
function eventIsTrusted() {
  const desc1 = Object.getOwnPropertyDescriptor(new Event("x"), "isTrusted");
  assert(desc1);
  assert.strictEqual(typeof desc1.get, "function");

  const desc2 = Object.getOwnPropertyDescriptor(new Event("x"), "isTrusted");
  assert(desc2);
  assert.strictEqual(typeof desc2!.get, "function");

  assert.strictEqual(desc1!.get, desc2!.get);
}
