import { paginationOptsValidator } from "convex/server";
import { queryPrivateSystem } from "../secretSystemTables";
import { performOp } from "udf-syscall-ffi";

export default queryPrivateSystem({
  args: {
    paginationOpts: paginationOptsValidator,
  },
  handler: async () => {
    const tables: Record<number, string> = performOp(
      "getTableMappingWithoutSystemTables",
    );
    // We don't need to paginate but keep the PaginationResult return type for backwards
    // compatibility.
    return {
      page: Object.values(tables).map((name) => ({ name })),
      isDone: true,
      continueCursor: "end",
    };
  },
});
