import { assert } from "chai";

// polyfill for node
if (!Uint8Array.fromBase64)
  Uint8Array.fromBase64 = (s: string) =>
    new Uint8Array(
      atob(s)
        .split("")
        .map((c) => c.charCodeAt(0)),
    );
if (!Uint8Array.prototype.toBase64)
  Uint8Array.prototype.toBase64 = function (this: Uint8Array) {
    return btoa(String.fromCodePoint(...this));
  } as any;

const RSA_ALGORITHMS = ["RSASSA-PKCS1-v1_5", "RSA-PSS", "RSA-OAEP"].flatMap(
  (name) =>
    [1024, 2048, 3072].flatMap((modulusLength) =>
      [new Uint8Array([3]), new Uint8Array([1, 0, 1])].map(
        (publicExponent) => ({
          name,
          modulusLength,
          publicExponent,
          hash: "SHA-256",
        }),
      ),
    ),
);
const EC_ALGORITHMS = ["P-256", "P-384", "P-521"].map((namedCurve) => ({
  name: "ECDSA",
  namedCurve,
}));
const AES_ALGORITHMS = ["AES-CTR", "AES-CBC", "AES-GCM"].flatMap((name) =>
  [128, 192, 256].map((length) => ({
    name,
    length,
  })),
);

const KEY_ALGORITHMS = [
  ...RSA_ALGORITHMS,
  ...EC_ALGORITHMS,
  { name: "HMAC", hash: "SHA-256" },
  ...AES_ALGORITHMS,
  { name: "Ed25519" },
  { name: "X25519" },
] as const;

const usages = (name: string): KeyUsage[] => {
  switch (name) {
    case "RSASSA-PKCS1-v1_5":
    case "RSA-PSS":
    case "ECDSA":
    case "HMAC":
    case "Ed25519":
      return ["sign", "verify"];
    case "RSA-OAEP":
    case "AES-CTR":
    case "AES-CBC":
    case "AES-GCM":
      return ["decrypt", "encrypt"];
    case "X25519":
      return ["deriveBits"];
    default:
      throw new Error(`unknown ${name}`);
  }
};
const formats = ["jwk", "spki", "pkcs8", "raw"];
const exportKey = async (key: CryptoKey): Promise<any> => {
  const r: Record<string, any> = {};
  for (const format of formats) {
    try {
      const exported = await crypto.subtle.exportKey(format as any, key);
      if (
        !("Convex" in globalThis) &&
        key.algorithm.name === "Ed25519" &&
        format === "jwk"
      ) {
        // Node.js doesn't write the `alg`
        (exported as JsonWebKey).alg = "Ed25519";
      }
      const usages = key.usages;
      if (exported instanceof ArrayBuffer) {
        r[format] = { data: new Uint8Array(exported).toBase64(), usages };
      } else {
        r[format] = { data: exported, usages };
      }
    } catch (e) {
      if (e instanceof DOMException) {
        let exception = e.name;
        if (!("Convex" in globalThis)) {
          if (
            (key.algorithm.name.startsWith("RSA") && format === "raw") ||
            (["HMAC", "AES-CTR", "AES-CBC", "AES-GCM"].includes(
              key.algorithm.name,
            ) &&
              ["spki", "pkcs8"].includes(format))
          ) {
            // Node.js before 24 throws InvalidAccessError here, contrary to the WebCrypto spec
            if (exception === "InvalidAccessError")
              exception = "NotSupportedError";
          }
          if (
            (format === "spki" && key.type === "private") ||
            (format === "pkcs8" && key.type === "public") ||
            (["ECDSA", "ECDH", "Ed25519", "X25519"].includes(
              key.algorithm.name,
            ) &&
              format === "raw" &&
              key.type === "private")
          ) {
            // Node.js 24+ throws NotSupportedError here, contrary to the WebCrypto spec
            if (exception === "NotSupportedError")
              exception = "InvalidAccessError";
          }
        }
        r[format] = { exception };
      } else {
        throw e;
      }
    }
  }
  return r;
};
const sampleData = Uint8Array.fromBase64("Y29udmV4LnN1Y2tz".repeat(5));

const encryptAlg = (alg: any) => {
  const a = { ...alg };
  if (alg.name === "AES-CTR") {
    const counter = new ArrayBuffer(16);
    new Uint8Array(counter).fill(0xff);
    a.counter = counter;
    a.length = 2;
  }
  if (alg.name === "AES-CBC") {
    a.iv = new ArrayBuffer(16);
  }
  if (alg.name === "AES-GCM") {
    // we only support 96 bit AES-GCM nonces
    a.iv = new ArrayBuffer(12);
  }
  return a;
};
const createVectors = async (
  alg: any,
  key: CryptoKey,
  vectors: Record<string, any>,
): Promise<void> => {
  if (key.usages.includes("sign")) {
    vectors.signature = new Uint8Array(
      await crypto.subtle.sign(
        { ...alg, saltLength: 32, hash: "SHA-256" },
        key,
        sampleData,
      ),
    ).toBase64();
  }
  if (key.usages.includes("encrypt")) {
    vectors.encrypted = new Uint8Array(
      await crypto.subtle.encrypt(encryptAlg(alg), key, sampleData),
    ).toBase64();
  }
};
const checkVectors = async (
  key: CryptoKey,
  vectors: Record<string, any>,
): Promise<void> => {
  const alg: any = key.algorithm;
  if (key.usages.includes("verify")) {
    assert.isTrue(
      await crypto.subtle.verify(
        { ...alg, saltLength: 32, hash: "SHA-256" },
        key,
        Uint8Array.fromBase64(vectors.signature),
        sampleData,
      ),
    );
  }
  if (key.usages.includes("decrypt")) {
    assert.deepEqual(
      new Uint8Array(
        await crypto.subtle.decrypt(
          encryptAlg(alg),
          key,
          Uint8Array.fromBase64(vectors.encrypted),
        ),
      ),
      sampleData,
    );
  }
};
const importAndVerify = async (
  alg: any,
  keys: Record<string, any>,
  vectors: Record<string, any>,
): Promise<void> => {
  for (const [format, key] of Object.entries(keys)) {
    if (typeof key === "object" && "exception" in key) continue;
    const cryptoKey = await crypto.subtle.importKey(
      format as any,
      format === "jwk" ? key.data : Uint8Array.fromBase64(key.data),
      alg,
      true,
      key.usages,
    );
    assert.deepEqual(await exportKey(cryptoKey), keys);
    checkVectors(cryptoKey, vectors);
  }
};
export const generateData = async () => {
  const algs: any[] = [];
  const promises: Promise<void>[] = [];
  for (const algorithm of KEY_ALGORITHMS) {
    const data: any = { vectors: {} };
    algs.push({ algorithm, data });
    // generate keys in parallel for a slight speedup
    promises.push(
      (async () => {
        const k = await crypto.subtle.generateKey(
          algorithm,
          true,
          usages(algorithm.name),
        );
        if (k instanceof CryptoKey) {
          await createVectors(algorithm, k, data.vectors);
          data.key = await exportKey(k);
        } else {
          await createVectors(algorithm, k.privateKey, data.vectors);
          await createVectors(algorithm, k.publicKey, data.vectors);
          data.privateKey = await exportKey(k.privateKey);
          data.publicKey = await exportKey(k.publicKey);
        }
      })(),
    );
  }
  await Promise.all(promises);
  return JSON.stringify(algs);
};
export const consumeData = async (algsJson: string) => {
  for (const { algorithm, data } of JSON.parse(algsJson)) {
    if ("privateKey" in data) {
      await importAndVerify(algorithm, data.privateKey, data.vectors);
      await importAndVerify(algorithm, data.publicKey, data.vectors);
    } else {
      await importAndVerify(algorithm, data.key, data.vectors);
    }
  }
};
