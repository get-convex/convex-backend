import { action } from "./_generated/server";

export const hello = action(async (_, { somebody }: { somebody: string }) => {
  return `Aloha, ${somebody}!`;
});
