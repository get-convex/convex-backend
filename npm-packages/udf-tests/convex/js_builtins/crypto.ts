// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

import { wrapInTests } from "./testHelpers";
import { assert, expect } from "chai";
import { query } from "../_generated/server.js";

// -----------------------------------------------------------------------------
// Begin tests from Deno
// -----------------------------------------------------------------------------
function getRandomValuesInt8Array() {
  const arr = new Int8Array(32);
  crypto.getRandomValues(arr);
  assert.notDeepEqual(arr, new Int8Array(32));
}

function getRandomValuesUint8Array() {
  const arr = new Uint8Array(32);
  crypto.getRandomValues(arr);

  assert.notDeepEqual(arr, new Uint8Array(32));
}

function getRandomValuesUint8ClampedArray() {
  const arr = new Uint8ClampedArray(32);
  crypto.getRandomValues(arr);
  assert.notDeepEqual(arr, new Uint8ClampedArray(32));
}

function getRandomValuesInt16Array() {
  const arr = new Int16Array(4);
  crypto.getRandomValues(arr);
  assert.notDeepEqual(arr, new Int16Array(4));
}

function getRandomValuesUint16Array() {
  const arr = new Uint16Array(4);
  crypto.getRandomValues(arr);
  assert.notDeepEqual(arr, new Uint16Array(4));
}

function getRandomValuesInt32Array() {
  const arr = new Int32Array(8);
  crypto.getRandomValues(arr);
  assert.notDeepEqual(arr, new Int32Array(8));
}

function getRandomValuesBigInt64Array() {
  const arr = new BigInt64Array(8);
  crypto.getRandomValues(arr);
  assert.notDeepEqual(arr, new BigInt64Array(8));
}

function getRandomValuesUint32Array() {
  const arr = new Uint32Array(8);
  crypto.getRandomValues(arr);
  assert.notDeepEqual(arr, new Uint32Array(8));
}

function getRandomValuesBigUint64Array() {
  const arr = new BigUint64Array(8);
  crypto.getRandomValues(arr);
  assert.notDeepEqual(arr, new BigUint64Array(8));
}

function getRandomValuesReturnValue() {
  const arr = new Uint32Array(8);
  const rtn = crypto.getRandomValues(arr);
  assert.notDeepEqual(arr, new Uint32Array(8));
  assert(rtn === arr, "Returned array did not strict equal original array");
}

function getRandomValuesFloatArray() {
  const arr = new Float32Array(32);
  assert.throws(
    () => crypto.getRandomValues(arr),
    DOMException,
    /not an integer array type/,
  );
}

function randomUUID() {
  // vaguely resembles what browsers do
  assert.strictEqual(crypto.randomUUID().length, 36);
  assert.isString(crypto.randomUUID());
}

// https://github.com/denoland/deno/issues/11664
async function testImportArrayBufferKey() {
  const subtle = crypto.subtle;
  assert(subtle);

  // deno-fmt-ignore
  const key = new Uint8Array([
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
  ]);

  const cryptoKey = await subtle.importKey(
    "raw",
    key.buffer,
    { name: "HMAC", hash: "SHA-1" },
    true,
    ["sign"],
  );
  assert(cryptoKey);

  // Test key usage
  await subtle.sign({ name: "HMAC" }, cryptoKey, new Uint8Array(8));
}

// async function testSignVerify() {
//   const subtle = crypto.subtle;
//   assert(subtle);
//   for (const algorithm of ["RSA-PSS", "RSASSA-PKCS1-v1_5"]) {
//     for (const hash of ["SHA-1", "SHA-256", "SHA-384", "SHA-512"]) {
//       const keyPair = await subtle.generateKey(
//         {
//           name: algorithm,
//           modulusLength: 2048,
//           publicExponent: new Uint8Array([1, 0, 1]),
//           hash,
//         },
//         true,
//         ["sign", "verify"],
//       );

//       const data = new Uint8Array([1, 2, 3]);

//       const signAlgorithm = { name: algorithm, saltLength: 32 };

//       const signature = await subtle.sign(
//         signAlgorithm,
//         keyPair.privateKey,
//         data,
//       );

//       assert(signature);
//       assert(signature.byteLength > 0);
//       assert(signature.byteLength % 8 == 0);
//       assert(signature instanceof ArrayBuffer);

//       const verified = await subtle.verify(
//         signAlgorithm,
//         keyPair.publicKey,
//         signature,
//         data,
//       );
//       assert(verified);
//     }
//   }
// }

// deno-fmt-ignore
const plainText = new Uint8Array([
  95, 77, 186, 79, 50, 12, 12, 232, 118, 114, 90, 252, 229, 251, 210, 91, 248,
  62, 90, 113, 37, 160, 140, 175, 231, 60, 62, 186, 196, 33, 119, 157, 249, 213,
  93, 24, 12, 58, 233, 148, 38, 69, 225, 216, 47, 238, 140, 157, 41, 75, 60,
  177, 160, 138, 153, 49, 32, 27, 60, 14, 129, 252, 71, 202, 207, 131, 21, 162,
  175, 102, 50, 65, 19, 195, 182, 98, 48, 195, 70, 8, 196, 244, 89, 54, 52, 206,
  2, 178, 103, 54, 34, 119, 240, 168, 64, 202, 116, 188, 61, 26, 98, 54, 149,
  44, 94, 215, 170, 248, 168, 254, 203, 221, 250, 117, 132, 230, 151, 140, 234,
  93, 42, 91, 159, 183, 241, 180, 140, 139, 11, 229, 138, 48, 82, 2, 117, 77,
  131, 118, 16, 115, 116, 121, 60, 240, 38, 170, 238, 83, 0, 114, 125, 131, 108,
  215, 30, 113, 179, 69, 221, 178, 228, 68, 70, 255, 197, 185, 1, 99, 84, 19,
  137, 13, 145, 14, 163, 128, 152, 74, 144, 25, 16, 49, 50, 63, 22, 219, 204,
  157, 107, 225, 104, 184, 72, 133, 56, 76, 160, 62, 18, 96, 10, 193, 194, 72,
  2, 138, 243, 114, 108, 201, 52, 99, 136, 46, 168, 192, 42, 171,
]);

// Passing
const hashPlainTextVector = [
  {
    hash: "SHA-1",
    plainText: plainText.slice(0, 214),
  },
  {
    hash: "SHA-256",
    plainText: plainText.slice(0, 190),
  },
  {
    hash: "SHA-384",
    plainText: plainText.slice(0, 158),
  },
  {
    hash: "SHA-512",
    plainText: plainText.slice(0, 126),
  },
];

// async function testEncryptDecrypt() {
//   const subtle = crypto.subtle;
//   assert(subtle);
//   for (const { hash, plainText } of hashPlainTextVector) {
//     const keyPair = await subtle.generateKey(
//       {
//         name: "RSA-OAEP",
//         modulusLength: 2048,
//         publicExponent: new Uint8Array([1, 0, 1]),
//         hash,
//       },
//       true,
//       ["encrypt", "decrypt"],
//     );

//     const encryptAlgorithm = { name: "RSA-OAEP" };
//     const cipherText = await subtle.encrypt(
//       encryptAlgorithm,
//       keyPair.publicKey,
//       plainText,
//     );

//     assert(cipherText);
//     assert(cipherText.byteLength > 0);
//     assertEquals(cipherText.byteLength * 8, 2048);
//     assert(cipherText instanceof ArrayBuffer);

//     const decrypted = await subtle.decrypt(
//       encryptAlgorithm,
//       keyPair.privateKey,
//       cipherText,
//     );
//     assert(decrypted);
//     assert(decrypted instanceof ArrayBuffer);
//     assertEquals(new Uint8Array(decrypted), plainText);

//     const badPlainText = new Uint8Array(plainText.byteLength + 1);
//     badPlainText.set(plainText, 0);
//     badPlainText.set(new Uint8Array([32]), plainText.byteLength);
//     await assertRejects(async () => {
//       // Should fail
//       await subtle.encrypt(encryptAlgorithm, keyPair.publicKey, badPlainText);
//       throw new TypeError("unreachable");
//     }, DOMException);
//   }
// }

// async function testGenerateRSAKey() {
//   const subtle = crypto.subtle;
//   assert(subtle);

//   const keyPair = await subtle.generateKey(
//     {
//       name: "RSA-PSS",
//       modulusLength: 2048,
//       publicExponent: new Uint8Array([1, 0, 1]),
//       hash: "SHA-256",
//     },
//     true,
//     ["sign", "verify"],
//   );

//   assert(keyPair.privateKey);
//   assert(keyPair.publicKey);
//   assertEquals(keyPair.privateKey.extractable, true);
//   assert(keyPair.privateKey.usages.includes("sign"));
// }

// async function testGenerateHMACKey() {
//   const key = await crypto.subtle.generateKey(
//     {
//       name: "HMAC",
//       hash: "SHA-512",
//     },
//     true,
//     ["sign", "verify"],
//   );

//   assert(key);
//   assertEquals(key.extractable, true);
//   assert(key.usages.includes("sign"));
// }

// async function testECDSASignVerify() {
//   const key = await crypto.subtle.generateKey(
//     {
//       name: "ECDSA",
//       namedCurve: "P-384",
//     },
//     true,
//     ["sign", "verify"],
//   );

//   const encoder = new TextEncoder();
//   const encoded = encoder.encode("Hello, World!");
//   const signature = await crypto.subtle.sign(
//     { name: "ECDSA", hash: "SHA-384" },
//     key.privateKey,
//     encoded,
//   );

//   assert(signature);
//   assert(signature instanceof ArrayBuffer);

//   const verified = await crypto.subtle.verify(
//     { hash: { name: "SHA-384" }, name: "ECDSA" },
//     key.publicKey,
//     signature,
//     encoded,
//   );
//   assert(verified);
// }

// Tests the "bad paths" as a temporary replacement for sign_verify/ecdsa WPT.
// async function testECDSASignVerifyFail() {
//   const key = await crypto.subtle.generateKey(
//     {
//       name: "ECDSA",
//       namedCurve: "P-384",
//     },
//     true,
//     ["sign", "verify"],
//   );

//   const encoded = new Uint8Array([1]);
//   // Signing with a public key (InvalidAccessError)
//   await assertRejects(async () => {
//     await crypto.subtle.sign(
//       { name: "ECDSA", hash: "SHA-384" },
//       key.publicKey,
//       new Uint8Array([1]),
//     );
//     throw new TypeError("unreachable");
//   }, DOMException);

//   // Do a valid sign for later verifying.
//   const signature = await crypto.subtle.sign(
//     { name: "ECDSA", hash: "SHA-384" },
//     key.privateKey,
//     encoded,
//   );

//   // Verifying with a private key (InvalidAccessError)
//   await assertRejects(async () => {
//     await crypto.subtle.verify(
//       { hash: { name: "SHA-384" }, name: "ECDSA" },
//       key.privateKey,
//       signature,
//       encoded,
//     );
//     throw new TypeError("unreachable");
//   }, DOMException);
// }

// https://github.com/denoland/deno/issues/11313
// async function testSignRSASSAKey() {
//   const subtle = crypto.subtle;
//   assert(subtle);

//   const keyPair = await subtle.generateKey(
//     {
//       name: "RSASSA-PKCS1-v1_5",
//       modulusLength: 2048,
//       publicExponent: new Uint8Array([1, 0, 1]),
//       hash: "SHA-256",
//     },
//     true,
//     ["sign", "verify"],
//   );

//   assert(keyPair.privateKey);
//   assert(keyPair.publicKey);
//   assertEquals(keyPair.privateKey.extractable, true);
//   assert(keyPair.privateKey.usages.includes("sign"));

//   const encoder = new TextEncoder();
//   const encoded = encoder.encode("Hello, World!");

//   const signature = await crypto.subtle.sign(
//     { name: "RSASSA-PKCS1-v1_5" },
//     keyPair.privateKey,
//     encoded,
//   );

//   assert(signature);
// }

const jwk: JsonWebKey = {
  kty: "oct",
  // unpadded base64 for rawKey.
  k: "AQIDBAUGBwgJCgsMDQ4PEA",
  alg: "HS256",
  ext: true,
  key_ops: ["sign"],
};

async function subtleCryptoHmacImportExport() {
  const rawKey = new Uint8Array([
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
  ]);

  const key1 = await crypto.subtle.importKey(
    "raw",
    rawKey,
    { name: "HMAC", hash: "SHA-256" },
    true,
    ["sign"],
  );
  const key2 = await crypto.subtle.importKey(
    "jwk",
    jwk,
    { name: "HMAC", hash: "SHA-256" },
    true,
    ["sign"],
  );
  const actual1 = await crypto.subtle.sign(
    { name: "HMAC" },
    key1,
    new Uint8Array([1, 2, 3, 4]),
  );
  const actual2 = await crypto.subtle.sign(
    { name: "HMAC" },
    key2,
    new Uint8Array([1, 2, 3, 4]),
  );
  // deno-fmt-ignore
  const expected = new Uint8Array([
    59, 170, 255, 216, 51, 141, 51, 194, 213, 48, 41, 191, 184, 40, 216, 47,
    130, 165, 203, 26, 163, 43, 38, 71, 23, 122, 222, 1, 146, 46, 182, 87,
  ]);
  assert.deepEqual(actual1, expected.buffer);
  assert.deepEqual(actual2, expected.buffer);

  // TODO: CX-4407
  // const exportedKey1 = await crypto.subtle.exportKey("raw", key1);
  // assertEquals(new Uint8Array(exportedKey1), rawKey);

  // const exportedKey2 = await crypto.subtle.exportKey("jwk", key2);
  // assertEquals(exportedKey2, jwk);
}

// 2048-bits publicExponent=65537
// const pkcs8TestVectors = [
//   // rsaEncryption
//   { pem: "cli/tests/testdata/webcrypto/id_rsaEncryption.pem", hash: "SHA-256" },
// ];

const rsaPemFile = `-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDYKBqT674QjFgP
LciOxf8UNtfIligtwGc+RjuNRdzT2z7sduGPqxfW7y6MYm/tP92GjUKNf2/XzLkI
Ay5pPtHKv21YTAH4GOFqNJL3FzYVJtf8zZmvFhXY2BVIhu6JFkaFCBYC4+Is5nMM
kP12FZvf6LbTTVQVMRV5wyRXP1gzQ16fHE9c115mhzJSVkohtqHw5mFfEV/I7DEB
QRTn+hvv5r4gGc7qn9gewwmlOh+6ow1lc7WNWNn7kigsW9hgLGsGZvkaUge0jOLs
QPzEWSdzmiOnpMxYLK1XjC73G3+0OHUjtRdG9q3cr7/yqKO5uI0Ays1q40t2UZrH
rMBeFSi/AgMBAAECggEAE7NMANFKiE2SNQfyMHkBL4F0FzcAQHM5taZHBTAp2TEx
QfHvyt1IFfHEp0zNcK0SbpHvT+AefGePMZjAoRz1l+nseFCtGUSDPt+9yUFXT4Qz
yTmf2SJFKXdAMVUC5oGeOb+r6eWFFpyGPc31G88KXtTh3M4+bJQFpgxQApemXT2K
vvkUppWr/LKIrLQ9TboBAWOOMtuZ0+nDbCzFwdRvQ1itR/Jbq4swt4+PTXKkWrHN
gXoxCoMXaS/QW4BxzlnCAFelEvguF95VKPFgzf6AvHp7mun4tHYgePxugsG4xpRR
U3fCBIqHp8LunG8TpCIY4jeUE4vfNgxWgJ/E+oSOWQKBgQDundzft515Ul2k+wb2
HaQbaVafYyDmwK/Ji8tthFVJqYPo7eWFbIsedAEGUBzxYd8WRJS59/jyV/MQ3mGS
Y2leCOVo/oGIxqugJNQl0NSnUYleXeweTWMeUhbbMPGutMHLe9hGByzOBwE4G1Oj
Yb8bu41WMzT9T1YBm8cuBQS02wKBgQDn5145sxlMDezj0kcOU+gQLXJfsIPhVIsf
aiLlH78BLDOTH3J+INMYnI869r1bCOlUjNjf3bdBgPA+YQQIPExmhQARAPk1riEp
x7O9egpcxlYzE7yNtR5LeEh5kW+nlSr6EcPPONJlHYgyv9DGxjhnA3lI93rBcqYy
0xSFF41O7QKBgQCiW2deEWFkm1Z8WxFxhNmUjSgTay+H0rPJPwU7jz84z86hPr1c
+23tWqEX2orW8vEIBcHsh30r6AvK/oUFRf77rLHrrsAUgJlmbair0lvfPOtq+h0e
wSkgCFrk6XiIlxUFj06o11j1Fm8N7goKsQeHpyWT8WOst76deZEdDu0U4QKBgEs/
HqrYO0AbUJ9HrriubyE4reDwtIob1ZyW4sW3vFFUF1QIoyzb18Mnoa3/O8fbJ2LA
5OoW1gySGuIStq05a4zkYPYje7l4S9hzrRWxEMWzsWqXX9oXR8IzQEj58OHOnAhS
VVfa1yHqKDRXWxX0YX7DeMo9Sv6UBet95C2fS8GtAoGALvUAEM/LEj/NY+J0542F
+b8FyXtosQ1mSKBV03JmmglDfQW/+O0Illuy8CmfIuqftLwHyc15u5cAIDd5a8jm
S8l+7/KYETXYGCA1mwB/Dr/LJwbVEQ/mw1ne6FrJ4MORd/2GWuvzrqTKrwRcJ+O9
mZ3aPB+XECOydno9s4rsxII=
-----END PRIVATE KEY-----`;

async function importRsaPkcs8() {
  const pemHeader = "-----BEGIN PRIVATE KEY-----";
  const pemFooter = "-----END PRIVATE KEY-----";
  const hash = "SHA-256";
  const keyFile = rsaPemFile;
  const pemContents = keyFile.substring(
    pemHeader.length,
    keyFile.length - pemFooter.length,
  );
  const binaryDerString = atob(pemContents);
  const binaryDer = new Uint8Array(binaryDerString.length);
  for (let i = 0; i < binaryDerString.length; i++) {
    binaryDer[i] = binaryDerString.charCodeAt(i);
  }

  const key = await crypto.subtle.importKey(
    "pkcs8",
    binaryDer,
    { name: "RSA-PSS", hash },
    true,
    ["sign"],
  );

  assert(key);
  assert.strictEqual(key.type, "private");
  assert.strictEqual(key.extractable, true);
  assert.deepEqual(key.usages, ["sign"]);
  const algorithm = key.algorithm as RsaHashedKeyAlgorithm;
  assert.strictEqual(algorithm.name, "RSA-PSS");
  assert.strictEqual(algorithm.hash.name, hash);
  assert.strictEqual(algorithm.modulusLength, 2048);
  assert.deepEqual(algorithm.publicExponent, new Uint8Array([1, 0, 1]));
}

const pem1 = `-----BEGIN PRIVATE KEY-----
MIIE7gIBADA9BgkqhkiG9w0BAQowMKANMAsGCWCGSAFlAwQCAaEaMBgGCSqGSIb3
DQEBCDALBglghkgBZQMEAgGiAwIBHgSCBKgwggSkAgEAAoIBAQClbSUIKRfpCk0W
WuKv4vzTnAW98n1LDwSbU0larwckrBe19waFaCrh1jiIaSzTjCR9UtHDMDaUuOu3
0Z33Yrsa3U/t6vVl+JVsSeqXTBbGXTZ1iXuCk1DywIAiBwrZJ4IH00p7zMJ7dN0B
peQ6oirGAiaEBGonq6cdAvE0WpDkEQ1lZ3YjSWggwMpkOv24dl1SwbANQzvX4s66
WD70LhmS2MwC9EyUz3ddV3Ha5S3pZrGis8CJ5kBMR65BMYwsaDEWjDkNd7fNFC2D
HiINO8GUm3Tb7FSqzqVCcrOPQYcb/3gBQIIFrGS5zk9KXIcJ6382iomN+f9EGhDQ
S9FQRgF9AgMBAAECggEAIt3HQVoP7RE7wCt1veaUrTEkUK2sVMacjPRX8KIVWlhf
Qv4GxNV4vFK/ZZjtVsUh57wt8/rUdjInlH280qhfkUD2GMo94LktHT7TEAJ8hmCd
GtbYTmQoWpHSL9nWIoDeOjIBb+hvUUAHWNKTMPx/EW7gmVCo4yWdulKlbghso3TR
1M4jbryhplLDzElPPUhmahT2De64CAt9OkDZV8b+47E6CU33qrlFCBOgurQGiIVK
GgmrIxu0efR9n9Df5BqLHTI5v9nqyugrnWYN6vdk/YWZK+RsHo1iqEhge5CPrI48
M6XIES1EQeSF2cdJaob9gsmyzszODw/w8smT5DgNwQKBgQDVZuvgqR9zcpIpMjtI
gFQvBIrHFyO6C48lQ37hlGpSjcKsWevTVRMw6Ywrm6Ls1AlKQ02tMETA9rcnOgj5
lHwOB1Neb41XsBkNgqwelqSAZkxmIKESN4/EBdVX89hdv6lLZ58IniWYfzEAZKoc
oZKelrU+wuk58ny4H7WwdBQjEQKBgQDGcpzdf/Hw60KWdq5fydlG0OwzK6t9TIfB
AZsxcEpHjckfe8/WJLcSZ4zD6kBSiBuLJ4G0l/rMYiF3I6dFfta+2tv8GuQ6n6ji
G859GMBJvqwLbHFFZb56PCtTtoYuI94Qiv1i0CjAfpR9uajLFJ/4aBmKBP04higM
mOQkDTdfrQKBgQDFedZxMU+/X5hNswQVFVvRnpxlm84dzbCLRShWyyaQekpZf2Sx
TZrSumxRtlZQXe6y4BEzYOVew2+9RwEjI/qgaLsHOqdLK9QjInbwX2qevwuXvj4P
Q4cwWE2SdD4WktOwyZTrPp1/vsOzn3OjxwiM5N2X1HpKe1Baek2BmglAYQKBgDOe
Zm07NZycJVLsBgmGgIzqFTZuOoo6GOm8WDjw43FfURxuYS5rmG8iHjxrt1WAb+Gv
Yg6upZ76O4g47u6lwogcd7JI8GrLUuuVng1uHb5Q6YGDMKeDgptbAU4iIR7pV37o
GGbWjHMdudRGMcC5Wa8MrO/4wkEsrEgykM1L3sZ1AoGBAMUtQvrtTDIQml38V+O5
2j3VFv/hmhE/+XMKd2DXl9aRiG6QMe3BVWcLP/rq2ZaxwRjAZg5TO0S9Dptn9eX+
0yASf6VghLGs+wVnWGseyZRcYHRv2O6+I8zzpGd4uSaB7uoBMFsEEp+lUTRe52EW
gFHhVC1yOzlseecZ7wWszDeO
-----END PRIVATE KEY-----`;

const pem2 = `-----BEGIN PRIVATE KEY-----
MIIEwwIBADASBgkqhkiG9w0BAQowBaIDAgEABIIEqDCCBKQCAQACggEBAMPqex5m
1JO1SOxN6F+yNN9AV1bhoGHpx7rqvOAjIadTSmK9g17GA8r5vYk/J/wVdPLOnVG2
ARAzk9T554rYd4fAN3uL9CzsK2eBb7CD/OAbq1a3RLGhFrNILot7rK2QnTPuDbAh
HUWGwsW+PQYhPW1JzdnOvfRD4DEZfzz5in6SZWmHvXCMdpewgWdjzQ9O1+MxNVe4
diw43q3fvDEZSGGgnV3eVp+adQxIq0pfNq6jB71CE82vZYMXAJ+MeypX6+xn9OG0
NzXXUsYTQjchNUALZTG4YM5ErGJCB/cikEwqscktirmU3l1lAN193wgykYUDGV8a
JdlMUwRe68vsoI8CAwEAAQKCAQABnH0Uu+3FpTkLUHy3xMRwjZvqSALEq2KMJAAX
q9JMCQBUnZBmCCTh13n6lf1jMl363T4n/OI3WuU9XCzOVIdvI2KRbo48jFizCjp7
7in3QeL/3JQBDf0czlwro4HBD65rTero5uzRtJEHhVAFK+LQcknyH6QqTSCb5sTI
IJlF/zSMc596zLsSdo/CdNBSvdlOXLoypJ7APW/dKs0nQ58dJ2yhNVkFfqnsckg1
i1M12WMVUlwQDyqb7OlodLr45SN9CM4EVXES6/D1I4WTHQQgVK+nyVv/1UgGWn8w
ZJv7z5WbA+fdCPRXNf9VRkVpL2TDF32Aouu9Ck729U/KesghAoGBAOl2SBiyeK+Y
z5G/XXNOUWvVoLKv70Fy8pevvv9uyIOvxriADLRFRxcObG0rgLrCYQhjsWzhXBss
5lY8Cnm4dBFxZsAMHYBB5C99bT/OX+D4Gh7JG302wehp/2nrl9Wqs5NV6p9r0nyH
PQdPzGTBRnHG2uzor9pawU6Sb7FoIfxfAoGBANbUS4zLRqiBqjXw5TVkxGetm6St
GM+gA0JuYDqXM53a6p6/brwTO2u2hQPIphTGc+ZQzjaimUQeX4cRRm7cs7Nzvb0U
iiwYCf4zDEKzhGvdKsA/SOyGR3a5yyV1jWZdRvNra1kL08bsUYMpMOomPlDoTtZv
kJaOMblGlRskWcnRAoGBAMOeqraXBO0N/A9B7Anr++mBbU+Mf8u1h3R2fHIH39gH
91ktYnfC/Nhi65NmOk1DBo9DCa4T/1t9+dsUICrx1b+v58rP3ABWNd9dF6e5Qzl8
alaVaIU41q3p82xLTkRo7mNsQFYowIw7vXVc9gUOtfosB9Eu//rUxNkTdqeOe7u9
AoGBAMEW9YTp3GtuiCeNiubP2Hx7nU6JSqirYOKfxJxE9N7oOkNPOt+OxbTNy4aY
HTbFHL0hWgffY7THPANxsoXIlzgvSpYdVQfG34x8I4P8SISOuHMtLoVxN+BtpDra
Cqq8Ih5+KXFS4RmFpMooBtAeaZpdAydYBXRdADJQT4qixJVRAoGAOIF7KIGi0TBs
nr/u4Hqyj+wc0DkQFJa9ILvLV7r+k33KYjiVZeBfbDKDLMSXCHMyTSfLjmDI9ZVd
HCo6+cUw4dwXsquKn3i+nmjMc4G82o48EQYfGoRlNb7fnyBVdULIV0lwRRsXn29A
dfNGr0wbasKwbrjL6XZerA0saoNdEOw=
-----END PRIVATE KEY-----`;

const pem3 = `-----BEGIN PRIVATE KEY-----
MIIEwQIBADASBgkqhkiG9w0BAQowBaIDAgEeBIIEpjCCBKICAQACggEBAMEWS3x9
dCE5KWAoh5UAXeq2E0uPWBJ2UrtX7asgrOX1Sj4Boak/zxpsgPGQeA3kQPlcD9xi
wxnlBMFqrMwoY1Trb5quX1ZU8NVE/w3kWkF2EhHKgbWt9LyoNsNkj7liw5Aez4fN
O2v+IIsqeVJslXa6+6jfe1xWSEcRLOpfjCQ76bA9I5i9wls/ThvPnlEuYD7jFaXb
EJiKcaYxQ+L+Ys0CC5lPLGnLb3NRgeQhtBjmaTOMh2PrXoaZ9RuuzpDHJ7Iw3Inl
qh9AbH5um2TEh0ZpIhPaC5grwlbMpoTLW46MAYynme2+FsD20paszPQL9afiuUll
9PjLO3Lr3JNWxyECAwEAAQKCAQAhdReHbsWcrCb7Poqfyvx7GymkwiXkrRQQ2l+Y
c4UCI8rFi8rTZPciAQMm2H21CoQnsUgeTA66gfCdAzsF9UmhAVrJVsW2U+mXSulX
IuZwyWDALHLlZEswFYXHvbkZyn9QjcYwJePSBqrk8msrWR1dAXFyBad+jIThq5w4
0G2cKWh1sHKTcIa3UZ6jnAB0WC/Fkv3hjybVn601Axq9YdVTbKAxfBPdfTmBXqc5
P98ghRqPrjnFkDET8LZsDJjAuwSL9S79QAQmPmOlKuNCWB2gQ7/4aIcgtiyGOTvK
3sD4f3aL+E8caTpJQQDQanQO7DRghci+grb2bVDAkhg7/cdBAoGBAOQRDc3KSQlT
Jl8W2pa8PvtTYcK+QQ0GCKMDeFyAMqvtFo216NjgTh92WZArTTQVFMzorXNSWUbq
tAqMa9lc6f7CtM/hhuxkKL1o93Hbb69pk4eohzhk5uAleQ9jhOi4ABTwONV9HDt6
xl8CDoZiBpqjR1hlHFSfVLDGKtG+8I/NAoGBANi8eHWMvYicyND21Kf9s4RZuVVT
sf2xj1wCv8M3Or72PRtINOGdH2ASELpyyU9nLwzPaEnQ9nwmAI3YUs9j7lqkyFv0
aHK/F9JZ8IZi7paRsIl2lXkXBAWICG5eoBVvpl3KWzh50n0rEDaXhTJuToBVQ0BV
MiX1or6pcZUEanilAoGASg2/jbLBRGXbb8Tb9VXqnXDVrYZZWQE8jLHzwxVdXrX9
PMZ0dPdgZpbnPgjRaLfvqRlkOK3kj0Jmc4Zk/o9M64wNafKw/NEI6XfL4Qx/l1WQ
sdvnDEi3LtD8HiMSZP5aCHJ4Ado98JJNF0xzqu7pdgzOfcVXDaMuvLeb778wHYEC
gYA0bjN9zFQ1biguVOfQ09DPnZz2BU8znfaePZQCN6QgehUCOo+AXLAwX25ojEgi
y0VYhfwmj0RxeAf+SGyP+w64ItDNXey+hXfPzS4gdGJfTlM0jdlO98BjTisr9/wl
82J9oew7V00SNo6vhiwUrRaUeQvRzkpZYHjEQt1VPUI8eQKBgFkjt/rEEgpojPIR
iC4qR35yRoTvFjOLj5WQPZmgEjeqWoE/xXMg58UXo74ViSavEP+8xQH+uSXXAG4q
G4298Uc9jNF0jTekEKI8yyi2MBThDOtQOTNZb/exAgrXkff/g1RG+XIbgHgx8qRR
XII4DFaG+/+toV/27x4mfuddXWNu
-----END PRIVATE KEY-----`;

const nonInteroperableVectors = [
  // id-RSASSA-PSS (sha256)
  // `openssl genpkey -algorithm rsa-pss -pkeyopt rsa_pss_keygen_md:sha256 -out id_rsassaPss.pem`
  { pem: pem1, hash: "SHA-256" },
  // id-RSASSA-PSS (default parameters)
  // `openssl genpkey -algorithm rsa-pss -out id_rsassaPss.pem`
  {
    pem: pem2,
    hash: "SHA-1",
  },
  // id-RSASSA-PSS (default hash)
  // `openssl genpkey -algorithm rsa-pss -pkeyopt rsa_pss_keygen_saltlen:30 -out rsaPss_saltLen_30.pem`
  {
    pem: pem3,
    hash: "SHA-1",
  },
];

async function importNonInteroperableRsaPkcs8() {
  const pemHeader = "-----BEGIN PRIVATE KEY-----";
  const pemFooter = "-----END PRIVATE KEY-----";
  for (const { pem, hash } of nonInteroperableVectors) {
    const keyFile = pem;
    const pemContents = keyFile.substring(
      pemHeader.length,
      keyFile.length - pemFooter.length,
    );
    const binaryDerString = atob(pemContents);
    const binaryDer = new Uint8Array(binaryDerString.length);
    for (let i = 0; i < binaryDerString.length; i++) {
      binaryDer[i] = binaryDerString.charCodeAt(i);
    }

    await expect(
      crypto.subtle.importKey(
        "pkcs8",
        binaryDer,
        { name: "RSA-PSS", hash },
        true,
        ["sign"],
      ),
    ).to.be.rejectedWith(
      // TODO: should be thrown as DOMException
      // DOMException,
      "unsupported algorithm",
    );
  }
}

const jwtRSAKeys = {
  "1024": {
    size: 1024,
    publicJWK: {
      kty: "RSA",
      n: "zZn4sRGfjQos56yL_Qy1R9NI-THMnFynn94g5RxA6wGrJh4BJT3x6I9x0IbpS3q-d4ORA6R2vuDMh8dDFRr9RDH6XY-gUScc9U5Jz3UA2KmVfsCbnUPvcAmMV_ENA7_TF0ivVjuIFodyDTx7EKHNVTrHHSlrbt7spbmcivs23Zc",
      e: "AQAB",
    },
    privateJWK: {
      kty: "RSA",
      n: "zZn4sRGfjQos56yL_Qy1R9NI-THMnFynn94g5RxA6wGrJh4BJT3x6I9x0IbpS3q-d4ORA6R2vuDMh8dDFRr9RDH6XY-gUScc9U5Jz3UA2KmVfsCbnUPvcAmMV_ENA7_TF0ivVjuIFodyDTx7EKHNVTrHHSlrbt7spbmcivs23Zc",
      e: "AQAB",
      d: "YqIK_GdH85F-GWZdgfgmv15NE78gOaL5h2g4v7DeM9-JC7A5PHSLKNYn87HFGcC4vv0PBIBRtyCA_mJJfEaGWORVCOXSBpWNepMYpio52n3w5uj5UZEsBnbtZc0EtWhVF2Auqa7VbiKrWcQUEgEI8V0gE5D4tyBg8GXv9975dQE",
      p: "9BrAg5L1zfqGPuWJDuDCBX-TmtZdrOI3Ys4ZaN-yMPlTjwWSEPO0qnfjEZcw2VgXHgJJmbVco6TxckJCmEYqeQ",
      q: "157jDJ1Ya5nmQvTPbhKAPAeMWogxCyaQTkBrp30pEKd6mGSB385hqr4BIk8s3f7MdXpM-USpaZgUoT4o_2VEjw",
      dp: "qdd_QUzcaB-6jkKo1Ug-1xKIAgDLFsIjJUUfWt_iHL8ti2Kl2dOnTcCypgebPm5TT1bqHN-agGYAdK5zpX2UiQ",
      dq: "hNRfwOSplNfhLvxLUN7a2qA3yYm-1MSz_1DWQP7srlLORlUcYPht2FZmsnEeDcAqynBGPQUcbG2Av_hgHz2OZw",
      qi: "zbpJQAhinrxSbVKxBQ2EZGFUD2e3WCXbAJRYpk8HVQ5AA52OhKTicOye2hEHnrgpFKzC8iznTsCG3FMkvwcj4Q",
    },
  },

  "2048": {
    size: 2048,
    publicJWK: {
      kty: "RSA",
      // unpadded base64 for rawKey.
      n: "09eVwAhT9SPBxdEN-74BBeEANGaVGwqH-YglIc4VV7jfhR2by5ivzVq8NCeQ1_ACDIlTDY8CTMQ5E1c1SEXmo_T7q84XUGXf8U9mx6uRg46sV7fF-hkwJR80BFVsvWxp4ahPlVJYj__94ft7rIVvchb5tyalOjrYFCJoFnSgq-i3ZjU06csI9XnO5klINucD_Qq0vUhO23_Add2HSYoRjab8YiJJR_Eths7Pq6HHd2RSXmwYp5foRnwe0_U75XmesHWDJlJUHYbwCZo0kP9G8g4QbucwU-MSNBkZOO2x2ZtZNexpHd0ThkATbnNlpVG_z2AGNORp_Ve3rlXwrGIXXw",
      e: "AQAB",
    },
    privateJWK: {
      kty: "RSA",
      // unpadded base64 for rawKey.
      n: "09eVwAhT9SPBxdEN-74BBeEANGaVGwqH-YglIc4VV7jfhR2by5ivzVq8NCeQ1_ACDIlTDY8CTMQ5E1c1SEXmo_T7q84XUGXf8U9mx6uRg46sV7fF-hkwJR80BFVsvWxp4ahPlVJYj__94ft7rIVvchb5tyalOjrYFCJoFnSgq-i3ZjU06csI9XnO5klINucD_Qq0vUhO23_Add2HSYoRjab8YiJJR_Eths7Pq6HHd2RSXmwYp5foRnwe0_U75XmesHWDJlJUHYbwCZo0kP9G8g4QbucwU-MSNBkZOO2x2ZtZNexpHd0ThkATbnNlpVG_z2AGNORp_Ve3rlXwrGIXXw",
      e: "AQAB",
      d: "H4xboN2co0VP9kXL71G8lUOM5EDis8Q9u8uqu_4U75t4rjpamVeD1vFMVfgOehokM_m_hKVnkkcmuNqj9L90ObaiRFPM5QxG7YkFpXbHlPAKeoXD1hsqMF0VQg_2wb8DhberInHA_rEA_kaVhHvavQLu7Xez45gf1d_J4I4931vjlCB6cupbLL0H5hHsxbMsX_5nnmAJdL_U3gD-U7ZdQheUPhDBJR2KeGzvnTm3KVKpOnwn-1Cd45MU4-KDdP0FcBVEuBsSrsQHliTaciBgkbyj__BangPj3edDxTkb-fKkEvhkXRjAoJs1ixt8nfSGDce9cM_GqAX9XGb4s2QkAQ",
      dp: "mM82RBwzGzi9LAqjGbi-badLtHRRBoH9sfMrJuOtzxRnmwBFccg_lwy-qAhUTqnN9kvD0H1FzXWzoFPFJbyi-AOmumYGpWm_PvzQGldne5CPJ02pYaeg-t1BePsT3OpIq0Am8E2Kjf9polpRJwIjO7Kx8UJKkhg5bISnsy0V8wE",
      dq: "ZlM4AvrWIpXwqsH_5Q-6BsLJdbnN_GypFCXoT9VXniXncSBZIWCkgDndBdWkSzyzIN65NiMRBfZaf9yduTFj4kvOPwb3ch3J0OxGJk0Ary4OGSlS1zNwMl93ALGal1FzpWUuiia9L9RraGqXAUr13L7TIIMRobRjpAV-z7M-ruM",
      p: "7VwGt_tJcAFQHrmDw5dM1EBru6fidM45NDv6VVOEbxKuD5Sh2EfAHfm5c6oouA1gZqwvKH0sn_XpB1NsyYyHEQd3sBVdK0zRjTo-E9mRP-1s-LMd5YDXVq6HE339nxpXsmO25slQEF6zBrj1bSNNXBFc7fgDnlq-HIeleMvsY_E",
      q: "5HqMHLzb4IgXhUl4pLz7E4kjY8PH2YGzaQfK805zJMbOXzmlZK0hizKo34Qqd2nB9xos7QgzOYQrNfSWheARwVsSQzAE0vGvw3zHIPP_lTtChBlCTPctQcURjw4dXcnK1oQ-IT321FNOW3EO-YTsyGcypJqJujlZrLbxYjOjQE8",
      qi: "OQXzi9gypDnpdHatIi0FaUGP8LSzfVH0AUugURJXs4BTJpvA9y4hcpBQLrcl7H_vq6kbGmvC49V-9I5HNVX_AuxGIXKuLZr5WOxPq8gLTqHV7X5ZJDtWIP_nq2NNgCQQyNNRrxebiWlwGK9GnX_unewT6jopI_oFhwp0Q13rBR0",
    },
  },
  "4096": {
    size: 4096,
    publicJWK: {
      kty: "RSA",
      n: "2qr2TL2c2JmbsN0OLIRnaAB_ZKb1-Gh9H0qb4lrBuDaqkW_eFPwT-JIsvnNJvDT7BLJ57tTMIj56ZMtv6efSSTWSk9MOoW2J1K_iEretZ2cegB_aRX7qQVjnoFsz9U02BKfAIUT0o_K7b9G08d1rrAUohi_SVQhwObodg7BddMbKUmz70QNIS487LN44WUVnn9OgE9atTYUARNukT0DuQb3J-K20ksTuVujXbSelohDmLobqlGoi5sY_548Qs9BtFmQ2nGuEHNB2zdlZ5EvEqbUFVZ2QboG6jXdoos6qcwdgUvAhj1Hz10Ngic_RFqL7bNDoIOzNp66hdA35uxbwuaygZ16ikxoPj7eTYud1hrkyQCgeGw2YhCiKIE6eos_U5dL7WHRD5aSkkzsgXtnF8pVmStsuf0QcdAoC-eeCex0tSTgRw9AtGTz8Yr1tGQD9l_580zAXnE6jmrwRRQ68EEA7vohGov3tnG8pGyg_zcxeADLtPlfTc1tEwmh3SGrioDClioYCipm1JvkweEgP9eMPpEC8SgRU1VNDSVe1SF4uNsH8vA7PHFKfg6juqJEc5ht-l10FYER-Qq6bZXsU2oNcfE5SLDeLTWmxiHmxK00M8ABMFIV5gUkPoMiWcl87O6XwzA2chsIERp7Vb-Vn2O-EELiXzv7lPhc6fTGQ0Nc",
      e: "AQAB",
    },
    privateJWK: {
      kty: "RSA",
      n: "2qr2TL2c2JmbsN0OLIRnaAB_ZKb1-Gh9H0qb4lrBuDaqkW_eFPwT-JIsvnNJvDT7BLJ57tTMIj56ZMtv6efSSTWSk9MOoW2J1K_iEretZ2cegB_aRX7qQVjnoFsz9U02BKfAIUT0o_K7b9G08d1rrAUohi_SVQhwObodg7BddMbKUmz70QNIS487LN44WUVnn9OgE9atTYUARNukT0DuQb3J-K20ksTuVujXbSelohDmLobqlGoi5sY_548Qs9BtFmQ2nGuEHNB2zdlZ5EvEqbUFVZ2QboG6jXdoos6qcwdgUvAhj1Hz10Ngic_RFqL7bNDoIOzNp66hdA35uxbwuaygZ16ikxoPj7eTYud1hrkyQCgeGw2YhCiKIE6eos_U5dL7WHRD5aSkkzsgXtnF8pVmStsuf0QcdAoC-eeCex0tSTgRw9AtGTz8Yr1tGQD9l_580zAXnE6jmrwRRQ68EEA7vohGov3tnG8pGyg_zcxeADLtPlfTc1tEwmh3SGrioDClioYCipm1JvkweEgP9eMPpEC8SgRU1VNDSVe1SF4uNsH8vA7PHFKfg6juqJEc5ht-l10FYER-Qq6bZXsU2oNcfE5SLDeLTWmxiHmxK00M8ABMFIV5gUkPoMiWcl87O6XwzA2chsIERp7Vb-Vn2O-EELiXzv7lPhc6fTGQ0Nc",
      e: "AQAB",
      d: "uXPRXBhcE5-DWabBRKQuhxgU8ype5gTISWefeYP7U96ZHqu_sBByZ5ihdgyU9pgAZGVx4Ep9rnVKnH2lNr2zrP9Qhyqy99nM0aMxmypIWLAuP__DwLj4t99M4sU29c48CAq1egHfccSFjzpNuetOTCA71EJuokt70pm0OmGzgTyvjuR7VTLxd5PMXitBowSn8_cphmnFpT8tkTiuy8CH0R3DU7MOuINomDD1s8-yPBcVAVTPUnwJiauNuzestLQKMLlhT5wn-cAbYk36XRKdgkjSc2AkhHRl4WDqT1nzWYdh_DVIYSLiKSktkPO9ovMrRYiPtozfhl0m9SR9Ll0wXtcnnDlWXc_MSGpw18vmUBSJ4PIhkiFsvLn-db3wUkA8uve-iqqfk0sxlGWughWx03kGmZDmprWbXugCBHfsI4X93w4exznXH_tapxPnmjbhVUQR6p41MvO2lcHWPLwGJgLIoejBHpnn3TmMN0UjFZki7q9B_dJ3fXh0mX9DzAlC0sil1NgCPhMPq02393_giinQquMknrBvgKxGSfGUrDKuflCx611ZZlRM3R7YMX2OIy1g4DyhPzBVjxRMtm8PnIs3m3Hi-O-C_PHF93w9J8Wqd0yIw7SpavDqZXLPC6Cqi8K7MBZyVECXHtRj1bBqT-h_xZmFCDjSU0NqfOdgApE",
      p: "9NrXwq4kY9kBBOwLoFZVQc4kJI_NbKa_W9FLdQdRIbMsZZHXJ3XDUR9vJAcaaR75WwIC7X6N55nVtWTq28Bys9flJ9RrCTfciOntHEphBhYaL5ZTUl-6khYmsOf_psff2VaOOCvHGff5ejuOmBQxkw2E-cv7knRgWFHoLWpku2NJIMuGHt9ks7OAUfIZVYl9YJnw4FYUzhgaxemknjLeZ8XTkGW2zckzF-d95YI9i8zD80Umubsw-YxriSfqFQ0rGHBsbQ8ZOTd_KJju42BWnXIjNDYmjFUqdzVjI4XQ8EGrCEf_8_iwphGyXD7LOJ4fqd97B3bYpoRTPnCgY_SEHQ",
      q: "5J758_NeKr1XPZiLxXohYQQnh0Lb4QtGZ1xzCgjhBQLcIBeTOG_tYjCues9tmLt93LpJfypSJ-SjDLwkR2s069_IByYGpxyeGtV-ulqYhSw1nD2CXKMDGyO5jXDs9tJrS_UhfobXKQH03CRdFugyPkSNmXY-AafFynG7xLr7oYBC05FnhUXPm3VBTPt9K-BpqwYd_h9vkAWeprSPo83UlwcLMupSJY9LaHxhRdz2yi0ZKNwXXHRwcszGjDBvvzUcCYbqWqjzbEvFY6KtH8Jh4LhM46rHaoEOTernJsDF6a6W8Df88RthqTExcwnaQf0O_dlbjSxEIPfbxx8t1EQugw",
      dp: "4Y7Hu5tYAnLhMXuQqj9dgqU3PkcKYdCp7xc6f7Ah2P2JJHfYz4z4RD7Ez1eLyNKzulZ8A_PVHUjlSZiRkaYTBAEaJDrV70P6cFWuC6WpA0ZREQ1V7EgrQnANbGILa8QsPbYyhSQu4YlB1IwQq5_OmzyVBtgWA7AZIMMzMsMT0FuB_if-gWohBjmRN-vh0p45VUf6UW568-_YmgDFmMYbg1UFs7s_TwrNenPR0h7MO4CB8hP9vJLoZrooRczzIjljPbwy5bRG9CJfjTJ0vhj9MUT3kR1hHV1HJVGU5iBbfTfBKnvJGSI6-IDM4ZUm-B0R5hbs6s9cfOjhFmACIJIbMQ",
      dq: "gT4iPbfyHyVEwWyQb4X4grjvg7bXSKSwG1SXMDAOzV9tg7LwJjKYNy8gJAtJgNNVdsfVLs-E_Epzpoph1AIWO9YZZXkov6Yc9zyEVONMX9S7ReU74hTBd8E9b2lMfMg9ogYk9jtSPTt-6kigW4fOh4cHqZ6_tP3cgfLD3JZ8FDPHE4WaySvLDq49yUBO5dQKyIU_xV6OGhQjOUjP_yEoMmzn9tOittsIHTxbXTxqQ6c1FvU9O6YTv8Jl5_Cl66khfX1I1RG38xvurcHULyUbYgeuZ_Iuo9XreT73h9_owo9RguGT29XH4vcNZmRGf5GIvRb4e5lvtleIZkwJA3u78w",
      qi: "JHmVKb1zwW5iRR6RCeexYnh2fmY-3DrPSdM8Dxhr0F8dayi-tlRqEdnG0hvp45n8gLUskWWcB9EXlUJObZGKDfGuxgMa3g_xeLA2vmFQ12MxPsyH4iCNZvsgmGxx7TuOHrnDh5EBVnM4_de63crEJON2sYI8Ozi-xp2OEmAr2seWKq4sxkFni6exLhqb-NE4m9HMKlng1EtQh2rLBFG1VYD3SYYpMLc5fxzqGvSxn3Fa-Xgg-IZPY3ubrcm52KYgmLUGmnYStfVqGSWSdhDXHlNgI5pdAA0FzpyBk3ZX-JsxhwcnneKrYBBweq06kRMGWgvdbdAQ-7wSeGqqj5VPwA",
    },
  },
};

async function testImportRsaJwk() {
  const subtle = crypto.subtle;
  assert(subtle);

  for (const [_key, jwkData] of Object.entries(jwtRSAKeys)) {
    const { size, publicJWK, privateJWK } = jwkData;
    if (size < 2048) {
      continue;
    }

    // 1. Test import PSS
    for (const hash of ["SHA-1", "SHA-256", "SHA-384", "SHA-512"]) {
      const hashMapPSS: Record<string, string> = {
        "SHA-1": "PS1",
        "SHA-256": "PS256",
        "SHA-384": "PS384",
        "SHA-512": "PS512",
      };

      if (size === 1024 && hash === "SHA-512") {
        continue;
      }

      const _privateKeyPSS = await crypto.subtle.importKey(
        "jwk",
        {
          alg: hashMapPSS[hash],
          ...privateJWK,
          ext: true,
          key_ops: ["sign"],
        },
        { name: "RSA-PSS", hash },
        true,
        ["sign"],
      );

      const _publicKeyPSS = await crypto.subtle.importKey(
        "jwk",
        {
          alg: hashMapPSS[hash],
          ...publicJWK,
          ext: true,
          key_ops: ["verify"],
        },
        { name: "RSA-PSS", hash },
        true,
        ["verify"],
      );

      // const signaturePSS = await crypto.subtle.sign(
      //   { name: "RSA-PSS", saltLength: 32 },
      //   privateKeyPSS,
      //   new Uint8Array([1, 2, 3, 4]),
      // );

      // const verifyPSS = await crypto.subtle.verify(
      //   { name: "RSA-PSS", saltLength: 32 },
      //   publicKeyPSS,
      //   signaturePSS,
      //   new Uint8Array([1, 2, 3, 4]),
      // );
      // assert(verifyPSS);
    }

    // 2. Test import PKCS1
    for (const hash of ["SHA-1", "SHA-256", "SHA-384", "SHA-512"]) {
      const hashMapPKCS1: Record<string, string> = {
        "SHA-1": "RS1",
        "SHA-256": "RS256",
        "SHA-384": "RS384",
        "SHA-512": "RS512",
      };

      if (size === 1024 && hash === "SHA-512") {
        continue;
      }

      const privateKeyPKCS1 = await crypto.subtle.importKey(
        "jwk",
        {
          alg: hashMapPKCS1[hash],
          ...privateJWK,
          ext: true,
          key_ops: ["sign"],
        },
        { name: "RSASSA-PKCS1-v1_5", hash },
        true,
        ["sign"],
      );

      const publicKeyPKCS1 = await crypto.subtle.importKey(
        "jwk",
        {
          alg: hashMapPKCS1[hash],
          ...publicJWK,
          ext: true,
          key_ops: ["verify"],
        },
        { name: "RSASSA-PKCS1-v1_5", hash },
        true,
        ["verify"],
      );

      const signaturePKCS1 = await crypto.subtle.sign(
        { name: "RSASSA-PKCS1-v1_5", saltLength: 32 },
        privateKeyPKCS1,
        new Uint8Array([1, 2, 3, 4]),
      );

      const verifyPKCS1 = await crypto.subtle.verify(
        { name: "RSASSA-PKCS1-v1_5", saltLength: 32 },
        publicKeyPKCS1,
        signaturePKCS1,
        new Uint8Array([1, 2, 3, 4]),
      );
      assert(verifyPKCS1);
    }

    // 3. Test import OAEP
    for (const { hash, plainText: _plainText } of hashPlainTextVector) {
      const hashMapOAEP: Record<string, string> = {
        "SHA-1": "RSA-OAEP",
        "SHA-256": "RSA-OAEP-256",
        "SHA-384": "RSA-OAEP-384",
        "SHA-512": "RSA-OAEP-512",
      };

      if (size === 1024 && hash === "SHA-512") {
        continue;
      }

      const encryptAlgorithm = { name: "RSA-OAEP" };

      const _privateKeyOAEP = await crypto.subtle.importKey(
        "jwk",
        {
          alg: hashMapOAEP[hash],
          ...privateJWK,
          ext: true,
          key_ops: ["decrypt"],
        },
        { ...encryptAlgorithm, hash },
        true,
        ["decrypt"],
      );

      const _publicKeyOAEP = await crypto.subtle.importKey(
        "jwk",
        {
          alg: hashMapOAEP[hash],
          ...publicJWK,
          ext: true,
          key_ops: ["encrypt"],
        },
        { ...encryptAlgorithm, hash },
        true,
        ["encrypt"],
      );
      // const cipherText = await subtle.encrypt(
      //   encryptAlgorithm,
      //   publicKeyOAEP,
      //   plainText,
      // );

      // assert(cipherText);
      // assert(cipherText.byteLength > 0);
      // assert.strictEqual(cipherText.byteLength * 8, size);
      // assert(cipherText instanceof ArrayBuffer);

      // const decrypted = await subtle.decrypt(
      //   encryptAlgorithm,
      //   privateKeyOAEP,
      //   cipherText,
      // );
      // assert(decrypted);
      // assert(decrypted instanceof ArrayBuffer);
      // assert.deepEqual(new Uint8Array(decrypted), plainText);
    }
  }
}

const _jwtECKeys = {
  "256": {
    size: 256,
    algo: "ES256",
    publicJWK: {
      kty: "EC",
      crv: "P-256",
      x: "0hCwpvnZ8BKGgFi0P6T0cQGFQ7ugDJJQ35JXwqyuXdE",
      y: "zgN1UtSBRQzjm00QlXAbF1v6s0uObAmeGPHBmDWDYeg",
    },
    privateJWK: {
      kty: "EC",
      crv: "P-256",
      x: "0hCwpvnZ8BKGgFi0P6T0cQGFQ7ugDJJQ35JXwqyuXdE",
      y: "zgN1UtSBRQzjm00QlXAbF1v6s0uObAmeGPHBmDWDYeg",
      d: "E9M6LVq_nPnrsh_4YNSu_m5W53eQ9N7ptAiE69M1ROo",
    },
  },
  "384": {
    size: 384,
    algo: "ES384",
    publicJWK: {
      kty: "EC",
      crv: "P-384",
      x: "IZwU1mYXs27G2IVrOFtzp000T9iude8EZDXdpU47RL1fvevR0I3Wni19wdwhjLQ1",
      y: "vSgTjMd4M3qEL2vWGyQOdCSfJGZ8KlgQp2v8KOAzX4imUB3sAZdtqFr7AIactqzo",
    },
    privateJWK: {
      kty: "EC",
      crv: "P-384",
      x: "IZwU1mYXs27G2IVrOFtzp000T9iude8EZDXdpU47RL1fvevR0I3Wni19wdwhjLQ1",
      y: "vSgTjMd4M3qEL2vWGyQOdCSfJGZ8KlgQp2v8KOAzX4imUB3sAZdtqFr7AIactqzo",
      d: "RTe1mQeE08LSLpao-S-hqkku6HPldqQVguFEGDyYiNEOa560ztSyzEAS5KxeqEBz",
    },
  },
};

// type JWK = Record<string, string>;

// function equalJwk(expected: JWK, got: JWK): boolean {
//   const fields = Object.keys(expected);

//   for (let i = 0; i < fields.length; i++) {
//     const fieldName = fields[i];

//     if (!(fieldName in got)) {
//       return false;
//     }
//     if (expected[fieldName] !== got[fieldName]) {
//       return false;
//     }
//   }

//   return true;
// }

// async function testImportExportEcDsaJwk() {
//   const subtle = crypto.subtle;
//   assert(subtle);

//   for (const [_key, keyData] of Object.entries(jwtECKeys)) {
//     const { publicJWK, privateJWK, algo } = keyData;

//     // 1. Test import EcDsa
//     const privateKeyECDSA = await subtle.importKey(
//       "jwk",
//       {
//         alg: algo,
//         ...privateJWK,
//         ext: true,
//         key_ops: ["sign"],
//       },
//       { name: "ECDSA", namedCurve: privateJWK.crv },
//       true,
//       ["sign"],
//     );
//     const expPrivateKeyJWK = await subtle.exportKey("jwk", privateKeyECDSA);
//     assert(equalJwk(privateJWK, expPrivateKeyJWK as JWK));

//     const publicKeyECDSA = await subtle.importKey(
//       "jwk",
//       {
//         alg: algo,
//         ...publicJWK,
//         ext: true,
//         key_ops: ["verify"],
//       },
//       { name: "ECDSA", namedCurve: publicJWK.crv },
//       true,
//       ["verify"],
//     );

//     const expPublicKeyJWK = await subtle.exportKey("jwk", publicKeyECDSA);

//     assert(equalJwk(publicJWK, expPublicKeyJWK as JWK));

//     const signatureECDSA = await subtle.sign(
//       { name: "ECDSA", hash: `SHA-${keyData.size}` },
//       privateKeyECDSA,
//       new Uint8Array([1, 2, 3, 4]),
//     );

//     const verifyECDSA = await subtle.verify(
//       { name: "ECDSA", hash: `SHA-${keyData.size}` },
//       publicKeyECDSA,
//       signatureECDSA,
//       new Uint8Array([1, 2, 3, 4]),
//     );
//     assert(verifyECDSA);
//   }
// }

// async function testImportEcDhJwk() {
//   const subtle = crypto.subtle;
//   assert(subtle);

//   for (const [_key, jwkData] of Object.entries(jwtECKeys)) {
//     const { size, publicJWK, privateJWK } = jwkData;

//     // 1. Test import EcDsa
//     const privateKeyECDH = await subtle.importKey(
//       "jwk",
//       {
//         ...privateJWK,
//         ext: true,
//         key_ops: ["deriveBits"],
//       },
//       { name: "ECDH", namedCurve: privateJWK.crv },
//       true,
//       ["deriveBits"],
//     );

//     // const expPrivateKeyJWK = await subtle.exportKey(
//     //   "jwk",
//     //   privateKeyECDH,
//     // );
//     // assert(equalJwk(privateJWK, expPrivateKeyJWK as JWK));

//     const publicKeyECDH = await subtle.importKey(
//       "jwk",
//       {
//         ...publicJWK,
//         ext: true,
//         key_ops: [],
//       },
//       { name: "ECDH", namedCurve: publicJWK.crv },
//       true,
//       [],
//     );
//     // const expPublicKeyJWK = await subtle.exportKey(
//     //   "jwk",
//     //   publicKeyECDH,
//     // );
//     // assert(equalJwk(publicJWK, expPublicKeyJWK as JWK));

//     const derivedKey = await subtle.deriveBits(
//       {
//         name: "ECDH",
//         public: publicKeyECDH,
//       },
//       privateKeyECDH,
//       size,
//     );

//     assert(derivedKey instanceof ArrayBuffer);
//     assert.strictEqual(derivedKey.byteLength, size / 8);
//   }
// }

const _ecTestKeys = [
  {
    size: 256,
    namedCurve: "P-256",
    signatureLength: 64,
    // deno-fmt-ignore
    raw: new Uint8Array([
      4, 210, 16, 176, 166, 249, 217, 240, 18, 134, 128, 88, 180, 63, 164, 244,
      113, 1, 133, 67, 187, 160, 12, 146, 80, 223, 146, 87, 194, 172, 174, 93,
      209, 206, 3, 117, 82, 212, 129, 69, 12, 227, 155, 77, 16, 149, 112, 27,
      23, 91, 250, 179, 75, 142, 108, 9, 158, 24, 241, 193, 152, 53, 131, 97,
      232,
    ]),
    // deno-fmt-ignore
    spki: new Uint8Array([
      48, 89, 48, 19, 6, 7, 42, 134, 72, 206, 61, 2, 1, 6, 8, 42, 134, 72, 206,
      61, 3, 1, 7, 3, 66, 0, 4, 210, 16, 176, 166, 249, 217, 240, 18, 134, 128,
      88, 180, 63, 164, 244, 113, 1, 133, 67, 187, 160, 12, 146, 80, 223, 146,
      87, 194, 172, 174, 93, 209, 206, 3, 117, 82, 212, 129, 69, 12, 227, 155,
      77, 16, 149, 112, 27, 23, 91, 250, 179, 75, 142, 108, 9, 158, 24, 241,
      193, 152, 53, 131, 97, 232,
    ]),
    // deno-fmt-ignore
    pkcs8: new Uint8Array([
      48, 129, 135, 2, 1, 0, 48, 19, 6, 7, 42, 134, 72, 206, 61, 2, 1, 6, 8, 42,
      134, 72, 206, 61, 3, 1, 7, 4, 109, 48, 107, 2, 1, 1, 4, 32, 19, 211, 58,
      45, 90, 191, 156, 249, 235, 178, 31, 248, 96, 212, 174, 254, 110, 86, 231,
      119, 144, 244, 222, 233, 180, 8, 132, 235, 211, 53, 68, 234, 161, 68, 3,
      66, 0, 4, 210, 16, 176, 166, 249, 217, 240, 18, 134, 128, 88, 180, 63,
      164, 244, 113, 1, 133, 67, 187, 160, 12, 146, 80, 223, 146, 87, 194, 172,
      174, 93, 209, 206, 3, 117, 82, 212, 129, 69, 12, 227, 155, 77, 16, 149,
      112, 27, 23, 91, 250, 179, 75, 142, 108, 9, 158, 24, 241, 193, 152, 53,
      131, 97, 232,
    ]),
  },
  {
    size: 384,
    namedCurve: "P-384",
    signatureLength: 96,
    // deno-fmt-ignore
    raw: new Uint8Array([
      4, 118, 64, 176, 165, 100, 177, 112, 49, 254, 58, 53, 158, 63, 73, 200,
      148, 248, 242, 216, 186, 80, 92, 160, 53, 64, 232, 157, 19, 1, 12, 226,
      115, 51, 42, 143, 98, 206, 55, 220, 108, 78, 24, 71, 157, 21, 120, 126,
      104, 157, 86, 48, 226, 110, 96, 52, 48, 77, 170, 9, 231, 159, 26, 165,
      200, 26, 164, 99, 46, 227, 169, 105, 172, 225, 60, 102, 141, 145, 139,
      165, 47, 72, 53, 17, 17, 246, 161, 220, 26, 21, 23, 219, 1, 107, 185, 163,
      215,
    ]),
    // deno-fmt-ignore
    spki: new Uint8Array([
      48, 118, 48, 16, 6, 7, 42, 134, 72, 206, 61, 2, 1, 6, 5, 43, 129, 4, 0,
      34, 3, 98, 0, 4, 118, 64, 176, 165, 100, 177, 112, 49, 254, 58, 53, 158,
      63, 73, 200, 148, 248, 242, 216, 186, 80, 92, 160, 53, 64, 232, 157, 19,
      1, 12, 226, 115, 51, 42, 143, 98, 206, 55, 220, 108, 78, 24, 71, 157, 21,
      120, 126, 104, 157, 86, 48, 226, 110, 96, 52, 48, 77, 170, 9, 231, 159,
      26, 165, 200, 26, 164, 99, 46, 227, 169, 105, 172, 225, 60, 102, 141, 145,
      139, 165, 47, 72, 53, 17, 17, 246, 161, 220, 26, 21, 23, 219, 1, 107, 185,
      163, 215,
    ]),
    // deno-fmt-ignore
    pkcs8: new Uint8Array([
      48, 129, 182, 2, 1, 0, 48, 16, 6, 7, 42, 134, 72, 206, 61, 2, 1, 6, 5, 43,
      129, 4, 0, 34, 4, 129, 158, 48, 129, 155, 2, 1, 1, 4, 48, 202, 7, 195,
      169, 124, 170, 81, 169, 253, 127, 56, 28, 98, 90, 255, 165, 72, 142, 133,
      138, 237, 200, 176, 92, 179, 192, 83, 28, 47, 118, 157, 152, 47, 65, 133,
      140, 50, 83, 182, 191, 224, 96, 216, 179, 59, 150, 15, 233, 161, 100, 3,
      98, 0, 4, 118, 64, 176, 165, 100, 177, 112, 49, 254, 58, 53, 158, 63, 73,
      200, 148, 248, 242, 216, 186, 80, 92, 160, 53, 64, 232, 157, 19, 1, 12,
      226, 115, 51, 42, 143, 98, 206, 55, 220, 108, 78, 24, 71, 157, 21, 120,
      126, 104, 157, 86, 48, 226, 110, 96, 52, 48, 77, 170, 9, 231, 159, 26,
      165, 200, 26, 164, 99, 46, 227, 169, 105, 172, 225, 60, 102, 141, 145,
      139, 165, 47, 72, 53, 17, 17, 246, 161, 220, 26, 21, 23, 219, 1, 107, 185,
      163, 215,
    ]),
  },
];

// async function testImportEcSpkiPkcs8() {
//   const subtle = crypto.subtle;
//   assert(subtle);

//   for (const { namedCurve, raw, spki, pkcs8, signatureLength } of ecTestKeys) {
//     const rawPublicKeyECDSA = await subtle.importKey(
//       "raw",
//       raw,
//       { name: "ECDSA", namedCurve },
//       true,
//       ["verify"],
//     );

//     // const expPublicKeyRaw = await subtle.exportKey(
//     //   "raw",
//     //   rawPublicKeyECDSA,
//     // );

//     // assertEquals(new Uint8Array(expPublicKeyRaw), raw);

//     const privateKeyECDSA = await subtle.importKey(
//       "pkcs8",
//       pkcs8,
//       { name: "ECDSA", namedCurve },
//       true,
//       ["sign"],
//     );

//     // const expPrivateKeyPKCS8 = await subtle.exportKey(
//     //   "pkcs8",
//     //   privateKeyECDSA,
//     // );

//     // assertEquals(new Uint8Array(expPrivateKeyPKCS8), pkcs8);

//     // const expPrivateKeyJWK = await subtle.exportKey(
//     //   "jwk",
//     //   privateKeyECDSA,
//     // );

//     // assertEquals(expPrivateKeyJWK.crv, namedCurve);

//     const publicKeyECDSA = await subtle.importKey(
//       "spki",
//       spki,
//       { name: "ECDSA", namedCurve },
//       true,
//       ["verify"],
//     );

//     // const expPublicKeySPKI = await subtle.exportKey(
//     //   "spki",
//     //   publicKeyECDSA,
//     // );

//     // assertEquals(new Uint8Array(expPublicKeySPKI), spki);

//     // const expPublicKeyJWK = await subtle.exportKey(
//     //   "jwk",
//     //   publicKeyECDSA,
//     // );

//     // assert.strictEqual(expPublicKeyJWK.crv, namedCurve);

//     // for (
//     //   const hash of ["SHA-1", "SHA-256", "SHA-384", "SHA-512"]
//     // ) {
//     //   if (
//     //     (hash == "SHA-256" && namedCurve == "P-256") ||
//     //     (hash == "SHA-384" && namedCurve == "P-384")
//     //   ) {
//     //     const signatureECDSA = await subtle.sign(
//     //       { name: "ECDSA", hash },
//     //       privateKeyECDSA,
//     //       new Uint8Array([1, 2, 3, 4]),
//     //     );

//     //     const verifyECDSA = await subtle.verify(
//     //       { name: "ECDSA", hash },
//     //       publicKeyECDSA,
//     //       signatureECDSA,
//     //       new Uint8Array([1, 2, 3, 4]),
//     //     );
//     //     assert(verifyECDSA);
//     //   } else {
//     //     await assertRejects(
//     //       async () => {
//     //         await subtle.sign(
//     //           { name: "ECDSA", hash },
//     //           privateKeyECDSA,
//     //           new Uint8Array([1, 2, 3, 4]),
//     //         );
//     //       },
//     //       DOMException,
//     //       "Not implemented",
//     //     );
//     //     await assertRejects(
//     //       async () => {
//     //         await subtle.verify(
//     //           { name: "ECDSA", hash },
//     //           publicKeyECDSA,
//     //           new Uint8Array(signatureLength),
//     //           new Uint8Array([1, 2, 3, 4]),
//     //         );
//     //       },
//     //       DOMException,
//     //       "Not implemented",
//     //     );
//     //   }
//     // }
//   }
// }

async function testAesGcmEncrypt() {
  const _key = await crypto.subtle.importKey(
    "raw",
    new Uint8Array(16),
    { name: "AES-GCM", length: 256 },
    true,
    ["encrypt", "decrypt"],
  );

  // const nonces = [
  //   {
  //     iv: new Uint8Array([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
  //     ciphertext: new Uint8Array([
  //       50, 223, 112, 178, 166, 156, 255, 110, 125, 138, 95, 141, 82, 47, 14,
  //       164, 134, 247, 22,
  //     ]),
  //   },
  //   {
  //     iv: new Uint8Array([
  //       0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
  //     ]),
  //     ciphertext: new Uint8Array([
  //       210, 101, 81, 216, 151, 9, 192, 197, 62, 254, 28, 132, 89, 106, 40, 29,
  //       175, 232, 201,
  //     ]),
  //   },
  // ];
  // for (const { iv, ciphertext: fixture } of nonces) {
  //   const data = new Uint8Array([1, 2, 3]);

  //   const cipherText = await crypto.subtle.encrypt(
  //     { name: "AES-GCM", iv },
  //     key,
  //     data,
  //   );

  //   assert(cipherText instanceof ArrayBuffer);
  //   assertEquals(cipherText.byteLength, 19);
  //   assertEquals(
  //     new Uint8Array(cipherText),
  //     fixture,
  //   );

  //   const plainText = await crypto.subtle.decrypt(
  //     { name: "AES-GCM", iv },
  //     key,
  //     cipherText,
  //   );
  //   assert(plainText instanceof ArrayBuffer);
  //   assertEquals(plainText.byteLength, 3);
  //   assertEquals(new Uint8Array(plainText), data);
  // }
}

// WARNING: doesn't actually test round trip because exportKey isn't implemented.
// It currently just tests that importKey works.
async function roundTripSecretJwk(
  jwk: JsonWebKey,
  algId: AlgorithmIdentifier | HmacImportParams,
  ops: KeyUsage[],
  _validateKeys: (
    key: CryptoKey,
    originalJwk: JsonWebKey,
    exportedJwk: JsonWebKey,
  ) => void,
) {
  const key = await crypto.subtle.importKey("jwk", jwk, algId, true, ops);

  assert(key instanceof CryptoKey);
  assert.strictEqual(key.type, "secret");

  // const exportedKey = await crypto.subtle.exportKey("jwk", key);

  // validateKeys(key, jwk, exportedKey);
}

async function testSecretJwkBase64Url() {
  // Test 16bits with "overflow" in 3rd pos of 'quartet', no padding
  const keyData = `{
      "kty": "oct",
      "k": "xxx",
      "alg": "HS512",
      "key_ops": ["sign", "verify"],
      "ext": true
    }`;

  await roundTripSecretJwk(
    JSON.parse(keyData),
    { name: "HMAC", hash: "SHA-512" },
    ["sign", "verify"],
    (key, _orig, exp) => {
      assert.strictEqual((key.algorithm as HmacKeyAlgorithm).length, 16);

      assert.strictEqual(exp.k, "xxw");
    },
  );

  // HMAC 128bits with base64url characters (-_)
  await roundTripSecretJwk(
    {
      kty: "oct",
      k: "HnZXRyDKn-_G5Fx4JWR1YA",
      alg: "HS256",
      key_ops: ["sign", "verify"],
      ext: true,
    },
    { name: "HMAC", hash: "SHA-256" },
    ["sign", "verify"],
    (key, orig, exp) => {
      assert.strictEqual((key.algorithm as HmacKeyAlgorithm).length, 128);

      assert.strictEqual(orig.k, exp.k);
    },
  );

  // HMAC 104bits/(12+1) bytes with base64url characters (-_), padding and overflow in 2rd pos of "quartet"
  await roundTripSecretJwk(
    {
      kty: "oct",
      k: "a-_AlFa-2-OmEGa_-z==",
      alg: "HS384",
      key_ops: ["sign", "verify"],
      ext: true,
    },
    { name: "HMAC", hash: "SHA-384" },
    ["sign", "verify"],
    (key, _orig, exp) => {
      assert.strictEqual((key.algorithm as HmacKeyAlgorithm).length, 104);

      assert.strictEqual("a-_AlFa-2-OmEGa_-w", exp.k);
    },
  );

  // AES-CBC 128bits with base64url characters (-_) no padding
  await roundTripSecretJwk(
    {
      kty: "oct",
      k: "_u3K_gEjRWf-7cr-ASNFZw",
      alg: "A128CBC",
      key_ops: ["encrypt", "decrypt"],
      ext: true,
    },
    { name: "AES-CBC" },
    ["encrypt", "decrypt"],
    (_key, orig, exp) => {
      assert.strictEqual(orig.k, exp.k);
    },
  );

  // AES-CBC 128bits of '1' with padding chars
  await roundTripSecretJwk(
    {
      kty: "oct",
      k: "_____________________w==",
      alg: "A128CBC",
      key_ops: ["encrypt", "decrypt"],
      ext: true,
    },
    { name: "AES-CBC" },
    ["encrypt", "decrypt"],
    (_key, _orig, exp) => {
      assert.strictEqual(exp.k, "_____________________w");
    },
  );
}

// async function testAESWrapKey() {
//   const key = await crypto.subtle.generateKey(
//     {
//       name: "AES-KW",
//       length: 128,
//     },
//     true,
//     ["wrapKey", "unwrapKey"],
//   );

//   const hmacKey = await crypto.subtle.generateKey(
//     {
//       name: "HMAC",
//       hash: "SHA-256",
//       length: 128,
//     },
//     true,
//     ["sign"],
//   );

//   //round-trip
//   // wrap-unwrap-export compare
//   const wrappedKey = await crypto.subtle.wrapKey(
//     "raw",
//     hmacKey,
//     key,
//     {
//       name: "AES-KW",
//     },
//   );

//   assert(wrappedKey instanceof ArrayBuffer);
//   assertEquals(wrappedKey.byteLength, 16 + 8); // 8 = 'auth tag'

//   const unwrappedKey = await crypto.subtle.unwrapKey(
//     "raw",
//     wrappedKey,
//     key,
//     {
//       name: "AES-KW",
//     },
//     {
//       name: "HMAC",
//       hash: "SHA-256",
//     },
//     true,
//     ["sign"],
//   );

//   assert(unwrappedKey instanceof CryptoKey);
//   assertEquals((unwrappedKey.algorithm as HmacKeyAlgorithm).length, 128);

//   const hmacKeyBytes = await crypto.subtle.exportKey("raw", hmacKey);
//   const unwrappedKeyBytes = await crypto.subtle.exportKey("raw", unwrappedKey);

//   assertEquals(new Uint8Array(hmacKeyBytes), new Uint8Array(unwrappedKeyBytes));
// }

// https://github.com/denoland/deno/issues/13534
async function testAesGcmTagLength() {
  const _key = await crypto.subtle.importKey(
    "raw",
    new Uint8Array(32),
    "AES-GCM",
    false,
    ["encrypt", "decrypt"],
  );

  const _iv = crypto.getRandomValues(new Uint8Array(12));

  // encrypt won't fail, it will simply truncate the tag
  // as expected.
  // const encrypted = await crypto.subtle.encrypt(
  //   { name: "AES-GCM", iv, tagLength: 96 },
  //   key,
  //   new Uint8Array(32),
  // );

  // await assertRejects(async () => {
  //   await crypto.subtle.decrypt(
  //     { name: "AES-GCM", iv, tagLength: 96 },
  //     key,
  //     encrypted,
  //   );
  // });
}

// async function ecPrivateKeyMaterialExportSpki() {
//   // `generateKey` generates a key pair internally stored as "private" key.
//   const keys = await crypto.subtle.generateKey(
//     { name: "ECDSA", namedCurve: "P-256" },
//     true,
//     ["sign", "verify"],
//   );

//   assert(keys.privateKey instanceof CryptoKey);
//   assert(keys.publicKey instanceof CryptoKey);

//   // `exportKey` should be able to perform necessary conversion to export spki.
//   const spki = await crypto.subtle.exportKey("spki", keys.publicKey);
//   assert(spki instanceof ArrayBuffer);
// }

// https://github.com/denoland/deno/issues/13911
async function importJwkWithUse() {
  const jwk = {
    kty: "EC",
    use: "sig",
    crv: "P-256",
    x: "FWZ9rSkLt6Dx9E3pxLybhdM6xgR5obGsj5_pqmnz5J4",
    y: "_n8G69C-A2Xl4xUW2lF0i8ZGZnk_KPYrhv4GbTGu5G4",
  };

  const algorithm = { name: "ECDSA", namedCurve: "P-256" };

  const key = await crypto.subtle.importKey("jwk", jwk, algorithm, true, [
    "verify",
  ]);

  assert(key instanceof CryptoKey);
}

// https://github.com/denoland/deno/issues/14215
// async function exportKeyNotExtractable() {
//   const key = await crypto.subtle.generateKey(
//     {
//       name: "HMAC",
//       hash: "SHA-512",
//     },
//     false,
//     ["sign", "verify"],
//   );

//   assert(key);
//   assertEquals(key.extractable, false);

//   await assertRejects(async () => {
//     // Should fail
//     await crypto.subtle.exportKey("raw", key);
//   }, DOMException);
// }

// https://github.com/denoland/deno/issues/15126
// async function testImportLeadingZeroesKey() {
//   const alg = { name: "ECDSA", namedCurve: "P-256" };

//   const jwk = {
//     kty: "EC",
//     crv: "P-256",
//     alg: "ES256",
//     x: "EvidcdFB1xC6tgfakqZsU9aIURxAJkcX62zHe1Nt6xU",
//     y: "AHsk6BioGM7MZWeXOE_49AGmtuaXFT3Ill3DYtz9uYg",
//     d: "WDeYo4o1heCF9l_2VIaClRyIeO16zsMlN8UG6Le9dU8",
//     key_ops: ["sign"],
//     ext: true,
//   };

//   const key = await crypto.subtle.importKey("jwk", jwk, alg, true, ["sign"]);

//   assert(key instanceof CryptoKey);
//   assert.strictEqual(key.type, "private");
// }

// https://github.com/denoland/deno/issues/15523
// async function testECspkiRoundTrip() {
//   const alg = { name: "ECDH", namedCurve: "P-256" };
//   const { publicKey } = await crypto.subtle.generateKey(alg, true, [
//     "deriveBits",
//   ]);
//   const spki = await crypto.subtle.exportKey("spki", publicKey);
//   await crypto.subtle.importKey("spki", spki, alg, true, []);
// }

async function testHmacJwkImport() {
  await crypto.subtle.importKey(
    "jwk",
    {
      kty: "oct",
      use: "sig",
      alg: "HS256",
      k: "hJtXIZ2uSN5kbQfbtTNWbpdmhkV8FJG-Onbc6mxCcYg",
    },
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign", "verify"],
  );
}

// -----------------------------------------------------------------------------
// End tests from Deno
// -----------------------------------------------------------------------------

async function testHMACSign() {
  // This is more or less what the Stripe SDK does
  // https://github.com/stripe/stripe-node/blob/68b32428a98f966eee19b54551cb4c3706ca3414/src/crypto/SubtleCryptoProvider.ts#L48C1-L52C7
  const subtle = crypto.subtle;

  const key = new Uint8Array([
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
  ]);

  const cryptoKey = await subtle.importKey(
    "raw",
    key.buffer,
    { name: "HMAC", hash: { name: "SHA-256" } },
    false,
    ["sign"],
  );

  const actual = await subtle.sign(
    { name: "HMAC" },
    cryptoKey,
    new Uint8Array(8),
  );
  const actualBase64String = btoa(
    String.fromCharCode(...new Uint8Array(actual)),
  );
  // This value is from running the above code in a browser
  const expected = "SdJ6jecfYHT1LzY/Vh03WauHRbzWZeVjDFL4ietJNCw=";
  assert.strictEqual(actualBase64String, expected);
}

async function testHMACVerify() {
  // Verify signature from testHMACSign.
  const base64Signature = "SdJ6jecfYHT1LzY/Vh03WauHRbzWZeVjDFL4ietJNCw=";
  const subtle = crypto.subtle;

  const key = new Uint8Array([
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
  ]);

  // Import the key for verification
  const cryptoKey = await subtle.importKey(
    "raw",
    key.buffer,
    { name: "HMAC", hash: { name: "SHA-256" } },
    false,
    ["verify"],
  );

  // Convert the base64 signature back to Uint8Array
  const signature = new Uint8Array(
    atob(base64Signature)
      .split("")
      .map((c) => c.charCodeAt(0)),
  );

  const isVerified = await subtle.verify(
    { name: "HMAC" },
    cryptoKey,
    signature,
    new Uint8Array(8),
  );

  assert.isTrue(isVerified, "signature should be verified");
}

async function testHMACSignAlternativeSyntax() {
  const subtle = crypto.subtle;

  const key = new Uint8Array([
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
  ]);

  const cryptoKey = await subtle.importKey(
    "raw",
    key.buffer,
    // This should also work
    { name: "hmac", hash: "sha-256" },
    false,
    ["sign"],
  );

  // Specifying an extra property should also be fine
  const importParams = { name: "hmac", hash: "sha-256", foo: "bar" };

  await subtle.importKey("raw", key.buffer, importParams, false, ["sign"]);

  const actual = await subtle.sign("hmac", cryptoKey, new Uint8Array(8));
  const actualBase64String = btoa(
    String.fromCharCode(...new Uint8Array(actual)),
  );
  // This value is from running the above code in a browser
  const expected = "SdJ6jecfYHT1LzY/Vh03WauHRbzWZeVjDFL4ietJNCw=";
  assert.strictEqual(actualBase64String, expected);
}

async function testInvalidAlgorithm() {
  const subtle = crypto.subtle;

  const key = new Uint8Array([
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
  ]);

  await expect(
    subtle.importKey(
      "raw",
      key.buffer,
      { name: "foo", hash: "sha-256" },
      false,
      ["sign"],
    ),
  ).to.be.rejectedWith(/Unrecognized algorithm/);
}

async function testDeriveBitsPBKDF2() {
  const rawKey = crypto.getRandomValues(new Uint8Array(16));
  const key = await crypto.subtle.importKey("raw", rawKey, "PBKDF2", false, [
    "deriveBits",
  ]);

  const salt = crypto.getRandomValues(new Uint8Array(16));
  await crypto.subtle.deriveBits(
    {
      name: "PBKDF2",
      hash: "SHA-256",
      salt,
      iterations: 1000,
    },
    key,
    16 * 8,
  );
}

async function testDeriveKeyPBKDF2() {
  // Test deriveKey
  const rawKey = crypto.getRandomValues(new Uint8Array(16));
  const key = await crypto.subtle.importKey("raw", rawKey, "PBKDF2", false, [
    "deriveKey",
    "deriveBits",
  ]);

  const salt = crypto.getRandomValues(new Uint8Array(16));
  const derivedKey = await crypto.subtle.deriveKey(
    {
      name: "PBKDF2",
      salt,
      iterations: 1000,
      hash: "SHA-256",
    },
    key,
    { name: "HMAC", hash: "SHA-256" },
    true,
    ["sign"],
  );

  assert(derivedKey instanceof CryptoKey);
  assert.strictEqual(derivedKey.type, "secret");
  assert.strictEqual(derivedKey.extractable, true);
  assert.deepEqual(derivedKey.usages, ["sign"]);

  const algorithm = derivedKey.algorithm as HmacKeyAlgorithm;
  assert.strictEqual(algorithm.name, "HMAC");
  assert.strictEqual(algorithm.hash.name, "SHA-256");
  assert.strictEqual(algorithm.length, 512);
}

async function testDigest() {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode("hello"),
  );
  const digestBase64 = btoa(String.fromCharCode(...new Uint8Array(digest)));
  assert.strictEqual(
    digestBase64,
    "LPJNul+wow4m6DsqxbninhsWHlwfp0JecwQzYpOLmCQ=",
  );
}

export const methodNotImplemented = query({
  handler: async () => {
    const subtle = crypto.subtle;

    const key = new Uint8Array([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
    ]);
    const cryptoKey = await subtle.importKey(
      "raw",
      key.buffer,
      { name: "HMAC", hash: { name: "SHA-256" } },
      false,
      ["sign"],
    );

    await subtle.exportKey("jwk", cryptoKey);
  },
});

export const test = query({
  handler: async () => {
    return await wrapInTests({
      getRandomValuesInt8Array,
      getRandomValuesUint8Array,
      getRandomValuesUint8ClampedArray,
      getRandomValuesInt16Array,
      getRandomValuesUint16Array,
      getRandomValuesInt32Array,
      getRandomValuesBigInt64Array,
      getRandomValuesUint32Array,
      getRandomValuesBigUint64Array,
      getRandomValuesReturnValue,
      getRandomValuesFloatArray,
      randomUUID,

      testImportArrayBufferKey,
      // testSignVerify,
      // testEncryptDecrypt,
      // testGenerateRSAKey,
      // testGenerateHMACKey,
      // testECDSASignVerify,
      // testECDSASignVerifyFail,
      // testSignRSASSAKey,
      subtleCryptoHmacImportExport,
      importRsaPkcs8,
      importNonInteroperableRsaPkcs8,
      testImportRsaJwk,
      // importing EC keys requires SecureRandom
      // testImportExportEcDsaJwk,
      // estImportEcDhJwk,
      // testImportEcSpkiPkcs8,
      testAesGcmEncrypt,
      testSecretJwkBase64Url,
      // testAESWrapKey,
      testAesGcmTagLength,
      // ecPrivateKeyMaterialExportSpki,
      importJwkWithUse,
      // exportKeyNotExtractable
      // testImportLeadingZeroesKey,
      // testECspkiRoundTrip,
      testHmacJwkImport,

      // End of Deno tests.

      testHMACSign,
      testHMACVerify,
      testHMACSignAlternativeSyntax,
      testInvalidAlgorithm,
      testDeriveBitsPBKDF2,
      testDeriveKeyPBKDF2,
      testDigest,
    });
  },
});
