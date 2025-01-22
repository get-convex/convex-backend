import { mutation } from "./_generated/server";

export default mutation(async ({ db }, { name }: { name: string }) => {
  await db.insert("users", { name });
});
