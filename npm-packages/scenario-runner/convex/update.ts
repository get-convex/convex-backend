import { MESSAGES_TABLE } from "../types";
import { mutation } from "./_generated/server";

export default mutation(async ({ db }): Promise<void> => {
  // Update an existing document. This makes the mutations conflicts with each
  // other and generates a record with a lot of versions which helps us test
  // the database is efficiently processing those.
  // We add a row with rand=0 in setup so this should never fail.
  const row = await db
    .query(MESSAGES_TABLE)
    .withIndex("by_channel_rand", (q) =>
      q.eq("channel", "global").eq("rand", 0),
    )
    .first();
  if (row === null) {
    throw new Error("No rows!");
  }
  const timestamp = Date.now();
  if (row.timestamp < timestamp) {
    await db.patch(row._id, { timestamp: timestamp });
  }
});
