"use node";

import { action } from "./_generated/server";

export const danglingFetch = action(async (_, { url }: { url: string }) => {
  console.log("hitting url:", url);
  void fetch(url);

  return "dangling fetch result";
});
