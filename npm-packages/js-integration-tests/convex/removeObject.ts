import { Id } from "./_generated/dataModel";
import { mutation } from "./_generated/server";

export default mutation(async ({ db }, { id }: { id: Id<any> }) => {
  await db.delete(id);
});
