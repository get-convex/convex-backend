import {
  changeSpinner,
  Context,
  logError,
  logFailure,
} from "../../bundler/context.js";
import { deploymentFetch, logAndHandleFetchError } from "./utils/utils.js";
import {
  schemaStatus,
  SchemaStatus,
  StartPushRequest,
  startPushResponse,
  StartPushResponse,
} from "./deployApi/startPush.js";
import {
  AppDefinitionConfig,
  ComponentDefinitionConfig,
} from "./deployApi/definitionConfig.js";
import chalk from "chalk";

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

// Long poll every 10s for progress on schema validation.
const SCHEMA_TIMEOUT_MS = 10_000;

export async function waitForSchema(
  ctx: Context,
  adminKey: string,
  url: string,
  startPush: StartPushResponse,
) {
  const fetch = deploymentFetch(url, adminKey);

  changeSpinner(
    ctx,
    "Backfilling indexes and checking that documents match your schema...",
  );

  while (true) {
    let currentStatus: SchemaStatus;
    try {
      const response = await fetch("/api/deploy2/wait_for_schema", {
        body: JSON.stringify({
          adminKey,
          schemaChange: startPush.schemaChange,
          timeoutMs: SCHEMA_TIMEOUT_MS,
        }),
        method: "POST",
      });
      currentStatus = schemaStatus.parse(await response.json());
    } catch (error: unknown) {
      logFailure(ctx, "Error: Unable to wait for schema from " + url);
      return await logAndHandleFetchError(ctx, error);
    }
    switch (currentStatus.type) {
      case "inProgress": {
        let schemaDone = true;
        let indexesComplete = 0;
        let indexesTotal = 0;
        for (const componentStatus of Object.values(currentStatus.components)) {
          if (!componentStatus.schemaValidationComplete) {
            schemaDone = false;
          }
          indexesComplete += componentStatus.indexesComplete;
          indexesTotal += componentStatus.indexesTotal;
        }
        const indexesDone = indexesComplete === indexesTotal;
        let msg: string;
        if (!indexesDone && !schemaDone) {
          msg = `Backfilling indexes (${indexesComplete}/${indexesTotal} ready) and checking that documents match your schema...`;
        } else if (!indexesDone) {
          msg = `Backfilling indexes (${indexesComplete}/${indexesTotal} ready)...`;
        } else {
          msg = "Checking that documents match your schema...";
        }
        changeSpinner(ctx, msg);
        break;
      }
      case "failed": {
        // Schema validation failed. This could be either because the data
        // is bad or the schema is wrong. Classify this as a filesystem error
        // because adjusting `schema.ts` is the most normal next step.
        let msg = "Schema validation failed";
        if (currentStatus.componentPath) {
          msg += ` in component "${currentStatus.componentPath}"`;
        }
        msg += ".";
        logFailure(ctx, msg);
        logError(ctx, chalk.red(`${currentStatus.error}`));
        return await ctx.crash({
          exitCode: 1,
          errorType: {
            "invalid filesystem or db data": currentStatus.tableName ?? null,
          },
          printedMessage: null, // TODO - move logging into here
        });
      }
      case "raceDetected": {
        return await ctx.crash({
          exitCode: 1,
          errorType: "fatal",
          printedMessage: `Schema was overwritten by another push.`,
        });
      }
      case "complete": {
        changeSpinner(ctx, "Schema validation complete.");
        return;
      }
    }
  }
}

export async function finishPush(
  ctx: Context,
  adminKey: string,
  url: string,
  startPush: StartPushResponse,
): Promise<void> {
  changeSpinner(ctx, "Finalizing push...");
  const fetch = deploymentFetch(url, adminKey);
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
