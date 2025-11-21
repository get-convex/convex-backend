import { PaginationResult } from "convex/server";
import { internal } from "./_generated/api";
import { Doc, TableNames } from "./_generated/dataModel";
import { ActionCtx, QueryCtx, internalQuery } from "./_generated/server";

export async function paginate<T extends TableNames>(
  ctx: ActionCtx,
  table: T,
  batchSize: number,
  callback: (documents: Doc<T>[]) => Promise<void>,
): Promise<void> {
  let isDone = false;
  let cursor = null;
  while (!isDone) {
    const result: PaginationResult<Doc<T>> = (await ctx.runQuery(
      internal.helpers.paginateQuery,
      {
        table,
        cursor,
        numItems: batchSize,
      },
    )) as any;
    await callback(result.page);
    ({ isDone, continueCursor: cursor } = result);
  }
}

export const paginateQuery = internalQuery({
  handler: async <T extends TableNames>(
    ctx: QueryCtx,
    args: { table: T; cursor: any; numItems: number },
  ) => {
    return await ctx.db
      .query(args.table)
      .paginate({ cursor: args.cursor, numItems: args.numItems });
  },
});
