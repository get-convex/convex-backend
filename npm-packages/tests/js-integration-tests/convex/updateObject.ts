import { Id } from "./_generated/dataModel";
import { mutation } from "./_generated/server";

export default mutation(
  async ({ db }, { id, field }: { id: Id<"any">; field: string }) => {
    const obj: any = await db.get(id);
    obj[field] = obj[field] + 1;
    // Could use `db.patch` but using `replace` for code coverage.
    await db.replace(id, obj);
    return await db.get(id);
  },
);
