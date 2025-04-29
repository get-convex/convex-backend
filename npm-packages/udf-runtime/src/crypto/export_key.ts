// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/00_crypto.js

import { performOp } from "../syscall.js";
import { CryptoKey, _algorithm, _extractable, _type } from "./crypto_key.js";
import { ObjectAssign, TypedArrayPrototypeGetBuffer } from "./helpers.js";

export function hmac(
  format: "raw" | "jwk" | "pkcs8" | "spki",
  key: CryptoKey,
  innerKey,
) {
  // 1.
  if (innerKey === null || innerKey === undefined) {
    throw new DOMException("Key is not available", "OperationError");
  }

  switch (format) {
    // 3.
    case "raw": {
      const bits = innerKey.data;
      // TODO(petamoriken): Uint8Array doesn't have push method
      // for (let _i = 7 & (8 - bits.length % 8); _i > 0; _i--) {
      //   bits.push(0);
      // }
      // 4-5.
      return TypedArrayPrototypeGetBuffer(bits);
    }
    case "jwk": {
      // 1-2.
      const jwk = {
        kty: "oct",
      } as Record<string, any>;

      // 3.
      const data = performOp(
        "crypto/exportKey",
        {
          format: "jwksecret",
          algorithm: key[_algorithm].name,
        },
        innerKey,
      );
      jwk.k = data.k;

      // 4.
      const algorithm = key[_algorithm];
      // 5.
      const hash = algorithm.hash;
      // 6.
      switch (hash.name) {
        case "SHA-1":
          jwk.alg = "HS1";
          break;
        case "SHA-256":
          jwk.alg = "HS256";
          break;
        case "SHA-384":
          jwk.alg = "HS384";
          break;
        case "SHA-512":
          jwk.alg = "HS512";
          break;
        default:
          throw new DOMException(
            "Hash algorithm not supported",
            "NotSupportedError",
          );
      }
      // 7.
      jwk.key_ops = key.usages;
      // 8.
      jwk.ext = key[_extractable];
      // 9.
      return jwk;
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }
}

export function rsa(format, key, innerKey) {
  switch (format) {
    case "pkcs8": {
      // 1.
      if (key[_type] !== "private") {
        throw new DOMException(
          "Key is not a private key",
          "InvalidAccessError",
        );
      }

      // 2.
      const data = performOp(
        "crypto/exportKey",
        {
          algorithm: key[_algorithm].name,
          format: "pkcs8",
        },
        innerKey,
      );

      // 3.
      return TypedArrayPrototypeGetBuffer(data);
    }
    case "spki": {
      // 1.
      if (key[_type] !== "public") {
        throw new DOMException("Key is not a public key", "InvalidAccessError");
      }

      // 2.
      const data = performOp(
        "crypto/exportKey",
        {
          algorithm: key[_algorithm].name,
          format: "spki",
        },
        innerKey,
      );

      // 3.
      return data.buffer;
    }
    case "jwk": {
      // 1-2.
      const jwk = {
        kty: "RSA",
      } as Record<string, any>;

      // 3.
      const hash = key[_algorithm].hash.name;

      // 4.
      if (key[_algorithm].name === "RSASSA-PKCS1-v1_5") {
        switch (hash) {
          case "SHA-1":
            jwk.alg = "RS1";
            break;
          case "SHA-256":
            jwk.alg = "RS256";
            break;
          case "SHA-384":
            jwk.alg = "RS384";
            break;
          case "SHA-512":
            jwk.alg = "RS512";
            break;
          default:
            throw new DOMException(
              "Hash algorithm not supported",
              "NotSupportedError",
            );
        }
      } else if (key[_algorithm].name === "RSA-PSS") {
        switch (hash) {
          case "SHA-1":
            jwk.alg = "PS1";
            break;
          case "SHA-256":
            jwk.alg = "PS256";
            break;
          case "SHA-384":
            jwk.alg = "PS384";
            break;
          case "SHA-512":
            jwk.alg = "PS512";
            break;
          default:
            throw new DOMException(
              "Hash algorithm not supported",
              "NotSupportedError",
            );
        }
      } else {
        switch (hash) {
          case "SHA-1":
            jwk.alg = "RSA-OAEP";
            break;
          case "SHA-256":
            jwk.alg = "RSA-OAEP-256";
            break;
          case "SHA-384":
            jwk.alg = "RSA-OAEP-384";
            break;
          case "SHA-512":
            jwk.alg = "RSA-OAEP-512";
            break;
          default:
            throw new DOMException(
              "Hash algorithm not supported",
              "NotSupportedError",
            );
        }
      }

      // 5-6.
      const data = performOp(
        "crypto/exportKey",
        {
          format: key[_type] === "private" ? "jwkprivate" : "jwkpublic",
          algorithm: key[_algorithm].name,
        },
        innerKey,
      );
      ObjectAssign(jwk, data);

      // 7.
      jwk.key_ops = key.usages;

      // 8.
      jwk.ext = key[_extractable];

      return jwk;
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }
}

export function ed25519(format, key, innerKey) {
  switch (format) {
    case "raw": {
      // 1.
      if (key[_type] !== "public") {
        throw new DOMException("Key is not a public key", "InvalidAccessError");
      }

      // 2-3.
      return TypedArrayPrototypeGetBuffer(innerKey);
    }
    case "spki": {
      // 1.
      if (key[_type] !== "public") {
        throw new DOMException("Key is not a public key", "InvalidAccessError");
      }

      const spkiDer = performOp("crypto/exportSpkiEd25519", innerKey);
      return TypedArrayPrototypeGetBuffer(spkiDer);
    }
    case "pkcs8": {
      // 1.
      if (key[_type] !== "private") {
        throw new DOMException("Key is not a public key", "InvalidAccessError");
      }

      const pkcs8Der = performOp(
        "crypto/exportPkcs8Ed25519",
        new Uint8Array([0x04, 0x22, ...innerKey]),
      );
      pkcs8Der[15] = 0x20;
      return TypedArrayPrototypeGetBuffer(pkcs8Der);
    }
    case "jwk": {
      const x =
        key[_type] === "private"
          ? performOp("crypto/JwkXEd25519", innerKey)
          : performOp("crypto/base64UrlEncode", innerKey);
      const jwk = {
        kty: "OKP",
        alg: "EdDSA",
        crv: "Ed25519",
        x,
        key_ops: key.usages,
        ext: key[_extractable],
      } as Record<string, any>;
      if (key[_type] === "private") {
        jwk.d = performOp("crypto/base64UrlEncode", innerKey);
      }
      return jwk;
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }
}

export function x25519(format, key, innerKey) {
  switch (format) {
    case "raw": {
      // 1.
      if (key[_type] !== "public") {
        throw new DOMException("Key is not a public key", "InvalidAccessError");
      }

      // 2-3.
      return TypedArrayPrototypeGetBuffer(innerKey);
    }
    case "spki": {
      // 1.
      if (key[_type] !== "public") {
        throw new DOMException("Key is not a public key", "InvalidAccessError");
      }

      const spkiDer = performOp("crypto/exportSpkiX25519", innerKey);
      return TypedArrayPrototypeGetBuffer(spkiDer);
    }
    case "pkcs8": {
      // 1.
      if (key[_type] !== "private") {
        throw new DOMException("Key is not a public key", "InvalidAccessError");
      }

      const pkcs8Der = performOp(
        "crypto/exportPkcs8X25519",
        new Uint8Array([0x04, 0x22, ...innerKey]),
      );
      pkcs8Der[15] = 0x20;
      return TypedArrayPrototypeGetBuffer(pkcs8Der);
    }
    case "jwk": {
      if (key[_type] === "private") {
        throw new DOMException("Not implemented", "NotSupportedError");
      }
      const x = performOp("crypto/base64UrlEncode", innerKey);
      const jwk = {
        kty: "OKP",
        crv: "X25519",
        x,
        key_ops: key.usages,
        ext: key[_extractable],
      };
      return jwk;
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }
}

export function ec(format, key, innerKey) {
  switch (format) {
    case "raw": {
      // 1.
      if (key[_type] !== "public") {
        throw new DOMException("Key is not a public key", "InvalidAccessError");
      }

      // 2.
      const data = performOp(
        "crypto/exportKey",
        {
          algorithm: key[_algorithm].name,
          namedCurve: key[_algorithm].namedCurve,
          format: "raw",
        },
        innerKey,
      );

      return TypedArrayPrototypeGetBuffer(data);
    }
    case "pkcs8": {
      // 1.
      if (key[_type] !== "private") {
        throw new DOMException(
          "Key is not a private key",
          "InvalidAccessError",
        );
      }

      // 2.
      const data = performOp(
        "crypto/exportKey",
        {
          algorithm: key[_algorithm].name,
          namedCurve: key[_algorithm].namedCurve,
          format: "pkcs8",
        },
        innerKey,
      );

      return TypedArrayPrototypeGetBuffer(data);
    }
    case "spki": {
      // 1.
      if (key[_type] !== "public") {
        throw new DOMException("Key is not a public key", "InvalidAccessError");
      }

      // 2.
      const data = performOp(
        "crypto/exportKey",
        {
          algorithm: key[_algorithm].name,
          namedCurve: key[_algorithm].namedCurve,
          format: "spki",
        },
        innerKey,
      );

      return TypedArrayPrototypeGetBuffer(data);
    }
    case "jwk": {
      if (key[_algorithm].name === "ECDSA") {
        // 1-2.
        const jwk = {
          kty: "EC",
        } as Record<string, any>;

        // 3.1
        jwk.crv = key[_algorithm].namedCurve;

        // Missing from spec
        let algNamedCurve;

        switch (key[_algorithm].namedCurve) {
          case "P-256": {
            algNamedCurve = "ES256";
            break;
          }
          case "P-384": {
            algNamedCurve = "ES384";
            break;
          }
          case "P-521": {
            algNamedCurve = "ES512";
            break;
          }
          default:
            throw new DOMException(
              "Curve algorithm not supported",
              "DataError",
            );
        }

        jwk.alg = algNamedCurve;

        // 3.2 - 3.4.
        const data = performOp(
          "crypto/exportKey",
          {
            format: key[_type] === "private" ? "jwkprivate" : "jwkpublic",
            algorithm: key[_algorithm].name,
            namedCurve: key[_algorithm].namedCurve,
          },
          innerKey,
        );
        ObjectAssign(jwk, data);

        // 4.
        jwk.key_ops = key.usages;

        // 5.
        jwk.ext = key[_extractable];

        return jwk;
      } else {
        // ECDH
        // 1-2.
        const jwk = {
          kty: "EC",
        } as Record<string, any>;

        // missing step from spec
        jwk.alg = "ECDH";

        // 3.1
        jwk.crv = key[_algorithm].namedCurve;

        // 3.2 - 3.4
        const data = performOp(
          "crypto/exportKey",
          {
            format: key[_type] === "private" ? "jwkprivate" : "jwkpublic",
            algorithm: key[_algorithm].name,
            namedCurve: key[_algorithm].namedCurve,
          },
          innerKey,
        );
        ObjectAssign(jwk, data);

        // 4.
        jwk.key_ops = key.usages;

        // 5.
        jwk.ext = key[_extractable];

        return jwk;
      }
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }
}

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

export function aes(format, key, innerKey) {
  switch (format) {
    // 2.
    case "raw": {
      // 1.
      const data = innerKey.data;
      // 2.
      return TypedArrayPrototypeGetBuffer(data);
    }
    case "jwk": {
      // 1-2.
      const jwk = {
        kty: "oct",
      } as Record<string, any>;

      // 3.
      const data = performOp(
        "crypto/exportKey",
        {
          format: "jwksecret",
          algorithm: "AES",
        },
        innerKey,
      );
      ObjectAssign(jwk, data);

      // 4.
      const algorithm = key[_algorithm];
      if (!Object.prototype.hasOwnProperty.call(aesJwkAlg, algorithm.name)) {
        throw new DOMException("Invalid JWK algorithm", "NotSupportedError");
      }
      const alg = aesJwkAlg[algorithm.name];
      if (!Object.prototype.hasOwnProperty.call(alg, algorithm.length)) {
        throw new DOMException("Invalid key length", "NotSupportedError");
      }
      jwk.alg = alg[algorithm.length];

      // 5.
      jwk.key_ops = key.usages;

      // 6.
      jwk.ext = key[_extractable];

      // 7.
      return jwk;
    }
    default:
      throw new DOMException("Not implemented", "NotSupportedError");
  }
}
