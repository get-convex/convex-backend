import { Id } from "./_generated/dataModel";
import { query } from "./_generated/server";

export default query(async ({ db }, { id }: { id: Id<any> }) => {
  return await (db.get(id) as any);
});
