// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { assert } from "chai";
import { wrapInTests } from "./testHelpers";
import { query } from "../_generated/server";

export default query(async () => {
  return await wrapInTests({
    fileEmptyFileBits,
    fileStringFileBits,
    fileUnicodeStringFileBits,
    // fileStringObjectFileBits,
    fileEmptyBlobFileBits,
    fileBlobFileBits,
    fileEmptyFileFileBits,
    fileFileFileBits,
    fileArrayBufferFileBits,
    fileTypedArrayFileBits,
    fileVariousFileBits,
    // fileNumberInFileBits,
    // fileArrayInFileBits,
    // fileObjectInFileBits,
    fileUsingFileName,
    fileUsingNullFileName,
    fileUsingNumberFileName,
    fileUsingEmptyStringFileName,
  });
});

function testFirstArgument(arg1: any[], expectedSize: number) {
  const file = new File(arg1, "name");
  assert(file instanceof File);
  assert.strictEqual(file.name, "name");
  assert.strictEqual(file.size, expectedSize);
  assert.strictEqual(file.type, "");
}

function fileEmptyFileBits() {
  testFirstArgument([], 0);
}

function fileStringFileBits() {
  testFirstArgument(["bits"], 4);
}

function fileUnicodeStringFileBits() {
  testFirstArgument(["𝓽𝓮𝔁𝓽"], 16);
}

// function fileStringObjectFileBits() {
//   testFirstArgument([new String("string object")], 13);
// }

function fileEmptyBlobFileBits() {
  testFirstArgument([new Blob()], 0);
}

function fileBlobFileBits() {
  testFirstArgument([new Blob(["bits"])], 4);
}

function fileEmptyFileFileBits() {
  testFirstArgument([new File([], "world.txt")], 0);
}

function fileFileFileBits() {
  testFirstArgument([new File(["bits"], "world.txt")], 4);
}

function fileArrayBufferFileBits() {
  testFirstArgument([new ArrayBuffer(8)], 8);
}

function fileTypedArrayFileBits() {
  testFirstArgument([new Uint8Array([0x50, 0x41, 0x53, 0x53])], 4);
}

function fileVariousFileBits() {
  testFirstArgument(
    [
      "bits",
      new Blob(["bits"]),
      new Blob(),
      new Uint8Array([0x50, 0x41]),
      new Uint16Array([0x5353]),
      new Uint32Array([0x53534150]),
    ],
    16,
  );
}

// function fileNumberInFileBits() {
//   testFirstArgument([12], 2);
// }

// function fileArrayInFileBits() {
//   testFirstArgument([[1, 2, 3]], 5);
// }

// function fileObjectInFileBits() {
//   // "[object Object]"
//   testFirstArgument([{}], 15);
// }

function testSecondArgument(arg2: any, expectedFileName: string) {
  const file = new File(["bits"], arg2);
  assert(file instanceof File);
  assert.strictEqual(file.name, expectedFileName);
}

function fileUsingFileName() {
  testSecondArgument("dummy", "dummy");
}

function fileUsingNullFileName() {
  testSecondArgument(null, "null");
}

function fileUsingNumberFileName() {
  testSecondArgument(1, "1");
}

function fileUsingEmptyStringFileName() {
  testSecondArgument("", "");
}
