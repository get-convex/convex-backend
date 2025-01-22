import { paginationOptsValidator } from "convex/server";
import { mutation, query } from "./_generated/server";

export const count = mutation(async ({ db }) => {
  const bigNumber =
    (await db.query("numbers").withIndex("number").order("desc").first())
      ?.number ?? 0;
  for (let i = 1; i <= 10; i++) {
    await db.insert("numbers", { number: bigNumber + i });
  }
});

export const numberPage = query({
  args: { paginationOpts: paginationOptsValidator },
  handler: async (ctx, args) => {
    const results = await ctx.db
      .query("numbers")
      .withIndex("number")
      .order("desc")
      .paginate(args.paginationOpts);
    const numbers = results.page.map((d) => d.number);
    const filtered = numbers.filter((n) => n % 100 === 0);
    return {
      ...results,
      page: filtered.map((n, i) => {
        if (i === 0) {
          return `${n} -- start page`;
        }
        return n;
      }),
    };
  },
});
