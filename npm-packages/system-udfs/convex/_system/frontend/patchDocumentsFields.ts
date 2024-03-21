import { Value, v, GenericId, ConvexError } from "convex/values";
import { mutationGeneric } from "../server";

// Since clients cannot send `undefined` through a serialized Convex Value to a server function,
// we use a placeholder for it.
export const UNDEFINED_PLACEHOLDER =
  "__CONVEX_PLACEHOLDER_undefined_I23atX0jcndVbFgXoQZffsih7eAqktCyFjgUuAeNBtfr3ySOljPSPSEOPFgprkdBO3zXNiGEJxmJ5ZFPc5C5qKesG80QRPvlJe8vgSxAt9feLTwxTg4PHfVwUaTEJU67FDwldWmTxp1guMPwxQ2jOuhEryTBf3mQ";

export default mutationGeneric({
  args: {
    table: v.string(),
    fields: v.any(),
    ids: v.optional(v.array(v.string())),
  },
  handler: async (
    ctx,
    args,
  ): Promise<{ success: false; error: string } | { success: true }> => {
    const fields = args.fields as Record<
      string,
      Value | typeof UNDEFINED_PLACEHOLDER
    >;
    const ids = args.ids as GenericId<string>[] | undefined;
    const documents =
      ids !== undefined
        ? await Promise.all(ids.map((id) => ctx.db.get(id)))
        : await ctx.db.query(args.table).collect();
    try {
      const patchFields: Record<string, Value | undefined> = {};
      for (const key in fields) {
        const value = fields[key];
        patchFields[key] = value === UNDEFINED_PLACEHOLDER ? undefined : value;
      }

      await Promise.all(
        documents.map((document) => ctx.db.patch(document._id, patchFields)),
      );
    } catch (e: any) {
      // Rewrapping this error because it could be a schema validation error.
      throw new ConvexError(e.message);
    }
    return { success: true };
  },
});
