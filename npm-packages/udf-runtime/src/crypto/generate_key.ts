import { performOp } from "../syscall.js";
import { CryptoKey, CryptoKeyPair, KEY_STORE } from "./crypto_key.js";
import * as ImportKey from "./import_key.js";
import getKeyLength from "./get_key_length.js";
import { WeakMapPrototypeSet } from "./helpers.js";
import {
  generateKeyAes,
  generateKeyHmac,
  generateKeyPublicKeyAlgorithm,
} from "./normalize_algorithm.js";
import { z } from "zod";
import { usageIntersection } from "./import_key.js";

const validateUsages = (
  algorithm: { name: string },
  keyUsages: readonly string[],
) => {
  const supportedUsages = ImportKey.SUPPORTED_KEY_USAGES[algorithm.name];
  for (const usage of keyUsages) {
    if (
      !supportedUsages.private.includes(usage) &&
      !supportedUsages.public.includes(usage)
    ) {
      throw new DOMException(
        `Unsupported key usage '${usage}' for ${algorithm.name}`,
        "SyntaxError",
      );
    }
  }
  const privateUsages = usageIntersection(keyUsages, supportedUsages.private);
  const publicUsages = usageIntersection(keyUsages, supportedUsages.public);
  if (!privateUsages.length) {
    throw new DOMException(
      `Usages cannot be empty when creating a ${algorithm.name} key`,
      "SyntaxError",
    );
  }
  return { privateUsages, publicUsages };
};

const validateSymmetricUsages = (
  algorithm: { name: string },
  keyUsages: readonly string[],
) => {
  const supportedUsages =
    ImportKey.SUPPORTED_SYMMETRIC_KEY_USAGES[algorithm.name];
  for (const usage of keyUsages) {
    if (!supportedUsages.includes(usage)) {
      throw new DOMException(
        `Unsupported key usage '${usage}' for ${algorithm.name}`,
        "SyntaxError",
      );
    }
  }
  const usages = usageIntersection(keyUsages, supportedUsages);
  if (!usages.length) {
    throw new DOMException(
      `Usages cannot be empty when creating a ${algorithm.name} key`,
      "SyntaxError",
    );
  }
  return usages;
};

export function keyPair(
  algorithm: z.infer<typeof generateKeyPublicKeyAlgorithm>,
  extractable: boolean,
  keyUsages: readonly string[],
): CryptoKeyPair {
  const { privateUsages, publicUsages } = validateUsages(algorithm, keyUsages);
  const { privateRawData, publicRawData } = performOp(
    "crypto/generateKeyPair",
    algorithm,
  );
  const privateHandle = {};
  WeakMapPrototypeSet(KEY_STORE, privateHandle, privateRawData);
  const publicHandle = {};
  WeakMapPrototypeSet(KEY_STORE, publicHandle, publicRawData);

  const privateKey = new CryptoKey(
    "private",
    extractable,
    privateUsages,
    { ...algorithm },
    privateHandle,
  );
  const publicKey = new CryptoKey(
    "public",
    extractable,
    publicUsages,
    { ...algorithm },
    publicHandle,
  );

  return { privateKey, publicKey };
}

export function hmac(
  algorithm: z.infer<typeof generateKeyHmac>,
  extractable: boolean,
  keyUsages: readonly string[],
) {
  const usages = validateSymmetricUsages(algorithm, keyUsages);
  const keyLength = algorithm.length ?? getKeyLength(algorithm);
  const keyData: Uint8Array = performOp(
    "crypto/generateKeyBytes",
    keyLength / 8,
  );
  return ImportKey.hmac("raw", algorithm, keyData, extractable, usages);
}

export function aes(
  algorithm: z.infer<typeof generateKeyAes>,
  extractable: boolean,
  keyUsages: readonly string[],
) {
  const usages = validateSymmetricUsages(algorithm, keyUsages);
  const keyData: Uint8Array = performOp(
    "crypto/generateKeyBytes",
    algorithm.length / 8,
  );
  return ImportKey.aes("raw", algorithm, keyData, extractable, usages);
}
