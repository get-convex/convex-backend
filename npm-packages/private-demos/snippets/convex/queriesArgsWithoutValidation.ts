import { query } from "./_generated/server";

// @skipNextLine
// eslint-disable-next-line @convex-dev/require-args-validator
export const sum = query({
  handler: (_, args: { a: number; b: number }) => {
    return args.a + args.b;
  },
});
