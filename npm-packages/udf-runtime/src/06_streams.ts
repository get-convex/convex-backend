// This is the same polyfill Lagon uses
import {
  ReadableStream,
  ReadableStreamBYOBReader,
  ReadableStreamDefaultReader,
  TransformStream,
  WritableStream,
  WritableStreamDefaultWriter,
} from "web-streams-polyfill";
import { performAsyncOp, performOp } from "./syscall";

export { ReadableStream };

export const constructStreamId = (stream: ReadableStream | null): string => {
  const streamId = performOp("stream/create");
  const reader = stream?.getReader();
  void populateStream();
  return streamId;

  async function populateStream() {
    if (!reader) {
      performOp("stream/extend", streamId, undefined, true);
      return;
    }
    const { value, done } = await reader.read();
    performOp("stream/extend", streamId, value, done);
    if (!done) {
      void populateStream();
    }
  }
};

export const extractStream = (streamId: string): ReadableStream => {
  return new ReadableStream({
    type: "bytes",
    async pull(controller) {
      while (true) {
        const { value, done } = await performAsyncOp(
          "stream/readPart",
          streamId,
        );
        if (done === true) {
          return controller.close();
        } else if (value.length > 0) {
          return controller.enqueue(value);
        }
      }
    },
  });
};

// For testing.
// Gives ownership of the ReadableStream to Rust.
ReadableStream.prototype["_persist"] = function () {
  const streamId = constructStreamId(this);
  return extractStream(streamId);
};

export const setupStreams = (global) => {
  global.ReadableStream = ReadableStream;
  global.ReadableStreamBYOBReader = ReadableStreamBYOBReader;
  global.ReadableStreamDefaultReader = ReadableStreamDefaultReader;
  global.TransformStream = TransformStream;
  global.WritableStream = WritableStream;
  global.WritableStreamDefaultWriter = WritableStreamDefaultWriter;
};
