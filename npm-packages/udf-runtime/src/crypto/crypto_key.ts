// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/00_crypto.js

import inspect from "object-inspect";

// This is a mapping from CryptoKey handle to key data
export const KEY_STORE = new WeakMap();

export const _handle = Symbol("[[handle]]");
export const _algorithm = Symbol("[[algorithm]]");
export const _extractable = Symbol("[[extractable]]");
export const _usages = Symbol("[[usages]]");
export const _type = Symbol("[[type]]");

export class CryptoKey {
  // Constructor not exposed publicly
  constructor(
    type: string,
    extractable: boolean,
    usages: string[],
    algorithm: object,
    handle: object,
  ) {
    this[_type] = type;
    this[_extractable] = extractable;
    this[_algorithm] = algorithm;
    this[_usages] = usages;
    this[_handle] = handle;
  }

  /** @returns {string} */
  get type() {
    return this[_type];
  }

  /** @returns {boolean} */
  get extractable() {
    return this[_extractable];
  }

  /** @returns {string[]} */
  get usages() {
    return this[_usages];
  }

  /** @returns {object} */
  get algorithm() {
    return this[_algorithm];
  }

  inspect() {
    return `${this.constructor.name} ${inspect({
      type: this.type,
      extractable: this.extractable,
      algorithm: this.algorithm,
      usages: this.usages,
    })}`;
  }
}

export type CryptoKeyPair = { privateKey: CryptoKey; publicKey: CryptoKey };
