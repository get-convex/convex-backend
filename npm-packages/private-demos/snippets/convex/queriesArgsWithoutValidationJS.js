import { query } from "./_generated/server";

export const sum = query({
  handler: (_, args) => {
    return args.a + args.b;
  },
});
