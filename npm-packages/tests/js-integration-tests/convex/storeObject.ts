import { mutation } from "./_generated/server";

export default mutation(async ({ db }, obj) => {
  const id = await db.insert("any", obj);
  return (await db.get(id))!;
});

export const throwError = mutation({
  args: {},
  handler: async () => {
    throw new Error("Failure is temporary, undefined is forever.");
  },
});
