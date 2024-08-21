import { Context, logVerbose } from "../../../bundler/context.js";
import {
  bigBrainAPI,
  bigBrainFetch,
  logAndHandleFetchError,
} from "../utils/utils.js";

export async function bigBrainStart(
  ctx: Context,
  data: {
    // cloud port
    port: number;
    projectSlug: string;
    teamSlug: string;
    instanceName: string | null;
  },
): Promise<{ deploymentName: string; adminKey: string }> {
  return bigBrainAPI({
    ctx,
    method: "POST",
    url: "/api/local_deployment/start",
    data,
  });
}

export async function bigBrainPause(
  ctx: Context,
  data: {
    projectSlug: string;
    teamSlug: string;
  },
): Promise<void> {
  const fetch = await bigBrainFetch(ctx);
  try {
    const resp = await fetch("/api/local_deployment/pause", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(data),
    });
    if (!resp.ok) {
      logVerbose(ctx, "Failed to pause local deployment");
    }
  } catch (e) {
    return logAndHandleFetchError(ctx, e);
  }
}

export async function bigBrainRecordActivity(
  ctx: Context,
  data: {
    instanceName: string;
  },
) {
  return bigBrainAPI({
    ctx,
    method: "POST",
    url: "/api/local_deployment/record_activity",
    data,
  });
}
