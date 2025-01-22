import { Context } from "../../../bundler/context.js";
import { bigBrainAPI } from "../utils/utils.js";

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
  return bigBrainAPI({
    ctx,
    method: "POST",
    url: "/api/local_deployment/pause",
    data,
  });
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

export async function bigBrainEnableFeatureMetadata(
  ctx: Context,
): Promise<{ totalProjects: { kind: "none" | "one" | "multiple" } }> {
  return bigBrainAPI({
    ctx,
    method: "POST",
    url: "/api/local_deployment/enable_feature_metadata",
    data: {},
  });
}

export async function projectHasExistingDev(
  ctx: Context,
  {
    projectSlug,
    teamSlug,
  }: {
    projectSlug: string;
    teamSlug: string;
  },
) {
  const response = await bigBrainAPI<
    | {
        kind: "exists";
      }
    | {
        kind: "doesNotExist";
      }
  >({
    ctx,
    method: "POST",
    url: "/api/deployment/existing_dev",
    data: { projectSlug, teamSlug },
  });
  return response;
}
