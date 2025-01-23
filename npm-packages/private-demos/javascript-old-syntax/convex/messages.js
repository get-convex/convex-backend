// @snippet start list
import { query, mutation } from "./_generated/server";

export const list = query(async ({ db }) => {
  return await db.query("messages").collect();
});
// @snippet end list

// @snippet start send
export const send = mutation(async ({ db }, { body, author }) => {
  const message = { body, author };
  await db.insert("messages", message);
});
// @snippet end send
