import { v } from "convex/values";
import { action } from "./_generated/server";

export const hello = action({
  args: { somebody: v.string() },
  returns: v.string(),
  handler: async (_ctx, { somebody }: { somebody: string }) => {
    console.log(`Aloha, ${somebody}!`);
    return `Aloha, ${somebody}!`;
  },
});

export const userError = action(async () => {
  throw new Error("I failed you!");
});

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export const userTimeout = action(async () => {
  await sleep(60 * 60 * 1000);
  return "Success";
});
