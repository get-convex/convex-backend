import { throwNotImplementedMethodError } from "./helpers.js";
import { performOp } from "./syscall.js";
import { copyBuffer } from "./crypto/helpers.js";

class TextEncoder {
  get encoding() {
    return "utf-8";
  }
  encode(text = "") {
    if (typeof text !== "string") text = (text as any).toString();
    return performOp("textEncoder/encode", text);
  }
  encodeInto(input: string, dest: Uint8Array) {
    const space = dest.length;
    const output = performOp("textEncoder/encodeInto", input, space);
    const { bytes, read, written } = output;
    dest.set(bytes, 0);
    return { read, written };
  }
  toString() {
    return "[object TextEncoder]";
  }
}

class TextDecoder {
  #encoding: string;
  #fatal: boolean;
  #ignoreBOM: boolean;
  constructor(label = "utf-8", options: TextDecoderOptions = {}) {
    const { label: encoding, error } = performOp(
      "textEncoder/normalizeLabel",
      label,
    );
    if (error) {
      throw new DOMException(error, "RangeError");
    }

    this.#encoding = encoding;
    this.#fatal = options.fatal || false;
    this.#ignoreBOM = options.ignoreBOM || false;
  }
  get encoding() {
    return this.#encoding;
  }

  get fatal() {
    return this.#fatal;
  }

  get ignoreBOM() {
    return this.#ignoreBOM;
  }

  decode(buffer?: ArrayBufferView | ArrayBuffer, options?: TextDecodeOptions) {
    if (options !== undefined && options.stream) {
      throwNotImplementedMethodError("{stream: true}", "TextDecoder.decode");
    }

    if (buffer === undefined) {
      return "";
    }

    const { text } = performOp("textEncoder/decode", {
      bytes: copyBuffer(buffer),
      encoding: this.encoding,
      fatal: this.fatal,
      ignoreBOM: this.ignoreBOM,
    });
    return text;
  }
  toString() {
    return "[object TextDecoder]";
  }
}

function atob(encoded: string): string {
  const { decoded, error } = performOp("atob", encoded);
  if (error) {
    throw new DOMException(
      `Failed to execute 'atob': ${error}`,
      "InvalidCharacterError",
    );
  }
  return decoded;
}

function btoa(text: string): string {
  const { encoded, error } = performOp("btoa", text);
  if (error) {
    throw new DOMException(
      `Failed to execute 'btoa': ${error}`,
      "InvalidCharacterError",
    );
  }
  return encoded;
}

export const setupTextEncoding = (global: any) => {
  global.atob = atob;
  global.btoa = btoa;
  global.TextEncoder = TextEncoder;
  global.TextDecoder = TextDecoder;
};
