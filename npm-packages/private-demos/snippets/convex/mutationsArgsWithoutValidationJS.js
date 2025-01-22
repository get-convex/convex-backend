import { mutation } from "./_generated/server";

export const mutateSomething = mutation({
  handler: (_, args) => {
    // do something with `args.a` and `args.b`

    // optionally return a value
    return "success";
  },
});
