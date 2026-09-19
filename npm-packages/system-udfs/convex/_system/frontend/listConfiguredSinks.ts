import { Integration } from "./common";
import { queryPrivateSystem } from "../secretSystemTables";
export default queryPrivateSystem("ViewIntegrations")({
  args: {},
  handler: async ({ db }): Promise<Integration[]> => {
    const sinks = await db.query("_log_sinks").collect();
    return sinks.map((sink) => {
      if (sink.config.type !== "s3Export") {
        return sink;
      }
      // The AWS secret access key is write-only: `ViewIntegrations` is a read
      // permission, so the stored secret must not leave the deployment.
      const { secretAccessKey: _secret, ...config } = sink.config;
      return { ...sink, config };
    });
  },
});
