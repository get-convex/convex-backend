// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/00_crypto.js

import * as z from "zod";
import { copyBuffer } from "./helpers";

const algorithmNameLiteral = <AlgorithmName extends string>(
  algorithmName: AlgorithmName,
) => {
  return z
    .string()
    .transform((n) => {
      if (n.toUpperCase() !== algorithmName.toUpperCase()) {
        return z.NEVER;
      }
      return algorithmName;
    })
    .refine((n): n is AlgorithmName => n === algorithmName);
};

const algorithmNameLiteralWithParams = <
  AlgorithmName extends string,
  T extends z.ZodRawShape,
  UnknownKeys extends z.UnknownKeysParam,
  Catchall extends z.ZodTypeAny,
  Output,
  Input,
>(
  algorithmName: AlgorithmName,
  params: z.ZodObject<T, UnknownKeys, Catchall, Output, Input>,
) => {
  return params.extend({
    name: algorithmNameLiteral(algorithmName),
  });
};

const algorithmNameLiteralWithoutParams = <AlgorithmName extends string>(
  algorithmName: AlgorithmName,
) => {
  return z
    .union([
      z.object({
        name: algorithmNameLiteral(algorithmName),
      }),
      algorithmNameLiteral(algorithmName).transform((x) => {
        return {
          name: x,
        };
      }),
    ])
    .refine((x): x is { name: AlgorithmName } => true);
};

const digest = z.union([
  algorithmNameLiteralWithoutParams("SHA-1"),
  algorithmNameLiteralWithoutParams("SHA-256"),
  algorithmNameLiteralWithoutParams("SHA-384"),
  algorithmNameLiteralWithoutParams("SHA-512"),
]);

const rsaHashedKeyGenParams = z.object({
  modulusLength: z.number(),
  publicExponent: z.instanceof(Uint8Array),
  hash: digest,
});

const ecKeyGenParams = z.object({
  // see `supportedNamedCurves`
  namedCurve: z.union([z.literal("P-256"), z.literal("P-384")]),
});

const hmacKeyGenParams = z.object({
  hash: digest,
  length: z.onumber(),
});

const aesKeyGenParams = z.object({
  length: z.union([z.literal(128), z.literal(192), z.literal(256)]),
});

export const hmacImportParams = z.object({
  hash: digest,
  length: z.onumber(),
});

const ecdsaParams = z.object({
  hash: digest,
});

export const rsaHashedImportParams = z.object({
  hash: digest,
});

const ecKeyImportParams = z.object({
  namedCurve: z.string(),
});

const bufferSource = z
  .custom<BufferSource>((input) => {
    return ArrayBuffer.isView(input) || input instanceof ArrayBuffer;
  })
  .transform((input) => {
    return copyBuffer(input);
  });

const hkdfParams = z.object({
  hash: digest,
  salt: z.optional(bufferSource),
  info: z.optional(bufferSource),
});

const pbkdf2Params = z.object({
  hash: digest,
  salt: z.optional(bufferSource),
  iterations: z.number(),
});

const ecdhKeyDeriveParams = z.object({
  public: z.custom<CryptoKey>((x) => x instanceof CryptoKey),
});

const aesDerivedKeyParams = z.object({
  length: z.number(),
});

const rsaPssParams = z.object({
  saltLength: z.number(),
});

export const generateKeyPublicKeyAlgorithm = z.union([
  algorithmNameLiteralWithParams("RSASSA-PKCS1-v1_5", rsaHashedKeyGenParams),
  algorithmNameLiteralWithParams("RSA-PSS", rsaHashedKeyGenParams),
  algorithmNameLiteralWithParams("RSA-OAEP", rsaHashedKeyGenParams),
  algorithmNameLiteralWithParams("ECDSA", ecKeyGenParams),
  algorithmNameLiteralWithParams("ECDH", ecKeyGenParams),
  algorithmNameLiteralWithoutParams("Ed25519"),
  algorithmNameLiteralWithoutParams("X25519"),
]);
export const generateKeyHmac = algorithmNameLiteralWithParams(
  "HMAC",
  hmacKeyGenParams,
);
export const generateKeyAes = z.union([
  algorithmNameLiteralWithParams("AES-CTR", aesKeyGenParams),
  algorithmNameLiteralWithParams("AES-CBC", aesKeyGenParams),
  algorithmNameLiteralWithParams("AES-GCM", aesKeyGenParams),
  algorithmNameLiteralWithParams("AES-KW", aesKeyGenParams),
]);
const generateKey = z.union([
  generateKeyPublicKeyAlgorithm,
  generateKeyHmac,
  generateKeyAes,
]);

const importKey = z.union([
  algorithmNameLiteralWithParams("RSASSA-PKCS1-v1_5", rsaHashedImportParams),
  algorithmNameLiteralWithParams("RSA-PSS", rsaHashedImportParams),
  algorithmNameLiteralWithParams("RSA-OAEP", rsaHashedImportParams),
  algorithmNameLiteralWithParams("ECDSA", ecKeyImportParams),
  algorithmNameLiteralWithParams("ECDH", ecKeyImportParams),
  algorithmNameLiteralWithParams("HMAC", hmacImportParams),
  algorithmNameLiteralWithoutParams("HKDF"),
  algorithmNameLiteralWithoutParams("PBKDF2"),
  algorithmNameLiteralWithoutParams("AES-CTR"),
  algorithmNameLiteralWithoutParams("AES-CBC"),
  algorithmNameLiteralWithoutParams("AES-GCM"),
  algorithmNameLiteralWithoutParams("AES-KW"),
  algorithmNameLiteralWithoutParams("Ed25519"),
  algorithmNameLiteralWithoutParams("X25519"),
]);

const sign = z.union([
  algorithmNameLiteralWithoutParams("RSASSA-PKCS1-v1_5"),
  algorithmNameLiteralWithParams("RSA-PSS", rsaPssParams),
  algorithmNameLiteralWithParams("ECDSA", ecdsaParams),
  algorithmNameLiteralWithoutParams("HMAC"),
  algorithmNameLiteralWithoutParams("Ed25519"),
]);
const verify = sign;

export const getKeyLength = z.union([
  algorithmNameLiteralWithParams("AES-CBC", aesDerivedKeyParams),
  algorithmNameLiteralWithParams("AES-CTR", aesDerivedKeyParams),
  algorithmNameLiteralWithParams("AES-GCM", aesDerivedKeyParams),
  algorithmNameLiteralWithParams("AES-KW", aesDerivedKeyParams),
  algorithmNameLiteralWithParams("HMAC", hmacImportParams),
  algorithmNameLiteralWithoutParams("HKDF"),
  algorithmNameLiteralWithoutParams("PBKDF2"),
]);

export const deriveBits = z.union([
  algorithmNameLiteralWithParams("HKDF", hkdfParams),
  algorithmNameLiteralWithParams("PBKDF2", pbkdf2Params),
  algorithmNameLiteralWithParams("ECDH", ecdhKeyDeriveParams),
  algorithmNameLiteralWithParams("X25519", ecdhKeyDeriveParams),
]);

const unknownAlgorithm = "Unrecognized or invalid algorithm";

export const normalizeAlgorithmSign = (
  input: unknown,
): z.infer<typeof sign> => {
  const result = sign.safeParse(input);
  if (!result.success) {
    throw new DOMException(unknownAlgorithm);
  } else {
    return result.data;
  }
};

export const normalizeAlgorithmVerify = (
  input: unknown,
): z.infer<typeof verify> => {
  const result = verify.safeParse(input);
  if (!result.success) {
    throw new DOMException(unknownAlgorithm);
  } else {
    return result.data;
  }
};

export const normalizeAlgorithmImportKey = (
  input: unknown,
): z.infer<typeof importKey> => {
  const result = importKey.safeParse(input);
  if (!result.success) {
    throw new DOMException(unknownAlgorithm);
  } else {
    return result.data;
  }
};

export const normalizeAlgorithmDeriveBits = (
  input: unknown,
): z.infer<typeof deriveBits> => {
  const result = deriveBits.safeParse(input);
  if (!result.success) {
    throw new DOMException(unknownAlgorithm);
  } else {
    return result.data;
  }
};

export const normalizeAlgorithmGetKeyLength = (
  input: unknown,
): z.infer<typeof getKeyLength> => {
  const result = getKeyLength.safeParse(input);
  if (!result.success) {
    throw new DOMException(unknownAlgorithm);
  } else {
    return result.data;
  }
};

export const normalizeAlgorithmDigest = (
  input: unknown,
): z.infer<typeof digest> => {
  const result = digest.safeParse(input);
  if (!result.success) {
    throw new DOMException(unknownAlgorithm);
  } else {
    return result.data;
  }
};

export const normalizeAlgorithmGenerateKey = (
  input: unknown,
): z.infer<typeof generateKey> => {
  const result = generateKey.safeParse(input);
  if (!result.success) {
    throw new DOMException(unknownAlgorithm);
  } else {
    return result.data;
  }
};

const _supportedAlgorithms = {
  digest: {
    "SHA-1": null,
    "SHA-256": null,
    "SHA-384": null,
    "SHA-512": null,
  },
  generateKey: {
    "RSASSA-PKCS1-v1_5": "RsaHashedKeyGenParams",
    "RSA-PSS": "RsaHashedKeyGenParams",
    "RSA-OAEP": "RsaHashedKeyGenParams",
    ECDSA: "EcKeyGenParams",
    ECDH: "EcKeyGenParams",
    "AES-CTR": "AesKeyGenParams",
    "AES-CBC": "AesKeyGenParams",
    "AES-GCM": "AesKeyGenParams",
    "AES-KW": "AesKeyGenParams",
    HMAC: "HmacKeyGenParams",
    X25519: null,
    Ed25519: null,
  },
  sign: {
    "RSASSA-PKCS1-v1_5": null,
    "RSA-PSS": "RsaPssParams",
    ECDSA: "EcdsaParams",
    HMAC: null,
    Ed25519: null,
  },
  verify: {
    "RSASSA-PKCS1-v1_5": null,
    "RSA-PSS": "RsaPssParams",
    ECDSA: "EcdsaParams",
    HMAC: null,
    Ed25519: null,
  },
  importKey: {
    "RSASSA-PKCS1-v1_5": "RsaHashedImportParams",
    "RSA-PSS": "RsaHashedImportParams",
    "RSA-OAEP": "RsaHashedImportParams",
    ECDSA: "EcKeyImportParams",
    ECDH: "EcKeyImportParams",
    HMAC: "HmacImportParams",
    HKDF: null,
    PBKDF2: null,
    "AES-CTR": null,
    "AES-CBC": null,
    "AES-GCM": null,
    "AES-KW": null,
    Ed25519: null,
    X25519: null,
  },
  deriveBits: {
    HKDF: "HkdfParams",
    PBKDF2: "Pbkdf2Params",
    ECDH: "EcdhKeyDeriveParams",
    X25519: "EcdhKeyDeriveParams",
  },
  encrypt: {
    "RSA-OAEP": "RsaOaepParams",
    "AES-CBC": "AesCbcParams",
    "AES-GCM": "AesGcmParams",
    "AES-CTR": "AesCtrParams",
  },
  decrypt: {
    "RSA-OAEP": "RsaOaepParams",
    "AES-CBC": "AesCbcParams",
    "AES-GCM": "AesGcmParams",
    "AES-CTR": "AesCtrParams",
  },
  "get key length": {
    "AES-CBC": "AesDerivedKeyParams",
    "AES-CTR": "AesDerivedKeyParams",
    "AES-GCM": "AesDerivedKeyParams",
    "AES-KW": "AesDerivedKeyParams",
    HMAC: "HmacImportParams",
    HKDF: null,
    PBKDF2: null,
  },
  wrapKey: {
    "AES-KW": null,
  },
  unwrapKey: {
    "AES-KW": null,
  },
};

const _simpleAlgorithmDictionaries = {
  AesGcmParams: { iv: "BufferSource", additionalData: "BufferSource" },
  RsaHashedKeyGenParams: { hash: "HashAlgorithmIdentifier" },
  EcKeyGenParams: {},
  HmacKeyGenParams: { hash: "HashAlgorithmIdentifier" },
  RsaPssParams: {},
  EcdsaParams: { hash: "HashAlgorithmIdentifier" },
  HmacImportParams: { hash: "HashAlgorithmIdentifier" },
  HkdfParams: {
    hash: "HashAlgorithmIdentifier",
    salt: "BufferSource",
    info: "BufferSource",
  },
  Pbkdf2Params: { hash: "HashAlgorithmIdentifier", salt: "BufferSource" },
  RsaOaepParams: { label: "BufferSource" },
  RsaHashedImportParams: { hash: "HashAlgorithmIdentifier" },
  EcKeyImportParams: {},
};
