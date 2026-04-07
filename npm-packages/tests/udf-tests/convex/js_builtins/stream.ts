import { assert } from "chai";
import { wrapInTests } from "./testHelpers";
import { query } from "../_generated/server";

export default query(async () => {
  return await wrapInTests({
    streamReadWrite,
    streamRoundTrip,
  });
});

async function streamReadWrite() {
  let controller: ReadableByteStreamController;
  const stream = new ReadableStream({
    type: "bytes",
    start(c) {
      controller = c;
    },
  });
  const reader = stream.getReader();
  const encoder = new TextEncoder();
  controller!.enqueue(encoder.encode("hi "));
  assert.deepEqual(await reader.read(), {
    value: encoder.encode("hi "),
    done: false,
  });
  controller!.enqueue(encoder.encode("there!"));
  assert.deepEqual(await reader.read(), {
    value: encoder.encode("there!"),
    done: false,
  });
  controller!.close();
  assert.deepEqual(await reader.read(), { value: undefined, done: true });
}

async function streamRoundTrip() {
  let controller: ReadableByteStreamController;
  const stream = new ReadableStream({
    type: "bytes",
    start(c) {
      controller = c;
    },
  });
  // Send it to Rust and back.
  const reader = (stream as any)["_persist"]().getReader();
  const encoder = new TextEncoder();
  controller!.enqueue(encoder.encode("hi "));
  assert.deepEqual(await reader.read(), {
    value: encoder.encode("hi "),
    done: false,
  });
  controller!.enqueue(encoder.encode("there!"));
  assert.deepEqual(await reader.read(), {
    value: encoder.encode("there!"),
    done: false,
  });
  controller!.close();
  assert.deepEqual(await reader.read(), { value: undefined, done: true });
}
