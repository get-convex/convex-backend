const alphabet = "0123456789abcdefghjkmnpqrstvwxyz";
const inverseAlphabet = new Map();
for (let i = 0; i < 32; i++) {
  inverseAlphabet.set(alphabet[i], i);
}
const version = 0;

class InvalidBase32Error extends Error {
  constructor(message: string) {
    super(`Invalid base32 string: ${message}`);
  }
}

function decodeBase32(data: string): Uint8Array {
  const outLength = Math.floor((data.length * 5) / 8);
  const buf = new Uint8Array(Math.floor((outLength + 4) / 5) * 5);
  const numChunks = Math.floor((data.length + 7) / 8);
  for (let i = 0; i < numChunks; i++) {
    const indexes = Array(8).fill(0);
    for (let j = 0; j < Math.min(8, data.length - i * 8); j++) {
      const char = data.charAt(i * 8 + j);
      const index = inverseAlphabet.get(char);
      if (typeof index === "undefined") {
        throw new InvalidBase32Error(
          `Invalid character ${char} at position ${i * 8 + j} in ${data}`,
        );
      }
      indexes[j] = index;
    }
    buf[5 * i] = (indexes[0] << 3) | (indexes[1] >> 2);
    buf[5 * i + 1] = (indexes[1] << 6) | (indexes[2] << 1) | (indexes[3] >> 4);
    buf[5 * i + 2] = (indexes[3] << 4) | (indexes[4] >> 1);
    buf[5 * i + 3] = (indexes[4] << 7) | (indexes[5] << 2) | (indexes[6] >> 3);
    buf[5 * i + 4] = (indexes[6] << 5) | indexes[7];
  }
  return buf.slice(0, outLength);
}

class InvalidIdError extends Error {
  constructor(message: string) {
    super(`Invalid ID: ${message}`);
  }
}

function vintDecode(buf: Uint8Array): { n: number; bytesRead: number } {
  let bytesRead = 0;
  let n = 0;
  for (let i = 0; ; i++) {
    if (i >= 5) {
      throw new InvalidIdError("Integer is too large");
    }
    if (bytesRead >= buf.length) {
      throw new InvalidIdError("Input truncated");
    }
    const byte = buf[bytesRead];
    bytesRead += 1;
    n |= (byte & 0x7f) << (i * 7);
    if (byte < 0x80) {
      break;
    }
  }
  // NB: JS bitwise operations and shifts operate on *signed* 32-bit integers,
  // not unsigned ones. We can convert to an unsigned 32-bit by using the
  // special "unsigned right shift" operator with shift zero.
  n = n >>> 0;
  return { bytesRead, n };
}

function fletcher16(buf: Uint8Array): number {
  let c0 = 0;
  let c1 = 0;
  for (const byte of buf) {
    c0 = (c0 + byte) % 256;
    c1 = (c1 + c0) % 256;
  }
  return (c1 << 8) | c0;
}

type DecodedId = { tableNumber: number; internalId: Uint8Array };

const MIN_BASE32_LEN = 31;
const MAX_BASE32_LEN = 37;

export function decodeId(s: string): DecodedId {
  if (s.length < MIN_BASE32_LEN || s.length > MAX_BASE32_LEN) {
    throw new InvalidIdError(
      `Invalid ID length (length ${s.length}, expected between ${MIN_BASE32_LEN} and ${MAX_BASE32_LEN})`,
    );
  }
  const buf = decodeBase32(s);
  const { n: tableNumber, bytesRead } = vintDecode(buf);
  const internalId = buf.slice(bytesRead, bytesRead + 16);
  if (internalId.length < 16) {
    throw new InvalidIdError("Input truncated");
  }
  const expectedFooter = fletcher16(buf.slice(0, bytesRead + 16)) ^ version;
  const footerView = new DataView(buf.slice(bytesRead + 16).buffer);
  if (footerView.byteLength !== 2) {
    throw new InvalidIdError("Input truncated");
  }
  const footer = footerView.getUint16(0, true);
  if (expectedFooter !== footer) {
    throw new InvalidIdError("Invalid version");
  }
  return { tableNumber, internalId };
}

export function isId(s: string): boolean {
  try {
    decodeId(s);
    return true;
  } catch (e) {
    if (e instanceof InvalidIdError || e instanceof InvalidBase32Error) {
      return false;
    } else {
      throw e;
    }
  }
}
