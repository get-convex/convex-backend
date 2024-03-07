import { queryPrivateSystem } from "../secretSystemTables";
export default queryPrivateSystem({
  args: {},
  handler: async function ({ db }): Promise<string | null> {
    const doc = await db.query("_udf_config").order("desc").unique();
    return doc?.serverVersion || null;
  },
});
