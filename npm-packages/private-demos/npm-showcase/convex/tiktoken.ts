"use node";
import { action } from "./_generated/server";
import { Tiktoken } from "tiktoken/lite";
// eslint-disable-next-line @typescript-eslint/no-var-requires
const gpt2_base = require("tiktoken/encoders/gpt2.json");

export const encode = action(
  async (_, { str }: { str: string }): Promise<number[]> => {
    const enc = new Tiktoken(
      gpt2_base.bpe_ranks,
      gpt2_base.special_tokens,
      gpt2_base.pat_str,
    );
    const tok = enc.encode(str);
    enc.free();
    return Array.from(tok);
  },
);

export const decode = action(
  async (_, { arr }: { arr: number[] }): Promise<string> => {
    const enc = new Tiktoken(
      gpt2_base.bpe_ranks,
      gpt2_base.special_tokens,
      gpt2_base.pat_str,
    );
    const buf = new Uint32Array(arr);
    const result = new TextDecoder().decode(enc.decode(buf));
    enc.free();
    return result;
  },
);
