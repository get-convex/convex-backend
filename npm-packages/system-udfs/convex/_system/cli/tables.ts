import { paginationOptsValidator } from "convex/server";
import { queryPrivateSystem } from "../secretSystemTables";
import { maximumBytesRead, maximumRowsRead } from "../paginationLimits";

export default queryPrivateSystem({
  args: {
    paginationOpts: paginationOptsValidator,
  },
  handler: async (ctx, { paginationOpts }) => {
    const results = await ctx.db
      .query("_tables")
      .filter((q) => q.eq(q.field("state"), "active"))
      .paginate({ ...paginationOpts, maximumBytesRead, maximumRowsRead });
    return {
      ...results,
      page: results.page
        .filter((table) => !table.name.startsWith("_"))
        .map((table) => ({ name: table.name })),
    };
  },
});
