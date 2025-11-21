import { query } from "./_generated/server";
import * as IdEncoding from "id-encoding";

export const decode = query({
  handler: async (_, args: { id: string }) => {
    const { internalId, tableNumber } = IdEncoding.decodeId(args.id);
    return { tableNumber, internalId: internalId.buffer };
  },
});

export const isId = query({
  handler: async (_, args: { id: string }) => {
    const result = IdEncoding.isId(args.id);
    return { result };
  },
});
