// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { action, query } from "../_generated/server";
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";

/**
 * The goal of this test is to run our V8 implementation of Response
 * and assert that it behaves as expected.
 *
 * The test cases were pulled from Deno and the goal is to either pass the test
 * case or throw an error with an appropriate error message for the things
 * we're intentionally skipping implementing.
 *
 * Known limitations:
 *  - We only support body inputs of type string | ArrayBuffer | null
 *  - We only support reading the body via `text()`, `json()`, and `arrayBuffer()`
 *  - We aren't validating input (e.g. status: 0 should error)
 *  - Many getters are not implemented (.body is perhaps the most notable one)
 */

async function responseText() {
  const response = new Response("hello world");
  const textPromise = response.text();
  assert(textPromise instanceof Promise, "textPromise instanceof Promise");
  const text = await textPromise;
  assert.isString(text);
  assert.strictEqual(text, "hello world");
}

async function responseArrayBuffer() {
  const response = new Response(new Uint8Array([1, 2, 3]).buffer);
  const arrayBufferPromise = response.arrayBuffer();
  assert(
    arrayBufferPromise instanceof Promise,
    "arrayBufferPromise instanceof Promise",
  );
  const arrayBuffer = await arrayBufferPromise;
  assert(
    arrayBuffer instanceof ArrayBuffer,
    "arrayBuffer instanceof ArrayBuffer",
  );
  assert.deepEqual(new Uint8Array(arrayBuffer), new Uint8Array([1, 2, 3]));
}

async function responseJson() {
  const response = new Response('{"hello": "world"}');
  const jsonPromise = response.json();
  assert(jsonPromise instanceof Promise, "jsonPromise instanceof Promise");
  const json = await jsonPromise;
  assert(json instanceof Object, "json instanceof Object");
  assert.strictEqual(JSON.stringify(json), JSON.stringify({ hello: "world" }));
}

async function responseBlob() {
  const response = new Response(new Uint8Array([1, 2, 3]).buffer);
  const blobPromise = response.blob();
  assert(blobPromise instanceof Promise);
  const blob = await blobPromise;
  assert(blob instanceof Blob);
  assert.strictEqual(blob.size, 3);
  assert.deepEqual(await blob.arrayBuffer(), new Uint8Array([1, 2, 3]).buffer);
}

async function responseFormData() {
  const input = new FormData();
  input.append("hello", "world");
  const response = new Response(input);
  const contentType = response.headers.get("content-type")!;
  assert(contentType.startsWith("multipart/form-data"));
  const formDataPromise = response.formData();
  assert(formDataPromise instanceof Promise);
  const formData = await formDataPromise;
  assert(formData instanceof FormData);
  assert.deepEqual([...(formData as any)], [...(input as any)]);
}

async function responseFormDataFile() {
  const input = new FormData();
  input.append("hello", new Blob(["world"]));
  input.append(
    "upload",
    new Blob(["abcdefg"], { type: "audio/webm" }),
    "my mixtape",
  );
  const response = new Response(input);
  const formData = await response.formData();
  const helloFile = formData.get("hello");
  assert(helloFile instanceof File);
  assert.strictEqual(helloFile.type, "application/octet-stream");
  assert.strictEqual(await helloFile.text(), "world");
  const uploadFile = formData.get("upload");
  assert(uploadFile instanceof File);
  assert.strictEqual(uploadFile.type, "audio/webm");
  assert.strictEqual(uploadFile.name, "my mixtape");
  assert.strictEqual(await uploadFile.text(), "abcdefg");
}

// function responseInvalidInit() {
//   // deno-lint-ignore ban-ts-comment
//   // @ts-expect-error
//   assertThrows(() => new Response("", 0), Error, "");
//   assertThrows(() => new Response("", { status: 0 }), Error, "");
//   // deno-lint-ignore ban-ts-comment
//   // @ts-expect-error
//   assertThrows(() => new Response("", { status: null }), Error, "");
// }

function responseInvalidStatus() {
  // @ts-expect-error -- Intentional error
  const res = new Response("", { status: "418" });
  // coerces to number
  assert.strictEqual(res.status, 418);
  assert.throws(
    // @ts-expect-error -- Intentional error
    () => new Response("", { status: "418foo" }),
    RangeError,
    /The status provided is outside the range \[200, 599\]\./,
  );
  assert.throws(
    // @ts-expect-error -- Intentional error
    () => new Response("", { status: null }),
    RangeError,
    /The status provided is outside the range \[200, 599\]\./,
  );
}

function responseNullInit() {
  // deno-lint-ignore ban-ts-comment
  // @ts-expect-error Intentional error
  const response = new Response("", null);
  assert.strictEqual(response.status, 200);
}

async function responseBodyUsed() {
  const response = new Response("body");
  assert(!response.bodyUsed, "body initially unused");
  await response.text();
  assert(response.bodyUsed, "body used");
}

async function responseClone() {
  const r1 = new Response("a test body", {
    status: 418,
  });

  const r2 = r1.clone();

  const b1 = await r1.text();
  const b2 = await r2.text();

  assert.strictEqual(b1, b2);
  assert.strictEqual(r1.status, r2.status);
}

async function responseReadableStream() {
  // Check it doesn't throw.
  new Response(new ReadableStream());
  // Check conversion ReadableStream => string.
  const encoder = new TextEncoder();
  const response = new Response(
    new ReadableStream({
      start(controller) {
        controller.enqueue(encoder.encode("part1"));
        controller.enqueue(encoder.encode("part2"));
        controller.close();
      },
    }),
  );
  assert.strictEqual(await response.text(), "part1part2");
}

async function responseJsonStatic() {
  // Static json method is only in newer versions of TS (which VS code is using),
  // but not in the version of TS we're actually using to compile this project.
  // The `ts-expect-error`s might show up as errors in the editor but are necessary.

  // @ts-expect-error -- see above
  const response = Response.json({ hello: "world" }, { status: 418 });
  assert.strictEqual(response.headers.get("content-type"), "application/json");
  assert.strictEqual(response.status, 418);

  const body = await response.json();
  assert.deepEqual(body, { hello: "world" });
  assert.throws(
    // @ts-expect-error -- see above
    () => Response.json(undefined),
    TypeError,
    /The data is not JSON serializable/,
  );
  const circularReference: any = {};
  circularReference.myself = circularReference;
  assert.throws(
    // @ts-expect-error -- see above
    () => Response.json(circularReference),
    TypeError,
    /The data is not JSON serializable/,
  );
}

export default query(async () => {
  return await wrapInTests({
    responseText,
    responseArrayBuffer,
    responseJson,
    responseBlob,
    responseInvalidStatus,
    // TODO: better input validation
    // responseInvalidInit,
    responseNullInit,
    responseClone,
    responseBodyUsed,
    responseReadableStream,
    responseJsonStatic,
  });
});

export const responseAction = action({
  args: {},
  handler: async () => {
    return await wrapInTests({
      responseFormData,
      responseFormDataFile,
    });
  },
});
