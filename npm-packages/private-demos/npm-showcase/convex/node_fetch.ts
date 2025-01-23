"use node";
import { action } from "./_generated/server";
import fetch from "node-fetch";

export const fetchUrl = action(
  async (_, { url }: { url: string }): Promise<string> => {
    const result = await fetch(url);
    return await result.text();
  },
);
