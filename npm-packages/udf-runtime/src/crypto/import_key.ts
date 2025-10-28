// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/00_crypto.js

import { z } from "zod";
import { performOp } from "../syscall.js";
import { CryptoKey, KEY_STORE } from "./crypto_key.js";
import {
  ArrayPrototypeEvery,
  ArrayPrototypeFind,
  ArrayPrototypeIncludes,
  TypedArrayPrototypeGetByteLength,
  WeakMapPrototypeSet,
} from "./helpers.js";
import {
  hmacImportParams,
  normalizeAlgorithmDigest,
  rsaHashedImportParams,
} from "./normalize_algorithm.js";

export function hmac(
  format: string,
  normalizedAlgorithm: z.infer<typeof hmacImportParams> & { name: "HMAC" },
  keyData: any, // type depends on `format`
  extractable: boolean,
  keyUsages: readonly string[],
): CryptoKey {
  // 2.
  if (
    ArrayPrototypeFind(
      keyUsages,
      (u) => !ArrayPrototypeIncludes(["sign", "verify"], u),
    ) !== undefined
  ) {
    throw new DOMException("Invalid key usages", "SyntaxError");
  }

  // 3.
  let hash: { name: any };
  let data: Uint8Array<ArrayBufferLike>;

  // 4. https://w3c.github.io/webcrypto/#hmac-operations
  switch (format) {
    case "raw": {
      data = keyData;
      hash = normalizedAlgorithm.hash;
      break;
    }
    case "jwk": {
      const jwk = keyData;

      // 2.
      if (jwk.kty !== "oct") {
        throw new DOMException(
          "'kty' property of JsonWebKey must be 'oct'",
          "DataError",
        );
      }

      // Section 6.4.1 of RFC7518
      if (jwk.k === undefined) {
        throw new DOMException(
          "'k' property of JsonWebKey must be present",
          "DataError",
        );
      }

      // 4.
      const { rawData } = performOp(
        "crypto/importKey",
        { algorithm: "HMAC" },
        { jwkSecret: jwk },
      );
      data = rawData.data;

      // 5.
      hash = normalizedAlgorithm.hash;

      // 6.
      switch (hash.name) {
        case "SHA-1": {
          if (jwk.alg !== undefined && jwk.alg !== "HS1") {
            throw new DOMException(
              "'alg' property of JsonWebKey must be 'HS1'",
              "DataError",
            );
          }
          break;
        }
        case "SHA-256": {
          if (jwk.alg !== undefined && jwk.alg !== "HS256") {
            throw new DOMException(
              "'alg' property of JsonWebKey must be 'HS256'",
              "DataError",
            );
          }
          break;
        }
        case "SHA-384": {
          if (jwk.alg !== undefined && jwk.alg !== "HS384") {
            throw new DOMException(
              "'alg' property of JsonWebKey must be 'HS384'",
              "DataError",
            );
          }
          break;
        }
        case "SHA-512": {
          if (jwk.alg !== undefined && jwk.alg !== "HS512") {
            throw new DOMException(
              "'alg' property of JsonWebKey must be 'HS512'",
              "DataError",
            );
          }
          break;
        }
        default:
          throw new TypeError("unreachable");
      }

      // 7.
      if (keyUsages.length > 0 && jwk.use !== undefined && jwk.use !== "sig") {
        throw new DOMException(
          "'use' property of JsonWebKey must be 'sig'",
          "DataError",
        );
      }

      // 8.
      // Section 4.3 of RFC7517
      if (jwk.key_ops !== undefined) {
        if (
          ArrayPrototypeFind(
            jwk.key_ops,
            (u) => !ArrayPrototypeIncludes(recognisedUsages, u),
          ) !== undefined
        ) {
          throw new DOMException(
            "'key_ops' property of JsonWebKey is invalid",
            "DataError",
          );
        }

        if (
          !ArrayPrototypeEvery(jwk.key_ops, (u) =>
            ArrayPrototypeIncludes(keyUsages, u),
          )
        ) {
          throw new DOMException(
            "'key_ops' property of JsonWebKey is invalid",
            "DataError",
          );
        }
      }

      // 9.
      if (jwk.ext === false && extractable === true) {
        throw new DOMException(
          "'ext' property of JsonWebKey must not be false if extractable is true",
          "DataError",
        );
      }

      break;
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }

  // 5.
  let length = TypedArrayPrototypeGetByteLength(data) * 8;
  // 6.
  if (length === 0) {
    throw new DOMException("Key length is zero", "DataError");
  }
  // 7.
  if (normalizedAlgorithm.length !== undefined) {
    if (
      normalizedAlgorithm.length > length ||
      normalizedAlgorithm.length <= length - 8
    ) {
      throw new DOMException("Key length is invalid", "DataError");
    }
    length = normalizedAlgorithm.length;
  }

  const handle = {};
  WeakMapPrototypeSet(KEY_STORE, handle, {
    type: "secret",
    data,
  });

  const algorithm = {
    name: "HMAC",
    length,
    hash,
  };

  const key = new CryptoKey(
    "secret",
    extractable,
    usageIntersection(keyUsages, recognisedUsages),
    algorithm,
    handle,
  );

  return key;
}

export function pbkdf2(
  format: "raw" | "jwk" | "pkcs8" | "spki",
  keyData: BufferSource,
  extractable: boolean,
  keyUsages: string[],
) {
  // 1.
  if (format !== "raw") {
    throw new DOMException("Format not supported", "NotSupportedError");
  }

  // 2.
  if (
    keyUsages.find((u) => !["deriveKey", "deriveBits"].includes(u)) !==
    undefined
  ) {
    throw new DOMException("Invalid key usages", "SyntaxError");
  }

  // 3.
  if (extractable !== false) {
    throw new DOMException("Key must not be extractable", "SyntaxError");
  }

  // 4.
  const handle = {};
  KEY_STORE.set(handle, {
    type: "secret",
    data: keyData,
  });

  // 5-9.
  const algorithm = {
    name: "PBKDF2",
  };
  const key = new CryptoKey("secret", false, keyUsages, algorithm, handle);

  // 10.
  return key;
}

export const SUPPORTED_KEY_USAGES = {
  "RSASSA-PKCS1-v1_5": {
    public: ["verify"],
    private: ["sign"],
    jwkUse: "sig",
  },
  "RSA-PSS": {
    public: ["verify"],
    private: ["sign"],
    jwkUse: "sig",
  },
  "RSA-OAEP": {
    public: ["encrypt", "wrapKey"],
    private: ["decrypt", "unwrapKey"],
    jwkUse: "enc",
  },
  ECDSA: {
    public: ["verify"],
    private: ["sign"],
    jwkUse: "sig",
  },
  ECDH: {
    public: [] as string[],
    private: ["deriveKey", "deriveBits"],
    jwkUse: "enc",
  },
  Ed25519: {
    public: ["verify"],
    private: ["sign"],
    jwkUse: "sig",
  },
  X25519: {
    public: [] as string[],
    private: ["deriveKey", "deriveBits"],
    jwkUse: "enc",
  },
};

export const SUPPORTED_SYMMETRIC_KEY_USAGES = {
  "AES-CTR": ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
  "AES-CBC": ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
  "AES-GCM": ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
  "AES-KW": ["wrapKey", "unwrapKey"],
  HMAC: ["sign", "verify"],
};

// P-521 is not yet supported.
export const supportedNamedCurves = ["P-256", "P-384"];
const recognisedUsages = [
  "encrypt",
  "decrypt",
  "sign",
  "verify",
  "deriveKey",
  "deriveBits",
  "wrapKey",
  "unwrapKey",
];

const aesJwkAlg = {
  "AES-CTR": {
    128: "A128CTR",
    192: "A192CTR",
    256: "A256CTR",
  },
  "AES-CBC": {
    128: "A128CBC",
    192: "A192CBC",
    256: "A256CBC",
  },
  "AES-GCM": {
    128: "A128GCM",
    192: "A192GCM",
    256: "A256GCM",
  },
  "AES-KW": {
    128: "A128KW",
    192: "A192KW",
    256: "A256KW",
  },
};

export function usageIntersection(a: readonly string[], b: readonly string[]) {
  return a.filter((i) => b.includes(i));
}

export function rsa(
  format: "raw" | "jwk" | "pkcs8" | "spki",
  normalizedAlgorithm: z.infer<typeof rsaHashedImportParams> & {
    name: "RSASSA-PKCS1-v1_5" | "RSA-PSS" | "RSA-OAEP";
  },
  keyData: any, // type depends on `format`
  extractable: boolean,
  keyUsages: string[],
) {
  switch (format) {
    case "pkcs8": {
      // 1.
      if (
        keyUsages.find(
          (u) =>
            !SUPPORTED_KEY_USAGES[normalizedAlgorithm.name].private.includes(u),
        ) !== undefined
      ) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      // 2-9.
      const { modulusLength, publicExponent, rawData } = performOp(
        "crypto/importKey",
        {
          algorithm: normalizedAlgorithm.name,
          // Needed to perform step 7 without normalization.
          hash: normalizedAlgorithm.hash.name,
        },
        { pkcs8: keyData },
      );

      const handle = {};
      KEY_STORE.set(handle, rawData);

      const algorithm = {
        name: normalizedAlgorithm.name,
        modulusLength,
        publicExponent,
        hash: normalizedAlgorithm.hash,
      };

      const key = new CryptoKey(
        "private",
        extractable,
        usageIntersection(keyUsages, recognisedUsages),
        algorithm,
        handle,
      );

      return key;
    }
    case "spki": {
      // 1.
      if (
        keyUsages.find(
          (u) =>
            !SUPPORTED_KEY_USAGES[normalizedAlgorithm.name].public.includes(u),
        ) !== undefined
      ) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      // 2-9.
      const { modulusLength, publicExponent, rawData } = performOp(
        "crypto/importKey",
        {
          algorithm: normalizedAlgorithm.name,
          // Needed to perform step 7 without normalization.
          hash: normalizedAlgorithm.hash.name,
        },
        { spki: keyData },
      );

      const handle = {};
      KEY_STORE.set(handle, rawData);

      const algorithm = {
        name: normalizedAlgorithm.name,
        modulusLength,
        publicExponent,
        hash: normalizedAlgorithm.hash,
      };

      const key = new CryptoKey(
        "public",
        extractable,
        usageIntersection(keyUsages, recognisedUsages),
        algorithm,
        handle,
      );

      return key;
    }
    case "jwk": {
      // 1.
      const jwk = keyData;

      // 2.
      if (jwk.d !== undefined) {
        if (
          keyUsages.find(
            (u) =>
              !SUPPORTED_KEY_USAGES[normalizedAlgorithm.name].private.includes(
                u,
              ),
          ) !== undefined
        ) {
          throw new DOMException("Invalid key usages", "SyntaxError");
        }
      } else if (
        keyUsages.find(
          (u) =>
            !SUPPORTED_KEY_USAGES[normalizedAlgorithm.name].public.includes(u),
        ) !== undefined
      ) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      // 3.
      if (jwk.kty.toUpperCase() !== "RSA") {
        throw new DOMException(
          "'kty' property of JsonWebKey must be 'RSA'",
          "DataError",
        );
      }

      // 4.
      if (
        keyUsages.length > 0 &&
        jwk.use !== undefined &&
        jwk.use.toLowerCase() !==
          SUPPORTED_KEY_USAGES[normalizedAlgorithm.name].jwkUse
      ) {
        throw new DOMException(
          `'use' property of JsonWebKey must be '${
            SUPPORTED_KEY_USAGES[normalizedAlgorithm.name].jwkUse
          }'`,
          "DataError",
        );
      }

      // 5.
      if (jwk.key_ops !== undefined) {
        if (
          jwk.key_ops.find((u: string) => !recognisedUsages.includes(u)) !==
          undefined
        ) {
          throw new DOMException(
            "'key_ops' property of JsonWebKey is invalid",
            "DataError",
          );
        }

        if (!jwk.key_ops.every((u: string) => keyUsages.includes(u))) {
          throw new DOMException(
            "'key_ops' property of JsonWebKey is invalid",
            "DataError",
          );
        }
      }

      if (jwk.ext === false && extractable === true) {
        throw new DOMException(
          "'ext' property of JsonWebKey must not be false if extractable is true",
          "DataError",
        );
      }

      // 7.
      let hash;

      // 8.
      if (normalizedAlgorithm.name === "RSASSA-PKCS1-v1_5") {
        switch (jwk.alg) {
          case undefined:
            hash = undefined;
            break;
          case "RS1":
            hash = "SHA-1";
            break;
          case "RS256":
            hash = "SHA-256";
            break;
          case "RS384":
            hash = "SHA-384";
            break;
          case "RS512":
            hash = "SHA-512";
            break;
          default:
            throw new DOMException(
              `'alg' property of JsonWebKey must be one of 'RS1', 'RS256', 'RS384', 'RS512'`,
              "DataError",
            );
        }
      } else if (normalizedAlgorithm.name === "RSA-PSS") {
        switch (jwk.alg) {
          case undefined:
            hash = undefined;
            break;
          case "PS1":
            hash = "SHA-1";
            break;
          case "PS256":
            hash = "SHA-256";
            break;
          case "PS384":
            hash = "SHA-384";
            break;
          case "PS512":
            hash = "SHA-512";
            break;
          default:
            throw new DOMException(
              `'alg' property of JsonWebKey must be one of 'PS1', 'PS256', 'PS384', 'PS512'`,
              "DataError",
            );
        }
      } else {
        switch (jwk.alg) {
          case undefined:
            hash = undefined;
            break;
          case "RSA-OAEP":
            hash = "SHA-1";
            break;
          case "RSA-OAEP-256":
            hash = "SHA-256";
            break;
          case "RSA-OAEP-384":
            hash = "SHA-384";
            break;
          case "RSA-OAEP-512":
            hash = "SHA-512";
            break;
          default:
            throw new DOMException(
              `'alg' property of JsonWebKey must be one of 'RSA-OAEP', 'RSA-OAEP-256', 'RSA-OAEP-384', or 'RSA-OAEP-512'`,
              "DataError",
            );
        }
      }

      // 9.
      if (hash !== undefined) {
        // 9.1.
        const normalizedHash = normalizeAlgorithmDigest(hash);

        // 9.2.
        if (normalizedHash.name !== normalizedAlgorithm.hash.name) {
          throw new DOMException(
            `'alg' property of JsonWebKey must be '${normalizedAlgorithm.name}'`,
            "DataError",
          );
        }
      }

      // 10.
      if (jwk.d !== undefined) {
        // Private key
        const optimizationsPresent =
          jwk.p !== undefined ||
          jwk.q !== undefined ||
          jwk.dp !== undefined ||
          jwk.dq !== undefined ||
          jwk.qi !== undefined;
        if (optimizationsPresent) {
          if (jwk.q === undefined) {
            throw new DOMException(
              "'q' property of JsonWebKey is required for private keys",
              "DataError",
            );
          }
          if (jwk.dp === undefined) {
            throw new DOMException(
              "'dp' property of JsonWebKey is required for private keys",
              "DataError",
            );
          }
          if (jwk.dq === undefined) {
            throw new DOMException(
              "'dq' property of JsonWebKey is required for private keys",
              "DataError",
            );
          }
          if (jwk.qi === undefined) {
            throw new DOMException(
              "'qi' property of JsonWebKey is required for private keys",
              "DataError",
            );
          }
          if (jwk.oth !== undefined) {
            throw new DOMException(
              "'oth' property of JsonWebKey is not supported",
              "NotSupportedError",
            );
          }
        } else {
          throw new DOMException(
            "only optimized private keys are supported",
            "NotSupportedError",
          );
        }

        const { modulusLength, publicExponent, rawData } = performOp(
          "crypto/importKey",
          {
            algorithm: normalizedAlgorithm.name,
            hash: normalizedAlgorithm.hash.name,
          },
          { jwkPrivateRsa: jwk },
        );

        const handle = {};
        KEY_STORE.set(handle, rawData);

        const algorithm = {
          name: normalizedAlgorithm.name,
          modulusLength,
          publicExponent,
          hash: normalizedAlgorithm.hash,
        };

        const key = new CryptoKey(
          "private",
          extractable,
          usageIntersection(keyUsages, recognisedUsages),
          algorithm,
          handle,
        );

        return key;
      } else {
        // Validate that this is a valid public key.
        if (jwk.n === undefined) {
          throw new DOMException(
            "'n' property of JsonWebKey is required for public keys",
            "DataError",
          );
        }
        if (jwk.e === undefined) {
          throw new DOMException(
            "'e' property of JsonWebKey is required for public keys",
            "DataError",
          );
        }

        const { modulusLength, publicExponent, rawData } = performOp(
          "crypto/importKey",
          {
            algorithm: normalizedAlgorithm.name,
            hash: normalizedAlgorithm.hash.name,
          },
          { jwkPublicRsa: jwk },
        );

        const handle = {};
        KEY_STORE.set(handle, rawData);

        const algorithm = {
          name: normalizedAlgorithm.name,
          modulusLength,
          publicExponent,
          hash: normalizedAlgorithm.hash,
        };

        const key = new CryptoKey(
          "public",
          extractable,
          usageIntersection(keyUsages, recognisedUsages),
          algorithm,
          handle,
        );

        return key;
      }
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }
}

export function ec(
  format: string,
  normalizedAlgorithm:
    | { name: "ECDSA"; namedCurve: string }
    | { name: "ECDH"; namedCurve: string },
  keyData: any, // type depends on `format`
  extractable: boolean,
  keyUsages: readonly string[],
) {
  const supportedUsages = SUPPORTED_KEY_USAGES[normalizedAlgorithm.name];

  switch (format) {
    case "raw": {
      // 1.
      if (
        !ArrayPrototypeIncludes(
          supportedNamedCurves,
          normalizedAlgorithm.namedCurve,
        )
      ) {
        throw new DOMException("Invalid namedCurve", "DataError");
      }

      // 2.
      if (
        ArrayPrototypeFind(
          keyUsages,
          (u) =>
            !ArrayPrototypeIncludes(
              SUPPORTED_KEY_USAGES[normalizedAlgorithm.name].public,
              u,
            ),
        ) !== undefined
      ) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      // 3.
      const { rawData } = performOp(
        "crypto/importKey",
        {
          algorithm: normalizedAlgorithm.name,
          namedCurve: normalizedAlgorithm.namedCurve,
        },
        { raw: keyData },
      );

      const handle = {};
      WeakMapPrototypeSet(KEY_STORE, handle, rawData);

      // 4-5.
      const algorithm = {
        name: normalizedAlgorithm.name,
        namedCurve: normalizedAlgorithm.namedCurve,
      };

      // 6-8.
      const key = new CryptoKey(
        "public",
        extractable,
        usageIntersection(keyUsages, recognisedUsages),
        algorithm,
        handle,
      );

      return key;
    }
    case "pkcs8": {
      // 1.
      if (
        ArrayPrototypeFind(
          keyUsages,
          (u) =>
            !ArrayPrototypeIncludes(
              SUPPORTED_KEY_USAGES[normalizedAlgorithm.name].private,
              u,
            ),
        ) !== undefined
      ) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      // 2-9.
      const { rawData } = performOp(
        "crypto/importKey",
        {
          algorithm: normalizedAlgorithm.name,
          namedCurve: normalizedAlgorithm.namedCurve,
        },
        { pkcs8: keyData },
      );

      const handle = {};
      WeakMapPrototypeSet(KEY_STORE, handle, rawData);

      const algorithm = {
        name: normalizedAlgorithm.name,
        namedCurve: normalizedAlgorithm.namedCurve,
      };

      const key = new CryptoKey(
        "private",
        extractable,
        usageIntersection(keyUsages, recognisedUsages),
        algorithm,
        handle,
      );

      return key;
    }
    case "spki": {
      // 1.
      if (normalizedAlgorithm.name === "ECDSA") {
        if (
          ArrayPrototypeFind(
            keyUsages,
            (u) =>
              !ArrayPrototypeIncludes(
                SUPPORTED_KEY_USAGES[normalizedAlgorithm.name].public,
                u,
              ),
          ) !== undefined
        ) {
          throw new DOMException("Invalid key usages", "SyntaxError");
        }
      } else if (keyUsages.length !== 0) {
        throw new DOMException("Key usage must be empty", "SyntaxError");
      }

      // 2-12
      const { rawData } = performOp(
        "crypto/importKey",
        {
          algorithm: normalizedAlgorithm.name,
          namedCurve: normalizedAlgorithm.namedCurve,
        },
        { spki: keyData },
      );

      const handle = {};
      WeakMapPrototypeSet(KEY_STORE, handle, rawData);

      const algorithm = {
        name: normalizedAlgorithm.name,
        namedCurve: normalizedAlgorithm.namedCurve,
      };

      // 6-8.
      const key = new CryptoKey(
        "public",
        extractable,
        usageIntersection(keyUsages, recognisedUsages),
        algorithm,
        handle,
      );

      return key;
    }
    case "jwk": {
      const jwk = keyData;

      const keyType = jwk.d !== undefined ? "private" : "public";

      // 2.
      if (
        ArrayPrototypeFind(
          keyUsages,
          (u) => !ArrayPrototypeIncludes(supportedUsages[keyType], u),
        ) !== undefined
      ) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      // 3.
      if (jwk.kty !== "EC") {
        throw new DOMException(
          "'kty' property of JsonWebKey must be 'EC'",
          "DataError",
        );
      }

      // 4.
      if (
        keyUsages.length > 0 &&
        jwk.use !== undefined &&
        jwk.use !== supportedUsages.jwkUse
      ) {
        throw new DOMException(
          `'use' property of JsonWebKey must be '${supportedUsages.jwkUse}'`,
          "DataError",
        );
      }

      // 5.
      // Section 4.3 of RFC7517
      if (jwk.key_ops !== undefined) {
        if (
          ArrayPrototypeFind(
            jwk.key_ops,
            (u) => !ArrayPrototypeIncludes(recognisedUsages, u),
          ) !== undefined
        ) {
          throw new DOMException(
            "'key_ops' member of JsonWebKey is invalid",
            "DataError",
          );
        }

        if (
          !ArrayPrototypeEvery(jwk.key_ops, (u) =>
            ArrayPrototypeIncludes(keyUsages, u),
          )
        ) {
          throw new DOMException(
            "'key_ops' member of JsonWebKey is invalid",
            "DataError",
          );
        }
      }

      // 6.
      if (jwk.ext === false && extractable === true) {
        throw new DOMException(
          "'ext' property of JsonWebKey must not be false if extractable is true",
          "DataError",
        );
      }

      // 9.
      if (jwk.alg !== undefined && normalizedAlgorithm.name === "ECDSA") {
        let algNamedCurve: string;

        switch (jwk.alg) {
          case "ES256": {
            algNamedCurve = "P-256";
            break;
          }
          case "ES384": {
            algNamedCurve = "P-384";
            break;
          }
          case "ES512": {
            algNamedCurve = "P-521";
            break;
          }
          default:
            throw new DOMException(
              "Curve algorithm not supported",
              "DataError",
            );
        }

        if (algNamedCurve) {
          if (algNamedCurve !== normalizedAlgorithm.namedCurve) {
            throw new DOMException("Mismatched curve algorithm", "DataError");
          }
        }
      }

      // Validate that this is a valid public key.
      if (jwk.x === undefined) {
        throw new DOMException(
          "'x' property of JsonWebKey is required for EC keys",
          "DataError",
        );
      }
      if (jwk.y === undefined) {
        throw new DOMException(
          "'y' property of JsonWebKey is required for EC keys",
          "DataError",
        );
      }

      if (jwk.d !== undefined) {
        // it's also a Private key
        const { rawData } = performOp(
          "crypto/importKey",
          {
            algorithm: normalizedAlgorithm.name,
            namedCurve: normalizedAlgorithm.namedCurve,
          },
          { jwkPrivateEc: jwk },
        );

        const handle = {};
        WeakMapPrototypeSet(KEY_STORE, handle, rawData);

        const algorithm = {
          name: normalizedAlgorithm.name,
          namedCurve: normalizedAlgorithm.namedCurve,
        };

        const key = new CryptoKey(
          "private",
          extractable,
          usageIntersection(keyUsages, recognisedUsages),
          algorithm,
          handle,
        );

        return key;
      } else {
        const { rawData } = performOp(
          "crypto/importKey",
          {
            algorithm: normalizedAlgorithm.name,
            namedCurve: normalizedAlgorithm.namedCurve,
          },
          { jwkPublicEc: jwk },
        );

        const handle = {};
        WeakMapPrototypeSet(KEY_STORE, handle, rawData);

        const algorithm = {
          name: normalizedAlgorithm.name,
          namedCurve: normalizedAlgorithm.namedCurve,
        };

        const key = new CryptoKey(
          "public",
          extractable,
          usageIntersection(keyUsages, recognisedUsages),
          algorithm,
          handle,
        );

        return key;
      }
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }
}

export function aes(
  format: string,
  normalizedAlgorithm: { name: "AES-CTR" | "AES-CBC" | "AES-GCM" | "AES-KW" },
  keyData: any, // type depends on `format`
  extractable: boolean,
  keyUsages: readonly string[],
) {
  const supportedKeyUsages =
    SUPPORTED_SYMMETRIC_KEY_USAGES[normalizedAlgorithm.name];

  // 1.
  if (
    ArrayPrototypeFind(
      keyUsages,
      (u) => !ArrayPrototypeIncludes(supportedKeyUsages, u),
    ) !== undefined
  ) {
    throw new DOMException("Invalid key usages", "SyntaxError");
  }

  const algorithmName = normalizedAlgorithm.name;

  // 2.
  let data = keyData;

  switch (format) {
    case "raw": {
      // 2.
      if (
        !ArrayPrototypeIncludes(
          [128, 192, 256],
          TypedArrayPrototypeGetByteLength(keyData) * 8,
        )
      ) {
        throw new DOMException("Invalid key length", "DataError");
      }

      break;
    }
    case "jwk": {
      // 1.
      const jwk = keyData;

      // 2.
      if (jwk.kty !== "oct") {
        throw new DOMException(
          "'kty' property of JsonWebKey must be 'oct'",
          "DataError",
        );
      }

      // Section 6.4.1 of RFC7518
      if (jwk.k === undefined) {
        throw new DOMException(
          "'k' property of JsonWebKey must be present",
          "DataError",
        );
      }

      // 4.
      const { rawData } = performOp(
        "crypto/importKey",
        { algorithm: "AES" },
        { jwkSecret: jwk },
      );
      data = rawData.data;

      // 5.
      switch (TypedArrayPrototypeGetByteLength(data) * 8) {
        case 128:
          if (
            jwk.alg !== undefined &&
            jwk.alg !== aesJwkAlg[algorithmName][128]
          ) {
            throw new DOMException("Invalid algorithm", "DataError");
          }
          break;
        case 192:
          if (
            jwk.alg !== undefined &&
            jwk.alg !== aesJwkAlg[algorithmName][192]
          ) {
            throw new DOMException("Invalid algorithm", "DataError");
          }
          break;
        case 256:
          if (
            jwk.alg !== undefined &&
            jwk.alg !== aesJwkAlg[algorithmName][256]
          ) {
            throw new DOMException("Invalid algorithm", "DataError");
          }
          break;
        default:
          throw new DOMException("Invalid key length", "DataError");
      }

      // 6.
      if (keyUsages.length > 0 && jwk.use !== undefined && jwk.use !== "enc") {
        throw new DOMException("Invalid key usages", "DataError");
      }

      // 7.
      // Section 4.3 of RFC7517
      if (jwk.key_ops !== undefined) {
        if (
          ArrayPrototypeFind(
            jwk.key_ops,
            (u) => !ArrayPrototypeIncludes(recognisedUsages, u),
          ) !== undefined
        ) {
          throw new DOMException(
            "'key_ops' property of JsonWebKey is invalid",
            "DataError",
          );
        }

        if (
          !ArrayPrototypeEvery(jwk.key_ops, (u) =>
            ArrayPrototypeIncludes(keyUsages, u),
          )
        ) {
          throw new DOMException(
            "'key_ops' property of JsonWebKey is invalid",
            "DataError",
          );
        }
      }

      // 8.
      if (jwk.ext === false && extractable === true) {
        throw new DOMException(
          "'ext' property of JsonWebKey must not be false if extractable is true",
          "DataError",
        );
      }

      break;
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }

  const handle = {};
  WeakMapPrototypeSet(KEY_STORE, handle, {
    type: "secret",
    data,
  });

  // 4-7.
  const algorithm = {
    name: algorithmName,
    length: TypedArrayPrototypeGetByteLength(data) * 8,
  };

  const key = new CryptoKey(
    "secret",
    extractable,
    usageIntersection(keyUsages, recognisedUsages),
    algorithm,
    handle,
  );

  // 8.
  return key;
}

export function hkdf(
  format: string,
  keyData: BufferSource,
  extractable: boolean,
  keyUsages: readonly string[],
) {
  if (format !== "raw") {
    throw new DOMException("Format not supported", "NotSupportedError");
  }

  // 1.
  if (
    ArrayPrototypeFind(
      keyUsages,
      (u) => !ArrayPrototypeIncludes(["deriveKey", "deriveBits"], u),
    ) !== undefined
  ) {
    throw new DOMException("Invalid key usages", "SyntaxError");
  }

  // 2.
  if (extractable !== false) {
    throw new DOMException("Key must not be extractable", "SyntaxError");
  }

  // 3.
  const handle = {};
  WeakMapPrototypeSet(KEY_STORE, handle, {
    type: "secret",
    data: keyData,
  });

  // 4-8.
  const algorithm = {
    name: "HKDF",
  };
  const key = new CryptoKey(
    "secret",
    false,
    usageIntersection(keyUsages, recognisedUsages),
    algorithm,
    handle,
  );

  // 9.
  return key;
}

export function ed25519(
  format: string,
  keyData: any, // type depends on `format`
  extractable: boolean,
  keyUsages: readonly string[],
) {
  switch (format) {
    case "raw": {
      // 1.
      if (
        ArrayPrototypeFind(
          keyUsages,
          (u) => !ArrayPrototypeIncludes(["verify"], u),
        ) !== undefined
      ) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      const handle = {};
      WeakMapPrototypeSet(KEY_STORE, handle, keyData);

      // 2-3.
      const algorithm = {
        name: "Ed25519",
      };

      // 4-6.
      return new CryptoKey(
        "public",
        extractable,
        usageIntersection(keyUsages, recognisedUsages),
        algorithm,
        handle,
      );
    }
    case "spki": {
      // 1.
      if (
        ArrayPrototypeFind(
          keyUsages,
          (u) => !ArrayPrototypeIncludes(["verify"], u),
        ) !== undefined
      ) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      const publicKeyData = performOp("crypto/importSpkiEd25519", keyData);
      if (publicKeyData === null) {
        throw new DOMException("Invalid key data", "DataError");
      }

      const handle = {};
      WeakMapPrototypeSet(KEY_STORE, handle, publicKeyData.buffer);

      const algorithm = {
        name: "Ed25519",
      };

      return new CryptoKey(
        "public",
        extractable,
        usageIntersection(keyUsages, recognisedUsages),
        algorithm,
        handle,
      );
    }
    case "pkcs8": {
      // 1.
      if (
        ArrayPrototypeFind(
          keyUsages,
          (u) => !ArrayPrototypeIncludes(["sign"], u),
        ) !== undefined
      ) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      const privateKeyData = performOp("crypto/importPkcs8Ed25519", keyData);
      if (privateKeyData === null) {
        throw new DOMException("Invalid key data", "DataError");
      }

      const handle = {};
      WeakMapPrototypeSet(KEY_STORE, handle, privateKeyData.buffer);

      const algorithm = {
        name: "Ed25519",
      };

      return new CryptoKey(
        "private",
        extractable,
        usageIntersection(keyUsages, recognisedUsages),
        algorithm,
        handle,
      );
    }
    case "jwk": {
      // 1.
      const jwk = keyData;

      // 2.
      if (jwk.d !== undefined) {
        if (
          ArrayPrototypeFind(
            keyUsages,
            (u) => !ArrayPrototypeIncludes(["sign"], u),
          ) !== undefined
        ) {
          throw new DOMException("Invalid key usages", "SyntaxError");
        }
      } else {
        if (
          ArrayPrototypeFind(
            keyUsages,
            (u) => !ArrayPrototypeIncludes(["verify"], u),
          ) !== undefined
        ) {
          throw new DOMException("Invalid key usages", "SyntaxError");
        }
      }

      // 3.
      if (jwk.kty !== "OKP") {
        throw new DOMException("Invalid key type", "DataError");
      }

      // 4.
      if (jwk.crv !== "Ed25519") {
        throw new DOMException("Invalid curve", "DataError");
      }

      // 5.
      if (jwk.alg !== undefined && jwk.alg !== "EdDSA") {
        throw new DOMException("Invalid algorithm", "DataError");
      }

      // 6.
      if (keyUsages.length > 0 && jwk.use !== undefined && jwk.use !== "sig") {
        throw new DOMException("Invalid key usage", "DataError");
      }

      // 7.
      if (jwk.key_ops !== undefined) {
        if (
          ArrayPrototypeFind(
            jwk.key_ops,
            (u) => !ArrayPrototypeIncludes(recognisedUsages, u),
          ) !== undefined
        ) {
          throw new DOMException(
            "'key_ops' property of JsonWebKey is invalid",
            "DataError",
          );
        }

        if (
          !ArrayPrototypeEvery(jwk.key_ops, (u) =>
            ArrayPrototypeIncludes(keyUsages, u),
          )
        ) {
          throw new DOMException(
            "'key_ops' property of JsonWebKey is invalid",
            "DataError",
          );
        }
      }

      // 8.
      if (jwk.ext !== undefined && jwk.ext === false && extractable) {
        throw new DOMException("Invalid key extractability", "DataError");
      }

      // 9.
      if (jwk.d !== undefined) {
        // https://www.rfc-editor.org/rfc/rfc8037#section-2
        let privateKeyData;
        try {
          privateKeyData = performOp("crypto/base64UrlDecode", jwk.d);
        } catch {
          throw new DOMException("invalid private key data", "DataError");
        }

        const handle = {};
        WeakMapPrototypeSet(KEY_STORE, handle, privateKeyData);

        const algorithm = {
          name: "Ed25519",
        };

        return new CryptoKey(
          "private",
          extractable,
          usageIntersection(keyUsages, recognisedUsages),
          algorithm,
          handle,
        );
      } else {
        // https://www.rfc-editor.org/rfc/rfc8037#section-2
        let publicKeyData;
        try {
          publicKeyData = performOp("crypto/base64UrlDecode", jwk.x);
        } catch {
          throw new DOMException("invalid public key data", "DataError");
        }

        const handle = {};
        WeakMapPrototypeSet(KEY_STORE, handle, publicKeyData);

        const algorithm = {
          name: "Ed25519",
        };

        return new CryptoKey(
          "public",
          extractable,
          usageIntersection(keyUsages, recognisedUsages),
          algorithm,
          handle,
        );
      }
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }
}

export function x25519(
  format: string,
  keyData: any, // type depends on `format`
  extractable: boolean,
  keyUsages: readonly string[],
) {
  switch (format) {
    case "raw": {
      // 1.
      if (keyUsages.length > 0) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      const handle = {};
      WeakMapPrototypeSet(KEY_STORE, handle, keyData);

      // 2-3.
      const algorithm = {
        name: "X25519",
      };

      // 4-6.
      return new CryptoKey("public", extractable, [], algorithm, handle);
    }
    case "spki": {
      // 1.
      if (keyUsages.length > 0) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      const publicKeyData = performOp("crypto/importSpkiX25519", keyData);
      if (publicKeyData === null) {
        throw new DOMException("Invalid key data", "DataError");
      }

      const handle = {};
      WeakMapPrototypeSet(KEY_STORE, handle, publicKeyData.buffer);

      const algorithm = {
        name: "X25519",
      };

      return new CryptoKey("public", extractable, [], algorithm, handle);
    }
    case "pkcs8": {
      // 1.
      if (
        ArrayPrototypeFind(
          keyUsages,
          (u) => !ArrayPrototypeIncludes(["deriveKey", "deriveBits"], u),
        ) !== undefined
      ) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      const privateKeyData = performOp("crypto/importPkcs8X25519", keyData);
      if (privateKeyData === null) {
        throw new DOMException("Invalid key data", "DataError");
      }

      const handle = {};
      WeakMapPrototypeSet(KEY_STORE, handle, privateKeyData.buffer);

      const algorithm = {
        name: "X25519",
      };

      return new CryptoKey(
        "private",
        extractable,
        usageIntersection(keyUsages, recognisedUsages),
        algorithm,
        handle,
      );
    }
    case "jwk": {
      // 1.
      const jwk = keyData;

      // 2.
      if (jwk.d !== undefined) {
        if (
          ArrayPrototypeFind(
            keyUsages,
            (u) => !ArrayPrototypeIncludes(["deriveKey", "deriveBits"], u),
          ) !== undefined
        ) {
          throw new DOMException("Invalid key usages", "SyntaxError");
        }
      }

      // 3.
      if (jwk.d === undefined && keyUsages.length > 0) {
        throw new DOMException("Invalid key usages", "SyntaxError");
      }

      // 4.
      if (jwk.kty !== "OKP") {
        throw new DOMException("Invalid key type", "DataError");
      }

      // 5.
      if (jwk.crv !== "X25519") {
        throw new DOMException("Invalid curve", "DataError");
      }

      // 6.
      if (keyUsages.length > 0 && jwk.use !== undefined) {
        if (jwk.use !== "enc") {
          throw new DOMException("Invalid key use", "DataError");
        }
      }

      // 7.
      if (jwk.key_ops !== undefined) {
        if (
          ArrayPrototypeFind(
            jwk.key_ops,
            (u) => !ArrayPrototypeIncludes(recognisedUsages, u),
          ) !== undefined
        ) {
          throw new DOMException(
            "'key_ops' property of JsonWebKey is invalid",
            "DataError",
          );
        }

        if (
          !ArrayPrototypeEvery(jwk.key_ops, (u) =>
            ArrayPrototypeIncludes(keyUsages, u),
          )
        ) {
          throw new DOMException(
            "'key_ops' property of JsonWebKey is invalid",
            "DataError",
          );
        }
      }

      // 8.
      if (jwk.ext !== undefined && jwk.ext === false && extractable) {
        throw new DOMException("Invalid key extractability", "DataError");
      }

      // 9.
      if (jwk.d !== undefined) {
        // https://www.rfc-editor.org/rfc/rfc8037#section-2
        const privateKeyData = performOp("crypto/base64UrlDecode", jwk.d);

        const handle = {};
        WeakMapPrototypeSet(KEY_STORE, handle, privateKeyData);

        const algorithm = {
          name: "X25519",
        };

        return new CryptoKey(
          "private",
          extractable,
          usageIntersection(keyUsages, ["deriveKey", "deriveBits"]),
          algorithm,
          handle,
        );
      } else {
        // https://www.rfc-editor.org/rfc/rfc8037#section-2
        const publicKeyData = performOp("crypto/base64UrlDecode", jwk.x);

        const handle = {};
        WeakMapPrototypeSet(KEY_STORE, handle, publicKeyData);

        const algorithm = {
          name: "X25519",
        };

        return new CryptoKey("public", extractable, [], algorithm, handle);
      }
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }
}
