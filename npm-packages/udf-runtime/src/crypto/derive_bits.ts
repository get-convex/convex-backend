// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/00_crypto.js

import * as z from "zod";
import { deriveBits as deriveBitsDef } from "./normalize_algorithm";
import { CryptoKey, _handle, KEY_STORE } from "./crypto_key";
import { copyBuffer } from "./helpers";
import { performOp } from "../syscall.js";
import { throwNotImplementedMethodError } from "../helpers";

export async function deriveBits(
  normalizedAlgorithm: z.infer<typeof deriveBitsDef>,
  baseKey: CryptoKey,
  length: number,
) {
  switch (normalizedAlgorithm.name) {
    case "PBKDF2": {
      // 1.
      if (
        length === null ||
        length === undefined ||
        length === 0 ||
        length % 8 !== 0
      ) {
        throw new DOMException("Invalid length", "OperationError");
      }

      if (normalizedAlgorithm.iterations === 0) {
        throw new DOMException("iterations must not be zero", "OperationError");
      }

      const handle = baseKey[_handle];
      const keyData = KEY_STORE.get(handle);

      normalizedAlgorithm.salt = copyBuffer(normalizedAlgorithm.salt);

      const buf = performOp(
        "crypto/deriveBits",
        {
          key: keyData,
          algorithm: "PBKDF2",
          hash: normalizedAlgorithm.hash.name,
          iterations: normalizedAlgorithm.iterations,
          length,
        },
        normalizedAlgorithm.salt,
      );

      return buf.buffer;
    }
    case "ECDH":
    case "HKDF":
      return throwNotImplementedMethodError(
        `deriveBits with algorithm ${normalizedAlgorithm.name}`,
        "SubtleCrypto",
      );
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }
}
