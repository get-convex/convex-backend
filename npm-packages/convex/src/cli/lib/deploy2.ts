import { changeSpinner, Context, logFailure } from "../../bundler/context.js";
import { deploymentFetch, logAndHandleFetchError } from "./utils/utils.js";
import {
  StartPushRequest,
  startPushResponse,
  StartPushResponse,
} from "./deployApi/startPush.js";
import {
  AppDefinitionConfig,
  ComponentDefinitionConfig,
} from "./deployApi/definitionConfig.js";

/** Push configuration2 to the given remote origin. */
export async function startPush(
  ctx: Context,
  url: string,
  request: StartPushRequest,
  verbose?: boolean,
): Promise<StartPushResponse> {
  if (verbose) {
    const custom = (_k: string | number, s: any) =>
      typeof s === "string" ? s.slice(0, 40) + (s.length > 40 ? "..." : "") : s;
    console.log(JSON.stringify(request, custom, 2));
  }
  const onError = (err: any) => {
    if (err.toString() === "TypeError: fetch failed") {
      changeSpinner(ctx, `Fetch failed, is ${url} correct? Retrying...`);
    }
  };
  const fetch = deploymentFetch(url, request.adminKey, onError);
  changeSpinner(ctx, "Analyzing and deploying source code...");
  try {
    const response = await fetch("/api/deploy2/start_push", {
      body: JSON.stringify(request),
      method: "POST",
    });
    return startPushResponse.parse(await response.json());
  } catch (error: unknown) {
    // TODO incorporate AuthConfigMissingEnvironmentVariable logic
    logFailure(ctx, "Error: Unable to start push to " + url);
    return await logAndHandleFetchError(ctx, error);
  }
}

export async function waitForSchema(
  ctx: Context,
  adminKey: string,
  url: string,
  startPush: StartPushResponse,
) {
  const fetch = deploymentFetch(url, adminKey);
  try {
    const response = await fetch("/api/deploy2/wait_for_schema", {
      body: JSON.stringify({
        adminKey,
        schemaChange: (startPush as any).schemaChange,
        dryRun: false,
      }),
      method: "POST",
    });
    return await response.json();
  } catch (error: unknown) {
    // TODO incorporate AuthConfigMissingEnvironmentVariable logic
    logFailure(ctx, "Error: Unable to wait for schema from " + url);
    return await logAndHandleFetchError(ctx, error);
  }
}

export async function finishPush(
  ctx: Context,
  adminKey: string,
  url: string,
  startPush: StartPushResponse,
): Promise<void> {
  const fetch = deploymentFetch(url, adminKey);
  changeSpinner(ctx, "Finalizing push...");
  try {
    const response = await fetch("/api/deploy2/finish_push", {
      body: JSON.stringify({
        adminKey,
        startPush,
        dryRun: false,
      }),
      method: "POST",
    });
    return await response.json();
  } catch (error: unknown) {
    // TODO incorporate AuthConfigMissingEnvironmentVariable logic
    logFailure(ctx, "Error: Unable to finish push to " + url);
    return await logAndHandleFetchError(ctx, error);
  }
}

export type ComponentDefinitionConfigWithoutImpls = Omit<
  ComponentDefinitionConfig,
  "schema" | "functions"
>;
export type AppDefinitionConfigWithoutImpls = Omit<
  AppDefinitionConfig,
  "schema" | "functions" | "auth"
>;
