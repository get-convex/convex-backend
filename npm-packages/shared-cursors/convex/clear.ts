import { mutation } from "./_generated/server";

export default mutation(async ({ db }) => {
  const toDelete = [];
  for await (const document of db.query("positions")) {
    toDelete.push(document._id);
  }
  for (const id of toDelete) {
    await db.delete(id);
  }
});
