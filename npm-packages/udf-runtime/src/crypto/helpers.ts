// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
// https://github.com/denoland/deno/blob/main/ext/crypto/00_crypto.js

/**
 * @param {ArrayBufferView | ArrayBuffer} input
 * @returns {Uint8Array}
 */
export function copyBuffer(input: ArrayBufferView | ArrayBuffer): Uint8Array {
  if (ArrayBuffer.isView(input)) {
    return new Uint8Array(
      input.buffer,
      input.byteOffset,
      input.byteLength,
    ).slice();
  }
  // ArrayBuffer
  return new Uint8Array(input, 0, input.byteLength).slice();
}

// In Deno these are primordials.

export function ArrayPrototypeIncludes<T>(array: T[], item: T) {
  return array.includes(item);
}

export function ArrayPrototypeFind<T>(
  array: T[],
  predicate: (t: T) => boolean,
) {
  return array.find(predicate);
}

export function WeakMapPrototypeSet<K extends object, T>(
  set: WeakMap<K, T>,
  key: K,
  item: T,
) {
  set.set(key, item);
}

export function WeakMapPrototypeGet<K extends object, T>(
  set: WeakMap<K, T>,
  key: K,
): T | undefined {
  return set.get(key);
}

export function ArrayPrototypeEvery<T>(
  array: T[],
  predicate: (t: T) => boolean,
) {
  return array.every(predicate);
}

export function TypedArrayPrototypeGetByteLength(typedArray: Uint8Array) {
  return typedArray.byteLength;
}

export function TypedArrayPrototypeGetBuffer(typedArray: Uint8Array) {
  return typedArray.buffer;
}

export function ObjectAssign(target: object, source: object) {
  Object.assign(target, source);
}
