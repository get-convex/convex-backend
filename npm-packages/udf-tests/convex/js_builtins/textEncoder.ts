// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { query } from "../_generated/server";
import { assert, expect } from "chai";
import { wrapInTests } from "./testHelpers";

function btoaSuccess() {
  const text = "hello world";
  const encoded = btoa(text);
  assert.strictEqual(encoded, "aGVsbG8gd29ybGQ=");
  const latin1 = String.fromCharCode(151);
  assert.strictEqual(btoa(latin1), "lw==");
}

function atobSuccess() {
  const encoded = "aGVsbG8gd29ybGQ=";
  const decoded = atob(encoded);
  assert.strictEqual(decoded, "hello world");
  const latin1 = String.fromCharCode(151);
  assert.strictEqual(atob("lw=="), latin1);
}

function atobWithAsciiWhitespace() {
  const encodedList = [
    " aGVsbG8gd29ybGQ=",
    "  aGVsbG8gd29ybGQ=",
    "aGVsbG8gd29ybGQ= ",
    "aGVsbG8gd29ybGQ=\n",
    "aGVsbG\t8gd29ybGQ=",
    `aGVsbG\t8g
                d29ybGQ=`,
  ];

  for (const encoded of encodedList) {
    const decoded = atob(encoded);
    assert.strictEqual(decoded, "hello world");
  }
}

function atobThrows() {
  assert.throws(() => atob("aGVsbG8gd29ybGQ=="));
}

function atobThrows2() {
  assert.throws(() => atob("aGVsbG8gd29ybGQ==="));
}

function atobThrows3() {
  expect(() => atob("foobar!!"))
    .throws(DOMException)
    .that.has.property("name")
    .equal("InvalidCharacterError");
}

function btoaFailed() {
  const text = "你好";
  assert.throws(() => {
    btoa(text);
  }, DOMException);
}

function textDecoder2() {
  const fixture = new Uint8Array([
    0xf0, 0x9d, 0x93, 0xbd, 0xf0, 0x9d, 0x93, 0xae, 0xf0, 0x9d, 0x94, 0x81,
    0xf0, 0x9d, 0x93, 0xbd,
  ]);
  const decoder = new TextDecoder();
  assert.strictEqual(decoder.decode(fixture), "𝓽𝓮𝔁𝓽");
}

// Deno tests ignoreBOM through WPT, which we don't do yet.
// https://linear.app/convex/issue/CX-3310/set-up-web-platform-tests

function textDecoderASCII() {
  const fixture = new Uint8Array([0x89, 0x95, 0x9f, 0xbf]);
  const decoder = new TextDecoder("ascii");
  assert.strictEqual(decoder.decode(fixture), "‰•Ÿ¿");
}

function textDecoderErrorEncoding() {
  assert.throws(
    () => new TextDecoder("Foo"),
    "The encoding label provided ('Foo') is invalid.",
  );
}

function textEncoder() {
  const fixture = "𝓽𝓮𝔁𝓽";
  const encoder = new TextEncoder();
  assert.deepEqual(
    Array.from(encoder.encode(fixture)),
    [
      0xf0, 0x9d, 0x93, 0xbd, 0xf0, 0x9d, 0x93, 0xae, 0xf0, 0x9d, 0x94, 0x81,
      0xf0, 0x9d, 0x93, 0xbd,
    ],
  );
}

function textEncodeInto() {
  const fixture = "text";
  const encoder = new TextEncoder();
  const bytes = new Uint8Array(5);
  const result = encoder.encodeInto(fixture, bytes);
  assert.strictEqual(result.read, 4);
  assert.strictEqual(result.written, 4);
  assert.deepEqual(Array.from(bytes), [0x74, 0x65, 0x78, 0x74, 0x00]);
}

function textEncodeInto2() {
  const fixture = "𝓽𝓮𝔁𝓽";
  const encoder = new TextEncoder();
  const bytes = new Uint8Array(17);
  const result = encoder.encodeInto(fixture, bytes);
  assert.strictEqual(result.read, 8);
  assert.strictEqual(result.written, 16);
  assert.deepEqual(
    Array.from(bytes),
    [
      0xf0, 0x9d, 0x93, 0xbd, 0xf0, 0x9d, 0x93, 0xae, 0xf0, 0x9d, 0x94, 0x81,
      0xf0, 0x9d, 0x93, 0xbd, 0x00,
    ],
  );
}

function textEncodeInto3() {
  const fixture = "𝓽𝓮𝔁𝓽";
  const encoder = new TextEncoder();
  const bytes = new Uint8Array(5);
  const result = encoder.encodeInto(fixture, bytes);
  assert.strictEqual(result.read, 2);
  assert.strictEqual(result.written, 4);
  assert.deepEqual(Array.from(bytes), [0xf0, 0x9d, 0x93, 0xbd, 0x00]);
}

// function loneSurrogateEncodeInto() {
//   const fixture = "lone𝄞\ud888surrogate";
//   const encoder = new TextEncoder();
//   const bytes = new Uint8Array(20);
//   const result = encoder.encodeInto(fixture, bytes);
//   assertEquals(result.read, 16);
//   assertEquals(result.written, 20);
//   assertEquals(
//     Array.from(bytes),
//     [
//       0x6c, 0x6f, 0x6e, 0x65, 0xf0, 0x9d, 0x84, 0x9e, 0xef, 0xbf, 0xbd, 0x73,
//       0x75, 0x72, 0x72, 0x6f, 0x67, 0x61, 0x74, 0x65,
//     ]
//   );
// }

// function loneSurrogateEncodeInto2() {
//   const fixture = "\ud800";
//   const encoder = new TextEncoder();
//   const bytes = new Uint8Array(3);
//   const result = encoder.encodeInto(fixture, bytes);
//   assertEquals(result.read, 1);
//   assertEquals(result.written, 3);
//   assertEquals(Array.from(bytes), [0xef, 0xbf, 0xbd]);
// }

// function loneSurrogateEncodeInto3() {
//   const fixture = "\udc00";
//   const encoder = new TextEncoder();
//   const bytes = new Uint8Array(3);
//   const result = encoder.encodeInto(fixture, bytes);
//   assertEquals(result.read, 1);
//   assertEquals(result.written, 3);
//   assertEquals(Array.from(bytes), [0xef, 0xbf, 0xbd]);
// }

// function swappedSurrogatePairEncodeInto4() {
//   const fixture = "\udc00\ud800";
//   const encoder = new TextEncoder();
//   const bytes = new Uint8Array(8);
//   const result = encoder.encodeInto(fixture, bytes);
//   assertEquals(result.read, 2);
//   assertEquals(result.written, 6);
//   assertEquals(
//     Array.from(bytes),
//     [0xef, 0xbf, 0xbd, 0xef, 0xbf, 0xbd, 0x00, 0x00]
//   );
// }

function textDecoderSharedUint8Array() {
  const ab = new SharedArrayBuffer(6);
  const dataView = new DataView(ab);
  const charCodeA = "A".charCodeAt(0);
  for (let i = 0; i < ab.byteLength; i++) {
    dataView.setUint8(i, charCodeA + i);
  }
  const ui8 = new Uint8Array(ab);
  const decoder = new TextDecoder();
  const actual = decoder.decode(ui8);
  assert.strictEqual(actual, "ABCDEF");
}

function textDecoderSharedInt32Array() {
  const ab = new SharedArrayBuffer(8);
  const dataView = new DataView(ab);
  const charCodeA = "A".charCodeAt(0);
  for (let i = 0; i < ab.byteLength; i++) {
    dataView.setUint8(i, charCodeA + i);
  }
  const i32 = new Int32Array(ab);
  const decoder = new TextDecoder();
  const actual = decoder.decode(i32);
  assert.strictEqual(actual, "ABCDEFGH");
}

function toStringShouldBeWebCompatibility() {
  const encoder = new TextEncoder();
  assert.strictEqual(encoder.toString(), "[object TextEncoder]");

  const decoder = new TextDecoder();
  assert.strictEqual(decoder.toString(), "[object TextDecoder]");
}

function textEncoderShouldCoerceToString() {
  const encoder = new TextEncoder();
  const fixutreText = "text";
  const fixture = {
    toString() {
      return fixutreText;
    },
  };

  const bytes = encoder.encode(fixture as unknown as string);
  const decoder = new TextDecoder();
  const decoded = decoder.decode(bytes);
  assert.strictEqual(decoded, fixutreText);
}

function atobCorrectAlphabet() {
  // Standard encoding (not URL safe)
  assert.strictEqual(atob("AAA+"), "\x00\x00>");
  assert.strictEqual(atob("AAA/"), "\x00\x00?");
}

function onlyAsciiWhitespaceRemoved() {
  assert.strictEqual(atob("aGVsbG 8gd29ybGQ="), "hello world");
  assert.throws(() => {
    atob("aGVsbG\u20058gd29ybGQ=");
  }, DOMException);
}

export default query(async (): Promise<string> => {
  return await wrapInTests({
    btoaSuccess,
    atobSuccess,
    atobWithAsciiWhitespace,

    // Node.js accepts these, browsers and deno error. We error here.
    atobThrows,
    atobThrows2,
    atobThrows3,
    btoaFailed,

    textDecoder2,
    textDecoderASCII,
    textDecoderErrorEncoding,

    textEncoder,
    textEncodeInto,
    textEncodeInto2,
    textEncodeInto3,

    // Extra work because Serde won't accept lone surrogate pairs.
    // https://linear.app/convex/issue/CX-3317/support-using-encoding-functions-on-lone-surrogate-pairs
    //loneSurrogateEncodeInto,
    //loneSurrogateEncodeInto2,
    //loneSurrogateEncodeInto3,
    //swappedSurrogatePairEncodeInto4,

    textDecoderSharedUint8Array,
    textDecoderSharedInt32Array,
    toStringShouldBeWebCompatibility,
    textEncoderShouldCoerceToString,

    // Tests from here down were not taken from Deno
    atobCorrectAlphabet,
    onlyAsciiWhitespaceRemoved,
  });
});
