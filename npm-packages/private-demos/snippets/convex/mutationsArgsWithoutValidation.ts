import { mutation } from "./_generated/server";

// @skipNextLine
// eslint-disable-next-line @convex-dev/require-args-validator
export const mutateSomething = mutation({
  handler: (_, args: { a: number; b: number }) => {
    // do something with `args.a` and `args.b`

    // optionally return a value
    return "success";
  },
});
