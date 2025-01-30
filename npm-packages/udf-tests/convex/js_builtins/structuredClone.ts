import { query } from "../_generated/server";
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";

function testStructuredCloneJson() {
  // Test simple string
  const stringValue = "hello";
  const clonedString = structuredClone(stringValue);
  assert.strictEqual(clonedString, stringValue);

  // Test nested object
  const objectValue = { a: ["b", "c"], d: 1 };
  const clonedObject = structuredClone(objectValue);
  assert.deepEqual(clonedObject, objectValue);
  assert.notEqual(clonedObject, objectValue); // Different object reference
}

function testStructuredCloneArrayBuffer() {
  const size = 5;
  const buffer = new ArrayBuffer(size);
  const view = new Uint8Array(buffer);
  for (let i = 0; i < size; i++) {
    view[i] = i % 256;
  }
  const cloned = structuredClone(buffer);
  const clonedView = new Uint8Array(cloned);
  assert.instanceOf(cloned, ArrayBuffer);
  assert.deepEqual(Array.from(clonedView), [0, 1, 2, 3, 4]);
}

function testStructuredCloneTypedArray() {
  const values = [1, 2, 4];
  const array = new Int32Array(values);
  const cloned = structuredClone(array);
  assert.instanceOf(cloned, Int32Array);
  assert.deepEqual(Array.from(cloned), values);
}

function testStructuredCloneNonJson() {
  // Test Date
  const date = new Date(100000.0);
  const cloneDate = structuredClone(date);
  assert.strictEqual(date.getTime(), cloneDate.getTime());

  // Test Map
  const map = new Map();
  map.set(1, 2);
  const cloneMap = structuredClone(map);
  assert.strictEqual(cloneMap.size, 1);
  assert.strictEqual(cloneMap.get(1), 2);

  // Test Set
  const set = new Set();
  set.add(1);
  const cloneSet = structuredClone(set);
  assert.strictEqual(cloneSet.size, 1);
  assert.isTrue(cloneSet.has(1));

  // Test BigInt
  const bigint = BigInt(1);
  const cloneBigint = structuredClone(bigint);
  assert.strictEqual(bigint, cloneBigint);

  // Test RegExp
  const regex = /a/g;
  const cloneRegex = structuredClone(regex);
  assert.strictEqual(regex.toString(), cloneRegex.toString());
  assert.instanceOf(cloneRegex, RegExp);

  // Test Function
  const func = () => {};
  assert.throws(() => structuredClone(func), /could not deserialize value/);

  // Test Symbol
  const symbol = Symbol("test");
  assert.throws(() => structuredClone(symbol), /could not deserialize value/);

  // Test Error
  const error = new Error("test");
  const cloneError = structuredClone(error);
  assert.strictEqual(error.message, cloneError.message);
  assert.instanceOf(cloneError, Error);
}

function testStructuredCloneRecursive() {
  // Test recursive object
  const recursiveObject: any = { a: 1 };
  recursiveObject["b"] = recursiveObject;
  const cloned = structuredClone(recursiveObject);
  assert.strictEqual(cloned.a, 1);
  assert.strictEqual(cloned.b, cloned);
  assert.notEqual(cloned, recursiveObject);

  // Test recursive array
  const recursiveArray: any[] = [1];
  recursiveArray.push(recursiveArray);
  recursiveArray.push(recursiveObject);
  const clonedArray = structuredClone(recursiveArray);
  assert.strictEqual(clonedArray[0], 1);
  assert.strictEqual(clonedArray[1], clonedArray);
  assert.strictEqual(clonedArray[2].b, clonedArray[2]);
  assert.notEqual(clonedArray, recursiveArray);
  assert.notEqual(clonedArray[2], recursiveObject);
}

export default query(async (): Promise<string> => {
  return await wrapInTests({
    testStructuredCloneJson,
    testStructuredCloneArrayBuffer,
    testStructuredCloneTypedArray,
    testStructuredCloneNonJson,
    testStructuredCloneRecursive,
  });
});

export const withTransfer = query(async () => {
  const array = new ArrayBuffer(8);
  const _tranferred = structuredClone(array, { transfer: [array] });
  return "expected uncatchable error";
});
