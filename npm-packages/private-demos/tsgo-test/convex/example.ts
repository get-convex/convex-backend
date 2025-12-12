import { query } from "./_generated/server";

export const hello = query({
  args: {},
  handler: async (): Promise<string> => {
    return "Hello from TypeScript!";
  },
});
