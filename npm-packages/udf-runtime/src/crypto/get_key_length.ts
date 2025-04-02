// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/00_crypto.js

import * as z from "zod";
import { getKeyLength } from "./normalize_algorithm";
import { throwUncatchableDeveloperError } from "../helpers";

export default function (algorithm: z.infer<typeof getKeyLength>): number {
  switch (algorithm.name) {
    case "AES-CBC":
    case "AES-CTR":
    case "AES-GCM":
    case "AES-KW": {
      // 1.
      if (![128, 192, 256].includes(algorithm.length)) {
        throw new DOMException(
          "length must be 128, 192, or 256",
          "OperationError",
        );
      }

      // 2.
      return algorithm.length;
    }
    case "HMAC": {
      // 1.
      let length: number;
      if (algorithm.length === undefined) {
        switch (algorithm.hash.name) {
          case "SHA-1":
            length = 512;
            break;
          case "SHA-256":
            length = 512;
            break;
          case "SHA-384":
            length = 1024;
            break;
          case "SHA-512":
            length = 1024;
            break;
          default:
            throw new DOMException(
              "Unrecognized hash algorithm",
              "NotSupportedError",
            );
        }
      } else if (algorithm.length !== 0) {
        length = algorithm.length;
      } else {
        throw new TypeError("Invalid length.");
      }

      // 2.
      return length;
    }
    case "HKDF": {
      // 1.
      return throwUncatchableDeveloperError(
        "deriving HKDF key not implemented",
      );
    }
    case "PBKDF2": {
      // 1.
      return throwUncatchableDeveloperError(
        "deriving PBKDF2 key not implemented",
      );
    }
  }
  const _: never = algorithm;
  throw new TypeError("unreachable");
}
