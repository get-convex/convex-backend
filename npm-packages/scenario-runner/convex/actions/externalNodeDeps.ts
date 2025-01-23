"use node";
import { load } from "langchain/load";
import { Tiktoken } from "tiktoken/lite";
import { action } from "../_generated/server";
// eslint-disable-next-line @typescript-eslint/no-var-requires
const gpt2_base = require("tiktoken/encoders/gpt2.json");

// This use makes sure that the package gets marked as external
load;

export const encode = action((_, { str }: { str: string }): number[] => {
  const enc = new Tiktoken(
    gpt2_base.bpe_ranks,
    gpt2_base.special_tokens,
    gpt2_base.pat_str,
  );
  const tok = enc.encode(str);
  enc.free();
  return Array.from(tok);
});
