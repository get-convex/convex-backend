// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";
import { query } from "../_generated/server";

export default query(async () => {
  return await wrapInTests({
    empty_blob,
    from_array_buffer,
    from_type_array,
    from_string,
    from_blob,
    multi_part,
    blobStream,
    blobString,
    blobBuffer,
    blobSlice,
    blobInvalidType,
  });
});

function empty_blob() {
  const b = new Blob();
  assert.strictEqual(b.size, 0);
  assert.strictEqual(b.type, "");
}

async function from_array_buffer() {
  const input = new Uint8Array([1, 2, 3]).buffer;
  const b = new Blob([input]);
  assert.strictEqual(b.size, 3);
  assert.deepEqual(await b.arrayBuffer(), input);
}

async function from_type_array() {
  const input = new Uint8Array([1, 2, 3]);
  const b = new Blob([input]);
  assert.strictEqual(b.size, 3);
  assert.deepEqual(await b.arrayBuffer(), input.buffer);
}

async function from_string() {
  const b = new Blob(["test123"]);
  assert.strictEqual(b.size, 7);
  assert.strictEqual(await b.text(), "test123");
}

async function from_blob() {
  const input = new Blob(["test123"]);
  const b = new Blob([input]);
  assert.strictEqual(b.size, 7);
  assert.deepEqual(await b.arrayBuffer(), await input.arrayBuffer());
}

async function multi_part() {
  const b = new Blob([
    new Uint8Array([1]),
    new Uint8Array([2]),
    new Uint8Array([3]),
  ]);
  assert.strictEqual(b.size, 3);
  assert.deepEqual(await b.arrayBuffer(), new Uint8Array([1, 2, 3]).buffer);
}

async function blobStream() {
  const blob = new Blob(["Hello World"]);
  const stream = blob.stream();
  assert(stream instanceof ReadableStream);
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  const read = async (): Promise<void> => {
    const { done, value } = await reader.read();
    if (!done && value) {
      chunks.push(value);
      return read();
    }
  };
  await read();
  const decoder = new TextDecoder();
  const bytes = await new Blob(chunks).arrayBuffer();
  assert.strictEqual(decoder.decode(bytes), "Hello World");
}

function blobString() {
  const b1 = new Blob(["Hello World"]);
  const str = "Test";
  const b2 = new Blob([b1, str]);
  assert.strictEqual(b2.size, b1.size + str.length);
}

function blobBuffer() {
  const buffer = new ArrayBuffer(12);
  const u8 = new Uint8Array(buffer);
  const f1 = new Float32Array(buffer);
  const b1 = new Blob([buffer, u8]);
  assert.strictEqual(b1.size, 2 * u8.length);
  const b2 = new Blob([b1, f1]);
  assert.strictEqual(b2.size, 3 * u8.length);
}

function blobSlice() {
  const blob = new Blob(["Deno", "Foo"]);
  const b1 = blob.slice(0, 3, "Text/HTML");
  assert(b1 instanceof Blob);
  assert.strictEqual(b1.size, 3);
  assert.strictEqual(b1.type, "text/html");
  const b2 = blob.slice(-1, 3);
  assert.strictEqual(b2.size, 0);
  const b3 = blob.slice(100, 3);
  assert.strictEqual(b3.size, 0);
  const b4 = blob.slice(0, 10);
  assert.strictEqual(b4.size, blob.size);
}

function blobInvalidType() {
  const blob = new Blob(["foo"], {
    type: "\u0521",
  });

  assert.strictEqual(blob.type, "");
}
