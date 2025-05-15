// The initial implementation were taken from Deno.
// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/LICENSE.md

import { throwNotImplementedMethodError } from "./helpers";
import { performOp } from "./syscall";
import { ReadableStream } from "./06_streams";

async function* toIterator(
  parts: (BlobReference | BlobStreamReference | Blob)[],
): AsyncGenerator<Uint8Array> {
  for (const part of parts) {
    if (part instanceof Blob) {
      yield* part.stream();
    } else if (part instanceof BlobReference) {
      yield new Uint8Array(part.arrayBuffer());
    } else if (part instanceof BlobStreamReference) {
      yield* part.stream();
    } else {
      throw new Error("unrecognized part");
    }
  }
}

// TODO(presley): To have proper streaming, BlobReference should be able to
// reference resources in rust and fetch them via ops. For now, just wrap Uint8Array.
class BlobReference {
  private _id: string;
  private _size: number;

  constructor(id: string, size: number) {
    this._id = id;
    this._size = size;
  }

  static fromUint8Array(data: Uint8Array) {
    const id = performOp("blob/createPart", data);
    return new BlobReference(id, data.byteLength);
  }

  slice(start: number, end: number): BlobReference {
    const size = end - start;
    const id = performOp("blob/slicePart", this._id, start, size);
    return new BlobReference(id, size);
  }

  arrayBuffer(): ArrayBuffer {
    return performOp("blob/readPart", this._id);
  }

  get size() {
    return this._size;
  }
}

class BlobStreamReference {
  private _stream: ReadableStream<Uint8Array>;
  private _size: number;

  constructor(stream: ReadableStream<Uint8Array>, size: number) {
    this._stream = stream;
    this._size = size;
  }

  slice(start: number, end: number): BlobStreamReference {
    const size = end - start;
    const [original, toSlice] = this._stream.tee();
    this._stream = original;

    const reader = toSlice.getReader();
    let bytesRead = 0;
    const sliced = new ReadableStream({
      type: "bytes",
      async pull(controller) {
        // eslint-disable-next-line no-constant-condition
        while (true) {
          const { value, done } = await reader.read();
          if (done || bytesRead >= end) return controller.close();
          const valueSlice = value.slice(
            Math.max(0, start - bytesRead),
            end - bytesRead,
          );
          bytesRead += value.length;
          if (valueSlice.byteLength > 0) {
            return controller.enqueue(value);
          }
        }
      },
    });
    return new BlobStreamReference(sliced, size);
  }

  stream(): ReadableStream<Uint8Array> {
    return this._stream;
  }

  get size() {
    return this._size;
  }
}

function iteratorToReadableStream(
  iterator: AsyncIterator<Uint8Array>,
): ReadableStream<Uint8Array> {
  return new ReadableStream({
    type: "bytes",
    async pull(controller) {
      // eslint-disable-next-line no-constant-condition
      while (true) {
        const { value, done } = await iterator.next();
        if (done) return controller.close();
        if (value.byteLength > 0) {
          return controller.enqueue(value);
        }
      }
    },
  });
}

const NORMALIZE_PATTERN = new RegExp(/^[\x20-\x7E]*$/);

export function isSupportedBlobPart(part): boolean {
  if (part === undefined || part === null) {
    return false;
  }
  return (
    typeof part === "string" ||
    part instanceof ArrayBuffer ||
    part instanceof Blob ||
    ((part.buffer instanceof ArrayBuffer ||
      part.buffer instanceof SharedArrayBuffer) &&
      typeof part.byteLength === "number" &&
      typeof part.byteOffset === "number")
  );
}

type BlobPart = string | BufferSource | Blob;

export class Blob {
  private _parts: (BlobReference | BlobStreamReference | Blob)[];
  private _size: number;
  private _type: string;

  constructor(blobParts?: BlobPart[], options?: BlobPropertyBag) {
    const { parts, size } = Blob._processBlobParts(
      blobParts ?? [],
      options?.endings,
    );
    this._parts = parts;
    this._size = size;
    this._type = Blob._normalizeType(options?.type);
  }

  get size(): number {
    return this._size;
  }

  get type(): string {
    return this._type;
  }

  slice(start?: number, end?: number, contentType?: string): Blob {
    // eslint-disable-next-line @typescript-eslint/no-this-alias
    const O = this;
    let relativeStart: number;
    if (start === undefined) {
      relativeStart = 0;
    } else {
      if (start < 0) {
        relativeStart = Math.max(O.size + start, 0);
      } else {
        relativeStart = Math.min(start, O.size);
      }
    }
    let relativeEnd;
    if (end === undefined) {
      relativeEnd = O.size;
    } else {
      if (end < 0) {
        relativeEnd = Math.max(O.size + end, 0);
      } else {
        relativeEnd = Math.min(end, O.size);
      }
    }

    const span = Math.max(relativeEnd - relativeStart, 0);
    const blobParts: (BlobReference | BlobStreamReference | Blob)[] = [];
    let added = 0;

    const parts = this._parts;
    for (let i = 0; i < parts.length; ++i) {
      const part = parts[i];
      // don't add the overflow to new blobParts
      if (added >= span) {
        // Could maybe be possible to remove variable `added`
        // and only use relativeEnd?
        break;
      }
      const size = part.size;
      if (relativeStart && size <= relativeStart) {
        // Skip the beginning and change the relative
        // start & end position as we skip the unwanted parts
        relativeStart -= size;
        relativeEnd -= size;
      } else {
        const chunk = part.slice(
          relativeStart,
          Math.min(part.size, relativeEnd),
        );
        added += chunk.size;
        relativeEnd -= part.size;
        blobParts.push(chunk);
        relativeStart = 0; // All next sequential parts should start at 0
      }
    }

    let relativeContentType: string;
    if (contentType === undefined) {
      relativeContentType = "";
    } else {
      relativeContentType = Blob._normalizeType(contentType);
    }

    const blob = new Blob([], { type: relativeContentType });
    blob._parts = blobParts;
    blob._size = span;
    return blob;
  }

  async text(): Promise<string> {
    const decoder = new TextDecoder();
    return this.arrayBuffer().then((array) => decoder.decode(array));
  }

  async arrayBuffer(): Promise<ArrayBuffer> {
    const bytes = new Uint8Array(this._size);
    const partIterator = toIterator(this._parts);
    let offset = 0;
    // eslint-disable-next-line no-constant-condition
    while (true) {
      const { value, done } = await partIterator.next();
      if (done) break;
      const byteLength = value.byteLength;
      if (byteLength > 0) {
        bytes.set(value, offset);
        offset += byteLength;
      }
    }
    return bytes.buffer;
  }

  stream(): ReadableStream<Uint8Array> {
    const partIterator = toIterator(this._parts);
    return iteratorToReadableStream(partIterator);
  }

  private static _processBlobParts(
    parts: BlobPart[],
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    ending?: string,
  ): { parts: (BlobReference | Blob)[]; size: number } {
    const processedParts: (BlobReference | Blob)[] = [];
    let size = 0;
    for (const element of parts) {
      if (element instanceof Blob) {
        size += element.size;
        processedParts.push(element);
      } else if (typeof element === "string") {
        const encoder = new TextEncoder();
        const bytes = encoder.encode(element);
        size += bytes.byteLength;
        processedParts.push(BlobReference.fromUint8Array(bytes));
      } else if (element instanceof ArrayBuffer) {
        size += element.byteLength;
        processedParts.push(
          BlobReference.fromUint8Array(new Uint8Array(element)),
        );
      } else if (
        (element.buffer instanceof ArrayBuffer ||
          element.buffer instanceof SharedArrayBuffer) &&
        typeof element.byteLength === "number" &&
        typeof element.byteOffset === "number"
      ) {
        size += element.byteLength;
        processedParts.push(
          BlobReference.fromUint8Array(
            new Uint8Array(
              element.buffer.slice(
                element.byteOffset,
                element.byteOffset + element.byteLength,
              ),
            ),
          ),
        );
      } else {
        throwNotImplementedMethodError(
          "constructor with unsupported Blob part",
          "Blob",
          "Only ArrayBuffer, Blob and string supported",
        );
      }
    }
    return { parts: processedParts, size };
  }

  private static _normalizeType(str?: string): string {
    let normalizedType;
    if (!str || !NORMALIZE_PATTERN.test(str)) {
      normalizedType = "";
    } else {
      normalizedType = str;
    }
    return normalizedType.toLowerCase();
  }

  static fromIdPart(id: string, size: number): Blob {
    const blob = new Blob();
    blob._parts = [new BlobReference(id, size)];
    blob._size = size;
    return blob;
  }

  static fromStream(
    stream: ReadableStream<Uint8Array>,
    size: number,
    type?: string,
  ): Blob {
    const blob = new Blob([], { type });
    // When creating a Blob from a stream we want to lock the stream synchronously
    // so `Request.body.locked` is true, while still retaining the ability to return
    // an unlocked stream from `Blob.stream()`.
    const newStream = iteratorToReadableStream(stream[Symbol.asyncIterator]());
    blob._parts = [new BlobStreamReference(newStream, size)];
    blob._size = size;
    return blob;
  }

  get [Symbol.toStringTag]() {
    return "Blob";
  }

  inspect() {
    return `Blob { size: ${this.size}, type: "${this.type}" }`;
  }
}

export class File extends Blob {
  private _fileName: string;
  private _lastModified: number;

  constructor(
    fileParts: BlobPart[],
    fileName: string,
    options?: FilePropertyBag,
  ) {
    super(fileParts, options);
    this._fileName = String(fileName);
    this._lastModified = options?.lastModified ?? Date.now();
  }

  get name() {
    return this._fileName;
  }

  get lastModified() {
    return this._lastModified;
  }

  get [Symbol.toStringTag]() {
    return "File";
  }
}

export const setupBlob = (global: any) => {
  global.Blob = Blob;
  global.File = File;
};
