import { query } from "../_generated/server.js";
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";

/**
 * The goal of this test is to run our V8 implementation of Headers
 * and assert that it behaves as expected.
 *
 * The test cases were pulled from Deno and the goal is to either pass the test
 * case or throw an error with an appropriate error message for the things
 * we're intentionally skipping implementing.
 * https://github.com/denoland/deno/blob/10e4b2e14046b74469f7310c599579a6611513fe/cli/tests/unit/url_test.ts
 *
 * Known limitations:
 *  - We are not validating header values
 *  - We are not handling header guards (https://fetch.spec.whatwg.org/#headers-class)
 *  - We are not properly throwing TypeError when given an incorrect number or arguments
 *  - We are not guaranteed that `Headers.name` === "Headers"
 */

// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// function headersHasCorrectNameProp() {
//   assertEquals(Headers.name, "Headers");
// }

// Logic heavily copied from web-platform-tests, make
// sure pass mostly header basic test
// ref: https://github.com/web-platform-tests/wpt/blob/7c50c216081d6ea3c9afe553ee7b64534020a1b2/fetch/api/headers/headers-basic.html
function newHeaderTest() {
  new Headers();
  new Headers(undefined);
  new Headers({});
  try {
    new Headers(null as any);
  } catch (e) {
    assert(e instanceof TypeError, "e instanceof TypeError");
  }
}

const headerDict: Record<string, string> = {
  name1: "value1",
  name2: "value2",
  name3: "value3",
  name4: undefined as any,
  "Content-Type": "value4",
};

const headerSeq: any[] = [];
for (const name in headerDict) {
  headerSeq.push([name, headerDict[name]]);
}

function newHeaderWithSequence() {
  const headers = new Headers(headerSeq);
  for (const name in headerDict) {
    assert.strictEqual(headers.get(name), String(headerDict[name]));
  }
  assert.strictEqual(headers.get("length"), null);
}

function newHeaderWithRecord() {
  const headers = new Headers(headerDict);
  for (const name in headerDict) {
    assert.strictEqual(headers.get(name), String(headerDict[name]));
  }
}

function newHeaderWithHeadersInstance() {
  const headers = new Headers(headerDict);
  const headers2 = new Headers(headers);
  for (const name in headerDict) {
    assert.strictEqual(headers2.get(name), String(headerDict[name]));
  }
}

function headerAppendSuccess() {
  const headers = new Headers();
  for (const name in headerDict) {
    headers.append(name, headerDict[name]);
    assert.strictEqual(headers.get(name), String(headerDict[name]));
  }
}

function headerSetSuccess() {
  const headers = new Headers();
  for (const name in headerDict) {
    headers.set(name, headerDict[name]);
    assert.strictEqual(headers.get(name), String(headerDict[name]));
  }
}

function headerHasSuccess() {
  const headers = new Headers(headerDict);
  for (const name in headerDict) {
    assert(headers.has(name), "headers has name " + name);
    assert(
      !headers.has("nameNotInHeaders"),
      "headers do not have header: nameNotInHeaders",
    );
  }
}

function headerDeleteSuccess() {
  const headers = new Headers(headerDict);
  for (const name in headerDict) {
    assert(headers.has(name), "headers have a header: " + name);
    headers.delete(name);
    assert(!headers.has(name), "headers do not have anymore a header: " + name);
  }
}

function headerGetSuccess() {
  const headers = new Headers(headerDict);
  for (const name in headerDict) {
    assert.strictEqual(headers.get(name), String(headerDict[name]));
    assert.strictEqual(headers.get("nameNotInHeaders"), null);
  }
}

function headerEntriesSuccess() {
  const headers = new Headers(headerDict);
  // @ts-expect-error lib.dom typings disagree with headers being iterable
  const iterators = headers.entries();
  for (const it of iterators) {
    const key = it[0];
    const value = it[1];
    assert(headers.has(key), "headers.has(key)");
    assert.strictEqual(value, headers.get(key));
  }
}

function headerKeysSuccess() {
  const headers = new Headers(headerDict);
  // @ts-expect-error lib.dom typings disagree with headers being iterable
  const iterators = headers.keys();
  for (const it of iterators) {
    assert(headers.has(it), "headers.has(it))");
  }
}

function headerValuesSuccess() {
  const headers = new Headers(headerDict);
  // @ts-expect-error lib.dom typings disagree with headers being iterable
  const iterators = headers.values();
  // @ts-expect-error lib.dom typings disagree with headers being iterable
  const entries = headers.entries();
  const values = [];
  for (const pair of entries) {
    values.push(pair[1]);
  }
  for (const it of iterators) {
    assert.include(values, it);
  }
}

function headerForEachSuccess() {
  const headerEntriesDict: Record<string, string> = {
    name1: "value1",
    Name2: "value2",
    name: "value3",
    "content-Type": "value4",
    "Content-Typ": "value5",
    "Content-Types": "value6",
  };
  const headers = new Headers(headerEntriesDict);
  const keys = Object.keys(headerEntriesDict);
  keys.forEach((key) => {
    const value = headerEntriesDict[key];
    const newkey = key.toLowerCase();
    headerEntriesDict[newkey] = value;
  });
  let callNum = 0;
  headers.forEach((value, key, container) => {
    assert.strictEqual(headers, container);
    assert.strictEqual(value, headerEntriesDict[key]);
    callNum++;
  });
  assert(callNum === keys.length, "callNum === keys.length");
}

function headerSymbolIteratorSuccess() {
  const headerEntriesDict: Record<string, string> = {
    name1: "value1",
    Name2: "value2",
    name: "value3",
    "content-Type": "value4",
    "Content-Typ": "value5",
    "Content-Types": "value6",
  };
  assert(
    Symbol.iterator in Headers.prototype,
    "Symbol.iterator in Headers.prototype",
  );
  const headers = new Headers(headerEntriesDict);
  // @ts-expect-error lib.dom typings disagree with headers being iterable
  for (const header of headers) {
    const key = header[0];
    const value = header[1];
    assert(headers.has(key), "headers.has(key)");
    assert.strictEqual(value, headers.get(key));
  }
}

function headerTypesAvailable() {
  function newHeaders(): Headers {
    return new Headers();
  }
  const headers = newHeaders();
  assert(headers instanceof Headers, "headers instanceof Headers");
}

// Modified from https://github.com/bitinn/node-fetch/blob/7d3293200a91ad52b5ca7962f9d6fd1c04983edb/test/test.js#L2001-L2014
// Copyright (c) 2016 David Frank. MIT License.
function headerIllegalReject() {
  assert.throws(() => new Headers({ "He y": "ok" }));
  assert.throws(() => new Headers({ "Hé-y": "ok" }));
  // TODO: validate header values
  // try {
  //   new Headers({ "He-y": "ăk" });
  // } catch (_e) {
  //   errorCount++;
  // }
  const headers = new Headers();
  assert.throws(() => headers.append("Hé-y", "ok"));
  assert.throws(() => headers.delete("Hé-y"));
  assert.throws(() => headers.get("Hé-y"));
  assert.throws(() => headers.has("Hé-y"));
  assert.throws(() => headers.set("Hé-y", "ok"));
  assert.throws(() => headers.set("", "ok"));
  // 'o k' is valid value but invalid name
  assert.doesNotThrow(() => new Headers({ "He-y": "o k" }));
}

// If pair does not contain exactly two items,then throw a TypeError.
function headerParamsShouldThrowTypeError() {
  assert.throws(() => new Headers([["1"]] as any), TypeError);
}

function toStringShouldBeWebCompatibility() {
  const headers = new Headers();
  assert.strictEqual(headers.toString(), "[object Headers]");
}

// function invalidHeadersFlaky() {
//   assertThrows(
//     () => new Headers([["x", "\u0000x"]]),
//     TypeError,
//     "Header value is not valid."
//   );
//   assertThrows(
//     () => new Headers([["x", "\u0000x"]]),
//     TypeError,
//     "Header value is not valid."
//   );
// }

export default query(async () => {
  return await wrapInTests({
    newHeaderTest,
    newHeaderWithSequence,
    newHeaderWithRecord,
    newHeaderWithHeadersInstance,
    headerAppendSuccess,
    headerSetSuccess,
    headerHasSuccess,
    headerDeleteSuccess,
    headerGetSuccess,
    headerEntriesSuccess,
    headerKeysSuccess,
    headerValuesSuccess,
    headerForEachSuccess,
    headerSymbolIteratorSuccess,
    headerTypesAvailable,
    headerIllegalReject,
    headerParamsShouldThrowTypeError,
    // TODO: argument length validation
    // headerParamsArgumentsCheck,

    toStringShouldBeWebCompatibility,

    // TODO: our bundler sometimes changes the class name, which we could
    // configure differently
    // headersHasCorrectNameProp,

    // TODO: validate header values
    // invalidHeadersFlaky,
  });
});
