import { Infer } from "convex/values";
import { queryPrivateSystem } from "../secretSystemTables";
import { noPermissionRequired } from "../server";

import { oldBackendState } from "../../tableDefs/deploymentAuditLogTable";
import { backendState } from "../../schema";

type OldBackendState = Infer<typeof oldBackendState>;
type BackendState = Infer<typeof backendState>;

export const deploymentState = queryPrivateSystem(noPermissionRequired)({
  args: {},
  handler: async function ({ db }): Promise<{ state: OldBackendState }> {
    const { state } = (await db.query("_backend_state").first())!;
    return { state: toOldBackendStateLossy(state) };
  },
});

function toOldBackendStateLossy(state: BackendState): OldBackendState {
  // TODO(nicolas) Remove this once the migration has run
  if (typeof state === "string") return state;

  if (state.system === "disabled") return "disabled";
  if (state.system === "suspended") return "suspended";
  if (state.user === "paused") return "paused";
  return "running";
}
