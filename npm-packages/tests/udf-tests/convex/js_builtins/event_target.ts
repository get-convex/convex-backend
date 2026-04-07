// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";
import { query } from "../_generated/server";

export default query(async () => {
  return await wrapInTests({
    addEventListenerTest,
    constructedEventTargetCanBeUsedAsExpected,
    anEventTargetCanBeSubclassed,
    removingNullEventListenerShouldSucceed,
    constructedEventTargetUseObjectPrototype,
    toStringShouldBeWebCompatible,
    dispatchEventShouldNotThrowError,
    eventTargetThisShouldDefaultToGlobal,
    eventTargetShouldAcceptEventListenerObject,
    eventTargetShouldAcceptAsyncFunction,
    eventTargetShouldAcceptAsyncFunctionForEventListenerObject,
    eventTargetDispatchShouldSetTargetNoListener,
    eventTargetDispatchShouldSetTargetInListener,
    eventTargetDispatchShouldFireCurrentListenersOnly,
    // eventTargetAddEventListenerGlobalAbort,
  });
});
function addEventListenerTest() {
  const document = new EventTarget();

  assert.strictEqual(document.addEventListener("x", null, false), undefined);
  assert.strictEqual(document.addEventListener("x", null, true), undefined);
  assert.strictEqual(document.addEventListener("x", null), undefined);
}

function constructedEventTargetCanBeUsedAsExpected() {
  const target = new EventTarget();
  const event = new Event("foo", { bubbles: true, cancelable: false });
  let callCount = 0;

  const listener = (e: Event) => {
    assert.strictEqual(e, event);
    ++callCount;
  };

  target.addEventListener("foo", listener);

  target.dispatchEvent(event);
  assert.strictEqual(callCount, 1);

  target.dispatchEvent(event);
  assert.strictEqual(callCount, 2);

  target.removeEventListener("foo", listener);
  target.dispatchEvent(event);
  assert.strictEqual(callCount, 2);
}

function anEventTargetCanBeSubclassed() {
  class NicerEventTarget extends EventTarget {
    on(
      type: string,
      callback: ((e: Event) => void) | null,
      options?: AddEventListenerOptions,
    ) {
      this.addEventListener(type, callback, options);
    }

    off(
      type: string,
      callback: ((e: Event) => void) | null,
      options?: EventListenerOptions,
    ) {
      this.removeEventListener(type, callback, options);
    }
  }

  const target = new NicerEventTarget();
  new Event("foo", { bubbles: true, cancelable: false });
  let callCount = 0;

  const listener = () => {
    ++callCount;
  };

  target.on("foo", listener);
  assert.strictEqual(callCount, 0);

  target.off("foo", listener);
  assert.strictEqual(callCount, 0);
}

function removingNullEventListenerShouldSucceed() {
  const document = new EventTarget();
  assert.strictEqual(document.removeEventListener("x", null, false), undefined);
  assert.strictEqual(document.removeEventListener("x", null, true), undefined);
  assert.strictEqual(document.removeEventListener("x", null), undefined);
}

function constructedEventTargetUseObjectPrototype() {
  const target = new EventTarget();
  const event = new Event("toString", { bubbles: true, cancelable: false });
  let callCount = 0;

  const listener = (e: Event) => {
    assert.strictEqual(e, event);
    ++callCount;
  };

  target.addEventListener("toString", listener);

  target.dispatchEvent(event);
  assert.strictEqual(callCount, 1);

  target.dispatchEvent(event);
  assert.strictEqual(callCount, 2);

  target.removeEventListener("toString", listener);
  target.dispatchEvent(event);
  assert.strictEqual(callCount, 2);
}

function toStringShouldBeWebCompatible() {
  const target = new EventTarget();
  assert.strictEqual(target.toString(), "[object EventTarget]");
}

function dispatchEventShouldNotThrowError() {
  let hasThrown = false;

  try {
    const target = new EventTarget();
    const event = new Event("hasOwnProperty", {
      bubbles: true,
      cancelable: false,
    });
    const listener = () => {
      // empty
    };
    target.addEventListener("hasOwnProperty", listener);
    target.dispatchEvent(event);
  } catch {
    hasThrown = true;
  }

  assert.strictEqual(hasThrown, false);
}

function eventTargetThisShouldDefaultToGlobal() {
  const { addEventListener, dispatchEvent, removeEventListener } =
    EventTarget.prototype;
  let n = 1;
  const event = new Event("hello");
  const listener = () => {
    n = 2;
  };

  addEventListener("hello", listener);
  globalThis.dispatchEvent(event);
  assert.strictEqual(n, 2);
  n = 1;
  removeEventListener("hello", listener);
  globalThis.dispatchEvent(event);
  assert.strictEqual(n, 1);

  globalThis.addEventListener("hello", listener);
  dispatchEvent(event);
  assert.strictEqual(n, 2);
  n = 1;
  globalThis.removeEventListener("hello", listener);
  dispatchEvent(event);
  assert.strictEqual(n, 1);
}

function eventTargetShouldAcceptEventListenerObject() {
  const target = new EventTarget();
  const event = new Event("foo", { bubbles: true, cancelable: false });
  let callCount = 0;

  const listener = {
    handleEvent(e: Event) {
      assert.strictEqual(e, event);
      ++callCount;
    },
  };

  target.addEventListener("foo", listener);

  target.dispatchEvent(event);
  assert.strictEqual(callCount, 1);

  target.dispatchEvent(event);
  assert.strictEqual(callCount, 2);

  target.removeEventListener("foo", listener);
  target.dispatchEvent(event);
  assert.strictEqual(callCount, 2);
}

function eventTargetShouldAcceptAsyncFunction() {
  const target = new EventTarget();
  const event = new Event("foo", { bubbles: true, cancelable: false });
  let callCount = 0;

  const listener = (e: Event) => {
    assert.strictEqual(e, event);
    ++callCount;
  };

  target.addEventListener("foo", listener);

  target.dispatchEvent(event);
  assert.strictEqual(callCount, 1);

  target.dispatchEvent(event);
  assert.strictEqual(callCount, 2);

  target.removeEventListener("foo", listener);
  target.dispatchEvent(event);
  assert.strictEqual(callCount, 2);
}

function eventTargetShouldAcceptAsyncFunctionForEventListenerObject() {
  const target = new EventTarget();
  const event = new Event("foo", { bubbles: true, cancelable: false });
  let callCount = 0;

  const listener = {
    handleEvent(e: Event) {
      assert.strictEqual(e, event);
      ++callCount;
    },
  };

  target.addEventListener("foo", listener);

  target.dispatchEvent(event);
  assert.strictEqual(callCount, 1);

  target.dispatchEvent(event);
  assert.strictEqual(callCount, 2);

  target.removeEventListener("foo", listener);
  target.dispatchEvent(event);
  assert.strictEqual(callCount, 2);
}

function eventTargetDispatchShouldSetTargetNoListener() {
  const target = new EventTarget();
  const event = new Event("foo");
  assert.strictEqual(event.target, null);
  target.dispatchEvent(event);
  assert.strictEqual(event.target, target);
}

function eventTargetDispatchShouldSetTargetInListener() {
  const target = new EventTarget();
  const event = new Event("foo");
  assert.strictEqual(event.target, null);
  let called = false;
  target.addEventListener("foo", (e) => {
    assert.strictEqual(e.target, target);
    called = true;
  });
  target.dispatchEvent(event);
  assert.strictEqual(called, true);
}

function eventTargetDispatchShouldFireCurrentListenersOnly() {
  const target = new EventTarget();
  const event = new Event("foo");
  let callCount = 0;
  target.addEventListener("foo", () => {
    ++callCount;
    target.addEventListener("foo", () => {
      ++callCount;
    });
  });
  target.dispatchEvent(event);
  assert.strictEqual(callCount, 1);
}

// AbortController not defined.
// function eventTargetAddEventListenerGlobalAbort(): Promise<void> {
//   return new Promise((resolve) => {
//     const c = new AbortController();

//     c.signal.addEventListener("abort", () => resolve());
//     addEventListener("test", () => {}, { signal: c.signal });
//     c.abort();
//   });
// }
