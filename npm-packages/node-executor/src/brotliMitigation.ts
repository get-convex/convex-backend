// Mitigates CVE-2026-111497, an out-of-bounds write in the Brotli decoder that
// Node.js bundles, on Lambda runtimes that run an unpatched Node.js
// (https://nodejs.org/en/blog/vulnerability/october-2026-security-releases).
//
// Every JS path to the native decoder goes through the four Brotli
// decompression exports of `node:zlib`: user code and npm packages call them
// directly, and Node's built-in `fetch` (undici) and
// `DecompressionStream("brotli")` look up `zlib.createBrotliDecompress` on the
// same module object at call time. Replacing those exports makes Brotli input
// fail before it reaches native code. Brotli compression is unaffected.
//
// To keep servers from choosing Brotli, `fetch` and `http(s).request` stop
// advertising `br` in `Accept-Encoding`. A server that sends `br` anyway makes
// the request fail.
//
// Not covered: `process.binding("zlib")`, and Node.js processes or worker
// threads started from user code, which load their own `node:zlib`.

import http from "node:http";
import { syncBuiltinESMExports } from "node:module";
import { Transform, TransformCallback, TransformOptions } from "node:stream";
import zlib from "node:zlib";

const MESSAGE =
  "Brotli decompression is disabled in Convex Node.js actions until the " +
  "Node.js runtime is patched for CVE-2026-111497. Request a different " +
  "Content-Encoding (for example `Accept-Encoding: gzip, deflate`).";

export class BrotliDecompressionDisabledError extends Error {
  code = "ERR_CONVEX_BROTLI_DISABLED";
  constructor() {
    super(MESSAGE);
    this.name = "BrotliDecompressionDisabledError";
  }
}

// Errors on the first input chunk rather than on construction, so a `br`
// response with an empty body still succeeds, and `fetch` reports the error
// through the body stream like any other decoding error.
class DisabledBrotliDecompress extends Transform {
  constructor(options?: TransformOptions) {
    super(options);
  }
  _transform(_chunk: unknown, _encoding: string, callback: TransformCallback) {
    // The native decoder fails on a later event loop turn. undici's `fetch`
    // loses an error raised synchronously (or on `process.nextTick`) while it
    // is pushing the chunk, and the response body then never settles.
    setImmediate(callback, new BrotliDecompressionDisabledError());
  }
}

function hasInput(buffer: unknown): boolean {
  if (typeof buffer === "string") {
    return buffer.length > 0;
  }
  if (ArrayBuffer.isView(buffer) || buffer instanceof ArrayBuffer) {
    return buffer.byteLength > 0;
  }
  // Defers to Node's own argument validation in `brotliDecompressSync`.
  return true;
}

const ACCEPT_ENCODING_WITHOUT_BR = "gzip, deflate";

function withoutBr(acceptEncoding: string): string {
  const codings = acceptEncoding
    .split(",")
    .map((coding) => coding.trim())
    .filter((coding) => coding && !/^br(\s*;|$)/i.test(coding));
  // A wildcard also admits `br`, so it needs an explicit exclusion.
  if (codings.some((coding) => /^\*(\s*;|$)/.test(coding))) {
    codings.push("br;q=0");
  }
  return codings.length > 0 ? codings.join(", ") : "identity";
}

function stopAdvertisingBrInFetch() {
  const originalFetch = globalThis.fetch;
  if (typeof originalFetch !== "function") {
    return;
  }
  // undici adds `Accept-Encoding: gzip, deflate, br` to HTTPS requests that
  // don't set the header, so the wrapper always sets it. It passes `init`
  // through rather than building a `Request`, which would drop undici-only
  // options such as `dispatcher`.
  globalThis.fetch = function fetch(
    input: Parameters<typeof originalFetch>[0],
    init?: Parameters<typeof originalFetch>[1],
  ) {
    const headers = new Headers(
      init?.headers ?? (input instanceof Request ? input.headers : undefined),
    );
    const acceptEncoding = headers.get("accept-encoding");
    headers.set(
      "accept-encoding",
      acceptEncoding === null
        ? ACCEPT_ENCODING_WITHOUT_BR
        : withoutBr(acceptEncoding),
    );
    return originalFetch(input, { ...init, headers });
  };
}

// `https.request` and `http.request` both build an `http.ClientRequest`, which
// routes headers from `options.headers` through `setHeader`. Clients such as
// axios, got, and node-fetch send `Accept-Encoding: ..., br` this way.
function stopAdvertisingBrInHttpClients() {
  const proto = http.ClientRequest.prototype;
  for (const method of ["setHeader", "appendHeader"] as const) {
    const original = proto[method] as (
      this: http.ClientRequest,
      name: string,
      value: number | string | readonly string[],
    ) => http.ClientRequest;
    proto[method] = function (
      this: http.ClientRequest,
      name: string,
      value: number | string | readonly string[],
    ) {
      if (name.toLowerCase() === "accept-encoding") {
        value =
          typeof value === "string"
            ? withoutBr(value)
            : Array.isArray(value)
              ? value.map(withoutBr)
              : value;
      }
      return original.call(this, name, value);
    };
  }
}

let installed = false;

// Set in the Lambda environment by the deploy worker in conductor.
export function disableBrotliDecompressionIfFlagged(
  env: NodeJS.ProcessEnv,
): boolean {
  if (env.CONVEX_DISABLE_BROTLI_DECOMPRESSION !== "true") {
    return false;
  }
  disableBrotliDecompression();
  return true;
}

export function disableBrotliDecompression() {
  if (installed) {
    return;
  }
  installed = true;

  const replacements = {
    BrotliDecompress: DisabledBrotliDecompress,
    createBrotliDecompress: (options?: TransformOptions) =>
      new DisabledBrotliDecompress(options),
    brotliDecompress: (
      buffer: unknown,
      optionsOrCallback: unknown,
      maybeCallback?: unknown,
    ) => {
      const callback =
        typeof optionsOrCallback === "function"
          ? optionsOrCallback
          : maybeCallback;
      if (typeof callback !== "function") {
        throw new TypeError('The "callback" argument must be of type function');
      }
      process.nextTick(() => {
        if (hasInput(buffer)) {
          callback(new BrotliDecompressionDisabledError());
        } else {
          callback(null, Buffer.alloc(0));
        }
      });
    },
    brotliDecompressSync: (buffer: unknown) => {
      if (hasInput(buffer)) {
        throw new BrotliDecompressionDisabledError();
      }
      return Buffer.alloc(0);
    },
  };
  for (const [name, value] of Object.entries(replacements)) {
    // `createBrotliDecompress` is non-writable but configurable.
    const descriptor = Object.getOwnPropertyDescriptor(zlib, name)!;
    Object.defineProperty(zlib, name, { ...descriptor, value });
  }
  // Updates named ESM imports (`import { brotliDecompress } from "node:zlib"`)
  // made before this ran.
  syncBuiltinESMExports();

  stopAdvertisingBrInFetch();
  stopAdvertisingBrInHttpClients();
}
