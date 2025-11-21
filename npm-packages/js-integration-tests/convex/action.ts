import { action } from "./_generated/server";

export const hello = action({
  handler: async (_, { somebody }: { somebody: string }) => {
    return `Aloha, ${somebody}!`;
  },
});
