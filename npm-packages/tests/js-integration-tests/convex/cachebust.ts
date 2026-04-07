import { query } from "./_generated/server";

export default query(async ({ db }, { cacheBust: _ }: { cacheBust?: any }) => {
  // invalidated by users changing
  return await db.query("users").collect();
});
