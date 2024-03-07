// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { assert, expect } from "chai";
import { query } from "../_generated/server";
import { wrapInTests } from "./testHelpers";

async function fromInit() {
  const req = new Request("http://foo/", {
    body: "ahoyhoy",
    method: "POST",
    headers: {
      "test-header": "value",
    },
  });

  assert.strictEqual("ahoyhoy", await req.text());
  assert.strictEqual(req.url, "http://foo/");
  assert.strictEqual(req.headers.get("test-header"), "value");
}

async function invalidJson() {
  const req = new Request("http://foo/", {
    body: "ahoyhoy",
    method: "POST",
  });

  await expect(req.json()).to.be.rejectedWith(SyntaxError, "Unexpected token");
}

async function clone() {
  const r1 = new Request("http://foo/", {
    body: "a test body",
    method: "POST",
  });

  const r2 = r1.clone();

  const b1 = await r1.text();
  const b2 = await r2.text();

  assert.strictEqual(b1, b2);
}

function methodNonString() {
  assert.strictEqual(
    new Request("http://foo/", { method: undefined }).method,
    "GET",
  );
}

function requestConstructorTakeURLObjectAsParameter() {
  assert.strictEqual(new Request(new URL("http://foo/")).url, "http://foo/");
}

async function requestURLSearchParams() {
  const request = new Request("http://foo/", {
    method: "POST",
    body: new URLSearchParams({ hello: "world" }),
  });

  const text = await request.text();
  assert.strictEqual(text, "hello=world");
}

async function consumeBodyTwice() {
  const req = new Request("http://foo/", {
    body: "ahoyhoy",
    method: "POST",
  });
  void req.text();
  await expect(req.text()).to.be.rejectedWith(/body stream already read/);
}

async function submitURLEncodedForData() {
  const req = new Request("http://foo/", {
    method: "POST",
    headers: {
      "Content-Type": "application/x-www-form-urlencoded",
    },
    body: "a=b&c=d",
  });
  const formData = await req.formData();
  assert.deepEqual(Array.from((formData as any).entries()), [
    ["a", "b"],
    ["c", "d"],
  ]);
}

function requestMethod() {
  const req = new Request("http://foo/", {
    method: "poST",
    body: "hello",
  });
  // Upper-case valid method
  assert.strictEqual(req.method, "POST");
  const req2 = new Request("http://foo/", {
    method: "foo",
    body: "hello",
  });
  assert.strictEqual(req2.method, "foo");
}

async function requestReadableStream() {
  // Check it doesn't throw.
  new Request("http://foo/", {
    method: "POST",
    body: new ReadableStream(),
  });
  // Check conversion ReadableStream => string.
  const encoder = new TextEncoder();
  const request = new Request("http://foo/", {
    method: "POST",
    body: new ReadableStream({
      start(controller) {
        controller.enqueue(encoder.encode("part1"));
        controller.enqueue(encoder.encode("part2"));
        controller.close();
      },
    }),
  });
  assert.strictEqual(await request.text(), "part1part2");
}

export default query(async () => {
  return await wrapInTests({
    fromInit,
    methodNonString,
    requestConstructorTakeURLObjectAsParameter,
    invalidJson,
    clone,
    requestURLSearchParams,
    consumeBodyTwice,
    submitURLEncodedForData,
    requestMethod,
    requestReadableStream,
  });
});
