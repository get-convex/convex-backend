// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/00_crypto.js

import {
  requiredArguments,
  throwNotImplementedMethodError,
  throwUncatchableDeveloperError,
} from "./helpers.js";
import { performOp } from "udf-syscall-ffi";
import {
  ArrayPrototypeIncludes,
  WeakMapPrototypeGet,
  copyBuffer,
} from "./crypto/helpers.js";
import {
  normalizeAlgorithmDeriveBits,
  normalizeAlgorithmDigest,
  normalizeAlgorithmGetKeyLength,
  normalizeAlgorithmImportKey,
  normalizeAlgorithmSign,
  normalizeAlgorithmVerify,
} from "./crypto/normalize_algorithm.js";
import {
  KEY_STORE,
  CryptoKey,
  _handle,
  _type,
  _algorithm,
  _usages,
} from "./crypto/crypto_key.js";
import * as ImportKey from "./crypto/import_key.js";
import * as ExportKey from "./crypto/export_key.js";
import { deriveBits } from "./crypto/derive_bits.js";
import getKeyLength from "./crypto/get_key_length.js";

class Crypto {
  constructor() {
    throwUncatchableDeveloperError("Illegal constructor: Crypto");
  }

  getRandomValues(typedArray: ArrayBufferView) {
    const prefix = "Failed to execute 'getRandomValues' on 'Crypto'";
    requiredArguments(arguments.length, 1, prefix);
    if (
      typedArray instanceof Float32Array ||
      typedArray instanceof Float64Array
    ) {
      throw new DOMException(
        "The provided ArrayBufferView is not an integer array type",
        "TypeMismatchError",
      );
    }
    const randomValues = performOp(
      "crypto/getRandomValues",
      typedArray.byteLength,
    );
    // TODO: CX-4404
    if (typedArray instanceof Uint8Array) {
      typedArray.set(randomValues);
      return typedArray;
    }
    const ui8 = new Uint8Array(
      typedArray.buffer,
      typedArray.byteOffset,
      typedArray.byteLength,
    );
    ui8.set(randomValues);
    return typedArray;
  }

  randomUUID() {
    return performOp("crypto/randomUUID");
  }

  get subtle() {
    return Object.create(SubtleCrypto.prototype);
  }

  inspect() {
    return "Crypto {}";
  }
}

class SubtleCrypto {
  constructor() {
    throwUncatchableDeveloperError("Illegal constructor: SubtleCrypto");
  }

  async digest(
    algorithm: AlgorithmIdentifier,
    data: BufferSource,
  ): Promise<ArrayBuffer> {
    const prefix = "Failed to execute 'digest' on 'SubtleCrypto'";
    requiredArguments(arguments.length, 2, prefix);

    data = copyBuffer(data);

    algorithm = normalizeAlgorithmDigest(algorithm);

    const result = performOp("crypto/digest", algorithm.name, data);

    return result.buffer;
  }

  async encrypt() {
    throwNotImplementedMethodError("encrypt", "SubtleCrypto");
  }

  async decrypt() {
    throwNotImplementedMethodError("decrypt", "SubtleCrypto");
  }

  async sign(
    algorithm: AlgorithmIdentifier | RsaPssParams | EcdsaParams,
    key: CryptoKey,
    data: BufferSource,
  ): Promise<ArrayBuffer> {
    const prefix = "Failed to execute 'sign' on 'SubtleCrypto'";
    requiredArguments(arguments.length, 3, prefix);

    // TODO: real input validation - CX-4399

    // 1.
    const dataCopy = copyBuffer(data);

    // 2.
    const normalizedAlgorithm = normalizeAlgorithmSign(algorithm);

    const handle = key[_handle];
    const keyData = KEY_STORE.get(handle).data;

    // 8.
    if (normalizedAlgorithm.name !== key.algorithm.name) {
      throw new DOMException(
        "Signing algorithm doesn't match key algorithm.",
        "InvalidAccessError",
      );
    }

    // 9.
    if (!key.usages.includes("sign")) {
      throw new DOMException(
        "Key does not support the 'sign' operation.",
        "InvalidAccessError",
      );
    }

    switch (normalizedAlgorithm.name) {
      case "RSASSA-PKCS1-v1_5": {
        // 1.
        if (key[_type] !== "private") {
          throw new DOMException(
            "Key type not supported",
            "InvalidAccessError",
          );
        }

        // 2.
        const hashAlgorithm = key[_algorithm].hash.name;
        const signature = performOp("crypto/sign", {
          key: keyData,
          algorithm: "RSASSA-PKCS1-v1_5",
          hash: hashAlgorithm,
          data: dataCopy,
        });

        return signature.buffer;
      }
      case "RSA-PSS": {
        // 1.
        if (key[_type] !== "private") {
          throw new DOMException(
            "Key type not supported",
            "InvalidAccessError",
          );
        }

        // 2.
        const hashAlgorithm = key[_algorithm].hash.name;
        const signature = performOp("crypto/sign", {
          key: keyData,
          algorithm: "RSA-PSS",
          hash: hashAlgorithm,
          saltLength: normalizedAlgorithm.saltLength,
          data: dataCopy,
        });

        return signature.buffer;
      }
      case "ECDSA": {
        // 1.
        if (key[_type] !== "private") {
          throw new DOMException(
            "Key type not supported",
            "InvalidAccessError",
          );
        }

        // 2.
        const hashAlgorithm = normalizedAlgorithm.hash.name;
        const namedCurve = key[_algorithm].namedCurve;
        if (
          !ArrayPrototypeIncludes(ImportKey.supportedNamedCurves, namedCurve)
        ) {
          throw new DOMException("Curve not supported", "NotSupportedError");
        }

        if (
          (key[_algorithm].namedCurve === "P-256" &&
            hashAlgorithm !== "SHA-256") ||
          (key[_algorithm].namedCurve === "P-384" &&
            hashAlgorithm !== "SHA-384")
        ) {
          throw new DOMException("Not implemented", "NotSupportedError");
        }

        const signature = performOp("crypto/sign", {
          key: keyData,
          algorithm: "ECDSA",
          hash: hashAlgorithm,
          namedCurve,
          data: dataCopy,
        });

        return signature.buffer;
      }
      case "HMAC": {
        const hashAlgorithm = key.algorithm.hash.name;

        const signature = performOp("crypto/sign", {
          key: keyData,
          algorithm: "HMAC",
          hash: hashAlgorithm,
          data: dataCopy,
        });

        return signature.buffer;
      }
      case "Ed25519": {
        // 1.
        if (key[_type] !== "private") {
          throw new DOMException(
            "Key type not supported",
            "InvalidAccessError",
          );
        }

        // https://briansmith.org/rustdoc/src/ring/ec/curve25519/ed25519/signing.rs.html#260
        const signature = performOp("crypto/signEd25519", keyData, dataCopy);
        if (signature === null) {
          throw new DOMException("Failed to sign", "OperationError");
        }
        return signature.buffer;
      }
    }

    throw new TypeError(`Unknown algorithm name ${normalizedAlgorithm.name}`);
  }

  async importKey(
    format: "jwk" | "pkcs8" | "raw" | "spki",
    keyData: BufferSource,
    algorithm:
      | AlgorithmIdentifier
      | RsaHashedImportParams
      | EcKeyImportParams
      | HmacImportParams
      | AesKeyAlgorithm,
    extractable: boolean,
    keyUsages: KeyUsage[],
  ): Promise<CryptoKey> {
    const prefix = "Failed to execute 'importKey' on 'SubtleCrypto'";
    requiredArguments(arguments.length, 4, prefix);

    if (format === "jwk") {
      if (ArrayBuffer.isView(keyData) || keyData instanceof ArrayBuffer) {
        throw new TypeError("keyData is not a JsonWebKey");
      }
    } else {
      if (ArrayBuffer.isView(keyData) || keyData instanceof ArrayBuffer) {
        keyData = copyBuffer(keyData);
      } else {
        throw new TypeError("keyData is a JsonWebKey");
      }
    }

    const normalizedAlgorithm = normalizeAlgorithmImportKey(algorithm);

    const algorithmName = normalizedAlgorithm.name;

    switch (algorithmName) {
      case "HMAC": {
        return ImportKey.hmac(
          format,
          normalizedAlgorithm,
          keyData,
          extractable,
          keyUsages,
        );
      }
      case "ECDH":
      case "ECDSA": {
        return ImportKey.ec(
          format,
          normalizedAlgorithm,
          keyData,
          extractable,
          keyUsages,
        );
      }
      case "RSASSA-PKCS1-v1_5":
      case "RSA-PSS":
      case "RSA-OAEP": {
        return ImportKey.rsa(
          format,
          normalizedAlgorithm,
          keyData,
          extractable,
          keyUsages,
        );
      }
      case "HKDF": {
        return ImportKey.hkdf(format, keyData, extractable, keyUsages);
      }
      case "PBKDF2": {
        return ImportKey.pbkdf2(format, keyData, extractable, keyUsages);
      }
      case "AES-CTR":
      case "AES-CBC":
      case "AES-GCM": {
        return ImportKey.aes(
          format,
          normalizedAlgorithm,
          keyData,
          extractable,
          keyUsages,
          ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
        );
      }
      case "AES-KW": {
        return ImportKey.aes(
          format,
          normalizedAlgorithm,
          keyData,
          extractable,
          keyUsages,
          ["wrapKey", "unwrapKey"],
        );
      }
      case "X25519": {
        return ImportKey.x25519(format, keyData, extractable, keyUsages);
      }
      case "Ed25519": {
        return ImportKey.ed25519(format, keyData, extractable, keyUsages);
      }
      default:
        throw new DOMException("Not implemented", "NotSupportedError");
    }
  }

  async exportKey(format: "jwk" | "pkcs8" | "raw" | "spki", key: CryptoKey) {
    const prefix = "Failed to execute 'exportKey' on 'SubtleCrypto'";
    requiredArguments(arguments.length, 2, prefix);

    const handle = key[_handle];
    // 2.
    const innerKey = WeakMapPrototypeGet(KEY_STORE, handle);

    const algorithmName = key[_algorithm].name;

    let result;

    switch (algorithmName) {
      case "HMAC": {
        result = ExportKey.hmac(format, key, innerKey);
        break;
      }
      case "RSASSA-PKCS1-v1_5":
      case "RSA-PSS":
      case "RSA-OAEP": {
        result = ExportKey.rsa(format, key, innerKey);
        break;
      }
      case "ECDH":
      case "ECDSA": {
        result = ExportKey.ec(format, key, innerKey);
        break;
      }
      case "Ed25519": {
        result = ExportKey.ed25519(format, key, innerKey);
        break;
      }
      case "X25519": {
        result = ExportKey.x25519(format, key, innerKey);
        break;
      }
      case "AES-CTR":
      case "AES-CBC":
      case "AES-GCM":
      case "AES-KW": {
        result = ExportKey.aes(format, key, innerKey);
        break;
      }
      default:
        throw new DOMException("Not implemented", "NotSupportedError");
    }

    if (key.extractable === false) {
      throw new DOMException("Key is not extractable", "InvalidAccessError");
    }

    return result;
  }

  async deriveBits(
    algorithm:
      | AlgorithmIdentifier
      | EcdhKeyDeriveParams
      | HkdfParams
      | Pbkdf2Params,
    baseKey: CryptoKey,
    length: number,
  ): Promise<ArrayBuffer> {
    const prefix = "Failed to execute 'deriveBits' on 'SubtleCrypto'";
    requiredArguments(arguments.length, 3, prefix);

    // TODO: real input validation - CX-4399

    // 2.
    const normalizedAlgorithm = normalizeAlgorithmDeriveBits(algorithm);
    // 4-6.
    const result = await deriveBits(normalizedAlgorithm, baseKey, length);
    // 7.
    if (normalizedAlgorithm.name !== baseKey[_algorithm].name) {
      throw new DOMException("Invalid algorithm name", "InvalidAccessError");
    }
    // 8.
    if (!baseKey[_usages].includes("deriveBits")) {
      throw new DOMException(
        "baseKey usages does not contain `deriveBits`",
        "InvalidAccessError",
      );
    }
    // 9-10.
    return result;
  }

  async deriveKey(
    algorithm:
      | AlgorithmIdentifier
      | EcdhKeyDeriveParams
      | HkdfParams
      | Pbkdf2Params,
    baseKey: CryptoKey,
    derivedKeyType:
      | AlgorithmIdentifier
      | HkdfParams
      | Pbkdf2Params
      | AesDerivedKeyParams
      | HmacImportParams,
    extractable: boolean,
    keyUsages: Array<KeyUsage>,
  ): Promise<CryptoKey> {
    const prefix = "Failed to execute 'deriveKey' on 'SubtleCrypto'";
    requiredArguments(arguments.length, 5, prefix);
    // TODO: real input validation for buffers.

    // 2-3.
    const normalizedAlgorithm = normalizeAlgorithmDeriveBits(algorithm);

    // 4-5.
    const normalizedDerivedKeyAlgorithmImport =
      normalizeAlgorithmImportKey(derivedKeyType);

    // 6-7.
    const normalizedDerivedKeyAlgorithmLength =
      normalizeAlgorithmGetKeyLength(derivedKeyType);

    // 8-10.

    // 11.
    if (normalizedAlgorithm.name !== baseKey[_algorithm].name) {
      throw new DOMException("Invalid algorithm name", "InvalidAccessError");
    }

    // 12.
    if (!baseKey[_usages].includes("deriveKey")) {
      throw new DOMException(
        "baseKey usages does not contain `deriveKey`",
        "InvalidAccessError",
      );
    }

    // 13.
    const length = getKeyLength(normalizedDerivedKeyAlgorithmLength);

    // 14.
    const secret = await deriveBits(normalizedAlgorithm, baseKey, length);

    // 15.
    const result = await this.importKey(
      "raw",
      secret,
      // @ts-expect-error TODO: figure out why these types don't match up
      normalizedDerivedKeyAlgorithmImport,
      extractable,
      keyUsages,
    );

    // 16.
    if (
      ["private", "secret"].includes(result[_type]) &&
      keyUsages.length === 0
    ) {
      throw new SyntaxError("Invalid key usages");
    }
    // 17.
    return result;
  }

  async verify(
    algorithm: AlgorithmIdentifier | RsaPssParams | EcdsaParams,
    key: CryptoKey,
    signature: BufferSource,
    data: BufferSource,
  ) {
    const prefix = "Failed to execute 'verify' on 'SubtleCrypto'";
    requiredArguments(arguments.length, 4, prefix);

    // TODO: real input validation for buffers.

    // 2.
    const signatureCopy = copyBuffer(signature);

    // 3.
    const dataCopy = copyBuffer(data);

    const normalizedAlgorithm = normalizeAlgorithmVerify(algorithm);

    const handle = key[_handle];
    const keyData = KEY_STORE.get(handle);

    // 8.
    if (normalizedAlgorithm.name !== key.algorithm.name) {
      throw new DOMException(
        "Verifying algorithm doesn't match key algorithm.",
        "InvalidAccessError",
      );
    }

    // 9.
    if (!key.usages.includes("verify")) {
      throw new DOMException(
        "Key does not support the 'verify' operation.",
        "InvalidAccessError",
      );
    }

    switch (normalizedAlgorithm.name) {
      case "RSASSA-PKCS1-v1_5": {
        if (key[_type] !== "public") {
          throw new DOMException(
            "Key type not supported",
            "InvalidAccessError",
          );
        }

        const hashAlgorithm = key[_algorithm].hash.name;
        return await performOp("crypto/verify", {
          key: keyData,
          algorithm: "RSASSA-PKCS1-v1_5",
          hash: hashAlgorithm,
          signature,
          data: dataCopy,
        });
      }
      case "RSA-PSS": {
        if (key[_type] !== "public") {
          throw new DOMException(
            "Key type not supported",
            "InvalidAccessError",
          );
        }

        const hashAlgorithm = key[_algorithm].hash.name;
        return await performOp("crypto/verify", {
          key: keyData,
          algorithm: "RSA-PSS",
          hash: hashAlgorithm,
          signature,
          data: dataCopy,
        });
      }
      case "HMAC": {
        const hash = key[_algorithm].hash.name;
        return performOp("crypto/verify", {
          key: keyData,
          algorithm: "HMAC",
          hash,
          signature: signatureCopy,
          data: dataCopy,
        });
      }
      case "ECDSA": {
        // 1.
        if (key[_type] !== "public") {
          throw new DOMException(
            "Key type not supported",
            "InvalidAccessError",
          );
        }
        // 2.
        const hash = normalizedAlgorithm.hash.name;

        if (
          (key[_algorithm].namedCurve === "P-256" && hash !== "SHA-256") ||
          (key[_algorithm].namedCurve === "P-384" && hash !== "SHA-384")
        ) {
          throw new DOMException("Not implemented", "NotSupportedError");
        }

        // 3-8.
        return await performOp("crypto/verify", {
          key: keyData,
          algorithm: "ECDSA",
          hash,
          signature,
          namedCurve: key[_algorithm].namedCurve,
          data: dataCopy,
        });
      }
      case "Ed25519": {
        // 1.
        if (key[_type] !== "public") {
          throw new DOMException(
            "Key type not supported",
            "InvalidAccessError",
          );
        }

        return performOp("crypto/verifyEd25519", keyData, dataCopy, signature);
      }
    }

    throw new TypeError(`Unknown algorithm name ${normalizedAlgorithm.name}`);
  }

  async wrapKey() {
    throwNotImplementedMethodError("wrapKey", "SubtleCrypto");
  }

  async unwrapKey() {
    throwNotImplementedMethodError("unwrapKey", "SubtleCrypto");
  }

  async generateKey() {
    throwNotImplementedMethodError("generateKey", "SubtleCrypto");
  }

  inspect() {
    return "SubtleCrypto {}";
  }
}

export const setupCrypto = (global: any) => {
  global.Crypto = Crypto;
  global.crypto = Object.create(Crypto.prototype);
  global.CryptoKey = CryptoKey;
  global.SubtleCrypto = SubtleCrypto;
};
