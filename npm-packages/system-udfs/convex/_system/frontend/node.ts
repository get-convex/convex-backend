import { queryPrivateSystem } from "../secretSystemTables";

export const version = queryPrivateSystem({
  args: {},
  handler: async function ({ db }) {
    const version = await db
      .query("_aws_lambda_versions")
      .order("desc")
      .filter((q) => q.eq(q.field("typeConfig.type"), "static"))
      .first();

    if (!version) {
      return null;
    }

    return version.lambdaConfig.runtime;
  },
});
